use blocksim_core::graph::{simulate_graph, simulate_graph_with_budget, GraphModel, SolverConfig};

#[test]
fn exact_budget_matches_full_results_and_one_less_rejects_both_solvers() {
    let original: GraphModel =
        serde_json::from_str(include_str!("../../../samples/numerics/first-order.json")).unwrap();
    for solver in [original.solver.clone().unwrap(), SolverConfig::Rk4 {}] {
        let mut model = original.clone();
        model.solver = Some(solver);
        let expected = simulate_graph(&model).unwrap();
        let stats = expected.statistics.as_ref().unwrap();
        let cost = (stats.rate_evaluations + stats.output_evaluations) * 3;
        assert_eq!(
            serde_json::to_string(&simulate_graph_with_budget(&model, cost).unwrap()).unwrap(),
            serde_json::to_string(&expected).unwrap()
        );
        assert!(simulate_graph_with_budget(&model, cost - 1)
            .unwrap_err()
            .contains("block-component evaluations"));
        assert!(simulate_graph_with_budget(&model, 0).is_err());
    }
}

#[test]
fn batch_budget_preserves_omitted_solver_json_and_accounts_for_off_grid_events() {
    let mut model: GraphModel =
        serde_json::from_str(include_str!("../../../samples/numerics/off-grid-step.json")).unwrap();
    model.solver = None;
    let exact = 3 * (11 + 11 * 4);
    let result = simulate_graph_with_budget(&model, exact).unwrap();
    assert!(result.statistics.is_none());
    assert_eq!(
        serde_json::to_string(&result).unwrap(),
        serde_json::to_string(&simulate_graph(&model).unwrap()).unwrap()
    );
    assert!(simulate_graph_with_budget(&model, exact - 1).is_err());
}
