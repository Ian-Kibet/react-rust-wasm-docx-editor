pub mod document_xml;
pub mod header_footer_xml;
pub mod media;
pub mod numbering_xml;
pub mod rels_xml;
pub mod styles_xml;
pub mod zip_reader;

use std::collections::HashMap;

use crate::model::Document;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid document: {0}")]
    Invalid(String),
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a `.docx` file (raw bytes of the ZIP archive) into a `Document`.
///
/// This is the main entry point for the parser module.  It orchestrates:
///
/// 1. ZIP extraction
/// 2. Relationship parsing
/// 3. Media / image extraction (base64 encoded)
/// 4. Styles and numbering definitions
/// 5. Main document body (paragraphs, tables, images)
/// 6. Headers and footers
pub fn parse(data: &[u8]) -> Result<Document, ParseError> {
    // 1. Extract all files from the ZIP archive
    let files = zip_reader::read_zip(data)?;

    // 2. Parse OPC relationships (package-level + word-level)
    let rels = rels_xml::parse_all_rels(&files)?;

    // 3. Extract and base64-encode media (images)
    let (images, media_path_to_id) = media::extract_images(&files);

    // 4. Parse styles (optional -- may not be present)
    let styles = match files.get("word/styles.xml") {
        Some(data) => {
            let xml = String::from_utf8_lossy(data);
            styles_xml::parse_styles(&xml)?
        }
        None => Vec::new(),
    };

    // 5. Parse numbering / lists (optional)
    let numbering = match files.get("word/numbering.xml") {
        Some(data) => {
            let xml = String::from_utf8_lossy(data);
            numbering_xml::parse_numbering(&xml)?
        }
        None => Vec::new(),
    };

    // 6. Parse the main document body
    let body = match files.get("word/document.xml") {
        Some(data) => {
            let xml = String::from_utf8_lossy(data);
            document_xml::parse_document(&xml, &rels, &media_path_to_id)?
        }
        None => {
            return Err(ParseError::Invalid(
                "Missing word/document.xml".to_string(),
            ));
        }
    };

    // 7. Parse headers and footers
    let (headers, footers) =
        parse_headers_and_footers(&files, &rels, &media_path_to_id)?;

    Ok(Document {
        body,
        styles,
        numbering,
        images,
        headers,
        footers,
    })
}

// ---------------------------------------------------------------------------
// Header / footer discovery
// ---------------------------------------------------------------------------

/// Discover header and footer XML files via the relationships map and parse
/// them.  Headers have relationship type ending in `/header`, footers end in
/// `/footer`.  We also detect them by filename pattern (`word/header*.xml`,
/// `word/footer*.xml`).
fn parse_headers_and_footers(
    files: &HashMap<String, Vec<u8>>,
    rels: &rels_xml::RelsMap,
    media_path_to_id: &HashMap<String, String>,
) -> Result<(Vec<crate::model::Header>, Vec<crate::model::Footer>), ParseError> {
    let mut headers = Vec::new();
    let mut footers = Vec::new();

    // Collect header/footer paths from rels targets
    let mut header_paths: Vec<String> = Vec::new();
    let mut footer_paths: Vec<String> = Vec::new();

    for target in rels.values() {
        let lower = target.to_lowercase();
        if lower.contains("header") && lower.ends_with(".xml") {
            if !header_paths.contains(target) {
                header_paths.push(target.clone());
            }
        } else if lower.contains("footer") && lower.ends_with(".xml") {
            if !footer_paths.contains(target) {
                footer_paths.push(target.clone());
            }
        }
    }

    // Also scan the file map directly for header/footer files that might not
    // appear in rels (unusual but defensive)
    for path in files.keys() {
        let lower = path.to_lowercase();
        if lower.starts_with("word/header") && lower.ends_with(".xml") {
            if !header_paths.contains(path) {
                header_paths.push(path.clone());
            }
        } else if lower.starts_with("word/footer") && lower.ends_with(".xml") {
            if !footer_paths.contains(path) {
                footer_paths.push(path.clone());
            }
        }
    }

    // Sort for deterministic ordering
    header_paths.sort();
    footer_paths.sort();

    // Parse each header
    for path in &header_paths {
        if let Some(data) = files.get(path.as_str()) {
            let xml = String::from_utf8_lossy(data);
            let header =
                header_footer_xml::parse_header(&xml, rels, media_path_to_id)?;
            headers.push(header);
        }
    }

    // Parse each footer
    for path in &footer_paths {
        if let Some(data) = files.get(path.as_str()) {
            let xml = String::from_utf8_lossy(data);
            let footer =
                header_footer_xml::parse_footer(&xml, rels, media_path_to_id)?;
            footers.push(footer);
        }
    }

    Ok((headers, footers))
}
