use std::collections::HashMap;
use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::model::{Endnote, Footnote};
use super::document_xml::{HyperlinkRidMap, ImageRidMap};
use super::GenerateError;

const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// Generate `word/footnotes.xml` from a list of footnotes.
///
/// Each footnote contains an id and block-level content. The generated XML
/// includes the standard separator and continuation-separator footnotes
/// (ids -1 and 0) followed by the user-defined footnotes.
pub fn generate_footnotes_xml(footnotes: &[Footnote]) -> Result<Vec<u8>, GenerateError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let mut root = BytesStart::new("w:footnotes");
    root.push_attribute(("xmlns:w", W_NS));
    root.push_attribute(("xmlns:r", R_NS));
    writer.write_event(Event::Start(root))?;

    // Standard separator footnote (id=-1)
    write_separator_note(&mut writer, "w:footnote", "-1", "separator")?;

    // Standard continuation-separator footnote (id=0)
    write_separator_note(&mut writer, "w:footnote", "0", "continuationSeparator")?;

    // Empty maps for block element writing
    let image_rid_map: ImageRidMap = HashMap::new();
    let hyperlink_rid_map: HyperlinkRidMap = HashMap::new();

    for footnote in footnotes {
        write_note(&mut writer, "w:footnote", &footnote.id, &footnote.content, &image_rid_map, &hyperlink_rid_map)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:footnotes")))?;

    Ok(writer.into_inner().into_inner())
}

/// Generate `word/endnotes.xml` from a list of endnotes.
///
/// Each endnote contains an id and block-level content. The generated XML
/// includes the standard separator and continuation-separator endnotes
/// (ids -1 and 0) followed by the user-defined endnotes.
pub fn generate_endnotes_xml(endnotes: &[Endnote]) -> Result<Vec<u8>, GenerateError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let mut root = BytesStart::new("w:endnotes");
    root.push_attribute(("xmlns:w", W_NS));
    root.push_attribute(("xmlns:r", R_NS));
    writer.write_event(Event::Start(root))?;

    // Standard separator endnote (id=-1)
    write_separator_note(&mut writer, "w:endnote", "-1", "separator")?;

    // Standard continuation-separator endnote (id=0)
    write_separator_note(&mut writer, "w:endnote", "0", "continuationSeparator")?;

    // Empty maps for block element writing
    let image_rid_map: ImageRidMap = HashMap::new();
    let hyperlink_rid_map: HyperlinkRidMap = HashMap::new();

    for endnote in endnotes {
        write_note(&mut writer, "w:endnote", &endnote.id, &endnote.content, &image_rid_map, &hyperlink_rid_map)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:endnotes")))?;

    Ok(writer.into_inner().into_inner())
}

/// Write a separator or continuation-separator note (footnote or endnote).
/// These are required by the OOXML spec with special type attributes.
fn write_separator_note(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    element_name: &str,
    id: &str,
    note_type: &str,
) -> Result<(), GenerateError> {
    let mut note = BytesStart::new(element_name);
    note.push_attribute(("w:type", note_type));
    note.push_attribute(("w:id", id));
    writer.write_event(Event::Start(note))?;

    // Separator notes contain a single paragraph with a separator run
    writer.write_event(Event::Start(BytesStart::new("w:p")))?;
    writer.write_event(Event::Start(BytesStart::new("w:r")))?;

    let sep = BytesStart::new(if note_type == "separator" {
        "w:separator"
    } else {
        "w:continuationSeparator"
    });
    writer.write_event(Event::Empty(sep))?;

    writer.write_event(Event::End(BytesEnd::new("w:r")))?;
    writer.write_event(Event::End(BytesEnd::new("w:p")))?;

    writer.write_event(Event::End(BytesEnd::new(element_name)))?;
    Ok(())
}

/// Write a user-defined footnote or endnote element.
fn write_note(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    element_name: &str,
    id: &str,
    content: &[crate::model::BlockElement],
    image_rid_map: &ImageRidMap,
    hyperlink_rid_map: &HyperlinkRidMap,
) -> Result<(), GenerateError> {
    let mut note = BytesStart::new(element_name);
    note.push_attribute(("w:id", id));
    writer.write_event(Event::Start(note))?;

    if content.is_empty() {
        // A note must contain at least one paragraph
        writer.write_event(Event::Start(BytesStart::new("w:p")))?;
        writer.write_event(Event::End(BytesEnd::new("w:p")))?;
    } else {
        for block in content {
            super::document_xml::write_block_element(writer, block, image_rid_map, hyperlink_rid_map)?;
        }
    }

    writer.write_event(Event::End(BytesEnd::new(element_name)))?;
    Ok(())
}
