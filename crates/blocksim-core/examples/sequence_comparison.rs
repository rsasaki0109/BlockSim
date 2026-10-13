//! Held arbitrary sample tables and independently derived ODE/IIR responses.
use blocksim_core::graph::{
    discretize::{discretize_tustin, discretize_zoh, TustinRequest, ZohRequest},
    simulate_graph_with_budget, BlockKind, GraphModel, SolverConfig,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases = vec![];
    for direct in [false, true] {
        for method in ["zoh", "tustin"] {
            let ts = if direct { 0.1 } else { 0.2 };
            let mut m: GraphModel =
                serde_json::from_str(include_str!("../../../samples/discrete/sequence.json"))?;
            m.dt = ts / 10.0;
            m.duration = 100.0 * ts;
            m.solver = Some(SolverConfig::Rk45 {
                abs_tolerance: 1e-11,
                rel_tolerance: 1e-11,
                min_step: ts * 1e-12,
                max_step: m.dt,
            });
            if direct {
                m.blocks[0].params = BlockKind::SampledInput {
                    values: (0..101)
                        .map(|k| {
                            vec![if k % 3 == 0 {
                                0.75
                            } else if k % 3 == 1 {
                                -0.5
                            } else {
                                0.25
                            }]
                        })
                        .collect(),
                    sample_time: ts,
                };
            }
            let num = if direct { vec![1.0, 2.0] } else { vec![1.0] };
            let den = if direct {
                vec![1.0, 1.0]
            } else {
                vec![1.0, 2.0, 1.0]
            };
            m.blocks[1].params = BlockKind::TransferFunction {
                numerator: num.clone(),
                denominator: den.clone(),
                initial: vec![0.0; den.len() - 1],
            };
            let a = simulate_graph_with_budget(&m, 10_000_000)?;
            let (num, den) = if method == "zoh" {
                let r = discretize_zoh(&ZohRequest {
                    numerator: num,
                    denominator: den,
                    sample_time: ts,
                })?;
                (r.numerator, r.denominator)
            } else {
                let r = discretize_tustin(&TustinRequest {
                    numerator: num,
                    denominator: den,
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
            let b = simulate_graph_with_budget(&m, 10_000_000)?;
            let input = a.traces.iter().find(|t| t.id == "input_record").unwrap();
            let other = b.traces.iter().find(|t| t.id == "input_record").unwrap();
            assert_eq!(input.values, other.values);
            for (i, v) in input.values.iter().enumerate() {
                assert_eq!(v[0], input.values[i / 10 * 10][0]);
            }
            let r = (-ts).exp();
            let (mut x1, mut x2, mut y1, mut y2, mut u1, mut u2) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            let (mut max, mut square, mut analog_error, mut digital_error) =
                (0.0_f64, 0.0, 0.0_f64, 0.0_f64);
            let mut points = vec![];
            for k in 0..=100 {
                let time = a.times[k * 10];
                let u = input.values[k * 10][0];
                let exact = if direct { u + x1 } else { x2 };
                let y = if direct {
                    (2.0 - ts) / (2.0 + ts) * y1 + ts / (2.0 + ts) * (u + u1)
                } else {
                    18.0 / 11.0 * y1 - 81.0 / 121.0 * y2 + (u + 2.0 * u1 + u2) / 121.0
                };
                let digital_exact = if method == "zoh" {
                    exact
                } else {
                    y + if direct { u } else { 0.0 }
                };
                let av = a.traces[0].values[k * 10][0];
                let bv = b.traces[0].values[k * 10][0];
                analog_error = analog_error.max((av - exact).abs());
                digital_error = digital_error.max((bv - digital_exact).abs());
                let delta = bv - av;
                max = max.max(delta.abs());
                square += delta * delta;
                if [0, 1, 5, 20, 30, 55, 70, 100].contains(&k) {
                    points.push(serde_json::json!({"sample":k,"time":time,"input":u,"continuous":av,"discrete":bv}));
                }
                x2 = r * x2 + ts * r * x1 + (1.0 - (1.0 + ts) * r) * u;
                x1 = r * x1 + (1.0 - r) * u;
                (y2, y1, u2, u1) = (y1, y, u1, u);
            }
            assert!(analog_error < 2e-10);
            assert!(digital_error < 2e-13);
            cases.push(serde_json::json!({"direct":direct,"method":method,"sample_time":ts,"samples":101,"max_abs_delta":max,"rms_delta":(square/101.0).sqrt(),"continuous_equation_max_error":analog_error,"discrete_equation_max_error":digital_error,"held_between_samples":true,"inputs_identical":true,"points":points}));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"isolated scalar zero-state arbitrary held sample table, sample instants only","cases":cases})
        )?
    );
    Ok(())
}
