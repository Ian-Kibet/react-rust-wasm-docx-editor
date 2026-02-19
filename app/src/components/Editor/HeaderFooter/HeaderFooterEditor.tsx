import React from 'react';
import { Header, Footer, ImageData, BlockElement } from '../../../types/document';
import { EditingZone } from '../../../types/editor';
import ParagraphRenderer from '../Surface/ParagraphRenderer';

interface HeaderFooterEditorProps {
  headers: Header[];
  footers: Footer[];
  images: ImageData[];
  editingZone: EditingZone;
  onClose: () => void;
}

function renderBlocks(blocks: BlockElement[], images: ImageData[]) {
  return blocks.map((block) => {
    if ('paragraph' in block) {
      return (
        <div key={block.paragraph.id} data-paragraph-id={block.paragraph.id}>
          <ParagraphRenderer paragraph={block.paragraph} images={images} />
        </div>
      );
    }
    return null;
  });
}

export const HeaderFooterEditor: React.FC<HeaderFooterEditorProps> = ({
  headers,
  footers,
  images,
  editingZone,
  onClose,
}) => {
  const header = headers[0];
  const footer = footers[0];

  return (
    <>
      {/* Header zone */}
      <div
        className={`header-footer-zone header-zone${editingZone === 'header' ? ' editing' : ''}`}
      >
        {editingZone === 'header' && (
          <div className="header-footer-label">
            <span>Header</span>
            <button className="header-footer-close" onClick={onClose}>
              Close Header
            </button>
          </div>
        )}
        <div
          className="header-footer-content"
          contentEditable={editingZone === 'header'}
          suppressContentEditableWarning
        >
          {header && header.content.length > 0 ? (
            renderBlocks(header.content, images)
          ) : editingZone === 'header' ? (
            <p className="header-footer-placeholder">Type header text here...</p>
          ) : null}
        </div>
      </div>

      {/* Footer zone - rendered at bottom of page */}
      <div
        className={`header-footer-zone footer-zone${editingZone === 'footer' ? ' editing' : ''}`}
      >
        {editingZone === 'footer' && (
          <div className="header-footer-label">
            <span>Footer</span>
            <button className="header-footer-close" onClick={onClose}>
              Close Footer
            </button>
          </div>
        )}
        <div
          className="header-footer-content"
          contentEditable={editingZone === 'footer'}
          suppressContentEditableWarning
        >
          {footer && footer.content.length > 0 ? (
            renderBlocks(footer.content, images)
          ) : editingZone === 'footer' ? (
            <p className="header-footer-placeholder">Type footer text here...</p>
          ) : null}
        </div>
      </div>
    </>
  );
};
