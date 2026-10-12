//! Controllable companion realization; independent copies for vector channels.
use super::*;
#[derive(Clone)]
pub(super) struct Coefficients {
    pub direct: f64,
    denominator: Vec<f64>,
    output: Vec<f64>,
}
pub(super) fn coefficients(numerator: &[f64], denominator: &[f64]) -> Result<Coefficients> {
    if numerator.is_empty()
        || denominator.is_empty()
        || denominator.len() > 9
        || numerator.len() > denominator.len()
    {
        return Err(
            "TransferFunction requires a nonempty proper numerator/denominator, order <= 8".into(),
        );
    }
    for value in numerator.iter().chain(denominator) {
        finite("TransferFunction coefficient", *value)?;
    }
    let leading = denominator[0];
    if leading == 0.0 {
        return Err("TransferFunction leading denominator must be nonzero".into());
    }
    let a: Vec<_> = denominator[1..].iter().map(|v| v / leading).collect();
    let mut b = vec![0.0; denominator.len()];
    let start = b.len() - numerator.len();
    for (dst, value) in b[start..].iter_mut().zip(numerator) {
        *dst = value / leading;
    }
    let output: Vec<_> = b[1..]
        .iter()
        .zip(&a)
        .map(|(coefficient, a)| coefficient - a * b[0])
        .collect();
    for value in a.iter().chain(b.iter()).chain(output.iter()) {
        finite("Normalized TransferFunction coefficient", *value)?;
    }
    Ok(Coefficients {
        direct: b[0],
        denominator: a,
        output,
    })
}
impl Coefficients {
    pub fn output(&self, state: &[f64], channel: usize, input: f64) -> f64 {
        let n = self.denominator.len();
        self.direct * input
            + self
                .output
                .iter()
                .zip(&state[channel * n..(channel + 1) * n])
                .map(|(c, x)| c * x)
                .sum::<f64>()
    }
    pub fn rates(&self, state: &[f64], inputs: &[f64]) -> Vec<f64> {
        let n = self.denominator.len();
        if n == 0 {
            return vec![];
        }
        state
            .chunks(n)
            .zip(inputs)
            .flat_map(|(x, u)| {
                std::iter::once(
                    u - self
                        .denominator
                        .iter()
                        .zip(x)
                        .map(|(a, x)| a * x)
                        .sum::<f64>(),
                )
                .chain(x[..n - 1].iter().copied())
            })
            .collect()
    }
}
