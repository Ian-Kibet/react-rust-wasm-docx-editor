use crate::model::*;

// ============================================================
// Model tests
// ============================================================

#[test]
fn test_document_new() {
    let doc = Document::new();
    assert_eq!(doc.body.len(), 1);
    assert!(doc.styles.is_empty());
    assert!(doc.numbering.is_empty());
    assert!(doc.headers.is_empty());
    assert!(doc.footers.is_empty());
    assert!(doc.images.is_empty());
    assert!(doc.comments.is_empty());
    assert!(doc.footnotes.is_empty());
    assert!(doc.endnotes.is_empty());
    assert!(doc.section_properties.is_none());
}

#[test]
fn test_document_default() {
    let doc = Document::default();
    assert_eq!(doc.body.len(), 1);
}

#[test]
fn test_word_count_empty() {
    let mut doc = Document::new();
    doc.body = vec![];
    assert_eq!(doc.word_count(), 0);
}

#[test]
fn test_word_count_simple() {
    let doc = make_doc_with_text("Hello world foo bar");
    assert_eq!(doc.word_count(), 4);
}

#[test]
fn test_word_count_multi_paragraph() {
    let mut doc = Document::new();
    doc.body = vec![
        make_para("Hello world"),
        make_para("foo bar baz"),
    ];
    assert_eq!(doc.word_count(), 5);
}

#[test]
fn test_char_count() {
    let doc = make_doc_with_text("Hello");
    assert_eq!(doc.char_count(), 5);
}

#[test]
fn test_paragraph_count() {
    let mut doc = Document::new();
    doc.body = vec![
        make_para("One"),
        make_para("Two"),
        make_para("Three"),
    ];
    assert_eq!(doc.paragraph_count(), 3);
}

#[test]
fn test_extract_text() {
    let mut doc = Document::new();
    doc.body = vec![
        make_para("Hello"),
        make_para("World"),
    ];
    let text = doc.extract_text();
    assert!(text.contains("Hello"));
    assert!(text.contains("World"));
}

#[test]
fn test_statistics() {
    let mut doc = Document::new();
    doc.body = vec![
        make_para("Hello world"),
        make_para("Foo bar"),
    ];
    doc.comments.push(Comment {
        id: "1".to_string(),
        author: "Test".to_string(),
        date: None,
        initials: None,
        content: vec![],
    });
    let stats = doc.statistics();
    assert_eq!(stats.word_count, 4);
    assert_eq!(stats.paragraph_count, 2);
    assert_eq!(stats.comment_count, 1);
}

#[test]
fn test_paragraph_properties_default() {
    let props = ParagraphProperties::default();
    assert!(props.alignment.is_none());
    assert!(props.heading_level.is_none());
    assert!(props.numbering.is_none());
    assert!(props.page_break_before.is_none());
    assert!(props.keep_next.is_none());
    assert!(props.keep_lines.is_none());
    assert!(props.line_spacing.is_none());
    assert!(props.tab_stops.is_empty());
}

#[test]
fn test_run_properties_default() {
    let props = RunProperties::default();
    assert!(props.bold.is_none());
    assert!(props.italic.is_none());
    assert!(props.superscript.is_none());
    assert!(props.subscript.is_none());
    assert!(props.hyperlink_url.is_none());
    assert!(props.line_break.is_none());
    assert!(props.page_break.is_none());
    assert!(props.tab.is_none());
}

#[test]
fn test_alignment_serde() {
    let align = Alignment::Center;
    let json = serde_json::to_string(&align).unwrap();
    assert_eq!(json, "\"center\"");
    let parsed: Alignment = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, Alignment::Center);
}

#[test]
fn test_page_orientation_serde() {
    let orient = PageOrientation::Landscape;
    let json = serde_json::to_string(&orient).unwrap();
    assert_eq!(json, "\"landscape\"");
    let parsed: PageOrientation = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, PageOrientation::Landscape);
}

#[test]
fn test_vertical_merge_serde() {
    let vm = VerticalMerge::Restart;
    let json = serde_json::to_string(&vm).unwrap();
    assert_eq!(json, "\"restart\"");
    let parsed: VerticalMerge = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, VerticalMerge::Restart);
}

// ============================================================
// JSON round-trip tests
// ============================================================

#[test]
fn test_document_json_roundtrip() {
    let doc = Document::new();
    let json = serde_json::to_string(&doc).unwrap();
    let parsed: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.body.len(), doc.body.len());
}

#[test]
fn test_document_with_all_features_roundtrip() {
    let mut doc = Document::new();
    doc.section_properties = Some(SectionProperties {
        page_width: Some(12240.0),
        page_height: Some(15840.0),
        margin_top: Some(1440.0),
        margin_right: Some(1440.0),
        margin_bottom: Some(1440.0),
        margin_left: Some(1440.0),
        margin_header: Some(720.0),
        margin_footer: Some(720.0),
        margin_gutter: Some(0.0),
        page_orientation: Some(PageOrientation::Portrait),
        columns: Some(1),
    });

    doc.comments.push(Comment {
        id: "1".to_string(),
        author: "Test Author".to_string(),
        date: Some("2024-01-01T00:00:00Z".to_string()),
        initials: Some("TA".to_string()),
        content: vec![make_para("Comment text")],
    });

    doc.footnotes.push(Footnote {
        id: "1".to_string(),
        content: vec![make_para("Footnote text")],
    });

    doc.endnotes.push(Endnote {
        id: "1".to_string(),
        content: vec![make_para("Endnote text")],
    });

    let json = serde_json::to_string(&doc).unwrap();
    let parsed: Document = serde_json::from_str(&json).unwrap();
    assert!(parsed.section_properties.is_some());
    assert_eq!(parsed.comments.len(), 1);
    assert_eq!(parsed.footnotes.len(), 1);
    assert_eq!(parsed.endnotes.len(), 1);
}

#[test]
fn test_run_properties_roundtrip() {
    let props = RunProperties {
        bold: Some(true),
        italic: Some(true),
        underline: Some(true),
        strikethrough: Some(true),
        double_strikethrough: Some(false),
        superscript: Some(true),
        subscript: None,
        font_family: Some("Arial".to_string()),
        font_size: Some(12.0),
        color: Some("FF0000".to_string()),
        highlight: Some("yellow".to_string()),
        background_color: Some("EEEEEE".to_string()),
        inline_image: None,
        hyperlink_url: Some("https://example.com".to_string()),
        hyperlink_tooltip: Some("Click here".to_string()),
        line_break: None,
        page_break: None,
        tab: None,
        small_caps: Some(true),
        all_caps: None,
        spacing: Some(20.0),
        footnote_ref: Some("1".to_string()),
        endnote_ref: None,
    };

    let json = serde_json::to_string(&props).unwrap();
    let parsed: RunProperties = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.bold, Some(true));
    assert_eq!(parsed.superscript, Some(true));
    assert_eq!(parsed.hyperlink_url, Some("https://example.com".to_string()));
    assert_eq!(parsed.small_caps, Some(true));
    assert_eq!(parsed.spacing, Some(20.0));
}

#[test]
fn test_paragraph_properties_roundtrip() {
    let props = ParagraphProperties {
        alignment: Some(Alignment::Justify),
        heading_level: Some(2),
        numbering: Some(NumberingRef {
            num_id: "1".to_string(),
            level: 0,
        }),
        spacing_before: Some(100.0),
        spacing_after: Some(200.0),
        line_spacing: Some(240.0),
        line_spacing_rule: Some(LineSpacingRule::Exact),
        indent_left: Some(720.0),
        indent_right: None,
        indent_first_line: Some(360.0),
        indent_hanging: None,
        page_break_before: Some(true),
        keep_next: Some(true),
        keep_lines: Some(false),
        widow_control: Some(true),
        style_id: Some("Heading2".to_string()),
        border_bottom: Some(Border {
            style: "single".to_string(),
            size: 4.0,
            color: "000000".to_string(),
        }),
        border_top: None,
        shading: Some("FFFF00".to_string()),
        tab_stops: vec![
            TabStop {
                position: 720.0,
                alignment: TabStopAlignment::Left,
                leader: None,
            },
        ],
    };

    let json = serde_json::to_string(&props).unwrap();
    let parsed: ParagraphProperties = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.alignment, Some(Alignment::Justify));
    assert_eq!(parsed.heading_level, Some(2));
    assert_eq!(parsed.page_break_before, Some(true));
    assert_eq!(parsed.tab_stops.len(), 1);
}

#[test]
fn test_table_cell_properties_roundtrip() {
    let props = TableCellProperties {
        width: Some(1440.0),
        vertical_align: Some("center".to_string()),
        borders: None,
        shading: Some("DDDDDD".to_string()),
        grid_span: Some(2),
        vertical_merge: Some(VerticalMerge::Restart),
        text_direction: Some("btLr".to_string()),
        no_wrap: Some(true),
        padding_top: Some(57.0),
        padding_bottom: Some(57.0),
        padding_left: Some(115.0),
        padding_right: Some(115.0),
    };

    let json = serde_json::to_string(&props).unwrap();
    let parsed: TableCellProperties = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.grid_span, Some(2));
    assert_eq!(parsed.vertical_merge, Some(VerticalMerge::Restart));
    assert_eq!(parsed.no_wrap, Some(true));
}

// ============================================================
// Backward compatibility test (old JSON without new fields)
// ============================================================

#[test]
fn test_backward_compatible_deserialization() {
    // Old format JSON without new fields should still parse
    let json = r#"{
        "body": [],
        "styles": [],
        "numbering": [],
        "headers": [],
        "footers": [],
        "images": []
    }"#;

    let doc: Document = serde_json::from_str(json).unwrap();
    assert!(doc.comments.is_empty());
    assert!(doc.footnotes.is_empty());
    assert!(doc.endnotes.is_empty());
    assert!(doc.section_properties.is_none());
}

#[test]
fn test_old_run_properties_deserialization() {
    let json = r#"{
        "bold": true,
        "italic": false
    }"#;

    let props: RunProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.bold, Some(true));
    assert!(props.superscript.is_none());
    assert!(props.hyperlink_url.is_none());
    assert!(props.line_break.is_none());
}

#[test]
fn test_old_paragraph_deserialization() {
    let json = r#"{
        "id": "test-id",
        "runs": [],
        "properties": {}
    }"#;

    let para: Paragraph = serde_json::from_str(json).unwrap();
    assert_eq!(para.id, "test-id");
    assert!(para.bookmarks.is_empty());
}

// ============================================================
// Generator tests
// ============================================================

#[test]
fn test_generate_empty_document() {
    let doc = Document::new();
    let result = crate::generator::generate(&doc);
    assert!(result.is_ok());
    let bytes = result.unwrap();
    // Should be a valid ZIP (starts with PK magic bytes)
    assert!(bytes.len() > 4);
    assert_eq!(bytes[0], 0x50); // P
    assert_eq!(bytes[1], 0x4B); // K
}

#[test]
fn test_generate_document_with_text() {
    let doc = make_doc_with_text("Hello World");
    let result = crate::generator::generate(&doc);
    assert!(result.is_ok());
}

#[test]
fn test_generate_document_with_formatting() {
    let mut doc = Document::new();
    let run = Run {
        id: "r1".to_string(),
        text: "Bold and italic".to_string(),
        properties: RunProperties {
            bold: Some(true),
            italic: Some(true),
            superscript: Some(true),
            font_size: Some(14.0),
            color: Some("FF0000".to_string()),
            ..RunProperties::default()
        },
    };
    doc.body = vec![BlockElement::Paragraph(Paragraph {
        id: "p1".to_string(),
        runs: vec![run],
        properties: ParagraphProperties {
            alignment: Some(Alignment::Center),
            page_break_before: Some(true),
            keep_next: Some(true),
            ..ParagraphProperties::default()
        },
        bookmarks: vec![],
    })];
    let result = crate::generator::generate(&doc);
    assert!(result.is_ok());
}

#[test]
fn test_generate_document_with_table() {
    let mut doc = Document::new();
    let cell = TableCell {
        id: "c1".to_string(),
        content: vec![make_para("Cell content")],
        properties: TableCellProperties {
            grid_span: Some(2),
            vertical_merge: Some(VerticalMerge::Restart),
            ..TableCellProperties::default()
        },
    };
    let table = Table {
        id: "t1".to_string(),
        rows: vec![TableRow {
            id: "tr1".to_string(),
            cells: vec![cell],
            properties: TableRowProperties::default(),
        }],
        properties: TableProperties::default(),
    };
    doc.body = vec![BlockElement::Table(table)];
    let result = crate::generator::generate(&doc);
    assert!(result.is_ok());
}

#[test]
fn test_generate_document_with_hyperlink() {
    let mut doc = Document::new();
    let run = Run {
        id: "r1".to_string(),
        text: "Click here".to_string(),
        properties: RunProperties {
            hyperlink_url: Some("https://example.com".to_string()),
            hyperlink_tooltip: Some("Example".to_string()),
            ..RunProperties::default()
        },
    };
    doc.body = vec![BlockElement::Paragraph(Paragraph {
        id: "p1".to_string(),
        runs: vec![run],
        properties: ParagraphProperties::default(),
        bookmarks: vec![],
    })];
    let result = crate::generator::generate(&doc);
    assert!(result.is_ok());
}

#[test]
fn test_generate_document_with_section_properties() {
    let mut doc = Document::new();
    doc.section_properties = Some(SectionProperties {
        page_width: Some(11906.0),
        page_height: Some(16838.0),
        margin_top: Some(1440.0),
        margin_right: Some(1800.0),
        margin_bottom: Some(1440.0),
        margin_left: Some(1800.0),
        margin_header: Some(720.0),
        margin_footer: Some(720.0),
        margin_gutter: None,
        page_orientation: Some(PageOrientation::Landscape),
        columns: Some(2),
    });
    let result = crate::generator::generate(&doc);
    assert!(result.is_ok());
}

#[test]
fn test_generate_document_with_comments() {
    let mut doc = Document::new();
    doc.comments.push(Comment {
        id: "1".to_string(),
        author: "Test".to_string(),
        date: Some("2024-01-01T00:00:00Z".to_string()),
        initials: Some("T".to_string()),
        content: vec![make_para("A comment")],
    });
    let result = crate::generator::generate(&doc);
    assert!(result.is_ok());
}

#[test]
fn test_generate_document_with_footnotes() {
    let mut doc = Document::new();
    doc.footnotes.push(Footnote {
        id: "1".to_string(),
        content: vec![make_para("A footnote")],
    });
    let result = crate::generator::generate(&doc);
    assert!(result.is_ok());
}

#[test]
fn test_generate_document_with_line_break_run() {
    let mut doc = Document::new();
    let run = Run {
        id: "r1".to_string(),
        text: String::new(),
        properties: RunProperties {
            line_break: Some(true),
            ..RunProperties::default()
        },
    };
    doc.body = vec![BlockElement::Paragraph(Paragraph {
        id: "p1".to_string(),
        runs: vec![run],
        properties: ParagraphProperties::default(),
        bookmarks: vec![],
    })];
    let result = crate::generator::generate(&doc);
    assert!(result.is_ok());
}

#[test]
fn test_generate_document_with_page_break_run() {
    let mut doc = Document::new();
    let run = Run {
        id: "r1".to_string(),
        text: String::new(),
        properties: RunProperties {
            page_break: Some(true),
            ..RunProperties::default()
        },
    };
    doc.body = vec![BlockElement::Paragraph(Paragraph {
        id: "p1".to_string(),
        runs: vec![run],
        properties: ParagraphProperties::default(),
        bookmarks: vec![],
    })];
    let result = crate::generator::generate(&doc);
    assert!(result.is_ok());
}

#[test]
fn test_generate_document_with_tab_run() {
    let mut doc = Document::new();
    let run = Run {
        id: "r1".to_string(),
        text: "\t".to_string(),
        properties: RunProperties {
            tab: Some(true),
            ..RunProperties::default()
        },
    };
    doc.body = vec![BlockElement::Paragraph(Paragraph {
        id: "p1".to_string(),
        runs: vec![run],
        properties: ParagraphProperties::default(),
        bookmarks: vec![],
    })];
    let result = crate::generator::generate(&doc);
    assert!(result.is_ok());
}

// ============================================================
// Round-trip tests (generate -> parse)
// ============================================================

#[test]
fn test_roundtrip_simple_document() {
    let doc = make_doc_with_text("Hello World");
    let bytes = crate::generator::generate(&doc).unwrap();
    let parsed = crate::parser::parse(&bytes).unwrap();

    // The parsed document should contain our text
    let text = parsed.extract_text();
    assert!(text.contains("Hello World"), "Expected 'Hello World' in: {}", text);
}

#[test]
fn test_roundtrip_multiple_paragraphs() {
    let mut doc = Document::new();
    doc.body = vec![
        make_para("First paragraph"),
        make_para("Second paragraph"),
        make_para("Third paragraph"),
    ];
    let bytes = crate::generator::generate(&doc).unwrap();
    let parsed = crate::parser::parse(&bytes).unwrap();

    assert_eq!(parsed.paragraph_count(), 3);
    let text = parsed.extract_text();
    assert!(text.contains("First paragraph"));
    assert!(text.contains("Second paragraph"));
    assert!(text.contains("Third paragraph"));
}

#[test]
fn test_roundtrip_bold_italic() {
    let mut doc = Document::new();
    let run = Run {
        id: "r1".to_string(),
        text: "formatted".to_string(),
        properties: RunProperties {
            bold: Some(true),
            italic: Some(true),
            underline: Some(true),
            ..RunProperties::default()
        },
    };
    doc.body = vec![BlockElement::Paragraph(Paragraph {
        id: "p1".to_string(),
        runs: vec![run],
        properties: ParagraphProperties::default(),
        bookmarks: vec![],
    })];

    let bytes = crate::generator::generate(&doc).unwrap();
    let parsed = crate::parser::parse(&bytes).unwrap();

    if let BlockElement::Paragraph(p) = &parsed.body[0] {
        assert!(!p.runs.is_empty());
        let r = &p.runs[0];
        assert_eq!(r.properties.bold, Some(true));
        assert_eq!(r.properties.italic, Some(true));
        assert_eq!(r.properties.underline, Some(true));
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_roundtrip_alignment() {
    let mut doc = Document::new();
    doc.body = vec![BlockElement::Paragraph(Paragraph {
        id: "p1".to_string(),
        runs: vec![Run {
            id: "r1".to_string(),
            text: "centered".to_string(),
            properties: RunProperties::default(),
        }],
        properties: ParagraphProperties {
            alignment: Some(Alignment::Center),
            ..ParagraphProperties::default()
        },
        bookmarks: vec![],
    })];

    let bytes = crate::generator::generate(&doc).unwrap();
    let parsed = crate::parser::parse(&bytes).unwrap();

    if let BlockElement::Paragraph(p) = &parsed.body[0] {
        assert_eq!(p.properties.alignment, Some(Alignment::Center));
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_roundtrip_numbering() {
    let mut doc = Document::new();
    doc.numbering = vec![ListDef {
        num_id: "1".to_string(),
        levels: vec![ListLevel {
            level: 0,
            format: "bullet".to_string(),
            text: "\u{2022}".to_string(),
            start: 1,
        }],
    }];
    doc.body = vec![BlockElement::Paragraph(Paragraph {
        id: "p1".to_string(),
        runs: vec![Run {
            id: "r1".to_string(),
            text: "list item".to_string(),
            properties: RunProperties::default(),
        }],
        properties: ParagraphProperties {
            numbering: Some(NumberingRef {
                num_id: "1".to_string(),
                level: 0,
            }),
            ..ParagraphProperties::default()
        },
        bookmarks: vec![],
    })];

    let bytes = crate::generator::generate(&doc).unwrap();
    let parsed = crate::parser::parse(&bytes).unwrap();

    if let BlockElement::Paragraph(p) = &parsed.body[0] {
        assert!(p.properties.numbering.is_some());
        let nr = p.properties.numbering.as_ref().unwrap();
        assert_eq!(nr.num_id, "1");
        assert_eq!(nr.level, 0);
    } else {
        panic!("Expected paragraph");
    }
}

// ============================================================
// Styles parser test
// ============================================================

#[test]
fn test_parse_styles_xml() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:style w:type="paragraph" w:styleId="Normal" w:default="1">
            <w:name w:val="Normal"/>
        </w:style>
        <w:style w:type="paragraph" w:styleId="Heading1">
            <w:name w:val="heading 1"/>
            <w:basedOn w:val="Normal"/>
            <w:pPr>
                <w:keepNext/>
                <w:spacing w:before="240" w:after="60"/>
            </w:pPr>
            <w:rPr>
                <w:b/>
                <w:sz w:val="32"/>
            </w:rPr>
        </w:style>
    </w:styles>"#;

    let styles = crate::parser::styles_xml::parse_styles(xml).unwrap();
    assert_eq!(styles.len(), 2);

    assert_eq!(styles[0].id, "Normal");
    assert_eq!(styles[0].style_type, "paragraph");

    assert_eq!(styles[1].id, "Heading1");
    assert_eq!(styles[1].based_on, Some("Normal".to_string()));
    assert!(styles[1].run_properties.is_some());
    let rpr = styles[1].run_properties.as_ref().unwrap();
    assert_eq!(rpr.bold, Some(true));
    assert_eq!(rpr.font_size, Some(16.0)); // 32 half-points = 16 points
}

// ============================================================
// Numbering parser test
// ============================================================

#[test]
fn test_parse_numbering_xml() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:abstractNum w:abstractNumId="0">
            <w:lvl w:ilvl="0">
                <w:start w:val="1"/>
                <w:numFmt w:val="bullet"/>
                <w:lvlText w:val="•"/>
            </w:lvl>
        </w:abstractNum>
        <w:num w:numId="1">
            <w:abstractNumId w:val="0"/>
        </w:num>
    </w:numbering>"#;

    let numbering = crate::parser::numbering_xml::parse_numbering(xml).unwrap();
    assert!(!numbering.is_empty());
}

// ============================================================
// Comments parser test
// ============================================================

#[test]
fn test_parse_comments_xml() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:comment w:id="1" w:author="Test User" w:date="2024-01-01T00:00:00Z" w:initials="TU">
            <w:p>
                <w:r>
                    <w:t>This is a comment</w:t>
                </w:r>
            </w:p>
        </w:comment>
    </w:comments>"#;

    let comments = crate::parser::comments_xml::parse_comments(xml).unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].author, "Test User");
    assert_eq!(comments[0].date, Some("2024-01-01T00:00:00Z".to_string()));
    assert_eq!(comments[0].initials, Some("TU".to_string()));
}

// ============================================================
// Footnotes parser test
// ============================================================

#[test]
fn test_parse_footnotes_xml() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:footnote w:type="separator" w:id="-1">
            <w:p><w:r><w:separator/></w:r></w:p>
        </w:footnote>
        <w:footnote w:id="1">
            <w:p>
                <w:r>
                    <w:t>Footnote content</w:t>
                </w:r>
            </w:p>
        </w:footnote>
    </w:footnotes>"#;

    let footnotes = crate::parser::footnotes_xml::parse_footnotes(xml).unwrap();
    // Should skip separator footnotes
    assert!(!footnotes.is_empty());
}

// ============================================================
// Search functionality test
// ============================================================

#[test]
fn test_search_text_case_sensitive() {
    let doc = make_doc_with_text("Hello World hello world");
    let json = serde_json::to_string(&doc).unwrap();

    // Mock search
    let results = crate::search_in_document(&doc, "hello", true);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].offset, 12);
}

#[test]
fn test_search_text_case_insensitive() {
    let doc = make_doc_with_text("Hello World hello world");
    let results = crate::search_in_document(&doc, "hello", false);
    assert_eq!(results.len(), 2);
}

#[test]
fn test_search_no_results() {
    let doc = make_doc_with_text("Hello World");
    let results = crate::search_in_document(&doc, "missing", false);
    assert_eq!(results.len(), 0);
}

// ============================================================
// Validation test
// ============================================================

#[test]
fn test_validate_empty_body() {
    let mut doc = Document::new();
    doc.body = vec![];
    let json = serde_json::to_string(&doc).unwrap();
    let issues = crate::validate_doc_json(&json);
    assert!(issues.iter().any(|i| i.message.contains("empty")));
}

#[test]
fn test_validate_invalid_json() {
    let issues = crate::validate_doc_json("not json");
    assert!(issues.iter().any(|i| i.severity == "error"));
}

#[test]
fn test_validate_valid_document() {
    let doc = make_doc_with_text("Valid doc");
    let json = serde_json::to_string(&doc).unwrap();
    let issues = crate::validate_doc_json(&json);
    // Should have no errors (may have empty ID warnings depending on UUID)
    assert!(issues.iter().all(|i| i.severity != "error"));
}

// ============================================================
// Helper functions
// ============================================================

fn make_para(text: &str) -> BlockElement {
    BlockElement::Paragraph(Paragraph {
        id: uuid::Uuid::new_v4().to_string(),
        runs: vec![Run {
            id: uuid::Uuid::new_v4().to_string(),
            text: text.to_string(),
            properties: RunProperties::default(),
        }],
        properties: ParagraphProperties::default(),
        bookmarks: vec![],
    })
}

fn make_doc_with_text(text: &str) -> Document {
    let mut doc = Document::new();
    doc.body = vec![make_para(text)];
    doc
}
