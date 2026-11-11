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
    /// Ports from the engine crates (`sa_core::hooks::ALL`, state `implemented` until verified):
    /// installed only when `SA_HOOK_PORTS` is set (see `win.rs`), starting in original mode unless
    /// listed there.
    Port,
}

#[derive(Clone, Copy)]
pub struct HookSpec {
    /// The sa-hook-local hooks (`Id`); `None` for ports registered by engine crates (`sa_core::hooks`).
    pub id: Option<Id>,
    pub addr: u32,
    pub name: &'static str,
    pub kind: Kind,
    /// Exact bytes expected at `addr` (at least the stolen range).
    pub expect: &'static [u8],
    /// Address of the Rust implementation (original calling convention).
    pub rust: fn() -> usize,
    /// Call counter kept by the port's own shim (engine-crate ports); `None`: counted here.
    pub calls: Option<fn() -> u32>,
    /// The thunk calls the frame-boundary handler before dispatching (one per frame).
    pub frame_boundary: bool,
    /// Mode at attach.
    pub start_rust: bool,
    pub group: Group,
    /// Fallback hooks (ADR-002): receives the original entry (trampoline or old stub target) at install,
    /// so the port can run the original body for the paths it has not ported.
    pub set_original: Option<fn(u32)>,
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

/// The hooks of sa-hook itself (indices = `Id`).
const LOCAL: [HookSpec; LOCAL_N] = [
    HookSpec {
        id: Some(Id::TimerUpdate),
        addr: 0x561B10,
        name: "CTimer::Update",
        kind: Kind::Inline,
        // mov ecx,[0xB7CB28]; sub esp,0xC
        expect: &[0x8B, 0x0D, 0x28, 0xCB, 0xB7, 0x00, 0x83, 0xEC, 0x0C],
        rust: || crate::game::timer_update_hook as extern "C" fn() as usize,
        calls: None,
        frame_boundary: true,
        start_rust: true,
        group: Group::Core,
        set_original: None,
        sig: sig(Conv::Cdecl, 0, Ret::Void),
    },
    HookSpec {
        id: Some(Id::LimitRadianAngle),
        addr: 0x53CB50,
        name: "CGeneral::LimitRadianAngle",
        kind: Kind::Inline,
        // fld dword [esp+4]; fld dword [0x863AC0]
        expect: &[0xD9, 0x44, 0x24, 0x04, 0xD9, 0x05, 0xC0, 0x3A, 0x86, 0x00],
        rust: || crate::game::limit_radian_angle_hook as extern "C" fn(f32) -> f64 as usize,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Core,
        set_original: None,
        sig: sig(Conv::Cdecl, 1, Ret::St0),
    },
    HookSpec {
        id: Some(Id::IsCoopGameGoingOn),
        addr: 0x441390,
        name: "CGameLogic::IsCoopGameGoingOn",
        kind: Kind::HoodlumStub,
        // jmp 0x156CF30
        expect: &[0xE9, 0x9B, 0xBB, 0x12, 0x01],
        rust: || crate::game::is_coop_game_going_on_hook as extern "C" fn() -> u32 as usize,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Core,
        set_original: None,
        sig: sig(Conv::Cdecl, 0, Ret::Eax),
    },
    HookSpec {
        id: Some(Id::PadUpdate),
        addr: 0x541C40,
        name: "CPad::Update",
        kind: Kind::Inline,
        // sub esp,0x30; push ebx; push ebp
        expect: &[0x83, 0xEC, 0x30, 0x53, 0x55],
        rust: crate::oracle::pad_update_entry,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        set_original: None,
        sig: None,
    },
    HookSpec {
        id: Some(Id::PadUpdateMouse),
        addr: 0x53F3C0,
        name: "CPad::UpdateMouse",
        kind: Kind::Inline,
        // sub esp,0x18; push ebp; mov ebp,ecx
        expect: &[0x83, 0xEC, 0x18, 0x55, 0x8B, 0xE9],
        rust: crate::oracle::pad_update_mouse_entry,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        set_original: None,
        sig: None,
    },
    HookSpec {
        id: Some(Id::CdStreamRead),
        addr: 0x406A20,
        name: "CdStream::CdStreamRead",
        kind: Kind::HoodlumStub,
        // jmp 0x156C2C0
        expect: &[0xE9, 0x9B, 0x58, 0x16, 0x01],
        rust: crate::oracle::cd_stream_read_entry,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        set_original: None,
        sig: None,
    },
    HookSpec {
        id: Some(Id::Rand),
        addr: 0x821B1E,
        name: "_rand",
        kind: Kind::Inline,
        // call _getptd (0x827B3D); mov ecx,[eax+0x14]
        expect: &[0xE8, 0x1A, 0x60, 0x00, 0x00, 0x8B, 0x48, 0x14],
        rust: crate::oracle::rand_entry,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        set_original: None,
        sig: None,
    },
    HookSpec {
        id: Some(Id::Srand),
        addr: 0x821B11,
        name: "_srand",
        kind: Kind::Inline,
        // call _getptd (0x827B3D); mov ecx,[esp+4]
        expect: &[0xE8, 0x27, 0x60, 0x00, 0x00, 0x8B, 0x4C, 0x24, 0x04],
        rust: crate::oracle::srand_entry,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        set_original: None,
        sig: None,
    },
    // The CRT heap (VC7.1 static CRT): every allocation path ends in one of these. In determinism
    // mode the main thread's blocks come from the deterministic arena (`arena.rs`).
    HookSpec {
        id: Some(Id::HeapAlloc),
        addr: 0x8241B0,
        name: "__heap_alloc",
        kind: Kind::Inline,
        // push 0xC; push 0x888048 (SEH prolog)
        expect: &[0x6A, 0x0C, 0x68, 0x48, 0x80, 0x88, 0x00],
        rust: crate::oracle::heap_alloc_entry,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        set_original: None,
        sig: None,
    },
    HookSpec {
        id: Some(Id::Free),
        addr: 0x82413F,
        name: "_free",
        kind: Kind::Inline,
        expect: &[0x6A, 0x0C, 0x68, 0x38, 0x80, 0x88, 0x00],
        rust: crate::oracle::free_entry,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        set_original: None,
        sig: None,
    },
    HookSpec {
        id: Some(Id::Realloc),
        addr: 0x824269,
        name: "_realloc",
        kind: Kind::Inline,
        expect: &[0x6A, 0x14, 0x68, 0x58, 0x80, 0x88, 0x00],
        rust: crate::oracle::realloc_entry,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        set_original: None,
        sig: None,
    },
    HookSpec {
        id: Some(Id::Calloc),
        addr: 0x824416,
        name: "_calloc",
        kind: Kind::Inline,
        expect: &[0x6A, 0x10, 0x68, 0x68, 0x80, 0x88, 0x00],
        rust: crate::oracle::calloc_entry,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        set_original: None,
        sig: None,
    },
    HookSpec {
        id: Some(Id::Msize),
        addr: 0x828C4A,
        name: "__msize",
        kind: Kind::Inline,
        expect: &[0x6A, 0x10, 0x68, 0x80, 0x84, 0x88, 0x00],
        rust: crate::oracle::msize_entry,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        set_original: None,
        sig: None,
    },
    HookSpec {
        id: Some(Id::AudioHardwareService),
        addr: 0x4D9870,
        name: "CAEAudioHardware::Service",
        kind: Kind::Inline,
        // push ebx; push esi; push edi; mov esi,ecx
        expect: &[0x53, 0x56, 0x57, 0x8B, 0xF1],
        rust: crate::oracle::audio_hw_service_entry,
        calls: None,
        frame_boundary: false,
        start_rust: true,
        group: Group::Oracle,
        set_original: None,
        sig: None,
    },
    // sa-math ports (L1-verified). Installed in original mode; `SA_HOOK_MODE` / `hook NAME rust|l2`
    // switch them.
    HookSpec {
        id: Some(Id::LimitAngle),
        addr: 0x53CB00,
        name: "CGeneral::LimitAngle",
        kind: Kind::Inline,
        // fld dword [esp+4]; fcom dword [0x85A994]
        expect: &[0xD9, 0x44, 0x24, 0x04, 0xD8, 0x15, 0x94, 0xA9, 0x85, 0x00],
        rust: || crate::game::limit_angle_hook as extern "C" fn(f32) -> f64 as usize,
        calls: None,
        frame_boundary: false,
        start_rust: false,
        group: Group::Core,
        set_original: None,
        sig: sig(Conv::Cdecl, 1, Ret::St0),
    },
    HookSpec {
        id: Some(Id::GetRadianAngleBetweenPoints),
        addr: 0x53CBE0,
        name: "CGeneral::GetRadianAngleBetweenPoints-",
        kind: Kind::Inline,
        // fld dword [esp+0xC]; fsub dword [esp+4]
        expect: &[0xD9, 0x44, 0x24, 0x0C, 0xD8, 0x64, 0x24, 0x04],
        rust: || {
            crate::game::get_radian_angle_between_points_hook as extern "C" fn(f32, f32, f32, f32) -> f64
                as usize
        },
        calls: None,
        frame_boundary: false,
        start_rust: false,
        group: Group::Core,
        set_original: None,
        sig: sig(Conv::Cdecl, 4, Ret::St0),
    },
    HookSpec {
        id: Some(Id::GetATanOfXY),
        addr: 0x53CC70,
        name: "CGeneral::GetATanOfXY",
        kind: Kind::Inline,
        // fld dword [esp+4]; fcomp dword [0x858B50]
        expect: &[0xD9, 0x44, 0x24, 0x04, 0xD8, 0x1D, 0x50, 0x8B, 0x85, 0x00],
        rust: || crate::game::get_atan_of_xy_hook as extern "C" fn(f32, f32) -> f64 as usize,
        calls: None,
        frame_boundary: false,
        start_rust: false,
        group: Group::Core,
        set_original: None,
        sig: sig(Conv::Cdecl, 2, Ret::St0),
    },
    HookSpec {
        id: Some(Id::SolveQuadratic),
        addr: 0x53CE30,
        name: "CGeneral::SolveQuadratic",
        kind: Kind::Inline,
        // fld dword [esp+8]; fmul dword [esp+8]
        expect: &[0xD9, 0x44, 0x24, 0x08, 0xD8, 0x4C, 0x24, 0x08],
        rust: || {
            crate::game::solve_quadratic_hook
                as unsafe extern "C" fn(f32, f32, f32, *mut f32, *mut f32) -> bool as usize
        },
        calls: None,
        frame_boundary: false,
        start_rust: false,
        group: Group::Core,
        set_original: None,
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
        id: Some(Id::VectorNormalise),
        addr: 0x59C910,
        name: "CVector::Normalise",
        kind: Kind::Inline,
        // fld dword [ecx+8]; fld dword [ecx+4]
        expect: &[0xD9, 0x41, 0x08, 0xD9, 0x41, 0x04],
        rust: crate::game::vector_normalise_entry,
        calls: None,
        frame_boundary: false,
        start_rust: false,
        group: Group::Core,
        set_original: None,
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

const LOCAL_N: usize = 19;
/// All hooks: sa-hook's own, then every port registered by sa-core (in `sa_core::hooks::ALL` order), then
/// sa-stream's (`sa_stream::hooks::ALL`), then sa-world's (`sa_world::hooks::ALL`), then sa-pop's
/// (`sa_pop::hooks::ALL`), sa-script's, sa-weapon's, sa-ped's, sa-input's, sa-anim's and sa-veh's.
pub const N: usize = LOCAL_N
    + sa_core::hooks::COUNT
    + sa_stream::hooks::COUNT
    + sa_world::hooks::COUNT
    + sa_pop::hooks::COUNT
    + sa_script::hooks::COUNT
    + sa_weapon::hooks::COUNT
    + sa_ped::hooks::COUNT
    + sa_input::hooks::COUNT
    + sa_anim::hooks::COUNT
    + sa_veh::hooks::COUNT;

/// The hook table.
pub static HOOKS: [HookSpec; N] = build();

/// The L2 calling signature of an engine-crate port (from its `#[hook]` registration).
const fn sig_of(r: &sa_core::abi::HookReg) -> Sig {
    use sa_core::abi::{AbiRet, Conv as C};
    Sig {
        conv: match r.conv {
            C::Cdecl => Conv::Cdecl,
            C::Stdcall => Conv::Stdcall,
            C::Thiscall => Conv::Thiscall,
            C::Fastcall => Conv::Fastcall,
        },
        args: r.stack_words,
        ret: match r.ret {
            AbiRet::Void => Ret::Void,
            AbiRet::Al => Ret::Al,
            AbiRet::Eax => Ret::Eax,
            AbiRet::EaxEdx => Ret::EaxEdx,
            AbiRet::St0 => Ret::St0,
        },
        this_type: r.this_type,
        this_size: r.this_size,
        ptr_args: &[],
    }
}

const fn from_core(r: &sa_core::abi::HookReg) -> HookSpec {
    HookSpec {
        id: None,
        addr: r.addr,
        name: r.name,
        kind: if r.is_hoodlum_stub() {
            Kind::HoodlumStub
        } else {
            Kind::Inline
        },
        expect: r.expect,
        rust: r.entry,
        calls: Some(r.calls),
        frame_boundary: false,
        start_rust: false,
        group: Group::Port,
        set_original: r.set_original,
        sig: Some(sig_of(r)),
    }
}

const fn build() -> [HookSpec; N] {
    let mut out = [LOCAL[0]; N];
    let mut i = 0;
    while i < LOCAL_N {
        out[i] = LOCAL[i];
        i += 1;
    }
    let mut j = 0;
    while j < sa_core::hooks::COUNT {
        out[LOCAL_N + j] = from_core(&sa_core::hooks::ALL[j]);
        j += 1;
    }
    let mut k = 0;
    while k < sa_stream::hooks::COUNT {
        out[LOCAL_N + sa_core::hooks::COUNT + k] = from_core(&sa_stream::hooks::ALL[k]);
        k += 1;
    }
    let mut w = 0;
    while w < sa_world::hooks::COUNT {
        out[LOCAL_N + sa_core::hooks::COUNT + sa_stream::hooks::COUNT + w] =
            from_core(&sa_world::hooks::ALL[w]);
        w += 1;
    }
    let mut p = 0;
    while p < sa_pop::hooks::COUNT {
        out[LOCAL_N + sa_core::hooks::COUNT + sa_stream::hooks::COUNT + sa_world::hooks::COUNT + p] =
            from_core(&sa_pop::hooks::ALL[p]);
        p += 1;
    }
    let mut c = 0;
    while c < sa_script::hooks::COUNT {
        out[LOCAL_N
            + sa_core::hooks::COUNT
            + sa_stream::hooks::COUNT
            + sa_world::hooks::COUNT
            + sa_pop::hooks::COUNT
            + c] = from_core(&sa_script::hooks::ALL[c]);
        c += 1;
    }
    let base_weapon = LOCAL_N
        + sa_core::hooks::COUNT
        + sa_stream::hooks::COUNT
        + sa_world::hooks::COUNT
        + sa_pop::hooks::COUNT
        + sa_script::hooks::COUNT;
    let mut w2 = 0;
    while w2 < sa_weapon::hooks::COUNT {
        out[base_weapon + w2] = from_core(&sa_weapon::hooks::ALL[w2]);
        w2 += 1;
    }
    let base_ped = base_weapon + sa_weapon::hooks::COUNT;
    let mut q = 0;
    while q < sa_ped::hooks::COUNT {
        out[base_ped + q] = from_core(&sa_ped::hooks::ALL[q]);
        q += 1;
    }
    let base_input = base_ped + sa_ped::hooks::COUNT;
    let mut i2 = 0;
    while i2 < sa_input::hooks::COUNT {
        out[base_input + i2] = from_core(&sa_input::hooks::ALL[i2]);
        i2 += 1;
    }
    let base_anim = base_input + sa_input::hooks::COUNT;
    let mut a = 0;
    while a < sa_anim::hooks::COUNT {
        out[base_anim + a] = from_core(&sa_anim::hooks::ALL[a]);
        a += 1;
    }
    let base_veh = base_anim + sa_anim::hooks::COUNT;
    let mut v = 0;
    while v < sa_veh::hooks::COUNT {
        out[base_veh + v] = from_core(&sa_veh::hooks::ALL[v]);
        v += 1;
    }
    out
}

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
    match HOOKS[i].calls {
        Some(f) => f(),
        None => CALLS[i].load(Ordering::Relaxed),
    }
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
    if let Some(f) = HOOKS[i].set_original {
        f(original);
    }
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

/// What `SA_HOOK_PORTS` asks for the `Group::Port` hooks at attach:
/// unset or empty → not installed (the original runs untouched);
/// `orig` → installed, all in original mode (toggle them with the control channel's `hook`);
/// `rust` → installed, all in Rust mode;
/// a comma-separated list of names or `0x` addresses → installed, those in Rust mode, the rest original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortPlan {
    Off,
    AllOriginal,
    AllRust,
    Rust(Vec<usize>),
}

impl PortPlan {
    pub fn parse(env: Option<&str>) -> Result<PortPlan, String> {
        let v = env.map(str::trim).unwrap_or("");
        Ok(match v {
            "" => PortPlan::Off,
            "orig" | "original" => PortPlan::AllOriginal,
            "rust" => PortPlan::AllRust,
            list => {
                let mut ids = Vec::new();
                for k in list.split(',').map(str::trim).filter(|k| !k.is_empty()) {
                    match find(k) {
                        Some(i) if HOOKS[i].group == Group::Port => ids.push(i),
                        Some(_) => return Err(format!("SA_HOOK_PORTS: {k} is not a port hook")),
                        None => return Err(format!("SA_HOOK_PORTS: unknown hook {k}")),
                    }
                }
                PortPlan::Rust(ids)
            }
        })
    }

    pub fn installs(&self) -> bool {
        *self != PortPlan::Off
    }

    /// Mode at attach of hook `i` (any group).
    pub fn start_mode(&self, i: usize) -> Mode {
        let rust = if HOOKS[i].group != Group::Port {
            HOOKS[i].start_rust
        } else {
            match self {
                PortPlan::AllRust => true,
                PortPlan::Rust(ids) => ids.contains(&i),
                PortPlan::Off | PortPlan::AllOriginal => false,
            }
        };
        if rust { Mode::Rust } else { Mode::Original }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_plans() {
        assert_eq!(PortPlan::parse(None), Ok(PortPlan::Off));
        assert_eq!(PortPlan::parse(Some(" ")), Ok(PortPlan::Off));
        assert_eq!(PortPlan::parse(Some("orig")), Ok(PortPlan::AllOriginal));
        let p = PortPlan::parse(Some("CPedPool::New, 0x41bd90")).unwrap();
        let (a, b) = (find("CPedPool::New").unwrap(), find("0x41bd90").unwrap());
        assert_eq!(p, PortPlan::Rust(vec![a, b]));
        assert_eq!(p.start_mode(a), Mode::Rust);
        assert_eq!(p.start_mode(find("CTimer::Stop").unwrap()), Mode::Original);
        // sa-hook's own hooks keep their configured mode
        assert_eq!(p.start_mode(Id::TimerUpdate as usize), Mode::Rust);
        assert!(PortPlan::parse(Some("CTimer::Update")).is_err());
        assert!(PortPlan::parse(Some("nope")).is_err());
        assert_eq!(PortPlan::AllRust.start_mode(a), Mode::Rust);
    }
    use crate::x86;

    #[test]
    fn table_is_consistent() {
        assert_eq!(HOOKS.len(), N);
        for (i, h) in HOOKS.iter().enumerate() {
            match h.id {
                Some(id) => assert_eq!(id as usize, i, "{}", h.name),
                None => assert!(i >= LOCAL_N && h.group == Group::Port, "{}", h.name),
            }
            match h.kind {
                Kind::Inline => {
                    let r = x86::relocate_keep_return(h.expect, h.addr, 0x1000_0000, 5).unwrap();
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
                if h.group == Group::Port {
                    // A port of a function gta-reversed does not hook (REA-named or unknown): its
                    // name is the catalogue's `name` column, or Ghidra's name while that is empty.
                    let c: Vec<&str> = p.split(',').collect();
                    let want = if c[8].is_empty() { c[4] } else { c[8] };
                    assert_eq!(h.name, want, "{p}");
                    continue;
                }
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
        for h in &HOOKS {
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

    /// The installer's checks (`win::install_one`) on the real exe for every hook: inline entries
    /// relocate, the stolen bytes lie within `expect`, no branch in the next 0x400 bytes lands in the
    /// patched range, and two hooks never overlap.
    #[test]
    fn every_hook_installs_on_the_exe() {
        let Some(exe) = crate::testexe::load() else { return };
        let mut ranges: Vec<(u32, u32, &str)> = Vec::new();
        for h in &HOOKS {
            let len = match h.kind {
                Kind::Inline => {
                    let code = exe.bytes(h.addr, 32);
                    let r = x86::relocate_keep_return(code, h.addr, 0x1000_0000, 5)
                        .unwrap_or_else(|e| panic!("{}: {e:?}", h.name));
                    assert!(
                        r.consumed <= h.expect.len(),
                        "{}: expect shorter than the steal",
                        h.name
                    );
                    let around = exe.bytes(h.addr, 0x400);
                    let hit = x86::inbound_branch(around, h.addr, h.addr, h.addr + r.consumed as u32);
                    assert_eq!(hit, None, "{}: branch into the patched range", h.name);
                    r.consumed as u32
                }
                Kind::HoodlumStub => 5,
            };
            ranges.push((h.addr, h.addr + len, h.name));
        }
        ranges.sort();
        for w in ranges.windows(2) {
            assert!(w[0].1 <= w[1].0, "{} overlaps {}", w[0].2, w[1].2);
        }
        // INF-20: no instruction reads (or points into) the bytes a hook changes, unless sa-hook
        // redirects it to a copy of the original dword (`keycells::READERS`). The candidates come from
        // catalog/code-data-reads.csv (tools/catalog/code_data_reads.py: every code section,
        // .HOODLUM and the no-CD .text included).
        let reads = crate::data_reads::rows();
        let mut bad = Vec::new();
        for h in &HOOKS {
            let (lo, hi) = crate::data_reads::changed(&exe, h.addr, h.kind);
            for r in crate::data_reads::unredirected(&reads, lo, hi) {
                bad.push(format!("{} ({:#x}): {r}", h.name, h.addr));
            }
        }
        assert!(
            bad.is_empty(),
            "hooks whose patched bytes are read as data:\n{}",
            bad.join("\n")
        );
    }

    #[test]
    fn port_hooks_follow_the_local_ones() {
        assert_eq!(
            N,
            LOCAL_N
                + sa_core::hooks::COUNT
                + sa_stream::hooks::COUNT
                + sa_world::hooks::COUNT
                + sa_pop::hooks::COUNT
                + sa_script::hooks::COUNT
                + sa_weapon::hooks::COUNT
                + sa_ped::hooks::COUNT
                + sa_input::hooks::COUNT
                + sa_anim::hooks::COUNT
                + sa_veh::hooks::COUNT
        );
        let ports = sa_core::hooks::ALL
            .iter()
            .chain(sa_stream::hooks::ALL.iter())
            .chain(sa_world::hooks::ALL.iter())
            .chain(sa_pop::hooks::ALL.iter())
            .chain(sa_script::hooks::ALL.iter())
            .chain(sa_weapon::hooks::ALL.iter())
            .chain(sa_ped::hooks::ALL.iter())
            .chain(sa_input::hooks::ALL.iter())
            .chain(sa_anim::hooks::ALL.iter())
            .chain(sa_veh::hooks::ALL.iter());
        for (k, r) in ports.enumerate() {
            let h = &HOOKS[LOCAL_N + k];
            assert_eq!((h.addr, h.name, h.expect), (r.addr, r.name, r.expect));
            assert_eq!(h.group, Group::Port);
            assert!(!h.start_rust && !h.frame_boundary);
        }
        assert_eq!(find("CPedPool::New"), Some(LOCAL_N + 2));
        // L2 signatures derived from the #[hook] registrations
        let s = HOOKS[find("CPedPool::GetAtRef").unwrap()].sig.unwrap();
        assert_eq!((s.conv, s.args, s.ret), (Conv::Thiscall, 1, Ret::Eax));
        assert_eq!((s.this_type, s.this_size), (Some("CPool<int32>"), 0x14));
        let s = HOOKS[find("0x41bd90").unwrap()].sig.unwrap();
        assert_eq!(
            (s.conv, s.args, s.ret, s.this_type),
            (Conv::Cdecl, 2, Ret::St0, None)
        );
        let s = HOOKS[find("CTimer::GetIsSlowMotionActive").unwrap()].sig.unwrap();
        assert_eq!((s.args, s.ret), (0, Ret::Al));
        let s = HOOKS[find("CTimer::UpdateVariables").unwrap()].sig.unwrap();
        assert_eq!((s.args, s.ret), (1, Ret::Void));
    }
}
