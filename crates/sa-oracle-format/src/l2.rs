//! L2 oracle data (INF-8): function signatures, the per-function read/write-set spec, the comparison
//! case (inputs + the original's outputs + the Rust implementation's outputs) with its binary encoding,
//! and the comparison itself. Shared by `sa-hook` (in-process: capture, compare, spool) and `sa-oracle`
//! (host: spec generation from the catalogue, fixture store, replay without the game).
//!
//! Comparison rules (what "identical after normalisation" means, PLAN §5 L2):
//! - return value per the declared [`Ret`]: EAX, AL (bool: MSVC callers only test AL), EAX:EDX, or the
//!   x87 stack (depth and every value's full 80 bits); the x87 depth is compared for every function;
//! - callee-saved EBX/ESI/EDI/EBP and the bytes popped by the callee (calling convention);
//! - x87 control word, MXCSR control bits (0xFFC0; the sticky flags are never read by game code);
//! - every snapshotted region after the call (globals of the read/write set, `this`, pointed-to args);
//! - every byte written in the watched windows (writable exe sections, configurable), so a write
//!   outside the declared set is both compared and reported ([`undeclared_writes`]);
//! - the sequence of hooked callees entered (when the callee log is on).
//!
//! Words are compared through [`crate::normalise`] (the one reviewed normaliser).
//! Not compared (documented): x87 status exception flags and condition codes (masked exceptions, never
//! read without a compare first), EFLAGS after return, caller-saved ECX/EDX for non-EAX:EDX returns.

use crate::normalise::{self, NormCtx};
use crate::subsys::{PoolHdr, PoolMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conv {
    Cdecl = 0,
    Stdcall = 1,
    Thiscall = 2,
    Fastcall = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ret {
    Void = 0,
    Eax = 1,
    /// bool in AL
    Al = 2,
    /// float/double in ST0
    St0 = 3,
    EaxEdx = 4,
}

/// A pointer argument whose pointee is part of the read/write set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PtrArg {
    /// Stack argument word index.
    pub arg: u8,
    /// Size of the pointee in bytes.
    pub size: u32,
}

/// What the L2 oracle needs to call a function generically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sig {
    pub conv: Conv,
    /// Stack argument words (after ECX/EDX for thiscall/fastcall).
    pub args: u8,
    pub ret: Ret,
    /// `this` type (layouts.csv name) for thiscall; its size bounds the `this` snapshot.
    pub this_type: Option<&'static str>,
    pub this_size: u32,
    pub ptr_args: &'static [PtrArg],
}

impl Sig {
    /// Bytes the callee pops on return.
    pub fn callee_pops(&self) -> u32 {
        match self.conv {
            Conv::Cdecl => 0,
            Conv::Stdcall | Conv::Thiscall | Conv::Fastcall => self.args as u32 * 4,
        }
    }
}

// ------------------------------------------------------------------ spec (read/write set) ----------

pub const ACC_R: u8 = 1;
pub const ACC_W: u8 = 2;

/// One global of a function's read/write set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Global {
    pub addr: u32,
    pub len: u32,
    pub access: u8,
    /// `false` for catalogue entries accessed through a base/index (array, struct field via pointer):
    /// the access starts at `addr` but its extent is not known statically.
    pub exact: bool,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FnSpec {
    pub addr: u32,
    pub name: String,
    pub globals: Vec<Global>,
}

/// Text form, one spec per function (written by `sa-oracle l2 spec`, read by sa-hook via `SA_L2_SPEC`):
/// ```text
/// fn 0x561b10 CTimer::Update
/// g 0xb7cb4c 4 RW exact CTimer::m_FrameCounter
/// g 0xa9b0c8 4 R unbounded CModelInfo::ms_modelInfoPtrs
/// ```
pub fn parse_spec(text: &str) -> Result<Vec<FnSpec>, String> {
    let mut out: Vec<FnSpec> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let w: Vec<&str> = t.split_whitespace().collect();
        let err = || format!("spec line {}: {t:?}", n + 1);
        match w[0] {
            "fn" if w.len() >= 2 => out.push(FnSpec {
                addr: parse_u32(w[1]).ok_or_else(err)?,
                name: w.get(2).unwrap_or(&"").to_string(),
                globals: Vec::new(),
            }),
            "g" if w.len() >= 5 => {
                let f = out.last_mut().ok_or_else(err)?;
                let access = match w[3] {
                    "R" => ACC_R,
                    "W" => ACC_W,
                    "RW" => ACC_R | ACC_W,
                    _ => return Err(err()),
                };
                f.globals.push(Global {
                    addr: parse_u32(w[1]).ok_or_else(err)?,
                    len: parse_u32(w[2]).ok_or_else(err)?,
                    access,
                    exact: match w[4] {
                        "exact" => true,
                        "unbounded" => false,
                        _ => return Err(err()),
                    },
                    name: w.get(5).unwrap_or(&"-").to_string(),
                });
            }
            _ => return Err(err()),
        }
    }
    Ok(out)
}

pub fn format_spec(specs: &[FnSpec]) -> String {
    let mut s =
        String::from("# sa-oracle l2 spec v1: fn ADDR NAME / g ADDR LEN R|W|RW exact|unbounded NAME\n");
    for f in specs {
        s += &format!("fn {:#x} {}\n", f.addr, f.name);
        for g in &f.globals {
            let acc = match g.access {
                ACC_R => "R",
                ACC_W => "W",
                _ => "RW",
            };
            let name = if g.name.is_empty() { "-" } else { &g.name };
            s += &format!(
                "g {:#x} {} {acc} {} {name}\n",
                g.addr,
                g.len,
                if g.exact { "exact" } else { "unbounded" }
            );
        }
    }
    s
}

pub fn parse_u32(s: &str) -> Option<u32> {
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(h) => u32::from_str_radix(h, 16).ok(),
        None => s.parse().ok(),
    }
}

// ------------------------------------------------------------------ case ----------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionKind {
    /// A global of the spec (`exact` from the spec).
    Global {
        exact: bool,
    },
    This,
    ArgPtr(u8),
    /// From the configurable extra list.
    Extra,
    /// Extra region whose address is evaluated after the run (exit registers, e.g. a returned object):
    /// `pre` is empty, only the post bytes are compared.
    Post,
    /// Observable OS state outside process memory (not bytes at `addr`): an encoded probe record
    /// ([`OsProbe`]), compared raw (never normalised). `addr` is the probed object (FILE*), or 0.
    Os(OsProbe),
}

/// OS-effect probes (INF-17 split-run L2): state a function changes outside game memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OsProbe {
    /// A CRT stream (`FILE*` at `addr`): the FILE block, its descriptor's CRT flags and the OS file
    /// behind it (path, position, size). The OS handle value itself is never recorded.
    File = 1,
    /// The process' current directory.
    Cwd = 2,
    /// The cursor's show counter state (`GetCursorInfo` flags).
    Cursor = 3,
}

impl OsProbe {
    pub fn from_u8(v: u8) -> Option<OsProbe> {
        match v {
            1 => Some(OsProbe::File),
            2 => Some(OsProbe::Cwd),
            3 => Some(OsProbe::Cursor),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            OsProbe::File => "file",
            OsProbe::Cwd => "cwd",
            OsProbe::Cursor => "cursor",
        }
    }
}

/// Decoded file probe record (see `sa-hook` l2win `probe_file`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileProbe {
    /// The 32-byte CRT `FILE` block (empty when unreadable).
    pub file: Vec<u8>,
    /// `_file` (CRT descriptor), -1 when the FILE is unreadable.
    pub fd: i32,
    /// `ioinfo.osfile` of the descriptor (0x01 = FOPEN), 0 when out of range.
    pub osfile: u8,
    /// The OS file: present when the descriptor is open.
    pub os: Option<(u64, u64, String)>,
}

impl FileProbe {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = W(Vec::new());
        w.bytes(&self.file);
        w.u32(self.fd as u32);
        w.u8(self.osfile);
        match &self.os {
            Some((pos, size, path)) => {
                w.u8(1);
                w.0.extend_from_slice(&pos.to_le_bytes());
                w.0.extend_from_slice(&size.to_le_bytes());
                w.bytes(path.as_bytes());
            }
            None => w.u8(0),
        }
        w.0
    }
    pub fn decode(b: &[u8]) -> Result<FileProbe, String> {
        let mut r = R(crate::Reader::new(b));
        let mut p = FileProbe {
            file: r.bytes()?,
            fd: r.u32()? as i32,
            osfile: r.u8()?,
            os: None,
        };
        if r.u8()? == 1 {
            let pos = r.0.u64()?;
            let size = r.0.u64()?;
            p.os = Some((pos, size, String::from_utf8_lossy(&r.bytes()?).into_owned()));
        }
        Ok(p)
    }
    pub fn describe(&self) -> String {
        let os = match &self.os {
            Some((pos, size, path)) => format!("{path:?} pos {pos} size {size}"),
            None => "no OS file".into(),
        };
        format!("fd {} osfile {:#x} {os} FILE {}", self.fd, self.osfile, hex(&self.file))
    }
}

/// Human-readable form of an OS probe record.
pub fn describe_os(p: OsProbe, b: &[u8]) -> String {
    match p {
        OsProbe::File => FileProbe::decode(b)
            .map(|f| f.describe())
            .unwrap_or_else(|e| format!("bad file probe: {e}")),
        OsProbe::Cwd => format!("cwd {:?}", String::from_utf8_lossy(b)),
        OsProbe::Cursor => format!("cursor {}", hex(b)),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    pub addr: u32,
    pub kind: RegionKind,
    pub access: u8,
    /// False when the range could not be read (null `this`, unmapped pointer): then all buffers are empty.
    pub readable: bool,
    pub pre: Vec<u8>,
    pub post_orig: Vec<u8>,
    pub post_rust: Vec<u8>,
}

impl Region {
    pub fn len(&self) -> u32 {
        self.pre.len() as u32
    }
    pub fn is_empty(&self) -> bool {
        self.pre.is_empty()
    }
    /// Bytes of memory the region covers (a `Post` region has no `pre`); 0 for an OS probe.
    pub fn extent(&self) -> u32 {
        match self.kind {
            RegionKind::Os(_) => 0,
            RegionKind::Post => self.post_orig.len().max(self.post_rust.len()) as u32,
            _ => self.len(),
        }
    }
    /// Memory region whose `pre` bytes are the memory at `addr` (not a post-only or OS region).
    pub fn has_pre_memory(&self) -> bool {
        !matches!(self.kind, RegionKind::Post | RegionKind::Os(_))
    }
    /// Declared as written (bytes changed here are inside the write set).
    pub fn declared_write(&self) -> bool {
        match self.kind {
            RegionKind::Global { .. } | RegionKind::Extra | RegionKind::Post => self.access & ACC_W != 0,
            RegionKind::This | RegionKind::ArgPtr(_) => true,
            RegionKind::Os(_) => false,
        }
    }
}

/// Bytes written in a watched window: `bytes` is the value after the run (the value before is in the
/// case's `window_pre`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Write {
    pub addr: u32,
    pub bytes: Vec<u8>,
}

/// Registers in pushad order is not used; this is the fixed order of the case format.
pub const REG_NAMES: [&str; 7] = ["eax", "ecx", "edx", "ebx", "esi", "edi", "ebp"];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// eax, ecx, edx, ebx, esi, edi, ebp after the call.
    pub regs: [u32; 7],
    /// Bytes popped by the callee.
    pub pop: u32,
    /// x87 stack after the call, ST0 first, 80 bits each.
    pub st: Vec<[u8; 10]>,
    pub cw: u16,
    pub sw: u16,
    pub mxcsr: u32,
    /// Writes found in the watched windows (merged ranges).
    pub writes: Vec<Write>,
    /// Hook ids entered during the run (callee log), in order.
    pub calls: Vec<u32>,
    /// Arena blocks allocated during the run (address, size), in order.
    pub allocs: Vec<(u32, u32)>,
    /// Stack argument words as the callee left them.
    pub args_out: Vec<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Case {
    pub fn_addr: u32,
    pub name: String,
    pub conv: u8,
    pub ret: u8,
    /// Provenance (not part of the content address): frame, call ordinal of this function, caller.
    pub frame: u32,
    pub ordinal: u32,
    pub caller: u32,
    /// eax, ecx, edx, ebx, esi, edi, ebp at entry.
    pub regs_in: [u32; 7],
    pub args: Vec<u32>,
    /// fnstenv image at entry (28 bytes) and MXCSR.
    pub fpu_env: Vec<u8>,
    pub mxcsr: u32,
    pub regions: Vec<Region>,
    /// Watched windows: [start, end) ranges and the pre-call bytes of every written range (merged over
    /// both runs).
    pub windows: Vec<(u32, u32)>,
    pub window_pre: Vec<Write>,
    pub orig: Outcome,
    pub rust: Outcome,
    /// Normaliser context shared by both runs (pools, image, model infos); allocations are per outcome.
    pub image: (u32, u32),
    pub pools: Vec<(u32, u32, u32)>,
    /// 0 = the game continued with the original's state, 1 = with Rust's.
    pub leave: u8,
    /// Rust side was a deliberately wrong implementation (self-test).
    pub mutant: bool,
    /// Split-run record (INF-17; [`LEAVE_SPLIT_ORIG`] / [`LEAVE_SPLIT_RUST`] in `leave`): global
    /// sequence number of the case in its run (provenance).
    pub seq: u32,
    /// Split-run record: hash of every 4 KiB page of the static watched windows before the call
    /// (page address, [`page_hash`]); the input fingerprint for state outside the declared regions.
    pub page_hashes: Vec<(u32, u64)>,
    /// Split-run record: raw values of the bytes excluded from the page hashes
    /// (`normalise::INPUT_EXCLUSIONS`), so the report can show when they differed.
    pub volatile: Vec<Write>,
    /// Split-run record: address of the first stack argument at the call (the caller's ESP + 4) and
    /// the thread's stack [limit, base) then. Stack pointers compare by their offset from `esp`
    /// (normalise.rs `Stack` class): Rust frames above a call (a ported ancestor) move the stack.
    pub esp: u32,
    pub stack: (u32, u32),
    /// Merged split-run case only: `esp` of the rust run (the Rust outcome's stack pointers).
    pub esp_rust: u32,
    /// Split-run record: OS-assigned object addresses (normalise.rs `Foreign` class): the distinct word
    /// values of this case (arguments, registers, regions, window bytes) that point into committed
    /// memory outside the exe image, the determinism arena and the stack (Wine/D3D/DirectInput/
    /// DirectSound heaps), sorted.
    pub foreign: Vec<u32>,
    /// Merged split-run case only: `foreign` of the rust run.
    pub foreign_rust: Vec<u32>,
    /// Split-run record: watched-window words that held an OS object address before the call, as
    /// flat (address, value) pairs. The page fingerprint replaces every word whose VALUE lies in the OS
    /// heap range (below the determinism arena, outside the image and the stack) by a marker, so the
    /// fingerprint does not depend on where the OS put its objects; these pairs show what was masked.
    pub foreign_words: Vec<u32>,
    /// Host only (not encoded): a merged split-run pair (plain record + rust outcome); the reviewed
    /// volatile bytes (`normalise::INPUT_EXCLUSIONS`) are excluded from its comparison too.
    pub merged: bool,
    /// Host only (not encoded), merged split-run pair: the rust run's window bytes before the call
    /// (`window_pre` then holds the plain run's); unchanged bytes of a written word may differ
    /// between the two processes (OS object addresses).
    pub window_pre_rust: Vec<Write>,
}

/// `leave` of a split-run record made in the plain run: `orig` holds the original's outcome.
pub const LEAVE_SPLIT_ORIG: u8 = 2;
/// `leave` of a split-run record made in the rust run: `orig` holds the Rust implementation's outcome.
pub const LEAVE_SPLIT_RUST: u8 = 3;

/// Hash of one page of the watched windows at `addr` (split-run input fingerprint): FNV-1a 64 over
/// the bytes, with the reviewed input exclusions (`normalise::INPUT_EXCLUSIONS`) zeroed.
pub fn page_hash(addr: u32, b: &[u8]) -> u64 {
    let mut h = crate::fnv::Fnv64::new();
    h.write(&normalise::fingerprint_bytes(addr, b));
    h.0
}

const MAGIC: &[u8; 8] = b"SAL2CAS1";
/// Optional trailing block of a case (split-run fields); absent in plain L2 cases, so their encoding
/// and content ids are unchanged.
const EXT_MAGIC: &[u8; 4] = b"EXT1";

struct W(Vec<u8>);
impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v)
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes())
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes())
    }
    fn bytes(&mut self, b: &[u8]) {
        self.u32(b.len() as u32);
        self.0.extend_from_slice(b)
    }
    fn words(&mut self, v: &[u32]) {
        self.u32(v.len() as u32);
        for &x in v {
            self.u32(x)
        }
    }
    fn writes(&mut self, v: &[Write]) {
        self.u32(v.len() as u32);
        for w in v {
            self.u32(w.addr);
            self.bytes(&w.bytes);
        }
    }
    fn outcome(&mut self, o: &Outcome) {
        for r in o.regs {
            self.u32(r)
        }
        self.u32(o.pop);
        self.u32(o.st.len() as u32);
        for s in &o.st {
            self.0.extend_from_slice(s)
        }
        self.u16(o.cw);
        self.u16(o.sw);
        self.u32(o.mxcsr);
        self.writes(&o.writes);
        self.words(&o.calls);
        self.u32(o.allocs.len() as u32);
        for &(a, n) in &o.allocs {
            self.u32(a);
            self.u32(n);
        }
        self.words(&o.args_out);
    }
}

struct R<'a>(crate::Reader<'a>);
impl R<'_> {
    fn u8(&mut self) -> Result<u8, String> {
        self.0.u8()
    }
    fn u16(&mut self) -> Result<u16, String> {
        let b = self.0.bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32, String> {
        self.0.u32()
    }
    fn len(&mut self) -> Result<usize, String> {
        let n = self.u32()? as usize;
        if n > self.0.buf.len() {
            return Err(format!("bad length {n}"));
        }
        Ok(n)
    }
    fn bytes(&mut self) -> Result<Vec<u8>, String> {
        let n = self.len()?;
        Ok(self.0.bytes(n)?.to_vec())
    }
    fn words(&mut self) -> Result<Vec<u32>, String> {
        let n = self.len()?;
        (0..n).map(|_| self.u32()).collect()
    }
    fn writes(&mut self) -> Result<Vec<Write>, String> {
        let n = self.len()?;
        (0..n)
            .map(|_| {
                Ok(Write {
                    addr: self.u32()?,
                    bytes: self.bytes()?,
                })
            })
            .collect()
    }
    fn outcome(&mut self) -> Result<Outcome, String> {
        let mut o = Outcome::default();
        for r in o.regs.iter_mut() {
            *r = self.u32()?;
        }
        o.pop = self.u32()?;
        let n = self.len()?;
        for _ in 0..n {
            let mut s = [0u8; 10];
            s.copy_from_slice(self.0.bytes(10)?);
            o.st.push(s);
        }
        o.cw = self.u16()?;
        o.sw = self.u16()?;
        o.mxcsr = self.u32()?;
        o.writes = self.writes()?;
        o.calls = self.words()?;
        let n = self.len()?;
        for _ in 0..n {
            o.allocs.push((self.u32()?, self.u32()?));
        }
        o.args_out = self.words()?;
        Ok(o)
    }
}

impl Case {
    /// Binary encoding. `with_provenance = false` zeroes frame/ordinal/caller (the content address).
    pub fn encode_with(&self, with_provenance: bool) -> Vec<u8> {
        let mut w = W(Vec::with_capacity(512));
        w.0.extend_from_slice(MAGIC);
        w.u32(self.fn_addr);
        w.bytes(self.name.as_bytes());
        w.u8(self.conv);
        w.u8(self.ret);
        let p = with_provenance as u32;
        w.u32(self.frame * p);
        w.u32(self.ordinal * p);
        w.u32(self.caller * p);
        for r in self.regs_in {
            w.u32(r)
        }
        w.words(&self.args);
        w.bytes(&self.fpu_env);
        w.u32(self.mxcsr);
        w.u32(self.regions.len() as u32);
        for r in &self.regions {
            w.u32(r.addr);
            let (k, x) = match r.kind {
                RegionKind::Global { exact } => (0u8, exact as u8),
                RegionKind::This => (1, 0),
                RegionKind::ArgPtr(i) => (2, i),
                RegionKind::Extra => (3, 0),
                RegionKind::Post => (4, 0),
                RegionKind::Os(p) => (5, p as u8),
            };
            w.u8(k);
            w.u8(x);
            w.u8(r.access);
            w.u8(r.readable as u8);
            w.bytes(&r.pre);
            w.bytes(&r.post_orig);
            w.bytes(&r.post_rust);
        }
        w.u32(self.windows.len() as u32);
        for &(a, b) in &self.windows {
            w.u32(a);
            w.u32(b);
        }
        w.writes(&self.window_pre);
        w.outcome(&self.orig);
        w.outcome(&self.rust);
        w.u32(self.image.0);
        w.u32(self.image.1);
        w.u32(self.pools.len() as u32);
        for &(a, b, c) in &self.pools {
            w.u32(a);
            w.u32(b);
            w.u32(c);
        }
        w.u8(self.leave);
        w.u8(self.mutant as u8);
        if self.is_split() || !self.page_hashes.is_empty() {
            w.0.extend_from_slice(EXT_MAGIC);
            w.u32(self.seq * p);
            w.u32(self.page_hashes.len() as u32);
            for &(a, h) in &self.page_hashes {
                w.u32(a);
                w.0.extend_from_slice(&h.to_le_bytes());
            }
            w.writes(&self.volatile);
            w.u32(self.esp);
            w.u32(self.stack.0);
            w.u32(self.stack.1);
            w.u32(self.esp_rust);
            w.words(&self.foreign);
            w.words(&self.foreign_rust);
            w.words(&self.foreign_words);
        }
        w.0
    }

    /// A split-run record (one implementation's outcome in `orig`, `rust` empty).
    pub fn is_split(&self) -> bool {
        self.leave == LEAVE_SPLIT_ORIG || self.leave == LEAVE_SPLIT_RUST
    }

    pub fn encode(&self) -> Vec<u8> {
        self.encode_with(true)
    }

    /// Content address: two FNV-1a 64 hashes (different seeds) of the encoding without provenance.
    pub fn content_id(&self) -> String {
        let b = self.encode_with(false);
        let mut h1 = crate::fnv::Fnv64::new();
        h1.write(&b);
        let mut h2 = crate::fnv::Fnv64(0x6c62_272e_07bb_0142);
        h2.write(&b);
        format!("{:016x}{:016x}", h1.0, h2.0)
    }

    pub fn decode(buf: &[u8]) -> Result<Case, String> {
        let mut r = R(crate::Reader::new(buf));
        if r.0.bytes(8)? != MAGIC {
            return Err("not an L2 case (magic)".into());
        }
        let mut c = Case {
            fn_addr: r.u32()?,
            name: String::from_utf8(r.bytes()?).map_err(|e| e.to_string())?,
            conv: r.u8()?,
            ret: r.u8()?,
            frame: r.u32()?,
            ordinal: r.u32()?,
            caller: r.u32()?,
            ..Case::default()
        };
        for x in c.regs_in.iter_mut() {
            *x = r.u32()?;
        }
        c.args = r.words()?;
        c.fpu_env = r.bytes()?;
        c.mxcsr = r.u32()?;
        let n = r.len()?;
        for _ in 0..n {
            let addr = r.u32()?;
            let (k, x) = (r.u8()?, r.u8()?);
            let kind = match k {
                0 => RegionKind::Global { exact: x != 0 },
                1 => RegionKind::This,
                2 => RegionKind::ArgPtr(x),
                3 => RegionKind::Extra,
                4 => RegionKind::Post,
                5 => RegionKind::Os(OsProbe::from_u8(x).ok_or_else(|| format!("bad OS probe {x}"))?),
                _ => return Err(format!("bad region kind {k}")),
            };
            c.regions.push(Region {
                addr,
                kind,
                access: r.u8()?,
                readable: r.u8()? != 0,
                pre: r.bytes()?,
                post_orig: r.bytes()?,
                post_rust: r.bytes()?,
            });
        }
        let n = r.len()?;
        for _ in 0..n {
            c.windows.push((r.u32()?, r.u32()?));
        }
        c.window_pre = r.writes()?;
        c.orig = r.outcome()?;
        c.rust = r.outcome()?;
        c.image = (r.u32()?, r.u32()?);
        let n = r.len()?;
        for _ in 0..n {
            c.pools.push((r.u32()?, r.u32()?, r.u32()?));
        }
        c.leave = r.u8()?;
        c.mutant = r.u8()? != 0;
        if !r.0.at_end() {
            if r.0.bytes(4)? != EXT_MAGIC {
                return Err("trailing bytes".into());
            }
            c.seq = r.u32()?;
            let n = r.len()?;
            for _ in 0..n {
                c.page_hashes.push((r.u32()?, r.0.u64()?));
            }
            c.volatile = r.writes()?;
            c.esp = r.u32()?;
            c.stack = (r.u32()?, r.u32()?);
            c.esp_rust = r.u32()?;
            c.foreign = r.words()?;
            c.foreign_rust = r.words()?;
            c.foreign_words = r.words()?;
        }
        if !r.0.at_end() {
            return Err("trailing bytes".into());
        }
        Ok(c)
    }

    pub fn ret_kind(&self) -> Ret {
        match self.ret {
            1 => Ret::Eax,
            2 => Ret::Al,
            3 => Ret::St0,
            4 => Ret::EaxEdx,
            _ => Ret::Void,
        }
    }

    /// Normaliser contexts of the two runs (model infos are not stored in a case; see normalise.rs).
    pub fn norm_ctx(&self) -> (NormCtx, NormCtx) {
        let pools = PoolMap {
            pools: self
                .pools
                .iter()
                .map(|&(objects, bytemap, size)| PoolHdr {
                    objects,
                    bytemap,
                    size,
                })
                .collect(),
        };
        let mk = |o: &Outcome, esp: u32, foreign: &Vec<u32>| NormCtx {
            image: self.image,
            pools: pools.clone(),
            model_infos: Vec::new(),
            allocs: o.allocs.clone(),
            stack: self.stack,
            esp,
            foreign: foreign.clone(),
        };
        let merged = self.esp_rust != 0 || !self.foreign_rust.is_empty();
        let (esp_r, f_r) = if merged {
            (self.esp_rust, &self.foreign_rust)
        } else {
            (self.esp, &self.foreign)
        };
        (mk(&self.orig, self.esp, &self.foreign), mk(&self.rust, esp_r, f_r))
    }
}

// ------------------------------------------------------------------ spool ---------------------------

/// Spool file: `SAL2SPL1` then records `[u32 len][case]`. Written append-only by sa-hook.
pub const SPOOL_MAGIC: &[u8; 8] = b"SAL2SPL1";

pub fn spool_record(c: &Case) -> Vec<u8> {
    let b = c.encode();
    let mut out = Vec::with_capacity(b.len() + 4);
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(&b);
    out
}

/// Record length marking the spool trailer (INF-18): `[TRAILER_MARK][u32 len][UTF-8 text]`, written
/// once when sa-hook stops spooling (quota reached); nothing follows it.
pub const TRAILER_MARK: u32 = u32::MAX;

pub fn spool_trailer(text: &str) -> Vec<u8> {
    let mut out = TRAILER_MARK.to_le_bytes().to_vec();
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
    out
}

pub fn parse_spool(buf: &[u8]) -> Result<Vec<Case>, String> {
    parse_spool_full(buf).map(|s| s.0)
}

/// The cases and the trailer text (present when sa-hook stopped spooling at the quota).
pub fn parse_spool_full(buf: &[u8]) -> Result<(Vec<Case>, Option<String>), String> {
    if buf.len() < 8 || &buf[..8] != SPOOL_MAGIC {
        return Err("not an L2 spool (magic)".into());
    }
    let mut out = Vec::new();
    let mut p = 8usize;
    while p < buf.len() {
        if p + 4 > buf.len() {
            return Err(format!("truncated record header at {p}"));
        }
        let n = u32::from_le_bytes([buf[p], buf[p + 1], buf[p + 2], buf[p + 3]]) as usize;
        p += 4;
        if n as u32 == TRAILER_MARK {
            let t = buf.get(p..p + 4).ok_or("truncated trailer")?;
            let k = u32::from_le_bytes([t[0], t[1], t[2], t[3]]) as usize;
            let s = buf.get(p + 4..p + 4 + k).ok_or("truncated trailer")?;
            if p + 4 + k != buf.len() {
                return Err("bytes after the spool trailer".into());
            }
            return Ok((out, Some(String::from_utf8_lossy(s).into_owned())));
        }
        let b = buf
            .get(p..p + n)
            .ok_or_else(|| format!("truncated record at {p}"))?;
        out.push(Case::decode(b).map_err(|e| format!("record at {p}: {e}"))?);
        p += n;
    }
    Ok((out, None))
}

// ------------------------------------------------------------------ comparison ----------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diff {
    pub field: String,
    pub detail: String,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Extended-precision bytes (little-endian 80-bit) shown as `sign/exp:mantissa`.
pub fn ext_str(e: &[u8; 10]) -> String {
    let m = u64::from_le_bytes([e[0], e[1], e[2], e[3], e[4], e[5], e[6], e[7]]);
    let se = u16::from_le_bytes([e[8], e[9]]);
    format!("{se:04x}:{m:016x}")
}

/// The 32-bit float a caller's `fstp dword` would store from an 80-bit value (round to nearest even).
pub fn ext_to_f32_bits(e: &[u8; 10]) -> u32 {
    let m = u64::from_le_bytes([e[0], e[1], e[2], e[3], e[4], e[5], e[6], e[7]]);
    let se = u16::from_le_bytes([e[8], e[9]]);
    let sign = ((se >> 15) as u32) << 31;
    let exp = (se & 0x7FFF) as i32;
    if exp == 0x7FFF {
        if m << 1 == 0 {
            return sign | 0x7F80_0000;
        }
        // NaN: quiet, top 22 payload bits kept.
        return sign | 0x7FC0_0000 | ((m >> 40) as u32 & 0x003F_FFFF);
    }
    if m == 0 {
        return sign;
    }
    // value = m * 2^(exp - 16383 - 63)
    let mut e2 = exp - 16383; // exponent of the leading bit (m normalised: bit 63 set)
    let lz = m.leading_zeros() as i32;
    let m = m << lz;
    e2 -= lz;
    let shift: u32 = if e2 >= -126 { 40 } else { (40 + (-126 - e2)) as u32 };
    if shift > 64 {
        return sign;
    }
    let (mut keep, rem, half) = if shift == 64 {
        (0u64, m, 1u64 << 63)
    } else {
        (m >> shift, m & ((1u64 << shift) - 1), 1u64 << (shift - 1))
    };
    if rem > half || (rem == half && keep & 1 == 1) {
        keep += 1;
    }
    if e2 >= -126 {
        let mut be = e2 + 127;
        if keep >> 24 != 0 {
            keep >>= 1;
            be += 1;
        }
        if be >= 255 {
            return sign | 0x7F80_0000;
        }
        sign | (be as u32) << 23 | (keep as u32 & 0x007F_FFFF)
    } else {
        // subnormal (keep may round up into the smallest normal, which the encoding handles)
        sign | keep as u32
    }
}

fn cmp_bytes(out: &mut Vec<Diff>, field: String, addr: u32, a: &[u8], ca: &NormCtx, b: &[u8], cb: &NormCtx) {
    if let Some(i) = normalise::bytes_equal(addr, a, ca, b, cb) {
        let lo = i.saturating_sub(i % 4);
        let hi = (lo + 8).min(a.len().max(b.len()));
        let sa = a.get(lo..hi.min(a.len())).unwrap_or(&[]);
        let sb = b.get(lo..hi.min(b.len())).unwrap_or(&[]);
        out.push(Diff {
            field,
            detail: format!(
                "first difference at {:#x} (+{i}): orig {} rust {}",
                addr as u64 + i as u64,
                hex(sa),
                hex(sb)
            ),
        });
    }
}

/// Flattens window writes into a sorted byte map (address -> value).
fn write_map(w: &[Write]) -> std::collections::BTreeMap<u32, u8> {
    let mut m = std::collections::BTreeMap::new();
    for x in w {
        for (i, &b) in x.bytes.iter().enumerate() {
            m.insert(x.addr.wrapping_add(i as u32), b);
        }
    }
    m
}

/// Groups a sorted byte map into contiguous runs.
fn runs(m: &std::collections::BTreeMap<u32, u8>) -> Vec<Write> {
    let mut out: Vec<Write> = Vec::new();
    for (&a, &b) in m {
        match out.last_mut() {
            Some(w) if w.addr as u64 + w.bytes.len() as u64 == a as u64 => w.bytes.push(b),
            _ => out.push(Write {
                addr: a,
                bytes: vec![b],
            }),
        }
    }
    out
}

/// Compares the Rust implementation's outcome with the original's. Empty = identical.
pub fn compare(c: &Case) -> Vec<Diff> {
    let mut d = Vec::new();
    let (ca, cb) = c.norm_ctx();
    let (o, r) = (&c.orig, &c.rust);
    let reg = |i: usize| (o.regs[i], r.regs[i]);
    let regdiff = |d: &mut Vec<Diff>, i: usize, mask: u32| {
        let (a, b) = reg(i);
        if a & mask != b & mask && !(mask == u32::MAX && normalise::words_equal(a, &ca, b, &cb)) {
            d.push(Diff {
                field: REG_NAMES[i].into(),
                detail: format!("orig {:#x} rust {:#x} (mask {mask:#x})", a & mask, b & mask),
            });
        }
    };
    match c.ret_kind() {
        Ret::Eax => regdiff(&mut d, 0, u32::MAX),
        Ret::Al => regdiff(&mut d, 0, 0xFF),
        Ret::EaxEdx => {
            regdiff(&mut d, 0, u32::MAX);
            regdiff(&mut d, 2, u32::MAX);
        }
        Ret::Void | Ret::St0 => {}
    }
    for i in 3..7 {
        regdiff(&mut d, i, u32::MAX);
    }
    if o.pop != r.pop {
        d.push(Diff {
            field: "pop".into(),
            detail: format!("callee popped orig {} rust {} bytes", o.pop, r.pop),
        });
    }
    if o.st.len() != r.st.len() {
        d.push(Diff {
            field: "x87-depth".into(),
            detail: format!("orig {} rust {}", o.st.len(), r.st.len()),
        });
    }
    for (i, (a, b)) in o.st.iter().zip(&r.st).enumerate() {
        if a != b {
            d.push(Diff {
                field: format!("st{i}"),
                detail: format!(
                    "orig {} (f32 {:#010x}) rust {} (f32 {:#010x})",
                    ext_str(a),
                    ext_to_f32_bits(a),
                    ext_str(b),
                    ext_to_f32_bits(b)
                ),
            });
        }
    }
    if o.cw != r.cw {
        d.push(Diff {
            field: "fpu-cw".into(),
            detail: format!("orig {:#06x} rust {:#06x}", o.cw, r.cw),
        });
    }
    if o.mxcsr & 0xFFC0 != r.mxcsr & 0xFFC0 {
        d.push(Diff {
            field: "mxcsr".into(),
            detail: format!("orig {:#x} rust {:#x}", o.mxcsr, r.mxcsr),
        });
    }
    // Bytes the reviewed normaliser excludes (original uninitialised data, evidence in normalise.rs):
    // both runs' values are replaced by zero before the comparison.
    let excl = normalise::exclusions(c);
    let masked = |addr: u32, b: &[u8]| -> Vec<u8> {
        let mut v = b.to_vec();
        for e in &excl {
            for k in 0..e.len {
                let a = e.addr.wrapping_add(k);
                if a >= addr && ((a - addr) as usize) < v.len() {
                    v[(a - addr) as usize] = 0;
                }
            }
        }
        v
    };
    for reg in &c.regions {
        if !reg.readable {
            continue;
        }
        let what = match reg.kind {
            RegionKind::Global { .. } => format!("global {:#x}", reg.addr),
            RegionKind::This => "this".into(),
            RegionKind::ArgPtr(i) => format!("*arg{i}"),
            RegionKind::Extra => format!("extra {:#x}", reg.addr),
            RegionKind::Post => format!("post {:#x}", reg.addr),
            RegionKind::Os(p) => {
                // OS state is compared raw: OS-assigned values are never normalised here.
                if reg.post_orig != reg.post_rust {
                    d.push(Diff {
                        field: format!("os-{} {:#x}", p.name(), reg.addr),
                        detail: format!(
                            "orig {} rust {}",
                            describe_os(p, &reg.post_orig),
                            describe_os(p, &reg.post_rust)
                        ),
                    });
                }
                continue;
            }
        };
        if excl.is_empty() {
            cmp_bytes(&mut d, what, reg.addr, &reg.post_orig, &ca, &reg.post_rust, &cb);
        } else {
            let (a, b) = (masked(reg.addr, &reg.post_orig), masked(reg.addr, &reg.post_rust));
            cmp_bytes(&mut d, what, reg.addr, &a, &ca, &b, &cb);
        }
    }
    // Window writes: same bytes written to the same addresses (normalised word-wise).
    let (wo, wr) = (write_map(&o.writes), write_map(&r.writes));
    if wo != wr {
        let keys: std::collections::BTreeSet<u32> = wo.keys().chain(wr.keys()).copied().collect();
        let pre = write_map(&c.window_pre);
        let pre_r = if c.window_pre_rust.is_empty() {
            pre.clone()
        } else {
            write_map(&c.window_pre_rust)
        };
        // a byte the run did not write keeps its value before the call (in that run)
        let get = |m: &std::collections::BTreeMap<u32, u8>, a: u32| {
            let p = if std::ptr::eq(m, &wr) { &pre_r } else { &pre };
            m.get(&a).or(p.get(&a)).copied().unwrap_or(0)
        };
        let mut bad = None;
        for &a in &keys {
            if get(&wo, a) == get(&wr, a) || excl.iter().any(|e| a.wrapping_sub(e.addr) < e.len) {
                continue;
            }
            let ws = a & !3;
            let word = |m| u32::from_le_bytes([get(m, ws), get(m, ws + 1), get(m, ws + 2), get(m, ws + 3)]);
            if normalise::words_equal(word(&wo), &ca, word(&wr), &cb) {
                continue;
            }
            bad = Some(a);
            break;
        }
        if let Some(a) = bad {
            let ws = a & !3;
            let show = |m| {
                (0..8)
                    .map(|k| format!("{:02x}", get(m, ws + k)))
                    .collect::<String>()
            };
            d.push(Diff {
                field: "window-writes".into(),
                detail: format!(
                    "first difference at {a:#x}: orig {} rust {} (orig wrote {} bytes, rust {})",
                    show(&wo),
                    show(&wr),
                    wo.len(),
                    wr.len()
                ),
            });
        }
    }
    if o.calls != r.calls {
        d.push(Diff {
            field: "callees".into(),
            detail: format!("orig {:?} rust {:?}", o.calls, r.calls),
        });
    }
    d
}

/// A byte range changed by a run but not declared as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Undeclared {
    pub addr: u32,
    pub len: u32,
    /// "orig" or "rust"
    pub run: &'static str,
    /// Within 64 KiB above a declared-but-unbounded entry (base/index access; extent unknown).
    pub near_unbounded: Option<u32>,
}

/// Bytes the run changed in the watched windows outside every declared written region.
pub fn undeclared_writes(c: &Case) -> Vec<Undeclared> {
    let declared: Vec<(u64, u64)> = c
        .regions
        .iter()
        .filter(|r| r.declared_write())
        .map(|r| (r.addr as u64, r.addr as u64 + r.extent().max(1) as u64))
        .collect();
    let unbounded: Vec<u32> = c
        .regions
        .iter()
        .filter(|r| r.access & ACC_W != 0 && r.kind == RegionKind::Global { exact: false })
        .map(|r| r.addr)
        .collect();
    let mut out = Vec::new();
    for (run, o) in [("orig", &c.orig), ("rust", &c.rust)] {
        let mut m = write_map(&o.writes);
        m.retain(|&a, _| !declared.iter().any(|&(s, e)| (s..e).contains(&(a as u64))));
        for w in runs(&m) {
            let near = unbounded
                .iter()
                .copied()
                .filter(|&b| w.addr >= b && w.addr - b < 0x10000)
                .max();
            out.push(Undeclared {
                addr: w.addr,
                len: w.bytes.len() as u32,
                run,
                near_unbounded: near,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Case {
        Case {
            fn_addr: 0x53CB50,
            name: "CGeneral::LimitRadianAngle".into(),
            conv: Conv::Cdecl as u8,
            ret: Ret::St0 as u8,
            frame: 7,
            ordinal: 3,
            caller: 0x401234,
            regs_in: [1, 2, 3, 4, 5, 6, 7],
            args: vec![0x40490FDB],
            fpu_env: vec![0x7F, 0, 0, 0],
            mxcsr: 0x9FA0,
            regions: vec![Region {
                addr: 0xB7CB4C,
                kind: RegionKind::Global { exact: true },
                access: ACC_R | ACC_W,
                readable: true,
                pre: vec![1, 0, 0, 0],
                post_orig: vec![2, 0, 0, 0],
                post_rust: vec![2, 0, 0, 0],
            }],
            windows: vec![(0x8A4000, 0xC9E000)],
            window_pre: vec![Write {
                addr: 0xB7CB4C,
                bytes: vec![1],
            }],
            orig: Outcome {
                regs: [0, 0, 0, 4, 5, 6, 7],
                st: vec![[0, 0, 0, 0, 0, 0, 0, 0x80, 0xFF, 0x3F]],
                cw: 0x7F,
                mxcsr: 0x9FA0,
                writes: vec![Write {
                    addr: 0xB7CB4C,
                    bytes: vec![2],
                }],
                args_out: vec![0x40490FDB],
                ..Outcome::default()
            },
            rust: Outcome::default(),
            image: (0x400000, 0x1577000),
            pools: vec![(0, 0, 0); 17],
            leave: 0,
            mutant: false,
            seq: 0,
            page_hashes: Vec::new(),
            volatile: Vec::new(),
            esp: 0,
            stack: (0, 0),
            esp_rust: 0,
            foreign: Vec::new(),
            foreign_rust: Vec::new(),
            foreign_words: Vec::new(),
            merged: false,
            window_pre_rust: Vec::new(),
        }
    }

    #[test]
    fn split_fields_trailer_and_os_regions() {
        let mut c = sample();
        c.rust = c.orig.clone();
        let plain = c.encode();
        let mut s = c.clone();
        s.leave = LEAVE_SPLIT_RUST;
        s.seq = 42;
        s.page_hashes = vec![(0x8A4000, 7), (0x8A5000, page_hash(0x8A5000, &[1, 2, 3]))];
        s.esp = 0x36F200;
        s.foreign = vec![0x0301_A530];
        s.foreign_words = vec![0xC97BB8, 0x0301_A530];
        s.stack = (0x270000, 0x370000);
        s.volatile = vec![Write {
            addr: 0x8E31BC,
            bytes: vec![1, 2, 3, 4],
        }];
        s.regions.push(Region {
            addr: 0x8E2C30,
            kind: RegionKind::Os(OsProbe::File),
            access: 0,
            readable: true,
            pre: vec![],
            post_orig: FileProbe {
                file: vec![0; 32],
                fd: 3,
                osfile: 1,
                os: Some((10, 20, "C:\\x".into())),
            }
            .encode(),
            post_rust: vec![],
        });
        let b = s.encode();
        assert!(b.len() > plain.len());
        assert_eq!(Case::decode(&b).unwrap(), s);
        assert_eq!(Case::decode(&plain).unwrap(), c, "plain cases keep their old encoding");
        let mut s2 = s.clone();
        s2.seq = 7;
        assert_eq!(s2.content_id(), s.content_id(), "seq is provenance");
        // OS probe regions compare raw and are reported
        s.regions[1].post_rust = s.regions[1].post_orig.clone();
        s.rust = s.orig.clone();
        assert!(compare(&s).is_empty());
        let mut f = FileProbe::decode(&s.regions[1].post_orig).unwrap();
        f.os = Some((11, 20, "C:\\x".into()));
        s.regions[1].post_rust = f.encode();
        assert_eq!(compare(&s)[0].field, "os-file 0x8e2c30");
        assert!(undeclared_writes(&s).is_empty());
        // trailer
        let mut spool = SPOOL_MAGIC.to_vec();
        spool.extend(spool_record(&c));
        spool.extend(spool_trailer("quota"));
        let (cases, t) = parse_spool_full(&spool).unwrap();
        assert_eq!((cases.len(), t.as_deref()), (1, Some("quota")));
        assert_eq!(parse_spool(&spool).unwrap().len(), 1);
        spool.push(0);
        assert!(parse_spool(&spool).is_err());
    }

    #[test]
    fn roundtrip_and_content_id() {
        let mut c = sample();
        c.rust = c.orig.clone();
        let b = c.encode();
        assert_eq!(Case::decode(&b).unwrap(), c);
        let id = c.content_id();
        let mut c2 = c.clone();
        c2.frame = 99;
        c2.caller = 5;
        assert_eq!(c2.content_id(), id, "provenance is not content");
        c2.args[0] ^= 1;
        assert_ne!(c2.content_id(), id);
        let mut spool = SPOOL_MAGIC.to_vec();
        spool.extend(spool_record(&c));
        spool.extend(spool_record(&c2));
        assert_eq!(parse_spool(&spool).unwrap(), vec![c, c2]);
        assert!(Case::decode(&b[..b.len() - 1]).is_err());
    }

    #[test]
    fn compare_detects_each_field() {
        let mut c = sample();
        c.rust = c.orig.clone();
        assert!(compare(&c).is_empty());
        let mut m = c.clone();
        m.rust.st[0][0] = 1;
        assert_eq!(compare(&m)[0].field, "st0");
        let mut m = c.clone();
        m.rust.st.clear();
        assert_eq!(compare(&m)[0].field, "x87-depth");
        let mut m = c.clone();
        m.rust.regs[3] = 9;
        assert_eq!(compare(&m)[0].field, "ebx");
        let mut m = c.clone();
        m.regions[0].post_rust[0] = 3;
        assert_eq!(compare(&m)[0].field, "global 0xb7cb4c");
        let mut m = c.clone();
        m.rust.writes[0].bytes[0] = 9;
        assert_eq!(compare(&m)[0].field, "window-writes");
        let mut m = c.clone();
        m.rust.writes.clear(); // rust did not write: the byte keeps its pre value (1 != 2)
        assert_eq!(compare(&m)[0].field, "window-writes");
        // AL returns compare the low byte only
        let mut m = c.clone();
        m.ret = Ret::Al as u8;
        m.orig.regs[0] = 0x4101;
        m.rust.regs[0] = 0x0001;
        assert!(compare(&m).is_empty());
        m.rust.regs[0] = 0;
        assert_eq!(compare(&m)[0].field, "eax");
        let mut m = c.clone();
        m.rust.mxcsr = 0x9FA1; // a sticky flag only
        assert!(compare(&m).is_empty());
        m.rust.cw = 0x27F;
        assert_eq!(compare(&m)[0].field, "fpu-cw");
    }

    #[test]
    fn undeclared() {
        let mut c = sample();
        c.rust = c.orig.clone();
        assert!(undeclared_writes(&c).is_empty());
        c.orig.writes.push(Write {
            addr: 0xB7CB84,
            bytes: vec![1, 2],
        });
        let u = undeclared_writes(&c);
        assert_eq!(
            u,
            vec![Undeclared {
                addr: 0xB7CB84,
                len: 2,
                run: "orig",
                near_unbounded: None
            }]
        );
    }

    #[test]
    fn spec_text() {
        let s = vec![FnSpec {
            addr: 0x561B10,
            name: "CTimer::Update".into(),
            globals: vec![
                Global {
                    addr: 0xB7CB4C,
                    len: 4,
                    access: ACC_R | ACC_W,
                    exact: true,
                    name: "CTimer::m_FrameCounter".into(),
                },
                Global {
                    addr: 0xA9B0C8,
                    len: 4,
                    access: ACC_R,
                    exact: false,
                    name: String::new(),
                },
            ],
        }];
        let t = format_spec(&s);
        let mut back = parse_spec(&t).unwrap();
        back[0].globals[1].name.clear();
        assert_eq!(back, s);
        assert!(parse_spec("g 1 2 R exact x").is_err());
    }

    #[test]
    fn ext_to_f32() {
        // 1.0, -2.5, pi as the x87 rounds it, a value needing round-to-even, inf, NaN
        let ext = |v: f64| {
            let b = v.to_bits();
            let s = (b >> 63) as u16;
            let e = ((b >> 52) & 0x7FF) as i32;
            let m = (b & ((1 << 52) - 1)) | (1 << 52);
            let se = (s << 15) | (e - 1023 + 16383) as u16;
            let mut out = [0u8; 10];
            out[..8].copy_from_slice(&(m << 11).to_le_bytes());
            out[8..].copy_from_slice(&se.to_le_bytes());
            out
        };
        for v in [
            1.0f64,
            -2.5,
            std::f64::consts::PI,
            1.0 + 2f64.powi(-24),
            3.4e38,
            1e-40,
            1e-45,
            1e-46,
        ] {
            assert_eq!(ext_to_f32_bits(&ext(v)), (v as f32).to_bits(), "{v}");
        }
        let mut inf = [0u8; 10];
        inf[7] = 0x80;
        inf[8..].copy_from_slice(&0x7FFFu16.to_le_bytes());
        assert_eq!(ext_to_f32_bits(&inf), 0x7F80_0000);
        assert_eq!(ext_to_f32_bits(&[0; 10]), 0);
    }
}
