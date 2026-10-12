use blocksim_core::graph::{
    pole_zero::{analyze_pole_zero, validate_pole_zero, AnalysisMode, PoleZeroRequest},
    BlockKind, GraphModel,
};
fn setup(n: Vec<f64>, d: Vec<f64>) -> (GraphModel, PoleZeroRequest) {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json")).unwrap();
    let order = d.len() - 1;
    m.blocks[1].params = BlockKind::TransferFunction {
        numerator: n,
        denominator: d,
        initial: vec![0.0; order],
    };
    (
        m,
        PoleZeroRequest {
            block: "plant".into(),
            mode: AnalysisMode::Block,
        },
    )
}
#[test]
fn quadratic_poles_and_nonminimum_phase_zero_match_analytics() {
    let (m, q) = setup(vec![1.0, -2.0], vec![1.0, 2.0, 5.0]);
    let before = serde_json::to_string(&m).unwrap();
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.stability.classification, "stable");
    assert_eq!(r.stability.right_half_plane_poles, Some(0));
    assert_eq!(r.poles.status, "verified_residual");
    assert_eq!(r.poles.roots.len(), 2);
    for (p, imag) in r.poles.roots.iter().zip([-2.0, 2.0]) {
        assert!((p.real + 1.0).abs() < 1e-12);
        assert!((p.imag - imag).abs() < 1e-12);
        assert!(p.relative_residual < 1e-12);
    }
    assert_eq!(r.zeros.roots.len(), 1);
    assert!((r.zeros.roots[0].real - 2.0).abs() < 1e-12);
    assert_eq!(r.zeros.roots[0].imag, 0.0);
    assert_eq!(serde_json::to_string(&m).unwrap(), before);
}
#[test]
fn unit_negative_feedback_detects_gain_four_stable_six_boundary_and_eight_unstable() {
    for (gain, class, right) in [
        (4.0, "stable", 0),
        (6.0, "boundary_or_uncertain", 0),
        (8.0, "unstable", 2),
    ] {
        let (m, mut q) = setup(vec![gain], vec![1.0, 3.0, 2.0, 0.0]);
        let open = analyze_pole_zero(&m, &q).unwrap();
        assert_eq!(open.stability.classification, "boundary_or_uncertain");
        assert!(open
            .poles
            .roots
            .iter()
            .any(|p| p.real == 0.0 && p.imag == 0.0));
        q.mode = AnalysisMode::UnitNegativeFeedback;
        let r = analyze_pole_zero(&m, &q).unwrap();
        assert_eq!(r.stability.classification, class);
        assert_eq!(r.stability.right_half_plane_poles, Some(right));
        assert_eq!(r.poles.status, "verified_residual");
        assert_eq!(r.poles.roots.len(), 3);
        let actual_right = r.poles.roots.iter().filter(|p| p.real > 1e-7).count();
        assert_eq!(actual_right, right);
        if gain == 6.0 {
            let real = r.poles.roots.iter().find(|p| p.imag == 0.0).unwrap();
            assert!((real.real + 3.0).abs() < 1e-10);
            for p in r.poles.roots.iter().filter(|p| p.imag != 0.0) {
                assert!(p.real.abs() < 1e-10);
                assert!((p.imag.abs() - 2.0_f64.sqrt()).abs() < 1e-10);
            }
        }
    }
}
#[test]
fn unstable_cancelled_and_zero_output_realizations_retain_hidden_poles() {
    let (mut m, q) = setup(vec![1.0, -1.0], vec![1.0, 1.0, -2.0]);
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.stability.classification, "unstable");
    assert_eq!(r.poles.roots.len(), 2);
    assert!(r.poles.roots.iter().any(|p| (p.real - 1.0).abs() < 1e-12));
    assert!((r.zeros.roots[0].real - 1.0).abs() < 1e-12);
    if let BlockKind::TransferFunction { numerator, .. } = &mut m.blocks[1].params {
        *numerator = vec![0.0];
    }
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.stability.classification, "unstable");
    assert_eq!(r.zeros.status, "identically_zero");
    assert_eq!(r.zeros.degree, None);
    assert!(r.zeros.roots.is_empty());
}
#[test]
fn eighth_order_distinct_real_and_complex_roots_and_repeated_root_residuals_are_verified() {
    // (s+1)(s+2)(s²+2s+5)(s²+4s+13)(s²+6s+25).
    let factors = [
        vec![1.0, 1.0],
        vec![1.0, 2.0],
        vec![1.0, 2.0, 5.0],
        vec![1.0, 4.0, 13.0],
        vec![1.0, 6.0, 25.0],
    ];
    let mut d = vec![1.0];
    for f in factors {
        let mut next = vec![0.0; d.len() + f.len() - 1];
        for (i, a) in d.iter().enumerate() {
            for (j, b) in f.iter().enumerate() {
                next[i + j] += a * b;
            }
        }
        d = next;
    }
    let (m, q) = setup(vec![1.0], d);
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.stability.classification, "stable");
    assert_eq!(r.poles.status, "verified_residual");
    assert_eq!(r.poles.roots.len(), 8);
    for (re, im) in [
        (-1.0, 0.0),
        (-2.0, 0.0),
        (-1.0, 2.0),
        (-1.0, -2.0),
        (-2.0, 3.0),
        (-2.0, -3.0),
        (-3.0, 4.0),
        (-3.0, -4.0),
    ] {
        assert!(r
            .poles
            .roots
            .iter()
            .any(|p| (p.real - re).hypot(p.imag - im) < 1e-7));
    }
    let (m, q) = setup(
        vec![1.0],
        vec![1.0, 8.0, 28.0, 56.0, 70.0, 56.0, 28.0, 8.0, 1.0],
    );
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.stability.classification, "stable");
    assert_eq!(r.poles.status, "verified_residual");
    assert_eq!(r.poles.roots.len(), 8);
    assert!(!r.poles.warnings.is_empty());
    assert!(r
        .poles
        .roots
        .iter()
        .all(|p| p.relative_residual <= 1e-9 && (p.real + 1.0).hypot(p.imag) < 0.2));
    let (m, q) = setup(
        vec![1.0],
        vec![1.0, -8.0, 28.0, -56.0, 70.0, -56.0, 28.0, -8.0, 1.0],
    );
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.stability.classification, "unstable");
    assert_eq!(r.stability.right_half_plane_poles, Some(8));
    assert_eq!(r.poles.roots.len(), 8);
    assert!(r.poles.roots.iter().all(|p| p.real == 1.0));
}
#[test]
fn zero_pivots_and_symmetric_real_roots_are_not_misclassified_stable() {
    let (m, q) = setup(vec![1.0], vec![1.0, 1.0, 1.0, 1.0, 1.0]);
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.stability.classification, "indeterminate");
    assert_eq!(r.stability.right_half_plane_poles, None);
    assert_eq!(r.poles.status, "verified_residual");
    assert_eq!(r.poles.roots.iter().filter(|p| p.real > 1e-5).count(), 2);
    let (m, q) = setup(vec![1.0], vec![1.0, 0.0, -1.0]);
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.stability.classification, "unstable");
    assert_eq!(r.stability.right_half_plane_poles, Some(1));
    for d in [vec![1.0, 0.0, 1.0], vec![1.0, 0.0, 0.0], vec![1.0, 0.0]] {
        let (m, q) = setup(vec![1.0], d);
        let r = analyze_pole_zero(&m, &q).unwrap();
        assert_eq!(r.stability.classification, "boundary_or_uncertain");
        assert_eq!(r.stability.right_half_plane_poles, Some(0));
    }
}
#[test]
fn near_axis_positive_and_negative_poles_remain_uncertain_instead_of_using_an_exact_count() {
    for d in [
        vec![1.0, 1e-12, 1.0],
        vec![1.0, -1e-12, 1.0],
        vec![1.0, -1e-12],
    ] {
        let (m, q) = setup(vec![1.0], d);
        let r = analyze_pole_zero(&m, &q).unwrap();
        assert_eq!(r.stability.classification, "boundary_or_uncertain");
        assert_eq!(r.stability.right_half_plane_poles, None);
        assert!(!r.stability.notes.is_empty());
    }
}
#[test]
fn padded_numerators_constants_and_scaled_coefficients_preserve_meaning() {
    let (m, q) = setup(vec![0.0, 0.0, 2.0], vec![1.0, 2.0, 1.0]);
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.zeros.degree, Some(0));
    assert!(r.zeros.roots.is_empty());
    let (m, q) = setup(vec![3.0], vec![2.0]);
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.stability.classification, "stable");
    assert_eq!(r.poles.degree, Some(0));
    assert!(r.poles.roots.is_empty());
    let (m, q) = setup(vec![1e307, -2e307], vec![1e307, 2e307, 5e307]);
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.stability.classification, "stable");
    assert!(r.poles.roots.iter().all(|p| (p.real + 1.0).abs() < 1e-12));
    assert!((r.zeros.roots[0].real - 2.0).abs() < 1e-12);
}
#[test]
fn exact_nested_paths_and_vector_coefficients_do_not_analyze_the_whole_graph() {
    let (mut child, _) = setup(vec![1.0, -2.0], vec![1.0, 2.0, 5.0]);
    for b in &mut child.blocks {
        b.width = 2;
    }
    if let BlockKind::Step { before, after, .. } = &mut child.blocks[0].params {
        *before = vec![0.0; 2];
        *after = vec![1.0; 2];
    }
    if let BlockKind::TransferFunction { initial, .. } = &mut child.blocks[1].params {
        *initial = vec![7.0; 4];
    }
    let mut output = child.blocks[2].clone();
    output.id = "boundary".into();
    output.params = BlockKind::Output { port: 0 };
    child.blocks.push(output);
    let mut e = child.edges[1].clone();
    e.to = "boundary".into();
    child.edges.push(e);
    let (mut parent, mut q) = setup(vec![1.0], vec![1.0, -1.0]);
    let mut b = parent.blocks[0].clone();
    b.id = "nested".into();
    b.width = 2;
    b.params = BlockKind::Subsystem {
        model: Box::new(child),
    };
    parent.blocks.push(b);
    q.block = "nested/plant".into();
    let r = analyze_pole_zero(&parent, &q).unwrap();
    assert_eq!(r.check.width, 2);
    assert_eq!(r.stability.classification, "stable");
    q.block = "plant".into();
    assert_eq!(
        analyze_pole_zero(&parent, &q)
            .unwrap()
            .stability
            .classification,
        "unstable"
    );
}
#[test]
fn invalid_queries_and_singular_unit_feedback_are_refused_before_execution() {
    let (m, mut q) = setup(vec![-1.0, 1.0], vec![1.0, 1.0]);
    q.mode = AnalysisMode::UnitNegativeFeedback;
    assert!(validate_pole_zero(&m, &q)
        .unwrap_err()
        .contains("direct gain -1"));
    q.mode = AnalysisMode::Block;
    assert!(validate_pole_zero(&m, &q).is_ok());
    q.block = "source".into();
    assert!(validate_pole_zero(&m, &q).is_err());
    q.block = "absent".into();
    assert!(analyze_pole_zero(&m, &q).is_err());
    for json in [
        r#"{"block":"plant","mode":"graph"}"#,
        r#"{"block":"plant","mode":"block","extra":true}"#,
        r#"{"block":"plant"}"#,
    ] {
        assert!(serde_json::from_str::<PoleZeroRequest>(json).is_err());
    }
}
#[test]
fn extreme_coefficients_report_unresolved_without_nonfinite_or_partial_roots() {
    let (m, q) = setup(vec![1.0], vec![1.0, 1e308]);
    let r = analyze_pole_zero(&m, &q).unwrap();
    assert_eq!(r.poles.status, "unresolved");
    assert!(r.poles.roots.is_empty());
    assert_eq!(r.stability.classification, "indeterminate");
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains("Infinity") && !json.contains("NaN"));
}
