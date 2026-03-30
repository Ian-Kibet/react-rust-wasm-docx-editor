import React, { useMemo, useCallback } from 'react';
import { ToolbarButton } from '../Toolbar/ToolbarButton';
import { ToolbarDropdown } from '../Toolbar/ToolbarDropdown';
import { RibbonGroup } from './RibbonGroup';
import { DocxDocument, Paragraph, Run } from '../../../types/document';
import { EditorSelection } from '../../../types/editor';
import { DocumentAction } from '../../../hooks/useDocumentModel';

interface HomeRibbonProps {
  dispatch: React.Dispatch<DocumentAction>;
  selection: EditorSelection | null;
  document: DocxDocument;
  onCut?: () => void;
  onCopy?: () => void;
  onPaste?: () => void;
}

function getSelectedParagraphAndRun(
  doc: DocxDocument,
  sel: EditorSelection | null,
): { paragraph: Paragraph | null; run: Run | null } {
  if (!sel) return { paragraph: null, run: null };
  const block = doc.body[sel.anchor.block_index];
  if (!block || !('paragraph' in block)) return { paragraph: null, run: null };
  const p = (block as { paragraph: Paragraph }).paragraph;
  const r = p.runs[sel.anchor.run_index] ?? null;
  return { paragraph: p, run: r };
}

const FONT_OPTIONS = [
  { value: '', label: 'Font' },
  { value: 'Arial', label: 'Arial' },
  { value: 'Times New Roman', label: 'Times New Roman' },
  { value: 'Courier New', label: 'Courier New' },
  { value: 'Georgia', label: 'Georgia' },
  { value: 'Verdana', label: 'Verdana' },
  { value: 'Helvetica', label: 'Helvetica' },
  { value: 'Calibri', label: 'Calibri' },
  { value: 'Cambria', label: 'Cambria' },
  { value: 'Tahoma', label: 'Tahoma' },
];

const HEADING_OPTIONS = [
  { value: '0', label: 'Normal' },
  { value: '1', label: 'Heading 1' },
  { value: '2', label: 'Heading 2' },
  { value: '3', label: 'Heading 3' },
  { value: '4', label: 'Heading 4' },
  { value: '5', label: 'Heading 5' },
  { value: '6', label: 'Heading 6' },
];

const LINE_SPACING_OPTIONS = [
  { value: '240', label: '1.0' },
  { value: '276', label: '1.15' },
  { value: '360', label: '1.5' },
  { value: '480', label: '2.0' },
  { value: '600', label: '2.5' },
  { value: '720', label: '3.0' },
];

export const HomeRibbon: React.FC<HomeRibbonProps> = ({
  dispatch,
  selection,
  document: doc,
  onCut,
  onCopy,
  onPaste,
}) => {
  const { paragraph, run } = useMemo(
    () => getSelectedParagraphAndRun(doc, selection),
    [doc, selection],
  );

  const rp = run?.properties;
  const pp = paragraph?.properties;

  const handleFontFamily = useCallback(
    (v: string) => dispatch({ type: 'set_font_family', payload: { family: v } }),
    [dispatch],
  );

  const handleFontSize = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const size = parseFloat(e.target.value);
      if (!isNaN(size) && size > 0) dispatch({ type: 'set_font_size', payload: { size } });
    },
    [dispatch],
  );

  const handleColor = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) =>
      dispatch({ type: 'set_color', payload: { color: e.target.value } }),
    [dispatch],
  );

  const handleHighlight = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) =>
      dispatch({ type: 'set_highlight', payload: { color: e.target.value } }),
    [dispatch],
  );

  const handleHeading = useCallback(
    (v: string) => {
      const level = parseInt(v, 10);
      dispatch({ type: 'set_heading_level', payload: { level: level === 0 ? null : level } });
    },
    [dispatch],
  );

  const handleLineSpacing = useCallback(
    (v: string) => {
      const spacing = parseInt(v, 10);
      dispatch({ type: 'set_line_spacing', payload: { spacing, rule: 'auto' } });
    },
    [dispatch],
  );

  return (
    <div className="ribbon-panel">
      {/* Clipboard */}
      <RibbonGroup label="Clipboard">
        <ToolbarButton label="Paste" title="Paste (Ctrl+V)" size="lg" onClick={onPaste ?? (() => {})} />
        <div className="ribbon-col">
          <div className="ribbon-row">
            <ToolbarButton label="Cut" title="Cut (Ctrl+X)" onClick={onCut ?? (() => {})} />
            <ToolbarButton label="Copy" title="Copy (Ctrl+C)" onClick={onCopy ?? (() => {})} />
          </div>
          <div className="ribbon-row">
            <ToolbarButton
              label="Undo"
              title="Undo (Ctrl+Z)"
              onClick={() => dispatch({ type: 'undo' })}
            />
            <ToolbarButton
              label="Redo"
              title="Redo (Ctrl+Y)"
              onClick={() => dispatch({ type: 'redo' })}
            />
          </div>
        </div>
      </RibbonGroup>

      {/* Font */}
      <RibbonGroup label="Font">
        <div className="ribbon-col">
          <div className="ribbon-row">
            <ToolbarDropdown
              label="Font"
              value={rp?.font_family ?? ''}
              options={FONT_OPTIONS}
              onChange={handleFontFamily}
            />
            <input
              type="number"
              className="toolbar-font-size"
              title="Font size (pt)"
              value={rp?.font_size ?? 12}
              min={6}
              max={96}
              step={0.5}
              onChange={handleFontSize}
            />
            <ToolbarButton
              label="A+"
              title="Grow Font (Ctrl+])"
              onClick={() => dispatch({ type: 'grow_font' })}
            />
            <ToolbarButton
              label="A-"
              title="Shrink Font (Ctrl+[)"
              onClick={() => dispatch({ type: 'shrink_font' })}
            />
          </div>
          <div className="ribbon-row">
            <ToolbarButton
              label="B"
              title="Bold (Ctrl+B)"
              active={rp?.bold === true}
              onClick={() => dispatch({ type: 'toggle_bold' })}
            />
            <ToolbarButton
              label="I"
              title="Italic (Ctrl+I)"
              active={rp?.italic === true}
              onClick={() => dispatch({ type: 'toggle_italic' })}
            />
            <ToolbarButton
              label="U"
              title="Underline (Ctrl+U)"
              active={rp?.underline === true}
              onClick={() => dispatch({ type: 'toggle_underline' })}
            />
            <ToolbarButton
              label="S"
              title="Strikethrough (Ctrl+Shift+X)"
              active={rp?.strikethrough === true}
              onClick={() => dispatch({ type: 'toggle_strikethrough' })}
            />
            <ToolbarButton
              label="x2"
              title="Subscript (Ctrl+=)"
              active={rp?.subscript === true}
              onClick={() => dispatch({ type: 'toggle_subscript' })}
            />
            <ToolbarButton
              label="x2"
              title="Superscript (Ctrl+Shift+=)"
              active={rp?.superscript === true}
              onClick={() => dispatch({ type: 'toggle_superscript' })}
            />
            <input
              type="color"
              className="toolbar-color"
              title="Text color"
              value={rp?.color ? `#${rp.color.replace('#', '')}` : '#000000'}
              onChange={handleColor}
            />
            <input
              type="color"
              className="toolbar-color"
              title="Highlight color"
              value={rp?.background_color ? `#${rp.background_color.replace('#', '')}` : '#FFFF00'}
              onChange={handleHighlight}
            />
          </div>
        </div>
      </RibbonGroup>

      {/* Paragraph */}
      <RibbonGroup label="Paragraph">
        <div className="ribbon-col">
          <div className="ribbon-row">
            <ToolbarButton
              label="&bull;"
              title="Bullet List"
              active={pp?.numbering?.num_id === '1'}
              onClick={() => dispatch({ type: 'toggle_bullet_list' })}
            />
            <ToolbarButton
              label="1."
              title="Numbered List"
              active={pp?.numbering?.num_id === '2'}
              onClick={() => dispatch({ type: 'toggle_numbered_list' })}
            />
            <ToolbarButton
              label="&lt;"
              title="Decrease Indent (Shift+Tab)"
              onClick={() => dispatch({ type: 'decrease_indent' })}
            />
            <ToolbarButton
              label="&gt;"
              title="Increase Indent (Tab)"
              onClick={() => dispatch({ type: 'increase_indent' })}
            />
            <ToolbarDropdown
              label="Line Spacing"
              value={String(pp?.line_spacing ?? 240)}
              options={LINE_SPACING_OPTIONS}
              onChange={handleLineSpacing}
            />
          </div>
          <div className="ribbon-row">
            <ToolbarButton
              label="&#8676;"
              title="Align Left (Ctrl+L)"
              active={pp?.alignment === 'left' || !pp?.alignment}
              onClick={() => dispatch({ type: 'set_alignment', payload: { alignment: 'left' } })}
            />
            <ToolbarButton
              label="&#8703;"
              title="Align Center (Ctrl+E)"
              active={pp?.alignment === 'center'}
              onClick={() => dispatch({ type: 'set_alignment', payload: { alignment: 'center' } })}
            />
            <ToolbarButton
              label="&#8677;"
              title="Align Right (Ctrl+R)"
              active={pp?.alignment === 'right'}
              onClick={() => dispatch({ type: 'set_alignment', payload: { alignment: 'right' } })}
            />
            <ToolbarButton
              label="&#8700;"
              title="Justify (Ctrl+J)"
              active={pp?.alignment === 'justify'}
              onClick={() => dispatch({ type: 'set_alignment', payload: { alignment: 'justify' } })}
            />
          </div>
        </div>
      </RibbonGroup>

      {/* Styles */}
      <RibbonGroup label="Styles">
        <div className="ribbon-col">
          <div className="ribbon-row">
            <ToolbarDropdown
              label="Heading"
              value={String(pp?.heading_level ?? 0)}
              options={HEADING_OPTIONS}
              onChange={handleHeading}
            />
          </div>
          <div className="ribbon-row" />
        </div>
      </RibbonGroup>
    </div>
  );
};
