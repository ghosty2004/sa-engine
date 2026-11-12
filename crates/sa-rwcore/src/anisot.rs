//! RpAnisot plugin functions (STD-4). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

/// `RpAnisotTextureGetMaxAnisotropy(RwTexture*)` (0x748F50, cdecl, result in AL): the texture's
/// extension byte at the plugin offset `[0xC92478]`, 0 for a NULL texture (`xor al, al`; the upper
/// EAX bits are the texture pointer's in both cases).
#[hook(addr = 0x748F50, conv = cdecl, name = "RpAnisotTextureGetMaxAnisotropy", expect = "8B 44 24 04 85 C0 74 0A",
       route = "L2", hazards = "ax-al-return", recs = "boot")]
pub fn texture_get_max_anisotropy(tex: GPtr<u8>) -> u8 {
    if tex.is_null() {
        return 0;
    }
    let off = GPtr::<u32>::from_addr(0xC92478).read();
    tex.byte_add(off as i32 as isize).read()
}

sa_core::orig! {
    /// `RwD3D9GetCaps()` (0x7FAD20)
    fn an_get_caps() -> GPtr<u8> = cdecl 0x7FAD20 => unported;
}

/// `RpAnisotGetMaxSupportedMaxAnisotropy()` (0x748F20, cdecl, AL): the byte at `RwD3D9GetCaps() + 0x6C`
/// (the low byte of `MaxAnisotropy`; the upper EAX bits are the caps pointer's).
#[hook(addr = 0x748F20, conv = cdecl, name = "RpAnisotGetMaxSupportedMaxAnisotropy", expect = "E8 FB 1D 0B 00",
       route = "L2", hazards = "ax-al-return", recs = "boot")]
pub fn get_max_supported_max_anisotropy() -> u8 {
    an_get_caps().byte_add(0x6C).read()
}

/// `RpAnisotTextureSetMaxAnisotropy(RwTexture*, int8)` (0x748F30, cdecl): the extension byte := value,
/// returns the texture (NULL → NULL).
#[hook(addr = 0x748F30, conv = cdecl, name = "RpAnisotTextureSetMaxAnisotropy", expect = "8B 44 24 04 85 C0 74 0E",
       route = "L2", hazards = "none", recs = "boot")]
pub fn texture_set_max_anisotropy(tex: GPtr<u8>, value: u32) -> GPtr<u8> {
    if tex.is_null() {
        return GPtr::NULL;
    }
    let off = GPtr::<u32>::from_addr(0xC92478).read();
    tex.byte_add(off as i32 as isize).write(value as u8);
    tex
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_TEXTURE_GET_MAX_ANISOTROPY, HOOK_GET_MAX_SUPPORTED_MAX_ANISOTROPY, HOOK_TEXTURE_SET_MAX_ANISOTROPY];
