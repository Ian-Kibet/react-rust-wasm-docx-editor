import React, { useState, useCallback, useEffect, useRef } from 'react';
import { DocxDocument } from '../../../types/document';
import { EditorSelection } from '../../../types/editor';

interface SearchResult {
  block_index: number;
  run_index: number;
  offset: number;
  length: number;
  context: string;
}

interface FindReplacePanelProps {
  document: DocxDocument;
  dispatch: React.Dispatch<any>;
  onClose: () => void;
}

export const FindReplacePanel: React.FC<FindReplacePanelProps> = ({
  document: doc,
  dispatch,
  onClose,
}) => {
  const [query, setQuery] = useState('');
  const [replacement, setReplacement] = useState('');
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [showReplace, setShowReplace] = useState(false);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [currentIndex, setCurrentIndex] = useState(-1);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handleKey);
    return () => document.removeEventListener('keydown', handleKey);
  }, [onClose]);

  // Search using simple JS implementation (WASM search_text could be used if available)
  const doSearch = useCallback(() => {
    if (!query) {
      setResults([]);
      setCurrentIndex(-1);
      return;
    }

    const found: SearchResult[] = [];
    const searchQuery = caseSensitive ? query : query.toLowerCase();

    function searchBlocks(blocks: typeof doc.body, baseBlockIndex: number) {
      for (let bi = 0; bi < blocks.length; bi++) {
        const block = blocks[bi];
        if ('paragraph' in block) {
          const para = block.paragraph;
          // Build full text and track run boundaries
          let fullText = '';
          const runBounds: { runIndex: number; start: number; end: number }[] = [];
          for (let ri = 0; ri < para.runs.length; ri++) {
            const start = fullText.length;
            fullText += para.runs[ri].text;
            runBounds.push({ runIndex: ri, start, end: fullText.length });
          }

          const compareText = caseSensitive ? fullText : fullText.toLowerCase();
          let pos = 0;
          while (pos < compareText.length) {
            const idx = compareText.indexOf(searchQuery, pos);
            if (idx === -1) break;
            // Find which run contains this offset
            const bound = runBounds.find(b => idx >= b.start && idx < b.end);
            if (bound) {
              const contextStart = Math.max(0, idx - 20);
              const contextEnd = Math.min(fullText.length, idx + query.length + 20);
              found.push({
                block_index: baseBlockIndex + bi,
                run_index: bound.runIndex,
                offset: idx - bound.start,
                length: query.length,
                context: fullText.slice(contextStart, contextEnd),
              });
            }
            pos = idx + 1;
          }
        } else if ('table' in block) {
          for (const row of block.table.rows) {
            for (const cell of row.cells) {
              searchBlocks(cell.content, baseBlockIndex + bi);
            }
          }
        }
      }
    }

    searchBlocks(doc.body, 0);
    setResults(found);
    setCurrentIndex(found.length > 0 ? 0 : -1);
  }, [query, caseSensitive, doc]);

  useEffect(() => {
    const timer = setTimeout(doSearch, 200);
    return () => clearTimeout(timer);
  }, [doSearch]);

  const navigateToResult = useCallback(
    (idx: number) => {
      if (idx < 0 || idx >= results.length) return;
      const r = results[idx];
      // Find paragraph ID
      const block = doc.body[r.block_index];
      if (!block || !('paragraph' in block)) return;
      const para = block.paragraph;

      const sel: EditorSelection = {
        anchor: {
          block_index: r.block_index,
          paragraph_id: para.id,
          run_index: r.run_index,
          offset: r.offset,
        },
        focus: {
          block_index: r.block_index,
          paragraph_id: para.id,
          run_index: r.run_index,
          offset: r.offset + r.length,
        },
        is_collapsed: false,
      };
      dispatch({ type: 'set_selection', payload: sel });
    },
    [results, doc, dispatch],
  );

  useEffect(() => {
    if (currentIndex >= 0) navigateToResult(currentIndex);
  }, [currentIndex, navigateToResult]);

  const handleNext = () => {
    if (results.length === 0) return;
    setCurrentIndex((currentIndex + 1) % results.length);
  };

  const handlePrev = () => {
    if (results.length === 0) return;
    setCurrentIndex((currentIndex - 1 + results.length) % results.length);
  };

  const handleReplace = () => {
    if (currentIndex < 0 || currentIndex >= results.length) return;
    // Delete the found text and insert replacement
    dispatch({ type: 'delete_range' });
    if (replacement) {
      dispatch({ type: 'insert_text', payload: { text: replacement } });
    }
    // Re-search after replacement
    setTimeout(doSearch, 50);
  };

  const handleReplaceAll = () => {
    if (results.length === 0) return;
    // Replace in reverse order to preserve indices
    for (let i = results.length - 1; i >= 0; i--) {
      navigateToResult(i);
      dispatch({ type: 'delete_range' });
      if (replacement) {
        dispatch({ type: 'insert_text', payload: { text: replacement } });
      }
    }
    setTimeout(doSearch, 100);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (e.shiftKey) handlePrev();
      else handleNext();
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  };

  return (
    <div className="find-replace-panel" onKeyDown={handleKeyDown}>
      <div className="find-replace-header">
        <button
          className="find-replace-toggle"
          onClick={() => setShowReplace(!showReplace)}
          title={showReplace ? 'Hide Replace' : 'Show Replace'}
        >
          {showReplace ? '\u25BC' : '\u25B6'}
        </button>
        <span className="find-replace-title">
          {showReplace ? 'Find & Replace' : 'Find'}
        </span>
        <button className="find-replace-close" onClick={onClose} title="Close (Esc)">
          \u2715
        </button>
      </div>

      <div className="find-replace-row">
        <input
          ref={inputRef}
          type="text"
          className="find-replace-input"
          placeholder="Find..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <button
          className={`find-replace-option${caseSensitive ? ' active' : ''}`}
          onClick={() => setCaseSensitive(!caseSensitive)}
          title="Match Case"
        >
          Aa
        </button>
        <button className="find-replace-btn" onClick={handlePrev} title="Previous (Shift+Enter)">
          \u25B2
        </button>
        <button className="find-replace-btn" onClick={handleNext} title="Next (Enter)">
          \u25BC
        </button>
        <span className="find-replace-count">
          {results.length > 0
            ? `${currentIndex + 1} of ${results.length}`
            : query
              ? 'No results'
              : ''}
        </span>
      </div>

      {showReplace && (
        <div className="find-replace-row">
          <input
            type="text"
            className="find-replace-input"
            placeholder="Replace with..."
            value={replacement}
            onChange={(e) => setReplacement(e.target.value)}
          />
          <button
            className="find-replace-btn"
            onClick={handleReplace}
            disabled={results.length === 0}
            title="Replace"
          >
            Replace
          </button>
          <button
            className="find-replace-btn"
            onClick={handleReplaceAll}
            disabled={results.length === 0}
            title="Replace All"
          >
            All
          </button>
        </div>
      )}
    </div>
  );
};
