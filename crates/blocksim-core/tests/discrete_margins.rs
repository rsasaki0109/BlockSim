use blocksim_core::graph::{
    frequency::{analyze_discrete_frequency, validate_discrete_frequency, FrequencyRequest},
    simulate_graph,
    state_space::{analyze_discrete_state_space, StateSpaceRequest},
    BlockKind, GraphModel,
};
fn model() -> GraphModel {
    serde_json::from_str(include_str!("../../../samples/discrete/open-loop.json")).unwrap()
}
fn query() -> FrequencyRequest {
    FrequencyRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
        min_omega: 0.01,
        max_omega: std::f64::consts::PI / 0.1,
        points: 201,
        open_loop: true,
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8 * (1.0 + b.abs()), "{a} != {b}")
}
fn closed_loop(mut m: GraphModel) -> GraphModel {
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
    m
}
#[test]
fn positive_zero_and_negative_margins_match_independent_unit_circle_formulas() {
    for k in [0.75, 1.0, 1.25] {
        let mut m = model();
        m.blocks[1].params = BlockKind::Gain { gain: k };
        let r = analyze_discrete_frequency(&m, &query()).unwrap();
        let a = r.response.margins.unwrap();
        assert_eq!(a.gain_crossings.len(), 1);
        assert_eq!(a.phase_crossings.len(), 1);
        assert!(a.issues.is_empty());
        let angle = (1.25 - k * k).acos();
        let phase = -angle - angle.sin().atan2(angle.cos() - 0.5);
        close(a.gain_crossings[0].omega, angle / 0.1);
        close(
            a.gain_crossings[0].phase_margin_deg,
            180.0 + phase.to_degrees(),
        );
        close(a.phase_crossings[0].omega, 0.25_f64.acos() / 0.1);
        close(a.phase_crossings[0].phase_level_deg, -180.0);
        close(a.phase_crossings[0].gain_margin_db, -20.0 * k.log10());
        close(a.phase_crossings[0].gain_margin.unwrap(), 1.0 / k);
        assert!(a.additional_evaluations <= 4096);
        assert!(a.method.contains("within Nyquist"));
    }
}
#[test]
fn opt_in_only_adds_margins_and_never_modifies_points_model_or_settings() {
    let m = model();
    let before = serde_json::to_string(&m).unwrap();
    let mut q = query();
    q.open_loop = false;
    let off = analyze_discrete_frequency(&m, &q).unwrap();
    q.open_loop = true;
    let on = analyze_discrete_frequency(&m, &q).unwrap();
    assert_eq!(
        serde_json::to_string(&off.response.points).unwrap(),
        serde_json::to_string(&on.response.points).unwrap()
    );
    assert!(off.response.margins.is_none());
    assert!(on.response.margins.is_some());
    assert_eq!(serde_json::to_string(&m).unwrap(), before);
    assert!(q.open_loop);
    assert_eq!(off.sample_time, on.sample_time);
    assert_eq!(off.nyquist_omega, on.nyquist_omega);
}
#[test]
fn explicitly_connected_closed_loops_have_known_poles_and_analytic_step_responses() {
    for (k, class, count) in [
        (0.75, "stable", 0),
        (1.0, "boundary_or_uncertain", 0),
        (1.25, "unstable", 2),
    ] {
        let mut m = model();
        m.blocks[1].params = BlockKind::Gain { gain: k };
        let m = closed_loop(m);
        let q = StateSpaceRequest {
            source: "source".into(),
            source_component: 0,
            scope: "output".into(),
            scope_component: 0,
        };
        let r = analyze_discrete_state_space(&m, &q).unwrap();
        assert_eq!(r.a, vec![vec![0.0, 1.0], vec![-k, 0.5]]);
        assert_eq!(r.stability.classification, class);
        assert_eq!(r.stability.outside_unit_circle_poles, Some(count));
        for p in &r.poles.roots {
            close(p.real, 0.25);
            close(p.imag.abs(), (k - 0.0625).sqrt());
            close(p.real.hypot(p.imag), k.sqrt())
        }
        let time = simulate_graph(&m).unwrap();
        let beta = (k - 0.0625).sqrt();
        let angle = beta.atan2(0.25);
        let dc = k / (0.5 + k);
        let real = -dc / 2.0;
        let imag = dc * 0.75 / (2.0 * beta);
        for (tick, y) in time.traces[0].values.iter().enumerate() {
            let n = (tick / 5) as f64;
            let expected =
                dc + 2.0 * k.sqrt().powf(n) * (real * (angle * n).cos() - imag * (angle * n).sin());
            close(y[0], expected)
        }
    }
}
fn delays(count: usize, gain: f64) -> GraphModel {
    let mut m = model();
    m.blocks
        .retain(|b| b.id == "source" || b.id == "gain" || b.id == "output");
    m.blocks[1].params = BlockKind::Gain { gain };
    m.edges.retain(|e| e.from == "source");
    let template = model().blocks[3].clone();
    for i in 1..=count {
        let mut b = template.clone();
        b.id = format!("d{i}");
        m.blocks.push(b);
        let mut edge = m.edges[0].clone();
        edge.from = if i == 1 {
            "gain".into()
        } else {
            format!("d{}", i - 1)
        };
        edge.to = format!("d{i}");
        m.edges.push(edge)
    }
    let mut edge = m.edges[0].clone();
    edge.from = format!("d{count}");
    edge.to = "output".into();
    m.edges.push(edge);
    m
}
#[test]
fn multiple_odd_phase_levels_and_nyquist_endpoint_candidates_remain_distinct() {
    let r = analyze_discrete_frequency(&delays(4, 0.75), &query()).unwrap();
    let m = r.response.margins.unwrap();
    assert!(m.gain_crossings.is_empty());
    assert_eq!(m.phase_crossings.len(), 2);
    for (i, p) in m.phase_crossings.iter().enumerate() {
        close(
            p.omega,
            std::f64::consts::PI * (1.0 + 2.0 * i as f64) / 4.0 / 0.1,
        );
        close(p.phase_level_deg, -180.0 - 360.0 * i as f64);
        close(p.gain_margin_db, -20.0 * 0.75_f64.log10())
    }
    let r = analyze_discrete_frequency(&delays(1, 0.75), &query()).unwrap();
    let m = r.response.margins.unwrap();
    assert_eq!(m.phase_crossings.len(), 1);
    assert_eq!(m.phase_crossings[0].omega, r.nyquist_omega);
    close(m.phase_crossings[0].gain_margin.unwrap(), 4.0 / 3.0);
}
#[test]
fn plateaus_zero_gaps_and_coarse_phase_ambiguity_do_not_fabricate_isolated_crossings() {
    let r = analyze_discrete_frequency(&delays(1, 1.0), &query()).unwrap();
    let m = r.response.margins.unwrap();
    assert!(m.gain_crossings.is_empty());
    assert!(m.issues.iter().any(|i| i.kind == "gain_plateau"));
    let r = analyze_discrete_frequency(&delays(1, 0.0), &query()).unwrap();
    let m = r.response.margins.unwrap();
    assert!(m.gain_crossings.is_empty() && m.phase_crossings.is_empty());
    assert!(m.issues.iter().any(|i| i.kind == "gap"));
    let r = analyze_discrete_frequency(
        &delays(1, 0.75),
        &FrequencyRequest {
            min_omega: 1e-9,
            points: 2,
            ..query()
        },
    )
    .unwrap();
    let m = r.response.margins.unwrap();
    assert!(m.phase_crossings.is_empty());
    assert!(m.issues.iter().any(|i| i.kind == "phase_ambiguity"));
    let mut m = delays(1, 0.75);
    let mut sum = model().blocks[2].clone();
    sum.id = "sum".into();
    sum.params = BlockKind::Sum {
        signs: vec![1.0, -1.0],
    };
    m.blocks.push(sum);
    let e = m.edges.iter_mut().find(|e| e.to == "d1").unwrap();
    e.to = "sum".into();
    let mut e = m.edges[0].clone();
    e.from = "d1".into();
    e.to = "sum".into();
    e.to_port = 1;
    m.edges.push(e.clone());
    e.from = "sum".into();
    e.to = "d1".into();
    e.to_port = 0;
    m.edges.push(e);
    let r = analyze_discrete_frequency(&m, &query()).unwrap();
    assert!(r.response.points.last().unwrap().error.is_some());
    let m = r.response.margins.unwrap();
    assert!(m.issues.iter().any(|i| i.kind == "gap"));
    assert!(m
        .phase_crossings
        .iter()
        .all(|p| p.omega < query().max_omega));
}
#[test]
fn margin_preflight_keeps_nyquist_rate_and_component_validation_with_strict_flags() {
    let m = model();
    assert!(validate_discrete_frequency(&m, &query()).is_ok());
    assert!(validate_discrete_frequency(
        &m,
        &FrequencyRequest {
            max_omega: 32.0,
            ..query()
        }
    )
    .is_err());
    assert!(validate_discrete_frequency(
        &m,
        &FrequencyRequest {
            source_component: 1,
            ..query()
        }
    )
    .is_err());
    let mut m = model();
    if let BlockKind::Delay { sample_time, .. } = &mut m.blocks[4].params {
        *sample_time = 0.2
    }
    assert!(validate_discrete_frequency(&m, &query())
        .unwrap_err()
        .contains("common sample"));
    let json = serde_json::to_string(&query()).unwrap();
    assert!(serde_json::from_str::<FrequencyRequest>(&json.replace("true", "1")).is_err());
}
#[test]
fn hierarchical_vector_queries_keep_exact_components_and_zero_branch_margins() {
    let mut child = model();
    for b in &mut child.blocks {
        b.width = 2;
        match &mut b.params {
            BlockKind::Step { before, after, .. } => {
                *before = vec![0.0; 2];
                *after = vec![1.0, 2.0]
            }
            BlockKind::Delay { initial, .. } => *initial = vec![0.0; 2],
            _ => (),
        }
    }
    child.blocks.last_mut().unwrap().params = BlockKind::Output { port: 0 };
    let mut outer = model();
    outer
        .blocks
        .retain(|b| b.id == "source" || b.id == "output");
    let mut nested = outer.blocks[0].clone();
    nested.id = "nested".into();
    nested.params = BlockKind::Subsystem {
        model: Box::new(child),
    };
    outer.blocks.push(nested);
    outer
        .blocks
        .iter_mut()
        .find(|b| b.id == "output")
        .unwrap()
        .width = 2;
    outer.edges.clear();
    let mut edge = model().edges[0].clone();
    edge.from = "nested".into();
    edge.to = "output".into();
    outer.edges.push(edge);
    let q = FrequencyRequest {
        source: "nested/source".into(),
        source_component: 1,
        scope_component: 1,
        ..query()
    };
    let r = analyze_discrete_frequency(&outer, &q).unwrap();
    assert_eq!(r.states, 2);
    assert_eq!(r.response.margins.unwrap().gain_crossings.len(), 1);
    let r = analyze_discrete_frequency(
        &outer,
        &FrequencyRequest {
            source_component: 0,
            ..q
        },
    )
    .unwrap();
    let m = r.response.margins.unwrap();
    assert!(m.gain_crossings.is_empty() && m.phase_crossings.is_empty());
    assert!(m.issues.iter().any(|i| i.kind == "gap"));
}
#[test]
fn positive_branch_margins_do_not_certify_an_unstable_signed_gain_closed_loop() {
    let mut m = model();
    m.blocks[1].params = BlockKind::Gain { gain: -0.75 };
    let r = analyze_discrete_frequency(&m, &query()).unwrap();
    let margins = r.response.margins.unwrap();
    assert_eq!(margins.gain_crossings.len(), 1);
    assert_eq!(margins.phase_crossings.len(), 1);
    close(
        margins.gain_crossings[0].phase_margin_deg,
        237.9100487437197,
    );
    close(
        margins.phase_crossings[0].gain_margin_db,
        20.0 * 2.0_f64.log10(),
    );
    let closed = closed_loop(m);
    let q = StateSpaceRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
    };
    let r = analyze_discrete_state_space(&closed, &q).unwrap();
    assert_eq!(r.stability.classification, "unstable");
    assert_eq!(r.stability.outside_unit_circle_poles, Some(1));
    assert!(r.poles.roots.iter().any(|p| p.real > 1.0));
}
