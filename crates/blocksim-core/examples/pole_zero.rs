//! Independent Cardano roots and actual time responses of the defined unit-feedback realization.
use blocksim_core::graph::{
    pole_zero::{analyze_pole_zero, PoleZeroRequest},
    simulate_graph, BlockKind, GraphModel,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let p: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        "samples/pole-zero/feedback-plan.json",
    )?)?;
    let m: GraphModel = serde_json::from_value(p["model"].clone())?;
    let q: PoleZeroRequest = serde_json::from_value(p["request"].clone())?;
    let mut cases = vec![];
    for (k, classification, right) in [
        (4.0, "stable", 0),
        (6.0, "boundary_or_uncertain", 0),
        (8.0, "unstable", 2),
    ] {
        let mut model = m.clone();
        if let BlockKind::TransferFunction { numerator, .. } = &mut model.blocks[1].params {
            numerator[0] = k;
        }
        let r = analyze_pole_zero(&model, &q)?;
        assert_eq!(r.stability.classification, classification);
        assert_eq!(r.stability.right_half_plane_poles, Some(right));
        assert_eq!(r.poles.status, "verified_residual");
        // s=t-1 gives t³-t+k=0; Cardano is independent of the simultaneous numerical solver.
        let disc = (k * k / 4.0 - 1.0 / 27.0).sqrt();
        let u = (-k / 2.0 + disc).cbrt();
        let v = (-k / 2.0 - disc).cbrt();
        let real = u + v;
        let expected = [
            (real - 1.0, 0.0),
            (-real / 2.0 - 1.0, 3.0_f64.sqrt() * (u - v) / 2.0),
            (-real / 2.0 - 1.0, -3.0_f64.sqrt() * (u - v) / 2.0),
        ];
        let error = expected
            .iter()
            .map(|(re, im)| {
                r.poles
                    .roots
                    .iter()
                    .map(|p| (p.real - re).hypot(p.imag - im))
                    .fold(f64::INFINITY, f64::min)
            })
            .fold(0.0_f64, f64::max);
        assert!(error < 1e-9);
        // Simulate T=N/(D+N) explicitly, rather than silently rewiring the original graph.
        model.duration = 100.0;
        model.dt = 0.05;
        model.blocks[1].params = BlockKind::TransferFunction {
            numerator: r.numerator.clone(),
            denominator: r.denominator.clone(),
            initial: vec![0.0; 3],
        };
        let time = simulate_graph(&model)?;
        let y = &time.traces[0].values;
        let late: Vec<_> = time
            .times
            .iter()
            .zip(y)
            .filter(|(t, _)| **t >= 80.0)
            .map(|(_, v)| v[0])
            .collect();
        let lo = late.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = late.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let max_abs = y.iter().map(|v| v[0].abs()).fold(0.0_f64, f64::max);
        if k == 4.0 {
            assert!(late.iter().all(|x| (x - 1.0).abs() < 0.001));
        } else if k == 6.0 {
            assert!(hi - lo > 1.0 && max_abs < 3.0);
        } else {
            assert!(max_abs > 100.0);
        }
        cases.push(serde_json::json!({"gain":k,"classification":classification,"right_half_plane_poles":right,"poles":r.poles.roots,"max_root_error_vs_cardano":error,"time_samples":time.times.len(),"late_window_s":[80,100],"late_min":lo,"late_max":hi,"max_absolute_output":max_abs}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"selected continuous realization T=N/(D+N), no cancellation, not arbitrary graph stability","root_reference":"independent Cardano formula for s³+3s²+2s+K","time_reference":"actual RK45 unit-step simulation of the explicit T(s) realization","cases":cases})
        )?
    );
    Ok(())
}
