//! Bounded augmented-matrix exponential and unreduced transfer reconstruction.
use super::*;
type Matrix = Vec<Vec<f64>>;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZohRequest {
    pub numerator: Vec<f64>,
    pub denominator: Vec<f64>,
    pub sample_time: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZohResult {
    pub request: ZohRequest,
    pub numerator: Vec<f64>,
    pub denominator: Vec<f64>,
    pub a: Matrix,
    pub b: Vec<f64>,
    pub c: Vec<f64>,
    pub d: f64,
    pub squarings: usize,
    pub terms: usize,
    pub exponential_identity_residual: f64,
    pub cayley_hamilton_residual: f64,
    pub reconstruction_residual: f64,
    pub method: String,
}
pub(super) fn identity(n: usize) -> Matrix {
    let mut m = vec![vec![0.0; n]; n];
    for (i, r) in m.iter_mut().enumerate() {
        r[i] = 1.0;
    }
    m
}
pub(super) fn norm(m: &Matrix) -> f64 {
    m.iter()
        .map(|r| r.iter().map(|v| v.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}
fn mul(a: &Matrix, b: &Matrix) -> Result<Matrix> {
    let n = a.len();
    let mut c = vec![vec![0.0; n]; n];
    for (i, row) in c.iter_mut().enumerate() {
        for (j, out) in row.iter_mut().enumerate() {
            for (k, r) in b.iter().enumerate() {
                let v = a[i][k] * r[j];
                finite("ZOH matrix product", v)?;
                if a[i][k] != 0.0 && r[j] != 0.0 && v == 0.0 {
                    return Err("ZOH matrix multiplication underflow; conversion unresolved".into());
                }
                *out += v;
            }
            finite("ZOH matrix sum", *out)?;
        }
    }
    Ok(c)
}
pub(super) fn exponential(m: &Matrix) -> Result<(Matrix, usize, usize)> {
    let bound = norm(m);
    finite("ZOH matrix norm", bound)?;
    let squares = if bound > 0.5 {
        (bound / 0.5).log2().ceil() as usize
    } else {
        0
    };
    if squares > 24 {
        return Err(
            "ZOH matrix exponential requires more than 24 squarings; conversion unresolved".into(),
        );
    }
    let divisor = 2.0_f64.powi(squares as i32);
    let mut scaled = m.clone();
    for row in &mut scaled {
        for v in row {
            let before = *v;
            *v /= divisor;
            if before != 0.0 && *v == 0.0 {
                return Err("ZOH matrix scaling underflow".into());
            }
        }
    }
    let mut out = identity(m.len());
    let mut term = out.clone();
    let mut used = 0;
    for k in 1..=32 {
        term = mul(&term, &scaled)?;
        for row in &mut term {
            for v in row {
                *v /= k as f64;
            }
        }
        for (row, t) in out.iter_mut().zip(&term) {
            for (v, t) in row.iter_mut().zip(t) {
                *v += t;
            }
        }
        used = k;
        if norm(&term) <= 2e-17 * norm(&out).max(1.0) {
            break;
        }
    }
    if used == 32 && norm(&term) > 2e-17 * norm(&out).max(1.0) {
        return Err("ZOH exponential series did not converge".into());
    }
    for _ in 0..squares {
        out = mul(&out, &out)?;
    }
    Ok((out, squares, used))
}
fn dot(a: &[f64], b: &[f64]) -> Result<f64> {
    let mut value = 0.0;
    for (&x, &y) in a.iter().zip(b) {
        let product = x * y;
        finite("ZOH dot product", product)?;
        if x != 0.0 && y != 0.0 && product == 0.0 {
            return Err("ZOH reconstruction product underflow".into());
        }
        value += product;
    }
    finite("ZOH output or reconstruction", value)?;
    Ok(value)
}
fn mv(a: &Matrix, x: &[f64]) -> Result<Vec<f64>> {
    a.iter().map(|row| dot(row, x)).collect()
}
pub fn discretize_zoh(q: &ZohRequest) -> Result<ZohResult> {
    let coefficients = transfer::coefficients(&q.numerator, &q.denominator)?;
    positive("ZOH sample_time", q.sample_time)?;
    let n = q.denominator.len() - 1;
    let leading = q.denominator[0];
    if q.numerator
        .iter()
        .chain(&q.denominator)
        .any(|v| *v != 0.0 && *v / leading == 0.0)
    {
        return Err("ZOH source coefficient normalization underflow; conversion unresolved".into());
    }
    let mut ac = vec![vec![0.0; n]; n];
    if n > 0 {
        for (j, v) in q.denominator[1..].iter().enumerate() {
            ac[0][j] = -v / leading;
        }
        for (i, row) in ac.iter_mut().enumerate().skip(1) {
            row[i - 1] = 1.0;
        }
    }
    let mut c = vec![];
    for i in 0..n {
        let mut x = vec![0.0; n];
        x[i] = 1.0;
        c.push(coefficients.output(&x, 0, 0.0));
    }
    let d = coefficients.direct;
    let mut augmented = vec![vec![0.0; n + 1]; n + 1];
    for (i, row) in ac.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            augmented[i][j] = v * q.sample_time;
            finite("ZOH scaled continuous state", augmented[i][j])?;
            if *v != 0.0 && augmented[i][j] == 0.0 {
                return Err("ZOH continuous state scaling underflow".into());
            }
        }
    }
    if n > 0 {
        augmented[0][n] = q.sample_time;
    }
    let (exp, squarings, terms) = exponential(&augmented)?;
    let a: Matrix = exp[..n].iter().map(|row| row[..n].to_vec()).collect();
    let b: Vec<_> = exp[..n].iter().map(|row| row[n]).collect();
    // A*Bd=(Ad-I)*B without assuming A is invertible (integrators are supported).
    let lhs = mv(&ac, &b)?;
    let mut identity_error: f64 = 0.0;
    for (i, v) in lhs.iter().enumerate() {
        let rhs = a[i][0] - if i == 0 { 1.0 } else { 0.0 };
        identity_error = identity_error.max((v - rhs).abs() / (1.0 + v.abs() + rhs.abs()));
    }
    if identity_error > 1e-10 {
        return Err("ZOH exponential input identity residual exceeds tolerance".into());
    }
    let mut den = vec![1.0];
    let mut num = vec![d];
    let mut f = identity(n);
    for k in 1..=n {
        let cb = dot(&c, &mv(&f, &b)?)?;
        let mut next = mul(&a, &f)?;
        let ak = -(0..n).map(|i| next[i][i]).sum::<f64>() / k as f64;
        finite("ZOH characteristic coefficient", ak)?;
        den.push(ak);
        num.push(d * ak + cb);
        for (i, row) in next.iter_mut().enumerate() {
            row[i] += ak;
        }
        f = next;
    }
    let bound = den
        .iter()
        .enumerate()
        .map(|(i, v)| v.abs() * norm(&a).powi((n - i) as i32))
        .sum::<f64>()
        .max(f64::MIN_POSITIVE);
    let ch = if n == 0 { 0.0 } else { norm(&f) / bound };
    if !ch.is_finite() || ch > 1e-10 {
        return Err("ZOH Cayley-Hamilton residual exceeds tolerance; transfer unresolved".into());
    }
    let converted = transfer::coefficients(&num, &den)?;
    let mut x = vec![0.0; n];
    let mut original = vec![0.0; n];
    let mut reconstruction: f64 = 0.0;
    for k in 0..(2 * n + 4) {
        let u = if k == 0 { 1.0 } else { 0.0 };
        let expected = dot(&c, &original)? + d * u;
        let value = converted.output(&x, 0, u);
        finite("ZOH reconstructed impulse", value)?;
        reconstruction =
            reconstruction.max((value - expected).abs() / (1.0 + value.abs() + expected.abs()));
        x = converted.rates(&x, &[u]);
        original = mv(&a, &original)?;
        for (v, bd) in original.iter_mut().zip(&b) {
            *v += bd * u;
        }
        for v in x.iter().chain(&original) {
            finite("ZOH reconstruction state", *v)?;
        }
    }
    if reconstruction > 1e-9 {
        return Err("ZOH companion impulse reconstruction exceeds tolerance".into());
    }
    Ok(ZohResult{request:q.clone(),numerator:num,denominator:den,a,b,c,d,squarings,terms,exponential_identity_residual:identity_error,cayley_hamilton_residual:ch,reconstruction_residual:reconstruction,method:"ZOH augmented matrix exponential / infinity-norm scaling, <=24 squarings, <=32 Taylor terms / unreduced characteristic and adjugate / sample-instant held-input response / no initial-state mapping or cancellation".into()})
}
