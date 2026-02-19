pub mod model;
pub mod parser;
pub mod generator;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_docx(data: &[u8]) -> Result<String, JsError> {
    let doc = parser::parse(data).map_err(|e| JsError::new(&e.to_string()))?;
    let json = serde_json::to_string(&doc).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(json)
}

#[wasm_bindgen]
pub fn generate_docx(json: &str) -> Result<js_sys::Uint8Array, JsError> {
    let doc: model::Document =
        serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
    let bytes = generator::generate(&doc).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(js_sys::Uint8Array::from(&bytes[..]))
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
