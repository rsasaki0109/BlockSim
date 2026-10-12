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
    use super::*;
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
