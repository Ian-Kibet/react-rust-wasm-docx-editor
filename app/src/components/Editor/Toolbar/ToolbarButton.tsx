import React from 'react';

interface ToolbarButtonProps {
  label: string;
  title: string;
  active?: boolean;
  onClick: () => void;
  disabled?: boolean;
  size?: 'sm' | 'lg';
}

export const ToolbarButton: React.FC<ToolbarButtonProps> = React.memo(
  ({ label, title, active, onClick, disabled, size }) => (
    <button
      className={`toolbar-btn${size === 'lg' ? ' toolbar-btn-lg' : ''}${active ? ' active' : ''}`}
      title={title}
      aria-pressed={active}
      onClick={onClick}
      disabled={disabled}
    >
      {label}
    </button>
  ),
);
