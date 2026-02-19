import React from 'react';
import { Run, ImageData } from '../../../types/document';
import ImageRenderer from './ImageRenderer';

interface RunRendererProps {
  run: Run;
  images: ImageData[];
}

const RunRenderer: React.FC<RunRendererProps> = React.memo(
  ({ run, images }) => {
    const { properties } = run;

    // If the run carries an inline image, render the image instead of text.
    if (properties.inline_image) {
      return <ImageRenderer imageId={properties.inline_image} images={images} />;
    }

    // Build inline styles from run properties.
    const style: React.CSSProperties = {};

    if (properties.bold) {
      style.fontWeight = 'bold';
    }

    if (properties.italic) {
      style.fontStyle = 'italic';
    }

    // Combine underline and strikethrough into a single textDecoration value.
    const decorations: string[] = [];
    if (properties.underline) decorations.push('underline');
    if (properties.strikethrough) decorations.push('line-through');
    if (decorations.length > 0) {
      style.textDecoration = decorations.join(' ');
    }

    if (properties.font_family) {
      style.fontFamily = properties.font_family;
    }

    if (properties.font_size != null) {
      style.fontSize = `${properties.font_size}pt`;
    }

    if (properties.color) {
      style.color = properties.color.startsWith('#')
        ? properties.color
        : `#${properties.color}`;
    }

    if (properties.highlight) {
      style.backgroundColor = properties.highlight;
    }

    // Empty runs need a zero-width space so the browser can place a caret inside
    // the span and it remains selectable / measurable.
    const content = run.text.length > 0 ? run.text : '\u200B';

    return (
      <span data-run-id={run.id} style={style}>
        {content}
      </span>
    );
  },
);

RunRenderer.displayName = 'RunRenderer';

export default RunRenderer;
