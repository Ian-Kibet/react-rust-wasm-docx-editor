export type Alignment = 'left' | 'center' | 'right' | 'justify';

export type LineSpacingRule = 'auto' | 'exact' | 'at_least';

export type TabStopAlignment = 'left' | 'center' | 'right' | 'decimal';

export type VerticalMerge = 'restart' | 'continue';

export type TableLayout = 'fixed' | 'autofit';

export type PageOrientation = 'portrait' | 'landscape';

export interface NumberingRef {
  num_id: string;
  level: number;
}

export interface TabStop {
  position: number;
  alignment: TabStopAlignment;
  leader?: string;
}

export interface Border {
  style: string;
  size: number;
  color: string;
}

export interface ParagraphProperties {
  alignment?: Alignment;
  heading_level?: number;
  numbering?: NumberingRef;
  spacing_before?: number;
  spacing_after?: number;
  line_spacing?: number;
  line_spacing_rule?: LineSpacingRule;
  indent_left?: number;
  indent_right?: number;
  indent_first_line?: number;
  indent_hanging?: number;
  page_break_before?: boolean;
  keep_next?: boolean;
  keep_lines?: boolean;
  widow_control?: boolean;
  style_id?: string;
  border_bottom?: Border;
  border_top?: Border;
  shading?: string;
  tab_stops?: TabStop[];
}

export interface RunProperties {
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strikethrough?: boolean;
  double_strikethrough?: boolean;
  superscript?: boolean;
  subscript?: boolean;
  font_family?: string;
  font_size?: number;
  color?: string;
  highlight?: string;
  background_color?: string;
  inline_image?: string;
  hyperlink_url?: string;
  hyperlink_tooltip?: string;
  line_break?: boolean;
  page_break?: boolean;
  tab?: boolean;
  small_caps?: boolean;
  all_caps?: boolean;
  spacing?: number;
  footnote_ref?: string;
  endnote_ref?: string;
}

export interface Run {
  id: string;
  text: string;
  properties: RunProperties;
}

export interface Bookmark {
  id: string;
  name: string;
}

export interface Paragraph {
  id: string;
  runs: Run[];
  properties: ParagraphProperties;
  bookmarks?: Bookmark[];
}

export interface TableBorders {
  top?: Border;
  bottom?: Border;
  left?: Border;
  right?: Border;
  inside_h?: Border;
  inside_v?: Border;
}

export interface TableRowProperties {
  height?: number;
  height_rule?: string;
  is_header?: boolean;
  cant_split?: boolean;
}

export interface TableProperties {
  width?: number;
  borders?: TableBorders;
  cell_spacing?: number;
  layout?: TableLayout;
  indent?: number;
}

export interface TableCellProperties {
  width?: number;
  vertical_align?: string;
  borders?: TableBorders;
  shading?: string;
  grid_span?: number;
  vertical_merge?: VerticalMerge;
  text_direction?: string;
  no_wrap?: boolean;
  padding_top?: number;
  padding_bottom?: number;
  padding_left?: number;
  padding_right?: number;
}

export interface TableCell {
  id: string;
  content: BlockElement[];
  properties: TableCellProperties;
}

export interface TableRow {
  id: string;
  cells: TableCell[];
  properties?: TableRowProperties;
}

export interface Table {
  id: string;
  rows: TableRow[];
  properties: TableProperties;
}

export type BlockElement =
  | { paragraph: Paragraph }
  | { table: Table };

export interface ImageData {
  id: string;
  data_base64: string;
  content_type: string;
  width?: number;
  height?: number;
  description?: string;
}

export interface ListLevel {
  level: number;
  format: string;
  text: string;
  start: number;
}

export interface ListDef {
  num_id: string;
  levels: ListLevel[];
}

export interface StyleDef {
  id: string;
  name: string;
  style_type: string;
  based_on?: string;
  run_properties?: RunProperties;
  paragraph_properties?: ParagraphProperties;
}

export interface Header {
  id: string;
  content: BlockElement[];
}

export interface Footer {
  id: string;
  content: BlockElement[];
}

export interface Comment {
  id: string;
  author: string;
  date?: string;
  initials?: string;
  content: BlockElement[];
}

export interface Footnote {
  id: string;
  content: BlockElement[];
}

export interface Endnote {
  id: string;
  content: BlockElement[];
}

export interface SectionProperties {
  page_width?: number;
  page_height?: number;
  margin_top?: number;
  margin_right?: number;
  margin_bottom?: number;
  margin_left?: number;
  margin_header?: number;
  margin_footer?: number;
  margin_gutter?: number;
  page_orientation?: PageOrientation;
  columns?: number;
}

export interface DocumentStatistics {
  word_count: number;
  char_count: number;
  paragraph_count: number;
  page_count_estimate: number;
  image_count: number;
  table_count: number;
  header_count: number;
  footer_count: number;
  style_count: number;
  comment_count: number;
  footnote_count: number;
  endnote_count: number;
}

export interface DocxDocument {
  body: BlockElement[];
  styles: StyleDef[];
  numbering: ListDef[];
  headers: Header[];
  footers: Footer[];
  images: ImageData[];
  comments?: Comment[];
  footnotes?: Footnote[];
  endnotes?: Endnote[];
  section_properties?: SectionProperties;
}
