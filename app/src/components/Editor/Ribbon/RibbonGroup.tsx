import React from 'react';

interface RibbonGroupProps {
  label: string;
  children: React.ReactNode;
}

export const RibbonGroup: React.FC<RibbonGroupProps> = ({ label, children }) => (
  <div className="ribbon-group">
    <div className="ribbon-group-content">{children}</div>
    <div className="ribbon-group-label">{label}</div>
  </div>
);
