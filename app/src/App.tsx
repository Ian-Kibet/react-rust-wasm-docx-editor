import React, { useState, useCallback, useMemo } from 'react';
import { useWasm } from './hooks/useWasm';
import { useDocumentModel, createDefaultDocument } from './hooks/useDocumentModel';
import { useFileHandler } from './hooks/useFileHandler';
import { FileBar } from './components/FileBar/FileBar';
import { Ribbon } from './components/Editor/Ribbon/Ribbon';
import { Ruler } from './components/Editor/Ruler/Ruler';
import EditSurface from './components/Editor/Surface/EditSurface';
import { Sidebar } from './components/Editor/Sidebar/Sidebar';
import { DocxDocument, Paragraph } from './types/document';

export const App: React.FC = () => {
  const { ready, error } = useWasm();
  const { state, dispatch } = useDocumentModel();
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  const handleLoadDocument = useCallback(
    (doc: DocxDocument) => dispatch({ type: 'load_document', payload: doc }),
    [dispatch],
  );

  const { handleUpload, handleDownload } = useFileHandler(handleLoadDocument);

  const handleNew = useCallback(() => {
    dispatch({ type: 'load_document', payload: createDefaultDocument() });
  }, [dispatch]);

  const handleSave = useCallback(() => {
    handleDownload(state.document);
  }, [handleDownload, state.document]);

  const selectedParagraph = useMemo((): Paragraph | null => {
    if (!state.selection) return null;
    const block = state.document.body[state.selection.anchor.block_index];
    if (!block || !('paragraph' in block)) return null;
    return (block as { paragraph: Paragraph }).paragraph;
  }, [state.document, state.selection]);

  const indentLeft = selectedParagraph?.properties.indent_left ?? 0;
  const indentRight = selectedParagraph?.properties.indent_right ?? 0;
  const indentFirstLine = selectedParagraph?.properties.indent_first_line ?? 0;

  if (error) {
    return (
      <div className="error-screen">
        <h2>Failed to load</h2>
        <p>{error}</p>
      </div>
    );
  }

  if (!ready) {
    return (
      <div className="loading-screen">
        <div className="loading-spinner" />
        <div className="loading-text">Initializing WASM engine...</div>
      </div>
    );
  }

  return (
    <div className="app-container">
      <FileBar
        onUpload={handleUpload}
        onDownload={handleSave}
        onNew={handleNew}
        isDirty={state.is_dirty}
      />
      <Ribbon
        dispatch={dispatch}
        selection={state.selection}
        document={state.document}
      />
      <Ruler
        dispatch={dispatch}
        indentLeft={indentLeft}
        indentRight={indentRight}
        indentFirstLine={indentFirstLine}
      />
      <div className="editor-layout">
        <div className="editor-main">
          <EditSurface
            document={state.document}
            selection={state.selection}
            dispatch={dispatch}
            images={state.document.images}
          />
        </div>
        <Sidebar
          document={state.document}
          selection={state.selection}
          collapsed={sidebarCollapsed}
          onToggle={() => setSidebarCollapsed((c) => !c)}
        />
      </div>
    </div>
  );
};
