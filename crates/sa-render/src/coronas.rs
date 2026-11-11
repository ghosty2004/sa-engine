//! PORT-RENDER: `CCoronas` / `CRegisteredCorona`: the initialiser and shutdown (first pass). Ported from
//! the 1.0 US bytes; gta-reversed `Coronas.cpp` and `docs/re/specs/RENDER/CCoronas.deviations.md` as
//! reading aids. `aCoronas[64]` (CRegisteredCorona, 0x3C bytes) at 0xC3E058, identifier at +0xC.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{rd, wr};

sa_core::globals! {
    list GLOBALS;
    /// `gpCoronaTexture[10]` (RwTexture*)
    pub static CORONA_TEXTURES @ 0xC3E000: [GPtr<u8>; 10] = [GPtr::NULL; 10];
    /// `CCoronas::NumCoronas`
    pub static NUM_CORONAS @ 0xC3E038: u32 = 0;
    /// `CCoronas::aCoronas[64]` (CRegisteredCorona)
    pub static CORONAS @ 0xC3E058: [u8; 64 * 0x3C] = [0; 64 * 0x3C];
}

const TEXTURES: usize = 0xC3E000;
const CORONAS_BASE: usize = 0xC3E058;
/// The corona texture names and masks (10 x 0x1A chars each) and "particle" (0x85C648).
const TEX_NAMES: usize = 0x8D4950;
const TEX_MASKS: usize = 0x8D4A58;
const PARTICLE_TXD: usize = 0x85C648;

orig! {
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    pub fn push_current_txd() = cdecl 0x7316A0 => sa_stream::txdstore::push_current_txd;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    pub fn pop_current_txd() = cdecl 0x7316B0 => sa_stream::txdstore::pop_current_txd;
    /// `CTxdStore::SetCurrentTxd(int)` (0x7319C0)
    pub fn set_current_txd(slot: i32) = cdecl 0x7319C0 => sa_stream::txdstore::set_current_txd;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => sa_stream::txdstore::find_txd_slot_name;
    /// `RwTextureRead(const char* name, const char* mask)` (0x7F3AC0)
    pub fn rw_texture_read(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    pub fn rw_texture_destroy(tex: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
}

/// `CCoronas::Init()` (0x6FAA70): with the "particle" TXD current, every corona texture not yet loaded
/// whose name is non-empty is read (`RwTextureRead(name, mask)`); then every corona's identifier is 0
/// and `NumCoronas = 0`.
#[hook(addr = 0x6FAA70, conv = cdecl, name = "CCoronas::Init", expect = "53 56 57 E8 28 6C 03 00")]
pub fn init() {
    push_current_txd();
    set_current_txd(find_txd_slot(GPtr::from_addr(PARTICLE_TXD)));
    let tex = GPtr::<GPtr<u8>>::from_addr(TEXTURES);
    let mut i = 0;
    while i < 10 {
        let slot = tex.wrapping_index(i);
        let name = GPtr::<u8>::from_addr(TEX_NAMES + i as usize * 0x1A);
        if slot.cast::<u32>().read() == 0 && rd::<u8>(name, 0) != 0 {
            let t = rw_texture_read(name, GPtr::from_addr(TEX_MASKS + i as usize * 0x1A));
            slot.write(t);
        }
        i += 1;
    }
    pop_current_txd();
    let mut c = 0;
    while c < 64 {
        wr::<u32>(GPtr::from_addr(CORONAS_BASE), c * 0x3C + 0xC, 0);
        c += 1;
    }
    NUM_CORONAS.set(0);
}

/// `CCoronas::Shutdown()` (0x6FAB00): destroys and clears every loaded corona texture.
#[hook(addr = 0x6FAB00, conv = cdecl, name = "CCoronas::Shutdown", expect = "56 BE 00 E0 C3 00")]
pub fn shutdown() {
    let tex = GPtr::<GPtr<u8>>::from_addr(TEXTURES);
    let mut i = 0;
    while i < 10 {
        let t = tex.wrapping_index(i).read();
        if !t.is_null() {
            rw_texture_destroy(t);
            tex.wrapping_index(i).write(GPtr::NULL);
        }
        i += 1;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_INIT, HOOK_SHUTDOWN];
