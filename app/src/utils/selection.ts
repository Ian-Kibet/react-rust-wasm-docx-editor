import { SelectionPoint, EditorSelection } from '../types/editor';

/**
 * Resolve a DOM node + offset pair to a model SelectionPoint by walking up the
 * DOM tree and reading the data-* attributes placed on rendered elements.
 *
 * Returns null when the node is outside the editor or when required attributes
 * are missing from the ancestor chain.
 */
function resolvePoint(
  node: Node,
  offset: number,
  editorEl: HTMLElement,
): SelectionPoint | null {
  let runId: string | null = null;
  let paragraphId: string | null = null;
  let blockIndex: number | null = null;
  let runIndex: number = 0;

  // If we landed on a text node, start the attribute search from its parent
  // element. For element nodes we start directly.
  const startEl =
    node.nodeType === Node.TEXT_NODE ? node.parentElement : (node as Element);

  if (!startEl || !editorEl.contains(startEl)) return null;

  let el: Element | null = startEl;

  while (el && el !== editorEl) {
    if (!runId && el.hasAttribute('data-run-id')) {
      runId = el.getAttribute('data-run-id')!;
    }

    if (!paragraphId && el.hasAttribute('data-paragraph-id')) {
      paragraphId = el.getAttribute('data-paragraph-id')!;

      // Determine the run's ordinal position among its sibling runs
      if (runId) {
        const spans = el.querySelectorAll('[data-run-id]');
        for (let i = 0; i < spans.length; i++) {
          if (spans[i].getAttribute('data-run-id') === runId) {
            runIndex = i;
            break;
          }
        }
      }
    }

    if (blockIndex === null && el.hasAttribute('data-block-index')) {
      blockIndex = parseInt(el.getAttribute('data-block-index')!, 10);
    }

    el = el.parentElement;
  }

  if (blockIndex === null || !paragraphId) return null;

  return {
    block_index: blockIndex,
    paragraph_id: paragraphId,
    run_index: runIndex,
    offset: node.nodeType === Node.TEXT_NODE ? offset : 0,
  };
}

/**
 * Convert the current browser DOM selection to model coordinates.
 *
 * Returns null when there is no active selection or when the selection falls
 * outside (or partially outside) the editor element.
 */
export function domSelectionToModel(
  editorEl: HTMLElement,
): EditorSelection | null {
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0) return null;

  const { anchorNode, anchorOffset, focusNode, focusOffset } = sel;
  if (!anchorNode || !focusNode) return null;

  const anchor = resolvePoint(anchorNode, anchorOffset, editorEl);
  const focus = resolvePoint(focusNode, focusOffset, editorEl);

  if (!anchor || !focus) return null;

  const is_collapsed =
    anchor.block_index === focus.block_index &&
    anchor.paragraph_id === focus.paragraph_id &&
    anchor.run_index === focus.run_index &&
    anchor.offset === focus.offset;

  return { anchor, focus, is_collapsed };
}

/**
 * Locate the DOM text node (or element) and offset that correspond to a given
 * run index + character offset within a paragraph element.
 *
 * Falls back to the end of the last run when `runIndex` exceeds the available
 * runs.
 */
function findTextNodeAtOffset(
  paragraphEl: Element,
  runIndex: number,
  offset: number,
): { node: Node; offset: number } | null {
  const runs = paragraphEl.querySelectorAll('[data-run-id]');

  if (runs.length === 0) return null;

  if (runIndex >= runs.length) {
    // Fallback: clamp to the last run, placing the cursor at the end.
    const lastRun = runs[runs.length - 1];
    const textNode = lastRun.firstChild;
    return textNode
      ? { node: textNode, offset: (textNode.textContent || '').length }
      : null;
  }

  const runEl = runs[runIndex];
  const textNode = runEl.firstChild;

  if (!textNode) {
    // The run is empty – return the element itself so that the browser can
    // still place a caret there.
    return { node: runEl, offset: 0 };
  }

  const maxOffset = (textNode.textContent || '').length;
  return { node: textNode, offset: Math.min(offset, maxOffset) };
}

/**
 * Map a model SelectionPoint back to a concrete DOM node + offset pair inside
 * the editor element.
 *
 * Returns null when the matching block, paragraph, or run cannot be found in
 * the DOM.
 */
function resolveModelToDOM(
  editorEl: HTMLElement,
  point: SelectionPoint,
): { node: Node; offset: number } | null {
  const blockEl = editorEl.querySelector(
    `[data-block-index="${point.block_index}"]`,
  );
  if (!blockEl) return null;

  const paraEl = blockEl.getAttribute('data-paragraph-id') === point.paragraph_id
    ? blockEl
    : blockEl.querySelector(`[data-paragraph-id="${point.paragraph_id}"]`);
  if (!paraEl) return null;

  return findTextNodeAtOffset(paraEl, point.run_index, point.offset);
}

/**
 * Restore (or set) the browser DOM selection to match the given model
 * coordinates.
 *
 * This is the inverse of `domSelectionToModel`.  It silently returns when the
 * target DOM nodes cannot be resolved – the caller should treat the stored
 * selection as stale in that case.
 */
export function restoreDomSelection(
  editorEl: HTMLElement,
  selection: EditorSelection,
): void {
  const sel = window.getSelection();
  if (!sel) return;

  const anchorResult = resolveModelToDOM(editorEl, selection.anchor);
  const focusResult = selection.is_collapsed
    ? anchorResult
    : resolveModelToDOM(editorEl, selection.focus);

  if (!anchorResult || !focusResult) return;

  const range = document.createRange();
  range.setStart(anchorResult.node, anchorResult.offset);
  range.setEnd(focusResult.node, focusResult.offset);

  sel.removeAllRanges();
  sel.addRange(range);
}
