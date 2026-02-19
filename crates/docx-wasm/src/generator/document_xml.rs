use std::collections::HashMap;
use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use crate::model::{
    Alignment, BlockElement, Border, Document, LineSpacingRule, PageOrientation, Paragraph,
    ParagraphProperties, Run, RunProperties, SectionProperties, TabStop, TabStopAlignment, Table,
    TableBorders, TableCell, TableCellProperties, TableLayout, TableProperties, TableRow,
    TableRowProperties, VerticalMerge,
};
use super::GenerateError;

const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const WP_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const PIC_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/picture";

/// Maps image IDs to their relationship IDs so inline images can reference
/// the correct `r:embed`.
pub type ImageRidMap = HashMap<String, String>;

/// Maps hyperlink URLs to their relationship IDs so hyperlink runs can
/// reference the correct `r:id`.
pub type HyperlinkRidMap = HashMap<String, String>;

/// Generate `word/document.xml` from the full document model.
///
/// `image_rid_map` maps `ImageData::id` to a relationship ID (e.g. `rId7`).
/// `hyperlink_rid_map` maps hyperlink URL to a relationship ID (e.g. `rId10`).
/// `header_rids` and `footer_rids` map header/footer index to rIds for the
/// `<w:sectPr>` section properties.
pub fn generate_document_xml(
    doc: &Document,
    image_rid_map: &ImageRidMap,
    hyperlink_rid_map: &HyperlinkRidMap,
    header_rids: &[String],
    footer_rids: &[String],
) -> Result<Vec<u8>, GenerateError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let mut root = BytesStart::new("w:document");
    root.push_attribute(("xmlns:w", W_NS));
    root.push_attribute(("xmlns:r", R_NS));
    root.push_attribute(("xmlns:wp", WP_NS));
    root.push_attribute(("xmlns:a", A_NS));
    root.push_attribute(("xmlns:pic", PIC_NS));
    writer.write_event(Event::Start(root))?;

    writer.write_event(Event::Start(BytesStart::new("w:body")))?;

    for block in &doc.body {
        write_block_element(&mut writer, block, image_rid_map, hyperlink_rid_map)?;
    }

    // Section properties (page setup, header/footer references)
    write_section_properties(&mut writer, header_rids, footer_rids, doc.section_properties.as_ref())?;

    writer.write_event(Event::End(BytesEnd::new("w:body")))?;
    writer.write_event(Event::End(BytesEnd::new("w:document")))?;

    Ok(writer.into_inner().into_inner())
}

/// Write a block element -- either a paragraph or a table.
pub fn write_block_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    block: &BlockElement,
    image_rid_map: &ImageRidMap,
    hyperlink_rid_map: &HyperlinkRidMap,
) -> Result<(), GenerateError> {
    match block {
        BlockElement::Paragraph(para) => write_paragraph(writer, para, image_rid_map, hyperlink_rid_map),
        BlockElement::Table(table) => write_table(writer, table, image_rid_map, hyperlink_rid_map),
    }
}

fn write_paragraph(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    para: &Paragraph,
    image_rid_map: &ImageRidMap,
    hyperlink_rid_map: &HyperlinkRidMap,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:p")))?;

    // Paragraph properties
    write_paragraph_properties(writer, &para.properties)?;

    // Bookmarks start
    for bookmark in &para.bookmarks {
        let mut bs = BytesStart::new("w:bookmarkStart");
        bs.push_attribute(("w:id", bookmark.id.as_str()));
        bs.push_attribute(("w:name", bookmark.name.as_str()));
        writer.write_event(Event::Empty(bs))?;
    }

    // Runs
    for run in &para.runs {
        write_run(writer, run, image_rid_map, hyperlink_rid_map)?;
    }

    // Bookmarks end
    for bookmark in &para.bookmarks {
        let mut be = BytesStart::new("w:bookmarkEnd");
        be.push_attribute(("w:id", bookmark.id.as_str()));
        writer.write_event(Event::Empty(be))?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:p")))?;
    Ok(())
}

fn write_paragraph_properties(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    props: &ParagraphProperties,
) -> Result<(), GenerateError> {
    // Only emit <w:pPr> if there is at least one property to write
    let has_props = props.alignment.is_some()
        || props.heading_level.is_some()
        || props.numbering.is_some()
        || props.spacing_before.is_some()
        || props.spacing_after.is_some()
        || props.line_spacing.is_some()
        || props.indent_left.is_some()
        || props.indent_right.is_some()
        || props.indent_first_line.is_some()
        || props.indent_hanging.is_some()
        || props.page_break_before == Some(true)
        || props.keep_next == Some(true)
        || props.keep_lines == Some(true)
        || props.widow_control == Some(true)
        || props.style_id.is_some()
        || props.border_bottom.is_some()
        || props.border_top.is_some()
        || props.shading.is_some()
        || !props.tab_stops.is_empty();

    if !has_props {
        return Ok(());
    }

    writer.write_event(Event::Start(BytesStart::new("w:pPr")))?;

    // Paragraph style reference
    if let Some(style_id) = &props.style_id {
        let mut ps = BytesStart::new("w:pStyle");
        ps.push_attribute(("w:val", style_id.as_str()));
        writer.write_event(Event::Empty(ps))?;
    }

    // Heading style reference (only if no explicit style_id is set)
    if props.style_id.is_none() {
        if let Some(level) = props.heading_level {
            let style_id = format!("Heading{level}");
            let mut ps = BytesStart::new("w:pStyle");
            ps.push_attribute(("w:val", style_id.as_str()));
            writer.write_event(Event::Empty(ps))?;
        }
    }

    // Keep next
    if props.keep_next == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:keepNext")))?;
    }

    // Keep lines
    if props.keep_lines == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:keepLines")))?;
    }

    // Page break before
    if props.page_break_before == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:pageBreakBefore")))?;
    }

    // Widow control
    if props.widow_control == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:widowControl")))?;
    }

    // Tab stops
    if !props.tab_stops.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("w:tabs")))?;
        for tab_stop in &props.tab_stops {
            write_tab_stop(writer, tab_stop)?;
        }
        writer.write_event(Event::End(BytesEnd::new("w:tabs")))?;
    }

    // Alignment
    if let Some(alignment) = &props.alignment {
        let val = alignment_value(alignment);
        let mut jc = BytesStart::new("w:jc");
        jc.push_attribute(("w:val", val));
        writer.write_event(Event::Empty(jc))?;
    }

    // Numbering
    if let Some(num_ref) = &props.numbering {
        writer.write_event(Event::Start(BytesStart::new("w:numPr")))?;

        let mut ilvl = BytesStart::new("w:ilvl");
        ilvl.push_attribute(("w:val", num_ref.level.to_string().as_str()));
        writer.write_event(Event::Empty(ilvl))?;

        let mut num_id = BytesStart::new("w:numId");
        num_id.push_attribute(("w:val", num_ref.num_id.as_str()));
        writer.write_event(Event::Empty(num_id))?;

        writer.write_event(Event::End(BytesEnd::new("w:numPr")))?;
    }

    // Paragraph borders
    if props.border_top.is_some() || props.border_bottom.is_some() {
        writer.write_event(Event::Start(BytesStart::new("w:pBdr")))?;
        if let Some(b) = &props.border_top {
            write_border_element(writer, "w:top", b)?;
        }
        if let Some(b) = &props.border_bottom {
            write_border_element(writer, "w:bottom", b)?;
        }
        writer.write_event(Event::End(BytesEnd::new("w:pBdr")))?;
    }

    // Shading
    if let Some(shading) = &props.shading {
        let color = shading.trim_start_matches('#');
        let mut shd = BytesStart::new("w:shd");
        shd.push_attribute(("w:val", "clear"));
        shd.push_attribute(("w:color", "auto"));
        shd.push_attribute(("w:fill", color));
        writer.write_event(Event::Empty(shd))?;
    }

    // Spacing (before, after, and line spacing)
    if props.spacing_before.is_some() || props.spacing_after.is_some() || props.line_spacing.is_some() {
        let mut spacing = BytesStart::new("w:spacing");
        if let Some(before) = props.spacing_before {
            spacing.push_attribute(("w:before", (before as u32).to_string().as_str()));
        }
        if let Some(after) = props.spacing_after {
            spacing.push_attribute(("w:after", (after as u32).to_string().as_str()));
        }
        if let Some(line) = props.line_spacing {
            spacing.push_attribute(("w:line", (line as u32).to_string().as_str()));
            let rule_val = match &props.line_spacing_rule {
                Some(LineSpacingRule::Exact) => "exact",
                Some(LineSpacingRule::AtLeast) => "atLeast",
                Some(LineSpacingRule::Auto) | None => "auto",
            };
            spacing.push_attribute(("w:lineRule", rule_val));
        }
        writer.write_event(Event::Empty(spacing))?;
    }

    // Indentation
    if props.indent_left.is_some()
        || props.indent_right.is_some()
        || props.indent_first_line.is_some()
        || props.indent_hanging.is_some()
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
        if let Some(hanging) = props.indent_hanging {
            ind.push_attribute(("w:hanging", (hanging as i32).to_string().as_str()));
        }
        writer.write_event(Event::Empty(ind))?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:pPr")))?;
    Ok(())
}

fn write_tab_stop(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    tab_stop: &TabStop,
) -> Result<(), GenerateError> {
    let mut tab = BytesStart::new("w:tab");
    let val = match &tab_stop.alignment {
        TabStopAlignment::Left => "left",
        TabStopAlignment::Center => "center",
        TabStopAlignment::Right => "right",
        TabStopAlignment::Decimal => "decimal",
    };
    tab.push_attribute(("w:val", val));
    tab.push_attribute(("w:pos", (tab_stop.position as i32).to_string().as_str()));
    if let Some(leader) = &tab_stop.leader {
        tab.push_attribute(("w:leader", leader.as_str()));
    }
    writer.write_event(Event::Empty(tab))?;
    Ok(())
}

fn write_run(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    run: &Run,
    image_rid_map: &ImageRidMap,
    hyperlink_rid_map: &HyperlinkRidMap,
) -> Result<(), GenerateError> {
    // If this run has a hyperlink URL, wrap in <w:hyperlink>
    let in_hyperlink = run.properties.hyperlink_url.is_some();
    if in_hyperlink {
        if let Some(url) = &run.properties.hyperlink_url {
            if let Some(rid) = hyperlink_rid_map.get(url.as_str()) {
                let mut hl = BytesStart::new("w:hyperlink");
                hl.push_attribute(("r:id", rid.as_str()));
                if let Some(tooltip) = &run.properties.hyperlink_tooltip {
                    hl.push_attribute(("w:tooltip", tooltip.as_str()));
                }
                writer.write_event(Event::Start(hl))?;
            }
        }
    }

    writer.write_event(Event::Start(BytesStart::new("w:r")))?;

    // Run properties
    write_run_properties(writer, &run.properties)?;

    // Determine what content to write
    if run.properties.page_break == Some(true) {
        // Page break
        let mut br = BytesStart::new("w:br");
        br.push_attribute(("w:type", "page"));
        writer.write_event(Event::Empty(br))?;
    } else if run.properties.line_break == Some(true) {
        // Line break
        writer.write_event(Event::Empty(BytesStart::new("w:br")))?;
    } else if run.properties.tab == Some(true) {
        // Tab character
        writer.write_event(Event::Empty(BytesStart::new("w:tab")))?;
    } else if let Some(footnote_id) = &run.properties.footnote_ref {
        // Footnote reference
        let mut fnref = BytesStart::new("w:footnoteReference");
        fnref.push_attribute(("w:id", footnote_id.as_str()));
        writer.write_event(Event::Empty(fnref))?;
    } else if let Some(endnote_id) = &run.properties.endnote_ref {
        // Endnote reference
        let mut enref = BytesStart::new("w:endnoteReference");
        enref.push_attribute(("w:id", endnote_id.as_str()));
        writer.write_event(Event::Empty(enref))?;
    } else if let Some(img_id) = &run.properties.inline_image {
        // Inline image
        if let Some(rid) = image_rid_map.get(img_id) {
            write_inline_image(writer, rid, &run.properties)?;
        }
    } else {
        // Text element
        let mut t = BytesStart::new("w:t");
        t.push_attribute(("xml:space", "preserve"));
        writer.write_event(Event::Start(t))?;
        writer.write_event(Event::Text(BytesText::new(&run.text)))?;
        writer.write_event(Event::End(BytesEnd::new("w:t")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:r")))?;

    // Close hyperlink wrapper if opened
    if in_hyperlink {
        if let Some(url) = &run.properties.hyperlink_url {
            if hyperlink_rid_map.contains_key(url.as_str()) {
                writer.write_event(Event::End(BytesEnd::new("w:hyperlink")))?;
            }
        }
    }

    Ok(())
}

fn write_run_properties(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    props: &RunProperties,
) -> Result<(), GenerateError> {
    let has_props = props.bold == Some(true)
        || props.italic == Some(true)
        || props.underline == Some(true)
        || props.strikethrough == Some(true)
        || props.double_strikethrough == Some(true)
        || props.superscript == Some(true)
        || props.subscript == Some(true)
        || props.small_caps == Some(true)
        || props.all_caps == Some(true)
        || props.font_family.is_some()
        || props.font_size.is_some()
        || props.color.is_some()
        || props.highlight.is_some()
        || props.background_color.is_some()
        || props.spacing.is_some()
        || props.footnote_ref.is_some()
        || props.endnote_ref.is_some();

    if !has_props {
        return Ok(());
    }

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
    if props.double_strikethrough == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:dstrike")))?;
    }
    if props.small_caps == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:smallCaps")))?;
    }
    if props.all_caps == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:caps")))?;
    }
    // Superscript / subscript via vertAlign
    if props.superscript == Some(true) {
        let mut va = BytesStart::new("w:vertAlign");
        va.push_attribute(("w:val", "superscript"));
        writer.write_event(Event::Empty(va))?;
    } else if props.subscript == Some(true) {
        let mut va = BytesStart::new("w:vertAlign");
        va.push_attribute(("w:val", "subscript"));
        writer.write_event(Event::Empty(va))?;
    }
    if let Some(font) = &props.font_family {
        let mut f = BytesStart::new("w:rFonts");
        f.push_attribute(("w:ascii", font.as_str()));
        f.push_attribute(("w:hAnsi", font.as_str()));
        writer.write_event(Event::Empty(f))?;
    }
    if let Some(size) = props.font_size {
        // OOXML font size is in half-points (multiply by 2)
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
    // Character spacing
    if let Some(spacing_val) = props.spacing {
        let mut sp = BytesStart::new("w:spacing");
        sp.push_attribute(("w:val", (spacing_val as i32).to_string().as_str()));
        writer.write_event(Event::Empty(sp))?;
    }
    // Run background (shading)
    if let Some(bg) = &props.background_color {
        let color = bg.trim_start_matches('#');
        let mut shd = BytesStart::new("w:shd");
        shd.push_attribute(("w:val", "clear"));
        shd.push_attribute(("w:color", "auto"));
        shd.push_attribute(("w:fill", color));
        writer.write_event(Event::Empty(shd))?;
    }
    // Footnote/endnote reference style
    if props.footnote_ref.is_some() || props.endnote_ref.is_some() {
        let mut rstyle = BytesStart::new("w:rStyle");
        rstyle.push_attribute(("w:val", "FootnoteReference"));
        writer.write_event(Event::Empty(rstyle))?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:rPr")))?;
    Ok(())
}

/// Write an inline image drawing element referencing the given relationship ID.
fn write_inline_image(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    rid: &str,
    props: &RunProperties,
) -> Result<(), GenerateError> {
    // Default image dimensions: 3 inches x 3 inches (in EMUs: 1 inch = 914400 EMUs)
    let default_emu = 914_400 * 3;
    // If the model has dimensions in points, convert: 1 pt = 12700 EMU
    let cx: u64 = props
        .font_size
        .map(|_| default_emu)
        .unwrap_or(default_emu);
    let cy = cx; // square by default; real dims come from ImageData if needed

    let cx_str = cx.to_string();
    let cy_str = cy.to_string();

    writer.write_event(Event::Start(BytesStart::new("w:drawing")))?;

    // wp:inline
    let mut inline = BytesStart::new("wp:inline");
    inline.push_attribute(("distT", "0"));
    inline.push_attribute(("distB", "0"));
    inline.push_attribute(("distL", "0"));
    inline.push_attribute(("distR", "0"));
    writer.write_event(Event::Start(inline))?;

    // wp:extent
    let mut extent = BytesStart::new("wp:extent");
    extent.push_attribute(("cx", cx_str.as_str()));
    extent.push_attribute(("cy", cy_str.as_str()));
    writer.write_event(Event::Empty(extent))?;

    // wp:docPr
    let mut doc_pr = BytesStart::new("wp:docPr");
    doc_pr.push_attribute(("id", "1"));
    doc_pr.push_attribute(("name", "Picture"));
    writer.write_event(Event::Empty(doc_pr))?;

    // a:graphic
    let mut graphic = BytesStart::new("a:graphic");
    graphic.push_attribute(("xmlns:a", A_NS));
    writer.write_event(Event::Start(graphic))?;

    let mut graphic_data = BytesStart::new("a:graphicData");
    graphic_data.push_attribute(("uri", PIC_NS));
    writer.write_event(Event::Start(graphic_data))?;

    // pic:pic
    let mut pic = BytesStart::new("pic:pic");
    pic.push_attribute(("xmlns:pic", PIC_NS));
    writer.write_event(Event::Start(pic))?;

    // pic:nvPicPr
    writer.write_event(Event::Start(BytesStart::new("pic:nvPicPr")))?;
    let mut c_nv_pr = BytesStart::new("pic:cNvPr");
    c_nv_pr.push_attribute(("id", "0"));
    c_nv_pr.push_attribute(("name", "Image"));
    writer.write_event(Event::Empty(c_nv_pr))?;
    writer.write_event(Event::Empty(BytesStart::new("pic:cNvPicPr")))?;
    writer.write_event(Event::End(BytesEnd::new("pic:nvPicPr")))?;

    // pic:blipFill
    writer.write_event(Event::Start(BytesStart::new("pic:blipFill")))?;
    let mut blip = BytesStart::new("a:blip");
    blip.push_attribute(("r:embed", rid));
    writer.write_event(Event::Empty(blip))?;
    writer.write_event(Event::Start(BytesStart::new("a:stretch")))?;
    writer.write_event(Event::Empty(BytesStart::new("a:fillRect")))?;
    writer.write_event(Event::End(BytesEnd::new("a:stretch")))?;
    writer.write_event(Event::End(BytesEnd::new("pic:blipFill")))?;

    // pic:spPr
    writer.write_event(Event::Start(BytesStart::new("pic:spPr")))?;
    let xfrm = BytesStart::new("a:xfrm");
    writer.write_event(Event::Start(xfrm))?;
    let mut off = BytesStart::new("a:off");
    off.push_attribute(("x", "0"));
    off.push_attribute(("y", "0"));
    writer.write_event(Event::Empty(off))?;
    let mut ext = BytesStart::new("a:ext");
    ext.push_attribute(("cx", cx_str.as_str()));
    ext.push_attribute(("cy", cy_str.as_str()));
    writer.write_event(Event::Empty(ext))?;
    writer.write_event(Event::End(BytesEnd::new("a:xfrm")))?;
    let mut prst = BytesStart::new("a:prstGeom");
    prst.push_attribute(("prst", "rect"));
    writer.write_event(Event::Start(prst))?;
    writer.write_event(Event::Empty(BytesStart::new("a:avLst")))?;
    writer.write_event(Event::End(BytesEnd::new("a:prstGeom")))?;
    writer.write_event(Event::End(BytesEnd::new("pic:spPr")))?;

    writer.write_event(Event::End(BytesEnd::new("pic:pic")))?;
    writer.write_event(Event::End(BytesEnd::new("a:graphicData")))?;
    writer.write_event(Event::End(BytesEnd::new("a:graphic")))?;
    writer.write_event(Event::End(BytesEnd::new("wp:inline")))?;
    writer.write_event(Event::End(BytesEnd::new("w:drawing")))?;

    Ok(())
}

// -----------------------------------------------------------------------
// Table writing
// -----------------------------------------------------------------------

fn write_table(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    table: &Table,
    image_rid_map: &ImageRidMap,
    hyperlink_rid_map: &HyperlinkRidMap,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:tbl")))?;

    write_table_properties(writer, &table.properties)?;

    for row in &table.rows {
        write_table_row(writer, row, image_rid_map, hyperlink_rid_map)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:tbl")))?;
    Ok(())
}

fn write_table_properties(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    props: &TableProperties,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:tblPr")))?;

    // Table style
    let mut style = BytesStart::new("w:tblStyle");
    style.push_attribute(("w:val", "TableGrid"));
    writer.write_event(Event::Empty(style))?;

    // Table width
    if let Some(width) = props.width {
        let mut w = BytesStart::new("w:tblW");
        w.push_attribute(("w:w", (width as u32).to_string().as_str()));
        w.push_attribute(("w:type", "dxa"));
        writer.write_event(Event::Empty(w))?;
    } else {
        let mut w = BytesStart::new("w:tblW");
        w.push_attribute(("w:w", "0"));
        w.push_attribute(("w:type", "auto"));
        writer.write_event(Event::Empty(w))?;
    }

    // Table layout
    if let Some(layout) = &props.layout {
        let layout_val = match layout {
            TableLayout::Fixed => "fixed",
            TableLayout::Autofit => "autofit",
        };
        let mut l = BytesStart::new("w:tblLayout");
        l.push_attribute(("w:type", layout_val));
        writer.write_event(Event::Empty(l))?;
    } else {
        let mut layout = BytesStart::new("w:tblLayout");
        layout.push_attribute(("w:type", "autofit"));
        writer.write_event(Event::Empty(layout))?;
    }

    // Table indent
    if let Some(indent) = props.indent {
        let mut ti = BytesStart::new("w:tblInd");
        ti.push_attribute(("w:w", (indent as i32).to_string().as_str()));
        ti.push_attribute(("w:type", "dxa"));
        writer.write_event(Event::Empty(ti))?;
    }

    // Cell spacing
    if let Some(spacing) = props.cell_spacing {
        let mut cs = BytesStart::new("w:tblCellSpacing");
        cs.push_attribute(("w:w", (spacing as u32).to_string().as_str()));
        cs.push_attribute(("w:type", "dxa"));
        writer.write_event(Event::Empty(cs))?;
    }

    // Borders
    if let Some(borders) = &props.borders {
        write_table_borders(writer, borders, "w:tblBorders")?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:tblPr")))?;
    Ok(())
}

fn write_table_borders(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    borders: &TableBorders,
    element_name: &str,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new(element_name)))?;

    if let Some(b) = &borders.top {
        write_border_element(writer, "w:top", b)?;
    }
    if let Some(b) = &borders.left {
        write_border_element(writer, "w:left", b)?;
    }
    if let Some(b) = &borders.bottom {
        write_border_element(writer, "w:bottom", b)?;
    }
    if let Some(b) = &borders.right {
        write_border_element(writer, "w:right", b)?;
    }
    if let Some(b) = &borders.inside_h {
        write_border_element(writer, "w:insideH", b)?;
    }
    if let Some(b) = &borders.inside_v {
        write_border_element(writer, "w:insideV", b)?;
    }

    writer.write_event(Event::End(BytesEnd::new(element_name)))?;
    Ok(())
}

fn write_border_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    name: &str,
    border: &Border,
) -> Result<(), GenerateError> {
    let size_str = (border.size as u32).to_string();
    let color = border.color.trim_start_matches('#');

    let mut elem = BytesStart::new(name);
    elem.push_attribute(("w:val", border.style.as_str()));
    elem.push_attribute(("w:sz", size_str.as_str()));
    elem.push_attribute(("w:space", "0"));
    elem.push_attribute(("w:color", color));
    writer.write_event(Event::Empty(elem))?;
    Ok(())
}

fn write_table_row(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    row: &TableRow,
    image_rid_map: &ImageRidMap,
    hyperlink_rid_map: &HyperlinkRidMap,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:tr")))?;

    // Table row properties
    write_table_row_properties(writer, &row.properties)?;

    for cell in &row.cells {
        write_table_cell(writer, cell, image_rid_map, hyperlink_rid_map)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:tr")))?;
    Ok(())
}

fn write_table_row_properties(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    props: &TableRowProperties,
) -> Result<(), GenerateError> {
    let has_props = props.height.is_some()
        || props.is_header == Some(true)
        || props.cant_split == Some(true);

    if !has_props {
        return Ok(());
    }

    writer.write_event(Event::Start(BytesStart::new("w:trPr")))?;

    // Row height
    if let Some(height) = props.height {
        let mut rh = BytesStart::new("w:trHeight");
        rh.push_attribute(("w:val", (height as u32).to_string().as_str()));
        let rule = props.height_rule.as_deref().unwrap_or("atLeast");
        rh.push_attribute(("w:hRule", rule));
        writer.write_event(Event::Empty(rh))?;
    }

    // Header row
    if props.is_header == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:tblHeader")))?;
    }

    // Can't split
    if props.cant_split == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:cantSplit")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:trPr")))?;
    Ok(())
}

fn write_table_cell(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    cell: &TableCell,
    image_rid_map: &ImageRidMap,
    hyperlink_rid_map: &HyperlinkRidMap,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:tc")))?;

    // Cell properties
    write_table_cell_properties(writer, &cell.properties)?;

    // Cell content -- must contain at least one paragraph per OOXML spec
    if cell.content.is_empty() {
        // Write an empty paragraph
        writer.write_event(Event::Start(BytesStart::new("w:p")))?;
        writer.write_event(Event::End(BytesEnd::new("w:p")))?;
    } else {
        for block in &cell.content {
            write_block_element(writer, block, image_rid_map, hyperlink_rid_map)?;
        }
    }

    writer.write_event(Event::End(BytesEnd::new("w:tc")))?;
    Ok(())
}

fn write_table_cell_properties(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    props: &TableCellProperties,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:tcPr")))?;

    // Width
    if let Some(width) = props.width {
        let mut w = BytesStart::new("w:tcW");
        w.push_attribute(("w:w", (width as u32).to_string().as_str()));
        w.push_attribute(("w:type", "dxa"));
        writer.write_event(Event::Empty(w))?;
    }

    // Grid span
    if let Some(span) = props.grid_span {
        if span > 1 {
            let mut gs = BytesStart::new("w:gridSpan");
            gs.push_attribute(("w:val", span.to_string().as_str()));
            writer.write_event(Event::Empty(gs))?;
        }
    }

    // Vertical merge
    if let Some(vmerge) = &props.vertical_merge {
        match vmerge {
            VerticalMerge::Restart => {
                let mut vm = BytesStart::new("w:vMerge");
                vm.push_attribute(("w:val", "restart"));
                writer.write_event(Event::Empty(vm))?;
            }
            VerticalMerge::Continue => {
                writer.write_event(Event::Empty(BytesStart::new("w:vMerge")))?;
            }
        }
    }

    // Borders
    if let Some(borders) = &props.borders {
        write_table_borders(writer, borders, "w:tcBorders")?;
    }

    // Shading
    if let Some(shading) = &props.shading {
        let color = shading.trim_start_matches('#');
        let mut shd = BytesStart::new("w:shd");
        shd.push_attribute(("w:val", "clear"));
        shd.push_attribute(("w:color", "auto"));
        shd.push_attribute(("w:fill", color));
        writer.write_event(Event::Empty(shd))?;
    }

    // No wrap
    if props.no_wrap == Some(true) {
        writer.write_event(Event::Empty(BytesStart::new("w:noWrap")))?;
    }

    // Cell padding (margins)
    if props.padding_top.is_some()
        || props.padding_bottom.is_some()
        || props.padding_left.is_some()
        || props.padding_right.is_some()
    {
        writer.write_event(Event::Start(BytesStart::new("w:tcMar")))?;
        if let Some(top) = props.padding_top {
            let mut m = BytesStart::new("w:top");
            m.push_attribute(("w:w", (top as u32).to_string().as_str()));
            m.push_attribute(("w:type", "dxa"));
            writer.write_event(Event::Empty(m))?;
        }
        if let Some(left) = props.padding_left {
            let mut m = BytesStart::new("w:left");
            m.push_attribute(("w:w", (left as u32).to_string().as_str()));
            m.push_attribute(("w:type", "dxa"));
            writer.write_event(Event::Empty(m))?;
        }
        if let Some(bottom) = props.padding_bottom {
            let mut m = BytesStart::new("w:bottom");
            m.push_attribute(("w:w", (bottom as u32).to_string().as_str()));
            m.push_attribute(("w:type", "dxa"));
            writer.write_event(Event::Empty(m))?;
        }
        if let Some(right) = props.padding_right {
            let mut m = BytesStart::new("w:right");
            m.push_attribute(("w:w", (right as u32).to_string().as_str()));
            m.push_attribute(("w:type", "dxa"));
            writer.write_event(Event::Empty(m))?;
        }
        writer.write_event(Event::End(BytesEnd::new("w:tcMar")))?;
    }

    // Text direction
    if let Some(dir) = &props.text_direction {
        let mut td = BytesStart::new("w:textDirection");
        td.push_attribute(("w:val", dir.as_str()));
        writer.write_event(Event::Empty(td))?;
    }

    // Vertical alignment
    if let Some(valign) = &props.vertical_align {
        let mut va = BytesStart::new("w:vAlign");
        va.push_attribute(("w:val", valign.as_str()));
        writer.write_event(Event::Empty(va))?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:tcPr")))?;
    Ok(())
}

// -----------------------------------------------------------------------
// Section properties
// -----------------------------------------------------------------------

fn write_section_properties(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    header_rids: &[String],
    footer_rids: &[String],
    section_props: Option<&SectionProperties>,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:sectPr")))?;

    // Header references
    for rid in header_rids {
        let mut href = BytesStart::new("w:headerReference");
        href.push_attribute(("w:type", "default"));
        href.push_attribute(("r:id", rid.as_str()));
        writer.write_event(Event::Empty(href))?;
    }

    // Footer references
    for rid in footer_rids {
        let mut fref = BytesStart::new("w:footerReference");
        fref.push_attribute(("w:type", "default"));
        fref.push_attribute(("r:id", rid.as_str()));
        writer.write_event(Event::Empty(fref))?;
    }

    if let Some(sp) = section_props {
        // Page size from section properties
        let page_w = sp.page_width.map(|v| v as u32).unwrap_or(12240);
        let page_h = sp.page_height.map(|v| v as u32).unwrap_or(15840);

        let mut pg_sz = BytesStart::new("w:pgSz");
        pg_sz.push_attribute(("w:w", page_w.to_string().as_str()));
        pg_sz.push_attribute(("w:h", page_h.to_string().as_str()));
        if let Some(orient) = &sp.page_orientation {
            if *orient == PageOrientation::Landscape {
                pg_sz.push_attribute(("w:orient", "landscape"));
            }
        }
        writer.write_event(Event::Empty(pg_sz))?;

        // Page margins from section properties
        let margin_top = sp.margin_top.map(|v| v as i32).unwrap_or(1440);
        let margin_right = sp.margin_right.map(|v| v as i32).unwrap_or(1440);
        let margin_bottom = sp.margin_bottom.map(|v| v as i32).unwrap_or(1440);
        let margin_left = sp.margin_left.map(|v| v as i32).unwrap_or(1440);
        let margin_header = sp.margin_header.map(|v| v as i32).unwrap_or(720);
        let margin_footer = sp.margin_footer.map(|v| v as i32).unwrap_or(720);
        let margin_gutter = sp.margin_gutter.map(|v| v as i32).unwrap_or(0);

        let mut pg_mar = BytesStart::new("w:pgMar");
        pg_mar.push_attribute(("w:top", margin_top.to_string().as_str()));
        pg_mar.push_attribute(("w:right", margin_right.to_string().as_str()));
        pg_mar.push_attribute(("w:bottom", margin_bottom.to_string().as_str()));
        pg_mar.push_attribute(("w:left", margin_left.to_string().as_str()));
        pg_mar.push_attribute(("w:header", margin_header.to_string().as_str()));
        pg_mar.push_attribute(("w:footer", margin_footer.to_string().as_str()));
        pg_mar.push_attribute(("w:gutter", margin_gutter.to_string().as_str()));
        writer.write_event(Event::Empty(pg_mar))?;

        // Columns
        if let Some(cols) = sp.columns {
            if cols > 1 {
                let mut col_elem = BytesStart::new("w:cols");
                col_elem.push_attribute(("w:num", cols.to_string().as_str()));
                writer.write_event(Event::Empty(col_elem))?;
            }
        }
    } else {
        // Default page size: US Letter (12240 x 15840 twips = 8.5 x 11 inches)
        let mut pg_sz = BytesStart::new("w:pgSz");
        pg_sz.push_attribute(("w:w", "12240"));
        pg_sz.push_attribute(("w:h", "15840"));
        writer.write_event(Event::Empty(pg_sz))?;

        // Default page margins (1-inch margins = 1440 twips)
        let mut pg_mar = BytesStart::new("w:pgMar");
        pg_mar.push_attribute(("w:top", "1440"));
        pg_mar.push_attribute(("w:right", "1440"));
        pg_mar.push_attribute(("w:bottom", "1440"));
        pg_mar.push_attribute(("w:left", "1440"));
        pg_mar.push_attribute(("w:header", "720"));
        pg_mar.push_attribute(("w:footer", "720"));
        pg_mar.push_attribute(("w:gutter", "0"));
        writer.write_event(Event::Empty(pg_mar))?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:sectPr")))?;
    Ok(())
}

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

/// Convert our `Alignment` enum to the OOXML `w:jc` value string.
pub fn alignment_value(alignment: &Alignment) -> &'static str {
    match alignment {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
        Alignment::Justify => "both",
    }
}

/// Collect all unique hyperlink URLs from the document body.
/// This is used by rels_xml to build hyperlink relationships.
pub fn collect_hyperlink_urls(doc: &Document) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    collect_urls_from_blocks(&doc.body, &mut urls);
    urls
}

fn collect_urls_from_blocks(blocks: &[BlockElement], urls: &mut Vec<String>) {
    for block in blocks {
        match block {
            BlockElement::Paragraph(para) => {
                for run in &para.runs {
                    if let Some(url) = &run.properties.hyperlink_url {
                        if !urls.contains(url) {
                            urls.push(url.clone());
                        }
                    }
                }
            }
            BlockElement::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_urls_from_blocks(&cell.content, urls);
                    }
                }
            }
        }
    }
}
