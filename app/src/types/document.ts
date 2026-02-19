export type Alignment = 'left' | 'center' | 'right' | 'justify';

export interface NumberingRef {
  num_id: string;
  level: number;
}

export interface ParagraphProperties {
  alignment?: Alignment;
  heading_level?: number;
  numbering?: NumberingRef;
  spacing_before?: number;
  spacing_after?: number;
  indent_left?: number;
  indent_right?: number;
  indent_first_line?: number;
}

export interface RunProperties {
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strikethrough?: boolean;
  font_family?: string;
  font_size?: number;
  color?: string;
  highlight?: string;
  inline_image?: string;
}

export interface Run {
  id: string;
  text: string;
  properties: RunProperties;
}

export interface Paragraph {
  id: string;
  runs: Run[];
  properties: ParagraphProperties;
}

export interface Border {
  style: string;
  size: number;
  color: string;
}

export interface TableBorders {
  top?: Border;
  bottom?: Border;
  left?: Border;
  right?: Border;
  inside_h?: Border;
  inside_v?: Border;
}

export interface TableProperties {
  width?: number;
  borders?: TableBorders;
}

export interface TableCellProperties {
  width?: number;
  vertical_align?: string;
  borders?: TableBorders;
  shading?: string;
}

export interface TableCell {
  id: string;
  content: BlockElement[];
  properties: TableCellProperties;
}

export interface TableRow {
  id: string;
  cells: TableCell[];
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

export interface DocxDocument {
  body: BlockElement[];
  styles: StyleDef[];
  numbering: ListDef[];
  headers: Header[];
  footers: Footer[];
  images: ImageData[];
}
