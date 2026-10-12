//! Independent held-step and complex-frequency references for actual converted models.
use blocksim_core::graph::{
    discretize::{discretize_zoh, ZohRequest},
    frequency::{analyze_discrete_frequency, FrequencyRequest},
    simulate_graph, BlockKind, GraphModel,
};
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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source: GraphModel =
        serde_json::from_str(include_str!("../../../samples/discrete/zoh.json"))?;
    let mut cases = vec![];
    for name in [
        "first_order",
        "second_order",
        "double_integrator",
        "oscillator",
    ] {
        let ts = 0.2;
        let (num, den) = match name {
            "first_order" => (vec![1.0], vec![1.0, 1.0]),
            "second_order" => (vec![1.0], vec![1.0, 2.0, 1.0]),
            "double_integrator" => (vec![1.0], vec![1.0, 0.0, 0.0]),
            _ => (vec![4.0], vec![1.0, 0.0, 4.0]),
        };
        let r = discretize_zoh(&ZohRequest {
            numerator: num,
            denominator: den,
            sample_time: ts,
        })?;
        let mut m = source.clone();
        m.blocks[1].params = BlockKind::DiscreteTransferFunction {
            numerator: r.numerator.clone(),
            denominator: r.denominator.clone(),
            initial: vec![0.0; r.denominator.len() - 1],
            sample_time: ts,
        };
        let time = simulate_graph(&m)?;
        let step = |t: f64| {
            if t < 0.0 {
                0.0
            } else {
                match name {
                    "first_order" => 1.0 - (-t).exp(),
                    "second_order" => 1.0 - (1.0 + t) * (-t).exp(),
                    "double_integrator" => t * t / 2.0,
                    _ => 1.0 - (2.0 * t).cos(),
                }
            }
        };
        let mut error: f64 = 0.0;
        for (tick, row) in time.traces[0].values.iter().enumerate() {
            let t = (tick / 20) as f64 * ts;
            error = error.max((row[0] - (step(t) - step(t - 2.0))).abs());
        }
        assert!(error < 1e-9);
        let q = FrequencyRequest {
            source: "source".into(),
            scope: "output".into(),
            source_component: 0,
            scope_component: 0,
            min_omega: 0.01,
            max_omega: std::f64::consts::PI / ts,
            points: 201,
            open_loop: false,
        };
        let frequency = analyze_discrete_frequency(&m, &q)?;
        let mut complex_error: f64 = 0.0;
        for p in &frequency.response.points {
            let angle = p.omega * ts;
            let z = C(angle.cos(), angle.sin());
            let e = (-ts).exp();
            let expected = match name {
                "first_order" => C(1.0 - e, 0.0).div(z.add(C(-e, 0.0))),
                "second_order" => z
                    .mul(C(1.0 - (1.0 + ts) * e, 0.0))
                    .add(C(e * e + (ts - 1.0) * e, 0.0))
                    .div(z.add(C(-e, 0.0)).mul(z.add(C(-e, 0.0)))),
                "double_integrator" => z
                    .add(C(1.0, 0.0))
                    .mul(C(ts * ts / 2.0, 0.0))
                    .div(z.add(C(-1.0, 0.0)).mul(z.add(C(-1.0, 0.0)))),
                _ => z.add(C(1.0, 0.0)).mul(C(1.0 - (2.0 * ts).cos(), 0.0)).div(
                    z.mul(z)
                        .add(z.mul(C(-2.0 * (2.0 * ts).cos(), 0.0)))
                        .add(C(1.0, 0.0)),
                ),
            };
            complex_error = complex_error.max(
                (p.real.unwrap() - expected.0).hypot(p.imag.unwrap() - expected.1)
                    / (1.0 + expected.0.hypot(expected.1)),
            );
        }
        assert!(complex_error < 1e-9);
        cases.push(serde_json::json!({"case":name,"conversion":r,"recorded_points":time.times.len(),"input_drop_s":2,"max_sample_and_hold_error_vs_independent_pulse":error,"max_normalized_complex_error":complex_error,"first_outputs":time.traces[0].values.iter().step_by(20).take(6).collect::<Vec<_>>(),"final_output":time.traces[0].values.last()}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"reference":"Ts=.2, unit input0..2s then zero; sample-instant output=independent step(t)-step(t-2), held between samples; first/second lowpass, 1/s^2 and4/(s^2+4). Independent closed-form discrete transfers, normalized complex error/(1+|G|).","cases":cases})
        )?
    );
    Ok(())
}
