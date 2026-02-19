use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use super::GenerateError;

/// Accepts a list of (archive-path, content) pairs and writes them into a
/// ZIP archive in memory, returning the final byte vector.
pub fn write_zip(parts: &[(&str, Vec<u8>)]) -> Result<Vec<u8>, GenerateError> {
    let buf = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(buf);

    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);

    for (path, content) in parts {
        zip.start_file(*path, options)?;
        zip.write_all(content)?;
    }

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}
