import { parse_docx, generate_docx, version } from './pkg/docx_wasm.js';
import { DocxDocument, DocumentStatistics } from '../types/document';

export function parseDocx(data: Uint8Array): DocxDocument {
  const json = parse_docx(data);
  return JSON.parse(json) as DocxDocument;
}

export function generateDocxBlob(doc: DocxDocument): Blob {
  const json = JSON.stringify(doc);
  const bytes = generate_docx(json);
  return new Blob([bytes], {
    type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  });
}

export function getWasmVersion(): string {
  return version();
}

/**
 * Compute document statistics (word count, character count, paragraph count)
 * using pure JS since the WASM module may not expose these.
 */
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
          if (run.properties && 'image_id' in run.properties) image_count++;
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
