use std::{
    fmt,
    io::{Cursor, Read, Result as IoResult},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use lune_utils::fs::FileSystem;
use zip::ZipArchive;

/**
    A filesystem implementation that reads from a ZIP archive in memory.
*/
#[derive(Clone)]
pub struct ZipFileSystem {
    archive: Arc<Mutex<ZipArchive<Cursor<Vec<u8>>>>>,
}

impl ZipFileSystem {
    pub fn new(data: Vec<u8>) -> IoResult<Self> {
        let reader = Cursor::new(data);
        let archive = ZipArchive::new(reader)?;
        Ok(Self {
            archive: Arc::new(Mutex::new(archive)),
        })
    }

    fn normalize_path(path: &Path) -> String {
        let path = if path.is_absolute() {
            if let Ok(cwd) = std::env::current_dir() {
                path.strip_prefix(&cwd).unwrap_or(path)
            } else {
                path
            }
        } else {
            path
        };
        let s = path.to_string_lossy().replace('\\', "/");
        s.trim_start_matches('/').to_string()
    }
}

impl fmt::Debug for ZipFileSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ZipFileSystem").finish()
    }
}

impl FileSystem for ZipFileSystem {
    fn is_file(&self, path: &Path) -> bool {
        let name = Self::normalize_path(path);
        let mut archive = self.archive.lock().unwrap();
        archive.by_name(&name).is_ok()
    }

    fn is_dir(&self, path: &Path) -> bool {
        let name = Self::normalize_path(path);
        if name.is_empty() {
            return true;
        }
        let mut archive = self.archive.lock().unwrap();
        // Zip entries usually explicitly have directories,
        // but sometimes they are implicit.
        // Try finding exact directory entry (often ends with /)
        if archive.by_name(&name).is_ok() {
            return true; // Use more robust check if needed
        }
        if archive.by_name(&format!("{}/", name)).is_ok() {
            return true;
        }
        // Fallback: check if any file starts with this prefix
        let prefix = format!("{}/", name);
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                if file.name().starts_with(&prefix) {
                    return true;
                }
            }
        }
        false
    }

    fn read(&self, path: &Path) -> IoResult<Vec<u8>> {
        let name = Self::normalize_path(path);
        let mut archive = self.archive.lock().unwrap();
        let mut file = archive.by_name(&name)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    fn read_to_string(&self, path: &Path) -> IoResult<String> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    fn read_dir(&self, path: &Path) -> IoResult<Vec<PathBuf>> {
        let name = Self::normalize_path(path);
        let prefix = if name.is_empty() {
            String::new()
        } else {
            format!("{}/", name)
        };

        let mut archive = self.archive.lock().unwrap();
        let mut entries = Vec::new();

        // Iterate all files to find direct children
        // This is O(N) for every read_dir, but fine for small archives.
        // Optimizing this would require building a tree index.
        let file_names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
            .collect();

        for file_name in file_names {
            if !file_name.starts_with(&prefix) {
                continue;
            }
            if file_name == prefix {
                continue; // self
            }

            let suffix = &file_name[prefix.len()..];
            // If suffix contains /, it's a sub-sub-file.
            // We only want direct children.
            // But if it's a directory, it might end with /

            let parts: Vec<&str> = suffix.split('/').filter(|s| !s.is_empty()).collect();
            if parts.is_empty() {
                continue;
            }

            // The direct child name is the first part
            let child_name = parts[0];
            let child_path = if prefix.is_empty() {
                PathBuf::from(child_name)
            } else {
                // Construct path correctly using Path (OS dependent separator)
                // But input path was normalized.
                // We should return PathBufs relative to root or whatever expected.
                // The trait returns Vec<PathBuf>. Usually absolute or relative depending on input?
                // StdFileSystem returns entries which are joined with input path.
                path.join(child_name)
            };

            if !entries.contains(&child_path) {
                entries.push(child_path);
            }
        }

        Ok(entries)
    }
}
