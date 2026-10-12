//! PID controllers, elementwise saturation and a reproducible closed-loop experiment.
pub mod graph;

use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputLimits {
    #[serde(default)]
    pub lower: Option<f64>,
    #[serde(default)]
    pub upper: Option<f64>,
}
impl OutputLimits {
    pub fn validate(&self) -> Result<()> {
        if self.lower.is_some_and(|v| !v.is_finite()) || self.upper.is_some_and(|v| !v.is_finite())
        {
            return Err("Output limits must be finite or null".into());
        }
        if matches!((self.lower, self.upper), (Some(l), Some(u)) if l > u) {
            return Err("Lower limit must not exceed upper limit".into());
        }
        Ok(())
    }
    pub fn clamp(&self, value: f64) -> f64 {
        let value = self.lower.map_or(value, |l| value.max(l));
        self.upper.map_or(value, |u| value.min(u))
    }
    /// Uses the sign of the weighted integral change, including negative Ki.
    fn worsens_saturation(&self, raw: f64, integral_change: f64) -> bool {
        (self.upper.is_some_and(|u| raw >= u) && integral_change > 0.0)
            || (self.lower.is_some_and(|l| raw <= l) && integral_change < 0.0)
    }
}

/// Saturation applies the same optional bounds independently to each element.
#[derive(Debug, Clone, Copy)]
pub struct Saturation {
    limits: OutputLimits,
}
impl Saturation {
    pub fn new(limits: OutputLimits) -> Result<Self> {
        limits.validate()?;
        Ok(Self { limits })
    }
    pub fn scalar(&self, input: f64) -> Result<f64> {
        finite("Saturation input", input)?;
        Ok(self.limits.clamp(input))
    }
    pub fn vector(&self, input: &[f64]) -> Result<Vec<f64>> {
        input.iter().map(|v| self.scalar(*v)).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PidConfig {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub derivative_tau: f64,
    #[serde(default)]
    pub limits: OutputLimits,
    #[serde(default)]
    pub anti_windup: bool,
}
impl Default for PidConfig {
    fn default() -> Self {
        Self {
            kp: 2.0,
            ki: 1.0,
            kd: 0.0,
            derivative_tau: 0.05,
            limits: OutputLimits::default(),
            anti_windup: false,
        }
    }
}
impl PidConfig {
    pub fn validate(&self) -> Result<()> {
        for (name, value) in [("Kp", self.kp), ("Ki", self.ki), ("Kd", self.kd)] {
            finite(name, value)?;
        }
        positive("Derivative filter time constant", self.derivative_tau)?;
        self.limits.validate()
    }
}

/// Continuous controller: I' = Ki*e, F' = (e-F)/tau, u = Kp*e + I + Kd*F'.
/// Conditional integration zeros I' only when it drives saturation further out.
#[derive(Debug, Clone)]
pub struct ContinuousPid {
    config: PidConfig,
    state: [f64; 2], // weighted integral and filtered error
}
impl ContinuousPid {
    pub fn new(config: PidConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            state: [0.0; 2],
        })
    }
    fn raw_at(&self, error: f64, state: [f64; 2]) -> f64 {
        self.config.kp * error
            + state[0]
            + self.config.kd * (error - state[1]) / self.config.derivative_tau
    }
    fn rate_at(&self, error: f64, state: [f64; 2]) -> [f64; 2] {
        let integral_rate = self.config.ki * error;
        let frozen = self.config.anti_windup
            && self
                .config
                .limits
                .worsens_saturation(self.raw_at(error, state), integral_rate);
        [
            if frozen { 0.0 } else { integral_rate },
            (error - state[1]) / self.config.derivative_tau,
        ]
    }
    pub fn output(&self, error: f64) -> Result<f64> {
        finite("PID error", error)?;
        let raw = self.raw_at(error, self.state);
        finite("PID internal output", raw)?;
        Ok(self.config.limits.clamp(raw))
    }
    /// Advance continuous state for a held error using RK4, then return output.
    pub fn advance(&mut self, error: f64, dt: f64) -> Result<f64> {
        finite("PID error", error)?;
        positive("Time step", dt)?;
        let next = rk4(self.state, dt, |s| self.rate_at(error, s));
        for value in next {
            finite("PID state", value)?;
        }
        self.state = next;
        self.output(error)
    }
    pub fn integral(&self) -> f64 {
        self.state[0]
    }
}

/// Discrete PID with a backward difference derivative and backward Euler filter.
#[derive(Debug, Clone)]
pub struct DiscretePid {
    config: PidConfig,
    integral: f64,
    previous_error: f64,
    derivative: f64,
}
impl DiscretePid {
    pub(crate) fn analysis_state(&self) -> [f64; 3] {
        [self.integral, self.previous_error, self.derivative]
    }
    pub(crate) fn set_analysis_state(&mut self, state: [f64; 3]) {
        [self.integral, self.previous_error, self.derivative] = state;
    }
    pub fn new(config: PidConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            integral: 0.0,
            previous_error: 0.0,
            derivative: 0.0,
        })
    }
    pub fn step(&mut self, error: f64, dt: f64) -> Result<f64> {
        finite("PID error", error)?;
        positive("Time step", dt)?;
        let derivative = (self.config.derivative_tau * self.derivative + error
            - self.previous_error)
            / (self.config.derivative_tau + dt);
        let base = self.config.kp * error + self.config.kd * derivative;
        let change = self.config.ki * error * dt;
        let candidate = self.integral + change;
        let frozen = self.config.anti_windup
            && self
                .config
                .limits
                .worsens_saturation(base + candidate, change);
        let integral = if frozen { self.integral } else { candidate };
        let raw = base + integral;
        finite("PID internal output", raw)?;
        finite("PID derivative", derivative)?;
        finite("PID integral", integral)?;
        self.derivative = derivative;
        self.integral = integral;
        self.previous_error = error;
        Ok(self.config.limits.clamp(raw))
    }
    pub fn integral(&self) -> f64 {
        self.integral
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PidMode {
    Continuous,
    Discrete,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelConfig {
    pub mode: PidMode,
    pub pid: PidConfig,
    pub dt: f64,
    pub duration: f64,
    pub step_time: f64,
    pub initial_reference: f64,
    pub final_reference: f64,
    pub plant_gain: f64,
    pub plant_tau: f64,
    pub initial_output: f64,
    pub settling_tolerance: f64,
}
impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            mode: PidMode::Continuous,
            pid: PidConfig {
                limits: OutputLimits {
                    lower: Some(0.0),
                    upper: Some(1.0),
                },
                anti_windup: true,
                ..PidConfig::default()
            },
            dt: 0.01,
            duration: 40.0,
            step_time: 5.0,
            initial_reference: 3.0,
            final_reference: 0.5,
            plant_gain: 1.0,
            plant_tau: 1.0,
            initial_output: 0.0,
            settling_tolerance: 0.02,
        }
    }
}
impl ModelConfig {
    pub fn validate(&self) -> Result<()> {
        self.pid.validate()?;
        for (name, value) in [
            ("Time step", self.dt),
            ("Duration", self.duration),
            ("Plant time constant", self.plant_tau),
            ("Settling tolerance", self.settling_tolerance),
        ] {
            positive(name, value)?;
        }
        for (name, value) in [
            ("Step time", self.step_time),
            ("Initial reference", self.initial_reference),
            ("Final reference", self.final_reference),
            ("Plant gain", self.plant_gain),
            ("Initial output", self.initial_output),
        ] {
            finite(name, value)?;
        }
        if self.step_time < 0.0 || self.step_time >= self.duration {
            return Err("Step time must be within [0, duration)".into());
        }
        if self.dt > self.duration || self.duration / self.dt > 100_000.0 {
            return Err("Simulation requires between 1 and 100000 time steps".into());
        }
        if self.mode == PidMode::Continuous
            && self.dt > self.plant_tau.min(self.pid.derivative_tau) / 2.0
        {
            return Err("Continuous RK4 requires dt <= min(plant_tau, derivative_tau)/2".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub time: f64,
    pub reference: f64,
    pub output: f64,
    pub control: f64,
    pub integral: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub recovery_iae: f64,
    pub settling_seconds: Option<f64>,
    pub upper_release_seconds: Option<f64>,
    pub peak_integral: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub samples: Vec<Sample>,
    pub metrics: Metrics,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    pub enabled: SimulationResult,
    pub disabled: SimulationResult,
}

pub fn compare(config: &ModelConfig) -> Result<Comparison> {
    let mut config = config.clone();
    config.pid.anti_windup = true;
    let enabled = simulate(&config)?;
    config.pid.anti_windup = false;
    Ok(Comparison {
        enabled,
        disabled: simulate(&config)?,
    })
}

/// Unity feedback with first-order plant y'=(gain*u-y)/tau.
/// Reference changes are observed at the first sampling point at/after step_time.
pub fn simulate(config: &ModelConfig) -> Result<SimulationResult> {
    config.validate()?;
    let mut continuous = ContinuousPid::new(config.pid)?;
    let mut discrete = DiscretePid::new(config.pid)?;
    let mut output = config.initial_output;
    let steps = (config.duration / config.dt).ceil() as usize;
    let mut samples = Vec::with_capacity(steps + 1);
    for index in 0..=steps {
        let time = (index as f64 * config.dt).min(config.duration);
        let reference = if time + 1e-12 >= config.step_time {
            config.final_reference
        } else {
            config.initial_reference
        };
        let dt = (config.duration - time).min(config.dt);
        let control = match config.mode {
            PidMode::Continuous => continuous.output(reference - output)?,
            PidMode::Discrete if dt > 0.0 => discrete.step(reference - output, dt)?,
            PidMode::Discrete => samples.last().map_or(0.0, |s: &Sample| s.control),
        };
        samples.push(Sample {
            time,
            reference,
            output,
            control,
            integral: match config.mode {
                PidMode::Continuous => continuous.integral(),
                PidMode::Discrete => discrete.integral(),
            },
        });
        if dt <= 0.0 {
            break;
        }
        match config.mode {
            PidMode::Continuous => {
                let next = rk4(
                    [continuous.state[0], continuous.state[1], output],
                    dt,
                    |s| {
                        let error = reference - s[2];
                        let rate = continuous.rate_at(error, [s[0], s[1]]);
                        let u = config
                            .pid
                            .limits
                            .clamp(continuous.raw_at(error, [s[0], s[1]]));
                        [
                            rate[0],
                            rate[1],
                            (config.plant_gain * u - s[2]) / config.plant_tau,
                        ]
                    },
                );
                for value in next {
                    finite("Simulation state", value)?;
                }
                continuous.state = [next[0], next[1]];
                output = next[2];
            }
            PidMode::Discrete => {
                let decay = (-dt / config.plant_tau).exp();
                output = output * decay + config.plant_gain * control * (1.0 - decay);
                finite("Plant output", output)?;
            }
        }
    }
    let metrics = metrics(config, &samples);
    finite("Recovery IAE", metrics.recovery_iae)?;
    Ok(SimulationResult { samples, metrics })
}
fn metrics(config: &ModelConfig, samples: &[Sample]) -> Metrics {
    let recovery: Vec<_> = samples
        .iter()
        .filter(|s| s.time + 1e-12 >= config.step_time)
        .collect();
    let recovery_start = recovery[0].time;
    let recovery_iae = recovery
        .windows(2)
        .map(|w| {
            let a = (w[0].output - config.final_reference).abs();
            let b = (w[1].output - config.final_reference).abs();
            (a + b) * 0.5 * (w[1].time - w[0].time)
        })
        .sum();
    let last_outside = recovery
        .iter()
        .rposition(|s| (s.output - config.final_reference).abs() > config.settling_tolerance);
    let settling_seconds = match last_outside {
        None => Some(0.0),
        Some(i) => recovery.get(i + 1).map(|s| s.time - recovery_start),
    };
    let upper_release_seconds = config.pid.limits.upper.and_then(|upper| {
        recovery
            .iter()
            .find(|s| s.control < upper - 1e-9)
            .map(|s| s.time - recovery_start)
    });
    Metrics {
        recovery_iae,
        settling_seconds,
        upper_release_seconds,
        peak_integral: samples.iter().map(|s| s.integral.abs()).fold(0.0, f64::max),
    }
}
fn rk4<const N: usize>(state: [f64; N], dt: f64, rate: impl Fn([f64; N]) -> [f64; N]) -> [f64; N] {
    let k1 = rate(state);
    let k2 = rate(std::array::from_fn(|i| state[i] + dt * k1[i] / 2.0));
    let k3 = rate(std::array::from_fn(|i| state[i] + dt * k2[i] / 2.0));
    let k4 = rate(std::array::from_fn(|i| state[i] + dt * k3[i]));
    std::array::from_fn(|i| state[i] + dt * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) / 6.0)
}
fn finite(name: &str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("{name} must be finite"))
    }
}
fn positive(name: &str, value: f64) -> Result<()> {
    finite(name, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(format!("{name} must be positive"))
    }
}
