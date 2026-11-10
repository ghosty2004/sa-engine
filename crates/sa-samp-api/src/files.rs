//! samp.saa file redirection and the user folder.
//!
//! samp serves every game file whose lower-case basename matches a samp.saa entry from memory. samp-rs today
//! patches gta_sa.exe's kernel32 import slots (`filehook.rs`) and keeps the rules host-testable in
//! `saafs.rs`. Here the rules stay in samp-rs and the backend asks a [`FileProvider`] before it opens a
//! file: the original backend from its kernel32 hooks, the engines from their `sa_platform::fs::GameFs`
//! overlay. Mapping: `docs/samp/game-adapter.md` §4.11.

/// A file served from memory.
pub trait FileSource: Send {
    /// Total size in bytes.
    fn len(&self) -> u64;
    /// Whether the file is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Reads at `offset` into `buf`; returns the bytes read (0 at the end).
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize>;
}

/// Answers "is this file replaced?" for the game. Called from the game thread and from the streaming
/// thread (IMG reads), so it is `Send + Sync`.
pub trait FileProvider: Send + Sync {
    /// Opens a replacement for `path` (game-relative, any case, `\` or `/`), or `None` to use the disk.
    fn open(&self, path: &str) -> Option<Box<dyn FileSource>>;
}

/// Where the game keeps the user's files (`gta_user_dir_path` 0xC92368: screenshots, `sa-mp.cfg`,
/// `chatlog.txt`), as a host path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserDir(pub String);
