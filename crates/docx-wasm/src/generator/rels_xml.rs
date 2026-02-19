use std::collections::HashMap;
use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use super::document_xml::HyperlinkRidMap;
use super::GenerateError;

const RELS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// A single relationship entry.
pub struct Relationship {
    pub id: String,
    pub rel_type: String,
    pub target: String,
    /// When set to `"External"`, indicates the target is outside the package.
    pub target_mode: Option<String>,
}

/// Generate `_rels/.rels` -- the package-level relationship file.
/// Points to `word/document.xml` as the main document part.
pub fn generate_package_rels() -> Result<Vec<u8>, GenerateError> {
    let rels = vec![Relationship {
        id: "rId1".to_string(),
        rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument".to_string(),
        target: "word/document.xml".to_string(),
        target_mode: None,
    }];
    write_rels(&rels)
}

/// Generate `word/_rels/document.xml.rels` containing relationships for
/// styles, numbering, images, headers, footers, hyperlinks, footnotes, and
/// endnotes.
pub fn generate_document_rels(rels: &[Relationship]) -> Result<Vec<u8>, GenerateError> {
    write_rels(rels)
}

/// Build the relationship list for the document part.
///
/// Returns a `(relationships, next_rid, hyperlink_rid_map)` tuple. The caller
/// supplies a starting relationship id counter and the document metadata.
pub fn build_document_rels(
    doc: &crate::model::Document,
    start_rid: u32,
) -> (Vec<Relationship>, u32, HyperlinkRidMap) {
    let mut rels: Vec<Relationship> = Vec::new();
    let mut rid = start_rid;
    let mut hyperlink_rid_map: HyperlinkRidMap = HashMap::new();

    // Styles
    rels.push(Relationship {
        id: format!("rId{rid}"),
        rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles"
            .to_string(),
        target: "styles.xml".to_string(),
        target_mode: None,
    });
    rid += 1;

    // Numbering (only if definitions exist)
    if !doc.numbering.is_empty() {
        rels.push(Relationship {
            id: format!("rId{rid}"),
            rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering"
                .to_string(),
            target: "numbering.xml".to_string(),
            target_mode: None,
        });
        rid += 1;
    }

    // Footnotes (only if footnotes exist)
    if !doc.footnotes.is_empty() {
        rels.push(Relationship {
            id: format!("rId{rid}"),
            rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes"
                .to_string(),
            target: "footnotes.xml".to_string(),
            target_mode: None,
        });
        rid += 1;
    }

    // Endnotes (only if endnotes exist)
    if !doc.endnotes.is_empty() {
        rels.push(Relationship {
            id: format!("rId{rid}"),
            rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/endnotes"
                .to_string(),
            target: "endnotes.xml".to_string(),
            target_mode: None,
        });
        rid += 1;
    }

    // Comments (only if comments exist)
    if !doc.comments.is_empty() {
        rels.push(Relationship {
            id: format!("rId{rid}"),
            rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments"
                .to_string(),
            target: "comments.xml".to_string(),
            target_mode: None,
        });
        rid += 1;
    }

    // Headers
    for (i, _header) in doc.headers.iter().enumerate() {
        rels.push(Relationship {
            id: format!("rId{rid}"),
            rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header"
                .to_string(),
            target: format!("header{}.xml", i + 1),
            target_mode: None,
        });
        rid += 1;
    }

    // Footers
    for (i, _footer) in doc.footers.iter().enumerate() {
        rels.push(Relationship {
            id: format!("rId{rid}"),
            rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer"
                .to_string(),
            target: format!("footer{}.xml", i + 1),
            target_mode: None,
        });
        rid += 1;
    }

    // Images
    for img in &doc.images {
        let ext = crate::generator::content_types::extension_for_content_type_pub(&img.content_type);
        rels.push(Relationship {
            id: format!("rId{rid}"),
            rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                .to_string(),
            target: format!("media/{}.{}", img.id, ext),
            target_mode: None,
        });
        rid += 1;
    }

    // Hyperlinks -- collect unique URLs from the document body
    let hyperlink_urls = crate::generator::document_xml::collect_hyperlink_urls(doc);
    for url in hyperlink_urls {
        let rel_id = format!("rId{rid}");
        hyperlink_rid_map.insert(url.clone(), rel_id.clone());
        rels.push(Relationship {
            id: rel_id,
            rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink"
                .to_string(),
            target: url,
            target_mode: Some("External".to_string()),
        });
        rid += 1;
    }

    (rels, rid, hyperlink_rid_map)
}

fn write_rels(rels: &[Relationship]) -> Result<Vec<u8>, GenerateError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let mut root = BytesStart::new("Relationships");
    root.push_attribute(("xmlns", RELS_NS));
    writer.write_event(Event::Start(root))?;

    for rel in rels {
        let mut elem = BytesStart::new("Relationship");
        elem.push_attribute(("Id", rel.id.as_str()));
        elem.push_attribute(("Type", rel.rel_type.as_str()));
        elem.push_attribute(("Target", rel.target.as_str()));
        if let Some(mode) = &rel.target_mode {
            elem.push_attribute(("TargetMode", mode.as_str()));
        }
        writer.write_event(Event::Empty(elem))?;
    }

    writer.write_event(Event::End(BytesEnd::new("Relationships")))?;

    Ok(writer.into_inner().into_inner())
}
