use blocksim_core::graph::{
    frequency::{analyze_frequency, FrequencyRequest},
    state_space::{analyze_state_space, validate_state_space, StateSpaceRequest, StateSpaceResult},
    BlockKind, GraphModel,
};
fn feedback() -> GraphModel {
    serde_json::from_str(include_str!("../../../samples/state-space/feedback.json")).unwrap()
}
fn query() -> StateSpaceRequest {
    StateSpaceRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
    }
}
// Independent real 2n-by-2n solve for (jωI-A)x=B; graph Bode uses complex nodal equations.
fn response(r: &StateSpaceResult, w: f64) -> (f64, f64) {
    let n = r.b.len();
    if n == 0 {
        return (r.d, 0.0);
    }
    let m = 2 * n;
    let mut matrix = vec![vec![0.0; m + 1]; m];
    for i in 0..n {
        for j in 0..n {
            matrix[i][j] = -r.a[i][j];
            matrix[n + i][n + j] = -r.a[i][j];
        }
        matrix[i][n + i] = -w;
        matrix[n + i][i] = w;
        matrix[i][m] = r.b[i];
    }
    for k in 0..m {
        let pivot = (k..m)
            .max_by(|i, j| matrix[*i][k].abs().total_cmp(&matrix[*j][k].abs()))
            .unwrap();
        matrix.swap(k, pivot);
        assert!(matrix[k][k].abs() > 1e-12);
        for i in k + 1..m {
            let factor = matrix[i][k] / matrix[k][k];
            for j in k..=m {
                matrix[i][j] -= factor * matrix[k][j];
            }
        }
    }
    let mut x = vec![0.0; m];
    for i in (0..m).rev() {
        x[i] = (matrix[i][m] - (i + 1..m).map(|j| matrix[i][j] * x[j]).sum::<f64>()) / matrix[i][i];
    }
    (
        r.d + r.c.iter().zip(&x[..n]).map(|(c, x)| c * x).sum::<f64>(),
        r.c.iter().zip(&x[n..]).map(|(c, x)| c * x).sum(),
    )
}
fn matches_bode(m: &GraphModel, q: &StateSpaceRequest, r: &StateSpaceResult) {
    let f = FrequencyRequest {
        source: q.source.clone(),
        source_component: q.source_component,
        scope: q.scope.clone(),
        scope_component: q.scope_component,
        min_omega: 0.01,
        max_omega: 100.0,
        points: 201,
        open_loop: false,
    };
    for p in analyze_frequency(m, &f).unwrap().points {
        let (re, im) = response(r, p.omega);
        assert!(p.error.is_none());
        assert!((re - p.real.unwrap()).hypot(im - p.imag.unwrap()) < 1e-9);
    }
}
#[test]
fn actual_connected_feedback_extracts_exact_matrices_and_detects_stable_boundary_unstable() {
    for (gain, class, right) in [
        (4.0, "stable", 0),
        (6.0, "boundary_or_uncertain", 0),
        (8.0, "unstable", 2),
    ] {
        let mut m = feedback();
        m.blocks[2].params = BlockKind::Gain { gain };
        let before = serde_json::to_string(&m).unwrap();
        let r = analyze_state_space(&m, &query()).unwrap();
        assert_eq!(
            r.a,
            vec![
                vec![-3.0, -2.0, -gain],
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0]
            ]
        );
        assert_eq!(r.b, vec![gain, 0.0, 0.0]);
        assert_eq!(r.c, vec![0.0, 0.0, 1.0]);
        assert_eq!(r.d, 0.0);
        assert_eq!(r.check.states, 3);
        assert_eq!(r.stability.classification, class);
        assert_eq!(r.stability.right_half_plane_poles, Some(right));
        assert_eq!(r.poles.status, "verified_residual");
        assert_eq!(r.poles.roots.len(), 3);
        assert!(r.matrix_residuals.iter().all(|r| *r < 1e-10));
        matches_bode(&m, &query(), &r);
        assert_eq!(serde_json::to_string(&m).unwrap(), before);
    }
}
#[test]
fn continuous_pi_retains_unobservable_filter_and_matches_analytic_closed_loop() {
    let m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/frequency/feedback.json")).unwrap();
    let mut q = query();
    q.source = "reference".into();
    let r = analyze_state_space(&m, &q).unwrap();
    assert_eq!(
        r.a,
        vec![
            vec![-3.0, 1.0, 0.0],
            vec![-1.0, 0.0, 0.0],
            vec![-20.0, 0.0, -20.0]
        ]
    );
    assert_eq!(r.b, vec![2.0, 1.0, 20.0]);
    assert_eq!(r.c, vec![1.0, 0.0, 0.0]);
    assert_eq!(r.stability.classification, "stable");
    assert_eq!(r.poles.roots.len(), 3);
    for expected in [
        -20.0,
        (-3.0 - 5.0_f64.sqrt()) / 2.0,
        (-3.0 + 5.0_f64.sqrt()) / 2.0,
    ] {
        assert!(r
            .poles
            .roots
            .iter()
            .any(|p| (p.real - expected).abs() < 1e-9 && p.imag.abs() < 1e-9));
    }
    assert!(r.states.iter().any(|s| s.description == "PID error filter"));
    matches_bode(&m, &q, &r);
    q.scope = "control".into();
    let control = analyze_state_space(&m, &q).unwrap();
    assert_eq!(control.c, vec![1.0, 0.0, -2.0]);
    assert_eq!(control.d, 2.0);
    matches_bode(&m, &q, &control);
    let mut filtered = m.clone();
    if let BlockKind::Pid { config, .. } = &mut filtered.blocks[2].params {
        config.kd = 0.3;
    }
    q.scope = "output".into();
    let r = analyze_state_space(&filtered, &q).unwrap();
    for (actual, expected) in
        r.a.iter()
            .flatten()
            .zip([-9.0, 1.0, -6.0, -1.0, 0.0, 0.0, -20.0, 0.0, -20.0])
    {
        assert!((actual - expected).abs() < 1e-12);
    }
    for (actual, expected) in r.b.iter().zip([8.0, 1.0, 20.0]) {
        assert!((actual - expected).abs() < 1e-12);
    }
    assert_eq!(r.stability.classification, "stable");
    matches_bode(&filtered, &q, &r);
}
#[test]
fn hidden_cancelled_or_zero_gain_modes_remain_in_internal_stability() {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json")).unwrap();
    m.blocks[1].params = BlockKind::TransferFunction {
        numerator: vec![1.0, -1.0],
        denominator: vec![1.0, 1.0, -2.0],
        initial: vec![0.0; 2],
    };
    let r = analyze_state_space(&m, &query()).unwrap();
    assert_eq!(r.stability.classification, "unstable");
    assert_eq!(r.check.states, 2);
    matches_bode(&m, &query(), &r);
    let mut b = m.blocks[0].clone();
    b.id = "zero".into();
    b.params = BlockKind::Gain { gain: 0.0 };
    m.blocks.push(b);
    m.edges[1].from = "zero".into();
    let mut e = m.edges[0].clone();
    e.from = "plant".into();
    e.to = "zero".into();
    m.edges.push(e);
    let r = analyze_state_space(&m, &query()).unwrap();
    assert_eq!(r.c, vec![0.0; 2]);
    assert_eq!(r.stability.classification, "unstable");
    assert_eq!(r.stability.right_half_plane_poles, Some(1));
}
#[test]
fn related_nonlinear_and_discrete_blocks_are_refused_but_unrelated_branches_excluded() {
    let mut m = feedback();
    let mut b = m.blocks[0].clone();
    b.id = "unrelated".into();
    b.params = BlockKind::TransferFunction {
        numerator: vec![1.0],
        denominator: vec![1.0, -1.0],
        initial: vec![0.0],
    };
    m.blocks.push(b);
    let mut e = m.edges[0].clone();
    e.to = "unrelated".into();
    m.edges.push(e);
    let mut nonlinear = m.blocks[0].clone();
    nonlinear.id = "unrelated-nonlinear".into();
    nonlinear.params = BlockKind::Saturation {
        limits: blocksim_core::OutputLimits {
            lower: Some(1.0),
            upper: Some(2.0),
        },
    };
    m.blocks.push(nonlinear);
    let mut e = m.edges[0].clone();
    e.from = "unrelated".into();
    e.to = "unrelated-nonlinear".into();
    m.edges.push(e);
    let r = analyze_state_space(&m, &query()).unwrap();
    assert_eq!(r.check.states, 3);
    assert!(!r.states.iter().any(|s| s.block == "unrelated"));
    m.blocks[2].params = BlockKind::Saturation {
        limits: blocksim_core::OutputLimits {
            lower: Some(0.0),
            upper: Some(1.0),
        },
    };
    assert!(validate_state_space(&m, &query())
        .unwrap_err()
        .contains("gain"));
    m.blocks[2].params = BlockKind::Delay {
        sample_time: 0.01,
        initial: vec![0.0],
    };
    assert!(analyze_state_space(&m, &query()).is_err());
}
#[test]
fn direct_feedthrough_zero_states_and_an_unrelated_input_are_explicit() {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json")).unwrap();
    m.blocks[1].params = BlockKind::TransferFunction {
        numerator: vec![2.0, 1.0],
        denominator: vec![1.0, 1.0],
        initial: vec![0.0],
    };
    let r = analyze_state_space(&m, &query()).unwrap();
    assert_eq!(r.a, vec![vec![-1.0]]);
    assert_eq!(r.b, vec![1.0]);
    assert_eq!(r.c, vec![-1.0]);
    assert_eq!(r.d, 2.0);
    matches_bode(&m, &query(), &r);
    m.blocks[1].params = BlockKind::Gain { gain: 3.0 };
    let r = analyze_state_space(&m, &query()).unwrap();
    assert!(r.a.is_empty() && r.b.is_empty() && r.c.is_empty());
    assert_eq!(r.d, 3.0);
    assert_eq!(r.stability.classification, "stable");
    assert!(r.poles.roots.is_empty());
    let mut b = m.blocks[0].clone();
    b.id = "other".into();
    m.blocks.push(b);
    let mut q = query();
    q.source = "other".into();
    let r = analyze_state_space(&m, &q).unwrap();
    assert!(!r.check.source_related);
    assert_eq!(r.d, 0.0);
}
#[test]
fn vector_mux_selector_and_nested_ids_extract_only_the_selected_ancestor_components() {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json")).unwrap();
    for b in &mut m.blocks {
        b.width = 2;
    }
    m.blocks[0].params = BlockKind::Constant {
        value: vec![7.0, 9.0],
    };
    m.blocks[1].params = BlockKind::TransferFunction {
        numerator: vec![1.0],
        denominator: vec![1.0, 2.0, 1.0],
        initial: vec![1.0, 2.0, 3.0, 4.0],
    };
    let mut q = query();
    q.source_component = 1;
    q.scope_component = 1;
    let r = analyze_state_space(&m, &q).unwrap();
    assert_eq!(r.check.states, 2);
    assert!(r.states.iter().all(|s| s.component == 1));
    matches_bode(&m, &q, &r);
    q.source_component = 0;
    let r = analyze_state_space(&m, &q).unwrap();
    assert!(!r.check.source_related);
    assert_eq!(r.b, vec![0.0; 2]);
    assert_eq!(r.d, 0.0);
    // Child output selects component1; nested boundaries/Mux/Selector retain its exact ID.
    m.blocks[2].width = 1;
    m.blocks[2].params = BlockKind::Output { port: 0 };
    let mut selector = m.blocks[2].clone();
    selector.id = "select".into();
    selector.params = BlockKind::Selector { indices: vec![1] };
    m.blocks.push(selector);
    m.edges[1].to = "select".into();
    let mut e = m.edges[1].clone();
    e.from = "select".into();
    e.to = "output".into();
    m.edges.push(e);
    let mut other = m.blocks[2].clone();
    other.id = "other-input".into();
    other.params = BlockKind::Constant { value: vec![10.0] };
    m.blocks.push(other);
    let mut mux = m.blocks[2].clone();
    mux.id = "combine".into();
    mux.width = 3;
    mux.params = BlockKind::Mux { inputs: 2 };
    m.blocks.push(mux);
    m.blocks[3].params = BlockKind::Selector { indices: vec![2] };
    m.edges[1].to = "combine".into();
    m.edges[1].to_port = 1;
    let mut e = m.edges[1].clone();
    e.from = "other-input".into();
    e.to_port = 0;
    m.edges.push(e);
    let mut e = m.edges[1].clone();
    e.from = "combine".into();
    e.to = "select".into();
    e.to_port = 0;
    m.edges.push(e);
    let mut parent = feedback();
    let mut b = parent.blocks[0].clone();
    b.id = "nested".into();
    b.params = BlockKind::Subsystem { model: Box::new(m) };
    parent.blocks.push(b);
    let mut scope = parent.blocks[4].clone();
    scope.id = "nested-output".into();
    parent.blocks.push(scope);
    let mut e = parent.edges[4].clone();
    e.from = "nested".into();
    e.to = "nested-output".into();
    parent.edges.push(e);
    let q = StateSpaceRequest {
        source: "nested/source".into(),
        source_component: 1,
        scope: "nested-output".into(),
        scope_component: 0,
    };
    let r = analyze_state_space(&parent, &q).unwrap();
    assert_eq!(r.check.states, 2);
    assert!(r
        .states
        .iter()
        .all(|s| s.block == "nested/plant" && s.component == 1));
    matches_bode(&parent, &q, &r);
}
#[test]
fn sixteen_state_cap_component_schema_and_invalid_queries_fail_before_analysis() {
    let mut m = feedback();
    m.blocks[3].params = BlockKind::TransferFunction {
        numerator: vec![1.0],
        denominator: vec![1.0; 9],
        initial: vec![0.0; 8],
    };
    assert_eq!(validate_state_space(&m, &query()).unwrap().states, 8);
    let mut b = m.blocks[0].clone();
    b.id = "extra".into();
    b.params = BlockKind::TransferFunction {
        numerator: vec![1.0],
        denominator: vec![1.0; 9],
        initial: vec![0.0; 8],
    };
    m.blocks.push(b);
    m.edges[3].to = "extra".into();
    let mut e = m.edges[3].clone();
    e.from = "extra".into();
    e.to = "plant".into();
    m.edges.push(e);
    assert_eq!(validate_state_space(&m, &query()).unwrap().states, 16);
    let mut extra = m.blocks[0].clone();
    extra.id = "over".into();
    extra.params = BlockKind::Integrator { initial: vec![0.] };
    m.blocks.push(extra);
    m.edges[3].to = "over".into();
    let mut edge = m.edges[3].clone();
    edge.from = "over".into();
    edge.to = "extra".into();
    m.edges.push(edge);
    assert!(validate_state_space(&m, &query())
        .unwrap_err()
        .contains("16 related"));
    let mut q = query();
    q.source = "plant".into();
    assert!(validate_state_space(&feedback(), &q).is_err());
    q = query();
    q.scope_component = 1;
    assert!(validate_state_space(&feedback(), &q).is_err());
    assert!(serde_json::from_str::<StateSpaceRequest>(r#"{"source":"source","source_component":0,"scope":"output","scope_component":0,"extra":true}"#).is_err());
}
#[test]
fn extreme_matrix_scaling_remains_unresolved_with_finite_matrices_and_no_partial_poles() {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json")).unwrap();
    m.blocks[1].params = BlockKind::TransferFunction {
        numerator: vec![1.0],
        denominator: vec![1.0, 1e308, 1e-100],
        initial: vec![0.0; 2],
    };
    let r = analyze_state_space(&m, &query()).unwrap();
    assert_eq!(r.poles.status, "unresolved");
    assert!(r.poles.roots.is_empty());
    assert_eq!(r.stability.classification, "indeterminate");
    assert!(r.characteristic.is_none());
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains("Infinity") && !json.contains("NaN"));
}
#[test]
fn eight_connected_first_order_states_have_known_distinct_eigenvalues() {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json")).unwrap();
    m.blocks.remove(1);
    m.edges.clear();
    for i in 0..8 {
        let mut b = m.blocks[0].clone();
        b.id = format!("stage{i}");
        b.params = BlockKind::FirstOrder {
            gain: 1.0,
            tau: (i + 1) as f64,
            initial: vec![0.0],
        };
        m.blocks.push(b);
        let mut e = feedback().edges[0].clone();
        e.from = if i == 0 {
            "source".into()
        } else {
            format!("stage{}", i - 1)
        };
        e.to = format!("stage{i}");
        m.edges.push(e);
    }
    let mut e = feedback().edges[0].clone();
    e.from = "stage7".into();
    e.to = "output".into();
    m.edges.push(e);
    let r = analyze_state_space(&m, &query()).unwrap();
    assert_eq!(r.check.states, 8);
    assert_eq!(
        r.poles.status,
        "verified_residual",
        "{}",
        serde_json::to_string(&r).unwrap()
    );
    assert_eq!(r.poles.roots.len(), 8);
    assert_eq!(r.stability.classification, "stable");
    for i in 1..=8 {
        assert!(r
            .poles
            .roots
            .iter()
            .any(|p| (p.real + 1.0 / i as f64).abs() < 1e-6 && p.imag.abs() < 1e-6));
    }
    matches_bode(&m, &query(), &r);
}
#[test]
fn coupled_integrators_retain_imaginary_axis_modes_and_exact_matrix() {
    let mut m = feedback();
    m.blocks[2].params = BlockKind::Integrator { initial: vec![0.0] };
    m.blocks[3].params = BlockKind::Integrator { initial: vec![0.0] };
    let r = analyze_state_space(&m, &query()).unwrap();
    assert_eq!(r.a, vec![vec![0.0, 1.0], vec![-1.0, 0.0]]);
    assert_eq!(r.b, vec![0.0, 1.0]);
    assert_eq!(r.c, vec![1.0, 0.0]);
    assert_eq!(r.stability.classification, "boundary_or_uncertain");
    assert_eq!(r.stability.right_half_plane_poles, Some(0));
    assert_eq!(r.poles.roots.len(), 2);
    for p in r.poles.roots {
        assert!(p.real.abs() < 1e-12);
        assert!((p.imag.abs() - 1.0).abs() < 1e-12);
    }
}
