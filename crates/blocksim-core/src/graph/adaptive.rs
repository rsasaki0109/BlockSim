//! Dormand–Prince 5(4), quartic dense output and exact hybrid event boundaries.
use super::*;
type State = Vec<Vec<f64>>;
const A: [&[f64]; 6] = [
    &[1.0 / 5.0],
    &[3.0 / 40.0, 9.0 / 40.0],
    &[44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0],
    &[
        19372.0 / 6561.0,
        -25360.0 / 2187.0,
        64448.0 / 6561.0,
        -212.0 / 729.0,
    ],
    &[
        9017.0 / 3168.0,
        -355.0 / 33.0,
        46732.0 / 5247.0,
        49.0 / 176.0,
        -5103.0 / 18656.0,
    ],
    &[
        35.0 / 384.0,
        0.0,
        500.0 / 1113.0,
        125.0 / 192.0,
        -2187.0 / 6784.0,
        11.0 / 84.0,
    ],
];
const C: [f64; 6] = [1.0 / 5.0, 3.0 / 10.0, 4.0 / 5.0, 8.0 / 9.0, 1.0, 1.0];
const LOW: [f64; 7] = [
    5179.0 / 57600.0,
    0.0,
    7571.0 / 16695.0,
    393.0 / 640.0,
    -92097.0 / 339200.0,
    187.0 / 2100.0,
    1.0 / 40.0,
];
const P: [[f64; 4]; 7] = [
    [
        1.0,
        -8048581381.0 / 2820520608.0,
        8663915743.0 / 2820520608.0,
        -12715105075.0 / 11282082432.0,
    ],
    [0.0, 0.0, 0.0, 0.0],
    [
        0.0,
        131558114200.0 / 32700410799.0,
        -68118460800.0 / 10900136933.0,
        87487479700.0 / 32700410799.0,
    ],
    [
        0.0,
        -1754552775.0 / 470086768.0,
        14199869525.0 / 1410260304.0,
        -10690763975.0 / 1880347072.0,
    ],
    [
        0.0,
        127303824393.0 / 49829197408.0,
        -318862633887.0 / 49829197408.0,
        701980252875.0 / 199316789632.0,
    ],
    [
        0.0,
        -282668133.0 / 205662961.0,
        2019193451.0 / 616988883.0,
        -1453857185.0 / 822651844.0,
    ],
    [
        0.0,
        40617522.0 / 29380423.0,
        -110615467.0 / 29380423.0,
        69997945.0 / 29380423.0,
    ],
];
fn mix(base: &State, k: &[State], h: f64, weights: &[f64]) -> State {
    base.iter()
        .enumerate()
        .map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(|(j, value)| {
                    value
                        + k.iter()
                            .zip(weights)
                            .map(|(rate, w)| rate[i][j] * (h * w))
                            .sum::<f64>()
                })
                .collect()
        })
        .collect()
}
fn budget(stats: &SolverStatistics, work: usize, limit: usize) -> Result<()> {
    if (stats.rate_evaluations + stats.output_evaluations) * work > limit {
        Err(format!("RK45 exceeds {limit} block-component evaluations; relax tolerances or reduce model size"))
    } else {
        Ok(())
    }
}
fn trial(
    graph: &Compiled,
    rt: &mut Runtime,
    time: f64,
    end: f64,
    dt: f64,
    stats: &mut SolverStatistics,
    limit: usize,
) -> Result<(State, Vec<State>)> {
    let base = rt.state.clone();
    let h = end - time;
    let mut k = vec![];
    for stage in 0..7 {
        if stage > 0 {
            rt.state = mix(&base, &k, h, A[stage - 1]);
        }
        stats.rate_evaluations += 1;
        budget(
            stats,
            graph.flat.blocks.iter().map(|b| b.width).sum(),
            limit,
        )?;
        k.push(rates(
            graph,
            rt,
            if stage == 0 {
                time
            } else if stage >= 5 {
                end
            } else {
                time + C[stage - 1] * h
            },
            dt,
            stage >= 5,
        )?);
    }
    Ok((rt.state.clone(), k))
}
fn record(
    graph: &Compiled,
    rt: &mut Runtime,
    tick: usize,
    dt: f64,
    result: &mut GraphResult,
    limit: usize,
) -> Result<()> {
    sampled::commit(graph, rt, tick, dt)?;
    let time = tick as f64 * dt;
    let stats = result.statistics.as_mut().unwrap();
    stats.output_evaluations += 1;
    budget(
        stats,
        graph.flat.blocks.iter().map(|b| b.width).sum(),
        limit,
    )?;
    let values = evaluate(graph, rt, time, Some(tick), dt, false)?;
    sampled::capture(graph, rt, &values, tick, dt)?;
    result.times.push(time);
    for (trace, i) in result.traces.iter_mut().zip(&graph.scopes) {
        trace.values.push(values[*i].clone());
    }
    Ok(())
}
pub(super) fn simulate(
    graph: &Compiled,
    model: &GraphModel,
    config: &SolverConfig,
    limit: usize,
) -> Result<GraphResult> {
    let SolverConfig::Rk45 {
        abs_tolerance,
        rel_tolerance,
        min_step,
        max_step,
    } = *config
    else {
        unreachable!()
    };
    let mut rt = runtime(graph)?;
    let steps = (model.duration / model.dt).round() as usize;
    let end = steps as f64 * model.dt;
    let mut result = GraphResult {
        times: vec![],
        traces: graph
            .scopes
            .iter()
            .map(|i| Trace {
                id: graph.flat.blocks[*i].id.clone(),
                label: graph.flat.blocks[*i].label.clone(),
                values: vec![],
            })
            .collect(),
        solver: "adaptive Dormand-Prince RK45 / quartic dense output / synchronous sample-and-hold"
            .into(),
        statistics: Some(SolverStatistics {
            accepted_steps: 0,
            rejected_steps: 0,
            rate_evaluations: 0,
            output_evaluations: 0,
            min_step: 0.0,
            max_step: 0.0,
            max_error_ratio: 0.0,
        }),
    };
    let mut events = vec![end];
    let mut periods = BTreeSet::new();
    for b in &graph.flat.blocks {
        match &b.params {
            BlockKind::Step { step_time, .. } if *step_time > 0.0 && *step_time < end => {
                events.push(*step_time)
            }
            BlockKind::Delay { sample_time, .. }
            | BlockKind::DiscreteTransferFunction { sample_time, .. }
            | BlockKind::Pid {
                mode: PidMode::Discrete,
                sample_time,
                ..
            } => {
                periods.insert(period(*sample_time, model.dt)?);
            }
            _ => {}
        }
    }
    for p in periods {
        for tick in (p..=steps).step_by(p) {
            events.push(tick as f64 * model.dt);
        }
    }
    events.sort_by(f64::total_cmp);
    events.dedup();
    record(graph, &mut rt, 0, model.dt, &mut result, limit)?;
    let has_state = rt.state.iter().any(|row| !row.is_empty());
    let mut time = 0.0;
    let mut next_tick = 1;
    let mut proposed = max_step.min(model.dt).max(min_step);
    for boundary in events {
        while time < boundary {
            let h = if has_state {
                proposed.min(boundary - time)
            } else {
                boundary - time
            };
            let next = if h == boundary - time {
                boundary
            } else {
                time + h
            };
            if next <= time {
                return Err("RK45 step cannot advance time at floating-point resolution".into());
            }
            let h = next - time;
            let base = rt.state.clone();
            let mut dense = None;
            let mut ratio: f64 = 0.0;
            if has_state {
                let stats = result.statistics.as_mut().unwrap();
                if stats.accepted_steps + stats.rejected_steps >= 100000 {
                    return Err("RK45 exceeds 100000 attempted integration steps".into());
                }
                let attempt = trial(graph, &mut rt, time, next, model.dt, stats, limit);
                match attempt {
                    Ok((high, k)) => {
                        let low = mix(&base, &k, h, &LOW);
                        for (i, row) in high.iter().enumerate() {
                            for (j, value) in row.iter().enumerate() {
                                finite("RK45 integrated state", *value)?;
                                let scale = base[i][j].abs().max(value.abs()).max(abs_tolerance);
                                let error = ((value - low[i][j]) / scale).abs()
                                    / (abs_tolerance / scale
                                        + rel_tolerance
                                            * (base[i][j].abs().max(value.abs()) / scale));
                                ratio = if error.is_finite() {
                                    ratio.max(error)
                                } else {
                                    f64::INFINITY
                                };
                            }
                        }
                        if ratio.is_finite() && ratio <= 1.0 {
                            dense = Some(k);
                        } else {
                            rt.state = base.clone();
                        }
                    }
                    Err(error) => {
                        rt.state = base.clone();
                        if error.contains("block-component evaluations") || h <= min_step {
                            return Err(error);
                        }
                        ratio = f64::INFINITY;
                    }
                }
                if dense.is_none() {
                    stats.rejected_steps += 1;
                    if h <= min_step {
                        return Err("RK45 cannot meet tolerance at min_step; relax tolerance or lower min_step".into());
                    }
                    proposed = (h * if ratio.is_finite() {
                        (0.9 * ratio.powf(-0.2)).clamp(0.1, 0.5)
                    } else {
                        0.1
                    })
                    .max(min_step);
                    continue;
                }
                stats.accepted_steps += 1;
                stats.min_step = if stats.accepted_steps == 1 {
                    h
                } else {
                    stats.min_step.min(h)
                };
                stats.max_step = stats.max_step.max(h);
                stats.max_error_ratio = stats.max_error_ratio.max(ratio);
                proposed = (h * if ratio == 0.0 {
                    5.0
                } else {
                    (0.9 * ratio.powf(-0.2)).clamp(0.2, 5.0)
                })
                .clamp(min_step, max_step);
            }
            let high = rt.state.clone();
            while next_tick <= steps && next_tick as f64 * model.dt <= next {
                let sample = next_tick as f64 * model.dt;
                if sample != next {
                    if let Some(k) = &dense {
                        let x = (sample - time) / h;
                        let weights: Vec<_> = P
                            .iter()
                            .map(|p| x * (p[0] + x * (p[1] + x * (p[2] + x * p[3]))))
                            .collect();
                        rt.state = mix(&base, k, h, &weights);
                        for row in &rt.state {
                            for value in row {
                                finite("RK45 dense state", *value)?;
                            }
                        }
                    }
                }
                record(graph, &mut rt, next_tick, model.dt, &mut result, limit)?;
                rt.state = high.clone();
                next_tick += 1;
            }
            time = next;
        }
    }
    Ok(result)
}
