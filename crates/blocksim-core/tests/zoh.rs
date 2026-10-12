use blocksim_core::graph::{
    discretize::{discretize_zoh, ZohRequest, ZohResult},
    simulate_graph,
    state_space::{analyze_discrete_state_space, StateSpaceRequest},
    BlockKind, GraphModel, SolverConfig,
};
fn q(n: Vec<f64>, d: Vec<f64>, ts: f64) -> ZohRequest {
    ZohRequest {
        numerator: n,
        denominator: d,
        sample_time: ts,
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-10 * (1.0 + b.abs()), "{a} != {b}");
}
fn model() -> GraphModel {
    serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json")).unwrap()
}
fn converted(m: &mut GraphModel, r: &ZohResult) {
    m.blocks[1].params = BlockKind::DiscreteTransferFunction {
        numerator: r.numerator.clone(),
        denominator: r.denominator.clone(),
        initial: vec![0.0; m.blocks[1].width * (r.denominator.len() - 1)],
        sample_time: r.request.sample_time,
    };
}
#[test]
fn first_order_static_and_integrator_match_exact_held_input_formulas() {
    for ts in [0.01, 0.1, 1.0, 3.0] {
        let r = discretize_zoh(&q(vec![1.0], vec![1.0, 1.0], ts)).unwrap();
        close(r.a[0][0], (-ts).exp());
        close(r.b[0], -(-ts).exp_m1());
        assert_eq!(r.numerator[0], 0.0);
        close(r.numerator[1], 1.0 - (-ts).exp());
        close(r.denominator[1], -(-ts).exp());
    }
    let r = discretize_zoh(&q(vec![1.0], vec![1.0, 0.0], 0.1)).unwrap();
    assert_eq!(r.denominator, vec![1.0, -1.0]);
    assert_eq!(r.numerator, vec![0.0, 0.1]);
    let r = discretize_zoh(&q(vec![6.0], vec![2.0], 0.1)).unwrap();
    assert_eq!(r.numerator, vec![3.0]);
    assert_eq!(r.denominator, vec![1.0]);
    assert!(r.a.is_empty());
}
#[test]
fn repeated_second_order_poles_match_independent_matrix_and_step_formulas() {
    for ts in [0.1, 0.2, 0.5] {
        let r = discretize_zoh(&q(vec![1.0], vec![1.0, 2.0, 1.0], ts)).unwrap();
        let e = (-ts).exp();
        for (a, b) in
            r.a.iter()
                .flatten()
                .zip([e * (1.0 - ts), -e * ts, e * ts, e * (1.0 + ts)])
        {
            close(*a, b);
        }
        close(r.b[0], ts * e);
        close(r.b[1], 1.0 - (1.0 + ts) * e);
        close(r.denominator[1], -2.0 * e);
        close(r.denominator[2], e * e);
        close(r.numerator[1], 1.0 - (1.0 + ts) * e);
        close(r.numerator[2], e * e + (ts - 1.0) * e);
        let mut m = model();
        converted(&mut m, &r);
        let time = simulate_graph(&m).unwrap();
        for (tick, row) in time.traces[0].values.iter().enumerate() {
            let t = (tick / ((ts / m.dt).round() as usize)) as f64 * ts;
            close(row[0], 1.0 - (1.0 + t) * (-t).exp());
        }
        assert!(r.cayley_hamilton_residual < 1e-10 && r.reconstruction_residual < 1e-9);
    }
}
#[test]
fn integrator_chain_and_undamped_oscillator_do_not_require_inverting_a() {
    let r = discretize_zoh(&q(vec![1.0], vec![1.0, 0.0, 0.0], 0.1)).unwrap();
    for (a, b) in r.numerator.iter().zip([0.0, 0.005, 0.005]) {
        close(*a, b);
    }
    assert_eq!(r.denominator, vec![1.0, -2.0, 1.0]);
    let r = discretize_zoh(&q(vec![4.0], vec![1.0, 0.0, 4.0], 0.1)).unwrap();
    let theta = 0.2_f64;
    for (a, b) in r.a.iter().flatten().zip([
        theta.cos(),
        -2.0 * theta.sin(),
        theta.sin() / 2.0,
        theta.cos(),
    ]) {
        close(*a, b);
    }
    close(r.b[0], theta.sin() / 2.0);
    close(r.b[1], (1.0 - theta.cos()) / 4.0);
}
#[test]
fn cancelled_stable_boundary_and_unstable_modes_are_retained() {
    for (p, class) in [
        (-1.0, "stable"),
        (0.0, "boundary_or_uncertain"),
        (1.0, "unstable"),
    ] {
        let r = discretize_zoh(&q(vec![1.0, -p], vec![1.0, -p], 0.1)).unwrap();
        close(r.a[0][0], (p * 0.1_f64).exp());
        let mut m = model();
        converted(&mut m, &r);
        let ss = analyze_discrete_state_space(
            &m,
            &StateSpaceRequest {
                source: "source".into(),
                scope: "output".into(),
                source_component: 0,
                scope_component: 0,
            },
        )
        .unwrap();
        assert_eq!(ss.check.states, 1);
        assert_eq!(ss.stability.classification, class);
        close(ss.d, 1.0);
        close(ss.c[0], 0.0);
    }
}
#[test]
fn held_sine_reference_continuous_rk45_matches_discrete_samples_with_independent_vectors() {
    let r = discretize_zoh(&q(vec![1.0], vec![1.0, 2.0, 1.0], 0.1)).unwrap();
    let mut m = model();
    m.blocks[0].params = BlockKind::Sine {
        amplitude: 1.0,
        frequency: 0.7,
        phase: 0.3,
        offset: 0.2,
    };
    let mut held = m.blocks[1].clone();
    held.id = "held".into();
    held.params = BlockKind::DiscreteTransferFunction {
        numerator: vec![1.0],
        denominator: vec![1.0],
        initial: vec![],
        sample_time: 0.1,
    };
    m.blocks.push(held);
    m.edges[0].from = "held".into();
    let mut e = m.edges[0].clone();
    e.from = "source".into();
    e.to = "held".into();
    m.edges.push(e);
    m.solver = Some(SolverConfig::Rk45 {
        abs_tolerance: 1e-11,
        rel_tolerance: 1e-11,
        min_step: 1e-12,
        max_step: 0.5,
    });
    let analog = simulate_graph(&m).unwrap();
    converted(&mut m, &r);
    let digital = simulate_graph(&m).unwrap();
    for tick in (0..analog.times.len()).step_by(10) {
        close(
            analog.traces[0].values[tick][0],
            digital.traces[0].values[tick][0],
        );
    }
    for b in &mut m.blocks {
        b.width = 2;
        if let BlockKind::DiscreteTransferFunction {
            initial,
            denominator,
            ..
        } = &mut b.params
        {
            *initial = vec![0.0; 2 * (denominator.len() - 1)];
        }
    }
    m.blocks[0].params = BlockKind::Constant {
        value: vec![1.0, 2.0],
    };
    let time = simulate_graph(&m).unwrap();
    for row in &time.traces[0].values {
        close(row[1], 2.0 * row[0]);
    }
}
#[test]
fn nonmonic_coefficients_and_existing_direct_feedthrough_are_preserved() {
    let a = discretize_zoh(&q(vec![1.0, 2.0, 3.0], vec![1.0, 2.0, 1.0], 0.2)).unwrap();
    let b = discretize_zoh(&q(
        vec![1e300, 2e300, 3e300],
        vec![1e300, 2e300, 1e300],
        0.2,
    ))
    .unwrap();
    for (x, y) in a
        .numerator
        .iter()
        .chain(&a.denominator)
        .zip(b.numerator.iter().chain(&b.denominator))
    {
        close(*x, *y);
    }
    assert_eq!(a.numerator[0], 1.0);
    let mut m = model();
    converted(&mut m, &a);
    let time = simulate_graph(&m).unwrap();
    for (tick, row) in time.traces[0].values.iter().enumerate() {
        let t = (tick / 20) as f64 * 0.2;
        close(row[0], 3.0 - 2.0 * (1.0 + t) * (-t).exp());
    }
}
#[test]
fn fourth_and_eighth_order_repeated_poles_match_exact_sampled_step() {
    for n in [4, 8] {
        let mut den = vec![1.0];
        for _ in 0..n {
            let mut next = vec![0.0; den.len() + 1];
            for (i, &v) in den.iter().enumerate() {
                next[i] += v;
                next[i + 1] += v;
            }
            den = next;
        }
        let r = discretize_zoh(&q(vec![1.0], den, 0.5)).unwrap();
        assert_eq!(r.denominator.len(), n + 1);
        let mut m = model();
        m.duration = 5.0;
        converted(&mut m, &r);
        let time = simulate_graph(&m).unwrap();
        for (tick, row) in time.traces[0].values.iter().enumerate() {
            let t = (tick / 50) as f64 * 0.5;
            let (mut sum, mut term) = (1.0, 1.0);
            for k in 1..n {
                term *= t / k as f64;
                sum += term;
            }
            close(row[0], 1.0 - (-t).exp() * sum);
        }
    }
}
#[test]
fn zoh_preserves_strictly_proper_feedback_without_creating_a_direct_loop() {
    let r = discretize_zoh(&q(vec![1.0], vec![1.0, 1.0], 0.1)).unwrap();
    let mut m = model();
    converted(&mut m, &r);
    m.edges[0].from = "plant".into();
    assert!(simulate_graph(&m).unwrap().traces[0]
        .values
        .iter()
        .all(|row| row[0] == 0.0));
}
#[test]
fn invalid_or_unrepresentable_exponentials_are_refused() {
    assert!(discretize_zoh(&q(vec![1e-308], vec![1e308], 0.1))
        .unwrap_err()
        .contains("normalization underflow"));
    for ts in [0.0, -1.0, f64::INFINITY] {
        assert!(discretize_zoh(&q(vec![1.0], vec![1.0, 1.0], ts)).is_err());
    }
    assert!(discretize_zoh(&q(vec![1.0], vec![1.0, -1.0], 1000.0)).is_err());
    assert!(discretize_zoh(&q(vec![1.0], vec![1.0, 1e20], 1.0))
        .unwrap_err()
        .contains("24 squarings"));
    for (n, d) in [
        (vec![], vec![1.0]),
        (vec![1.0, 2.0], vec![1.0]),
        (vec![1.0], vec![0.0, 1.0]),
        (vec![1.0], vec![1.0; 10]),
    ] {
        assert!(discretize_zoh(&q(n, d, 0.1)).is_err());
    }
}
