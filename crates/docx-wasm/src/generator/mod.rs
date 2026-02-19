pub mod content_types;
pub mod document_xml;
pub mod header_footer_xml;
pub mod media;
pub mod numbering_xml;
pub mod rels_xml;
pub mod styles_xml;
pub mod zip_writer;

use std::collections::HashMap;

use crate::model::Document;

/// Errors that can occur during DOCX generation.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("Generation error: {0}")]
    Other(String),
}

/// Generate a complete `.docx` file (ZIP archive containing OOXML parts)
/// from the in-memory `Document` model.
///
/// Returns the raw bytes of the ZIP archive suitable for saving to disk or
/// sending as a download.
pub fn generate(doc: &Document) -> Result<Vec<u8>, GenerateError> {
    // ------------------------------------------------------------------
    // 1. Build the relationship map so every part knows its rId
    // ------------------------------------------------------------------
    let (doc_rels, _next_rid) = rels_xml::build_document_rels(doc, 1);

    // Build an image-id -> rId lookup for the document XML writer
    let mut image_rid_map: HashMap<String, String> = HashMap::new();
    for rel in &doc_rels {
        if rel.rel_type.ends_with("/image") {
            // Target looks like "media/{id}.{ext}" -- extract the image id
            if let Some(filename) = rel.target.strip_prefix("media/") {
                if let Some(dot) = filename.rfind('.') {
                    let img_id = &filename[..dot];
                    image_rid_map.insert(img_id.to_string(), rel.id.clone());
                }
            }
        }
    }

    // Collect header and footer rIds for sectPr references
    let header_rids: Vec<String> = doc_rels
        .iter()
        .filter(|r| r.rel_type.ends_with("/header"))
        .map(|r| r.id.clone())
        .collect();

    let footer_rids: Vec<String> = doc_rels
        .iter()
        .filter(|r| r.rel_type.ends_with("/footer"))
        .map(|r| r.id.clone())
        .collect();

    // ------------------------------------------------------------------
    // 2. Generate all XML parts
    // ------------------------------------------------------------------
    let content_types_xml = content_types::generate_content_types(doc)?;
    let package_rels_xml = rels_xml::generate_package_rels()?;
    let document_rels_xml = rels_xml::generate_document_rels(&doc_rels)?;

    let document_xml =
        document_xml::generate_document_xml(doc, &image_rid_map, &header_rids, &footer_rids)?;
    let styles_xml = styles_xml::generate_styles_xml(doc)?;

    let numbering_xml = if !doc.numbering.is_empty() {
        Some(numbering_xml::generate_numbering_xml(doc)?)
    } else {
        None
    };

    // Headers
    let mut header_parts: Vec<(String, Vec<u8>)> = Vec::new();
    for (i, header) in doc.headers.iter().enumerate() {
        let xml = header_footer_xml::generate_header_xml(header, &image_rid_map)?;
        header_parts.push((format!("word/header{}.xml", i + 1), xml));
    }

    // Footers
    let mut footer_parts: Vec<(String, Vec<u8>)> = Vec::new();
    for (i, footer) in doc.footers.iter().enumerate() {
        let xml = header_footer_xml::generate_footer_xml(footer, &image_rid_map)?;
        footer_parts.push((format!("word/footer{}.xml", i + 1), xml));
    }

    // ------------------------------------------------------------------
    // 3. Decode media files
    // ------------------------------------------------------------------
    let media_files = media::decode_images(&doc.images)?;

    // ------------------------------------------------------------------
    // 4. Assemble all parts into a list for the ZIP writer
    // ------------------------------------------------------------------
    let mut parts: Vec<(&str, Vec<u8>)> = Vec::new();

    // Package-level
    parts.push(("[Content_Types].xml", content_types_xml));
    parts.push(("_rels/.rels", package_rels_xml));

    // Word directory
    parts.push(("word/_rels/document.xml.rels", document_rels_xml));
    parts.push(("word/document.xml", document_xml));
    parts.push(("word/styles.xml", styles_xml));

    if let Some(num_xml) = numbering_xml {
        parts.push(("word/numbering.xml", num_xml));
    }

    // Owned string paths for headers/footers -- we need to keep them alive
    // while `parts` borrows them, so collect references after pushing.
    let header_footer_owned: Vec<(String, Vec<u8>)> = header_parts
        .into_iter()
        .chain(footer_parts)
        .collect();

    // We cannot push &str from owned Strings directly into parts because of
    // lifetime issues, so we build the final list differently.
    let final_parts: Vec<(&str, Vec<u8>)> = parts;

    // We need a slightly different approach for owned paths. Let's build a
    // helper vec that owns the strings and we reference them.
    // Actually, the simplest approach is to use the zip_writer with a
    // slightly modified interface. Let's just collect everything as owned.
    let mut owned_parts: Vec<(String, Vec<u8>)> = final_parts
        .into_iter()
        .map(|(path, data)| (path.to_string(), data))
        .collect();

    for (path, data) in header_footer_owned {
        owned_parts.push((path, data));
    }

    // Media
    for mf in media_files {
        owned_parts.push((mf.path, mf.data));
    }

    // ------------------------------------------------------------------
    // 5. Write ZIP archive
    // ------------------------------------------------------------------
    let ref_parts: Vec<(&str, Vec<u8>)> = owned_parts
        .iter()
        .map(|(path, data)| (path.as_str(), data.clone()))
        .collect();

    zip_writer::write_zip(&ref_parts)
}
