//! Bounded small-polynomial solver. Residual verification is not a root-error bound.
use super::{PolynomialRoots, Root};
use std::ops::{Add, Mul, Sub};
#[derive(Clone, Copy, Default)]
struct C {
    r: f64,
    i: f64,
}
impl C {
    fn real(r: f64) -> Self {
        Self { r, i: 0.0 }
    }
    fn abs(self) -> f64 {
        self.r.hypot(self.i)
    }
    fn div(self, b: Self) -> Self {
        let scale = b.r.abs().max(b.i.abs());
        let r = b.r / scale;
        let i = b.i / scale;
        let d = r * r + i * i;
        Self {
            r: (self.r / scale * r + self.i / scale * i) / d,
            i: (self.i / scale * r - self.r / scale * i) / d,
        }
    }
    fn finite(self) -> bool {
        self.r.is_finite() && self.i.is_finite()
    }
}
impl Add for C {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self {
            r: self.r + b.r,
            i: self.i + b.i,
        }
    }
}
impl Sub for C {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self {
            r: self.r - b.r,
            i: self.i - b.i,
        }
    }
}
impl Mul for C {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Self {
            r: self.r * b.r - self.i * b.i,
            i: self.r * b.i + self.i * b.r,
        }
    }
}
fn evaluate(a: &[f64], z: C) -> (C, C, f64) {
    let mut p = C::real(a[0]);
    let mut d = C::default();
    let mut bound = a[0].abs();
    for v in &a[1..] {
        d = d * z + p;
        p = p * z + C::real(*v);
        bound = bound * z.abs() + v.abs();
    }
    (p, d, bound)
}
fn failure(degree: Option<usize>, warning: &str) -> PolynomialRoots {
    PolynomialRoots {
        status: "unresolved".into(),
        degree,
        roots: vec![],
        reconstruction_error: None,
        warnings: vec![warning.into()],
    }
}
pub(crate) fn solve(coefficients: &[f64]) -> PolynomialRoots {
    let Some(first) = coefficients.iter().position(|x| *x != 0.0) else {
        return PolynomialRoots {
            status: "identically_zero".into(),
            degree: None,
            roots: vec![],
            reconstruction_error: None,
            warnings: vec!["Zero polynomial: zeros are not a finite isolated set".into()],
        };
    };
    let a = &coefficients[first..];
    let degree = a.len() - 1;
    let trailing = a.iter().rev().take_while(|x| **x == 0.0).count();
    let a = &a[..a.len() - trailing];
    let n = a.len() - 1;
    // Exact floating-point binomial coefficients admit a repeated-real-root shortcut.
    // No fuzzy GCD, approximate cancellation or assumed multiplicity is used.
    if n > 2 {
        let center = -a[1] / n as f64 / a[0];
        let mut binomial = 1_usize;
        let matched = center.is_finite()
            && center.abs() < 1e30
            && a.iter().enumerate().all(|(i, v)| {
                if i > 0 {
                    binomial = binomial * (n - i + 1) / i;
                }
                a[0] * binomial as f64 * (-center).powi(i as i32) == *v
            });
        if matched {
            let (p, _, bound) = evaluate(a, C::real(center));
            let residual = p.abs() / bound.max(f64::MIN_POSITIVE);
            if residual.is_finite() && residual <= 1e-9 {
                let mut roots: Vec<_> = (0..n)
                    .map(|_| Root {
                        real: center,
                        imag: 0.0,
                        relative_residual: residual,
                    })
                    .collect();
                roots.extend((0..trailing).map(|_| Root {
                    real: 0.0,
                    imag: 0.0,
                    relative_residual: 0.0,
                }));
                roots.sort_by(|a, b| a.real.total_cmp(&b.real));
                return PolynomialRoots{status:"verified_residual".into(),degree:Some(degree),roots,reconstruction_error:Some(0.0),warnings:vec!["Repeated real root recognized by exact binomial coefficient equality; coefficient perturbations can strongly change root coordinates".into()]};
            }
        }
    }
    let log_scale = if n == 0 {
        0.0
    } else {
        a.iter()
            .enumerate()
            .skip(1)
            .filter(|(_, x)| **x != 0.0)
            .map(|(j, x)| (x.abs().ln() - a[0].abs().ln()) / j as f64)
            .fold(f64::NEG_INFINITY, f64::max)
            + 2.0_f64.ln()
    };
    if !(-700.0..=700.0).contains(&log_scale) {
        return failure(
            Some(degree),
            "Polynomial root scale exceeds the supported floating-point range",
        );
    }
    let scale = log_scale.exp();
    let mut b = vec![1.0];
    for (j, x) in a.iter().enumerate().skip(1) {
        let v = if *x == 0.0 {
            0.0
        } else {
            (x.abs().ln() - a[0].abs().ln() - j as f64 * log_scale).exp()
                * x.signum()
                * a[0].signum()
        };
        if *x != 0.0 && v == 0.0 {
            return failure(
                Some(degree),
                "Root scaling would underflow a nonzero coefficient",
            );
        }
        b.push(v);
    }
    for attempt in 0..4 {
        let mut zs = match n {
            0 => vec![],
            1 => vec![C::real(-b[1])],
            2 => {
                let disc = b[1] * b[1] - 4.0 * b[2];
                if disc >= 0.0 {
                    let q = -0.5 * (b[1] + disc.sqrt().copysign(b[1]));
                    if q == 0.0 {
                        vec![C::default(); 2]
                    } else {
                        vec![C::real(q), C::real(b[2] / q)]
                    }
                } else {
                    vec![
                        C {
                            r: -b[1] / 2.0,
                            i: (-disc).sqrt() / 2.0,
                        },
                        C {
                            r: -b[1] / 2.0,
                            i: -(-disc).sqrt() / 2.0,
                        },
                    ]
                }
            }
            _ => (0..n)
                .map(|j| {
                    let t = std::f64::consts::TAU * (j as f64 + 0.35 + attempt as f64 * 0.17)
                        / n as f64;
                    let r = 0.75 + attempt as f64 * 0.15;
                    C {
                        r: r * t.cos(),
                        i: r * t.sin(),
                    }
                })
                .collect(),
        };
        if n > 2 {
            for iteration in 0..800 {
                let previous = zs.clone();
                let mut largest: f64 = 0.0;
                for j in 0..n {
                    let (p, d, _) = evaluate(&b, previous[j]);
                    let mut sum = C::default();
                    for k in 0..n {
                        if k != j {
                            sum = sum + C::real(1.0).div(previous[j] - previous[k]);
                        }
                    }
                    let mut delta = p.div(d - p * sum);
                    if !delta.finite() {
                        let angle = (iteration + j + attempt) as f64;
                        delta = C {
                            r: 1e-8 * angle.cos(),
                            i: 1e-8 * angle.sin(),
                        };
                    }
                    let cap = 0.5 * (1.0 + previous[j].abs());
                    if delta.abs() > cap {
                        delta = delta * C::real(cap / delta.abs());
                    }
                    largest = largest.max(delta.abs() / (1.0 + previous[j].abs()));
                    zs[j] = previous[j] - delta;
                }
                if largest < 1e-13 {
                    break;
                }
            }
        }
        if zs.iter().any(|z| !z.finite()) {
            continue;
        }
        for z in &mut zs {
            if z.i.abs() < 1e-12 * (1.0 + z.r.abs()) {
                z.i = 0.0;
            }
        }
        let mut reconstructed = vec![C::real(1.0)];
        for z in &zs {
            let mut next = vec![C::default(); reconstructed.len() + 1];
            for (j, v) in reconstructed.iter().enumerate() {
                next[j] = next[j] + *v;
                next[j + 1] = next[j + 1] - *v * *z;
            }
            reconstructed = next;
        }
        let reconstruction_error = reconstructed
            .iter()
            .zip(&b)
            .map(|(x, v)| {
                (*x - C::real(*v)).abs() / if *v == 0.0 { 1.0 } else { v.abs().max(1e-300) }
            })
            .fold(0.0, f64::max);
        if !reconstruction_error.is_finite() || reconstruction_error > 1e-7 {
            continue;
        }
        let mut roots = vec![];
        let mut good = true;
        for z in &zs {
            let (p, _, bound) = evaluate(&b, *z);
            let residual = p.abs() / bound.max(f64::MIN_POSITIVE);
            let root = Root {
                real: z.r * scale,
                imag: z.i * scale,
                relative_residual: residual,
            };
            if !root.real.is_finite()
                || !root.imag.is_finite()
                || !residual.is_finite()
                || residual > 1e-9
            {
                good = false;
                break;
            }
            roots.push(root);
        }
        if !good {
            continue;
        }
        roots.extend((0..trailing).map(|_| Root {
            real: 0.0,
            imag: 0.0,
            relative_residual: 0.0,
        }));
        roots.sort_by(|a, b| a.real.total_cmp(&b.real).then(a.imag.total_cmp(&b.imag)));
        let clustered = roots.iter().enumerate().any(|(j, a)| {
            roots[j + 1..].iter().any(|b| {
                (a.real - b.real).hypot(a.imag - b.imag) < 0.05 * (1.0 + a.real.hypot(a.imag))
            })
        });
        let warnings = if clustered {
            vec!["Repeated or clustered roots: small polynomial residual does not bound root-coordinate error".into()]
        } else {
            vec![]
        };
        return PolynomialRoots {
            status: "verified_residual".into(),
            degree: Some(degree),
            roots,
            reconstruction_error: Some(reconstruction_error),
            warnings,
        };
    }
    failure(Some(degree),"Polynomial roots did not pass residual/reconstruction checks within four attempts of at most 800 simultaneous updates")
}
