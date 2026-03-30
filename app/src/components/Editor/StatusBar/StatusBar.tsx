import React, { useMemo } from 'react';
import { DocxDocument } from '../../../types/document';
import { EditorSelection } from '../../../types/editor';

interface StatusBarProps {
  document: DocxDocument;
  selection: EditorSelection | null;
  zoom: number;
  onZoomChange: (zoom: number) => void;
}

function countDocStats(doc: DocxDocument) {
  let words = 0;
  let chars = 0;
  let paragraphs = 0;

  function processBlocks(blocks: typeof doc.body) {
    for (const block of blocks) {
      if ('paragraph' in block) {
        paragraphs++;
        for (const run of block.paragraph.runs) {
          chars += run.text.length;
          words += run.text.split(/\s+/).filter(Boolean).length;
        }
      } else if ('table' in block) {
        for (const row of block.table.rows) {
          for (const cell of row.cells) {
            processBlocks(cell.content);
          }
        }
      }
    }
  }

  processBlocks(doc.body);
  return { words, chars, paragraphs };
}

const ZOOM_LEVELS = [50, 75, 100, 125, 150, 175, 200];

export const StatusBar: React.FC<StatusBarProps> = ({
  document: doc,
  selection,
  zoom,
  onZoomChange,
}) => {
  const stats = useMemo(() => countDocStats(doc), [doc]);
  const pageEstimate = Math.max(1, Math.ceil(stats.paragraphs / 25));

  const cursorInfo = useMemo(() => {
    if (!selection) return '';
    const blockNum = selection.anchor.block_index + 1;
    return `Block ${blockNum}`;
  }, [selection]);

  return (
    <div className="status-bar">
      <div className="status-bar-left">
        <span className="status-bar-item" title="Page estimate">
          Page {pageEstimate > 1 ? `1 of ${pageEstimate}` : '1 of 1'}
        </span>
        <span className="status-bar-separator">|</span>
        <span className="status-bar-item" title="Word count">
          {stats.words.toLocaleString()} words
        </span>
        <span className="status-bar-separator">|</span>
        <span className="status-bar-item" title="Character count">
          {stats.chars.toLocaleString()} characters
        </span>
        {cursorInfo && (
          <>
            <span className="status-bar-separator">|</span>
            <span className="status-bar-item">{cursorInfo}</span>
          </>
        )}
      </div>
      <div className="status-bar-right">
        <button
          className="status-bar-zoom-btn"
          onClick={() => onZoomChange(Math.max(50, zoom - 10))}
          title="Zoom out"
        >
          -
        </button>
        <input
          type="range"
          className="status-bar-zoom-slider"
          min={50}
          max={200}
          step={10}
          value={zoom}
          onChange={(e) => onZoomChange(parseInt(e.target.value, 10))}
          title={`Zoom: ${zoom}%`}
        />
        <button
          className="status-bar-zoom-btn"
          onClick={() => onZoomChange(Math.min(200, zoom + 10))}
          title="Zoom in"
        >
          +
        </button>
        <select
          className="status-bar-zoom-select"
          value={zoom}
          onChange={(e) => onZoomChange(parseInt(e.target.value, 10))}
        >
          {ZOOM_LEVELS.map((z) => (
            <option key={z} value={z}>
              {z}%
            </option>
          ))}
        </select>
      </div>
    </div>
  );
};
