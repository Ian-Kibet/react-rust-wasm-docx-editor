use std::collections::HashMap;
use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use crate::model::{
    Alignment, BlockElement, Border, Document, Paragraph, ParagraphProperties, Run, RunProperties,
    Table, TableBorders, TableCell, TableCellProperties, TableProperties, TableRow,
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

/// Generate `word/document.xml` from the full document model.
///
/// `image_rid_map` maps `ImageData::id` to a relationship ID (e.g. `rId7`).
/// `header_rids` and `footer_rids` map header/footer index to rIds for the
/// `<w:sectPr>` section properties.
pub fn generate_document_xml(
    doc: &Document,
    image_rid_map: &ImageRidMap,
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
        write_block_element(&mut writer, block, image_rid_map)?;
    }

    // Section properties (page setup, header/footer references)
    write_section_properties(&mut writer, header_rids, footer_rids)?;

    writer.write_event(Event::End(BytesEnd::new("w:body")))?;
    writer.write_event(Event::End(BytesEnd::new("w:document")))?;

    Ok(writer.into_inner().into_inner())
}

/// Write a block element -- either a paragraph or a table.
pub fn write_block_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    block: &BlockElement,
    image_rid_map: &ImageRidMap,
) -> Result<(), GenerateError> {
    match block {
        BlockElement::Paragraph(para) => write_paragraph(writer, para, image_rid_map),
        BlockElement::Table(table) => write_table(writer, table, image_rid_map),
    }
}

fn write_paragraph(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    para: &Paragraph,
    image_rid_map: &ImageRidMap,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:p")))?;

    // Paragraph properties
    write_paragraph_properties(writer, &para.properties)?;

    // Runs
    for run in &para.runs {
        write_run(writer, run, image_rid_map)?;
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
        || props.indent_left.is_some()
        || props.indent_right.is_some()
        || props.indent_first_line.is_some();

    if !has_props {
        return Ok(());
    }

    writer.write_event(Event::Start(BytesStart::new("w:pPr")))?;

    // Heading style reference
    if let Some(level) = props.heading_level {
        let style_id = format!("Heading{level}");
        let mut ps = BytesStart::new("w:pStyle");
        ps.push_attribute(("w:val", style_id.as_str()));
        writer.write_event(Event::Empty(ps))?;
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

    // Spacing
    if props.spacing_before.is_some() || props.spacing_after.is_some() {
        let mut spacing = BytesStart::new("w:spacing");
        if let Some(before) = props.spacing_before {
            spacing.push_attribute(("w:before", (before as u32).to_string().as_str()));
        }
        if let Some(after) = props.spacing_after {
            spacing.push_attribute(("w:after", (after as u32).to_string().as_str()));
        }
        writer.write_event(Event::Empty(spacing))?;
    }

    // Indentation
    if props.indent_left.is_some()
        || props.indent_right.is_some()
        || props.indent_first_line.is_some()
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
        writer.write_event(Event::Empty(ind))?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:pPr")))?;
    Ok(())
}

fn write_run(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    run: &Run,
    image_rid_map: &ImageRidMap,
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:r")))?;

    // Run properties
    write_run_properties(writer, &run.properties)?;

    // Inline image or text content
    if let Some(img_id) = &run.properties.inline_image {
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
        || props.font_family.is_some()
        || props.font_size.is_some()
        || props.color.is_some()
        || props.highlight.is_some();

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
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:tbl")))?;

    write_table_properties(writer, &table.properties)?;

    for row in &table.rows {
        write_table_row(writer, row, image_rid_map)?;
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
    let mut layout = BytesStart::new("w:tblLayout");
    layout.push_attribute(("w:type", "autofit"));
    writer.write_event(Event::Empty(layout))?;

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
) -> Result<(), GenerateError> {
    writer.write_event(Event::Start(BytesStart::new("w:tr")))?;

    for cell in &row.cells {
        write_table_cell(writer, cell, image_rid_map)?;
    }

    writer.write_event(Event::End(BytesEnd::new("w:tr")))?;
    Ok(())
}

fn write_table_cell(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    cell: &TableCell,
    image_rid_map: &ImageRidMap,
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
            write_block_element(writer, block, image_rid_map)?;
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

    // Page size: US Letter (12240 x 15840 twips = 8.5 x 11 inches)
    let mut pg_sz = BytesStart::new("w:pgSz");
    pg_sz.push_attribute(("w:w", "12240"));
    pg_sz.push_attribute(("w:h", "15840"));
    writer.write_event(Event::Empty(pg_sz))?;

    // Page margins (1-inch margins = 1440 twips)
    let mut pg_mar = BytesStart::new("w:pgMar");
    pg_mar.push_attribute(("w:top", "1440"));
    pg_mar.push_attribute(("w:right", "1440"));
    pg_mar.push_attribute(("w:bottom", "1440"));
    pg_mar.push_attribute(("w:left", "1440"));
    pg_mar.push_attribute(("w:header", "720"));
    pg_mar.push_attribute(("w:footer", "720"));
    pg_mar.push_attribute(("w:gutter", "0"));
    writer.write_event(Event::Empty(pg_mar))?;

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
