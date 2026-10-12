export interface Limits { lower: number | null; upper: number | null }
export interface PidConfig {
  kp: number; ki: number; kd: number; derivative_tau: number;
  limits: Limits; anti_windup: boolean;
}
export interface ModelConfig {
  mode: 'continuous' | 'discrete'; pid: PidConfig;
  dt: number; duration: number; step_time: number;
  initial_reference: number; final_reference: number;
  plant_gain: number; plant_tau: number; initial_output: number; settling_tolerance: number;
}
export interface Sample { time: number; reference: number; output: number; control: number; integral: number }
export interface Metrics {
  recovery_iae: number; settling_seconds: number | null;
  upper_release_seconds: number | null; peak_integral: number;
}
export interface SimulationResult { samples: Sample[]; metrics: Metrics }
export interface Comparison { enabled: SimulationResult; disabled: SimulationResult }
export interface AppDocument {
  version: 1; model: ModelConfig;
  saturation: { input: number[]; limits: Limits };
}
