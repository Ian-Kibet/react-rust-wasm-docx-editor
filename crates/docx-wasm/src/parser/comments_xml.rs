use quick_xml::events::Event;
use quick_xml::Reader;
use uuid::Uuid;

use crate::model::{
    BlockElement, Comment, Paragraph, ParagraphProperties, Run, RunProperties,
};
use super::ParseError;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse `word/comments.xml` into a list of `Comment` values.
///
/// Each `<w:comment>` element is extracted with its attributes (`w:id`,
/// `w:author`, `w:date`, `w:initials`) and its inner paragraphs (simplified:
/// paragraph > run > text).
pub fn parse_comments(xml: &str) -> Result<Vec<Comment>, ParseError> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    let mut comments: Vec<Comment> = Vec::new();

    // Current comment being built
    let mut current_comment: Option<CommentBuilder> = None;

    // Paragraph / run state inside a comment
    let mut current_para: Option<ParaBuilder> = None;
    let mut current_run: Option<CmRunBuilder> = None;
    let mut in_ppr = false;
    let mut in_rpr = false;
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"comment" => {
                        let mut builder = CommentBuilder::new();
                        for attr in e.attributes().flatten() {
                            match attr.key.local_name().as_ref() {
                                b"id" => {
                                    builder.id =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                }
                                b"author" => {
                                    builder.author =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                }
                                b"date" => {
                                    builder.date = Some(
                                        String::from_utf8_lossy(&attr.value).to_string(),
                                    );
                                }
                                b"initials" => {
                                    builder.initials = Some(
                                        String::from_utf8_lossy(&attr.value).to_string(),
                                    );
                                }
                                _ => {}
                            }
                        }
                        current_comment = Some(builder);
                    }

                    b"p" if current_comment.is_some() => {
                        current_para = Some(ParaBuilder::new());
                    }
                    b"pPr" if current_para.is_some() => {
                        in_ppr = true;
                    }
                    b"r" if current_para.is_some() => {
                        current_run = Some(CmRunBuilder::new());
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

            Ok(Event::Empty(ref e)) => {
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

            Ok(Event::Text(ref t)) => {
                if in_text {
                    if let Some(ref mut run) = current_run {
                        let text = t.unescape().unwrap_or_default().to_string();
                        run.text.push_str(&text);
                    }
                }
            }

            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"comment" => {
                        // Flush any pending paragraph
                        flush_cm_run(&mut current_run, &mut current_para);
                        flush_para(&mut current_para, &mut current_comment);

                        if let Some(builder) = current_comment.take() {
                            comments.push(builder.build());
                        }
                    }
                    b"p" => {
                        flush_cm_run(&mut current_run, &mut current_para);
                        flush_para(&mut current_para, &mut current_comment);
                    }
                    b"r" => {
                        flush_cm_run(&mut current_run, &mut current_para);
                    }
                    b"t" => in_text = false,
                    b"pPr" => in_ppr = false,
                    b"rPr" => in_rpr = false,
                    _ => {}
                }
            }

            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(comments)
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

struct CommentBuilder {
    id: String,
    author: String,
    date: Option<String>,
    initials: Option<String>,
    content: Vec<BlockElement>,
}

impl CommentBuilder {
    fn new() -> Self {
        Self {
            id: String::new(),
            author: String::new(),
            date: None,
            initials: None,
            content: Vec::new(),
        }
    }

    fn build(self) -> Comment {
        Comment {
            id: self.id,
            author: self.author,
            date: self.date,
            initials: self.initials,
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

struct CmRunBuilder {
    id: String,
    properties: RunProperties,
    text: String,
}

impl CmRunBuilder {
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
fn flush_cm_run(
    current_run: &mut Option<CmRunBuilder>,
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

/// Flush the current paragraph into the current comment (if both exist).
fn flush_para(
    current_para: &mut Option<ParaBuilder>,
    current_comment: &mut Option<CommentBuilder>,
) {
    if let Some(pb) = current_para.take() {
        if let Some(ref mut comment) = current_comment {
            comment.content.push(BlockElement::Paragraph(pb.build()));
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

    #[test]
    fn test_parse_empty_comments() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        </w:comments>"#;

        let comments = parse_comments(xml).unwrap();
        assert!(comments.is_empty());
    }

    #[test]
    fn test_parse_single_comment() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
          <w:comment w:id="1" w:author="John Doe" w:date="2024-01-01T00:00:00Z" w:initials="JD">
            <w:p>
              <w:r><w:t>This is a comment</w:t></w:r>
            </w:p>
          </w:comment>
        </w:comments>"#;

        let comments = parse_comments(xml).unwrap();
        assert_eq!(comments.len(), 1);

        let c = &comments[0];
        assert_eq!(c.id, "1");
        assert_eq!(c.author, "John Doe");
        assert_eq!(c.date.as_deref(), Some("2024-01-01T00:00:00Z"));
        assert_eq!(c.initials.as_deref(), Some("JD"));
        assert_eq!(c.content.len(), 1);

        if let BlockElement::Paragraph(ref p) = c.content[0] {
            assert_eq!(p.runs.len(), 1);
            assert_eq!(p.runs[0].text, "This is a comment");
        } else {
            panic!("Expected paragraph in comment content");
        }
    }

    #[test]
    fn test_parse_multiple_comments() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
          <w:comment w:id="1" w:author="Alice">
            <w:p><w:r><w:t>First comment</w:t></w:r></w:p>
          </w:comment>
          <w:comment w:id="2" w:author="Bob" w:date="2024-06-15T12:00:00Z">
            <w:p><w:r><w:t>Second comment</w:t></w:r></w:p>
            <w:p><w:r><w:t>with two paragraphs</w:t></w:r></w:p>
          </w:comment>
        </w:comments>"#;

        let comments = parse_comments(xml).unwrap();
        assert_eq!(comments.len(), 2);

        assert_eq!(comments[0].id, "1");
        assert_eq!(comments[0].author, "Alice");
        assert!(comments[0].date.is_none());
        assert!(comments[0].initials.is_none());
        assert_eq!(comments[0].content.len(), 1);

        assert_eq!(comments[1].id, "2");
        assert_eq!(comments[1].author, "Bob");
        assert_eq!(comments[1].date.as_deref(), Some("2024-06-15T12:00:00Z"));
        assert_eq!(comments[1].content.len(), 2);
    }

    #[test]
    fn test_parse_comment_with_formatted_run() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
          <w:comment w:id="3" w:author="Test">
            <w:p>
              <w:r>
                <w:rPr><w:b/><w:i/></w:rPr>
                <w:t>Bold italic text</w:t>
              </w:r>
            </w:p>
          </w:comment>
        </w:comments>"#;

        let comments = parse_comments(xml).unwrap();
        assert_eq!(comments.len(), 1);

        if let BlockElement::Paragraph(ref p) = comments[0].content[0] {
            assert_eq!(p.runs[0].text, "Bold italic text");
            assert_eq!(p.runs[0].properties.bold, Some(true));
            assert_eq!(p.runs[0].properties.italic, Some(true));
        } else {
            panic!("Expected paragraph");
        }
    }
}
