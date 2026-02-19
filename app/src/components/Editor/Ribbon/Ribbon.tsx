import React, { useState } from 'react';
import { TabStrip } from './TabStrip';
import { HomeRibbon } from './HomeRibbon';
import { InsertRibbon } from './InsertRibbon';
import { DocxDocument } from '../../../types/document';
import { EditorSelection } from '../../../types/editor';
import { DocumentAction } from '../../../hooks/useDocumentModel';

interface RibbonProps {
  dispatch: React.Dispatch<DocumentAction>;
  selection: EditorSelection | null;
  document: DocxDocument;
}

export const Ribbon: React.FC<RibbonProps> = ({ dispatch, selection, document: doc }) => {
  const [activeTab, setActiveTab] = useState('Home');

  const renderPanel = () => {
    switch (activeTab) {
      case 'Home':
        return <HomeRibbon dispatch={dispatch} selection={selection} document={doc} />;
      case 'Insert':
        return <InsertRibbon dispatch={dispatch} />;
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
