//! Independently derived sample sequences and unit-circle formulas.
use blocksim_core::graph::{
    frequency::{analyze_discrete_frequency, FrequencyRequest},
    simulate_graph,
    state_space::{analyze_discrete_state_space, StateSpaceRequest},
    BlockKind, GraphModel,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let original: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/transfer.json"))?;
    let mut cases = vec![];
    for name in ["direct_iir", "delayed_iir", "fir"] {
        let mut m = original.clone();
        if name == "delayed_iir" {
            m.blocks[1].params = BlockKind::DiscreteTransferFunction {
                numerator: vec![0.2],
                denominator: vec![1.0, -0.8],
                initial: vec![0.0],
                sample_time: 0.1,
            };
        } else if name == "fir" {
            m.blocks[1].params = BlockKind::DiscreteTransferFunction {
                numerator: vec![0.25, 0.5, 0.25],
                denominator: vec![1.0, 0.0, 0.0],
                initial: vec![0.0, 0.0],
                sample_time: 0.1,
            };
        }
        let r = simulate_graph(&m)?;
        let mut time_error: f64 = 0.0;
        for (tick, row) in r.traces[0].values.iter().enumerate() {
            let n = tick / 5;
            let expected = match name {
                "direct_iir" => 1.0 - 0.8_f64.powi(n as i32 + 1),
                "delayed_iir" => 1.0 - 0.8_f64.powi(n as i32),
                _ => {
                    if n == 0 {
                        0.25
                    } else if n == 1 {
                        0.75
                    } else {
                        1.0
                    }
                }
            };
            time_error = time_error.max((row[0] - expected).abs());
        }
        assert!(time_error < 1e-13);
        let ss = analyze_discrete_state_space(
            &m,
            &StateSpaceRequest {
                source: "source".into(),
                scope: "output".into(),
                source_component: 0,
                scope_component: 0,
            },
        )?;
        let f = analyze_discrete_frequency(
            &m,
            &FrequencyRequest {
                source: "source".into(),
                scope: "output".into(),
                source_component: 0,
                scope_component: 0,
                min_omega: 0.01,
                max_omega: std::f64::consts::PI / 0.1,
                points: 201,
                open_loop: false,
            },
        )?;
        let mut complex_error: f64 = 0.0;
        for p in &f.response.points {
            let a = p.omega * 0.1;
            let (re, im) = if name == "fir" {
                (
                    0.25 + 0.5 * a.cos() + 0.25 * (2.0 * a).cos(),
                    -0.5 * a.sin() - 0.25 * (2.0 * a).sin(),
                )
            } else {
                let x = a.cos() - 0.8;
                let y = a.sin();
                let d = x * x + y * y;
                let v = if name == "direct_iir" { 0.16 } else { 0.2 };
                (
                    (if name == "direct_iir" { 0.2 } else { 0.0 }) + v * x / d,
                    -v * y / d,
                )
            };
            complex_error = complex_error.max((p.real.unwrap() - re).hypot(p.imag.unwrap() - im));
        }
        assert!(complex_error < 1e-13);
        // Fit actual sine-driven samples after IIR transients, for 20 complete cycles.
        let omega = std::f64::consts::PI / (4.0 * 0.1);
        m.duration = 40.0;
        m.blocks[0].params = BlockKind::Sine {
            amplitude: 1.0,
            frequency: omega / std::f64::consts::TAU,
            phase: 0.0,
            offset: 0.0,
        };
        let time = simulate_graph(&m)?;
        let mut re = 0.0;
        let mut im = 0.0;
        for n in 240..400 {
            let tick = n * 5;
            let angle = omega * time.times[tick];
            let y = time.traces[0].values[tick][0];
            re += y * angle.sin() / 80.0;
            im += y * angle.cos() / 80.0;
        }
        let f = analyze_discrete_frequency(
            &m,
            &FrequencyRequest {
                source: "source".into(),
                scope: "output".into(),
                source_component: 0,
                scope_component: 0,
                min_omega: omega,
                max_omega: 2.0 * omega,
                points: 2,
                open_loop: false,
            },
        )?;
        let p = &f.response.points[0];
        let harmonic_error = (re - p.real.unwrap()).hypot(im - p.imag.unwrap());
        assert!(harmonic_error < 1e-10);
        cases.push(serde_json::json!({"case":name,"sample_time_s":0.1,"recorded_step_points":r.times.len(),"max_step_error":time_error,"max_complex_error_vs_independent_formula":complex_error,"a":ss.a,"b":ss.b,"c":ss.c,"d":ss.d,"internal_stability":ss.stability,"harmonic_window_s":[24,40],"harmonic_samples":160,"harmonic_omega_rad_s":omega,"harmonic_real":re,"harmonic_imag":im,"frequency_real":p.real,"frequency_imag":p.imag,"harmonic_complex_error":harmonic_error}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"reference":"z-descending proper coefficients; direct step 1-.8^(n+1), delayed step 1-.8^n, FIR step [.25,.75,1,...]; independent polynomial unit-circle evaluation and actual time sine fit, no hold reconstruction","cases":cases})
        )?
    );
    Ok(())
}
