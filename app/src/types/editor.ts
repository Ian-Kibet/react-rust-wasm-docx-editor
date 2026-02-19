export interface SelectionPoint {
  block_index: number;
  paragraph_id: string;
  run_index: number;
  offset: number;
}

export interface EditorSelection {
  anchor: SelectionPoint;
  focus: SelectionPoint;
  is_collapsed: boolean;
}

export type EditingZone = 'body' | 'header' | 'footer';
