use quick_xml::events::Event;
use quick_xml::Reader;

use crate::model::{ListDef, ListLevel};
use super::ParseError;

/// Parse `word/numbering.xml` into a list of numbering definitions.
///
/// The OOXML numbering model has two parts:
/// 1. `<w:abstractNum>` defines the pattern (levels, formats, text templates).
/// 2. `<w:num>` maps a `numId` to an `abstractNumId`.
///
/// We flatten these into `ListDef` entries keyed by `numId`, each carrying the
/// levels inherited from the referenced abstract numbering definition.
pub fn parse_numbering(xml: &str) -> Result<Vec<ListDef>, ParseError> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    // First pass state: collect abstract numbering definitions
    let mut abstract_defs: Vec<AbstractNumDef> = Vec::new();
    let mut current_abstract: Option<AbstractNumDef> = None;
    let mut current_level: Option<LevelBuilder> = None;
    let mut in_lvl = false;

    // Second pass state: collect num -> abstractNumId mappings
    let mut num_mappings: Vec<(String, String)> = Vec::new();
    let mut current_num_id: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"abstractNum" => {
                        let mut def = AbstractNumDef::default();
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"abstractNumId" {
                                def.abstract_num_id =
                                    String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        current_abstract = Some(def);
                    }
                    b"lvl" if current_abstract.is_some() => {
                        in_lvl = true;
                        let mut lb = LevelBuilder::default();
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"ilvl" {
                                lb.level = String::from_utf8_lossy(&attr.value)
                                    .parse::<u32>()
                                    .unwrap_or(0);
                            }
                        }
                        current_level = Some(lb);
                    }
                    b"num" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"numId" {
                                current_num_id = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }

            Ok(Event::Empty(ref e)) => {
                let local = e.local_name();

                if in_lvl {
                    if let Some(ref mut lb) = current_level {
                        handle_level_property(e, lb);
                    }
                }

                // <w:abstractNumId w:val="..."/> inside <w:num>
                if local.as_ref() == b"abstractNumId" {
                    if let Some(ref num_id) = current_num_id {
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"val" {
                                let abstract_id =
                                    String::from_utf8_lossy(&attr.value).to_string();
                                num_mappings.push((num_id.clone(), abstract_id));
                            }
                        }
                    }
                }
            }

            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"lvl" => {
                        in_lvl = false;
                        if let (Some(ref mut abs), Some(lb)) =
                            (&mut current_abstract, current_level.take())
                        {
                            abs.levels.push(lb.build());
                        }
                    }
                    b"abstractNum" => {
                        if let Some(abs) = current_abstract.take() {
                            abstract_defs.push(abs);
                        }
                    }
                    b"num" => {
                        current_num_id = None;
                    }
                    _ => {}
                }
            }

            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }

    // Build the final ListDef list by resolving num -> abstractNum
    let list_defs: Vec<ListDef> = num_mappings
        .into_iter()
        .filter_map(|(num_id, abstract_num_id)| {
            let abs = abstract_defs
                .iter()
                .find(|a| a.abstract_num_id == abstract_num_id)?;
            Some(ListDef {
                num_id,
                levels: abs.levels.clone(),
            })
        })
        .collect();

    Ok(list_defs)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[derive(Default)]
struct AbstractNumDef {
    abstract_num_id: String,
    levels: Vec<ListLevel>,
}

#[derive(Default)]
struct LevelBuilder {
    level: u32,
    start: u32,
    num_fmt: String,
    text: String,
}

impl LevelBuilder {
    fn build(self) -> ListLevel {
        ListLevel {
            level: self.level,
            start: self.start,
            format: self.num_fmt,
            text: self.text,
        }
    }
}

fn handle_level_property(
    e: &quick_xml::events::BytesStart<'_>,
    lb: &mut LevelBuilder,
) {
    let local = e.local_name();
    match local.as_ref() {
        b"start" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    lb.start = String::from_utf8_lossy(&attr.value)
                        .parse::<u32>()
                        .unwrap_or(1);
                }
            }
        }
        b"numFmt" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    lb.num_fmt =
                        String::from_utf8_lossy(&attr.value).to_string();
                }
            }
        }
        b"lvlText" => {
            for attr in e.attributes().flatten() {
                if attr.key.local_name().as_ref() == b"val" {
                    lb.text =
                        String::from_utf8_lossy(&attr.value).to_string();
                }
            }
        }
        // lvlJc / jc: alignment info parsed but not stored (model omits it)
        _ => {}
    }
}
