import React, { useCallback } from 'react';
import { ToolbarButton } from '../Toolbar/ToolbarButton';
import { RibbonGroup } from './RibbonGroup';
import { DocumentAction } from '../../../hooks/useDocumentModel';
import { EditorSelection } from '../../../types/editor';

interface LayoutRibbonProps {
  dispatch: React.Dispatch<DocumentAction>;
  selection: EditorSelection | null;
  activeTableId: string | null;
  activeRowIndex: number;
  activeColIndex: number;
}

export const LayoutRibbon: React.FC<LayoutRibbonProps> = ({
  dispatch,
  activeTableId,
  activeRowIndex,
  activeColIndex,
}) => {
  const tId = activeTableId ?? '';

  const insertRowAbove = useCallback(() => {
    if (!tId) return;
    dispatch({ type: 'insert_row', payload: { table_id: tId, after_row_index: activeRowIndex - 1 } });
  }, [dispatch, tId, activeRowIndex]);

  const insertRowBelow = useCallback(() => {
    if (!tId) return;
    dispatch({ type: 'insert_row', payload: { table_id: tId, after_row_index: activeRowIndex } });
  }, [dispatch, tId, activeRowIndex]);

  const insertColLeft = useCallback(() => {
    if (!tId) return;
    dispatch({ type: 'insert_column', payload: { table_id: tId, after_col_index: activeColIndex - 1 } });
  }, [dispatch, tId, activeColIndex]);

  const insertColRight = useCallback(() => {
    if (!tId) return;
    dispatch({ type: 'insert_column', payload: { table_id: tId, after_col_index: activeColIndex } });
  }, [dispatch, tId, activeColIndex]);

  const deleteRow = useCallback(() => {
    if (!tId) return;
    dispatch({ type: 'delete_row', payload: { table_id: tId, row_index: activeRowIndex } });
  }, [dispatch, tId, activeRowIndex]);

  const deleteCol = useCallback(() => {
    if (!tId) return;
    dispatch({ type: 'delete_column', payload: { table_id: tId, col_index: activeColIndex } });
  }, [dispatch, tId, activeColIndex]);

  return (
    <div className="ribbon-panel">
      <RibbonGroup label="Rows & Columns">
        <div className="ribbon-col">
          <div className="ribbon-row">
            <ToolbarButton label="\u2191 Row" title="Insert Row Above" onClick={insertRowAbove} disabled={!tId} />
            <ToolbarButton label="\u2193 Row" title="Insert Row Below" onClick={insertRowBelow} disabled={!tId} />
          </div>
          <div className="ribbon-row">
            <ToolbarButton label="\u2190 Col" title="Insert Column Left" onClick={insertColLeft} disabled={!tId} />
            <ToolbarButton label="\u2192 Col" title="Insert Column Right" onClick={insertColRight} disabled={!tId} />
          </div>
        </div>
      </RibbonGroup>

      <RibbonGroup label="Delete">
        <div className="ribbon-col">
          <div className="ribbon-row">
            <ToolbarButton label="Del Row" title="Delete Row" onClick={deleteRow} disabled={!tId} />
          </div>
          <div className="ribbon-row">
            <ToolbarButton label="Del Col" title="Delete Column" onClick={deleteCol} disabled={!tId} />
          </div>
        </div>
      </RibbonGroup>
    </div>
  );
};
