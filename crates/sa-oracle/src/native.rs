//! Native backend: the same calls executed by the real CPU. A tiny i686 Windows program
//! (`native/ref.c` + `native/thunk.S`, built with `i686-w64-mingw32-gcc`) loads the flat image at
//! 0x400000, scratch at 0x10000000 and probe code at 0x10100000 (the Unicorn addresses), and runs a batch
//! of context blocks streamed over stdin/stdout under Wine.
//!
//! This is the reference for the x87 fidelity measurements and for functions routed away from Unicorn.
//! It has no guard rails: only calls that passed the Unicorn guard are sent (see `runner`).
//!
//! Wine prefix: `$SA_WINEPREFIX`, else `~/sa-engine/wine/prefix` (the project's lane-less prefix).

use crate::call::*;
use crate::pe::Image;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

const REF_C: &str = include_str!("../native/ref.c");
const THUNK_S: &str = include_str!("../native/thunk.S");

fn work_dir() -> PathBuf {
    crate::workspace_root().join("target/sa-oracle-native")
}

pub fn wine_prefix() -> PathBuf {
    if let Ok(p) = std::env::var("SA_WINEPREFIX") {
        return p.into();
    }
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("sa-engine/wine/prefix")
}

/// True when mingw and Wine are installed and the prefix exists.
pub fn available() -> bool {
    let ok = |c: &str| {
        Command::new(c)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    };
    ok("i686-w64-mingw32-gcc") && ok("wine") && wine_prefix().is_dir()
}

/// mingw's default PE linker script with a `.gta` section (ref.c's `gta_space`) placed first, at
/// 0x401000, so the PE loader reserves the game's address range for ref.exe.
fn linker_script() -> Result<String, String> {
    let out = Command::new("i686-w64-mingw32-ld")
        .arg("--verbose")
        .output()
        .map_err(|e| format!("ld: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut parts = text.split("==================================================");
    let script = parts.nth(1).ok_or("unexpected `ld --verbose` output")?;
    let text_start =
        "  .text  __image_base__ + ( __section_alignment__ < 0x1000 ? . : __section_alignment__ ) :";
    if !script.contains(text_start) {
        return Err("unexpected mingw linker script (no .text start line)".into());
    }
    Ok(script.replace(
        text_start,
        "  .gta __image_base__ + 0x1000 : { *(.gta) }\n  .text BLOCK(__section_alignment__) :",
    ))
}

/// Builds `ref.exe` (when its sources changed) and writes the flat image; returns (exe, image).
fn prepare(img: &Image) -> Result<(PathBuf, PathBuf), String> {
    let dir = work_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let (c, s, exe) = (dir.join("ref.c"), dir.join("thunk.S"), dir.join("ref.exe"));
    let stale = std::fs::read_to_string(&c).ok().as_deref() != Some(REF_C)
        || std::fs::read_to_string(&s).ok().as_deref() != Some(THUNK_S)
        || !exe.is_file();
    if stale {
        std::fs::write(&c, REF_C).map_err(|e| e.to_string())?;
        std::fs::write(&s, THUNK_S).map_err(|e| e.to_string())?;
        let ld = dir.join("ref.ld");
        std::fs::write(&ld, linker_script()?).map_err(|e| e.to_string())?;
        let out = Command::new("i686-w64-mingw32-gcc")
            .args(["-O2", "-Wall", "-static", "-o"])
            .arg(&exe)
            .arg(&c)
            .arg(&s)
            // linked at the game's base with the game's range as the first section (see ref.c)
            .args([
                "-Wl,--image-base=0x400000",
                "-Wl,--disable-dynamicbase",
                "-Wl,--disable-reloc-section",
            ])
            .arg(format!("-Wl,-T,{}", ld.display()))
            .output()
            .map_err(|e| format!("i686-w64-mingw32-gcc: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "building ref.exe failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }
    // One file per image content (specs may patch the image), so concurrent runs never share a file
    // whose content changes; written to a temporary name and renamed.
    let h = img.flat.iter().fold(0xCBF2_9CE4_8422_2325u64, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x100_0000_01B3)
    });
    let image = dir.join(format!("image-{h:016x}.bin"));
    if !std::fs::metadata(&image).is_ok_and(|m| m.len() as usize == img.flat.len()) {
        let tmp = dir.join(format!("image-{h:016x}.{}.tmp", tmp_tag()));
        std::fs::write(&tmp, &img.flat).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &image).map_err(|e| e.to_string())?;
    }
    Ok((exe, image))
}

/// A temporary-file tag unique per process and call.
fn tmp_tag() -> String {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{}-{n}", std::process::id())
}

/// FNV-1a 64 (file names by content).
fn fnv(b: &[u8]) -> u64 {
    b.iter().fold(0xCBF2_9CE4_8422_2325u64, |h, &x| {
        (h ^ x as u64).wrapping_mul(0x100_0000_01B3)
    })
}

fn winpath(p: &std::path::Path) -> String {
    format!("Z:{}", p.display()).replace('/', "\\")
}

/// Runs `calls` on the real CPU. `probes` is loaded at [`PROBE`] first.
pub fn run(img: &Image, calls: &[Call], probes: Option<&[u8]>) -> Result<Vec<Outcome>, String> {
    let (exe, image) = prepare(img)?;
    let probe_arg = match probes {
        Some(code) => {
            // one file per content, written under a temporary name: concurrent runs never share it
            let p = work_dir().join(format!("probes-{:016x}.bin", fnv(code)));
            let tmp = work_dir().join(format!("probes.{}.tmp", tmp_tag()));
            std::fs::write(&tmp, code).map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, &p).map_err(|e| e.to_string())?;
            winpath(&p)
        }
        None => "-".into(),
    };
    let mut child = Command::new("wine")
        .arg(&exe)
        .arg(winpath(&image))
        .arg(probe_arg)
        .args(["-", "-"])
        .env("WINEPREFIX", wine_prefix())
        .env("WINEDEBUG", "-all")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("wine: {e}"))?;
    let mut stdin = child.stdin.take().unwrap();
    let input: Vec<u8> = calls
        .iter()
        .flat_map(|c| {
            let mut r = (c.scratch.len() as u32).to_le_bytes().to_vec();
            r.extend_from_slice(&c.ctx());
            r.extend_from_slice(&c.scratch);
            r
        })
        .collect();
    let writer = std::thread::spawn(move || {
        let r = stdin.write_all(&input);
        drop(stdin);
        r
    });
    let mut out = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut out)
        .map_err(|e| e.to_string())?;
    let mut err = String::new();
    child.stderr.take().unwrap().read_to_string(&mut err).ok();
    let status = child.wait().map_err(|e| e.to_string())?;
    let written = writer.join().map_err(|_| "writer thread panicked".to_string())?;
    let want: usize = calls.iter().map(|c| 4 + CTX_LEN + c.scratch.len()).sum();
    if let Err(e) = written {
        return Err(format!("ref.exe: status {status}, stdin: {e}\n{}", err.trim()));
    }
    if !status.success() || out.len() != want {
        return Err(format!(
            "ref.exe: status {status}, {} of {} bytes\n{}",
            out.len(),
            want,
            err.trim()
        ));
    }
    let mut res = Vec::with_capacity(calls.len());
    let mut o = 0;
    for c in calls {
        let n = c.scratch.len();
        let r = &out[o + 4..o + 4 + CTX_LEN + n];
        res.push(Outcome::from_ctx(&r[..CTX_LEN], &r[CTX_LEN..]));
        o += 4 + CTX_LEN + n;
    }
    Ok(res)
}
