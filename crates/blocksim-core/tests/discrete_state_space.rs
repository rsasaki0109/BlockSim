use blocksim_core::graph::{
    simulate_graph,
    state_space::{
        analyze_discrete_state_space, analyze_state_space, validate_discrete_state_space,
        StateSpaceRequest, StateSpaceResult,
    },
    BlockKind, GraphModel,
};
use blocksim_core::{PidConfig, PidMode};
fn model() -> GraphModel {
    serde_json::from_str(include_str!("../../../samples/discrete/feedback.json")).unwrap()
}
fn query() -> StateSpaceRequest {
    StateSpaceRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-11 * (1.0 + b.abs()), "{a} != {b}");
}
// Independent recurrence uses only the exported matrices, while the simulator ticks PID/Delay blocks.
fn matches_time(m: &GraphModel, r: &StateSpaceResult) {
    let t = simulate_graph(m).unwrap();
    let ts = r.sample_time.unwrap();
    let stride = (ts / m.dt).round() as usize;
    let mut x = vec![0.0; r.b.len()];
    for (tick, values) in t
        .traces
        .iter()
        .find(|t| t.id == r.request.scope)
        .unwrap()
        .values
        .iter()
        .enumerate()
    {
        if tick % stride != 0 {
            continue;
        }
        let u = match m.blocks[0].params {
            BlockKind::Sine {
                amplitude,
                frequency,
                phase,
                offset,
            } => {
                offset
                    + amplitude * (std::f64::consts::TAU * frequency * t.times[tick] + phase).sin()
            }
            _ => 1.0,
        };
        let y = r.d * u + r.c.iter().zip(&x).map(|(c, x)| c * x).sum::<f64>();
        close(values[0], y);
        x =
            r.a.iter()
                .zip(&r.b)
                .map(|(row, b)| b * u + row.iter().zip(&x).map(|(a, x)| a * x).sum::<f64>())
                .collect();
    }
}
#[test]
fn synchronous_delay_feedback_has_exact_matrices_and_known_complex_unit_circle_modes() {
    for (radius, class, outside) in [
        (0.8, "stable", 0),
        (1.0, "boundary_or_uncertain", 0),
        (1.2, "unstable", 2),
    ] {
        let mut m = model();
        m.blocks[1].params = BlockKind::Sum {
            signs: vec![1.0, radius, -radius * radius],
        };
        let r = analyze_discrete_state_space(&m, &query()).unwrap();
        assert_eq!(r.a, vec![vec![radius, -radius * radius], vec![1.0, 0.0]]);
        assert_eq!(r.b, vec![1.0, 0.0]);
        assert_eq!(r.c, vec![1.0, 0.0]);
        assert_eq!(r.d, 0.0);
        assert_eq!(r.sample_time, Some(0.1));
        assert_eq!(r.stability.classification, class);
        assert_eq!(r.stability.outside_unit_circle_poles, Some(outside));
        assert_eq!(r.stability.right_half_plane_poles, None);
        assert_eq!(r.poles.status, "verified_residual");
        assert_eq!(r.poles.roots.len(), 2);
        for p in &r.poles.roots {
            close(p.real, radius / 2.0);
            close(p.imag.abs(), radius * 3.0_f64.sqrt() / 2.0);
        }
        matches_time(&m, &r);
    }
}
#[test]
fn sampled_pid_preserves_all_three_states_and_matches_actual_filtered_update_with_delay() {
    let mut m = model();
    m.blocks.remove(3);
    m.blocks[1].params = BlockKind::Sum {
        signs: vec![1.0, -1.0],
    };
    let mut pid = m.blocks[1].clone();
    pid.id = "pid".into();
    pid.label = "PID".into();
    let config = PidConfig {
        kp: 0.4,
        ki: 0.5,
        kd: 0.02,
        derivative_tau: 0.2,
        ..PidConfig::default()
    };
    pid.params = BlockKind::Pid {
        config,
        mode: PidMode::Discrete,
        sample_time: 0.1,
    };
    m.blocks.push(pid);
    m.edges.retain(|e| e.from != "delay2" && e.to != "delay2");
    m.edges.iter_mut().find(|e| e.from == "sum").unwrap().to = "pid".into();
    let mut edge = m.edges[0].clone();
    edge.from = "pid".into();
    edge.to = "delay1".into();
    m.edges.push(edge);
    m.blocks[0].params = BlockKind::Sine {
        amplitude: 0.7,
        frequency: 0.41,
        phase: 0.2,
        offset: 0.3,
    };
    let r = analyze_discrete_state_space(&m, &query()).unwrap();
    assert_eq!(r.check.states, 4);
    let alpha = 0.2 / 0.3;
    let beta = 1.0 / 0.3;
    let k = 0.4 + 0.05 + 0.02 * beta;
    let expected = [
        [-k, 1.0, -0.02 * beta, 0.02 * alpha],
        [-0.05, 1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0, 0.0],
        [-beta, 0.0, -beta, alpha],
    ];
    for (a, b) in r.a.iter().flatten().zip(expected.iter().flatten()) {
        close(*a, *b)
    }
    for (a, b) in r.b.iter().zip([k, 0.05, 1.0, beta]) {
        close(*a, b)
    }
    assert_eq!(r.c, vec![1.0, 0.0, 0.0, 0.0]);
    assert_eq!(r.d, 0.0);
    assert_eq!(r.stability.classification, "stable");
    assert!(r
        .states
        .iter()
        .any(|s| s.description == "previous PID error"));
    matches_time(&m, &r);
    let mut scope = m.blocks.iter().find(|b| b.id == "output").unwrap().clone();
    scope.id = "control".into();
    m.blocks.push(scope);
    let mut edge = m.edges[0].clone();
    edge.from = "pid".into();
    edge.to = "control".into();
    m.edges.push(edge);
    let mut control_query = query();
    control_query.scope = "control".into();
    let control = analyze_discrete_state_space(&m, &control_query).unwrap();
    close(control.d, k);
    matches_time(&m, &control);
    if let BlockKind::Pid { config, .. } =
        &mut m.blocks.iter_mut().find(|b| b.id == "pid").unwrap().params
    {
        config.ki = 0.0;
        config.kd = 0.0;
    }
    let r = analyze_discrete_state_space(&m, &query()).unwrap();
    assert_eq!(r.check.states, 4);
    assert_eq!(r.stability.classification, "boundary_or_uncertain");
    // The unchanged integral row analytically retains z=1. Tiny characteristic coefficients
    // can fail strict reconstruction: retain the boundary and report no partial coordinates.
    assert_eq!(r.a[1], vec![0.0, 1.0, 0.0, 0.0]);
    assert_eq!(r.poles.status, "unresolved");
    assert!(r.poles.roots.is_empty());
    assert_eq!(r.stability.outside_unit_circle_poles, None);
    matches_time(&m, &r);
}
fn scalar(gain: f64) -> GraphModel {
    let mut m = model();
    m.blocks.remove(3);
    m.blocks[1].params = BlockKind::Sum {
        signs: vec![1.0, gain],
    };
    m.edges.retain(|e| e.from != "delay2" && e.to != "delay2");
    m
}
#[test]
fn positive_negative_origin_and_near_unit_roots_are_not_continuous_half_plane_tests() {
    for (gain, class, count) in [
        (0.5, "stable", Some(0)),
        (-0.5, "stable", Some(0)),
        (1.0, "boundary_or_uncertain", Some(0)),
        (-1.0, "boundary_or_uncertain", Some(0)),
        (1.5, "unstable", Some(1)),
        (-1.5, "unstable", Some(1)),
        (0.0, "stable", Some(0)),
        (1.0 + 1e-12, "boundary_or_uncertain", None),
    ] {
        let r = analyze_discrete_state_space(&scalar(gain), &query()).unwrap();
        assert_eq!(r.stability.classification, class, "gain={gain}");
        assert_eq!(r.stability.outside_unit_circle_poles, count, "gain={gain}");
        close(r.poles.roots[0].real, gain);
    }
}
#[test]
fn mixed_rates_continuous_states_bounds_and_bad_queries_are_refused_before_analysis() {
    let mut m = model();
    if let BlockKind::Delay { sample_time, .. } = &mut m.blocks[3].params {
        *sample_time = 0.2;
    }
    assert!(validate_discrete_state_space(&m, &query())
        .unwrap_err()
        .contains("common sample"));
    m.blocks[3].params = BlockKind::FirstOrder {
        gain: 1.0,
        tau: 1.0,
        initial: vec![0.0],
    };
    assert!(validate_discrete_state_space(&m, &query())
        .unwrap_err()
        .contains("continuous states"));
    let mut q = query();
    q.source_component = 1;
    assert!(validate_discrete_state_space(&model(), &q).is_err());
    assert!(analyze_state_space(&model(), &query()).is_err());
    let mut m = model();
    m.blocks[1].params = BlockKind::Saturation {
        limits: blocksim_core::OutputLimits {
            lower: Some(0.0),
            upper: None,
        },
    };
    m.edges.retain(|e| e.to != "sum" || e.to_port == 0);
    assert!(validate_discrete_state_space(&m, &query()).is_err());
}
#[test]
fn unrelated_unstable_branch_zero_gain_and_static_direct_maps_retain_their_scope() {
    let mut m = scalar(1.5);
    m.blocks[1].params = BlockKind::Sum {
        signs: vec![0.0, 1.5],
    };
    let r = analyze_discrete_state_space(&m, &query()).unwrap();
    assert_eq!(r.b, vec![0.0]);
    assert_eq!(r.stability.classification, "unstable");
    m.blocks[2].params = BlockKind::Gain { gain: 3.0 };
    m.blocks[1].params = BlockKind::Gain { gain: 2.0 };
    m.edges.retain(|e| e.from != "delay1" || e.to != "sum");
    let r = analyze_discrete_state_space(&m, &query()).unwrap();
    assert_eq!(r.check.states, 0);
    assert_eq!(r.d, 6.0);
    assert_eq!(r.sample_time, Some(m.dt));
    assert_eq!(r.stability.classification, "stable");
    let mut hidden = scalar(2.0);
    for b in &mut hidden.blocks {
        b.id = format!("hidden_{}", b.id)
    }
    for e in &mut hidden.edges {
        e.from = format!("hidden_{}", e.from);
        e.to = format!("hidden_{}", e.to)
    }
    m.blocks.extend(hidden.blocks);
    m.edges.extend(hidden.edges);
    let r = analyze_discrete_state_space(&m, &query()).unwrap();
    assert_eq!(r.check.states, 0);
    assert_eq!(r.stability.classification, "stable");
}
#[test]
fn selected_vector_and_mux_selector_paths_keep_only_related_channel_states() {
    let mut m = model();
    for b in &mut m.blocks {
        b.width = 2;
        match &mut b.params {
            BlockKind::Step { before, after, .. } => {
                *before = vec![0.0, 0.0];
                *after = vec![1.0, 2.0]
            }
            BlockKind::Delay { initial, .. } => *initial = vec![0.0, 0.0],
            _ => (),
        }
    }
    let mut q = query();
    q.source_component = 1;
    q.scope_component = 1;
    let r = analyze_discrete_state_space(&m, &q).unwrap();
    assert_eq!(r.check.states, 2);
    assert!(r.states.iter().all(|s| s.component == 1));
    assert_eq!(r.b, vec![1.0, 0.0]);
    q.source_component = 0;
    let r = analyze_discrete_state_space(&m, &q).unwrap();
    assert!(!r.check.source_related);
    assert_eq!(r.b, vec![0.0, 0.0]);
    // Scope selects channel1 through the explicit Selector/Mux mapping.
    m.blocks[4].width = 1;
    let mut mux = m.blocks[4].clone();
    mux.id = "mux".into();
    mux.width = 2;
    mux.params = BlockKind::Mux { inputs: 1 };
    let mut select = mux.clone();
    select.id = "select".into();
    select.width = 1;
    select.params = BlockKind::Selector { indices: vec![1] };
    m.blocks.extend([mux, select]);
    m.edges.last_mut().unwrap().to = "mux".into();
    let mut edge = m.edges[0].clone();
    edge.from = "mux".into();
    edge.to = "select".into();
    m.edges.push(edge.clone());
    edge.from = "select".into();
    edge.to = "output".into();
    m.edges.push(edge);
    q.scope_component = 0;
    q.source_component = 1;
    let r = analyze_discrete_state_space(&m, &q).unwrap();
    assert!(r.states.iter().all(|s| s.component == 1));
    assert_eq!(r.b, vec![1.0, 0.0]);
}
#[test]
fn eight_synchronous_delay_states_keep_known_zero_poles_and_nine_are_refused() {
    let mut m = scalar(0.0);
    m.edges.retain(|e| e.to != "sum" || e.to_port == 0);
    m.blocks[1].params = BlockKind::Gain { gain: 1.0 };
    for count in 2..=9 {
        let mut delay = m.blocks[2].clone();
        delay.id = format!("delay{count}");
        m.blocks.push(delay);
        let mut edge = m.edges[0].clone();
        edge.from = format!("delay{}", count - 1);
        edge.to = format!("delay{count}");
        m.edges.push(edge);
        m.edges.iter_mut().find(|e| e.to == "output").unwrap().from = format!("delay{count}");
        if count <= 8 {
            let r = analyze_discrete_state_space(&m, &query()).unwrap();
            assert_eq!(r.check.states, count);
            assert_eq!(r.poles.roots.len(), count);
            assert!(r.poles.roots.iter().all(|p| p.real == 0.0 && p.imag == 0.0));
            assert_eq!(r.stability.classification, "stable");
        } else {
            assert!(validate_discrete_state_space(&m, &query())
                .unwrap_err()
                .contains("8 related discrete"));
        }
    }
}
#[test]
fn extreme_polynomial_scales_keep_finite_matrices_and_unknown_unit_circle_counts() {
    for gain in [1e308, 1e-308] {
        let r = analyze_discrete_state_space(&scalar(gain), &query()).unwrap();
        assert_eq!(r.a, vec![vec![gain]]);
        assert_eq!(r.stability.classification, "indeterminate");
        assert_eq!(r.stability.outside_unit_circle_poles, None);
        assert!(r
            .a
            .iter()
            .flatten()
            .chain(&r.b)
            .chain(&r.c)
            .all(|v| v.is_finite()));
        assert!(r.d.is_finite());
        assert!(r.stability.notes.iter().any(|s| s.contains("resolution")));
    }
}
#[test]
fn nested_discrete_realizations_preserve_hierarchical_ids_and_boundary_outputs() {
    let mut inner = model();
    inner.blocks.last_mut().unwrap().params = BlockKind::Output { port: 0 };
    let mut m = model();
    m.blocks[1].params = BlockKind::Subsystem {
        model: Box::new(inner),
    };
    m.blocks
        .retain(|b| b.id == "source" || b.id == "sum" || b.id == "output");
    m.edges.retain(|e| e.from == "source");
    let mut edge = m.edges[0].clone();
    edge.from = "sum".into();
    edge.to = "output".into();
    m.edges.push(edge);
    // No Input boundary: the subsystem's independent source is the requested input.
    m.edges.retain(|e| e.from != "source");
    let mut q = query();
    q.source = "sum/source".into();
    let r = analyze_discrete_state_space(&m, &q).unwrap();
    assert_eq!(r.check.states, 2);
    assert!(r.states.iter().all(|s| s.block.starts_with("sum/delay")));
    assert_eq!(r.a, vec![vec![0.8, -0.64], vec![1.0, 0.0]]);
    assert_eq!(r.b, vec![1.0, 0.0]);
}
#[test]
fn coupled_real_boundary_mode_keeps_an_additional_unstable_mode() {
    let mut m = model();
    m.blocks[1].params = BlockKind::Sum {
        signs: vec![1.0, -1.0],
    };
    m.edges.retain(|e| e.to != "sum" || e.to_port != 2);
    let mut sum = m.blocks[1].clone();
    sum.id = "second_sum".into();
    sum.params = BlockKind::Sum {
        signs: vec![1.0, 2.0],
    };
    m.blocks.push(sum);
    m.edges.iter_mut().find(|e| e.to == "delay2").unwrap().to = "second_sum".into();
    let mut edge = m.edges[0].clone();
    edge.from = "delay2".into();
    edge.to = "second_sum".into();
    edge.to_port = 1;
    m.edges.push(edge.clone());
    edge.from = "second_sum".into();
    edge.to = "delay2".into();
    edge.to_port = 0;
    m.edges.push(edge);
    m.edges.iter_mut().find(|e| e.to == "output").unwrap().from = "delay2".into();
    let r = analyze_discrete_state_space(&m, &query()).unwrap();
    assert_eq!(
        r.stability.classification,
        "unstable",
        "{}",
        serde_json::to_string_pretty(&r).unwrap()
    );
    assert_eq!(r.stability.outside_unit_circle_poles, Some(1));
    assert!(r.poles.roots.iter().any(|p| (p.real + 1.0).abs() < 1e-10));
    assert!(r.poles.roots.iter().any(|p| (p.real - 2.0).abs() < 1e-10));
}
