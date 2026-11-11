//! `CCollisionPlugin::SetModelInfo`, the clump collision stream read and the scratch-pad lock no-ops
//! (STD-6, batch `std6-render-1`). Ported from the 1.0 US bytes (0x41B350, 0x41B1D0, 0x72F4A0,
//! 0x72F4B0).

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CMemoryMgr::LockScratchPad` (0x72F4A0)
    fn lock_scratch_pad() = cdecl 0x72F4A0 => lock_scratch_pad_port;
    /// `CMemoryMgr::ReleaseScratchPad` (0x72F4B0)
    fn release_scratch_pad() = cdecl 0x72F4B0 => release_scratch_pad_port;
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    /// `CColModel::operator new(size)` (0x40FC30)
    fn col_model_new(size: u32) -> GPtr<u8> = cdecl 0x40FC30 => unported;
    /// `CColModel::CColModel()` (0x40FB60)
    fn col_model_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FB60 => unported;
    /// `CFileLoader::LoadCollisionModel(data, colModel, name)` (0x537580)
    fn load_collision_model(data: GPtr<u8>, cm: GPtr<u8>, name: u32) = cdecl 0x537580 => unported;
    /// `CFileLoader::LoadCollisionModelVer2(data, size, colModel, name)` (0x537EE0)
    fn load_collision_model_ver2(data: GPtr<u8>, size: u32, cm: GPtr<u8>, name: u32) = cdecl 0x537EE0 => unported;
    /// `CFileLoader::LoadCollisionModelVer3(data, size, colModel, name)` (0x537CE0)
    fn load_collision_model_ver3(data: GPtr<u8>, size: u32, cm: GPtr<u8>, name: u32) = cdecl 0x537CE0 => unported;
    /// `CColModel::MakeMultipleAlloc()` (0x40F740)
    fn make_multiple_alloc(this: GPtr<u8>) = thiscall 0x40F740 => unported;
    /// `CBaseModelInfo::SetColModel(colModel, owns)` (0x4C4BC0)
    fn set_col_model(mi: GPtr<u8>, cm: GPtr<u8>, owns: u32) = thiscall 0x4C4BC0 => unported;
}

/// The collision chunk buffer (`gCollisionFileBuffer`-style static at 0xC8E0C8: fourcc, size, name,
/// model data from +0x20).
const COL_BUFFER: usize = 0xC8E0C8;

/// `CCollisionPlugin::ms_currentModel` (the model info the collision stream read attaches to)
const CURRENT_MODEL: usize = 0x9689E0;

/// `CCollisionPlugin::SetModelInfo(modelInfo)` (0x41B350): stores the model info.
#[hook(addr = 0x41B350, conv = cdecl, name = "CCollisionPlugin::SetModelInfo", expect = "8B 44 24 04 A3 E0 89 96 00 C3",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn collision_plugin_set_model_info(mi: GPtr<u8>) {
    GPtr::<u32>::from_addr(CURRENT_MODEL).write(mi.addr() as u32);
}

/// `CMemoryMgr::LockScratchPad` (0x72F4A0): a lone `ret`.
#[sa_core::native(addr = 0x72F4A0, conv = cdecl, name = "CMemoryMgr::LockScratchPad")]
pub fn lock_scratch_pad_port() {}

/// `CMemoryMgr::ReleaseScratchPad` (0x72F4B0): a lone `ret`.
#[sa_core::native(addr = 0x72F4B0, conv = cdecl, name = "CMemoryMgr::ReleaseScratchPad")]
pub fn release_scratch_pad_port() {}

/// `cb_RSCOLLISION_RpClump_StreamRead(stream, length, clump, offset, size)` (0x41B1D0): with the scratch
/// pad locked, the chunk is read into the static buffer 0xC8E0C8; a new `CColModel` (operator new +
/// constructor; NULL when the allocation fails) is filled by the loader its fourcc selects ("COLL" v1,
/// "COL2", "COL3" from +0x20 with size `[+4] - 0x18`; any other fourcc runs the v1 loader on the buffer
/// itself), made a single allocation (`MakeMultipleAlloc`), given to the current model info
/// (`SetColModel(cm, true)`) whose +0x13 gets bit 3. Returns the stream. (The original's SEH frame only
/// guards the constructor.)
#[hook(addr = 0x41B1D0, conv = cdecl, name = "cb_RSCOLLISION_RpClump_StreamRead", expect = "64 A1 00 00 00 00 6A FF 68 3B 72 83 00",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn collision_clump_stream_read(s: GPtr<u8>, len: u32, _clump: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    lock_scratch_pad();
    let buf = GPtr::<u8>::from_addr(COL_BUFFER);
    stream_read(s, buf, len);
    let mem = col_model_new(0x30);
    let cm = if mem.is_null() { GPtr::NULL } else { col_model_ctor(mem) };
    let fourcc = buf.cast::<u32>().read();
    let ver = match fourcc {
        0x324C_4F43 => 2,
        0x334C_4F43 => 3,
        x => (x == 0x4C4C_4F43) as u32,
    };
    let data = buf.byte_add(0x20);
    let size = buf.byte_add(4).cast::<u32>().read().wrapping_sub(0x18);
    match ver {
        1 => load_collision_model(data, cm, 0),
        2 => load_collision_model_ver2(data, size, cm, 0),
        3 => load_collision_model_ver3(data, size, cm, 0),
        _ => load_collision_model(buf, cm, 0),
    }
    make_multiple_alloc(cm);
    let mi = GPtr::<u8>::from_addr(GPtr::<u32>::from_addr(CURRENT_MODEL).read() as usize);
    set_col_model(mi, cm, 1);
    let mi = GPtr::<u8>::from_addr(GPtr::<u32>::from_addr(CURRENT_MODEL).read() as usize);
    let f = mi.byte_add(0x13);
    f.write(f.read() | 8);
    release_scratch_pad();
    s
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_COLLISION_PLUGIN_SET_MODEL_INFO, HOOK_COLLISION_CLUMP_STREAM_READ];

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_LOCK_SCRATCH_PAD_PORT, NATIVE_RELEASE_SCRATCH_PAD_PORT];
