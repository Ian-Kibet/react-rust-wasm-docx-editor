import React, { useRef, useCallback, useEffect, useLayoutEffect, useState } from 'react';
import { DocxDocument, BlockElement, Paragraph, ImageData, Run, SectionProperties } from '../../../types/document';
import { EditorSelection } from '../../../types/editor';
import { domSelectionToModel, restoreDomSelection } from '../../../utils/selection';
import { getRunsInRange, getSelectedText } from '../../../utils/rangeOps';
import ParagraphRenderer from './ParagraphRenderer';
import ListRenderer from './ListRenderer';
import TableRenderer from './TableRenderer';
import { ContextMenu, ContextMenuItem } from '../ContextMenu/ContextMenu';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type DocumentAction = any;

interface EditSurfaceProps {
  document: DocxDocument;
  selection: EditorSelection | null;
  dispatch: React.Dispatch<DocumentAction>;
  images: ImageData[];
  onSave?: () => void;
  onToggleFindReplace?: () => void;
  zoom?: number;
  sectionProperties?: SectionProperties;
}

// ---------------------------------------------------------------------------
// Module-level clipboard state for internal rich paste
// ---------------------------------------------------------------------------

let lastCopiedRuns: Run[] | null = null;

// ---------------------------------------------------------------------------
// Helpers for grouping consecutive list paragraphs
// ---------------------------------------------------------------------------

type BodyChunk =
  | { kind: 'paragraph'; block: BlockElement; index: number }
  | { kind: 'table'; block: BlockElement; index: number }
  | {
      kind: 'list';
      paragraphs: Paragraph[];
      listType: 'bullet' | 'numbered';
      startIndex: number;
    };

/**
 * Walk the document body and group consecutive paragraphs that share the same
 * `numbering.num_id` into list chunks. Non-list paragraphs and tables pass
 * through as individual chunks.
 */
function groupBodyChunks(body: BlockElement[]): BodyChunk[] {
  const chunks: BodyChunk[] = [];

  let i = 0;
  while (i < body.length) {
    const block = body[i];

    if ('table' in block) {
      chunks.push({ kind: 'table', block, index: i });
      i++;
      continue;
    }

    if ('paragraph' in block) {
      const para = block.paragraph;
      const numbering = para.properties.numbering;

      if (numbering) {
        // Start a list group. Collect consecutive paragraphs with the same
        // num_id.
        const numId = numbering.num_id;
        const startIndex = i;
        const paragraphs: Paragraph[] = [para];

        i++;
        while (i < body.length) {
          const next = body[i];
          if (
            'paragraph' in next &&
            next.paragraph.properties.numbering?.num_id === numId
          ) {
            paragraphs.push(next.paragraph);
            i++;
          } else {
            break;
          }
        }

        // Determine list type from the num_id convention used by the reducer:
        // num_id "1" = bullet, num_id "2" = numbered. For document-loaded
        // numbering we fall back to checking the numbering definitions, but
        // a simple heuristic works for now.
        const listType: 'bullet' | 'numbered' =
          numId === '2' ? 'numbered' : 'bullet';

        chunks.push({ kind: 'list', paragraphs, listType, startIndex });
        continue;
      }

      // Plain paragraph (no numbering).
      chunks.push({ kind: 'paragraph', block, index: i });
      i++;
      continue;
    }

    // Fallback for unknown block shapes.
    i++;
  }

  return chunks;
}

// ---------------------------------------------------------------------------
// Helper: find first paragraph in a block element (walks into tables)
// ---------------------------------------------------------------------------

function findFirstParagraph(block: BlockElement): Paragraph | null {
  if ('paragraph' in block) return block.paragraph;
  if ('table' in block) {
    for (const row of block.table.rows) {
      for (const cell of row.cells) {
        for (const cellBlock of cell.content) {
          const found = findFirstParagraph(cellBlock);
          if (found) return found;
        }
      }
    }
  }
  return null;
}

function findLastParagraph(block: BlockElement): Paragraph | null {
  if ('paragraph' in block) return block.paragraph;
  if ('table' in block) {
    const rows = block.table.rows;
    for (let r = rows.length - 1; r >= 0; r--) {
      const cells = rows[r].cells;
      for (let c = cells.length - 1; c >= 0; c--) {
        const content = cells[c].content;
        for (let b = content.length - 1; b >= 0; b--) {
          const found = findLastParagraph(content[b]);
          if (found) return found;
        }
      }
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const EditSurface: React.FC<EditSurfaceProps> = ({
  document: doc,
  selection: stateSel,
  dispatch,
  images,
  onSave,
  onToggleFindReplace,
  zoom,
  sectionProperties,
}) => {
  const editorRef = useRef<HTMLDivElement>(null);
  const isRestoringRef = useRef(false);
  const prevDocRef = useRef(doc);

  // Context menu state
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    items: ContextMenuItem[];
  } | null>(null);

  // -----------------------------------------------------------------------
  // Selection sync: DOM -> model
  // -----------------------------------------------------------------------

  const handleSelectionChange = useCallback(() => {
    if (isRestoringRef.current) return;

    const el = editorRef.current;
    if (!el) return;

    // Only update when the selection is inside the editor — clicking ribbon
    // buttons or other UI should not clear the editor selection.
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return;
    if (!el.contains(sel.anchorNode)) return;

    const modelSelection = domSelectionToModel(el);
    dispatch({ type: 'set_selection', payload: modelSelection });
  }, [dispatch]);

  useEffect(() => {
    const handler = handleSelectionChange;
    document.addEventListener('selectionchange', handler);
    return () => {
      document.removeEventListener('selectionchange', handler);
    };
  }, [handleSelectionChange]);

  // -----------------------------------------------------------------------
  // Selection sync: model -> DOM (restore after document mutations)
  // -----------------------------------------------------------------------

  useLayoutEffect(() => {
    const el = editorRef.current;
    if (!el || !stateSel) return;

    // Only restore when document content changed — user-initiated selection
    // changes are already reflected in the DOM.
    if (prevDocRef.current === doc) return;
    prevDocRef.current = doc;

    isRestoringRef.current = true;
    restoreDomSelection(el, stateSel);
    requestAnimationFrame(() => {
      isRestoringRef.current = false;
    });
  }, [doc, stateSel]);

  // -----------------------------------------------------------------------
  // Clipboard: Copy
  // -----------------------------------------------------------------------

  const handleCopy = useCallback(async () => {
    if (!stateSel || stateSel.is_collapsed) return;

    const text = getSelectedText(doc, stateSel);
    const segments = getRunsInRange(doc, stateSel);
    const runs = segments.map(s => ({
      ...s.run,
      text: s.run.text.slice(s.startOffset, s.endOffset),
    }));

    try {
      await navigator.clipboard.write([
        new ClipboardItem({
          'text/plain': new Blob([text], { type: 'text/plain' }),
        }),
      ]);
    } catch {
      // Fallback: execCommand
      const ta = document.createElement('textarea');
      ta.value = text;
      document.body.appendChild(ta);
      ta.select();
      document.execCommand('copy');
      document.body.removeChild(ta);
    }

    // Store runs in module-level variable for internal paste
    lastCopiedRuns = runs;
  }, [doc, stateSel]);

  // -----------------------------------------------------------------------
  // Clipboard: Cut
  // -----------------------------------------------------------------------

  const handleCut = useCallback(async () => {
    await handleCopy();
    dispatch({ type: 'delete_range' });
  }, [handleCopy, dispatch]);

  // -----------------------------------------------------------------------
  // Clipboard: Paste
  // -----------------------------------------------------------------------

  const handlePaste = useCallback(async () => {
    if (lastCopiedRuns) {
      dispatch({
        type: 'paste_runs',
        payload: { runs: JSON.parse(JSON.stringify(lastCopiedRuns)) },
      });
    } else {
      try {
        const text = await navigator.clipboard.readText();
        if (text) {
          dispatch({ type: 'insert_text', payload: { text } });
        }
      } catch {
        // Clipboard read failed — ignore
      }
    }
  }, [dispatch]);

  // -----------------------------------------------------------------------
  // Select All helper
  // -----------------------------------------------------------------------

  const handleSelectAll = useCallback(() => {
    const body = doc.body;
    if (body.length === 0) return;

    const firstBlock = body[0];
    const lastBlock = body[body.length - 1];
    const firstPara = findFirstParagraph(firstBlock);
    const lastPara = findLastParagraph(lastBlock);

    if (firstPara && lastPara) {
      const lastRunIndex = Math.max(0, lastPara.runs.length - 1);
      const lastRunLength = lastPara.runs[lastRunIndex]?.text.length ?? 0;

      const sel: EditorSelection = {
        anchor: {
          block_index: 0,
          paragraph_id: firstPara.id,
          run_index: 0,
          offset: 0,
        },
        focus: {
          block_index: body.length - 1,
          paragraph_id: lastPara.id,
          run_index: lastRunIndex,
          offset: lastRunLength,
        },
        is_collapsed: false,
      };
      dispatch({ type: 'set_selection', payload: sel });
    }
  }, [doc, dispatch]);

  // -----------------------------------------------------------------------
  // Input interception
  // -----------------------------------------------------------------------

  const handleBeforeInput = useCallback(
    (e: React.FormEvent<HTMLDivElement>) => {
      const event = e.nativeEvent as InputEvent;

      switch (event.inputType) {
        case 'insertText':
        case 'insertCompositionText': {
          e.preventDefault();
          if (event.data) {
            dispatch({ type: 'insert_text', payload: { text: event.data } });
          }
          break;
        }

        case 'deleteContentBackward': {
          e.preventDefault();
          dispatch({
            type: 'delete_text',
            payload: { direction: 'backward' },
          });
          break;
        }

        case 'deleteContentForward': {
          e.preventDefault();
          dispatch({
            type: 'delete_text',
            payload: { direction: 'forward' },
          });
          break;
        }

        case 'insertParagraph': {
          e.preventDefault();
          dispatch({ type: 'split_paragraph' });
          break;
        }

        case 'insertLineBreak': {
          e.preventDefault();
          dispatch({ type: 'insert_line_break' });
          break;
        }

        case 'insertFromPaste':
        case 'insertFromDrop': {
          e.preventDefault();
          const transfer = (event as InputEvent).dataTransfer;
          const text = transfer?.getData('text/plain') ?? event.data ?? '';
          if (text) {
            dispatch({ type: 'insert_text', payload: { text } });
          }
          break;
        }

        case 'insertReplacementText': {
          e.preventDefault();
          if (event.data) {
            dispatch({ type: 'insert_text', payload: { text: event.data } });
          }
          break;
        }

        default:
          // Prevent all unhandled input types from modifying the DOM directly,
          // which would desync it from the document model.
          e.preventDefault();
          break;
      }
    },
    [dispatch],
  );

  // -----------------------------------------------------------------------
  // Keyboard shortcuts
  // -----------------------------------------------------------------------

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      const mod = e.metaKey || e.ctrlKey;

      // -------------------------------------------------------------------
      // Clipboard shortcuts
      // -------------------------------------------------------------------

      if (mod && e.key === 'c') {
        e.preventDefault();
        handleCopy();
        return;
      }

      if (mod && e.key === 'x') {
        e.preventDefault();
        handleCopy().then(() => dispatch({ type: 'delete_range' }));
        return;
      }

      if (mod && e.key === 'v') {
        e.preventDefault();
        if (lastCopiedRuns) {
          dispatch({
            type: 'paste_runs',
            payload: { runs: JSON.parse(JSON.stringify(lastCopiedRuns)) },
          });
        } else {
          navigator.clipboard
            .readText()
            .then(text => {
              if (text) dispatch({ type: 'insert_text', payload: { text } });
            })
            .catch(() => {});
        }
        return;
      }

      // -------------------------------------------------------------------
      // Select All
      // -------------------------------------------------------------------

      if (mod && e.key === 'a') {
        e.preventDefault();
        handleSelectAll();
        return;
      }

      // -------------------------------------------------------------------
      // Basic formatting
      // -------------------------------------------------------------------

      if (mod && e.key === 'b') {
        e.preventDefault();
        dispatch({ type: 'toggle_bold' });
        return;
      }

      if (mod && e.key === 'i') {
        e.preventDefault();
        dispatch({ type: 'toggle_italic' });
        return;
      }

      if (mod && e.key === 'u') {
        e.preventDefault();
        dispatch({ type: 'toggle_underline' });
        return;
      }

      // Strikethrough: Ctrl+Shift+X
      if (mod && e.shiftKey && e.key === 'X') {
        e.preventDefault();
        dispatch({ type: 'toggle_strikethrough' });
        return;
      }

      // Superscript / Subscript: Ctrl+= / Ctrl+Shift+=
      if (mod && e.key === '=') {
        e.preventDefault();
        if (e.shiftKey) {
          dispatch({ type: 'toggle_superscript' });
        } else {
          dispatch({ type: 'toggle_subscript' });
        }
        return;
      }

      // -------------------------------------------------------------------
      // Alignment shortcuts
      // -------------------------------------------------------------------

      if (mod && e.key === 'l') {
        e.preventDefault();
        dispatch({ type: 'set_alignment', payload: { alignment: 'left' } });
        return;
      }

      if (mod && e.key === 'e') {
        e.preventDefault();
        dispatch({ type: 'set_alignment', payload: { alignment: 'center' } });
        return;
      }

      if (mod && e.key === 'r') {
        e.preventDefault();
        dispatch({ type: 'set_alignment', payload: { alignment: 'right' } });
        return;
      }

      if (mod && e.key === 'j') {
        e.preventDefault();
        dispatch({ type: 'set_alignment', payload: { alignment: 'justify' } });
        return;
      }

      // -------------------------------------------------------------------
      // Font size shortcuts
      // -------------------------------------------------------------------

      if (mod && e.key === ']') {
        e.preventDefault();
        dispatch({ type: 'grow_font' });
        return;
      }

      if (mod && e.key === '[') {
        e.preventDefault();
        dispatch({ type: 'shrink_font' });
        return;
      }

      // -------------------------------------------------------------------
      // Undo / Redo
      // -------------------------------------------------------------------

      if (mod && e.key === 'z' && !e.shiftKey) {
        e.preventDefault();
        dispatch({ type: 'undo' });
        return;
      }

      if (mod && (e.key === 'y' || (e.key === 'z' && e.shiftKey))) {
        e.preventDefault();
        dispatch({ type: 'redo' });
        return;
      }

      // -------------------------------------------------------------------
      // Find / Replace
      // -------------------------------------------------------------------

      if (mod && e.key === 'f') {
        e.preventDefault();
        onToggleFindReplace?.();
        return;
      }

      if (mod && e.key === 'h') {
        e.preventDefault();
        onToggleFindReplace?.();
        return;
      }

      // -------------------------------------------------------------------
      // Save
      // -------------------------------------------------------------------

      if (mod && e.key === 's') {
        e.preventDefault();
        onSave?.();
        return;
      }

      // -------------------------------------------------------------------
      // Page break (Ctrl+Enter)
      // -------------------------------------------------------------------

      if (mod && e.key === 'Enter') {
        e.preventDefault();
        dispatch({ type: 'insert_page_break' });
        return;
      }

      // -------------------------------------------------------------------
      // Line break (Shift+Enter)
      // -------------------------------------------------------------------

      if (!mod && e.shiftKey && e.key === 'Enter') {
        e.preventDefault();
        dispatch({ type: 'insert_line_break' });
        return;
      }

      // -------------------------------------------------------------------
      // Tab: indent / dedent in lists
      // -------------------------------------------------------------------

      if (e.key === 'Tab') {
        e.preventDefault();
        if (e.shiftKey) {
          dispatch({ type: 'decrease_indent' });
        } else {
          dispatch({ type: 'increase_indent' });
        }
        return;
      }
    },
    [dispatch, handleCopy, handleSelectAll, onSave, onToggleFindReplace],
  );

  // -----------------------------------------------------------------------
  // Context menu
  // -----------------------------------------------------------------------

  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();

      const target = e.target as HTMLElement;
      const tableEl = target.closest('[data-table-id]');

      const baseItems: ContextMenuItem[] = [
        {
          label: 'Cut',
          shortcut: 'Ctrl+X',
          onClick: () => {
            handleCopy().then(() => dispatch({ type: 'delete_range' }));
          },
        },
        {
          label: 'Copy',
          shortcut: 'Ctrl+C',
          onClick: () => {
            handleCopy();
          },
        },
        {
          label: 'Paste',
          shortcut: 'Ctrl+V',
          onClick: () => {
            navigator.clipboard
              .readText()
              .then(t => {
                if (t) dispatch({ type: 'insert_text', payload: { text: t } });
              })
              .catch(() => {});
          },
        },
        { separator: true },
        {
          label: 'Select All',
          shortcut: 'Ctrl+A',
          onClick: () => {
            handleSelectAll();
          },
        },
      ];

      let items = baseItems;

      if (tableEl) {
        const tableId = tableEl.getAttribute('data-table-id') || '';
        const rowEl = target.closest('tr');
        const cellEl = target.closest('td');
        const rowIndex = rowEl
          ? Array.from(rowEl.parentElement?.children ?? []).indexOf(rowEl)
          : 0;
        const colIndex =
          cellEl && rowEl ? Array.from(rowEl.children).indexOf(cellEl) : 0;

        items = [
          ...baseItems,
          { separator: true },
          {
            label: 'Insert Row Above',
            onClick: () =>
              dispatch({
                type: 'insert_row',
                payload: { table_id: tableId, after_row_index: rowIndex - 1 },
              }),
          },
          {
            label: 'Insert Row Below',
            onClick: () =>
              dispatch({
                type: 'insert_row',
                payload: { table_id: tableId, after_row_index: rowIndex },
              }),
          },
          {
            label: 'Insert Column Left',
            onClick: () =>
              dispatch({
                type: 'insert_column',
                payload: { table_id: tableId, after_col_index: colIndex - 1 },
              }),
          },
          {
            label: 'Insert Column Right',
            onClick: () =>
              dispatch({
                type: 'insert_column',
                payload: { table_id: tableId, after_col_index: colIndex },
              }),
          },
          { separator: true },
          {
            label: 'Delete Row',
            onClick: () =>
              dispatch({
                type: 'delete_row',
                payload: { table_id: tableId, row_index: rowIndex },
              }),
          },
          {
            label: 'Delete Column',
            onClick: () =>
              dispatch({
                type: 'delete_column',
                payload: { table_id: tableId, col_index: colIndex },
              }),
          },
        ];
      }

      setContextMenu({ x: e.clientX, y: e.clientY, items });
    },
    [dispatch, handleCopy, handleSelectAll],
  );

  // Close context menu on any click within the editor surface
  const handleClick = useCallback(() => {
    if (contextMenu) {
      setContextMenu(null);
    }
  }, [contextMenu]);

  // -----------------------------------------------------------------------
  // Zoom & section properties styling
  // -----------------------------------------------------------------------

  const surfaceStyle: React.CSSProperties = {};
  if (zoom && zoom !== 100) {
    surfaceStyle.transform = `scale(${zoom / 100})`;
    surfaceStyle.transformOrigin = 'top center';
  }
  if (sectionProperties) {
    if (sectionProperties.page_width) {
      surfaceStyle.maxWidth = `${sectionProperties.page_width / 20}pt`; // twips to pt
    }
    if (
      sectionProperties.margin_left != null ||
      sectionProperties.margin_right != null
    ) {
      surfaceStyle.paddingLeft = `${(sectionProperties.margin_left ?? 1440) / 20}pt`;
      surfaceStyle.paddingRight = `${(sectionProperties.margin_right ?? 1440) / 20}pt`;
    }
  }

  // -----------------------------------------------------------------------
  // Render body
  // -----------------------------------------------------------------------

  const chunks = groupBodyChunks(doc.body);

  return (
    <>
      <div
        ref={editorRef}
        className="edit-surface"
        contentEditable={true}
        suppressContentEditableWarning={true}
        onBeforeInput={handleBeforeInput}
        onKeyDown={handleKeyDown}
        onContextMenu={handleContextMenu}
        onClick={handleClick}
        spellCheck={false}
        role="textbox"
        aria-multiline={true}
        aria-label="Document editor"
        style={Object.keys(surfaceStyle).length > 0 ? surfaceStyle : undefined}
      >
        {chunks.map((chunk) => {
          switch (chunk.kind) {
            case 'paragraph': {
              const para = (chunk.block as { paragraph: Paragraph }).paragraph;
              return (
                <div key={para.id} data-block-index={chunk.index}>
                  <ParagraphRenderer paragraph={para} images={images} />
                </div>
              );
            }

            case 'list': {
              // Use a composite key from the first paragraph id + startIndex.
              const key = `list-${chunk.paragraphs[0].id}`;
              return (
                <div key={key} data-block-index={chunk.startIndex}>
                  <ListRenderer
                    paragraphs={chunk.paragraphs}
                    listType={chunk.listType}
                    images={images}
                    startIndex={chunk.startIndex}
                  />
                </div>
              );
            }

            case 'table': {
              const tbl = (chunk.block as { table: import('../../../types/document').Table }).table;
              return (
                <div key={tbl.id} data-block-index={chunk.index}>
                  <TableRenderer
                    table={tbl}
                    images={images}
                    dispatch={dispatch}
                  />
                </div>
              );
            }

            default:
              return null;
          }
        })}
      </div>
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={contextMenu.items}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  );
};

EditSurface.displayName = 'EditSurface';

export default EditSurface;
