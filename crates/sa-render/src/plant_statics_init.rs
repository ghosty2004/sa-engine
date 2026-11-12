//! Two C++ static initialisers of the plant / grass renderer globals (batch `boot-render-1`):
//! `__init_g_c03970` (0x8508C0, `CGrassRenderer::PlantModelsTab`) and `__init_CPlantMgr__m_AmbientColor`
//! (0x850910). Ported from the 1.0 US bytes (`docs/re/specs/RENDER/_static-init-R1.md`). Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CRGBA::CRGBA(uint8 r, uint8 g, uint8 b, uint8 a)` (0x7170C0, thiscall, the bytes pushed as dwords); returns `this`
    fn rgba_ctor(this: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// CRT `_atexit` (0x821D1E)
    fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
}

/// `CGrassRenderer::PlantModelsTab` (`RpAtomic*[4]`, 0xC03970; catalogue `g_c03970`).
const PLANT_MODELS_TAB: usize = 0xC0_3970;
/// `CPlantMgr::m_AmbientColor` (`CRGBA`, 0xC03A44).
const AMBIENT_COLOR: usize = 0xC0_3A44;
/// The empty `atexit` destructors `__dtor_g_c03970` (0x856700) and `__dtor_CPlantMgr__m_AmbientColor` (0x856710).
const DTOR_PLANT_MODELS_TAB: u32 = 0x85_6700;
const DTOR_AMBIENT_COLOR: u32 = 0x85_6710;

/// `__init_g_c03970` (0x8508C0, cdecl): the four `PlantModelsTab` dwords zeroed in address order (the fourth
/// store after the `atexit` argument push, no observable difference), then `atexit(0x856700)`; EAX = the
/// `_atexit` result.
#[hook(addr = 0x8508C0, conv = cdecl, name = "__init_g_c03970", expect = "33 C0 A3 70 39 C0 00",
       route = "split", hazards = "none", recs = "boot",
       note = "C++ static initialiser: runs once from _initterm before WinMain; registers an atexit destructor")]
pub fn init_plant_models_tab() -> i32 {
    for i in 0..4 {
        GPtr::<u32>::from_addr(PLANT_MODELS_TAB + 4 * i).write(0);
    }
    atexit(DTOR_PLANT_MODELS_TAB)
}

/// `__init_CPlantMgr__m_AmbientColor` (0x850910, cdecl): `m_AmbientColor.CRGBA(255, 255, 255, 255)` (0x7170C0),
/// then `atexit(0x856710)`; EAX = the `_atexit` result.
#[hook(addr = 0x850910, conv = cdecl, name = "__init_CPlantMgr__m_AmbientColor", expect = "68 FF 00 00 00",
       route = "split", hazards = "none", recs = "boot",
       note = "C++ static initialiser: runs once from _initterm before WinMain; registers an atexit destructor")]
pub fn init_plant_ambient_color() -> i32 {
    rgba_ctor(GPtr::from_addr(AMBIENT_COLOR), 0xFF, 0xFF, 0xFF, 0xFF);
    atexit(DTOR_AMBIENT_COLOR)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_INIT_PLANT_MODELS_TAB, HOOK_INIT_PLANT_AMBIENT_COLOR];
