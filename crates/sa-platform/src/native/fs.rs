//! `std::fs` game folder with case-insensitive path resolution.

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::fs::{GameFs, RandomRead, components};

/// The game folder on a native file system. Every path component is matched
/// case-insensitively (directory listings are cached).
pub struct NativeFs {
    root: PathBuf,
    dirs: Mutex<HashMap<PathBuf, HashMap<String, String>>>,
}

impl NativeFs {
    pub fn new(root: impl Into<PathBuf>) -> io::Result<NativeFs> {
        let root = root.into();
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{} is not a directory", root.display()),
            ));
        }
        Ok(NativeFs {
            root,
            dirs: Mutex::new(HashMap::new()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolves a game path to a real path.
    pub fn resolve(&self, path: &str) -> io::Result<PathBuf> {
        let mut cur = self.root.clone();
        let mut dirs = self.dirs.lock().unwrap_or_else(|e| e.into_inner());
        for c in components(path) {
            if c == ".." {
                cur.pop();
                continue;
            }
            let listing = match dirs.get(&cur) {
                Some(l) => l,
                None => {
                    let mut l = HashMap::new();
                    for e in std::fs::read_dir(&cur)?.flatten() {
                        let n = e.file_name().to_string_lossy().into_owned();
                        l.insert(n.to_ascii_lowercase(), n);
                    }
                    dirs.entry(cur.clone()).or_insert(l)
                }
            };
            let real = listing.get(&c.to_ascii_lowercase()).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("{path}: no {c} in {}", cur.display()),
                )
            })?;
            cur.push(real);
        }
        Ok(cur)
    }
}

struct NativeFile {
    file: File,
    len: u64,
}

impl RandomRead for NativeFile {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileExt;
            self.file.read_exact_at(buf, offset)
        }
        #[cfg(not(unix))]
        {
            use std::io::{Read, Seek, SeekFrom};
            self.file.seek(SeekFrom::Start(offset))?;
            self.file.read_exact(buf)
        }
    }
    fn len(&self) -> u64 {
        self.len
    }
}

impl GameFs for NativeFs {
    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        std::fs::read(self.resolve(path)?)
    }
    fn open(&self, path: &str) -> io::Result<Box<dyn RandomRead>> {
        let file = File::open(self.resolve(path)?)?;
        let len = file.metadata()?.len();
        Ok(Box::new(NativeFile { file, len }))
    }
    fn exists(&self, path: &str) -> bool {
        self.resolve(path).is_ok_and(|p| p.exists())
    }
    fn is_dir(&self, path: &str) -> bool {
        self.resolve(path).is_ok_and(|p| p.is_dir())
    }
    fn write_file(&self, path: &str, data: &[u8]) -> io::Result<()> {
        let p = self.resolve_for_create(path)?;
        std::fs::write(&p, data)?;
        self.forget(&p);
        Ok(())
    }
    fn create_dir(&self, path: &str) -> io::Result<()> {
        let p = self.resolve_for_create(path)?;
        if !p.is_dir() {
            std::fs::create_dir(&p)?;
            self.forget(&p);
        }
        Ok(())
    }
    fn remove_file(&self, path: &str) -> io::Result<()> {
        let p = self.resolve(path)?;
        std::fs::remove_file(&p)?;
        self.forget(&p);
        Ok(())
    }
    fn list_dir(&self, path: &str) -> io::Result<Vec<String>> {
        let mut v: Vec<String> = std::fs::read_dir(self.resolve(path)?)?
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        Ok(v)
    }
}

impl NativeFs {
    /// Like [`NativeFs::resolve`], but the last component may not exist yet: an existing entry
    /// is matched case-insensitively, otherwise the name is used as written.
    pub fn resolve_for_create(&self, path: &str) -> io::Result<PathBuf> {
        let c = components(path);
        let Some((last, dir)) = c.split_last() else {
            return Ok(self.root.clone());
        };
        let parent = self.resolve(&dir.join("/"))?;
        if let Ok(p) = self.resolve(path) {
            return Ok(p);
        }
        Ok(parent.join(last))
    }

    /// Drops the cached listing of `p`'s directory (after a create/remove).
    fn forget(&self, p: &Path) {
        if let Some(d) = p.parent() {
            self.dirs.lock().unwrap_or_else(|e| e.into_inner()).remove(d);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_case_insensitively() {
        let dir = std::env::temp_dir().join(format!("sa-platform-fs-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Data/Maps")).unwrap();
        std::fs::write(dir.join("Data/Maps/LAe.ide"), b"objs").unwrap();
        let fs = NativeFs::new(&dir).unwrap();
        assert_eq!(fs.read("DATA\\MAPS\\lae.IDE ").unwrap(), b"objs");
        let mut f = fs.open("data/maps/LAE.IDE").unwrap();
        assert_eq!(f.len(), 4);
        let mut b = [0u8; 2];
        f.read_at(2, &mut b).unwrap();
        assert_eq!(&b, b"js");
        assert!(!fs.exists("data/none"));
        fs.create_dir("DATA/User Files").unwrap();
        fs.write_file("data\\user files\\Save1.b", b"s").unwrap();
        assert_eq!(std::fs::read(dir.join("Data/User Files/Save1.b")).unwrap(), b"s");
        fs.write_file("DATA/USER FILES/SAVE1.B", b"t").unwrap();
        assert_eq!(fs.list_dir("data/user files").unwrap(), vec!["Save1.b"]);
        assert_eq!(fs.read("data/user files/save1.b").unwrap(), b"t");
        assert!(fs.is_dir("data/USER FILES"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
