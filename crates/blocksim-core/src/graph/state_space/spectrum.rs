use super::*;
type Spectrum = (
    Option<Vec<f64>>,
    f64,
    Option<f64>,
    PolynomialRoots,
    Vec<f64>,
    Stability,
);
fn unresolved(n: usize, scale: f64, reason: String) -> Spectrum {
    (
        None,
        scale,
        None,
        PolynomialRoots {
            status: "unresolved".into(),
            degree: Some(n),
            roots: vec![],
            reconstruction_error: None,
            warnings: vec![reason.clone()],
        },
        vec![],
        Stability {
            classification: "indeterminate".into(),
            right_half_plane_poles: None,
            tolerance: 1e-10,
            method: "scaled characteristic polynomial / Routh".into(),
            notes: vec![reason],
        },
    )
}
fn product(a: f64, b: f64) -> Result<f64> {
    let v = a * b;
    finite("State-space characteristic product", v)?;
    if a != 0.0 && b != 0.0 && v == 0.0 {
        return Err("State-space characteristic multiplication underflow".into());
    }
    Ok(v)
}
fn characteristic(a: &[Vec<f64>]) -> Result<(Vec<f64>, f64)> {
    let n = a.len();
    let mut previous = vec![vec![0.0; n]; n];
    for (i, row) in previous.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    let mut coefficients = vec![1.0];
    for k in 1..=n {
        let mut next = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                for (l, row) in previous.iter().enumerate() {
                    next[i][j] += product(a[i][l], row[j])?;
                }
                finite("State-space characteristic sum", next[i][j])?;
            }
        }
        let c = -(0..n).map(|i| next[i][i]).sum::<f64>() / k as f64;
        finite("State-space characteristic coefficient", c)?;
        coefficients.push(c);
        for (i, row) in next.iter_mut().enumerate() {
            row[i] += c;
        }
        previous = next;
    }
    let norm = a
        .iter()
        .map(|r| r.iter().map(|x| x.abs()).sum::<f64>())
        .fold(0.0, f64::max);
    let bound = coefficients
        .iter()
        .enumerate()
        .map(|(i, c)| c.abs() * norm.powi((n - i) as i32))
        .sum::<f64>()
        .max(f64::MIN_POSITIVE);
    let residual = previous
        .iter()
        .map(|r| r.iter().map(|x| x.abs()).sum::<f64>())
        .fold(0.0, f64::max)
        / bound;
    if !residual.is_finite() || residual > 1e-8 {
        return Err("State-space Cayley-Hamilton residual exceeds tolerance".into());
    }
    Ok((coefficients, residual))
}
#[derive(Clone, Copy, Default)]
struct C {
    r: f64,
    i: f64,
}
impl C {
    fn abs(self) -> f64 {
        self.r.hypot(self.i)
    }
    fn sub(self, b: Self) -> Self {
        Self {
            r: self.r - b.r,
            i: self.i - b.i,
        }
    }
    fn mul(self, b: Self) -> Self {
        Self {
            r: self.r * b.r - self.i * b.i,
            i: self.r * b.i + self.i * b.r,
        }
    }
    fn div(self, b: Self) -> Self {
        let s = b.r.abs().max(b.i.abs());
        let r = b.r / s;
        let i = b.i / s;
        let d = r * r + i * i;
        Self {
            r: (self.r / s * r + self.i / s * i) / d,
            i: (self.i / s * r - self.r / s * i) / d,
        }
    }
}
fn matrix_residual(a: &[Vec<f64>], r: f64, im: f64) -> Result<f64> {
    let n = a.len();
    let mut m = vec![vec![C::default(); n]; n];
    for i in 0..n {
        for j in 0..n {
            m[i][j] = C {
                r: if i == j { r - a[i][j] } else { -a[i][j] },
                i: if i == j { im } else { 0.0 },
            };
        }
        // Use the uncancelled row bound: normalizing by a tiny (lambda-Aii) would erase the root residual.
        let scale = r.hypot(im) + a[i].iter().map(|x| x.abs()).sum::<f64>();
        if scale == 0.0 {
            return Ok(0.0);
        }
        for z in &mut m[i] {
            z.r /= scale;
            z.i /= scale;
        }
    }
    let mut determinant = C { r: 1.0, i: 0.0 };
    for k in 0..n {
        let pivot = (k..n)
            .max_by(|i, j| m[*i][k].abs().total_cmp(&m[*j][k].abs()))
            .unwrap();
        if m[pivot][k].abs() < 1e-14 {
            return Ok(0.0);
        }
        if k != pivot {
            m.swap(k, pivot);
            determinant.r = -determinant.r;
            determinant.i = -determinant.i;
        }
        determinant = determinant.mul(m[k][k]);
        for i in k + 1..n {
            let factor = m[i][k].div(m[k][k]);
            for j in k + 1..n {
                m[i][j] = m[i][j].sub(factor.mul(m[k][j]));
            }
            m[i][k] = C::default();
        }
    }
    let residual = determinant.abs();
    finite("State-space determinant residual", residual)?;
    Ok(residual)
}
pub(super) fn analyze(a: &[Vec<f64>]) -> Spectrum {
    let n = a.len();
    let scale = a.iter().flatten().map(|x| x.abs()).fold(0.0, f64::max);
    let scale = if scale == 0.0 { 1.0 } else { scale };
    let mut normalized = a.to_vec();
    for row in &mut normalized {
        for value in row {
            let old = *value;
            *value /= scale;
            if old != 0.0 && *value == 0.0 {
                return unresolved(
                    n,
                    scale,
                    "State-space matrix scaling underflow; eigenvalues and stability unresolved"
                        .into(),
                );
            }
        }
    }
    let (coefficients, ch) = match characteristic(&normalized) {
        Ok(x) => x,
        Err(e) => return unresolved(n, scale, e),
    };
    let mut poles = pole_zero::roots::solve(&coefficients);
    let mut stability = pole_zero::routh(&coefficients);
    let mut residuals = vec![];
    if poles.status == "verified_residual" {
        for p in &poles.roots {
            match matrix_residual(&normalized, p.real, p.imag) {
                Ok(r) if r <= 1e-8 => residuals.push(r),
                _ => {
                    return unresolved(
                        n,
                        scale,
                        "Eigenvalue candidates fail the independent matrix determinant check"
                            .into(),
                    )
                }
            }
        }
        for p in &mut poles.roots {
            let old_real = p.real;
            let old_imag = p.imag;
            p.real *= scale;
            p.imag *= scale;
            if !p.real.is_finite()
                || !p.imag.is_finite()
                || (old_real != 0.0 && p.real == 0.0)
                || (old_imag != 0.0 && p.imag == 0.0)
            {
                poles.status = "unresolved".into();
                poles.roots.clear();
                poles.reconstruction_error = None;
                poles.warnings.push(
                    "Physical eigenvalue component overflows or underflows the finite range; no partial eigenvalues returned"
                        .into(),
                );
                residuals.clear();
                break;
            }
        }
    } else {
        stability.notes.push("Eigenvalue coordinates remain unresolved; Routh classification is evaluated independently on the characteristic polynomial".into());
    }
    (
        Some(coefficients),
        scale,
        Some(ch),
        poles,
        residuals,
        stability,
    )
}
