//! Independent closed-form transfer and sampled harmonic time verification.
use blocksim_core::graph::{
    frequency::{analyze_discrete_frequency, FrequencyRequest},
    simulate_graph, BlockKind, GraphModel,
};
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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let original: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/feedback.json"))?;
    let mut cases = vec![];
    for name in ["two_delay_feedback", "one_sample_delay"] {
        let mut m = original.clone();
        if name == "one_sample_delay" {
            m.blocks
                .retain(|b| b.id == "source" || b.id == "delay1" || b.id == "output");
            m.edges.retain(|e| e.to == "output");
            let mut edge = m.edges[0].clone();
            edge.from = "source".into();
            edge.to = "delay1".into();
            m.edges.push(edge)
        }
        let q = FrequencyRequest {
            source: "source".into(),
            source_component: 0,
            scope: "output".into(),
            scope_component: 0,
            min_omega: 0.01,
            max_omega: std::f64::consts::PI / 0.1,
            points: 201,
            open_loop: false,
        };
        let r = analyze_discrete_frequency(&m, &q)?;
        let mut max_error = 0.0_f64;
        let mut max_residual = 0.0_f64;
        for p in &r.response.points {
            let z = Complex((p.omega * 0.1).cos(), (p.omega * 0.1).sin());
            let expected = if name == "one_sample_delay" {
                Complex(1.0, 0.0).div(z)
            } else {
                z.div(
                    z.mul(z)
                        .add(Complex(-0.8, 0.0).mul(z))
                        .add(Complex(0.64, 0.0)),
                )
            };
            assert!(p.error.is_none());
            max_error =
                max_error.max((p.real.unwrap() - expected.0).hypot(p.imag.unwrap() - expected.1));
            max_residual = max_residual.max(p.relative_residual.unwrap())
        }
        assert!(max_error < 1e-10);
        let w = std::f64::consts::PI / 4.0 / 0.1;
        m.duration = 40.0;
        m.blocks[0].params = BlockKind::Sine {
            amplitude: 1.0,
            frequency: w / std::f64::consts::TAU,
            phase: 0.0,
            offset: 0.0,
        };
        let r2 = analyze_discrete_frequency(
            &m,
            &FrequencyRequest {
                min_omega: w,
                max_omega: w * 1.1,
                points: 2,
                ..q
            },
        )?;
        let p = &r2.response.points[0];
        let time = simulate_graph(&m)?;
        let mut re = 0.0;
        let mut im = 0.0;
        for k in 240..400 {
            let y = time.traces[0].values[5 * k][0];
            let t = time.times[5 * k];
            re += y * (w * t).sin() / 80.0;
            im += y * (w * t).cos() / 80.0
        }
        let harmonic_error = (re - p.real.unwrap()).hypot(im - p.imag.unwrap());
        assert!(harmonic_error < 1e-10);
        cases.push(serde_json::json!({"case":name,"sample_time_s":r.sample_time,"nyquist_rad_s":r.nyquist_omega,"states":r.states,"points":r.response.points.len(),"max_complex_error_vs_closed_form":max_error,"max_linear_solve_residual":max_residual,"nyquist_response":r.response.points.last(),"harmonic_omega_rad_s":w,"harmonic_window_s":[24,40],"harmonic_samples":160,"time_recorded_points":time.times.len(),"harmonic_real":re,"harmonic_imag":im,"frequency_real":p.real,"frequency_imag":p.imag,"harmonic_complex_error":harmonic_error}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"frequency_reference":"z/(z^2-0.8z+0.64), or 1/z; z=exp(j*omega*0.1)","time_reference":"actual sine-driven simulator fitted on 160 sample instants (20 full cycles), sample-and-hold reconstruction excluded","cases":cases})
        )?
    );
    Ok(())
}
