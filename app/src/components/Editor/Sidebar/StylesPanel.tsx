import React from 'react';
import { DocxDocument } from '../../../types/document';

interface StylesPanelProps {
  document: DocxDocument;
}

export const StylesPanel: React.FC<StylesPanelProps> = ({ document: doc }) => {
  if (doc.styles.length === 0) {
    return <div className="styles-empty">No styles defined</div>;
  }

  return (
    <div className="styles-panel">
      {doc.styles.map((s) => (
        <div key={s.id} className="style-item">
          <span className="style-name">{s.name}</span>
          <span className="style-type">{s.style_type}</span>
        </div>
      ))}
    </div>
  );
};
