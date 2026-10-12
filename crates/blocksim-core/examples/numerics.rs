//! Reproducible analytic/dense-output/event tests and a fine-RK4 hybrid reference.
use blocksim_core::graph::{simulate_graph, GraphModel, SolverConfig};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rows = vec![];
    for name in ["first-order", "oscillator", "off-grid-step", "mixed-pid"] {
        let model: GraphModel = serde_json::from_str(&std::fs::read_to_string(format!(
            "samples/numerics/{name}.json"
        ))?)?;
        let adaptive = simulate_graph(&model)?;
        let mut reference = model.clone();
        reference.solver = Some(SolverConfig::Rk4 {});
        let factor = if name == "mixed-pid" { 4 } else { 1 };
        reference.dt /= factor as f64;
        let fixed = simulate_graph(&reference)?;
        let mut max_error: f64 = 0.0;
        for (trace_index, trace) in adaptive.traces.iter().enumerate() {
            for (i, values) in trace.values.iter().enumerate() {
                for (c, value) in values.iter().enumerate() {
                    let time = adaptive.times[i];
                    let expected = match name {
                        "first-order" => 1.0 - (-time / 0.25).exp(),
                        "oscillator" => time.cos(),
                        "off-grid-step" => 2.0 * (time - 0.015).max(0.0),
                        _ => fixed.traces[trace_index].values[i * factor][c],
                    };
                    max_error = max_error.max((value - expected).abs());
                }
            }
        }
        let limit = if name == "off-grid-step" {
            1e-12
        } else if name == "mixed-pid" {
            1e-5
        } else {
            2e-8
        };
        if max_error > limit {
            return Err(format!("{name}: error {max_error} exceeds {limit}").into());
        }
        let adaptive_stats = adaptive.statistics.unwrap();
        let fixed_stats = fixed.statistics.unwrap();
        rows.push(serde_json::json!({"sample":name,"reference":if name=="mixed-pid"{"RK4 at dt/4"}else{"analytic"},"points":adaptive.times.len(),"max_abs_error":max_error,"error_limit":limit,"rk45":adaptive_stats,"rk4":fixed_stats,"rate_evaluation_reduction_percent":100.0*(1.0-adaptive_stats.rate_evaluations as f64/fixed_stats.rate_evaluations as f64)}));
    }
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}
