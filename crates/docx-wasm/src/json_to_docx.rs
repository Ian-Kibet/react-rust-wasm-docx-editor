use serde_json::Value;
use wasm_bindgen::prelude::*;

/// Convert a JSON string (in docx-rs format) back to DOCX bytes.
/// This parses the JSON produced by `readDocx` and reconstructs the document
/// using docx-rs's builder API, then builds and packs it into a ZIP.
#[allow(non_snake_case)]
#[wasm_bindgen]
pub fn generateDocx(json: &str) -> Result<Vec<u8>, JsValue> {
    let v: Value =
        serde_json::from_str(json).map_err(|e| JsValue::from_str(&format!("JSON parse error: {e}")))?;

    let mut docx = build_docx_from_json(&v).map_err(|e| JsValue::from_str(&e))?;

    // Check if document has numberings
    let has_numberings = has_numberings_in_doc(&v);
    if has_numberings {
        docx.document_rels.has_numberings = true;
    }

    let buf = Vec::new();
    let mut cur = std::io::Cursor::new(buf);
    let res = docx.build().pack(&mut cur);
    if let Err(e) = res {
        return Err(JsValue::from_str(&format!("DOCX pack error: {e:?}")));
    }
    Ok(cur.into_inner())
}

/// Alias for our bridge.ts compatibility
#[wasm_bindgen]
pub fn parse_docx(data: &[u8]) -> Result<String, JsValue> {
    let mut d = docx_rs::read_docx(data);
    match d {
        Ok(ref mut d) => Ok(d.json()),
        Err(e) => Err(e.to_string().into()),
    }
}

#[wasm_bindgen]
pub fn generate_docx(json: &str) -> Result<Vec<u8>, JsValue> {
    generateDocx(json)
}

#[wasm_bindgen]
pub fn version() -> String {
    "0.4.19-docx-rs".to_string()
}

fn has_numberings_in_doc(v: &Value) -> bool {
    if let Some(numberings) = v.get("numberings") {
        if let Some(nums) = numberings.get("numberings") {
            if let Some(arr) = nums.as_array() {
                return !arr.is_empty();
            }
        }
    }
    false
}

fn build_docx_from_json(v: &Value) -> Result<docx_rs::Docx, String> {
    let mut docx = docx_rs::Docx::new();

    // Parse styles
    if let Some(styles_val) = v.get("styles") {
        parse_styles(&mut docx, styles_val);
    }

    // Parse numberings
    if let Some(numberings_val) = v.get("numberings") {
        parse_numberings(&mut docx, numberings_val);
    }

    // Parse settings
    if let Some(settings_val) = v.get("settings") {
        parse_settings(&mut docx, settings_val);
    }

    // Parse document (body content + section properties)
    if let Some(doc_val) = v.get("document") {
        parse_document(&mut docx, doc_val);
    }

    // Parse comments
    if let Some(comments_val) = v.get("comments") {
        parse_comments(&mut docx, comments_val);
    }

    // Parse footnotes
    if let Some(footnotes_val) = v.get("footnotes") {
        parse_footnotes(&mut docx, footnotes_val);
    }

    Ok(docx)
}

// ======================= Document =======================

fn parse_document(docx: &mut docx_rs::Docx, v: &Value) {
    // Parse children (paragraphs, tables)
    if let Some(children) = v.get("children").and_then(|c| c.as_array()) {
        for child in children {
            let child_type = child.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match child_type {
                "paragraph" => {
                    if let Some(data) = child.get("data") {
                        let p = parse_paragraph(data);
                        docx.document = docx.document.clone().add_paragraph(p);
                    }
                }
                "table" => {
                    if let Some(data) = child.get("data") {
                        let t = parse_table(data);
                        docx.document = docx.document.clone().add_table(t);
                    }
                }
                _ => {} // Skip unsupported child types for now
            }
        }
    }

    // Parse section property
    if let Some(sp) = v.get("sectionProperty") {
        let section_prop = parse_section_property(sp);
        docx.document.section_property = section_prop;
    }
}

// ======================= Paragraph =======================

fn parse_paragraph(v: &Value) -> docx_rs::Paragraph {
    let mut p = docx_rs::Paragraph::new();

    // Set paragraph ID if present
    if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
        p = p.id(id);
    }

    // Parse paragraph property
    if let Some(prop) = v.get("property") {
        apply_paragraph_property(&mut p, prop);
    }

    // Parse children (runs, hyperlinks, etc.)
    if let Some(children) = v.get("children").and_then(|c| c.as_array()) {
        for child in children {
            let child_type = child.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match child_type {
                "run" => {
                    if let Some(data) = child.get("data") {
                        let r = parse_run(data);
                        p = p.add_run(r);
                    }
                }
                "hyperlink" => {
                    if let Some(data) = child.get("data") {
                        let h = parse_hyperlink(data);
                        p = p.add_hyperlink(h);
                    }
                }
                "bookmarkStart" => {
                    if let Some(data) = child.get("data") {
                        let id = data.get("id").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                        let name = data.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        p = p.add_bookmark_start(id, name);
                    }
                }
                "bookmarkEnd" => {
                    if let Some(data) = child.get("data") {
                        let id = data.get("id").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                        p = p.add_bookmark_end(id);
                    }
                }
                _ => {}
            }
        }
    }

    p
}

fn apply_paragraph_property(p: &mut docx_rs::Paragraph, prop: &Value) {
    // Alignment
    if let Some(alignment) = prop.get("alignment").and_then(|a| a.as_str()) {
        if let Some(align_type) = parse_alignment_type(alignment) {
            p.property = p.property.clone().align(align_type);
        }
    }

    // Style
    if let Some(style) = prop.get("style").and_then(|s| s.as_str()) {
        if !style.is_empty() {
            p.property = p.property.clone().style(style);
        }
    }

    // Indent
    if let Some(indent) = prop.get("indent") {
        let left = indent.get("left").and_then(|l| l.as_i64());
        let right = indent.get("end").and_then(|r| r.as_i64());
        let start_chars = indent.get("startChars").and_then(|s| s.as_i64());

        let special = if let Some(first_line) = indent.get("firstLineChars").and_then(|f| f.as_i64()) {
            Some(docx_rs::SpecialIndentType::FirstLine(first_line as i32))
        } else if let Some(first_line) = indent.get("firstLine").and_then(|f| f.as_i64()) {
            Some(docx_rs::SpecialIndentType::FirstLine(first_line as i32))
        } else if let Some(hanging) = indent.get("hangingChars").and_then(|h| h.as_i64()) {
            Some(docx_rs::SpecialIndentType::Hanging(hanging as i32))
        } else if let Some(hanging) = indent.get("hanging").and_then(|h| h.as_i64()) {
            Some(docx_rs::SpecialIndentType::Hanging(hanging as i32))
        } else {
            None
        };

        p.property = p.property.clone().indent(
            left.map(|l| l as i32),
            special,
            right.map(|r| r as i32),
            start_chars.map(|s| s as i32),
        );
    }

    // Line spacing
    if let Some(ls) = prop.get("lineSpacing") {
        let mut spacing = docx_rs::LineSpacing::new();
        if let Some(before) = ls.get("before").and_then(|b| b.as_u64()) {
            spacing = spacing.before(before as u32);
        }
        if let Some(after) = ls.get("after").and_then(|a| a.as_u64()) {
            spacing = spacing.after(after as u32);
        }
        if let Some(line) = ls.get("line").and_then(|l| l.as_u64()) {
            spacing = spacing.line(line as i32);
        }
        if let Some(rule) = ls.get("lineRule").and_then(|r| r.as_str()) {
            let rule_type = match rule {
                "auto" | "Auto" => docx_rs::LineSpacingType::Auto,
                "atLeast" | "AtLeast" => docx_rs::LineSpacingType::AtLeast,
                "exact" | "Exact" => docx_rs::LineSpacingType::Exact,
                _ => docx_rs::LineSpacingType::Auto,
            };
            spacing = spacing.line_rule(rule_type);
        }
        p.property = p.property.clone().line_spacing(spacing);
    }

    // Numbering
    if let Some(numbering) = prop.get("numbering") {
        if let (Some(id), Some(level)) = (
            numbering.get("id").and_then(|i| i.as_u64()),
            numbering.get("level").and_then(|l| l.as_u64()),
        ) {
            p.property = p.property.clone().numbering(
                docx_rs::NumberingId::new(id as usize),
                docx_rs::IndentLevel::new(level as usize),
            );
        }
    }

    // Keep next / keep lines / page break before / widow control
    if let Some(keep_next) = prop.get("keepNext").and_then(|k| k.as_bool()) {
        p.property = p.property.clone().keep_next(keep_next);
    }
    if let Some(keep_lines) = prop.get("keepLines").and_then(|k| k.as_bool()) {
        p.property = p.property.clone().keep_lines(keep_lines);
    }
    if let Some(page_break) = prop.get("pageBreakBefore").and_then(|k| k.as_bool()) {
        p.property = p.property.clone().page_break_before(page_break);
    }
    if let Some(widow) = prop.get("widowControl").and_then(|k| k.as_bool()) {
        p.property = p.property.clone().widow_control(widow);
    }

    // Tabs
    if let Some(tabs) = prop.get("tabs").and_then(|t| t.as_array()) {
        for tab in tabs {
            if let Some(t) = parse_tab(tab) {
                p.property = p.property.clone().add_tab(t);
            }
        }
    }

    // Run property on paragraph (for default formatting)
    if let Some(rp) = prop.get("runProperty") {
        let run_prop = parse_run_property(rp);
        p.property.run_property = run_prop;
    }
}

fn parse_tab(v: &Value) -> Option<docx_rs::Tab> {
    let val = v.get("val").and_then(|v| v.as_str())?;
    let leader = v.get("leader").and_then(|l| l.as_str());
    let pos = v.get("pos").and_then(|p| p.as_u64());

    let tab_type = match val {
        "left" => Some(docx_rs::TabValueType::Left),
        "center" => Some(docx_rs::TabValueType::Center),
        "right" => Some(docx_rs::TabValueType::Right),
        "decimal" => Some(docx_rs::TabValueType::Decimal),
        "bar" => Some(docx_rs::TabValueType::Bar),
        "clear" => Some(docx_rs::TabValueType::Clear),
        "num" => Some(docx_rs::TabValueType::Num),
        _ => None,
    }?;

    let leader_type = leader.and_then(|l| match l {
        "dot" => Some(docx_rs::TabLeaderType::Dot),
        "heavy" => Some(docx_rs::TabLeaderType::Heavy),
        "hyphen" => Some(docx_rs::TabLeaderType::Hyphen),
        "middleDot" => Some(docx_rs::TabLeaderType::MiddleDot),
        "none" => Some(docx_rs::TabLeaderType::None),
        "underscore" => Some(docx_rs::TabLeaderType::Underscore),
        _ => None,
    });

    let mut tab = docx_rs::Tab::new().val(tab_type);
    if let Some(lt) = leader_type {
        tab = tab.leader(lt);
    }
    if let Some(p) = pos {
        tab = tab.pos(p as usize);
    }
    Some(tab)
}

// ======================= Run =======================

fn parse_run(v: &Value) -> docx_rs::Run {
    let mut r = docx_rs::Run::new();

    // Parse run property
    if let Some(rp) = v.get("runProperty") {
        r.run_property = parse_run_property(rp);
    }

    // Parse children
    if let Some(children) = v.get("children").and_then(|c| c.as_array()) {
        for child in children {
            let child_type = child.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match child_type {
                "text" => {
                    if let Some(data) = child.get("data") {
                        let text = data.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        r = r.add_text(text);
                    }
                }
                "tab" => {
                    r = r.add_tab();
                }
                "break" => {
                    if let Some(data) = child.get("data") {
                        let break_type = data.get("breakType").and_then(|b| b.as_str()).unwrap_or("textWrapping");
                        match break_type {
                            "page" => r = r.add_break(docx_rs::BreakType::Page),
                            "column" => r = r.add_break(docx_rs::BreakType::Column),
                            _ => r = r.add_break(docx_rs::BreakType::TextWrapping),
                        }
                    } else {
                        r = r.add_break(docx_rs::BreakType::TextWrapping);
                    }
                }
                "deleteText" => {
                    if let Some(data) = child.get("data") {
                        let text = data.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        r = r.add_delete_text(text);
                    }
                }
                _ => {} // Skip unsupported run children for now
            }
        }
    }

    r
}

fn parse_run_property(v: &Value) -> docx_rs::RunProperty {
    let mut rp = docx_rs::RunProperty::new();

    // Bold
    if let Some(bold) = v.get("bold") {
        if bold.as_bool().unwrap_or(false) {
            rp = rp.bold();
        }
    }

    // Italic
    if let Some(italic) = v.get("italic") {
        if italic.as_bool().unwrap_or(false) {
            rp = rp.italic();
        }
    }

    // Size (in half-points)
    if let Some(sz) = v.get("sz").and_then(|s| s.as_u64()) {
        rp = rp.size(sz as usize);
    }

    // Size for complex script
    if let Some(sz_cs) = v.get("szCs").and_then(|s| s.as_u64()) {
        // Set szCs through the size method which sets both, or directly
        // RunProperty doesn't have a separate szCs setter, size() sets both
        let _ = sz_cs; // szCs is set along with sz
    }

    // Color
    if let Some(color) = v.get("color").and_then(|c| c.as_str()) {
        if !color.is_empty() && color != "null" {
            rp = rp.color(color);
        }
    }

    // Highlight
    if let Some(highlight) = v.get("highlight").and_then(|h| h.as_str()) {
        if !highlight.is_empty() {
            rp = rp.highlight(highlight);
        }
    }

    // Underline
    if let Some(underline) = v.get("underline").and_then(|u| u.as_str()) {
        if !underline.is_empty() {
            rp = rp.underline(underline);
        }
    }

    // Strikethrough
    if let Some(strike) = v.get("strike") {
        if strike.as_bool().unwrap_or(false) {
            rp = rp.strike();
        }
    }

    // Double strikethrough
    if let Some(dstrike) = v.get("dstrike") {
        if dstrike.as_bool().unwrap_or(false) {
            rp = rp.dstrike();
        }
    }

    // Caps
    if let Some(caps) = v.get("caps") {
        if caps.as_bool().unwrap_or(false) {
            rp = rp.caps();
        }
    }

    // Vanish
    if let Some(vanish) = v.get("vanish") {
        if vanish.as_bool().unwrap_or(false) {
            rp = rp.vanish();
        }
    }

    // Character spacing
    if let Some(spacing) = v.get("characterSpacing").and_then(|s| s.as_i64()) {
        rp = rp.spacing(spacing as i32);
    }

    // Vertical alignment (superscript/subscript)
    if let Some(vert) = v.get("vertAlign").and_then(|v| v.as_str()) {
        match vert {
            "superscript" => rp = rp.vert_align(docx_rs::VertAlignType::SuperScript),
            "subscript" => rp = rp.vert_align(docx_rs::VertAlignType::SubScript),
            _ => {}
        }
    }

    // Fonts
    if let Some(fonts) = v.get("fonts") {
        let rf = parse_run_fonts(fonts);
        rp = rp.fonts(rf);
    }

    // Style
    if let Some(style) = v.get("style").and_then(|s| s.as_str()) {
        if !style.is_empty() {
            rp = rp.style(style);
        }
    }

    rp
}

fn parse_run_fonts(v: &Value) -> docx_rs::RunFonts {
    let mut rf = docx_rs::RunFonts::new();

    if let Some(ascii) = v.get("ascii").and_then(|a| a.as_str()) {
        rf = rf.ascii(ascii);
    }
    if let Some(hi_ansi) = v.get("hiAnsi").and_then(|h| h.as_str()) {
        rf = rf.hi_ansi(hi_ansi);
    }
    if let Some(cs) = v.get("cs").and_then(|c| c.as_str()) {
        rf = rf.cs(cs);
    }
    if let Some(east_asia) = v.get("eastAsia").and_then(|e| e.as_str()) {
        rf = rf.east_asia(east_asia);
    }

    rf
}

// ======================= Hyperlink =======================

fn parse_hyperlink(v: &Value) -> docx_rs::Hyperlink {
    let rid = v.get("id").and_then(|i| i.as_str()).unwrap_or("");
    let mut h = docx_rs::Hyperlink::new(rid, docx_rs::HyperlinkType::External);

    // Parse children (runs)
    if let Some(children) = v.get("children").and_then(|c| c.as_array()) {
        for child in children {
            let child_type = child.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if child_type == "run" {
                if let Some(data) = child.get("data") {
                    let r = parse_run(data);
                    h = h.add_run(r);
                }
            }
        }
    }

    h
}

// ======================= Table =======================

fn parse_table(v: &Value) -> docx_rs::Table {
    let mut rows = Vec::new();

    if let Some(rows_val) = v.get("rows").and_then(|r| r.as_array()) {
        for row_val in rows_val {
            if let Some(data) = row_val.get("data") {
                rows.push(parse_table_row(data));
            }
        }
    }

    let mut table = docx_rs::Table::new(rows);

    // Grid
    if let Some(grid) = v.get("grid").and_then(|g| g.as_array()) {
        let grid_widths: Vec<usize> = grid.iter().filter_map(|g| g.as_u64().map(|w| w as usize)).collect();
        table = table.set_grid(grid_widths);
    }

    // Table property
    if let Some(prop) = v.get("property") {
        apply_table_property(&mut table, prop);
    }

    table
}

fn apply_table_property(table: &mut docx_rs::Table, prop: &Value) {
    // Width
    if let Some(width) = prop.get("width") {
        let w = width.get("width").and_then(|w| w.as_u64()).unwrap_or(0) as usize;
        let wtype = width.get("widthType").and_then(|t| t.as_str()).unwrap_or("auto");
        let width_type = match wtype {
            "dxa" | "Dxa" => docx_rs::WidthType::Dxa,
            "auto" | "Auto" => docx_rs::WidthType::Auto,
            "pct" | "Pct" => docx_rs::WidthType::Pct,
            "nil" | "Nil" => docx_rs::WidthType::Nil,
            _ => docx_rs::WidthType::Auto,
        };
        *table = table.clone().width(w, width_type);
    }

    // Justification/alignment
    if let Some(just) = prop.get("justification").and_then(|j| j.as_str()) {
        let align = match just {
            "left" => docx_rs::TableAlignmentType::Left,
            "center" => docx_rs::TableAlignmentType::Center,
            "right" => docx_rs::TableAlignmentType::Right,
            _ => docx_rs::TableAlignmentType::Left,
        };
        *table = table.clone().align(align);
    }

    // Indent
    if let Some(indent) = prop.get("indent") {
        if let Some(val) = indent.get("val").and_then(|v| v.as_i64()) {
            *table = table.clone().indent(val as i32);
        }
    }

    // Layout
    if let Some(layout) = prop.get("layout").and_then(|l| l.as_str()) {
        let layout_type = match layout {
            "fixed" | "Fixed" => docx_rs::TableLayoutType::Fixed,
            _ => docx_rs::TableLayoutType::Autofit,
        };
        *table = table.clone().layout(layout_type);
    }

    // Style
    if let Some(style) = prop.get("style").and_then(|s| s.as_str()) {
        if !style.is_empty() {
            *table = table.clone().style(style);
        }
    }
}

fn parse_table_row(v: &Value) -> docx_rs::TableRow {
    let mut cells = Vec::new();

    if let Some(cells_val) = v.get("cells").and_then(|c| c.as_array()) {
        for cell_val in cells_val {
            if let Some(data) = cell_val.get("data") {
                cells.push(parse_table_cell(data));
            }
        }
    }

    let mut row = docx_rs::TableRow::new(cells);

    // Row property
    if let Some(prop) = v.get("property") {
        if let Some(height) = prop.get("rowHeight").and_then(|h| h.as_f64()) {
            row = row.row_height(height as f32);
        }
        if let Some(rule) = prop.get("heightRule").and_then(|r| r.as_str()) {
            let height_rule = match rule {
                "atLeast" | "AtLeast" => docx_rs::HeightRule::AtLeast,
                "exact" | "Exact" => docx_rs::HeightRule::Exact,
                _ => docx_rs::HeightRule::Auto,
            };
            row = row.height_rule(height_rule);
        }
    }

    row
}

fn parse_table_cell(v: &Value) -> docx_rs::TableCell {
    let mut cell = docx_rs::TableCell::new();
    // Clear the default empty paragraph that TableCell::new() adds
    cell.children = vec![];

    // Parse children (paragraphs, tables)
    if let Some(children) = v.get("children").and_then(|c| c.as_array()) {
        for child in children {
            let child_type = child.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match child_type {
                "paragraph" => {
                    if let Some(data) = child.get("data") {
                        let p = parse_paragraph(data);
                        cell = cell.add_paragraph(p);
                    }
                }
                "table" => {
                    if let Some(data) = child.get("data") {
                        let t = parse_table(data);
                        cell = cell.add_table(t);
                    }
                }
                _ => {}
            }
        }
    }

    // Ensure at least one paragraph exists (required by DOCX spec)
    if cell.children.is_empty() {
        cell = cell.add_paragraph(docx_rs::Paragraph::new());
    }

    // Parse cell property
    if let Some(prop) = v.get("property") {
        apply_table_cell_property(&mut cell, prop);
    }

    cell
}

fn apply_table_cell_property(cell: &mut docx_rs::TableCell, prop: &Value) {
    // Width
    if let Some(width) = prop.get("width") {
        let w = width.get("width").and_then(|w| w.as_u64()).unwrap_or(0) as usize;
        let wtype = width.get("widthType").and_then(|t| t.as_str()).unwrap_or("auto");
        let width_type = match wtype {
            "dxa" | "Dxa" => docx_rs::WidthType::Dxa,
            "auto" | "Auto" => docx_rs::WidthType::Auto,
            "pct" | "Pct" => docx_rs::WidthType::Pct,
            "nil" | "Nil" => docx_rs::WidthType::Nil,
            _ => docx_rs::WidthType::Auto,
        };
        *cell = cell.clone().width(w, width_type);
    }

    // Grid span
    if let Some(gs) = prop.get("gridSpan").and_then(|g| g.as_u64()) {
        if gs > 1 {
            *cell = cell.clone().grid_span(gs as usize);
        }
    }

    // Vertical merge
    if let Some(vm) = prop.get("verticalMerge").and_then(|v| v.as_str()) {
        let merge_type = match vm {
            "restart" | "Restart" => docx_rs::VMergeType::Restart,
            "continue" | "Continue" => docx_rs::VMergeType::Continue,
            _ => docx_rs::VMergeType::Continue,
        };
        *cell = cell.clone().vertical_merge(merge_type);
    }

    // Vertical alignment
    if let Some(va) = prop.get("verticalAlign").and_then(|v| v.as_str()) {
        let align = match va {
            "top" | "Top" => docx_rs::VAlignType::Top,
            "center" | "Center" => docx_rs::VAlignType::Center,
            "bottom" | "Bottom" => docx_rs::VAlignType::Bottom,
            _ => docx_rs::VAlignType::Top,
        };
        *cell = cell.clone().vertical_align(align);
    }

    // Shading
    if let Some(shading) = prop.get("shading") {
        if let Some(fill) = shading.get("fill").and_then(|f| f.as_str()) {
            let shading_type_str = shading
                .get("shdType")
                .and_then(|s| s.as_str())
                .unwrap_or("clear");
            let shd_type = shading_type_str.parse::<docx_rs::ShdType>().unwrap_or(docx_rs::ShdType::Clear);
            let color = shading.get("color").and_then(|c| c.as_str()).unwrap_or("auto");
            *cell = cell.clone().shading(docx_rs::Shading::new().shd_type(shd_type).fill(fill).color(color));
        }
    }

    // Text direction
    if let Some(td) = prop.get("textDirection").and_then(|t| t.as_str()) {
        let dir = match td {
            "tbRl" | "TbRl" => docx_rs::TextDirectionType::TbRl,
            "btLr" | "BtLr" => docx_rs::TextDirectionType::BtLr,
            "lrTbV" | "LrTbV" => docx_rs::TextDirectionType::LrTbV,
            "tbRlV" | "TbRlV" => docx_rs::TextDirectionType::TbRlV,
            "tbV" | "TbV" => docx_rs::TextDirectionType::TbV,
            _ => docx_rs::TextDirectionType::Lr,
        };
        *cell = cell.clone().text_direction(dir);
    }
}

// ======================= Section Property =======================

fn parse_section_property(v: &Value) -> docx_rs::SectionProperty {
    let mut sp = docx_rs::SectionProperty::new();

    // Page size
    if let Some(ps) = v.get("pageSize") {
        let w = ps.get("w").and_then(|w| w.as_u64()).unwrap_or(11906) as u32;
        let h = ps.get("h").and_then(|h| h.as_u64()).unwrap_or(16838) as u32;
        sp = sp.page_size(docx_rs::PageSize::new().size(w, h));

        if let Some(orient) = ps.get("orient").and_then(|o| o.as_str()) {
            if orient == "landscape" {
                sp = sp.page_orient(docx_rs::PageOrientationType::Landscape);
            }
        }
    }

    // Page margin
    if let Some(pm) = v.get("pageMargin") {
        let top = pm.get("top").and_then(|t| t.as_i64()).unwrap_or(1440) as i32;
        let right = pm.get("right").and_then(|r| r.as_i64()).unwrap_or(1440) as i32;
        let bottom = pm.get("bottom").and_then(|b| b.as_i64()).unwrap_or(1440) as i32;
        let left = pm.get("left").and_then(|l| l.as_i64()).unwrap_or(1440) as i32;
        let header = pm.get("header").and_then(|h| h.as_i64()).unwrap_or(720) as i32;
        let footer = pm.get("footer").and_then(|f| f.as_i64()).unwrap_or(720) as i32;
        let gutter = pm.get("gutter").and_then(|g| g.as_i64()).unwrap_or(0) as i32;

        sp = sp.page_margin(
            docx_rs::PageMargin::new()
                .top(top)
                .right(right)
                .bottom(bottom)
                .left(left)
                .header(header)
                .footer(footer)
                .gutter(gutter),
        );
    }

    // Columns
    if let Some(columns) = v.get("columns").and_then(|c| c.as_u64()) {
        sp.columns = columns as usize;
    }

    // Text direction
    if let Some(td) = v.get("textDirection").and_then(|t| t.as_str()) {
        sp.text_direction = td.to_string();
    }

    // Doc grid
    if let Some(dg) = v.get("docGrid") {
        let grid_type = dg.get("gridType").and_then(|g| g.as_str()).unwrap_or("default");
        let line_pitch = dg.get("linePitch").and_then(|l| l.as_u64()).map(|l| l as usize);
        let char_space = dg.get("charSpace").and_then(|c| c.as_i64()).map(|c| c as isize);
        let gt = match grid_type {
            "lines" => docx_rs::DocGridType::Lines,
            "linesAndChars" => docx_rs::DocGridType::LinesAndChars,
            "snapToChars" => docx_rs::DocGridType::SnapToChars,
            _ => docx_rs::DocGridType::Default,
        };
        let mut doc_grid = docx_rs::DocGrid::with_empty().grid_type(gt);
        if let Some(lp) = line_pitch {
            doc_grid = doc_grid.line_pitch(lp);
        }
        if let Some(cs) = char_space {
            doc_grid = doc_grid.char_space(cs);
        }
        sp.doc_grid = Some(doc_grid);
    }

    sp
}

// ======================= Styles =======================

fn parse_styles(docx: &mut docx_rs::Docx, v: &Value) {
    // Parse doc defaults
    if let Some(doc_defaults) = v.get("docDefaults") {
        // Run property default
        if let Some(rpd) = doc_defaults.get("runPropertyDefault") {
            if let Some(rp) = rpd.get("runProperty") {
                if let Some(sz) = rp.get("sz").and_then(|s| s.as_u64()) {
                    docx.styles = docx.styles.clone().default_size(sz as usize);
                }
                if let Some(fonts) = rp.get("fonts") {
                    let rf = parse_run_fonts(fonts);
                    docx.styles = docx.styles.clone().default_fonts(rf);
                }
                if let Some(spacing) = rp.get("characterSpacing").and_then(|s| s.as_i64()) {
                    docx.styles = docx.styles.clone().default_spacing(spacing as i32);
                }
            }
        }
        // Paragraph property default
        if let Some(ppd) = doc_defaults.get("paragraphPropertyDefault") {
            if let Some(pp) = ppd.get("paragraphProperty") {
                if let Some(ls) = pp.get("lineSpacing") {
                    let mut spacing = docx_rs::LineSpacing::new();
                    if let Some(before) = ls.get("before").and_then(|b| b.as_u64()) {
                        spacing = spacing.before(before as u32);
                    }
                    if let Some(after) = ls.get("after").and_then(|a| a.as_u64()) {
                        spacing = spacing.after(after as u32);
                    }
                    if let Some(line) = ls.get("line").and_then(|l| l.as_u64()) {
                        spacing = spacing.line(line as i32);
                    }
                    docx.styles = docx.styles.clone().default_line_spacing(spacing);
                }
            }
        }
    }

    // Parse individual styles
    if let Some(styles) = v.get("styles").and_then(|s| s.as_array()) {
        for style_val in styles {
            if let Some(style) = parse_style(style_val) {
                docx.styles = docx.styles.clone().add_style(style);
            }
        }
    }
}

fn parse_style(v: &Value) -> Option<docx_rs::Style> {
    let style_id = v.get("styleId").and_then(|s| s.as_str())?;
    let style_type_str = v.get("styleType").and_then(|s| s.as_str()).unwrap_or("paragraph");

    let style_type = match style_type_str {
        "paragraph" => docx_rs::StyleType::Paragraph,
        "character" => docx_rs::StyleType::Character,
        "numbering" => docx_rs::StyleType::Numbering,
        "table" => docx_rs::StyleType::Table,
        _ => docx_rs::StyleType::Paragraph,
    };

    let mut style = docx_rs::Style::new(style_id, style_type);

    if let Some(name) = v.get("name").and_then(|n| n.as_str()) {
        style = style.name(name);
    }

    if let Some(based_on) = v.get("basedOn").and_then(|b| b.as_str()) {
        if !based_on.is_empty() {
            style = style.based_on(based_on);
        }
    }

    // Run property
    if let Some(rp) = v.get("runProperty") {
        if let Some(sz) = rp.get("sz").and_then(|s| s.as_u64()) {
            style = style.size(sz as usize);
        }
        if let Some(color) = rp.get("color").and_then(|c| c.as_str()) {
            if !color.is_empty() {
                style = style.color(color);
            }
        }
        if let Some(bold) = rp.get("bold") {
            if bold.as_bool().unwrap_or(false) {
                style = style.bold();
            }
        }
        if let Some(italic) = rp.get("italic") {
            if italic.as_bool().unwrap_or(false) {
                style = style.italic();
            }
        }
        if let Some(underline) = rp.get("underline").and_then(|u| u.as_str()) {
            if !underline.is_empty() {
                style = style.underline(underline);
            }
        }
        // Style doesn't have a strike() method; skip for now
    }

    Some(style)
}

// ======================= Numberings =======================

fn parse_numberings(docx: &mut docx_rs::Docx, v: &Value) {
    // Abstract numberings
    if let Some(abstract_nums) = v.get("abstractNums").and_then(|a| a.as_array()) {
        for an in abstract_nums {
            if let Some(abs_num) = parse_abstract_numbering(an) {
                docx.numberings = docx.numberings.clone().add_abstract_numbering(abs_num);
            }
        }
    }

    // Numberings
    if let Some(numberings) = v.get("numberings").and_then(|n| n.as_array()) {
        for num in numberings {
            if let Some(numbering) = parse_numbering(num) {
                docx.numberings = docx.numberings.clone().add_numbering(numbering);
            }
        }
    }
}

fn parse_abstract_numbering(v: &Value) -> Option<docx_rs::AbstractNumbering> {
    let id = v.get("id").and_then(|i| i.as_u64())? as usize;
    let mut abs_num = docx_rs::AbstractNumbering::new(id);

    if let Some(levels) = v.get("levels").and_then(|l| l.as_array()) {
        for level_val in levels {
            if let Some(level) = parse_level(level_val) {
                abs_num = abs_num.add_level(level);
            }
        }
    }

    Some(abs_num)
}

fn parse_level(v: &Value) -> Option<docx_rs::Level> {
    let level_num = v.get("level").and_then(|l| l.as_u64())? as usize;
    let start = v.get("start").and_then(|s| s.as_u64()).unwrap_or(1) as usize;
    let format = v.get("format").and_then(|f| f.as_str()).unwrap_or("decimal");
    let text = v.get("text").and_then(|t| t.as_str()).unwrap_or("%1.");
    let jc = v.get("jc").and_then(|j| j.as_str()).unwrap_or("left");

    let mut level = docx_rs::Level::new(
        level_num,
        docx_rs::Start::new(start),
        docx_rs::NumberFormat::new(format),
        docx_rs::LevelText::new(text),
        docx_rs::LevelJc::new(jc),
    );

    // Indent
    if let Some(pp) = v.get("paragraphProperty") {
        if let Some(indent) = pp.get("indent") {
            let left = indent.get("left").and_then(|l| l.as_i64());
            let hanging = indent.get("hanging").and_then(|h| h.as_i64());
            let special = hanging.map(|h| docx_rs::SpecialIndentType::Hanging(h as i32));
            level = level.indent(left.map(|l| l as i32), special, None, None);
        }
    }

    // Suffix
    if let Some(suffix) = v.get("suffix").and_then(|s| s.as_str()) {
        match suffix {
            "tab" => level = level.suffix(docx_rs::LevelSuffixType::Tab),
            "space" => level = level.suffix(docx_rs::LevelSuffixType::Space),
            "nothing" => level = level.suffix(docx_rs::LevelSuffixType::Nothing),
            _ => {}
        }
    }

    // Run property
    if let Some(rp) = v.get("runProperty") {
        if let Some(sz) = rp.get("sz").and_then(|s| s.as_u64()) {
            level = level.size(sz as usize);
        }
        if let Some(fonts) = rp.get("fonts") {
            let rf = parse_run_fonts(fonts);
            level = level.fonts(rf);
        }
        if let Some(bold) = rp.get("bold") {
            if bold.as_bool().unwrap_or(false) {
                level = level.bold();
            }
        }
        if let Some(italic) = rp.get("italic") {
            if italic.as_bool().unwrap_or(false) {
                level = level.italic();
            }
        }
    }

    Some(level)
}

fn parse_numbering(v: &Value) -> Option<docx_rs::Numbering> {
    let id = v.get("id").and_then(|i| i.as_u64())? as usize;
    let abstract_num_id = v.get("abstractNumId").and_then(|a| a.as_u64())? as usize;
    let mut num = docx_rs::Numbering::new(id, abstract_num_id);

    if let Some(overrides) = v.get("levelOverrides").and_then(|l| l.as_array()) {
        for ovr in overrides {
            if let Some(level) = ovr.get("level").and_then(|l| l.as_u64()) {
                let mut lo = docx_rs::LevelOverride::new(level as usize);
                if let Some(start) = ovr.get("overrideStart").and_then(|s| s.as_u64()) {
                    lo = lo.start(start as usize);
                }
                if let Some(level_val) = ovr.get("overrideLevel") {
                    if let Some(parsed_level) = parse_level(level_val) {
                        lo = lo.level(parsed_level);
                    }
                }
                num = num.add_override(lo);
            }
        }
    }

    Some(num)
}

// ======================= Settings =======================

fn parse_settings(docx: &mut docx_rs::Docx, v: &Value) {
    if let Some(tab_stop) = v.get("defaultTabStop").and_then(|d| d.as_u64()) {
        docx.settings = docx.settings.clone().default_tab_stop(tab_stop as usize);
    }

    if let Some(doc_id) = v.get("docId").and_then(|d| d.as_str()) {
        if !doc_id.is_empty() {
            docx.settings = docx.settings.clone().doc_id(doc_id);
        }
    }

    if let Some(adjust) = v.get("adjustLineHeightInTable") {
        if adjust.as_bool().unwrap_or(false) {
            docx.settings = docx.settings.clone().adjust_line_height_in_table();
        }
    }
}

// ======================= Comments =======================

fn parse_comments(_docx: &mut docx_rs::Docx, _v: &Value) {
    // Comments::add_comments is pub(crate) in docx-rs, so we cannot
    // reconstruct comments from the WASM layer. Comments are preserved
    // in the JSON for reading but cannot be round-tripped via generation yet.
    // This could be fixed by making Comments::add_comments public in docx-core.
}

// ======================= Footnotes =======================

fn parse_footnotes(_docx: &mut docx_rs::Docx, _v: &Value) {
    // Footnotes are reconstructed from the document structure
    // The footnotes section in the JSON is primarily for reading
    // The builder handles footnotes through the document flow
}

// ======================= Utilities =======================

fn parse_alignment_type(s: &str) -> Option<docx_rs::AlignmentType> {
    match s {
        "left" | "Left" => Some(docx_rs::AlignmentType::Left),
        "center" | "Center" => Some(docx_rs::AlignmentType::Center),
        "right" | "Right" => Some(docx_rs::AlignmentType::Right),
        "both" | "Both" => Some(docx_rs::AlignmentType::Both),
        "justified" | "Justified" => Some(docx_rs::AlignmentType::Justified),
        "distribute" | "Distribute" => Some(docx_rs::AlignmentType::Distribute),
        "start" | "Start" => Some(docx_rs::AlignmentType::Start),
        "end" | "End" => Some(docx_rs::AlignmentType::End),
        _ => None,
    }
}
