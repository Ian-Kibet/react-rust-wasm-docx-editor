pub mod comments_xml;
pub mod content_types;
pub mod document_xml;
pub mod footnotes_xml;
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
    let (doc_rels, _next_rid, hyperlink_rid_map) = rels_xml::build_document_rels(doc, 1);

    // Build an image-id -> rId lookup for the document XML writer
    let mut image_rid_map: HashMap<String, String> = HashMap::new();
    for rel in &doc_rels {
        if rel.rel_type.ends_with("/image") {
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

    let document_xml = document_xml::generate_document_xml(
        doc,
        &image_rid_map,
        &hyperlink_rid_map,
        &header_rids,
        &footer_rids,
    )?;
    let styles_xml = styles_xml::generate_styles_xml(doc)?;

    let numbering_xml = if !doc.numbering.is_empty() {
        Some(numbering_xml::generate_numbering_xml(doc)?)
    } else {
        None
    };

    let comments_xml = if !doc.comments.is_empty() {
        Some(comments_xml::generate_comments_xml(&doc.comments)?)
    } else {
        None
    };

    let footnotes_xml = if !doc.footnotes.is_empty() {
        Some(footnotes_xml::generate_footnotes_xml(&doc.footnotes)?)
    } else {
        None
    };

    let endnotes_xml = if !doc.endnotes.is_empty() {
        Some(footnotes_xml::generate_endnotes_xml(&doc.endnotes)?)
    } else {
        None
    };

    // Headers
    let empty_hyperlink_map: HashMap<String, String> = HashMap::new();
    let mut header_parts: Vec<(String, Vec<u8>)> = Vec::new();
    for (i, header) in doc.headers.iter().enumerate() {
        let xml = header_footer_xml::generate_header_xml(header, &image_rid_map, &empty_hyperlink_map)?;
        header_parts.push((format!("word/header{}.xml", i + 1), xml));
    }

    // Footers
    let mut footer_parts: Vec<(String, Vec<u8>)> = Vec::new();
    for (i, footer) in doc.footers.iter().enumerate() {
        let xml = header_footer_xml::generate_footer_xml(footer, &image_rid_map, &empty_hyperlink_map)?;
        footer_parts.push((format!("word/footer{}.xml", i + 1), xml));
    }

    // ------------------------------------------------------------------
    // 3. Decode media files
    // ------------------------------------------------------------------
    let media_files = media::decode_images(&doc.images)?;

    // ------------------------------------------------------------------
    // 4. Assemble all parts into a list for the ZIP writer
    // ------------------------------------------------------------------
    let mut owned_parts: Vec<(String, Vec<u8>)> = Vec::new();

    owned_parts.push(("[Content_Types].xml".to_string(), content_types_xml));
    owned_parts.push(("_rels/.rels".to_string(), package_rels_xml));
    owned_parts.push(("word/_rels/document.xml.rels".to_string(), document_rels_xml));
    owned_parts.push(("word/document.xml".to_string(), document_xml));
    owned_parts.push(("word/styles.xml".to_string(), styles_xml));

    if let Some(num_xml) = numbering_xml {
        owned_parts.push(("word/numbering.xml".to_string(), num_xml));
    }

    if let Some(comments) = comments_xml {
        owned_parts.push(("word/comments.xml".to_string(), comments));
    }

    if let Some(footnotes) = footnotes_xml {
        owned_parts.push(("word/footnotes.xml".to_string(), footnotes));
    }

    if let Some(endnotes) = endnotes_xml {
        owned_parts.push(("word/endnotes.xml".to_string(), endnotes));
    }

    for (path, data) in header_parts {
        owned_parts.push((path, data));
    }
    for (path, data) in footer_parts {
        owned_parts.push((path, data));
    }

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
