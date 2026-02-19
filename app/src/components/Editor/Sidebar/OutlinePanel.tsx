import React from 'react';
import { DocxDocument, Paragraph } from '../../../types/document';

interface OutlinePanelProps {
  document: DocxDocument;
}

export const OutlinePanel: React.FC<OutlinePanelProps> = ({ document: doc }) => {
  const headings = doc.body
    .filter((b): b is { paragraph: Paragraph } => 'paragraph' in b)
    .map((b) => b.paragraph)
    .filter((p) => p.properties.heading_level);

  if (headings.length === 0) {
    return <div className="outline-empty">No headings in document</div>;
  }

  return (
    <div className="outline-panel">
      {headings.map((h) => (
        <div
          key={h.id}
          className="outline-item"
          style={{ paddingLeft: `${((h.properties.heading_level ?? 1) - 1) * 12 + 4}px` }}
          onClick={() => {
            const el = window.document.querySelector(`[data-paragraph-id="${h.id}"]`);
            el?.scrollIntoView({ behavior: 'smooth', block: 'center' });
          }}
        >
          <span className="outline-level">H{h.properties.heading_level}</span>
          {h.runs.map((r) => r.text).join('')}
        </div>
      ))}
    </div>
  );
};
