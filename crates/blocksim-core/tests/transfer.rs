use blocksim_core::graph::{diagnose_graph, simulate_graph, validate_draft, GraphModel};
fn model() -> GraphModel {
    serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json")).unwrap()
}
fn edit(
    model: &mut GraphModel,
    numerator: serde_json::Value,
    denominator: serde_json::Value,
    initial: serde_json::Value,
) {
    model.blocks[1].params=serde_json::from_value(serde_json::json!({"kind":"transfer_function","numerator":numerator,"denominator":denominator,"initial":initial})).unwrap();
}
#[test]
fn second_order_step_matches_analytic_solution_and_nonmonic_realization() {
    let mut m = model();
    let r = simulate_graph(&m).unwrap();
    for (t, row) in r.times.iter().zip(&r.traces[0].values) {
        assert!((row[0] - (1.0 - (1.0 + t) * (-t).exp())).abs() < 2e-8);
    }
    edit(
        &mut m,
        serde_json::json!([2]),
        serde_json::json!([2, 4, 2]),
        serde_json::json!([0, 0]),
    );
    assert_eq!(
        r.traces[0].values,
        simulate_graph(&m).unwrap().traces[0].values
    );
}
#[test]
fn vector_channels_and_nonzero_initial_states_are_independent() {
    let mut m = model();
    for b in &mut m.blocks {
        b.width = 2;
    }
    m.blocks[0].params =
        serde_json::from_value(serde_json::json!({"kind":"constant","value":[0,0]})).unwrap();
    edit(
        &mut m,
        serde_json::json!([1]),
        serde_json::json!([1, 2, 1]),
        serde_json::json!([0, 1, 0, 2]),
    );
    let r = simulate_graph(&m).unwrap();
    assert_eq!(r.traces[0].values[0], vec![1.0, 2.0]);
    for (t, row) in r.times.iter().zip(&r.traces[0].values) {
        for (c, value) in row.iter().enumerate() {
            assert!((value - (c + 1) as f64 * (1.0 + t) * (-t).exp()).abs() < 2e-8);
        }
    }
}
#[test]
fn direct_feedthrough_and_order_zero_gain_have_correct_outputs() {
    let mut m = model();
    edit(
        &mut m,
        serde_json::json!([1, 2, 3]),
        serde_json::json!([1, 2, 1]),
        serde_json::json!([0, 0]),
    );
    let r = simulate_graph(&m).unwrap();
    assert_eq!(r.traces[0].values[0][0], 1.0);
    for (t, row) in r.times.iter().zip(&r.traces[0].values) {
        assert!((row[0] - (3.0 - 2.0 * (1.0 + t) * (-t).exp())).abs() < 2e-8);
    }
    edit(
        &mut m,
        serde_json::json!([6]),
        serde_json::json!([2]),
        serde_json::json!([]),
    );
    let r = simulate_graph(&m).unwrap();
    assert!(r.traces[0].values.iter().all(|row| row[0] == 3.0));
    assert_eq!(r.statistics.unwrap().rate_evaluations, 0);
}
#[test]
fn strictly_proper_feedback_breaks_the_loop_but_direct_feedthrough_does_not() {
    let mut m = model();
    m.edges[0].from = "plant".into();
    edit(
        &mut m,
        serde_json::json!([1]),
        serde_json::json!([1, 1]),
        serde_json::json!([2]),
    );
    let r = simulate_graph(&m).unwrap();
    assert!(r.traces[0].values.iter().all(|row| row[0] == 2.0));
    edit(
        &mut m,
        serde_json::json!([1, 1]),
        serde_json::json!([1, 1]),
        serde_json::json!([2]),
    );
    assert!(diagnose_graph(&m)
        .diagnostics
        .iter()
        .any(|d| format!("{:?}", d.code) == "AlgebraicLoop"));
}
#[test]
fn invalid_coefficients_states_and_unrepresentable_normalization_fail_before_execution() {
    for (n, d, x) in [
        (
            serde_json::json!([]),
            serde_json::json!([1, 1]),
            serde_json::json!([0]),
        ),
        (
            serde_json::json!([1, 2, 3]),
            serde_json::json!([1, 1]),
            serde_json::json!([0]),
        ),
        (
            serde_json::json!([1]),
            serde_json::json!([0, 1]),
            serde_json::json!([0]),
        ),
        (
            serde_json::json!([1]),
            serde_json::json!([1, 1]),
            serde_json::json!([]),
        ),
        (
            serde_json::json!([1]),
            serde_json::json!([1e-308, 1e308]),
            serde_json::json!([0]),
        ),
    ] {
        let mut m = model();
        edit(&mut m, n, d, x);
        assert!(validate_draft(&m).is_err());
        assert!(simulate_graph(&m).is_err());
    }
    let mut m = model();
    edit(
        &mut m,
        serde_json::json!([1]),
        serde_json::json!(vec![1; 10]),
        serde_json::json!(vec![0; 9]),
    );
    assert!(validate_draft(&m).is_err());
}
