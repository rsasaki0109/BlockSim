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
fn edge(from: &str, to: &str, to_port: usize) -> Edge {
    Edge {
        from: from.into(),
        from_port: 0,
        to: to.into(),
        to_port,
    }
}
fn model(blocks: Vec<Block>, edges: Vec<Edge>) -> GraphModel {
    GraphModel {
        version: 1,
        dt: 0.01,
        duration: 1.0,
        blocks,
        edges,
    }
}
#[test]
fn complete_samples_have_no_issues_and_diagnosis_preserves_models_and_results() {
    for name in ["continuous-pid", "mixed-pid", "vector", "subsystem"] {
        let graph: GraphModel = serde_json::from_str(
            &std::fs::read_to_string(format!("../../samples/graphs/{name}.json")).unwrap(),
        )
        .unwrap();
        let before = graph.clone();
        let result = serde_json::to_value(simulate_graph(&graph).unwrap()).unwrap();
        let report = diagnose_graph(&graph);
        assert!(report.runnable && report.diagnostics.is_empty());
        assert_eq!(graph, before);
        assert_eq!(
            serde_json::to_value(simulate_graph(&graph).unwrap()).unwrap(),
            result
        );
    }
}
#[test]
fn collects_all_missing_inputs_and_matches_execution_rejection_until_repaired() {
    let mut graph = model(
        vec![
            block("one", BlockKind::Constant { value: vec![1.0] }),
            block(
                "sum",
                BlockKind::Sum {
                    signs: vec![1.0, -1.0],
                },
            ),
            block("scope", BlockKind::Scope {}),
        ],
        vec![edge("one", "sum", 0)],
    );
    let report = diagnose_graph(&graph);
    assert!(!report.runnable);
    assert_eq!(
        report
            .diagnostics
            .iter()
            .map(|d| (d.block_id.as_deref(), d.port, d.code))
            .collect::<Vec<_>>(),
        vec![
            (Some("sum"), Some(1), DiagnosticCode::UnconnectedInput),
            (Some("scope"), Some(0), DiagnosticCode::UnconnectedInput)
        ]
    );
    assert_eq!(
        validate_graph(&graph).unwrap_err(),
        report.diagnostics[0].message
    );
    graph
        .edges
        .extend([edge("one", "sum", 1), edge("sum", "scope", 0)]);
    assert!(diagnose_graph(&graph).runnable);
    assert!(validate_graph(&graph).is_ok());
}
#[test]
fn nested_boundary_diagnostics_use_exact_expanded_ids_and_detect_alias_duplicates() {
    let mut graph: GraphModel =
        serde_json::from_str(include_str!("../../../samples/graphs/subsystem.json")).unwrap();
    graph.edges.retain(|e| e.to != "amplifier");
    let BlockKind::Subsystem { model: inner } = &mut graph.blocks[1].params else {
        panic!()
    };
    inner.edges.retain(|e| e.to != "out");
    let report = diagnose_graph(&graph);
    assert_eq!(
        report
            .diagnostics
            .iter()
            .map(|d| d.block_id.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("amplifier/in"), Some("amplifier/out")]
    );
    // Parent input plus an inner wire to the same Input aliases the same flat port.
    graph.edges.push(edge("wave", "amplifier", 0));
    let BlockKind::Subsystem { model: inner } = &mut graph.blocks[1].params else {
        panic!()
    };
    inner.edges.push(edge("gain", "in", 0));
    let duplicate = diagnose_graph(&graph)
        .diagnostics
        .into_iter()
        .find(|d| d.code == DiagnosticCode::DuplicateInput)
        .unwrap();
    assert_eq!(duplicate.block_id.as_deref(), Some("amplifier/in"));
    assert_eq!(duplicate.port, Some(0));
}
#[test]
fn width_selector_and_partial_mux_checks_include_ports_and_sources() {
    let mut source = block(
        "source",
        BlockKind::Constant {
            value: vec![1.0, 2.0],
        },
    );
    source.width = 2;
    let graph = model(
        vec![
            source.clone(),
            block(
                "sum",
                BlockKind::Sum {
                    signs: vec![1.0, 1.0],
                },
            ),
            block("scope", BlockKind::Scope {}),
        ],
        vec![
            edge("source", "sum", 0),
            edge("source", "sum", 1),
            edge("sum", "scope", 0),
        ],
    );
    let issues = diagnose_graph(&graph).diagnostics;
    assert_eq!(issues.len(), 2);
    assert!(
        issues
            .iter()
            .all(|d| d.code == DiagnosticCode::SignalWidth
                && d.source_id.as_deref() == Some("source"))
    );
    assert_eq!(issues[1].port, Some(1));
    let select = model(
        vec![
            source.clone(),
            block("select", BlockKind::Selector { indices: vec![2] }),
            block("scope", BlockKind::Scope {}),
        ],
        vec![edge("source", "select", 0), edge("select", "scope", 0)],
    );
    assert_eq!(
        diagnose_graph(&select).diagnostics[0].code,
        DiagnosticCode::SelectorIndex
    );
    let mux = model(
        vec![
            source,
            block("mux", BlockKind::Mux { inputs: 2 }),
            block("scope", BlockKind::Scope {}),
        ],
        vec![edge("source", "mux", 0), edge("mux", "scope", 0)],
    );
    let issues = diagnose_graph(&mux).diagnostics;
    assert!(issues
        .iter()
        .any(|d| d.code == DiagnosticCode::UnconnectedInput));
    assert!(issues.iter().any(|d| d.code == DiagnosticCode::MuxWidth));
}
#[test]
fn cycles_blame_only_participants_and_state_outputs_break_dependencies() {
    let mut graph = model(
        vec![
            block("loop", BlockKind::Gain { gain: 1.0 }),
            block("scope", BlockKind::Scope {}),
        ],
        vec![edge("loop", "loop", 0), edge("loop", "scope", 0)],
    );
    let report = diagnose_graph(&graph);
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].code, DiagnosticCode::AlgebraicLoop);
    assert_eq!(report.diagnostics[0].block_id.as_deref(), Some("loop"));
    for kind in [
        BlockKind::Integrator { initial: vec![0.0] },
        BlockKind::FirstOrder {
            gain: 1.0,
            tau: 1.0,
            initial: vec![0.0],
        },
        BlockKind::Delay {
            initial: vec![0.0],
            sample_time: 0.01,
        },
    ] {
        graph.blocks[0].params = kind;
        assert!(diagnose_graph(&graph).runnable);
        assert!(validate_graph(&graph).is_ok());
    }
}
#[test]
fn reports_global_structure_scope_and_allocation_budgets_without_simulating() {
    let empty = model(vec![], vec![]);
    assert_eq!(
        diagnose_graph(&empty).diagnostics[0].code,
        DiagnosticCode::EmptyGraph
    );
    let mut graph = model(
        vec![block("one", BlockKind::Constant { value: vec![1.0] })],
        vec![],
    );
    assert_eq!(
        diagnose_graph(&graph).diagnostics[0].code,
        DiagnosticCode::MissingScope
    );
    graph.dt = 0.0;
    let report = diagnose_graph(&graph);
    assert!(!report.runnable);
    assert_eq!(report.diagnostics[0].code, DiagnosticCode::Structure);
    assert_eq!(report.diagnostics[0].block_id, None);
    let mut graph = model(
        vec![
            block(
                "one",
                BlockKind::Constant {
                    value: vec![1.0; 64],
                },
            ),
            block("a", BlockKind::Gain { gain: 1.0 }),
            block("b", BlockKind::Gain { gain: 1.0 }),
            block("scope", BlockKind::Scope {}),
        ],
        vec![
            edge("one", "a", 0),
            edge("a", "b", 0),
            edge("b", "scope", 0),
        ],
    );
    for b in &mut graph.blocks {
        b.width = 64;
    }
    graph.duration = 1000.0;
    let issues = diagnose_graph(&graph).diagnostics;
    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].code, DiagnosticCode::WorkLimit);
    assert_eq!(issues[1].code, DiagnosticCode::RecordingLimit);
    assert!(issues.iter().all(|d| d.block_id.is_none()));
    assert!(simulate_graph(&graph).is_err());
}
