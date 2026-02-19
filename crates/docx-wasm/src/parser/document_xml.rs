use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;
use uuid::Uuid;

use crate::model::{
    BlockElement, NumberingRef, Paragraph, ParagraphProperties, Run,
    RunProperties, Table, TableBorders, TableCell, TableCellProperties,
    TableProperties, TableRow, Border,
};
use super::rels_xml::RelsMap;
use super::styles_xml::parse_alignment;
use super::ParseError;

/// Parse the main `word/document.xml` content into a flat list of block-level
/// elements (paragraphs and tables).
///
/// * `xml` -- the raw XML string of `word/document.xml`
/// * `rels` -- relationship-id -> target-path map (for resolving images)
/// * `media_path_to_id` -- archive-path -> `ImageData::id` (from media extraction)
pub fn parse_document(
    xml: &str,
    rels: &RelsMap,
    media_path_to_id: &HashMap<String, String>,
) -> Result<Vec<BlockElement>, ParseError> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    let mut ctx = ParseContext::new(rels, media_path_to_id);
    let mut body_elements: Vec<BlockElement> = Vec::new();
    let mut in_body = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"body" => {
                        in_body = true;
                    }

                    // ----- Paragraphs -----
                    b"p" if in_body || ctx.in_table_cell() => {
                        ctx.push_paragraph();
                    }
                    b"pPr" if ctx.in_paragraph() => {
                        ctx.state = ElementState::ParagraphProps;
                    }
                    b"numPr" if ctx.state == ElementState::ParagraphProps => {
                        ctx.state = ElementState::NumPr;
                    }
                    b"r" if ctx.in_paragraph() => {
                        ctx.push_run();
                    }
                    b"rPr" if ctx.in_run() => {
                        ctx.state = ElementState::RunProps;
                    }
                    b"t" if ctx.in_run() => {
                        ctx.state = ElementState::Text;
                    }

                    // ----- Drawing / Images -----
                    b"drawing" if ctx.in_run() => {
                        ctx.state = ElementState::Drawing;
                    }

                    // ----- Tables -----
                    b"tbl" if in_body || ctx.in_table_cell() => {
                        ctx.push_table();
                    }
                    b"tblPr" if ctx.in_table() => {
                        ctx.state = ElementState::TableProps;
                    }
                    b"tblBorders" if ctx.state == ElementState::TableProps => {
                        ctx.state = ElementState::TableBorders;
                    }
                    b"tr" if ctx.in_table() => {
                        ctx.push_table_row();
                    }
                    b"tc" if ctx.in_table_row() => {
                        ctx.push_table_cell();
                    }
                    b"tcPr" if ctx.in_table_cell() => {
                        ctx.state = ElementState::CellProps;
                    }
                    b"tcBorders" if ctx.state == ElementState::CellProps => {
                        ctx.state = ElementState::CellBorders;
                    }

                    _ => {
                        // Handle self-closing-like starts for properties
                        handle_start_element(e, &mut ctx);
                    }
                }
            }

            Ok(Event::Empty(ref e)) => {
                handle_empty_element(e, &mut ctx);
            }

            Ok(Event::Text(ref t)) => {
                if ctx.state == ElementState::Text {
                    if let Some(ref mut run) = ctx.current_run {
                        let text = t.unescape().unwrap_or_default().to_string();
                        run.text.push_str(&text);
                    }
                }
            }

            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"body" => {
                        in_body = false;
                    }

                    b"p" => {
                        if let Some(para) = ctx.pop_paragraph() {
                            if ctx.in_table_cell() {
                                if let Some(ref mut cell) = ctx.current_cell {
                                    cell.content.push(BlockElement::Paragraph(para));
                                }
                            } else {
                                body_elements.push(BlockElement::Paragraph(para));
                            }
                        }
                    }

                    b"r" => {
                        ctx.pop_run();
                    }
                    b"t" => {
                        if ctx.state == ElementState::Text {
                            ctx.state = ElementState::Run;
                        }
                    }
                    b"pPr" => {
                        if ctx.state == ElementState::ParagraphProps {
                            ctx.state = ElementState::Paragraph;
                        }
                    }
                    b"numPr" => {
                        if ctx.state == ElementState::NumPr {
                            ctx.state = ElementState::ParagraphProps;
                        }
                    }
                    b"rPr" => {
                        if ctx.state == ElementState::RunProps {
                            ctx.state = ElementState::Run;
                        }
                    }
                    b"drawing" => {
                        if ctx.state == ElementState::Drawing {
                            ctx.state = ElementState::Run;
                        }
                    }

                    // ----- Table end tags -----
                    b"tbl" => {
                        if let Some(table) = ctx.pop_table() {
                            if ctx.in_table_cell() {
                                // Nested table
                                if let Some(ref mut cell) = ctx.current_cell {
                                    cell.content.push(BlockElement::Table(table));
                                }
                            } else {
                                body_elements.push(BlockElement::Table(table));
                            }
                        }
                    }
                    b"tr" => {
                        ctx.pop_table_row();
                    }
                    b"tc" => {
                        ctx.pop_table_cell();
                    }
                    b"tblPr" => {
                        if ctx.state == ElementState::TableProps {
                            ctx.state = ElementState::Table;
                        }
                    }
                    b"tblBorders" => {
                        if ctx.state == ElementState::TableBorders {
                            ctx.state = ElementState::TableProps;
                        }
                    }
                    b"tcPr" => {
                        if ctx.state == ElementState::CellProps {
                            ctx.state = ElementState::Cell;
                        }
                    }
                    b"tcBorders" => {
                        if ctx.state == ElementState::CellBorders {
                            ctx.state = ElementState::CellProps;
                        }
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

    Ok(body_elements)
}

// ---------------------------------------------------------------------------
// Parsing context
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ElementState {
    None,
    Paragraph,
    ParagraphProps,
    NumPr,
    Run,
    RunProps,
    Text,
    Drawing,
    Table,
    TableProps,
    TableBorders,
    TableRow,
    Cell,
    CellProps,
    CellBorders,
}

struct ParseContext<'a> {
    state: ElementState,
    rels: &'a RelsMap,
    media_path_to_id: &'a HashMap<String, String>,

    // Current paragraph being built
    current_paragraph: Option<ParagraphBuilder>,
    // Current run being built
    current_run: Option<RunBuilder>,

    // Table stack (for nested tables)
    table_stack: Vec<TableBuilder>,
    current_row: Option<TableRowBuilder>,
    current_cell: Option<TableCellBuilder>,
}

impl<'a> ParseContext<'a> {
    fn new(
        rels: &'a RelsMap,
        media_path_to_id: &'a HashMap<String, String>,
    ) -> Self {
        Self {
            state: ElementState::None,
            rels,
            media_path_to_id,
            current_paragraph: None,
            current_run: None,
            table_stack: Vec::new(),
            current_row: None,
            current_cell: None,
        }
    }

    fn in_paragraph(&self) -> bool {
        self.current_paragraph.is_some()
    }

    fn in_run(&self) -> bool {
        self.current_run.is_some()
    }

    fn in_table(&self) -> bool {
        !self.table_stack.is_empty()
    }

    fn in_table_row(&self) -> bool {
        self.current_row.is_some()
    }

    fn in_table_cell(&self) -> bool {
        self.current_cell.is_some()
    }

    // ----- Paragraph -----

    fn push_paragraph(&mut self) {
        self.current_paragraph = Some(ParagraphBuilder::new());
        self.state = ElementState::Paragraph;
    }

    fn pop_paragraph(&mut self) -> Option<Paragraph> {
        // Flush any pending run first
        self.flush_run();
        let builder = self.current_paragraph.take()?;
        self.state = if self.in_table_cell() {
            ElementState::Cell
        } else {
            ElementState::None
        };
        Some(builder.build())
    }

    // ----- Run -----

    fn push_run(&mut self) {
        self.current_run = Some(RunBuilder::new());
        self.state = ElementState::Run;
    }

    fn pop_run(&mut self) {
        self.flush_run();
        self.state = ElementState::Paragraph;
    }

    fn flush_run(&mut self) {
        if let Some(run_builder) = self.current_run.take() {
            let run = run_builder.build();
            // Only add non-empty runs (has text or an image)
            if !run.text.is_empty() || run.properties.inline_image.is_some() {
                if let Some(ref mut para) = self.current_paragraph {
                    para.runs.push(run);
                }
            }
        }
    }

    // ----- Tables -----

    fn push_table(&mut self) {
        self.table_stack.push(TableBuilder::new());
        self.state = ElementState::Table;
    }

    fn pop_table(&mut self) -> Option<Table> {
        let builder = self.table_stack.pop()?;
        self.state = if self.in_table_cell() {
            ElementState::Cell
        } else {
            ElementState::None
        };
        Some(builder.build())
    }

    fn push_table_row(&mut self) {
        self.current_row = Some(TableRowBuilder::new());
        self.state = ElementState::TableRow;
    }

    fn pop_table_row(&mut self) {
        if let Some(row_builder) = self.current_row.take() {
            let row = row_builder.build();
            if let Some(table) = self.table_stack.last_mut() {
                table.rows.push(row);
            }
        }
        self.state = ElementState::Table;
    }

    fn push_table_cell(&mut self) {
        self.current_cell = Some(TableCellBuilder::new());
        self.state = ElementState::Cell;
    }

    fn pop_table_cell(&mut self) {
        if let Some(cell_builder) = self.current_cell.take() {
            let cell = cell_builder.build();
            if let Some(ref mut row) = self.current_row {
                row.cells.push(cell);
            }
        }
        self.state = ElementState::TableRow;
    }

    /// Resolve a relationship ID (e.g. "rId5") to an `ImageData::id`.
    fn resolve_image_id(&self, rel_id: &str) -> Option<String> {
        let target = self.rels.get(rel_id)?;
        self.media_path_to_id.get(target).cloned()
    }
}

// ---------------------------------------------------------------------------
// Builder types
// ---------------------------------------------------------------------------

struct ParagraphBuilder {
    id: String,
    properties: ParagraphProperties,
    runs: Vec<Run>,
}

impl ParagraphBuilder {
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
        }
    }
}

struct RunBuilder {
    id: String,
    properties: RunProperties,
    text: String,
}

impl RunBuilder {
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

struct TableBuilder {
    id: String,
    properties: TableProperties,
    rows: Vec<TableRow>,
}

impl TableBuilder {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            properties: TableProperties::default(),
            rows: Vec::new(),
        }
    }

    fn build(self) -> Table {
        Table {
            id: self.id,
            properties: self.properties,
            rows: self.rows,
        }
    }
}

struct TableRowBuilder {
    id: String,
    cells: Vec<TableCell>,
}

impl TableRowBuilder {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            cells: Vec::new(),
        }
    }

    fn build(self) -> TableRow {
        TableRow {
            id: self.id,
            cells: self.cells,
        }
    }
}

struct TableCellBuilder {
    id: String,
    properties: TableCellProperties,
    content: Vec<BlockElement>,
}

impl TableCellBuilder {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            properties: TableCellProperties::default(),
            content: Vec::new(),
        }
    }

    fn build(self) -> TableCell {
        TableCell {
            id: self.id,
            properties: self.properties,
            content: self.content,
        }
    }
}

// ---------------------------------------------------------------------------
// Element handling helpers
// ---------------------------------------------------------------------------

/// Handle start elements that carry attributes but also have children (not
/// self-closing).  Many OOXML property elements can appear either as
/// `<w:b/>` (empty) or `<w:b w:val="true"/>` (empty with attr) or
/// `<w:b>...</w:b>` (start/end).  We handle the attribute reading here.
fn handle_start_element(
    e: &quick_xml::events::BytesStart<'_>,
    ctx: &mut ParseContext<'_>,
) {
    let local = e.local_name();
    match local.as_ref() {
        // Inside w:rPr -- run formatting properties
        b"b" if ctx.state == ElementState::RunProps => {
            if let Some(ref mut run) = ctx.current_run {
                run.properties.bold = Some(!is_val_false(e));
            }
        }
        b"i" if ctx.state == ElementState::RunProps => {
            if let Some(ref mut run) = ctx.current_run {
                run.properties.italic = Some(!is_val_false(e));
            }
        }
        b"u" if ctx.state == ElementState::RunProps => {
            if let Some(ref mut run) = ctx.current_run {
                run.properties.underline = Some(true);
            }
        }
        b"strike" if ctx.state == ElementState::RunProps => {
            if let Some(ref mut run) = ctx.current_run {
                run.properties.strikethrough = Some(!is_val_false(e));
            }
        }

        // Inside w:pPr -- paragraph style reference
        b"pStyle" if ctx.state == ElementState::ParagraphProps => {
            if let Some(ref mut para) = ctx.current_paragraph {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"val" {
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        // Detect heading levels from style name
                        if let Some(level) = heading_level_from_style(&val) {
                            para.properties.heading_level = Some(level);
                        }
                    }
                }
            }
        }

        _ => {}
    }
}

/// Handle self-closing (empty) XML elements.  This is where most OOXML
/// property elements land (e.g. `<w:b/>`, `<w:sz w:val="24"/>`).
fn handle_empty_element(
    e: &quick_xml::events::BytesStart<'_>,
    ctx: &mut ParseContext<'_>,
) {
    let local = e.local_name();

    match ctx.state {
        // ---- Run properties ----
        ElementState::RunProps => match local.as_ref() {
            b"b" => {
                if let Some(ref mut run) = ctx.current_run {
                    run.properties.bold = Some(!is_val_false(e));
                }
            }
            b"i" => {
                if let Some(ref mut run) = ctx.current_run {
                    run.properties.italic = Some(!is_val_false(e));
                }
            }
            b"u" => {
                if let Some(ref mut run) = ctx.current_run {
                    run.properties.underline = Some(true);
                }
            }
            b"strike" => {
                if let Some(ref mut run) = ctx.current_run {
                    run.properties.strikethrough = Some(!is_val_false(e));
                }
            }
            b"sz" => {
                if let Some(ref mut run) = ctx.current_run {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            if let Ok(half_pts) =
                                String::from_utf8_lossy(&attr.value).parse::<f64>()
                            {
                                run.properties.font_size = Some(half_pts / 2.0);
                            }
                        }
                    }
                }
            }
            b"color" => {
                if let Some(ref mut run) = ctx.current_run {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            let val =
                                String::from_utf8_lossy(&attr.value).to_string();
                            if val != "auto" {
                                run.properties.color = Some(val);
                            }
                        }
                    }
                }
            }
            b"rFonts" => {
                if let Some(ref mut run) = ctx.current_run {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"ascii" {
                            run.properties.font_family = Some(
                                String::from_utf8_lossy(&attr.value).to_string(),
                            );
                        }
                    }
                }
            }
            b"highlight" => {
                if let Some(ref mut run) = ctx.current_run {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            run.properties.highlight = Some(
                                String::from_utf8_lossy(&attr.value).to_string(),
                            );
                        }
                    }
                }
            }
            _ => {}
        },

        // ---- Paragraph properties ----
        ElementState::ParagraphProps => match local.as_ref() {
            b"jc" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            para.properties.alignment = Some(parse_alignment(
                                &String::from_utf8_lossy(&attr.value),
                            ));
                        }
                    }
                }
            }
            b"pStyle" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            let val =
                                String::from_utf8_lossy(&attr.value).to_string();
                            if let Some(level) = heading_level_from_style(&val) {
                                para.properties.heading_level = Some(level);
                            }
                        }
                    }
                }
            }
            b"spacing" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    for attr in e.attributes().flatten() {
                        match attr.key.local_name().as_ref() {
                            b"before" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    para.properties.spacing_before = Some(v);
                                }
                            }
                            b"after" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    para.properties.spacing_after = Some(v);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            b"ind" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    for attr in e.attributes().flatten() {
                        match attr.key.local_name().as_ref() {
                            b"left" | b"start" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    para.properties.indent_left = Some(v);
                                }
                            }
                            b"right" | b"end" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    para.properties.indent_right = Some(v);
                                }
                            }
                            b"firstLine" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    para.properties.indent_first_line = Some(v);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        },

        // ---- Numbering reference inside pPr ----
        ElementState::NumPr => match local.as_ref() {
            b"numId" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            let val =
                                String::from_utf8_lossy(&attr.value).to_string();
                            if let Some(ref mut nr) = para.properties.numbering {
                                nr.num_id = val;
                            } else {
                                para.properties.numbering = Some(NumberingRef {
                                    num_id: val,
                                    level: 0,
                                });
                            }
                        }
                    }
                }
            }
            b"ilvl" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            let level = String::from_utf8_lossy(&attr.value)
                                .parse::<u32>()
                                .unwrap_or(0);
                            if let Some(ref mut nr) = para.properties.numbering {
                                nr.level = level;
                            } else {
                                para.properties.numbering = Some(NumberingRef {
                                    num_id: String::new(),
                                    level,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        },

        // ---- Drawing / image ----
        ElementState::Drawing => {
            // Look for <a:blip r:embed="rId..."/>
            if local.as_ref() == b"blip" {
                // Resolve the image ID first to avoid borrow conflict
                let mut resolved_image_id = None;
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"embed" {
                        let rel_id =
                            String::from_utf8_lossy(&attr.value).to_string();
                        resolved_image_id = ctx.resolve_image_id(&rel_id);
                        break;
                    }
                }
                if let (Some(image_id), Some(ref mut run)) =
                    (resolved_image_id, &mut ctx.current_run)
                {
                    run.properties.inline_image = Some(image_id);
                }
            }
        }

        // ---- Table properties ----
        ElementState::TableProps => match local.as_ref() {
            b"tblW" => {
                if let Some(table) = ctx.table_stack.last_mut() {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"w" {
                            if let Ok(w) =
                                String::from_utf8_lossy(&attr.value).parse::<f64>()
                            {
                                table.properties.width = Some(w);
                            }
                        }
                    }
                }
            }
            _ => {}
        },

        // ---- Table borders ----
        ElementState::TableBorders => {
            if let Some(table) = ctx.table_stack.last_mut() {
                let border = parse_border(e);
                let borders = table.properties.borders.get_or_insert_with(TableBorders::default);
                apply_border(borders, local.as_ref(), border);
            }
        }

        // ---- Cell properties ----
        ElementState::CellProps => match local.as_ref() {
            b"tcW" => {
                if let Some(ref mut cell) = ctx.current_cell {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"w" {
                            if let Ok(w) =
                                String::from_utf8_lossy(&attr.value).parse::<f64>()
                            {
                                cell.properties.width = Some(w);
                            }
                        }
                    }
                }
            }
            b"vMerge" => {
                if let Some(ref mut cell) = ctx.current_cell {
                    let mut val = "continue".to_string();
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            val = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                    cell.properties.vertical_align = Some(val);
                }
            }
            b"shd" => {
                if let Some(ref mut cell) = ctx.current_cell {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"fill" {
                            let fill =
                                String::from_utf8_lossy(&attr.value).to_string();
                            if fill != "auto" {
                                cell.properties.shading = Some(fill);
                            }
                        }
                    }
                }
            }
            _ => {}
        },

        // ---- Cell borders ----
        ElementState::CellBorders => {
            if let Some(ref mut cell) = ctx.current_cell {
                let border = parse_border(e);
                let borders = cell.properties.borders.get_or_insert_with(TableBorders::default);
                apply_border(borders, local.as_ref(), border);
            }
        }

        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

/// Check if a boolean-valued OOXML element is explicitly set to false.
/// Elements like `<w:b w:val="false"/>` or `<w:b w:val="0"/>` mean "not bold".
/// If no `val` attribute is present, the element's mere presence means true.
fn is_val_false(e: &quick_xml::events::BytesStart<'_>) -> bool {
    for attr in e.attributes().flatten() {
        if attr.key.local_name().as_ref() == b"val" {
            let v = String::from_utf8_lossy(&attr.value);
            return v == "false" || v == "0";
        }
    }
    false
}

/// Try to extract a heading level from a paragraph style id.
/// Common patterns: "Heading1", "heading 1", "Heading2", etc.
fn heading_level_from_style(style_id: &str) -> Option<u8> {
    let lower = style_id.to_lowercase();
    if lower.starts_with("heading") {
        // "Heading1" -> "1", "heading 2" -> " 2"
        let suffix = &lower["heading".len()..];
        suffix.trim().parse::<u8>().ok()
    } else {
        None
    }
}

/// Parse border attributes from a border element (top, bottom, left, right,
/// insideH, insideV).
fn parse_border(e: &quick_xml::events::BytesStart<'_>) -> Border {
    let mut border = Border {
        style: String::new(),
        size: 0.0,
        color: String::new(),
    };

    for attr in e.attributes().flatten() {
        match attr.key.local_name().as_ref() {
            b"val" => {
                border.style = String::from_utf8_lossy(&attr.value).to_string();
            }
            b"sz" => {
                if let Ok(sz) = String::from_utf8_lossy(&attr.value).parse::<f64>() {
                    border.size = sz;
                }
            }
            b"color" => {
                let color = String::from_utf8_lossy(&attr.value).to_string();
                if color != "auto" {
                    border.color = color;
                }
            }
            _ => {}
        }
    }

    border
}

/// Apply a parsed border to the correct side of a `TableBorders` struct based
/// on the XML element local name.
fn apply_border(borders: &mut TableBorders, tag: &[u8], border: Border) {
    match tag {
        b"top" => borders.top = Some(border),
        b"bottom" => borders.bottom = Some(border),
        b"left" | b"start" => borders.left = Some(border),
        b"right" | b"end" => borders.right = Some(border),
        b"insideH" => borders.inside_h = Some(border),
        b"insideV" => borders.inside_v = Some(border),
        _ => {}
    }
}
