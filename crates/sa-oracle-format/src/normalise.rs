//! Pointer normalisation for the L2 oracle (PLAN §4.3, §5). This is the ONE reviewed file that decides
//! when two different raw values count as "the same": every L2 comparison (in-process in `sa-hook`, and
//! the host replay in `sa-oracle`) goes through [`words_equal`] / [`bytes_equal`].
//!
//! Rule: raw-identical bytes are always equal. Only where raw bytes differ, each aligned 32-bit word is
//! classified, and the two words are equal iff both classify to the same normal form:
//!
//! | class | normal form | when |
//! |---|---|---|
//! | heap allocation made *during the case* | `Alloc { ordinal, off }` | inside a block the run allocated (determinism arena), ordinal = allocation index within that run |
//! | entity pool slot | `Pool { pool, slot, off }` | inside an entity pool's storage (ped, vehicle, building, object, dummy; strides from INF-6) |
//! | model info | `Model { id, off }` | inside `CModelInfo::ms_modelInfoPtrs[id]` .. +0x20 (`CBaseModelInfo` size, layouts.csv) |
//! | exe image | `Image { off }` | 0x400000 + off inside the image (symbol + offset; symbolised by the host report) |
//! | OS object | `Foreign` | split-run only: one of the case's `foreign` values (a pointer into committed memory outside the image, the arena and the stack, i.e. Wine/D3D/DirectInput/DirectSound objects); no identity |
//! | stack | `Stack { frame, off }` | inside the thread's stack [limit, base) of a split-run case: in the frames above the nearest enclosing recorded call `frame` (1-based, [`NormCtx::frames`]) whose argument block lies at or below the value, off = value - that block; else `frame` 0 and off = value - the case's first-argument address (split-run only: `esp` is 0 elsewhere) |
//! | anything else | `Raw(v)` | |
//!
//! Inside one process, pools, model infos and the image are at the same addresses for the original's
//! run and the Rust run, so these classes only ever make raw-equal words equal; the class that changes a
//! verdict in-process is `Alloc` (the original and the Rust implementation allocate the "same" block at
//! different arena addresses). The other classes make fixtures comparable across runs. A word that is
//! not 4-byte aligned inside the compared buffer is never normalised (raw comparison).
//!
//! `Stack` (split-run L2, INF-17): the plain and the rust run are different processes. When an ancestor
//! of a recorded call is a Rust implementation (its frames differ in size from the original's, e.g.
//! VideoMode::SetVideoMode above the RW file reads at frame 0: plain 0x36F208, rust 0x36F1D8, the whole
//! stack 0x30 lower), every stack address below it moves by the same amount. A stack pointer is then
//! compared by its distance from the call's own argument block, which identifies the same caller
//! variable in both runs. `sa-oracle l2 split-report` counts the pairs compared with a stack shift.
//!
//! Stack frames of enclosing calls (HARNESS-3): the shift is not the same over the whole stack. Each
//! Rust frame between two locations moves the lower one by its own size difference, so a stack
//! address that lies in the frames ABOVE an enclosing recorded call (an ancestor whose own
//! implementation is Rust in the rust run) moved by THAT call's shift, not by the current call's. The
//! address of a `CEntity*` slot registered in `CReferences` (0xB9B9A8 `aRefs`) by a Rust
//! `CWeapon::FireInstantHit` is seen by `CWeapon::GenerateDamageEvent` below the Rust
//! `DoBulletImpact` (PORT-WEAPON-2 split-combat: 0xB9C4FC plain 0x0036F06C rust 0x0036EF5C, 0x110
//! apart, while GenerateDamageEvent's own argument blocks were further apart). The split report
//! gives each case the argument-block addresses of its enclosing recorded calls ([`NormCtx::frames`],
//! index-aligned between the two runs; nearest first), and a stack address is classified by the
//! frame it lies in: the enclosing call with the highest argument block at or below it, with the
//! offset from that block. Two addresses are then equal when they are the same distance above the
//! same enclosing call, which identifies the same variable of that call's callers in both runs.
//!
//! `Foreign` (split-run L2, INF-17; OS-assigned values): objects the OS / Wine / D3D allocate outside
//! the determinism arena (IDirect3D9 0xC97C20, the depth-stencil surface 0xC97C2C, vertex
//! declarations 0x8E2444, DirectInput devices PsGlobal+0x18, the D3D driver resource pointers in
//! RwEngineInstance+0x94 / 0xC980F0 / 0xC9AB40) live at addresses that differ between two processes:
//! Rust code allocates from the same process heap (evidence: identical windows up to
//! Win::Win32_InitInstance, then these words differ, plain 0x0301A530 rust 0x0301A340). Such a word is
//! equal to another such word whatever its value; that it IS an OS object in both runs is still
//! checked (0, a game pointer or a number in one run does not match). The class only exists in
//! split-run cases (their `foreign` list); every use is listed by `sa-oracle l2 split-report`.

use crate::subsys::PoolMap;

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8, catalogue rwsets name; gta-reversed ModelInfo.h).
pub const MODEL_INFO_PTRS: u32 = 0xA9B0C8;
/// `NUM_MODEL_INFOS` (gta-reversed ModelInfo.h: `ms_modelInfoPtrs[20000]`).
pub const NUM_MODEL_INFOS: u32 = 20000;
/// `sizeof(CBaseModelInfo)` (catalog/layouts.csv).
pub const BASE_MODEL_INFO_SIZE: u32 = 0x20;

/// Normal form of a word (module table). `Stack::frame` 0: above the case's own argument block; k:
/// above that of `NormCtx::frames[k - 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Norm {
    Raw(u32),
    Image { off: u32 },
    Pool { pool: u8, slot: u32, off: u32 },
    Model { id: u32, off: u32 },
    Alloc { ordinal: u32, off: u32 },
    Stack { frame: u8, off: u32 },
    Foreign,
}

/// What the normaliser knows about one run (the original's or the Rust implementation's).
#[derive(Debug, Clone, Default)]
pub struct NormCtx {
    /// Exe image [start, end).
    pub image: (u32, u32),
    pub pools: PoolMap,
    /// Model info pointers, indexed by model id (empty: class not used).
    pub model_infos: Vec<u32>,
    /// Blocks allocated by this run during the case, in allocation order: (address, size).
    pub allocs: Vec<(u32, u32)>,
    /// Split-run: the thread's stack [limit, base) and the case's first-argument address (0: no class).
    pub stack: (u32, u32),
    pub esp: u32,
    /// Split-run (host only): the argument-block addresses of the enclosing recorded calls of the
    /// case in this run, index-aligned with the other run's context (the same ancestors); 0 = not
    /// known in this run. Empty: only the case's own block (`esp`) is used.
    pub frames: Vec<u32>,
    /// Split-run: this run's OS object addresses (sorted; empty: no class).
    pub foreign: Vec<u32>,
}

impl NormCtx {
    /// Index into `frames` of the enclosing call whose callers' frames hold stack address `v`: the
    /// one with the highest argument block at or below `v` (above the case's own block `esp`).
    pub fn frame_of(&self, v: u32) -> Option<usize> {
        self.frames
            .iter()
            .enumerate()
            .take(254)
            .filter(|&(_, &f)| f != 0 && f > self.esp && f <= v)
            .max_by_key(|&(_, &f)| f)
            .map(|(k, _)| k)
    }

    pub fn normalise(&self, v: u32) -> Norm {
        for (i, &(a, n)) in self.allocs.iter().enumerate() {
            if v >= a && (v as u64) < a as u64 + n.max(1) as u64 {
                return Norm::Alloc {
                    ordinal: i as u32,
                    off: v - a,
                };
            }
        }
        let (tag, rest) = self.pools.normalise(v);
        if tag != v {
            return Norm::Pool {
                pool: ((tag >> 24) & 0x1F) as u8,
                slot: rest >> 16,
                off: rest & 0xFFFF,
            };
        }
        if v != 0 {
            // Exact model info objects (first BASE_MODEL_INFO_SIZE bytes only: derived sizes vary).
            if let Some(id) = self
                .model_infos
                .iter()
                .position(|&p| p != 0 && v >= p && v - p < BASE_MODEL_INFO_SIZE)
            {
                return Norm::Model {
                    id: id as u32,
                    off: v - self.model_infos[id],
                };
            }
        }
        if !self.foreign.is_empty() && self.foreign.binary_search(&v).is_ok() {
            return Norm::Foreign;
        }
        if self.esp != 0 && v >= self.stack.0 && v < self.stack.1 {
            return match self.frame_of(v) {
                Some(k) => Norm::Stack {
                    frame: (k + 1) as u8,
                    off: v - self.frames[k],
                },
                None => Norm::Stack {
                    frame: 0,
                    off: v.wrapping_sub(self.esp),
                },
            };
        }
        if v >= self.image.0 && v < self.image.1 {
            return Norm::Image {
                off: v - self.image.0,
            };
        }
        Norm::Raw(v)
    }
}

/// Two words from the original's run (`a`, context `ca`) and the Rust run (`b`, context `cb`).
pub fn words_equal(a: u32, ca: &NormCtx, b: u32, cb: &NormCtx) -> bool {
    a == b || ca.normalise(a) == cb.normalise(b)
}

/// Byte buffers starting at `addr` (alignment is taken from the absolute address). Returns the first
/// differing offset after normalisation, or None when equal.
pub fn bytes_equal(addr: u32, a: &[u8], ca: &NormCtx, b: &[u8], cb: &NormCtx) -> Option<usize> {
    if a == b {
        return None;
    }
    if a.len() != b.len() {
        return Some(a.len().min(b.len()));
    }
    let mut i = 0usize;
    while i < a.len() {
        if a[i] == b[i] {
            i += 1;
            continue;
        }
        // The aligned word containing byte i, if it lies completely inside both buffers.
        let abs = addr as u64 + i as u64;
        let ws = abs & !3;
        if ws >= addr as u64 && ws + 4 <= addr as u64 + a.len() as u64 {
            let w = (ws - addr as u64) as usize;
            let wa = u32::from_le_bytes([a[w], a[w + 1], a[w + 2], a[w + 3]]);
            let wb = u32::from_le_bytes([b[w], b[w + 1], b[w + 2], b[w + 3]]);
            if words_equal(wa, ca, wb, cb) {
                i = w + 4;
                continue;
            }
        }
        return Some(i);
    }
    None
}

// ------------------------------------------------------------------ excluded bytes ----------------
//
// Bytes the comparison ignores because the ORIGINAL leaves them uninitialised (stack or heap residue
// that no port can reproduce bit for bit). Every rule is narrow (one function, computed addresses),
// carries its byte evidence, and every use is listed by `sa-oracle l2 split-report` (`excluded:` lines)
// so no exclusion is silent. OS-assigned values (descriptors, handles) are NOT excluded here: the file
// probe records no handle value, and CRT descriptor numbers are compared raw.

/// One excluded byte range of a case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exclusion {
    pub addr: u32,
    pub len: u32,
    pub rule: &'static str,
}

/// `Light::LightsCreate` (0x5BA520).
pub const LIGHTS_CREATE: u32 = 0x5BA520;
/// `pAmbient`, `pDirect`, `pExtraDirectionals[6]` (0xC886E8..0xC88708).
const LIGHT_PTRS: (u32, u32) = (0xC886E8, 8);
/// `RpLight.color.alpha`: RpLightSetColor (0x751A90) copies the 16-byte RwRGBAReal to light+0x18.
const RPLIGHT_ALPHA: u32 = 0x24;
/// The three pad words of `RwFrame.modelling` (RwFrame+0x10; RwMatrix pad1/pad2/pad3 at +0x1C/+0x2C/+0x3C).
const FRAME_MODELLING_PADS: [u32; 3] = [0x10 + 0x1C, 0x10 + 0x2C, 0x10 + 0x3C];

pub const RULE_LIGHT_ALPHA: &str = "LightsCreate: RpLight colour alpha (+0x24) is uninitialised stack in the original: \
     the RwRGBAReal passed to RpLightSetColor is built at 0x5BA54E-0x5BA55E (and 0x5BA586-0x5BA596 \
     for pDirect, likewise for the extra lights) with red/green/blue only; RpLightSetColor 0x751A90 \
     copies all 16 bytes to light+0x18";
pub const RULE_FRAME_PADS: &str = "LightsCreate: pad1/pad2/pad3 of pDirect's frame modelling matrix are left \
     uninitialised by RwFrameRotate inside RenderWare (stack residue; CORE-L2b l2all evidence: orig \
     1 / 0x7BCDAEA0 / 0x00460000, Rust 4 / 0x0036F294 / 0x03219A28)";

/// The word at `a` after the run (`orig` run or the Rust run), from any region of the case that
/// holds memory after the call.
fn post_word(c: &crate::l2::Case, a: u32, orig: bool) -> Option<u32> {
    for r in c.regions.iter().filter(|r| r.readable && r.extent() > 0) {
        let b = if orig { &r.post_orig } else { &r.post_rust };
        if a >= r.addr && (a - r.addr) as u64 + 4 <= b.len() as u64 {
            let o = (a - r.addr) as usize;
            return Some(u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]));
        }
    }
    None
}

/// The byte ranges of case `c` excluded from the comparison (both runs' views; empty for almost every
/// function).
pub fn exclusions(c: &crate::l2::Case) -> Vec<Exclusion> {
    let mut out: Vec<Exclusion> = Vec::new();
    if c.merged {
        // Split-run pair: values the OS / CPU assign differ between the two processes, also when the
        // function itself writes them (psCameraBeginUpdate stores `rdtsc`).
        for e in INPUT_EXCLUSIONS {
            out.push(Exclusion {
                addr: e.addr,
                len: e.len,
                rule: e.rule,
            });
        }
        // known residue: excluded for every function but its writer
        for e in RESIDUE.iter().filter(|e| e.writer != c.fn_addr) {
            out.push(Exclusion {
                addr: e.addr,
                len: e.len,
                rule: e.rule,
            });
        }
    }
    if c.fn_addr != LIGHTS_CREATE {
        return out;
    }
    let mut push = |addr: u32, len: u32, rule: &'static str| {
        let e = Exclusion { addr, len, rule };
        if !out.contains(&e) {
            out.push(e);
        }
    };
    for orig in [true, false] {
        for k in 0..LIGHT_PTRS.1 {
            let Some(light) = post_word(c, LIGHT_PTRS.0 + 4 * k, orig).filter(|&p| p != 0) else {
                continue;
            };
            push(light + RPLIGHT_ALPHA, 4, RULE_LIGHT_ALPHA);
            // pDirect (k == 1): its frame is RwObject.parent (light+4).
            if k == 1
                && let Some(frame) = post_word(c, light + 4, orig).filter(|&p| p != 0)
            {
                for p in FRAME_MODELLING_PADS {
                    push(frame + p, 4, RULE_FRAME_PADS);
                }
            }
        }
    }
    out
}

// ------------------------------------------------------------------ input fingerprint exclusions ---
//
// Split-run L2 (INF-17) checks that a call's inputs are identical in the plain and the rust run, among
// them a hash of every 4 KiB page of the watched windows. Bytes that differ between two runs of the SAME
// binary for reasons outside the game (values the OS or the CRT derive from time / process ids) are
// zeroed before hashing. Each entry: evidence below, test in this file; the raw values are recorded in
// every split-run case (`Case::volatile`) and `sa-oracle l2 split-report` lists every pair in which they
// differed, so the exclusion is never silent.

/// One input range excluded from the page fingerprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputExclusion {
    pub addr: u32,
    pub len: u32,
    pub rule: &'static str,
}

/// `CCollision::ProcessColModels` (0x4185C0)'s two function-local `static CMatrix` (0x48 bytes each;
/// guard bits in 0x9689D0, constructors 0x418602/0x41862C zero `m_pAttachMatrix` / `m_bOwnsAttachedMatrix`
/// at +0x40/+0x44, `atexit` destructor thunks 0x856010 / 0x856020): 0x968940 (B's matrix in A's space,
/// 0x4186F1..0x418705) and 0x968988 (A's, 0x418671..0x418697).
pub const COL_MATRICES: [u32; 2] = [0x968940, 0x968988];

pub const RULE_COL_MATRIX_PADS: &str = "ProcessColModels' static CMatrix pads (0x968940/0x968988 + 0x0C, 0x1C, \
    0x2C, 0x3C): uninitialised stack residue, not data. Each matrix is set by CMatrix::Invert-1 \
    (0x59B920: writes +0x00..0x08, 0x10..0x18, 0x20..0x28, 0x30..0x38 only), CMatrix::operator= \
    (0x59BBC0: `rep movsd`, 16 dwords incl. the pads) and CMatrix::operator*= (0x411A80 -> 0x156FF00: \
    operator* 0x59BE30 builds the product in a local at [esp+0xC] writing right/up/at/pos only, copies \
    16 dwords incl. its never-written pads out (0x59BF8F), then operator= into the static). The pads \
    therefore hold whatever operator*'s frame held before: stack addresses, floats or 0, depending on \
    the stack depth (below a Rust ancestor: 0x96896C plain 0x0036ECDC rust 0x0036EB74; 0x96894C plain \
    0x20F3CAA0 rust 0). No code reads them except these copies. The data words (vectors, +0x40/+0x44, \
    the guard 0x9689D0) stay compared";

/// `TheCamera.m_mMatInverse` (TheCamera 0xB6F028 + 0xA4C; the six CCamera matrices start at +0x974,
/// 0x48 apart, destructors 0x50A8C2..0x50A8F2).
pub const CAMERA_MAT_INVERSE: u32 = 0xB6FA74;

pub const RULE_CAMERA_MAT_INVERSE_PADS: &str = "TheCamera.m_mMatInverse pads (0xB6FA74 + 0x1C, 0x2C, 0x3C): \
    uninitialised stack residue. 0x5150F0 sets it as `m_mMatInverse = Invert(m_mCameraMatrix)`: \
    CMatrix::Invert-2 (0x59BDD0) fills a local through Invert-1 (0x59B920, pads never written) and copies \
    16 dwords incl. the pads into its result (0x59BE10), then operator= (0x59BBC0) copies them into \
    TheCamera. Observed (Rs::RsEventHandler split run, frame 242): 0xB6FAA0 plain 0x0036F0CC rust \
    0x0036F098. RenderWare treats these words as padding (RwMatrix pad1..pad3); +0x0C (RwMatrix \
    `flags`, which RW functions may read) stays compared";

pub const INPUT_EXCLUSIONS: &[InputExclusion] = &[
    InputExclusion {
        addr: 0x8E31BC,
        len: 4,
        rule: "CRT __security_cookie (0x8E31BC): __security_init_cookie 0x8339CA-0x833A1A XORs \
               GetSystemTimeAsFileTime, GetCurrentProcessId, GetCurrentThreadId, GetTickCount and \
               QueryPerformanceCounter (default 0xBB40E64E only when the XOR is 0); per-process random, \
               read only by /GS prologues/epilogues (XOR with the frame address, compared on return)",
    },
    InputExclusion {
        addr: 0x8E4000,
        len: 4,
        rule: "gStreamingThreadId (0x8E4000): the OS thread id CreateThread writes (lpThreadId pushed \
               at 0x4069AB); OS-assigned",
    },
    InputExclusion {
        addr: 0xC8CF74,
        len: 4,
        rule: "MEMORYSTATUS.dwAvailPhys copy (0xC8CF74): GlobalMemoryStatus at 0x7475CD, stored at \
               0x7475EF; free physical memory of the machine at that moment",
    },
    InputExclusion {
        addr: 0xC8CF7C,
        len: 4,
        rule: "MEMORYSTATUS.dwAvailVirtual copy (0xC8CF7C): GlobalMemoryStatus at 0x7475CD, stored at \
               0x7475FA; free address space of the process (Rust code allocates from the same heap)",
    },
    InputExclusion {
        addr: 0xB606D0,
        len: 4,
        rule: "AEAudioHardware+0xE18 (0xB606D0; AEAudioHardware at 0xB5F8B8): an OS handle of the \
               audio stream thread (observed: two processes hold 0x2C / 0x04 after the same input; \
               gta-reversed CAEAudioHardware::m_pStreamThread area, CAEStreamThread::m_nHandle)",
    },
    InputExclusion {
        addr: 0xB608D0,
        len: 8,
        rule: "AESmoothFadeThread (0xB608D0) m_threadHandle / m_dwThreadId (gta-reversed \
               AESmoothFadeThread.h, +0 HANDLE, +4 DWORD): OS-assigned handle and thread id",
    },
    InputExclusion {
        addr: 0x9655C8,
        len: 8,
        rule: "gpColCache2 / gpColCache (0x9655C8 / 0x9655CC): 0x41B000 stores the addresses of two of its \
               own stack buffers there on every entry (`lea eax,[esp+0x50]` / `lea ecx,[esp+0x690]`, \
               0x41B016-0x41B028) before any use; the value is a stack depth, which differs below a \
               Rust ancestor (InitialiseGame: plain 0x0036E960 rust 0x0036E950)",
    },
    InputExclusion {
        addr: COL_MATRICES[0] + 0x0C,
        len: 4,
        rule: RULE_COL_MATRIX_PADS,
    },
    InputExclusion {
        addr: COL_MATRICES[0] + 0x1C,
        len: 4,
        rule: RULE_COL_MATRIX_PADS,
    },
    InputExclusion {
        addr: COL_MATRICES[0] + 0x2C,
        len: 4,
        rule: RULE_COL_MATRIX_PADS,
    },
    InputExclusion {
        addr: COL_MATRICES[0] + 0x3C,
        len: 4,
        rule: RULE_COL_MATRIX_PADS,
    },
    InputExclusion {
        addr: COL_MATRICES[1] + 0x0C,
        len: 4,
        rule: RULE_COL_MATRIX_PADS,
    },
    InputExclusion {
        addr: COL_MATRICES[1] + 0x1C,
        len: 4,
        rule: RULE_COL_MATRIX_PADS,
    },
    InputExclusion {
        addr: COL_MATRICES[1] + 0x2C,
        len: 4,
        rule: RULE_COL_MATRIX_PADS,
    },
    InputExclusion {
        addr: COL_MATRICES[1] + 0x3C,
        len: 4,
        rule: RULE_COL_MATRIX_PADS,
    },
    InputExclusion {
        addr: CAMERA_MAT_INVERSE + 0x1C,
        len: 4,
        rule: RULE_CAMERA_MAT_INVERSE_PADS,
    },
    InputExclusion {
        addr: CAMERA_MAT_INVERSE + 0x2C,
        len: 4,
        rule: RULE_CAMERA_MAT_INVERSE_PADS,
    },
    InputExclusion {
        addr: CAMERA_MAT_INVERSE + 0x3C,
        len: 4,
        rule: RULE_CAMERA_MAT_INVERSE_PADS,
    },
    InputExclusion {
        addr: 0x8E2B8C,
        len: 4,
        rule: "CRT __NLG_Destination.Frame (0x8E2B80 = {0x19930520, code, info, frame}): __NLG_Notify \
               (0x821961/0x82196A, called by the SEH/C++ EH unwinders) stores EBP there (0x82197A) for \
               debuggers; nothing in the exe reads it (only references: `mov ebx, 0x8E2B80` at 0x821963 \
               and 0x82196C). A frame address: differs with the stack depth (Rs::RsEventHandler split \
               run, frame 273: plain 0x0036EF14 rust 0x0036EEE0)",
    },
    InputExclusion {
        addr: 0xC92104,
        len: 8,
        rule: "raw time stamp counter (0xC92104/0xC92108): `rdtsc` at 0x746202, stored at \
               0x746204/0x74620A (not virtualised by determinism mode); CPU-assigned",
    },
];

// ------------------------------------------------------------------ OS handle words (HARNESS-2) ----
//
// Words that hold Win32 handle values (kernel object handles: semaphore, thread, files). Two processes
// number their handles differently as soon as one of them has opened one handle more (observed in the
// PORT-WEAPON-PED split runs c-split2..4, g-split3, w-split2, p-split2: gStreamSemaphore plain 0xE8 rust
// 0xE4, gStreamFileHandles[0] plain 0xD0 rust 0xCC, in every pair from frame 201; the same list was
// identical in g-split2 and in HARNESS-2's hz-e1c). They are not excluded: each word is compared by its
// class (0, INVALID_HANDLE_VALUE, or "a handle": non-zero, a multiple of 4, below 0x10000, as NT and
// Wine hand them out), so a port that stores no handle, a different sentinel or a pointer still
// differs. The page fingerprint hashes the class, the merged comparison compares the class, and the
// raw values are recorded per case (`Case::volatile`) and listed by `split-report`.

/// One range of handle words (4-aligned dwords).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandleWords {
    pub addr: u32,
    pub len: u32,
    pub rule: &'static str,
}

pub const OS_HANDLES: &[HandleWords] = &[
    HandleWords {
        addr: 0x8E4004,
        len: 8,
        rule: "CdStream gStreamSemaphore (0x8E4004) and the streaming thread handle (0x8E4008): \
               CdStreamInitThread (0x4068F0) stores the CreateSemaphoreA result at 0x406978 \
               (`mov [0x8E4004], eax` after `call ebx`) and the CreateThread result (`call [0x8580B4]`, \
               lpThreadId = 0x8E4000) at 0x4069C8; OS handles (observed 0xE8 / 0xE4)",
    },
    HandleWords {
        addr: 0x8E4010,
        len: 0x80,
        rule: "CdStream gStreamFileHandles[32] (0x8E4010..0x8E408F; next global 0x8E4090): the CreateFileA \
               results of CdStreamOpen (0x4067B0) / the reopen loop (`mov [edi*4+0x8E4010], eax` at \
               0x406794), zeroed by CdStreamRemoveImages / CdStreamShutdown (0x4068CD, 0x4066EA after \
               CloseHandle); OS handles (observed 0xD0 / 0xCC)",
    },
    HandleWords {
        addr: 0xB606CC,
        len: 4,
        rule: "CAEStreamThread::m_nHandle of the audio stream thread object 0xB606CC (sa-hook det status \
               `audio stream_thread=0xb606cc`): CAEStreamThread::Initialise (0x4F1680) calls CreateThread \
               (0x4F16E6, lpThreadId = this+4 = 0xB606D0, already an input exclusion) and stores the \
               handle at 0x4F16EE (`mov [esi], eax`); the only handle-valued word of page 0xB60000 besides \
               the excluded 0xB606D0 / 0xB608D0 / 0xB608D4 (hz-e2g window dump seq 1500: 0x1E8 in both \
               runs). Page 0xB60000 is the second page the PORT-WEAPON-PED PASS-FP runs listed, always \
               together with the 0x8E4000 handle shift",
    },
];

/// Handle fields of a heap array that a global points to (compared by class like [`OS_HANDLES`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeapHandles {
    /// The global holding the array's address (must be in the case: a spec global or an extra region).
    pub ptr: u32,
    pub stride: u32,
    /// Offsets of the handle words inside one element.
    pub fields: &'static [u32],
    pub rule: &'static str,
}

pub const HEAP_HANDLES: &[HeapHandles] = &[HeapHandles {
    ptr: 0x8E3FFC,
    stride: 0x30,
    fields: &[0x14, 0x18],
    rule: "gCdStreams (*0x8E3FFC, LocalAlloc'd by CdStreamInit, 0x30-byte CdStream elements, gta-reversed \
           CdStreamInfo.h): +0x14 the request semaphore, +0x18 hFile; OS handles. HARNESS-2 hz-f3g (core \
           split list, grove): the extra region [0x8E3FFC]:0xF0 differed at +0x14 plain 0xD4 rust 0xD0 and \
           +0x18 plain 0xD0 rust 0xCC before every pair from frame 0 (the one-handle shift)",
}];

/// The heap handle words of `c`'s region at `addr` (the region must start at the array, i.e. at the
/// value the case holds for the pointer global): absolute word addresses.
pub fn heap_handle_words(c: &crate::l2::Case, addr: u32, len: usize) -> Vec<u32> {
    let mut out = Vec::new();
    for h in HEAP_HANDLES {
        let p = c.regions.iter().filter(|r| r.readable).find_map(|r| {
            let o = h.ptr.checked_sub(r.addr)? as usize;
            let b = r.pre.get(o..o + 4)?;
            Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        });
        if p != Some(addr) || addr == 0 {
            continue;
        }
        let mut base = 0u32;
        while (base as usize) < len {
            for &f in h.fields {
                if (base + f) as usize + 4 <= len {
                    out.push(addr + base + f);
                }
            }
            base += h.stride;
        }
    }
    out
}

/// [`classify_handles`] plus the heap handle words of `c` inside `b` (at `addr`).
pub fn classify_case_handles(c: &crate::l2::Case, addr: u32, b: &mut [u8]) {
    classify_handles(addr, b);
    for w in heap_handle_words(c, addr, b.len()) {
        let o = (w - addr) as usize;
        let v = handle_class(u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]));
        b[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
}

/// The class of a handle word: 0, INVALID_HANDLE_VALUE, 4 for any plausible handle, else the value.
pub fn handle_class(v: u32) -> u32 {
    match v {
        0 | u32::MAX => v,
        _ if v & 3 == 0 && v < 0x10000 => 4,
        _ => v,
    }
}

/// The handle-word range holding `a`, if any.
pub fn os_handle_at(a: u32) -> Option<&'static HandleWords> {
    OS_HANDLES.iter().find(|h| a.wrapping_sub(h.addr) < h.len)
}

/// Every handle word of `b` (at `addr`, whole aligned words inside `b` only) replaced by its class.
pub fn classify_handles(addr: u32, b: &mut [u8]) {
    for h in OS_HANDLES {
        let mut w = h.addr;
        while w < h.addr + h.len {
            if w >= addr && (w - addr) as usize + 4 <= b.len() {
                let o = (w - addr) as usize;
                let c = handle_class(u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]));
                b[o..o + 4].copy_from_slice(&c.to_le_bytes());
            }
            w += 4;
        }
    }
}

// ------------------------------------------------------------------ known residue (HARNESS-2) -------
//
// Bytes that an ORIGINAL function leaves as uninitialised stack residue and its port writes as 0 (a
// divergence-ledger item, PLAN §5): the bytes differ between a split run's plain and rust process from
// the moment the port has run. They are excluded from the page fingerprint and from the comparison of
// every OTHER function's outputs (which only see them as unrelated state); the writer's own comparison
// keeps them, so the ledger difference stays visible there.

/// One residue range and the function that writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Residue {
    pub addr: u32,
    pub len: u32,
    /// The function whose own comparison keeps the bytes.
    pub writer: u32,
    pub rule: &'static str,
}

/// `CModelInfo::ms_vehicleModelInfoStore` (0xB1F650: count, then CVehicleModelInfo[] of 0x308 bytes).
pub const VEHICLE_MODEL_INFO_STORE: u32 = 0xB1F650;
/// The skimmer's slot (vehicles.ide id 460, the 61st vehicle line: store index 60 = LoadVehicleObject's
/// call ordinal 60 at frame 201).
pub const SKIMMER_MODEL_INFO: u32 = VEHICLE_MODEL_INFO_STORE + 4 + 60 * 0x308;

pub const RESIDUE: &[Residue] = &[Residue {
    addr: SKIMMER_MODEL_INFO + 0x40,
    len: 10,
    writer: 0x5B6F30,
    rule: "CFileLoader::LoadVehicleObject (0x5B6F30), skimmer (vehicles.ide 460, 11 fields): the original \
           stores uninitialised locals as m_fWheelSizeFront / m_fWheelSizeRear / the wheel model index \
           (CVehicleModelInfo +0x40, +0x44, int16 +0x48 = 0xB2AC74..0xB2AC7D), the port writes 0 \
           (ledger: STREAM-L2 frame 201 ordinal 60: +0x40 = 8, +0x44 = 0x0036F144; HARNESS-2 hz-e1c \
           window dumps seq 14..1500: 0xB2AC78 plain 3333333fffff rust 000000000000, the only \
           differing bytes of page 0xB2A000)",
}];

/// `page` (at `addr`) with the excluded input bytes zeroed and the handle words replaced by their
/// class (a copy only when one lies inside).
pub fn fingerprint_bytes(addr: u32, page: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    let end = addr as u64 + page.len() as u64;
    let overlaps = |a: u32, n: u32| (a as u64) < end && a as u64 + n as u64 > addr as u64;
    let zero: Vec<(u32, u32)> = INPUT_EXCLUSIONS
        .iter()
        .map(|e| (e.addr, e.len))
        .chain(RESIDUE.iter().map(|e| (e.addr, e.len)))
        .filter(|&(a, n)| overlaps(a, n))
        .collect();
    let handles = OS_HANDLES.iter().any(|h| overlaps(h.addr, h.len));
    if zero.is_empty() && !handles {
        return std::borrow::Cow::Borrowed(page);
    }
    let mut v = page.to_vec();
    for (ea, en) in zero {
        for k in 0..en {
            let a = ea as u64 + k as u64;
            if a >= addr as u64 && a < end {
                v[(a - addr as u64) as usize] = 0;
            }
        }
    }
    if handles {
        classify_handles(addr, &mut v);
    }
    std::borrow::Cow::Owned(v)
}

/// Every reviewed input range whose raw bytes a split-run case records (`Case::volatile`), in a fixed
/// order: the input exclusions, the handle words, the residue ranges.
pub fn volatile_ranges() -> Vec<(u32, u32, &'static str)> {
    INPUT_EXCLUSIONS
        .iter()
        .map(|e| (e.addr, e.len, e.rule))
        .chain(OS_HANDLES.iter().map(|e| (e.addr, e.len, e.rule)))
        .chain(RESIDUE.iter().map(|e| (e.addr, e.len, e.rule)))
        .collect()
}

/// Byte `a` is zeroed in every split-run input comparison (input exclusion or residue).
pub fn input_excluded(a: u32) -> bool {
    INPUT_EXCLUSIONS.iter().any(|e| a.wrapping_sub(e.addr) < e.len)
        || RESIDUE.iter().any(|e| a.wrapping_sub(e.addr) < e.len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsys::{PoolHdr, addr};

    #[test]
    fn security_cookie_is_excluded_from_the_fingerprint_only() {
        let mut a = vec![7u8; 4096];
        let mut b = a.clone();
        a[0x1BC..0x1C0].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        b[0x1BC..0x1C0].copy_from_slice(&0x9ABC_DEF0u32.to_le_bytes());
        assert_eq!(fingerprint_bytes(0x8E3000, &a), fingerprint_bytes(0x8E3000, &b));
        b[0x1C0] = 1;
        assert_ne!(fingerprint_bytes(0x8E3000, &a), fingerprint_bytes(0x8E3000, &b));
        // another page: untouched
        assert!(matches!(
            fingerprint_bytes(0x8E5000, &a),
            std::borrow::Cow::Borrowed(_)
        ));
        // the streaming thread id at the start of the next page is excluded too
        assert_eq!(&fingerprint_bytes(0x8E4000, &a)[..8], &[0, 0, 0, 0, 7, 7, 7, 7]);
        // a page that starts inside the range
        let c = vec![1u8; 8];
        assert_eq!(&*fingerprint_bytes(0x8E31BE, &c), &[0, 0, 1, 1, 1, 1, 1, 1]);
    }

    /// ProcessColModels' static matrices: exactly the four pad words of each are excluded, the
    /// vectors, `m_pAttachMatrix`, `m_bOwnsAttachedMatrix` and the guard word are not.
    #[test]
    fn col_matrix_pads_are_excluded_and_nothing_else() {
        let excluded = |a: u32| INPUT_EXCLUSIONS.iter().any(|e| a.wrapping_sub(e.addr) < e.len);
        let mut got = Vec::new();
        for a in 0x968940..0x9689D4u32 {
            if excluded(a) {
                got.push(a);
            }
        }
        let want: Vec<u32> = [
            0x96894C, 0x96895C, 0x96896C, 0x96897C, 0x968994, 0x9689A4, 0x9689B4, 0x9689C4,
        ]
        .iter()
        .flat_map(|&w| w..w + 4)
        .collect();
        assert_eq!(got, want);
        // the page fingerprint ignores them (the reported pairs: 0x96896C, 0x96894C) ...
        let page = 0x968000;
        let mut a = vec![0x11u8; 4096];
        let mut b = a.clone();
        a[0x96C..0x970].copy_from_slice(&0x0036_ECDCu32.to_le_bytes());
        b[0x96C..0x970].copy_from_slice(&0x0036_EB74u32.to_le_bytes());
        a[0x94C..0x950].copy_from_slice(&0x20F3_CAA0u32.to_le_bytes());
        b[0x94C..0x950].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(fingerprint_bytes(page, &a), fingerprint_bytes(page, &b));
        // ... but not a matrix element next to them (pos.z of 0x968940 at +0x38)
        b[0x978] ^= 1;
        assert_ne!(fingerprint_bytes(page, &a), fingerprint_bytes(page, &b));
    }

    /// TheCamera.m_mMatInverse: pad1..pad3 excluded, `flags` (+0x0C) and the vectors are not.
    #[test]
    fn camera_mat_inverse_pads_are_excluded_and_nothing_else() {
        let excluded = |a: u32| INPUT_EXCLUSIONS.iter().any(|e| a.wrapping_sub(e.addr) < e.len);
        let got: Vec<u32> = (CAMERA_MAT_INVERSE..CAMERA_MAT_INVERSE + 0x48)
            .filter(|&a| excluded(a))
            .collect();
        let want: Vec<u32> = [0xB6FA90, 0xB6FAA0, 0xB6FAB0]
            .iter()
            .flat_map(|&w| w..w + 4)
            .collect();
        assert_eq!(got, want);
        assert!(!excluded(0xB6FA80) && !excluded(0xB6FAA4));
        // __NLG_Destination: only the frame word, not the signature/code/info words
        let got: Vec<u32> = (0x8E2B80..0x8E2B90).filter(|&a| excluded(a)).collect();
        assert_eq!(got, (0x8E2B8C..0x8E2B90).collect::<Vec<u32>>());
    }

    /// gStreamSemaphore / thread handle / gStreamFileHandles: hashed and compared by class, nothing
    /// around them.
    #[test]
    fn os_handle_words_compare_by_class() {
        assert_eq!(handle_class(0xE8), handle_class(0xE4));
        assert_ne!(handle_class(0xE8), handle_class(0));
        assert_ne!(handle_class(0xE8), handle_class(u32::MAX));
        assert_ne!(handle_class(0xE8), handle_class(0xE9), "not a multiple of 4");
        assert_ne!(
            handle_class(0xE8),
            handle_class(0x0036_EE48),
            "a pointer is not a handle"
        );
        let covered: Vec<u32> = (0x8E3FFC..0x8E4098)
            .filter(|&a| os_handle_at(a).is_some())
            .collect();
        assert_eq!(
            covered,
            (0x8E4004..0x8E400C)
                .chain(0x8E4010..0x8E4090)
                .collect::<Vec<u32>>()
        );
        let covered: Vec<u32> = (0xB60600..0xB60900)
            .filter(|&a| os_handle_at(a).is_some())
            .collect();
        assert_eq!(covered, (0xB606CC..0xB606D0).collect::<Vec<u32>>());
        // the page fingerprint: plain 0xE8/0xD0, rust 0xE4/0xCC (PORT-WEAPON-PED c-split3) hash alike
        let mut a = vec![0u8; 4096];
        a[0..4].copy_from_slice(&0x1DCu32.to_le_bytes()); // gStreamingThreadId (excluded)
        let mut b = a.clone();
        a[4..8].copy_from_slice(&0xE8u32.to_le_bytes());
        b[4..8].copy_from_slice(&0xE4u32.to_le_bytes());
        a[0x10..0x14].copy_from_slice(&0xD0u32.to_le_bytes());
        b[0x10..0x14].copy_from_slice(&0xCCu32.to_le_bytes());
        assert_eq!(fingerprint_bytes(0x8E4000, &a), fingerprint_bytes(0x8E4000, &b));
        // a closed handle (0) against an open one, and the next global (0x8E4090), still differ
        b[0x14..0x18].copy_from_slice(&0xC8u32.to_le_bytes());
        assert_ne!(fingerprint_bytes(0x8E4000, &a), fingerprint_bytes(0x8E4000, &b));
        b[0x14..0x18].copy_from_slice(&[0; 4]);
        b[0x90] = 1;
        assert_ne!(fingerprint_bytes(0x8E4000, &a), fingerprint_bytes(0x8E4000, &b));
        // merged split-run outputs: a region over the handles compares by class too
        use crate::l2::{Case, Region, RegionKind};
        let mut c = Case {
            fn_addr: 0x4067B0,
            merged: true,
            regions: vec![Region {
                addr: 0x8E4010,
                kind: RegionKind::Global { exact: false },
                access: 3,
                readable: true,
                pre: vec![0; 8],
                post_orig: [0xD0u32, 0].iter().flat_map(|w| w.to_le_bytes()).collect(),
                post_rust: [0xCCu32, 0].iter().flat_map(|w| w.to_le_bytes()).collect(),
            }],
            ..Case::default()
        };
        assert!(crate::l2::compare(&c).is_empty());
        c.regions[0].post_rust[4] = 0xC8;
        assert_eq!(
            crate::l2::compare(&c).len(),
            1,
            "a handle where the original left 0"
        );
        c.regions[0].post_rust[4] = 0;
        c.merged = false;
        assert_eq!(crate::l2::compare(&c).len(), 1, "in-process L2 compares raw");
    }

    /// gCdStreams' semaphore and file handles (heap, *0x8E3FFC): by class, only when the case holds the
    /// pointer and the region starts at the array.
    #[test]
    fn heap_handle_words_follow_the_pointer() {
        use crate::l2::{Case, Region, RegionKind};
        let reg = |addr: u32, pre: Vec<u8>| Region {
            addr,
            kind: RegionKind::Extra,
            access: 3,
            readable: true,
            post_orig: pre.clone(),
            post_rust: pre.clone(),
            pre,
        };
        let arr = 0x0348_E048u32;
        let c = Case {
            regions: vec![reg(0x8E3FFC, arr.to_le_bytes().to_vec()), reg(arr, vec![0; 0xF0])],
            ..Case::default()
        };
        let w = heap_handle_words(&c, arr, 0xF0);
        assert_eq!(w.len(), 10);
        assert_eq!(&w[..4], &[arr + 0x14, arr + 0x18, arr + 0x44, arr + 0x48]);
        assert!(heap_handle_words(&c, arr + 4, 0xF0).is_empty(), "another region");
        let mut a = vec![0u8; 0xF0];
        let mut b = a.clone();
        a[0x14..0x1C].copy_from_slice(&[0xD4, 0, 0, 0, 0xD0, 0, 0, 0]);
        b[0x14..0x1C].copy_from_slice(&[0xD0, 0, 0, 0, 0xCC, 0, 0, 0]);
        classify_case_handles(&c, arr, &mut a);
        classify_case_handles(&c, arr, &mut b);
        assert_eq!(a, b);
        b[0x10] = 1; // status: still compared
        assert_ne!(a, b);
        let no_ptr = Case {
            regions: vec![reg(arr, vec![0; 0xF0])],
            ..Case::default()
        };
        assert!(heap_handle_words(&no_ptr, arr, 0xF0).is_empty());
    }

    /// The skimmer residue: only those 10 bytes, excluded for every function but LoadVehicleObject.
    #[test]
    fn skimmer_residue_is_narrow_and_kept_for_its_writer() {
        assert_eq!(SKIMMER_MODEL_INFO + 0x44, 0xB2AC78, "hz-e1c dump address");
        let ex: Vec<u32> = (0xB2AC00..0xB2AD00).filter(|&a| input_excluded(a)).collect();
        assert_eq!(ex, (0xB2AC74..0xB2AC7E).collect::<Vec<u32>>());
        let mut a = vec![0u8; 4096];
        let mut b = a.clone();
        a[0xC78..0xC7E].copy_from_slice(&[0x33, 0x33, 0x33, 0x3F, 0xFF, 0xFF]);
        assert_eq!(fingerprint_bytes(0xB2A000, &a), fingerprint_bytes(0xB2A000, &b));
        b[0xC7E] = 1; // the next field
        assert_ne!(fingerprint_bytes(0xB2A000, &a), fingerprint_bytes(0xB2A000, &b));
        let mut c = crate::l2::Case {
            merged: true,
            fn_addr: 0x5B6F30,
            ..Default::default()
        };
        assert!(
            !exclusions(&c).iter().any(|e| e.addr == 0xB2AC74),
            "the writer keeps them"
        );
        c.fn_addr = 0x5E4500;
        assert!(exclusions(&c).iter().any(|e| e.addr == 0xB2AC74 && e.len == 10));
        c.merged = false;
        assert!(
            !exclusions(&c).iter().any(|e| e.addr == 0xB2AC74),
            "split-run pairs only"
        );
    }

    #[test]
    fn lights_create_exclusions_are_narrow() {
        use crate::l2::{Case, Region, RegionKind};
        let mut ptrs = vec![0u8; 32];
        ptrs[4..8].copy_from_slice(&0x2000_0000u32.to_le_bytes()); // pDirect
        ptrs[0..4].copy_from_slice(&0x2000_0100u32.to_le_bytes()); // pAmbient
        let mut light = vec![0u8; 0x30];
        light[4..8].copy_from_slice(&0x3000_0000u32.to_le_bytes()); // frame
        let reg = |addr: u32, kind, post: Vec<u8>| Region {
            addr,
            kind,
            access: 3,
            readable: true,
            pre: if kind == RegionKind::Post {
                vec![]
            } else {
                vec![0; post.len()]
            },
            post_orig: post.clone(),
            post_rust: post,
        };
        let mut c = Case {
            fn_addr: LIGHTS_CREATE,
            regions: vec![
                reg(0xC886E8, RegionKind::Global { exact: true }, ptrs),
                reg(0x2000_0000, RegionKind::Post, light),
            ],
            ..Case::default()
        };
        let e = exclusions(&c);
        let addrs: Vec<u32> = e.iter().map(|x| x.addr).collect();
        assert_eq!(
            addrs,
            vec![0x2000_0124, 0x2000_0024, 0x3000_002C, 0x3000_003C, 0x3000_004C]
        );
        // alpha differs, everything else equal: identical after exclusion; a red difference is not
        c.regions[1].post_rust[0x24] = 0x50;
        assert!(crate::l2::compare(&c).is_empty());
        c.regions[1].post_rust[0x18] = 1;
        assert_eq!(crate::l2::compare(&c).len(), 1);
        c.fn_addr = 0x5BA521;
        assert!(exclusions(&c).is_empty(), "other functions get no exclusion");
    }

    fn ctx(allocs: Vec<(u32, u32)>) -> NormCtx {
        let mut pools = PoolMap {
            pools: vec![PoolHdr::default(); addr::POOL_COUNT as usize],
        };
        pools.pools[3] = PoolHdr {
            objects: 0x3000_0000,
            bytemap: 0x3100_0000,
            size: 140,
        };
        NormCtx {
            image: (0x400000, 0x1577000),
            pools,
            model_infos: vec![0, 0x00A0_0000, 0x00A0_1000],
            allocs,
            stack: (0, 0),
            esp: 0,
            frames: Vec::new(),
            foreign: Vec::new(),
        }
    }

    #[test]
    fn classes() {
        let c = ctx(vec![(0x2000_0100, 0x40)]);
        assert_eq!(c.normalise(5), Norm::Raw(5));
        assert_eq!(
            c.normalise(0x2000_0110),
            Norm::Alloc {
                ordinal: 0,
                off: 0x10
            }
        );
        assert_eq!(
            c.normalise(0x3000_0000 + 2 * addr::PED_STRIDE + 8),
            Norm::Pool {
                pool: 3,
                slot: 2,
                off: 8
            }
        );
        assert_eq!(c.normalise(0x00A0_1004), Norm::Model { id: 2, off: 4 });
        assert_eq!(c.normalise(0x00B7_CD98), Norm::Image { off: 0x77CD98 });
        assert_eq!(c.normalise(0x00A0_1020), Norm::Image { off: 0x601020 });
    }

    #[test]
    fn stack_pointers_compare_by_offset_from_the_call() {
        let mut a = ctx(vec![]);
        let mut b = ctx(vec![]);
        assert!(
            !words_equal(0x36F208, &a, 0x36F1D8, &b),
            "no stack class without esp"
        );
        (a.stack, a.esp) = ((0x270000, 0x370000), 0x36F1F0);
        (b.stack, b.esp) = ((0x270000, 0x370000), 0x36F1C0);
        assert_eq!(a.normalise(0x36F208), Norm::Stack { frame: 0, off: 0x18 });
        assert!(words_equal(0x36F208, &a, 0x36F1D8, &b));
        assert!(!words_equal(0x36F208, &a, 0x36F1DC, &b), "different variable");
        assert!(!words_equal(0x36F208, &a, 0x5000, &b), "a non-stack value");
        assert!(!words_equal(0x1234, &a, 0x1235, &b));
    }

    /// HARNESS-3: a stack address above an enclosing recorded call moved by that call's shift. The
    /// numbers of PORT-WEAPON-2 split-combat (docs/arch/weapon-ped-port.md): FireInstantHit's
    /// `CEntity*` slot registered in CReferences, plain 0x0036F06C rust 0x0036EF5C (0x110 apart: the
    /// shift of the enclosing DoBulletImpact call), seen by GenerateDamageEvent whose own argument
    /// blocks are 0x124 apart (the Rust DoBulletImpact's frame is 0x14 larger).
    #[test]
    fn stack_addresses_above_an_enclosing_call_compare_by_its_frame() {
        let mut a = ctx(vec![]);
        let mut b = ctx(vec![]);
        (a.stack, a.esp) = ((0x270000, 0x370000), 0x36EE00);
        (b.stack, b.esp) = ((0x270000, 0x370000), 0x36EE00 - 0x124);
        // only the case's own block: the slot is 0x110 apart, the blocks 0x124: different
        assert!(!words_equal(0x36F06C, &a, 0x36EF5C, &b));
        // DoBulletImpact's argument block (the enclosing recorded call) in each run
        a.frames = vec![0x36F000];
        b.frames = vec![0x36F000 - 0x110];
        assert_eq!(a.normalise(0x36F06C), Norm::Stack { frame: 1, off: 0x6C });
        assert!(words_equal(0x36F06C, &a, 0x36EF5C, &b));
        assert!(
            !words_equal(0x36F06C, &a, 0x36EF60, &b),
            "another variable of that frame"
        );
        // below the enclosing block (the Rust DoBulletImpact's own frame): the case's own shift
        assert_eq!(a.normalise(0x36EE10), Norm::Stack { frame: 0, off: 0x10 });
        assert!(words_equal(0x36EE10, &a, 0x36EE10 - 0x124, &b));
        assert!(!words_equal(0x36EE10, &a, 0x36EE10 - 0x110, &b));
        // a block not known in one run, or a slot on different sides of the block: different
        b.frames = vec![0];
        assert!(!words_equal(0x36F06C, &a, 0x36EF5C, &b));
        b.frames = vec![0x36EF60];
        assert!(!words_equal(0x36F06C, &a, 0x36EF5C, &b));
        // the nearest enclosing call is used (two levels)
        a.frames = vec![0x36F000, 0x36F100];
        b.frames = vec![0x36F000 - 0x110, 0x36F100 - 0x40];
        assert!(words_equal(0x36F120, &a, 0x36F120 - 0x40, &b));
        assert!(!words_equal(0x36F120, &a, 0x36F120 - 0x110, &b));
        assert!(words_equal(0x36F06C, &a, 0x36EF5C, &b));
    }

    #[test]
    fn foreign_objects_are_equal_only_to_foreign_objects() {
        let mut a = ctx(vec![]);
        let mut b = ctx(vec![]);
        assert!(
            !words_equal(0x0301_A530, &a, 0x0301_A340, &b),
            "no class without the lists"
        );
        a.foreign = vec![0x0301_A530, 0x05F0_C5E0];
        b.foreign = vec![0x0301_A340];
        assert_eq!(a.normalise(0x0301_A530), Norm::Foreign);
        assert!(words_equal(0x0301_A530, &a, 0x0301_A340, &b));
        assert!(!words_equal(0x0301_A530, &a, 0, &b), "an OS object vs null");
        assert!(
            !words_equal(0x0301_A531, &a, 0x0301_A340, &b),
            "not in the plain run's list"
        );
        assert!(
            !words_equal(0x0301_A530, &a, 0x00B7_CD98, &b),
            "an OS object vs a game global"
        );
    }

    #[test]
    fn allocation_ordinals_make_different_addresses_equal() {
        let (a, b) = (ctx(vec![(0x2000_0100, 0x40)]), ctx(vec![(0x2000_0200, 0x40)]));
        assert!(words_equal(0x2000_0104, &a, 0x2000_0204, &b));
        assert!(!words_equal(0x2000_0104, &a, 0x2000_0208, &b));
        // A raw value that is no allocation in either run must be identical.
        assert!(!words_equal(0x1234, &a, 0x1235, &b));
        // Pool / image pointers: raw equal only.
        assert!(!words_equal(0x3000_0000, &a, 0x3000_0000 + addr::PED_STRIDE, &b));
        let mut x = vec![0u8; 12];
        let mut y = vec![0u8; 12];
        x[4..8].copy_from_slice(&0x2000_0104u32.to_le_bytes());
        y[4..8].copy_from_slice(&0x2000_0204u32.to_le_bytes());
        assert_eq!(bytes_equal(0x1000, &x, &a, &y, &b), None);
        y[9] = 1;
        assert_eq!(bytes_equal(0x1000, &x, &a, &y, &b), Some(9));
        // A word cut by the buffer start is never normalised (raw comparison).
        assert_eq!(bytes_equal(0x1005, &x[5..], &a, &y[5..], &b), Some(0));
        assert_eq!(bytes_equal(0x1002, &x[2..], &a, &y[2..], &b), Some(7));
    }
}
