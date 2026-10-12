use blocksim_core::graph::{
    frequency::{analyze_frequency, FrequencyRequest},
    BlockKind, GraphModel,
};
fn plan() -> (GraphModel, FrequencyRequest) {
    let p: serde_json::Value = serde_json::from_str(include_str!(
        "../../../samples/frequency/open-loop-plan.json"
    ))
    .unwrap();
    (
        serde_json::from_value(p["model"].clone()).unwrap(),
        serde_json::from_value(p["request"].clone()).unwrap(),
    )
}
fn transfer(m: &mut GraphModel, numerator: Vec<f64>, denominator: Vec<f64>) {
    let n = denominator.len() - 1;
    m.blocks[1].params = BlockKind::TransferFunction {
        numerator,
        denominator,
        initial: vec![0.0; n],
    };
}
#[test]
fn open_loop_margins_match_independent_analytic_crossings_and_leave_bode_unchanged() {
    let (m, mut q) = plan();
    let before = serde_json::to_string(&m).unwrap();
    let result = analyze_frequency(&m, &q).unwrap();
    let out = result.margins.as_ref().unwrap();
    assert!(out.issues.is_empty());
    assert_eq!(out.gain_crossings.len(), 1);
    assert_eq!(out.phase_crossings.len(), 1);
    // |L|² = 16 / (x(x+1)(x+4)), x=ω². Independent scalar analytic equation.
    let (mut lo, mut hi) = (0.0_f64, 4.0_f64);
    for _ in 0..100 {
        let x = (lo + hi) / 2.0;
        if x * (x + 1.0) * (x + 4.0) < 16.0 {
            lo = x;
        } else {
            hi = x;
        }
    }
    let w = ((lo + hi) / 2.0).sqrt();
    let pm = 90.0 - w.atan().to_degrees() - (w / 2.0).atan().to_degrees();
    assert!((out.gain_crossings[0].omega - w).abs() < 1e-10);
    assert!((out.gain_crossings[0].phase_margin_deg - pm).abs() < 1e-8);
    assert!((out.phase_crossings[0].omega - 2.0_f64.sqrt()).abs() < 1e-10);
    assert!((out.phase_crossings[0].gain_margin_db - 20.0 * 1.5_f64.log10()).abs() < 1e-8);
    assert!((out.phase_crossings[0].gain_margin.unwrap() - 1.5).abs() < 1e-9);
    assert!(out.additional_evaluations > 0 && out.additional_evaluations < 100);
    q.open_loop = false;
    let plain = analyze_frequency(&m, &q).unwrap();
    assert!(plain.margins.is_none());
    assert_eq!(
        serde_json::to_value(&plain.points).unwrap(),
        serde_json::to_value(&result.points).unwrap()
    );
    assert_eq!(serde_json::to_string(&m).unwrap(), before);
    assert!(serde_json::to_value(plain)
        .unwrap()
        .get("margins")
        .is_none());
    assert!(serde_json::to_value(q).unwrap().get("open_loop").is_none());
}
#[test]
fn resonance_returns_both_gain_crossings_instead_of_selecting_one() {
    let (mut m, q) = plan();
    transfer(&mut m, vec![0.2], vec![1.0, 0.1, 1.0]);
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert!(out.issues.is_empty());
    assert_eq!(out.gain_crossings.len(), 2);
    assert!(out.phase_crossings.is_empty());
    // (1-ω²)² + .01ω² = .04.
    let disc = (1.99_f64 * 1.99 - 3.84).sqrt();
    for (g, x) in out
        .gain_crossings
        .iter()
        .zip([(1.99 - disc) / 2.0, (1.99 + disc) / 2.0])
    {
        assert!((g.omega - x.sqrt()).abs() < 1e-10);
    }
}
#[test]
fn all_odd_180_degree_branches_are_retained() {
    let (mut m, q) = plan();
    transfer(
        &mut m,
        vec![4.0],
        vec![1.0, 8.0, 28.0, 56.0, 70.0, 56.0, 28.0, 8.0, 1.0],
    );
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert!(out.issues.is_empty());
    assert_eq!(out.phase_crossings.len(), 2);
    for (crossing, theta, target) in [
        (0, std::f64::consts::PI / 8.0, -180.0),
        (1, 3.0 * std::f64::consts::PI / 8.0, -540.0),
    ] {
        let p = &out.phase_crossings[crossing];
        let w = theta.tan();
        assert!((p.omega - w).abs() < 1e-9);
        assert_eq!(p.phase_level_deg, target);
        assert!(
            (p.gain_margin_db - (80.0 * (1.0 + w * w).log10() - 20.0 * 4.0_f64.log10())).abs()
                < 1e-8
        );
    }
}
#[test]
fn exact_grid_crossings_and_endpoints_are_not_duplicated() {
    let (mut m, mut q) = plan();
    transfer(&mut m, vec![2.0], vec![1.0, 2.0, 1.0]);
    q.min_omega = 0.1;
    q.max_omega = 10.0;
    q.points = 3;
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert_eq!(out.gain_crossings.len(), 1);
    assert!((out.gain_crossings[0].omega - 1.0).abs() < 1e-12);
    assert!((out.gain_crossings[0].phase_margin_deg - 90.0).abs() < 1e-10);
    q.min_omega = 1.0;
    q.points = 2;
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert_eq!(out.gain_crossings.len(), 1);
    assert!((out.gain_crossings[0].omega - 1.0).abs() < 1e-12);
    transfer(&mut m, vec![4.0], vec![1.0, 3.0, 2.0, 0.0]);
    q.min_omega = 2.0_f64.sqrt();
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert_eq!(out.phase_crossings.len(), 1);
    assert_eq!(out.phase_crossings[0].omega, q.min_omega);
}
#[test]
fn finite_range_no_crossing_is_distinct_from_zero_gaps_and_plateaus() {
    let (mut m, mut q) = plan();
    q.min_omega = 0.01;
    q.max_omega = 0.02;
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert!(
        out.gain_crossings.is_empty() && out.phase_crossings.is_empty() && out.issues.is_empty()
    );
    transfer(&mut m, vec![0.0], vec![1.0]);
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert_eq!(out.issues.len(), 1);
    assert_eq!(out.issues[0].kind, "gap");
    assert_eq!(out.issues[0].min_omega, q.min_omega);
    assert_eq!(out.issues[0].max_omega, q.max_omega);
    transfer(&mut m, vec![1.0], vec![1.0]);
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert!(out.gain_crossings.is_empty());
    assert_eq!(out.issues.len(), 1);
    assert_eq!(out.issues[0].kind, "gain_plateau");
    transfer(&mut m, vec![-2.0], vec![1.0]);
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert!(out.phase_crossings.is_empty());
    assert_eq!(out.issues.len(), 1);
    assert_eq!(out.issues[0].kind, "phase_plateau");
    transfer(&mut m, vec![1.0], vec![1.0, 0.0, 1.0]);
    q.min_omega = 0.5;
    q.max_omega = 2.0;
    q.points = 3;
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert!(out.issues.iter().any(|i| i.kind == "gap"));
    assert!(out.phase_crossings.is_empty());
}
#[test]
fn narrow_resonance_and_unstable_open_loop_are_not_claimed_stable_or_complete() {
    let (mut m, mut q) = plan();
    transfer(&mut m, vec![0.2], vec![1.0, 0.1, 1.0]);
    q.min_omega = 0.1;
    q.max_omega = 10.0;
    q.points = 2;
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert!(out.gain_crossings.is_empty());
    assert!(out.method.contains("sampled brackets"));
    transfer(&mut m, vec![2.0], vec![1.0, -1.0]);
    q.points = 201;
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert_eq!(out.gain_crossings.len(), 1);
    // A positive PM is possible even with an unstable open-loop pole.
    assert!((out.gain_crossings[0].phase_margin_deg - 60.0).abs() < 1e-8);
    let json = serde_json::to_string(&out).unwrap();
    assert!(!json.contains("stable") && !json.contains("Infinity") && !json.contains("NaN"));
}
#[test]
fn exhausted_extra_solve_budget_reports_unresolved_brackets_without_fabricated_margins() {
    let (mut m, mut q) = plan();
    // 22 relevant variables; floor(20M/22³)=1878 grid solves leaves no extra solve.
    m.edges[1].from = "g18".into();
    for i in 0..19 {
        let mut b = m.blocks[0].clone();
        b.id = format!("g{i}");
        b.params = BlockKind::Gain { gain: 1.0 };
        m.blocks.push(b);
        let mut e = m.edges[0].clone();
        e.from = if i == 0 {
            "plant".into()
        } else {
            format!("g{}", i - 1)
        };
        e.to = format!("g{i}");
        m.edges.push(e);
    }
    q.points = 20_000_000 / 22_usize.pow(3);
    let out = analyze_frequency(&m, &q).unwrap().margins.unwrap();
    assert_eq!(out.additional_evaluations, 0);
    assert!(out.gain_crossings.is_empty() && out.phase_crossings.is_empty());
    assert_eq!(out.issues.len(), 2);
    assert!(out
        .issues
        .iter()
        .all(|i| i.message.contains("budget exhausted")));
}
