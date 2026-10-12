//! Independent analytic crossings for the shipped margin example and two harder cases.
use blocksim_core::graph::{
    frequency::{analyze_frequency, FrequencyRequest},
    BlockKind, GraphModel,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plan: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        "samples/frequency/open-loop-plan.json",
    )?)?;
    let model: GraphModel = serde_json::from_value(plan["model"].clone())?;
    let q: FrequencyRequest = serde_json::from_value(plan["request"].clone())?;
    let mut rows = vec![];
    for k in [4.0, 8.0] {
        let mut m = model.clone();
        if let BlockKind::TransferFunction { numerator, .. } = &mut m.blocks[1].params {
            numerator[0] = k;
        }
        let out = analyze_frequency(&m, &q)?.margins.unwrap();
        assert!(out.issues.is_empty());
        assert_eq!(out.gain_crossings.len(), 1);
        assert_eq!(out.phase_crossings.len(), 1);
        let (mut lo, mut hi) = (0.0_f64, 10.0_f64);
        for _ in 0..100 {
            let x = (lo + hi) / 2.0;
            if x * (x + 1.0) * (x + 4.0) < k * k {
                lo = x;
            } else {
                hi = x;
            }
        }
        let w = ((lo + hi) / 2.0).sqrt();
        let pm = 90.0 - w.atan().to_degrees() - (w / 2.0).atan().to_degrees();
        let gm = 20.0 * (6.0 / k).log10();
        let g = &out.gain_crossings[0];
        let p = &out.phase_crossings[0];
        let we = (g.omega - w).abs().max((p.omega - 2.0_f64.sqrt()).abs());
        let ae = (g.phase_margin_deg - pm)
            .abs()
            .max((p.gain_margin_db - gm).abs());
        assert!(we < 1e-10 && ae < 1e-8);
        rows.push(serde_json::json!({"model":format!("{k}/s(s+1)(s+2)"),"gain_crossing_rad_s":g.omega,"phase_margin_deg":g.phase_margin_deg,"phase_crossing_rad_s":p.omega,"gain_margin_db":p.gain_margin_db,"gain_margin_ratio":p.gain_margin,"max_frequency_error":we,"max_margin_error":ae,"additional_evaluations":out.additional_evaluations}));
    }
    let mut m = model.clone();
    m.blocks[1].params = BlockKind::TransferFunction {
        numerator: vec![0.2],
        denominator: vec![1.0, 0.1, 1.0],
        initial: vec![0.0; 2],
    };
    let out = analyze_frequency(&m, &q)?.margins.unwrap();
    assert!(out.issues.is_empty());
    assert_eq!(out.gain_crossings.len(), 2);
    assert!(out.phase_crossings.is_empty());
    let disc = (1.99_f64 * 1.99 - 3.84).sqrt();
    let expected = [((1.99 - disc) / 2.0).sqrt(), ((1.99 + disc) / 2.0).sqrt()];
    let error = out
        .gain_crossings
        .iter()
        .zip(expected)
        .map(|(p, w)| (p.omega - w).abs())
        .fold(0.0_f64, f64::max);
    assert!(error < 1e-10);
    rows.push(serde_json::json!({"model":"0.2/(s²+0.1s+1)","gain_crossings":out.gain_crossings,"phase_crossings":out.phase_crossings,"max_frequency_error":error,"additional_evaluations":out.additional_evaluations}));
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"interpretation":"explicit open-loop L(s), unit negative feedback; finite sampled range, no stability inference","min_omega":q.min_omega,"max_omega":q.max_omega,"points":q.points,"cases":rows})
        )?
    );
    Ok(())
}
