use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Top-level document
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Document {
    pub body: Vec<BlockElement>,
    pub styles: Vec<StyleDef>,
    pub numbering: Vec<ListDef>,
    pub headers: Vec<Header>,
    pub footers: Vec<Footer>,
    pub images: Vec<ImageData>,
}

impl Document {
    /// Returns a default empty document containing a single empty paragraph.
    pub fn new() -> Self {
        let empty_paragraph = Paragraph {
            id: Uuid::new_v4().to_string(),
            runs: Vec::new(),
            properties: ParagraphProperties::default(),
        };

        Self {
            body: vec![BlockElement::Paragraph(empty_paragraph)],
            styles: Vec::new(),
            numbering: Vec::new(),
            headers: Vec::new(),
            footers: Vec::new(),
            images: Vec::new(),
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block-level elements
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BlockElement {
    Paragraph(Paragraph),
    Table(Table),
}

// ---------------------------------------------------------------------------
// Paragraph
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Paragraph {
    pub id: String,
    pub runs: Vec<Run>,
    pub properties: ParagraphProperties,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ParagraphProperties {
    pub alignment: Option<Alignment>,
    pub heading_level: Option<u8>,
    pub numbering: Option<NumberingRef>,
    pub spacing_before: Option<f64>,
    pub spacing_after: Option<f64>,
    pub indent_left: Option<f64>,
    pub indent_right: Option<f64>,
    pub indent_first_line: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NumberingRef {
    pub num_id: String,
    pub level: u32,
}

// ---------------------------------------------------------------------------
// Alignment
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justify,
}

// ---------------------------------------------------------------------------
// Run (inline content)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Run {
    pub id: String,
    pub text: String,
    pub properties: RunProperties,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RunProperties {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub color: Option<String>,
    pub highlight: Option<String>,
    /// Reference to an `ImageData::id` for an inline image.
    pub inline_image: Option<String>,
}

// ---------------------------------------------------------------------------
// Table
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Table {
    pub id: String,
    pub rows: Vec<TableRow>,
    pub properties: TableProperties,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TableRow {
    pub id: String,
    pub cells: Vec<TableCell>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TableCell {
    pub id: String,
    pub content: Vec<BlockElement>,
    pub properties: TableCellProperties,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TableProperties {
    pub width: Option<f64>,
    pub borders: Option<TableBorders>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TableCellProperties {
    pub width: Option<f64>,
    pub vertical_align: Option<String>,
    pub borders: Option<TableBorders>,
    pub shading: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TableBorders {
    pub top: Option<Border>,
    pub bottom: Option<Border>,
    pub left: Option<Border>,
    pub right: Option<Border>,
    pub inside_h: Option<Border>,
    pub inside_v: Option<Border>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Border {
    pub style: String,
    pub size: f64,
    pub color: String,
}

// ---------------------------------------------------------------------------
// Images
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImageData {
    pub id: String,
    pub data_base64: String,
    pub content_type: String,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

// ---------------------------------------------------------------------------
// Numbering / Lists
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListDef {
    pub num_id: String,
    pub levels: Vec<ListLevel>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListLevel {
    pub level: u32,
    pub format: String,
    pub text: String,
    pub start: u32,
}

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StyleDef {
    pub id: String,
    pub name: String,
    pub style_type: String,
    pub based_on: Option<String>,
    pub run_properties: Option<RunProperties>,
    pub paragraph_properties: Option<ParagraphProperties>,
}

// ---------------------------------------------------------------------------
// Headers & Footers
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Header {
    pub id: String,
    pub content: Vec<BlockElement>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Footer {
    pub id: String,
    pub content: Vec<BlockElement>,
}
