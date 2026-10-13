use blocksim_core::graph::{
    discretize::{discretize_tustin, discretize_zoh, TustinRequest, ZohRequest},
    simulate_graph_with_budget, BlockKind, GraphModel,
};
fn model(ts: f64, f: f64, phase: f64, amplitude: f64, offset: f64, direct: bool) -> GraphModel {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/held-sine.json")).unwrap();
    m.dt = ts / 10.0;
    m.duration = 100.0 * ts;
    m.blocks[0].params = BlockKind::Sine {
        amplitude,
        frequency: f,
        phase,
        offset,
    };
    if direct {
        m.blocks[1].params = BlockKind::TransferFunction {
            numerator: vec![1.0, 2.0],
            denominator: vec![1.0, 1.0],
            initial: vec![0.0],
        };
    }
    m.blocks[3].params = BlockKind::DiscreteTransferFunction {
        numerator: vec![1.0],
        denominator: vec![1.0],
        initial: vec![],
        sample_time: ts,
    };
    m
}
#[test]
fn actual_sampled_sine_zoh_matches_independent_cascade_and_direct_term_at_each_clock() {
    for (ts, f, phase, amp, offset, direct) in [
        (0.2, 0.5, 0.4, 1.0, 0.25, false),
        (0.2, 2.5, std::f64::consts::FRAC_PI_2, -0.5, 0.25, false),
        (0.1, 1.3, 0.4, 1.0, -0.25, true),
        (0.2, 0.0, 0.4, 1.0, 0.25, true),
    ] {
        let mut m = model(ts, f, phase, amp, offset, direct);
        let a = simulate_graph_with_budget(&m, 10_000_000).unwrap();
        let BlockKind::TransferFunction {
            numerator,
            denominator,
            ..
        } = &m.blocks[1].params
        else {
            unreachable!()
        };
        let r = discretize_zoh(&ZohRequest {
            numerator: numerator.clone(),
            denominator: denominator.clone(),
            sample_time: ts,
        })
        .unwrap();
        m.blocks[1].params = BlockKind::DiscreteTransferFunction {
            numerator: r.numerator,
            initial: vec![0.0; r.denominator.len() - 1],
            denominator: r.denominator,
            sample_time: ts,
        };
        let b = simulate_graph_with_budget(&m, 10_000_000).unwrap();
        let input = a.traces.iter().find(|t| t.id == "input_record").unwrap();
        let digital_input = b.traces.iter().find(|t| t.id == "input_record").unwrap();
        assert_eq!(input.values, digital_input.values);
        for (i, v) in input.values.iter().enumerate() {
            assert_eq!(v[0], input.values[i / 10 * 10][0]);
        }
        let r = (-ts).exp();
        let (mut x1, mut x2) = (0.0, 0.0);
        for k in 0..=100 {
            let u = offset + amp * (std::f64::consts::TAU * f * a.times[k * 10] + phase).sin();
            assert!((input.values[k * 10][0] - u).abs() < 2e-14);
            let exact = if direct { u + x1 } else { x2 };
            assert!((a.traces[0].values[k * 10][0] - exact).abs() < 2e-10);
            assert!((b.traces[0].values[k * 10][0] - exact).abs() < 2e-13);
            x2 = r * x2 + ts * r * x1 + (1.0 - (1.0 + ts) * r) * u;
            x1 = r * x1 + (1.0 - r) * u;
        }
    }
}
#[test]
fn sampled_sine_tustin_matches_independent_iir_for_second_order_and_direct_first_order() {
    for direct in [false, true] {
        let ts = 0.2;
        let mut m = model(ts, 0.5, 0.4, 1.0, 0.25, direct);
        let BlockKind::TransferFunction {
            numerator,
            denominator,
            ..
        } = &m.blocks[1].params
        else {
            unreachable!()
        };
        let r = discretize_tustin(&TustinRequest {
            numerator: numerator.clone(),
            denominator: denominator.clone(),
            sample_time: ts,
            prewarp_omega: None,
        })
        .unwrap();
        m.blocks[1].params = BlockKind::DiscreteTransferFunction {
            numerator: r.numerator,
            initial: vec![0.0; r.denominator.len() - 1],
            denominator: r.denominator,
            sample_time: ts,
        };
        let b = simulate_graph_with_budget(&m, 10_000_000).unwrap();
        let input = b.traces.iter().find(|t| t.id == "input_record").unwrap();
        let (mut y1, mut y2, mut u1, mut u2) = (0.0, 0.0, 0.0, 0.0);
        for k in 0..=100 {
            let u = input.values[k * 10][0];
            let y = if direct {
                9.0 / 11.0 * y1 + (u + u1) / 11.0
            } else {
                18.0 / 11.0 * y1 - 81.0 / 121.0 * y2 + (u + 2.0 * u1 + u2) / 121.0
            };
            assert!(
                (b.traces[0].values[k * 10][0] - (y + if direct { u } else { 0.0 })).abs() < 2e-13
            );
            (y2, y1, u2, u1) = (y1, y, u1, u);
        }
    }
}
