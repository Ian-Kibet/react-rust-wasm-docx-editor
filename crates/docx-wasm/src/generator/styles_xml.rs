use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::model::{Document, ParagraphProperties, RunProperties, StyleDef};
use super::GenerateError;

const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// Generate `word/styles.xml` from the document's style definitions.
/// Always emits a minimal `Normal` base style even when no custom styles are
/// defined so the document remains valid.
pub fn generate_styles_xml(doc: &Document) -> Result<Vec<u8>, GenerateError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let mut root = BytesStart::new("w:styles");
    root.push_attribute(("xmlns:w", W_NS));
    root.push_attribute((
        "xmlns:mc",
        "http://schemas.openxmlformats.org/markup-compatibility/2006",
    ));
    writer.write_event(Event::Start(root))?;

    // Default Normal style (always present)
    write_default_normal_style(&mut writer)?;

    // Heading styles 1-9
    write_default_heading_styles(&mut writer)?;

    // User-defined styles from the document model
    for style in &doc.styles {
        write_style(&mut writer, style)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:styles")))?;

    Ok(writer.into_inner().into_inner())
}

fn write_default_normal_style(
    writer: &mut Writer<Cursor<Vec<u8>>>,
) -> Result<(), GenerateError> {
    let mut style_el = BytesStart::new("w:style");
    style_el.push_attribute(("w:type", "paragraph"));
    style_el.push_attribute(("w:default", "1"));
    style_el.push_attribute(("w:styleId", "Normal"));
    writer.write_event(Event::Start(style_el))?;

    let mut name_el = BytesStart::new("w:name");
    name_el.push_attribute(("w:val", "Normal"));
    writer.write_event(Event::Empty(name_el))?;

    // qFormat
    writer.write_event(Event::Empty(BytesStart::new("w:qFormat")))?;

    writer.write_event(Event::End(BytesEnd::new("w:style")))?;
    Ok(())
}

fn write_default_heading_styles(
    writer: &mut Writer<Cursor<Vec<u8>>>,
) -> Result<(), GenerateError> {
    for level in 1..=9u8 {
        let style_id = format!("Heading{level}");
        let name = format!("heading {level}");

        let mut style_el = BytesStart::new("w:style");
        style_el.push_attribute(("w:type", "paragraph"));
        style_el.push_attribute(("w:styleId", style_id.as_str()));
        writer.write_event(Event::Start(style_el))?;

        let mut name_el = BytesStart::new("w:name");
        name_el.push_attribute(("w:val", name.as_str()));
        writer.write_event(Event::Empty(name_el))?;

        let mut based_on = BytesStart::new("w:basedOn");
        based_on.push_attribute(("w:val", "Normal"));
        writer.write_event(Event::Empty(based_on))?;

        writer.write_event(Event::Empty(BytesStart::new("w:qFormat")))?;

        // Outline level
        writer.write_event(Event::Start(BytesStart::new("w:pPr")))?;
        let mut outline = BytesStart::new("w:outlineLvl");
        outline.push_attribute(("w:val", (level - 1).to_string().as_str()));
        writer.write_event(Event::Empty(outline))?;
        writer.write_event(Event::End(BytesEnd::new("w:pPr")))?;

        // Run properties with scaled font size
        let font_size = match level {
            1 => 32,
            2 => 26,
            3 => 24,
            4 => 22,
            _ => 20,
        };
        writer.write_event(Event::Start(BytesStart::new("w:rPr")))?;
        writer.write_event(Event::Empty(BytesStart::new("w:b")))?;
        let mut sz = BytesStart::new("w:sz");
        sz.push_attribute(("w:val", (font_size * 2).to_string().as_str()));
        writer.write_event(Event::Empty(sz))?;
        let mut sz_cs = BytesStart::new("w:szCs");
        sz_cs.push_attribute(("w:val", (font_size * 2).to_string().as_str()));
        writer.write_event(Event::Empty(sz_cs))?;
        writer.write_event(Event::End(BytesEnd::new("w:rPr")))?;

        writer.write_event(Event::End(BytesEnd::new("w:style")))?;
    }
    Ok(())
}

fn write_style(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    style: &StyleDef,
) -> Result<(), GenerateError> {
    let mut style_el = BytesStart::new("w:style");
    style_el.push_attribute(("w:type", style.style_type.as_str()));
    style_el.push_attribute(("w:styleId", style.id.as_str()));
    writer.write_event(Event::Start(style_el))?;

    let mut name_el = BytesStart::new("w:name");
    name_el.push_attribute(("w:val", style.name.as_str()));
    writer.write_event(Event::Empty(name_el))?;

    if let Some(based_on) = &style.based_on {
        let mut bo = BytesStart::new("w:basedOn");
        bo.push_attribute(("w:val", based_on.as_str()));
        writer.write_event(Event::Empty(bo))?;
    }

    writer.write_event(Event::Empty(BytesStart::new("w:qFormat")))?;

    if let Some(ppr) = &style.paragraph_properties {
        write_paragraph_properties(writer, ppr)?;
    }

    if let Some(rpr) = &style.run_properties {
        write_run_properties_element(writer, rpr)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:style")))?;
    Ok(())
}

/// Write `<w:pPr>` for a style definition.
pub fn write_paragraph_properties(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    props: &ParagraphProperties,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:pPr")))?;

    if let Some(alignment) = &props.alignment {
        let val = super::document_xml::alignment_value(alignment);
        let mut jc = BytesStart::new("w:jc");
        jc.push_attribute(("w:val", val));
        writer.write_event(Event::Empty(jc))?;
    }

    // Spacing
    if props.spacing_before.is_some() || props.spacing_after.is_some() {
        let mut spacing = BytesStart::new("w:spacing");
        if let Some(before) = props.spacing_before {
            spacing.push_attribute(("w:before", (before as u32).to_string().as_str()));
        }
        if let Some(after) = props.spacing_after {
            spacing.push_attribute(("w:after", (after as u32).to_string().as_str()));
        }
        writer.write_event(Event::Empty(spacing))?;
    }

    // Indentation
    if props.indent_left.is_some()
        || props.indent_right.is_some()
        || props.indent_first_line.is_some()
    {
        let mut ind = BytesStart::new("w:ind");
        if let Some(left) = props.indent_left {
            ind.push_attribute(("w:left", (left as i32).to_string().as_str()));
        }
        if let Some(right) = props.indent_right {
            ind.push_attribute(("w:right", (right as i32).to_string().as_str()));
        }
        if let Some(first) = props.indent_first_line {
            ind.push_attribute(("w:firstLine", (first as i32).to_string().as_str()));
        }
        writer.write_event(Event::Empty(ind))?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:pPr")))?;
    Ok(())
}

/// Write `<w:rPr>` for a style definition.
pub fn write_run_properties_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    props: &RunProperties,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:rPr")))?;

    if props.bold == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:b")))?;
    }
    if props.italic == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:i")))?;
    }
    if props.underline == Some(true) {
        let mut u = BytesStart::new("w:u");
        u.push_attribute(("w:val", "single"));
        writer.write_event(Event::Empty(u))?;
    }
    if props.strikethrough == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:strike")))?;
    }
    if let Some(font) = &props.font_family {
        let mut f = BytesStart::new("w:rFonts");
        f.push_attribute(("w:ascii", font.as_str()));
        f.push_attribute(("w:hAnsi", font.as_str()));
        writer.write_event(Event::Empty(f))?;
    }
    if let Some(size) = props.font_size {
        // OOXML font size is in half-points
        let half_points = (size * 2.0) as u32;
        let val_str = half_points.to_string();
        let mut sz = BytesStart::new("w:sz");
        sz.push_attribute(("w:val", val_str.as_str()));
        writer.write_event(Event::Empty(sz))?;
        let mut sz_cs = BytesStart::new("w:szCs");
        sz_cs.push_attribute(("w:val", val_str.as_str()));
        writer.write_event(Event::Empty(sz_cs))?;
    }
    if let Some(color) = &props.color {
        let c = color.trim_start_matches('#');
        let mut col = BytesStart::new("w:color");
        col.push_attribute(("w:val", c));
        writer.write_event(Event::Empty(col))?;
    }
    if let Some(highlight) = &props.highlight {
        let mut hl = BytesStart::new("w:highlight");
        hl.push_attribute(("w:val", highlight.as_str()));
        writer.write_event(Event::Empty(hl))?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:rPr")))?;
    Ok(())
}
