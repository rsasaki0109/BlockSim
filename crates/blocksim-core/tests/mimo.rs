use blocksim_core::graph::{
    simulate_graph,
    state_space::{analyze_discrete_state_space, analyze_state_space, mimo::*, StateSpaceRequest},
    BlockKind, GraphModel,
};
fn plan(discrete: bool) -> (GraphModel, MimoRequest) {
    let p: serde_json::Value = serde_json::from_str(if discrete {
        include_str!("../../../samples/mimo/discrete-plan.json")
    } else {
        include_str!("../../../samples/mimo/continuous-plan.json")
    })
    .unwrap();
    (
        serde_json::from_value(p["model"].clone()).unwrap(),
        serde_json::from_value(p["request"].clone()).unwrap(),
    )
}
#[test]
fn union_has_two_shared_states_and_joint_rank_two_even_when_each_input_rank_is_one() {
    for discrete in [false, true] {
        let (m, q) = plan(discrete);
        let r = analyze_mimo(&m, &q).unwrap();
        assert_eq!(r.states.len(), 2);
        assert_eq!(r.poles.roots.len(), 2);
        assert_eq!(r.structure.controllability.numerical_rank, Some(2));
        assert_eq!(r.structure.observability.numerical_rank, Some(2));
        assert_eq!(r.b, vec![vec![1., 1.], vec![1., -1.]]);
        assert_eq!(r.c, vec![vec![1., 2.], vec![1., -1.]]);
        assert_eq!(r.d, vec![vec![0.25, 0.], vec![0., 0.5]]);
        assert_eq!(
            r.a,
            if discrete {
                vec![vec![0., 0.], vec![0., 0.]]
            } else {
                vec![vec![-1., 0.], vec![0., -1.]]
            }
        );
        assert_eq!(r.check.sample_time, discrete.then_some(0.1));
        assert_eq!(r.stability.classification, "stable");
        let s = StateSpaceRequest {
            source: "u1".into(),
            source_component: 0,
            scope: "y1".into(),
            scope_component: 0,
        };
        let single = if discrete {
            analyze_discrete_state_space(&m, &s)
        } else {
            analyze_state_space(&m, &s)
        }
        .unwrap();
        assert_eq!(
            single.structure.unwrap().controllability.numerical_rank,
            Some(1)
        );
    }
}
#[test]
fn all_four_transfers_match_independent_complex_expressions_at_every_frequency() {
    for discrete in [false, true] {
        let (m, q) = plan(discrete);
        let r = analyze_mimo(&m, &q).unwrap();
        for response in &r.responses {
            assert_eq!(response.points.len(), 101);
            for p in &response.points {
                let w = p.omega;
                let (re, im) = if discrete {
                    ((w * 0.1).cos(), -(w * 0.1).sin())
                } else {
                    (1. / (1. + w * w), -w / (1. + w * w))
                };
                let (gain, direct) = match (response.output, response.input) {
                    (0, 0) => (3., 0.25),
                    (0, 1) => (-1., 0.),
                    (1, 0) => (0., 0.),
                    (1, 1) => (2., 0.5),
                    _ => unreachable!(),
                };
                assert!(p.error.is_none());
                assert!((p.real.unwrap() - gain * re - direct).abs() < 1e-12);
                assert!((p.imag.unwrap() - gain * im).abs() < 1e-12);
                assert!(p.relative_residual.unwrap() <= 1e-10);
                if gain == 0. && direct == 0. {
                    assert_eq!(p.magnitude, Some(0.));
                    assert_eq!(p.magnitude_db, None);
                    assert_eq!(p.phase_deg, None);
                }
            }
        }
    }
}
#[test]
fn every_pair_matches_legacy_siso_matrices_with_state_identity_mapping() {
    for discrete in [false, true] {
        let (m, q) = plan(discrete);
        let r = analyze_mimo(&m, &q).unwrap();
        for (i, input) in q.inputs.iter().enumerate() {
            for (o, output) in q.outputs.iter().enumerate() {
                let sq = StateSpaceRequest {
                    source: input.block.clone(),
                    source_component: 0,
                    scope: output.block.clone(),
                    scope_component: 0,
                };
                let s = if discrete {
                    analyze_discrete_state_space(&m, &sq)
                } else {
                    analyze_state_space(&m, &sq)
                }
                .unwrap();
                for (j, state) in s.states.iter().enumerate() {
                    let k = r
                        .states
                        .iter()
                        .position(|t| {
                            t.block == state.block
                                && t.component == state.component
                                && t.state == state.state
                        })
                        .unwrap();
                    assert_eq!(s.b[j], r.b[k][i]);
                    assert_eq!(s.c[j], r.c[o][k]);
                    for (l, t) in s.states.iter().enumerate() {
                        let h = r
                            .states
                            .iter()
                            .position(|u| u.block == t.block && u.state == t.state)
                            .unwrap();
                        assert_eq!(s.a[j][l], r.a[k][h]);
                    }
                }
                assert_eq!(s.d, r.d[o][i]);
            }
        }
    }
}
#[test]
fn sample_time_outputs_match_independent_superposition_and_hold_semantics() {
    for discrete in [false, true] {
        let (m, _) = plan(discrete);
        let r = simulate_graph(&m).unwrap();
        for (k, &t) in r.times.iter().enumerate() {
            let x = if discrete {
                if k < 10 {
                    0.
                } else {
                    1.
                }
            } else {
                1. - (-t).exp()
            };
            for trace in &r.traces {
                let expected = if trace.id == "y1" {
                    x + 0.25
                } else {
                    4. * x + 1.
                };
                assert!((trace.values[k][0] - expected).abs() < 1e-8);
            }
        }
    }
}
#[test]
fn duplicate_empty_excessive_channels_bad_components_and_ranges_are_rejected() {
    let (m, q) = plan(false);
    let mut bad = q.clone();
    bad.inputs.push(bad.inputs[0].clone());
    assert!(validate_mimo(&m, &bad).unwrap_err().contains("duplicate"));
    let mut bad = q.clone();
    bad.inputs.clear();
    assert!(validate_mimo(&m, &bad).is_err());
    bad = q.clone();
    bad.outputs[0].component = 1;
    assert!(validate_mimo(&m, &bad).is_err());
    bad = q.clone();
    bad.inputs[0].block = "x1".into();
    assert!(validate_mimo(&m, &bad).is_err());
    for points in [0, 1, 513] {
        bad = q.clone();
        bad.points = points;
        assert!(validate_mimo(&m, &bad).is_err());
    }
    bad = q.clone();
    bad.outputs = (0..9)
        .map(|component| Channel {
            block: "y1".into(),
            component,
        })
        .collect();
    assert!(validate_mimo(&m, &bad).unwrap_err().contains("1..8"));
    bad = q.clone();
    bad.max_omega = bad.min_omega;
    assert!(validate_mimo(&m, &bad).is_err());
}
#[test]
fn union_clock_and_nonlinearity_guards_apply_across_all_outputs() {
    let (mut m, q) = plan(true);
    m.blocks.iter_mut().find(|b| b.id == "x2").unwrap().params = BlockKind::Delay {
        initial: vec![0.],
        sample_time: 0.2,
    };
    assert!(validate_mimo(&m, &q)
        .unwrap_err()
        .contains("common sample period"));
    let (mut m, q) = plan(false);
    m.blocks
        .iter_mut()
        .find(|b| b.id == "twice")
        .unwrap()
        .params = BlockKind::DeadZone {
        lower: -1.,
        upper: 1.,
    };
    assert!(validate_mimo(&m, &q).is_err());
    let (m, mut q) = plan(true);
    q.max_omega = 32.;
    assert!(validate_mimo(&m, &q).unwrap_err().contains("Nyquist"));
}
#[test]
fn selected_subset_and_zero_state_direct_matrix_retain_zero_paths_and_rank() {
    let (mut m, mut q) = plan(false);
    for b in &mut m.blocks {
        if matches!(b.params, BlockKind::FirstOrder { .. }) {
            b.params = BlockKind::Gain { gain: 1. };
        }
    }
    let r = analyze_mimo(&m, &q).unwrap();
    assert!(r.states.is_empty());
    assert_eq!(r.d, vec![vec![3.25, -1.], vec![0., 2.5]]);
    assert_eq!(r.structure.controllability.status, "no_states");
    assert_eq!(r.responses[2].points[0].real, Some(0.));
    q.inputs.remove(1);
    q.outputs.remove(1);
    let r = analyze_mimo(&m, &q).unwrap();
    assert_eq!(r.d, vec![vec![3.25]]);
}
#[test]
fn singular_frequency_is_missing_not_zero_or_a_partial_success() {
    let (mut m, mut q) = plan(false);
    for b in &mut m.blocks {
        if b.id == "x1" {
            b.params = BlockKind::TransferFunction {
                numerator: vec![1.],
                denominator: vec![1., 0., 1.],
                initial: vec![0., 0.],
            };
        }
    }
    q.min_omega = 1.;
    q.max_omega = 2.;
    q.points = 2;
    let r = analyze_mimo(&m, &q).unwrap();
    for response in &r.responses {
        assert!(response.points[0].error.is_some());
        assert_eq!(response.points[0].real, None);
        assert!(response.points[1].error.is_none());
    }
    assert_eq!(r.states.len(), 3);
}
#[test]
fn distinct_vector_components_keep_four_modes_without_false_full_rank() {
    let (mut m, mut q) = plan(false);
    for b in &mut m.blocks {
        b.width = 2;
        match &mut b.params {
            BlockKind::Step { before, after, .. } => {
                *before = vec![0., 0.];
                *after = vec![1., 2.];
            }
            BlockKind::FirstOrder { initial, .. } => *initial = vec![0., 0.],
            _ => {}
        }
    }
    q.inputs[0].component = 1;
    q.outputs[0].component = 1;
    let r = analyze_mimo(&m, &q).unwrap();
    assert_eq!(r.states.len(), 4);
    assert_eq!(r.structure.controllability.numerical_rank, Some(2));
    assert_eq!(r.structure.observability.numerical_rank, Some(2));
    assert_eq!(r.poles.roots.len(), 4);
    for pair in r.responses.iter().filter(|p| p.input != p.output) {
        assert!(pair
            .points
            .iter()
            .all(|p| p.real == Some(0.) && p.imag == Some(0.)));
    }
}
#[test]
fn state_and_variable_caps_apply_to_union_even_when_each_output_fits() {
    use blocksim_core::graph::{Block, Edge};
    let plan: serde_json::Value = serde_json::from_str(include_str!(
        "../../../samples/state-space/sixteen-integrators-plan.json"
    ))
    .unwrap();
    let mut m: GraphModel = serde_json::from_value(plan["model"].clone()).unwrap();
    m.blocks.push(Block {
        id: "extra".into(),
        label: "extra".into(),
        width: 1,
        position: [0., 0.],
        params: BlockKind::Integrator { initial: vec![0.] },
    });
    m.blocks.push(Block {
        id: "second".into(),
        label: "second".into(),
        width: 1,
        position: [0., 0.],
        params: BlockKind::Scope {},
    });
    for (from, to) in [("source", "extra"), ("extra", "second")] {
        m.edges.push(Edge {
            from: from.into(),
            from_port: 0,
            to: to.into(),
            to_port: 0,
        });
    }
    let (_, mut q) = self::plan(false);
    q.inputs = vec![Channel {
        block: "source".into(),
        component: 0,
    }];
    q.outputs = vec![
        Channel {
            block: "output".into(),
            component: 0,
        },
        Channel {
            block: "second".into(),
            component: 0,
        },
    ];
    assert!(validate_mimo(&m, &q).unwrap_err().contains("16 related"));
    for b in &mut m.blocks {
        if matches!(b.params, BlockKind::Integrator { .. }) {
            b.params = BlockKind::Gain { gain: 1. };
        }
    }
    for branch in 0..2 {
        let scope = if branch == 0 { "output" } else { "second" };
        let edge = m.edges.iter_mut().find(|e| e.to == scope).unwrap();
        let mut previous = edge.from.clone();
        edge.from = format!("chain{branch}_79");
        for k in 0..80 {
            let id = format!("chain{branch}_{k}");
            m.blocks.push(Block {
                id: id.clone(),
                label: id.clone(),
                width: 1,
                position: [0., 0.],
                params: BlockKind::Gain { gain: 1. },
            });
            m.edges.push(Edge {
                from: previous,
                from_port: 0,
                to: id.clone(),
                to_port: 0,
            });
            previous = id;
        }
    }
    assert!(validate_mimo(&m, &q).unwrap_err().contains("128 union"));
}
#[test]
fn maximum_eight_by_eight_sixteen_state_grid_retains_all_paths_and_joint_rank() {
    let m:GraphModel=serde_json::from_value(serde_json::json!({"version":1,"dt":0.01,"duration":1,"blocks":[
 {"id":"u","label":"u","width":8,"position":[0,0],"params":{"kind":"constant","value":[0,0,0,0,0,0,0,0]}},
 {"id":"plant","label":"plant","width":8,"position":[0,0],"params":{"kind":"transfer_function","numerator":[1],"denominator":[1,2,1],"initial":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}},
 {"id":"y","label":"y","width":8,"position":[0,0],"params":{"kind":"scope"}}],"edges":[{"from":"u","from_port":0,"to":"plant","to_port":0},{"from":"plant","from_port":0,"to":"y","to_port":0}]})).unwrap();
    let q = MimoRequest {
        inputs: (0..8)
            .map(|component| Channel {
                block: "u".into(),
                component,
            })
            .collect(),
        outputs: (0..8)
            .map(|component| Channel {
                block: "y".into(),
                component,
            })
            .collect(),
        discrete: false,
        min_omega: 0.1,
        max_omega: 10.,
        points: 512,
    };
    let r = analyze_mimo(&m, &q).unwrap();
    assert_eq!(r.states.len(), 16);
    assert_eq!(r.poles.roots.len(), 16);
    assert_eq!(r.responses.len(), 64);
    assert_eq!(r.structure.controllability.numerical_rank, Some(16));
    assert_eq!(r.structure.observability.numerical_rank, Some(16));
    for pair in r.responses {
        assert_eq!(pair.points.len(), 512);
        for p in pair.points {
            let w = p.omega;
            let (re, im) = if pair.input == pair.output {
                (
                    (1. - w * w) / (1. + w * w).powi(2),
                    -2. * w / (1. + w * w).powi(2),
                )
            } else {
                (0., 0.)
            };
            assert!(p.error.is_none());
            assert!((p.real.unwrap() - re).abs() < 1e-12);
            assert!((p.imag.unwrap() - im).abs() < 1e-12);
        }
    }
}
