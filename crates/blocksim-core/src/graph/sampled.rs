//! Synchronous sample-state commits and captures, shared by RK4 and RK45.
use super::*;
pub(super) fn commit(graph: &Compiled, rt: &mut Runtime, tick: usize, dt: f64) -> Result<()> {
    for (i, b) in graph.flat.blocks.iter().enumerate() {
        if let BlockKind::Delay { sample_time, .. }
        | BlockKind::DiscreteTransferFunction { sample_time, .. } = b.params
        {
            if tick.is_multiple_of(period(sample_time, dt)?) {
                if let Some(pending) = rt.pending[i].take() {
                    if matches!(b.params, BlockKind::Delay { .. }) {
                        rt.held[i] = pending;
                    } else {
                        rt.sampled_state[i] = pending;
                    }
                }
            }
        }
    }
    Ok(())
}
pub(super) fn next_transfer(
    graph: &Compiled,
    rt: &Runtime,
    values: &[Vec<f64>],
    i: usize,
) -> Result<Vec<f64>> {
    let next = rt.transfer[i]
        .as_ref()
        .unwrap()
        .rates(&rt.sampled_state[i], &values[graph.incoming[i][0]]);
    for v in &next {
        finite(
            &format!("{} next sample state", graph.flat.blocks[i].id),
            *v,
        )?;
    }
    Ok(next)
}
pub(super) fn capture(
    graph: &Compiled,
    rt: &mut Runtime,
    values: &[Vec<f64>],
    tick: usize,
    dt: f64,
) -> Result<()> {
    for (i, b) in graph.flat.blocks.iter().enumerate() {
        if let BlockKind::Delay { sample_time, .. }
        | BlockKind::DiscreteTransferFunction { sample_time, .. } = b.params
        {
            if tick.is_multiple_of(period(sample_time, dt)?) {
                rt.pending[i] = Some(if matches!(b.params, BlockKind::Delay { .. }) {
                    values[graph.incoming[i][0]].clone()
                } else {
                    next_transfer(graph, rt, values, i)?
                });
            }
        }
    }
    Ok(())
}
