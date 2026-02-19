import React from 'react';
import { Table, BlockElement, ImageData } from '../../../types/document';
import ParagraphRenderer from './ParagraphRenderer';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type DocumentAction = any;

interface TableRendererProps {
  table: Table;
  images: ImageData[];
  dispatch: React.Dispatch<DocumentAction>;
}

/**
 * Render the block elements inside a table cell. This supports nested
 * paragraphs and tables (recursive).
 */
function renderCellContent(
  blocks: BlockElement[],
  images: ImageData[],
  dispatch: React.Dispatch<DocumentAction>,
): React.ReactNode {
  return blocks.map((block, idx) => {
    if ('paragraph' in block) {
      return (
        <ParagraphRenderer
          key={block.paragraph.id}
          paragraph={block.paragraph}
          images={images}
        />
      );
    }

    if ('table' in block) {
      return (
        <TableRenderer
          key={block.table.id}
          table={block.table}
          images={images}
          dispatch={dispatch}
        />
      );
    }

    // Fallback for unexpected block element shapes.
    return <div key={idx} />;
  });
}

const TableRenderer: React.FC<TableRendererProps> = React.memo(
  ({ table, images, dispatch }) => {
    // Build table-level styles from properties.
    const tableStyle: React.CSSProperties = {
      borderCollapse: 'collapse',
    };

    if (table.properties.width != null) {
      tableStyle.width = table.properties.width;
    }

    return (
      <table
        contentEditable={false}
        data-table-id={table.id}
        className="docx-table"
        style={tableStyle}
      >
        <tbody>
          {table.rows.map((row) => (
            <tr key={row.id}>
              {row.cells.map((cell) => {
                const cellStyle: React.CSSProperties = {};

                if (cell.properties.width != null) {
                  cellStyle.width = cell.properties.width;
                }

                if (cell.properties.shading) {
                  cellStyle.backgroundColor = cell.properties.shading;
                }

                if (cell.properties.vertical_align) {
                  cellStyle.verticalAlign = cell.properties
                    .vertical_align as React.CSSProperties['verticalAlign'];
                }

                // Apply cell borders if present.
                const borders = cell.properties.borders;
                if (borders) {
                  if (borders.top) {
                    cellStyle.borderTop = `${borders.top.size}px ${borders.top.style} ${borders.top.color}`;
                  }
                  if (borders.bottom) {
                    cellStyle.borderBottom = `${borders.bottom.size}px ${borders.bottom.style} ${borders.bottom.color}`;
                  }
                  if (borders.left) {
                    cellStyle.borderLeft = `${borders.left.size}px ${borders.left.style} ${borders.left.color}`;
                  }
                  if (borders.right) {
                    cellStyle.borderRight = `${borders.right.size}px ${borders.right.style} ${borders.right.color}`;
                  }
                }

                return (
                  <td
                    key={cell.id}
                    contentEditable={true}
                    suppressContentEditableWarning={true}
                    style={cellStyle}
                  >
                    {renderCellContent(cell.content, images, dispatch)}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    );
  },
);

TableRenderer.displayName = 'TableRenderer';

export default TableRenderer;
