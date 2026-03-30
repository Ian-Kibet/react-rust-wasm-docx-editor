import React, { useCallback } from 'react';
import { ToolbarButton } from '../Toolbar/ToolbarButton';
import { ToolbarDropdown } from '../Toolbar/ToolbarDropdown';
import { RibbonGroup } from './RibbonGroup';
import { SectionProperties } from '../../../types/document';
import { DocumentAction } from '../../../hooks/useDocumentModel';

interface PageLayoutRibbonProps {
  dispatch: React.Dispatch<DocumentAction>;
  sectionProperties?: SectionProperties;
}

const PAGE_SIZES = [
  { value: 'letter', label: 'Letter (8.5" x 11")' },
  { value: 'a4', label: 'A4 (210mm x 297mm)' },
  { value: 'legal', label: 'Legal (8.5" x 14")' },
  { value: 'a3', label: 'A3 (297mm x 420mm)' },
];

const MARGIN_PRESETS = [
  { value: 'normal', label: 'Normal' },
  { value: 'narrow', label: 'Narrow' },
  { value: 'wide', label: 'Wide' },
  { value: 'moderate', label: 'Moderate' },
];

const COLUMN_OPTIONS = [
  { value: '1', label: 'One' },
  { value: '2', label: 'Two' },
  { value: '3', label: 'Three' },
];

// Page sizes in twips (1 inch = 1440 twips)
const PAGE_SIZE_MAP: Record<string, { page_width: number; page_height: number }> = {
  letter: { page_width: 12240, page_height: 15840 },
  a4: { page_width: 11906, page_height: 16838 },
  legal: { page_width: 12240, page_height: 20160 },
  a3: { page_width: 16838, page_height: 23811 },
};

// Margins in twips
const MARGIN_MAP: Record<string, Partial<SectionProperties>> = {
  normal: { margin_top: 1440, margin_right: 1440, margin_bottom: 1440, margin_left: 1440 },
  narrow: { margin_top: 720, margin_right: 720, margin_bottom: 720, margin_left: 720 },
  wide: { margin_top: 1440, margin_right: 2880, margin_bottom: 1440, margin_left: 2880 },
  moderate: { margin_top: 1440, margin_right: 1080, margin_bottom: 1440, margin_left: 1080 },
};

export const PageLayoutRibbon: React.FC<PageLayoutRibbonProps> = ({
  dispatch,
  sectionProperties,
}) => {
  const sp = sectionProperties;

  const handlePageSize = useCallback(
    (value: string) => {
      const size = PAGE_SIZE_MAP[value];
      if (size) {
        dispatch({ type: 'set_section_properties', payload: size });
      }
    },
    [dispatch],
  );

  const handleMargins = useCallback(
    (value: string) => {
      const margins = MARGIN_MAP[value];
      if (margins) {
        dispatch({ type: 'set_section_properties', payload: margins });
      }
    },
    [dispatch],
  );

  const handleOrientation = useCallback(
    (orient: 'portrait' | 'landscape') => {
      const currentWidth = sp?.page_width ?? 12240;
      const currentHeight = sp?.page_height ?? 15840;
      if (orient === 'landscape' && currentWidth < currentHeight) {
        dispatch({
          type: 'set_section_properties',
          payload: {
            page_width: currentHeight,
            page_height: currentWidth,
            page_orientation: 'landscape',
          },
        });
      } else if (orient === 'portrait' && currentWidth > currentHeight) {
        dispatch({
          type: 'set_section_properties',
          payload: {
            page_width: currentHeight,
            page_height: currentWidth,
            page_orientation: 'portrait',
          },
        });
      }
    },
    [dispatch, sp],
  );

  const handleColumns = useCallback(
    (value: string) => {
      dispatch({
        type: 'set_section_properties',
        payload: { columns: parseInt(value, 10) },
      });
    },
    [dispatch],
  );

  const isLandscape =
    sp?.page_orientation === 'landscape' ||
    (sp?.page_width != null && sp?.page_height != null && sp.page_width > sp.page_height);

  return (
    <div className="ribbon-panel">
      <RibbonGroup label="Page Setup">
        <div className="ribbon-col">
          <div className="ribbon-row">
            <ToolbarDropdown
              label="Size"
              value=""
              options={[{ value: '', label: 'Size' }, ...PAGE_SIZES]}
              onChange={handlePageSize}
            />
          </div>
          <div className="ribbon-row">
            <ToolbarDropdown
              label="Margins"
              value=""
              options={[{ value: '', label: 'Margins' }, ...MARGIN_PRESETS]}
              onChange={handleMargins}
            />
          </div>
        </div>
      </RibbonGroup>

      <RibbonGroup label="Orientation">
        <div className="ribbon-col">
          <div className="ribbon-row">
            <ToolbarButton
              label="Portrait"
              title="Portrait"
              active={!isLandscape}
              onClick={() => handleOrientation('portrait')}
            />
          </div>
          <div className="ribbon-row">
            <ToolbarButton
              label="Landscape"
              title="Landscape"
              active={isLandscape}
              onClick={() => handleOrientation('landscape')}
            />
          </div>
        </div>
      </RibbonGroup>

      <RibbonGroup label="Columns">
        <div className="ribbon-col">
          <div className="ribbon-row">
            <ToolbarDropdown
              label="Columns"
              value={String(sp?.columns ?? 1)}
              options={COLUMN_OPTIONS}
              onChange={handleColumns}
            />
          </div>
          <div className="ribbon-row" />
        </div>
      </RibbonGroup>
    </div>
  );
};
