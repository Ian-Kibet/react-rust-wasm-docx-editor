import React, { useCallback } from 'react';
import { ToolbarButton } from '../Toolbar/ToolbarButton';
import { RibbonGroup } from './RibbonGroup';
import { DocumentAction } from '../../../hooks/useDocumentModel';

interface InsertRibbonProps {
  dispatch: React.Dispatch<DocumentAction>;
}

export const InsertRibbon: React.FC<InsertRibbonProps> = ({ dispatch }) => {
  const handleInsertTable = useCallback(() => {
    dispatch({ type: 'insert_table', payload: { rows: 3, cols: 3 } });
  }, [dispatch]);

  const handleInsertImage = useCallback(() => {
    const input = window.document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = () => {
        const base64 = (reader.result as string).split(',')[1];
        const id = crypto.randomUUID();
        dispatch({
          type: 'insert_image',
          payload: {
            id,
            data_base64: base64,
            content_type: file.type,
          },
        });
      };
      reader.readAsDataURL(file);
    };
    input.click();
  }, [dispatch]);

  return (
    <div className="ribbon-panel">
      <RibbonGroup label="Tables">
        <ToolbarButton label="Table" title="Insert 3x3 Table" size="lg" onClick={handleInsertTable} />
      </RibbonGroup>
      <RibbonGroup label="Illustrations">
        <ToolbarButton label="Image" title="Insert Image" size="lg" onClick={handleInsertImage} />
      </RibbonGroup>
    </div>
  );
};
