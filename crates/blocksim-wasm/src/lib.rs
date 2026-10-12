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

#[cfg(test)]
mod tests {
    use super::*;
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
