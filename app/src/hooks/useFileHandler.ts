import { useCallback } from 'react';
import { parseDocx, generateDocxBlob } from '../wasm/bridge';
import { DocxDocument } from '../types/document';

export function useFileHandler(onLoad: (doc: DocxDocument) => void) {
  const handleUpload = useCallback((file: File) => {
    const reader = new FileReader();
    reader.onload = () => {
      const data = new Uint8Array(reader.result as ArrayBuffer);
      try {
        const doc = parseDocx(data);
        onLoad(doc);
      } catch (e) {
        console.error('Failed to parse DOCX:', e);
        alert('Failed to parse DOCX file: ' + (e as Error).message);
      }
    };
    reader.readAsArrayBuffer(file);
  }, [onLoad]);

  const handleDownload = useCallback((doc: DocxDocument, filename?: string) => {
    try {
      const blob = generateDocxBlob(doc);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename || 'document.docx';
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error('Failed to generate DOCX:', e);
      alert('Failed to generate DOCX: ' + (e as Error).message);
    }
  }, []);

  return { handleUpload, handleDownload };
}
