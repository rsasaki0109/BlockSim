//! Sampled feedback vs known complex roots and an independent finite geometric step formula.
use blocksim_core::graph::{
    simulate_graph,
    state_space::{analyze_discrete_state_space, StateSpaceRequest},
    BlockKind, GraphModel,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/feedback.json"))?;
    let q = StateSpaceRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
    };
    let theta = std::f64::consts::PI / 3.0;
    let mut cases = vec![];
    for (radius, class, count) in [
        (0.8, "stable", 0),
        (1.0, "boundary_or_uncertain", 0),
        (1.2, "unstable", 2),
    ] {
        let mut m = model.clone();
        m.blocks[1].params = BlockKind::Sum {
            signs: vec![1.0, radius, -radius * radius],
        };
        let r = analyze_discrete_state_space(&m, &q)?;
        assert_eq!(r.stability.classification, class);
        assert_eq!(r.stability.outside_unit_circle_poles, Some(count));
        assert_eq!(r.poles.status, "verified_residual");
        let root_error = r
            .poles
            .roots
            .iter()
            .map(|p| (p.real - radius / 2.0).hypot(p.imag.abs() - radius * 3.0_f64.sqrt() / 2.0))
            .fold(0.0_f64, f64::max);
        assert!(root_error < 1e-10);
        let time = simulate_graph(&m)?;
        let mut absolute = 0.0_f64;
        let mut relative = 0.0_f64;
        let mut max_abs = 0.0_f64;
        let mut samples = 0;
        for (tick, y) in time.traces[0].values.iter().enumerate() {
            // Away from sample instants the Scope holds the last sample.
            let k = tick / 5;
            let rho = radius.powi(k as i32);
            let angle = theta * k as f64;
            let nr = 1.0 - rho * angle.cos();
            let ni = -rho * angle.sin();
            let dr = 1.0 - radius / 2.0;
            let di = -radius * theta.sin();
            let norm = dr * dr + di * di;
            let gr = (nr * dr + ni * di) / norm;
            let gi = (ni * dr - nr * di) / norm;
            let expected = (theta.sin() * gr + theta.cos() * gi) / theta.sin();
            let error = (y[0] - expected).abs();
            absolute = absolute.max(error);
            relative = relative.max(error / (1.0 + expected.abs()));
            max_abs = max_abs.max(y[0].abs());
            if tick % 5 == 0 {
                samples += 1
            }
        }
        assert!(relative < 1e-10);
        cases.push(serde_json::json!({"radius":radius,"classification":class,"outside_unit_circle_poles":count,"sample_time_s":r.sample_time,"sample_instants":samples,"recorded_points":time.times.len(),"A":r.a,"B":r.b,"C":r.c,"D":r.d,"poles":r.poles.roots,"max_root_error_vs_radius_angle":root_error,"max_step_absolute_error":absolute,"max_step_scaled_error":relative,"max_absolute_output":max_abs,"characteristic_residual":r.characteristic_residual,"matrix_residuals":r.matrix_residuals}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"actual same-clock connected two-Delay feedback, Scope ancestor realization","root_reference":"radius * exp(+-j*pi/3)","step_reference":"Im(exp(j*theta)*(1-(radius*exp(j*theta))^k)/(1-radius*exp(j*theta)))/sin(theta); held between sample instants","cases":cases})
        )?
    );
    Ok(())
}
