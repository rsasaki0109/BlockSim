use blocksim_core::graph::*;
fn block(id: &str, params: BlockKind) -> Block {
    Block {
        id: id.into(),
        label: id.into(),
        width: 1,
        position: [0.0, 0.0],
        params,
    }
}
fn edge(from: &str, to: &str) -> Edge {
    Edge {
        from: from.into(),
        from_port: 0,
        to: to.into(),
        to_port: 0,
    }
}
fn solver(tol: f64) -> Option<SolverConfig> {
    Some(SolverConfig::Rk45 {
        abs_tolerance: tol,
        rel_tolerance: tol,
        min_step: 1e-12,
        max_step: 0.8,
    })
}
fn plant() -> GraphModel {
    GraphModel {
        version: 1,
        dt: 0.01,
        duration: 10.0,
        solver: None,
        blocks: vec![
            block("one", BlockKind::Constant { value: vec![1.0] }),
            block(
                "plant",
                BlockKind::FirstOrder {
                    gain: 1.0,
                    tau: 0.25,
                    initial: vec![0.0],
                },
            ),
            block("scope", BlockKind::Scope {}),
        ],
        edges: vec![edge("one", "plant"), edge("plant", "scope")],
    }
}
fn error(r: &GraphResult) -> f64 {
    r.times
        .iter()
        .zip(&r.traces[0].values)
        .map(|(t, v)| (v[0] - (1.0 - (-t / 0.25).exp())).abs())
        .fold(0.0, f64::max)
}
fn oscillator(omega: f64, width: usize) -> GraphModel {
    let mut g = GraphModel {
        version: 1,
        dt: 0.01,
        duration: 6.0,
        solver: solver(1e-10),
        blocks: vec![
            block(
                "x",
                BlockKind::Integrator {
                    initial: vec![1.0; width],
                },
            ),
            block("negative", BlockKind::Gain { gain: -omega }),
            block(
                "v",
                BlockKind::Integrator {
                    initial: vec![0.0; width],
                },
            ),
            block("positive", BlockKind::Gain { gain: omega }),
            block("scope", BlockKind::Scope {}),
        ],
        edges: vec![
            edge("x", "negative"),
            edge("negative", "v"),
            edge("v", "positive"),
            edge("positive", "x"),
            edge("x", "scope"),
        ],
    };
    for b in &mut g.blocks {
        b.width = width;
    }
    g
}
#[test]
fn omitted_solver_preserves_legacy_json_and_explicit_rk4_adds_only_statistics() {
    let mut g = plant();
    let legacy = simulate_graph(&g).unwrap();
    assert!(legacy.statistics.is_none());
    assert!(!serde_json::to_string(&g).unwrap().contains("solver"));
    g.solver = Some(SolverConfig::Rk4 {});
    let explicit = simulate_graph(&g).unwrap();
    let mut json = serde_json::to_value(&explicit).unwrap();
    json.as_object_mut().unwrap().remove("statistics");
    assert_eq!(json, serde_json::to_value(&legacy).unwrap());
    let stats = explicit.statistics.unwrap();
    assert_eq!(stats.accepted_steps, 1000);
    assert_eq!(stats.rate_evaluations, 4000);
    assert_eq!(stats.output_evaluations, 1001);
}
#[test]
fn dense_output_matches_first_order_solution_and_reduces_rate_evaluations() {
    let mut g = plant();
    g.solver = solver(1e-9);
    let r = simulate_graph(&g).unwrap();
    let stats = r.statistics.as_ref().unwrap();
    assert_eq!(r.times.len(), 1001);
    assert!(error(&r) < 2e-8, "error {}", error(&r));
    assert!(stats.rate_evaluations < 2000, "{stats:?}");
    assert!(stats.max_step > g.dt);
    assert!(stats.max_error_ratio <= 1.0);
    assert!(stats.accepted_steps > 0);
}
#[test]
fn tighter_tolerance_improves_measured_error_with_more_work() {
    let mut g = plant();
    g.dt = 0.5;
    g.solver = solver(1e-3);
    let loose = simulate_graph(&g).unwrap();
    g.solver = solver(1e-10);
    let tight = simulate_graph(&g).unwrap();
    assert!(error(&tight) < error(&loose) / 100.0);
    assert!(tight.statistics.as_ref().unwrap().rejected_steps > 0);
    assert!(
        tight.statistics.unwrap().rate_evaluations > loose.statistics.unwrap().rate_evaluations
    );
}
#[test]
fn off_grid_step_is_split_exactly_even_below_the_requested_minimum_step() {
    let mut g = plant();
    g.dt = 0.1;
    g.duration = 0.5;
    g.blocks[0].params = BlockKind::Step {
        before: vec![0.0],
        after: vec![2.0],
        step_time: 0.015,
    };
    g.blocks[1].params = BlockKind::Integrator { initial: vec![0.0] };
    g.solver = Some(SolverConfig::Rk45 {
        abs_tolerance: 1e-9,
        rel_tolerance: 1e-9,
        min_step: 0.02,
        max_step: 0.2,
    });
    let r = simulate_graph(&g).unwrap();
    for (t, v) in r.times.iter().zip(&r.traces[0].values) {
        assert!((v[0] - 2.0 * (t - 0.015).max(0.0)).abs() < 1e-12);
    }
    assert!(r.statistics.unwrap().min_step < 0.02);
}
#[test]
fn coupled_oscillator_and_vector_states_match_analytic_solutions() {
    let r = simulate_graph(&oscillator(1.0, 2)).unwrap();
    for (t, v) in r.times.iter().zip(&r.traces[0].values) {
        for component in v {
            assert!((component - t.cos()).abs() < 2e-9);
        }
    }
    let mut g = plant();
    for b in &mut g.blocks {
        b.width = 2;
    }
    g.blocks[0].params = BlockKind::Constant {
        value: vec![-2.0, 3.0],
    };
    g.blocks[1].params = BlockKind::Integrator {
        initial: vec![1.0, 4.0],
    };
    g.solver = solver(1e-9);
    let r = simulate_graph(&g).unwrap();
    for (t, v) in r.times.iter().zip(&r.traces[0].values) {
        assert!((v[0] - (1.0 - 2.0 * t)).abs() < 1e-10);
        assert!((v[1] - (4.0 + 3.0 * t)).abs() < 1e-10);
    }
}
#[test]
fn discrete_pid_and_delay_update_on_exact_clocks_with_adaptive_continuous_states() {
    let mut g: GraphModel =
        serde_json::from_str(include_str!("../../../samples/graphs/mixed-pid.json")).unwrap();
    let fixed = simulate_graph(&g).unwrap();
    g.solver = solver(1e-9);
    let adaptive = simulate_graph(&g).unwrap();
    assert_eq!(adaptive.times, fixed.times);
    for (a, b) in adaptive.traces.iter().zip(&fixed.traces) {
        assert_eq!(a.id, b.id);
        for (x, y) in a.values.iter().zip(&b.values) {
            assert!(
                (x[0] - y[0]).abs() < 2e-4,
                "{}: {}",
                a.id,
                (x[0] - y[0]).abs()
            );
        }
    }
    for block in &g.blocks {
        let period = match block.params {
            BlockKind::Pid {
                mode: blocksim_core::PidMode::Discrete,
                sample_time,
                ..
            }
            | BlockKind::Delay { sample_time, .. } => (sample_time / g.dt).round() as usize,
            _ => continue,
        };
        for e in g.edges.iter().filter(|e| e.from == block.id) {
            if let Some(t) = adaptive.traces.iter().find(|t| t.id == e.to) {
                for i in 1..t.values.len() {
                    if !i.is_multiple_of(period) {
                        assert_eq!(t.values[i], t.values[i - 1]);
                    }
                }
            }
        }
    }
    assert!(adaptive.statistics.unwrap().max_step <= 0.05 + 1e-12);
}
#[test]
fn rejects_invalid_tolerances_and_conflicting_nested_solvers_but_inherits_valid_settings() {
    for cfg in [
        SolverConfig::Rk45 {
            abs_tolerance: 0.0,
            rel_tolerance: 1e-6,
            min_step: 1e-9,
            max_step: 1.0,
        },
        SolverConfig::Rk45 {
            abs_tolerance: 1e-9,
            rel_tolerance: -1.0,
            min_step: 1e-9,
            max_step: 1.0,
        },
        SolverConfig::Rk45 {
            abs_tolerance: 1e-9,
            rel_tolerance: 1e-6,
            min_step: 2.0,
            max_step: 1.0,
        },
    ] {
        let mut g = plant();
        g.solver = Some(cfg);
        assert!(validate_draft(&g).is_err());
    }
    let mut g: GraphModel =
        serde_json::from_str(include_str!("../../../samples/graphs/subsystem.json")).unwrap();
    g.solver = solver(1e-9);
    let r = simulate_graph(&g).unwrap();
    assert_eq!(r.statistics.unwrap().rate_evaluations, 0);
    if let BlockKind::Subsystem { model } = &mut g.blocks[1].params {
        model.solver = Some(SolverConfig::Rk4 {});
    }
    assert!(validate_draft(&g).unwrap_err().contains("solver"));
}
#[test]
fn reports_unattainable_tolerance_and_bounds_work_for_fast_oscillatory_models() {
    let mut g = plant();
    g.dt = 0.1;
    g.duration = 1.0;
    if let BlockKind::FirstOrder { tau, .. } = &mut g.blocks[1].params {
        *tau = 1e-6;
    }
    g.solver = Some(SolverConfig::Rk45 {
        abs_tolerance: 1e-12,
        rel_tolerance: 1e-12,
        min_step: 0.01,
        max_step: 0.1,
    });
    assert!(simulate_graph(&g).unwrap_err().contains("min_step"));
    let mut fast = oscillator(1e5, 64);
    fast.duration = 1.0;
    assert!(simulate_graph(&fast)
        .unwrap_err()
        .contains("block-component evaluations"));
}
