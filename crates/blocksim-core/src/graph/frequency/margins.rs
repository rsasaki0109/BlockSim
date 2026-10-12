//! Finite-range margin candidates. Sampling is not a proof of stability or completeness.
use super::{FrequencyPoint, Result, C};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GainCrossing {
    pub omega: f64,
    pub phase_deg: f64,
    /// 180 + locally unwrapped phase. No automatic branch or critical-margin selection.
    pub phase_margin_deg: f64,
    pub relative_residual: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseCrossing {
    pub omega: f64,
    pub phase_level_deg: f64,
    pub magnitude_db: f64,
    pub gain_margin_db: f64,
    /// None if the multiplicative gain margin is outside the finite positive range.
    pub gain_margin: Option<f64>,
    pub relative_residual: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarginIssue {
    pub kind: String,
    pub min_omega: f64,
    pub max_omega: f64,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarginAnalysis {
    pub gain_crossings: Vec<GainCrossing>,
    pub phase_crossings: Vec<PhaseCrossing>,
    pub issues: Vec<MarginIssue>,
    pub additional_evaluations: usize,
    pub method: String,
}
#[derive(Clone, Copy)]
struct Sample {
    omega: f64,
    db: f64,
    phase: f64,
    residual: f64,
}
fn sample(p: &FrequencyPoint) -> Option<Sample> {
    if p.error.is_some() {
        return None;
    }
    Some(Sample {
        omega: p.omega,
        db: p.magnitude_db?,
        phase: p.phase_deg?,
        residual: p.relative_residual?,
    })
}
fn level(phase: f64) -> f64 {
    -180.0 + 360.0 * ((phase + 180.0) / 360.0).round()
}
fn gain(s: Sample) -> GainCrossing {
    GainCrossing {
        omega: s.omega,
        phase_deg: s.phase,
        phase_margin_deg: 180.0 + s.phase,
        relative_residual: s.residual,
    }
}
fn phase(s: Sample, target: f64) -> PhaseCrossing {
    let ratio = 10.0_f64.powf(-s.db / 20.0);
    PhaseCrossing {
        omega: s.omega,
        phase_level_deg: target,
        magnitude_db: s.db,
        gain_margin_db: -s.db,
        gain_margin: (ratio.is_finite() && ratio > 0.0).then_some(ratio),
        relative_residual: s.residual,
    }
}
fn issue(out: &mut MarginAnalysis, kind: &str, lo: f64, hi: f64, message: &str) {
    // Compact consecutive gaps/plateaus without hiding any affected interval.
    if let Some(previous) = out
        .issues
        .iter_mut()
        .rev()
        .find(|i| i.kind == kind && i.message == message)
    {
        if previous.max_omega == lo {
            previous.max_omega = hi;
            return;
        }
    }
    out.issues.push(MarginIssue {
        kind: kind.into(),
        min_omega: lo,
        max_omega: hi,
        message: message.into(),
    });
}
fn refine<F>(
    mut a: Sample,
    mut b: Sample,
    target: Option<f64>,
    remaining: &mut usize,
    evaluate: &F,
) -> Result<Sample>
where
    F: Fn(f64) -> Result<(C, f64)>,
{
    let value = |s: Sample| target.map_or(s.db, |t| s.phase - t);
    for _ in 0..48 {
        if *remaining == 0 {
            return Err(
                "Margin refinement budget exhausted; increase sampling or simplify the model"
                    .into(),
            );
        }
        *remaining -= 1;
        let omega = ((a.omega.ln() + b.omega.ln()) / 2.0).exp();
        let (v, residual) = evaluate(omega)?;
        let magnitude = v.abs();
        if magnitude == 0.0 {
            return Err("Zero response inside crossing bracket; phase is undefined".into());
        }
        let raw = v.im.atan2(v.re).to_degrees();
        let anchor = (a.phase + b.phase) / 2.0;
        let s = Sample {
            omega,
            db: 20.0 * magnitude.log10(),
            phase: raw + 360.0 * ((anchor - raw) / 360.0).round(),
            residual,
        };
        if value(s).abs() < 1e-10 {
            return Ok(s);
        }
        if (b.omega.ln() - a.omega.ln()).abs() < 1e-12 {
            return Err("Crossing bracket did not converge to the target; possible singularity or discontinuity".into());
        }
        if value(a).is_sign_positive() == value(s).is_sign_positive() {
            a = s;
        } else {
            b = s;
        }
    }
    Err("Margin crossing refinement did not converge".into())
}
pub(super) fn analyze<F>(points: &[FrequencyPoint], budget: usize, evaluate: F) -> MarginAnalysis
where
    F: Fn(f64) -> Result<(C, f64)>,
{
    let mut out=MarginAnalysis {gain_crossings:vec![],phase_crossings:vec![],issues:vec![],additional_evaluations:0,method:"finite sampled brackets / log-frequency bisection with direct complex solves / PM=180+locally unwrapped phase / no stability inference".into()};
    let mut remaining = budget;
    let mut gain_plateaus = vec![];
    let mut phase_plateaus = vec![];
    for pair in points.windows(2) {
        let (Some(a), Some(b)) = (sample(&pair[0]), sample(&pair[1])) else {
            issue(&mut out,"gap",pair[0].omega,pair[1].omega,"Undefined phase, zero response or failed solve; no crossing is inferred across this interval");
            continue;
        };
        if (b.phase - a.phase).abs() >= 180.0 - 1e-8 {
            issue(
                &mut out,
                "phase_ambiguity",
                a.omega,
                b.omega,
                "Adjacent phase rotation is ambiguous; use a finer logarithmic grid",
            );
            continue;
        }
        if a.db.abs() <= 1e-10 && b.db.abs() <= 1e-10 {
            gain_plateaus.push((a.omega, b.omega));
            issue(
                &mut out,
                "gain_plateau",
                a.omega,
                b.omega,
                "Adjacent samples lie on 0 dB; no unique isolated gain crossing is reported",
            );
        } else if a.db.abs() > 1e-10
            && b.db.abs() > 1e-10
            && a.db.is_sign_positive() != b.db.is_sign_positive()
        {
            match refine(a, b, None, &mut remaining, &evaluate) {
                Ok(s) => out.gain_crossings.push(gain(s)),
                Err(e) => issue(&mut out, "gain_refinement", a.omega, b.omega, &e),
            }
        }
        // Adjacent unwrapped phases differ by <180°, so at most one odd-180° level is bracketed.
        let target = level((a.phase + b.phase) / 2.0);
        let va = a.phase - target;
        let vb = b.phase - target;
        if va.abs() <= 1e-8 && vb.abs() <= 1e-8 {
            phase_plateaus.push((a.omega, b.omega));
            issue(&mut out,"phase_plateau",a.omega,b.omega,"Adjacent samples lie on an odd-180-degree level; no unique isolated phase crossing is reported");
        } else if va.abs() > 1e-8
            && vb.abs() > 1e-8
            && va.is_sign_positive() != vb.is_sign_positive()
        {
            match refine(a, b, Some(target), &mut remaining, &evaluate) {
                Ok(s) => out.phase_crossings.push(phase(s, target)),
                Err(e) => issue(&mut out, "phase_refinement", a.omega, b.omega, &e),
            }
        }
    }
    // Exact grid hits, including range endpoints, are retained once. Plateau endpoints are excluded.
    for (index, point) in points.iter().enumerate() {
        let Some(s) = sample(point) else {
            continue;
        };
        // A lone valid sample cannot establish an isolated crossing beside a gap.
        let adjacent = [
            index.checked_sub(1),
            (index + 1 < points.len()).then_some(index + 1),
        ]
        .into_iter()
        .flatten()
        .filter_map(|i| sample(&points[i]))
        .any(|neighbor| (neighbor.phase - s.phase).abs() < 180.0 - 1e-8);
        if !adjacent {
            continue;
        }
        if s.db.abs() <= 1e-10
            && !gain_plateaus
                .iter()
                .any(|(a, b)| s.omega >= *a && s.omega <= *b)
        {
            out.gain_crossings.push(gain(s));
        }
        let target = level(s.phase);
        if (s.phase - target).abs() <= 1e-8
            && !phase_plateaus
                .iter()
                .any(|(a, b)| s.omega >= *a && s.omega <= *b)
        {
            out.phase_crossings.push(phase(s, target));
        }
    }
    out.gain_crossings
        .sort_by(|a, b| a.omega.total_cmp(&b.omega));
    out.gain_crossings
        .dedup_by(|a, b| (a.omega / b.omega - 1.0).abs() < 1e-9);
    out.phase_crossings
        .sort_by(|a, b| a.omega.total_cmp(&b.omega));
    out.phase_crossings
        .dedup_by(|a, b| (a.omega / b.omega - 1.0).abs() < 1e-9);
    out.additional_evaluations = budget - remaining;
    out
}
