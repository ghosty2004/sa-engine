//! MVP-C writer batch `mvpc-fx-1`: the FX project loaders (`FxManager_c::LoadFxProject` down to the
//! `FxInterpInfo*_c::Load` keyframe readers), the three `CFileMgr` thunks they call and
//! `FxInfoManager_c::AddFxInfo`. Ported from the 1.0 US bytes; gta-reversed as a reading aid. Every hook
//! is unverified.
//!
//! The loaders read `effects.fxp` line by line: `ReadLine(file, line, 256)` through the thunk 0x4AB250,
//! then CRT `sscanf` (0x8220AD) with the original format strings. Field names are compared with
//! `repe cmpsb` over `strlen + 1` bytes (stops at the first difference, like [`field_is`]). Locals the
//! original leaves unwritten when a `sscanf` fails stay zero here (the project file is well formed).
//! The SEH frames of the originals are not needed (nothing between set-up and tear-down throws).

use sa_core::abi::{Fp, HookReg, stack_footprint};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_stream::modelinfo::vehicle::sscanf_n;

use crate::common::{ftol32, k, rd, wr};
use crate::fx::FX_POOL;

orig! {
    /// `CFileMgr::OpenFile(path, mode)` (0x538900)
    pub fn cfm_open_file(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
    /// `CFileMgr::ReadLine(file, buf, n)` (0x5389B0)
    pub fn cfm_read_line(f: GPtr<u8>, buf: GPtr<u8>, n: i32) -> bool = cdecl 0x5389B0 => sa_core::filemgr::read_line;
    /// `CFileMgr::CloseFile(file)` (0x5389D0)
    pub fn cfm_close_file(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
    /// thunk 0x4AB240 → `CFileMgr::OpenFile`
    pub fn fx_open_file(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x4AB240 => thunk_open_file;
    /// thunk 0x4AB250 → `CFileMgr::ReadLine`
    pub fn fx_read_line(f: GPtr<u8>, buf: GPtr<u8>, n: i32) -> bool = cdecl 0x4AB250 => thunk_read_line;
    /// thunk 0x4AB260 → `CFileMgr::CloseFile`
    pub fn fx_close_file(f: GPtr<u8>) -> i32 = cdecl 0x4AB260 => thunk_close_file;
    /// `CTxdStore::AddTxdSlot(const char*)` (0x731C80)
    pub fn txd_add_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731C80 => sa_stream::txdstore::add_txd_slot;
    /// `CTxdStore::LoadTxd(int, const char* filename)` (0x7320B0)
    pub fn txd_load_txd(slot: i32, path: GPtr<u8>) -> u8 = cdecl 0x7320B0 => sa_stream::txdstore::load_txd_filename;
    /// `CTxdStore::AddRef(int)` (0x731A00)
    pub fn txd_add_ref(slot: i32) = cdecl 0x731A00 => sa_stream::txdstore::add_ref;
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    pub fn txd_push_current() = cdecl 0x7316A0 => sa_stream::txdstore::push_current_txd;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    pub fn txd_pop_current() = cdecl 0x7316B0 => sa_stream::txdstore::pop_current_txd;
    /// `CTxdStore::SetCurrentTxd(int)` (0x7319C0)
    pub fn txd_set_current(slot: i32) = cdecl 0x7319C0 => sa_stream::txdstore::set_current_txd;
    /// `FxMemoryPool_c::Optimise()` (0x4A9CD0)
    pub fn pool_optimise(this: GPtr<u8>) = thiscall 0x4A9CD0 => crate::fx::fx_memory_pool_optimise;
    /// `FxMemoryPool_c::GetMem(size, align)` (0x4A9CA0)
    pub fn pool_get_mem(this: GPtr<u8>, size: u32, align: u32) -> GPtr<u8> = thiscall 0x4A9CA0 => crate::fx::fx_memory_pool_get_mem;
    /// `FxManager_c::LoadFxSystemBP(const char* filename, FILESTREAM)` (0x5C1F50)
    pub fn load_fx_system_bp_call(this: GPtr<u8>, name: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> = thiscall 0x5C1F50 => load_fx_system_bp;
    /// `FxSystemBP_c::operator new(size)` (0x4AA100)
    pub fn system_bp_new(size: u32) -> GPtr<u8> = cdecl 0x4AA100 => crate::fx::fx_system_bp_new;
    /// `FxSystemBP_c::FxSystemBP_c()` (0x4AA0D0)
    pub fn system_bp_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AA0D0 => crate::fx::fx_system_bp_constructor;
    /// `FxSystemBP_c::Load(const char* filename, FILESTREAM, int version)` (0x5C05F0)
    pub fn system_bp_load_call(this: GPtr<u8>, name: GPtr<u8>, f: GPtr<u8>, version: i32) -> u8 = thiscall 0x5C05F0 => system_bp_load;
    /// `List_c::AddItem(ListItem_c*)` (0x4A8DF0)
    pub fn list_add_item(this: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8DF0 => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    pub fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => sa_core::keygen::get_uppercase_key;
    /// `FxSystemBP_c::SetBoundingSphere(const CVector*, float)` (0x4AA200)
    pub fn system_bp_set_bounding_sphere(this: GPtr<u8>, c: GPtr<u8>, r: u32) = thiscall 0x4AA200 => crate::fx::fx_system_bp_set_bounding_sphere;
    /// `FxEmitterBP_c::operator new(size)` (0x4A1900): `g_fxMan` pool `GetMem(size, 4)`
    pub fn emitter_bp_new(size: u32) -> GPtr<u8> = cdecl 0x4A1900 => unported;
    /// `FxEmitterBP_c::FxEmitterBP_c()` (0x4A18D0)
    pub fn emitter_bp_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A18D0 => crate::fx::fx_emitter_bp_constructor;
    /// `FxEmitterBP_c::IsFxInfoPresent(int type)` (0x4A24D0)
    pub fn emitter_bp_is_fx_info_present(this: GPtr<u8>, ty: u32) -> u8 = thiscall 0x4A24D0 => crate::fx::fx_emitter_bp_is_fx_info_present;
    /// `FxPrimBP_c::Load(FILESTREAM, int version, FxName32_t* textures)`: vtable slot +4
    pub fn prim_bp_vload(this: GPtr<u8>, f: GPtr<u8>, version: i32, names: GPtr<u8>) -> u8 = thiscall vslot 0x4 => unported;
    /// `FxPrimBP_c::LoadTextures(FxName32_t* textures, int version)`: vtable slot +8
    pub fn prim_bp_vload_textures(this: GPtr<u8>, names: GPtr<u8>, version: i32) -> u8 = thiscall vslot 0x8 => unported;
    /// `FxPrimBP_c::Load(FILESTREAM, int version, FxName32_t* textures)` (0x5C2010)
    pub fn prim_bp_load_call(this: GPtr<u8>, f: GPtr<u8>, version: i32, names: GPtr<u8>) -> u8 = thiscall 0x5C2010 => prim_bp_load;
    /// `FxInfoManager_c::Load(FILESTREAM, int version)` (0x5C0B70)
    pub fn info_manager_load_call(this: GPtr<u8>, f: GPtr<u8>, version: i32) = thiscall 0x5C0B70 => info_manager_load;
    /// `FxInfoManager_c::AddFxInfo(int type)` (0x4A7B00)
    pub fn add_fx_info_call(this: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x4A7B00 => add_fx_info;
    /// `FxInfo_c::Load(FILESTREAM, int version)`: vtable slot +4
    pub fn info_vload(this: GPtr<u8>, f: GPtr<u8>, version: i32) = thiscall vslot 0x4 => unported;
    /// `RwTextureRead(const char* name, const char* mask)` (0x7F3AC0)
    pub fn rw_texture_read(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
    /// CRT `sprintf` with one argument (0x821BB5)
    pub fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
}

/// `"%s"` (0x85C714).
const FMT_S: usize = 0x85C714;
/// `"%d"` (0x859508).
const FMT_D: usize = 0x859508;
/// `"%s %s"` (0x869860).
const FMT_S_S: usize = 0x869860;
/// `"%s %d"` (0x86A6AC).
const FMT_S_D: usize = 0x86A6AC;
/// `"%s %f"` (0x86AA68).
const FMT_S_F: usize = 0x86AA68;

fn s(a: usize) -> GPtr<u8> {
    GPtr::from_addr(a)
}

fn lp<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

fn bp(b: &mut [u8]) -> GPtr<u8> {
    GPtr::from_raw(b.as_mut_ptr())
}

/// `repe cmpsb` of `n` bytes between a local buffer and the string at `at` (stops at the first
/// difference; `n` is the string's length + 1).
fn field_is(buf: &[u8], at: usize, n: usize) -> bool {
    (0..n).all(|i| buf[i] == s(at).byte_add(i as isize).read())
}

/// One scratch line + field buffer, as the loaders keep them on their frames.
struct Rd {
    file: GPtr<u8>,
    line: [u8; 0x100],
    field: [u8; 0x100],
}

impl Rd {
    fn new(file: GPtr<u8>) -> Rd {
        Rd { file, line: [0; 0x100], field: [0; 0x100] }
    }
    /// `ReadLine(file, line, 256)` through the thunk 0x4AB250.
    fn line(&mut self) {
        fx_read_line(self.file, bp(&mut self.line), 0x100);
    }
    /// `ReadLine` + `sscanf(line, "%s", field)`.
    fn skip(&mut self) {
        self.line();
        sscanf_n(bp(&mut self.line), s(FMT_S), &[bp(&mut self.field)]);
    }
    /// `ReadLine` + `sscanf(line, fmt, field, out)`.
    fn field2(&mut self, fmt: usize, out: GPtr<u8>) {
        self.line();
        sscanf_n(bp(&mut self.line), s(fmt), &[bp(&mut self.field), out]);
    }
    /// `"%s %d"` into a local int.
    fn int(&mut self) -> i32 {
        let mut v = 0i32;
        self.field2(FMT_S_D, lp(&mut v));
        v
    }
    /// `"%s %f"` into a local float.
    fn float(&mut self) -> f32 {
        let mut v = 0f32;
        self.field2(FMT_S_F, lp(&mut v));
        v
    }
}

/// `(u16)_ftol(v * K)` (`fld dword; fmul dword [K]; call _ftol`; the low word stored).
fn ftol_scaled(v: f32, kbits: u32) -> u16 {
    ftol32(X87::from_f32(v).fmul_m(k(kbits))) as u16
}

/// `thunk_FUN_008232d8` (0x4AB240, cdecl): `jmp CFileMgr::OpenFile` (0x538900).
#[hook(addr = 0x4AB240, conv = cdecl, name = "thunk_FUN_008232d8", expect = "E9 BB D6 08 00 90 90 90",
       unverified = "mvpc-fx-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "opens a file (the FX project)")]
pub fn thunk_open_file(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> {
    cfm_open_file(path, mode)
}

/// `thunk_FUN_005389b0` (0x4AB250, cdecl): `jmp CFileMgr::ReadLine` (0x5389B0).
#[hook(addr = 0x4AB250, conv = cdecl, name = "thunk_FUN_005389b0", expect = "E9 5B D7 08 00 90 90 90",
       unverified = "mvpc-fx-1", route = "split", hazards = "ax-al-return", recs = "grove-walk",
       note = "reads a line of the FX project file")]
pub fn thunk_read_line(f: GPtr<u8>, buf: GPtr<u8>, n: i32) -> bool {
    cfm_read_line(f, buf, n)
}

/// `_fclose` (0x4AB260, cdecl): `jmp CFileMgr::CloseFile` (0x5389D0).
#[hook(addr = 0x4AB260, conv = cdecl, name = "_fclose", expect = "E9 6B D7 08 00 90 90 90",
       unverified = "mvpc-fx-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "closes a file (the FX project)")]
pub fn thunk_close_file(f: GPtr<u8>) -> i32 {
    cfm_close_file(f)
}

/// `FxManager_c::LoadFxProject(const char* path)` (0x5C2420, thiscall, `ret 4`, bool in AL): the path is
/// copied and its last four characters replaced by `"PC.txd"`; `this+0x28 = AddTxdSlot("fx")`,
/// `LoadTxd(slot, txdPath)`, `AddRef`, `PushCurrentTxd`, `SetCurrentTxd`. `OpenFile(path, "r")`; a handle
/// `<= 0` (signed) returns 0 without popping the TXD. Otherwise: three lines (the second and third scanned
/// with `"%s"`), then while the field is `"FX_SYSTEM_DATA:"` (16 bytes): `LoadFxSystemBP(path, file)`
/// and two more lines (`"%s"` of the second). Close, `PopCurrentTxd`, `this+0xAC` pool `Optimise`; 1.
#[hook(addr = 0x5C2420, conv = thiscall, name = "FxManager_c::LoadFxProject", expect = "81 EC 00 02 00 00",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes,ax-al-return", recs = "grove-walk",
       note = "reads effects.fxp and effectsPC.txd (one-time loader)")]
pub fn load_fx_project(this: GPtr<u8>, path: GPtr<u8>) -> u8 {
    // 4 spare bytes in front: the original writes at `len - 4` even for shorter paths.
    let mut txd = [0u8; 0x204];
    let mut i = 0usize;
    loop {
        let c = path.byte_add(i as isize).read();
        txd[4 + i] = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
    for (j, c) in b"PC.txd\0".iter().enumerate() {
        txd[i + j] = *c;
    }
    let slot = txd_add_txd_slot(s(0x86B2FC));
    wr::<i32>(this, 0x28, slot);
    txd_load_txd(slot, bp(&mut txd[4..]));
    txd_add_ref(rd::<i32>(this, 0x28));
    txd_push_current();
    txd_set_current(rd::<i32>(this, 0x28));
    let f = fx_open_file(path, s(0x85A53C));
    if (f.addr() as u32 as i32) <= 0 {
        return 0;
    }
    let mut r = Rd::new(f);
    r.skip();
    r.line();
    r.skip();
    while field_is(&r.field, 0x86B2EC, 0x10) {
        load_fx_system_bp_call(this, path, f);
        r.line();
        r.skip();
    }
    fx_close_file(f);
    txd_pop_current();
    pool_optimise(this.byte_add(0xAC));
    1
}

/// `FxManager_c::LoadFxSystemBP(const char* filename, FILESTREAM file)` (0x5C1F50, thiscall, `ret 8`):
/// `version = sscanf(line, "%d")`; `bp = new(0x24)` (`FxSystemBP_c::operator new`), constructed when
/// non-null; `bp->Load(filename, file, version)` (called on the null pointer too, like the original);
/// `List_c::AddItem(this, bp)`; returns bp.
#[hook(addr = 0x5C1F50, conv = thiscall, name = "FxManager_c::LoadFxSystemBP", expect = "6A FF 68 BE D5 83 00",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads one FX system of effects.fxp")]
pub fn load_fx_system_bp(this: GPtr<u8>, name: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> {
    let mut r = Rd::new(f);
    let mut version = 0i32;
    r.line();
    sscanf_n(bp(&mut r.line), s(FMT_D), &[lp(&mut version)]);
    let mut sys = system_bp_new(0x24);
    if !sys.is_null() {
        sys = system_bp_ctor(sys);
    }
    system_bp_load_call(sys, name, f, version);
    list_add_item(this, sys);
    sys
}

/// `FxSystemBP_c::Load(const char* filename, FILESTREAM file, int version)` (0x5C05F0, thiscall, `ret
/// 0xC`, returns 0 in AL): one line; `version > 100`: FILENAME (`"%s %s"`); NAME (`"%s %s"`) → +8 =
/// `GetUppercaseKey(name)`; LENGTH → +0xC (`"%s %f"` straight into the member); `version >= 106`: +0x10,
/// +0x14 (floats), else both 0; PLAYMODE → byte +0x1A; CULLDIST → +0x18 = `_ftol(v * 256.0f)`; `version >
/// 103`: `"%s %f %f %f %f"` → +0x20 = 0, `SetBoundingSphere(&c, r)`; NUM_PRIMS → byte +0x1B, +0x1C =
/// `GetMem(4 * n, 4)`. Per prim: `"%s"`; for `"FX_PRIM_EMITTER_DATA:"` (22 bytes): `new(0x40)` +
/// constructor (null stays null), stored; flags +0x3C/+0x3D = 0; one line; `vtable+4 Load(file,
/// version, names[i])`; +0x3C = `IsFxInfoPresent(0x4020)`, +0x3D = `IsFxInfoPresent(0x4008)`.
/// `version >= 108`: one `"%s %d"`; `version >= 109`: one `"%s %s"`. Then every prim's `vtable+8
/// LoadTextures(names[i], version)` (also slots the original skipped). The texture names are 0x80
/// bytes per prim on the original's frame (room for 8; more overruns its frame); here 128 slots.
#[hook(addr = 0x5C05F0, conv = thiscall, name = "FxSystemBP_c::Load", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes,uninit-locals,ax-al-return", recs = "grove-walk",
       note = "reads effects.fxp; LoadTextures reads texture names a prim Load may leave unwritten")]
pub fn system_bp_load(this: GPtr<u8>, _name: GPtr<u8>, f: GPtr<u8>, version: i32) -> u8 {
    let mut r = Rd::new(f);
    let mut names = [[0u8; 0x80]; 128];
    let mut sbuf = [0u8; 0x100];
    r.line();
    if version > 0x64 {
        r.field2(FMT_S_S, bp(&mut sbuf));
    }
    r.field2(FMT_S_S, bp(&mut sbuf));
    wr::<u32>(this, 8, get_uppercase_key(bp(&mut sbuf)));
    r.field2(FMT_S_F, this.byte_add(0xC));
    if version < 0x6A {
        wr::<u32>(this, 0x10, 0);
        wr::<u32>(this, 0x14, 0);
    } else {
        r.field2(FMT_S_F, this.byte_add(0x10));
        r.field2(FMT_S_F, this.byte_add(0x14));
    }
    wr::<u8>(this, 0x1A, r.int() as u8);
    let cull = r.float();
    wr::<u16>(this, 0x18, ftol_scaled(cull, 0x4380_0000));
    if version > 0x67 {
        let mut c = [0f32; 3];
        let mut rad = 0f32;
        r.line();
        let cp = GPtr::from_raw(c.as_mut_ptr()).cast::<u8>();
        sscanf_n(bp(&mut r.line), s(0x86AFB0), &[bp(&mut r.field), cp, cp.byte_add(4), cp.byte_add(8), lp(&mut rad)]);
        wr::<u32>(this, 0x20, 0);
        system_bp_set_bounding_sphere(this, cp, rad.to_bits());
    }
    let n = r.int() as u8;
    wr::<u8>(this, 0x1B, n);
    let prims = pool_get_mem(GPtr::from_addr(FX_POOL), ((n as i8 as i32) << 2) as u32, 4);
    wr::<GPtr<u8>>(this, 0x1C, prims);
    let count = || rd::<i8>(this, 0x1B) as i32;
    let prim = |i: i32| rd::<GPtr<GPtr<u8>>>(this, 0x1C).wrapping_index(i).read();
    let mut i = 0;
    while i < count() {
        r.skip();
        if field_is(&r.field, 0x86AF98, 0x16) {
            let mut e = emitter_bp_new(0x40);
            if !e.is_null() {
                e = emitter_bp_ctor(e);
            }
            rd::<GPtr<GPtr<u8>>>(this, 0x1C).wrapping_index(i).write(e);
            wr::<u8>(prim(i), 0x3C, 0);
            wr::<u8>(prim(i), 0x3D, 0);
            r.line();
            prim_bp_vload(prim(i), f, version, bp(&mut names[(i & 127) as usize]));
            let v = emitter_bp_is_fx_info_present(prim(i), 0x4020);
            wr::<u8>(prim(i), 0x3C, v);
            let v = emitter_bp_is_fx_info_present(prim(i), 0x4008);
            wr::<u8>(prim(i), 0x3D, v);
        }
        i += 1;
    }
    if version >= 0x6C {
        r.int();
    }
    if version >= 0x6D {
        r.field2(FMT_S_S, bp(&mut sbuf));
    }
    let mut i = 0;
    while i < count() {
        prim_bp_vload_textures(prim(i), bp(&mut names[(i & 127) as usize]), version);
        i += 1;
    }
    0
}

/// `FxEmitterBP_c::Load(FILESTREAM file, int version, FxName32_t* textures)` (0x5C25F0, thiscall, `ret
/// 0xC`, 1 in AL): `FxPrimBP_c::Load(file, version, textures)`; then two `"%s %f"` fields → +0x38 and
/// +0x3A = `(u16)_ftol(v * 64.0f)` (LOD start / end).
#[hook(addr = 0x5C25F0, conv = thiscall, name = "FxEmitterBP_c::Load", expect = "8B 44 24 0C 81 EC 84 01 00 00",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes,ax-al-return", recs = "grove-walk",
       note = "reads effects.fxp")]
pub fn emitter_bp_load(this: GPtr<u8>, f: GPtr<u8>, version: i32, names: GPtr<u8>) -> u8 {
    prim_bp_load_call(this, f, version, names);
    let mut r = Rd::new(f);
    let v = r.float();
    wr::<u16>(this, 0x38, ftol_scaled(v, 0x4280_0000));
    let v = r.float();
    wr::<u16>(this, 0x3A, ftol_scaled(v, 0x4280_0000));
    1
}

/// `FxPrimBP_c::Load(FILESTREAM file, int version, FxName32_t* textures)` (0x5C2010, thiscall, `ret 0xC`,
/// 1 in AL): one `"%s"` line, one `"%s %s"` line, then a matrix line (`"%s"` + 12 floats). When the
/// floats are exactly the identity rows (1,0,0, 0,1,0, 0,0,1, 0,0,0; x87 `fcomp` + `jp` on 0x44, any
/// NaN counts as different) +8 = null, else +8 = `GetMem(0x18, 2)` with the 12 shorts
/// `_ftol(f[i] * 32767.0f)`. Texture names: `"%s %s"` into `textures[0]`, with `version > 101` also
/// `[1..3]` (0x20 apart); then three `"%s %d"` → bytes +7 (src blend), +5 (dst blend), +6 (alpha on);
/// finally `FxInfoManager_c::Load(this + 0x2C, file, version)`.
#[hook(addr = 0x5C2010, conv = thiscall, name = "FxPrimBP_c::Load", expect = "81 EC B4 01 00 00",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes,ax-al-return", recs = "grove-walk",
       note = "reads effects.fxp")]
pub fn prim_bp_load(this: GPtr<u8>, f: GPtr<u8>, version: i32, names: GPtr<u8>) -> u8 {
    let mut r = Rd::new(f);
    let mut sbuf = [0u8; 0x100];
    r.skip();
    r.field2(FMT_S_S, bp(&mut sbuf));
    r.line();
    let mut m = [0f32; 12];
    let mp = GPtr::from_raw(m.as_mut_ptr()).cast::<u8>();
    let mut args = [GPtr::<u8>::NULL; 13];
    args[0] = bp(&mut r.field);
    for (j, a) in args.iter_mut().skip(1).enumerate() {
        *a = mp.byte_add(4 * j as isize);
    }
    sscanf_n(bp(&mut r.line), s(0x86B2C4), &args);
    const ID: [bool; 12] = [true, false, false, false, true, false, false, false, true, false, false, false];
    let identity = m.iter().zip(ID).all(|(v, one)| {
        let c = if one { 1.0f32 } else { 0.0f32 };
        !crate::common::jp(X87::from_f32(*v), X87::from_f32(c), 0x44)
    });
    if identity {
        wr::<u32>(this, 8, 0);
    } else {
        let p = pool_get_mem(GPtr::from_addr(FX_POOL), 0x18, 2);
        wr::<GPtr<u8>>(this, 8, p);
        for (j, v) in m.iter().enumerate() {
            let q = ftol_scaled(*v, 0x46FF_FE00);
            rd::<GPtr<u16>>(this, 8).wrapping_index(j as i32).write(q);
        }
    }
    r.field2(FMT_S_S, names);
    if version > 0x65 {
        r.field2(FMT_S_S, names.byte_add(0x20));
        r.field2(FMT_S_S, names.byte_add(0x40));
        r.field2(FMT_S_S, names.byte_add(0x60));
    }
    wr::<u8>(this, 7, r.int() as u8);
    wr::<u8>(this, 5, r.int() as u8);
    wr::<u8>(this, 6, r.int() as u8);
    info_manager_load_call(this.byte_add(0x2C), f, version);
    1
}

/// `FxEmitterBP_c::LoadTextures(FxName32_t* textures, int version)` (0x5C0A30, thiscall, `ret 8`, 1 in
/// AL): `+0xC = RwTextureRead(name, sprintf("%sm", name))`, retried with a null mask when null; with
/// `version > 101` the same for textures 1..3 (+0x10, +0x14, +0x18), each skipped when its name is
/// `"NULL"` (5-byte compare).
#[hook(addr = 0x5C0A30, conv = thiscall, name = "FxEmitterBP_c::LoadTextures", expect = "83 EC 40 53 55",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes,ax-al-return", recs = "grove-walk",
       note = "reads FX textures (one-time loader)")]
pub fn emitter_bp_load_textures(this: GPtr<u8>, names: GPtr<u8>, version: i32) -> u8 {
    let mut mask = [0u8; 0x40];
    let read = |name: GPtr<u8>, off: isize, mask: &mut [u8; 0x40]| {
        sprintf1(bp(mask), s(0x86AFC0), name);
        let t = rw_texture_read(name, bp(mask));
        wr::<GPtr<u8>>(this, off, t);
        if t.is_null() {
            let t = rw_texture_read(name, GPtr::NULL);
            wr::<GPtr<u8>>(this, off, t);
        }
    };
    read(names, 0xC, &mut mask);
    if version > 0x65 {
        for j in 1..4isize {
            let name = names.byte_add(0x20 * j);
            let is_null = (0..5).all(|i| name.byte_add(i).read() == s(0x859E24).byte_add(i).read());
            if !is_null {
                read(name, 0xC + 4 * j, &mut mask);
            }
        }
    }
    1
}

/// The `FxInfoManager_c::Load` name table: (string, compare length, type, has `TIMEMODEPRT:`).
const INFO_NAMES: [(usize, usize, u32, bool); 32] = [
    (0x86B2A8, 0x15, 0x1001, false),
    (0x86B290, 0x15, 0x1004, false),
    (0x86B278, 0x16, 0x1008, false),
    (0x86B264, 0x14, 0x1010, false),
    (0x86B24C, 0x16, 0x1020, false),
    (0x86B234, 0x15, 0x1040, false),
    (0x86B220, 0x14, 0x1080, false),
    (0x86B208, 0x18, 0x1100, false),
    (0x86B1EC, 0x19, 0x1200, false),
    (0x86B1D8, 0x14, 0x2001, true),
    (0x86B1B8, 0x14, 0x2002, true),
    (0x86B1A0, 0x17, 0x2004, true),
    (0x86B188, 0x18, 0x2008, true),
    (0x86B16C, 0x1A, 0x2010, true),
    (0x86B150, 0x1C, 0x2020, true),
    (0x86B13C, 0x13, 0x2040, true),
    (0x86B124, 0x15, 0x2080, true),
    (0x86B10C, 0x17, 0x2100, true),
    (0x86B0F8, 0x14, 0x2200, true),
    (0x86B0DC, 0x19, 0x2400, true),
    (0x86B0C4, 0x15, 0x4001, true),
    (0x86B0B0, 0x13, 0x4002, true),
    (0x86B094, 0x19, 0x4004, true),
    (0x86B07C, 0x17, 0x4008, true),
    (0x86B068, 0x14, 0x4010, true),
    (0x86B054, 0x13, 0x4020, true),
    (0x86B040, 0x12, 0x4040, true),
    (0x86B028, 0x16, 0x4080, true),
    (0x86B00C, 0x1A, 0x4100, true),
    (0x86AFF4, 0x16, 0x4200, true),
    (0x86AFD8, 0x1B, 0x4400, true),
    (0x86AFC4, 0x14, 0x8001, true),
];

/// `FxInfoManager_c::Load(FILESTREAM file, int version)` (0x5C0B70, thiscall, `ret 8`): two lines, the
/// second `"%s %d"` → +0 (info count); +8 = +9 = -1; +4 = `GetMem(4 * n, 4)`. Per info: timeMode = 1;
/// `"%s"`; the name is looked up in the 32-entry table (first match). Non-emission types read
/// `TIMEMODEPRT:` (`"%s %d"`) when `(double)version >= 0.7` (`fild` + `fcomp` 0.7, `test ah, 1`). A
/// known name: `infos[i] = AddFxInfo(type)`, `infos[i]->Load(file, version)` (vtable +4), one line. For
/// every name (also unknown ones, reading `infos[i]` the pool left unwritten): byte +6 of `infos[i]` =
/// timeMode; type word +4: `& 0x1000` none, `& 0x2000` → +8 = i if still -1, `& 0xC000` → +9 likewise.
/// Afterwards +9 = (u8)count when still -1, then +8 = +9 when still -1.
#[hook(addr = 0x5C0B70, conv = thiscall, name = "FxInfoManager_c::Load", expect = "81 EC 88 01 00 00",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads effects.fxp; an unknown info name writes through an unwritten pool slot like the original")]
pub fn info_manager_load(this: GPtr<u8>, f: GPtr<u8>, version: i32) {
    let mut r = Rd::new(f);
    r.line();
    r.field2(FMT_S_D, this);
    wr::<u8>(this, 8, 0xFF);
    wr::<u8>(this, 9, 0xFF);
    let infos = pool_get_mem(GPtr::from_addr(FX_POOL), (rd::<i32>(this, 0) << 2) as u32, 4);
    wr::<GPtr<u8>>(this, 4, infos);
    let info = |i: i32| rd::<GPtr<GPtr<u8>>>(this, 4).wrapping_index(i).read();
    let mut i = 0i32;
    while i < rd::<i32>(this, 0) {
        let mut time_mode = 1i32;
        r.skip();
        if let Some(&(_, _, ty, prt)) = INFO_NAMES.iter().find(|(a, n, _, _)| field_is(&r.field, *a, *n)) {
            if prt && version >= 1 {
                r.field2(FMT_S_D, lp(&mut time_mode));
            }
            let p = add_fx_info_call(this, ty);
            rd::<GPtr<GPtr<u8>>>(this, 4).wrapping_index(i).write(p);
            info_vload(info(i), f, version);
            r.line();
        }
        let p = info(i);
        wr::<u8>(p, 6, time_mode as u8);
        let hi = (rd::<u16>(p, 4) >> 8) as u8;
        if hi & 0x10 == 0 {
            if hi & 0x20 != 0 {
                if rd::<u8>(this, 8) == 0xFF {
                    wr::<u8>(this, 8, i as u8);
                }
            } else if hi & 0xC0 != 0 && rd::<u8>(this, 9) == 0xFF {
                wr::<u8>(this, 9, i as u8);
            }
        }
        i += 1;
    }
    if rd::<u8>(this, 9) == 0xFF {
        wr::<u8>(this, 9, rd::<u8>(this, 0));
    }
    if rd::<u8>(this, 8) == 0xFF {
        wr::<u8>(this, 8, rd::<u8>(this, 9));
    }
}

orig! {
    /// `FxInfoEmRate_c::FxInfoEmRate_c()` (0x4A6A00)
    pub fn c_em_rate(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A6A00 => crate::fx::info::fx_info_em_rate_ctor;
    /// `FxInfoEmSize_c::FxInfoEmSize_c()` (0x4A4C20)
    pub fn c_em_size(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A4C20 => crate::fx::info::fx_info_em_size_ctor;
    /// `FxInfoEmSpeed_c::FxInfoEmSpeed_c()` (0x4A4D00)
    pub fn c_em_speed(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A4D00 => crate::fx::info::fx_info_em_speed_ctor;
    /// `FxInfoEmDir_c::FxInfoEmDir_c()` (0x4A4DC0)
    pub fn c_em_dir(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A4DC0 => crate::fx::info::fx_info_em_dir_ctor;
    /// `FxInfoEmAngle_c::FxInfoEmAngle_c()` (0x4A4E90)
    pub fn c_em_angle(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A4E90 => crate::fx::info::fx_info_em_angle_ctor;
    /// `FxInfoEmLife_c::FxInfoEmLife_c()` (0x4A4F50)
    pub fn c_em_life(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A4F50 => crate::fx::info::fx_info_em_life_ctor;
    /// `FxInfoEmPos_c::FxInfoEmPos_c()` (0x4A5010)
    pub fn c_em_pos(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5010 => crate::fx::info::fx_info_em_pos_ctor;
    /// `FxInfoEmWeather_c::FxInfoEmWeather_c()` (0x4A50E0)
    pub fn c_em_weather(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A50E0 => crate::fx::info::fx_info_em_weather_ctor;
    /// `FxInfoEmRotation_c::FxInfoEmRotation_c()` (0x4A51B0)
    pub fn c_em_rotation(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A51B0 => crate::fx::info::fx_info_em_rotation_ctor;
    /// `FxInfoNoise_c::FxInfoNoise_c()` (0x4A5270)
    pub fn c_noise(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5270 => crate::mvpa_fx1::fx_info_noise_ctor;
    /// `FxInfoForce_c::FxInfoForce_c()` (0x4A5450)
    pub fn c_force(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5450 => crate::fx::info::fx_info_force_ctor;
    /// `FxInfoFriction_c::FxInfoFriction_c()` (0x4A5550)
    pub fn c_friction(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5550 => crate::fx::info::fx_info_friction_ctor;
    /// `FxInfoAttractPt_c::FxInfoAttractPt_c()` (0x4A5650)
    pub fn c_attract_pt(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5650 => crate::fx::info::fx_info_attract_pt_ctor;
    /// `FxInfoAttractLine_c::FxInfoAttractLine_c()` (0x4A57C0)
    pub fn c_attract_line(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A57C0 => crate::fx::info::fx_info_attract_line_ctor;
    /// `FxInfoGroundCollide_c::FxInfoGroundCollide_c()` (0x4A5980)
    pub fn c_ground_collide(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5980 => crate::fx::info::fx_info_ground_collide_ctor;
    /// `FxInfoWind_c::FxInfoWind_c()` (0x4A5A10)
    pub fn c_wind(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5A10 => crate::fx::info::fx_info_wind_ctor;
    /// `FxInfoJitter_c::FxInfoJitter_c()` (0x4A5B20)
    pub fn c_jitter(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5B20 => crate::fx::info::fx_info_jitter_ctor;
    /// `FxInfoRotSpeed_c::FxInfoRotSpeed_c()` (0x4A5CB0)
    pub fn c_rot_speed(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5CB0 => crate::fx::info::fx_info_rot_speed_ctor;
    /// `FxInfoFloat_c::FxInfoFloat_c()` (0x4A5DA0)
    pub fn c_float(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5DA0 => crate::fx::info::fx_info_float_ctor;
    /// `FxInfoUnderwater_c::FxInfoUnderwater_c()` (0x4A5E20)
    pub fn c_underwater(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5E20 => crate::fx::info::fx_info_underwater_ctor;
    /// `FxInfoColour_c::FxInfoColour_c()` (0x4A5EA0)
    pub fn c_colour(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5EA0 => crate::fx::info::fx_info_colour_ctor;
    /// `FxInfoSize_c::FxInfoSize_c()` (0x4A5FA0)
    pub fn c_size(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A5FA0 => crate::fx::info::fx_info_size_ctor;
    /// `FxInfoSpriteRect_c::FxInfoSpriteRect_c()` (0x4A60B0)
    pub fn c_sprite_rect(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A60B0 => crate::fx::info::fx_info_sprite_rect_ctor;
    /// `FxInfoHeatHaze_c::FxInfoHeatHaze_c()` (0x4A61A0)
    pub fn c_heat_haze(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A61A0 => crate::fx::info::fx_info_heat_haze_ctor;
    /// `FxInfoTrail_c::FxInfoTrail_c()` (0x4A6220)
    pub fn c_trail(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A6220 => crate::fx::info::fx_info_trail_ctor;
    /// `FxInfoFlat_c::FxInfoFlat_c()` (0x4A6320)
    pub fn c_flat(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A6320 => crate::fx::info::fx_info_flat_ctor;
    /// `FxInfoDir_c::FxInfoDir_c()` (0x4A6440)
    pub fn c_dir(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A6440 => crate::fx::info::fx_info_dir_ctor;
    /// `FxInfoAnimTexture_c::FxInfoAnimTexture_c()` (0x4A6550)
    pub fn c_anim_texture(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A6550 => crate::fx::info::fx_info_anim_texture_ctor;
    /// `FxInfoColourRange_c::FxInfoColourRange_c()` (0x4A6630)
    pub fn c_colour_range(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A6630 => crate::fx::info::fx_info_colour_range_ctor;
    /// `FxInfoSelfLit_c::FxInfoSelfLit_c()` (0x4A6760)
    pub fn c_self_lit(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A6760 => crate::fx::info::fx_info_self_lit_ctor;
    /// `FxInfoColourBright_c::FxInfoColourBright_c()` (0x4A67D0)
    pub fn c_colour_bright(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A67D0 => crate::fx::info::fx_info_colour_bright_ctor;
    /// `FxInfoSmoke_c::FxInfoSmoke_c()` (0x4A68E0)
    pub fn c_smoke(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A68E0 => crate::fx::info::fx_info_smoke_ctor;
}

/// `FxInfoManager_c::AddFxInfo(int type)` (0x4A7B00, thiscall, `ret 4`): for the 31 known types (0x1002
/// is not one) `GetMem(0x18, 4)` from the `g_fxMan` pool and, when non-null, the type's constructor
/// (its return value is returned); anything else (or a null allocation) returns null. `this` is unused.
#[hook(addr = 0x4A7B00, conv = thiscall, name = "FxInfoManager_c::AddFxInfo", expect = "64 A1 00 00 00 00",
       unverified = "mvpc-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "allocates from the FX memory pool")]
pub fn add_fx_info(_this: GPtr<u8>, ty: u32) -> GPtr<u8> {
    let ctor: fn(GPtr<u8>) -> GPtr<u8> = match ty {
        0x1001 => c_em_rate,
        0x1004 => c_em_size,
        0x1008 => c_em_speed,
        0x1010 => c_em_dir,
        0x1020 => c_em_angle,
        0x1040 => c_em_life,
        0x1080 => c_em_pos,
        0x1100 => c_em_weather,
        0x1200 => c_em_rotation,
        0x2001 => c_noise,
        0x2002 => c_force,
        0x2004 => c_friction,
        0x2008 => c_attract_pt,
        0x2010 => c_attract_line,
        0x2020 => c_ground_collide,
        0x2040 => c_wind,
        0x2080 => c_jitter,
        0x2100 => c_rot_speed,
        0x2200 => c_float,
        0x2400 => c_underwater,
        0x4001 => c_colour,
        0x4002 => c_size,
        0x4004 => c_sprite_rect,
        0x4008 => c_heat_haze,
        0x4010 => c_trail,
        0x4020 => c_flat,
        0x4040 => c_dir,
        0x4080 => c_anim_texture,
        0x4100 => c_colour_range,
        0x4200 => c_self_lit,
        0x4400 => c_colour_bright,
        0x8001 => c_smoke,
        _ => return GPtr::NULL,
    };
    let p = pool_get_mem(GPtr::from_addr(FX_POOL), 0x18, 4);
    if p.is_null() {
        return GPtr::NULL;
    }
    ctor(p)
}

/// The shared body of the four `FxInterpInfo*_c::Load(FILESTREAM)` (thiscall, `ret 4`). Layout: +4
/// looped, +5 key count, +6 value count, +8 key times (u16), +0xC per-value key arrays. Per value i:
/// two `"%s"` lines; LOOPED and NUM_KEYS (`"%s %d"`, low bytes → +4, +5); for i == 0 +8 = `GetMem(2 *
/// keys, 2)`; `keys[i] = GetMem(elem * keys, elem)`; per key j: a `"%s"` line, TIME (`"%s %f"`) →
/// `times[j] = _ftol(t * 256.0f)`, VAL (`"%s %f"`) → `store(keys[i], j, v)`. Counts are re-read from
/// the members (signed bytes) each loop.
fn interp_load(this: GPtr<u8>, f: GPtr<u8>, elem: u32, store: impl Fn(GPtr<u8>, i32, &mut Rd)) {
    let mut r = Rd::new(f);
    let mut i = 0i32;
    while i < rd::<i8>(this, 6) as i32 {
        r.skip();
        r.skip();
        wr::<u8>(this, 4, r.int() as u8);
        let n = r.int() as u8;
        wr::<u8>(this, 5, n);
        if i == 0 {
            let t = pool_get_mem(GPtr::from_addr(FX_POOL), ((n as i8 as i32) << 1) as u32, 2);
            wr::<GPtr<u8>>(this, 8, t);
        }
        let n = rd::<i8>(this, 5) as i32;
        let kp = pool_get_mem(GPtr::from_addr(FX_POOL), (n * elem as i32) as u32, elem);
        rd::<GPtr<GPtr<u8>>>(this, 0xC).wrapping_index(i).write(kp);
        let mut j = 0i32;
        while j < rd::<i8>(this, 5) as i32 {
            r.skip();
            let t = r.float();
            let q = ftol_scaled(t, 0x4380_0000);
            rd::<GPtr<u16>>(this, 8).wrapping_index(j).write(q);
            let keys = rd::<GPtr<GPtr<u8>>>(this, 0xC).wrapping_index(i).read();
            store(keys, j, &mut r);
            j += 1;
        }
        i += 1;
    }
}

/// `FxInterpInfoFloat_c::Load(FILESTREAM)` (0x5C16F0): [`interp_load`] with float keys scanned straight
/// into `keys[i][j]`.
#[hook(addr = 0x5C16F0, conv = thiscall, name = "FxInterpInfoFloat_c::Load", expect = "81 EC 88 01 00 00",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads effects.fxp")]
pub fn interp_float_load(this: GPtr<u8>, f: GPtr<u8>) {
    interp_load(this, f, 4, |keys, j, r| r.field2(FMT_S_F, keys.cast::<u32>().wrapping_index(j).cast()));
}

/// The short-key variants: `keys[i][j] = (u16)_ftol(v * K)`.
fn interp_load_short(this: GPtr<u8>, f: GPtr<u8>, kbits: u32) {
    interp_load(this, f, 2, |keys, j, r| {
        let v = r.float();
        keys.cast::<u16>().wrapping_index(j).write(ftol_scaled(v, kbits));
    });
}

/// `FxInterpInfoU255_c::Load(FILESTREAM)` (0x5C18F0): [`interp_load`] with short keys `_ftol(v * 256.0f)`.
#[hook(addr = 0x5C18F0, conv = thiscall, name = "FxInterpInfoU255_c::Load", expect = "81 EC 88 01 00 00",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads effects.fxp")]
pub fn interp_u255_load(this: GPtr<u8>, f: GPtr<u8>) {
    interp_load_short(this, f, 0x4380_0000);
}

/// `FxInterpInfo32_c::Load(FILESTREAM)` (0x5C1B10): [`interp_load`] with short keys `_ftol(v * 1000.0f)`.
#[hook(addr = 0x5C1B10, conv = thiscall, name = "FxInterpInfo32_c::Load", expect = "81 EC 88 01 00 00",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads effects.fxp")]
pub fn interp_32_load(this: GPtr<u8>, f: GPtr<u8>) {
    interp_load_short(this, f, 0x447A_0000);
}

/// `FxInterpInfo255_c::Load(FILESTREAM)` (0x5C1D30): [`interp_load`] with short keys `_ftol(v * 128.0f)`.
#[hook(addr = 0x5C1D30, conv = thiscall, name = "FxInterpInfo255_c::Load", expect = "81 EC 88 01 00 00",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads effects.fxp")]
pub fn interp_255_load(this: GPtr<u8>, f: GPtr<u8>) {
    interp_load_short(this, f, 0x4300_0000);
}

orig! {
    /// `FxManager_c::CreateFxSystem(const char* name, const RwV3d* pos, RwMatrix* parent, bool ignoreBounds)` (0x4A9BE0)
    pub fn create_fx_system_name(this: GPtr<u8>, name: GPtr<u8>, pos: GPtr<u8>, parent: GPtr<u8>, ignore: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => crate::fx::fx_manager_create_fx_system_name;
    /// `FxManager_c::ShouldCreate(FxSystemBP_c*, RwMatrix*, RwMatrix* parent, bool ignoreBounds)` (0x4A9500)
    pub fn should_create(this: GPtr<u8>, bp: GPtr<u8>, m: GPtr<u8>, parent: GPtr<u8>, ignore: u32) -> u8 = thiscall 0x4A9500 => crate::mvpa_fx2::fx_manager_should_create;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    pub fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `operator new(size_t)` (0x82119A)
    pub fn operator_new(n: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `operator new(size_t)` (0x821195, the second entry FxSystem_c::Init calls)
    pub fn operator_new_821195(n: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// `FxSystem_c::FxSystem_c()` (0x4AAF00)
    pub fn system_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AAF00 => crate::fx::fx_system_constructor;
    /// `FxSystem_c::Init(FxSystemBP_c*, RwMatrix* local, RwMatrix* parent)` (0x4AA750)
    pub fn system_init_call(this: GPtr<u8>, bp: GPtr<u8>, m: GPtr<u8>, parent: GPtr<u8>) -> u8 = thiscall 0x4AA750 => system_init;
    /// `Fx_c::GetFxQuality()` (0x49EA50)
    pub fn get_fx_quality(this: GPtr<u8>) -> i32 = thiscall 0x49EA50 => unported;
    /// `FxSystem_c::SetRateMult(float)` (0x4AA6F0)
    pub fn system_set_rate_mult(this: GPtr<u8>, mult: u32) = thiscall 0x4AA6F0 => crate::fx::fx_system_set_rate_mult;
    /// `ListItem_c::ListItem_c()` (0x4A8DB0)
    pub fn list_item_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8DB0 => crate::fx::list_item_ctor_port;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    pub fn system_play(this: GPtr<u8>) = thiscall 0x4AA2F0 => crate::fx::fx_system_play;
    /// `CMemoryMgr::Malloc(size, hint)` (0x72F420)
    pub fn memory_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `FxPrimBP_c::CreateInstance()`: vtable slot +0xC
    pub fn prim_bp_vcreate_instance(this: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0xC => unported;
    /// `FxPrim_c::Init(FxPrimBP_c*, FxSystem_c*)`: vtable slot +4
    pub fn prim_vinit(this: GPtr<u8>, bp: GPtr<u8>, sys: GPtr<u8>) = thiscall vslot 0x4 => unported;
    /// `CAEFireAudioEntity::Initialise(FxSystem_c*)` (0x4DCF20)
    pub fn fire_audio_initialise(this: GPtr<u8>, sys: GPtr<u8>) = thiscall 0x4DCF20 => unported;
}

/// `g_fxMan` (FxManager_c, 0xA9AE80).
const FX_MAN: usize = 0xA9AE80;
/// `g_fx` (Fx_c, 0xA9AE00).
const G_FX: usize = 0xA9AE00;

/// The 17 `Fx_c` static systems (`prt_blood` .. `prt_glass`), in member order (+0 .. +0x40).
const STATIC_SYSTEMS: [usize; 17] = [
    0x85A6C8, 0x85A6B8, 0x85A6AC, 0x85A69C, 0x85A688, 0x85A678, 0x85A66C, 0x85A660, 0x85A650, 0x85A638,
    0x85A62C, 0x85A620, 0x85A614, 0x85A608, 0x85A5F8, 0x85A5E8, 0x85A5DC,
];

/// `Fx_c::InitStaticSystems()` (0x49E660, thiscall): member `i` = `g_fxMan.CreateFxSystem(name_i, &zero,
/// null, true)` with one zeroed local vector shared by all 17 calls.
#[hook(addr = 0x49E660, conv = thiscall, name = "Fx_c::InitStaticSystems", expect = "83 EC 0C 56 6A 01",
       unverified = "mvpc-fx-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "one-time initialiser: creates the 17 static particle systems")]
pub fn fx_init_static_systems(this: GPtr<u8>) {
    let mut pos = [0u32; 3];
    let pp = GPtr::from_raw(pos.as_mut_ptr()).cast::<u8>();
    for (i, name) in STATIC_SYSTEMS.iter().enumerate() {
        let sys = create_fx_system_name(GPtr::from_addr(FX_MAN), s(*name), pp, GPtr::NULL, 1);
        wr::<GPtr<u8>>(this, 4 * i as isize, sys);
    }
}

/// `FxManager_c::CreateFxSystem(FxSystemBP_c* bp, const RwV3d* pos, RwMatrix* parent, bool
/// ignoreBounds)` (0x4A96B0, thiscall, `ret 0x10`): null bp → null. Takes the next matrix of the pool
/// (`[0xA9AF0C + 4 * [0xA9AF08]++]`), sets it to the identity at `pos` (flags +0xC `|= 0x20003`),
/// `RwMatrixUpdate`; `ShouldCreate(bp, m, parent, ignoreBounds)` false → pool index back, null.
/// Otherwise `new(0x104)` + constructor (null stays null), `Init(bp, m, parent)`, `List_c::AddItem(this +
/// 0xC, sys)`, rate multiplier by `g_fx.GetFxQuality()` (0: 0.5, 1: 0.75, >= 2: 1.0, negative: none,
/// asked up to three times), pool index back; returns sys.
#[hook(addr = 0x4A96B0, conv = thiscall, name = "FxManager_c::CreateFxSystem-3", expect = "90 E9 89 81 F5 FF",
       unverified = "mvpc-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "allocates an FxSystem_c and writes a pooled matrix")]
pub fn create_fx_system_bp(this: GPtr<u8>, bp_: GPtr<u8>, pos: GPtr<u8>, parent: GPtr<u8>, ignore: u32) -> GPtr<u8> {
    if bp_.is_null() {
        return GPtr::NULL;
    }
    let idx_cell = s(0xA9AF08).cast::<i32>();
    let idx = idx_cell.read();
    let m = s(0xA9AF0C).cast::<GPtr<u8>>().wrapping_index(idx).read();
    idx_cell.write(idx.wrapping_add(1));
    let flags = rd::<u32>(m, 0xC);
    for o in [0x28, 0x14, 0] {
        wr::<u32>(m, o, 0x3F80_0000);
    }
    for o in [0x38, 0x34, 0x30, 0x10, 8, 4, 0x24, 0x20, 0x18] {
        wr::<u32>(m, o, 0);
    }
    wr::<u32>(m, 0xC, flags | 0x20003);
    for o in [0, 4, 8] {
        wr::<u32>(m, 0x30 + o, rd::<u32>(pos, o));
    }
    rw_matrix_update(m);
    if should_create(this, bp_, m, parent, ignore) == 0 {
        idx_cell.write(idx_cell.read().wrapping_sub(1));
        return GPtr::NULL;
    }
    let mut sys = operator_new(0x104);
    if !sys.is_null() {
        sys = system_ctor(sys);
    }
    system_init_call(sys, bp_, m, parent);
    list_add_item(this.byte_add(0xC), sys);
    let fx = GPtr::from_addr(G_FX);
    if get_fx_quality(fx) == 0 {
        system_set_rate_mult(sys, 0x3F00_0000);
    } else if get_fx_quality(fx) == 1 {
        system_set_rate_mult(sys, 0x3F40_0000);
    } else if get_fx_quality(fx) >= 2 {
        system_set_rate_mult(sys, 0x3F80_0000);
    }
    idx_cell.write(idx_cell.read().wrapping_sub(1));
    sys
}

/// `FxSystem_c::Init(FxSystemBP_c* bp, RwMatrix* local, RwMatrix* parent)` (0x4AA750, thiscall, `ret
/// 0xC`, 1 in AL): +8 = bp, the 16 dwords of `local` → +0x10, +0xC = parent; +0x50 = 1, +0x51 = +0x52 =
/// 0, +0x54 = 0, word +0x5C = 0, +0x5E = +0x60 = 1000, +0x68 .. +0x74 = 0, byte +0x62 = `(b & 0xFC) |
/// 4`. With a bounding sphere in bp (+0x20): `p = CMemoryMgr::Malloc(0x14, 0)`, `p+0x10 = 0` when
/// non-null, +0x74 = p and the 5 dwords of bp's sphere copied into p (also when null, like the
/// original). +0x78 = `operator new(4 * prims)` (0x821195); per prim `prims[i] =
/// bpPrim[i]->CreateInstance()` (vtable +0xC) and `prims[i]->Init(bpPrim[i], this)` (vtable +4);
/// `CAEFireAudioEntity::Initialise(this + 0x7C, this)`.
#[hook(addr = 0x4AA750, conv = thiscall, name = "FxSystem_c::Init", expect = "8B 44 24 0C 53",
       unverified = "mvpc-fx-1", route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk",
       note = "allocates the prim array and the bounding sphere")]
pub fn system_init(this: GPtr<u8>, bp_: GPtr<u8>, local: GPtr<u8>, parent: GPtr<u8>) -> u8 {
    wr::<GPtr<u8>>(this, 8, bp_);
    for o in 0..16isize {
        wr::<u32>(this, 0x10 + 4 * o, rd::<u32>(local, 4 * o));
    }
    let b = rd::<u8>(this, 0x62);
    wr::<GPtr<u8>>(this, 0xC, parent);
    wr::<u32>(this, 0x54, 0);
    wr::<u8>(this, 0x50, 1);
    wr::<u8>(this, 0x51, 0);
    wr::<u8>(this, 0x52, 0);
    wr::<u16>(this, 0x5C, 0);
    wr::<u16>(this, 0x5E, 1000);
    wr::<u16>(this, 0x60, 1000);
    wr::<u32>(this, 0x68, 0);
    wr::<u32>(this, 0x6C, 0);
    wr::<u32>(this, 0x70, 0);
    wr::<u8>(this, 0x62, (b & 0xFC) | 4);
    wr::<u32>(this, 0x74, 0);
    if !rd::<GPtr<u8>>(bp_, 0x20).is_null() {
        let p = memory_malloc(0x14, 0);
        if !p.is_null() {
            wr::<u32>(p, 0x10, 0);
        }
        wr::<GPtr<u8>>(this, 0x74, p);
        let src = rd::<GPtr<u8>>(bp_, 0x20);
        for o in 0..5isize {
            wr::<u32>(p, 4 * o, rd::<u32>(src, 4 * o));
        }
    }
    let n = rd::<i8>(bp_, 0x1B) as i32;
    wr::<GPtr<u8>>(this, 0x78, operator_new_821195((n << 2) as u32));
    let mut i = 0;
    while i < rd::<i8>(bp_, 0x1B) as i32 {
        let bpp = rd::<GPtr<GPtr<u8>>>(bp_, 0x1C).wrapping_index(i).read();
        let prim = prim_bp_vcreate_instance(bpp);
        rd::<GPtr<GPtr<u8>>>(this, 0x78).wrapping_index(i).write(prim);
        let prim = rd::<GPtr<GPtr<u8>>>(this, 0x78).wrapping_index(i).read();
        let bpp = rd::<GPtr<GPtr<u8>>>(bp_, 0x1C).wrapping_index(i).read();
        prim_vinit(prim, bpp, this);
        i += 1;
    }
    fire_audio_initialise(this.byte_add(0x7C), this);
    1
}

/// `Fx_c::CreateEntityFx(CEntity* entity, const char* name, RwV3d* pos, RwMatrix* m)` (0x4A11E0,
/// thiscall, `ret 0x10`): `sys = g_fxMan.CreateFxSystem(name, pos, m, true)`; when non-null: `item =
/// new(0x10)` + `ListItem_c` constructor (a null item is written through, like the original), item+8 =
/// sys, item+0xC = entity, `List_c::AddItem(this + 0x44, item)`, `item->sys->Play()`.
#[hook(addr = 0x4A11E0, conv = thiscall, name = "Fx_c::CreateEntityFx", expect = "64 A1 00 00 00 00",
       unverified = "mvpc-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "allocates the entity FX list item")]
pub fn fx_create_entity_fx(this: GPtr<u8>, entity: GPtr<u8>, name: GPtr<u8>, pos: GPtr<u8>, m: GPtr<u8>) {
    let sys = create_fx_system_name(GPtr::from_addr(FX_MAN), name, pos, m, 1);
    if sys.is_null() {
        return;
    }
    let item = operator_new(0x10);
    if !item.is_null() {
        list_item_ctor(item);
    }
    wr::<GPtr<u8>>(item, 8, sys);
    wr::<GPtr<u8>>(item, 0xC, entity);
    list_add_item(this.byte_add(0x44), item);
    system_play(rd::<GPtr<u8>>(item, 8));
}

orig! {
    /// `RwV3dNormalize(RwV3d* out, const RwV3d* in)` (0x7ED9B0): the length in ST0
    pub fn rw_v3d_normalize(out: GPtr<u8>, inp: GPtr<u8>) -> f64 = cdecl 0x7ED9B0 => unported;
    /// `RwV3dTransformVectors(RwV3d* out, const RwV3d* in, int n, const RwMatrix*)` (0x7EDDF0)
    pub fn rw_v3d_transform_vectors(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDDF0 => unported;
    /// `FxManager_c::FxRwMatrixCreate()` (0x4A9440)
    pub fn fx_rw_matrix_create(this: GPtr<u8>) -> u32 = thiscall 0x4A9440 => crate::fx::fx_manager_fx_rw_matrix_create;
    /// `FxManager_c::FxRwMatrixDestroy(RwMatrix*)` (0x4A9460)
    pub fn fx_rw_matrix_destroy(this: GPtr<u8>, m: u32) = thiscall 0x4A9460 => crate::fx::fx_manager_fx_rw_matrix_destroy;
}

fn xf(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `(f32)(a * b - c * d)`: `fld a; fmul b; fld c; fmul d; fsubp st(1); fstp dword`.
fn cross1(a: f32, b: f32, c: f32, d: f32) -> f32 {
    xf(a).fmul_m(b).fsub(xf(c).fmul_m(d)).to_f32()
}

/// `Fx_c::RotateVecIntoVec(RwV3d* out, const RwV3d* in, const RwV3d* dir)` (0x4A1660, cdecl): the
/// constant `u` = (0.4243, 0.5657, 0.7071) (0x85A77C); `r = normalize(dir × u)` (each component
/// `(f32)(a*b - c*d)`, `RwV3dNormalize(r, r)`, its result popped); `f = r × dir` likewise. A pooled
/// matrix (`g_fxMan.FxRwMatrixCreate`) gets right = r, up = dir, at = f, pos = 0 (flags untouched),
/// `RwMatrixUpdate`, `RwV3dTransformVectors(out, in, 1, m)`, `FxRwMatrixDestroy`.
#[hook(addr = 0x4A1660, conv = cdecl, name = "Fx_c::RotateVecIntoVec", expect = "83 EC 18 56 57",
       unverified = "mvpc-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "uses (and returns) a pooled FX matrix")]
pub fn fx_rotate_vec_into_vec(out: GPtr<u8>, inp: GPtr<u8>, dir: GPtr<u8>) {
    let (ux, uy, uz) = (k(0x3ED9_3DD9), k(0x3F10_D1B7), k(0x3F35_0481));
    let d = |o: isize| rd::<f32>(dir, o);
    let mut r = [0f32; 3];
    r[0] = cross1(d(4), uz, d(8), uy);
    r[1] = cross1(d(8), ux, d(0), uz);
    r[2] = cross1(d(0), uy, d(4), ux);
    let rp = GPtr::from_raw(r.as_mut_ptr()).cast::<u8>();
    rw_v3d_normalize(rp, rp);
    let f = [cross1(r[2], d(4), r[1], d(8)), cross1(r[0], d(8), r[2], d(0)), cross1(r[1], d(0), r[0], d(4))];
    let fxm = GPtr::from_addr(FX_MAN);
    let m = GPtr::<u8>::from_addr(fx_rw_matrix_create(fxm) as usize);
    for (j, v) in r.iter().enumerate() {
        wr::<f32>(m, 4 * j as isize, *v);
    }
    for j in 0..3isize {
        wr::<u32>(m, 0x10 + 4 * j, rd::<u32>(dir, 4 * j));
    }
    for (j, v) in f.iter().enumerate() {
        wr::<f32>(m, 0x20 + 4 * j as isize, *v);
    }
    for j in 0..3isize {
        wr::<u32>(m, 0x30 + 4 * j, 0);
    }
    rw_matrix_update(m);
    rw_v3d_transform_vectors(out, inp, 1, m);
    fx_rw_matrix_destroy(fxm, m.addr() as u32);
}

/// `Fx_c::RotateVecAboutVec(RwV3d* out, const RwV3d* in, const RwV3d* axis, float angle)` (0x4A1780,
/// cdecl): `a = angle * 40.74366f` (256 / 2pi); `c = sinTable[_ftol(a + 64) & 0xFF]`, `s =
/// sinTable[_ftol(a) & 0xFF]` (0xBB3E00); `t = 1 - c`; the rotation matrix terms are stored as f32
/// locals (`t*ax*ay`, `az*ax*t`, `t*az*ay`, `s*ax`, `s*ay`, `ay*ay*t + c`, `az*az*t + c`) and `s*az`, `c`
/// in the original's own argument slots (+0xC, +0x10: stack residue, recorded); the diagonal x term
/// stays on the x87 stack. `in` is re-read per output component (out may alias in).
#[hook(addr = 0x4A1780, conv = cdecl, name = "Fx_c::RotateVecAboutVec", expect = "83 EC 1C D9 44 24 2C",
       unverified = "mvpc-fx-1", route = "L2", hazards = "stack-residue,heap-writes", recs = "grove-walk",
       footprint = 0x800,
       note = "writes its own argument slots (s*az at +0xC, c at +0x10) and f32 locals; the residue of _ftol's 16-aligned frame below -0x24 is not recorded")]
pub fn fx_rotate_vec_about_vec(out: GPtr<u8>, inp: GPtr<u8>, axis: GPtr<u8>, angle: u32) {
    let a = xf(f32::from_bits(angle)).fmul_m(k(0x4222_F983));
    let tab = s(0xBB3E00).cast::<f32>();
    let ci = ftol32(xf(k(0x4280_0000)).fadd(a)) & 0xFF;
    let c = tab.wrapping_index(ci).read();
    let si = ftol32(a) & 0xFF;
    let sv = xf(tab.wrapping_index(si).read());
    let (ax, ay, az) = (rd::<f32>(axis, 0), rd::<f32>(axis, 4), rd::<f32>(axis, 8));
    let t = xf(k(0x3F80_0000)).fsub_m(c);
    let l0 = t.fmul_m(ax).fmul_m(ay).to_f32();
    let l4 = xf(az).fmul_m(ax).fmul(t).to_f32();
    let l10 = t.fmul_m(az).fmul_m(ay).to_f32();
    let l0c = sv.fmul_m(ax).to_f32();
    let l8 = sv.fmul_m(ay).to_f32();
    let saz = sv.fmul_m(az).to_f32();
    let l14 = xf(ay).fmul(xf(ay)).fmul(t).fadd_m(c).to_f32();
    let l18 = xf(az).fmul(xf(az)).fmul(t).fadd_m(c).to_f32();
    let x0 = xf(ax).fmul(xf(ax)).fmul(t).fadd_m(c);
    let i = |o: isize| rd::<f32>(inp, o);
    let v = x0.fmul_m(i(0)).fadd(xf(l0).fsub_m(saz).fmul_m(i(4))).fadd(xf(l8).fadd_m(l4).fmul_m(i(8)));
    wr::<f32>(out, 0, v.to_f32());
    let v = xf(saz).fadd_m(l0).fmul_m(i(0)).fadd(xf(l10).fsub_m(l0c).fmul_m(i(8))).fadd(xf(l14).fmul_m(i(4)));
    wr::<f32>(out, 4, v.to_f32());
    let v = xf(l4).fsub_m(l8).fmul_m(i(0)).fadd(xf(l0c).fadd_m(l10).fmul_m(i(4))).fadd(xf(l18).fmul_m(i(8)));
    wr::<f32>(out, 8, v.to_f32());
    let b = |v: f32| Fp::Val(v.to_bits());
    stack_footprint(&[
        (-0x24, Fp::Ebp),
        (-0x20, Fp::Val(0x4A17AF)),
        (0x10, b(c)),
        (-0x1C, b(l0)),
        (-0x18, b(l4)),
        (-0xC, b(l10)),
        (-0x10, b(l0c)),
        (-0x14, b(l8)),
        (0xC, b(saz)),
        (-8, b(l14)),
        (-4, b(l18)),
    ]);
}

/// The two short-key `GetVal(float* out, float t)` (thiscall, `ret 8`), same shape as
/// `FxInterpInfoFloat_c::GetVal` (0x4A8470) with keys `fild(word) * K` (`signed`: `movsx`, else
/// `movzx`). Layout: +4 looped, +5 key count, +6 value count, +8 key times (u16, 1/256 s), +0xC
/// per-value key arrays. The original reuses its own argument slot `t` (+8) and the pushed-ECX slot
/// (-4) as scratch (the wrapped time, the integer keys, the fraction).
pub(crate) fn interp_short_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32, signed: bool, kbits: u32) {
    let out = out.cast::<f32>();
    let key = |p: GPtr<u8>, j: i32| -> X87 {
        let w = p.cast::<u16>().wrapping_index(j).read();
        let v = if signed { i32::from(w as i16) } else { i32::from(w) };
        X87::from_i32(v).fmul_m(k(kbits))
    };
    let vals = |i: i32| rd::<GPtr<GPtr<u8>>>(this, 0xC).wrapping_index(i).read();
    let count = || rd::<i8>(this, 6) as i32;
    let n = rd::<i8>(this, 5) as i32;
    if n == 1 {
        let mut i = 0;
        while i < count() {
            out.wrapping_index(i).write(key(vals(i), 0).to_f32());
            i += 1;
        }
        return;
    }
    let times = rd::<GPtr<u16>>(this, 8);
    let time = |j: i32| X87::from_i32(i32::from(times.wrapping_index(j).read())).fmul_m(k(0x3B80_0000));
    let mut t = f32::from_bits(t);
    if rd::<u8>(this, 4) != 0 {
        let tt = time(n - 1);
        let q = ftol32(xf(t).fdiv(tt));
        t = X87::from_i32(q).fmul(tt).fsubr_m(t).to_f32();
    }
    let mut j = 1;
    let mut found = None;
    while j < n {
        let tj = time(j);
        if !crate::common::jp(xf(t), tj, 5) {
            found = Some((j, tj));
            break;
        }
        j += 1;
    }
    let Some((j, tj)) = found else {
        let mut i = 0;
        while i < count() {
            let nn = rd::<i8>(this, 5) as i32;
            out.wrapping_index(i).write(key(vals(i), nn - 1).to_f32());
            i += 1;
        }
        return;
    };
    let tp = time(j - 1);
    let frac = xf(t).fsub(tp).fdiv(tj.fsub(tp)).to_f32();
    let mut i = 0;
    while i < count() {
        let a = key(vals(i), j - 1);
        let b = key(vals(i), j);
        out.wrapping_index(i).write(b.fsub(a).fmul_m(frac).fadd(a).to_f32());
        i += 1;
    }
}

/// `FxInterpInfo32_c::GetVal(float* out, float t)` (0x4A89C0): [`interp_short_get_val`] with signed
/// keys `* 0.001f` (0x858CDC).
#[hook(addr = 0x4A89C0, conv = thiscall, name = "FxInterpInfo32_c::GetVal", expect = "51 53 56 8B F1",
       unverified = "mvpc-fx-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "writes the caller's key buffer; uses its own argument slot t (+8) and the pushed-ECX slot as scratch (no footprint recorded)")]
pub fn interp_32_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) {
    interp_short_get_val(this, out, t, true, 0x3A83_126F);
}

/// `FxInterpInfoU255_c::GetVal(float* out, float t)` (0x4A8800): [`interp_short_get_val`] with unsigned
/// keys `* (1/256)` (0x859AA0).
#[hook(addr = 0x4A8800, conv = thiscall, name = "FxInterpInfoU255_c::GetVal", expect = "51 53 56 8B F1",
       unverified = "mvpc-fx-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "writes the caller's key buffer; uses its own argument slot t (+8) and the pushed-ECX slot as scratch (no footprint recorded)")]
pub fn interp_u255_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) {
    interp_short_get_val(this, out, t, false, 0x3B80_0000);
}

orig! {
    /// `FxInterpInfo32_c::GetVal(float* out, float t)` (0x4A89C0)
    pub fn interp_32_get_val_call(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A89C0 => interp_32_get_val;
    /// `RwV3dLength(const RwV3d*)` (0x7EDAC0): the length in ST0
    pub fn rw_v3d_length(v: GPtr<u8>) -> f64 = cdecl 0x7EDAC0 => unported;
}

/// The `FxInfo*_c::GetValue` key time: `mult` with byte +6 set, else `(float)(currTime / totalTime)`
/// (the original stores it into its `mult` argument slot).
fn key_time(this: GPtr<u8>, curr: u32, mult: u32, total: u32) -> u32 {
    if rd::<u8>(this, 6) != 0 {
        return mult;
    }
    xf(f32::from_bits(curr)).fdiv_m(f32::from_bits(total)).to_f32().to_bits()
}

/// A random unit vector component: `(f32)((rand() % 10000) * 0.0002f - 1.0f)` (`idiv`, `fild`, `fmul`,
/// `fsub`), the `rand` call attributed to its original return address.
fn rand_comp(ret: u32) -> f32 {
    let r = sa_core::orig::at_site(ret, sa_core::crt::rand);
    X87::from_i32(r.wrapping_rem(10000)).fmul_m(k(0x3951_B717)).fsub_m(k(0x3F80_0000)).to_f32()
}

/// `FxInfoNoise_c::GetValue(currTime, mult, timeStep, totalTime, useConst, out)` (0x4A5300, thiscall,
/// `ret 0x18`): `key = FxInterpInfo32_c::GetVal(key time)[0]`; `v = normalize(rand3)` (three
/// [`rand_comp`], `RwV3dNormalize(v, v)`); `s = key * timeStep`; `v *= s` (f32 each); `len =
/// RwV3dLength(&out.vel)` (f32); `out.vel (+0xC..+0x14) += v`; when `RwV3dLength(&out.vel) > 0`
/// (ordered) `RwV3dNormalize(vel, vel)`; `vel *= len`. The original keeps the key time, the `rand`
/// remainders and `len` in its own argument slots (+8).
#[hook(addr = 0x4A5300, conv = thiscall, name = "FxInfoNoise_c::GetValue", expect = "8A 41 06 83 EC 4C",
       unverified = "mvpc-fx-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "three rand() calls; writes the particle movement info; scratch in its own mult slot (+8), no footprint recorded")]
pub fn fx_info_noise_get_value(this: GPtr<u8>, curr: u32, mult: u32, step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let mut buf = [0f32; 16];
    interp_32_get_val_call(this.byte_add(8), GPtr::from_raw(buf.as_mut_ptr()).cast(), t);
    let mut v = [rand_comp(0x4A5339), rand_comp(0x4A535E), rand_comp(0x4A5383)];
    let vp = GPtr::from_raw(v.as_mut_ptr()).cast::<u8>();
    rw_v3d_normalize(vp, vp);
    let sx = xf(buf[0]).fmul_m(f32::from_bits(step));
    for c in v.iter_mut() {
        *c = xf(*c).fmul(sx).to_f32();
    }
    let vel = out.byte_add(0xC);
    let len = X87::from_f64_exact(rw_v3d_length(vel)).to_f32();
    for (j, c) in v.iter().enumerate() {
        let o = 4 * j as isize;
        wr::<f32>(vel, o, xf(*c).fadd_m(rd::<f32>(vel, o)).to_f32());
    }
    let l2 = X87::from_f64_exact(rw_v3d_length(vel));
    if !l2.fcom(X87::zero()).test_any(0x41) {
        rw_v3d_normalize(vel, vel);
    }
    for j in 0..3isize {
        wr::<f32>(vel, 4 * j, xf(len).fmul_m(rd::<f32>(vel, 4 * j)).to_f32());
    }
}

/// `FxInfoJitter_c::GetValue(currTime, mult, timeStep, totalTime, useConst, out)` (0x4A5BB0, thiscall,
/// `ret 0x18`): `key = FxInterpInfo32_c::GetVal(key time)[0]`; `v = normalize(rand3)`; `s = key *
/// timeStep` (x87); `out.pos (+0, +4, +8) += v * s` (the x term stays on the x87 stack, y and z are
/// stored as f32 first). Scratch in its own `mult` slot (+8) like the Noise info.
#[hook(addr = 0x4A5BB0, conv = thiscall, name = "FxInfoJitter_c::GetValue", expect = "8A 41 06 83 EC 4C",
       unverified = "mvpc-fx-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "three rand() calls; writes the particle movement info; scratch in its own mult slot (+8), no footprint recorded")]
pub fn fx_info_jitter_get_value(this: GPtr<u8>, curr: u32, mult: u32, step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let mut buf = [0f32; 16];
    interp_32_get_val_call(this.byte_add(8), GPtr::from_raw(buf.as_mut_ptr()).cast(), t);
    let mut v = [rand_comp(0x4A5BE7), rand_comp(0x4A5C0C), rand_comp(0x4A5C31)];
    let vp = GPtr::from_raw(v.as_mut_ptr()).cast::<u8>();
    rw_v3d_normalize(vp, vp);
    let sx = xf(buf[0]).fmul_m(f32::from_bits(step));
    let x = xf(v[0]).fmul(sx);
    let y = sx.fmul_m(v[1]).to_f32();
    let z = sx.fmul_m(v[2]).to_f32();
    wr::<f32>(out, 0, x.fadd_m(rd::<f32>(out, 0)).to_f32());
    wr::<f32>(out, 4, xf(y).fadd_m(rd::<f32>(out, 4)).to_f32());
    wr::<f32>(out, 8, xf(z).fadd_m(rd::<f32>(out, 8)).to_f32());
}

orig! {
    /// `FxInfo_c::GetValue(currTime, mult, timeStep, totalTime, useConst, out)`: vtable slot +8
    pub fn info_vget_value(this: GPtr<u8>, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, out: GPtr<u8>) = thiscall vslot 0x8 => unported;
}

/// `FxInfoManager_c::ProcessEmissionInfo(currTime, timeStep, totalTime, useConst, out)` (0x4A4960,
/// thiscall, `ret 0x14`): `out+0 = (f32)(timeStep * [0x85862C])`; +0x30 = +0x3C = 1.0, +0x5C = +0x64 =
/// 2.0, every other dword of +4 .. +0x60 = 0. For each info i < (i8)+8 whose type has 0x1000 (byte +5
/// & 0x10): `GetValue(currTime, 0, timeStep, totalTime, useConst, out)` (vtable +8).
#[hook(addr = 0x4A4960, conv = thiscall, name = "FxInfoManager_c::ProcessEmissionInfo", expect = "D9 44 24 08 53",
       unverified = "mvpc-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "fills the caller's emission info")]
pub fn info_manager_process_emission_info(this: GPtr<u8>, curr: u32, step: u32, total: u32, use_const: u32, out: GPtr<u8>) {
    wr::<f32>(out, 0, xf(f32::from_bits(step)).fmul_m(k(0x4120_0000)).to_f32());
    for o in (4..=0x60isize).step_by(4) {
        let v = match o {
            0x30 | 0x3C => 0x3F80_0000,
            0x5C => 0x4000_0000,
            _ => 0,
        };
        wr::<u32>(out, o, v);
    }
    wr::<u32>(out, 0x64, 0x4000_0000);
    let mut i = 0;
    while i < rd::<i8>(this, 8) as i32 {
        let info = rd::<GPtr<GPtr<u8>>>(this, 4).wrapping_index(i).read();
        if rd::<u8>(info, 5) & 0x10 != 0 {
            info_vget_value(info, curr, 0, step, total, use_const, out);
        }
        i += 1;
    }
}

/// `FxInfoManager_c::ProcessRenderInfo(a1, a2, a3, a4, a5, out)` (0x4A4A80, thiscall, `ret 0x18`):
/// defaults (colour bytes 0..3 = 0xFF, +0xC = +0x10 = 0.1, +8/+0x14/+0x18/+0x2C = 0, +0x1C = +0x24 =
/// 0.5, +0x20 = +0x28 = -0.5, bytes +0x34/+0x36/+0x37/+0x78/+0x79/+0x7A = 0, +0x7B = 0xFF); then
/// for i from the first render info ((i8)+9) while i < (i32)+0: infos with type & 0xC000 get
/// `GetValue(a1, a2, a3, a4, a5, out)` (vtable +8).
#[hook(addr = 0x4A4A80, conv = thiscall, name = "FxInfoManager_c::ProcessRenderInfo", expect = "53 56 8B 74 24 20",
       unverified = "mvpc-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "fills the caller's render info")]
pub fn info_manager_process_render_info(this: GPtr<u8>, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, out: GPtr<u8>) {
    for o in 0..4isize {
        wr::<u8>(out, o, 0xFF);
    }
    wr::<u32>(out, 0xC, 0x3DCC_CCCD);
    wr::<u32>(out, 0x10, 0x3DCC_CCCD);
    wr::<u32>(out, 8, 0);
    wr::<u32>(out, 0x14, 0);
    wr::<u32>(out, 0x18, 0);
    wr::<u32>(out, 0x1C, 0x3F00_0000);
    wr::<u32>(out, 0x20, 0xBF00_0000);
    wr::<u32>(out, 0x24, 0x3F00_0000);
    wr::<u32>(out, 0x28, 0xBF00_0000);
    wr::<u8>(out, 0x36, 0);
    wr::<u8>(out, 0x37, 0);
    wr::<u8>(out, 0x78, 0);
    wr::<u8>(out, 0x34, 0);
    wr::<u32>(out, 0x2C, 0);
    wr::<u8>(out, 0x79, 0);
    wr::<u8>(out, 0x7A, 0);
    wr::<u8>(out, 0x7B, 0xFF);
    let mut i = rd::<i8>(this, 9) as i32;
    while i < rd::<i32>(this, 0) {
        let info = rd::<GPtr<GPtr<u8>>>(this, 4).wrapping_index(i).read();
        if rd::<u16>(info, 4) & 0xC000 != 0 {
            info_vget_value(info, a1, a2, a3, a4, a5, out);
        }
        i += 1;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_THUNK_OPEN_FILE,
    HOOK_THUNK_READ_LINE,
    HOOK_THUNK_CLOSE_FILE,
    HOOK_LOAD_FX_PROJECT,
    HOOK_LOAD_FX_SYSTEM_BP,
    HOOK_SYSTEM_BP_LOAD,
    HOOK_EMITTER_BP_LOAD,
    HOOK_PRIM_BP_LOAD,
    HOOK_EMITTER_BP_LOAD_TEXTURES,
    HOOK_INFO_MANAGER_LOAD,
    HOOK_ADD_FX_INFO,
    HOOK_INTERP_FLOAT_LOAD,
    HOOK_INTERP_U255_LOAD,
    HOOK_INTERP_32_LOAD,
    HOOK_INTERP_255_LOAD,
    HOOK_FX_INIT_STATIC_SYSTEMS,
    HOOK_CREATE_FX_SYSTEM_BP,
    HOOK_SYSTEM_INIT,
    HOOK_FX_CREATE_ENTITY_FX,
    HOOK_FX_ROTATE_VEC_INTO_VEC,
    HOOK_FX_ROTATE_VEC_ABOUT_VEC,
    HOOK_INTERP_32_GET_VAL,
    HOOK_INTERP_U255_GET_VAL,
    HOOK_FX_INFO_NOISE_GET_VALUE,
    HOOK_FX_INFO_JITTER_GET_VALUE,
    HOOK_INFO_MANAGER_PROCESS_EMISSION_INFO,
    HOOK_INFO_MANAGER_PROCESS_RENDER_INFO,
];
