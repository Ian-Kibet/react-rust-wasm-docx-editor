import React, { useState } from 'react';
import { TabStrip } from './TabStrip';
import { HomeRibbon } from './HomeRibbon';
import { InsertRibbon } from './InsertRibbon';
import { ReviewRibbon } from './ReviewRibbon';
import { PageLayoutRibbon } from './PageLayoutRibbon';
import { LayoutRibbon } from './LayoutRibbon';
import { DocxDocument } from '../../../types/document';
import { EditorSelection } from '../../../types/editor';
import { DocumentAction } from '../../../hooks/useDocumentModel';

interface RibbonProps {
  dispatch: React.Dispatch<DocumentAction>;
  selection: EditorSelection | null;
  document: DocxDocument;
  onCut?: () => void;
  onCopy?: () => void;
  onPaste?: () => void;
  activeTableId?: string | null;
  activeRowIndex?: number;
  activeColIndex?: number;
}

export const Ribbon: React.FC<RibbonProps> = ({
  dispatch,
  selection,
  document: doc,
  onCut,
  onCopy,
  onPaste,
  activeTableId,
  activeRowIndex,
  activeColIndex,
}) => {
  const [activeTab, setActiveTab] = useState('Home');

  const renderPanel = () => {
    switch (activeTab) {
      case 'Home':
        return (
          <HomeRibbon
            dispatch={dispatch}
            selection={selection}
            document={doc}
            onCut={onCut}
            onCopy={onCopy}
            onPaste={onPaste}
          />
        );
      case 'Insert':
        return <InsertRibbon dispatch={dispatch} />;
      case 'Review':
        return <ReviewRibbon dispatch={dispatch} />;
      case 'Layout':
        return (
          <LayoutRibbon
            dispatch={dispatch}
            selection={selection}
            activeTableId={activeTableId ?? null}
            activeRowIndex={activeRowIndex ?? 0}
            activeColIndex={activeColIndex ?? 0}
          />
        );
      case 'Design':
        return (
          <PageLayoutRibbon
            dispatch={dispatch}
            sectionProperties={doc.section_properties}
          />
        );
      default:
        return (
          <div className="ribbon-panel ribbon-stub">
            {activeTab} tools coming soon
          </div>
        );
    }
  };

  return (
    <div className="ribbon-container">
      <TabStrip activeTab={activeTab} onTabChange={setActiveTab} />
      {renderPanel()}
    </div>
  );
};
