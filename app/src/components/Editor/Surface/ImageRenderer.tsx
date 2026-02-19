import React from 'react';
import { ImageData } from '../../../types/document';

interface ImageRendererProps {
  imageId: string;
  images: ImageData[];
}

const ImageRenderer: React.FC<ImageRendererProps> = React.memo(
  ({ imageId, images }) => {
    const image = images.find((img) => img.id === imageId);

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

    if (image.width) {
      style.width = image.width;
    }
    if (image.height) {
      style.height = image.height;
    }

    return (
      <img
        contentEditable={false}
        src={`data:${image.content_type};base64,${image.data_base64}`}
        alt=""
        className="docx-image"
        style={style}
      />
    );
  },
);

ImageRenderer.displayName = 'ImageRenderer';

export default ImageRenderer;
