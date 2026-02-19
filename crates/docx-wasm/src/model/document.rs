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
    #[serde(default)]
    pub comments: Vec<Comment>,
    #[serde(default)]
    pub footnotes: Vec<Footnote>,
    #[serde(default)]
    pub endnotes: Vec<Endnote>,
    #[serde(default)]
    pub section_properties: Option<SectionProperties>,
}

impl Document {
    /// Returns a default empty document containing a single empty paragraph.
    pub fn new() -> Self {
        let empty_paragraph = Paragraph {
            id: Uuid::new_v4().to_string(),
            runs: Vec::new(),
            properties: ParagraphProperties::default(),
            bookmarks: Vec::new(),
        };

        Self {
            body: vec![BlockElement::Paragraph(empty_paragraph)],
            styles: Vec::new(),
            numbering: Vec::new(),
            headers: Vec::new(),
            footers: Vec::new(),
            images: Vec::new(),
            comments: Vec::new(),
            footnotes: Vec::new(),
            endnotes: Vec::new(),
            section_properties: None,
        }
    }

    /// Count the total number of words across all paragraphs in the document.
    pub fn word_count(&self) -> usize {
        fn count_block_words(blocks: &[BlockElement]) -> usize {
            blocks.iter().map(|b| match b {
                BlockElement::Paragraph(p) => {
                    p.runs.iter()
                        .map(|r| r.text.split_whitespace().count())
                        .sum::<usize>()
                }
                BlockElement::Table(t) => {
                    t.rows.iter()
                        .flat_map(|row| &row.cells)
                        .map(|cell| count_block_words(&cell.content))
                        .sum::<usize>()
                }
            }).sum()
        }
        count_block_words(&self.body)
    }

    /// Count the total number of characters across all paragraphs.
    pub fn char_count(&self) -> usize {
        fn count_block_chars(blocks: &[BlockElement]) -> usize {
            blocks.iter().map(|b| match b {
                BlockElement::Paragraph(p) => {
                    p.runs.iter().map(|r| r.text.len()).sum::<usize>()
                }
                BlockElement::Table(t) => {
                    t.rows.iter()
                        .flat_map(|row| &row.cells)
                        .map(|cell| count_block_chars(&cell.content))
                        .sum::<usize>()
                }
            }).sum()
        }
        count_block_chars(&self.body)
    }

    /// Count the total number of paragraphs in the document.
    pub fn paragraph_count(&self) -> usize {
        fn count_block_paragraphs(blocks: &[BlockElement]) -> usize {
            blocks.iter().map(|b| match b {
                BlockElement::Paragraph(_) => 1,
                BlockElement::Table(t) => {
                    t.rows.iter()
                        .flat_map(|row| &row.cells)
                        .map(|cell| count_block_paragraphs(&cell.content))
                        .sum::<usize>()
                }
            }).sum()
        }
        count_block_paragraphs(&self.body)
    }

    /// Extract all text from the document as a single string.
    pub fn extract_text(&self) -> String {
        let mut result = String::new();
        fn extract_block_text(blocks: &[BlockElement], out: &mut String) {
            for block in blocks {
                match block {
                    BlockElement::Paragraph(p) => {
                        for run in &p.runs {
                            out.push_str(&run.text);
                        }
                        out.push('\n');
                    }
                    BlockElement::Table(t) => {
                        for row in &t.rows {
                            for cell in &row.cells {
                                extract_block_text(&cell.content, out);
                            }
                        }
                    }
                }
            }
        }
        extract_block_text(&self.body, &mut result);
        result
    }

    /// Compute document statistics.
    pub fn statistics(&self) -> DocumentStatistics {
        fn count_tables(blocks: &[BlockElement]) -> usize {
            blocks.iter().map(|b| match b {
                BlockElement::Paragraph(_) => 0,
                BlockElement::Table(t) => {
                    1 + t.rows.iter()
                        .flat_map(|row| &row.cells)
                        .map(|cell| count_tables(&cell.content))
                        .sum::<usize>()
                }
            }).sum()
        }

        DocumentStatistics {
            word_count: self.word_count(),
            char_count: self.char_count(),
            paragraph_count: self.paragraph_count(),
            page_count_estimate: std::cmp::max(1, self.paragraph_count() / 25),
            image_count: self.images.len(),
            table_count: count_tables(&self.body),
            header_count: self.headers.len(),
            footer_count: self.footers.len(),
            style_count: self.styles.len(),
            comment_count: self.comments.len(),
            footnote_count: self.footnotes.len(),
            endnote_count: self.endnotes.len(),
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
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ParagraphProperties {
    pub alignment: Option<Alignment>,
    pub heading_level: Option<u8>,
    pub numbering: Option<NumberingRef>,
    pub spacing_before: Option<f64>,
    pub spacing_after: Option<f64>,
    #[serde(default)]
    pub line_spacing: Option<f64>,
    #[serde(default)]
    pub line_spacing_rule: Option<LineSpacingRule>,
    pub indent_left: Option<f64>,
    pub indent_right: Option<f64>,
    pub indent_first_line: Option<f64>,
    #[serde(default)]
    pub indent_hanging: Option<f64>,
    #[serde(default)]
    pub page_break_before: Option<bool>,
    #[serde(default)]
    pub keep_next: Option<bool>,
    #[serde(default)]
    pub keep_lines: Option<bool>,
    #[serde(default)]
    pub widow_control: Option<bool>,
    #[serde(default)]
    pub style_id: Option<String>,
    #[serde(default)]
    pub border_bottom: Option<Border>,
    #[serde(default)]
    pub border_top: Option<Border>,
    #[serde(default)]
    pub shading: Option<String>,
    #[serde(default)]
    pub tab_stops: Vec<TabStop>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NumberingRef {
    pub num_id: String,
    pub level: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum LineSpacingRule {
    Auto,
    Exact,
    AtLeast,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TabStop {
    pub position: f64,
    pub alignment: TabStopAlignment,
    pub leader: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TabStopAlignment {
    Left,
    Center,
    Right,
    Decimal,
}

// ---------------------------------------------------------------------------
// Alignment
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
    #[serde(default)]
    pub double_strikethrough: Option<bool>,
    #[serde(default)]
    pub superscript: Option<bool>,
    #[serde(default)]
    pub subscript: Option<bool>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub color: Option<String>,
    pub highlight: Option<String>,
    #[serde(default)]
    pub background_color: Option<String>,
    /// Reference to an `ImageData::id` for an inline image.
    pub inline_image: Option<String>,
    /// URL for a hyperlink (set when this run is inside a hyperlink element).
    #[serde(default)]
    pub hyperlink_url: Option<String>,
    /// Tooltip for a hyperlink.
    #[serde(default)]
    pub hyperlink_tooltip: Option<String>,
    /// Whether this run contains a line break.
    #[serde(default)]
    pub line_break: Option<bool>,
    /// Whether this run contains a page break.
    #[serde(default)]
    pub page_break: Option<bool>,
    /// Whether this run contains a tab character.
    #[serde(default)]
    pub tab: Option<bool>,
    /// Small caps.
    #[serde(default)]
    pub small_caps: Option<bool>,
    /// All caps.
    #[serde(default)]
    pub all_caps: Option<bool>,
    /// Character spacing (in twips).
    #[serde(default)]
    pub spacing: Option<f64>,
    /// Reference to a footnote ID.
    #[serde(default)]
    pub footnote_ref: Option<String>,
    /// Reference to an endnote ID.
    #[serde(default)]
    pub endnote_ref: Option<String>,
}

// ---------------------------------------------------------------------------
// Bookmark
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Bookmark {
    pub id: String,
    pub name: String,
}

// ---------------------------------------------------------------------------
// Comment
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub date: Option<String>,
    pub initials: Option<String>,
    pub content: Vec<BlockElement>,
}

// ---------------------------------------------------------------------------
// Footnotes & Endnotes
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Footnote {
    pub id: String,
    pub content: Vec<BlockElement>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Endnote {
    pub id: String,
    pub content: Vec<BlockElement>,
}

// ---------------------------------------------------------------------------
// Section Properties (page layout)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SectionProperties {
    pub page_width: Option<f64>,
    pub page_height: Option<f64>,
    pub margin_top: Option<f64>,
    pub margin_right: Option<f64>,
    pub margin_bottom: Option<f64>,
    pub margin_left: Option<f64>,
    pub margin_header: Option<f64>,
    pub margin_footer: Option<f64>,
    pub margin_gutter: Option<f64>,
    pub page_orientation: Option<PageOrientation>,
    pub columns: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PageOrientation {
    Portrait,
    Landscape,
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
    #[serde(default)]
    pub properties: TableRowProperties,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TableRowProperties {
    pub height: Option<f64>,
    pub height_rule: Option<String>,
    pub is_header: Option<bool>,
    pub cant_split: Option<bool>,
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
    #[serde(default)]
    pub cell_spacing: Option<f64>,
    #[serde(default)]
    pub layout: Option<TableLayout>,
    #[serde(default)]
    pub indent: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TableLayout {
    Fixed,
    Autofit,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TableCellProperties {
    pub width: Option<f64>,
    pub vertical_align: Option<String>,
    pub borders: Option<TableBorders>,
    pub shading: Option<String>,
    #[serde(default)]
    pub grid_span: Option<u32>,
    #[serde(default)]
    pub vertical_merge: Option<VerticalMerge>,
    #[serde(default)]
    pub text_direction: Option<String>,
    #[serde(default)]
    pub no_wrap: Option<bool>,
    #[serde(default)]
    pub padding_top: Option<f64>,
    #[serde(default)]
    pub padding_bottom: Option<f64>,
    #[serde(default)]
    pub padding_left: Option<f64>,
    #[serde(default)]
    pub padding_right: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VerticalMerge {
    Restart,
    Continue,
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
    #[serde(default)]
    pub description: Option<String>,
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

// ---------------------------------------------------------------------------
// Document statistics (returned via WASM)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DocumentStatistics {
    pub word_count: usize,
    pub char_count: usize,
    pub paragraph_count: usize,
    pub page_count_estimate: usize,
    pub image_count: usize,
    pub table_count: usize,
    pub header_count: usize,
    pub footer_count: usize,
    pub style_count: usize,
    pub comment_count: usize,
    pub footnote_count: usize,
    pub endnote_count: usize,
}
