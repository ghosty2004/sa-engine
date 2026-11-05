//! The hook table: one entry per original function, keyed by its address in gta_sa.exe 1.0 US.
//!
//! Every entry names the function (catalogue `owner::name`), lists the entry bytes that must be found
//! before patching (so a wrong exe or a foreign patch is never overwritten), and points at its Rust
//! implementation. Runtime state (original entry, mode, call counters) lives in parallel atomics.

use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

use sa_oracle_format::l2::{Conv, PtrArg, Ret, Sig};

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
    PadUpdate = 3,
    PadUpdateMouse = 4,
    CdStreamRead = 5,
    Rand = 6,
    Srand = 7,
    HeapAlloc = 8,
    Free = 9,
    Realloc = 10,
    Calloc = 11,
    Msize = 12,
    AudioHardwareService = 13,
    LimitAngle = 14,
    GetRadianAngleBetweenPoints = 15,
    GetATanOfXY = 16,
    SolveQuadratic = 17,
    VectorNormalise = 18,
}

/// When a hook is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// Always (the INF-3 set).
    Core,
    /// Only when an oracle feature (determinism mode, hashing, record/replay) is enabled at attach;
    /// these are observers/injectors that pass through to the original unless their feature is on.
    Oracle,
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
    pub group: Group,
    /// Calling signature for the L2 oracle (None: L2 not supported for this hook).
    pub sig: Option<Sig>,
}

const fn sig(conv: Conv, args: u8, ret: Ret) -> Option<Sig> {
    Some(Sig {
        conv,
        args,
        ret,
        this_type: None,
        this_size: 0,
        ptr_args: &[],
    })
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
        group: Group::Core,
        sig: sig(Conv::Cdecl, 0, Ret::Void),
    },
    HookSpec {
        id: Id::LimitRadianAngle,
        addr: 0x53CB50,
        name: "CGeneral::LimitRadianAngle",
        kind: Kind::Inline,
        // fld dword [esp+4]; fld dword [0x863AC0]
        expect: &[0xD9, 0x44, 0x24, 0x04, 0xD9, 0x05, 0xC0, 0x3A, 0x86, 0x00],
        rust: || crate::game::limit_radian_angle_hook as extern "C" fn(f32) -> f64 as usize,
        frame_boundary: false,
        start_rust: true,
        group: Group::Core,
        sig: sig(Conv::Cdecl, 1, Ret::St0),
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
        group: Group::Core,
        sig: sig(Conv::Cdecl, 0, Ret::Eax),
    },
    HookSpec {
        id: Id::PadUpdate,
        addr: 0x541C40,
        name: "CPad::Update",
        kind: Kind::Inline,
        // sub esp,0x30; push ebx; push ebp
        expect: &[0x83, 0xEC, 0x30, 0x53, 0x55],
        rust: crate::oracle::pad_update_entry,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        sig: None,
    },
    HookSpec {
        id: Id::PadUpdateMouse,
        addr: 0x53F3C0,
        name: "CPad::UpdateMouse",
        kind: Kind::Inline,
        // sub esp,0x18; push ebp; mov ebp,ecx
        expect: &[0x83, 0xEC, 0x18, 0x55, 0x8B, 0xE9],
        rust: crate::oracle::pad_update_mouse_entry,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        sig: None,
    },
    HookSpec {
        id: Id::CdStreamRead,
        addr: 0x406A20,
        name: "CdStream::CdStreamRead",
        kind: Kind::HoodlumStub,
        // jmp 0x156C2C0
        expect: &[0xE9, 0x9B, 0x58, 0x16, 0x01],
        rust: crate::oracle::cd_stream_read_entry,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        sig: None,
    },
    HookSpec {
        id: Id::Rand,
        addr: 0x821B1E,
        name: "_rand",
        kind: Kind::Inline,
        // call _getptd (0x827B3D); mov ecx,[eax+0x14]
        expect: &[0xE8, 0x1A, 0x60, 0x00, 0x00, 0x8B, 0x48, 0x14],
        rust: crate::oracle::rand_entry,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        sig: None,
    },
    HookSpec {
        id: Id::Srand,
        addr: 0x821B11,
        name: "_srand",
        kind: Kind::Inline,
        // call _getptd (0x827B3D); mov ecx,[esp+4]
        expect: &[0xE8, 0x27, 0x60, 0x00, 0x00, 0x8B, 0x4C, 0x24, 0x04],
        rust: crate::oracle::srand_entry,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        sig: None,
    },
    // The CRT heap (VC7.1 static CRT): every allocation path ends in one of these. In determinism
    // mode the main thread's blocks come from the deterministic arena (`arena.rs`).
    HookSpec {
        id: Id::HeapAlloc,
        addr: 0x8241B0,
        name: "__heap_alloc",
        kind: Kind::Inline,
        // push 0xC; push 0x888048 (SEH prolog)
        expect: &[0x6A, 0x0C, 0x68, 0x48, 0x80, 0x88, 0x00],
        rust: crate::oracle::heap_alloc_entry,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        sig: None,
    },
    HookSpec {
        id: Id::Free,
        addr: 0x82413F,
        name: "_free",
        kind: Kind::Inline,
        expect: &[0x6A, 0x0C, 0x68, 0x38, 0x80, 0x88, 0x00],
        rust: crate::oracle::free_entry,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        sig: None,
    },
    HookSpec {
        id: Id::Realloc,
        addr: 0x824269,
        name: "_realloc",
        kind: Kind::Inline,
        expect: &[0x6A, 0x14, 0x68, 0x58, 0x80, 0x88, 0x00],
        rust: crate::oracle::realloc_entry,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        sig: None,
    },
    HookSpec {
        id: Id::Calloc,
        addr: 0x824416,
        name: "_calloc",
        kind: Kind::Inline,
        expect: &[0x6A, 0x10, 0x68, 0x68, 0x80, 0x88, 0x00],
        rust: crate::oracle::calloc_entry,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        sig: None,
    },
    HookSpec {
        id: Id::Msize,
        addr: 0x828C4A,
        name: "__msize",
        kind: Kind::Inline,
        expect: &[0x6A, 0x10, 0x68, 0x80, 0x84, 0x88, 0x00],
        rust: crate::oracle::msize_entry,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        sig: None,
    },
    HookSpec {
        id: Id::AudioHardwareService,
        addr: 0x4D9870,
        name: "CAEAudioHardware::Service",
        kind: Kind::Inline,
        // push ebx; push esi; push edi; mov esi,ecx
        expect: &[0x53, 0x56, 0x57, 0x8B, 0xF1],
        rust: crate::oracle::audio_hw_service_entry,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        sig: None,
    },
    // sa-math ports (L1-verified). Installed in original mode; `SA_HOOK_MODE` / `hook NAME rust|l2`
    // switch them.
    HookSpec {
        id: Id::LimitAngle,
        addr: 0x53CB00,
        name: "CGeneral::LimitAngle",
        kind: Kind::Inline,
        // fld dword [esp+4]; fcom dword [0x85A994]
        expect: &[0xD9, 0x44, 0x24, 0x04, 0xD8, 0x15, 0x94, 0xA9, 0x85, 0x00],
        rust: || crate::game::limit_angle_hook as extern "C" fn(f32) -> f64 as usize,
        frame_boundary: false,
        start_rust: false,
        group: Group::Core,
        sig: sig(Conv::Cdecl, 1, Ret::St0),
    },
    HookSpec {
        id: Id::GetRadianAngleBetweenPoints,
        addr: 0x53CBE0,
        name: "CGeneral::GetRadianAngleBetweenPoints-",
        kind: Kind::Inline,
        // fld dword [esp+0xC]; fsub dword [esp+4]
        expect: &[0xD9, 0x44, 0x24, 0x0C, 0xD8, 0x64, 0x24, 0x04],
        rust: || {
            crate::game::get_radian_angle_between_points_hook as extern "C" fn(f32, f32, f32, f32) -> f64
                as usize
        },
        frame_boundary: false,
        start_rust: false,
        group: Group::Core,
        sig: sig(Conv::Cdecl, 4, Ret::St0),
    },
    HookSpec {
        id: Id::GetATanOfXY,
        addr: 0x53CC70,
        name: "CGeneral::GetATanOfXY",
        kind: Kind::Inline,
        // fld dword [esp+4]; fcomp dword [0x858B50]
        expect: &[0xD9, 0x44, 0x24, 0x04, 0xD8, 0x1D, 0x50, 0x8B, 0x85, 0x00],
        rust: || crate::game::get_atan_of_xy_hook as extern "C" fn(f32, f32) -> f64 as usize,
        frame_boundary: false,
        start_rust: false,
        group: Group::Core,
        sig: sig(Conv::Cdecl, 2, Ret::St0),
    },
    HookSpec {
        id: Id::SolveQuadratic,
        addr: 0x53CE30,
        name: "CGeneral::SolveQuadratic",
        kind: Kind::Inline,
        // fld dword [esp+8]; fmul dword [esp+8]
        expect: &[0xD9, 0x44, 0x24, 0x08, 0xD8, 0x4C, 0x24, 0x08],
        rust: || {
            crate::game::solve_quadratic_hook
                as unsafe extern "C" fn(f32, f32, f32, *mut f32, *mut f32) -> bool as usize
        },
        frame_boundary: false,
        start_rust: false,
        group: Group::Core,
        sig: Some(Sig {
            conv: Conv::Cdecl,
            args: 5,
            ret: Ret::Al,
            this_type: None,
            this_size: 0,
            // float* x1, float* x2
            ptr_args: &[PtrArg { arg: 3, size: 4 }, PtrArg { arg: 4, size: 4 }],
        }),
    },
    HookSpec {
        id: Id::VectorNormalise,
        addr: 0x59C910,
        name: "CVector::Normalise",
        kind: Kind::Inline,
        // fld dword [ecx+8]; fld dword [ecx+4]
        expect: &[0xD9, 0x41, 0x08, 0xD9, 0x41, 0x04],
        rust: crate::game::vector_normalise_entry,
        frame_boundary: false,
        start_rust: false,
        group: Group::Core,
        sig: Some(Sig {
            conv: Conv::Thiscall,
            args: 0,
            ret: Ret::Void,
            this_type: Some("CVector"),
            // sizeof(CVector) = 0xC (catalog/layouts.csv)
            this_size: 0xC,
            ptr_args: &[],
        }),
    },
];

const N: usize = 19;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Original = 1,
    Rust = 2,
    /// Every call goes through the L2 oracle dispatcher (`l2win`).
    L2 = 3,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Original => "orig",
            Mode::Rust => "rust",
            Mode::L2 => "l2",
        }
    }
    fn from_u8(v: u8) -> Option<Mode> {
        match v {
            1 => Some(Mode::Original),
            2 => Some(Mode::Rust),
            3 => Some(Mode::L2),
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
/// Per-hook L2 entry stub (`push i; jmp l2_common`), 0 when L2 is not available.
static L2_STUB: [AtomicU32; N] = [Z32; N];

pub fn set_l2_stub(i: usize, addr: u32) {
    L2_STUB[i].store(addr, Ordering::Release);
}

pub fn l2_stub(i: usize) -> u32 {
    L2_STUB[i].load(Ordering::Acquire)
}

/// The Rust implementation's entry.
pub fn rust_entry(i: usize) -> u32 {
    (HOOKS[i].rust)() as u32
}

/// Original entry by index.
pub fn original_at(i: usize) -> u32 {
    ORIGINAL[i].load(Ordering::Acquire)
}

/// Sets the mode immediately. Only on the game thread (e.g. the L2 dispatcher retiring a hook).
pub fn set_mode_now(i: usize, mode: Mode) {
    if installed(i) {
        TARGET[i].store(target_for(i, mode), Ordering::Release);
    }
}

pub fn spec(id: Id) -> &'static HookSpec {
    &HOOKS[id as usize]
}

pub fn original(id: Id) -> u32 {
    ORIGINAL[id as usize].load(Ordering::Acquire)
}

pub fn count(id: Id) {
    CALLS[id as usize].fetch_add(1, Ordering::Relaxed);
    crate::l2::note_call(id as u32);
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
        Mode::L2 => match L2_STUB[i].load(Ordering::Acquire) {
            0 => TARGET[i].load(Ordering::Acquire),
            s => s,
        },
    }
}

pub fn mode(i: usize) -> Mode {
    let t = TARGET[i].load(Ordering::Acquire);
    if t == ORIGINAL[i].load(Ordering::Acquire) {
        Mode::Original
    } else if t != 0 && t == L2_STUB[i].load(Ordering::Acquire) {
        Mode::L2
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
        let procs = std::fs::read_to_string(root.join("catalog/procedures.csv")).unwrap();
        for h in HOOKS {
            let key = format!("{:#x},", h.addr);
            let Some(row) = csv.lines().find(|l| l.starts_with(&key)) else {
                // Not a gta-reversed hook: the CRT's rand/srand (catalogue class excluded:crt, found by
                // the MSVC LCG constants 214013/2531011 at 0x821B28/0x821B2E).
                let p = procs
                    .lines()
                    .find(|l| l.starts_with(&key))
                    .unwrap_or_else(|| panic!("{} not in catalogue", h.name));
                assert!(p.split(',').any(|c| c == "excluded:crt"), "{}: {p}", h.name);
                assert_eq!(h.group, Group::Oracle);
                continue;
            };
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
            // The installer refuses an inline hook with a branch into the patched range; check it here.
            if h.kind == Kind::Inline {
                let r = x86::relocate(h.expect, h.addr, 0x1000_0000, 5).unwrap();
                let around = exe.bytes(h.addr, 0x400);
                assert_eq!(
                    x86::inbound_branch(around, h.addr, h.addr, h.addr + r.consumed as u32),
                    None,
                    "{}",
                    h.name
                );
            }
        }
    }
}
