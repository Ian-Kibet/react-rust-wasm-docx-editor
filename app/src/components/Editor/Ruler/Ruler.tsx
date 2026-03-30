import React, { useRef, useCallback, useState, useEffect } from 'react';
import { DocumentAction } from '../../../hooks/useDocumentModel';
import { SectionProperties } from '../../../types/document';

interface RulerProps {
  dispatch: React.Dispatch<DocumentAction>;
  indentLeft: number;
  indentRight: number;
  indentFirstLine: number;
  sectionProperties?: SectionProperties;
  zoom?: number;
}

type HandleType = 'left' | 'firstLine' | 'right' | 'marginLeft' | 'marginRight';

const DPI = 96;

function twipsToInches(twips: number): number {
  return twips / 1440;
}

export const Ruler: React.FC<RulerProps> = ({
  dispatch,
  indentLeft,
  indentRight,
  indentFirstLine,
  sectionProperties,
  zoom = 100,
}) => {
  const rulerRef = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState<HandleType | null>(null);
  const [dragValues, setDragValues] = useState({
    left: indentLeft,
    right: indentRight,
    firstLine: indentFirstLine,
  });

  // Page dimensions from section properties (in inches)
  const pageWidthIn = twipsToInches(sectionProperties?.page_width ?? 12240);
  const marginLeftIn = twipsToInches(sectionProperties?.margin_left ?? 1440);
  const marginRightIn = twipsToInches(sectionProperties?.margin_right ?? 1440);

  const [marginDrag, setMarginDrag] = useState({
    left: marginLeftIn,
    right: marginRightIn,
  });

  // Sync props into drag values when not dragging
  useEffect(() => {
    if (!dragging) {
      setDragValues({ left: indentLeft, right: indentRight, firstLine: indentFirstLine });
    }
  }, [indentLeft, indentRight, indentFirstLine, dragging]);

  useEffect(() => {
    if (!dragging) {
      setMarginDrag({ left: marginLeftIn, right: marginRightIn });
    }
  }, [marginLeftIn, marginRightIn, dragging]);

  const contentWidth = pageWidthIn - marginDrag.left - marginDrag.right;
  const scale = zoom / 100;
  const scaledWidthPx = pageWidthIn * DPI * scale;

  const inchToContentPx = useCallback(
    (inches: number) => (marginDrag.left + inches) * DPI * scale,
    [marginDrag.left, scale],
  );

  const pxToContentInch = useCallback(
    (px: number) => px / (DPI * scale) - marginDrag.left,
    [marginDrag.left, scale],
  );

  const pxToAbsoluteInch = useCallback(
    (px: number) => px / (DPI * scale),
    [scale],
  );

  const clamp = (val: number, min: number, max: number) => Math.max(min, Math.min(max, val));

  const handleMouseDown = useCallback(
    (type: HandleType) => (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setDragging(type);
    },
    [],
  );

  useEffect(() => {
    if (!dragging) return;

    const onMouseMove = (e: MouseEvent) => {
      const ruler = rulerRef.current;
      if (!ruler) return;
      const rect = ruler.getBoundingClientRect();
      const relX = e.clientX - rect.left;

      if (dragging === 'marginLeft') {
        const inch = pxToAbsoluteInch(relX);
        setMarginDrag(prev => ({
          ...prev,
          left: clamp(inch, 0.25, pageWidthIn - prev.right - 1),
        }));
      } else if (dragging === 'marginRight') {
        const inch = pageWidthIn - pxToAbsoluteInch(relX);
        setMarginDrag(prev => ({
          ...prev,
          right: clamp(inch, 0.25, pageWidthIn - prev.left - 1),
        }));
      } else {
        const inch = pxToContentInch(relX);
        setDragValues((prev) => {
          const next = { ...prev };
          if (dragging === 'left') {
            next.left = clamp(inch, 0, contentWidth - prev.right - 0.1);
          } else if (dragging === 'firstLine') {
            next.firstLine = clamp(
              inch - prev.left,
              -prev.left,
              contentWidth - prev.left - prev.right - 0.1,
            );
          } else if (dragging === 'right') {
            const fromRight = contentWidth - inch;
            next.right = clamp(fromRight, 0, contentWidth - prev.left - 0.1);
          }
          return next;
        });
      }
    };

    const onMouseUp = () => {
      setDragging(null);

      if (dragging === 'marginLeft' || dragging === 'marginRight') {
        setMarginDrag((current) => {
          const round = (v: number) => Math.round(v * 20) / 20;
          const rounded = { left: round(current.left), right: round(current.right) };
          dispatch({
            type: 'set_section_properties',
            payload: {
              margin_left: Math.round(rounded.left * 1440),
              margin_right: Math.round(rounded.right * 1440),
            },
          });
          return rounded;
        });
      } else {
        setDragValues((current) => {
          const round = (v: number) => Math.round(v * 20) / 20;
          const rounded = {
            left: round(current.left),
            right: round(current.right),
            firstLine: round(current.firstLine),
          };
          dispatch({
            type: 'set_paragraph_indent',
            payload: {
              indent_left: rounded.left,
              indent_right: rounded.right,
              indent_first_line: rounded.firstLine,
            },
          });
          return rounded;
        });
      }
    };

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
    return () => {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
  }, [dragging, contentWidth, pageWidthIn, pxToContentInch, pxToAbsoluteInch, dispatch, scale]);

  // Generate tick marks
  const ticks: React.ReactNode[] = [];
  for (let i = 0; i <= pageWidthIn * 4; i++) {
    const inches = i / 4;
    const px = inches * DPI * scale;
    const isInMargin = inches < marginDrag.left || inches > pageWidthIn - marginDrag.right;
    if (isInMargin) continue;

    if (i % 4 === 0) {
      const num = inches - marginDrag.left;
      ticks.push(
        <div key={`t${i}`} className="ruler-tick ruler-tick-major" style={{ left: px }}>
          <span className="ruler-number">{num > 0 ? num : ''}</span>
        </div>,
      );
    } else if (i % 2 === 0) {
      ticks.push(<div key={`t${i}`} className="ruler-tick ruler-tick-half" style={{ left: px }} />);
    } else {
      ticks.push(<div key={`t${i}`} className="ruler-tick ruler-tick-quarter" style={{ left: px }} />);
    }
  }

  // Handle positions
  const leftHandlePx = inchToContentPx(dragValues.left);
  const firstLineHandlePx = inchToContentPx(dragValues.left + dragValues.firstLine);
  const rightHandlePx = inchToContentPx(contentWidth - dragValues.right);

  // Margin boundary positions
  const marginLeftPx = marginDrag.left * DPI * scale;
  const marginRightPx = marginDrag.right * DPI * scale;

  return (
    <div className="ruler-wrapper">
      <div
        ref={rulerRef}
        className={`ruler${dragging ? ' ruler-dragging' : ''}`}
        style={{ width: scaledWidthPx }}
      >
        {/* Left margin zone */}
        <div
          className="ruler-margin ruler-margin-left"
          style={{ left: 0, width: marginLeftPx }}
        />
        {/* Right margin zone */}
        <div
          className="ruler-margin ruler-margin-right"
          style={{ right: 0, width: marginRightPx }}
        />

        {/* Margin drag handles */}
        <div
          className={`ruler-margin-handle ruler-margin-handle-left${dragging === 'marginLeft' ? ' dragging' : ''}`}
          style={{ left: marginLeftPx }}
          onMouseDown={handleMouseDown('marginLeft')}
          title={`Left Margin: ${marginDrag.left.toFixed(2)}"`}
        />
        <div
          className={`ruler-margin-handle ruler-margin-handle-right${dragging === 'marginRight' ? ' dragging' : ''}`}
          style={{ left: scaledWidthPx - marginRightPx }}
          onMouseDown={handleMouseDown('marginRight')}
          title={`Right Margin: ${marginDrag.right.toFixed(2)}"`}
        />

        {/* Tick marks */}
        {ticks}

        {/* First-line indent handle (top triangle) */}
        <div
          className={`ruler-handle ruler-handle-first-line${dragging === 'firstLine' ? ' dragging' : ''}`}
          style={{ left: firstLineHandlePx }}
          onMouseDown={handleMouseDown('firstLine')}
          title={`First Line Indent: ${dragValues.firstLine.toFixed(2)}"`}
        />

        {/* Left indent handle (bottom triangle) */}
        <div
          className={`ruler-handle ruler-handle-left${dragging === 'left' ? ' dragging' : ''}`}
          style={{ left: leftHandlePx }}
          onMouseDown={handleMouseDown('left')}
          title={`Left Indent: ${dragValues.left.toFixed(2)}"`}
        />

        {/* Right indent handle (bottom triangle) */}
        <div
          className={`ruler-handle ruler-handle-right${dragging === 'right' ? ' dragging' : ''}`}
          style={{ left: rightHandlePx }}
          onMouseDown={handleMouseDown('right')}
          title={`Right Indent: ${dragValues.right.toFixed(2)}"`}
        />
      </div>
    </div>
  );
};
