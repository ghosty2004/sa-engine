//! STD-5: the vehicle dirt textures (`CCarFXRenderer::InitialiseDirtTexture`, 0x5D5BC0) and the
//! texture copy they are made with (`CClothesBuilder::CopyTexture`, 0x5A5730). Ported from the 1.0 US
//! bytes.

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::GPtr;
use sa_core::{hook, native, orig};

orig! {
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    fn txd_find_slot(name: u32) -> i32 = cdecl 0x731850 => unported;
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    fn txd_push_current() = cdecl 0x7316A0 => unported;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    fn txd_pop_current() = cdecl 0x7316B0 => unported;
    /// `CTxdStore::SetCurrentTxd(int32)` (0x7319C0)
    fn txd_set_current(slot: i32) = cdecl 0x7319C0 => unported;
    /// `RwTextureRead(name, mask)` (0x7F3AC0)
    fn texture_read(name: u32, mask: u32) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
    /// `RwTextureSetName(texture, name)` (0x7F38A0)
    fn texture_set_name(t: GPtr<u8>, name: u32) -> GPtr<u8> = cdecl 0x7F38A0 => unported;
    /// `RwTextureCreate(raster)` (0x7F37C0)
    fn texture_create(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F37C0 => unported;
    /// `RwRasterCreate(width, height, depth, flags)` (0x7FB230)
    fn raster_create(w: u32, h: u32, d: u32, flags: u32) -> GPtr<u8> = cdecl 0x7FB230 => unported;
    /// `RwRasterLock(raster, level, mode)` (0x7FB2D0)
    fn raster_lock(r: GPtr<u8>, level: u32, mode: u32) -> GPtr<u8> = cdecl 0x7FB2D0 => unported;
    /// `RwRasterUnlock(raster)` (0x7FAEC0)
    fn raster_unlock(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7FAEC0 => unported;
    /// `CClothesBuilder::CopyTexture(RwTexture*)` (0x5A5730)
    fn copy_texture_call(t: GPtr<u8>) -> GPtr<u8> = cdecl 0x5A5730 => copy_texture;
}

/// "vehicle" (0x86982C), the dirt texture's name "vehiclegrunge256" (0x86BB28).
const VEHICLE_TXD: u32 = 0x86982C;
const DIRT_NAME: u32 = 0x86BB28;
/// `CCarFXRenderer::ms_aDirtTextures` (0xC02BD0, 16).
const DIRT_TEXTURES: usize = 0xC02BD0;

fn rd32(p: GPtr<u8>, o: isize) -> u32 {
    p.byte_add(o).cast::<u32>().read()
}

/// `CCarFXRenderer::InitialiseDirtTexture()` (0x5D5BC0): in the "vehicle" TXD, reads the grunge texture
/// (filter mode 2) and makes 16 copies; copy `i` (named like the source) has every pixel's three
/// colour bytes replaced by `(c * i) / 16 + level / 16` (bytes, wrapping), `level` = 0xFF0 - 0xFF i,
/// walking rows of `width * 4` bytes from the locked pixels (mode 3). 1.
#[hook(addr = 0x5D5BC0, conv = cdecl, name = "CCarFXRenderer::InitialiseDirtTexture", expect = "83 EC 28 53 55 56 57 68 2C 98",
       unverified = "std5-rw-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn initialise_dirt_texture() -> u32 {
    let slot = txd_find_slot(VEHICLE_TXD);
    txd_push_current();
    txd_set_current(slot);
    let tex = texture_read(DIRT_NAME, 0);
    tex.byte_add(0x50).write(2);
    let raster = GPtr::<u8>::from_addr(rd32(tex, 0) as usize);
    let w = rd32(raster, 0xC) as i32;
    let h = rd32(raster, 0x10) as i32;
    let mut level: i32 = 0xFF0;
    let mut i: i32 = 0;
    loop {
        let copy = copy_texture_call(tex);
        GPtr::<u32>::from_addr(DIRT_TEXTURES + 4 * i as usize).write(copy.addr() as u32);
        texture_set_name(copy, DIRT_NAME);
        let r = GPtr::<u8>::from_addr(rd32(GPtr::from_addr(GPtr::<u32>::from_addr(DIRT_TEXTURES + 4 * i as usize).read() as usize), 0) as usize);
        let pix = raster_lock(r, 0, 3);
        let add = (level / 16) as u8;
        let mut row = 0isize;
        for _ in 0..h.max(0) {
            for x in 0..w.max(0) as isize {
                for c in 0..3isize {
                    let p = pix.byte_add(row + 4 * x + c);
                    let v = (i32::from(p.read()) * i) / 16;
                    p.write((v as u8).wrapping_add(add));
                }
            }
            row += 4 * w as isize;
        }
        raster_unlock(r);
        level -= 0xFF;
        i += 1;
        if level <= 0 {
            break;
        }
    }
    txd_pop_current();
    1
}

/// `CClothesBuilder::CopyTexture(RwTexture*)` (0x5A5730; protected entry, its chunk 0x407EDA masks the
/// format): a raster of the source raster's width, height and depth with flags `((format byte +0x23) &
/// 0xFFFFFF6F) << 8 | 4`, the source locked for reading (mode 2) and the copy for writing (mode 1),
/// `stride * height` bytes copied, both unlocked, `RwTextureCreate` of the copy with filter mode 2.
#[native(addr = 0x5A5730, conv = cdecl, name = "CClothesBuilder::CopyTexture")]
pub fn copy_texture(t: GPtr<u8>) -> GPtr<u8> {
    let src = GPtr::<u8>::from_addr(rd32(t, 0) as usize);
    let fmt = u32::from(src.byte_add(0x23).read());
    let flags = ((fmt & 0xFFFF_FF6F) << 8) | 4;
    let dst = raster_create(rd32(src, 0xC), rd32(src, 0x10), rd32(src, 0x14), flags);
    let s = raster_lock(src, 0, 2);
    let d = raster_lock(dst, 0, 1);
    let n = rd32(src, 0x18).wrapping_mul(rd32(src, 0x10));
    for k in 0..n as isize {
        d.byte_add(k).write(s.byte_add(k).read());
    }
    raster_unlock(src);
    raster_unlock(dst);
    let tex = texture_create(dst);
    tex.byte_add(0x50).write(2);
    tex
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_INITIALISE_DIRT_TEXTURE];
/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_COPY_TEXTURE];
