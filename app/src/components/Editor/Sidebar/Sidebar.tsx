import React, { useState } from 'react';
import { OutlinePanel } from './OutlinePanel';
import { PropertiesPanel } from './PropertiesPanel';
import { StylesPanel } from './StylesPanel';
import { CommentsPanel } from '../Comments/CommentsPanel';
import { DocxDocument } from '../../../types/document';
import { EditorSelection } from '../../../types/editor';

interface SidebarProps {
  document: DocxDocument;
  selection: EditorSelection | null;
  collapsed: boolean;
  onToggle: () => void;
  dispatch: React.Dispatch<any>;
}

type Tab = 'outline' | 'properties' | 'styles' | 'comments';

export const Sidebar: React.FC<SidebarProps> = ({
  document: doc,
  selection,
  collapsed,
  onToggle,
  dispatch,
}) => {
  const [tab, setTab] = useState<Tab>('outline');

  return (
    <div className={`sidebar${collapsed ? ' collapsed' : ''}`}>
      <button className="sidebar-toggle" onClick={onToggle} title={collapsed ? 'Expand' : 'Collapse'}>
        {collapsed ? '\u25C0' : '\u25B6'}
      </button>
      {!collapsed && (
        <>
          <div className="sidebar-tabs">
            <button
              className={tab === 'outline' ? 'active' : ''}
              onClick={() => setTab('outline')}
            >
              Outline
            </button>
            <button
              className={tab === 'properties' ? 'active' : ''}
              onClick={() => setTab('properties')}
            >
              Properties
            </button>
            <button
              className={tab === 'styles' ? 'active' : ''}
              onClick={() => setTab('styles')}
            >
              Styles
            </button>
            <button
              className={tab === 'comments' ? 'active' : ''}
              onClick={() => setTab('comments')}
            >
              Comments
            </button>
          </div>
          <div className="sidebar-content">
            {tab === 'outline' && <OutlinePanel document={doc} />}
            {tab === 'properties' && (
              <PropertiesPanel document={doc} selection={selection} />
            )}
            {tab === 'styles' && <StylesPanel document={doc} />}
            {tab === 'comments' && (
              <CommentsPanel document={doc} dispatch={dispatch} />
            )}
          </div>
        </>
      )}
    </div>
  );
};
