use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::model::{Document, ListDef, ListLevel};
use super::GenerateError;

const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// Generate `word/numbering.xml` from the document's list definitions.
/// Each `ListDef` produces a paired `<w:abstractNum>` and `<w:num>` entry.
pub fn generate_numbering_xml(doc: &Document) -> Result<Vec<u8>, GenerateError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let mut root = BytesStart::new("w:numbering");
    root.push_attribute(("xmlns:w", W_NS));
    writer.write_event(Event::Start(root))?;

    // Write abstractNum definitions
    for (idx, list_def) in doc.numbering.iter().enumerate() {
        write_abstract_num(&mut writer, idx, list_def)?;
    }

    // Write num elements that reference the abstract definitions
    for (idx, list_def) in doc.numbering.iter().enumerate() {
        write_num(&mut writer, &list_def.num_id, idx)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:numbering")))?;

    Ok(writer.into_inner().into_inner())
}

fn write_abstract_num(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    abstract_id: usize,
    list_def: &ListDef,
) -> Result<(), GenerateError> {
    let id_str = abstract_id.to_string();

    let mut an = BytesStart::new("w:abstractNum");
    an.push_attribute(("w:abstractNumId", id_str.as_str()));
    writer.write_event(Event::Start(an))?;

    // Multi-level type
    let mut multi = BytesStart::new("w:multiLevelType");
    multi.push_attribute(("w:val", "hybridMultilevel"));
    writer.write_event(Event::Empty(multi))?;

    for level in &list_def.levels {
        write_level(writer, level)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:abstractNum")))?;
    Ok(())
}

fn write_level(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    level: &ListLevel,
) -> Result<(), GenerateError> {
    let lvl_str = level.level.to_string();

    let mut lvl = BytesStart::new("w:lvl");
    lvl.push_attribute(("w:ilvl", lvl_str.as_str()));
    writer.write_event(Event::Start(lvl))?;

    // Start value
    let mut start = BytesStart::new("w:start");
    start.push_attribute(("w:val", level.start.to_string().as_str()));
    writer.write_event(Event::Empty(start))?;

    // Number format
    let fmt = ooxml_num_format(&level.format);
    let mut num_fmt = BytesStart::new("w:numFmt");
    num_fmt.push_attribute(("w:val", fmt));
    writer.write_event(Event::Empty(num_fmt))?;

    // Level text (e.g., "%1.", "%1.%2.")
    let mut lvl_text = BytesStart::new("w:lvlText");
    lvl_text.push_attribute(("w:val", level.text.as_str()));
    writer.write_event(Event::Empty(lvl_text))?;

    // Justification
    let mut jc = BytesStart::new("w:lvlJc");
    jc.push_attribute(("w:val", "left"));
    writer.write_event(Event::Empty(jc))?;

    // Paragraph properties with hanging indent based on level
    let indent = (level.level + 1) * 720;
    let indent_str = indent.to_string();

    writer.write_event(Event::Start(BytesStart::new("w:pPr")))?;
    let mut ind = BytesStart::new("w:ind");
    ind.push_attribute(("w:left", indent_str.as_str()));
    ind.push_attribute(("w:hanging", "360"));
    writer.write_event(Event::Empty(ind))?;
    writer.write_event(Event::End(BytesEnd::new("w:pPr")))?;

    writer.write_event(Event::End(BytesEnd::new("w:lvl")))?;
    Ok(())
}

fn write_num(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    num_id: &str,
    abstract_id: usize,
) -> Result<(), GenerateError> {
    let mut num = BytesStart::new("w:num");
    num.push_attribute(("w:numId", num_id));
    writer.write_event(Event::Start(num))?;

    let mut abs_ref = BytesStart::new("w:abstractNumId");
    abs_ref.push_attribute(("w:val", abstract_id.to_string().as_str()));
    writer.write_event(Event::Empty(abs_ref))?;

    writer.write_event(Event::End(BytesEnd::new("w:num")))?;
    Ok(())
}

/// Map user-facing format names to OOXML numFmt values.
fn ooxml_num_format(fmt: &str) -> &str {
    match fmt {
        "decimal" | "number" => "decimal",
        "bullet" => "bullet",
        "lowerLetter" | "lower-alpha" => "lowerLetter",
        "upperLetter" | "upper-alpha" => "upperLetter",
        "lowerRoman" | "lower-roman" => "lowerRoman",
        "upperRoman" | "upper-roman" => "upperRoman",
        _ => fmt,
    }
}
