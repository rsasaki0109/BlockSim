//! Independent pulse and direct-term equations for sample-aligned held inputs.
use blocksim_core::graph::{
    discretize::{discretize_tustin, discretize_zoh, TustinRequest, ZohRequest},
    simulate_graph_with_budget, BlockKind, GraphModel, SolverConfig,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases = vec![];
    for (name, ts, before, after, change, direct) in [
        ("lowpass_drop", 0.2, 1.0, 0.0, 10, false),
        ("rounded_direct", 0.1, 2.0, -0.5, 3, true),
        ("terminal_direct", 0.2, 0.0, 1.0, 100, true),
    ] {
        for method in ["zoh", "tustin"] {
            let denominator = if direct {
                vec![1.0, 1.0]
            } else {
                vec![1.0, 2.0, 1.0]
            };
            let numerator = if direct { vec![1.0, 2.0] } else { vec![1.0] };
            let mut m: GraphModel =
                serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json"))?;
            m.dt = ts / 10.0;
            m.duration = 100.0 * ts;
            m.solver = Some(SolverConfig::Rk45 {
                abs_tolerance: 1e-11,
                rel_tolerance: 1e-11,
                min_step: ts * 1e-12,
                max_step: m.dt,
            });
            let change_time = (change * 10) as f64 * m.dt;
            m.blocks[0].params = BlockKind::Step {
                before: vec![before],
                after: vec![after],
                step_time: change_time,
            };
            m.blocks[1].params = BlockKind::TransferFunction {
                numerator: numerator.clone(),
                denominator: denominator.clone(),
                initial: vec![0.0; denominator.len() - 1],
            };
            let analog = simulate_graph_with_budget(&m, 10_000_000)?;
            let (num, den) = if method == "zoh" {
                let r = discretize_zoh(&ZohRequest {
                    numerator,
                    denominator,
                    sample_time: ts,
                })?;
                (r.numerator, r.denominator)
            } else {
                let r = discretize_tustin(&TustinRequest {
                    numerator,
                    denominator,
                    sample_time: ts,
                    prewarp_omega: None,
                })?;
                (r.numerator, r.denominator)
            };
            m.blocks[1].params = BlockKind::DiscreteTransferFunction {
                numerator: num,
                initial: vec![0.0; den.len() - 1],
                denominator: den,
                sample_time: ts,
            };
            let digital = simulate_graph_with_budget(&m, 10_000_000)?;
            let step = |k: usize| {
                let t = k as f64 * ts;
                if direct {
                    2.0 - (-t).exp()
                } else {
                    1.0 - (1.0 + t) * (-t).exp()
                }
            };
            let (
                mut max,
                mut squares,
                mut post_max,
                mut post_squares,
                mut analog_error,
                mut digital_error,
            ) = (0.0_f64, 0.0, 0.0_f64, 0.0, 0.0_f64, 0.0_f64);
            let (mut y1, mut y2, mut u1, mut u2) = (0.0, 0.0, 0.0, 0.0);
            let mut points = vec![];
            for k in 0..=100 {
                let u = if k < change { before } else { after };
                let exact = before * step(k)
                    + if k >= change {
                        (after - before) * step(k - change)
                    } else {
                        0.0
                    };
                let av = analog.traces[0].values[k * 10][0];
                let bv = digital.traces[0].values[k * 10][0];
                let y = if direct {
                    let p = (2.0 - ts) / (2.0 + ts);
                    p * y1 + ts / (2.0 + ts) * (u + u1)
                } else {
                    18.0 / 11.0 * y1 - 81.0 / 121.0 * y2 + (u + 2.0 * u1 + u2) / 121.0
                };
                let discrete_exact = if method == "zoh" {
                    exact
                } else if direct {
                    u + y
                } else {
                    y
                };
                (y2, y1, u2, u1) = (y1, y, u1, u);
                analog_error = analog_error.max((av - exact).abs());
                digital_error = digital_error.max((bv - discrete_exact).abs());
                let delta = bv - av;
                max = max.max(delta.abs());
                squares += delta * delta;
                if k >= change {
                    post_max = post_max.max(delta.abs());
                    post_squares += delta * delta;
                }
                if [0, change - 1, change, change + 1, 100].contains(&k) {
                    points.push(serde_json::json!({"sample":k,"time":analog.times[k*10],"input":u,"continuous":av,"discrete":bv}));
                }
            }
            assert!(analog_error < 2e-10);
            assert!(digital_error < 2e-13);
            cases.push(serde_json::json!({"name":name,"method":method,"sample_time":ts,"samples":101,"input":{"before":before,"after":after,"change_sample":change,"change_time":change_time},"max_abs_delta":max,"rms_delta":(squares/101.0).sqrt(),"post_change_max_abs_delta":post_max,"post_change_rms_delta":(post_squares/(101-change) as f64).sqrt(),"continuous_equation_max_error":analog_error,"discrete_equation_max_error":digital_error,"points":points}));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"isolated scalar zero-state held input, one sample-aligned change, sample instants only","cases":cases})
        )?
    );
    Ok(())
}
