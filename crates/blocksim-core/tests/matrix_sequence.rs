use blocksim_core::{
    graph::{discretize::*, *},
    PidMode,
};
fn plan() -> (MatrixRequest, Vec<Vec<f64>>, usize) {
    let p: serde_json::Value = serde_json::from_str(include_str!(
        "../../../samples/matrix/sequence-comparison-plan.json"
    ))
    .unwrap();
    (
        serde_json::from_value(p["request"].clone()).unwrap(),
        serde_json::from_value(p["input"]["values"].clone()).unwrap(),
        p["samples"].as_u64().unwrap() as usize,
    )
}
fn source_model(q: &MatrixRequest, rows: &[Vec<f64>], repeat: bool, samples: usize) -> GraphModel {
    let mut m: GraphModel =
        serde_json::from_str(include_str!("../../../samples/matrix/discretization.json")).unwrap();
    m.duration = (samples - 1) as f64 * q.sample_time;
    m.dt = q.sample_time / 10.;
    m.solver = Some(SolverConfig::Rk45 {
        abs_tolerance: 1e-11,
        rel_tolerance: 1e-11,
        min_step: 1e-12,
        max_step: m.dt,
    });
    m.blocks[0].width = q.d[0].len();
    m.blocks[0].params = BlockKind::SampledInput {
        values: rows.to_vec(),
        sample_time: q.sample_time,
        repeat,
    };
    m.blocks[1].width = q.c.len();
    m.blocks[2].width = q.c.len();
    m.blocks[1].params = BlockKind::StateSpace {
        a: q.a.clone(),
        b: q.b.clone(),
        c: q.c.clone(),
        d: q.d.clone(),
        initial: q.initial.clone(),
        mode: PidMode::Continuous,
        sample_time: q.sample_time,
    };
    m.blocks.push(Block {
        id: "input_record".into(),
        label: "Actual input".into(),
        width: q.d[0].len(),
        position: [0., 0.],
        params: BlockKind::Scope {},
    });
    m.edges.push(Edge {
        from: "source".into(),
        from_port: 0,
        to: "input_record".into(),
        to_port: 0,
    });
    m
}
fn converted(m: &GraphModel, q: &MatrixRequest) -> GraphModel {
    let r = discretize_matrix(q).unwrap();
    let mut b = m.clone();
    b.blocks[1].params = BlockKind::StateSpace {
        a: r.a,
        b: r.b,
        c: r.c,
        d: r.d,
        initial: r.initial,
        mode: PidMode::Discrete,
        sample_time: q.sample_time,
    };
    b
}
fn exact(t: f64, x: [f64; 2], u: &[f64]) -> [f64; 2] {
    let a = u[0] + u[1];
    let b = 2. * u[1];
    let e = (-t).exp();
    let e2 = (-2. * t).exp();
    [
        a + (x[0] - a) * e,
        x[1] * e2 + (a + b) / 2. * (1. - e2) + (x[0] - a) * (e - e2),
    ]
}
fn trace<'a>(r: &'a GraphResult, id: &str) -> &'a Vec<Vec<f64>> {
    &r.traces.iter().find(|t| t.id == id).unwrap().values
}
#[test]
fn zoh_nonzero_vector_sequence_matches_independent_intervals_and_repeats_without_resetting_states()
{
    let (q, rows, samples) = plan();
    for repeat in [false, true] {
        let m = source_model(&q, &rows, repeat, samples);
        let a = simulate_graph(&m).unwrap();
        let b = simulate_graph(&converted(&m, &q)).unwrap();
        assert_eq!(a.times, b.times);
        let mut x = [q.initial[0], q.initial[1]];
        let mut previous = &rows[0];
        let mut held = vec![0.; 2];
        for i in 0..a.times.len() {
            let k = i / 10;
            let u = &rows[if repeat {
                k % rows.len()
            } else {
                k.min(rows.len() - 1)
            }];
            if i % 10 == 0 {
                if i > 0 {
                    x = exact(q.sample_time, x, previous);
                }
                held = vec![x[0] + 0.25 * u[0], x[1] + 0.5 * u[1]];
                previous = u;
            }
            let expected = exact((i % 10) as f64 * m.dt, x, u);
            assert_eq!(&trace(&a, "input_record")[i], u);
            assert_eq!(&trace(&b, "input_record")[i], u);
            for j in 0..2 {
                assert!(
                    (trace(&a, "output")[i][j] - expected[j] - [0.25, 0.5][j] * u[j]).abs() < 3e-10
                );
                assert!((trace(&b, "output")[i][j] - held[j]).abs() < 3e-14);
            }
        }
        if repeat {
            assert_ne!(trace(&b, "output")[0], trace(&b, "output")[40]);
        }
    }
}
#[test]
fn tustin_prewarp_sequence_matches_independent_matrix_recurrence_at_every_held_record() {
    let (mut q, rows, samples) = plan();
    q.initial = vec![0., 0.];
    q.method = MatrixMethod::Tustin;
    q.prewarp_omega = Some(3.);
    let h = (3. * q.sample_time / 2.).tan() / 3.;
    let f = 1. / (1. + h);
    let g = 1. / (1. + 2. * h);
    let fg = f * g;
    let ad = [[2. * f - 1., 0.], [2. * h * fg, 2. * g - 1.]];
    let bd = [
        [2. * h * f, 2. * h * f],
        [2. * h * h * fg, 2. * h * (h * fg + 2. * g)],
    ];
    let cd = [[f, 0.], [h * fg, g]];
    let dd = [
        [0.25 + h * f, h * f],
        [h * h * fg, 0.5 + h * h * fg + 2. * h * g],
    ];
    for repeat in [false, true] {
        let m = source_model(&q, &rows, repeat, samples);
        let r = simulate_graph(&converted(&m, &q)).unwrap();
        let mut x = [0., 0.];
        let mut previous = &rows[0];
        let mut held = [0., 0.];
        for i in 0..r.times.len() {
            let k = i / 10;
            let u = &rows[if repeat {
                k % rows.len()
            } else {
                k.min(rows.len() - 1)
            }];
            if i % 10 == 0 {
                if i > 0 {
                    x = [
                        ad[0][0] * x[0]
                            + ad[0][1] * x[1]
                            + bd[0][0] * previous[0]
                            + bd[0][1] * previous[1],
                        ad[1][0] * x[0]
                            + ad[1][1] * x[1]
                            + bd[1][0] * previous[0]
                            + bd[1][1] * previous[1],
                    ];
                }
                held = [
                    cd[0][0] * x[0] + cd[0][1] * x[1] + dd[0][0] * u[0] + dd[0][1] * u[1],
                    cd[1][0] * x[0] + cd[1][1] * x[1] + dd[1][0] * u[0] + dd[1][1] * u[1],
                ];
                previous = u;
            }
            assert_eq!(&trace(&r, "input_record")[i], u);
            for (j, y) in held.iter().enumerate() {
                assert!((trace(&r, "output")[i][j] - y).abs() < 3e-14);
            }
        }
    }
}
#[test]
fn static_rectangular_matrix_matches_all_sequence_components_and_final_row_or_repeated_rows() {
    let (mut q, rows, samples) = plan();
    q.a.clear();
    q.b.clear();
    q.c = vec![vec![], vec![], vec![]];
    q.d = vec![vec![1., 2.], vec![0., -1.], vec![0., 0.]];
    q.initial.clear();
    for repeat in [false, true] {
        let m = source_model(&q, &rows, repeat, samples);
        for model in [&m, &converted(&m, &q)] {
            let r = simulate_graph(model).unwrap();
            for (i, y) in trace(&r, "output").iter().enumerate() {
                let k = i / 10;
                let u = &rows[if repeat {
                    k % rows.len()
                } else {
                    k.min(rows.len() - 1)
                }];
                assert_eq!(y, &vec![u[0] + 2. * u[1], -u[1], 0.]);
            }
        }
    }
}
