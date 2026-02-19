use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::model::{Footer, Header};
use super::document_xml::{ImageRidMap, HyperlinkRidMap};
use super::GenerateError;

const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const WP_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const PIC_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/picture";

/// Generate `word/header{n}.xml` for a single header.
pub fn generate_header_xml(
    header: &Header,
    image_rid_map: &ImageRidMap,
    hyperlink_rid_map: &HyperlinkRidMap,
) -> Result<Vec<u8>, GenerateError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let mut root = BytesStart::new("w:hdr");
    root.push_attribute(("xmlns:w", W_NS));
    root.push_attribute(("xmlns:r", R_NS));
    root.push_attribute(("xmlns:wp", WP_NS));
    root.push_attribute(("xmlns:a", A_NS));
    root.push_attribute(("xmlns:pic", PIC_NS));
    writer.write_event(Event::Start(root))?;

    for block in &header.content {
        super::document_xml::write_block_element(&mut writer, block, image_rid_map, hyperlink_rid_map)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:hdr")))?;

    Ok(writer.into_inner().into_inner())
}

/// Generate `word/footer{n}.xml` for a single footer.
pub fn generate_footer_xml(
    footer: &Footer,
    image_rid_map: &ImageRidMap,
    hyperlink_rid_map: &HyperlinkRidMap,
) -> Result<Vec<u8>, GenerateError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let mut root = BytesStart::new("w:ftr");
    root.push_attribute(("xmlns:w", W_NS));
    root.push_attribute(("xmlns:r", R_NS));
    root.push_attribute(("xmlns:wp", WP_NS));
    root.push_attribute(("xmlns:a", A_NS));
    root.push_attribute(("xmlns:pic", PIC_NS));
    writer.write_event(Event::Start(root))?;

    for block in &footer.content {
        super::document_xml::write_block_element(&mut writer, block, image_rid_map, hyperlink_rid_map)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:ftr")))?;

    Ok(writer.into_inner().into_inner())
}
