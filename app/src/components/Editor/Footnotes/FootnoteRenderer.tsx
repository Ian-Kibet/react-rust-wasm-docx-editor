import React from 'react';
import { Footnote, Endnote, ImageData } from '../../../types/document';
import ParagraphRenderer from '../Surface/ParagraphRenderer';

interface FootnoteRendererProps {
  footnotes: Footnote[];
  endnotes: Endnote[];
  images: ImageData[];
}

export const FootnoteRenderer: React.FC<FootnoteRendererProps> = ({
  footnotes,
  endnotes,
  images,
}) => {
  if (footnotes.length === 0 && endnotes.length === 0) return null;

  return (
    <div className="footnotes-section">
      {footnotes.length > 0 && (
        <>
          <hr className="footnotes-separator" />
          <div className="footnotes-list">
            {footnotes.map((fn, idx) => (
              <div key={fn.id} className="footnote-item" id={`footnote-${fn.id}`}>
                <span className="footnote-number">{idx + 1}</span>
                <div className="footnote-content">
                  {fn.content.map((block) => {
                    if ('paragraph' in block) {
                      return (
                        <ParagraphRenderer
                          key={block.paragraph.id}
                          paragraph={block.paragraph}
                          images={images}
                        />
                      );
                    }
                    return null;
                  })}
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      {endnotes.length > 0 && (
        <>
          <hr className="endnotes-separator" />
          <div className="endnotes-header">Endnotes</div>
          <div className="endnotes-list">
            {endnotes.map((en, idx) => (
              <div key={en.id} className="endnote-item" id={`endnote-${en.id}`}>
                <span className="endnote-number">{idx + 1}</span>
                <div className="endnote-content">
                  {en.content.map((block) => {
                    if ('paragraph' in block) {
                      return (
                        <ParagraphRenderer
                          key={block.paragraph.id}
                          paragraph={block.paragraph}
                          images={images}
                        />
                      );
                    }
                    return null;
                  })}
                </div>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
};
