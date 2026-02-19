import React from 'react';
import { Paragraph, ImageData } from '../../../types/document';
import RunRenderer from './RunRenderer';

interface ListRendererProps {
  paragraphs: Paragraph[];
  listType: 'bullet' | 'numbered';
  images: ImageData[];
  startIndex: number;
}

const ListRenderer: React.FC<ListRendererProps> = React.memo(
  ({ paragraphs, listType, images, startIndex }) => {
    const items = paragraphs.map((paragraph, idx) => {
      const children =
        paragraph.runs.length > 0 ? (
          paragraph.runs.map((run) => (
            <RunRenderer key={run.id} run={run} images={images} />
          ))
        ) : (
          <br />
        );

      return (
        <li key={paragraph.id} data-paragraph-id={paragraph.id} data-block-index={startIndex + idx}>
          {children}
        </li>
      );
    });

    if (listType === 'numbered') {
      return <ol>{items}</ol>;
    }

    return <ul>{items}</ul>;
  },
);

ListRenderer.displayName = 'ListRenderer';

export default ListRenderer;
