use std::collections::HashMap;
use std::io::{Cursor, Read};

use zip::ZipArchive;

use super::ParseError;

/// Read all entries from a ZIP archive (the .docx file) into an in-memory map
/// of `archive-path -> file-contents`.
pub fn read_zip(data: &[u8]) -> Result<HashMap<String, Vec<u8>>, ParseError> {
    let cursor = Cursor::new(data.to_vec());
    let mut archive = ZipArchive::new(cursor)?;
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }

        let name = entry.name().to_string();
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        files.insert(name, buf);
    }

    Ok(files)
}
