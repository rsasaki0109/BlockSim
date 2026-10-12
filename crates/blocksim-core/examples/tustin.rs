//! Actual discrete Bode/Step responses checked against independent bilinear formulas.
use blocksim_core::graph::{
    discretize::{discretize_tustin, TustinRequest},
    frequency::{analyze_discrete_frequency, FrequencyRequest},
    simulate_graph, BlockKind, GraphModel,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let original: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/tustin.json"))?;
    let mut cases = vec![];
    for (ts, prewarp) in [(0.1, None), (0.2, None), (0.2, Some(5.0))] {
        let q = TustinRequest {
            numerator: vec![1.0],
            denominator: vec![1.0, 2.0, 1.0],
            sample_time: ts,
            prewarp_omega: prewarp,
        };
        let conversion = discretize_tustin(&q)?;
        let mut m = original.clone();
        m.blocks[1].params = BlockKind::DiscreteTransferFunction {
            numerator: conversion.numerator.clone(),
            denominator: conversion.denominator.clone(),
            initial: vec![0.0, 0.0],
            sample_time: ts,
        };
        let time = simulate_graph(&m)?;
        let k = conversion.mapping_scale;
        let alpha = (k - 1.0) / (k + 1.0);
        let beta = 1.0 / (k + 1.0);
        let a = beta * beta - 1.0;
        let b = ((2.0 * alpha + 3.0) * beta * beta - 1.0) / alpha - a;
        let period = (ts / m.dt).round() as usize;
        let mut step_error: f64 = 0.0;
        for (tick, row) in time.traces[0].values.iter().enumerate() {
            let n = (tick / period) as i32;
            let expected = 1.0 + (a + b * n as f64) * alpha.powi(n);
            step_error = step_error.max((row[0] - expected).abs());
        }
        assert!(step_error < 1e-10);
        let query = FrequencyRequest {
            source: "source".into(),
            scope: "output".into(),
            source_component: 0,
            scope_component: 0,
            min_omega: 0.01,
            max_omega: std::f64::consts::PI / ts,
            points: 201,
            open_loop: false,
        };
        let frequency = analyze_discrete_frequency(&m, &query)?;
        let mut complex_error: f64 = 0.0;
        for p in &frequency.response.points {
            let w = k * (p.omega * ts / 2.0).tan();
            let d = (1.0 + w * w).powi(2);
            let re = (1.0 - w * w) / d;
            let im = -2.0 * w / d;
            complex_error = complex_error.max((p.real.unwrap() - re).hypot(p.imag.unwrap() - im));
        }
        assert!(complex_error < 1e-10);
        let selected = analyze_discrete_frequency(
            &m,
            &FrequencyRequest {
                min_omega: 5.0,
                max_omega: 10.0,
                points: 2,
                ..query
            },
        )?;
        let p = &selected.response.points[0];
        let selected_error = (p.real.unwrap() + 24.0 / 676.0).hypot(p.imag.unwrap() + 10.0 / 676.0);
        if prewarp.is_some() {
            assert!(selected_error < 1e-12);
        } else {
            assert!(selected_error > 1e-4);
        }
        let mut response = vec![];
        for n in 0..=5 {
            response.push(serde_json::json!({"time_s":n as f64*ts,"output":time.traces[0].values[n*period][0]}));
        }
        cases.push(serde_json::json!({"conversion":conversion,"recorded_points":time.times.len(),"max_step_error_vs_independent_closed_form":step_error,"max_complex_error_vs_warped_continuous_formula":complex_error,"first_sample_outputs":response,"selected_physical_omega_rad_s":5,"selected_real":p.real,"selected_imag":p.imag,"selected_error_vs_continuous_same_frequency":selected_error}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"reference":"G(s)=1/(s+1)^2; s=j*k*tan(omega*Ts/2). Step y[n]=1+(A+B*n)*alpha^n, alpha=(k-1)/(k+1), beta=1/(k+1), A=beta^2-1, B=((2alpha+3)beta^2-1)/alpha-A; output held between samples. Not ZOH equivalence.","cases":cases})
        )?
    );
    Ok(())
}
