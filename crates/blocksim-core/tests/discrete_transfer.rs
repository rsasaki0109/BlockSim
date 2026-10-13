use blocksim_core::graph::{
    frequency::{analyze_discrete_frequency, FrequencyRequest},
    simulate_graph,
    state_space::{analyze_discrete_state_space, StateSpaceRequest},
    validate_graph, BlockKind, GraphModel, SolverConfig,
};
fn model() -> GraphModel {
    serde_json::from_str(include_str!("../../../samples/discrete/transfer.json")).unwrap()
}
fn close(a: f64, b: f64, tolerance: f64) {
    assert!((a - b).abs() < tolerance * (1.0 + b.abs()), "{a} != {b}");
}
fn set(m: &mut GraphModel, n: Vec<f64>, d: Vec<f64>, x: Vec<f64>, ts: f64) {
    m.blocks[1].params = BlockKind::DiscreteTransferFunction {
        numerator: n,
        denominator: d,
        initial: x,
        sample_time: ts,
    };
}
fn fq() -> FrequencyRequest {
    FrequencyRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
        min_omega: 0.01,
        max_omega: std::f64::consts::PI / 0.1,
        points: 201,
        open_loop: false,
    }
}
fn sq() -> StateSpaceRequest {
    StateSpaceRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
    }
}
#[test]
fn direct_and_delayed_step_match_closed_form_and_nonmonic_scaling() {
    let mut m = model();
    let r = simulate_graph(&m).unwrap();
    for (tick, (direct, delayed)) in r.traces[0]
        .values
        .iter()
        .zip(&r.traces[1].values)
        .enumerate()
    {
        let n = (tick / 5) as i32;
        close(direct[0], 1.0 - 0.8_f64.powi(n + 1), 1e-14);
        close(delayed[0], 1.0 - 0.8_f64.powi(n), 1e-14);
    }
    set(&mut m, vec![0.4, 0.0], vec![2.0, -1.6], vec![0.0], 0.1);
    assert_eq!(
        r.traces[0].values,
        simulate_graph(&m).unwrap().traces[0].values
    );
}
#[test]
fn fir_matches_independent_sampled_input_convolution() {
    let mut m = model();
    m.blocks[0].params = BlockKind::Sine {
        amplitude: 2.0,
        frequency: 0.7,
        phase: 0.4,
        offset: -0.2,
    };
    set(
        &mut m,
        vec![0.25, 0.5, 0.25],
        vec![1.0, 0.0, 0.0],
        vec![0.0, 0.0],
        0.1,
    );
    let r = simulate_graph(&m).unwrap();
    for (tick, row) in r.traces[0].values.iter().enumerate() {
        let n = (tick / 5) as i32;
        let u = |k: i32| {
            if k < 0 {
                0.0
            } else {
                -0.2 + 2.0 * (std::f64::consts::TAU * 0.7 * (k as f64 * 0.1) + 0.4).sin()
            }
        };
        close(
            row[0],
            0.25 * u(n) + 0.5 * u(n - 1) + 0.25 * u(n - 2),
            1e-14,
        );
    }
}
#[test]
fn vector_nonzero_companion_initial_states_are_independent() {
    let mut m = model();
    for b in &mut m.blocks {
        b.width = 2;
    }
    m.blocks[0].params = BlockKind::Constant {
        value: vec![0.0, 0.0],
    };
    set(&mut m, vec![0.2], vec![1.0, -0.8], vec![1.0, -2.0], 0.1);
    if let BlockKind::DiscreteTransferFunction { initial, .. } = &mut m.blocks[2].params {
        *initial = vec![0.0, 0.0];
    }
    let r = simulate_graph(&m).unwrap();
    for (tick, row) in r.traces[0].values.iter().enumerate() {
        let n = (tick / 5) as i32;
        close(row[0], 0.2 * 0.8_f64.powi(n), 1e-14);
        close(row[1], -0.4 * 0.8_f64.powi(n), 1e-14);
    }
}
#[test]
fn synchronous_cascade_is_block_order_independent_and_supports_different_clocks() {
    let mut m = model();
    set(&mut m, vec![1.0], vec![1.0, 0.0], vec![0.0], 0.1);
    m.blocks[2].params = BlockKind::DiscreteTransferFunction {
        numerator: vec![1.0],
        denominator: vec![1.0, 0.0],
        initial: vec![0.0],
        sample_time: 0.2,
    };
    m.edges[1].from = "direct".into();
    let r = simulate_graph(&m).unwrap();
    assert!(r.traces[1]
        .values
        .iter()
        .enumerate()
        .all(|(tick, row)| row[0] == if tick < 20 { 0.0 } else { 1.0 }));
    m.blocks.reverse();
    m.edges.reverse();
    assert_eq!(
        r.traces[1].values,
        simulate_graph(&m).unwrap().traces[0].values
    );
    assert!(analyze_discrete_state_space(
        &m,
        &StateSpaceRequest {
            scope: "delayed-output".into(),
            ..sq()
        }
    )
    .unwrap_err()
    .contains("mixed rates"));
}
#[test]
fn strictly_proper_feedback_breaks_loop_but_direct_feedthrough_rejects_it() {
    let mut m = model();
    set(&mut m, vec![0.5], vec![1.0, -0.5], vec![2.0], 0.1);
    m.edges[0].from = "direct".into();
    let r = simulate_graph(&m).unwrap();
    assert!(r.traces[0].values.iter().all(|row| row[0] == 1.0));
    set(&mut m, vec![0.5, 0.0], vec![1.0, -0.5], vec![2.0], 0.1);
    assert!(validate_graph(&m).unwrap_err().contains("Algebraic loop"));
}
#[test]
fn state_space_bode_and_cancelled_unstable_modes_use_actual_sample_realization() {
    let mut m = model();
    let r = analyze_discrete_state_space(&m, &sq()).unwrap();
    assert_eq!(r.a, vec![vec![0.8]]);
    assert_eq!(r.b, vec![1.0]);
    close(r.c[0], 0.16, 1e-14);
    assert_eq!(r.d, 0.2);
    assert_eq!(r.sample_time, Some(0.1));
    let f = analyze_discrete_frequency(&m, &fq()).unwrap();
    for p in f.response.points {
        let a = p.omega * 0.1;
        let re = a.cos() - 0.8;
        let im = a.sin();
        let q = re * re + im * im;
        close(p.real.unwrap(), 0.2 + 0.16 * re / q, 1e-13);
        close(p.imag.unwrap(), -0.16 * im / q, 1e-13);
    }
    set(&mut m, vec![1.0, -1.1], vec![1.0, -1.1], vec![3.0], 0.1);
    let r = analyze_discrete_state_space(&m, &sq()).unwrap();
    assert_eq!(r.stability.classification, "unstable");
    assert_eq!(r.stability.outside_unit_circle_poles, Some(1));
    assert_eq!(r.c, vec![0.0]);
    assert_eq!(r.check.states, 1);
    assert!(simulate_graph(&m).unwrap().traces[0]
        .values
        .iter()
        .all(|row| row[0] == 1.0));
}
#[test]
fn zero_order_static_gain_is_sampled_and_has_no_internal_states() {
    let mut m = model();
    m.blocks[0].params = BlockKind::Sine {
        amplitude: 1.0,
        frequency: 1.0,
        phase: 0.0,
        offset: 0.0,
    };
    set(&mut m, vec![6.0], vec![2.0], vec![], 0.1);
    let r = simulate_graph(&m).unwrap();
    for (tick, row) in r.traces[0].values.iter().enumerate() {
        close(
            row[0],
            3.0 * (std::f64::consts::TAU * (tick / 5) as f64 * 0.1).sin(),
            2e-14,
        );
    }
    assert_eq!(
        analyze_discrete_state_space(&m, &sq())
            .unwrap()
            .check
            .states,
        0
    );
}
#[test]
fn hybrid_rk45_rejections_do_not_advance_sampled_states_or_sample_off_clock_steps() {
    let mut m = model();
    m.blocks[0].params = BlockKind::Step {
        before: vec![0.0],
        after: vec![1.0],
        step_time: 0.13,
    };
    m.blocks[2].params = BlockKind::FirstOrder {
        gain: 1.0,
        tau: 0.05,
        initial: vec![0.0],
    };
    m.edges[1].from = "direct".into();
    m.solver = Some(SolverConfig::Rk45 {
        abs_tolerance: 1e-12,
        rel_tolerance: 1e-12,
        min_step: 1e-8,
        max_step: 0.2,
    });
    let r = simulate_graph(&m).unwrap();
    assert!(r.statistics.as_ref().unwrap().rejected_steps > 0);
    let mut expected = 0.0;
    let mut held = 0.0;
    for (tick, row) in r.traces[1].values.iter().enumerate() {
        if tick > 0 {
            expected = held + (expected - held) * (-m.dt / 0.05).exp();
        }
        close(row[0], expected, 1e-10);
        let n = tick / 5;
        held = if n < 2 {
            0.0
        } else {
            1.0 - 0.8_f64.powi(n as i32 - 1)
        };
        close(r.traces[0].values[tick][0], held, 1e-14);
    }
}
#[test]
fn invalid_coefficients_states_period_and_numerical_overflow_are_explicit() {
    for (n, d, x, t) in [
        (vec![], vec![1.0], vec![], 0.1),
        (vec![1.0, 2.0], vec![1.0], vec![], 0.1),
        (vec![1.0], vec![0.0, 1.0], vec![0.0], 0.1),
        (vec![1.0], vec![1.0; 10], vec![0.0; 9], 0.1),
        (vec![1.0], vec![1.0, -0.8], vec![], 0.1),
        (vec![1.0], vec![1.0], vec![], 0.03),
        (vec![f64::INFINITY], vec![1.0], vec![], 0.1),
        (vec![1e308], vec![1e-308], vec![], 0.1),
    ] {
        let mut m = model();
        set(&mut m, n, d, x, t);
        assert!(validate_graph(&m).is_err());
    }
    let mut m = model();
    set(&mut m, vec![0.0], vec![1.0, -1e308], vec![1e308], 0.1);
    assert!(simulate_graph(&m)
        .unwrap_err()
        .contains("next sample state"));
}
#[test]
fn compact_second_order_transfer_matches_existing_delay_loop_bode_and_margins() {
    let original: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/open-loop.json")).unwrap();
    let mut m = original.clone();
    m.blocks
        .retain(|b| ["source", "gain", "output"].contains(&b.id.as_str()));
    m.blocks[1].params = BlockKind::DiscreteTransferFunction {
        numerator: vec![0.75],
        denominator: vec![1.0, -0.5, 0.0],
        initial: vec![0.0, 0.0],
        sample_time: 0.1,
    };
    m.edges.retain(|e| e.from == "source");
    let mut e = m.edges[0].clone();
    e.from = "gain".into();
    e.to = "output".into();
    m.edges.push(e);
    let mut q = fq();
    q.open_loop = true;
    let a = analyze_discrete_frequency(&original, &q).unwrap();
    let b = analyze_discrete_frequency(&m, &q).unwrap();
    for (x, y) in a.response.points.iter().zip(&b.response.points) {
        close(x.real.unwrap(), y.real.unwrap(), 1e-13);
        close(x.imag.unwrap(), y.imag.unwrap(), 1e-13);
    }
    let a = a.response.margins.unwrap();
    let b = b.response.margins.unwrap();
    close(
        a.gain_crossings[0].phase_margin_deg,
        b.gain_crossings[0].phase_margin_deg,
        1e-10,
    );
    close(
        a.phase_crossings[0].gain_margin_db,
        b.phase_crossings[0].gain_margin_db,
        1e-10,
    );
    assert_eq!(
        simulate_graph(&original).unwrap().traces[0].values,
        simulate_graph(&m).unwrap().traces[0].values
    );
}
#[test]
fn eighth_order_delay_keeps_origin_states_and_seventeenth_related_state_is_rejected() {
    let mut m = model();
    set(
        &mut m,
        vec![1.0],
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0; 8],
        0.1,
    );
    let r = simulate_graph(&m).unwrap();
    assert!(r.traces[0]
        .values
        .iter()
        .enumerate()
        .all(|(tick, row)| row[0] == if tick < 40 { 0.0 } else { 1.0 }));
    let ss = analyze_discrete_state_space(&m, &sq()).unwrap();
    assert_eq!(ss.check.states, 8);
    assert_eq!(ss.stability.classification, "stable");
    assert_eq!(ss.poles.roots.len(), 8);
    assert!(ss
        .poles
        .roots
        .iter()
        .all(|p| p.real == 0.0 && p.imag == 0.0));
    m.blocks[2].params = BlockKind::Delay {
        initial: vec![0.0],
        sample_time: 0.1,
    };
    m.edges[1].from = "direct".into();
    let query = StateSpaceRequest {
        scope: "delayed-output".into(),
        ..sq()
    };
    assert_eq!(
        analyze_discrete_state_space(&m, &query)
            .unwrap()
            .check
            .states,
        9
    );
    let mut extra = m.blocks[1].clone();
    extra.id = "extra-transfer".into();
    m.blocks.push(extra);
    m.edges[1].from = "extra-transfer".into();
    let mut edge = m.edges[1].clone();
    edge.from = "direct".into();
    edge.to = "extra-transfer".into();
    m.edges.push(edge);
    assert!(analyze_discrete_state_space(&m, &query)
        .unwrap_err()
        .contains("at most 16"));
}
