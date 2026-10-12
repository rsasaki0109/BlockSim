use blocksim_core::graph::{
    discretize::{discretize_tustin, discretize_zoh, TustinRequest, ZohRequest},
    simulate_graph_with_budget, BlockKind, GraphModel, SolverConfig,
};
fn model(ts: f64, denominator: Vec<f64>) -> GraphModel {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json")).unwrap();
    m.dt = ts / 10.0;
    m.duration = ts * 100.0;
    m.solver = Some(SolverConfig::Rk45 {
        abs_tolerance: 1e-11,
        rel_tolerance: 1e-11,
        min_step: ts * 1e-12,
        max_step: ts / 10.0,
    });
    m.blocks[0].params = BlockKind::Constant { value: vec![1.0] };
    m.blocks[1].params = BlockKind::TransferFunction {
        numerator: vec![1.0],
        initial: vec![0.0; denominator.len() - 1],
        denominator,
    };
    m
}
#[test]
fn first_order_tustin_comparison_matches_independent_sample_equation_and_improves_with_period() {
    let mut previous = f64::INFINITY;
    for ts in [0.4, 0.2, 0.1] {
        let mut m = model(ts, vec![1.0, 1.0]);
        let a = simulate_graph_with_budget(&m, 10_000_000).unwrap();
        let r = discretize_tustin(&TustinRequest {
            numerator: vec![1.0],
            denominator: vec![1.0, 1.0],
            sample_time: ts,
            prewarp_omega: None,
        })
        .unwrap();
        m.blocks[1].params = BlockKind::DiscreteTransferFunction {
            numerator: r.numerator,
            denominator: r.denominator,
            initial: vec![0.0],
            sample_time: ts,
        };
        let b = simulate_graph_with_budget(&m, 10_000_000).unwrap();
        let mut max: f64 = 0.0;
        for k in 0..=100 {
            let t = k as f64 * ts;
            let analog = 1.0 - (-t).exp();
            let digital = 1.0 - 2.0 / (2.0 + ts) * ((2.0 - ts) / (2.0 + ts)).powi(k as i32);
            assert!((a.traces[0].values[k * 10][0] - analog).abs() < 2e-10);
            assert!((b.traces[0].values[k * 10][0] - digital).abs() < 2e-13);
            max = max.max((b.traces[0].values[k * 10][0] - a.traces[0].values[k * 10][0]).abs());
        }
        assert!((max - ts / (2.0 + ts)).abs() < 2e-13);
        assert!(max < previous);
        previous = max;
    }
}
#[test]
fn second_order_zoh_matches_the_continuous_reference_only_at_conversion_instants() {
    let ts = 0.2;
    let mut m = model(ts, vec![1.0, 2.0, 1.0]);
    let a = simulate_graph_with_budget(&m, 10_000_000).unwrap();
    let r = discretize_zoh(&ZohRequest {
        numerator: vec![1.0],
        denominator: vec![1.0, 2.0, 1.0],
        sample_time: ts,
    })
    .unwrap();
    m.blocks[1].params = BlockKind::DiscreteTransferFunction {
        numerator: r.numerator,
        denominator: r.denominator,
        initial: vec![0.0; 2],
        sample_time: ts,
    };
    let b = simulate_graph_with_budget(&m, 10_000_000).unwrap();
    for k in 0..=100 {
        let t = k as f64 * ts;
        let exact = 1.0 - (1.0 + t) * (-t).exp();
        assert!((a.traces[0].values[k * 10][0] - exact).abs() < 2e-10);
        assert!((b.traces[0].values[k * 10][0] - exact).abs() < 2e-13);
    }
    assert_eq!(b.traces[0].values[1][0], 0.0);
    assert!(a.traces[0].values[1][0] > 0.0001);
}
