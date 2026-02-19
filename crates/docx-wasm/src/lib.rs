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

/// Get document statistics (word count, char count, etc.) from document JSON.
#[wasm_bindgen]
pub fn get_document_stats(json: &str) -> Result<String, JsError> {
    let doc: model::Document =
        serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
    let stats = doc.statistics();
    let json = serde_json::to_string(&stats).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(json)
}

/// Extract all text from a document as a plain string.
#[wasm_bindgen]
pub fn extract_text(json: &str) -> Result<String, JsError> {
    let doc: model::Document =
        serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(doc.extract_text())
}

/// Search for text in the document and return positions as JSON.
/// Returns a JSON array of objects: [{block_index, run_index, offset, length, context}]
#[wasm_bindgen]
pub fn search_text(json: &str, query: &str, case_sensitive: bool) -> Result<String, JsError> {
    let doc: model::Document =
        serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
    let results = search_in_document(&doc, query, case_sensitive);
    let json = serde_json::to_string(&results).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(json)
}

/// Create a new empty document as JSON.
#[wasm_bindgen]
pub fn create_empty_document() -> Result<String, JsError> {
    let doc = model::Document::new();
    let json = serde_json::to_string(&doc).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(json)
}

/// Validate a document JSON string and return any issues found.
#[wasm_bindgen]
pub fn validate_document(json: &str) -> Result<String, JsError> {
    let issues = validate_doc_json(json);
    let json = serde_json::to_string(&issues).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(json)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct SearchResult {
    block_index: usize,
    run_index: usize,
    offset: usize,
    length: usize,
    context: String,
}

fn search_in_document(doc: &model::Document, query: &str, case_sensitive: bool) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let query_match = if case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };

    for (block_idx, block) in doc.body.iter().enumerate() {
        if let model::BlockElement::Paragraph(para) = block {
            for (run_idx, run) in para.runs.iter().enumerate() {
                let text_to_search = if case_sensitive {
                    run.text.clone()
                } else {
                    run.text.to_lowercase()
                };

                let mut start = 0;
                while let Some(pos) = text_to_search[start..].find(&query_match) {
                    let actual_pos = start + pos;
                    let context_start = actual_pos.saturating_sub(20);
                    let context_end = std::cmp::min(run.text.len(), actual_pos + query.len() + 20);
                    let context = run.text[context_start..context_end].to_string();

                    results.push(SearchResult {
                        block_index: block_idx,
                        run_index: run_idx,
                        offset: actual_pos,
                        length: query.len(),
                        context,
                    });
                    start = actual_pos + 1;
                }
            }
        }
    }
    results
}

#[derive(serde::Serialize)]
struct ValidationIssue {
    severity: String,
    message: String,
    location: Option<String>,
}

fn validate_doc_json(json: &str) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    match serde_json::from_str::<model::Document>(json) {
        Ok(doc) => {
            if doc.body.is_empty() {
                issues.push(ValidationIssue {
                    severity: "warning".to_string(),
                    message: "Document body is empty".to_string(),
                    location: Some("body".to_string()),
                });
            }

            for (i, block) in doc.body.iter().enumerate() {
                match block {
                    model::BlockElement::Paragraph(p) => {
                        if p.id.is_empty() {
                            issues.push(ValidationIssue {
                                severity: "error".to_string(),
                                message: "Paragraph has empty ID".to_string(),
                                location: Some(format!("body[{}]", i)),
                            });
                        }
                        for (j, run) in p.runs.iter().enumerate() {
                            if run.id.is_empty() {
                                issues.push(ValidationIssue {
                                    severity: "error".to_string(),
                                    message: "Run has empty ID".to_string(),
                                    location: Some(format!("body[{}].runs[{}]", i, j)),
                                });
                            }
                        }
                    }
                    model::BlockElement::Table(t) => {
                        if t.id.is_empty() {
                            issues.push(ValidationIssue {
                                severity: "error".to_string(),
                                message: "Table has empty ID".to_string(),
                                location: Some(format!("body[{}]", i)),
                            });
                        }
                        if t.rows.is_empty() {
                            issues.push(ValidationIssue {
                                severity: "warning".to_string(),
                                message: "Table has no rows".to_string(),
                                location: Some(format!("body[{}]", i)),
                            });
                        }
                    }
                }
            }

            // Check for orphaned image references
            let image_ids: Vec<&str> = doc.images.iter().map(|img| img.id.as_str()).collect();
            for (i, block) in doc.body.iter().enumerate() {
                if let model::BlockElement::Paragraph(p) = block {
                    for (j, run) in p.runs.iter().enumerate() {
                        if let Some(ref img_id) = run.properties.inline_image {
                            if !image_ids.contains(&img_id.as_str()) {
                                issues.push(ValidationIssue {
                                    severity: "warning".to_string(),
                                    message: format!("Run references non-existent image: {}", img_id),
                                    location: Some(format!("body[{}].runs[{}]", i, j)),
                                });
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            issues.push(ValidationIssue {
                severity: "error".to_string(),
                message: format!("Invalid JSON: {}", e),
                location: None,
            });
        }
    }

    issues
}

#[cfg(test)]
mod tests;
