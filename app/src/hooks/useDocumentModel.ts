import { useReducer } from 'react';
import {
  DocxDocument,
  BlockElement,
  Paragraph,
  Run,
  RunProperties,
  ImageData,
  Alignment,
  Table,
  TableRow,
  TableCell,
  TableCellProperties,
  TableProperties,
  Comment,
  Footnote,
  Endnote,
  SectionProperties,
} from '../types/document';
import { EditorSelection, EditingZone } from '../types/editor';
import {
  normalizeSelection,
  getRunsInRange,
  applyRunPropertyToRange,
  deleteRange,
  getSelectedText,
  splitRunAtOffset,
} from '../utils/rangeOps';

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export interface DocumentState {
  document: DocxDocument;
  selection: EditorSelection | null;
  undo_stack: DocxDocument[];
  redo_stack: DocxDocument[];
  is_dirty: boolean;
  editing_zone: EditingZone;
  find_replace_open: boolean;
  zoom: number;
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export type DocumentAction =
  | { type: 'load_document'; payload: DocxDocument }
  | { type: 'set_selection'; payload: EditorSelection | null }
  | { type: 'insert_text'; payload: { text: string } }
  | { type: 'delete_text'; payload: { direction: 'forward' | 'backward' } }
  | { type: 'split_paragraph' }
  | { type: 'toggle_bold' }
  | { type: 'toggle_italic' }
  | { type: 'toggle_underline' }
  | { type: 'toggle_strikethrough' }
  | { type: 'toggle_superscript' }
  | { type: 'toggle_subscript' }
  | { type: 'toggle_all_caps' }
  | { type: 'toggle_small_caps' }
  | { type: 'set_font_size'; payload: { size: number } }
  | { type: 'set_font_family'; payload: { family: string } }
  | { type: 'set_color'; payload: { color: string } }
  | { type: 'set_highlight'; payload: { color: string | null } }
  | { type: 'set_alignment'; payload: { alignment: Alignment } }
  | { type: 'set_heading_level'; payload: { level: number | null } }
  | { type: 'set_line_spacing'; payload: { spacing: number; rule?: string } }
  | { type: 'toggle_bullet_list' }
  | { type: 'toggle_numbered_list' }
  | { type: 'insert_table'; payload: { rows: number; cols: number } }
  | { type: 'insert_image'; payload: ImageData }
  | { type: 'set_paragraph_indent'; payload: { indent_left?: number; indent_right?: number; indent_first_line?: number } }
  | { type: 'insert_hyperlink'; payload: { url: string; text?: string; tooltip?: string } }
  | { type: 'insert_page_break' }
  | { type: 'insert_line_break' }
  | { type: 'grow_font' }
  | { type: 'shrink_font' }
  | { type: 'increase_indent' }
  | { type: 'decrease_indent' }
  | { type: 'delete_range' }
  | { type: 'insert_row'; payload: { table_id: string; after_row_index: number } }
  | { type: 'insert_column'; payload: { table_id: string; after_col_index: number } }
  | { type: 'delete_row'; payload: { table_id: string; row_index: number } }
  | { type: 'delete_column'; payload: { table_id: string; col_index: number } }
  | { type: 'set_cell_shading'; payload: { cell_id: string; color: string } }
  | { type: 'resize_image'; payload: { image_id: string; width: number; height: number } }
  | { type: 'add_comment'; payload: { author: string; content: string } }
  | { type: 'delete_comment'; payload: { comment_id: string } }
  | { type: 'insert_footnote'; payload: { content: string } }
  | { type: 'insert_endnote'; payload: { content: string } }
  | { type: 'set_section_properties'; payload: Partial<SectionProperties> }
  | { type: 'set_editing_zone'; payload: EditingZone }
  | { type: 'toggle_find_replace' }
  | { type: 'set_zoom'; payload: { zoom: number } }
  | { type: 'paste_runs'; payload: { runs: Run[] } }
  | { type: 'undo' }
  | { type: 'redo' };

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FONT_SIZE_SEQUENCE = [8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function cloneDocument(doc: DocxDocument): DocxDocument {
  return JSON.parse(JSON.stringify(doc));
}

function pushUndo(state: DocumentState): DocumentState {
  const stack = [...state.undo_stack, cloneDocument(state.document)];
  // Cap undo stack at 50 entries
  if (stack.length > 50) {
    stack.splice(0, stack.length - 50);
  }
  return {
    ...state,
    undo_stack: stack,
    redo_stack: [],
  };
}

function createEmptyRun(): Run {
  return {
    id: crypto.randomUUID(),
    text: '',
    properties: {},
  };
}

function createEmptyParagraph(): Paragraph {
  return {
    id: crypto.randomUUID(),
    runs: [createEmptyRun()],
    properties: {},
  };
}

interface FoundLocation {
  blockIndex: number;
  paragraph: Paragraph;
  runIndex: number;
  run: Run;
}

function findParagraphAndRun(
  doc: DocxDocument,
  selection: EditorSelection,
): FoundLocation | null {
  const point = selection.anchor;
  const block = doc.body[point.block_index];
  if (!block) return null;

  // Top-level paragraph
  if ('paragraph' in block) {
    const paragraph = block.paragraph;
    if (paragraph.id !== point.paragraph_id) return null;
    const run = paragraph.runs[point.run_index];
    if (!run) return null;
    return { blockIndex: point.block_index, paragraph, runIndex: point.run_index, run };
  }

  // Table block — search cells for the target paragraph
  if ('table' in block) {
    for (const row of block.table.rows) {
      for (const cell of row.cells) {
        for (const cellBlock of cell.content) {
          if ('paragraph' in cellBlock && cellBlock.paragraph.id === point.paragraph_id) {
            const paragraph = cellBlock.paragraph;
            const run = paragraph.runs[point.run_index];
            if (!run) return null;
            return { blockIndex: point.block_index, paragraph, runIndex: point.run_index, run };
          }
        }
      }
    }
  }

  return null;
}

function findTableById(doc: DocxDocument, tableId: string): Table | null {
  for (const block of doc.body) {
    if ('table' in block && block.table.id === tableId) {
      return block.table;
    }
  }
  return null;
}

export function createDefaultDocument(): DocxDocument {
  return {
    body: [{ paragraph: createEmptyParagraph() }],
    styles: [],
    numbering: [],
    headers: [],
    footers: [],
    images: [],
    comments: [],
    footnotes: [],
    endnotes: [],
  };
}

// ---------------------------------------------------------------------------
// Reducer
// ---------------------------------------------------------------------------

function documentReducer(
  state: DocumentState,
  action: DocumentAction,
): DocumentState {
  switch (action.type) {
    // ----- load_document -----
    case 'load_document': {
      return {
        document: action.payload,
        selection: null,
        undo_stack: [],
        redo_stack: [],
        is_dirty: false,
        editing_zone: 'body',
        find_replace_open: false,
        zoom: 100,
      };
    }

    // ----- set_selection -----
    case 'set_selection': {
      return { ...state, selection: action.payload };
    }

    // ----- insert_text -----
    case 'insert_text': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      let workingSelection = state.selection;

      // If there's a range selection, delete it first
      if (!state.selection.is_collapsed) {
        const deleteResult = deleteRange(doc, state.selection);
        if (!deleteResult) return state;
        workingSelection = deleteResult;
      }

      const loc = findParagraphAndRun(doc, workingSelection);
      if (!loc) return state;

      const { run } = loc;
      const offset = workingSelection.anchor.offset;
      run.text =
        run.text.slice(0, offset) + action.payload.text + run.text.slice(offset);

      const newOffset = offset + action.payload.text.length;
      const newSelection: EditorSelection = {
        anchor: { ...workingSelection.anchor, offset: newOffset },
        focus: { ...workingSelection.anchor, offset: newOffset },
        is_collapsed: true,
      };

      return {
        ...newState,
        document: doc,
        selection: newSelection,
        is_dirty: true,
      };
    }

    // ----- delete_text -----
    case 'delete_text': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      // Range selection: delete the range and return
      if (!state.selection.is_collapsed) {
        const resultSelection = deleteRange(doc, state.selection);
        if (!resultSelection) return state;
        return {
          ...newState,
          document: doc,
          selection: resultSelection,
          is_dirty: true,
        };
      }

      // Collapsed selection: single-char delete
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const { blockIndex, paragraph, runIndex, run } = loc;
      const offset = state.selection.anchor.offset;

      if (action.payload.direction === 'backward') {
        if (offset > 0) {
          // Delete one character before offset
          run.text = run.text.slice(0, offset - 1) + run.text.slice(offset);
          const sel: EditorSelection = {
            anchor: { ...state.selection.anchor, offset: offset - 1 },
            focus: { ...state.selection.anchor, offset: offset - 1 },
            is_collapsed: true,
          };
          return {
            ...newState,
            document: doc,
            selection: sel,
            is_dirty: true,
          };
        } else if (runIndex > 0) {
          // Merge with previous run
          const prevRun = paragraph.runs[runIndex - 1];
          const prevLength = prevRun.text.length;
          prevRun.text = prevRun.text + run.text;
          paragraph.runs.splice(runIndex, 1);
          const sel: EditorSelection = {
            anchor: {
              ...state.selection.anchor,
              run_index: runIndex - 1,
              offset: prevLength,
            },
            focus: {
              ...state.selection.anchor,
              run_index: runIndex - 1,
              offset: prevLength,
            },
            is_collapsed: true,
          };
          return {
            ...newState,
            document: doc,
            selection: sel,
            is_dirty: true,
          };
        } else if (blockIndex > 0) {
          // Merge with previous paragraph
          const prevBlock = doc.body[blockIndex - 1];
          if (!prevBlock || !('paragraph' in prevBlock)) return state;
          const prevParagraph = prevBlock.paragraph;
          const prevRunCount = prevParagraph.runs.length;
          const lastPrevRun = prevParagraph.runs[prevRunCount - 1];
          const prevOffset = lastPrevRun ? lastPrevRun.text.length : 0;

          // Append all runs from current paragraph to previous
          prevParagraph.runs.push(...paragraph.runs);
          doc.body.splice(blockIndex, 1);

          const sel: EditorSelection = {
            anchor: {
              block_index: blockIndex - 1,
              paragraph_id: prevParagraph.id,
              run_index: prevRunCount - 1,
              offset: prevOffset,
            },
            focus: {
              block_index: blockIndex - 1,
              paragraph_id: prevParagraph.id,
              run_index: prevRunCount - 1,
              offset: prevOffset,
            },
            is_collapsed: true,
          };
          return {
            ...newState,
            document: doc,
            selection: sel,
            is_dirty: true,
          };
        }
        return state;
      }

      // Forward delete
      if (offset < run.text.length) {
        run.text = run.text.slice(0, offset) + run.text.slice(offset + 1);
        return {
          ...newState,
          document: doc,
          selection: state.selection,
          is_dirty: true,
        };
      } else if (runIndex < paragraph.runs.length - 1) {
        // Merge with next run
        const nextRun = paragraph.runs[runIndex + 1];
        run.text = run.text + nextRun.text;
        paragraph.runs.splice(runIndex + 1, 1);
        return {
          ...newState,
          document: doc,
          selection: state.selection,
          is_dirty: true,
        };
      } else if (blockIndex < doc.body.length - 1) {
        // Merge with next paragraph
        const nextBlock = doc.body[blockIndex + 1];
        if (!nextBlock || !('paragraph' in nextBlock)) return state;
        const nextParagraph = nextBlock.paragraph;
        paragraph.runs.push(...nextParagraph.runs);
        doc.body.splice(blockIndex + 1, 1);
        return {
          ...newState,
          document: doc,
          selection: state.selection,
          is_dirty: true,
        };
      }

      return state;
    }

    // ----- split_paragraph -----
    case 'split_paragraph': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      let workingSelection = state.selection;

      // If there's a range selection, delete it first
      if (!state.selection.is_collapsed) {
        const deleteResult = deleteRange(doc, state.selection);
        if (!deleteResult) return state;
        workingSelection = deleteResult;
      }

      const loc = findParagraphAndRun(doc, workingSelection);
      if (!loc) return state;

      const { blockIndex, paragraph, runIndex, run } = loc;
      const offset = workingSelection.anchor.offset;

      // Split the current run at offset into two runs
      const beforeText = run.text.slice(0, offset);
      const afterText = run.text.slice(offset);

      run.text = beforeText;

      const newFirstRun: Run = {
        id: crypto.randomUUID(),
        text: afterText,
        properties: { ...run.properties },
      };

      // Runs before the split point (including the modified current run)
      const runsForCurrent = paragraph.runs.slice(0, runIndex + 1);
      // Runs after the split point
      const runsForNew = [newFirstRun, ...paragraph.runs.slice(runIndex + 1)];

      paragraph.runs = runsForCurrent;

      const newParagraph: Paragraph = {
        id: crypto.randomUUID(),
        runs: runsForNew.length > 0 ? runsForNew : [createEmptyRun()],
        properties: { ...paragraph.properties },
      };

      doc.body.splice(blockIndex + 1, 0, { paragraph: newParagraph });

      const newSelection: EditorSelection = {
        anchor: {
          block_index: blockIndex + 1,
          paragraph_id: newParagraph.id,
          run_index: 0,
          offset: 0,
        },
        focus: {
          block_index: blockIndex + 1,
          paragraph_id: newParagraph.id,
          run_index: 0,
          offset: 0,
        },
        is_collapsed: true,
      };

      return {
        ...newState,
        document: doc,
        selection: newSelection,
        is_dirty: true,
      };
    }

    // ----- toggle_bold -----
    case 'toggle_bold': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (state.selection.is_collapsed) {
        const loc = findParagraphAndRun(doc, state.selection);
        if (!loc) return state;
        loc.run.properties.bold = !loc.run.properties.bold;
      } else {
        const segments = getRunsInRange(doc, state.selection);
        const allBold = segments.every(s => s.run.properties.bold);
        applyRunPropertyToRange(doc, state.selection, 'bold', !allBold);
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- toggle_italic -----
    case 'toggle_italic': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (state.selection.is_collapsed) {
        const loc = findParagraphAndRun(doc, state.selection);
        if (!loc) return state;
        loc.run.properties.italic = !loc.run.properties.italic;
      } else {
        const segments = getRunsInRange(doc, state.selection);
        const allItalic = segments.every(s => s.run.properties.italic);
        applyRunPropertyToRange(doc, state.selection, 'italic', !allItalic);
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- toggle_underline -----
    case 'toggle_underline': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (state.selection.is_collapsed) {
        const loc = findParagraphAndRun(doc, state.selection);
        if (!loc) return state;
        loc.run.properties.underline = !loc.run.properties.underline;
      } else {
        const segments = getRunsInRange(doc, state.selection);
        const allUnderline = segments.every(s => s.run.properties.underline);
        applyRunPropertyToRange(doc, state.selection, 'underline', !allUnderline);
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- toggle_strikethrough -----
    case 'toggle_strikethrough': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (state.selection.is_collapsed) {
        const loc = findParagraphAndRun(doc, state.selection);
        if (!loc) return state;
        loc.run.properties.strikethrough = !loc.run.properties.strikethrough;
      } else {
        const segments = getRunsInRange(doc, state.selection);
        const allStrike = segments.every(s => s.run.properties.strikethrough);
        applyRunPropertyToRange(doc, state.selection, 'strikethrough', !allStrike);
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- toggle_superscript -----
    case 'toggle_superscript': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (state.selection.is_collapsed) {
        const loc = findParagraphAndRun(doc, state.selection);
        if (!loc) return state;
        loc.run.properties.superscript = !loc.run.properties.superscript;
        // Superscript and subscript are mutually exclusive
        if (loc.run.properties.superscript) {
          loc.run.properties.subscript = false;
        }
      } else {
        const segments = getRunsInRange(doc, state.selection);
        const allSuper = segments.every(s => s.run.properties.superscript);
        applyRunPropertyToRange(doc, state.selection, 'superscript', !allSuper);
        if (!allSuper) {
          applyRunPropertyToRange(doc, state.selection, 'subscript', false);
        }
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- toggle_subscript -----
    case 'toggle_subscript': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (state.selection.is_collapsed) {
        const loc = findParagraphAndRun(doc, state.selection);
        if (!loc) return state;
        loc.run.properties.subscript = !loc.run.properties.subscript;
        // Superscript and subscript are mutually exclusive
        if (loc.run.properties.subscript) {
          loc.run.properties.superscript = false;
        }
      } else {
        const segments = getRunsInRange(doc, state.selection);
        const allSub = segments.every(s => s.run.properties.subscript);
        applyRunPropertyToRange(doc, state.selection, 'subscript', !allSub);
        if (!allSub) {
          applyRunPropertyToRange(doc, state.selection, 'superscript', false);
        }
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- toggle_all_caps -----
    case 'toggle_all_caps': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (state.selection.is_collapsed) {
        const loc = findParagraphAndRun(doc, state.selection);
        if (!loc) return state;
        loc.run.properties.all_caps = !loc.run.properties.all_caps;
        if (loc.run.properties.all_caps) {
          loc.run.properties.small_caps = false;
        }
      } else {
        const segments = getRunsInRange(doc, state.selection);
        const allCaps = segments.every(s => s.run.properties.all_caps);
        applyRunPropertyToRange(doc, state.selection, 'all_caps', !allCaps);
        if (!allCaps) {
          applyRunPropertyToRange(doc, state.selection, 'small_caps', false);
        }
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- toggle_small_caps -----
    case 'toggle_small_caps': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (state.selection.is_collapsed) {
        const loc = findParagraphAndRun(doc, state.selection);
        if (!loc) return state;
        loc.run.properties.small_caps = !loc.run.properties.small_caps;
        if (loc.run.properties.small_caps) {
          loc.run.properties.all_caps = false;
        }
      } else {
        const segments = getRunsInRange(doc, state.selection);
        const allSmallCaps = segments.every(s => s.run.properties.small_caps);
        applyRunPropertyToRange(doc, state.selection, 'small_caps', !allSmallCaps);
        if (!allSmallCaps) {
          applyRunPropertyToRange(doc, state.selection, 'all_caps', false);
        }
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_font_size -----
    case 'set_font_size': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      applyRunPropertyToRange(doc, state.selection, 'font_size', action.payload.size);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_font_family -----
    case 'set_font_family': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      applyRunPropertyToRange(doc, state.selection, 'font_family', action.payload.family);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_color -----
    case 'set_color': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      applyRunPropertyToRange(doc, state.selection, 'color', action.payload.color);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_highlight -----
    case 'set_highlight': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (action.payload.color === null) {
        applyRunPropertyToRange(doc, state.selection, 'highlight', undefined);
      } else {
        applyRunPropertyToRange(doc, state.selection, 'highlight', action.payload.color);
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_alignment -----
    case 'set_alignment': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      loc.paragraph.properties.alignment = action.payload.alignment;

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_heading_level -----
    case 'set_heading_level': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      if (action.payload.level === null) {
        delete loc.paragraph.properties.heading_level;
      } else {
        loc.paragraph.properties.heading_level = action.payload.level;
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_line_spacing -----
    case 'set_line_spacing': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      loc.paragraph.properties.line_spacing = action.payload.spacing;
      if (action.payload.rule) {
        loc.paragraph.properties.line_spacing_rule = action.payload.rule as 'auto' | 'exact' | 'at_least';
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- toggle_bullet_list -----
    case 'toggle_bullet_list': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const numbering = loc.paragraph.properties.numbering;
      if (numbering && numbering.num_id === '1') {
        delete loc.paragraph.properties.numbering;
      } else {
        loc.paragraph.properties.numbering = { num_id: '1', level: 0 };
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- toggle_numbered_list -----
    case 'toggle_numbered_list': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const numbering = loc.paragraph.properties.numbering;
      if (numbering && numbering.num_id === '2') {
        delete loc.paragraph.properties.numbering;
      } else {
        loc.paragraph.properties.numbering = { num_id: '2', level: 0 };
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- insert_table -----
    case 'insert_table': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const blockIndex = state.selection.anchor.block_index;

      const { rows, cols } = action.payload;
      const tableRows: TableRow[] = [];

      for (let r = 0; r < rows; r++) {
        const cells: TableCell[] = [];
        for (let c = 0; c < cols; c++) {
          cells.push({
            id: crypto.randomUUID(),
            content: [{ paragraph: createEmptyParagraph() }],
            properties: {} as TableCellProperties,
          });
        }
        tableRows.push({
          id: crypto.randomUUID(),
          cells,
        });
      }

      const table: Table = {
        id: crypto.randomUUID(),
        rows: tableRows,
        properties: {} as TableProperties,
      };

      const tableBlock: BlockElement = { table };
      doc.body.splice(blockIndex + 1, 0, tableBlock);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- insert_image -----
    case 'insert_image': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      // Add image to document images array
      doc.images.push(action.payload);

      // Create a new run with the inline image reference
      const imageRun: Run = {
        id: crypto.randomUUID(),
        text: '',
        properties: {
          inline_image: action.payload.id,
        },
      };

      // Insert the image run after the current run
      loc.paragraph.runs.splice(loc.runIndex + 1, 0, imageRun);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_paragraph_indent -----
    case 'set_paragraph_indent': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const { indent_left, indent_right, indent_first_line } = action.payload;
      if (indent_left !== undefined) loc.paragraph.properties.indent_left = indent_left;
      if (indent_right !== undefined) loc.paragraph.properties.indent_right = indent_right;
      if (indent_first_line !== undefined) loc.paragraph.properties.indent_first_line = indent_first_line;

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- insert_hyperlink -----
    case 'insert_hyperlink': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const displayText = action.payload.text || action.payload.url;
      const hyperlinkRun: Run = {
        id: crypto.randomUUID(),
        text: displayText,
        properties: {
          hyperlink_url: action.payload.url,
          hyperlink_tooltip: action.payload.tooltip,
          color: '#0563C1',
          underline: true,
        },
      };

      // Insert the hyperlink run after the current run
      loc.paragraph.runs.splice(loc.runIndex + 1, 0, hyperlinkRun);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- insert_page_break -----
    case 'insert_page_break': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const pageBreakRun: Run = {
        id: crypto.randomUUID(),
        text: '',
        properties: {
          page_break: true,
        },
      };

      loc.paragraph.runs.splice(loc.runIndex + 1, 0, pageBreakRun);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- insert_line_break -----
    case 'insert_line_break': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const lineBreakRun: Run = {
        id: crypto.randomUUID(),
        text: '',
        properties: {
          line_break: true,
        },
      };

      loc.paragraph.runs.splice(loc.runIndex + 1, 0, lineBreakRun);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- grow_font -----
    case 'grow_font': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (state.selection.is_collapsed) {
        const loc = findParagraphAndRun(doc, state.selection);
        if (!loc) return state;
        const currentSize = loc.run.properties.font_size || 11;
        const nextSize = FONT_SIZE_SEQUENCE.find(s => s > currentSize);
        loc.run.properties.font_size = nextSize !== undefined ? nextSize : currentSize;
      } else {
        const segments = getRunsInRange(doc, state.selection);
        for (const seg of segments) {
          const currentSize = seg.run.properties.font_size || 11;
          const nextSize = FONT_SIZE_SEQUENCE.find(s => s > currentSize);
          seg.run.properties.font_size = nextSize !== undefined ? nextSize : currentSize;
        }
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- shrink_font -----
    case 'shrink_font': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (state.selection.is_collapsed) {
        const loc = findParagraphAndRun(doc, state.selection);
        if (!loc) return state;
        const currentSize = loc.run.properties.font_size || 11;
        let prevSize = currentSize;
        for (let i = FONT_SIZE_SEQUENCE.length - 1; i >= 0; i--) {
          if (FONT_SIZE_SEQUENCE[i] < currentSize) {
            prevSize = FONT_SIZE_SEQUENCE[i];
            break;
          }
        }
        loc.run.properties.font_size = prevSize;
      } else {
        const segments = getRunsInRange(doc, state.selection);
        for (const seg of segments) {
          const currentSize = seg.run.properties.font_size || 11;
          let prevSize = currentSize;
          for (let i = FONT_SIZE_SEQUENCE.length - 1; i >= 0; i--) {
            if (FONT_SIZE_SEQUENCE[i] < currentSize) {
              prevSize = FONT_SIZE_SEQUENCE[i];
              break;
            }
          }
          seg.run.properties.font_size = prevSize;
        }
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- increase_indent -----
    case 'increase_indent': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const currentIndent = loc.paragraph.properties.indent_left || 0;
      loc.paragraph.properties.indent_left = currentIndent + 36;

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- decrease_indent -----
    case 'decrease_indent': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const currentIndent = loc.paragraph.properties.indent_left || 0;
      loc.paragraph.properties.indent_left = Math.max(0, currentIndent - 36);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- delete_range -----
    case 'delete_range': {
      if (!state.selection || state.selection.is_collapsed) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      const resultSelection = deleteRange(doc, state.selection);
      if (!resultSelection) return state;

      return {
        ...newState,
        document: doc,
        selection: resultSelection,
        is_dirty: true,
      };
    }

    // ----- insert_row -----
    case 'insert_row': {
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const table = findTableById(doc, action.payload.table_id);
      if (!table) return state;

      const { after_row_index } = action.payload;
      const sourceRow = table.rows[after_row_index];
      if (!sourceRow) return state;

      const newCells: TableCell[] = sourceRow.cells.map(cell => ({
        id: crypto.randomUUID(),
        content: [{ paragraph: createEmptyParagraph() }],
        properties: { ...cell.properties },
      }));

      const newRow: TableRow = {
        id: crypto.randomUUID(),
        cells: newCells,
        properties: sourceRow.properties ? { ...sourceRow.properties } : undefined,
      };

      table.rows.splice(after_row_index + 1, 0, newRow);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- insert_column -----
    case 'insert_column': {
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const table = findTableById(doc, action.payload.table_id);
      if (!table) return state;

      const { after_col_index } = action.payload;

      for (const row of table.rows) {
        const newCell: TableCell = {
          id: crypto.randomUUID(),
          content: [{ paragraph: createEmptyParagraph() }],
          properties: {} as TableCellProperties,
        };
        row.cells.splice(after_col_index + 1, 0, newCell);
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- delete_row -----
    case 'delete_row': {
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const table = findTableById(doc, action.payload.table_id);
      if (!table) return state;

      const { row_index } = action.payload;
      if (row_index < 0 || row_index >= table.rows.length) return state;

      table.rows.splice(row_index, 1);

      // If table becomes empty, remove the table block from doc.body
      if (table.rows.length === 0) {
        const tableBlockIndex = doc.body.findIndex(
          block => 'table' in block && block.table.id === action.payload.table_id,
        );
        if (tableBlockIndex !== -1) {
          doc.body.splice(tableBlockIndex, 1);
        }
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- delete_column -----
    case 'delete_column': {
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const table = findTableById(doc, action.payload.table_id);
      if (!table) return state;

      const { col_index } = action.payload;

      for (const row of table.rows) {
        if (col_index >= 0 && col_index < row.cells.length) {
          row.cells.splice(col_index, 1);
        }
      }

      // If all rows end up with 0 cells, remove the table
      const allEmpty = table.rows.every(row => row.cells.length === 0);
      if (allEmpty) {
        const tableBlockIndex = doc.body.findIndex(
          block => 'table' in block && block.table.id === action.payload.table_id,
        );
        if (tableBlockIndex !== -1) {
          doc.body.splice(tableBlockIndex, 1);
        }
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_cell_shading -----
    case 'set_cell_shading': {
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      let found = false;
      for (const block of doc.body) {
        if ('table' in block) {
          for (const row of block.table.rows) {
            for (const cell of row.cells) {
              if (cell.id === action.payload.cell_id) {
                cell.properties.shading = action.payload.color;
                found = true;
                break;
              }
            }
            if (found) break;
          }
        }
        if (found) break;
      }

      if (!found) return state;

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- resize_image -----
    case 'resize_image': {
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      const image = doc.images.find(img => img.id === action.payload.image_id);
      if (!image) return state;

      image.width = action.payload.width;
      image.height = action.payload.height;

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- add_comment -----
    case 'add_comment': {
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (!doc.comments) {
        doc.comments = [];
      }

      const commentParagraph = createEmptyParagraph();
      commentParagraph.runs[0].text = action.payload.content;

      const comment: Comment = {
        id: crypto.randomUUID(),
        author: action.payload.author,
        date: new Date().toISOString(),
        content: [{ paragraph: commentParagraph }],
      };

      doc.comments.push(comment);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- delete_comment -----
    case 'delete_comment': {
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      if (!doc.comments) return state;

      doc.comments = doc.comments.filter(c => c.id !== action.payload.comment_id);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- insert_footnote -----
    case 'insert_footnote': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      if (!doc.footnotes) {
        doc.footnotes = [];
      }

      const footnoteParagraph = createEmptyParagraph();
      footnoteParagraph.runs[0].text = action.payload.content;

      const footnote: Footnote = {
        id: crypto.randomUUID(),
        content: [{ paragraph: footnoteParagraph }],
      };

      doc.footnotes.push(footnote);

      // Insert a run with footnote_ref at the cursor position
      const footnoteRefRun: Run = {
        id: crypto.randomUUID(),
        text: '',
        properties: {
          footnote_ref: footnote.id,
          superscript: true,
        },
      };

      loc.paragraph.runs.splice(loc.runIndex + 1, 0, footnoteRefRun);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- insert_endnote -----
    case 'insert_endnote': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      if (!doc.endnotes) {
        doc.endnotes = [];
      }

      const endnoteParagraph = createEmptyParagraph();
      endnoteParagraph.runs[0].text = action.payload.content;

      const endnote: Endnote = {
        id: crypto.randomUUID(),
        content: [{ paragraph: endnoteParagraph }],
      };

      doc.endnotes.push(endnote);

      // Insert a run with endnote_ref at the cursor position
      const endnoteRefRun: Run = {
        id: crypto.randomUUID(),
        text: '',
        properties: {
          endnote_ref: endnote.id,
          superscript: true,
        },
      };

      loc.paragraph.runs.splice(loc.runIndex + 1, 0, endnoteRefRun);

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_section_properties -----
    case 'set_section_properties': {
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      doc.section_properties = {
        ...(doc.section_properties || {}),
        ...action.payload,
      };

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_editing_zone -----
    case 'set_editing_zone': {
      return { ...state, editing_zone: action.payload };
    }

    // ----- toggle_find_replace -----
    case 'toggle_find_replace': {
      return { ...state, find_replace_open: !state.find_replace_open };
    }

    // ----- set_zoom -----
    case 'set_zoom': {
      const clamped = Math.max(25, Math.min(400, action.payload.zoom));
      return { ...state, zoom: clamped };
    }

    // ----- paste_runs -----
    case 'paste_runs': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);

      let workingSelection = state.selection;

      // If selection is not collapsed, delete the range first
      if (!state.selection.is_collapsed) {
        const deleteResult = deleteRange(doc, state.selection);
        if (!deleteResult) return state;
        workingSelection = deleteResult;
      }

      const loc = findParagraphAndRun(doc, workingSelection);
      if (!loc) return state;

      // Clone the pasted runs with new IDs
      const runsToInsert: Run[] = action.payload.runs.map(r => ({
        id: crypto.randomUUID(),
        text: r.text,
        properties: { ...r.properties },
      }));

      // Splice the pasted runs into the current paragraph after the current run
      loc.paragraph.runs.splice(loc.runIndex + 1, 0, ...runsToInsert);

      // Place cursor at the end of the last pasted run
      if (runsToInsert.length > 0) {
        const lastPastedRun = runsToInsert[runsToInsert.length - 1];
        const newRunIndex = loc.runIndex + runsToInsert.length;
        const newOffset = lastPastedRun.text.length;
        const sel: EditorSelection = {
          anchor: {
            ...workingSelection.anchor,
            run_index: newRunIndex,
            offset: newOffset,
          },
          focus: {
            ...workingSelection.anchor,
            run_index: newRunIndex,
            offset: newOffset,
          },
          is_collapsed: true,
        };
        return {
          ...newState,
          document: doc,
          selection: sel,
          is_dirty: true,
        };
      }

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- undo -----
    case 'undo': {
      if (state.undo_stack.length === 0) return state;
      const previousDoc = state.undo_stack[state.undo_stack.length - 1];
      return {
        ...state,
        document: previousDoc,
        undo_stack: state.undo_stack.slice(0, -1),
        redo_stack: [...state.redo_stack, cloneDocument(state.document)],
        is_dirty: true,
      };
    }

    // ----- redo -----
    case 'redo': {
      if (state.redo_stack.length === 0) return state;
      const nextDoc = state.redo_stack[state.redo_stack.length - 1];
      return {
        ...state,
        document: nextDoc,
        undo_stack: [...state.undo_stack, cloneDocument(state.document)],
        redo_stack: state.redo_stack.slice(0, -1),
        is_dirty: true,
      };
    }

    default:
      return state;
  }
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

const initialState: DocumentState = {
  document: createDefaultDocument(),
  selection: null,
  undo_stack: [],
  redo_stack: [],
  is_dirty: false,
  editing_zone: 'body',
  find_replace_open: false,
  zoom: 100,
};

export function useDocumentModel() {
  const [state, dispatch] = useReducer(documentReducer, initialState);
  return { state, dispatch };
}
