import React, { useMemo, useCallback } from 'react';
import { ToolbarButton } from './ToolbarButton';
import { ToolbarDropdown } from './ToolbarDropdown';
import { DocxDocument, Paragraph, Run, BlockElement } from '../../../types/document';
import { EditorSelection } from '../../../types/editor';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type DocumentAction = any;

interface ToolbarProps {
  dispatch: React.Dispatch<DocumentAction>;
  selection: EditorSelection | null;
  document: DocxDocument;
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

export const Toolbar: React.FC<ToolbarProps> = ({ dispatch, selection, document: doc }) => {
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

  const handleHeading = useCallback(
    (v: string) => {
      const level = parseInt(v, 10);
      dispatch({ type: 'set_heading_level', payload: { level: level === 0 ? null : level } });
    },
    [dispatch],
  );

  const handleInsertTable = useCallback(() => {
    dispatch({ type: 'insert_table', payload: { rows: 3, cols: 3 } });
  }, [dispatch]);

  const handleInsertImage = useCallback(() => {
    const input = window.document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = () => {
        const base64 = (reader.result as string).split(',')[1];
        const id = crypto.randomUUID();
        dispatch({
          type: 'insert_image',
          payload: {
            id,
            data_base64: base64,
            content_type: file.type,
          },
        });
      };
      reader.readAsDataURL(file);
    };
    input.click();
  }, [dispatch]);

  return (
    <div className="toolbar">
      <div className="toolbar-group">
        <ToolbarDropdown
          label="Heading"
          value={String(pp?.heading_level ?? 0)}
          options={HEADING_OPTIONS}
          onChange={handleHeading}
        />
      </div>

      <div className="toolbar-group">
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
      </div>

      <div className="toolbar-group">
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
        <input
          type="color"
          className="toolbar-color"
          title="Text color"
          value={rp?.color ? `#${rp.color.replace('#', '')}` : '#000000'}
          onChange={handleColor}
        />
      </div>

      <div className="toolbar-group">
        <ToolbarButton
          label="&#8676;"
          title="Align Left"
          active={pp?.alignment === 'left' || !pp?.alignment}
          onClick={() => dispatch({ type: 'set_alignment', payload: { alignment: 'left' } })}
        />
        <ToolbarButton
          label="&#8703;"
          title="Align Center"
          active={pp?.alignment === 'center'}
          onClick={() => dispatch({ type: 'set_alignment', payload: { alignment: 'center' } })}
        />
        <ToolbarButton
          label="&#8677;"
          title="Align Right"
          active={pp?.alignment === 'right'}
          onClick={() => dispatch({ type: 'set_alignment', payload: { alignment: 'right' } })}
        />
        <ToolbarButton
          label="&#8700;"
          title="Justify"
          active={pp?.alignment === 'justify'}
          onClick={() => dispatch({ type: 'set_alignment', payload: { alignment: 'justify' } })}
        />
      </div>

      <div className="toolbar-group">
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
      </div>

      <div className="toolbar-group">
        <ToolbarButton label="Table" title="Insert 3x3 Table" onClick={handleInsertTable} />
        <ToolbarButton label="Image" title="Insert Image" onClick={handleInsertImage} />
      </div>
    </div>
  );
};
