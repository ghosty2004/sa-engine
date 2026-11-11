//! CHeli and CPlane (PORT-VEH). Ported from the 1.0 US bytes; gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `CHeli::pHelis[2]`
    pub static HELIS @ 0xC1C964: [GPtr<u8>; 2] = [GPtr::NULL; 2];
    /// `CHeli::NumberOfSearchLights`
    pub static NUMBER_OF_SEARCH_LIGHTS @ 0xC1C96C: u32 = 0;
    /// `CHeli::HeliSearchLights[4]` (`tHeliLight`, 0x4C bytes each)
    pub static HELI_SEARCH_LIGHTS @ 0xC1C990: [[u32; 0x13]; 4] = [[0; 0x13]; 4];
    /// `CHeli::bPoliceHelisAllowed`
    pub static POLICE_HELIS_ALLOWED @ 0x8D338C: u8 = 1;
    /// `CPlane::GenPlane_Status` 0xC1CADC, `CPlane::GenPlane_LastTimeGenerated` 0xC1CAE0
    pub static PLANE_GEN @ 0xC1CADC: [u32; 2] = [0; 2];
    /// `CPlane::GenPlane_Active`
    pub static PLANE_GEN_ACTIVE @ 0x8D33BC: u8 = 1;
}

orig! {
    /// `RwRenderStateSet` (RwEngineInstance table slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CDamageManager::ResetDamageStatus()` (0x6C20E0)
    pub fn reset_damage_status(dm: GPtr<u8>) = thiscall 0x6C20E0 => crate::damage::reset_damage_status;
}

/// `CHeli::InitHelis()` (0x6C4560): pHelis cleared; origin and target of search lights 0 and 1 only
/// (the loop bound 0xC1CA30 stops after two of the four entries; gta-reversed clears all four);
/// NumberOfSearchLights 0; bPoliceHelisAllowed true.
#[hook(addr = 0x6C4560, conv = cdecl, name = "CHeli::InitHelis", expect = "33 C0 A3 64 C9 C1 00")]
pub fn init_helis() {
    HELIS.set([GPtr::NULL; 2]);
    let base = HELI_SEARCH_LIGHTS.ptr().cast::<u8>();
    for k in 0..2isize {
        for w in 0..8isize {
            wr::<u32>(base, k * 0x4C + 4 * w, 0);
        }
    }
    NUMBER_OF_SEARCH_LIGHTS.set(0);
    POLICE_HELIS_ALLOWED.set(1);
}

/// `CHeli::AddHeliSearchLight(const CVector& origin, const CVector& target, float targetRadius,
/// float power, uint32 coronaIndex, uint8 unknown, uint8 drawShadow)` (0x6C45B0): fills entry
/// NumberOfSearchLights (no bound check) and increments it. x and the dwords are copied as integers,
/// y/z go through the FPU (fld/fstp of f32: bit-exact except signalling NaNs, which fld quiets).
#[hook(addr = 0x6C45B0, conv = cdecl, name = "CHeli::AddHeliSearchLight", expect = "8B 15 6C C9 C1 00")]
pub fn add_heli_search_light(
    origin: GPtr<u8>,
    target: GPtr<u8>,
    radius: u32,
    power: u32,
    corona: u32,
    unk: u32,
    shadow: u32,
) {
    let n = NUMBER_OF_SEARCH_LIGHTS.get();
    let e = HELI_SEARCH_LIGHTS
        .ptr()
        .cast::<u8>()
        .byte_add((n.wrapping_mul(0x4C)) as i32 as isize);
    let oz = x(rd::<f32>(origin, 8));
    let oy = x(rd::<f32>(origin, 4));
    wr::<u32>(e, 0, rd::<u32>(origin, 0));
    wr::<f32>(e, 4, oy.to_f32());
    wr::<f32>(e, 8, oz.to_f32());
    let tz = x(rd::<f32>(target, 8));
    let ty = x(rd::<f32>(target, 4));
    wr::<u32>(e, 0xC, rd::<u32>(target, 0));
    wr::<f32>(e, 0x10, ty.to_f32());
    wr::<f32>(e, 0x14, tz.to_f32());
    wr::<u32>(e, 0x18, radius);
    wr::<u32>(e, 0x1C, power);
    wr::<u32>(e, 0x20, corona);
    wr::<u8>(e, 0x24, unk as u8);
    wr::<u8>(e, 0x25, shadow as u8);
    NUMBER_OF_SEARCH_LIGHTS.set(n.wrapping_add(1));
}

/// `CHeli::Pre_SearchLightCone()` (0x6C4650): ten `RwRenderStateSet` calls.
#[hook(addr = 0x6C4650, conv = cdecl, name = "CHeli::Pre_SearchLightCone", expect = "A1 24 7B C9 00")]
pub fn pre_search_light_cone() {
    for (s, v) in [
        (8, 0),
        (6, 1),
        (0xA, 2),
        (0xB, 2),
        (0xC, 1),
        (1, 0),
        (0xE, 0),
        (7, 2),
        (0x1D, 7),
        (0x1E, 0),
    ] {
        rw_render_state_set(s, v);
    }
}

/// `CHeli::Post_SearchLightCone()` (0x6C46E0): eight `RwRenderStateSet` calls.
#[hook(addr = 0x6C46E0, conv = cdecl, name = "CHeli::Post_SearchLightCone", expect = "A1 24 7B C9 00")]
pub fn post_search_light_cone() {
    for (s, v) in [
        (8, 1),
        (6, 1),
        (0xA, 5),
        (0xB, 6),
        (0xC, 0),
        (0x14, 2),
        (0x1D, 5),
        (0x1E, 2),
    ] {
        rw_render_state_set(s, v);
    }
}

/// `CHeli::SwitchPoliceHelis(bool)` (0x6C4800).
#[hook(addr = 0x6C4800, conv = cdecl, name = "CHeli::SwitchPoliceHelis", expect = "8A 44 24 04 A2 8C 33 8D 00")]
pub fn switch_police_helis(enable: u32) {
    POLICE_HELIS_ALLOWED.set(enable as u8);
}

/// `CHeli::SetUpWheelColModel(CColModel*)` (0x6C4320): false (`xor al, al`; EAX's upper bytes are the
/// caller's).
#[hook(addr = 0x6C4320, conv = thiscall, name = "CHeli::SetUpWheelColModel", expect = "32 C0 C2 04 00", this = "CHeli")]
pub fn heli_set_up_wheel_col_model(_h: GPtr<u8>, _col: GPtr<u8>) -> u8 {
    0
}

/// `CHeli::BurstTyre(uint8, bool)` (0x6C4330): false (`xor al, al`).
#[hook(addr = 0x6C4330, conv = thiscall, name = "CHeli::BurstTyre", expect = "32 C0 C2 08 00", this = "CHeli")]
pub fn heli_burst_tyre(_h: GPtr<u8>, _tyre: u32, _fx: u32) -> u8 {
    0
}

/// `CHeli::SetupDamageAfterLoad()` (0x6C4550): clears bit 0 of `+0x42A`.
#[hook(addr = 0x6C4550, conv = thiscall, name = "CHeli::SetupDamageAfterLoad", expect = "80 A1 2A 04 00 00 FE", this = "CHeli")]
pub fn heli_setup_damage_after_load(h: GPtr<u8>) {
    wr::<u8>(h, 0x42A, rd::<u8>(h, 0x42A) & 0xFE);
}

/// `CHeli::Fix()` (0x6C4530): `CDamageManager::ResetDamageStatus` of `+0x5A0`, then clears bit 0 of
/// `+0x42A`.
#[hook(addr = 0x6C4530, conv = thiscall, name = "CHeli::Fix", expect = "56 8B F1 8D 8E A0 05 00 00", this = "CHeli")]
pub fn heli_fix(h: GPtr<u8>) {
    reset_damage_status(h.byte_add(0x5A0));
    wr::<u8>(h, 0x42A, rd::<u8>(h, 0x42A) & 0xFE);
}

/// `CPlane::SetUpWheelColModel(CColModel*)` (0x6C9140): false (`xor al, al`).
#[hook(addr = 0x6C9140, conv = thiscall, name = "CPlane::SetUpWheelColModel", expect = "32 C0 C2 04 00", this = "CPlane")]
pub fn plane_set_up_wheel_col_model(_p: GPtr<u8>, _col: GPtr<u8>) -> u8 {
    0
}

/// `CPlane::BurstTyre(uint8, bool)` (0x6C9150): false (`xor al, al`).
#[hook(addr = 0x6C9150, conv = thiscall, name = "CPlane::BurstTyre", expect = "32 C0 C2 08 00", this = "CPlane")]
pub fn plane_burst_tyre(_p: GPtr<u8>, _tyre: u32, _fx: u32) -> u8 {
    0
}

/// `CPlane::SetupDamageAfterLoad()` (0x6CAC10): clears bit 0 of `+0x42A`.
#[hook(addr = 0x6CAC10, conv = thiscall, name = "CPlane::SetupDamageAfterLoad", expect = "80 A1 2A 04 00 00 FE", this = "CPlane")]
pub fn plane_setup_damage_after_load(p: GPtr<u8>) {
    wr::<u8>(p, 0x42A, rd::<u8>(p, 0x42A) & 0xFE);
}

/// `CPlane::IsAlreadyFlying()` (0x6CAB90): `+0x9C0 = CTimer::m_snTimeInMilliseconds - 20000`.
#[hook(addr = 0x6CAB90, conv = thiscall, name = "CPlane::IsAlreadyFlying", expect = "A1 84 CB B7 00", this = "CPlane")]
pub fn is_already_flying(p: GPtr<u8>) {
    wr::<u32>(p, 0x9C0, g::<u32>(0xB7CB84).read().wrapping_sub(20000));
}

/// `CPlane::InitPlaneGenerationAndRemoval()` (0x6CAD90).
#[hook(addr = 0x6CAD90, conv = cdecl, name = "CPlane::InitPlaneGenerationAndRemoval", expect = "33 C0 A3 DC CA C1 00")]
pub fn init_plane_generation_and_removal() {
    PLANE_GEN.set([0; 2]);
    PLANE_GEN_ACTIVE.set(1);
}

/// `fHandling->m_fMass / 1000.0 * 0.5` (the `+0x384` handling `+0x10`).
fn gear_base(p: GPtr<u8>) -> sa_math::x87::X87 {
    let h = rd::<GPtr<u8>>(p, 0x384);
    x(rd::<f32>(h, 0x10)).fdiv_m(1000.0).fmul_m(0.5)
}

/// `CPlane::SetGearUp()` (0x6CAC20): landing gear ratio `+0x9CC = 1.0`; `+0x98 = mass / 1000 * 0.5 *
/// flying handling (+0x388) +0x30`; the four wheel states `+0x5A5..` = 2.
#[hook(addr = 0x6CAC20, conv = thiscall, name = "CPlane::SetGearUp", expect = "8B 81 84 03 00 00", this = "CPlane", preserve = "ecx")]
pub fn set_gear_up(p: GPtr<u8>) {
    wr::<u32>(p, 0x9CC, 0x3F80_0000);
    let fh = rd::<GPtr<u8>>(p, 0x388);
    let v = gear_base(p).fmul_m(rd::<f32>(fh, 0x30));
    wr::<u32>(p, 0x5A5, 0x0202_0202);
    wr::<f32>(p, 0x98, v.to_f32());
}

/// `CPlane::SetGearDown()` (0x6CAC70): `+0x9CC = 0`; `+0x98 = mass / 1000 * 0.5`; wheel states 0.
#[hook(addr = 0x6CAC70, conv = thiscall, name = "CPlane::SetGearDown", expect = "8B 91 84 03 00 00", this = "CPlane")]
pub fn set_gear_down(p: GPtr<u8>) {
    wr::<u32>(p, 0x9CC, 0);
    let v = gear_base(p);
    wr::<u32>(p, 0x5A5, 0);
    wr::<f32>(p, 0x98, v.to_f32());
}

/// `CPlane::CountPlanesAndHelis()` (0x6CCA50): vehicles of the pool (from the top) whose slot is used
/// and whose `+0x594` (vehicle sub-type) is 3 (heli) or 4 (plane).
#[hook(addr = 0x6CCA50, conv = cdecl, name = "CPlane::CountPlanesAndHelis", expect = "57 8B 3D 94 44 B7 00")]
pub fn count_planes_and_helis() -> i32 {
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let size = rd::<i32>(pool, 8);
    let flags = rd::<GPtr<u8>>(pool, 4);
    let mut n = 0i32;
    let mut i = size;
    while i != 0 {
        i = i.wrapping_sub(1);
        if (flags.byte_add(i as isize).read() as i8) >= 0 {
            let objs = rd::<u32>(pool, 0);
            let v = objs.wrapping_add((i as u32).wrapping_mul(0xA18));
            if v != 0 {
                let t = rd::<i32>(GPtr::from_addr(v as usize), 0x594);
                if t == 3 || t == 4 {
                    n += 1;
                }
            }
        }
    }
    n
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_HELIS,
    HOOK_ADD_HELI_SEARCH_LIGHT,
    HOOK_PRE_SEARCH_LIGHT_CONE,
    HOOK_POST_SEARCH_LIGHT_CONE,
    HOOK_SWITCH_POLICE_HELIS,
    HOOK_HELI_SET_UP_WHEEL_COL_MODEL,
    HOOK_HELI_BURST_TYRE,
    HOOK_HELI_SETUP_DAMAGE_AFTER_LOAD,
    HOOK_HELI_FIX,
    HOOK_PLANE_SET_UP_WHEEL_COL_MODEL,
    HOOK_PLANE_BURST_TYRE,
    HOOK_PLANE_SETUP_DAMAGE_AFTER_LOAD,
    HOOK_IS_ALREADY_FLYING,
    HOOK_INIT_PLANE_GENERATION_AND_REMOVAL,
    HOOK_SET_GEAR_UP,
    HOOK_SET_GEAR_DOWN,
    HOOK_COUNT_PLANES_AND_HELIS,
];
