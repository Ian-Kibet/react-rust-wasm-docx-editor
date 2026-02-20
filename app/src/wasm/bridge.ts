import { parse_docx, generate_docx, version } from './pkg/docx_wasm.js';
import {
  DocxDocument,
  DocumentStatistics,
  BlockElement,
  Paragraph,
  Run,
  RunProperties,
  ParagraphProperties,
  Table,
  TableRow,
  TableCell,
  TableCellProperties,
  SectionProperties,
  StyleDef,
  ListDef,
  ListLevel,
  ImageData,
  Comment,
  Footnote,
  Header,
  Footer,
} from '../types/document';

// =====================================================================
// Parse: DOCX bytes -> docx-rs JSON -> our model
// =====================================================================

export function parseDocx(data: Uint8Array): DocxDocument {
  const json = parse_docx(data);
  const raw = JSON.parse(json);
  return convertDocxRsToModel(raw);
}

// =====================================================================
// Generate: our model -> docx-rs JSON -> DOCX bytes
// =====================================================================

export function generateDocxBlob(doc: DocxDocument): Blob {
  const docxRsJson = convertModelToDocxRs(doc);
  const json = JSON.stringify(docxRsJson);
  const bytes = generate_docx(json);
  return new Blob([bytes], {
    type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  });
}

export function getWasmVersion(): string {
  return version();
}

// =====================================================================
// docx-rs JSON -> our DocxDocument model
// =====================================================================

let nextId = 1;
function genId(): string {
  return `id_${nextId++}`;
}

function convertDocxRsToModel(raw: any): DocxDocument {
  nextId = 1;

  const body = convertDocumentChildren(raw?.document?.children ?? []);
  const sectionProperties = convertSectionProperty(raw?.document?.sectionProperty);
  const styles = convertStyles(raw?.styles);
  const numbering = convertNumberings(raw?.numberings);
  const images = convertImages(raw?.images ?? []);
  const comments = convertComments(raw?.comments);
  const footnotes = convertFootnotes(raw?.footnotes);
  const headers = convertHeaders(raw?.document?.sectionProperty);
  const footers = convertFooters(raw?.document?.sectionProperty);

  return {
    body,
    styles,
    numbering,
    headers,
    footers,
    images,
    comments,
    footnotes,
    endnotes: [],
    section_properties: sectionProperties,
  };
}

function convertDocumentChildren(children: any[]): BlockElement[] {
  const result: BlockElement[] = [];
  for (const child of children) {
    const type = child?.type;
    const data = child?.data;
    if (!type || !data) continue;

    if (type === 'paragraph') {
      result.push({ paragraph: convertParagraph(data) });
    } else if (type === 'table') {
      result.push({ table: convertTable(data) });
    }
  }
  return result;
}

function convertParagraph(data: any): Paragraph {
  const id = data?.id || genId();
  const runs: Run[] = [];

  for (const child of data?.children ?? []) {
    const type = child?.type;
    const childData = child?.data;
    if (!type) continue;

    if (type === 'run' && childData) {
      const runResult = convertRun(childData);
      runs.push(...runResult);
    } else if (type === 'hyperlink' && childData) {
      for (const hChild of childData?.children ?? []) {
        if (hChild?.type === 'run' && hChild?.data) {
          const hRuns = convertRun(hChild.data);
          for (const r of hRuns) {
            r.properties.hyperlink_url = childData?.id || '';
          }
          runs.push(...hRuns);
        }
      }
    }
  }

  const properties = convertParagraphProperty(data?.property);

  return { id, runs, properties };
}

function convertRun(data: any): Run[] {
  const rp = data?.runProperty ?? {};
  const baseProps = convertRunProperty(rp);
  const runs: Run[] = [];

  for (const child of data?.children ?? []) {
    const type = child?.type;
    const childData = child?.data;

    if (type === 'text' && childData) {
      runs.push({
        id: genId(),
        text: childData.text ?? '',
        properties: { ...baseProps },
      });
    } else if (type === 'tab') {
      runs.push({
        id: genId(),
        text: '\t',
        properties: { ...baseProps, tab: true },
      });
    } else if (type === 'break') {
      const breakType = childData?.breakType ?? 'textWrapping';
      if (breakType === 'page') {
        runs.push({
          id: genId(),
          text: '',
          properties: { ...baseProps, page_break: true },
        });
      } else {
        runs.push({
          id: genId(),
          text: '\n',
          properties: { ...baseProps, line_break: true },
        });
      }
    } else if (type === 'drawing' && childData) {
      const pic = childData?.pic;
      if (pic) {
        runs.push({
          id: genId(),
          text: '',
          properties: { ...baseProps, inline_image: pic?.id ?? '' },
        });
      }
    }
  }

  if (runs.length === 0) {
    runs.push({
      id: genId(),
      text: '',
      properties: baseProps,
    });
  }

  return runs;
}

function convertRunProperty(rp: any): RunProperties {
  const props: RunProperties = {};

  if (rp.bold === true) props.bold = true;
  if (rp.italic === true) props.italic = true;
  if (rp.underline && rp.underline !== 'none') props.underline = true;
  if (rp.strike === true) props.strikethrough = true;
  if (rp.dstrike === true) props.double_strikethrough = true;
  if (rp.caps === true) props.all_caps = true;

  if (rp.sz != null) {
    props.font_size = rp.sz / 2;
  }

  if (rp.color && rp.color !== 'auto') {
    props.color = `#${rp.color}`;
  }

  if (rp.highlight) {
    props.highlight = rp.highlight;
  }

  if (rp.characterSpacing != null) {
    props.spacing = rp.characterSpacing;
  }

  if (rp.vertAlign === 'superscript') props.superscript = true;
  if (rp.vertAlign === 'subscript') props.subscript = true;

  if (rp.fonts) {
    const f = rp.fonts;
    props.font_family = f.ascii || f.hiAnsi || f.eastAsia || f.cs || undefined;
  }

  return props;
}

function convertParagraphProperty(prop: any): ParagraphProperties {
  if (!prop) return {};
  const pp: ParagraphProperties = {};

  if (prop.alignment) {
    const a = prop.alignment;
    if (a === 'both' || a === 'justified') pp.alignment = 'justify';
    else if (a === 'left' || a === 'start') pp.alignment = 'left';
    else if (a === 'right' || a === 'end') pp.alignment = 'right';
    else if (a === 'center') pp.alignment = 'center';
  }

  if (prop.style) {
    pp.style_id = prop.style;
    const headingMatch = prop.style.match(/^Heading(\d)$/i);
    if (headingMatch) {
      pp.heading_level = parseInt(headingMatch[1], 10);
    }
  }

  if (prop.indent) {
    const i = prop.indent;
    if (i.left != null) pp.indent_left = i.left;
    if (i.end != null) pp.indent_right = i.end;
    if (i.firstLine != null) pp.indent_first_line = i.firstLine;
    if (i.hanging != null) pp.indent_hanging = i.hanging;
    if (i.firstLineChars != null) pp.indent_first_line = i.firstLineChars;
    if (i.hangingChars != null) pp.indent_hanging = i.hangingChars;
  }

  if (prop.lineSpacing) {
    const ls = prop.lineSpacing;
    if (ls.before != null) pp.spacing_before = ls.before;
    if (ls.after != null) pp.spacing_after = ls.after;
    if (ls.line != null) pp.line_spacing = ls.line;
    if (ls.lineRule) {
      const rule = ls.lineRule.toLowerCase();
      if (rule === 'atleast') pp.line_spacing_rule = 'at_least';
      else if (rule === 'exact') pp.line_spacing_rule = 'exact';
      else pp.line_spacing_rule = 'auto';
    }
  }

  if (prop.numbering) {
    pp.numbering = {
      num_id: String(prop.numbering.id ?? ''),
      level: prop.numbering.level ?? 0,
    };
  }

  if (prop.keepNext === true) pp.keep_next = true;
  if (prop.keepLines === true) pp.keep_lines = true;
  if (prop.pageBreakBefore === true) pp.page_break_before = true;
  if (prop.widowControl != null) pp.widow_control = prop.widowControl;

  if (prop.tabs && Array.isArray(prop.tabs)) {
    pp.tab_stops = prop.tabs
      .filter((t: any) => t?.val && t?.pos != null)
      .map((t: any) => ({
        position: t.pos,
        alignment: (t.val || 'left') as any,
        leader: t.leader || undefined,
      }));
  }

  return pp;
}

function convertTable(data: any): Table {
  const rows: TableRow[] = [];

  for (const rowChild of data?.rows ?? []) {
    if (rowChild?.type === 'tableRow' && rowChild?.data) {
      rows.push(convertTableRow(rowChild.data));
    }
  }

  const prop = data?.property ?? {};
  const width = prop?.width?.width;

  return {
    id: genId(),
    rows,
    properties: {
      width: width != null ? width : undefined,
      layout: prop?.layout === 'fixed' ? 'fixed' : 'autofit',
      indent: prop?.indent?.val,
    },
  };
}

function convertTableRow(data: any): TableRow {
  const cells: TableCell[] = [];

  for (const cellChild of data?.cells ?? []) {
    if (cellChild?.type === 'tableCell' && cellChild?.data) {
      cells.push(convertTableCell(cellChild.data));
    }
  }

  const prop = data?.property ?? {};

  return {
    id: genId(),
    cells,
    properties: {
      height: prop.rowHeight,
      height_rule: prop.heightRule,
    },
  };
}

function convertTableCell(data: any): TableCell {
  const content = convertDocumentChildren(data?.children ?? []);
  const prop = data?.property ?? {};

  const cellProps: TableCellProperties = {};

  if (prop.width?.width != null) cellProps.width = prop.width.width;
  if (prop.gridSpan != null && prop.gridSpan > 1) cellProps.grid_span = prop.gridSpan;
  if (prop.verticalMerge) cellProps.vertical_merge = prop.verticalMerge;
  if (prop.verticalAlign) cellProps.vertical_align = prop.verticalAlign;
  if (prop.shading?.fill) cellProps.shading = `#${prop.shading.fill}`;
  if (prop.textDirection) cellProps.text_direction = prop.textDirection;

  return {
    id: genId(),
    content,
    properties: cellProps,
  };
}

function convertSectionProperty(sp: any): SectionProperties {
  if (!sp) return {};

  return {
    page_width: sp.pageSize?.w,
    page_height: sp.pageSize?.h,
    margin_top: sp.pageMargin?.top,
    margin_right: sp.pageMargin?.right,
    margin_bottom: sp.pageMargin?.bottom,
    margin_left: sp.pageMargin?.left,
    margin_header: sp.pageMargin?.header,
    margin_footer: sp.pageMargin?.footer,
    margin_gutter: sp.pageMargin?.gutter,
    page_orientation: sp.pageSize?.orient === 'landscape' ? 'landscape' : 'portrait',
    columns: sp.columns,
  };
}

function convertStyles(stylesData: any): StyleDef[] {
  if (!stylesData?.styles) return [];
  return (stylesData.styles as any[]).map((s: any) => {
    const def: StyleDef = {
      id: s.styleId ?? '',
      name: s.name ?? s.styleId ?? '',
      style_type: s.styleType ?? 'paragraph',
      based_on: s.basedOn || undefined,
    };

    if (s.runProperty) {
      def.run_properties = convertRunProperty(s.runProperty);
    }

    if (s.paragraphProperty) {
      def.paragraph_properties = convertParagraphProperty(s.paragraphProperty);
    }

    return def;
  });
}

function convertNumberings(numbData: any): ListDef[] {
  if (!numbData) return [];

  const abstractNums = numbData.abstractNums ?? [];
  const numberings = numbData.numberings ?? [];

  return numberings.map((n: any) => {
    const absNum = abstractNums.find((a: any) => a.id === n.abstractNumId);
    const levels: ListLevel[] = (absNum?.levels ?? []).map((l: any) => ({
      level: l.level ?? 0,
      format: l.format ?? 'decimal',
      text: l.text ?? '',
      start: l.start ?? 1,
    }));

    return {
      num_id: String(n.id ?? ''),
      levels,
    };
  });
}

function convertImages(images: any[]): ImageData[] {
  return images.map((img: any) => ({
    id: img[0] ?? genId(),
    data_base64: img[2] ?? '',
    content_type: 'image/png',
    description: img[1] ?? '',
  }));
}

function convertComments(commentsData: any): Comment[] {
  if (!commentsData?.comments) return [];
  return (commentsData.comments as any[]).map((c: any) => ({
    id: String(c.id ?? ''),
    author: c.author ?? '',
    date: c.date || undefined,
    initials: c.initials || undefined,
    content: convertDocumentChildren(c.children ?? []),
  }));
}

function convertFootnotes(fnData: any): Footnote[] {
  if (!fnData?.footnotes) return [];
  return (fnData.footnotes as any[])
    .filter((f: any) => f.id != null && f.id > 0)
    .map((f: any) => ({
      id: String(f.id ?? ''),
      content: convertDocumentChildren(f.children ?? []),
    }));
}

function convertHeaders(sp: any): Header[] {
  const headers: Header[] = [];
  if (sp?.header?.children) {
    headers.push({
      id: sp.headerReference?.id ?? genId(),
      content: convertDocumentChildren(sp.header.children),
    });
  }
  return headers;
}

function convertFooters(sp: any): Footer[] {
  const footers: Footer[] = [];
  if (sp?.footer && Array.isArray(sp.footer) && sp.footer.length >= 2) {
    const footerData = sp.footer[1];
    if (footerData?.children) {
      footers.push({
        id: sp.footer[0] ?? genId(),
        content: convertDocumentChildren(footerData.children),
      });
    }
  }
  return footers;
}

// =====================================================================
// our DocxDocument model -> docx-rs JSON
// =====================================================================

function convertModelToDocxRs(doc: DocxDocument): any {
  const children = doc.body.map(convertBlockToDocxRs);
  const sp = doc.section_properties ?? {};

  const sectionProperty: any = {
    pageSize: {
      w: sp.page_width ?? 11906,
      h: sp.page_height ?? 16838,
      orient: sp.page_orientation === 'landscape' ? 'landscape' : null,
    },
    pageMargin: {
      top: sp.margin_top ?? 1440,
      right: sp.margin_right ?? 1440,
      bottom: sp.margin_bottom ?? 1440,
      left: sp.margin_left ?? 1440,
      header: sp.margin_header ?? 720,
      footer: sp.margin_footer ?? 720,
      gutter: sp.margin_gutter ?? 0,
    },
    columns: sp.columns ?? 1,
    titlePg: false,
    textDirection: 'lrTb',
  };

  const styles = convertStylesBack(doc.styles);
  const numberings = convertNumberingsBack(doc.numbering);

  return {
    contentType: { types: {}, web_extension_count: 1, custom_xml_count: 1, header_count: 0, footer_count: 0 },
    rels: { rels: [] },
    documentRels: { hasComments: false, hasNumberings: (doc.numbering?.length ?? 0) > 0, hasFootnotes: false, images: [], hyperlinks: [], customXmlCount: 0, headerCount: 0, footerCount: 0 },
    docProps: { app: {}, core: { config: {} }, custom: { properties: {} } },
    styles,
    document: { children, sectionProperty, hasNumbering: false },
    comments: { comments: [] },
    numberings,
    settings: { defaultTabStop: 840, zoom: 100, docId: null, docVars: [], evenAndOddHeaders: false },
    fontTable: {},
    media: [],
    commentsExtended: { comments: [] },
    webSettings: {},
    taskpanes: null,
    taskpanesRels: { rels: [] },
    webExtensions: [],
    customItems: [],
    customItemProps: [],
    customItemRels: [],
    themes: [],
    images: [],
    hyperlinks: [],
    footnotes: { footnotes: [] },
  };
}

function convertBlockToDocxRs(block: BlockElement): any {
  if ('paragraph' in block) {
    return { type: 'paragraph', data: convertParagraphBack(block.paragraph) };
  } else if ('table' in block) {
    return { type: 'table', data: convertTableBack(block.table) };
  }
  return null;
}

function convertParagraphBack(p: Paragraph): any {
  const children: any[] = [];

  for (const run of p.runs) {
    children.push({
      type: 'run',
      data: convertRunBack(run),
    });
  }

  return {
    id: p.id,
    children,
    property: convertParagraphPropertyBack(p.properties),
    hasNumbering: !!p.properties.numbering,
  };
}

function convertRunBack(run: Run): any {
  const rp = run.properties;
  const runProperty: any = {};
  const children: any[] = [];

  if (rp.bold) runProperty.bold = true;
  if (rp.italic) runProperty.italic = true;
  if (rp.underline) runProperty.underline = 'single';
  if (rp.strikethrough) runProperty.strike = true;
  if (rp.double_strikethrough) runProperty.dstrike = true;
  if (rp.all_caps) runProperty.caps = true;

  if (rp.font_size != null) {
    runProperty.sz = rp.font_size * 2;
    runProperty.szCs = rp.font_size * 2;
  }

  if (rp.color) {
    runProperty.color = rp.color.replace('#', '');
  }

  if (rp.highlight) runProperty.highlight = rp.highlight;
  if (rp.superscript) runProperty.vertAlign = 'superscript';
  if (rp.subscript) runProperty.vertAlign = 'subscript';

  if (rp.font_family) {
    runProperty.fonts = {
      ascii: rp.font_family,
      hiAnsi: rp.font_family,
      eastAsia: rp.font_family,
      cs: rp.font_family,
    };
  }

  if (rp.spacing != null) runProperty.characterSpacing = rp.spacing;

  if (rp.page_break) {
    children.push({ type: 'break', data: { breakType: 'page' } });
  } else if (rp.line_break) {
    children.push({ type: 'break', data: { breakType: 'textWrapping' } });
  } else if (rp.tab) {
    children.push({ type: 'tab', data: {} });
  } else if (run.text) {
    children.push({
      type: 'text',
      data: { preserveSpace: true, text: run.text },
    });
  }

  return { runProperty, children };
}

function convertParagraphPropertyBack(pp: ParagraphProperties): any {
  const prop: any = { runProperty: {}, tabs: [] };

  if (pp.alignment) {
    const alignMap: Record<string, string> = {
      left: 'left',
      center: 'center',
      right: 'right',
      justify: 'both',
    };
    prop.alignment = alignMap[pp.alignment] || 'left';
  }

  if (pp.style_id) prop.style = pp.style_id;

  if (pp.indent_left != null || pp.indent_right != null || pp.indent_first_line != null || pp.indent_hanging != null) {
    const indent: any = {};
    if (pp.indent_left != null) indent.left = pp.indent_left;
    if (pp.indent_right != null) indent.end = pp.indent_right;
    if (pp.indent_first_line != null) indent.firstLine = pp.indent_first_line;
    if (pp.indent_hanging != null) indent.hanging = pp.indent_hanging;
    prop.indent = indent;
  }

  if (pp.spacing_before != null || pp.spacing_after != null || pp.line_spacing != null) {
    const ls: any = {};
    if (pp.spacing_before != null) ls.before = pp.spacing_before;
    if (pp.spacing_after != null) ls.after = pp.spacing_after;
    if (pp.line_spacing != null) ls.line = pp.line_spacing;
    if (pp.line_spacing_rule) {
      const ruleMap: Record<string, string> = { auto: 'auto', exact: 'exact', at_least: 'atLeast' };
      ls.lineRule = ruleMap[pp.line_spacing_rule] || 'auto';
    }
    prop.lineSpacing = ls;
  }

  if (pp.numbering) {
    prop.numbering = {
      id: parseInt(pp.numbering.num_id, 10) || 0,
      level: pp.numbering.level,
    };
  }

  if (pp.keep_next) prop.keepNext = true;
  if (pp.keep_lines) prop.keepLines = true;
  if (pp.page_break_before) prop.pageBreakBefore = true;
  if (pp.widow_control != null) prop.widowControl = pp.widow_control;

  if (pp.tab_stops) {
    prop.tabs = pp.tab_stops.map((ts) => ({
      val: ts.alignment,
      leader: ts.leader || null,
      pos: ts.position,
    }));
  }

  return prop;
}

function convertTableBack(table: Table): any {
  const rows = table.rows.map((row) => ({
    type: 'tableRow',
    data: {
      cells: row.cells.map((cell) => ({
        type: 'tableCell',
        data: {
          children: cell.content.map(convertBlockToDocxRs),
          property: convertCellPropertyBack(cell.properties),
          hasNumbering: false,
        },
      })),
      property: {},
    },
  }));

  const prop: any = {};
  if (table.properties.width != null) {
    prop.width = { width: table.properties.width, widthType: 'dxa' };
  }
  if (table.properties.layout) {
    prop.layout = table.properties.layout;
  }
  if (table.properties.indent != null) {
    prop.indent = { val: table.properties.indent };
  }

  return { rows, grid: [], property: prop, hasNumbering: false };
}

function convertCellPropertyBack(cp: TableCellProperties): any {
  const prop: any = {};
  if (cp.width != null) prop.width = { width: cp.width, widthType: 'dxa' };
  if (cp.grid_span != null && cp.grid_span > 1) prop.gridSpan = cp.grid_span;
  if (cp.vertical_merge) prop.verticalMerge = cp.vertical_merge;
  if (cp.vertical_align) prop.verticalAlign = cp.vertical_align;
  if (cp.shading) {
    prop.shading = { shdType: 'clear', fill: cp.shading.replace('#', ''), color: 'auto' };
  }
  if (cp.text_direction) prop.textDirection = cp.text_direction;
  return prop;
}

function convertStylesBack(styles: StyleDef[]): any {
  return {
    docDefaults: {
      runPropertyDefault: { runProperty: {} },
      paragraphPropertyDefault: { paragraphProperty: { runProperty: {}, tabs: [] } },
    },
    styles: styles.map((s) => ({
      styleId: s.id,
      name: s.name,
      styleType: s.style_type,
      basedOn: s.based_on || null,
      runProperty: {},
      paragraphProperty: { runProperty: {}, tabs: [] },
      tableProperty: {},
      tableCellProperty: {},
      next: null,
    })),
  };
}

function convertNumberingsBack(numbering: ListDef[]): any {
  const abstractNums = numbering.map((n, idx) => ({
    id: idx,
    styleLink: null,
    numStyleLink: null,
    levels: n.levels.map((l) => ({
      level: l.level,
      start: l.start,
      format: l.format,
      text: l.text,
      jc: 'left',
      paragraphProperty: { runProperty: {}, tabs: [] },
      runProperty: {},
      suffix: 'tab',
      pstyle: null,
      levelRestart: null,
    })),
  }));

  const nums = numbering.map((n, idx) => ({
    id: parseInt(n.num_id, 10) || idx + 1,
    abstractNumId: idx,
    levelOverrides: [],
  }));

  return { abstractNums, numberings: nums };
}

// =====================================================================
// Document Statistics (pure JS)
// =====================================================================

export function getDocumentStats(doc: DocxDocument): DocumentStatistics {
  let word_count = 0;
  let char_count = 0;
  let paragraph_count = 0;
  let table_count = 0;
  let image_count = 0;

  function processBlocks(blocks: typeof doc.body) {
    for (const block of blocks) {
      if ('paragraph' in block) {
        paragraph_count++;
        for (const run of block.paragraph.runs) {
          if (run.properties && run.properties.inline_image) image_count++;
          const text = run.text;
          char_count += text.length;
          const trimmed = text.trim();
          if (trimmed.length > 0) {
            word_count += trimmed.split(/\s+/).length;
          }
        }
      } else if ('table' in block) {
        table_count++;
        for (const row of block.table.rows) {
          for (const cell of row.cells) {
            processBlocks(cell.content);
          }
        }
      }
    }
  }

  processBlocks(doc.body);

  return {
    word_count,
    char_count,
    paragraph_count,
    page_count_estimate: Math.max(1, Math.ceil(paragraph_count / 30)),
    image_count,
    table_count,
    header_count: doc.headers?.length ?? 0,
    footer_count: doc.footers?.length ?? 0,
    style_count: doc.styles?.length ?? 0,
    comment_count: doc.comments?.length ?? 0,
    footnote_count: doc.footnotes?.length ?? 0,
    endnote_count: doc.endnotes?.length ?? 0,
  };
}
