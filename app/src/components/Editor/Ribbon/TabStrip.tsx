import React from 'react';

const TABS = ['Home', 'Styles', 'Insert', 'Draw', 'Design', 'Layout', 'References', 'Mailings', 'Review', 'View'];

interface TabStripProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
}

export const TabStrip: React.FC<TabStripProps> = ({ activeTab, onTabChange }) => (
  <div className="tab-strip">
    {TABS.map((tab) => (
      <button
        key={tab}
        className={`tab-strip-btn${tab === activeTab ? ' active' : ''}`}
        onClick={() => onTabChange(tab)}
      >
        {tab}
      </button>
    ))}
  </div>
);
