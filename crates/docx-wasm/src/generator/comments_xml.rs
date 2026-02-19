use std::collections::HashMap;
use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::model::Comment;
use super::document_xml::{HyperlinkRidMap, ImageRidMap};
use super::GenerateError;

const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// Generate `word/comments.xml` from a list of comments.
///
/// Each comment has an id, author, optional date, optional initials, and a
/// list of block elements as content. The generated XML wraps each comment
/// in `<w:comment>` tags within the `<w:comments>` root.
pub fn generate_comments_xml(comments: &[Comment]) -> Result<Vec<u8>, GenerateError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let mut root = BytesStart::new("w:comments");
    root.push_attribute(("xmlns:w", W_NS));
    root.push_attribute(("xmlns:r", R_NS));
    writer.write_event(Event::Start(root))?;

    // Empty maps -- comments don't typically reference images or hyperlinks
    // but we need to satisfy the block_element writer signature.
    let image_rid_map: ImageRidMap = HashMap::new();
    let hyperlink_rid_map: HyperlinkRidMap = HashMap::new();

    for comment in comments {
        write_comment(&mut writer, comment, &image_rid_map, &hyperlink_rid_map)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:comments")))?;

    Ok(writer.into_inner().into_inner())
}

fn write_comment(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    comment: &Comment,
    image_rid_map: &ImageRidMap,
    hyperlink_rid_map: &HyperlinkRidMap,
) -> Result<(), GenerateError> {
    let mut elem = BytesStart::new("w:comment");
    elem.push_attribute(("w:id", comment.id.as_str()));
    elem.push_attribute(("w:author", comment.author.as_str()));
    if let Some(date) = &comment.date {
        elem.push_attribute(("w:date", date.as_str()));
    }
    if let Some(initials) = &comment.initials {
        elem.push_attribute(("w:initials", initials.as_str()));
    }
    writer.write_event(Event::Start(elem))?;

    if comment.content.is_empty() {
        // A comment must contain at least one paragraph
        writer.write_event(Event::Start(BytesStart::new("w:p")))?;
        writer.write_event(Event::End(BytesEnd::new("w:p")))?;
    } else {
        for block in &comment.content {
            super::document_xml::write_block_element(writer, block, image_rid_map, hyperlink_rid_map)?;
        }
    }

    writer.write_event(Event::End(BytesEnd::new("w:comment")))?;
    Ok(())
}
