use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;

use super::ParseError;

/// A parsed relationship entry: relationship-id -> target path.
pub type RelsMap = HashMap<String, String>;

/// Parse an OPC relationships file (e.g. `_rels/.rels` or
/// `word/_rels/document.xml.rels`) and return a map from relationship ID to
/// target path.
///
/// Each `<Relationship>` element has attributes `Id`, `Type`, and `Target`.
/// We store `Id -> Target`.
pub fn parse_rels(xml: &str) -> Result<RelsMap, ParseError> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut map = HashMap::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                if e.local_name().as_ref() == b"Relationship" {
                    let mut id = None;
                    let mut target = None;

                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"Id" => {
                                id = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                            b"Target" => {
                                target = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                            _ => {}
                        }
                    }

                    if let (Some(id), Some(target)) = (id, target) {
                        map.insert(id, target);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(map)
}

/// Convenience: parse the package-level relationships (`_rels/.rels`) and the
/// word-level relationships (`word/_rels/document.xml.rels`), merging into a
/// single map.  For word-level relationships the target paths are normalized to
/// include the `word/` prefix when they are relative.
pub fn parse_all_rels(
    files: &HashMap<String, Vec<u8>>,
) -> Result<RelsMap, ParseError> {
    let mut combined = RelsMap::new();

    // Package-level rels
    if let Some(data) = files.get("_rels/.rels") {
        let xml = String::from_utf8_lossy(data);
        let rels = parse_rels(&xml)?;
        combined.extend(rels);
    }

    // Word-level rels (the main one for images, headers, footers, etc.)
    if let Some(data) = files.get("word/_rels/document.xml.rels") {
        let xml = String::from_utf8_lossy(data);
        let rels = parse_rels(&xml)?;

        for (id, target) in rels {
            // Normalize relative targets to include the word/ prefix
            let normalized = if target.starts_with('/') || target.starts_with("http") {
                target
            } else {
                format!("word/{}", target)
            };
            combined.insert(id, normalized);
        }
    }

    Ok(combined)
}
