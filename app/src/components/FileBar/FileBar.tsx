import React, { useRef, useCallback, useState } from 'react';

interface FileBarProps {
  onUpload: (file: File) => void;
  onDownload: () => void;
  onNew: () => void;
  isDirty: boolean;
}

export const FileBar: React.FC<FileBarProps> = ({ onUpload, onDownload, onNew, isDirty }) => {
  const fileRef = useRef<HTMLInputElement>(null);
  const [dragOver, setDragOver] = useState(false);

  const handleFileChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (file) onUpload(file);
      if (fileRef.current) fileRef.current.value = '';
    },
    [onUpload],
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragOver(false);
      const file = e.dataTransfer.files[0];
      if (file && file.name.endsWith('.docx')) onUpload(file);
    },
    [onUpload],
  );

  return (
    <div
      className={`file-bar${dragOver ? ' drag-over' : ''}`}
      onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
      onDragLeave={() => setDragOver(false)}
      onDrop={handleDrop}
    >
      <div className="file-bar-left">
        <button className="file-btn" onClick={onNew} title="New Document">
          New
        </button>
        <button className="file-btn" onClick={() => fileRef.current?.click()} title="Open DOCX">
          Open
        </button>
        <input
          ref={fileRef}
          type="file"
          accept=".docx"
          style={{ display: 'none' }}
          onChange={handleFileChange}
        />
        <button className="file-btn" onClick={onDownload} title="Download as DOCX">
          Save
        </button>
      </div>
      <div className="file-bar-right">
        {isDirty && <span className="dirty-indicator" title="Unsaved changes">&bull;</span>}
        <span className="app-title">DOCX Editor</span>
      </div>
    </div>
  );
};
