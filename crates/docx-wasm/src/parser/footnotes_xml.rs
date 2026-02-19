use quick_xml::events::Event;
use quick_xml::Reader;
use uuid::Uuid;

use crate::model::{
    BlockElement, Endnote, Footnote, Paragraph, ParagraphProperties, Run,
    RunProperties,
};
use super::ParseError;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse `word/footnotes.xml` into a list of `Footnote` values.
///
/// Footnotes with `w:type="separator"` or `w:type="continuationSeparator"`
/// (typically negative IDs like -1 and 0) are skipped since they are
/// internal separators, not user-authored content.
pub fn parse_footnotes(xml: &str) -> Result<Vec<Footnote>, ParseError> {
    let entries = parse_note_entries(xml, b"footnote", b"footnotes")?;
    Ok(entries
        .into_iter()
        .map(|e| Footnote {
            id: e.id,
            content: e.content,
        })
        .collect())
}

/// Parse `word/endnotes.xml` into a list of `Endnote` values.
///
/// Endnotes with `w:type="separator"` or `w:type="continuationSeparator"`
/// are skipped just as with footnotes.
pub fn parse_endnotes(xml: &str) -> Result<Vec<Endnote>, ParseError> {
    let entries = parse_note_entries(xml, b"endnote", b"endnotes")?;
    Ok(entries
        .into_iter()
        .map(|e| Endnote {
            id: e.id,
            content: e.content,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Shared note entry parser
// ---------------------------------------------------------------------------

/// Internal representation of a footnote or endnote before it is converted
/// into the concrete model type.
struct NoteEntry {
    id: String,
    content: Vec<BlockElement>,
}

/// Generic parser for both `<w:footnote>` and `<w:endnote>` elements.
///
/// The `note_tag` parameter is the local name of the individual note element
/// (e.g. `b"footnote"` or `b"endnote"`), and `container_tag` is the root
/// element (e.g. `b"footnotes"` or `b"endnotes"`).
fn parse_note_entries(
    xml: &str,
    note_tag: &[u8],
    _container_tag: &[u8],
) -> Result<Vec<NoteEntry>, ParseError> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    let mut entries: Vec<NoteEntry> = Vec::new();

    // Current note being built
    let mut current_note: Option<NoteBuilder> = None;
    // Whether the current note should be skipped (separator types)
    let mut skip_current = false;

    // Paragraph / run state inside a note
    let mut current_para: Option<ParaBuilder> = None;
    let mut current_run: Option<NtRunBuilder> = None;
    let mut in_ppr = false;
    let mut in_rpr = false;
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == note_tag {
                    // Read attributes to determine id and type
                    let mut id = String::new();
                    let mut note_type: Option<String> = None;

                    for attr in e.attributes().flatten() {
                        match attr.key.local_name().as_ref() {
                            b"id" => {
                                id = String::from_utf8_lossy(&attr.value)
                                    .to_string();
                            }
                            b"type" => {
                                note_type = Some(
                                    String::from_utf8_lossy(&attr.value)
                                        .to_string(),
                                );
                            }
                            _ => {}
                        }
                    }

                    // Skip separator and continuationSeparator notes
                    if let Some(ref t) = note_type {
                        if t == "separator" || t == "continuationSeparator" {
                            skip_current = true;
                            current_note = None;
                        } else {
                            skip_current = false;
                            current_note = Some(NoteBuilder::new(id));
                        }
                    } else {
                        skip_current = false;
                        current_note = Some(NoteBuilder::new(id));
                    }
                } else if !skip_current {
                    match local.as_ref() {
                        b"p" if current_note.is_some() => {
                            current_para = Some(ParaBuilder::new());
                        }
                        b"pPr" if current_para.is_some() => {
                            in_ppr = true;
                        }
                        b"r" if current_para.is_some() => {
                            current_run = Some(NtRunBuilder::new());
                        }
                        b"rPr" if current_run.is_some() => {
                            in_rpr = true;
                        }
                        b"t" if current_run.is_some() => {
                            in_text = true;
                        }
                        _ => {}
                    }
                }
            }

            Ok(Event::Empty(ref e)) => {
                if skip_current {
                    // Ignore everything inside a skipped note
                } else {
                    // Run properties
                    if in_rpr {
                        if let Some(ref mut run) = current_run {
                            handle_run_property(e, &mut run.properties);
                        }
                    }

                    // Paragraph properties
                    if in_ppr {
                        if let Some(ref mut para) = current_para {
                            handle_para_property(e, &mut para.properties);
                        }
                    }
                }
            }

            Ok(Event::Text(ref t)) => {
                if !skip_current && in_text {
                    if let Some(ref mut run) = current_run {
                        let text = t.unescape().unwrap_or_default().to_string();
                        run.text.push_str(&text);
                    }
                }
            }

            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == note_tag {
                    if !skip_current {
                        // Flush any pending paragraph
                        flush_nt_run(&mut current_run, &mut current_para);
                        flush_para(&mut current_para, &mut current_note);

                        if let Some(builder) = current_note.take() {
                            entries.push(builder.build());
                        }
                    }
                    skip_current = false;
                    current_note = None;
                } else if !skip_current {
                    match local.as_ref() {
                        b"p" => {
                            flush_nt_run(&mut current_run, &mut current_para);
                            flush_para(&mut current_para, &mut current_note);
                        }
                        b"r" => {
                            flush_nt_run(&mut current_run, &mut current_para);
                        }
                        b"t" => in_text = false,
                        b"pPr" => in_ppr = false,
                        b"rPr" => in_rpr = false,
                        _ => {}
                    }
                }
            }

            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

struct NoteBuilder {
    id: String,
    content: Vec<BlockElement>,
}

impl NoteBuilder {
    fn new(id: String) -> Self {
        Self {
            id,
            content: Vec::new(),
        }
    }

    fn build(self) -> NoteEntry {
        NoteEntry {
            id: self.id,
            content: self.content,
        }
    }
}

struct ParaBuilder {
    id: String,
    properties: ParagraphProperties,
    runs: Vec<Run>,
}

impl ParaBuilder {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            properties: ParagraphProperties::default(),
            runs: Vec::new(),
        }
    }

    fn build(self) -> Paragraph {
        Paragraph {
            id: self.id,
            properties: self.properties,
            runs: self.runs,
            bookmarks: Vec::new(),
        }
    }
}

struct NtRunBuilder {
    id: String,
    properties: RunProperties,
    text: String,
}

impl NtRunBuilder {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            properties: RunProperties::default(),
            text: String::new(),
        }
    }

    fn build(self) -> Run {
        Run {
            id: self.id,
            text: self.text,
            properties: self.properties,
        }
    }
}

// ---------------------------------------------------------------------------
// Flush helpers
// ---------------------------------------------------------------------------

/// Flush the current run into the current paragraph (if both exist).
fn flush_nt_run(
    current_run: &mut Option<NtRunBuilder>,
    current_para: &mut Option<ParaBuilder>,
) {
    if let Some(run_builder) = current_run.take() {
        let run = run_builder.build();
        if !run.text.is_empty() {
            if let Some(ref mut para) = current_para {
                para.runs.push(run);
            }
        }
    }
}

/// Flush the current paragraph into the current note (if both exist).
fn flush_para(
    current_para: &mut Option<ParaBuilder>,
    current_note: &mut Option<NoteBuilder>,
) {
    if let Some(pb) = current_para.take() {
        if let Some(ref mut note) = current_note {
            note.content.push(BlockElement::Paragraph(pb.build()));
        }
    }
}

// ---------------------------------------------------------------------------
// Property helpers
// ---------------------------------------------------------------------------

fn handle_run_property(
    e: &quick_xml::events::BytesStart<'_>,
    props: &mut RunProperties,
) {
    let local = e.local_name();
    match local.as_ref() {
        b"b" => props.bold = Some(!is_val_false(e)),
        b"i" => props.italic = Some(!is_val_false(e)),
        b"u" => props.underline = Some(true),
        b"strike" => props.strikethrough = Some(!is_val_false(e)),
        b"sz" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    if let Ok(half_pts) =
                        String::from_utf8_lossy(&attr.value).parse::<f64>()
                    {
                        props.font_size = Some(half_pts / 2.0);
                    }
                }
            }
        }
        b"color" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    if val != "auto" {
                        props.color = Some(val);
                    }
                }
            }
        }
        b"rFonts" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"ascii" {
                    props.font_family =
                        Some(String::from_utf8_lossy(&attr.value).to_string());
                }
            }
        }
        b"highlight" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    props.highlight =
                        Some(String::from_utf8_lossy(&attr.value).to_string());
                }
            }
        }
        _ => {}
    }
}

fn handle_para_property(
    e: &quick_xml::events::BytesStart<'_>,
    props: &mut ParagraphProperties,
) {
    let local = e.local_name();
    match local.as_ref() {
        b"jc" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    let val = String::from_utf8_lossy(&attr.value);
                    props.alignment = Some(parse_alignment(&val));
                }
            }
        }
        b"pStyle" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    props.style_id = Some(val);
                }
            }
        }
        b"spacing" => {
            for attr in e.attributes().flatten() {
                match attr.key.local_name().as_ref() {
                    b"before" => {
                        if let Ok(v) =
                            String::from_utf8_lossy(&attr.value).parse::<f64>()
                        {
                            props.spacing_before = Some(v);
                        }
                    }
                    b"after" => {
                        if let Ok(v) =
                            String::from_utf8_lossy(&attr.value).parse::<f64>()
                        {
                            props.spacing_after = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

fn parse_alignment(val: &str) -> crate::model::Alignment {
    match val {
        "center" => crate::model::Alignment::Center,
        "right" | "end" => crate::model::Alignment::Right,
        "both" | "justify" => crate::model::Alignment::Justify,
        _ => crate::model::Alignment::Left,
    }
}

fn is_val_false(e: &quick_xml::events::BytesStart<'_>) -> bool {
    for attr in e.attributes().flatten() {
        if attr.key.local_name().as_ref() == b"val" {
            let v = String::from_utf8_lossy(&attr.value);
            return v == "false" || v == "0";
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Footnotes ----

    #[test]
    fn test_parse_empty_footnotes() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        </w:footnotes>"#;

        let footnotes = parse_footnotes(xml).unwrap();
        assert!(footnotes.is_empty());
    }

    #[test]
    fn test_parse_footnotes_skips_separators() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
          <w:footnote w:type="separator" w:id="-1">
            <w:p><w:r><w:t>sep</w:t></w:r></w:p>
          </w:footnote>
          <w:footnote w:type="continuationSeparator" w:id="0">
            <w:p><w:r><w:t>cont</w:t></w:r></w:p>
          </w:footnote>
          <w:footnote w:id="1">
            <w:p><w:r><w:t>First footnote</w:t></w:r></w:p>
          </w:footnote>
        </w:footnotes>"#;

        let footnotes = parse_footnotes(xml).unwrap();
        assert_eq!(footnotes.len(), 1);
        assert_eq!(footnotes[0].id, "1");

        if let BlockElement::Paragraph(ref p) = footnotes[0].content[0] {
            assert_eq!(p.runs.len(), 1);
            assert_eq!(p.runs[0].text, "First footnote");
        } else {
            panic!("Expected paragraph in footnote content");
        }
    }

    #[test]
    fn test_parse_multiple_footnotes() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
          <w:footnote w:type="separator" w:id="-1">
            <w:p><w:r><w:t>---</w:t></w:r></w:p>
          </w:footnote>
          <w:footnote w:id="1">
            <w:p><w:r><w:t>Footnote one</w:t></w:r></w:p>
          </w:footnote>
          <w:footnote w:id="2">
            <w:p><w:r><w:t>Footnote two</w:t></w:r></w:p>
            <w:p><w:r><w:t>continued</w:t></w:r></w:p>
          </w:footnote>
        </w:footnotes>"#;

        let footnotes = parse_footnotes(xml).unwrap();
        assert_eq!(footnotes.len(), 2);
        assert_eq!(footnotes[0].id, "1");
        assert_eq!(footnotes[1].id, "2");
        assert_eq!(footnotes[1].content.len(), 2);
    }

    #[test]
    fn test_parse_footnote_with_formatting() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
          <w:footnote w:id="1">
            <w:p>
              <w:r>
                <w:rPr><w:b/><w:sz w:val="20"/></w:rPr>
                <w:t>Bold small text</w:t>
              </w:r>
            </w:p>
          </w:footnote>
        </w:footnotes>"#;

        let footnotes = parse_footnotes(xml).unwrap();
        assert_eq!(footnotes.len(), 1);

        if let BlockElement::Paragraph(ref p) = footnotes[0].content[0] {
            assert_eq!(p.runs[0].text, "Bold small text");
            assert_eq!(p.runs[0].properties.bold, Some(true));
            assert_eq!(p.runs[0].properties.font_size, Some(10.0));
        } else {
            panic!("Expected paragraph");
        }
    }

    // ---- Endnotes ----

    #[test]
    fn test_parse_empty_endnotes() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:endnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        </w:endnotes>"#;

        let endnotes = parse_endnotes(xml).unwrap();
        assert!(endnotes.is_empty());
    }

    #[test]
    fn test_parse_endnotes_skips_separators() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:endnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
          <w:endnote w:type="separator" w:id="-1">
            <w:p><w:r><w:t>sep</w:t></w:r></w:p>
          </w:endnote>
          <w:endnote w:type="continuationSeparator" w:id="0">
            <w:p><w:r><w:t>cont</w:t></w:r></w:p>
          </w:endnote>
          <w:endnote w:id="1">
            <w:p><w:r><w:t>First endnote</w:t></w:r></w:p>
          </w:endnote>
          <w:endnote w:id="2">
            <w:p><w:r><w:t>Second endnote</w:t></w:r></w:p>
          </w:endnote>
        </w:endnotes>"#;

        let endnotes = parse_endnotes(xml).unwrap();
        assert_eq!(endnotes.len(), 2);
        assert_eq!(endnotes[0].id, "1");
        assert_eq!(endnotes[1].id, "2");

        if let BlockElement::Paragraph(ref p) = endnotes[0].content[0] {
            assert_eq!(p.runs[0].text, "First endnote");
        } else {
            panic!("Expected paragraph in endnote content");
        }
    }
}
