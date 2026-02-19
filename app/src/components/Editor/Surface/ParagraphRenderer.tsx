import React from 'react';
import { Paragraph, ImageData } from '../../../types/document';
import RunRenderer from './RunRenderer';

interface ParagraphRendererProps {
  paragraph: Paragraph;
  images: ImageData[];
}

const ParagraphRenderer: React.FC<ParagraphRendererProps> = React.memo(
  ({ paragraph, images }) => {
    const { properties } = paragraph;

    // Build inline styles from paragraph properties.
    const style: React.CSSProperties = {};

    if (properties.alignment) {
      style.textAlign = properties.alignment;
    }

    if (properties.spacing_before != null) {
      style.marginTop = `${properties.spacing_before}pt`;
    }

    if (properties.spacing_after != null) {
      style.marginBottom = `${properties.spacing_after}pt`;
    }

    if (properties.indent_left != null) {
      style.paddingLeft = `${properties.indent_left}pt`;
    }

    if (properties.indent_right != null) {
      style.paddingRight = `${properties.indent_right}pt`;
    }

    if (properties.indent_first_line != null) {
      style.textIndent = `${properties.indent_first_line}pt`;
    }

    // Render runs, or a <br> for empty paragraphs so they still have height.
    const children =
      paragraph.runs.length > 0 ? (
        paragraph.runs.map((run) => (
          <RunRenderer key={run.id} run={run} images={images} />
        ))
      ) : (
        <br />
      );

    // Common props shared by all heading / paragraph tags.
    const commonProps = {
      'data-paragraph-id': paragraph.id,
      style,
    };

    // Select the appropriate HTML element for the heading level.
    const level = properties.heading_level;

    if (level === 1) return <h1 {...commonProps}>{children}</h1>;
    if (level === 2) return <h2 {...commonProps}>{children}</h2>;
    if (level === 3) return <h3 {...commonProps}>{children}</h3>;
    if (level === 4) return <h4 {...commonProps}>{children}</h4>;
    if (level === 5) return <h5 {...commonProps}>{children}</h5>;
    if (level === 6) return <h6 {...commonProps}>{children}</h6>;

    return <p {...commonProps}>{children}</p>;
  },
);

ParagraphRenderer.displayName = 'ParagraphRenderer';

export default ParagraphRenderer;
