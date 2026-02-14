use std::{
    io::Result,
    path::{Path, PathBuf},
};

/**
    A trait for abstracting filesystem operations.
*/
pub trait FileSystem: Send + Sync + std::fmt::Debug {
    fn is_file(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn read(&self, path: &Path) -> Result<Vec<u8>>;
    fn read_to_string(&self, path: &Path) -> Result<String>;
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
}

/**
    Standard filesystem implementation using `std::fs`.
*/
#[derive(Debug, Clone, Copy)]
pub struct StdFileSystem;

impl FileSystem for StdFileSystem {
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(path)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            entries.push(entry?.path());
        }
        Ok(entries)
    }
}
