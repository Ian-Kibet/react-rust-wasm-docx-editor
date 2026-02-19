use std::collections::HashMap;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use uuid::Uuid;

use crate::model::ImageData;

/// Extract all files under `word/media/` from the ZIP contents, base64-encode
/// them, and return a list of `ImageData` entries.
///
/// The returned `ImageData::id` is a freshly generated UUID that will be
/// referenced by relationship ID -> image-id lookups during document parsing.
///
/// Also returns a map of `archive-path -> image-id` so the document parser can
/// resolve relationship targets to the correct `ImageData::id`.
pub fn extract_images(
    files: &HashMap<String, Vec<u8>>,
) -> (Vec<ImageData>, HashMap<String, String>) {
    let mut images = Vec::new();
    let mut path_to_id: HashMap<String, String> = HashMap::new();

    for (path, data) in files {
        if !path.starts_with("word/media/") {
            continue;
        }

        let id = Uuid::new_v4().to_string();
        let filename = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .to_string();
        let content_type = mime_from_filename(&filename);
        let data_base64 = STANDARD.encode(data);

        path_to_id.insert(path.clone(), id.clone());

        images.push(ImageData {
            id,
            data_base64,
            content_type,
            width: None,
            height: None,
            description: None,
        });
    }

    (images, path_to_id)
}

/// Derive a MIME content-type from a filename extension.
fn mime_from_filename(filename: &str) -> String {
    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "emf" => "image/x-emf",
        "wmf" => "image/x-wmf",
        _ => "application/octet-stream",
    }
    .to_string()
}
