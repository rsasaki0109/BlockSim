//! Same-clock sampled response on z=exp(j*omega*Ts), never a continuous approximation.
use super::*;
use crate::graph::state_space::{
    discrete::{check_discrete, discrete_realization},
    StateSpaceRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscreteFrequencyCheck {
    pub source_label: String,
    pub scope_label: String,
    pub variables: usize,
    pub states: usize,
    pub sample_time: f64,
    pub nyquist_omega: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscreteFrequencyResult {
    #[serde(flatten)]
    pub response: FrequencyResult,
    pub sample_time: f64,
    pub nyquist_omega: f64,
    pub states: usize,
}
fn state_query(q: &FrequencyRequest) -> StateSpaceRequest {
    StateSpaceRequest {
        source: q.source.clone(),
        source_component: q.source_component,
        scope: q.scope.clone(),
        scope_component: q.scope_component,
    }
}
fn range(q: &FrequencyRequest, ts: f64) -> Result<f64> {
    let nyquist = std::f64::consts::PI / ts;
    finite("Discrete Nyquist angular frequency", nyquist)?;
    if !(1e-9..=1e9).contains(&q.min_omega)
        || !q.max_omega.is_finite()
        || q.max_omega > 1e9
        || q.max_omega > nyquist
        || q.max_omega <= q.min_omega
        || !(2..=2048).contains(&q.points)
    {
        return Err(format!("Discrete frequency requires 1e-9 <= min < max <= min(1e9, Nyquist {nyquist} rad/s), and 2..2048 points"));
    }
    if (q.max_omega.ln() - q.min_omega.ln()) / (q.points - 1) as f64 <= 64.0 * f64::EPSILON {
        return Err(
            "Discrete frequency logarithmic grid is too narrow for floating-point resolution"
                .into(),
        );
    }
    Ok(nyquist)
}
pub fn validate_discrete_frequency(
    model: &GraphModel,
    q: &FrequencyRequest,
) -> Result<DiscreteFrequencyCheck> {
    let (c, ts) = check_discrete(model, &state_query(q))?;
    let nyquist = range(q, ts)?;
    // State-space has <=8 states, yet division keeps this bound safe on wasm32.
    if q.points > 20_000_000 / c.states.max(1).pow(3) {
        return Err("Discrete frequency dense-solve budget exceeds 20000000".into());
    }
    Ok(DiscreteFrequencyCheck {
        source_label: c.source_label,
        scope_label: c.scope_label,
        variables: c.variables,
        states: c.states,
        sample_time: ts,
        nyquist_omega: nyquist,
    })
}
pub fn analyze_discrete_frequency(
    model: &GraphModel,
    q: &FrequencyRequest,
) -> Result<DiscreteFrequencyResult> {
    let check = validate_discrete_frequency(model, q)?;
    let r = discrete_realization(model, &state_query(q))?;
    let evaluate = |omega: f64| {
        let n = r.b.len();
        // Set the Nyquist endpoint exactly to z=-1 so a pole there remains a gap.
        let angle = if omega == check.nyquist_omega {
            std::f64::consts::PI
        } else {
            omega * r.sample_time
        };
        if !angle.is_finite() || angle == 0.0 {
            return Err(
                "Discrete unit-circle angle overflows or underflows finite resolution".into(),
            );
        }
        let z = if omega == check.nyquist_omega {
            C::real(-1.0)
        } else {
            C {
                re: angle.cos(),
                im: angle.sin(),
            }
        };
        let mut matrix = vec![vec![C::default(); n + 1]; n];
        for (i, row) in matrix.iter_mut().enumerate() {
            for (j, value) in row[..n].iter_mut().enumerate() {
                *value = (if i == j { z } else { C::default() } - C::real(r.a[i][j])).checked()?;
            }
            row[n] = C::real(r.b[i]);
        }
        let (x, residual) = if n == 0 {
            (vec![], 0.0)
        } else {
            solve(matrix)?
        };
        let value = r.c.iter().zip(&x).try_fold(C::real(r.d), |sum, (c, x)| {
            (sum + C::real(*c) * *x).checked()
        })?;
        finite("Discrete frequency magnitude", value.abs())?;
        Ok((value, residual))
    };
    let points = sample_points(q, &evaluate);
    let margins = q.open_loop.then(|| {
        let remaining = (20_000_000 / check.states.max(1).pow(3) - q.points).min(4096);
        let mut m = margins::analyze(&points, remaining, evaluate);
        m.method.push_str(" / discrete z=exp(j*omega*Ts) within Nyquist / sample-instant unit negative feedback / no stability inference");
        m
    });
    Ok(DiscreteFrequencyResult {response:FrequencyResult {request:q.clone(),check:FrequencyCheck {source_label:check.source_label,scope_label:check.scope_label,variables:check.variables},points,margins,method:"discrete same-clock Scope ancestry / actual sample-instant matrices / C(exp(j*omega*Ts)I-A)^-1 B+D / scaled partial-pivot solve / sampled phase unwrap / no hold reconstruction or stability inference".into()},sample_time:check.sample_time,nyquist_omega:check.nyquist_omega,states:check.states})
}
