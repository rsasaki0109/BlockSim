use blocksim_core::graph::{
    frequency::{validate_frequency, FrequencyRequest},
    simulate_graph,
    state_space::{validate_discrete_state_space, validate_state_space, StateSpaceRequest},
    validate_graph, BlockKind, GraphModel, SolverConfig,
};

fn sample() -> GraphModel {
    serde_json::from_str(include_str!("../../../samples/graphs/dead-zone.json")).unwrap()
}
fn constant(values: Vec<f64>, lower: f64, upper: f64) -> GraphModel {
    let mut m = sample();
    for b in &mut m.blocks {
        b.width = values.len();
    }
    m.blocks[0].params = BlockKind::Constant { value: values };
    m.blocks[1].params = BlockKind::DeadZone { lower, upper };
    m.duration = 0.02;
    m
}
fn adaptive(m: &mut GraphModel) {
    m.solver = Some(SolverConfig::Rk45 {
        abs_tolerance: 1e-11,
        rel_tolerance: 1e-10,
        min_step: 1e-9,
        max_step: 0.1,
    });
}
#[test]
fn vector_outputs_include_both_boundaries_and_remove_only_the_excess() {
    let m = constant(vec![-2., -1., -0.5, 0., 0.5, 1., 2.], -1., 1.);
    let r = simulate_graph(&m).unwrap();
    let out = r.traces.iter().find(|t| t.id == "output").unwrap();
    assert!(out
        .values
        .iter()
        .all(|row| row == &vec![-1., 0., 0., 0., 0., 0., 1.]));
    let asymmetric =
        simulate_graph(&constant(vec![-0.8, -0.25, -0.2, 0.3, 0.65], -0.2, 0.3)).unwrap();
    let expected = [-0.6, -0.05, 0., 0., 0.35];
    for row in &asymmetric.traces[0].values {
        for (actual, expected) in row.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-14);
        }
    }
}
#[test]
fn zero_width_and_shifted_intervals_are_defined_without_span_overflow() {
    for (bounds, values, expected) in [
        ([0., 0.], vec![-2., 0., 2.], vec![-2., 0., 2.]),
        ([1., 1.], vec![-2., 1., 3.], vec![-3., 0., 2.]),
        (
            [1., 2.],
            vec![0., 1., 1.5, 2., 3.],
            vec![-1., 0., 0., 0., 1.],
        ),
        (
            [-2., -1.],
            vec![-3., -2., -1.5, -1., 0.],
            vec![-1., 0., 0., 0., 1.],
        ),
        ([-1e308, 1e308], vec![-1e308, 0., 1e308], vec![0., 0., 0.]),
    ] {
        let r = simulate_graph(&constant(values, bounds[0], bounds[1])).unwrap();
        assert!(r.traces[0].values.iter().all(|row| row == &expected));
    }
}
#[test]
fn invalid_thresholds_and_algebraic_feedback_are_rejected() {
    for (lower, upper) in [
        (1., -1.),
        (f64::NAN, 1.),
        (-1., f64::INFINITY),
        (f64::NEG_INFINITY, 1.),
    ] {
        let err = validate_graph(&constant(vec![0.], lower, upper)).unwrap_err();
        assert!(err.contains("DeadZone"), "{err}");
    }
    let mut m = constant(vec![0.], -0.1, 0.1);
    m.edges[0].from = "zone".into();
    assert!(validate_graph(&m).unwrap_err().contains("Algebraic loop"));
}
#[test]
fn computed_overflow_is_a_runtime_error_and_does_not_change_the_model() {
    let m = constant(vec![1e308], -1e308, -1e308);
    let before = serde_json::to_string(&m).unwrap();
    let err = simulate_graph(&m).unwrap_err();
    assert!(err.contains("zone at t=0"), "{err}");
    assert_eq!(serde_json::to_string(&m).unwrap(), before);
}
#[test]
fn sample_matches_independent_held_rows_with_rk4_and_rk45() {
    let expected = [
        [-0.6, 0.5],
        [-0.05, 0.],
        [0., 0.],
        [0., 0.],
        [0., 0.],
        [0., 0.],
        [0., 0.],
        [0.35, -0.45],
    ];
    for use_adaptive in [false, true] {
        let mut m = sample();
        if use_adaptive {
            adaptive(&mut m);
        }
        let before = serde_json::to_string(&m).unwrap();
        let r = simulate_graph(&m).unwrap();
        assert_eq!(r.times.len(), 161);
        let out = r.traces.iter().find(|t| t.id == "output").unwrap();
        for (k, row) in out.values.iter().enumerate() {
            for (actual, expected) in row.iter().zip(expected[(k / 10) % 8]) {
                assert!((actual - expected).abs() < 1e-14);
            }
        }
        assert_eq!(serde_json::to_string(&m).unwrap(), before);
        let restored: GraphModel = serde_json::from_str(&before).unwrap();
        assert_eq!(
            serde_json::to_string(&simulate_graph(&restored).unwrap()).unwrap(),
            serde_json::to_string(&r).unwrap()
        );
    }
}
#[test]
fn continuous_step_between_ticks_drives_exact_integral_after_threshold_subtraction() {
    for use_adaptive in [false, true] {
        let mut m = constant(vec![0.], -0.5, 0.25);
        m.duration = 0.5;
        m.blocks[0].params = BlockKind::Step {
            before: vec![-0.5],
            after: vec![1.25],
            step_time: 0.157,
        };
        m.blocks[2].params = BlockKind::Integrator { initial: vec![0.] };
        m.blocks[2].id = "integral".into();
        m.blocks[3].params = BlockKind::Scope {};
        m.edges[1].to = "integral".into();
        m.edges[2].from = "integral".into();
        if use_adaptive {
            adaptive(&mut m);
        }
        let r = simulate_graph(&m).unwrap();
        for (time, row) in r.times.iter().zip(&r.traces[0].values) {
            assert!((row[0] - (time - 0.157).max(0.)).abs() < 1e-12);
        }
    }
}
#[test]
fn nonlinear_ancestry_is_not_treated_as_linear_even_with_zero_thresholds() {
    let m = constant(vec![0.], 0., 0.);
    let mut q = StateSpaceRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
    };
    let mut f = FrequencyRequest {
        source: q.source.clone(),
        source_component: 0,
        scope: q.scope.clone(),
        scope_component: 0,
        min_omega: 0.1,
        max_omega: 10.,
        points: 10,
        open_loop: false,
    };
    for error in [
        validate_frequency(&m, &f).unwrap_err(),
        validate_state_space(&m, &q).unwrap_err(),
        validate_discrete_state_space(&m, &q).unwrap_err(),
    ] {
        assert!(error.contains("zone: DeadZone is nonlinear"), "{error}");
    }
    // A separate linear Scope excludes the Dead Zone from its selected ancestry.
    q.scope = "raw".into();
    f.scope = q.scope.clone();
    validate_frequency(&m, &f).unwrap();
    validate_state_space(&m, &q).unwrap();
}
