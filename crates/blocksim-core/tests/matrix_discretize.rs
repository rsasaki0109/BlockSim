use blocksim_core::{
    graph::{discretize::*, state_space::mimo::*, *},
    PidMode,
};
fn request() -> MatrixRequest {
    MatrixRequest {
        a: vec![vec![-1., 0.], vec![1., -2.]],
        b: vec![vec![1., 1.], vec![0., 2.]],
        c: vec![vec![1., 0.], vec![0., 1.]],
        d: vec![vec![0.25, 0.], vec![0., 0.5]],
        initial: vec![0., 0.],
        sample_time: 0.1,
        method: MatrixMethod::Zoh,
        prewarp_omega: None,
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-13, "{a} != {b}");
}
fn model(r: &MatrixResult) -> GraphModel {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/matrix/discretization.json")).unwrap();
    m.blocks[1].params = BlockKind::StateSpace {
        a: r.a.clone(),
        b: r.b.clone(),
        c: r.c.clone(),
        d: r.d.clone(),
        initial: r.initial.clone(),
        mode: PidMode::Discrete,
        sample_time: r.request.sample_time,
    };
    m
}
fn exact(t: f64, x: [f64; 2], u: [f64; 2]) -> [f64; 2] {
    let a = u[0] + u[1];
    let b = 2. * u[1];
    let e = (-t).exp();
    let e2 = (-2. * t).exp();
    [
        a + (x[0] - a) * e,
        x[1] * e2 + (a + b) / 2. * (1. - e2) + (x[0] - a) * (e - e2),
    ]
}
#[test]
fn zoh_coupled_rectangular_coefficients_match_independent_exponential_and_preserve_initial() {
    let mut q = request();
    q.initial = vec![0.5, -0.25];
    q.c.push(vec![1., 1.]);
    q.d.push(vec![1., -1.]);
    let r = discretize_matrix(&q).unwrap();
    let e = (-q.sample_time).exp();
    let e2 = e * e;
    let aa = [[e, 0.], [e - e2, e2]];
    let bb = [
        [1. - e, 1. - e],
        [(1. - e2) / 2. - e + e2, 1.5 * (1. - e2) - e + e2],
    ];
    for i in 0..2 {
        for j in 0..2 {
            close(r.a[i][j], aa[i][j]);
            close(r.b[i][j], bb[i][j]);
        }
    }
    assert_eq!(r.initial, q.initial);
    assert_eq!(r.c, q.c);
    assert_eq!(r.d, q.d);
    assert!(r.residual < 1e-14);
}
#[test]
fn zoh_nonzero_initial_step_and_output_hold_match_independent_piecewise_solution() {
    for adaptive in [false, true] {
        let mut q = request();
        q.initial = vec![0.5, -0.25];
        let r = discretize_matrix(&q).unwrap();
        let mut m = model(&r);
        if adaptive {
            m.solver = Some(SolverConfig::Rk45 {
                abs_tolerance: 1e-10,
                rel_tolerance: 1e-9,
                min_step: 1e-8,
                max_step: 0.2,
            });
        }
        let out = simulate_graph(&m).unwrap();
        let boundary = exact(1.2, [0.5, -0.25], [1., 2.]);
        for (k, row) in out.traces[0].values.iter().enumerate() {
            let t = (k / 10) as f64 * 0.1;
            let (x, u) = if k < 120 {
                (exact(t, [0.5, -0.25], [1., 2.]), [1., 2.])
            } else {
                (exact(t - 1.2, boundary, [0.25, -1.]), [0.25, -1.])
            };
            close(row[0], x[0] + 0.25 * u[0]);
            close(row[1], x[1] + 0.5 * u[1]);
        }
    }
}
#[test]
fn zoh_singular_integrator_and_jordan_block_do_not_require_inverse_a() {
    let mut q = request();
    q.a = vec![vec![0., 1.], vec![0., 0.]];
    q.b = vec![vec![0., 1.], vec![1., 0.]];
    let r = discretize_matrix(&q).unwrap();
    assert_eq!(r.a, vec![vec![1., 0.1], vec![0., 1.]]);
    close(r.b[0][0], 0.005);
    close(r.b[0][1], 0.1);
    close(r.b[1][0], 0.1);
    close(r.b[1][1], 0.);
    q.a = vec![vec![-1., 1.], vec![0., -1.]];
    let r = discretize_matrix(&q).unwrap();
    let e = (-0.1_f64).exp();
    close(r.a[0][1], 0.1 * e);
    close(r.b[0][0], 1. - 1.1 * e);
}
#[test]
fn tustin_with_and_without_prewarp_matches_all_four_independent_complex_paths() {
    for prewarp in [None, Some(3.)] {
        let mut q = request();
        q.method = MatrixMethod::Tustin;
        q.prewarp_omega = prewarp;
        let r = discretize_matrix(&q).unwrap();
        let k = r.mapping_scale.unwrap();
        close(k, prewarp.map_or(20., |w| w / (w * 0.05).tan()));
        let channels = |id: &str| {
            (0..2)
                .map(|component| Channel {
                    block: id.into(),
                    component,
                })
                .collect()
        };
        let analysis = analyze_mimo(
            &model(&r),
            &MimoRequest {
                inputs: channels("source"),
                outputs: channels("output"),
                discrete: true,
                min_omega: 0.1,
                max_omega: 20.,
                points: 101,
            },
        )
        .unwrap();
        for path in analysis.responses {
            for point in path.points {
                let w = k * (point.omega * 0.05).tan();
                let inv = |v: f64| (v / (v * v + w * w), -w / (v * v + w * w));
                let f = inv(1.);
                let g = inv(2.);
                let fg = (f.0 * g.0 - f.1 * g.1, f.0 * g.1 + f.1 * g.0);
                let expected = [
                    [(f.0 + 0.25, f.1), f],
                    [fg, (fg.0 + 2. * g.0 + 0.5, fg.1 + 2. * g.1)],
                ][path.output][path.input];
                close(point.real.unwrap(), expected.0);
                close(point.imag.unwrap(), expected.1);
            }
        }
    }
}
#[test]
fn both_methods_keep_zero_state_rectangular_static_gain_and_all_hidden_modes() {
    for method in [MatrixMethod::Zoh, MatrixMethod::Tustin] {
        let mut q = request();
        q.method = method;
        q.a.clear();
        q.b.clear();
        q.c = vec![vec![], vec![], vec![]];
        q.d.push(vec![0., 0.]);
        q.initial.clear();
        let r = discretize_matrix(&q).unwrap();
        assert!(r.a.is_empty());
        assert_eq!(r.c, q.c);
        assert_eq!(r.d, q.d);
        q = request();
        q.method = method;
        q.a = vec![vec![1., 0.], vec![0., -2.]];
        q.b = vec![vec![0., 0.], vec![0., 0.]];
        q.c = vec![vec![0., 0.], vec![0., 0.]];
        let r = discretize_matrix(&q).unwrap();
        assert_eq!(r.a.len(), 2);
        assert!(r.a[0][0] > 1.);
        assert_eq!(r.c, q.c);
    }
}
#[test]
fn sixteen_dense_states_eight_inputs_and_outputs_match_rank_one_exponential() {
    let n = 16;
    let m = 8;
    let mut q = request();
    q.a = (0..n)
        .map(|i| (0..n).map(|j| if i == j { -1.01 } else { 0.01 }).collect())
        .collect();
    q.b = vec![vec![1.; m]; n];
    q.c = vec![vec![1.; n]; m];
    q.d = vec![vec![0.; m]; m];
    q.initial = vec![0.; n];
    let r = discretize_matrix(&q).unwrap();
    let common = (-0.86_f64 * 0.1).exp();
    let separate = (-1.02_f64 * 0.1).exp();
    for i in 0..n {
        for j in 0..n {
            close(
                r.a[i][j],
                if i == j { separate } else { 0. } + (common - separate) / 16.,
            );
        }
        for j in 0..m {
            close(r.b[i][j], (1. - common) / 0.86);
        }
    }
    assert_eq!(r.c.len(), 8);
    assert_eq!(r.a.len(), 16);
    assert_eq!(r.b[0].len(), 8);
    q.method = MatrixMethod::Tustin;
    let r = discretize_matrix(&q).unwrap();
    let common = (1. - 0.86 * 0.05) / (1. + 0.86 * 0.05);
    let separate = (1. - 1.02 * 0.05) / (1. + 1.02 * 0.05);
    for i in 0..n {
        for j in 0..n {
            close(
                r.a[i][j],
                if i == j { separate } else { 0. } + (common - separate) / 16.,
            );
        }
        for j in 0..m {
            close(r.b[i][j], 0.1 / (1. + 0.86 * 0.05));
        }
    }
    for row in &r.c {
        for v in row {
            close(*v, 1. / (1. + 0.86 * 0.05));
        }
    }
    for row in &r.d {
        for v in row {
            close(*v, 16. * 0.05 / (1. + 0.86 * 0.05));
        }
    }
}
#[test]
fn invalid_shapes_limits_nonfinite_periods_prewarp_and_nonzero_tustin_are_rejected() {
    let q = request();
    let mut cases = vec![];
    let mut x = q.clone();
    x.a[0].push(0.);
    cases.push(x);
    let mut x = q.clone();
    x.b.pop();
    cases.push(x);
    let mut x = q.clone();
    x.d[0] = vec![1.; 9];
    cases.push(x);
    let mut x = q.clone();
    x.initial.pop();
    cases.push(x);
    let mut x = q.clone();
    x.a[0][0] = f64::NAN;
    cases.push(x);
    let mut x = q.clone();
    x.sample_time = 0.;
    cases.push(x);
    let mut x = q.clone();
    x.sample_time = f64::INFINITY;
    cases.push(x);
    let mut x = q.clone();
    x.prewarp_omega = Some(1.);
    cases.push(x);
    let mut x = q.clone();
    x.method = MatrixMethod::Tustin;
    x.initial[0] = 1.;
    cases.push(x);
    for x in cases {
        assert!(discretize_matrix(&x).is_err());
    }
    for w in [0., -1., f64::NAN, std::f64::consts::PI / 0.1] {
        let mut x = q.clone();
        x.method = MatrixMethod::Tustin;
        x.prewarp_omega = Some(w);
        assert!(discretize_matrix(&x).is_err());
    }
}
#[test]
fn singular_unresolved_overflow_and_underflow_never_produce_usable_coefficients() {
    let mut q = request();
    q.method = MatrixMethod::Tustin;
    q.a[0][0] = 20.;
    assert!(discretize_matrix(&q).unwrap_err().contains("unresolved"));
    q.a[0][0] = 20. - 1e-13;
    assert!(discretize_matrix(&q).is_err());
    q = request();
    q.a[0][0] = 1e10;
    assert!(discretize_matrix(&q).is_err());
    q.a[0][0] = 1e5;
    assert!(discretize_matrix(&q).is_err());
    q = request();
    q.sample_time = 1e-300;
    q.b[0][0] = 1e-100;
    assert!(discretize_matrix(&q).is_err());
}
