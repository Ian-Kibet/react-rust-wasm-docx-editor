import React, { useState, useCallback, useRef } from 'react';
import { ImageData } from '../../../types/document';

interface ImageRendererProps {
  imageId: string;
  images: ImageData[];
  onResize?: (imageId: string, width: number, height: number) => void;
}

const HANDLE_SIZE = 8;

const ImageRenderer: React.FC<ImageRendererProps> = React.memo(
  ({ imageId, images, onResize }) => {
    const image = images.find((img) => img.id === imageId);
    const [selected, setSelected] = useState(false);
    const [resizing, setResizing] = useState(false);
    const startRef = useRef<{ x: number; y: number; w: number; h: number } | null>(null);
    const imgRef = useRef<HTMLImageElement>(null);

    const handleSelect = useCallback((e: React.MouseEvent) => {
      e.stopPropagation();
      setSelected(true);
    }, []);

    const handleResizeStart = useCallback(
      (e: React.MouseEvent, _corner: string) => {
        e.preventDefault();
        e.stopPropagation();
        if (!imgRef.current) return;
        const rect = imgRef.current.getBoundingClientRect();
        startRef.current = { x: e.clientX, y: e.clientY, w: rect.width, h: rect.height };
        setResizing(true);

        const handleMove = (me: MouseEvent) => {
          if (!startRef.current) return;
          const dx = me.clientX - startRef.current.x;
          const dy = me.clientY - startRef.current.y;
          let newW = Math.max(20, startRef.current.w + dx);
          let newH = Math.max(20, startRef.current.h + dy);
          // Maintain aspect ratio unless shift is held
          if (!me.shiftKey && startRef.current.w > 0 && startRef.current.h > 0) {
            const ratio = startRef.current.w / startRef.current.h;
            newH = newW / ratio;
          }
          if (imgRef.current) {
            imgRef.current.style.width = `${newW}px`;
            imgRef.current.style.height = `${newH}px`;
          }
        };

        const handleUp = (me: MouseEvent) => {
          document.removeEventListener('mousemove', handleMove);
          document.removeEventListener('mouseup', handleUp);
          setResizing(false);
          if (startRef.current && imgRef.current) {
            const rect = imgRef.current.getBoundingClientRect();
            onResize?.(imageId, Math.round(rect.width), Math.round(rect.height));
          }
          startRef.current = null;
        };

        document.addEventListener('mousemove', handleMove);
        document.addEventListener('mouseup', handleUp);
      },
      [imageId, onResize],
    );

    // Click outside deselects
    React.useEffect(() => {
      if (!selected) return;
      const handler = () => setSelected(false);
      document.addEventListener('click', handler);
      return () => document.removeEventListener('click', handler);
    }, [selected]);

    if (!image) {
      return (
        <span
          contentEditable={false}
          style={{
            display: 'inline-block',
            width: 100,
            height: 100,
            backgroundColor: '#f0f0f0',
            border: '1px dashed #ccc',
            lineHeight: '100px',
            textAlign: 'center',
            fontSize: 12,
            color: '#999',
          }}
        >
          Image not found
        </span>
      );
    }

    const style: React.CSSProperties = {
      maxWidth: '100%',
      display: 'block',
    };

    if (image.width) style.width = image.width;
    if (image.height) style.height = image.height;

    return (
      <span
        contentEditable={false}
        className={`image-wrapper${selected ? ' selected' : ''}${resizing ? ' resizing' : ''}`}
        style={{ position: 'relative', display: 'inline-block' }}
        onClick={handleSelect}
      >
        <img
          ref={imgRef}
          src={`data:${image.content_type};base64,${image.data_base64}`}
          alt={image.description || ''}
          className="docx-image"
          style={style}
          draggable={false}
        />
        {selected && (
          <>
            {/* Corner resize handles */}
            {['nw', 'ne', 'sw', 'se'].map((corner) => {
              const pos: React.CSSProperties = { position: 'absolute', width: HANDLE_SIZE, height: HANDLE_SIZE, background: '#2563eb', border: '1px solid white', zIndex: 10 };
              if (corner.includes('n')) pos.top = -HANDLE_SIZE / 2;
              if (corner.includes('s')) pos.bottom = -HANDLE_SIZE / 2;
              if (corner.includes('w')) pos.left = -HANDLE_SIZE / 2;
              if (corner.includes('e')) pos.right = -HANDLE_SIZE / 2;
              pos.cursor = `${corner}-resize`;
              return (
                <div
                  key={corner}
                  className="image-resize-handle"
                  style={pos}
                  onMouseDown={(e) => handleResizeStart(e, corner)}
                />
              );
            })}
          </>
        )}
      </span>
    );
  },
);

ImageRenderer.displayName = 'ImageRenderer';

export default ImageRenderer;
