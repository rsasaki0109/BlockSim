//! JSON boundary shared by browser WASM and native contract tests.
use blocksim_core::{compare, simulate, ModelConfig, OutputLimits, Saturation};
use wasm_bindgen::prelude::*;

const MAX_JSON_BYTES: usize = 1_000_000;
fn decode<T: serde::de::DeserializeOwned>(json: &str) -> Result<T, String> {
    if json.len() > MAX_JSON_BYTES {
        return Err("JSON input exceeds 1 MB".into());
    }
    serde_json::from_str(json).map_err(|e| e.to_string())
}
fn encode<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}
pub fn run_json(json: &str) -> Result<String, String> {
    let config: ModelConfig = decode(json)?;
    encode(&simulate(&config)?)
}
pub fn compare_json(json: &str) -> Result<String, String> {
    let config: ModelConfig = decode(json)?;
    encode(&compare(&config)?)
}
pub fn validate_json(json: &str) -> Result<String, String> {
    let config: ModelConfig = decode(json)?;
    config.validate()?;
    encode(&config)
}
pub fn saturation_json(input: &str, limits: &str) -> Result<String, String> {
    let input: Vec<f64> = decode(input)?;
    let limits: OutputLimits = decode(limits)?;
    encode(&Saturation::new(limits)?.vector(&input)?)
}
#[wasm_bindgen]
pub fn run_model(json: &str) -> Result<String, JsValue> {
    run_json(json).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn compare_model(json: &str) -> Result<String, JsValue> {
    compare_json(json).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn validate_model(json: &str) -> Result<String, JsValue> {
    validate_json(json).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn saturate_vector(input: &str, limits: &str) -> Result<String, JsValue> {
    saturation_json(input, limits).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn default_model() -> String {
    serde_json::to_string(&ModelConfig::default()).expect("finite default config")
}

pub fn graph_json(json: &str) -> Result<String, String> {
    let graph: blocksim_core::graph::GraphModel = decode(json)?;
    encode(&blocksim_core::graph::simulate_graph(&graph)?)
}
pub fn tustin_json(request: &str) -> Result<String, String> {
    let q = decode(request)?;
    encode(&blocksim_core::graph::discretize::discretize_tustin(&q)?)
}
pub fn zoh_json(request: &str) -> Result<String, String> {
    let q = decode(request)?;
    encode(&blocksim_core::graph::discretize::discretize_zoh(&q)?)
}
#[wasm_bindgen]
pub fn discretize_zoh(request: &str) -> Result<String, JsValue> {
    zoh_json(request).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn discretize_tustin(request: &str) -> Result<String, JsValue> {
    tustin_json(request).map_err(|e| JsValue::from_str(&e))
}
pub fn graph_budget_json(json: &str, limit: usize) -> Result<String, String> {
    let graph = decode(json)?;
    encode(&blocksim_core::graph::simulate_graph_with_budget(
        &graph, limit,
    )?)
}
pub fn frequency_json(model: &str, request: &str) -> Result<String, String> {
    let graph = decode(model)?;
    let request = decode(request)?;
    encode(&blocksim_core::graph::frequency::analyze_frequency(
        &graph, &request,
    )?)
}
pub fn frequency_check_json(model: &str, request: &str) -> Result<String, String> {
    let graph = decode(model)?;
    let request = decode(request)?;
    encode(&blocksim_core::graph::frequency::validate_frequency(
        &graph, &request,
    )?)
}
pub fn discrete_frequency_json(model: &str, request: &str) -> Result<String, String> {
    let graph = decode(model)?;
    let request = decode(request)?;
    encode(&blocksim_core::graph::frequency::analyze_discrete_frequency(&graph, &request)?)
}
pub fn discrete_frequency_check_json(model: &str, request: &str) -> Result<String, String> {
    let graph = decode(model)?;
    let request = decode(request)?;
    encode(&blocksim_core::graph::frequency::validate_discrete_frequency(&graph, &request)?)
}
#[wasm_bindgen]
pub fn analyze_discrete_frequency(model: &str, request: &str) -> Result<String, JsValue> {
    discrete_frequency_json(model, request).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn validate_discrete_frequency(model: &str, request: &str) -> Result<String, JsValue> {
    discrete_frequency_check_json(model, request).map_err(|e| JsValue::from_str(&e))
}
pub fn pole_zero_json(model: &str, request: &str) -> Result<String, String> {
    let graph = decode(model)?;
    let request = decode(request)?;
    encode(&blocksim_core::graph::pole_zero::analyze_pole_zero(
        &graph, &request,
    )?)
}
pub fn state_space_json(model: &str, request: &str) -> Result<String, String> {
    let graph = decode(model)?;
    let request = decode(request)?;
    encode(&blocksim_core::graph::state_space::analyze_state_space(
        &graph, &request,
    )?)
}
pub fn state_space_check_json(model: &str, request: &str) -> Result<String, String> {
    let graph = decode(model)?;
    let request = decode(request)?;
    encode(&blocksim_core::graph::state_space::validate_state_space(
        &graph, &request,
    )?)
}
pub fn discrete_state_space_json(model: &str, request: &str) -> Result<String, String> {
    let graph = decode(model)?;
    let request = decode(request)?;
    encode(&blocksim_core::graph::state_space::analyze_discrete_state_space(&graph, &request)?)
}
pub fn discrete_state_space_check_json(model: &str, request: &str) -> Result<String, String> {
    let graph = decode(model)?;
    let request = decode(request)?;
    encode(&blocksim_core::graph::state_space::validate_discrete_state_space(&graph, &request)?)
}
#[wasm_bindgen]
pub fn analyze_discrete_state_space(model: &str, request: &str) -> Result<String, JsValue> {
    discrete_state_space_json(model, request).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn validate_discrete_state_space(model: &str, request: &str) -> Result<String, JsValue> {
    discrete_state_space_check_json(model, request).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn analyze_state_space(model: &str, request: &str) -> Result<String, JsValue> {
    state_space_json(model, request).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn validate_state_space(model: &str, request: &str) -> Result<String, JsValue> {
    state_space_check_json(model, request).map_err(|e| JsValue::from_str(&e))
}
pub fn pole_zero_check_json(model: &str, request: &str) -> Result<String, String> {
    let graph = decode(model)?;
    let request = decode(request)?;
    encode(&blocksim_core::graph::pole_zero::validate_pole_zero(
        &graph, &request,
    )?)
}
#[wasm_bindgen]
pub fn analyze_pole_zero(model: &str, request: &str) -> Result<String, JsValue> {
    pole_zero_json(model, request).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn validate_pole_zero(model: &str, request: &str) -> Result<String, JsValue> {
    pole_zero_check_json(model, request).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn analyze_frequency(model: &str, request: &str) -> Result<String, JsValue> {
    frequency_json(model, request).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn validate_frequency(model: &str, request: &str) -> Result<String, JsValue> {
    frequency_check_json(model, request).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn run_graph_budgeted(json: &str, limit: usize) -> Result<String, JsValue> {
    graph_budget_json(json, limit).map_err(|e| JsValue::from_str(&e))
}
pub fn normalize_graph_json(json: &str, runnable: bool) -> Result<String, String> {
    let graph: blocksim_core::graph::GraphModel = decode(json)?;
    if runnable {
        blocksim_core::graph::validate_graph(&graph)?;
    } else {
        blocksim_core::graph::validate_draft(&graph)?;
    }
    encode(&graph)
}
pub fn diagnose_graph_json(json: &str) -> Result<String, String> {
    let graph: blocksim_core::graph::GraphModel = decode(json)?;
    encode(&blocksim_core::graph::diagnose_graph(&graph))
}
#[wasm_bindgen]
pub fn diagnose_graph(json: &str) -> Result<String, JsValue> {
    diagnose_graph_json(json).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn run_graph(json: &str) -> Result<String, JsValue> {
    graph_json(json).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn validate_graph(json: &str) -> Result<String, JsValue> {
    normalize_graph_json(json, true).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn normalize_graph(json: &str) -> Result<String, JsValue> {
    normalize_graph_json(json, false).map_err(|e| JsValue::from_str(&e))
}

#[cfg(test)]
mod tests {
    #[test]
    fn rate_limiter_boundary_roundtrips_initial_rates_period_and_all_sampled_holds() {
        let text = include_str!("../../../samples/graphs/rate-limiter.json");
        let normalized = normalize_graph_json(text, true).unwrap();
        let model: serde_json::Value = serde_json::from_str(&normalized).unwrap();
        assert_eq!(
            model["blocks"][1]["params"],
            serde_json::json!({"kind":"discrete_rate_limiter","rising_rate":2.0,"falling_rate":1.0,"initial":[0.0,0.0],"sample_time":0.125})
        );
        assert_eq!(
            normalize_graph_json(&normalized, false).unwrap(),
            normalized
        );
        let r: serde_json::Value = serde_json::from_str(&graph_json(&normalized).unwrap()).unwrap();
        assert_eq!(r["times"].as_array().unwrap().len(), 151);
        for (k, row) in r["traces"][0]["values"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let n = k / 5;
            let expected = if n < 12 {
                [0.25 * n as f64, -0.125 * n as f64].map(|v| v.clamp(-1., 1.))
            } else {
                [
                    (1. - 0.125 * (n - 11) as f64).max(-1.),
                    (-1. + 0.25 * (n - 11) as f64).min(1.),
                ]
            };
            assert_eq!(row, &serde_json::json!(expected));
        }
    }
    #[test]
    fn rate_limiter_boundary_rejects_missing_unknown_negative_rates_bad_initial_and_period() {
        let original: serde_json::Value =
            serde_json::from_str(include_str!("../../../samples/graphs/rate-limiter.json"))
                .unwrap();
        for (key, value) in [
            ("rising_rate", serde_json::json!(-1)),
            ("falling_rate", serde_json::json!(-1)),
            ("rising_rate", serde_json::json!(null)),
            ("falling_rate", serde_json::json!([1, 1])),
            ("initial", serde_json::json!([0])),
            ("sample_time", serde_json::json!(0.13)),
            ("extra", serde_json::json!(0)),
        ] {
            let mut m = original.clone();
            m["blocks"][1]["params"][key] = value;
            assert!(normalize_graph_json(&m.to_string(), false).is_err());
            assert!(graph_json(&m.to_string()).is_err());
        }
        for key in ["rising_rate", "falling_rate", "initial", "sample_time"] {
            let mut m = original.clone();
            m["blocks"][1]["params"]
                .as_object_mut()
                .unwrap()
                .remove(key);
            assert!(graph_json(&m.to_string()).is_err());
        }
    }
    #[test]
    fn dead_zone_boundary_roundtrips_thresholds_and_actual_vector_rows() {
        let text = include_str!("../../../samples/graphs/dead-zone.json");
        let normalized = normalize_graph_json(text, true).unwrap();
        let m: serde_json::Value = serde_json::from_str(&normalized).unwrap();
        assert_eq!(
            m["blocks"][1]["params"],
            serde_json::json!({"kind":"dead_zone","lower":-0.2,"upper":0.3})
        );
        assert_eq!(
            normalize_graph_json(&normalized, false).unwrap(),
            normalized
        );
        let r: serde_json::Value = serde_json::from_str(&graph_json(&normalized).unwrap()).unwrap();
        assert_eq!(r["times"].as_array().unwrap().len(), 161);
        let expected = [
            [-0.6, 0.5],
            [-0.05, 0.],
            [0., 0.],
            [0., 0.],
            [0., 0.],
            [0., 0.],
            [0., 0.],
            [0.35, -0.45],
        ];
        for (k, row) in r["traces"][0]["values"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            for (j, value) in expected[(k / 10) % 8].iter().enumerate() {
                assert!((row[j].as_f64().unwrap() - value).abs() < 1e-14);
            }
        }
    }
    #[test]
    fn dead_zone_boundary_rejects_bad_thresholds_fields_and_computed_overflow() {
        let original: serde_json::Value =
            serde_json::from_str(include_str!("../../../samples/graphs/dead-zone.json")).unwrap();
        for params in [
            serde_json::json!({"kind":"dead_zone","lower":1,"upper":0}),
            serde_json::json!({"kind":"dead_zone","lower":null,"upper":1}),
            serde_json::json!({"kind":"dead_zone","lower":0}),
            serde_json::json!({"kind":"dead_zone","lower":0,"upper":1,"limits":{}}),
            serde_json::json!({"kind":"dead_zone","lower":[0,0],"upper":1}),
        ] {
            let mut m = original.clone();
            m["blocks"][1]["params"] = params;
            assert!(normalize_graph_json(&m.to_string(), false).is_err());
            assert!(graph_json(&m.to_string()).is_err());
        }
        let mut m = original;
        m["blocks"][0]["params"] = serde_json::json!({"kind":"constant","value":[1e308,1e308]});
        m["blocks"][1]["params"] =
            serde_json::json!({"kind":"dead_zone","lower":-1e308,"upper":-1e308});
        normalize_graph_json(&m.to_string(), true).unwrap();
        assert!(graph_json(&m.to_string())
            .unwrap_err()
            .contains("zone at t=0"));
    }
    #[test]
    fn sampled_input_boundary_preserves_table_and_rejects_bad_rows_and_unknown_fields() {
        let text = include_str!("../../../samples/discrete/sequence.json");
        let normalized = normalize_graph_json(text, false).unwrap();
        let original: serde_json::Value = serde_json::from_str(text).unwrap();
        let source_model: blocksim_core::graph::GraphModel = serde_json::from_str(text).unwrap();
        let stored_model: blocksim_core::graph::GraphModel =
            serde_json::from_str(&normalized).unwrap();
        assert_eq!(stored_model.blocks[0].params, source_model.blocks[0].params);
        let run: serde_json::Value = serde_json::from_str(&graph_json(text).unwrap()).unwrap();
        let input = run["traces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == "input_record")
            .unwrap();
        for k in 0..101 {
            assert_eq!(
                input["values"][k * 10][0].as_f64(),
                original["blocks"][0]["params"]["values"][k][0].as_f64()
            );
        }
        let mut bad = original.clone();
        bad["blocks"][0]["params"]["values"] = serde_json::json!([[1, 2]]);
        assert!(graph_json(&bad.to_string()).is_err());
        bad = original.clone();
        bad["blocks"][0]["params"]["interpolate"] = serde_json::json!(true);
        assert!(normalize_graph_json(&bad.to_string(), true).is_err());
        bad["blocks"][0]["params"]
            .as_object_mut()
            .unwrap()
            .remove("interpolate");
        bad["blocks"][0]["params"]["values"] = serde_json::json!(vec![vec![0.0]; 2002]);
        assert!(graph_json(&bad.to_string()).is_err());
    }
    #[test]
    fn repeating_input_roundtrips_and_runs_every_cycle_through_json_boundary() {
        let text = include_str!("../../../samples/discrete/repeat.json");
        let normalized = normalize_graph_json(text, false).unwrap();
        let stored: serde_json::Value = serde_json::from_str(&normalized).unwrap();
        assert_eq!(stored["blocks"][0]["params"]["repeat"], true);
        let result: serde_json::Value =
            serde_json::from_str(&graph_json(&normalized).unwrap()).unwrap();
        let input = result["traces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == "input_record")
            .unwrap();
        for (i, row) in input["values"].as_array().unwrap().iter().enumerate() {
            assert_eq!(row[0].as_f64().unwrap(), [0.0, 1.0, -0.5][(i / 10) % 3]);
        }
        let mut bad = stored;
        bad["blocks"][0]["params"]["repeat"] = serde_json::json!(1);
        assert!(normalize_graph_json(&bad.to_string(), true).is_err());
    }
    use super::*;
    #[test]
    fn zoh_boundary_serializes_realization_and_refuses_prewarp_or_bad_request() {
        let q = r#"{"numerator":[1],"denominator":[1,1],"sample_time":0.1}"#;
        let r: serde_json::Value = serde_json::from_str(&zoh_json(q).unwrap()).unwrap();
        assert_eq!(r["numerator"][0], 0.0);
        assert!((r["a"][0][0].as_f64().unwrap() - (-0.1_f64).exp()).abs() < 1e-13);
        assert!(r["reconstruction_residual"].as_f64().unwrap() < 1e-9);
        for bad in [
            q.replace("0.1", "0"),
            q.replace("0.1", "null"),
            q.replace("0.1}", "0.1,\"prewarp_omega\":1}"),
            q.replace("0.1}", "0.1,\"method\":\"zoh\"}"),
        ] {
            assert!(zoh_json(&bad).is_err());
        }
        assert!(zoh_json(&" ".repeat(MAX_JSON_BYTES + 1)).is_err());
    }
    #[test]
    fn tustin_boundary_serializes_finite_coefficients_and_rejects_invalid_or_unknown_input() {
        let q = r#"{"numerator":[1],"denominator":[1,1],"sample_time":0.1}"#;
        let r: serde_json::Value = serde_json::from_str(&tustin_json(q).unwrap()).unwrap();
        assert_eq!(r["mapping_scale"], 20.0);
        assert!((r["numerator"][0].as_f64().unwrap() - 1.0 / 21.0).abs() < 1e-14);
        for bad in [
            q.replace("0.1", "0"),
            q.replace("0.1", "null"),
            q.replace("0.1", "\"0.1\""),
            q.replace("0.1}", "0.1,\"method\":\"zoh\"}"),
            q.replace("0.1}", "0.1,\"prewarp_omega\":40}"),
        ] {
            assert!(tustin_json(&bad).is_err());
        }
        assert!(tustin_json(&" ".repeat(MAX_JSON_BYTES + 1)).is_err());
    }
    #[test]
    fn discrete_transfer_roundtrip_execution_analysis_and_invalid_period_boundary() {
        let model = include_str!("../../../samples/discrete/transfer.json");
        let normalized = normalize_graph_json(model, true).unwrap();
        assert_eq!(
            normalize_graph_json(&normalized, false).unwrap(),
            normalized
        );
        let r: serde_json::Value = serde_json::from_str(&graph_json(&normalized).unwrap()).unwrap();
        assert!((r["traces"][0]["values"][0][0].as_f64().unwrap() - 0.2).abs() < 1e-14);
        assert_eq!(r["traces"][1]["values"][0][0], 0.0);
        let q = r#"{"source":"source","source_component":0,"scope":"output","scope_component":0}"#;
        let ss: serde_json::Value =
            serde_json::from_str(&discrete_state_space_json(&normalized, q).unwrap()).unwrap();
        assert_eq!(ss["sample_time"], 0.1);
        assert_eq!(ss["stability"]["classification"], "stable");
        assert!(state_space_json(&normalized, q).is_err());
        let mut bad: serde_json::Value = serde_json::from_str(&normalized).unwrap();
        bad["blocks"][1]["params"]["sample_time"] = serde_json::json!(0.03);
        assert!(normalize_graph_json(&bad.to_string(), false).is_err());
        bad["blocks"][1]["params"]["sample_time"] = serde_json::json!(0.1);
        bad["blocks"][1]["params"]["initial"] = serde_json::json!([]);
        assert!(normalize_graph_json(&bad.to_string(), true).is_err());
    }
    #[test]
    fn discrete_margin_boundary_serializes_candidates_period_and_strict_open_loop_mode() {
        let p: serde_json::Value =
            serde_json::from_str(include_str!("../../../samples/discrete/margins-plan.json"))
                .unwrap();
        let model = p["model"].to_string();
        let q = p["request"].to_string();
        let r: serde_json::Value =
            serde_json::from_str(&discrete_frequency_json(&model, &q).unwrap()).unwrap();
        assert_eq!(r["request"]["open_loop"], true);
        assert_eq!(r["sample_time"], 0.1);
        assert_eq!(r["margins"]["gain_crossings"].as_array().unwrap().len(), 1);
        assert_eq!(r["margins"]["phase_crossings"].as_array().unwrap().len(), 1);
        assert!(
            (r["margins"]["gain_crossings"][0]["phase_margin_deg"]
                .as_f64()
                .unwrap()
                - 57.9100487437)
                .abs()
                < 1e-8
        );
        let off = q.replace("true", "false");
        let off: serde_json::Value =
            serde_json::from_str(&discrete_frequency_json(&model, &off).unwrap()).unwrap();
        assert!(off.get("margins").is_none());
        assert_eq!(off["points"], r["points"]);
        assert!(discrete_frequency_check_json(&model, &q.replace("true", "null")).is_err());
    }
    #[test]
    fn discrete_frequency_boundary_retains_period_nyquist_points_and_rejects_invalid_flags() {
        let model = include_str!("../../../samples/discrete/feedback.json");
        let q = r#"{"source":"source","source_component":0,"scope":"output","scope_component":0,"min_omega":0.01,"max_omega":30,"points":201}"#;
        let r: serde_json::Value =
            serde_json::from_str(&discrete_frequency_json(model, q).unwrap()).unwrap();
        assert_eq!(r["sample_time"], 0.1);
        assert_eq!(r["states"], 2);
        assert_eq!(r["points"].as_array().unwrap().len(), 201);
        assert!(r.get("margins").is_none());
        assert!((r["nyquist_omega"].as_f64().unwrap() - std::f64::consts::PI / 0.1).abs() < 1e-12);
        assert!(discrete_frequency_check_json(
            model,
            &q.replace("\"max_omega\":30", "\"max_omega\":32")
        )
        .is_err());
        assert!(discrete_frequency_check_json(
            model,
            &q.replace("\"points\":201", "\"points\":201,\"open_loop\":1")
        )
        .is_err());
        assert!(discrete_frequency_json(model, &" ".repeat(1_000_001)).is_err());
        assert!(discrete_frequency_json("{}", q).is_err());
    }
    #[test]
    fn discrete_boundary_has_dimensionless_poles_and_rejects_mixed_and_invalid_queries() {
        let model = include_str!("../../../samples/discrete/feedback.json");
        let q = r#"{"source":"source","source_component":0,"scope":"output","scope_component":0}"#;
        let r: serde_json::Value =
            serde_json::from_str(&discrete_state_space_json(model, q).unwrap()).unwrap();
        assert_eq!(r["a"], serde_json::json!([[0.8, -0.64], [1.0, 0.0]]));
        assert_eq!(r["sample_time"], 0.1);
        assert_eq!(r["stability"]["outside_unit_circle_poles"], 0);
        assert!(r["stability"]["right_half_plane_poles"].is_null());
        assert!(discrete_state_space_check_json(
            include_str!("../../../samples/graphs/mixed-pid.json"),
            q
        )
        .is_err());
        assert!(discrete_state_space_json(
            model,
            &q.replace("\"source_component\":0", "\"source_component\":1")
        )
        .is_err());
        assert!(discrete_state_space_json(model, &" ".repeat(1_000_001)).is_err());
        assert!(discrete_state_space_json("{}", q).is_err());
    }
    #[test]
    fn connected_state_space_boundary_reports_matrices_and_validates_components_and_limits() {
        let model = include_str!("../../../samples/state-space/feedback.json");
        let query =
            r#"{"source":"source","source_component":0,"scope":"output","scope_component":0}"#;
        let r: serde_json::Value =
            serde_json::from_str(&state_space_json(model, query).unwrap()).unwrap();
        assert_eq!(
            r["a"],
            serde_json::json!([[-3.0, -2.0, -4.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]])
        );
        assert_eq!(r["b"], serde_json::json!([4.0, 0.0, 0.0]));
        assert_eq!(r["check"]["states"], 3);
        assert_eq!(r["stability"]["classification"], "stable");
        assert_eq!(r["matrix_residuals"].as_array().unwrap().len(), 3);
        assert!(state_space_check_json(
            model,
            &query.replace("\"scope_component\":0", "\"scope_component\":1")
        )
        .is_err());
        assert!(state_space_json(model, &" ".repeat(1_000_001)).is_err());
        assert!(state_space_json("{}", query).is_err());
    }
    #[test]
    fn pole_zero_boundary_keeps_uncancelled_poles_and_refuses_invalid_queries() {
        let model = include_str!("../../../samples/frequency/open-loop.json");
        let query = r#"{"block":"plant","mode":"unit_negative_feedback"}"#;
        let r: serde_json::Value =
            serde_json::from_str(&pole_zero_json(model, query).unwrap()).unwrap();
        assert_eq!(r["stability"]["classification"], "stable");
        assert_eq!(r["poles"]["roots"].as_array().unwrap().len(), 3);
        assert_eq!(r["zeros"]["degree"], 0);
        assert!(pole_zero_check_json(model, r#"{"block":"plant","mode":"graph"}"#).is_err());
        assert!(pole_zero_json(model, &" ".repeat(1_000_001)).is_err());
        let mut zero: serde_json::Value = serde_json::from_str(model).unwrap();
        zero["blocks"][1]["params"]["numerator"] = serde_json::json!([0]);
        let r: serde_json::Value =
            serde_json::from_str(&pole_zero_json(&zero.to_string(), query).unwrap()).unwrap();
        assert_eq!(r["zeros"]["status"], "identically_zero");
        assert!(r["zeros"]["degree"].is_null());
        assert_eq!(r["stability"]["classification"], "boundary_or_uncertain");
        zero["blocks"][1]["params"]["numerator"] = serde_json::json!([-1, 0, 0, 0]);
        assert!(pole_zero_check_json(&zero.to_string(), query)
            .unwrap_err()
            .contains("direct gain -1"));
    }
    #[test]
    fn explicit_open_loop_margin_boundary_preserves_default_and_rejects_invalid_flags() {
        let p: serde_json::Value = serde_json::from_str(include_str!(
            "../../../samples/frequency/open-loop-plan.json"
        ))
        .unwrap();
        let model = p["model"].to_string();
        let result: serde_json::Value =
            serde_json::from_str(&frequency_json(&model, &p["request"].to_string()).unwrap())
                .unwrap();
        let out = &result["margins"];
        assert_eq!(out["gain_crossings"].as_array().unwrap().len(), 1);
        assert_eq!(out["phase_crossings"].as_array().unwrap().len(), 1);
        assert!((out["phase_crossings"][0]["gain_margin"].as_f64().unwrap() - 1.5).abs() < 1e-9);
        let mut q = p["request"].clone();
        q.as_object_mut().unwrap().remove("open_loop");
        let plain: serde_json::Value =
            serde_json::from_str(&frequency_json(&model, &q.to_string()).unwrap()).unwrap();
        assert_eq!(plain["points"], result["points"]);
        assert!(plain.get("margins").is_none());
        for invalid in [
            serde_json::json!(1),
            serde_json::json!("true"),
            serde_json::Value::Null,
        ] {
            q["open_loop"] = invalid;
            assert!(frequency_check_json(&model, &q.to_string()).is_err());
            assert!(frequency_json(&model, &q.to_string()).is_err());
        }
    }
    #[test]
    fn frequency_boundary_checks_linear_models_and_reports_zero_singular_and_finite_points() {
        let model = include_str!("../../../samples/frequency/lowpass.json");
        let request = r#"{"source":"source","source_component":0,"scope":"output","scope_component":0,"min_omega":1,"max_omega":2,"points":2}"#;
        let check: serde_json::Value =
            serde_json::from_str(&frequency_check_json(model, request).unwrap()).unwrap();
        assert_eq!(check["variables"], 3);
        let result: serde_json::Value =
            serde_json::from_str(&frequency_json(model, request).unwrap()).unwrap();
        assert!((result["points"][0]["imag"].as_f64().unwrap() + 0.5).abs() < 1e-12);
        assert!((result["points"][0]["phase_deg"].as_f64().unwrap() + 90.0).abs() < 1e-12);
        assert!(frequency_json("{}", request).is_err());
        assert!(frequency_json(model, "{}").is_err());
        assert!(
            frequency_check_json(model, &request.replace("\"points\":2", "\"points\":1")).is_err()
        );
        let mut singular: serde_json::Value = serde_json::from_str(model).unwrap();
        singular["blocks"][1]["params"]["denominator"] = serde_json::json!([1, 0, 1]);
        let result: serde_json::Value =
            serde_json::from_str(&frequency_json(&singular.to_string(), request).unwrap()).unwrap();
        assert!(result["points"][0]["error"].is_string());
        assert!(result["points"][0]["magnitude_db"].is_null());
        singular["blocks"][1]["params"]["numerator"] = serde_json::json!([0]);
        let result: serde_json::Value = serde_json::from_str(
            &frequency_json(
                &singular.to_string(),
                &request.replace("\"min_omega\":1", "\"min_omega\":1.5"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(result["points"][0]["magnitude"], 0.0);
        assert!(result["points"][0]["phase_deg"].is_null());
    }
    #[test]
    fn batch_boundary_preserves_adaptive_result_and_rejects_insufficient_allowance() {
        let json = include_str!("../../../samples/numerics/first-order.json");
        let original = graph_json(json).unwrap();
        let result: serde_json::Value = serde_json::from_str(&original).unwrap();
        let stats = &result["statistics"];
        let cost = (stats["rate_evaluations"].as_u64().unwrap()
            + stats["output_evaluations"].as_u64().unwrap()) as usize
            * 3;
        assert_eq!(graph_budget_json(json, cost).unwrap(), original);
        assert!(graph_budget_json(json, cost - 1).is_err());
        assert!(graph_budget_json("{}", 20_000_000).is_err());
    }
    #[test]
    fn graph_diagnostics_boundary_returns_structured_reports_and_rejects_bad_json() {
        let json = include_str!("../../../samples/graphs/subsystem.json");
        let valid: serde_json::Value =
            serde_json::from_str(&diagnose_graph_json(json).unwrap()).unwrap();
        assert_eq!(valid["runnable"], true);
        let mut graph: serde_json::Value = serde_json::from_str(json).unwrap();
        graph["edges"] = serde_json::json!([]);
        let report: serde_json::Value =
            serde_json::from_str(&diagnose_graph_json(&graph.to_string()).unwrap()).unwrap();
        assert_eq!(report["runnable"], false);
        assert_eq!(report["diagnostics"][0]["code"], "unconnected_input");
        assert_eq!(report["diagnostics"][0]["block_id"], "amplifier/in");
        assert_eq!(report["diagnostics"][0]["port"], 0);
        assert!(diagnose_graph_json("{}").is_err());
        assert!(diagnose_graph_json(&" ".repeat(MAX_JSON_BYTES + 1)).is_err());
    }
    #[test]
    fn graph_boundary_executes_vectors_and_draft_validation() {
        let json = include_str!("../../../samples/graphs/vector.json");
        let run: serde_json::Value = serde_json::from_str(&graph_json(json).unwrap()).unwrap();
        assert_eq!(
            run["traces"][0]["values"][0],
            serde_json::json!([-1.0, 1.0, 1.0])
        );
        let normalized = normalize_graph_json(json, true).unwrap();
        assert_eq!(
            normalize_graph_json(&normalized, false).unwrap(),
            normalized
        );
        assert!(graph_json("{}").is_err());
    }
    #[test]
    fn boundary_runs_and_roundtrips() {
        let json = default_model();
        assert_eq!(validate_json(&json).unwrap(), json);
        let result: serde_json::Value = serde_json::from_str(&run_json(&json).unwrap()).unwrap();
        assert_eq!(result["samples"].as_array().unwrap().len(), 4001);
        let comparison: serde_json::Value =
            serde_json::from_str(&compare_json(&json).unwrap()).unwrap();
        assert!(
            comparison["enabled"]["metrics"]["recovery_iae"]
                .as_f64()
                .unwrap()
                < comparison["disabled"]["metrics"]["recovery_iae"]
                    .as_f64()
                    .unwrap()
        );
    }
    #[test]
    fn boundary_rejects_invalid_data() {
        assert!(run_json("{}").is_err());
        assert!(validate_json("null").is_err());
        assert!(saturation_json("[1, null]", "{}").is_err());
        assert!(saturation_json("[1]", r#"{"lower":2,"upper":1}"#).is_err());
        assert!(run_json(&" ".repeat(MAX_JSON_BYTES + 1)).is_err());
    }
    #[test]
    fn boundary_vector_saturation() {
        assert_eq!(
            saturation_json("[-2,0,2]", r#"{"lower":-1,"upper":1}"#).unwrap(),
            "[-1.0,0.0,1.0]"
        );
    }
}
