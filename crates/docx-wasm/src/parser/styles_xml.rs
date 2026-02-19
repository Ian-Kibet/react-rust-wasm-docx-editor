use quick_xml::events::Event;
use quick_xml::Reader;

use crate::model::{
    Alignment, ParagraphProperties, RunProperties, StyleDef,
};
use super::ParseError;

/// Parse `word/styles.xml` into a vector of style definitions.
///
/// Each `<w:style>` element is converted into a `StyleDef`. We capture the
/// style id, name, type, basedOn reference, and any embedded paragraph or run
/// properties.
pub fn parse_styles(xml: &str) -> Result<Vec<StyleDef>, ParseError> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut styles: Vec<StyleDef> = Vec::new();

    // Parsing state
    let mut current_style: Option<StyleBuilder> = None;
    let mut in_name = false;
    let mut in_rpr = false;
    let mut in_ppr = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"style" => {
                        let mut builder = StyleBuilder::default();
                        for attr in e.attributes().flatten() {
                            match attr.key.local_name().as_ref() {
                                b"styleId" => {
                                    builder.id =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                }
                                b"type" => {
                                    builder.style_type =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                }
                                b"default" => {
                                    // w:default="1" means this is the default style for its type
                                    let val = String::from_utf8_lossy(&attr.value);
                                    builder.is_default = val == "1" || val == "true";
                                }
                                _ => {}
                            }
                        }
                        current_style = Some(builder);
                    }
                    b"name" if current_style.is_some() => {
                        in_name = true;
                        // The name is typically in the val attribute
                        if let Some(ref mut s) = current_style {
                            for attr in e.attributes().flatten() {
                                if attr.key.local_name().as_ref() == b"val" {
                                    s.name =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                }
                            }
                        }
                    }
                    b"rPr" if current_style.is_some() => {
                        in_rpr = true;
                    }
                    b"pPr" if current_style.is_some() => {
                        in_ppr = true;
                    }
                    _ => {
                        if current_style.is_some() {
                            handle_property_element(
                                e, &mut current_style, in_rpr, in_ppr,
                            );
                        }
                    }
                }
            }

            Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                if current_style.is_some() {
                    match local.as_ref() {
                        b"name" => {
                            if let Some(ref mut s) = current_style {
                                for attr in e.attributes().flatten() {
                                    if attr.key.local_name().as_ref() == b"val" {
                                        s.name = String::from_utf8_lossy(&attr.value)
                                            .to_string();
                                    }
                                }
                            }
                        }
                        b"basedOn" => {
                            if let Some(ref mut s) = current_style {
                                for attr in e.attributes().flatten() {
                                    if attr.key.local_name().as_ref() == b"val" {
                                        s.based_on = Some(
                                            String::from_utf8_lossy(&attr.value)
                                                .to_string(),
                                        );
                                    }
                                }
                            }
                        }
                        _ => {
                            handle_property_element(
                                e, &mut current_style, in_rpr, in_ppr,
                            );
                        }
                    }
                }
            }

            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"style" => {
                        if let Some(builder) = current_style.take() {
                            styles.push(builder.build());
                        }
                        in_rpr = false;
                        in_ppr = false;
                    }
                    b"name" => in_name = false,
                    b"rPr" => in_rpr = false,
                    b"pPr" => in_ppr = false,
                    _ => {}
                }
            }

            Ok(Event::Text(ref t)) => {
                if in_name {
                    if let Some(ref mut s) = current_style {
                        if s.name.is_empty() {
                            s.name = t.unescape().unwrap_or_default().to_string();
                        }
                    }
                }
            }

            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(styles)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[derive(Default)]
struct StyleBuilder {
    id: String,
    name: String,
    style_type: String,
    based_on: Option<String>,
    is_default: bool,
    rpr: RunProperties,
    ppr: ParagraphProperties,
    has_rpr: bool,
    has_ppr: bool,
}

impl StyleBuilder {
    fn build(self) -> StyleDef {
        StyleDef {
            id: self.id,
            name: self.name,
            style_type: self.style_type,
            based_on: self.based_on,
            run_properties: if self.has_rpr {
                Some(self.rpr)
            } else {
                None
            },
            paragraph_properties: if self.has_ppr {
                Some(self.ppr)
            } else {
                None
            },
        }
    }
}

/// Handle property child elements that may appear inside either `<w:rPr>` or
/// `<w:pPr>`, or directly under `<w:style>` as self-closing shorthand.
fn handle_property_element(
    e: &quick_xml::events::BytesStart<'_>,
    current_style: &mut Option<StyleBuilder>,
    in_rpr: bool,
    in_ppr: bool,
) {
    let Some(ref mut s) = current_style else {
        return;
    };

    let local = e.local_name();

    if in_rpr {
        s.has_rpr = true;
        match local.as_ref() {
            b"b" => s.rpr.bold = Some(true),
            b"i" => s.rpr.italic = Some(true),
            b"u" => s.rpr.underline = Some(true),
            b"strike" => s.rpr.strikethrough = Some(true),
            b"sz" => {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"val" {
                        if let Ok(half_pts) =
                            String::from_utf8_lossy(&attr.value).parse::<f64>()
                        {
                            s.rpr.font_size = Some(half_pts / 2.0);
                        }
                    }
                }
            }
            b"color" => {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"val" {
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        if val != "auto" {
                            s.rpr.color = Some(val);
                        }
                    }
                }
            }
            b"rFonts" => {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"ascii" {
                        s.rpr.font_family =
                            Some(String::from_utf8_lossy(&attr.value).to_string());
                    }
                }
            }
            b"highlight" => {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"val" {
                        s.rpr.highlight =
                            Some(String::from_utf8_lossy(&attr.value).to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if in_ppr {
        s.has_ppr = true;
        match local.as_ref() {
            b"jc" => {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"val" {
                        s.ppr.alignment =
                            Some(parse_alignment(&String::from_utf8_lossy(&attr.value)));
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
                                s.ppr.spacing_before = Some(v);
                            }
                        }
                        b"after" => {
                            if let Ok(v) =
                                String::from_utf8_lossy(&attr.value).parse::<f64>()
                            {
                                s.ppr.spacing_after = Some(v);
                            }
                        }
                        _ => {}
                    }
                }
            }
            b"ind" => {
                for attr in e.attributes().flatten() {
                    match attr.key.local_name().as_ref() {
                        b"left" => {
                            if let Ok(v) =
                                String::from_utf8_lossy(&attr.value).parse::<f64>()
                            {
                                s.ppr.indent_left = Some(v);
                            }
                        }
                        b"right" => {
                            if let Ok(v) =
                                String::from_utf8_lossy(&attr.value).parse::<f64>()
                            {
                                s.ppr.indent_right = Some(v);
                            }
                        }
                        b"firstLine" => {
                            if let Ok(v) =
                                String::from_utf8_lossy(&attr.value).parse::<f64>()
                            {
                                s.ppr.indent_first_line = Some(v);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    // basedOn can appear as a direct child of <w:style>
    if !in_rpr && !in_ppr && local.as_ref() == b"basedOn" {
        for attr in e.attributes().flatten() {
            if attr.key.local_name().as_ref() == b"val" {
                s.based_on =
                    Some(String::from_utf8_lossy(&attr.value).to_string());
            }
        }
    }
}

/// Map an OOXML alignment value string to our `Alignment` enum.
pub(crate) fn parse_alignment(val: &str) -> Alignment {
    match val {
        "center" => Alignment::Center,
        "right" | "end" => Alignment::Right,
        "both" | "justify" => Alignment::Justify,
        _ => Alignment::Left,
    }
}
