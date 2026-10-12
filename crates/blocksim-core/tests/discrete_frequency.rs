use blocksim_core::graph::{
    frequency::{analyze_discrete_frequency, validate_discrete_frequency, FrequencyRequest},
    simulate_graph, BlockKind, GraphModel,
};
use blocksim_core::{PidConfig, PidMode};
fn model() -> GraphModel {
    serde_json::from_str(include_str!("../../../samples/discrete/feedback.json")).unwrap()
}
fn query() -> FrequencyRequest {
    FrequencyRequest {
        source: "source".into(),
        source_component: 0,
        scope: "output".into(),
        scope_component: 0,
        min_omega: 0.01,
        max_omega: std::f64::consts::PI / 0.1,
        points: 201,
        open_loop: false,
    }
}
#[derive(Clone, Copy)]
struct Complex(f64, f64);
impl Complex {
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
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-10 * (1.0 + b.abs()), "{a} != {b}")
}
fn pid() -> GraphModel {
    let mut m = model();
    m.blocks
        .retain(|b| b.id == "source" || b.id == "sum" || b.id == "output");
    m.blocks[1].id = "pid".into();
    m.blocks[1].params = BlockKind::Pid {
        mode: PidMode::Discrete,
        sample_time: 0.1,
        config: PidConfig {
            kp: 0.4,
            ki: 0.5,
            kd: 0.02,
            derivative_tau: 0.2,
            ..PidConfig::default()
        },
    };
    m.edges.retain(|e| e.from == "source");
    m.edges[0].to = "pid".into();
    let mut edge = m.edges[0].clone();
    edge.from = "pid".into();
    edge.to = "output".into();
    m.edges.push(edge);
    m
}
fn pid_analytic(w: f64) -> Complex {
    let zinv = Complex((w * 0.1).cos(), -(w * 0.1).sin());
    let one_minus = Complex(1.0 - zinv.0, -zinv.1);
    Complex(0.4, 0.0)
        .add(Complex(0.05, 0.0).div(one_minus))
        .add(
            Complex(0.02, 0.0)
                .mul(one_minus)
                .div(Complex(0.3, 0.0).add(Complex(-0.2, 0.0).mul(zinv))),
        )
}
#[test]
fn actual_two_delay_feedback_matches_all_complex_points_of_the_independent_transfer() {
    let r = analyze_discrete_frequency(&model(), &query()).unwrap();
    assert_eq!(r.sample_time, 0.1);
    assert_eq!(r.states, 2);
    assert_eq!(r.response.points.len(), 201);
    assert!(r.response.margins.is_none());
    for p in &r.response.points {
        let theta = p.omega * 0.1;
        let z = Complex(theta.cos(), theta.sin());
        let expected = z.div(
            z.mul(z)
                .add(Complex(-0.8, 0.0).mul(z))
                .add(Complex(0.64, 0.0)),
        );
        assert!(p.error.is_none());
        close(p.real.unwrap(), expected.0);
        close(p.imag.unwrap(), expected.1);
        assert!(p.relative_residual.unwrap() < 1e-10)
    }
}
#[test]
fn one_sample_delay_has_unity_magnitude_exact_nyquist_and_unwrapped_phase() {
    let mut m = model();
    m.blocks
        .retain(|b| b.id == "source" || b.id == "delay1" || b.id == "output");
    m.edges.retain(|e| e.to == "output");
    let mut edge = m.edges[0].clone();
    edge.from = "source".into();
    edge.to = "delay1".into();
    m.edges.push(edge);
    let r = analyze_discrete_frequency(&m, &query()).unwrap();
    for p in &r.response.points {
        close(p.magnitude.unwrap(), 1.0);
        close(p.magnitude_db.unwrap(), 0.0);
        close(
            p.phase_deg.unwrap(),
            -p.omega * 0.1 * 180.0 / std::f64::consts::PI,
        )
    }
    let last = r.response.points.last().unwrap();
    assert_eq!(last.real, Some(-1.0));
    assert_eq!(last.imag.unwrap(), 0.0);
    close(last.phase_deg.unwrap(), -180.0);
}
#[test]
fn sampled_pid_frequency_matches_backward_euler_integral_and_filtered_difference_derivative() {
    let r = analyze_discrete_frequency(&pid(), &query()).unwrap();
    assert_eq!(r.states, 3);
    for p in &r.response.points {
        let expected = pid_analytic(p.omega);
        close(p.real.unwrap(), expected.0);
        close(p.imag.unwrap(), expected.1)
    }
    let last = r.response.points.last().unwrap();
    close(last.real.unwrap(), 0.4 + 0.05 / 2.0 + 0.02 * 2.0 / 0.5);
    close(last.imag.unwrap(), 0.0);
}
#[test]
fn sampled_harmonic_time_responses_agree_with_bode_at_actual_sample_instants() {
    let theta = std::f64::consts::PI / 4.0;
    let w = theta / 0.1;
    for mut m in [model(), pid()] {
        m.duration = 40.0;
        m.blocks[0].params = BlockKind::Sine {
            amplitude: 1.0,
            frequency: w / std::f64::consts::TAU,
            phase: 0.0,
            offset: 0.0,
        };
        let mut q = query();
        q.min_omega = w;
        q.max_omega = w * 1.1;
        q.points = 2;
        let p = analyze_discrete_frequency(&m, &q).unwrap().response.points[0].clone();
        let time = simulate_graph(&m).unwrap();
        let mut re = 0.0;
        let mut im = 0.0;
        for k in 240..400 {
            let y = time.traces[0].values[5 * k][0];
            let t = time.times[5 * k];
            re += y * (w * t).sin() / 80.0;
            im += y * (w * t).cos() / 80.0
        }
        close(re, p.real.unwrap());
        close(im, p.imag.unwrap());
    }
}
#[test]
fn unit_circle_poles_become_explicit_gaps_including_the_exact_nyquist_endpoint() {
    let mut m = model();
    m.blocks[1].params = BlockKind::Sum {
        signs: vec![1.0, 1.0, -1.0],
    };
    let mut q = query();
    q.min_omega = std::f64::consts::PI / 3.0 / 0.1;
    q.max_omega = std::f64::consts::PI / 2.0 / 0.1;
    q.points = 2;
    let r = analyze_discrete_frequency(&m, &q).unwrap();
    assert!(r.response.points[0]
        .error
        .as_ref()
        .unwrap()
        .contains("singular"));
    assert_eq!(r.response.points[0].real, None);
    assert!(r.response.points[1].error.is_none());
    m.blocks.remove(3);
    m.blocks[1].params = BlockKind::Sum {
        signs: vec![1.0, -1.0],
    };
    m.edges.retain(|e| e.from != "delay2" && e.to != "delay2");
    let r = analyze_discrete_frequency(&m, &query()).unwrap();
    let last = r.response.points.last().unwrap();
    assert!(last.error.is_some());
    assert_eq!(last.magnitude, None);
    assert!(!serde_json::to_string(&r).unwrap().contains("Infinity"));
}
#[test]
fn rate_range_flags_and_nonlinear_or_continuous_ancestors_are_refused_before_analysis() {
    let m = model();
    for q in [
        FrequencyRequest {
            max_omega: 32.0,
            ..query()
        },
        FrequencyRequest {
            open_loop: true,
            ..query()
        },
        FrequencyRequest {
            points: 1,
            ..query()
        },
        FrequencyRequest {
            min_omega: 0.0,
            ..query()
        },
        FrequencyRequest {
            max_omega: f64::from_bits(0.01_f64.to_bits() + 1),
            ..query()
        },
    ] {
        assert!(validate_discrete_frequency(&m, &q).is_err())
    }
    let mut m = model();
    if let BlockKind::Delay { sample_time, .. } = &mut m.blocks[3].params {
        *sample_time = 0.2
    }
    assert!(validate_discrete_frequency(&m, &query())
        .unwrap_err()
        .contains("common sample"));
    m.blocks[3].params = BlockKind::FirstOrder {
        gain: 1.0,
        tau: 1.0,
        initial: vec![0.0],
    };
    assert!(validate_discrete_frequency(&m, &query())
        .unwrap_err()
        .contains("continuous"));
    let mut m = pid();
    if let BlockKind::Pid { config, .. } = &mut m.blocks[1].params {
        config.limits.upper = Some(1.0)
    }
    assert!(validate_discrete_frequency(&m, &query()).is_err());
}
#[test]
fn vector_channel_selection_and_static_unrelated_zero_transfers_are_explicit() {
    let mut m = model();
    for b in &mut m.blocks {
        b.width = 2;
        match &mut b.params {
            BlockKind::Step { before, after, .. } => {
                *before = vec![0.0; 2];
                *after = vec![1.0, 2.0]
            }
            BlockKind::Delay { initial, .. } => *initial = vec![0.0; 2],
            _ => (),
        }
    }
    let mut q = query();
    q.source_component = 1;
    q.scope_component = 1;
    let r = analyze_discrete_frequency(&m, &q).unwrap();
    assert_eq!(r.states, 2);
    assert!(r.response.points.iter().all(|p| p.error.is_none()));
    q.source_component = 0;
    let r = analyze_discrete_frequency(&m, &q).unwrap();
    assert!(r
        .response
        .points
        .iter()
        .all(|p| p.magnitude == Some(0.0) && p.magnitude_db.is_none() && p.phase_deg.is_none()));
    let mut m = pid();
    m.blocks[1].params = BlockKind::Gain { gain: 3.0 };
    let r = analyze_discrete_frequency(&m, &query()).unwrap();
    assert_eq!(r.states, 0);
    assert_eq!(r.sample_time, m.dt);
    assert!(r
        .response
        .points
        .iter()
        .all(|p| p.real == Some(3.0) && p.imag == Some(0.0)));
}
#[test]
fn spectrum_reconstruction_failure_does_not_block_finite_frequency_response() {
    let mut m = pid();
    if let BlockKind::Pid { config, .. } = &mut m.blocks[1].params {
        config.ki = 0.0;
        config.kd = 0.0
    }
    let r = analyze_discrete_frequency(&m, &query()).unwrap();
    assert_eq!(r.states, 3);
    for p in r.response.points {
        close(p.real.unwrap(), 0.4);
        close(p.imag.unwrap(), 0.0)
    }
}
