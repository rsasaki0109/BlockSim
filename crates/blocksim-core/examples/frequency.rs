//! Independent analytic frequency/time checks for the two shipped LTI examples.
use blocksim_core::graph::{
    frequency::{analyze_frequency, FrequencyRequest},
    simulate_graph, GraphModel,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rows = vec![];
    for name in ["lowpass", "feedback"] {
        let plan: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(format!(
            "samples/frequency/{name}-plan.json"
        ))?)?;
        let model: GraphModel = serde_json::from_value(plan["model"].clone())?;
        let request: FrequencyRequest = serde_json::from_value(plan["request"].clone())?;
        let response = analyze_frequency(&model, &request)?;
        let mut complex_error: f64 = 0.0;
        let mut db_error: f64 = 0.0;
        let mut phase_error: f64 = 0.0;
        let mut residual: f64 = 0.0;
        for p in &response.points {
            if let Some(error) = &p.error {
                return Err(error.clone().into());
            }
            let w = p.omega;
            let a = 1.0 - w * w;
            let b = if name == "lowpass" { 2.0 * w } else { 3.0 * w };
            let den = a * a + b * b;
            let (real, imag, phase) = if name == "lowpass" {
                (a / den, -b / den, -2.0 * w.atan().to_degrees())
            } else {
                (
                    (a + 2.0 * w * b) / den,
                    (2.0 * w * a - b) / den,
                    ((2.0 * w).atan() - b.atan2(a)).to_degrees(),
                )
            };
            complex_error =
                complex_error.max((p.real.unwrap() - real).hypot(p.imag.unwrap() - imag));
            db_error =
                db_error.max((p.magnitude_db.unwrap() - 20.0 * real.hypot(imag).log10()).abs());
            phase_error = phase_error.max((p.phase_deg.unwrap() - phase).abs());
            residual = residual.max(p.relative_residual.unwrap());
        }
        let time = simulate_graph(&model)?;
        let trace = time.traces.iter().find(|t| t.id == "output").unwrap();
        let mut time_error: f64 = 0.0;
        for (t, row) in time.times.iter().zip(&trace.values) {
            let expected = if name == "lowpass" {
                1.0 - (1.0 + t) * (-t).exp()
            } else {
                let root = 5.0_f64.sqrt();
                let r1 = (-3.0 + root) / 2.0;
                let r2 = (-3.0 - root) / 2.0;
                let c1 = (2.0 + r2) / (r1 - r2);
                1.0 + c1 * (r1 * t).exp() + (-1.0 - c1) * (r2 * t).exp()
            };
            time_error = time_error.max((row[0] - expected).abs());
        }
        if complex_error > 1e-10 || db_error > 1e-8 || phase_error > 1e-8 || time_error > 2e-8 {
            return Err(format!("{name}: analytic validation failed").into());
        }
        rows.push(serde_json::json!({"sample":name,"frequency_points":response.points.len(),"related_variables":response.check.variables,"max_complex_error":complex_error,"max_db_error":db_error,"max_phase_degrees_error":phase_error,"max_relative_residual":residual,"time_points":time.times.len(),"max_time_error":time_error}));
    }
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}
