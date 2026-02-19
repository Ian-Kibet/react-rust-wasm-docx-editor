import React, { useCallback, useState } from 'react';
import { ToolbarButton } from '../Toolbar/ToolbarButton';
import { RibbonGroup } from './RibbonGroup';
import { DocumentAction } from '../../../hooks/useDocumentModel';

interface InsertRibbonProps {
  dispatch: React.Dispatch<DocumentAction>;
}

const MAX_GRID_ROWS = 8;
const MAX_GRID_COLS = 10;

const TableGridPicker: React.FC<{ onSelect: (rows: number, cols: number) => void }> = ({ onSelect }) => {
  const [hover, setHover] = useState<{ r: number; c: number }>({ r: 0, c: 0 });

  return (
    <div className="table-grid-picker">
      <div className="table-grid-label">
        {hover.r > 0 && hover.c > 0
          ? `${hover.r} x ${hover.c} Table`
          : 'Insert Table'}
      </div>
      <div className="table-grid">
        {Array.from({ length: MAX_GRID_ROWS }, (_, r) => (
          <div key={r} className="table-grid-row">
            {Array.from({ length: MAX_GRID_COLS }, (_, c) => (
              <div
                key={c}
                className={`table-grid-cell${
                  r < hover.r && c < hover.c ? ' highlighted' : ''
                }`}
                onMouseEnter={() => setHover({ r: r + 1, c: c + 1 })}
                onClick={() => onSelect(r + 1, c + 1)}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
};

export const InsertRibbon: React.FC<InsertRibbonProps> = ({ dispatch }) => {
  const [showGrid, setShowGrid] = useState(false);

  const handleInsertTable = useCallback(
    (rows: number, cols: number) => {
      dispatch({ type: 'insert_table', payload: { rows, cols } });
      setShowGrid(false);
    },
    [dispatch],
  );

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
        <div style={{ position: 'relative' }}>
          <ToolbarButton
            label="Table"
            title="Insert Table"
            size="lg"
            onClick={() => setShowGrid(!showGrid)}
          />
          {showGrid && (
            <div className="table-grid-dropdown">
              <TableGridPicker onSelect={handleInsertTable} />
            </div>
          )}
        </div>
      </RibbonGroup>
      <RibbonGroup label="Illustrations">
        <ToolbarButton label="Image" title="Insert Image" size="lg" onClick={handleInsertImage} />
      </RibbonGroup>
      <RibbonGroup label="Pages">
        <ToolbarButton
          label="Page Break"
          title="Page Break (Ctrl+Enter)"
          size="lg"
          onClick={() => dispatch({ type: 'insert_page_break' })}
        />
      </RibbonGroup>
      <RibbonGroup label="Links">
        <ToolbarButton
          label="Link"
          title="Insert Hyperlink"
          size="lg"
          onClick={() => {
            const url = prompt('Enter URL:');
            if (url) {
              const text = prompt('Display text:', url);
              dispatch({
                type: 'insert_hyperlink',
                payload: { url, text: text || url },
              });
            }
          }}
        />
      </RibbonGroup>
    </div>
  );
};
