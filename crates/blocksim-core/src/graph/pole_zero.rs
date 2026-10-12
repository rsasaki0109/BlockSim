//! Selected continuous transfer realization, without pole/zero cancellation.
//! This does not determine the stability of an arbitrary connected graph.
use super::*;
mod roots;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisMode {
    Block,
    UnitNegativeFeedback,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoleZeroRequest {
    pub block: String,
    pub mode: AnalysisMode,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoleZeroCheck {
    pub label: String,
    pub width: usize,
    pub order: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Root {
    pub real: f64,
    pub imag: f64,
    pub relative_residual: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolynomialRoots {
    pub status: String,
    pub degree: Option<usize>,
    pub roots: Vec<Root>,
    pub reconstruction_error: Option<f64>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stability {
    pub classification: String,
    pub right_half_plane_poles: Option<usize>,
    pub tolerance: f64,
    pub method: String,
    pub notes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoleZeroResult {
    pub request: PoleZeroRequest,
    pub check: PoleZeroCheck,
    pub numerator: Vec<f64>,
    pub denominator: Vec<f64>,
    pub poles: PolynomialRoots,
    pub zeros: PolynomialRoots,
    pub stability: Stability,
    pub method: String,
}
type Prepared = (Vec<f64>, Vec<f64>, PoleZeroCheck);
fn prepare(model: &GraphModel, q: &PoleZeroRequest) -> Result<Prepared> {
    let graph = compile(model)?;
    let b = graph
        .flat
        .blocks
        .iter()
        .find(|b| b.id == q.block)
        .ok_or("Pole/zero block does not exist")?;
    let BlockKind::TransferFunction {
        numerator,
        denominator,
        ..
    } = &b.params
    else {
        return Err("Pole/zero analysis requires a continuous TransferFunction block".into());
    };
    if q.mode == AnalysisMode::Block {
        return Ok((
            numerator.clone(),
            denominator.clone(),
            PoleZeroCheck {
                label: b.label.clone(),
                width: b.width,
                order: denominator.len() - 1,
            },
        ));
    }
    let scale = numerator
        .iter()
        .chain(denominator)
        .map(|x| x.abs())
        .fold(0.0, f64::max);
    let normalize = |xs: &[f64]| -> Result<Vec<f64>> {
        xs.iter()
            .map(|x| {
                let v = x / scale;
                if *x != 0.0 && v == 0.0 {
                    Err("Polynomial normalization underflow; coefficient range is too wide".into())
                } else {
                    Ok(v)
                }
            })
            .collect()
    };
    let n = normalize(numerator)?;
    let mut d = normalize(denominator)?;
    if q.mode == AnalysisMode::UnitNegativeFeedback {
        let direct = n
            .first()
            .copied()
            .filter(|_| n.len() == d.len())
            .unwrap_or(0.0);
        if (d[0] + direct).abs() <= 1e-12 * (d[0].abs() + direct.abs()) {
            return Err("Unit negative feedback is singular or ill-conditioned at direct gain -1; D+N leading term cannot define a causal realization".into());
        }
        let offset = d.len() - n.len();
        for (i, v) in n.iter().enumerate() {
            d[offset + i] += v;
        }
    }
    let check = PoleZeroCheck {
        label: b.label.clone(),
        width: b.width,
        order: d.len() - 1,
    };
    Ok((n, d, check))
}
pub fn validate_pole_zero(model: &GraphModel, q: &PoleZeroRequest) -> Result<PoleZeroCheck> {
    prepare(model, q).map(|(_, _, c)| c)
}
pub fn analyze_pole_zero(model: &GraphModel, q: &PoleZeroRequest) -> Result<PoleZeroResult> {
    let (numerator, denominator, check) = prepare(model, q)?;
    let poles = roots::solve(&denominator);
    let zeros = roots::solve(&numerator);
    let stability = routh(&denominator);
    Ok(PoleZeroResult{request:q.clone(),check,numerator,denominator,poles,zeros,stability,method:"selected continuous companion realization; no cancellation; unit feedback uses D+N; polynomial roots verified by residual and reconstruction; independent scaled Routh table".into()})
}
fn routh(coefficients: &[f64]) -> Stability {
    const EPS: f64 = 1e-10;
    let mut out=Stability{classification:"indeterminate".into(),right_half_plane_poles:None,tolerance:EPS,method:"scaled real Routh table; near-zero pivots unresolved; auxiliary derivative for zero rows".into(),notes:vec![]};
    let n = coefficients.len() - 1;
    if n == 0 {
        out.classification = "stable".into();
        out.right_half_plane_poles = Some(0);
        out.notes.push("Order zero: no dynamic poles".into());
        return out;
    }
    let columns = (n + 2) / 2;
    let mut table = vec![vec![0.0; columns]; n + 1];
    let coefficient_scale = coefficients.iter().map(|x| x.abs()).fold(0.0, f64::max);
    for (i, v) in coefficients.iter().enumerate() {
        table[i % 2][i / 2] = *v / coefficient_scale * coefficients[0].signum();
    }
    let mut boundary = false;
    let mut previous_sign = 1.0_f64;
    let mut changes = 0;
    for i in 0..=n {
        if i >= 2 {
            for j in 0..columns - 1 {
                table[i][j] =
                    table[i - 2][j + 1] - table[i - 2][0] * table[i - 1][j + 1] / table[i - 1][0];
            }
        }
        let scale = table[i].iter().map(|x| x.abs()).fold(0.0, f64::max);
        if !scale.is_finite() {
            out.notes.push("Routh table became nonfinite".into());
            return out;
        }
        if scale <= EPS {
            if i == 0 {
                out.notes
                    .push("Leading coefficient is below the numerical threshold".into());
                return out;
            }
            if scale > 0.0 {
                out.classification = "boundary_or_uncertain".into();
                out.notes.push(format!("Routh row {i} is small but not exactly zero; right-half-plane count is unresolved at this tolerance"));
                return out;
            }
            boundary = true;
            let power = n - i + 1;
            for j in 0..columns {
                table[i][j] = if 2 * j < power {
                    (power - 2 * j) as f64 * table[i - 1][j]
                } else {
                    0.0
                };
            }
            let scale = table[i].iter().map(|x| x.abs()).fold(0.0, f64::max);
            if scale == 0.0 {
                out.notes
                    .push("Auxiliary polynomial derivative is unresolved".into());
                return out;
            }
            for x in &mut table[i] {
                *x /= scale;
            }
        } else {
            for x in &mut table[i] {
                *x /= scale;
            }
        }
        if table[i][0].abs() <= EPS {
            out.notes.push(format!("Routh row {i}: near-zero first-column pivot; no epsilon substitution or stability claim"));
            return out;
        }
        let sign = table[i][0].signum();
        if sign != previous_sign {
            changes += 1;
        }
        previous_sign = sign;
    }
    out.right_half_plane_poles = Some(changes);
    out.classification = if changes > 0 {
        "unstable"
    } else if boundary {
        "boundary_or_uncertain"
    } else {
        "stable"
    }
    .into();
    if boundary {
        out.notes.push("Zero or near-zero row: symmetric/imaginary-axis roots or a nearby boundary; not classified as asymptotically stable".into());
    }
    out.notes.push("Numerical classification of this uncancelled single-channel realization, not the connected graph or nonlinear/discrete system".into());
    out
}
