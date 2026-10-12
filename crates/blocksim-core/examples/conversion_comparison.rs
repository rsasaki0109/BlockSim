//! Reproducible bounded scalar unit-step comparisons, with independent equations.
use blocksim_core::graph::{
    discretize::{discretize_tustin, discretize_zoh, TustinRequest, ZohRequest},
    simulate_graph_with_budget, BlockKind, GraphModel, SolverConfig,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases = vec![];
    for (order, ts) in [(1, 0.4), (1, 0.2), (1, 0.1), (2, 0.2)] {
        for method in ["zoh", "tustin"] {
            let den = if order == 1 {
                vec![1.0, 1.0]
            } else {
                vec![1.0, 2.0, 1.0]
            };
            let mut a: GraphModel =
                serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json"))?;
            a.dt = ts / 10.0;
            a.duration = 100.0 * ts;
            a.solver = Some(SolverConfig::Rk45 {
                abs_tolerance: 1e-11,
                rel_tolerance: 1e-11,
                min_step: ts * 1e-12,
                max_step: ts / 10.0,
            });
            a.blocks[0].params = BlockKind::Constant { value: vec![1.0] };
            a.blocks[1].params = BlockKind::TransferFunction {
                numerator: vec![1.0],
                denominator: den.clone(),
                initial: vec![0.0; order],
            };
            let mut b = a.clone();
            let (num, den) = if method == "zoh" {
                let r = discretize_zoh(&ZohRequest {
                    numerator: vec![1.0],
                    denominator: den,
                    sample_time: ts,
                })?;
                (r.numerator, r.denominator)
            } else {
                let r = discretize_tustin(&TustinRequest {
                    numerator: vec![1.0],
                    denominator: den,
                    sample_time: ts,
                    prewarp_omega: None,
                })?;
                (r.numerator, r.denominator)
            };
            b.blocks[1].params = BlockKind::DiscreteTransferFunction {
                numerator: num,
                denominator: den,
                initial: vec![0.0; order],
                sample_time: ts,
            };
            let analog = simulate_graph_with_budget(&a, 10_000_000)?;
            let digital = simulate_graph_with_budget(&b, 10_000_000)?;
            let mut max: f64 = 0.0;
            let mut squares = 0.0;
            let mut analog_error: f64 = 0.0;
            let mut digital_error: f64 = 0.0;
            for k in 0..=100 {
                let t = k as f64 * ts;
                let exact = if order == 1 {
                    1.0 - (-t).exp()
                } else {
                    1.0 - (1.0 + t) * (-t).exp()
                };
                let av = analog.traces[0].values[k * 10][0];
                let bv = digital.traces[0].values[k * 10][0];
                analog_error = analog_error.max((av - exact).abs());
                let discrete_exact = if method == "zoh" {
                    Some(exact)
                } else if order == 1 {
                    Some(1.0 - 2.0 / (2.0 + ts) * ((2.0 - ts) / (2.0 + ts)).powi(k as i32))
                } else {
                    None
                };
                if let Some(reference) = discrete_exact {
                    digital_error = digital_error.max((bv - reference).abs());
                }
                max = max.max((bv - av).abs());
                squares += (bv - av).powi(2);
            }
            assert!(analog_error < 2e-10);
            assert!(digital_error < 2e-13);
            cases.push(serde_json::json!({"order":order,"method":method,"sample_time":ts,"samples":101,"max_abs_delta":max,"rms_delta":(squares/101.0).sqrt(),"initial_continuous":analog.traces[0].values[0][0],"initial_discrete":digital.traces[0].values[0][0],"continuous_equation_max_error":analog_error,"discrete_equation_max_error":if method=="zoh"||order==1 {Some(digital_error)} else {None}}));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"isolated scalar unit step, zero initial state, sample instants only","cases":cases})
        )?
    );
    Ok(())
}
