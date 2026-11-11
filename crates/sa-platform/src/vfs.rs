//! The Windows file namespace and the file APIs the ported code calls (CRT stdio for `CFileMgr`,
//! Win32 handles for `CdStream`), on [`GameFs`] trees.
//!
//! The original works with Windows paths: `CFileMgr::Initialise` stores `_getcwd()` (the game
//! folder) as its root, `ChangeDir` `_chdir`s into subfolders, data files are opened relative to
//! the current directory, and the user-files folder is `<My Documents>\GTA San Andreas User Files`
//! (`SHGetFolderPathA(CSIDL_PERSONAL)` in `CFileMgr::InitUserDirectories`). Phase B keeps those
//! strings: [`Vfs`] mounts the user's install at [`GAME_ROOT`] and a writable per-user folder at
//! [`DOCUMENTS`], keeps a current directory, and resolves every path case-insensitively through the
//! mounted trees. [`FileTable`] holds the open files behind the integer handles the game stores
//! (`FILE*` values for stdio, `HANDLE`s for Win32), with MSVC CRT semantics where the game can
//! observe them (text-mode CR/LF translation, `fgets`, `feof`/error flags, `ftell`).

use std::io;

use crate::fs::{GameFs, RandomRead, components};

/// Where the game folder appears in the Windows namespace (what `_getcwd` reports at start).
pub const GAME_ROOT: &str = "C:\\Program Files\\Rockstar Games\\GTA San Andreas";
/// `SHGetFolderPathA(CSIDL_PERSONAL)`.
pub const DOCUMENTS: &str = "C:\\Users\\Player\\Documents";

struct Mount {
    /// Lower-cased components of the Windows prefix, drive first (`c:`).
    prefix: Vec<String>,
    fs: Box<dyn GameFs>,
}

/// A tree with every write refused.
struct ReadOnly(Box<dyn GameFs>);

impl GameFs for ReadOnly {
    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        self.0.read(path)
    }
    fn open(&self, path: &str) -> io::Result<Box<dyn RandomRead>> {
        self.0.open(path)
    }
    fn exists(&self, path: &str) -> bool {
        self.0.exists(path)
    }
    fn is_dir(&self, path: &str) -> bool {
        self.0.is_dir(path)
    }
    fn list_dir(&self, path: &str) -> io::Result<Vec<String>> {
        self.0.list_dir(path)
    }
}

/// The Windows file namespace of the game process.
pub struct Vfs {
    mounts: Vec<Mount>,
    /// Current directory, absolute, without a trailing `\` (`C:\…\GTA San Andreas`).
    cwd: String,
}

/// Lower-cased absolute components of `path` relative to `cwd` (drive first), `.`/`..` applied.
fn absolute(cwd: &str, path: &str) -> Vec<String> {
    let p = path.trim();
    let bytes = p.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let rest = if bytes.len() >= 2 && bytes[1] == b':' {
        // drive-absolute (a drive-relative "C:foo" is treated as absolute; the game never uses it)
        out.push(p[..2].to_ascii_lowercase());
        &p[2..]
    } else {
        let base = components(cwd);
        if p.starts_with(['\\', '/']) {
            out.extend(base.first().map(|d| d.to_ascii_lowercase()));
        } else {
            out.extend(base.iter().map(|c| c.to_ascii_lowercase()));
        }
        p
    };
    for c in components(rest) {
        if c == ".." {
            if out.len() > 1 {
                out.pop();
            }
        } else {
            out.push(c.to_ascii_lowercase());
        }
    }
    out
}

impl Vfs {
    /// The game folder at [`GAME_ROOT`] (current directory) and the per-user documents folder
    /// (`documents`, writable) at [`DOCUMENTS`].
    pub fn new(game: Box<dyn GameFs>, documents: Box<dyn GameFs>) -> Vfs {
        let mut v = Vfs {
            mounts: Vec::new(),
            cwd: GAME_ROOT.to_owned(),
        };
        // the user's install is never written to
        v.mount(GAME_ROOT, Box::new(ReadOnly(game)));
        v.mount(DOCUMENTS, documents);
        v
    }

    /// Mounts `fs` at the Windows directory `prefix` (longest prefix wins).
    pub fn mount(&mut self, prefix: &str, fs: Box<dyn GameFs>) {
        self.mounts.push(Mount {
            prefix: absolute("C:", prefix),
            fs,
        });
        self.mounts.sort_by_key(|m| std::cmp::Reverse(m.prefix.len()));
    }

    /// `_getcwd`: the current directory (no trailing `\`, like the CRT except at a drive root).
    pub fn getcwd(&self) -> String {
        self.cwd.clone()
    }

    /// `_chdir`: 0 on success, -1 if the directory does not exist (`ENOENT`).
    pub fn chdir(&mut self, path: &str) -> i32 {
        let abs = absolute(&self.cwd, path);
        if !self.is_dir_abs(&abs) {
            return -1;
        }
        self.cwd = self.display(&abs);
        0
    }

    /// The Windows spelling of absolute components: mount prefixes keep their configured case,
    /// the rest is lower case (Windows paths compare case-insensitively).
    fn display(&self, abs: &[String]) -> String {
        if let Some(m) = self.mounts.iter().find(|m| abs.starts_with(&m.prefix)) {
            let base = if m.prefix == absolute("C:", GAME_ROOT) {
                GAME_ROOT.to_owned()
            } else if m.prefix == absolute("C:", DOCUMENTS) {
                DOCUMENTS.to_owned()
            } else {
                m.prefix.join("\\").to_ascii_uppercase()
            };
            let rest = &abs[m.prefix.len()..];
            if rest.is_empty() {
                base
            } else {
                format!("{base}\\{}", rest.join("\\"))
            }
        } else if abs.len() == 1 {
            format!("{}\\", abs[0].to_ascii_uppercase())
        } else {
            abs.join("\\")
        }
    }

    fn locate(&self, abs: &[String]) -> Option<(&dyn GameFs, String)> {
        let m = self.mounts.iter().find(|m| abs.starts_with(&m.prefix))?;
        Some((m.fs.as_ref(), abs[m.prefix.len()..].join("/")))
    }

    fn is_dir_abs(&self, abs: &[String]) -> bool {
        // every prefix of a mount point exists as a directory
        if self.mounts.iter().any(|m| m.prefix.starts_with(abs)) {
            return true;
        }
        self.locate(abs).is_some_and(|(fs, p)| fs.is_dir(&p))
    }

    /// Resolves a Windows path to (tree, path inside the tree).
    pub fn resolve(&self, path: &str) -> io::Result<(&dyn GameFs, String)> {
        let abs = absolute(&self.cwd, path);
        self.locate(&abs)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{path}: outside every mount")))
    }

    pub fn exists(&self, path: &str) -> bool {
        let abs = absolute(&self.cwd, path);
        self.is_dir_abs(&abs) || self.locate(&abs).is_some_and(|(fs, p)| fs.exists(&p))
    }

    pub fn is_dir(&self, path: &str) -> bool {
        self.is_dir_abs(&absolute(&self.cwd, path))
    }

    /// `_mkdir`/`CreateDirectoryA`.
    pub fn mkdir(&self, path: &str) -> io::Result<()> {
        let (fs, p) = self.resolve(path)?;
        fs.create_dir(&p)
    }

    pub fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        let (fs, p) = self.resolve(path)?;
        fs.read(&p)
    }

    pub fn write(&self, path: &str, data: &[u8]) -> io::Result<()> {
        let (fs, p) = self.resolve(path)?;
        fs.write_file(&p, data)
    }

    pub fn remove(&self, path: &str) -> io::Result<()> {
        let (fs, p) = self.resolve(path)?;
        fs.remove_file(&p)
    }

    fn open_read(&self, path: &str) -> io::Result<Box<dyn RandomRead>> {
        let (fs, p) = self.resolve(path)?;
        fs.open(&p)
    }
}

// --------------------------------------------------------------------------- open files ---

/// A parsed `fopen` mode string (MSVC: `r w a`, optional `+`, `b`/`t`; text is the default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenMode {
    pub read: bool,
    pub write: bool,
    pub append: bool,
    pub truncate: bool,
    pub create: bool,
    pub text: bool,
}

impl OpenMode {
    pub fn parse(mode: &str) -> Option<OpenMode> {
        let mut c = mode.chars();
        let mut m = match c.next()? {
            'r' => OpenMode {
                read: true,
                write: false,
                append: false,
                truncate: false,
                create: false,
                text: true,
            },
            'w' => OpenMode {
                read: false,
                write: true,
                append: false,
                truncate: true,
                create: true,
                text: true,
            },
            'a' => OpenMode {
                read: false,
                write: true,
                append: true,
                truncate: false,
                create: true,
                text: true,
            },
            _ => return None,
        };
        for ch in c {
            match ch {
                '+' => {
                    m.read = true;
                    m.write = true;
                }
                'b' => m.text = false,
                't' => m.text = true,
                // MSVC extensions (c, n, S, R, T, D) do not change the data
                _ => {}
            }
        }
        Some(m)
    }
}

enum Body {
    /// Read-only, streamed from the tree (the IMG archives are ~1 GB).
    Stream(Box<dyn RandomRead>),
    /// Read/write, in memory; written back on flush/close.
    Buffer {
        data: Vec<u8>,
        path: String,
        dirty: bool,
    },
}

struct OpenFile {
    body: Body,
    pos: u64,
    mode: OpenMode,
    eof: bool,
    error: bool,
}

impl OpenFile {
    fn len(&self) -> u64 {
        match &self.body {
            Body::Stream(r) => r.len(),
            Body::Buffer { data, .. } => data.len() as u64,
        }
    }

    fn read_raw(&mut self, out: &mut [u8]) -> usize {
        let len = self.len();
        let n = (len.saturating_sub(self.pos)).min(out.len() as u64) as usize;
        let ok = match &mut self.body {
            Body::Stream(r) => r.read_at(self.pos, &mut out[..n]).is_ok(),
            Body::Buffer { data, .. } => {
                out[..n].copy_from_slice(&data[self.pos as usize..self.pos as usize + n]);
                true
            }
        };
        if !ok {
            self.error = true;
            return 0;
        }
        self.pos += n as u64;
        n
    }
}

/// Seek origins (`SEEK_SET`, `SEEK_CUR`, `SEEK_END`; `FILE_BEGIN`… have the same values).
pub const SEEK_SET: i32 = 0;
pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;

/// Open files behind integer handles. Handles start at [`FileTable::FIRST_HANDLE`] and are
/// multiples of 4 (like real `FILE*`/`HANDLE` values, so code that tests them against 0 or
/// `INVALID_HANDLE_VALUE` behaves).
#[derive(Default)]
pub struct FileTable {
    files: Vec<Option<OpenFile>>,
}

impl FileTable {
    pub const FIRST_HANDLE: u32 = 0x1000;

    fn handle_of(i: usize) -> u32 {
        Self::FIRST_HANDLE + 4 * i as u32
    }

    fn get(&mut self, h: u32) -> Option<&mut OpenFile> {
        let i = h.checked_sub(Self::FIRST_HANDLE)?;
        if i % 4 != 0 {
            return None;
        }
        self.files.get_mut((i / 4) as usize)?.as_mut()
    }

    fn insert(&mut self, f: OpenFile) -> u32 {
        let i = match self.files.iter().position(Option::is_none) {
            Some(i) => {
                self.files[i] = Some(f);
                i
            }
            None => {
                self.files.push(Some(f));
                self.files.len() - 1
            }
        };
        Self::handle_of(i)
    }

    /// Number of open files.
    pub fn open_count(&self) -> usize {
        self.files.iter().flatten().count()
    }

    /// `fopen(path, mode)`: a handle, or `None` (the CRT returns NULL).
    pub fn fopen(&mut self, vfs: &Vfs, path: &str, mode: &str) -> Option<u32> {
        let m = OpenMode::parse(mode)?;
        let body = if !m.write {
            Body::Stream(vfs.open_read(path).ok()?)
        } else {
            let (fs, p) = vfs.resolve(path).ok()?;
            let data = if m.truncate {
                Vec::new()
            } else {
                match fs.read(&p) {
                    Ok(d) => d,
                    Err(_) if m.create => Vec::new(),
                    Err(_) => return None,
                }
            };
            // creating the file now makes a failing (read-only) target fail at fopen, like the CRT
            if m.truncate || !fs.exists(&p) {
                fs.write_file(&p, &data).ok()?;
            }
            Body::Buffer {
                data,
                path: path.to_owned(),
                dirty: false,
            }
        };
        let mut f = OpenFile {
            body,
            pos: 0,
            mode: m,
            eof: false,
            error: false,
        };
        if m.append {
            f.pos = f.len();
        }
        Some(self.insert(f))
    }

    /// `fread(buf, 1, n, f)` / `ReadFile`: bytes read. Text mode drops the `\r` of `\r\n` pairs
    /// and stops at Ctrl-Z, as the MSVC CRT does; a short read sets the EOF flag.
    pub fn read(&mut self, h: u32, out: &mut [u8]) -> usize {
        let Some(f) = self.get(h) else { return 0 };
        if !f.mode.read {
            f.error = true;
            return 0;
        }
        if !f.mode.text {
            let n = f.read_raw(out);
            if n < out.len() {
                f.eof = true;
            }
            return n;
        }
        let mut n = 0;
        let mut byte = [0u8; 1];
        while n < out.len() {
            if f.read_raw(&mut byte) == 0 {
                f.eof = true;
                break;
            }
            match byte[0] {
                0x1A => {
                    f.pos -= 1;
                    f.eof = true;
                    break;
                }
                b'\r' => {
                    let mut next = [0u8; 1];
                    out[n] = match f.read_raw(&mut next) {
                        1 if next[0] == b'\n' => b'\n',
                        1 => {
                            f.pos -= 1;
                            b'\r'
                        }
                        // `\r` at the end of the file stays a `\r`
                        _ => b'\r',
                    };
                }
                b => out[n] = b,
            }
            n += 1;
        }
        n
    }

    /// `fgets(buf, n, f)`: at most `n - 1` bytes, up to and including `\n`, NUL-terminated.
    /// `None` when nothing could be read (EOF or error), like the CRT's NULL.
    pub fn gets(&mut self, h: u32, n: usize) -> Option<Vec<u8>> {
        let mut line = Vec::new();
        let mut b = [0u8; 1];
        while line.len() + 1 < n {
            if self.read(h, &mut b) == 0 {
                break;
            }
            line.push(b[0]);
            if b[0] == b'\n' {
                break;
            }
        }
        if line.is_empty() && n > 1 {
            return None;
        }
        line.push(0);
        Some(line)
    }

    /// `fwrite(buf, 1, n, f)` / `WriteFile`: bytes written (text mode writes `\n` as `\r\n`).
    pub fn write(&mut self, h: u32, src: &[u8]) -> usize {
        let Some(f) = self.get(h) else { return 0 };
        if !f.mode.write {
            f.error = true;
            return 0;
        }
        if f.mode.append {
            f.pos = f.len();
        }
        let text = f.mode.text;
        let Body::Buffer { data, dirty, .. } = &mut f.body else {
            f.error = true;
            return 0;
        };
        let mut bytes = Vec::with_capacity(src.len());
        for &b in src {
            if text && b == b'\n' {
                bytes.push(b'\r');
            }
            bytes.push(b);
        }
        let pos = f.pos as usize;
        if data.len() < pos {
            data.resize(pos, 0);
        }
        let end = pos + bytes.len();
        if data.len() < end {
            data.resize(end, 0);
        }
        data[pos..end].copy_from_slice(&bytes);
        *dirty = true;
        f.pos = end as u64;
        src.len()
    }

    /// `fseek`/`SetFilePointer`: 0 on success, -1 on a bad origin or a negative position. Clears
    /// the EOF flag.
    pub fn seek(&mut self, h: u32, offset: i64, origin: i32) -> i32 {
        let Some(f) = self.get(h) else { return -1 };
        let base = match origin {
            SEEK_SET => 0,
            SEEK_CUR => f.pos as i64,
            SEEK_END => f.len() as i64,
            _ => return -1,
        };
        let p = base + offset;
        if p < 0 {
            return -1;
        }
        f.pos = p as u64;
        f.eof = false;
        0
    }

    /// `ftell`: the byte position (MSVC reports the file offset, also in text mode).
    pub fn tell(&mut self, h: u32) -> i64 {
        self.get(h).map_or(-1, |f| f.pos as i64)
    }

    /// The file size (`GetFileSize`, or the RW `rwfseek(END)`/`rwftell` pair).
    pub fn size(&mut self, h: u32) -> Option<u64> {
        self.get(h).map(|f| f.len())
    }

    pub fn eof(&mut self, h: u32) -> bool {
        self.get(h).is_some_and(|f| f.eof)
    }

    pub fn error(&mut self, h: u32) -> bool {
        self.get(h).is_some_and(|f| f.error)
    }

    /// Reads `out.len()` bytes at `offset` without moving the position (overlapped `ReadFile`).
    pub fn read_at(&mut self, h: u32, offset: u64, out: &mut [u8]) -> usize {
        let Some(f) = self.get(h) else { return 0 };
        let saved = f.pos;
        f.pos = offset;
        let n = f.read_raw(out);
        f.pos = saved;
        n
    }

    /// Writes buffered changes back (`fflush`).
    pub fn flush(&mut self, vfs: &Vfs, h: u32) -> io::Result<()> {
        let Some(f) = self.get(h) else {
            return Err(io::ErrorKind::InvalidInput.into());
        };
        if let Body::Buffer { data, path, dirty } = &mut f.body
            && *dirty
        {
            vfs.write(path, data)?;
            *dirty = false;
        }
        Ok(())
    }

    /// `fclose`/`CloseHandle`: 0, or -1 (EOF) for an unknown handle or a failed write-back.
    pub fn close(&mut self, vfs: &Vfs, h: u32) -> i32 {
        if self.get(h).is_none() {
            return -1;
        }
        let r = self.flush(vfs, h);
        let i = ((h - Self::FIRST_HANDLE) / 4) as usize;
        self.files[i] = None;
        if r.is_ok() { 0 } else { -1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MemFs;

    fn vfs() -> Vfs {
        let mut game = MemFs::default();
        game.insert(
            "DATA/gta.dat",
            b"IDE DATA\\DEFAULT.IDE\r\nIMG MODELS\\GTA_INT.IMG\r\n\x1Atail".to_vec(),
        );
        game.insert("models/gta3.img", vec![7; 64]);
        Vfs::new(Box::new(game), Box::new(MemFs::default()))
    }

    #[test]
    fn cwd_and_paths() {
        let mut v = vfs();
        assert_eq!(v.getcwd(), GAME_ROOT);
        assert!(v.exists("data\\GTA.DAT"));
        assert_eq!(v.chdir("DATA\\"), 0);
        assert!(v.getcwd().ends_with("GTA San Andreas\\data"));
        assert!(v.exists("gta.dat") && v.exists("..\\MODELS\\gta3.img"));
        assert_eq!(v.chdir("\\nowhere"), -1);
        assert_eq!(v.chdir(&format!("{GAME_ROOT}\\")), 0);
        assert_eq!(v.getcwd(), GAME_ROOT);
        assert!(v.is_dir("C:\\Users") && v.is_dir(DOCUMENTS));
        let user = format!("{DOCUMENTS}\\GTA San Andreas User Files");
        assert!(!v.exists(&user));
        v.mkdir(&user).unwrap();
        assert!(v.is_dir(&user.to_uppercase()));
    }

    #[test]
    fn text_mode_reads() {
        let v = vfs();
        let mut t = FileTable::default();
        let h = t.fopen(&v, "data\\gta.dat", "r").unwrap();
        assert_eq!(t.gets(h, 256).unwrap(), b"IDE DATA\\DEFAULT.IDE\n\0");
        assert_eq!(t.gets(h, 5).unwrap(), b"IMG \0");
        assert_eq!(t.gets(h, 256).unwrap(), b"MODELS\\GTA_INT.IMG\n\0");
        assert!(t.gets(h, 256).is_none() && t.eof(h));
        assert_eq!(t.close(&v, h), 0);
        let h = t.fopen(&v, "data\\gta.dat", "rb").unwrap();
        let mut b = [0u8; 100];
        assert_eq!(t.read(h, &mut b), 51);
        assert_eq!(t.tell(h), 51);
        assert_eq!(t.seek(h, -4, SEEK_END), 0);
        assert_eq!(t.read(h, &mut b[..4]), 4);
        assert_eq!(&b[..4], b"tail");
        assert!(t.fopen(&v, "data\\none.dat", "rb").is_none());
        assert!(
            t.fopen(&v, "data\\gta.dat", "wb").is_none(),
            "the game folder is read-only"
        );
    }

    #[test]
    fn user_files_round_trip() {
        let v = vfs();
        let mut t = FileTable::default();
        let p = format!("{DOCUMENTS}\\gta_sa.set");
        let h = t.fopen(&v, &p, "wb").unwrap();
        assert_eq!(t.write(h, b"abc"), 3);
        assert_eq!(t.close(&v, h), 0);
        let h = t.fopen(&v, &p, "ab").unwrap();
        t.write(h, b"d\n");
        t.close(&v, h);
        assert_eq!(v.read(&p).unwrap(), b"abcd\n");
        let h = t.fopen(&v, &p, "w").unwrap();
        t.write(h, b"x\ny");
        t.close(&v, h);
        assert_eq!(v.read(&p).unwrap(), b"x\r\ny");
        assert_eq!(t.open_count(), 0);
    }
}
