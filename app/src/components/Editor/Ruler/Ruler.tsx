import React, { useRef, useCallback, useState, useEffect } from 'react';
import { DocumentAction } from '../../../hooks/useDocumentModel';

interface RulerProps {
  dispatch: React.Dispatch<DocumentAction>;
  indentLeft: number;
  indentRight: number;
  indentFirstLine: number;
  pageWidth?: number;
  marginLeft?: number;
  marginRight?: number;
}

type HandleType = 'left' | 'firstLine' | 'right';

const DPI = 96;

export const Ruler: React.FC<RulerProps> = ({
  dispatch,
  indentLeft,
  indentRight,
  indentFirstLine,
  pageWidth = 8.5,
  marginLeft = 1.0,
  marginRight = 1.0,
}) => {
  const rulerRef = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState<HandleType | null>(null);
  const [dragValues, setDragValues] = useState({ left: indentLeft, right: indentRight, firstLine: indentFirstLine });

  // Sync props into drag values when not dragging
  useEffect(() => {
    if (!dragging) {
      setDragValues({ left: indentLeft, right: indentRight, firstLine: indentFirstLine });
    }
  }, [indentLeft, indentRight, indentFirstLine, dragging]);

  const contentWidth = pageWidth - marginLeft - marginRight; // e.g. 6.5"
  const totalWidthPx = pageWidth * DPI; // 816px

  // Convert inch position within content zone to px from left edge of ruler
  const inchToContentPx = useCallback(
    (inches: number) => (marginLeft + inches) * DPI,
    [marginLeft],
  );

  // Convert px from left edge of ruler to inches within content zone
  const pxToContentInch = useCallback(
    (px: number) => px / DPI - marginLeft,
    [marginLeft],
  );

  const clamp = (val: number, min: number, max: number) => Math.max(min, Math.min(max, val));

  const handleMouseDown = useCallback(
    (type: HandleType) => (e: React.MouseEvent) => {
      e.preventDefault();
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
      const inch = pxToContentInch(relX);

      setDragValues((prev) => {
        const next = { ...prev };
        if (dragging === 'left') {
          next.left = clamp(inch, 0, contentWidth - prev.right - 0.1);
        } else if (dragging === 'firstLine') {
          next.firstLine = clamp(inch - prev.left, -prev.left, contentWidth - prev.left - prev.right - 0.1);
        } else if (dragging === 'right') {
          const fromRight = contentWidth - inch;
          next.right = clamp(fromRight, 0, contentWidth - prev.left - 0.1);
        }
        return next;
      });
    };

    const onMouseUp = () => {
      setDragging(null);
      setDragValues((current) => {
        // Round to nearest 0.05"
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
    };

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
    return () => {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
  }, [dragging, contentWidth, pxToContentInch, dispatch]);

  // Generate tick marks
  const ticks: React.ReactNode[] = [];
  for (let i = 0; i <= pageWidth * 4; i++) {
    const inches = i / 4;
    const px = inches * DPI;
    const isInMargin = inches < marginLeft || inches > pageWidth - marginRight;
    if (isInMargin) continue;

    if (i % 4 === 0) {
      // Major tick + number
      const num = inches - marginLeft;
      ticks.push(
        <div key={`t${i}`} className="ruler-tick ruler-tick-major" style={{ left: px }}>
          <span className="ruler-number">{num > 0 ? num : ''}</span>
        </div>,
      );
    } else if (i % 2 === 0) {
      // Half-inch tick
      ticks.push(<div key={`t${i}`} className="ruler-tick ruler-tick-half" style={{ left: px }} />);
    } else {
      // Quarter-inch tick
      ticks.push(<div key={`t${i}`} className="ruler-tick ruler-tick-quarter" style={{ left: px }} />);
    }
  }

  // Handle positions (px from left edge)
  const leftHandlePx = inchToContentPx(dragValues.left);
  const firstLineHandlePx = inchToContentPx(dragValues.left + dragValues.firstLine);
  const rightHandlePx = inchToContentPx(contentWidth - dragValues.right);

  return (
    <div className="ruler-wrapper">
      <div
        ref={rulerRef}
        className={`ruler${dragging ? ' ruler-dragging' : ''}`}
        style={{ width: totalWidthPx }}
      >
        {/* Margin zones */}
        <div
          className="ruler-margin"
          style={{ left: 0, width: marginLeft * DPI }}
        />
        <div
          className="ruler-margin"
          style={{ right: 0, width: marginRight * DPI }}
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

        {/* Right indent handle (bottom triangle, flipped) */}
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
