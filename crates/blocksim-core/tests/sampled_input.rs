use blocksim_core::graph::{
    discretize::{discretize_tustin, discretize_zoh, TustinRequest, ZohRequest},
    frequency::{analyze_frequency, FrequencyRequest},
    simulate_graph, simulate_graph_with_budget,
    state_space::{analyze_state_space, StateSpaceRequest},
    validate_graph, BlockKind, GraphModel, SolverConfig,
};
fn vector_model() -> GraphModel {
    serde_json::from_value(serde_json::json!({"version":1,"dt":0.01,"duration":1,
        "blocks":[{"id":"source","label":"source","width":2,"position":[30,100],"params":{"kind":"sampled_input","values":[[1,-1],[0.5,2],[0,-0.5]],"sample_time":0.1}},
        {"id":"output","label":"output","width":2,"position":[260,100],"params":{"kind":"scope"}}],
        "edges":[{"from":"source","from_port":0,"to":"output","to_port":0}]})).unwrap()
}
#[test]
fn vector_table_starts_immediately_holds_each_clock_and_repeats_last_row() {
    let m = vector_model();
    let r = simulate_graph(&m).unwrap();
    let rows = [vec![1.0, -1.0], vec![0.5, 2.0], vec![0.0, -0.5]];
    for (i, row) in r.traces[0].values.iter().enumerate() {
        assert_eq!(*row, rows[(i / 10).min(2)]);
    }
    assert_eq!(r.times[30], 0.3);
}
#[test]
fn adaptive_rejection_does_not_advance_table_and_direct_term_changes_on_clock() {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/sequence.json")).unwrap();
    m.dt = 0.01;
    m.duration = 1.0;
    m.solver = Some(SolverConfig::Rk45 {
        abs_tolerance: 1e-12,
        rel_tolerance: 1e-12,
        min_step: 1e-13,
        max_step: 0.8,
    });
    let rows = [0.0, 1.0, -0.5, 0.25];
    m.blocks[0].params = BlockKind::SampledInput {
        values: rows.iter().map(|v| vec![*v]).collect(),
        repeat: false,
        sample_time: 0.1,
    };
    m.blocks[1].params = BlockKind::TransferFunction {
        numerator: vec![0.01, 2.0],
        denominator: vec![0.01, 1.0],
        initial: vec![0.0],
    };
    let r = simulate_graph(&m).unwrap();
    assert!(r.statistics.as_ref().unwrap().rejected_steps > 0);
    let input = r.traces.iter().find(|t| t.id == "input_record").unwrap();
    for i in 0..r.times.len() {
        let index = (i / 10).min(3);
        assert_eq!(input.values[i][0], rows[index]);
        let t = r.times[i];
        let mut state = 0.0;
        for k in 1..=index {
            state += (rows[k] - rows[k - 1]) * (1.0 - (-(t - (k * 10) as f64 * m.dt) / 0.01).exp());
        }
        assert!(
            (r.traces[0].values[i][0] - rows[index] - state).abs() < 2e-9,
            "i={i} t={t} actual={} exact={} error={}",
            r.traces[0].values[i][0],
            rows[index] + state,
            (r.traces[0].values[i][0] - rows[index] - state).abs()
        );
    }
}
#[test]
fn table_rejects_empty_ragged_nonfinite_oversized_and_noninteger_clock() {
    let original = vector_model();
    for (values, ts) in [
        (vec![], 0.1),
        (vec![vec![1.0]], 0.1),
        (vec![vec![f64::NAN, 0.0]], 0.1),
        (vec![vec![0.0; 2]; 2002], 0.1),
        (vec![vec![0.0; 2]], 0.015),
        (vec![vec![0.0; 2]], 0.0),
    ] {
        let mut m = original.clone();
        m.blocks[0].params = BlockKind::SampledInput {
            values,
            repeat: false,
            sample_time: ts,
        };
        assert!(validate_graph(&m).is_err());
    }
}
#[test]
fn aggregate_input_storage_is_bounded_before_simulation() {
    let mut m = vector_model();
    m.blocks[0].width = 64;
    m.blocks[1].width = 64;
    m.blocks[0].params = BlockKind::SampledInput {
        values: vec![vec![0.0; 64]; 2001],
        repeat: false,
        sample_time: 0.1,
    };
    for k in 1..=16 {
        let mut b = m.blocks[0].clone();
        b.id = format!("source{k}");
        m.blocks.push(b);
    }
    assert!(validate_graph(&m).unwrap_err().contains("2000000 cells"));
}
#[test]
fn transfer_analysis_treats_table_as_external_input_instead_of_its_spectrum() {
    let m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/sequence.json")).unwrap();
    let q:FrequencyRequest=serde_json::from_str(r#"{"source":"source","source_component":0,"scope":"output","scope_component":0,"min_omega":1,"max_omega":2,"points":2}"#).unwrap();
    let f = analyze_frequency(&m, &q).unwrap();
    assert!((f.points[0].imag.unwrap() + 0.5).abs() < 1e-12);
    let q = StateSpaceRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
    };
    let a = analyze_state_space(&m, &q).unwrap();
    let mut other = m.clone();
    other.blocks[0].params = BlockKind::Constant { value: vec![999.0] };
    let b = analyze_state_space(&other, &q).unwrap();
    assert_eq!(a.a, b.a);
    assert_eq!(a.b, b.b);
    assert_eq!(a.c, b.c);
    assert_eq!(a.d, b.d);
}
#[test]
fn arbitrary_table_zoh_and_tustin_match_independent_cascade_and_iir_equations() {
    let original: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/sequence.json")).unwrap();
    let a = simulate_graph_with_budget(&original, 10_000_000).unwrap();
    let input = a.traces.iter().find(|t| t.id == "input_record").unwrap();
    for method in ["zoh", "tustin"] {
        let mut m = original.clone();
        let (num, den) = if method == "zoh" {
            let r = discretize_zoh(&ZohRequest {
                numerator: vec![1.0],
                denominator: vec![1.0, 2.0, 1.0],
                sample_time: 0.2,
            })
            .unwrap();
            (r.numerator, r.denominator)
        } else {
            let r = discretize_tustin(&TustinRequest {
                numerator: vec![1.0],
                denominator: vec![1.0, 2.0, 1.0],
                sample_time: 0.2,
                prewarp_omega: None,
            })
            .unwrap();
            (r.numerator, r.denominator)
        };
        m.blocks[1].params = BlockKind::DiscreteTransferFunction {
            numerator: num,
            denominator: den,
            initial: vec![0.0; 2],
            sample_time: 0.2,
        };
        let b = simulate_graph_with_budget(&m, 10_000_000).unwrap();
        assert_eq!(
            input.values,
            b.traces
                .iter()
                .find(|t| t.id == "input_record")
                .unwrap()
                .values
        );
        let r = (-0.2_f64).exp();
        let (mut x1, mut x2, mut y1, mut y2, mut u1, mut u2) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for k in 0..=100 {
            let u = input.values[k * 10][0];
            let y = 18.0 / 11.0 * y1 - 81.0 / 121.0 * y2 + (u + 2.0 * u1 + u2) / 121.0;
            assert!((a.traces[0].values[k * 10][0] - x2).abs() < 2e-10);
            assert!(
                (b.traces[0].values[k * 10][0] - if method == "zoh" { x2 } else { y }).abs()
                    < 2e-13
            );
            x2 = r * x2 + 0.2 * r * x1 + (1.0 - 1.2 * r) * u;
            x1 = r * x1 + (1.0 - r) * u;
            (y2, y1, u2, u1) = (y1, y, u1, u);
        }
    }
}

#[test]
fn repeat_vector_wraps_on_integer_clock_and_one_row_stays_constant() {
    let mut m = vector_model();
    if let BlockKind::SampledInput { repeat, .. } = &mut m.blocks[0].params {
        *repeat = true;
    }
    for solver in [
        None,
        Some(SolverConfig::Rk45 {
            abs_tolerance: 1e-11,
            rel_tolerance: 1e-11,
            min_step: 1e-13,
            max_step: 0.8,
        }),
    ] {
        m.solver = solver;
        let r = simulate_graph(&m).unwrap();
        let rows = [vec![1.0, -1.0], vec![0.5, 2.0], vec![0.0, -0.5]];
        for (i, v) in r.traces[0].values.iter().enumerate() {
            assert_eq!(*v, rows[(i / 10) % 3]);
        }
        assert_eq!(r.traces[0].values[30], rows[0]);
    }
    m.blocks[0].params = BlockKind::SampledInput {
        values: vec![vec![0.75, -0.25]],
        repeat: true,
        sample_time: 0.1,
    };
    let r = simulate_graph(&m).unwrap();
    assert!(r.traces[0].values.iter().all(|v| *v == vec![0.75, -0.25]));
}
#[test]
fn absent_or_false_repeat_preserves_legacy_model_and_rejects_nonboolean() {
    let m = vector_model();
    let old = serde_json::to_value(&m).unwrap();
    assert!(old["blocks"][0]["params"].get("repeat").is_none());
    let mut json = old.clone();
    json["blocks"][0]["params"]["repeat"] = false.into();
    let decoded: GraphModel = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded.clone()).unwrap(), old);
    assert_eq!(
        serde_json::to_value(simulate_graph(&decoded).unwrap()).unwrap(),
        serde_json::to_value(simulate_graph(&m).unwrap()).unwrap()
    );
    for invalid in [
        serde_json::json!("true"),
        serde_json::json!(1),
        serde_json::Value::Null,
    ] {
        json["blocks"][0]["params"]["repeat"] = invalid;
        assert!(serde_json::from_value::<GraphModel>(json.clone()).is_err());
    }
}
#[test]
fn repeating_table_does_not_reset_continuous_state_or_advance_on_rejection() {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/sequence.json")).unwrap();
    m.dt = 0.01;
    m.duration = 1.0;
    m.solver = Some(SolverConfig::Rk45 {
        abs_tolerance: 1e-12,
        rel_tolerance: 1e-12,
        min_step: 1e-13,
        max_step: 0.8,
    });
    let rows = [0.0, 1.0, -0.5];
    m.blocks[0].params = BlockKind::SampledInput {
        values: rows.iter().map(|v| vec![*v]).collect(),
        repeat: true,
        sample_time: 0.1,
    };
    m.blocks[1].params = BlockKind::TransferFunction {
        numerator: vec![0.01, 2.0],
        denominator: vec![0.01, 1.0],
        initial: vec![0.0],
    };
    let r = simulate_graph(&m).unwrap();
    assert!(r.statistics.as_ref().unwrap().rejected_steps > 0);
    let input = r.traces.iter().find(|t| t.id == "input_record").unwrap();
    for i in 0..r.times.len() {
        let index = i / 10;
        let u = rows[index % 3];
        assert_eq!(input.values[i][0], u);
        let mut state = 0.0;
        for k in 1..=index {
            state += (rows[k % 3] - rows[(k - 1) % 3])
                * (1.0 - (-(r.times[i] - (k * 10) as f64 * m.dt) / 0.01).exp());
        }
        assert!((r.traces[0].values[i][0] - u - state).abs() < 2e-9, "i={i}");
    }
}
