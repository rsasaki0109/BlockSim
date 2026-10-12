use blocksim_core::{compare, ModelConfig, PidMode};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "samples/pid-recovery.json".into());
    let config: ModelConfig = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    println!("mode,anti_windup,recovery_iae,settling_seconds,upper_release_seconds,peak_integral");
    for mode in [PidMode::Continuous, PidMode::Discrete] {
        let result = compare(&ModelConfig {
            mode,
            ..config.clone()
        })?;
        for (enabled, run) in [(true, result.enabled), (false, result.disabled)] {
            let m = run.metrics;
            println!(
                "{mode:?},{enabled},{:.6},{:?},{:?},{:.6}",
                m.recovery_iae, m.settling_seconds, m.upper_release_seconds, m.peak_integral
            );
        }
    }
    Ok(())
}
