//! Independent margin equations plus actual closed-loop pole/step checks.
use blocksim_core::graph::{
    frequency::{analyze_discrete_frequency, FrequencyRequest},
    simulate_graph,
    state_space::{analyze_discrete_state_space, StateSpaceRequest},
    BlockKind, GraphModel,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let original: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/open-loop.json"))?;
    let q = FrequencyRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
        min_omega: 0.01,
        max_omega: std::f64::consts::PI / 0.1,
        points: 201,
        open_loop: true,
    };
    let mut cases = vec![];
    for (k, class, outside) in [
        (0.75, "stable", 0),
        (1.0, "boundary_or_uncertain", 0),
        (1.25, "unstable", 2),
    ] {
        let mut m = original.clone();
        m.blocks[1].params = BlockKind::Gain { gain: k };
        let r = analyze_discrete_frequency(&m, &q)?;
        let margins = r.response.margins.as_ref().unwrap();
        assert_eq!(margins.gain_crossings.len(), 1);
        assert_eq!(margins.phase_crossings.len(), 1);
        let angle = (1.25 - k * k).acos();
        let expected_pm = 180.0 - (angle + angle.sin().atan2(angle.cos() - 0.5)).to_degrees();
        let expected_gc = angle / 0.1;
        let expected_pc = 0.25_f64.acos() / 0.1;
        let expected_gm = -20.0 * k.log10();
        let gc = &margins.gain_crossings[0];
        let pc = &margins.phase_crossings[0];
        let omega_error = (gc.omega - expected_gc)
            .abs()
            .max((pc.omega - expected_pc).abs());
        let margin_error = (gc.phase_margin_deg - expected_pm)
            .abs()
            .max((pc.gain_margin_db - expected_gm).abs());
        assert!(omega_error < 1e-8 && margin_error < 1e-8);
        // Explicitly add the outer negative-feedback edges; margin analysis itself never rewires.
        let mut error = m.blocks[2].clone();
        error.id = "error".into();
        error.params = BlockKind::Sum {
            signs: vec![1.0, -1.0],
        };
        m.blocks.push(error);
        m.edges[0].from = "error".into();
        let mut edge = m.edges[0].clone();
        edge.from = "source".into();
        edge.to = "error".into();
        m.edges.push(edge.clone());
        edge.from = "delay2".into();
        edge.to_port = 1;
        m.edges.push(edge);
        let state = analyze_discrete_state_space(
            &m,
            &StateSpaceRequest {
                source: "source".into(),
                source_component: 0,
                scope: "output".into(),
                scope_component: 0,
            },
        )?;
        assert_eq!(state.stability.classification, class);
        assert_eq!(state.stability.outside_unit_circle_poles, Some(outside));
        let beta = (k - 0.0625).sqrt();
        let pole_error = state
            .poles
            .roots
            .iter()
            .map(|p| (p.real - 0.25).hypot(p.imag.abs() - beta))
            .fold(0.0_f64, f64::max);
        assert!(pole_error < 1e-10);
        let time = simulate_graph(&m)?;
        let dc = k / (0.5 + k);
        let phase = beta.atan2(0.25);
        let a = -dc / 2.0;
        let b = dc * 0.75 / (2.0 * beta);
        let mut absolute = 0.0_f64;
        let mut scaled = 0.0_f64;
        let mut peak = 0.0_f64;
        for (tick, y) in time.traces[0].values.iter().enumerate() {
            let n = (tick / 5) as f64;
            let expected =
                dc + 2.0 * k.sqrt().powf(n) * (a * (phase * n).cos() - b * (phase * n).sin());
            let error = (y[0] - expected).abs();
            absolute = absolute.max(error);
            scaled = scaled.max(error / (1.0 + expected.abs()));
            peak = peak.max(y[0].abs())
        }
        assert!(scaled < 1e-9);
        cases.push(serde_json::json!({"gain":k,"sample_time_s":r.sample_time,"nyquist_rad_s":r.nyquist_omega,"gain_crossing_rad_s":gc.omega,"phase_margin_deg":gc.phase_margin_deg,"phase_crossing_rad_s":pc.omega,"gain_margin_db":pc.gain_margin_db,"gain_margin_ratio":pc.gain_margin,"max_crossing_frequency_error":omega_error,"max_margin_error_deg_or_db":margin_error,"additional_evaluations":margins.additional_evaluations,"closed_loop_A":state.a,"closed_loop_classification":state.stability.classification,"closed_loop_outside_count":state.stability.outside_unit_circle_poles,"closed_loop_poles":state.poles.roots,"max_pole_error":pole_error,"recorded_points":time.times.len(),"sample_instants":101,"max_step_absolute_error":absolute,"max_step_scaled_error":scaled,"max_absolute_output":peak}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"explicitly selected same-clock open-loop K/[z(z-0.5)]","margin_reference":"Omega_gc=acos(1.25-K^2), Omega_pc=acos(0.25), GM=1/K, PM=180-(Omega_gc+atan2(sin(Omega_gc),cos(Omega_gc)-0.5))*180/pi","closed_loop_reference":"actual additional outer negative-feedback edges, poles 0.25+-j*sqrt(K-0.0625), analytic held step from y[k+2]=0.5y[k+1]-Ky[k]+K","cases":cases})
        )?
    );
    Ok(())
}
