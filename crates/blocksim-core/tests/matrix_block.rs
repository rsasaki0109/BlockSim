use blocksim_core::{
    graph::{
        state_space::{analyze_state_space, mimo::*, StateSpaceRequest},
        *,
    },
    PidMode,
};
fn plan(discrete: bool) -> (GraphModel, MimoRequest) {
    let p: serde_json::Value = serde_json::from_str(if discrete {
        include_str!("../../../samples/matrix/discrete-plan.json")
    } else {
        include_str!("../../../samples/matrix/continuous-plan.json")
    })
    .unwrap();
    (
        serde_json::from_value(p["model"].clone()).unwrap(),
        serde_json::from_value(p["request"].clone()).unwrap(),
    )
}
fn exact(t: f64, x: [f64; 2], u: [f64; 2]) -> [f64; 2] {
    let a = u[0] + u[1];
    let b = 2. * u[1];
    let e = (-t).exp();
    let e2 = (-2. * t).exp();
    [
        a + (x[0] - a) * e,
        x[1] * e2 + (a + b) / 2. * (1. - e2) + (x[0] - a) * (e - e2),
    ]
}
#[test]
fn continuous_coupled_matrix_initial_conditions_and_off_grid_step_match_independent_solution() {
    for (adaptive, change) in [(false, 1.23), (true, 1.23), (false, 1.235), (true, 1.235)] {
        let (mut m, _) = plan(false);
        if let BlockKind::Step { step_time, .. } = &mut m.blocks[0].params {
            *step_time = change;
        }
        if adaptive {
            m.solver = Some(SolverConfig::Rk45 {
                abs_tolerance: 1e-11,
                rel_tolerance: 1e-10,
                min_step: 1e-8,
                max_step: 0.05,
            });
        }
        let r = simulate_graph(&m).unwrap();
        let boundary = exact(change, [0.5, -0.25], [1., 2.]);
        for (t, row) in r.times.iter().zip(&r.traces[0].values) {
            let (x, u) = if *t < change {
                (exact(*t, [0.5, -0.25], [1., 2.]), [1., 2.])
            } else {
                (exact(*t - change, boundary, [0.25, -1.]), [0.25, -1.])
            };
            assert!((row[0] - x[0] - 0.25 * u[0]).abs() < 3e-9);
            assert!((row[1] - x[1] - 0.5 * u[1]).abs() < 3e-9);
        }
        assert_eq!(r.traces[0].values[0], vec![0.75, 0.75]);
    }
}
#[test]
fn discrete_matrix_commits_synchronously_and_holds_output_and_direct_term_between_samples() {
    for adaptive in [false, true] {
        let (mut m, _) = plan(true);
        if adaptive {
            m.solver = Some(SolverConfig::Rk45 {
                abs_tolerance: 1e-9,
                rel_tolerance: 1e-7,
                min_step: 1e-8,
                max_step: 0.05,
            });
        }
        let r = simulate_graph(&m).unwrap();
        let mut x = [0.5, -0.25];
        let mut previous = [1., 2.];
        let mut held = [0.75, 0.75];
        for (k, row) in r.traces[0].values.iter().enumerate() {
            if k > 0 && k % 10 == 0 {
                x = [
                    0.8 * x[0] + 0.2 * previous[0] + 0.1 * previous[1],
                    0.1 * x[0] + 0.7 * x[1] + 0.3 * previous[1],
                ];
                previous = if k < 15 { [1., 2.] } else { [0.25, -1.] };
                held = [x[0] + 0.25 * previous[0], x[1] + 0.5 * previous[1]];
            }
            for j in 0..2 {
                assert!((row[j] - held[j]).abs() < 2e-15);
            }
        }
        assert_eq!(r.traces[0].values[14], r.traces[0].values[15]);
    }
}
fn inverse(re: f64, im: f64) -> (f64, f64) {
    let d = re * re + im * im;
    (re / d, -im / d)
}
fn mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}
#[test]
fn coupled_mimo_matrix_and_all_frequency_paths_match_independent_resolvents() {
    for discrete in [false, true] {
        let (m, q) = plan(discrete);
        let r = analyze_mimo(&m, &q).unwrap();
        assert_eq!(r.states.len(), 2);
        assert_eq!(r.poles.roots.len(), 2);
        assert_eq!(r.structure.controllability.numerical_rank, Some(2));
        assert_eq!(r.structure.observability.numerical_rank, Some(2));
        for pair in &r.responses {
            for p in &pair.points {
                let w = p.omega;
                let (f, g) = if discrete {
                    let z = ((w * 0.1).cos(), (w * 0.1).sin());
                    (inverse(z.0 - 0.8, z.1), inverse(z.0 - 0.7, z.1))
                } else {
                    (inverse(1., w), inverse(2., w))
                };
                let fg = mul(f, g);
                let expected = if discrete {
                    match (pair.output, pair.input) {
                        (0, 0) => (0.2 * f.0 + 0.25, 0.2 * f.1),
                        (0, 1) => (0.1 * f.0, 0.1 * f.1),
                        (1, 0) => (0.02 * fg.0, 0.02 * fg.1),
                        _ => (0.01 * fg.0 + 0.3 * g.0 + 0.5, 0.01 * fg.1 + 0.3 * g.1),
                    }
                } else {
                    match (pair.output, pair.input) {
                        (0, 0) => (f.0 + 0.25, f.1),
                        (0, 1) => f,
                        (1, 0) => fg,
                        _ => (fg.0 + 2. * g.0 + 0.5, fg.1 + 2. * g.1),
                    }
                };
                assert!(p.error.is_none());
                assert!((p.real.unwrap() - expected.0).abs() < 1e-12);
                assert!((p.imag.unwrap() - expected.1).abs() < 1e-12);
            }
        }
    }
}
#[test]
fn rectangular_zero_state_matrices_support_two_inputs_three_outputs_and_discrete_hold() {
    for discrete in [false, true] {
        let (mut m, mut q) = plan(discrete);
        m.blocks[1].width = 3;
        m.blocks[2].width = 3;
        m.blocks[1].params = BlockKind::StateSpace {
            a: vec![],
            b: vec![],
            c: vec![vec![], vec![], vec![]],
            d: vec![vec![1., 2.], vec![0., -1.], vec![0., 0.]],
            initial: vec![],
            mode: if discrete {
                PidMode::Discrete
            } else {
                PidMode::Continuous
            },
            sample_time: 0.1,
        };
        q.outputs.push(Channel {
            block: "output".into(),
            component: 2,
        });
        let r = simulate_graph(&m).unwrap();
        for (k, row) in r.traces[0].values.iter().enumerate() {
            let after = if discrete { k >= 20 } else { k >= 123 };
            assert_eq!(
                row,
                &if after {
                    vec![-1.75, 1., 0.]
                } else {
                    vec![5., -2., 0.]
                }
            );
        }
        let r = analyze_mimo(&m, &q).unwrap();
        assert!(r.states.is_empty());
        assert_eq!(r.d, vec![vec![1., 2.], vec![0., -1.], vec![0., 0.]]);
        assert_eq!(r.responses.len(), 6);
    }
}
#[test]
fn zero_c_keeps_hidden_unstable_modes_without_changing_zero_output() {
    let (mut m, _) = plan(false);
    m.blocks[1].params = BlockKind::StateSpace {
        a: vec![vec![1., 0.], vec![0., -2.]],
        b: vec![vec![0., 0.], vec![0., 0.]],
        c: vec![vec![0., 0.], vec![0., 0.]],
        d: vec![vec![0., 0.], vec![0., 0.]],
        initial: vec![1., 2.],
        mode: PidMode::Continuous,
        sample_time: 0.1,
    };
    let r = analyze_state_space(
        &m,
        &StateSpaceRequest {
            source: "source".into(),
            source_component: 0,
            scope: "output".into(),
            scope_component: 0,
        },
    )
    .unwrap();
    assert_eq!(r.states.len(), 2);
    assert_eq!(r.poles.roots.len(), 2);
    assert_eq!(r.stability.classification, "unstable");
    assert_eq!(r.structure.unwrap().observability.numerical_rank, Some(0));
    assert!(simulate_graph(&m).unwrap().traces[0]
        .values
        .iter()
        .all(|v| v == &vec![0., 0.]));
}
#[test]
fn all_dimensions_periods_and_finite_values_are_validated_before_execution() {
    let (m, _) = plan(false);
    let original = serde_json::to_value(&m).unwrap();
    for (key, value) in [
        ("a", serde_json::json!([[1, 2]])),
        ("b", serde_json::json!([[1], [2]])),
        ("c", serde_json::json!([[1]])),
        ("d", serde_json::json!([[1], [2]])),
        ("initial", serde_json::json!([0])),
        ("sample_time", serde_json::json!(0)),
    ] {
        let mut json = original.clone();
        json["blocks"][1]["params"][key] = value;
        let m: GraphModel = serde_json::from_value(json).unwrap();
        assert!(validate_draft(&m).is_err());
    }
    let (mut m, _) = plan(true);
    if let BlockKind::StateSpace { sample_time, .. } = &mut m.blocks[1].params {
        *sample_time = 0.015;
    }
    assert!(validate_draft(&m).unwrap_err().contains("integer"));
    let (mut m, _) = plan(false);
    if let BlockKind::StateSpace { a, .. } = &mut m.blocks[1].params {
        a[0][0] = f64::INFINITY;
    }
    assert!(validate_draft(&m).is_err());
    for (n, m, p) in [(17, 2, 2), (2, 9, 2), (2, 2, 9)] {
        let (mut model, _) = plan(false);
        model.blocks[1].width = p;
        model.blocks[1].params = BlockKind::StateSpace {
            a: vec![vec![0.; n]; n],
            b: vec![vec![0.; m]; n],
            c: vec![vec![0.; n]; p],
            d: vec![vec![0.; m]; p],
            initial: vec![0.; n],
            mode: PidMode::Continuous,
            sample_time: 0.1,
        };
        assert!(validate_draft(&model).is_err());
    }
    let (mut model, _) = plan(false);
    if let BlockKind::StateSpace { a, initial, .. } = &mut model.blocks[1].params {
        *a = vec![vec![1e308, 1e308], vec![0., 0.]];
        *initial = vec![1., 1.];
    }
    validate_draft(&model).unwrap();
    let error = simulate_graph(&model).unwrap_err();
    assert!(error.to_lowercase().contains("finite"), "{error}");
}
#[test]
fn serialization_retains_original_matrix_block_and_reexecution_starts_at_initial_state() {
    let (m, _) = plan(false);
    let text = serde_json::to_string(&m).unwrap();
    validate_draft(&m).unwrap();
    let restored: GraphModel = serde_json::from_str(&text).unwrap();
    assert_eq!(m, restored);
    assert_eq!(
        serde_json::to_string(&simulate_graph(&m).unwrap()).unwrap(),
        serde_json::to_string(&simulate_graph(&restored).unwrap()).unwrap()
    );
    assert!(!text.contains("state-0"));
    assert!(!text.contains("held-"));
    let mut q: serde_json::Value = serde_json::from_str(&text).unwrap();
    q["blocks"][1]["params"]["extra"] = serde_json::json!(1);
    assert!(serde_json::from_value::<GraphModel>(q).is_err());
}
#[test]
fn zero_direct_term_allows_state_feedback_while_nonzero_direct_loop_is_rejected() {
    for discrete in [false, true] {
        let mut m:GraphModel=serde_json::from_value(serde_json::json!({"version":1,"dt":0.01,"duration":1,"blocks":[{"id":"source","label":"source","width":1,"position":[0,0],"params":{"kind":"constant","value":[1]}},{"id":"error","label":"error","width":1,"position":[0,0],"params":{"kind":"sum","signs":[1,-1]}},{"id":"matrix","label":"matrix","width":1,"position":[0,0],"params":{"kind":"state_space","a":[[if discrete{0.8}else{0.0}]],"b":[[if discrete{0.2}else{1.0}]],"c":[[1]],"d":[[0]],"initial":[0],"mode":if discrete{"discrete"}else{"continuous"},"sample_time":0.1}},{"id":"output","label":"output","width":1,"position":[0,0],"params":{"kind":"scope"}}],"edges":[{"from":"source","from_port":0,"to":"error","to_port":0},{"from":"matrix","from_port":0,"to":"error","to_port":1},{"from":"error","from_port":0,"to":"matrix","to_port":0},{"from":"matrix","from_port":0,"to":"output","to_port":0}]})).unwrap();
        let r = simulate_graph(&m).unwrap();
        for (k, (t, row)) in r.times.iter().zip(&r.traces[0].values).enumerate() {
            let expected = if discrete {
                0.5 * (1. - 0.6f64.powi((k / 10) as i32))
            } else {
                1. - (-t).exp()
            };
            assert!((row[0] - expected).abs() < 1e-9);
        }
        if let BlockKind::StateSpace { d, .. } = &mut m.blocks[2].params {
            d[0][0] = 0.1;
        }
        let error = simulate_graph(&m).unwrap_err();
        assert!(error.to_lowercase().contains("algebraic"), "{error}");
    }
}
#[test]
fn dense_maximum_matrix_and_eight_user_levels_keep_all_sixteen_states() {
    let (mut m, mut q) = plan(false);
    m.blocks[0].width = 8;
    m.blocks[0].params = BlockKind::Constant { value: vec![1.; 8] };
    m.blocks[1].width = 8;
    m.blocks[2].width = 8;
    let a: Vec<Vec<f64>> = (0..16)
        .map(|i| (0..16).map(|j| if i == j { -2.0 } else { 0.01 }).collect())
        .collect();
    let b = vec![vec![0.1; 8]; 16];
    let c = vec![vec![0.1; 16]; 8];
    m.blocks[1].params = BlockKind::StateSpace {
        a: a.clone(),
        b,
        c,
        d: vec![vec![0.01; 8]; 8],
        initial: vec![1.; 16],
        mode: PidMode::Continuous,
        sample_time: 0.1,
    };
    q.inputs = (0..8)
        .map(|component| Channel {
            block: "source".into(),
            component,
        })
        .collect();
    q.outputs = (0..8)
        .map(|component| Channel {
            block: "output".into(),
            component,
        })
        .collect();
    q.points = 2;
    let r = analyze_mimo(&m, &q).unwrap();
    assert_eq!(r.states.len(), 16);
    assert_eq!(r.responses.len(), 64);
    assert_eq!(r.check.inputs, 8);
    assert_eq!(r.check.outputs, 8);
    assert!(r.check.variables <= 128);
    assert!(r.poles.status == "verified_residual" || r.poles.status == "unresolved");
    let sim = simulate_graph(&m).unwrap();
    for (t, row) in sim.times.iter().zip(&sim.traces[0].values) {
        let x = 0.8 / 1.85 + (1. - 0.8 / 1.85) * (-1.85 * t).exp();
        assert!(row.iter().all(|v| (*v - 1.6 * x - 0.08).abs() < 1e-8));
    }
    // A generated matrix implementation does not consume the user's subsystem depth budget.
    let mut child = m;
    child.blocks[2].params = BlockKind::Output { port: 0 };
    for _ in 0..8 {
        child = GraphModel {
            version: 1,
            dt: child.dt,
            duration: child.duration,
            solver: None,
            blocks: vec![
                Block {
                    id: "nested".into(),
                    label: "nested".into(),
                    width: 8,
                    position: [0., 0.],
                    params: BlockKind::Subsystem {
                        model: Box::new(child),
                    },
                },
                Block {
                    id: "boundary".into(),
                    label: "boundary".into(),
                    width: 8,
                    position: [0., 0.],
                    params: BlockKind::Output { port: 0 },
                },
            ],
            edges: vec![Edge {
                from: "nested".into(),
                from_port: 0,
                to: "boundary".into(),
                to_port: 0,
            }],
        };
    }
    child.blocks[1].params = BlockKind::Scope {};
    assert!(simulate_graph(&child).is_ok());
}
#[test]
fn zero_inputs_still_evolve_initial_state_and_invalid_connection_width_is_diagnosed() {
    let (mut m, _) = plan(false);
    m.blocks[0].params = BlockKind::Constant {
        value: vec![0., 0.],
    };
    let r = simulate_graph(&m).unwrap();
    for (t, row) in r.times.iter().zip(&r.traces[0].values) {
        let x = exact(*t, [0.5, -0.25], [0., 0.]);
        assert!((row[0] - x[0]).abs() < 1e-8);
        assert!((row[1] - x[1]).abs() < 1e-8);
    }
    m.blocks[0].width = 1;
    m.blocks[0].params = BlockKind::Constant { value: vec![0.] };
    let issues = diagnose_graph(&m).diagnostics;
    assert!(issues
        .iter()
        .any(|d| d.code == DiagnosticCode::SignalWidth
            && d.block_id.as_deref() == Some("matrix/input")));
}
