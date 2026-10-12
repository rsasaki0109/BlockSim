//! Single-clock sample-instant realization, sharing actual PID/Delay update semantics.
use super::*;
mod unit_circle;

pub(super) fn dependencies(graph: &Compiled, node: (usize, usize)) -> Result<Vec<(usize, usize)>> {
    let b = &graph.flat.blocks[node.0];
    match &b.params {
        BlockKind::Delay { .. } => Ok(vec![(graph.incoming[node.0][0], node.1)]),
        BlockKind::Pid { config, mode: PidMode::Discrete, .. }
            if config.limits.lower.is_none() && config.limits.upper.is_none() =>
                Ok(vec![(graph.incoming[node.0][0], node.1)]),
        BlockKind::Integrator { .. } | BlockKind::FirstOrder { .. } | BlockKind::Pid { .. } =>
            Err(format!("{}: discrete state-space requires unlimited discrete PID and a single clock; continuous states, bounded PID and mixed rates are unsupported", b.id)),
        BlockKind::TransferFunction { denominator, .. } if denominator.len() > 1 =>
            Err(format!("{}: continuous TransferFunction states are unsupported in discrete state-space", b.id)),
        _ => frequency::dependencies(graph, node).map_err(|e| e.replace("frequency analysis requires continuous linear blocks; bounded PID/Saturation, Product and sampled blocks are unsupported", "discrete state-space requires linear blocks; bounded PID/Saturation and multi-input Product are unsupported")),
    }
}
fn prepare_discrete(model: &GraphModel, q: &StateSpaceRequest) -> Result<(Prepared, f64)> {
    let p = prepare_with_domain(model, q, true)?;
    let mut clock = None;
    for b in &p.graph.flat.blocks {
        let t = match b.params {
            BlockKind::Delay { sample_time, .. } | BlockKind::Pid { sample_time, .. } => {
                sample_time
            }
            _ => continue,
        };
        // Match the actual validated integer recording-clock period, not a fuzzy float comparison.
        let ticks = period(t, model.dt)?;
        if clock.is_some_and(|(previous, _)| previous != ticks) {
            return Err(format!("{}: discrete state-space requires one common sample period; mixed rates are unsupported", b.id));
        }
        clock = Some((ticks, t));
    }
    // Even approximately equal sample_time values change PID arithmetic; require the same value.
    let sample_time = clock.map(|(_, t)| t).unwrap_or(model.dt);
    for b in &p.graph.flat.blocks {
        if let BlockKind::Delay { sample_time: t, .. } | BlockKind::Pid { sample_time: t, .. } =
            b.params
        {
            if t != sample_time {
                return Err(
                    "Discrete state-space requires exactly equal sample_time values".into(),
                );
            }
        }
    }
    Ok((p, sample_time))
}
pub fn validate_discrete_state_space(
    model: &GraphModel,
    q: &StateSpaceRequest,
) -> Result<StateSpaceCheck> {
    check_discrete(model, q).map(|(check, _)| check)
}
pub(in crate::graph) fn check_discrete(
    model: &GraphModel,
    q: &StateSpaceRequest,
) -> Result<(StateSpaceCheck, f64)> {
    prepare_discrete(model, q).map(|(p, t)| (p.check, t))
}
pub(in crate::graph) struct DiscreteRealization {
    pub check: StateSpaceCheck,
    pub states: Vec<StateInfo>,
    pub a: Vec<Vec<f64>>,
    pub b: Vec<f64>,
    pub c: Vec<f64>,
    pub d: f64,
    pub sample_time: f64,
}
pub(in crate::graph) fn discrete_realization(
    model: &GraphModel,
    q: &StateSpaceRequest,
) -> Result<DiscreteRealization> {
    let (mut p, sample_time) = prepare_discrete(model, q)?;
    let n = p.states.len();
    let base = runtime(&p.graph)?;
    let mut a = vec![vec![0.0; n]; n];
    let mut b = vec![0.0; n];
    let mut c = vec![0.0; n];
    let mut d = 0.0;
    for (column, output_coefficient) in c.iter_mut().chain(std::iter::once(&mut d)).enumerate() {
        if let Some(input) = p.input {
            if let BlockKind::Constant { value } = &mut p.graph.flat.blocks[input].params {
                value[0] = if column == n { 1.0 } else { 0.0 };
            }
        }
        let mut rt = base.clone();
        if column < n {
            let (i, state) = p.offsets[column];
            match p.graph.flat.blocks[i].params {
                BlockKind::Delay { .. } => rt.held[i][0] = 1.0,
                BlockKind::Pid { .. } => {
                    let mut values = [0.0; 3];
                    values[state] = 1.0;
                    rt.discrete[i][0].set_analysis_state(values);
                }
                _ => unreachable!(),
            }
        }
        // At a sample instant PID updates in topological order; Delay outputs its previous sample.
        let values = evaluate(&p.graph, &mut rt, 0.0, Some(0), model.dt, false)?;
        for (row, &(i, state)) in p.offsets.iter().enumerate() {
            // Capture ALL Delay inputs from the same evaluation: synchronous, never in-place updates.
            let next = match p.graph.flat.blocks[i].params {
                BlockKind::Delay { .. } => values[p.graph.incoming[i][0]][0],
                BlockKind::Pid { .. } => rt.discrete[i][0].analysis_state()[state],
                _ => unreachable!(),
            };
            finite("Discrete state-space next state", next)?;
            if column < n {
                a[row][column] = next;
            } else {
                b[row] = next;
            }
        }
        *output_coefficient = values[0][0];
    }
    Ok(DiscreteRealization {
        check: p.check,
        states: p.states,
        a,
        b,
        c,
        d,
        sample_time,
    })
}
pub fn analyze_discrete_state_space(
    model: &GraphModel,
    q: &StateSpaceRequest,
) -> Result<StateSpaceResult> {
    let DiscreteRealization {
        check,
        states,
        a,
        b,
        c,
        d,
        sample_time,
    } = discrete_realization(model, q)?;
    let (characteristic, pole_scale, characteristic_residual, poles, matrix_residuals, _) =
        spectrum::analyze(&a);
    let mut stability = unit_circle::classify(characteristic.as_deref(), pole_scale);
    stability.notes.push("Numerical internal stability of selected Scope ancestry at sample instants; unrelated branches excluded; no cancellation or state removal".into());
    if !check.source_related {
        stability.notes.push(
            "Selected source is outside Scope ancestry: B and D are zero; internal modes retained"
                .into(),
        );
    }
    Ok(StateSpaceResult { request: q.clone(), check, states, a, b, c, d,
        characteristic, pole_scale, characteristic_residual, poles, matrix_residuals, stability,
        sample_time: Some(sample_time),
        method: "discrete LTI Scope ancestry / actual synchronous sample-instant PID and Delay unit basis / x[k+1]=Ax[k]+Bu[k],y[k]=Cx[k]+Du[k] / unit-circle bilinear polynomial and Routh / no state removal".into() })
}
