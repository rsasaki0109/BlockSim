//! Connected feedback matrices vs independent Cardano poles and analytic step residues.
use blocksim_core::graph::{
    simulate_graph,
    state_space::{analyze_state_space, StateSpaceRequest},
    BlockKind, GraphModel,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let p: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        "samples/state-space/feedback-plan.json",
    )?)?;
    let model: GraphModel = serde_json::from_value(p["model"].clone())?;
    let q: StateSpaceRequest = serde_json::from_value(p["request"].clone())?;
    let mut cases = vec![];
    for (k, class, right) in [
        (4.0, "stable", 0),
        (6.0, "boundary_or_uncertain", 0),
        (8.0, "unstable", 2),
    ] {
        let mut m = model.clone();
        m.blocks[2].params = BlockKind::Gain { gain: k };
        m.duration = 100.0;
        m.dt = 0.05;
        let r = analyze_state_space(&m, &q)?;
        assert_eq!(r.stability.classification, class);
        assert_eq!(r.stability.right_half_plane_poles, Some(right));
        assert_eq!(r.poles.status, "verified_residual");
        assert_eq!(
            r.a,
            vec![
                vec![-3.0, -2.0, -k],
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0]
            ]
        );
        assert_eq!(r.b, vec![k, 0.0, 0.0]);
        assert_eq!(r.c, vec![0.0, 0.0, 1.0]);
        assert_eq!(r.d, 0.0);
        let disc = (k * k / 4.0 - 1.0 / 27.0).sqrt();
        let u = (-k / 2.0 + disc).cbrt();
        let v = (-k / 2.0 - disc).cbrt();
        let real = u + v - 1.0;
        let alpha = -(u + v) / 2.0 - 1.0;
        let beta = 3.0_f64.sqrt() * (u - v) / 2.0;
        let expected = [(real, 0.0), (alpha, beta), (alpha, -beta)];
        let root_error = expected
            .iter()
            .map(|(re, im)| {
                r.poles
                    .roots
                    .iter()
                    .map(|p| (p.real - re).hypot(p.imag - im))
                    .fold(f64::INFINITY, f64::min)
            })
            .fold(0.0_f64, f64::max);
        assert!(root_error < 1e-9);
        let real_residue = k / (real * (3.0 * real * real + 6.0 * real + 2.0));
        let dr = 3.0 * (alpha * alpha - beta * beta) + 6.0 * alpha + 2.0;
        let di = 6.0 * alpha * beta + 6.0 * beta;
        let denominator_real = alpha * dr - beta * di;
        let denominator_imag = alpha * di + beta * dr;
        let norm = denominator_real * denominator_real + denominator_imag * denominator_imag;
        let rr = k * denominator_real / norm;
        let ri = -k * denominator_imag / norm;
        let time = simulate_graph(&m)?;
        let mut time_error = 0.0_f64;
        let mut max_abs = 0.0_f64;
        let mut late_min = f64::INFINITY;
        let mut late_max = f64::NEG_INFINITY;
        for (t, y) in time.times.iter().zip(&time.traces[0].values) {
            let analytic = 1.0
                + real_residue * (real * t).exp()
                + 2.0 * (alpha * t).exp() * (rr * (beta * t).cos() - ri * (beta * t).sin());
            time_error = time_error.max((y[0] - analytic).abs());
            max_abs = max_abs.max(y[0].abs());
            if *t >= 80.0 {
                late_min = late_min.min(y[0]);
                late_max = late_max.max(y[0]);
            }
        }
        assert!(time_error < if k == 8.0 { 2e-4 } else { 2e-7 });
        cases.push(serde_json::json!({"gain":k,"classification":class,"right_half_plane_poles":right,"A":r.a,"B":r.b,"C":r.c,"D":r.d,"poles":r.poles.roots,"max_root_error_vs_cardano":root_error,"cayley_hamilton_residual":r.characteristic_residual,"max_matrix_determinant_residual":r.matrix_residuals.iter().copied().fold(0.0,f64::max),"time_samples":time.times.len(),"max_time_error_vs_analytic_step":time_error,"late_window_s":[80,100],"late_min":late_min,"late_max":late_max,"max_absolute_output":max_abs}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"actual connected Source-Sum-Gain-TransferFunction-Scope negative feedback, selected Scope ancestry","root_reference":"independent Cardano for s³+3s²+2s+K","time_reference":"analytic partial-fraction unit step of K/(s³+3s²+2s+K), compared with actual connected RK45 graph","cases":cases})
        )?
    );
    Ok(())
}
