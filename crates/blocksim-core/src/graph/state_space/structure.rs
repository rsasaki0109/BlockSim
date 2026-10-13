//! Scaled single-source/single-Scope Krylov diagnostics; never remove states.
use super::*;
const TOLERANCE: f64 = 1e-10;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericalRank {
    pub status: String,
    pub numerical_rank: Option<usize>,
    pub dimension: usize,
    pub tolerance: f64,
    pub pivot_norms: Vec<f64>,
    pub notes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSpaceStructure {
    pub controllability: NumericalRank,
    pub observability: NumericalRank,
    pub method: String,
    pub notes: Vec<String>,
}
fn unknown(n: usize, reason: String) -> NumericalRank {
    NumericalRank {
        status: "unresolved".into(),
        numerical_rank: None,
        dimension: n,
        tolerance: TOLERANCE,
        pivot_norms: vec![],
        notes: vec![reason],
    }
}
fn normalized(values: &[f64]) -> Result<Vec<f64>> {
    let scale = values.iter().map(|v| v.abs()).fold(0., f64::max);
    if scale == 0. {
        return Ok(values.to_vec());
    }
    let mut out = vec![];
    for v in values {
        let x = v / scale;
        if !x.is_finite() || (*v != 0. && x == 0.) {
            return Err("Krylov vector scaling loses finite resolution".into());
        }
        out.push(x);
    }
    let norm = out.iter().map(|v| v * v).sum::<f64>().sqrt();
    for v in &mut out {
        *v /= norm;
    }
    Ok(out)
}
fn krylov(a: &[Vec<f64>], v: &[f64]) -> Result<Vec<Vec<f64>>> {
    let n = v.len();
    let mut x = normalized(v)?;
    let mut columns = vec![];
    for k in 0..n {
        columns.push(x.clone());
        if k + 1 == n {
            break;
        }
        let mut next = vec![];
        for row in a {
            let mut sum = 0.;
            for (a, b) in row.iter().zip(&x) {
                let term = a * b;
                if *a != 0. && *b != 0. && term == 0. {
                    return Err(
                        "Krylov multiplication underflows; numerical rank unresolved".into(),
                    );
                }
                sum += term;
            }
            finite("Krylov product", sum)?;
            next.push(sum);
        }
        x = normalized(&next)?;
    }
    Ok(columns)
}
fn rank(columns: Vec<Vec<f64>>) -> NumericalRank {
    let n = columns.len();
    let mut remaining = columns;
    let mut basis: Vec<Vec<f64>> = vec![];
    let mut pivots = vec![];
    let mut near = false;
    while !remaining.is_empty() {
        let mut best = (0, vec![], 0.);
        for (i, column) in remaining.iter().enumerate() {
            let mut v = column.clone();
            for _ in 0..2 {
                for q in &basis {
                    let projection = v.iter().zip(q).map(|(a, b)| a * b).sum::<f64>();
                    for (a, b) in v.iter_mut().zip(q) {
                        *a -= projection * b;
                    }
                }
            }
            let norm = v.iter().map(|v| v * v).sum::<f64>().sqrt();
            if !norm.is_finite() {
                return unknown(n, "Orthogonalization became nonfinite".into());
            }
            if i == 0 || norm > best.2 {
                best = (i, v, norm);
            }
        }
        let (index, mut v, norm) = best;
        pivots.push(norm);
        if (TOLERANCE / 100. ..=TOLERANCE * 100.).contains(&norm) {
            near = true;
        }
        if norm <= TOLERANCE {
            break;
        }
        for x in &mut v {
            *x /= norm;
        }
        basis.push(v);
        remaining.swap_remove(index);
    }
    let count = basis.len();
    NumericalRank {status:if n==0 {"no_states"}else if near {"near_threshold"}else if count==n {"full_numerical_rank"}else{"numerical_rank_deficient"}.into(),numerical_rank:Some(count),dimension:n,tolerance:TOLERANCE,pivot_norms:pivots,notes:vec!["Rank uses separately normalized Krylov columns; gain magnitude and energy are not assessed".into(),"This is a finite-precision diagnostic, not an exact controllability/observability or minimal-realization proof".into()]}
}
pub(super) fn analyze(a: &[Vec<f64>], b: &[f64], c: &[f64]) -> StateSpaceStructure {
    let n = a.len();
    let compute = || -> Result<(NumericalRank, NumericalRank)> {
        if n > 16 || a.iter().any(|r| r.len() != n) || b.len() != n || c.len() != n {
            return Err("Structural matrix dimensions are invalid".into());
        }
        if a.iter().flatten().chain(b).chain(c).any(|v| !v.is_finite()) {
            return Err("Structural matrices must be finite".into());
        }
        let scale = a.iter().flatten().map(|v| v.abs()).fold(0., f64::max);
        let scale = if scale == 0. { 1. } else { scale };
        let mut scaled = a.to_vec();
        for row in &mut scaled {
            for v in row {
                let old = *v;
                *v /= scale;
                if old != 0. && *v == 0. {
                    return Err(
                        "Structural matrix scaling underflows; no numerical rank claim".into(),
                    );
                }
            }
        }
        let transpose: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..n).map(|j| scaled[j][i]).collect())
            .collect();
        let control = match krylov(&scaled, b) {
            Ok(columns) => rank(columns),
            Err(e) => unknown(n, e),
        };
        let observation = match krylov(&transpose, c) {
            Ok(columns) => rank(columns),
            Err(e) => unknown(n, e),
        };
        Ok((control, observation))
    };
    let (controllability, observability) =
        compute().unwrap_or_else(|e| (unknown(n, e.clone()), unknown(n, e)));
    StateSpaceStructure{controllability,observability,method:"column-normalized Krylov matrices / column-pivoted twice-reorthogonalized numerical rank".into(),notes:vec!["Selected source and Scope components only; all ancestor states are retained".into(),"Near-threshold pivots are reported as uncertain; no pole cancellation or state deletion".into()]}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scalar_rescaling_preserves_full_rank_without_assessing_absolute_gain() {
        for scale in [1e-308, 1., 1e308] {
            let r = analyze(
                &[vec![0., 0.], vec![scale, 0.]],
                &[1e-100, 0.],
                &[0., 1e100],
            );
            assert_eq!(r.controllability.status, "full_numerical_rank");
            assert_eq!(r.observability.status, "full_numerical_rank");
            assert_eq!(r.controllability.numerical_rank, Some(2));
        }
    }
    #[test]
    fn canonical_sixteen_state_chain_is_full_rank_and_zero_inputs_remain_zero_rank() {
        let mut a = vec![vec![0.; 16]; 16];
        for (i, row) in a.iter_mut().enumerate().skip(1) {
            row[i - 1] = 1.;
        }
        let mut b = vec![0.; 16];
        b[0] = 1.;
        let mut c = vec![0.; 16];
        c[15] = 1.;
        let r = analyze(&a, &b, &c);
        assert_eq!(r.controllability.numerical_rank, Some(16));
        assert_eq!(r.observability.numerical_rank, Some(16));
        assert_eq!(r.controllability.status, "full_numerical_rank");
        assert_eq!(r.controllability.pivot_norms, vec![1.; 16]);
        assert_eq!(
            analyze(&a, &[0.; 16], &c).controllability.numerical_rank,
            Some(0)
        );
    }
    #[test]
    fn dependent_and_near_threshold_directions_are_not_declared_full_rank() {
        let b = [1., 1.];
        let c = [1., 1.];
        let exact = analyze(&[vec![1., 0.], vec![0., 1.]], &b, &c);
        assert_eq!(exact.controllability.numerical_rank, Some(1));
        assert_eq!(exact.controllability.status, "numerical_rank_deficient");
        let near = analyze(&[vec![1., 0.], vec![0., 1. + 1e-9]], &b, &c);
        assert_eq!(near.controllability.status, "near_threshold");
        let full = analyze(&[vec![1., 0.], vec![0., 2.]], &b, &c);
        assert_eq!(full.controllability.status, "full_numerical_rank");
    }
    #[test]
    fn invalid_shapes_and_scaling_loss_return_unresolved_without_partial_claims() {
        for a in [
            vec![vec![0.]],
            vec![vec![1e308, 0.], vec![0., 1e-308]],
            vec![vec![f64::NAN, 0.], vec![0., 1.]],
        ] {
            let r = analyze(&a, &[1., 1.], &[1., 1.]);
            assert_eq!(r.controllability.status, "unresolved");
            assert_eq!(r.controllability.numerical_rank, None);
        }
        assert_eq!(analyze(&[], &[], &[]).controllability.status, "no_states");
    }
}
