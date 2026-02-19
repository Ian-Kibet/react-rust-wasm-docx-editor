import { parse_docx, generate_docx, version } from './pkg/docx_wasm.js';
import { DocxDocument } from '../types/document';

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
