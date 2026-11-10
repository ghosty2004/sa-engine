//! Game-folder file system.
//!
//! Paths are relative to the game folder and written as the game writes them
//! (`DATA\MAPS\LA\LAe.IDE`, `models\gta3.img`): `\` or `/` separated, case-insensitive (the
//! original runs on Windows). Implementations resolve them to real files.

use std::io;

/// Random-access reads from one open file (the IMG archives, `CdStream`).
pub trait RandomRead: Send {
    /// Reads exactly `buf.len()` bytes at `offset`.
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()>;
    fn len(&self) -> u64;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The user's game folder.
pub trait GameFs: Send + Sync {
    /// Reads a whole file.
    fn read(&self, path: &str) -> io::Result<Vec<u8>>;
    /// Opens a file for random access.
    fn open(&self, path: &str) -> io::Result<Box<dyn RandomRead>>;
    fn exists(&self, path: &str) -> bool;
}

/// Splits a game path into its non-empty components (`\` and `/` separators, surrounding
/// blanks trimmed: the game passes `gta.dat` arguments with a trailing space).
pub fn components(path: &str) -> Vec<&str> {
    path.trim().split(['\\', '/']).map(str::trim).filter(|c| !c.is_empty() && *c != ".").collect()
}

/// An in-memory file system (tests, and a base for the web implementation's OPFS cache).
#[derive(Default)]
pub struct MemFs {
    files: std::collections::HashMap<String, std::sync::Arc<Vec<u8>>>,
}

impl MemFs {
    fn key(path: &str) -> String {
        components(path).join("/").to_ascii_lowercase()
    }

    pub fn insert(&mut self, path: &str, data: Vec<u8>) {
        self.files.insert(Self::key(path), std::sync::Arc::new(data));
    }
}

struct MemFile(std::sync::Arc<Vec<u8>>);

impl RandomRead for MemFile {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        let start = usize::try_from(offset).map_err(|_| io::ErrorKind::UnexpectedEof)?;
        let src = self.0.get(start..start + buf.len()).ok_or(io::ErrorKind::UnexpectedEof)?;
        buf.copy_from_slice(src);
        Ok(())
    }
    fn len(&self) -> u64 {
        self.0.len() as u64
    }
}

impl GameFs for MemFs {
    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        self.files.get(&Self::key(path)).map(|d| d.to_vec()).ok_or_else(|| io::ErrorKind::NotFound.into())
    }
    fn open(&self, path: &str) -> io::Result<Box<dyn RandomRead>> {
        let d = self.files.get(&Self::key(path)).ok_or(io::ErrorKind::NotFound)?;
        Ok(Box::new(MemFile(d.clone())))
    }
    fn exists(&self, path: &str) -> bool {
        self.files.contains_key(&Self::key(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_split_and_trim() {
        assert_eq!(components("DATA\\MAPS\\LA\\LAe.IDE "), vec!["DATA", "MAPS", "LA", "LAe.IDE"]);
        assert_eq!(components("./models/gta3.img"), vec!["models", "gta3.img"]);
    }

    #[test]
    fn memfs_is_case_insensitive() {
        let mut fs = MemFs::default();
        fs.insert("data/gta.dat", b"abc".to_vec());
        assert_eq!(fs.read("DATA\\GTA.DAT").unwrap(), b"abc");
        let mut f = fs.open("Data/Gta.dat").unwrap();
        let mut b = [0u8; 2];
        f.read_at(1, &mut b).unwrap();
        assert_eq!(&b, b"bc");
        assert!(f.read_at(2, &mut b).is_err());
    }
}
