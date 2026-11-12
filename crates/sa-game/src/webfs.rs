//! The game folder in the browser (wasm32): a read-only [`GameFs`] over files the page provides.
//!
//! The page registers every file of the user's own GTA San Andreas folder (chosen with a folder picker, or listed by
//! the local development server; never shipped) with its path relative to the folder and its size; reads go back to
//! the page synchronously (`saGameIo.read(id, offset, length)` on the worker's global scope: `FileReaderSync` on the
//! picked `File`, or a ranged synchronous request to the localhost server), with a block cache on the JS side.
//! Paths are matched case-insensitively with either separator, as the game spells them.

use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::Mutex;

use sa_platform::fs::{GameFs, RandomRead, components};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// `saGameIo.read(id, offset, length)`: the bytes `[offset, offset + length)` of file `id` (fewer at its end).
    #[wasm_bindgen(js_namespace = saGameIo, js_name = read)]
    fn io_read(id: u32, offset: f64, length: u32) -> js_sys::Uint8Array;
}

struct Entry {
    id: u32,
    len: u64,
}

#[derive(Default)]
struct Table {
    files: HashMap<String, Entry>,
    dirs: HashSet<String>,
}

static TABLE: Mutex<Option<Table>> = Mutex::new(None);

fn key(path: &str) -> String {
    components(path).join("/").to_ascii_lowercase()
}

/// Registers file `id` of the game folder at `path` (relative to the folder) with `len` bytes.
pub fn add(path: &str, id: u32, len: u64) {
    let mut t = TABLE.lock().unwrap_or_else(|e| e.into_inner());
    let t = t.get_or_insert_with(Table::default);
    let k = key(path);
    let mut d = k.as_str();
    while let Some(i) = d.rfind('/') {
        d = &d[..i];
        t.dirs.insert(d.to_owned());
    }
    t.dirs.insert(String::new());
    t.files.insert(k, Entry { id, len });
}

/// Files registered so far.
pub fn count() -> usize {
    TABLE.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map_or(0, |t| t.files.len())
}

fn lookup(path: &str) -> Option<(u32, u64)> {
    let t = TABLE.lock().unwrap_or_else(|e| e.into_inner());
    t.as_ref()?.files.get(&key(path)).map(|e| (e.id, e.len))
}

fn read_into(id: u32, offset: u64, buf: &mut [u8]) -> io::Result<()> {
    // the page answers in chunks of at most 16 MiB
    let mut done = 0usize;
    while done < buf.len() {
        let n = (buf.len() - done).min(16 << 20);
        let a = io_read(id, (offset + done as u64) as f64, n as u32);
        let got = a.length() as usize;
        if got < n {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "short read from the game folder"));
        }
        a.copy_to(&mut buf[done..done + n]);
        done += n;
    }
    Ok(())
}

/// The game folder as the page registered it.
pub struct WebFs;

/// Bytes a small read fetches ahead: the game reads its text data files a line (or a byte) at a time, and every
/// fetch crosses into JavaScript.
const READ_AHEAD: usize = 64 * 1024;

struct WebFile {
    id: u32,
    len: u64,
    /// the last window fetched: `[at, at + window.len())`
    at: u64,
    window: Vec<u8>,
}

impl RandomRead for WebFile {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        if offset + buf.len() as u64 > self.len {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "read past the end"));
        }
        if buf.len() >= READ_AHEAD {
            return read_into(self.id, offset, buf);
        }
        let end = offset + buf.len() as u64;
        if offset < self.at || end > self.at + self.window.len() as u64 {
            let n = (READ_AHEAD as u64).min(self.len - offset) as usize;
            self.window.resize(n, 0);
            read_into(self.id, offset, &mut self.window)?;
            self.at = offset;
        }
        let from = (offset - self.at) as usize;
        buf.copy_from_slice(&self.window[from..from + buf.len()]);
        Ok(())
    }

    fn len(&self) -> u64 {
        self.len
    }
}

fn not_found(path: &str) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("{path}: not in the game folder"))
}

impl GameFs for WebFs {
    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        let (id, len) = lookup(path).ok_or_else(|| not_found(path))?;
        let mut v = vec![0u8; len as usize];
        read_into(id, 0, &mut v)?;
        Ok(v)
    }

    fn open(&self, path: &str) -> io::Result<Box<dyn RandomRead>> {
        let (id, len) = lookup(path).ok_or_else(|| not_found(path))?;
        Ok(Box::new(WebFile { id, len, at: 0, window: Vec::new() }))
    }

    fn exists(&self, path: &str) -> bool {
        lookup(path).is_some() || self.is_dir(path)
    }

    fn is_dir(&self, path: &str) -> bool {
        let t = TABLE.lock().unwrap_or_else(|e| e.into_inner());
        t.as_ref().is_some_and(|t| t.dirs.contains(&key(path)))
    }

    fn list_dir(&self, path: &str) -> io::Result<Vec<String>> {
        let k = key(path);
        let t = TABLE.lock().unwrap_or_else(|e| e.into_inner());
        let t = t.as_ref().ok_or_else(|| not_found(path))?;
        if !t.dirs.contains(&k) {
            return Err(not_found(path));
        }
        let prefix = if k.is_empty() { String::new() } else { format!("{k}/") };
        let mut out: Vec<String> = t
            .files
            .keys()
            .map(String::as_str)
            .chain(t.dirs.iter().map(String::as_str))
            .filter_map(|p| p.strip_prefix(prefix.as_str()))
            .filter(|rest| !rest.is_empty() && !rest.contains('/'))
            .map(str::to_owned)
            .collect();
        out.sort();
        out.dedup();
        Ok(out)
    }
}
