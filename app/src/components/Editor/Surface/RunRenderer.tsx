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

    // Special run types that render as non-text elements.
    if (properties.line_break) {
      return <br data-run-id={run.id} />;
    }

    if (properties.page_break) {
      return (
        <div
          data-run-id={run.id}
          className="page-break-indicator"
          contentEditable={false}
        >
          <span className="page-break-label">--- Page Break ---</span>
        </div>
      );
    }

    if (properties.tab) {
      return (
        <span data-run-id={run.id} className="tab-character">
          {'\t'}
        </span>
      );
    }

    // Footnote/endnote reference renders as superscript number
    if (properties.footnote_ref) {
      return (
        <a
          data-run-id={run.id}
          className="footnote-ref"
          href={`#footnote-${properties.footnote_ref}`}
          contentEditable={false}
        >
          <sup>{properties.footnote_ref}</sup>
        </a>
      );
    }

    if (properties.endnote_ref) {
      return (
        <a
          data-run-id={run.id}
          className="endnote-ref"
          href={`#endnote-${properties.endnote_ref}`}
          contentEditable={false}
        >
          <sup>{properties.endnote_ref}</sup>
        </a>
      );
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
    if (properties.double_strikethrough) decorations.push('line-through');
    if (decorations.length > 0) {
      style.textDecoration = decorations.join(' ');
      if (properties.double_strikethrough) {
        style.textDecorationStyle = 'double';
      }
    }

    if (properties.superscript) {
      style.verticalAlign = 'super';
      style.fontSize = '0.75em';
    }

    if (properties.subscript) {
      style.verticalAlign = 'sub';
      style.fontSize = '0.75em';
    }

    if (properties.small_caps) {
      style.fontVariant = 'small-caps';
    }

    if (properties.all_caps) {
      style.textTransform = 'uppercase';
    }

    if (properties.font_family) {
      style.fontFamily = properties.font_family;
    }

    if (properties.font_size != null) {
      // Don't override if super/subscript already set a relative size
      if (!properties.superscript && !properties.subscript) {
        style.fontSize = `${properties.font_size}pt`;
      }
    }

    if (properties.color) {
      style.color = properties.color.startsWith('#')
        ? properties.color
        : `#${properties.color}`;
    }

    if (properties.highlight) {
      style.backgroundColor = properties.highlight;
    } else if (properties.background_color) {
      style.backgroundColor = properties.background_color.startsWith('#')
        ? properties.background_color
        : `#${properties.background_color}`;
    }

    if (properties.spacing != null) {
      style.letterSpacing = `${properties.spacing / 20}pt`;
    }

    // Empty runs need a zero-width space so the browser can place a caret inside
    // the span and it remains selectable / measurable.
    const content = run.text.length > 0 ? run.text : '\u200B';

    // Hyperlink wrapping
    if (properties.hyperlink_url) {
      return (
        <a
          data-run-id={run.id}
          href={properties.hyperlink_url}
          title={properties.hyperlink_tooltip || properties.hyperlink_url}
          className="docx-hyperlink"
          style={{ ...style, color: style.color || '#2563eb', textDecoration: 'underline' }}
          onClick={(e) => {
            if (e.ctrlKey || e.metaKey) {
              window.open(properties.hyperlink_url!, '_blank', 'noopener');
            }
            e.preventDefault();
          }}
        >
          {content}
        </a>
      );
    }

    return (
      <span data-run-id={run.id} style={style}>
        {content}
      </span>
    );
  },
);

RunRenderer.displayName = 'RunRenderer';

export default RunRenderer;
