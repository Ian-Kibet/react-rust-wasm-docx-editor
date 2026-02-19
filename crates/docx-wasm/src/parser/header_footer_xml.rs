use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;
use uuid::Uuid;

use crate::model::{
    BlockElement, Footer, Header, NumberingRef, Paragraph, ParagraphProperties,
    Run, RunProperties,
};
use super::rels_xml::RelsMap;
use super::styles_xml::parse_alignment;
use super::ParseError;

/// Parse a header XML file (e.g. `word/header1.xml`) into a `Header`.
pub fn parse_header(
    xml: &str,
    rels: &RelsMap,
    media_path_to_id: &HashMap<String, String>,
) -> Result<Header, ParseError> {
    let content = parse_header_footer_body(xml, rels, media_path_to_id)?;
    Ok(Header {
        id: Uuid::new_v4().to_string(),
        content,
    })
}

/// Parse a footer XML file (e.g. `word/footer1.xml`) into a `Footer`.
pub fn parse_footer(
    xml: &str,
    rels: &RelsMap,
    media_path_to_id: &HashMap<String, String>,
) -> Result<Footer, ParseError> {
    let content = parse_header_footer_body(xml, rels, media_path_to_id)?;
    Ok(Footer {
        id: Uuid::new_v4().to_string(),
        content,
    })
}

// ---------------------------------------------------------------------------
// Shared header/footer body parser
// ---------------------------------------------------------------------------

fn parse_header_footer_body(
    xml: &str,
    rels: &RelsMap,
    media_path_to_id: &HashMap<String, String>,
) -> Result<Vec<BlockElement>, ParseError> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    let mut elements: Vec<BlockElement> = Vec::new();

    let mut current_para: Option<ParaBuilder> = None;
    let mut current_run: Option<HfRunBuilder> = None;
    let mut in_ppr = false;
    let mut in_rpr = false;
    let mut in_text = false;
    let mut in_numpr = false;
    let mut in_drawing = false;
    let mut in_hyperlink = false;
    let mut hyperlink_url: Option<String> = None;
    let mut hyperlink_tooltip: Option<String> = None;
    let mut in_pbdr = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"p" => {
                        current_para = Some(ParaBuilder::new());
                    }
                    b"pPr" if current_para.is_some() => {
                        in_ppr = true;
                    }
                    b"numPr" if in_ppr => {
                        in_numpr = true;
                    }
                    b"pBdr" if in_ppr => {
                        in_pbdr = true;
                    }
                    b"hyperlink" if current_para.is_some() => {
                        in_hyperlink = true;
                        hyperlink_url = None;
                        hyperlink_tooltip = None;
                        for attr in e.attributes().flatten() {
                            match attr.key.local_name().as_ref() {
                                b"id" => {
                                    let rel_id = String::from_utf8_lossy(&attr.value).to_string();
                                    if let Some(target) = rels.get(&rel_id) {
                                        hyperlink_url = Some(target.clone());
                                    }
                                }
                                b"tooltip" => {
                                    hyperlink_tooltip = Some(
                                        String::from_utf8_lossy(&attr.value).to_string(),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    b"r" if current_para.is_some() => {
                        let mut rb = HfRunBuilder::new();
                        if in_hyperlink {
                            rb.properties.hyperlink_url = hyperlink_url.clone();
                            rb.properties.hyperlink_tooltip = hyperlink_tooltip.clone();
                        }
                        current_run = Some(rb);
                    }
                    b"rPr" if current_run.is_some() => {
                        in_rpr = true;
                    }
                    b"t" if current_run.is_some() => {
                        in_text = true;
                    }
                    b"drawing" if current_run.is_some() => {
                        in_drawing = true;
                    }
                    _ => {}
                }
            }

            Ok(Event::Empty(ref e)) => {
                let local = e.local_name();

                // Run properties
                if in_rpr {
                    if let Some(ref mut run) = current_run {
                        handle_run_property(e, &mut run.properties);
                    }
                }

                // Paragraph properties
                if in_ppr && !in_numpr && !in_pbdr {
                    if let Some(ref mut para) = current_para {
                        handle_para_property(e, &mut para.properties);
                    }
                }

                // Paragraph borders
                if in_pbdr {
                    if let Some(ref mut para) = current_para {
                        handle_para_border(e, &mut para.properties);
                    }
                }

                // Numbering reference
                if in_numpr {
                    if let Some(ref mut para) = current_para {
                        handle_numpr_property(e, &mut para.properties);
                    }
                }

                // Line/page break and tab inside a run
                if current_run.is_some() && !in_rpr {
                    match local.as_ref() {
                        b"br" => {
                            if let Some(ref mut run) = current_run {
                                let mut is_page = false;
                                for attr in e.attributes().flatten() {
                                    if attr.key.local_name().as_ref() == b"type" {
                                        let val = String::from_utf8_lossy(&attr.value);
                                        if val == "page" {
                                            is_page = true;
                                        }
                                    }
                                }
                                if is_page {
                                    run.properties.page_break = Some(true);
                                } else {
                                    run.properties.line_break = Some(true);
                                }
                            }
                        }
                        b"tab" => {
                            if let Some(ref mut run) = current_run {
                                run.properties.tab = Some(true);
                                run.text.push('\t');
                            }
                        }
                        _ => {}
                    }
                }

                // Image via drawing
                if in_drawing && local.as_ref() == b"blip" {
                    if let Some(ref mut run) = current_run {
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"embed" {
                                let rel_id =
                                    String::from_utf8_lossy(&attr.value).to_string();
                                if let Some(target) = rels.get(&rel_id) {
                                    if let Some(image_id) =
                                        media_path_to_id.get(target)
                                    {
                                        run.properties.inline_image =
                                            Some(image_id.clone());
                                    }
                                }
                            }
                        }
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
                    b"p" => {
                        flush_hf_run(&mut current_run, &mut current_para);
                        if let Some(pb) = current_para.take() {
                            elements.push(BlockElement::Paragraph(pb.build()));
                        }
                    }
                    b"r" => {
                        flush_hf_run(&mut current_run, &mut current_para);
                    }
                    b"hyperlink" => {
                        in_hyperlink = false;
                        hyperlink_url = None;
                        hyperlink_tooltip = None;
                    }
                    b"t" => in_text = false,
                    b"pPr" => in_ppr = false,
                    b"rPr" => in_rpr = false,
                    b"numPr" => in_numpr = false,
                    b"pBdr" => in_pbdr = false,
                    b"drawing" => in_drawing = false,
                    _ => {}
                }
            }

            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(elements)
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

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

struct HfRunBuilder {
    id: String,
    properties: RunProperties,
    text: String,
}

impl HfRunBuilder {
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

fn flush_hf_run(
    current_run: &mut Option<HfRunBuilder>,
    current_para: &mut Option<ParaBuilder>,
) {
    if let Some(run_builder) = current_run.take() {
        let run = run_builder.build();
        if !run.text.is_empty()
            || run.properties.inline_image.is_some()
            || run.properties.line_break == Some(true)
            || run.properties.page_break == Some(true)
            || run.properties.tab == Some(true)
        {
            if let Some(ref mut para) = current_para {
                para.runs.push(run);
            }
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
        b"dstrike" => props.double_strikethrough = Some(!is_val_false(e)),
        b"smallCaps" => props.small_caps = Some(!is_val_false(e)),
        b"caps" => props.all_caps = Some(!is_val_false(e)),
        b"vertAlign" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    let val = String::from_utf8_lossy(&attr.value);
                    match val.as_ref() {
                        "superscript" => props.superscript = Some(true),
                        "subscript" => props.subscript = Some(true),
                        _ => {}
                    }
                }
            }
        }
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
        b"shd" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"fill" {
                    let fill = String::from_utf8_lossy(&attr.value).to_string();
                    if fill != "auto" {
                        props.background_color = Some(fill);
                    }
                }
            }
        }
        b"spacing" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    if let Ok(v) = String::from_utf8_lossy(&attr.value).parse::<f64>() {
                        props.spacing = Some(v);
                    }
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
                    props.alignment =
                        Some(parse_alignment(&String::from_utf8_lossy(&attr.value)));
                }
            }
        }
        b"pStyle" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    props.style_id = Some(val.clone());
                    if let Some(level) = heading_level_from_style(&val) {
                        props.heading_level = Some(level);
                    }
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
                    b"line" => {
                        if let Ok(v) =
                            String::from_utf8_lossy(&attr.value).parse::<f64>()
                        {
                            props.line_spacing = Some(v);
                        }
                    }
                    b"lineRule" => {
                        let val = String::from_utf8_lossy(&attr.value);
                        props.line_spacing_rule = Some(parse_line_spacing_rule(&val));
                    }
                    _ => {}
                }
            }
        }
        b"ind" => {
            for attr in e.attributes().flatten() {
                match attr.key.local_name().as_ref() {
                    b"left" | b"start" => {
                        if let Ok(v) =
                            String::from_utf8_lossy(&attr.value).parse::<f64>()
                        {
                            props.indent_left = Some(v);
                        }
                    }
                    b"right" | b"end" => {
                        if let Ok(v) =
                            String::from_utf8_lossy(&attr.value).parse::<f64>()
                        {
                            props.indent_right = Some(v);
                        }
                    }
                    b"firstLine" => {
                        if let Ok(v) =
                            String::from_utf8_lossy(&attr.value).parse::<f64>()
                        {
                            props.indent_first_line = Some(v);
                        }
                    }
                    b"hanging" => {
                        if let Ok(v) =
                            String::from_utf8_lossy(&attr.value).parse::<f64>()
                        {
                            props.indent_hanging = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }
        b"pageBreakBefore" => {
            props.page_break_before = Some(!is_val_false(e));
        }
        b"keepNext" => {
            props.keep_next = Some(!is_val_false(e));
        }
        b"keepLines" => {
            props.keep_lines = Some(!is_val_false(e));
        }
        b"widowControl" => {
            props.widow_control = Some(!is_val_false(e));
        }
        b"shd" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"fill" {
                    let fill = String::from_utf8_lossy(&attr.value).to_string();
                    if fill != "auto" {
                        props.shading = Some(fill);
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_para_border(
    e: &quick_xml::events::BytesStart<'_>,
    props: &mut ParagraphProperties,
) {
    let local = e.local_name();
    let border = parse_border(e);
    match local.as_ref() {
        b"bottom" => props.border_bottom = Some(border),
        b"top" => props.border_top = Some(border),
        _ => {}
    }
}

fn handle_numpr_property(
    e: &quick_xml::events::BytesStart<'_>,
    props: &mut ParagraphProperties,
) {
    let local = e.local_name();
    match local.as_ref() {
        b"numId" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    if let Some(ref mut nr) = props.numbering {
                        nr.num_id = val;
                    } else {
                        props.numbering = Some(NumberingRef {
                            num_id: val,
                            level: 0,
                        });
                    }
                }
            }
        }
        b"ilvl" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    let level = String::from_utf8_lossy(&attr.value)
                        .parse::<u32>()
                        .unwrap_or(0);
                    if let Some(ref mut nr) = props.numbering {
                        nr.level = level;
                    } else {
                        props.numbering = Some(NumberingRef {
                            num_id: String::new(),
                            level,
                        });
                    }
                }
            }
        }
        _ => {}
    }
}

fn heading_level_from_style(style_id: &str) -> Option<u8> {
    let lower = style_id.to_lowercase();
    if lower.starts_with("heading") {
        let suffix = &lower["heading".len()..];
        suffix.trim().parse::<u8>().ok()
    } else {
        None
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

fn parse_border(e: &quick_xml::events::BytesStart<'_>) -> crate::model::Border {
    let mut border = crate::model::Border {
        style: String::new(),
        size: 0.0,
        color: String::new(),
    };
    for attr in e.attributes().flatten() {
        match attr.key.local_name().as_ref() {
            b"val" => border.style = String::from_utf8_lossy(&attr.value).to_string(),
            b"sz" => {
                if let Ok(sz) = String::from_utf8_lossy(&attr.value).parse::<f64>() {
                    border.size = sz;
                }
            }
            b"color" => {
                let c = String::from_utf8_lossy(&attr.value).to_string();
                if c != "auto" {
                    border.color = c;
                }
            }
            _ => {}
        }
    }
    border
}

fn parse_line_spacing_rule(val: &str) -> crate::model::LineSpacingRule {
    match val {
        "exact" => crate::model::LineSpacingRule::Exact,
        "atLeast" => crate::model::LineSpacingRule::AtLeast,
        _ => crate::model::LineSpacingRule::Auto,
    }
}
