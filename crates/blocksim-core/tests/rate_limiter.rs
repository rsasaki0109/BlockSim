use blocksim_core::graph::{
    frequency::{validate_frequency, FrequencyRequest},
    simulate_graph,
    state_space::{validate_discrete_state_space, validate_state_space, StateSpaceRequest},
    validate_graph, Block, BlockKind, Edge, GraphModel, GraphResult, SolverConfig,
};
fn sample() -> GraphModel {
    serde_json::from_str(include_str!("../../../samples/graphs/rate-limiter.json")).unwrap()
}
fn output(r: &GraphResult, id: &str) -> Vec<Vec<f64>> {
    r.traces.iter().find(|t| t.id == id).unwrap().values.clone()
}
fn expected(n: usize) -> Vec<f64> {
    if n < 12 {
        vec![(n as f64 * 0.25).min(1.), (-(n as f64) * 0.125).max(-1.)]
    } else {
        vec![
            (1. - (n - 11) as f64 * 0.125).max(-1.),
            (-1. + (n - 11) as f64 * 0.25).min(1.),
        ]
    }
}
fn adaptive(m: &mut GraphModel) {
    m.solver = Some(SolverConfig::Rk45 {
        abs_tolerance: 1e-11,
        rel_tolerance: 1e-10,
        min_step: 1e-10,
        max_step: 0.5,
    });
}
fn edge(from: &str, to: &str) -> Edge {
    Edge {
        from: from.into(),
        from_port: 0,
        to: to.into(),
        to_port: 0,
    }
}
#[test]
fn vector_sample_has_initial_output_asymmetric_rates_and_exact_holds_with_both_solvers() {
    for use_adaptive in [false, true] {
        let mut m = sample();
        if use_adaptive {
            adaptive(&mut m);
        }
        let r = simulate_graph(&m).unwrap();
        assert_eq!(r.times.len(), 151);
        let values = output(&r, "output");
        let raw = output(&r, "raw");
        for k in 0..151 {
            assert_eq!(values[k], expected(k / 5));
            assert_eq!(raw[k], if k < 60 { vec![1., -1.] } else { vec![-1., 1.] });
        }
        for n in 1..=30 {
            for j in 0..2 {
                let delta = values[n * 5][j] - values[(n - 1) * 5][j];
                assert!((-0.125..=0.25).contains(&delta));
            }
        }
        assert_eq!(values[95][1], 1.);
        assert_eq!(values[135][0], -1.);
        let stored = serde_json::to_string(&m).unwrap();
        let restored: GraphModel = serde_json::from_str(&stored).unwrap();
        assert_eq!(
            serde_json::to_string(&simulate_graph(&restored).unwrap()).unwrap(),
            serde_json::to_string(&r).unwrap()
        );
    }
}
#[test]
fn zero_rates_freeze_only_the_requested_direction_and_initial_vector_is_not_consumed_at_zero() {
    for (rise, fall, initial, last) in [
        (0., 1., vec![0., 0.], vec![-1., -1.]),
        (2., 0., vec![0., 0.], vec![1., 1.]),
        (0., 0., vec![0.3, -0.7], vec![0.3, -0.7]),
    ] {
        let mut m = sample();
        m.blocks[1].params = BlockKind::DiscreteRateLimiter {
            rising_rate: rise,
            falling_rate: fall,
            initial: initial.clone(),
            sample_time: 0.125,
        };
        let r = simulate_graph(&m).unwrap();
        let out = output(&r, "output");
        assert_eq!(out[0], initial);
        assert_eq!(out.last().unwrap(), &last);
        if rise == 0. && fall == 0. {
            assert!(out.iter().all(|row| row == &initial));
        }
    }
}
#[test]
fn direct_sampled_chain_and_synchronous_delay_use_current_and_previous_samples_independent_of_block_order(
) {
    let mut m = sample();
    let second = Block {
        id: "second".into(),
        label: "second".into(),
        ..m.blocks[1].clone()
    };
    m.blocks.push(second);
    m.edges[1].from = "second".into();
    m.edges.push(edge("limiter", "second"));
    m.blocks.push(Block {
        id: "delay".into(),
        label: "delay".into(),
        width: 2,
        position: [0., 0.],
        params: BlockKind::Delay {
            initial: vec![3., -3.],
            sample_time: 0.125,
        },
    });
    m.blocks.push(Block {
        id: "delayed".into(),
        label: "delayed".into(),
        width: 2,
        position: [0., 0.],
        params: BlockKind::Scope {},
    });
    m.edges
        .extend([edge("second", "delay"), edge("delay", "delayed")]);
    for reversed in [false, true] {
        if reversed {
            m.blocks.reverse();
            m.edges.reverse();
        }
        let r = simulate_graph(&m).unwrap();
        let out = output(&r, "output");
        let delay = output(&r, "delayed");
        for k in 0..151 {
            assert_eq!(out[k], expected(k / 5));
            assert_eq!(
                delay[k],
                if k < 5 {
                    vec![3., -3.]
                } else {
                    expected(k / 5 - 1)
                }
            );
        }
    }
}
#[test]
fn finite_nonnegative_rates_representable_increments_widths_and_integer_periods_are_required() {
    for (rise, fall, ts, initial) in [
        (-1., 1., 0.125, vec![0., 0.]),
        (1., -1., 0.125, vec![0., 0.]),
        (f64::NAN, 1., 0.125, vec![0., 0.]),
        (1., f64::INFINITY, 0.125, vec![0., 0.]),
        (1e308, 1., 10., vec![0., 0.]),
        (1., 1., 0., vec![0., 0.]),
        (1., 1., 0.13, vec![0., 0.]),
        (1., 1., 0.125, vec![0.]),
        (1., 1., 0.125, vec![0., f64::NAN]),
    ] {
        let mut m = sample();
        m.blocks[1].params = BlockKind::DiscreteRateLimiter {
            rising_rate: rise,
            falling_rate: fall,
            initial,
            sample_time: ts,
        };
        assert!(validate_graph(&m).is_err());
    }
    let mut m = sample();
    m.dt = 0.5;
    m.duration = 1.;
    m.blocks[1].params = BlockKind::DiscreteRateLimiter {
        rising_rate: f64::from_bits(1),
        falling_rate: 1.,
        initial: vec![0., 0.],
        sample_time: 0.5,
    };
    assert!(validate_graph(&m).unwrap_err().contains("representable"));
    let mut m = sample();
    m.edges[0].from = "limiter".into();
    assert!(validate_graph(&m).unwrap_err().contains("Algebraic loop"));
}
#[test]
fn extreme_finite_inputs_do_not_overflow_candidate_bounds_or_opposite_sign_distances() {
    for (input, initial, expected) in [
        (vec![1e308, -1e308], vec![-1e308, 1e308], vec![0., 0.]),
        (
            vec![1.1e308, -1.1e308],
            vec![1e308, -1e308],
            vec![1.1e308, -1.1e308],
        ),
    ] {
        let mut m = sample();
        m.dt = 0.25;
        m.duration = 2.;
        m.blocks[0].params = BlockKind::Constant {
            value: input.clone(),
        };
        m.blocks[1].params = BlockKind::DiscreteRateLimiter {
            rising_rate: 1e308,
            falling_rate: 1e308,
            initial: initial.clone(),
            sample_time: 1.,
        };
        let r = simulate_graph(&m).unwrap();
        let out = output(&r, "output");
        assert_eq!(out[0], initial);
        assert_eq!(out[4], expected);
        assert_eq!(out[8], input);
    }
}
#[test]
fn mixed_source_and_limiter_periods_read_current_input_and_preserve_every_intervening_hold() {
    let rows = vec![vec![1., -1.], vec![-1., 1.], vec![0.125, -0.125]];
    for use_adaptive in [false, true] {
        let mut m = sample();
        m.blocks[0].params = BlockKind::SampledInput {
            values: rows.clone(),
            sample_time: 0.075,
            repeat: true,
        };
        m.blocks[1].params = BlockKind::DiscreteRateLimiter {
            rising_rate: 20.,
            falling_rate: 20.,
            initial: vec![0., 0.],
            sample_time: 0.125,
        };
        if use_adaptive {
            adaptive(&mut m);
        }
        let r = simulate_graph(&m).unwrap();
        let out = output(&r, "output");
        for (k, row) in out.iter().enumerate() {
            let n = k / 5;
            assert_eq!(
                row,
                &if n == 0 {
                    vec![0., 0.]
                } else {
                    rows[(5 * n / 3) % 3].clone()
                }
            );
        }
    }
}
#[test]
fn continuous_plant_matches_independent_hold_solution_and_rk45_rejection_does_not_advance_limiter()
{
    let levels = [0., 0.2, 0.4, 0.6, 0.8, 1., 1., 0.9, 0.8, 0.7, 0.6];
    let tau: f64 = 0.01;
    let mut states = vec![0.];
    for n in 0..10 {
        states.push(levels[n] + (states[n] - levels[n]) * (-0.1 / tau).exp());
    }
    for use_adaptive in [false, true] {
        let mut m = sample();
        m.dt = 0.001;
        m.duration = 1.;
        for b in &mut m.blocks {
            b.width = 1;
        }
        m.blocks[0].params = BlockKind::Step {
            before: vec![1.],
            after: vec![0.],
            step_time: 0.65,
        };
        m.blocks[1].params = BlockKind::DiscreteRateLimiter {
            rising_rate: 2.,
            falling_rate: 1.,
            initial: vec![0.],
            sample_time: 0.1,
        };
        m.blocks.push(Block {
            id: "plant".into(),
            label: "plant".into(),
            width: 1,
            position: [0., 0.],
            params: BlockKind::FirstOrder {
                gain: 1.,
                tau,
                initial: vec![0.],
            },
        });
        m.edges[2].from = "plant".into();
        m.edges.push(edge("limiter", "plant"));
        if use_adaptive {
            adaptive(&mut m);
        }
        let r = simulate_graph(&m).unwrap();
        let out = output(&r, "output");
        let plant = output(&r, "raw");
        for (k, t) in r.times.iter().enumerate() {
            let n = k / 100;
            assert!((out[k][0] - levels[n]).abs() < 1e-14);
            let exact = levels[n] + (states[n] - levels[n]) * (-(*t - n as f64 * 0.1) / tau).exp();
            assert!((plant[k][0] - exact).abs() < if use_adaptive { 1e-9 } else { 1e-6 });
        }
        if use_adaptive {
            assert!(r.statistics.unwrap().rejected_steps > 0);
        }
    }
}
#[test]
fn linear_analysis_refuses_limiter_ancestry_and_allows_separate_original_scope() {
    let m = sample();
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
        assert!(
            error.contains("limiter: DiscreteRateLimiter is nonlinear"),
            "{error}"
        );
    }
    q.scope = "raw".into();
    f.scope = q.scope.clone();
    validate_frequency(&m, &f).unwrap();
    validate_state_space(&m, &q).unwrap();
}
