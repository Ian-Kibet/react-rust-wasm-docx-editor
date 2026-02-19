use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::model::ImageData;
use super::GenerateError;

/// A decoded media file ready to be written into the ZIP archive.
pub struct MediaFile {
    /// Archive path, e.g. `word/media/img1.png`
    pub path: String,
    /// Raw decoded bytes of the image.
    pub data: Vec<u8>,
}

/// Decode all base64-encoded images from the document model and return a
/// list of `MediaFile` entries to include in the ZIP package.
pub fn decode_images(images: &[ImageData]) -> Result<Vec<MediaFile>, GenerateError> {
    let mut files = Vec::with_capacity(images.len());
    for img in images {
        let ext = extension_for_content_type(&img.content_type);
        let data = STANDARD.decode(&img.data_base64)?;
        files.push(MediaFile {
            path: format!("word/media/{}.{}", img.id, ext),
            data,
        });
    }
    Ok(files)
}

fn extension_for_content_type(ct: &str) -> &str {
    match ct {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpeg",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/tiff" => "tiff",
        "image/svg+xml" => "svg",
        "image/webp" => "webp",
        _ => "bin",
    }
}
