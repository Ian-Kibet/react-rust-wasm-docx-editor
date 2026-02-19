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

    return <div key={idx} />;
  });
}

const TableRenderer: React.FC<TableRendererProps> = React.memo(
  ({ table, images, dispatch }) => {
    const tableStyle: React.CSSProperties = {
      borderCollapse: 'collapse',
    };

    if (table.properties.width != null) {
      tableStyle.width = table.properties.width;
    }

    if (table.properties.layout === 'fixed') {
      tableStyle.tableLayout = 'fixed';
    }

    if (table.properties.indent != null) {
      tableStyle.marginLeft = `${table.properties.indent / 20}pt`;
    }

    if (table.properties.cell_spacing != null) {
      tableStyle.borderSpacing = `${table.properties.cell_spacing / 20}pt`;
      tableStyle.borderCollapse = 'separate';
    }

    return (
      <table
        contentEditable={false}
        data-table-id={table.id}
        className="docx-table"
        style={tableStyle}
      >
        <tbody>
          {table.rows.map((row, rowIdx) => {
            const rowStyle: React.CSSProperties = {};
            if (row.properties?.height != null) {
              rowStyle.height = `${row.properties.height / 20}pt`;
            }

            return (
              <tr key={row.id} data-row-index={rowIdx} style={rowStyle}>
                {row.cells.map((cell, colIdx) => {
                  if (cell.properties.vertical_merge === 'continue') {
                    return null;
                  }

                  const cellStyle: React.CSSProperties = {};

                  if (cell.properties.width != null) {
                    cellStyle.width = cell.properties.width;
                  }
                  if (cell.properties.shading) {
                    cellStyle.backgroundColor = cell.properties.shading;
                  }
                  if (cell.properties.vertical_align) {
                    cellStyle.verticalAlign = cell.properties.vertical_align as React.CSSProperties['verticalAlign'];
                  }
                  if (cell.properties.no_wrap) {
                    cellStyle.whiteSpace = 'nowrap';
                  }
                  if (cell.properties.padding_top != null) {
                    cellStyle.paddingTop = `${cell.properties.padding_top / 20}pt`;
                  }
                  if (cell.properties.padding_bottom != null) {
                    cellStyle.paddingBottom = `${cell.properties.padding_bottom / 20}pt`;
                  }
                  if (cell.properties.padding_left != null) {
                    cellStyle.paddingLeft = `${cell.properties.padding_left / 20}pt`;
                  }
                  if (cell.properties.padding_right != null) {
                    cellStyle.paddingRight = `${cell.properties.padding_right / 20}pt`;
                  }

                  const borders = cell.properties.borders;
                  if (borders) {
                    if (borders.top) cellStyle.borderTop = `${borders.top.size}px ${borders.top.style} ${borders.top.color}`;
                    if (borders.bottom) cellStyle.borderBottom = `${borders.bottom.size}px ${borders.bottom.style} ${borders.bottom.color}`;
                    if (borders.left) cellStyle.borderLeft = `${borders.left.size}px ${borders.left.style} ${borders.left.color}`;
                    if (borders.right) cellStyle.borderRight = `${borders.right.size}px ${borders.right.style} ${borders.right.color}`;
                  }

                  let rowSpan = 1;
                  if (cell.properties.vertical_merge === 'restart') {
                    for (let r = rowIdx + 1; r < table.rows.length; r++) {
                      const nextCell = table.rows[r].cells[colIdx];
                      if (nextCell?.properties.vertical_merge === 'continue') {
                        rowSpan++;
                      } else {
                        break;
                      }
                    }
                  }

                  return (
                    <td
                      key={cell.id}
                      data-cell-id={cell.id}
                      data-col-index={colIdx}
                      contentEditable={true}
                      suppressContentEditableWarning={true}
                      style={cellStyle}
                      colSpan={cell.properties.grid_span && cell.properties.grid_span > 1 ? cell.properties.grid_span : undefined}
                      rowSpan={rowSpan > 1 ? rowSpan : undefined}
                    >
                      {renderCellContent(cell.content, images, dispatch)}
                    </td>
                  );
                })}
              </tr>
            );
          })}
        </tbody>
      </table>
    );
  },
);

TableRenderer.displayName = 'TableRenderer';

export default TableRenderer;
