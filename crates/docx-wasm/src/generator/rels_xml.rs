use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use super::GenerateError;

const RELS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// A single relationship entry.
pub struct Relationship {
    pub id: String,
    pub rel_type: String,
    pub target: String,
}

/// Generate `_rels/.rels` -- the package-level relationship file.
/// Points to `word/document.xml` as the main document part.
pub fn generate_package_rels() -> Result<Vec<u8>, GenerateError> {
    let rels = vec![Relationship {
        id: "rId1".to_string(),
        rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument".to_string(),
        target: "word/document.xml".to_string(),
    }];
    write_rels(&rels)
}

/// Generate `word/_rels/document.xml.rels` containing relationships for
/// styles, numbering, images, headers, and footers.
pub fn generate_document_rels(rels: &[Relationship]) -> Result<Vec<u8>, GenerateError> {
    write_rels(rels)
}

/// Build the relationship list for the document part.
///
/// Returns a `(relationships, next_rid)` tuple. The caller supplies a
/// starting relationship id counter and the document metadata.
pub fn build_document_rels(
    doc: &crate::model::Document,
    start_rid: u32,
) -> (Vec<Relationship>, u32) {
    let mut rels: Vec<Relationship> = Vec::new();
    let mut rid = start_rid;

    // Styles
    rels.push(Relationship {
        id: format!("rId{rid}"),
        rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles"
            .to_string(),
        target: "styles.xml".to_string(),
    });
    rid += 1;

    // Numbering (only if definitions exist)
    if !doc.numbering.is_empty() {
        rels.push(Relationship {
            id: format!("rId{rid}"),
            rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering"
                .to_string(),
            target: "numbering.xml".to_string(),
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
        });
        rid += 1;
    }

    (rels, rid)
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
        writer.write_event(Event::Empty(elem))?;
    }

    writer.write_event(Event::End(BytesEnd::new("Relationships")))?;

    Ok(writer.into_inner().into_inner())
}
