//! STD-5: `CPlayerSkin::GetSkinTexture(const char*)` (0x6FFA10), the player's skin texture: from the
//! player TXD, else a BMP from the skins folder. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    fn txd_push_current() = cdecl 0x7316A0 => unported;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    fn txd_pop_current() = cdecl 0x7316B0 => unported;
    /// `CTxdStore::SetCurrentTxd(int32)` (0x7319C0)
    fn txd_set_current(slot: u32) = cdecl 0x7319C0 => unported;
    /// `RwTextureRead(name, mask)` (0x7F3AC0)
    fn texture_read(name: GPtr<u8>, mask: u32) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
    /// CRT `sprintf(buf, fmt, s)` (0x821BB5)
    fn sprintf1(buf: u32, fmt: u32, s: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    /// `RtBMPImageRead(path)` (0x7CDF60)
    fn bmp_image_read(path: u32) -> GPtr<u8> = cdecl 0x7CDF60 => unported;
    /// `RwImageFindRasterFormat(image, type, &w, &h, &depth, &format)` (0x8042C0)
    fn image_find_raster_format(img: GPtr<u8>, ty: u32, w: GPtr<u8>, h: GPtr<u8>, d: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> = cdecl 0x8042C0 => unported;
    /// `RwRasterCreate(w, h, depth, flags)` (0x7FB230)
    fn raster_create(w: u32, h: u32, d: u32, f: u32) -> GPtr<u8> = cdecl 0x7FB230 => unported;
    /// `RwRasterSetFromImage(raster, image)` (0x804290)
    fn raster_set_from_image(r: GPtr<u8>, img: GPtr<u8>) -> GPtr<u8> = cdecl 0x804290 => unported;
    /// `RwTextureCreate(raster)` (0x7F37C0)
    fn texture_create(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F37C0 => unported;
    /// `RwTextureSetName(texture, name)` (0x7F38A0)
    fn texture_set_name(t: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F38A0 => unported;
    /// `RwTexDictionaryAddTexture(dict, texture)` (0x7F3980)
    fn tex_dictionary_add_texture(d: u32, t: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3980 => unported;
    /// `RwImageDestroy(image)` (0x802740)
    fn image_destroy(img: GPtr<u8>) -> u32 = cdecl 0x802740 => unported;
}

/// The player TXD slot (0xC3F03C), the TXD pool (0xC8800C), the path buffer (0xB71670).
const PLAYER_TXD: usize = 0xC3F03C;
const TXD_POOL: usize = 0xC8800C;
const PATH: u32 = 0xB71670;

/// `CPlayerSkin::GetSkinTexture(const char* name)` (0x6FFA10): `RwTextureRead(name)` in the player TXD;
/// without one, the path is formatted (format 0x8725C4 when the name's first five bytes equal 0x8725F0's
/// or the name is empty, else 0x8725E0) and read as a BMP: NULL on failure, else a texture (raster of
/// the image's format, filter mode 2, named `name`) added to the player TXD's dictionary.
#[hook(addr = 0x6FFA10, conv = cdecl, name = "CPlayerSkin::GetSkinTexture", expect = "83 EC 10 53 55 56 57 E8 84 1C",
       unverified = "std5-ped-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn get_skin_texture(name: GPtr<u8>) -> GPtr<u8> {
    txd_push_current();
    txd_set_current(GPtr::<u32>::from_addr(PLAYER_TXD).read());
    let tex = texture_read(name, 0);
    txd_pop_current();
    if !tex.is_null() {
        return tex;
    }
    let same = (0..5isize).all(|i| name.byte_add(i).read() == GPtr::<u8>::from_addr(0x8725F0).byte_add(i).read());
    let fmt = if same || name.read() == 0 { 0x8725C4 } else { 0x8725E0 };
    sprintf1(PATH, fmt, name);
    let img = bmp_image_read(PATH);
    if img.is_null() {
        return tex;
    }
    let mut l = [0u32; 4];
    let base = l.as_mut_ptr() as usize;
    let p = |i: usize| -> GPtr<u8> { GPtr::from_addr(base + 4 * i) };
    image_find_raster_format(img, 4, p(3), p(2), p(1), p(0));
    let r = raster_create(l[3], l[2], l[1], l[0]);
    raster_set_from_image(r, img);
    let t = texture_create(r);
    texture_set_name(t, name);
    t.byte_add(0x50).write(2);
    let pool = GPtr::<u8>::from_addr(GPtr::<u32>::from_addr(TXD_POOL).read() as usize);
    let idx = GPtr::<u32>::from_addr(PLAYER_TXD).read() as i32;
    let map = GPtr::<i8>::from_addr(pool.byte_add(4).cast::<u32>().read() as usize);
    let slot = if map.byte_add(idx as isize).read() < 0 {
        GPtr::<u8>::NULL
    } else {
        GPtr::from_addr((pool.cast::<u32>().read() as usize).wrapping_add(12 * idx as usize))
    };
    tex_dictionary_add_texture(slot.cast::<u32>().read(), t);
    image_destroy(img);
    t
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_GET_SKIN_TEXTURE];
