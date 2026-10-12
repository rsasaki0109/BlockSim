use blocksim_core::graph::{
    discretize::{discretize_tustin, TustinRequest},
    simulate_graph,
    state_space::{analyze_discrete_state_space, StateSpaceRequest},
    BlockKind, GraphModel, SolverConfig,
};
fn q(n: Vec<f64>, d: Vec<f64>, t: f64) -> TustinRequest {
    TustinRequest {
        numerator: n,
        denominator: d,
        sample_time: t,
        prewarp_omega: None,
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-11 * (1.0 + b.abs()), "{a} != {b}");
}
fn model() -> GraphModel {
    serde_json::from_str(include_str!("../../../samples/frequency/lowpass.json")).unwrap()
}
fn convert(m: &mut GraphModel, r: &blocksim_core::graph::discretize::TustinResult) {
    m.blocks[1].params = BlockKind::DiscreteTransferFunction {
        numerator: r.numerator.clone(),
        denominator: r.denominator.clone(),
        initial: vec![0.0; m.blocks[1].width * (r.denominator.len() - 1)],
        sample_time: r.request.sample_time,
    };
}
#[derive(Clone, Copy)]
struct C(f64, f64);
impl C {
    fn add(self, b: Self) -> Self {
        Self(self.0 + b.0, self.1 + b.1)
    }
    fn mul(self, b: Self) -> Self {
        Self(self.0 * b.0 - self.1 * b.1, self.0 * b.1 + self.1 * b.0)
    }
    fn div(self, b: Self) -> Self {
        let d = b.0 * b.0 + b.1 * b.1;
        Self(
            (self.0 * b.0 + self.1 * b.1) / d,
            (self.1 * b.0 - self.0 * b.1) / d,
        )
    }
}
fn poly(p: &[f64], x: C) -> C {
    p.iter().fold(C(0.0, 0.0), |a, &c| a.mul(x).add(C(c, 0.0)))
}
#[test]
fn first_order_integrator_and_static_match_independently_derived_coefficients() {
    let r = discretize_tustin(&q(vec![1.0], vec![1.0, 1.0], 0.1)).unwrap();
    close(r.numerator[0], 1.0 / 21.0);
    close(r.numerator[1], 1.0 / 21.0);
    close(r.denominator[1], -19.0 / 21.0);
    assert_eq!(r.mapping_scale, 20.0);
    let r = discretize_tustin(&q(vec![1.0], vec![1.0, 0.0], 0.1)).unwrap();
    assert_eq!(r.numerator, vec![0.05, 0.05]);
    assert_eq!(r.denominator, vec![1.0, -1.0]);
    let r = discretize_tustin(&q(vec![6.0], vec![2.0], 0.1)).unwrap();
    assert_eq!(r.numerator, vec![3.0]);
    assert_eq!(r.denominator, vec![1.0]);
}
#[test]
fn unit_and_subunit_mapping_scales_match_low_frequency_bilinear_response() {
    for (ts, prewarp) in [(2.0, None), (5.0, None), (5.0, Some(0.4))] {
        let mut request = q(vec![1.0], vec![1.0, 1.0], ts);
        request.prewarp_omega = prewarp;
        let r = discretize_tustin(&request).unwrap();
        let k = r.mapping_scale;
        assert!(k <= 1.0);
        close(r.numerator[0], 1.0 / (k + 1.0));
        close(r.numerator[1], 1.0 / (k + 1.0));
        close(r.denominator[1], (1.0 - k) / (1.0 + k));
        for angle in [0.2_f64, 1.0, 2.0] {
            let z = C(angle.cos(), angle.sin());
            let a = poly(&r.numerator, z).div(poly(&r.denominator, z));
            let e = C(1.0, 0.0).div(C(1.0, k * (angle / 2.0).tan()));
            close(a.0, e.0);
            close(a.1, e.1);
        }
    }
}
#[test]
fn second_order_and_nonmonic_scaling_preserve_degree_direct_term_and_zero_numerator() {
    let r = discretize_tustin(&q(vec![1.0], vec![1.0, 2.0, 1.0], 0.1)).unwrap();
    for (a, b) in r
        .numerator
        .iter()
        .zip([1.0 / 441.0, 2.0 / 441.0, 1.0 / 441.0])
    {
        close(*a, b);
    }
    for (a, b) in r.denominator.iter().zip([1.0, -38.0 / 21.0, 361.0 / 441.0]) {
        close(*a, b);
    }
    let scaled = discretize_tustin(&q(vec![1e300], vec![1e300, 2e300, 1e300], 0.1)).unwrap();
    for (a, b) in r
        .numerator
        .iter()
        .chain(&r.denominator)
        .zip(scaled.numerator.iter().chain(&scaled.denominator))
    {
        close(*a, *b);
    }
    let z = discretize_tustin(&q(vec![0.0], vec![1.0, -1.0], 0.1)).unwrap();
    assert_eq!(z.numerator, vec![0.0, 0.0]);
    assert_eq!(z.denominator.len(), 2);
}
#[test]
fn first_through_eighth_order_match_independent_warped_complex_polynomial_evaluation() {
    for n in [1, 2, 4, 8] {
        let mut d = vec![1.0];
        for rate in 1..=n {
            let mut next = vec![0.0; d.len() + 1];
            for (i, &c) in d.iter().enumerate() {
                next[i] += c;
                next[i + 1] += c * rate as f64;
            }
            d = next;
        }
        let request = q(vec![1.0], d, 0.1);
        let r = discretize_tustin(&request).unwrap();
        assert_eq!(r.denominator.len(), n + 1);
        for i in 1..=30 {
            let angle = 0.1 + i as f64 * 0.09;
            let z = C(angle.cos(), angle.sin());
            let actual = poly(&r.numerator, z).div(poly(&r.denominator, z));
            let expected = poly(&request.numerator, C(0.0, 20.0 * (angle / 2.0).tan())).div(poly(
                &request.denominator,
                C(0.0, 20.0 * (angle / 2.0).tan()),
            ));
            close(actual.0, expected.0);
            close(actual.1, expected.1);
        }
    }
}
#[test]
fn prewarp_preserves_selected_continuous_frequency_instead_of_unwarped_value() {
    let mut request = q(vec![1.0], vec![1.0, 2.0, 1.0], 0.2);
    request.prewarp_omega = Some(5.0);
    let r = discretize_tustin(&request).unwrap();
    close(r.mapping_scale, 5.0 / 0.5_f64.tan());
    let z = C(1.0_f64.cos(), 1.0_f64.sin());
    let a = poly(&r.numerator, z).div(poly(&r.denominator, z));
    let e = poly(&request.numerator, C(0.0, 5.0)).div(poly(&request.denominator, C(0.0, 5.0)));
    close(a.0, e.0);
    close(a.1, e.1);
    request.prewarp_omega = None;
    let off = discretize_tustin(&request).unwrap();
    let v = poly(&off.numerator, z).div(poly(&off.denominator, z));
    assert!((v.0 - e.0).hypot(v.1 - e.1) > 0.001);
}
#[test]
fn stable_boundary_and_unstable_continuous_poles_map_without_cancellation() {
    for (p, class) in [
        (-1.0, "stable"),
        (0.0, "boundary_or_uncertain"),
        (1.0, "unstable"),
    ] {
        let r = discretize_tustin(&q(vec![1.0, -p], vec![1.0, -p], 0.1)).unwrap();
        let mut m = model();
        convert(&mut m, &r);
        let ss = analyze_discrete_state_space(
            &m,
            &StateSpaceRequest {
                source: "source".into(),
                scope: "output".into(),
                source_component: 0,
                scope_component: 0,
            },
        )
        .unwrap();
        close(ss.a[0][0], (20.0 + p) / (20.0 - p));
        assert_eq!(ss.stability.classification, class);
        assert_eq!(ss.check.states, 1);
        close(ss.c[0], 0.0);
        close(ss.d, 1.0);
    }
}
#[test]
fn real_rk4_and_rk45_sampled_steps_match_two_independent_trapezoidal_sections() {
    let r = discretize_tustin(&q(vec![1.0], vec![1.0, 2.0, 1.0], 0.1)).unwrap();
    for solver in [
        None,
        Some(SolverConfig::Rk45 {
            abs_tolerance: 1e-9,
            rel_tolerance: 1e-9,
            min_step: 1e-12,
            max_step: 0.8,
        }),
    ] {
        let mut m = model();
        m.solver = solver;
        convert(&mut m, &r);
        let time = simulate_graph(&m).unwrap();
        let a = 19.0 / 21.0;
        let b = 1.0 / 21.0;
        let (mut v, mut y, mut previous_u) = (0.0, 0.0, 0.0);
        for (tick, row) in time.traces[0].values.iter().enumerate() {
            if tick % 10 == 0 {
                let next_v = a * v + b * (1.0 + previous_u);
                y = a * y + b * (next_v + v);
                v = next_v;
                previous_u = 1.0;
            }
            close(row[0], y);
        }
    }
}
#[test]
fn invalid_ranges_and_near_singular_or_unrepresentable_mapping_fail_explicitly() {
    for t in [0.0, -0.1, f64::INFINITY, 1e-309] {
        assert!(discretize_tustin(&q(vec![1.0], vec![1.0, 1.0], t)).is_err());
    }
    for w in [0.0, -1.0, f64::INFINITY, std::f64::consts::PI / 0.1] {
        let mut r = q(vec![1.0], vec![1.0, 1.0], 0.1);
        r.prewarp_omega = Some(w);
        assert!(discretize_tustin(&r).is_err());
    }
    assert!(discretize_tustin(&q(vec![1.0], vec![1.0, -20.0], 0.1))
        .unwrap_err()
        .contains("leading"));
    assert!(
        discretize_tustin(&q(vec![1.0], vec![1.0, -20.0 * (1.0 - 1e-13)], 0.1))
            .unwrap_err()
            .contains("ill-conditioned")
    );
    assert!(discretize_tustin(&q(
        vec![1.0],
        vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        1e-100
    ))
    .unwrap_err()
    .contains("underflows"));
    for (n, d) in [
        (vec![], vec![1.0]),
        (vec![1.0, 2.0], vec![1.0]),
        (vec![1.0], vec![0.0, 1.0]),
        (vec![1.0], vec![1.0; 10]),
    ] {
        assert!(discretize_tustin(&q(n, d, 0.1)).is_err());
    }
}
