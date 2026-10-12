use blocksim_core::*;
fn limited(anti_windup: bool) -> PidConfig {
    PidConfig {
        kp: 2.0,
        ki: 1.0,
        kd: 0.0,
        anti_windup,
        limits: OutputLimits {
            lower: Some(-1.0),
            upper: Some(1.0),
        },
        ..PidConfig::default()
    }
}
fn near(a: f64, b: f64, tolerance: f64) {
    assert!((a - b).abs() < tolerance, "{a} != {b}");
}
#[test]
fn unlimited_continuous_matches_analytic_legacy_pid() {
    for anti_windup in [false, true] {
        let cfg = PidConfig {
            kp: 1.3,
            ki: 0.7,
            kd: 0.2,
            derivative_tau: 0.1,
            anti_windup,
            ..PidConfig::default()
        };
        let mut pid = ContinuousPid::new(cfg).unwrap();
        let error = 0.8;
        for n in 1..=400 {
            let time = n as f64 * 0.005;
            let actual = pid.advance(error, 0.005).unwrap();
            let expected = cfg.kp * error
                + cfg.ki * error * time
                + cfg.kd * error / cfg.derivative_tau * (-time / cfg.derivative_tau).exp();
            near(actual, expected, 1e-7);
        }
    }
}
#[test]
fn unlimited_discrete_matches_legacy_recurrence() {
    let cfg = PidConfig {
        kp: 0.4,
        ki: 0.7,
        kd: 0.3,
        ..PidConfig::default()
    };
    for anti_windup in [false, true] {
        let mut pid = DiscretePid::new(PidConfig { anti_windup, ..cfg }).unwrap();
        let (mut integral, mut previous, mut derivative) = (0.0, 0.0, 0.0);
        for error in [2.0, 1.0, -1.0, -4.0, 0.0, 0.1] {
            integral += cfg.ki * error * 0.01;
            derivative =
                (cfg.derivative_tau * derivative + error - previous) / (cfg.derivative_tau + 0.01);
            previous = error;
            near(
                pid.step(error, 0.01).unwrap(),
                cfg.kp * error + integral + cfg.kd * derivative,
                1e-12,
            );
        }
    }
}
#[test]
fn anti_windup_has_no_effect_without_limits_on_closed_loop() {
    for mode in [PidMode::Continuous, PidMode::Discrete] {
        let cfg = ModelConfig {
            mode,
            pid: PidConfig::default(),
            ..ModelConfig::default()
        };
        let result = compare(&cfg).unwrap();
        assert_eq!(
            serde_json::to_string(&result.enabled).unwrap(),
            serde_json::to_string(&result.disabled).unwrap()
        );
    }
}
#[test]
fn output_obeys_two_sided_one_sided_and_equal_limits() {
    for limits in [
        OutputLimits {
            lower: Some(-0.3),
            upper: Some(0.7),
        },
        OutputLimits {
            lower: None,
            upper: Some(0.7),
        },
        OutputLimits {
            lower: Some(-0.3),
            upper: None,
        },
        OutputLimits {
            lower: Some(0.2),
            upper: Some(0.2),
        },
    ] {
        for anti_windup in [false, true] {
            let cfg = PidConfig {
                limits,
                anti_windup,
                ..limited(false)
            };
            let mut c = ContinuousPid::new(cfg).unwrap();
            let mut d = DiscretePid::new(cfg).unwrap();
            for error in [10.0, 10.0, -10.0, -10.0, 0.0, 2.0] {
                for u in [
                    c.advance(error, 0.01).unwrap(),
                    d.step(error, 0.01).unwrap(),
                ] {
                    assert!(limits.lower.is_none_or(|v| u >= v));
                    assert!(limits.upper.is_none_or(|v| u <= v));
                }
            }
        }
    }
}
#[test]
fn freezes_only_worsening_direction_at_both_bounds_including_negative_ki() {
    for (kp, ki, error) in [
        (2.0, 1.0, 2.0),
        (2.0, 1.0, -2.0),
        (-2.0, -1.0, 2.0),
        (-2.0, -1.0, -2.0),
    ] {
        let config = PidConfig {
            kp,
            ki,
            ..limited(false)
        };
        let mut c = ContinuousPid::new(PidConfig {
            anti_windup: true,
            ..config
        })
        .unwrap();
        let mut d = DiscretePid::new(PidConfig {
            anti_windup: true,
            ..config
        })
        .unwrap();
        for _ in 0..100 {
            c.advance(error, 0.01).unwrap();
            d.step(error, 0.01).unwrap();
        }
        near(c.integral(), 0.0, 1e-12);
        near(d.integral(), 0.0, 1e-12);
    }
}
#[test]
fn inward_integral_continues_while_raw_output_is_still_saturated() {
    // Negative proportional gain separates the P and I directions. Large P
    // keeps raw output at a bound while the I increment points inward.
    for error in [-2.0, 2.0] {
        let cfg = PidConfig {
            kp: -10.0,
            ki: 1.0,
            ..limited(true)
        };
        let mut c = ContinuousPid::new(cfg).unwrap();
        let mut d = DiscretePid::new(cfg).unwrap();
        for _ in 0..100 {
            c.advance(error, 0.01).unwrap();
            d.step(error, 0.01).unwrap();
        }
        near(c.integral(), error, 1e-12);
        near(d.integral(), error, 1e-12);
        let bound = if error < 0.0 { 1.0 } else { -1.0 };
        near(c.output(error).unwrap(), bound, 1e-12);
    }
}
#[test]
fn saturated_sample_demonstrates_recovery_improvement_for_both_modes() {
    for mode in [PidMode::Continuous, PidMode::Discrete] {
        let cfg = ModelConfig {
            mode,
            ..ModelConfig::default()
        };
        let result = compare(&cfg).unwrap();
        for run in [&result.enabled, &result.disabled] {
            assert_eq!(run.samples.len(), 4001);
            assert!(run
                .samples
                .iter()
                .all(|s| s.control >= 0.0 && s.control <= 1.0));
        }
        let enabled = &result.enabled.metrics;
        let disabled = &result.disabled.metrics;
        println!("{mode:?}: enabled={enabled:?}, disabled={disabled:?}");
        assert!(enabled.recovery_iae < disabled.recovery_iae * 0.25);
        assert!(enabled.upper_release_seconds.unwrap() < 0.02);
        assert!(disabled.upper_release_seconds.unwrap() > 10.0);
        assert!(enabled.settling_seconds.unwrap() < disabled.settling_seconds.unwrap());
    }
}
#[test]
fn saturation_handles_vectors_empty_inputs_and_invalid_values() {
    let sat = Saturation::new(OutputLimits {
        lower: Some(-1.0),
        upper: Some(2.0),
    })
    .unwrap();
    assert_eq!(
        sat.vector(&[-4.0, -1.0, 0.5, 2.0, 9.0]).unwrap(),
        vec![-1.0, -1.0, 0.5, 2.0, 2.0]
    );
    assert!(sat.vector(&[]).unwrap().is_empty());
    assert!(sat.scalar(f64::NAN).is_err());
    assert!(sat.vector(&[f64::INFINITY]).is_err());
    assert!(Saturation::new(OutputLimits {
        lower: Some(2.0),
        upper: Some(1.0)
    })
    .is_err());
    assert!(Saturation::new(OutputLimits {
        lower: None,
        upper: Some(f64::INFINITY)
    })
    .is_err());
}
#[test]
fn legacy_config_defaults_to_unlimited_and_disabled_and_roundtrips_new_fields() {
    let legacy = r#"{"kp":2,"ki":1,"kd":0,"derivative_tau":0.05}"#;
    let parsed: PidConfig = serde_json::from_str(legacy).unwrap();
    assert_eq!(parsed, PidConfig::default());
    let model = ModelConfig::default();
    let restored: ModelConfig =
        serde_json::from_str(&serde_json::to_string(&model).unwrap()).unwrap();
    assert_eq!(model, restored);
}
#[test]
fn invalid_simulations_fail_before_allocation_and_controllers_reject_nonfinite_inputs() {
    let default = ModelConfig::default();
    for cfg in [
        ModelConfig {
            dt: 0.0,
            ..default.clone()
        },
        ModelConfig {
            dt: 1e-12,
            ..default.clone()
        },
        ModelConfig {
            dt: 0.1,
            ..default.clone()
        },
        ModelConfig {
            step_time: 40.0,
            ..default.clone()
        },
        ModelConfig {
            plant_tau: -1.0,
            ..default.clone()
        },
        ModelConfig {
            final_reference: f64::NAN,
            ..default.clone()
        },
    ] {
        assert!(simulate(&cfg).is_err());
    }
    assert!(DiscretePid::new(PidConfig {
        ki: f64::NAN,
        ..PidConfig::default()
    })
    .is_err());
    let mut c = ContinuousPid::new(PidConfig::default()).unwrap();
    let mut d = DiscretePid::new(PidConfig::default()).unwrap();
    assert!(c.advance(f64::INFINITY, 0.01).is_err());
    assert!(d.step(1.0, 0.0).is_err());
}
