use blocksim_core::graph::{
    frequency::{analyze_frequency, validate_frequency, FrequencyRequest},
    BlockKind, GraphModel,
};
fn model() -> GraphModel {
    serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json")).unwrap()
}
fn request() -> FrequencyRequest {
    FrequencyRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
        min_omega: 0.01,
        max_omega: 100.0,
        points: 201,
    }
}
#[test]
fn second_order_response_matches_every_complex_analytic_value_and_unwrapped_phase() {
    let m = model();
    let before = serde_json::to_string(&m).unwrap();
    let r = analyze_frequency(&m, &request()).unwrap();
    assert_eq!(r.points.len(), 201);
    assert_eq!(r.points[0].omega, 0.01);
    assert_eq!(r.points[200].omega, 100.0);
    for p in r.points {
        let w = p.omega;
        let a = 1.0 - w * w;
        let b = 2.0 * w;
        let den = a * a + b * b;
        assert!(p.error.is_none());
        assert!((p.real.unwrap() - a / den).abs() < 1e-12);
        assert!((p.imag.unwrap() + b / den).abs() < 1e-12);
        assert!((p.magnitude_db.unwrap() + 20.0 * (1.0 + w * w).log10()).abs() < 1e-10);
        assert!((p.phase_deg.unwrap() + 2.0 * w.atan().to_degrees()).abs() < 1e-10);
        assert!(p.relative_residual.unwrap() < 1e-12);
    }
    assert_eq!(serde_json::to_string(&m).unwrap(), before);
    let expected = analyze_frequency(&m, &request()).unwrap();
    let mut scaled = m.clone();
    scaled.blocks[1].params=serde_json::from_value(serde_json::json!({"kind":"transfer_function","numerator":[1e307],"denominator":[1e307,2e307,1e307],"initial":[0,0]})).unwrap();
    assert_eq!(
        serde_json::to_string(&expected.points).unwrap(),
        serde_json::to_string(&analyze_frequency(&scaled, &request()).unwrap().points).unwrap()
    );
}
#[test]
fn closed_loop_continuous_pi_and_plant_match_rational_analytic_response() {
    let m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/frequency/feedback.json")).unwrap();
    let mut q = request();
    q.source = "reference".into();
    let r = analyze_frequency(&m, &q).unwrap();
    for p in r.points {
        let w = p.omega;
        let a = 1.0 - w * w;
        let b = 3.0 * w;
        let den = a * a + b * b;
        assert!(p.error.is_none());
        assert!((p.real.unwrap() - (a + 2.0 * w * b) / den).abs() < 1e-11);
        assert!((p.imag.unwrap() - (2.0 * w * a - b) / den).abs() < 1e-11);
    }
}
#[test]
fn third_order_phase_unwrap_crosses_minus_180_degrees() {
    let mut m = model();
    m.blocks[1].params=serde_json::from_value(serde_json::json!({"kind":"transfer_function","numerator":[1],"denominator":[1,3,3,1],"initial":[0,0,0]})).unwrap();
    let r = analyze_frequency(&m, &request()).unwrap();
    for p in r.points {
        assert!(p.error.is_none());
        assert!((p.phase_deg.unwrap() + 3.0 * p.omega.atan().to_degrees()).abs() < 1e-10);
    }
}
#[test]
fn unlimited_filtered_pid_response_includes_integral_and_derivative_terms() {
    let mut m = model();
    m.blocks[1].params=serde_json::from_value(serde_json::json!({"kind":"pid","mode":"continuous","sample_time":0.01,"config":{"kp":2,"ki":1,"kd":0.5,"derivative_tau":0.2,"limits":{"lower":null,"upper":null},"anti_windup":true}})).unwrap();
    for p in analyze_frequency(&m, &request()).unwrap().points {
        let w = p.omega;
        let den = 1.0 + 0.04 * w * w;
        assert!(p.error.is_none());
        assert!((p.real.unwrap() - (2.0 + 0.1 * w * w / den)).abs() < 1e-11);
        assert!((p.imag.unwrap() - (0.5 * w / den - 1.0 / w)).abs() < 1e-11);
    }
}
#[test]
fn harmonic_time_response_converges_to_the_independent_frequency_prediction() {
    let mut m = model();
    m.duration = 80.0;
    m.blocks[0].params=serde_json::from_value(serde_json::json!({"kind":"sine","amplitude":1,"frequency":1.0/std::f64::consts::TAU,"phase":0,"offset":0})).unwrap();
    let mut q = request();
    q.min_omega = 1.0;
    q.max_omega = 2.0;
    q.points = 2;
    let p = analyze_frequency(&m, &q).unwrap().points.remove(0);
    let r = blocksim_core::graph::simulate_graph(&m).unwrap();
    for (i, time) in r.times.iter().enumerate().filter(|(_, t)| **t >= 40.0) {
        let expected = p.real.unwrap() * time.sin() + p.imag.unwrap() * time.cos();
        assert!((r.traces[0].values[i][0] - expected).abs() < 5e-8);
    }
}
#[test]
fn component_dependencies_support_nested_paths_mux_selector_and_unrelated_sources() {
    let mut m = model();
    m.blocks[0].width = 2;
    m.blocks[0].params =
        serde_json::from_value(serde_json::json!({"kind":"constant","value":[0,0]})).unwrap();
    m.blocks[1].width = 2;
    m.blocks[1].params=serde_json::from_value(serde_json::json!({"kind":"transfer_function","numerator":[1],"denominator":[1,2,1],"initial":[0,0,0,0]})).unwrap();
    m.blocks[2].width = 3;
    for (id, width, params) in [
        (
            "other",
            1,
            serde_json::json!({"kind":"constant","value":[9]}),
        ),
        ("mux", 3, serde_json::json!({"kind":"mux","inputs":2})),
        (
            "select",
            3,
            serde_json::json!({"kind":"selector","indices":[2,0,1]}),
        ),
    ] {
        m.blocks.push(serde_json::from_value(serde_json::json!({"id":id,"label":id,"width":width,"position":[20,20],"params":params})).unwrap());
    }
    m.edges[1].to = "mux".into();
    for (from, to, port) in [
        ("other", "mux", 1),
        ("mux", "select", 0),
        ("select", "output", 0),
    ] {
        m.edges.push(
            serde_json::from_value(
                serde_json::json!({"from":from,"from_port":0,"to":to,"to_port":port}),
            )
            .unwrap(),
        );
    }
    let mut q = request();
    q.source_component = 1;
    q.scope_component = 2;
    let r = analyze_frequency(&m, &q).unwrap();
    assert_eq!(r.check.variables, 5);
    assert!(r
        .points
        .iter()
        .all(|p| p.error.is_none() && p.magnitude.unwrap() > 0.0));
    q.scope_component = 0;
    assert!(analyze_frequency(&m, &q)
        .unwrap()
        .points
        .iter()
        .all(|p| p.magnitude == Some(0.0)));
    q.source = "other".into();
    q.source_component = 0;
    assert!(analyze_frequency(&m, &q)
        .unwrap()
        .points
        .iter()
        .all(|p| p.magnitude == Some(1.0)));
    let mut nested: GraphModel =
        serde_json::from_str(include_str!("../../../samples/graphs/subsystem.json")).unwrap();
    let BlockKind::Subsystem { model: inner } = &mut nested.blocks[1].params else {
        panic!("sample subsystem");
    };
    inner.blocks.push(serde_json::from_value(serde_json::json!({"id":"wave-output","label":"wave-output","width":1,"position":[10,10],"params":{"kind":"scope"}})).unwrap());
    inner.edges.push(
        serde_json::from_value(
            serde_json::json!({"from":"gain","from_port":0,"to":"wave-output","to_port":0}),
        )
        .unwrap(),
    );
    q = request();
    q.source = nested.blocks[0].id.clone();
    q.scope = "amplifier/wave-output".into();
    let r = analyze_frequency(&nested, &q).unwrap();
    assert_eq!(r.check.variables, 4);
    assert!(r.points.iter().all(|p| p.magnitude == Some(3.0)));
}
#[test]
fn singular_frequencies_and_zero_responses_are_explicit_without_nonfinite_json() {
    let mut m = model();
    m.blocks[1].params=serde_json::from_value(serde_json::json!({"kind":"transfer_function","numerator":[1],"denominator":[1,0,1],"initial":[0,0]})).unwrap();
    let mut q = request();
    q.min_omega = 1.0;
    q.max_omega = 2.0;
    q.points = 2;
    let r = analyze_frequency(&m, &q).unwrap();
    assert!(r.points[0].error.is_some());
    assert!(r.points[1].error.is_none());
    assert!(r.points[0].real.is_none());
    assert!(!serde_json::to_string(&r).unwrap().contains("Infinity"));
    m.blocks[1].params =
        serde_json::from_value(serde_json::json!({"kind":"gain","gain":0})).unwrap();
    let r = analyze_frequency(&m, &q).unwrap();
    assert!(r.points.iter().all(|p| p.magnitude == Some(0.0)
        && p.magnitude_db.is_none()
        && p.phase_deg.is_none()
        && p.error.is_none()));
}
#[test]
fn sampled_and_nonlinear_ancestry_is_refused_but_unrelated_branches_do_not_change_response() {
    let mut m = model();
    for params in [
        serde_json::json!({"kind":"saturation","limits":{"lower":0,"upper":1}}),
        serde_json::json!({"kind":"delay","initial":[0],"sample_time":0.01}),
    ] {
        m.blocks[1].params = serde_json::from_value(params).unwrap();
        assert!(validate_frequency(&m, &request())
            .unwrap_err()
            .contains("plant:"));
    }
    let pid: GraphModel =
        serde_json::from_str(include_str!("../../../samples/graphs/continuous-pid.json")).unwrap();
    let mut q = request();
    q.source = "reference".into();
    assert!(validate_frequency(&pid, &q)
        .unwrap_err()
        .contains("controller:"));
    let mut m = model();
    let before = analyze_frequency(&m, &request()).unwrap();
    m.blocks.push(serde_json::from_value(serde_json::json!({"id":"limit","label":"limit","width":1,"position":[10,10],"params":{"kind":"saturation","limits":{"lower":0,"upper":1}}})).unwrap());
    m.edges.push(
        serde_json::from_value(
            serde_json::json!({"from":"source","from_port":0,"to":"limit","to_port":0}),
        )
        .unwrap(),
    );
    assert_eq!(
        serde_json::to_string(&before.points).unwrap(),
        serde_json::to_string(&analyze_frequency(&m, &request()).unwrap().points).unwrap()
    );
}
#[test]
fn requests_reject_invalid_clocks_components_schema_and_solve_budget() {
    let m = model();
    for n in [0, 1, 2049] {
        let mut q = request();
        q.points = n;
        assert!(validate_frequency(&m, &q).is_err());
    }
    let mut thin = request();
    thin.min_omega = 1e9 - 1e-7;
    thin.max_omega = 1e9;
    thin.points = 2;
    assert!(validate_frequency(&m, &thin)
        .unwrap_err()
        .contains("resolution"));
    let mut q = request();
    q.min_omega = f64::NAN;
    assert!(validate_frequency(&m, &q).is_err());
    q = request();
    q.source_component = 1;
    assert!(validate_frequency(&m, &q).is_err());
    q = request();
    q.scope = "missing".into();
    assert!(validate_frequency(&m, &q).is_err());
    assert!(serde_json::from_value::<FrequencyRequest>(serde_json::json!({"source":"source","source_component":0,"scope":"output","scope_component":0,"min_omega":0.01,"max_omega":100,"points":201,"extra":0})).is_err());
    let mut m = model();
    m.edges[1].from = "g39".into();
    for i in 0..40 {
        let id = format!("g{i}");
        m.blocks.push(serde_json::from_value(serde_json::json!({"id":id,"label":id,"width":1,"position":[10,10],"params":{"kind":"gain","gain":1}})).unwrap());
        m.edges.push(serde_json::from_value(serde_json::json!({"from":if i==0 {"plant".to_string()}else{format!("g{}",i-1)},"from_port":0,"to":id,"to_port":0})).unwrap());
    }
    let mut q = request();
    q.points = 2048;
    assert!(validate_frequency(&m, &q).unwrap_err().contains("budget"));
    q.points = 2;
    assert!(analyze_frequency(&m, &q).is_ok());
}
