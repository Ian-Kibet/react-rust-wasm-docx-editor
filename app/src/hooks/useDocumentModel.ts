import { useReducer } from 'react';
import { v4 as uuidv4 } from 'uuid';
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
} from '../types/document';
import { EditorSelection } from '../types/editor';

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export interface DocumentState {
  document: DocxDocument;
  selection: EditorSelection | null;
  undo_stack: DocxDocument[];
  redo_stack: DocxDocument[];
  is_dirty: boolean;
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
  | { type: 'set_font_size'; payload: { size: number } }
  | { type: 'set_font_family'; payload: { family: string } }
  | { type: 'set_color'; payload: { color: string } }
  | { type: 'set_alignment'; payload: { alignment: Alignment } }
  | { type: 'set_heading_level'; payload: { level: number | null } }
  | { type: 'toggle_bullet_list' }
  | { type: 'toggle_numbered_list' }
  | { type: 'insert_table'; payload: { rows: number; cols: number } }
  | { type: 'insert_image'; payload: ImageData }
  | { type: 'set_paragraph_indent'; payload: { indent_left?: number; indent_right?: number; indent_first_line?: number } }
  | { type: 'undo' }
  | { type: 'redo' };

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function cloneDocument(doc: DocxDocument): DocxDocument {
  return JSON.parse(JSON.stringify(doc));
}

function pushUndo(state: DocumentState): DocumentState {
  return {
    ...state,
    undo_stack: [...state.undo_stack, cloneDocument(state.document)],
    redo_stack: [],
  };
}

function createEmptyRun(): Run {
  return {
    id: uuidv4(),
    text: '',
    properties: {},
  };
}

function createEmptyParagraph(): Paragraph {
  return {
    id: uuidv4(),
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

export function createDefaultDocument(): DocxDocument {
  return {
    body: [{ paragraph: createEmptyParagraph() }],
    styles: [],
    numbering: [],
    headers: [],
    footers: [],
    images: [],
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
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const { run } = loc;
      const offset = state.selection.anchor.offset;
      run.text =
        run.text.slice(0, offset) + action.payload.text + run.text.slice(offset);

      const newOffset = offset + action.payload.text.length;
      const newSelection: EditorSelection = {
        anchor: { ...state.selection.anchor, offset: newOffset },
        focus: { ...state.selection.anchor, offset: newOffset },
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
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const { blockIndex, paragraph, runIndex, run } = loc;
      const offset = state.selection.anchor.offset;

      if (action.payload.direction === 'backward') {
        if (offset > 0) {
          // Delete one character before offset
          run.text = run.text.slice(0, offset - 1) + run.text.slice(offset);
          const newSelection: EditorSelection = {
            anchor: { ...state.selection.anchor, offset: offset - 1 },
            focus: { ...state.selection.anchor, offset: offset - 1 },
            is_collapsed: true,
          };
          return {
            ...newState,
            document: doc,
            selection: newSelection,
            is_dirty: true,
          };
        } else if (runIndex > 0) {
          // Merge with previous run
          const prevRun = paragraph.runs[runIndex - 1];
          const prevLength = prevRun.text.length;
          prevRun.text = prevRun.text + run.text;
          paragraph.runs.splice(runIndex, 1);
          const newSelection: EditorSelection = {
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
            selection: newSelection,
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

          const newSelection: EditorSelection = {
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
            selection: newSelection,
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
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      const { blockIndex, paragraph, runIndex, run } = loc;
      const offset = state.selection.anchor.offset;

      // Split the current run at offset into two runs
      const beforeText = run.text.slice(0, offset);
      const afterText = run.text.slice(offset);

      run.text = beforeText;

      const newFirstRun: Run = {
        id: uuidv4(),
        text: afterText,
        properties: { ...run.properties },
      };

      // Runs before the split point (including the modified current run)
      const runsForCurrent = paragraph.runs.slice(0, runIndex + 1);
      // Runs after the split point
      const runsForNew = [newFirstRun, ...paragraph.runs.slice(runIndex + 1)];

      paragraph.runs = runsForCurrent;

      const newParagraph: Paragraph = {
        id: uuidv4(),
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
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      loc.run.properties.bold = !loc.run.properties.bold;

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- toggle_italic -----
    case 'toggle_italic': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      loc.run.properties.italic = !loc.run.properties.italic;

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- toggle_underline -----
    case 'toggle_underline': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      loc.run.properties.underline = !loc.run.properties.underline;

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_font_size -----
    case 'set_font_size': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      loc.run.properties.font_size = action.payload.size;

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_font_family -----
    case 'set_font_family': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      loc.run.properties.font_family = action.payload.family;

      return { ...newState, document: doc, is_dirty: true };
    }

    // ----- set_color -----
    case 'set_color': {
      if (!state.selection) return state;
      const newState = pushUndo(state);
      const doc = cloneDocument(newState.document);
      const loc = findParagraphAndRun(doc, state.selection);
      if (!loc) return state;

      loc.run.properties.color = action.payload.color;

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
            id: uuidv4(),
            content: [{ paragraph: createEmptyParagraph() }],
            properties: {} as TableCellProperties,
          });
        }
        tableRows.push({
          id: uuidv4(),
          cells,
        });
      }

      const table: Table = {
        id: uuidv4(),
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
        id: uuidv4(),
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
};

export function useDocumentModel() {
  const [state, dispatch] = useReducer(documentReducer, initialState);
  return { state, dispatch };
}
