use blocksim_core::graph::{
    frequency::{analyze_frequency, FrequencyRequest},
    simulate_graph,
    state_space::{
        analyze_discrete_state_space, analyze_state_space, validate_state_space, StateSpaceRequest,
        StateSpaceResult,
    },
    Block, BlockKind, Edge, GraphModel,
};
fn plan(name: &str) -> GraphModel {
    let p: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(format!("../../samples/state-space/{name}-plan.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(p["model"].clone()).unwrap()
}
fn query() -> StateSpaceRequest {
    StateSpaceRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
    }
}
fn rank16(r: &StateSpaceResult) {
    let s = r.structure.as_ref().unwrap();
    assert_eq!(s.controllability.numerical_rank, Some(16));
    assert_eq!(s.observability.numerical_rank, Some(16));
    assert_eq!(s.controllability.status, "full_numerical_rank");
    assert_eq!(s.observability.status, "full_numerical_rank");
    assert_eq!(r.states.len(), 16);
}
#[test]
fn sixteen_integrators_preserve_exact_realization_rank_poles_and_independent_time_response() {
    let m = plan("sixteen-integrators");
    let r = analyze_state_space(&m, &query()).unwrap();
    rank16(&r);
    assert_eq!(r.poles.roots.len(), 16);
    assert!(r.poles.roots.iter().all(|p| p.real == 0. && p.imag == 0.));
    assert_eq!(r.stability.classification, "boundary_or_uncertain");
    let mut active = r.b.clone();
    for _ in 0..16 {
        assert_eq!(active.iter().filter(|v| **v == 1.).count(), 1);
        active =
            r.a.iter()
                .map(|row| row.iter().zip(&active).map(|(a, b)| a * b).sum())
                .collect();
    }
    assert_eq!(active, vec![0.; 16]);
    let simulation = simulate_graph(&m).unwrap();
    let factorial = (1..=16).fold(1., |p, i| p * i as f64);
    for (t, row) in simulation.times.iter().zip(&simulation.traces[0].values) {
        assert!((row[0] - t.powi(16) / factorial).abs() < 1e-9);
    }
}
#[test]
fn sixteen_coupled_feedback_states_have_independently_known_unit_circle_eigenvalues() {
    let mut m = plan("sixteen-integrators");
    m.blocks.push(Block {
        id: "error".into(),
        label: "error".into(),
        width: 1,
        position: [0., 0.],
        params: BlockKind::Sum {
            signs: vec![1., -1.],
        },
    });
    m.edges[0].from = "error".into();
    m.edges.push(Edge {
        from: "source".into(),
        from_port: 0,
        to: "error".into(),
        to_port: 0,
    });
    m.edges.push(Edge {
        from: "state-15".into(),
        from_port: 0,
        to: "error".into(),
        to_port: 1,
    });
    let r = analyze_state_space(&m, &query()).unwrap();
    rank16(&r);
    assert_eq!(r.poles.status, "verified_residual");
    assert_eq!(r.poles.roots.len(), 16);
    for k in 0..16 {
        let theta = (2 * k + 1) as f64 * std::f64::consts::PI / 16.;
        assert!(r
            .poles
            .roots
            .iter()
            .any(|p| (p.real - theta.cos()).hypot(p.imag - theta.sin()) < 1e-8));
    }
    assert_ne!(r.stability.classification, "stable");
    if let Some(n) = r.stability.right_half_plane_poles {
        assert_eq!(n, 8);
    }
    assert!(r.matrix_residuals.iter().all(|r| *r <= 1e-8));
}
#[test]
fn sixteen_lowpass_states_keep_repeated_negative_poles_and_matrix_coefficients() {
    let m = plan("sixteen-lowpass");
    let r = analyze_state_space(&m, &query()).unwrap();
    assert_eq!(r.check.states, 16);
    assert_eq!(r.poles.status, "verified_residual");
    assert_eq!(r.poles.roots.len(), 16);
    assert!(r.poles.roots.iter().all(|p| p.real == -1. && p.imag == 0.));
    assert_eq!(r.stability.classification, "stable");
    assert_eq!(r.structure.as_ref().unwrap().controllability.dimension, 16);
    assert!(r.characteristic_residual.unwrap() < 1e-8);
}
#[test]
fn sixteen_delays_match_step_acquisitions_and_independent_discrete_realization() {
    let m = plan("sixteen-delays");
    let r = analyze_discrete_state_space(&m, &query()).unwrap();
    rank16(&r);
    assert_eq!(r.sample_time, Some(0.2));
    assert_eq!(r.poles.roots.len(), 16);
    assert!(r.poles.roots.iter().all(|p| p.real == 0. && p.imag == 0.));
    assert_eq!(r.stability.classification, "stable");
    let simulation = simulate_graph(&m).unwrap();
    for (k, row) in simulation.traces[0].values.iter().enumerate() {
        assert_eq!(row[0], if k < 64 { 0. } else { 1. });
    }
}
#[test]
fn high_order_bode_matches_independent_transfer_formula_with_all_sixteen_states_retained() {
    let m = plan("sixteen-integrators");
    let q = FrequencyRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
        min_omega: 0.2,
        max_omega: 5.,
        points: 101,
        open_loop: false,
    };
    for p in analyze_frequency(&m, &q).unwrap().points {
        let expected = p.omega.powi(-16);
        assert!(p.error.is_none());
        assert!((p.real.unwrap() - expected).hypot(p.imag.unwrap()) / expected < 1e-10);
    }
}
#[test]
fn discrete_sixteen_delay_bode_matches_unit_circle_formula_at_every_frequency() {
    let m = plan("sixteen-delays");
    let q = FrequencyRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
        min_omega: 0.01,
        max_omega: std::f64::consts::PI / 0.2,
        points: 101,
        open_loop: false,
    };
    for p in blocksim_core::graph::frequency::analyze_discrete_frequency(&m, &q)
        .unwrap()
        .response
        .points
    {
        let theta = 16. * p.omega * 0.2;
        assert!(p.error.is_none());
        assert!((p.real.unwrap() - theta.cos()).hypot(p.imag.unwrap() + theta.sin()) < 1e-10);
    }
}
#[test]
fn unrelated_source_and_zero_output_retain_internal_modes_with_rank_zero() {
    let mut m = plan("sixteen-integrators");
    m.blocks.push(Block {
        id: "unrelated".into(),
        label: "unrelated".into(),
        width: 1,
        position: [0., 0.],
        params: BlockKind::Constant { value: vec![1.] },
    });
    let mut q = query();
    q.source = "unrelated".into();
    let r = analyze_state_space(&m, &q).unwrap();
    assert_eq!(r.structure.unwrap().controllability.numerical_rank, Some(0));
    assert_eq!(r.states.len(), 16);
    assert_eq!(r.poles.roots.len(), 16);
    let mut m = plan("sixteen-integrators");
    m.blocks.push(Block {
        id: "zero".into(),
        label: "zero".into(),
        width: 1,
        position: [0., 0.],
        params: BlockKind::Gain { gain: 0. },
    });
    m.edges.last_mut().unwrap().to = "zero".into();
    m.edges.push(Edge {
        from: "zero".into(),
        from_port: 0,
        to: "output".into(),
        to_port: 0,
    });
    let r = analyze_state_space(&m, &query()).unwrap();
    assert_eq!(r.structure.unwrap().observability.numerical_rank, Some(0));
    assert_eq!(r.states.len(), 16);
    assert_eq!(r.poles.roots.len(), 16);
}
#[test]
fn seventeen_related_states_are_refused_without_silent_reduction() {
    let mut m = plan("sixteen-integrators");
    let mut b = m.blocks[1].clone();
    b.id = "extra".into();
    m.blocks.push(b);
    m.edges.last_mut().unwrap().to = "extra".into();
    m.edges.push(Edge {
        from: "extra".into(),
        from_port: 0,
        to: "output".into(),
        to_port: 0,
    });
    assert!(validate_state_space(&m, &query())
        .unwrap_err()
        .contains("16 related continuous"));
}
