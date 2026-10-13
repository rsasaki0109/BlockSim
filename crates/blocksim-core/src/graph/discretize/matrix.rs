//! MIMO discretization without cancellation or a transfer-polynomial reconstruction.
use super::*;
use crate::graph::{matrix_block, Block, BlockKind};
type Matrix = Vec<Vec<f64>>;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MatrixMethod {
    Zoh,
    Tustin,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixRequest {
    pub a: Matrix,
    pub b: Matrix,
    pub c: Matrix,
    pub d: Matrix,
    pub initial: Vec<f64>,
    pub sample_time: f64,
    pub method: MatrixMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prewarp_omega: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixResult {
    pub request: MatrixRequest,
    pub a: Matrix,
    pub b: Matrix,
    pub c: Matrix,
    pub d: Matrix,
    pub initial: Vec<f64>,
    pub mapping_scale: Option<f64>,
    pub squarings: usize,
    pub terms: usize,
    pub residual: f64,
    pub method: String,
}
fn product(a: &Matrix, b: &Matrix, columns: usize) -> Result<Matrix> {
    let mut out = vec![vec![0.; columns]; a.len()];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            for (k, br) in b.iter().enumerate() {
                let v = a[i][k] * br[j];
                finite("matrix conversion product", v)?;
                if a[i][k] != 0. && br[j] != 0. && v == 0. {
                    return Err("matrix conversion product underflow; unresolved".into());
                }
                *value += v;
            }
            finite("matrix conversion sum", *value)?;
        }
    }
    Ok(out)
}
fn scaled(value: f64, factor: f64) -> Result<f64> {
    let out = value * factor;
    finite("matrix conversion scaling", out)?;
    if value != 0. && factor != 0. && out == 0. {
        return Err("matrix conversion scaling underflow; unresolved".into());
    }
    Ok(out)
}
fn inverse(a: &Matrix) -> Result<(Matrix, f64)> {
    let n = a.len();
    let scale = zoh::norm(a);
    finite("Tustin matrix norm", scale)?;
    if scale == 0. {
        return Err("Tustin I-A/k is singular; conversion unresolved".into());
    }
    let mut work = a.clone();
    let mut inv = zoh::identity(n);
    for (row, right) in work.iter_mut().zip(&mut inv) {
        for v in row.iter_mut().chain(right) {
            *v /= scale;
        }
    }
    for k in 0..n {
        let pivot = (k..n)
            .max_by(|i, j| work[*i][k].abs().total_cmp(&work[*j][k].abs()))
            .unwrap();
        if work[pivot][k].abs() < 1e-12 {
            return Err("Tustin I-A/k is singular or numerically unresolved".into());
        }
        work.swap(k, pivot);
        inv.swap(k, pivot);
        let divisor = work[k][k];
        for j in 0..n {
            work[k][j] /= divisor;
            inv[k][j] /= divisor;
        }
        for i in 0..n {
            if i != k {
                let factor = work[i][k];
                for j in 0..n {
                    work[i][j] -= factor * work[k][j];
                    inv[i][j] -= factor * inv[k][j];
                }
            }
        }
        for v in work.iter().flatten().chain(inv.iter().flatten()) {
            finite("Tustin solve", *v)?;
        }
    }
    let check = product(a, &inv, n)?;
    let mut residual: f64 = 0.;
    for (i, row) in check.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            residual = residual.max((v - if i == j { 1. } else { 0. }).abs());
        }
    }
    if residual > 1e-10 {
        return Err("Tustin inverse residual exceeds tolerance; unresolved".into());
    }
    Ok((inv, residual))
}
pub fn discretize_matrix(q: &MatrixRequest) -> Result<MatrixResult> {
    let block = Block {
        id: "matrix".into(),
        label: "matrix".into(),
        width: q.c.len(),
        position: [0., 0.],
        params: BlockKind::StateSpace {
            a: q.a.clone(),
            b: q.b.clone(),
            c: q.c.clone(),
            d: q.d.clone(),
            initial: q.initial.clone(),
            mode: PidMode::Continuous,
            sample_time: q.sample_time,
        },
    };
    matrix_block::validate(&block, q.sample_time)?;
    let n = q.a.len();
    let m = q.d[0].len();
    let p = q.c.len();
    let (a, b, c, d, mapping_scale, squarings, terms, residual, description) = match q.method {
        MatrixMethod::Zoh => {
            if q.prewarp_omega.is_some() {
                return Err("ZOH does not accept prewarp_omega".into());
            }
            let mut augmented = vec![vec![0.; n + m]; n + m];
            for (i, row) in augmented.iter_mut().enumerate().take(n) {
                for (j, v) in row.iter_mut().enumerate() {
                    *v = scaled(if j < n { q.a[i][j] } else { q.b[i][j - n] }, q.sample_time)?;
                }
            }
            let (exp, squares, terms) = zoh::exponential(&augmented)?;
            let a: Matrix = exp[..n].iter().map(|r| r[..n].to_vec()).collect();
            let b: Matrix = exp[..n].iter().map(|r| r[n..].to_vec()).collect();
            let lhs = product(&q.a, &b, m)?;
            let mut delta = a.clone();
            for (i, r) in delta.iter_mut().enumerate() {
                r[i] -= 1.;
            }
            let rhs = product(&delta, &q.b, m)?;
            let bound = 1. + zoh::norm(&q.a) * zoh::norm(&b) + zoh::norm(&delta) * zoh::norm(&q.b);
            finite("ZOH residual scale", bound)?;
            let residual = lhs
                .iter()
                .flatten()
                .zip(rhs.iter().flatten())
                .map(|(x, y)| (x - y).abs() / bound)
                .fold(0., f64::max);
            (a,b,q.c.clone(),q.d.clone(),None,squares,terms,residual,"ZOH augmented exponential; original state coordinates and initial state; sampled held-input response")
        }
        MatrixMethod::Tustin => {
            if q.initial.iter().any(|v| *v != 0.) {
                return Err("Tustin matrix conversion requires zero initial state; original state is preserved on rejection".into());
            }
            let k = match q.prewarp_omega {
                None => 2. / q.sample_time,
                Some(w) => {
                    positive("Tustin prewarp_omega", w)?;
                    if w * q.sample_time >= std::f64::consts::PI {
                        return Err("Tustin prewarp must be below Nyquist".into());
                    }
                    w / (w * q.sample_time / 2.).tan()
                }
            };
            positive("Tustin mapping scale", k)?;
            let h = 1. / k;
            let mut matrix = zoh::identity(n);
            for (i, row) in matrix.iter_mut().enumerate() {
                for (j, v) in row.iter_mut().enumerate() {
                    *v -= scaled(q.a[i][j], h)?;
                    finite("Tustin I-A/k", *v)?;
                }
            }
            let (inv, residual) = if n == 0 {
                (vec![], 0.)
            } else {
                inverse(&matrix)?
            };
            // M=I-A/k. Ad=2 M^-1-I, Bd=2 M^-1 B/k, Cd=C M^-1,
            // Dd=D+C M^-1 B/k. This realization maps s=k(z-1)/(z+1).
            // Its transformed initial state depends on the first input, so do
            // not copy nonzero continuous initial coordinates into it.
            let mut a = inv.clone();
            for (i, row) in a.iter_mut().enumerate() {
                for (j, v) in row.iter_mut().enumerate() {
                    *v = 2. * *v - if i == j { 1. } else { 0. };
                }
            }
            let ib = product(&inv, &q.b, m)?;
            let mut b = ib.clone();
            for row in &mut b {
                for v in row {
                    *v = scaled(*v, 2. * h)?;
                }
            }
            let c = product(&q.c, &inv, n)?;
            let correction = product(&q.c, &ib, m)?;
            let mut d = q.d.clone();
            for i in 0..p {
                for j in 0..m {
                    d[i][j] += scaled(correction[i][j], h)?;
                }
            }
            (a,b,c,d,Some(k),0,0,residual,"Tustin bilinear realization; zero initial only; state coordinates changed; no cancellation")
        }
    };
    if !residual.is_finite() || residual > 1e-10 {
        return Err("matrix conversion residual exceeds tolerance; unresolved".into());
    }
    for v in a
        .iter()
        .flatten()
        .chain(b.iter().flatten())
        .chain(c.iter().flatten())
        .chain(d.iter().flatten())
    {
        finite("converted matrix coefficient", *v)?;
    }
    Ok(MatrixResult {
        request: q.clone(),
        a,
        b,
        c,
        d,
        initial: q.initial.clone(),
        mapping_scale,
        squarings,
        terms,
        residual,
        method: description.into(),
    })
}
