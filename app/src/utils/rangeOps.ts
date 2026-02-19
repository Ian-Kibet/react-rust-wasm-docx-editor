import {
  BlockElement,
  DocxDocument,
  Paragraph,
  Run,
  RunProperties,
  Table,
} from '../types/document';
import { EditorSelection, SelectionPoint } from '../types/editor';

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface RangeSegment {
  blockIndex: number;
  paragraph: Paragraph;
  runIndex: number;
  run: Run;
  startOffset: number;
  endOffset: number;
  isFullRun: boolean;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Recursively search a BlockElement for a paragraph with the given id.
 * For plain paragraphs this is a direct match; for tables we walk every cell's
 * content (which itself can contain nested tables).
 */
export function findParagraphInBlock(
  block: BlockElement,
  paragraphId: string,
): Paragraph | null {
  if ('paragraph' in block) {
    return block.paragraph.id === paragraphId ? block.paragraph : null;
  }

  if ('table' in block) {
    const table: Table = block.table;
    for (const row of table.rows) {
      for (const cell of row.cells) {
        for (const innerBlock of cell.content) {
          const found = findParagraphInBlock(innerBlock, paragraphId);
          if (found) return found;
        }
      }
    }
  }

  return null;
}

/**
 * Compare two SelectionPoints in document order.
 * Returns < 0 if a comes before b, 0 if equal, > 0 if a comes after b.
 */
function comparePoints(a: SelectionPoint, b: SelectionPoint): number {
  if (a.block_index !== b.block_index) return a.block_index - b.block_index;
  if (a.run_index !== b.run_index) return a.run_index - b.run_index;
  return a.offset - b.offset;
}

// ---------------------------------------------------------------------------
// normalizeSelection
// ---------------------------------------------------------------------------

/**
 * Return the selection endpoints in document order (start <= end) regardless
 * of the direction in which the user dragged.
 */
export function normalizeSelection(
  sel: EditorSelection,
): { start: SelectionPoint; end: SelectionPoint } {
  const cmp = comparePoints(sel.anchor, sel.focus);
  if (cmp <= 0) {
    return { start: { ...sel.anchor }, end: { ...sel.focus } };
  }
  return { start: { ...sel.focus }, end: { ...sel.anchor } };
}

// ---------------------------------------------------------------------------
// getRunsInRange
// ---------------------------------------------------------------------------

/**
 * Walk the document body from the normalised start to end of the selection
 * and collect every RangeSegment that falls (fully or partially) within the
 * selected range.
 */
export function getRunsInRange(
  doc: DocxDocument,
  selection: EditorSelection,
): RangeSegment[] {
  const { start, end } = normalizeSelection(selection);
  const segments: RangeSegment[] = [];

  for (
    let blockIdx = start.block_index;
    blockIdx <= end.block_index;
    blockIdx++
  ) {
    const block = doc.body[blockIdx];
    if (!block) continue;

    // Determine which paragraph(s) we need from this block.
    // For the start/end blocks we must match by paragraph_id because the
    // block might be a table containing many paragraphs.
    const isStartBlock = blockIdx === start.block_index;
    const isEndBlock = blockIdx === end.block_index;

    // Collect the paragraph we care about from this block.
    let paragraph: Paragraph | null = null;

    if ('paragraph' in block) {
      paragraph = block.paragraph;
    } else {
      // Table block – we need to find the paragraph(s) by id.
      // For start and end blocks we search for the specific paragraph_id.
      // For blocks strictly between start and end we include ALL paragraphs.
      if (isStartBlock || isEndBlock) {
        const targetId = isStartBlock
          ? start.paragraph_id
          : end.paragraph_id;
        paragraph = findParagraphInBlock(block, targetId);
      }

      // For purely-middle blocks or if start and end are in the same table
      // but different paragraphs we need a broader sweep.  However the
      // SelectionPoint model uses block_index, so two different paragraphs
      // inside the same table block would share a block_index but have
      // different paragraph_ids.  When start and end fall in the same block
      // but reference different paragraphs we need both plus everything in
      // between.
      if (
        isStartBlock &&
        isEndBlock &&
        start.paragraph_id !== end.paragraph_id
      ) {
        // Collect all paragraphs between start and end within this block.
        const allParas = collectParagraphsInBlock(block);
        const startIdx = allParas.findIndex(
          (p) => p.id === start.paragraph_id,
        );
        const endIdx = allParas.findIndex(
          (p) => p.id === end.paragraph_id,
        );

        if (startIdx !== -1 && endIdx !== -1) {
          const lo = Math.min(startIdx, endIdx);
          const hi = Math.max(startIdx, endIdx);
          for (let pi = lo; pi <= hi; pi++) {
            const para = allParas[pi];
            const isFirst = para.id === start.paragraph_id;
            const isLast = para.id === end.paragraph_id;
            collectRunSegments(
              segments,
              blockIdx,
              para,
              isFirst ? start.run_index : 0,
              isFirst ? start.offset : 0,
              isLast ? end.run_index : para.runs.length - 1,
              isLast ? end.offset : para.runs[para.runs.length - 1]?.text.length ?? 0,
            );
          }
          continue; // already processed this block
        }
        // Fallback: couldn't locate both – just use the one we found above.
      }

      if (!isStartBlock && !isEndBlock) {
        // Middle block inside a table: include every paragraph.
        const allParas = collectParagraphsInBlock(block);
        for (const para of allParas) {
          collectRunSegments(
            segments,
            blockIdx,
            para,
            0,
            0,
            para.runs.length - 1,
            para.runs[para.runs.length - 1]?.text.length ?? 0,
          );
        }
        continue;
      }
    }

    if (!paragraph) continue;
    if (paragraph.runs.length === 0) continue;

    // Determine run range + offsets for this paragraph.
    const firstRun = isStartBlock ? start.run_index : 0;
    const firstOffset = isStartBlock ? start.offset : 0;
    const lastRun = isEndBlock
      ? end.run_index
      : paragraph.runs.length - 1;
    const lastOffset = isEndBlock
      ? end.offset
      : paragraph.runs[paragraph.runs.length - 1]?.text.length ?? 0;

    collectRunSegments(
      segments,
      blockIdx,
      paragraph,
      firstRun,
      firstOffset,
      lastRun,
      lastOffset,
    );
  }

  return segments;
}

/** Gather all Paragraphs inside a block in document order (flattened). */
function collectParagraphsInBlock(block: BlockElement): Paragraph[] {
  const out: Paragraph[] = [];
  if ('paragraph' in block) {
    out.push(block.paragraph);
  } else if ('table' in block) {
    for (const row of block.table.rows) {
      for (const cell of row.cells) {
        for (const inner of cell.content) {
          out.push(...collectParagraphsInBlock(inner));
        }
      }
    }
  }
  return out;
}

/**
 * Push RangeSegments for runs [firstRun..lastRun] within a single paragraph
 * into the provided array.
 */
function collectRunSegments(
  out: RangeSegment[],
  blockIndex: number,
  paragraph: Paragraph,
  firstRun: number,
  firstOffset: number,
  lastRun: number,
  lastOffset: number,
): void {
  // Clamp indices to the actual runs array.
  const clampedFirst = Math.max(0, Math.min(firstRun, paragraph.runs.length - 1));
  const clampedLast = Math.max(0, Math.min(lastRun, paragraph.runs.length - 1));

  for (let ri = clampedFirst; ri <= clampedLast; ri++) {
    const run = paragraph.runs[ri];
    const sOffset = ri === clampedFirst ? firstOffset : 0;
    const eOffset = ri === clampedLast ? lastOffset : run.text.length;

    out.push({
      blockIndex,
      paragraph,
      runIndex: ri,
      run,
      startOffset: sOffset,
      endOffset: eOffset,
      isFullRun: sOffset === 0 && eOffset === run.text.length,
    });
  }
}

// ---------------------------------------------------------------------------
// splitRunAtOffset
// ---------------------------------------------------------------------------

/**
 * Split the run at `runIndex` inside the given paragraph at `offset`.
 *
 * After the call the original run contains text[0..offset) and a new run
 * (with a fresh UUID) containing text[offset..) is inserted immediately
 * after it in `paragraph.runs`.
 *
 * This is a no-op when offset is 0 or >= the run's text length (nothing to
 * split).
 */
export function splitRunAtOffset(
  paragraph: Paragraph,
  runIndex: number,
  offset: number,
): void {
  const run = paragraph.runs[runIndex];
  if (!run) return;
  if (offset <= 0 || offset >= run.text.length) return;

  const newRun: Run = {
    id: crypto.randomUUID(),
    text: run.text.slice(offset),
    properties: { ...run.properties },
  };

  run.text = run.text.slice(0, offset);
  paragraph.runs.splice(runIndex + 1, 0, newRun);
}

// ---------------------------------------------------------------------------
// applyRunPropertyToRange
// ---------------------------------------------------------------------------

/**
 * Set a single RunProperty on every run (or partial run) that falls within the
 * selection range.  Mutates `doc` in place.
 *
 * When the selection is collapsed the property is simply set on the run at the
 * anchor position (typing will inherit the property).
 *
 * When there is a range:
 *  1. The first run is split at startOffset (if not at the beginning).
 *  2. The last run is split at endOffset (if not at the end).
 *  3. The property is applied to every fully-covered run in between.
 *
 * Returns an updated EditorSelection whose indices account for any splits
 * that occurred.
 */
export function applyRunPropertyToRange(
  doc: DocxDocument,
  selection: EditorSelection,
  property: keyof RunProperties,
  value: unknown,
): EditorSelection {
  // --- Collapsed selection: apply to the current run -----------------------
  if (selection.is_collapsed) {
    const block = doc.body[selection.anchor.block_index];
    if (!block) return selection;
    const para = findParagraphInBlock(block, selection.anchor.paragraph_id);
    if (!para) return selection;
    const run = para.runs[selection.anchor.run_index];
    if (!run) return selection;
    (run.properties as Record<string, unknown>)[property] = value;
    return selection;
  }

  // --- Range selection -----------------------------------------------------
  const { start, end } = normalizeSelection(selection);

  // Locate the start and end paragraphs.
  const startBlock = doc.body[start.block_index];
  const endBlock = doc.body[end.block_index];
  if (!startBlock || !endBlock) return selection;

  const startPara = findParagraphInBlock(startBlock, start.paragraph_id);
  const endPara = findParagraphInBlock(endBlock, end.paragraph_id);
  if (!startPara || !endPara) return selection;

  // Track how many runs were inserted before the "end" pointer so we can
  // adjust the returned selection.
  let endRunShift = 0;
  let startRunShift = 0;

  // --- Split last run first (so indices for the first run stay valid) ------
  const endRun = endPara.runs[end.run_index];
  if (endRun && end.offset < endRun.text.length && end.offset > 0) {
    splitRunAtOffset(endPara, end.run_index, end.offset);
    // The part we want to format is now fully in endPara.runs[end.run_index].
  }

  // --- Split first run -----------------------------------------------------
  const startRun = startPara.runs[start.run_index];
  if (startRun && start.offset > 0) {
    splitRunAtOffset(startPara, start.run_index, start.offset);
    // After the split the text we want begins in the NEXT run.
    startRunShift = 1;

    // If start and end are in the same paragraph and after the same or later
    // run, all subsequent run indices shift by 1.
    if (
      start.paragraph_id === end.paragraph_id &&
      end.run_index >= start.run_index
    ) {
      endRunShift = 1;
    }
  }

  const effectiveStartRunIdx = start.run_index + startRunShift;
  const effectiveEndRunIdx = end.run_index + endRunShift;

  // --- Apply the property across the range ---------------------------------
  // We now walk every affected paragraph and set the property on the
  // appropriate runs.
  const sameParagraph = start.paragraph_id === end.paragraph_id;

  if (sameParagraph) {
    for (let ri = effectiveStartRunIdx; ri <= effectiveEndRunIdx; ri++) {
      const run = startPara.runs[ri];
      if (run) {
        (run.properties as Record<string, unknown>)[property] = value;
      }
    }
  } else {
    // First paragraph: from effectiveStartRunIdx to the end.
    for (let ri = effectiveStartRunIdx; ri < startPara.runs.length; ri++) {
      (startPara.runs[ri].properties as Record<string, unknown>)[property] =
        value;
    }

    // Middle blocks / paragraphs.
    for (
      let bi = start.block_index;
      bi <= end.block_index;
      bi++
    ) {
      const block = doc.body[bi];
      if (!block) continue;

      const parasInBlock = collectParagraphsInBlock(block);
      for (const para of parasInBlock) {
        if (para.id === start.paragraph_id || para.id === end.paragraph_id) {
          continue; // handled separately
        }

        // Check if this paragraph is between start and end paragraphs.
        // For blocks strictly between start and end blocks it always is.
        // For the start or end block we need to check ordering within the
        // block.
        if (bi > start.block_index && bi < end.block_index) {
          for (const run of para.runs) {
            (run.properties as Record<string, unknown>)[property] = value;
          }
        } else if (bi === start.block_index || bi === end.block_index) {
          // Same block as start or end but different paragraph.
          // We include it if it falls between start and end paragraphs.
          const allInBlock = parasInBlock;
          const startIdx = allInBlock.findIndex(
            (p) => p.id === start.paragraph_id,
          );
          const endIdx = allInBlock.findIndex(
            (p) => p.id === end.paragraph_id,
          );
          const thisIdx = allInBlock.findIndex((p) => p.id === para.id);
          if (
            thisIdx > Math.min(startIdx, endIdx) &&
            thisIdx < Math.max(startIdx, endIdx)
          ) {
            for (const run of para.runs) {
              (run.properties as Record<string, unknown>)[property] = value;
            }
          }
        }
      }
    }

    // Last paragraph: from start to effectiveEndRunIdx.
    for (let ri = 0; ri <= effectiveEndRunIdx; ri++) {
      const run = endPara.runs[ri];
      if (run) {
        (run.properties as Record<string, unknown>)[property] = value;
      }
    }
  }

  // --- Build updated selection ---------------------------------------------
  const newStart: SelectionPoint = {
    ...start,
    run_index: effectiveStartRunIdx,
    offset: 0,
  };
  const newEnd: SelectionPoint = {
    ...end,
    run_index: effectiveEndRunIdx,
    offset: endPara.runs[effectiveEndRunIdx]?.text.length ?? 0,
  };

  // Preserve original anchor/focus direction.
  const anchorIsStart = comparePoints(selection.anchor, selection.focus) <= 0;
  return {
    anchor: anchorIsStart ? newStart : newEnd,
    focus: anchorIsStart ? newEnd : newStart,
    is_collapsed: false,
  };
}

// ---------------------------------------------------------------------------
// deleteRange
// ---------------------------------------------------------------------------

/**
 * Delete all content within the given (non-collapsed) selection range.
 * Mutates `doc` in place.
 *
 * Algorithm:
 *  - Normalise the selection.
 *  - Same paragraph: trim boundary runs, splice out fully enclosed runs.
 *  - Cross-paragraph: keep text before start in first paragraph, keep text
 *    after end in last paragraph, merge the two, remove paragraphs in between
 *    (and any blocks that become empty), splice doc.body.
 *
 * Returns a collapsed selection at the start of the deleted range.
 */
export function deleteRange(
  doc: DocxDocument,
  selection: EditorSelection,
): EditorSelection {
  if (selection.is_collapsed) return selection;

  const { start, end } = normalizeSelection(selection);

  const startBlock = doc.body[start.block_index];
  const endBlock = doc.body[end.block_index];
  if (!startBlock || !endBlock) return selection;

  const startPara = findParagraphInBlock(startBlock, start.paragraph_id);
  const endPara = findParagraphInBlock(endBlock, end.paragraph_id);
  if (!startPara || !endPara) return selection;

  const collapsed: EditorSelection = {
    anchor: { ...start },
    focus: { ...start },
    is_collapsed: true,
  };

  // --- Same paragraph ------------------------------------------------------
  if (start.paragraph_id === end.paragraph_id) {
    if (start.run_index === end.run_index) {
      // Same run: remove the selected substring.
      const run = startPara.runs[start.run_index];
      if (run) {
        run.text =
          run.text.slice(0, start.offset) + run.text.slice(end.offset);
      }
    } else {
      // Trim the start run.
      const sRun = startPara.runs[start.run_index];
      if (sRun) {
        sRun.text = sRun.text.slice(0, start.offset);
      }

      // Trim the end run.
      const eRun = startPara.runs[end.run_index];
      if (eRun) {
        eRun.text = eRun.text.slice(end.offset);
      }

      // Remove fully-enclosed runs between start and end.
      const removeStart = start.run_index + 1;
      const removeCount = end.run_index - start.run_index - 1;
      if (removeCount > 0) {
        startPara.runs.splice(removeStart, removeCount);
      }

      // Clean up empty runs (but keep at least one run in the paragraph so
      // the caret has somewhere to live).
      cleanEmptyRuns(startPara);
    }

    // Ensure the collapsed offset stays within the now-modified run.
    const clampedRun = startPara.runs[start.run_index];
    if (clampedRun && start.offset > clampedRun.text.length) {
      collapsed.anchor.offset = clampedRun.text.length;
      collapsed.focus.offset = clampedRun.text.length;
    }

    return collapsed;
  }

  // --- Cross-paragraph deletion --------------------------------------------

  // 1. Trim the first paragraph: keep text before start, remove everything
  //    after (including fully-enclosed runs).
  const sRun = startPara.runs[start.run_index];
  if (sRun) {
    sRun.text = sRun.text.slice(0, start.offset);
  }
  // Remove all runs after the start run in the first paragraph.
  if (start.run_index + 1 < startPara.runs.length) {
    startPara.runs.splice(start.run_index + 1);
  }

  // 2. Trim the last paragraph: keep text after end, remove everything
  //    before.
  const eRun = endPara.runs[end.run_index];
  if (eRun) {
    eRun.text = eRun.text.slice(end.offset);
  }
  // Remove all runs before the end run in the last paragraph.
  if (end.run_index > 0) {
    endPara.runs.splice(0, end.run_index);
  }

  // 3. Merge: append remaining runs of the last paragraph onto the first.
  //    If the first paragraph's last run is empty after trimming, we can
  //    just replace it with the end paragraph's runs; otherwise append.
  const trailingRuns = endPara.runs;

  // If the start run is now empty AND there are trailing runs to merge,
  // replace the empty start run with the trailing runs.
  const lastStartRun = startPara.runs[startPara.runs.length - 1];
  if (lastStartRun && lastStartRun.text.length === 0 && trailingRuns.length > 0) {
    startPara.runs.splice(startPara.runs.length - 1, 1, ...trailingRuns);
  } else {
    startPara.runs.push(...trailingRuns);
  }

  // Clean up empty runs.
  cleanEmptyRuns(startPara);

  // 4. Remove intermediate blocks/paragraphs from doc.body.
  //    For top-level paragraphs this means removing the blocks between
  //    start.block_index and end.block_index.
  //    - If start and end share the same block_index (both inside the same
  //      table) we do NOT remove the block, but we do need to clean out the
  //      paragraphs in between.
  if (start.block_index === end.block_index) {
    // Both in the same table: remove intermediate paragraphs.
    // This is a simplification – we remove all cell content paragraphs
    // strictly between startPara and endPara.
    if ('table' in startBlock) {
      removeIntermediateParagraphsInTable(
        startBlock.table,
        start.paragraph_id,
        end.paragraph_id,
      );
    }
  } else {
    // Remove all blocks strictly between start and end.
    const removeBlockStart = start.block_index + 1;
    const removeBlockCount = end.block_index - start.block_index - 1;
    if (removeBlockCount > 0) {
      doc.body.splice(removeBlockStart, removeBlockCount);
    }

    // The end block is now at start.block_index + 1.  If it was a plain
    // paragraph we remove it entirely (its content has been merged).  If it
    // was a table we need to remove the end paragraph from within it.
    const shiftedEndIdx = start.block_index + 1;
    const shiftedEndBlock = doc.body[shiftedEndIdx];
    if (shiftedEndBlock) {
      if ('paragraph' in shiftedEndBlock) {
        doc.body.splice(shiftedEndIdx, 1);
      } else if ('table' in shiftedEndBlock) {
        removeParagraphFromTable(shiftedEndBlock.table, end.paragraph_id);
        // If the table is now empty, remove it.
        if (isTableEmpty(shiftedEndBlock.table)) {
          doc.body.splice(shiftedEndIdx, 1);
        }
      }
    }
  }

  // Ensure the collapsed offset is valid.
  const collapsedRun = startPara.runs[start.run_index];
  if (collapsedRun && start.offset > collapsedRun.text.length) {
    collapsed.anchor.offset = collapsedRun.text.length;
    collapsed.focus.offset = collapsedRun.text.length;
  }

  return collapsed;
}

// ---------------------------------------------------------------------------
// deleteRange helpers
// ---------------------------------------------------------------------------

/** Remove empty runs from a paragraph but always keep at least one. */
function cleanEmptyRuns(paragraph: Paragraph): void {
  for (let i = paragraph.runs.length - 1; i >= 0; i--) {
    if (paragraph.runs.length <= 1) break;
    if (paragraph.runs[i].text.length === 0) {
      paragraph.runs.splice(i, 1);
    }
  }
}

/** Remove all paragraphs strictly between two paragraph IDs inside a table. */
function removeIntermediateParagraphsInTable(
  table: Table,
  startId: string,
  endId: string,
): void {
  let foundStart = false;
  let foundEnd = false;

  for (const row of table.rows) {
    for (const cell of row.cells) {
      for (let i = cell.content.length - 1; i >= 0; i--) {
        const block = cell.content[i];
        if ('paragraph' in block) {
          if (block.paragraph.id === startId) {
            foundStart = true;
          } else if (block.paragraph.id === endId) {
            foundEnd = true;
          } else if (foundStart && !foundEnd) {
            cell.content.splice(i, 1);
          }
        } else if ('table' in block) {
          removeIntermediateParagraphsInTable(
            block.table,
            startId,
            endId,
          );
        }
      }
    }
  }
}

/** Remove a single paragraph (by id) from within a table. */
function removeParagraphFromTable(table: Table, paragraphId: string): void {
  for (const row of table.rows) {
    for (const cell of row.cells) {
      for (let i = cell.content.length - 1; i >= 0; i--) {
        const block = cell.content[i];
        if (
          'paragraph' in block &&
          block.paragraph.id === paragraphId
        ) {
          cell.content.splice(i, 1);
          return;
        }
        if ('table' in block) {
          removeParagraphFromTable(block.table, paragraphId);
        }
      }
    }
  }
}

/** Check whether a table has any content left. */
function isTableEmpty(table: Table): boolean {
  for (const row of table.rows) {
    for (const cell of row.cells) {
      if (cell.content.length > 0) return false;
    }
  }
  return true;
}

// ---------------------------------------------------------------------------
// getSelectedText
// ---------------------------------------------------------------------------

/**
 * Extract the plain text content of the current selection.
 *
 * Runs within a single paragraph are concatenated directly.  When the
 * selection spans multiple paragraphs a newline is inserted between them.
 */
export function getSelectedText(
  doc: DocxDocument,
  selection: EditorSelection,
): string {
  if (selection.is_collapsed) return '';

  const segments = getRunsInRange(doc, selection);
  if (segments.length === 0) return '';

  const parts: string[] = [];
  let lastParagraphId: string | null = null;

  for (const seg of segments) {
    // Insert a newline when we transition to a new paragraph.
    if (lastParagraphId !== null && seg.paragraph.id !== lastParagraphId) {
      parts.push('\n');
    }
    lastParagraphId = seg.paragraph.id;

    parts.push(seg.run.text.slice(seg.startOffset, seg.endOffset));
  }

  return parts.join('');
}
