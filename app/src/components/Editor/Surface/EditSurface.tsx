import React, { useRef, useCallback, useEffect, useLayoutEffect } from 'react';
import { DocxDocument, BlockElement, Paragraph, ImageData } from '../../../types/document';
import { EditorSelection } from '../../../types/editor';
import { domSelectionToModel, restoreDomSelection } from '../../../utils/selection';
import ParagraphRenderer from './ParagraphRenderer';
import ListRenderer from './ListRenderer';
import TableRenderer from './TableRenderer';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type DocumentAction = any;

interface EditSurfaceProps {
  document: DocxDocument;
  selection: EditorSelection | null;
  dispatch: React.Dispatch<DocumentAction>;
  images: ImageData[];
}

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
// Component
// ---------------------------------------------------------------------------

const EditSurface: React.FC<EditSurfaceProps> = ({
  document: doc,
  selection,
  dispatch,
  images,
}) => {
  const editorRef = useRef<HTMLDivElement>(null);
  const isRestoringRef = useRef(false);
  const prevDocRef = useRef(doc);

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
    if (!el || !selection) return;

    // Only restore when document content changed — user-initiated selection
    // changes are already reflected in the DOM.
    if (prevDocRef.current === doc) return;
    prevDocRef.current = doc;

    isRestoringRef.current = true;
    restoreDomSelection(el, selection);
    requestAnimationFrame(() => {
      isRestoringRef.current = false;
    });
  }, [doc, selection]);

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

        case 'insertParagraph':
        case 'insertLineBreak': {
          e.preventDefault();
          dispatch({ type: 'split_paragraph' });
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

      // Tab inside a list could be used for indent. For now, just prevent the
      // default so the browser does not move focus away from the editor.
      if (e.key === 'Tab') {
        // TODO: implement indent/dedent for list items
        e.preventDefault();
        return;
      }
    },
    [dispatch],
  );

  // -----------------------------------------------------------------------
  // Render body
  // -----------------------------------------------------------------------

  const chunks = groupBodyChunks(doc.body);

  return (
    <div
      ref={editorRef}
      className="edit-surface"
      contentEditable={true}
      suppressContentEditableWarning={true}
      onBeforeInput={handleBeforeInput}
      onKeyDown={handleKeyDown}
      spellCheck={false}
      role="textbox"
      aria-multiline={true}
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
  );
};

EditSurface.displayName = 'EditSurface';

export default EditSurface;
