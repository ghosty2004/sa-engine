//! The hook table: one entry per original function, keyed by its address in gta_sa.exe 1.0 US.
//!
//! Every entry names the function (catalogue `owner::name`), lists the entry bytes that must be found
//! before patching (so a wrong exe or a foreign patch is never overwritten), and points at its Rust
//! implementation. Runtime state (original entry, mode, call counters) lives in parallel atomics.

use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

/// How the entry is patched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Ordinary function: entry bytes are moved into a trampoline (relocated), the entry gets `jmp`.
    Inline,
    /// One of the 501 `.HOODLUM` stubs (`jmp rel32` at the function address): the stub is retargeted,
    /// and "original" is the stub's old target.
    HoodlumStub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id {
    TimerUpdate = 0,
    LimitRadianAngle = 1,
    IsCoopGameGoingOn = 2,
}

pub struct HookSpec {
    pub id: Id,
    pub addr: u32,
    pub name: &'static str,
    pub kind: Kind,
    /// Exact bytes expected at `addr` (at least the stolen range).
    pub expect: &'static [u8],
    /// Address of the Rust implementation (original calling convention).
    pub rust: fn() -> usize,
    /// The thunk calls the frame-boundary handler before dispatching (one per frame).
    pub frame_boundary: bool,
    /// Mode at attach.
    pub start_rust: bool,
}

pub static HOOKS: &[HookSpec] = &[
    HookSpec {
        id: Id::TimerUpdate,
        addr: 0x561B10,
        name: "CTimer::Update",
        kind: Kind::Inline,
        // mov ecx,[0xB7CB28]; sub esp,0xC
        expect: &[0x8B, 0x0D, 0x28, 0xCB, 0xB7, 0x00, 0x83, 0xEC, 0x0C],
        rust: || crate::game::timer_update_hook as extern "C" fn() as usize,
        frame_boundary: true,
        start_rust: true,
    },
    HookSpec {
        id: Id::LimitRadianAngle,
        addr: 0x53CB50,
        name: "CGeneral::LimitRadianAngle",
        kind: Kind::Inline,
        // fld dword [esp+4]; fld dword [0x863AC0]
        expect: &[0xD9, 0x44, 0x24, 0x04, 0xD9, 0x05, 0xC0, 0x3A, 0x86, 0x00],
        rust: || crate::game::limit_radian_angle_hook as extern "C" fn(f32) -> f32 as usize,
        frame_boundary: false,
        start_rust: true,
    },
    HookSpec {
        id: Id::IsCoopGameGoingOn,
        addr: 0x441390,
        name: "CGameLogic::IsCoopGameGoingOn",
        kind: Kind::HoodlumStub,
        // jmp 0x156CF30
        expect: &[0xE9, 0x9B, 0xBB, 0x12, 0x01],
        rust: || crate::game::is_coop_game_going_on_hook as extern "C" fn() -> u32 as usize,
        frame_boundary: false,
        start_rust: true,
    },
];

const N: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Original = 1,
    Rust = 2,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Original => "orig",
            Mode::Rust => "rust",
        }
    }
    fn from_u8(v: u8) -> Option<Mode> {
        match v {
            1 => Some(Mode::Original),
            2 => Some(Mode::Rust),
            _ => None,
        }
    }
}

#[allow(clippy::declare_interior_mutable_const)]
const Z32: AtomicU32 = AtomicU32::new(0);
#[allow(clippy::declare_interior_mutable_const)]
const Z8: AtomicU8 = AtomicU8::new(0);

/// Original entry per hook: the trampoline (inline) or the old stub target (`.HOODLUM`).
static ORIGINAL: [AtomicU32; N] = [Z32; N];
/// What the thunk's `jmp [TARGET+i*4]` jumps to: the Rust implementation or `ORIGINAL`.
pub static TARGET: [AtomicU32; N] = [Z32; N];
/// Requested mode, applied at the next frame boundary (0 = nothing pending).
static PENDING: [AtomicU8; N] = [Z8; N];
static CALLS: [AtomicU32; N] = [Z32; N];
static INSTALLED: [AtomicU8; N] = [Z8; N];

pub fn spec(id: Id) -> &'static HookSpec {
    &HOOKS[id as usize]
}

pub fn original(id: Id) -> u32 {
    ORIGINAL[id as usize].load(Ordering::Acquire)
}

pub fn count(id: Id) {
    CALLS[id as usize].fetch_add(1, Ordering::Relaxed);
}

pub fn calls(i: usize) -> u32 {
    CALLS[i].load(Ordering::Relaxed)
}

pub fn installed(i: usize) -> bool {
    INSTALLED[i].load(Ordering::Acquire) != 0
}

/// Address of `TARGET[i]` (embedded into the thunk's indirect jmp).
pub fn target_slot(i: usize) -> usize {
    &TARGET[i] as *const AtomicU32 as usize
}

/// Called once per hook by the installer before the entry is patched.
pub fn set_installed(i: usize, original: u32, mode: Mode) {
    ORIGINAL[i].store(original, Ordering::Release);
    TARGET[i].store(target_for(i, mode), Ordering::Release);
    INSTALLED[i].store(1, Ordering::Release);
}

fn target_for(i: usize, mode: Mode) -> u32 {
    match mode {
        Mode::Rust => (HOOKS[i].rust)() as u32,
        Mode::Original => ORIGINAL[i].load(Ordering::Acquire),
    }
}

pub fn mode(i: usize) -> Mode {
    if TARGET[i].load(Ordering::Acquire) == ORIGINAL[i].load(Ordering::Acquire) {
        Mode::Original
    } else {
        Mode::Rust
    }
}

pub fn request(i: usize, mode: Mode) {
    PENDING[i].store(mode as u8, Ordering::Release);
}

pub fn pending(i: usize) -> Option<Mode> {
    Mode::from_u8(PENDING[i].load(Ordering::Acquire))
}

/// Applies pending toggles. Called on the game thread at a frame boundary; returns the changes made.
pub fn apply_pending() -> Vec<(usize, Mode)> {
    let mut out = Vec::new();
    for i in 0..N {
        if let Some(m) = Mode::from_u8(PENDING[i].swap(0, Ordering::AcqRel))
            && installed(i)
        {
            TARGET[i].store(target_for(i, m), Ordering::Release);
            out.push((i, m));
        }
    }
    out
}

/// Finds a hook by address (`0x561b10`) or by name (`CTimer::Update`, case-insensitive).
pub fn find(key: &str) -> Option<usize> {
    let addr = key
        .strip_prefix("0x")
        .or_else(|| key.strip_prefix("0X"))
        .and_then(|h| u32::from_str_radix(h, 16).ok());
    HOOKS
        .iter()
        .position(|h| Some(h.addr) == addr || h.name.eq_ignore_ascii_case(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x86;

    #[test]
    fn table_is_consistent() {
        assert_eq!(HOOKS.len(), N);
        for (i, h) in HOOKS.iter().enumerate() {
            assert_eq!(h.id as usize, i, "{}", h.name);
            match h.kind {
                Kind::Inline => {
                    let r = x86::relocate(h.expect, h.addr, 0x1000_0000, 5).unwrap();
                    assert!(r.consumed <= h.expect.len(), "{}: expect too short", h.name);
                }
                Kind::HoodlumStub => {
                    assert_eq!(h.expect.len(), 5);
                    let t = x86::decode(h.expect).unwrap().target(h.expect, h.addr).unwrap();
                    assert!((0x1556000..0x1600000).contains(&t), "{}: {t:#x}", h.name);
                }
            }
        }
        assert_eq!(HOOKS.iter().filter(|h| h.frame_boundary).count(), 1);
        assert_eq!(find("0x561b10"), Some(0));
        assert_eq!(find("cgeneral::limitradianangle"), Some(1));
        assert_eq!(find("nope"), None);
    }

    #[test]
    fn toggles_apply_only_at_the_boundary() {
        set_installed(1, 0x1234, Mode::Rust);
        assert_eq!(mode(1), Mode::Rust);
        request(1, Mode::Original);
        assert_eq!(mode(1), Mode::Rust);
        assert_eq!(pending(1), Some(Mode::Original));
        assert_eq!(apply_pending(), vec![(1, Mode::Original)]);
        assert_eq!(TARGET[1].load(Ordering::Relaxed), 0x1234);
        assert_eq!(mode(1), Mode::Original);
        assert_eq!(pending(1), None);
        assert!(apply_pending().is_empty());
    }

    /// The entry bytes, names and kinds agree with the catalogue and the real exe (when present).
    #[test]
    fn matches_catalogue_and_exe() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let csv = std::fs::read_to_string(root.join("catalog/functions.csv")).unwrap();
        for h in HOOKS {
            let key = format!("{:#x},", h.addr);
            let row = csv
                .lines()
                .find(|l| l.starts_with(&key))
                .unwrap_or_else(|| panic!("{} not in catalogue", h.name));
            let f: Vec<&str> = row.split(',').collect();
            assert_eq!(format!("{}::{}", f[1], f[2]), h.name);
            assert_eq!(
                f[7] == "stub-to-.HOODLUM",
                h.kind == Kind::HoodlumStub,
                "{}",
                h.name
            );
        }
        let Some(exe) = crate::testexe::load() else { return };
        for h in HOOKS {
            assert_eq!(exe.bytes(h.addr, h.expect.len()), h.expect, "{}", h.name);
        }
    }
}
