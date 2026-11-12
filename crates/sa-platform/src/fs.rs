//! Game-folder file system.
//!
//! Paths are relative to the game folder and written as the game writes them
//! (`DATA\MAPS\LA\LAe.IDE`, `models\gta3.img`): `\` or `/` separated, case-insensitive (the
//! original runs on Windows). Implementations resolve them to real files.

use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};

/// Random-access reads from one open file (the IMG archives, `CdStream`).
pub trait RandomRead: Send {
    /// Reads exactly `buf.len()` bytes at `offset`.
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()>;
    fn len(&self) -> u64;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The user's game folder (and, writable, the user-files directory).
pub trait GameFs: Send + Sync {
    /// Reads a whole file.
    fn read(&self, path: &str) -> io::Result<Vec<u8>>;
    /// Opens a file for random access.
    fn open(&self, path: &str) -> io::Result<Box<dyn RandomRead>>;
    fn exists(&self, path: &str) -> bool;
    /// Whether `path` names a directory.
    fn is_dir(&self, path: &str) -> bool {
        let _ = path;
        false
    }
    /// Creates or replaces a file (writable trees only: the user-files directory).
    fn write_file(&self, path: &str, data: &[u8]) -> io::Result<()> {
        let _ = (path, data);
        Err(read_only())
    }
    /// Creates one directory level (writable trees only). An existing directory is not an error.
    fn create_dir(&self, path: &str) -> io::Result<()> {
        let _ = path;
        Err(read_only())
    }
    /// Removes a file (writable trees only).
    fn remove_file(&self, path: &str) -> io::Result<()> {
        let _ = path;
        Err(read_only())
    }
    /// The names in a directory (as stored, not case-folded), sorted.
    fn list_dir(&self, path: &str) -> io::Result<Vec<String>> {
        let _ = path;
        Err(io::ErrorKind::Unsupported.into())
    }
}

fn read_only() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "read-only file system")
}

/// Splits a game path into its non-empty components (`\` and `/` separators, surrounding
/// blanks trimmed: the game passes `gta.dat` arguments with a trailing space).
pub fn components(path: &str) -> Vec<&str> {
    path.trim()
        .split(['\\', '/'])
        .map(str::trim)
        .filter(|c| !c.is_empty() && *c != ".")
        .collect()
}

type FileMap = HashMap<String, Arc<Vec<u8>>>;

/// An in-memory file system (tests, and a base for the web implementation's OPFS cache).
/// Writable; directories exist implicitly (any prefix of a file path) or explicitly
/// ([`GameFs::create_dir`]).
#[derive(Default)]
pub struct MemFs {
    files: Mutex<FileMap>,
    dirs: Mutex<HashSet<String>>,
}

impl MemFs {
    fn key(path: &str) -> String {
        components(path).join("/").to_ascii_lowercase()
    }

    pub fn insert(&mut self, path: &str, data: Vec<u8>) {
        self.files().insert(Self::key(path), Arc::new(data));
    }

    fn files(&self) -> MutexGuard<'_, FileMap> {
        self.files.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn dirs(&self) -> MutexGuard<'_, HashSet<String>> {
        self.dirs.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn get(&self, path: &str) -> Option<Arc<Vec<u8>>> {
        self.files().get(&Self::key(path)).cloned()
    }
}

pub(crate) struct MemFile(pub(crate) Arc<Vec<u8>>);

impl RandomRead for MemFile {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        let start = usize::try_from(offset).map_err(|_| io::ErrorKind::UnexpectedEof)?;
        let src = self
            .0
            .get(start..start + buf.len())
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        buf.copy_from_slice(src);
        Ok(())
    }
    fn len(&self) -> u64 {
        self.0.len() as u64
    }
}

impl GameFs for MemFs {
    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        self.get(path)
            .map(|d| d.to_vec())
            .ok_or_else(|| io::ErrorKind::NotFound.into())
    }
    fn open(&self, path: &str) -> io::Result<Box<dyn RandomRead>> {
        let d = self.get(path).ok_or(io::ErrorKind::NotFound)?;
        Ok(Box::new(MemFile(d)))
    }
    fn exists(&self, path: &str) -> bool {
        self.get(path).is_some() || self.is_dir(path)
    }
    fn is_dir(&self, path: &str) -> bool {
        let k = Self::key(path);
        if k.is_empty() || self.dirs().contains(&k) {
            return true;
        }
        let p = format!("{k}/");
        self.files().keys().any(|f| f.starts_with(&p))
    }
    fn write_file(&self, path: &str, data: &[u8]) -> io::Result<()> {
        self.files().insert(Self::key(path), Arc::new(data.to_vec()));
        Ok(())
    }
    fn create_dir(&self, path: &str) -> io::Result<()> {
        self.dirs().insert(Self::key(path));
        Ok(())
    }
    fn remove_file(&self, path: &str) -> io::Result<()> {
        self.files()
            .remove(&Self::key(path))
            .map(|_| ())
            .ok_or_else(|| io::ErrorKind::NotFound.into())
    }
    fn list_dir(&self, path: &str) -> io::Result<Vec<String>> {
        let k = Self::key(path);
        let p = if k.is_empty() {
            String::new()
        } else {
            format!("{k}/")
        };
        let mut all: Vec<String> = self.files().keys().cloned().collect();
        all.extend(self.dirs().iter().cloned());
        let mut v: Vec<String> = all
            .iter()
            .filter_map(|f| f.strip_prefix(&p))
            .filter_map(|r| r.split('/').next())
            .filter(|r| !r.is_empty())
            .map(str::to_owned)
            .collect();
        v.sort();
        v.dedup();
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_split_and_trim() {
        assert_eq!(
            components("DATA\\MAPS\\LA\\LAe.IDE "),
            vec!["DATA", "MAPS", "LA", "LAe.IDE"]
        );
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
        fs.write_file("User/Save.b", b"x").unwrap();
        assert!(fs.is_dir("USER") && fs.exists("user\\save.B"));
        assert_eq!(fs.list_dir("").unwrap(), vec!["data", "user"]);
    }
}
