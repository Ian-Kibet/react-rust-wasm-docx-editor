use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;
use uuid::Uuid;

use crate::model::{
    BlockElement, Bookmark, Border, LineSpacingRule, NumberingRef,
    PageOrientation, Paragraph, ParagraphProperties, Run, RunProperties,
    SectionProperties, Table, TableBorders, TableCell, TableCellProperties,
    TableLayout, TableProperties, TableRow, TableRowProperties, TabStop,
    TabStopAlignment, VerticalMerge,
};
use super::rels_xml::RelsMap;
use super::styles_xml::parse_alignment;
use super::ParseError;

/// Parse the main `word/document.xml` content into a flat list of block-level
/// elements (paragraphs and tables) and optional section properties.
///
/// * `xml` -- the raw XML string of `word/document.xml`
/// * `rels` -- relationship-id -> target-path map (for resolving images and hyperlinks)
/// * `media_path_to_id` -- archive-path -> `ImageData::id` (from media extraction)
pub fn parse_document(
    xml: &str,
    rels: &RelsMap,
    media_path_to_id: &HashMap<String, String>,
) -> Result<(Vec<BlockElement>, Option<SectionProperties>), ParseError> {
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

                    // ----- Hyperlinks -----
                    b"hyperlink" if ctx.in_paragraph() => {
                        // Resolve the hyperlink URL from r:id attribute
                        let mut url: Option<String> = None;
                        let mut tooltip: Option<String> = None;
                        let mut anchor: Option<String> = None;
                        for attr in e.attributes().flatten() {
                            match attr.key.local_name().as_ref() {
                                b"id" => {
                                    let rel_id =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                    if let Some(target) = ctx.rels.get(&rel_id) {
                                        if target.starts_with("http") {
                                            url = Some(target.clone());
                                        } else if target.starts_with('#') {
                                            url = Some(target.clone());
                                        } else {
                                            // Relative path / internal target
                                            url = Some(target.clone());
                                        }
                                    }
                                }
                                b"tooltip" => {
                                    tooltip = Some(
                                        String::from_utf8_lossy(&attr.value).to_string(),
                                    );
                                }
                                b"anchor" => {
                                    anchor = Some(format!(
                                        "#{}",
                                        String::from_utf8_lossy(&attr.value)
                                    ));
                                }
                                _ => {}
                            }
                        }
                        // If we have an anchor but no r:id URL, use the anchor
                        if url.is_none() {
                            if let Some(anchor_url) = anchor {
                                url = Some(anchor_url);
                            }
                        }
                        ctx.hyperlink_url = url;
                        ctx.hyperlink_tooltip = tooltip;
                        ctx.state = ElementState::Hyperlink;
                    }

                    // ----- Paragraphs -----
                    b"p" if in_body || ctx.in_table_cell() => {
                        ctx.push_paragraph();
                    }
                    b"pPr" if ctx.in_paragraph() => {
                        ctx.state = ElementState::ParagraphProps;
                    }
                    b"pBdr" if ctx.state == ElementState::ParagraphProps => {
                        ctx.state = ElementState::ParagraphBorders;
                    }
                    b"tabs" if ctx.state == ElementState::ParagraphProps => {
                        ctx.state = ElementState::ParagraphTabs;
                    }
                    b"numPr" if ctx.state == ElementState::ParagraphProps => {
                        ctx.state = ElementState::NumPr;
                    }
                    b"r" if ctx.in_paragraph() || ctx.state == ElementState::Hyperlink => {
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
                    b"trPr" if ctx.in_table_row() => {
                        ctx.state = ElementState::TableRowProps;
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
                    b"tcMar" if ctx.state == ElementState::CellProps => {
                        ctx.state = ElementState::CellMargins;
                    }

                    // ----- Section properties -----
                    b"sectPr" if in_body => {
                        ctx.state = ElementState::SectionProps;
                        ctx.section_properties = Some(SectionProperties {
                            page_width: None,
                            page_height: None,
                            margin_top: None,
                            margin_right: None,
                            margin_bottom: None,
                            margin_left: None,
                            margin_header: None,
                            margin_footer: None,
                            margin_gutter: None,
                            page_orientation: None,
                            columns: None,
                        });
                    }

                    _ => {
                        // Handle self-closing-like starts for properties
                        handle_start_element(e, &mut ctx);
                    }
                }
            }

            Ok(Event::Empty(ref e)) => {
                handle_empty_element(e, &mut ctx, in_body);
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

                    b"hyperlink" => {
                        // End of hyperlink scope -- clear hyperlink context
                        ctx.hyperlink_url = None;
                        ctx.hyperlink_tooltip = None;
                        if ctx.in_paragraph() {
                            ctx.state = ElementState::Paragraph;
                        }
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
                    b"pBdr" => {
                        if ctx.state == ElementState::ParagraphBorders {
                            ctx.state = ElementState::ParagraphProps;
                        }
                    }
                    b"tabs" => {
                        if ctx.state == ElementState::ParagraphTabs {
                            ctx.state = ElementState::ParagraphProps;
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
                    b"trPr" => {
                        if ctx.state == ElementState::TableRowProps {
                            ctx.state = ElementState::TableRow;
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
                    b"tcMar" => {
                        if ctx.state == ElementState::CellMargins {
                            ctx.state = ElementState::CellProps;
                        }
                    }

                    // ----- Section properties end -----
                    b"sectPr" => {
                        if ctx.state == ElementState::SectionProps {
                            ctx.state = ElementState::None;
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

    Ok((body_elements, ctx.section_properties))
}

// ---------------------------------------------------------------------------
// Parsing context
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ElementState {
    None,
    Paragraph,
    ParagraphProps,
    ParagraphBorders,
    ParagraphTabs,
    NumPr,
    Run,
    RunProps,
    Text,
    Drawing,
    Hyperlink,
    Table,
    TableProps,
    TableBorders,
    TableRow,
    TableRowProps,
    Cell,
    CellProps,
    CellBorders,
    CellMargins,
    SectionProps,
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

    // Hyperlink context (set when inside a <w:hyperlink> element)
    hyperlink_url: Option<String>,
    hyperlink_tooltip: Option<String>,

    // Section properties parsed from <w:sectPr>
    section_properties: Option<SectionProperties>,
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
            hyperlink_url: None,
            hyperlink_tooltip: None,
            section_properties: None,
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
        let mut run = RunBuilder::new();
        // If we are inside a hyperlink, propagate the URL and tooltip
        if let Some(ref url) = self.hyperlink_url {
            run.properties.hyperlink_url = Some(url.clone());
        }
        if let Some(ref tooltip) = self.hyperlink_tooltip {
            run.properties.hyperlink_tooltip = Some(tooltip.clone());
        }
        self.current_run = Some(run);
        self.state = ElementState::Run;
    }

    fn pop_run(&mut self) {
        self.flush_run();
        // Restore state: if we are inside a hyperlink, return to Hyperlink state
        if self.hyperlink_url.is_some() {
            self.state = ElementState::Hyperlink;
        } else {
            self.state = ElementState::Paragraph;
        }
    }

    fn flush_run(&mut self) {
        if let Some(run_builder) = self.current_run.take() {
            let run = run_builder.build();
            // Add runs that have text, an image, a break, a tab, or a footnote/endnote ref
            if !run.text.is_empty()
                || run.properties.inline_image.is_some()
                || run.properties.line_break == Some(true)
                || run.properties.page_break == Some(true)
                || run.properties.tab == Some(true)
                || run.properties.footnote_ref.is_some()
                || run.properties.endnote_ref.is_some()
            {
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
    bookmarks: Vec<Bookmark>,
}

impl ParagraphBuilder {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            properties: ParagraphProperties::default(),
            runs: Vec::new(),
            bookmarks: Vec::new(),
        }
    }

    fn build(self) -> Paragraph {
        Paragraph {
            id: self.id,
            properties: self.properties,
            runs: self.runs,
            bookmarks: self.bookmarks,
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
    properties: TableRowProperties,
    cells: Vec<TableCell>,
}

impl TableRowBuilder {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            properties: TableRowProperties::default(),
            cells: Vec::new(),
        }
    }

    fn build(self) -> TableRow {
        TableRow {
            id: self.id,
            cells: self.cells,
            properties: self.properties,
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
        // ----- Inside w:rPr -- run formatting properties -----
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
        b"dstrike" if ctx.state == ElementState::RunProps => {
            if let Some(ref mut run) = ctx.current_run {
                run.properties.double_strikethrough = Some(!is_val_false(e));
            }
        }
        b"smallCaps" if ctx.state == ElementState::RunProps => {
            if let Some(ref mut run) = ctx.current_run {
                run.properties.small_caps = Some(!is_val_false(e));
            }
        }
        b"caps" if ctx.state == ElementState::RunProps => {
            if let Some(ref mut run) = ctx.current_run {
                run.properties.all_caps = Some(!is_val_false(e));
            }
        }
        b"vertAlign" if ctx.state == ElementState::RunProps => {
            if let Some(ref mut run) = ctx.current_run {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"val" {
                        let val = String::from_utf8_lossy(&attr.value);
                        match val.as_ref() {
                            "superscript" => {
                                run.properties.superscript = Some(true);
                                run.properties.subscript = Some(false);
                            }
                            "subscript" => {
                                run.properties.subscript = Some(true);
                                run.properties.superscript = Some(false);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        b"sz" if ctx.state == ElementState::RunProps => {
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
        b"color" if ctx.state == ElementState::RunProps => {
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
        b"rFonts" if ctx.state == ElementState::RunProps => {
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
        b"highlight" if ctx.state == ElementState::RunProps => {
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
        b"shd" if ctx.state == ElementState::RunProps => {
            if let Some(ref mut run) = ctx.current_run {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"fill" {
                        let fill =
                            String::from_utf8_lossy(&attr.value).to_string();
                        if fill != "auto" {
                            run.properties.background_color = Some(fill);
                        }
                    }
                }
            }
        }
        b"spacing" if ctx.state == ElementState::RunProps => {
            if let Some(ref mut run) = ctx.current_run {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"val" {
                        if let Ok(v) =
                            String::from_utf8_lossy(&attr.value).parse::<f64>()
                        {
                            run.properties.spacing = Some(v);
                        }
                    }
                }
            }
        }

        // ----- Inside w:pPr -- paragraph formatting properties -----
        b"pStyle" if ctx.state == ElementState::ParagraphProps => {
            if let Some(ref mut para) = ctx.current_paragraph {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"val" {
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        para.properties.style_id = Some(val.clone());
                        // Detect heading levels from style name
                        if let Some(level) = heading_level_from_style(&val) {
                            para.properties.heading_level = Some(level);
                        }
                    }
                }
            }
        }
        b"spacing" if ctx.state == ElementState::ParagraphProps => {
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
                        b"line" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                para.properties.line_spacing = Some(v);
                            }
                        }
                        b"lineRule" => {
                            let rule_val = String::from_utf8_lossy(&attr.value);
                            para.properties.line_spacing_rule =
                                Some(parse_line_spacing_rule(&rule_val));
                        }
                        _ => {}
                    }
                }
            }
        }
        b"ind" if ctx.state == ElementState::ParagraphProps => {
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
                        b"hanging" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                para.properties.indent_hanging = Some(v);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        b"pageBreakBefore" if ctx.state == ElementState::ParagraphProps => {
            if let Some(ref mut para) = ctx.current_paragraph {
                para.properties.page_break_before = Some(!is_val_false(e));
            }
        }
        b"keepNext" if ctx.state == ElementState::ParagraphProps => {
            if let Some(ref mut para) = ctx.current_paragraph {
                para.properties.keep_next = Some(!is_val_false(e));
            }
        }
        b"keepLines" if ctx.state == ElementState::ParagraphProps => {
            if let Some(ref mut para) = ctx.current_paragraph {
                para.properties.keep_lines = Some(!is_val_false(e));
            }
        }
        b"widowControl" if ctx.state == ElementState::ParagraphProps => {
            if let Some(ref mut para) = ctx.current_paragraph {
                para.properties.widow_control = Some(!is_val_false(e));
            }
        }
        b"shd" if ctx.state == ElementState::ParagraphProps => {
            if let Some(ref mut para) = ctx.current_paragraph {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"fill" {
                        let fill =
                            String::from_utf8_lossy(&attr.value).to_string();
                        if fill != "auto" {
                            para.properties.shading = Some(fill);
                        }
                    }
                }
            }
        }

        // ----- Inside w:pBdr -- paragraph borders -----
        b"top" if ctx.state == ElementState::ParagraphBorders => {
            if let Some(ref mut para) = ctx.current_paragraph {
                para.properties.border_top = Some(parse_border(e));
            }
        }
        b"bottom" if ctx.state == ElementState::ParagraphBorders => {
            if let Some(ref mut para) = ctx.current_paragraph {
                para.properties.border_bottom = Some(parse_border(e));
            }
        }

        // ----- Inside w:trPr -- table row properties -----
        b"trHeight" if ctx.state == ElementState::TableRowProps => {
            if let Some(ref mut row) = ctx.current_row {
                for attr in e.attributes().flatten() {
                    match attr.key.local_name().as_ref() {
                        b"val" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                row.properties.height = Some(v);
                            }
                        }
                        b"hRule" => {
                            row.properties.height_rule = Some(
                                String::from_utf8_lossy(&attr.value).to_string(),
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
        b"tblHeader" if ctx.state == ElementState::TableRowProps => {
            if let Some(ref mut row) = ctx.current_row {
                row.properties.is_header = Some(!is_val_false(e));
            }
        }
        b"cantSplit" if ctx.state == ElementState::TableRowProps => {
            if let Some(ref mut row) = ctx.current_row {
                row.properties.cant_split = Some(!is_val_false(e));
            }
        }

        // ----- Inside w:sectPr -- section properties -----
        b"pgSz" if ctx.state == ElementState::SectionProps => {
            if let Some(ref mut sp) = ctx.section_properties {
                for attr in e.attributes().flatten() {
                    match attr.key.local_name().as_ref() {
                        b"w" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                sp.page_width = Some(v);
                            }
                        }
                        b"h" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                sp.page_height = Some(v);
                            }
                        }
                        b"orient" => {
                            let val = String::from_utf8_lossy(&attr.value);
                            sp.page_orientation = Some(parse_page_orientation(&val));
                        }
                        _ => {}
                    }
                }
            }
        }
        b"pgMar" if ctx.state == ElementState::SectionProps => {
            if let Some(ref mut sp) = ctx.section_properties {
                for attr in e.attributes().flatten() {
                    match attr.key.local_name().as_ref() {
                        b"top" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                sp.margin_top = Some(v);
                            }
                        }
                        b"right" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                sp.margin_right = Some(v);
                            }
                        }
                        b"bottom" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                sp.margin_bottom = Some(v);
                            }
                        }
                        b"left" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                sp.margin_left = Some(v);
                            }
                        }
                        b"header" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                sp.margin_header = Some(v);
                            }
                        }
                        b"footer" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                sp.margin_footer = Some(v);
                            }
                        }
                        b"gutter" => {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<f64>()
                            {
                                sp.margin_gutter = Some(v);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        b"cols" if ctx.state == ElementState::SectionProps => {
            if let Some(ref mut sp) = ctx.section_properties {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"num" {
                        if let Ok(v) = String::from_utf8_lossy(&attr.value)
                            .parse::<u32>()
                        {
                            sp.columns = Some(v);
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
    in_body: bool,
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
            b"dstrike" => {
                if let Some(ref mut run) = ctx.current_run {
                    run.properties.double_strikethrough = Some(!is_val_false(e));
                }
            }
            b"smallCaps" => {
                if let Some(ref mut run) = ctx.current_run {
                    run.properties.small_caps = Some(!is_val_false(e));
                }
            }
            b"caps" => {
                if let Some(ref mut run) = ctx.current_run {
                    run.properties.all_caps = Some(!is_val_false(e));
                }
            }
            b"vertAlign" => {
                if let Some(ref mut run) = ctx.current_run {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            let val = String::from_utf8_lossy(&attr.value);
                            match val.as_ref() {
                                "superscript" => {
                                    run.properties.superscript = Some(true);
                                    run.properties.subscript = Some(false);
                                }
                                "subscript" => {
                                    run.properties.subscript = Some(true);
                                    run.properties.superscript = Some(false);
                                }
                                _ => {}
                            }
                        }
                    }
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
            b"shd" => {
                if let Some(ref mut run) = ctx.current_run {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"fill" {
                            let fill =
                                String::from_utf8_lossy(&attr.value).to_string();
                            if fill != "auto" {
                                run.properties.background_color = Some(fill);
                            }
                        }
                    }
                }
            }
            b"spacing" => {
                if let Some(ref mut run) = ctx.current_run {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            if let Ok(v) =
                                String::from_utf8_lossy(&attr.value).parse::<f64>()
                            {
                                run.properties.spacing = Some(v);
                            }
                        }
                    }
                }
            }
            _ => {}
        },

        // ---- Run-level elements (inside <w:r> but outside rPr) ----
        ElementState::Run => match local.as_ref() {
            b"br" => {
                if let Some(ref mut run) = ctx.current_run {
                    // Check for w:type="page" for page break, otherwise line break
                    let mut is_page_break = false;
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"type" {
                            let val = String::from_utf8_lossy(&attr.value);
                            if val == "page" {
                                is_page_break = true;
                            }
                        }
                    }
                    if is_page_break {
                        run.properties.page_break = Some(true);
                    } else {
                        run.properties.line_break = Some(true);
                    }
                }
            }
            b"tab" => {
                if let Some(ref mut run) = ctx.current_run {
                    run.properties.tab = Some(true);
                }
            }
            b"footnoteReference" => {
                if let Some(ref mut run) = ctx.current_run {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"id" {
                            run.properties.footnote_ref = Some(
                                String::from_utf8_lossy(&attr.value).to_string(),
                            );
                        }
                    }
                }
            }
            b"endnoteReference" => {
                if let Some(ref mut run) = ctx.current_run {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"id" {
                            run.properties.endnote_ref = Some(
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
                            para.properties.style_id = Some(val.clone());
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
                            b"line" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    para.properties.line_spacing = Some(v);
                                }
                            }
                            b"lineRule" => {
                                let rule_val = String::from_utf8_lossy(&attr.value);
                                para.properties.line_spacing_rule =
                                    Some(parse_line_spacing_rule(&rule_val));
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
                            b"hanging" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    para.properties.indent_hanging = Some(v);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            b"pageBreakBefore" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    para.properties.page_break_before = Some(!is_val_false(e));
                }
            }
            b"keepNext" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    para.properties.keep_next = Some(!is_val_false(e));
                }
            }
            b"keepLines" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    para.properties.keep_lines = Some(!is_val_false(e));
                }
            }
            b"widowControl" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    para.properties.widow_control = Some(!is_val_false(e));
                }
            }
            b"shd" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"fill" {
                            let fill =
                                String::from_utf8_lossy(&attr.value).to_string();
                            if fill != "auto" {
                                para.properties.shading = Some(fill);
                            }
                        }
                    }
                }
            }
            _ => {}
        },

        // ---- Paragraph borders ----
        ElementState::ParagraphBorders => match local.as_ref() {
            b"top" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    para.properties.border_top = Some(parse_border(e));
                }
            }
            b"bottom" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    para.properties.border_bottom = Some(parse_border(e));
                }
            }
            _ => {}
        },

        // ---- Paragraph tab stops ----
        ElementState::ParagraphTabs => {
            if local.as_ref() == b"tab" {
                if let Some(ref mut para) = ctx.current_paragraph {
                    let tab_stop = parse_tab_stop(e);
                    para.properties.tab_stops.push(tab_stop);
                }
            }
        }

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
            b"tblLayout" => {
                if let Some(table) = ctx.table_stack.last_mut() {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"type" {
                            let val = String::from_utf8_lossy(&attr.value);
                            table.properties.layout = Some(parse_table_layout(&val));
                        }
                    }
                }
            }
            b"tblInd" => {
                if let Some(table) = ctx.table_stack.last_mut() {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"w" {
                            if let Ok(v) =
                                String::from_utf8_lossy(&attr.value).parse::<f64>()
                            {
                                table.properties.indent = Some(v);
                            }
                        }
                    }
                }
            }
            b"tblCellSpacing" => {
                if let Some(table) = ctx.table_stack.last_mut() {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"w" {
                            if let Ok(v) =
                                String::from_utf8_lossy(&attr.value).parse::<f64>()
                            {
                                table.properties.cell_spacing = Some(v);
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

        // ---- Table row properties ----
        ElementState::TableRowProps => match local.as_ref() {
            b"trHeight" => {
                if let Some(ref mut row) = ctx.current_row {
                    for attr in e.attributes().flatten() {
                        match attr.key.local_name().as_ref() {
                            b"val" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    row.properties.height = Some(v);
                                }
                            }
                            b"hRule" => {
                                row.properties.height_rule = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
            b"tblHeader" => {
                if let Some(ref mut row) = ctx.current_row {
                    row.properties.is_header = Some(!is_val_false(e));
                }
            }
            b"cantSplit" => {
                if let Some(ref mut row) = ctx.current_row {
                    row.properties.cant_split = Some(!is_val_false(e));
                }
            }
            _ => {}
        },

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
            b"gridSpan" => {
                if let Some(ref mut cell) = ctx.current_cell {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            if let Ok(span) = String::from_utf8_lossy(&attr.value)
                                .parse::<u32>()
                            {
                                cell.properties.grid_span = Some(span);
                            }
                        }
                    }
                }
            }
            b"vMerge" => {
                if let Some(ref mut cell) = ctx.current_cell {
                    let mut merge_val = VerticalMerge::Continue;
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            let val = String::from_utf8_lossy(&attr.value);
                            merge_val = parse_vertical_merge(&val);
                        }
                    }
                    cell.properties.vertical_merge = Some(merge_val);
                    // Also set the legacy vertical_align field for backward compatibility
                    let val_str = match cell.properties.vertical_merge {
                        Some(VerticalMerge::Restart) => "restart",
                        Some(VerticalMerge::Continue) => "continue",
                        None => "continue",
                    };
                    cell.properties.vertical_align = Some(val_str.to_string());
                }
            }
            b"vAlign" => {
                if let Some(ref mut cell) = ctx.current_cell {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            cell.properties.vertical_align = Some(
                                String::from_utf8_lossy(&attr.value).to_string(),
                            );
                        }
                    }
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
            b"textDirection" => {
                if let Some(ref mut cell) = ctx.current_cell {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            cell.properties.text_direction = Some(
                                String::from_utf8_lossy(&attr.value).to_string(),
                            );
                        }
                    }
                }
            }
            b"noWrap" => {
                if let Some(ref mut cell) = ctx.current_cell {
                    cell.properties.no_wrap = Some(!is_val_false(e));
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

        // ---- Cell margins / padding ----
        ElementState::CellMargins => {
            if let Some(ref mut cell) = ctx.current_cell {
                let mut width_val: Option<f64> = None;
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"w" {
                        if let Ok(v) =
                            String::from_utf8_lossy(&attr.value).parse::<f64>()
                        {
                            width_val = Some(v);
                        }
                    }
                }
                if let Some(w) = width_val {
                    match local.as_ref() {
                        b"top" => cell.properties.padding_top = Some(w),
                        b"bottom" => cell.properties.padding_bottom = Some(w),
                        b"left" | b"start" => cell.properties.padding_left = Some(w),
                        b"right" | b"end" => cell.properties.padding_right = Some(w),
                        _ => {}
                    }
                }
            }
        }

        // ---- Section properties ----
        ElementState::SectionProps => match local.as_ref() {
            b"pgSz" => {
                if let Some(ref mut sp) = ctx.section_properties {
                    for attr in e.attributes().flatten() {
                        match attr.key.local_name().as_ref() {
                            b"w" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    sp.page_width = Some(v);
                                }
                            }
                            b"h" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    sp.page_height = Some(v);
                                }
                            }
                            b"orient" => {
                                let val = String::from_utf8_lossy(&attr.value);
                                sp.page_orientation =
                                    Some(parse_page_orientation(&val));
                            }
                            _ => {}
                        }
                    }
                }
            }
            b"pgMar" => {
                if let Some(ref mut sp) = ctx.section_properties {
                    for attr in e.attributes().flatten() {
                        match attr.key.local_name().as_ref() {
                            b"top" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    sp.margin_top = Some(v);
                                }
                            }
                            b"right" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    sp.margin_right = Some(v);
                                }
                            }
                            b"bottom" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    sp.margin_bottom = Some(v);
                                }
                            }
                            b"left" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    sp.margin_left = Some(v);
                                }
                            }
                            b"header" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    sp.margin_header = Some(v);
                                }
                            }
                            b"footer" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    sp.margin_footer = Some(v);
                                }
                            }
                            b"gutter" => {
                                if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                    .parse::<f64>()
                                {
                                    sp.margin_gutter = Some(v);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            b"cols" => {
                if let Some(ref mut sp) = ctx.section_properties {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"num" {
                            if let Ok(v) = String::from_utf8_lossy(&attr.value)
                                .parse::<u32>()
                            {
                                sp.columns = Some(v);
                            }
                        }
                    }
                }
            }
            _ => {}
        },

        // ---- Bookmark and other paragraph-level elements ----
        ElementState::Paragraph | ElementState::Hyperlink => match local.as_ref() {
            b"bookmarkStart" => {
                if let Some(ref mut para) = ctx.current_paragraph {
                    let mut bk_id = String::new();
                    let mut bk_name = String::new();
                    for attr in e.attributes().flatten() {
                        match attr.key.local_name().as_ref() {
                            b"id" => {
                                bk_id =
                                    String::from_utf8_lossy(&attr.value).to_string();
                            }
                            b"name" => {
                                bk_name =
                                    String::from_utf8_lossy(&attr.value).to_string();
                            }
                            _ => {}
                        }
                    }
                    para.bookmarks.push(Bookmark {
                        id: bk_id,
                        name: bk_name,
                    });
                }
            }
            _ => {}
        },

        _ => {
            // Also handle bookmarkStart when in other paragraph-child states
            // (it can appear between runs at various levels)
            if local.as_ref() == b"bookmarkStart" && ctx.in_paragraph() {
                if let Some(ref mut para) = ctx.current_paragraph {
                    let mut bk_id = String::new();
                    let mut bk_name = String::new();
                    for attr in e.attributes().flatten() {
                        match attr.key.local_name().as_ref() {
                            b"id" => {
                                bk_id =
                                    String::from_utf8_lossy(&attr.value).to_string();
                            }
                            b"name" => {
                                bk_name =
                                    String::from_utf8_lossy(&attr.value).to_string();
                            }
                            _ => {}
                        }
                    }
                    para.bookmarks.push(Bookmark {
                        id: bk_id,
                        name: bk_name,
                    });
                }
            }

            // Handle sectPr as empty element at body level
            if local.as_ref() == b"sectPr" && in_body && ctx.state == ElementState::None {
                // Empty sectPr -- unlikely but handle for completeness
                // Already handled in the Start branch; nothing to do for empty
            }
        }
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

/// Parse a `LineSpacingRule` from an OOXML `w:lineRule` attribute value.
fn parse_line_spacing_rule(val: &str) -> LineSpacingRule {
    match val {
        "exact" => LineSpacingRule::Exact,
        "atLeast" => LineSpacingRule::AtLeast,
        _ => LineSpacingRule::Auto,
    }
}

/// Parse a `PageOrientation` from an OOXML `w:orient` attribute value.
fn parse_page_orientation(val: &str) -> PageOrientation {
    match val {
        "landscape" => PageOrientation::Landscape,
        _ => PageOrientation::Portrait,
    }
}

/// Parse a `VerticalMerge` from an OOXML `w:val` attribute value.
fn parse_vertical_merge(val: &str) -> VerticalMerge {
    match val {
        "restart" => VerticalMerge::Restart,
        _ => VerticalMerge::Continue,
    }
}

/// Parse a `TableLayout` from an OOXML `w:type` attribute value.
fn parse_table_layout(val: &str) -> TableLayout {
    match val {
        "fixed" => TableLayout::Fixed,
        _ => TableLayout::Autofit,
    }
}

/// Parse a `TabStop` from a `<w:tab>` element inside `<w:tabs>`.
fn parse_tab_stop(e: &quick_xml::events::BytesStart<'_>) -> TabStop {
    let mut position: f64 = 0.0;
    let mut alignment = TabStopAlignment::Left;
    let mut leader: Option<String> = None;

    for attr in e.attributes().flatten() {
        match attr.key.local_name().as_ref() {
            b"pos" => {
                if let Ok(v) = String::from_utf8_lossy(&attr.value).parse::<f64>() {
                    position = v;
                }
            }
            b"val" => {
                let val = String::from_utf8_lossy(&attr.value);
                alignment = parse_tab_stop_alignment(&val);
            }
            b"leader" => {
                let val = String::from_utf8_lossy(&attr.value).to_string();
                if val != "none" {
                    leader = Some(val);
                }
            }
            _ => {}
        }
    }

    TabStop {
        position,
        alignment,
        leader,
    }
}

/// Parse a `TabStopAlignment` from an OOXML `w:val` attribute value.
fn parse_tab_stop_alignment(val: &str) -> TabStopAlignment {
    match val {
        "center" => TabStopAlignment::Center,
        "right" | "end" => TabStopAlignment::Right,
        "decimal" => TabStopAlignment::Decimal,
        _ => TabStopAlignment::Left,
    }
}
