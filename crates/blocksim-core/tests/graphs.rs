use blocksim_core::graph::*;
use blocksim_core::{simulate, ModelConfig};
fn load(name: &str) -> GraphModel {
    serde_json::from_str(
        &std::fs::read_to_string(format!("../../samples/graphs/{name}.json")).unwrap(),
    )
    .unwrap()
}
fn block(id: &str, params: BlockKind) -> Block {
    Block {
        id: id.into(),
        label: id.into(),
        width: 1,
        position: [0.0, 0.0],
        params,
    }
}
fn edge(from: &str, to: &str, to_port: usize) -> Edge {
    Edge {
        from: from.into(),
        from_port: 0,
        to: to.into(),
        to_port,
    }
}
fn model(blocks: Vec<Block>, edges: Vec<Edge>, dt: f64, duration: f64) -> GraphModel {
    GraphModel {
        version: 1,
        dt,
        duration,
        blocks,
        edges,
    }
}
fn trace<'a>(r: &'a GraphResult, id: &str) -> &'a Trace {
    r.traces.iter().find(|s| s.id == id).unwrap()
}
#[test]
fn graph_pid_matches_existing_continuous_experiment_with_aw_on_and_off() {
    for aw in [true, false] {
        let mut graph = load("continuous-pid");
        if let BlockKind::Pid { config, .. } = &mut graph.blocks[2].params {
            config.anti_windup = aw;
        }
        let r = simulate_graph(&graph).unwrap();
        let mut cfg = ModelConfig::default();
        cfg.pid.anti_windup = aw;
        let original = simulate(&cfg).unwrap();
        for (actual, expected) in trace(&r, "output").values.iter().zip(original.samples) {
            assert!((actual[0] - expected.output).abs() < 1e-8);
        }
        assert!(trace(&r, "control")
            .values
            .iter()
            .all(|u| u[0] >= 0.0 && u[0] <= 1.0));
    }
}
#[test]
fn integrator_resolves_step_between_ticks_and_matches_exact_area() {
    for step_time in [0.0, 0.05, 0.1, 0.157] {
        let g = model(
            vec![
                block(
                    "step",
                    BlockKind::Step {
                        before: vec![0.0],
                        after: vec![2.0],
                        step_time,
                    },
                ),
                block("i", BlockKind::Integrator { initial: vec![1.0] }),
                block("scope", BlockKind::Scope {}),
            ],
            vec![edge("step", "i", 0), edge("i", "scope", 0)],
            0.1,
            1.0,
        );
        let r = simulate_graph(&g).unwrap();
        let actual = trace(&r, "scope").values.last().unwrap()[0];
        assert!(
            (actual - (1.0 + 2.0 * (1.0 - step_time))).abs() < 1e-10,
            "{step_time}: {actual}"
        );
    }
}
#[test]
fn rk4_first_order_matches_analytic_solution_and_refinement() {
    let g = model(
        vec![
            block("one", BlockKind::Constant { value: vec![1.0] }),
            block(
                "p",
                BlockKind::FirstOrder {
                    gain: 2.0,
                    tau: 1.0,
                    initial: vec![0.0],
                },
            ),
            block("scope", BlockKind::Scope {}),
        ],
        vec![edge("one", "p", 0), edge("p", "scope", 0)],
        0.1,
        2.0,
    );
    let exact = 2.0 * (1.0 - (-2.0f64).exp());
    let coarse = (trace(&simulate_graph(&g).unwrap(), "scope")
        .values
        .last()
        .unwrap()[0]
        - exact)
        .abs();
    let fine = (trace(
        &simulate_graph(&GraphModel { dt: 0.05, ..g }).unwrap(),
        "scope",
    )
    .values
    .last()
    .unwrap()[0]
        - exact)
        .abs();
    assert!(coarse < 1e-6 && fine < coarse / 10.0);
}
#[test]
fn mixed_discrete_pid_holds_between_ticks_and_delay_has_its_own_period() {
    let g = load("mixed-pid");
    let r = simulate_graph(&g).unwrap();
    let u = trace(&r, "control");
    let y = trace(&r, "output");
    let delayed = trace(&r, "sampled-output");
    for (i, v) in u.values.iter().enumerate() {
        assert!((0.0..=1.0).contains(&v[0]));
        if i % 5 != 0 {
            assert_eq!(*v, u.values[i - 1]);
        }
    }
    for i in 0..r.times.len() {
        let source = if i < 10 {
            0.0
        } else {
            y.values[(i / 10 - 1) * 10][0]
        };
        assert!((delayed.values[i][0] - source).abs() < 1e-12);
    }
}
#[test]
fn unit_delay_feedback_is_synchronous_and_block_order_independent() {
    let g = model(
        vec![
            block("one", BlockKind::Constant { value: vec![1.0] }),
            block(
                "add",
                BlockKind::Sum {
                    signs: vec![1.0, 1.0],
                },
            ),
            block(
                "delay",
                BlockKind::Delay {
                    initial: vec![0.0],
                    sample_time: 0.2,
                },
            ),
            block("scope", BlockKind::Scope {}),
        ],
        vec![
            edge("one", "add", 0),
            edge("delay", "add", 1),
            edge("add", "delay", 0),
            edge("delay", "scope", 0),
        ],
        0.1,
        1.0,
    );
    let first = simulate_graph(&g).unwrap();
    let mut reordered = g.clone();
    reordered.blocks.reverse();
    let second = simulate_graph(&reordered).unwrap();
    assert_eq!(
        trace(&first, "scope").values,
        trace(&second, "scope").values
    );
    assert_eq!(
        trace(&first, "scope")
            .values
            .iter()
            .map(|v| v[0])
            .collect::<Vec<_>>(),
        vec![0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 5.]
    );
}
#[test]
fn vector_saturation_selector_mux_and_product_have_expected_values() {
    let r = simulate_graph(&load("vector")).unwrap();
    assert_eq!(trace(&r, "vector-output").values[0], vec![-1., 1., 1.]);
    assert_eq!(trace(&r, "component").values[0], vec![1.]);
    let mut g = model(
        vec![
            block("a", BlockKind::Constant { value: vec![2.] }),
            block("b", BlockKind::Constant { value: vec![3.] }),
            block("product", BlockKind::Product { inputs: 2 }),
            block("scope", BlockKind::Scope {}),
        ],
        vec![
            edge("a", "product", 0),
            edge("b", "product", 1),
            edge("product", "scope", 0),
        ],
        0.1,
        1.0,
    );
    assert_eq!(
        trace(&simulate_graph(&g).unwrap(), "scope").values[0],
        vec![6.]
    );
    g.blocks[2].params = BlockKind::Mux { inputs: 2 };
    g.blocks[2].width = 2;
    g.blocks[3].width = 2;
    assert_eq!(
        trace(&simulate_graph(&g).unwrap(), "scope").values[0],
        vec![2., 3.]
    );
}
#[test]
fn subsystem_matches_flat_model_and_nested_roundtrip() {
    let g = load("subsystem");
    let r = simulate_graph(&g).unwrap();
    for (time, v) in r.times.iter().zip(&trace(&r, "wave-output").values) {
        assert!((v[0] - 3.0 * (std::f64::consts::PI * time).sin()).abs() < 1e-10);
    }
    let json = serde_json::to_string(&g).unwrap();
    assert_eq!(serde_json::from_str::<GraphModel>(&json).unwrap(), g);
    let mut nested = g.clone();
    if let BlockKind::Subsystem { model: inner } = &mut nested.blocks[1].params {
        let child = inner.clone();
        inner.blocks[1].params = BlockKind::Subsystem { model: child };
    }
    // Replacing gain by another amplifier still produces the same gain once.
    assert_eq!(
        trace(&simulate_graph(&nested).unwrap(), "wave-output").values,
        trace(&r, "wave-output").values
    );
}
#[test]
fn rejects_missing_ports_widths_duplicates_and_algebraic_loops_but_saves_drafts() {
    let base = load("vector");
    let mut g = base.clone();
    g.edges.remove(0);
    assert!(validate_draft(&g).is_ok());
    assert!(validate_graph(&g).unwrap_err().contains("unconnected"));
    let mut g = base.clone();
    g.blocks[1].width = 2;
    assert!(validate_graph(&g).unwrap_err().contains("width mismatch"));
    let mut g = base.clone();
    g.edges.push(g.edges[0].clone());
    assert!(validate_draft(&g)
        .unwrap_err()
        .contains("already connected"));
    let mut g = base.clone();
    g.edges[0].from_port = 1;
    assert!(validate_draft(&g).unwrap_err().contains("output port"));
    let mut g = base.clone();
    g.blocks[1].id = "source".into();
    assert!(validate_draft(&g).unwrap_err().contains("Duplicate"));
    let g = model(
        vec![
            block("gain", BlockKind::Gain { gain: 1.0 }),
            block("scope", BlockKind::Scope {}),
        ],
        vec![edge("gain", "gain", 0), edge("gain", "scope", 0)],
        0.1,
        1.0,
    );
    assert!(validate_graph(&g).unwrap_err().contains("Algebraic loop"));
}
#[test]
fn rejects_invalid_clocks_and_nonfinite_overflow() {
    let mut g = load("mixed-pid");
    if let BlockKind::Pid { sample_time, .. } = &mut g.blocks[2].params {
        *sample_time = 0.015;
    }
    assert!(validate_graph(&g).unwrap_err().contains("integer multiple"));
    let mut g = load("vector");
    g.duration = 1.005;
    assert!(validate_graph(&g).is_err());
    let mut g = load("vector");
    g.dt = 1e-12;
    assert!(validate_graph(&g).is_err());
    let mut g = load("vector");
    if let BlockKind::Gain { gain } = &mut g.blocks[1].params {
        *gain = f64::MAX;
    }
    assert!(simulate_graph(&g).unwrap_err().contains("scale at t="));
    let mut g = load("vector");
    g.blocks[0].position[0] = f64::NAN;
    assert!(validate_draft(&g).is_err());
}

#[test]
fn multiport_subsystem_maps_distinct_vector_widths_and_output_ports() {
    let wide = |id: &str, width: usize, params: BlockKind| Block {
        width,
        ..block(id, params)
    };
    let inner = model(
        vec![
            wide("a", 2, BlockKind::Input { port: 0 }),
            block("b", BlockKind::Input { port: 1 }),
            wide("mux", 3, BlockKind::Mux { inputs: 2 }),
            wide("gain", 2, BlockKind::Gain { gain: 2.0 }),
            wide("out0", 3, BlockKind::Output { port: 0 }),
            wide("out1", 2, BlockKind::Output { port: 1 }),
        ],
        vec![
            edge("a", "mux", 0),
            edge("b", "mux", 1),
            edge("a", "gain", 0),
            edge("mux", "out0", 0),
            edge("gain", "out1", 0),
        ],
        0.1,
        1.0,
    );
    let mut g = model(
        vec![
            wide(
                "source0",
                2,
                BlockKind::Constant {
                    value: vec![2.0, 4.0],
                },
            ),
            block("source1", BlockKind::Constant { value: vec![3.0] }),
            block(
                "sub",
                BlockKind::Subsystem {
                    model: Box::new(inner),
                },
            ),
            wide("scope0", 3, BlockKind::Scope {}),
            wide("scope1", 2, BlockKind::Scope {}),
        ],
        vec![
            edge("source0", "sub", 0),
            edge("source1", "sub", 1),
            edge("sub", "scope0", 0),
            edge("sub", "scope1", 0),
        ],
        0.1,
        1.0,
    );
    g.edges[3].from_port = 1;
    let r = simulate_graph(&g).unwrap();
    assert_eq!(trace(&r, "scope0").values[0], vec![2.0, 4.0, 3.0]);
    assert_eq!(trace(&r, "scope1").values[0], vec![4.0, 8.0]);
}
#[test]
fn workload_and_recording_limits_fail_before_running() {
    let mut g = load("vector");
    g.duration = 1000.0;
    // 3 vector components * 100001 points exceeds the record limit only after
    // adding enough scopes; all blocks remain valid and connected.
    for i in 0..8 {
        let id = format!("extra{i}");
        g.blocks.push(Block {
            width: 3,
            ..block(&id, BlockKind::Scope {})
        });
        g.edges.push(edge("limit", &id, 0));
    }
    assert!(validate_graph(&g).unwrap_err().contains("2000000 cells"));
    let mut g = load("vector");
    g.duration = 1000.0;
    for i in 0..80 {
        let id = format!("extra{i}");
        g.blocks.push(Block {
            width: 3,
            ..block(&id, BlockKind::Gain { gain: 1.0 })
        });
        g.edges.push(edge("limit", &id, 0));
    }
    assert!(validate_graph(&g)
        .unwrap_err()
        .contains("block-component steps"));
}

#[test]
fn vector_pid_has_independent_continuous_and_discrete_integral_states() {
    for mode in [
        blocksim_core::PidMode::Continuous,
        blocksim_core::PidMode::Discrete,
    ] {
        let config = blocksim_core::PidConfig {
            kp: 0.0,
            ki: 1.0,
            kd: 0.0,
            ..blocksim_core::PidConfig::default()
        };
        let wide = |id: &str, params: BlockKind| Block {
            width: 2,
            ..block(id, params)
        };
        let g = model(
            vec![
                wide(
                    "e",
                    BlockKind::Constant {
                        value: vec![0.1, 0.2],
                    },
                ),
                wide(
                    "pid",
                    BlockKind::Pid {
                        config,
                        mode,
                        sample_time: 0.02,
                    },
                ),
                wide("scope", BlockKind::Scope {}),
            ],
            vec![edge("e", "pid", 0), edge("pid", "scope", 0)],
            0.01,
            1.0,
        );
        let r = simulate_graph(&g).unwrap();
        let actual = trace(&r, "scope").values.last().unwrap();
        let duration = if mode == blocksim_core::PidMode::Continuous {
            1.0
        } else {
            1.02
        };
        assert!((actual[0] - 0.1 * duration).abs() < 1e-10);
        assert!((actual[1] - 0.2 * duration).abs() < 1e-10);
    }
}
