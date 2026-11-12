//! samp.dll's loading-screen patches in samp mode (docs/samp/startup.md).
//!
//! samp.dll serves files from samp.saa by hooking `CreateFileA`; its `loadscs.txd` holds one texture, `title` (the
//! SA-MP logo screen). It also renames the splash textures `CLoadingScreen::LoadSplashes` asks for: the strings
//! "title_pc_US" (0x866CD8) and "loadsc%d" (0x866CCC) become "title", so every slide is samp's logo. The engine
//! does the same at the same places: the strings are rewritten in the mapped image, and `sa_platform::vfs`'s
//! redirect (where file opens reach the platform, the native counterpart of `CreateFileA`) serves samp.saa's
//! `loadscs.txd`. samp.saa is read at run time from the user's SA:MP install (the game folder, or `SA_SAMP_DIR`
//! natively) and never shipped. Only `loadscs.txd` is served by default (main.scm and script.img with
//! `SA_SAMP_SCM=1`); the other entries (gta.dat, the IDE files ...) need samp's `SAMP` folder in the engine's
//! file layer first.

use std::sync::{Arc, OnceLock};

/// One served entry: its lower-case basename (samp's lookup key) and its bytes.
type Served = (&'static [u8], Arc<Vec<u8>>);

/// The served entries.
static SERVED: OnceLock<Vec<Served>> = OnceLock::new();

/// The entries served from samp.saa (subset of samp's, see the module comment).
const ENTRIES: [&str; 1] = ["loadscs.txd"];

/// samp's own scripts, served only with `SA_SAMP_SCM=1` (exploration): samp's main.scm starts threads that
/// reach unported code (the script search lights, docs/samp/startup.md), so GTA's main.scm stays the default.
const SCRIPT_ENTRIES: [&str; 2] = ["main.scm", "script.img"];

/// samp.dll's string patches: (address, original bytes, samp's bytes).
const STRING_PATCHES: [(usize, &[u8], &[u8]); 2] = [
    (0x866CD8, b"title_pc_US\0", b"title\0"),
    (0x866CCC, b"loadsc%d\0", b"title\0"),
];

fn read_saa() -> Option<Vec<u8>> {
    // The game folder first (where samp's installer puts it), through the game's own file layer.
    if let Ok(d) = crate::bind::with("samp.saa", |p| p.vfs.read("samp.saa")) {
        return Some(d);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(dir) = std::env::var_os("SA_SAMP_DIR") {
        return std::fs::read(std::path::Path::new(&dir).join("samp.saa")).ok();
    }
    None
}

fn redirect(path: &str) -> Option<Arc<Vec<u8>>> {
    let key = samp_saa::lookup_key(path.as_bytes());
    SERVED
        .get()?
        .iter()
        .find(|(k, _)| *k == key.as_slice())
        .map(|(_, d)| d.clone())
}

/// Applies the loading-screen patches when samp.saa is found; returns what was done, for the log.
pub fn install() -> String {
    let Some(data) = read_saa() else {
        return "samp.saa not found (game folder, SA_SAMP_DIR): GTA's own loading screen".into();
    };
    let archive = match samp_saa::SaaArchive::from_bytes(data) {
        Ok(a) => a,
        Err(e) => return format!("samp.saa: {e:?}: GTA's own loading screen"),
    };
    let scripts = crate::env::var_os("SA_SAMP_SCM").is_some();
    let served: Vec<_> = ENTRIES
        .iter()
        .chain(SCRIPT_ENTRIES.iter().filter(|_| scripts))
        .filter_map(|n| archive.read(n).map(|d| (n.as_bytes(), Arc::new(d))))
        .collect();
    if served.is_empty() {
        return "samp.saa has no loadscs.txd: GTA's own loading screen".into();
    }
    let _ = SERVED.set(served);
    for (addr, original, patched) in STRING_PATCHES {
        // gta_sa.exe's .rdata, mapped by `arena::map_image` before the host installs; like samp-rs's patch
        // table, only over the original bytes.
        match crate::arena::patch_image(addr, original, patched) {
            Ok(true) => {}
            Ok(false) => return format!("samp.saa: splash name at {addr:#x} is not the original; not patched"),
            Err(e) => return format!("samp.saa: splash name at {addr:#x}: {e}"),
        }
    }
    sa_platform::vfs::set_redirect(Some(redirect));
    let what = if scripts { "loadscs.txd, main.scm, script.img" } else { "loadscs.txd" };
    format!("samp.saa: {what} served, splash names \"title\" (samp's loading screen)")
}
