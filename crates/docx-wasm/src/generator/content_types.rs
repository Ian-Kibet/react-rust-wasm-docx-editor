use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::model::Document;
use super::GenerateError;

/// Generate `[Content_Types].xml` declaring MIME types for every part in the
/// package.
pub fn generate_content_types(doc: &Document) -> Result<Vec<u8>, GenerateError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let mut types = BytesStart::new("Types");
    types.push_attribute((
        "xmlns",
        "http://schemas.openxmlformats.org/package/2006/content-types",
    ));
    writer.write_event(Event::Start(types))?;

    // Default extensions
    write_default(&mut writer, "rels", "application/vnd.openxmlformats-package.relationships+xml")?;
    write_default(&mut writer, "xml", "application/xml")?;

    // Image extensions referenced by media
    let mut seen_extensions: Vec<String> = Vec::new();
    for img in &doc.images {
        let ext = extension_for_content_type(&img.content_type);
        if !seen_extensions.contains(&ext) {
            write_default(&mut writer, &ext, &img.content_type)?;
            seen_extensions.push(ext);
        }
    }

    // Override parts
    write_override(
        &mut writer,
        "/word/document.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
    )?;
    write_override(
        &mut writer,
        "/word/styles.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml",
    )?;

    if !doc.numbering.is_empty() {
        write_override(
            &mut writer,
            "/word/numbering.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml",
        )?;
    }

    // Comments
    if !doc.comments.is_empty() {
        write_override(
            &mut writer,
            "/word/comments.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml",
        )?;
    }

    // Footnotes
    if !doc.footnotes.is_empty() {
        write_override(
            &mut writer,
            "/word/footnotes.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml",
        )?;
    }

    // Endnotes
    if !doc.endnotes.is_empty() {
        write_override(
            &mut writer,
            "/word/endnotes.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml",
        )?;
    }

    for (i, _header) in doc.headers.iter().enumerate() {
        let part = format!("/word/header{}.xml", i + 1);
        write_override(
            &mut writer,
            &part,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml",
        )?;
    }

    for (i, _footer) in doc.footers.iter().enumerate() {
        let part = format!("/word/footer{}.xml", i + 1);
        write_override(
            &mut writer,
            &part,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml",
        )?;
    }

    writer.write_event(Event::End(BytesEnd::new("Types")))?;

    Ok(writer.into_inner().into_inner())
}

fn write_default(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    extension: &str,
    content_type: &str,
) -> Result<(), GenerateError> {
    let mut elem = BytesStart::new("Default");
    elem.push_attribute(("Extension", extension));
    elem.push_attribute(("ContentType", content_type));
    writer.write_event(Event::Empty(elem))?;
    Ok(())
}

fn write_override(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    part_name: &str,
    content_type: &str,
) -> Result<(), GenerateError> {
    let mut elem = BytesStart::new("Override");
    elem.push_attribute(("PartName", part_name));
    elem.push_attribute(("ContentType", content_type));
    writer.write_event(Event::Empty(elem))?;
    Ok(())
}

/// Public accessor for other modules that need the same mapping.
pub fn extension_for_content_type_pub(ct: &str) -> String {
    extension_for_content_type(ct)
}

/// Map a MIME content-type to a file extension for the Default element.
fn extension_for_content_type(ct: &str) -> String {
    match ct {
        "image/png" => "png".to_string(),
        "image/jpeg" | "image/jpg" => "jpeg".to_string(),
        "image/gif" => "gif".to_string(),
        "image/bmp" => "bmp".to_string(),
        "image/tiff" => "tiff".to_string(),
        "image/svg+xml" => "svg".to_string(),
        "image/webp" => "webp".to_string(),
        _ => {
            // Fallback: take whatever is after the slash
            ct.split('/').last().unwrap_or("bin").to_string()
        }
    }
}
