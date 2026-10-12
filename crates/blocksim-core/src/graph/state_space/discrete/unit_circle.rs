use super::*;
const TOLERANCE: f64 = 1e-10;
fn unknown(reason: &str) -> StateSpaceStability {
    StateSpaceStability {
        classification: "indeterminate".into(),
        right_half_plane_poles: None,
        outside_unit_circle_poles: None,
        tolerance: TOLERANCE,
        method: "unit-circle bilinear polynomial / Routh".into(),
        notes: vec![reason.into()],
    }
}
fn binomial(n: usize, k: usize) -> f64 {
    (0..k).fold(1.0, |v, i| v * (n - i) as f64 / (i + 1) as f64)
}
pub(super) fn classify(coefficients: Option<&[f64]>, scale: f64) -> StateSpaceStability {
    let Some(c) = coefficients else {
        return unknown(
            "Characteristic polynomial is unresolved; unit-circle stability is indeterminate",
        );
    };
    let n = c.len() - 1;
    // Recover a normalized physical p(z) without overflowing scale^i.
    let logs: Vec<_> = c
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if *v == 0.0 {
                f64::NEG_INFINITY
            } else {
                v.abs().ln() + i as f64 * scale.ln()
            }
        })
        .collect();
    let max = logs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut p = vec![0.0; c.len()];
    for i in 0..c.len() {
        if c[i] != 0.0 {
            if !logs[i].is_finite() || logs[i] - max < -700.0 {
                return unknown("Physical characteristic normalization exceeds finite resolution; no unit-circle classification");
            }
            p[i] = c[i].signum() * (logs[i] - max).exp();
        }
    }
    // q(s)=(1-s)^n p((1+s)/(1-s)); Re(s)>0 iff |z|>1.
    let mut q = vec![0.0; n + 1];
    let mut bounds = vec![0.0; n + 1];
    for (i, v) in p.iter().enumerate() {
        for plus in 0..=n - i {
            for minus in 0..=i {
                let term = v
                    * binomial(n - i, plus)
                    * binomial(i, minus)
                    * if minus % 2 == 0 { 1.0 } else { -1.0 };
                q[plus + minus] += term;
                bounds[plus + minus] += term.abs();
            }
        }
    }
    if q.iter().chain(&bounds).any(|v| !v.is_finite()) {
        return unknown("Bilinear polynomial overflow");
    }
    let mut boundary = false;
    while q.len() > 1 && q.last() == Some(&0.0) {
        q.pop();
        bounds.pop();
        boundary = true;
    }
    if q.iter()
        .zip(&bounds)
        .any(|(v, b)| *v != 0.0 && v.abs() <= TOLERANCE * b)
    {
        let mut r = unknown("Bilinear polynomial cancellation is near numerical resolution; boundary/nearby modes unresolved");
        r.classification = "boundary_or_uncertain".into();
        return r;
    }
    q.reverse();
    let r = pole_zero::routh(&q);
    let mut notes: Vec<_> = r
        .notes
        .into_iter()
        .filter(|s| !s.contains("single-channel realization"))
        .collect();
    notes.push("Unit-circle test uses z=(1+s)/(1-s), independently of approximate eigenvalue coordinates; no stability proof".into());
    if boundary {
        notes.push("Exact bilinear leading zero retains a z=-1 boundary mode; it is not removed as a stable mode".into());
    }
    StateSpaceStability {
        classification: if boundary && r.classification == "stable" {
            "boundary_or_uncertain".into()
        } else {
            r.classification
        },
        right_half_plane_poles: None,
        outside_unit_circle_poles: r.right_half_plane_poles,
        tolerance: TOLERANCE,
        method: "unit-circle bilinear polynomial / scaled Routh".into(),
        notes,
    }
}
