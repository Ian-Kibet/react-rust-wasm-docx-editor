import React from 'react';
import { DocxDocument, Paragraph, Run } from '../../../types/document';
import { EditorSelection } from '../../../types/editor';

interface PropertiesPanelProps {
  document: DocxDocument;
  selection: EditorSelection | null;
}

export const PropertiesPanel: React.FC<PropertiesPanelProps> = ({
  document: doc,
  selection,
}) => {
  if (!selection) {
    return <div className="props-empty">No selection</div>;
  }

  const block = doc.body[selection.anchor.block_index];
  if (!block || !('paragraph' in block)) {
    return <div className="props-empty">Select a paragraph</div>;
  }

  const p = (block as { paragraph: Paragraph }).paragraph;
  const r: Run | undefined = p.runs[selection.anchor.run_index];
  const pp = p.properties;
  const rp = r?.properties;

  return (
    <div className="properties-panel">
      <h4>Paragraph</h4>
      <div className="prop-row">
        <span>Alignment:</span> <span>{pp.alignment ?? 'left'}</span>
      </div>
      {pp.heading_level && (
        <div className="prop-row">
          <span>Heading:</span> <span>H{pp.heading_level}</span>
        </div>
      )}
      {pp.spacing_before != null && (
        <div className="prop-row">
          <span>Spacing before:</span> <span>{pp.spacing_before}pt</span>
        </div>
      )}
      {pp.spacing_after != null && (
        <div className="prop-row">
          <span>Spacing after:</span> <span>{pp.spacing_after}pt</span>
        </div>
      )}

      {rp && (
        <>
          <h4>Run</h4>
          <div className="prop-row">
            <span>Font:</span> <span>{rp.font_family ?? 'default'}</span>
          </div>
          <div className="prop-row">
            <span>Size:</span> <span>{rp.font_size ?? 12}pt</span>
          </div>
          <div className="prop-row">
            <span>Color:</span> <span>{rp.color ?? '#000000'}</span>
          </div>
          <div className="prop-row">
            <span>Bold:</span> <span>{rp.bold ? 'Yes' : 'No'}</span>
          </div>
          <div className="prop-row">
            <span>Italic:</span> <span>{rp.italic ? 'Yes' : 'No'}</span>
          </div>
          <div className="prop-row">
            <span>Underline:</span> <span>{rp.underline ? 'Yes' : 'No'}</span>
          </div>
        </>
      )}
    </div>
  );
};
