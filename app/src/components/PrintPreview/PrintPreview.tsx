import React from 'react';
import { DocxDocument, ImageData, Paragraph, BlockElement } from '../../types/document';
import ParagraphRenderer from '../Editor/Surface/ParagraphRenderer';
import TableRenderer from '../Editor/Surface/TableRenderer';

interface PrintPreviewProps {
  document: DocxDocument;
  images: ImageData[];
  onClose: () => void;
}

const noop = () => {};

function renderBlock(block: BlockElement, index: number, images: ImageData[]) {
  if ('paragraph' in block) {
    return (
      <div key={block.paragraph.id} data-block-index={index}>
        <ParagraphRenderer paragraph={block.paragraph} images={images} />
      </div>
    );
  }
  if ('table' in block) {
    return (
      <div key={block.table.id} data-block-index={index}>
        <TableRenderer table={block.table} images={images} dispatch={noop} />
      </div>
    );
  }
  return null;
}

export const PrintPreview: React.FC<PrintPreviewProps> = ({
  document: doc,
  images,
  onClose,
}) => {
  const sp = doc.section_properties;
  const pageWidth = sp?.page_width ? `${sp.page_width / 20}pt` : '8.5in';
  const pageMinHeight = sp?.page_height ? `${sp.page_height / 20}pt` : '11in';
  const marginTop = sp?.margin_top ? `${sp.margin_top / 20}pt` : '1in';
  const marginRight = sp?.margin_right ? `${sp.margin_right / 20}pt` : '1in';
  const marginBottom = sp?.margin_bottom ? `${sp.margin_bottom / 20}pt` : '1in';
  const marginLeft = sp?.margin_left ? `${sp.margin_left / 20}pt` : '1in';

  const handlePrint = () => {
    window.print();
  };

  return (
    <div className="print-preview-overlay">
      <div className="print-preview-toolbar">
        <button className="print-preview-btn" onClick={handlePrint}>
          Print
        </button>
        <button className="print-preview-btn" onClick={onClose}>
          Close
        </button>
      </div>
      <div className="print-preview-scroll">
        <div
          className="print-preview-page"
          style={{
            width: pageWidth,
            minHeight: pageMinHeight,
            paddingTop: marginTop,
            paddingRight: marginRight,
            paddingBottom: marginBottom,
            paddingLeft: marginLeft,
          }}
        >
          {doc.body.map((block, index) => renderBlock(block, index, images))}
        </div>
      </div>
    </div>
  );
};
