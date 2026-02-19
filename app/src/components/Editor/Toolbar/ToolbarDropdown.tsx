import React from 'react';

interface ToolbarDropdownProps {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
}

export const ToolbarDropdown: React.FC<ToolbarDropdownProps> = React.memo(
  ({ label, value, options, onChange }) => (
    <label className="toolbar-dropdown" title={label}>
      <select value={value} onChange={(e) => onChange(e.target.value)}>
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </label>
  ),
);
