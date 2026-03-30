import React, { useCallback, useState } from 'react';
import { ToolbarButton } from '../Toolbar/ToolbarButton';
import { RibbonGroup } from './RibbonGroup';
import { DocumentAction } from '../../../hooks/useDocumentModel';

interface ReviewRibbonProps {
  dispatch: React.Dispatch<DocumentAction>;
}

export const ReviewRibbon: React.FC<ReviewRibbonProps> = ({ dispatch }) => {
  const [author, setAuthor] = useState('User');

  const handleNewComment = useCallback(() => {
    const content = prompt('Enter comment text:');
    if (content) {
      dispatch({ type: 'add_comment', payload: { author, content } });
    }
  }, [dispatch, author]);

  const handleInsertFootnote = useCallback(() => {
    const content = prompt('Enter footnote text:');
    if (content) {
      dispatch({ type: 'insert_footnote', payload: { content } });
    }
  }, [dispatch]);

  const handleInsertEndnote = useCallback(() => {
    const content = prompt('Enter endnote text:');
    if (content) {
      dispatch({ type: 'insert_endnote', payload: { content } });
    }
  }, [dispatch]);

  return (
    <div className="ribbon-panel">
      <RibbonGroup label="Comments">
        <div className="ribbon-col">
          <div className="ribbon-row">
            <input
              type="text"
              className="toolbar-font-size"
              style={{ width: 80 }}
              title="Author name"
              placeholder="Author"
              value={author}
              onChange={(e) => setAuthor(e.target.value)}
            />
          </div>
          <div className="ribbon-row">
            <ToolbarButton label="+" title="New Comment" onClick={handleNewComment} />
          </div>
        </div>
      </RibbonGroup>

      <RibbonGroup label="Footnotes">
        <div className="ribbon-col">
          <div className="ribbon-row">
            <ToolbarButton label="FN" title="Insert Footnote" onClick={handleInsertFootnote} />
          </div>
          <div className="ribbon-row">
            <ToolbarButton label="EN" title="Insert Endnote" onClick={handleInsertEndnote} />
          </div>
        </div>
      </RibbonGroup>
    </div>
  );
};
