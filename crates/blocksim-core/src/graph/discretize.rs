//! Continuous-to-discrete coefficients, without initial-state mapping or cancellation.
use super::*;
mod zoh;
pub use zoh::{discretize_zoh, ZohRequest, ZohResult};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TustinRequest {
    pub numerator: Vec<f64>,
    pub denominator: Vec<f64>,
    pub sample_time: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prewarp_omega: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TustinResult {
    pub request: TustinRequest,
    pub numerator: Vec<f64>,
    pub denominator: Vec<f64>,
    pub mapping_scale: f64,
    pub leading_condition_ratio: f64,
    pub method: String,
}
fn basis(minus: usize, plus: usize) -> Vec<f64> {
    let mut p = vec![1.0];
    for sign in std::iter::repeat_n(-1.0, minus).chain(std::iter::repeat_n(1.0, plus)) {
        let mut next = vec![0.0; p.len() + 1];
        for (i, v) in p.iter().enumerate() {
            next[i] += sign * v;
            next[i + 1] += v;
        }
        p = next;
    }
    p.reverse();
    p
}
pub fn discretize_tustin(q: &TustinRequest) -> Result<TustinResult> {
    transfer::coefficients(&q.numerator, &q.denominator)?;
    positive("Tustin sample_time", q.sample_time)?;
    let k = if let Some(w) = q.prewarp_omega {
        positive("Tustin prewarp angular frequency", w)?;
        let angle = w * q.sample_time / 2.0;
        if !angle.is_finite() || angle <= 0.0 || angle >= std::f64::consts::FRAC_PI_2 {
            return Err("Tustin prewarp must be positive and strictly below Nyquist".into());
        }
        w / angle.tan()
    } else {
        2.0 / q.sample_time
    };
    positive("Tustin mapping scale", k)?;
    let n = q.denominator.len() - 1;
    let mut numerator = vec![0.0; n + 1];
    numerator[n + 1 - q.numerator.len()..].copy_from_slice(&q.numerator);
    // One common positive scale preserves N/D; use reciprocal powers when k>=1
    // so neither k^n nor large raw binomial products overflow during expansion.
    let scale = q
        .numerator
        .iter()
        .chain(&q.denominator)
        .map(|v| v.abs())
        .fold(0.0, f64::max);
    let convert = |p: &[f64]| -> Result<(Vec<f64>, f64)> {
        let mut out = vec![0.0; n + 1];
        let mut leading_sum = 0.0;
        for (i, &c) in p.iter().enumerate() {
            if c == 0.0 {
                continue;
            }
            let normalized = c / scale;
            let factor = if k >= 1.0 {
                (1.0 / k).powi(i as i32)
            } else {
                k.powi((n - i) as i32)
            };
            let weight = normalized * factor;
            if normalized == 0.0 || weight == 0.0 || !weight.is_finite() {
                return Err(
                    "Tustin coefficient range underflows or overflows; conversion unresolved"
                        .into(),
                );
            }
            leading_sum += weight.abs();
            for (dst, b) in out.iter_mut().zip(basis(n - i, i)) {
                *dst += weight * b;
            }
        }
        Ok((out, leading_sum))
    };
    let (mut a, sum) = convert(&q.denominator)?;
    let (mut b, _) = convert(&numerator)?;
    let leading = a[0];
    let ratio = leading.abs() / sum;
    if !ratio.is_finite() || ratio <= 1e-12 {
        return Err("Tustin transformed leading denominator is zero or ill-conditioned; conversion unresolved".into());
    }
    for c in a.iter_mut().chain(&mut b) {
        let original = *c;
        *c /= leading;
        if !c.is_finite() || original != 0.0 && *c == 0.0 {
            return Err(
                "Tustin normalized coefficients underflow or overflow; conversion unresolved"
                    .into(),
            );
        }
    }
    transfer::coefficients(&b, &a)?;
    Ok(TustinResult {request:q.clone(),numerator:b,denominator:a,mapping_scale:k,leading_condition_ratio:ratio,method:"Tustin bilinear substitution s=k(z-1)/(z+1); z-descending monic coefficients; optional prewarp; degree retained; no initial-state mapping, cancellation or ZOH equivalence".into()})
}
