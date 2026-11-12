//! `CShadows::RenderIndicatorShadow`, `CShadows::GunShotSetsOilOnFire` and `CBirds::HandleGunShot` (batch
//! `samp-render-3`), ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::too_many_arguments)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{k, rd, wr};

orig! {
    /// `C3dMarkers::PlaceMarkerSet(uint32 id, uint16 type, CVector& pos, float size, uint8 r, g, b, a, uint16
    /// pulsePeriod, float pulseFraction, int16 rotateRate)` (0x725BA0, cdecl; every argument pushed as a dword)
    fn place_marker_set(id: u32, ty: u32, pos: GPtr<u8>, size: u32, r: u32, g: u32, b: u32, a: u32, period: u32,
        fraction: u32, rotate: u32) = cdecl 0x725BA0 => unported;
    /// `CCollision::DistToLine(const CVector* start, const CVector* end, const CVector* point)` (0x417610, ST0)
    fn dist_to_line(a: GPtr<u8>, b: GPtr<u8>, p: GPtr<u8>) -> f64 = cdecl 0x417610 => unported;
    /// `CFireManager::StartFire(CVector pos, float size, uint8, CEntity* creator, uint32 time, int8 generations,
    /// uint8)` (0x539F00, thiscall, the vector by value)
    fn fire_manager_start_fire(this: GPtr<u8>, px: u32, py: u32, pz: u32, size: u32, a: u32, creator: u32, time: u32,
        gens: u32, b: u32) -> GPtr<u8> = thiscall 0x539F00 => unported;
    /// `CColLine::CColLine(const CVector& start, const CVector& end)` (0x40EF50, thiscall, `ret 8`)
    fn col_line_ctor(this: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) -> GPtr<u8> = thiscall 0x40EF50 => unported;
    /// `CCollision::TestLineSphere(const CColLine&, const CColSphere&)` (0x417470, cdecl, AL)
    fn test_line_sphere(line: GPtr<u8>, sphere: GPtr<u8>) -> u8 = cdecl 0x417470 => unported;
}

fn gp<T: sa_core::mem::GameData>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

/// `CShadows::RenderIndicatorShadow(uint32 id, uint8 type, RwTexture*, CVector* pos, float frontX, float frontY,
/// float sideX, float sideY, int16 intensity)` (0x70CCB0, cdecl): `m = frontX` when `frontX > -sideY` (ordered), else
/// `-sideY` (`fchs`, stored as f32 into the `pos` argument slot); three `C3dMarkers::PlaceMarkerSet(id, 1, &posCopy,
/// size, 0xFF, 0, 0, 0xFF, 0x800, 0.2f, 0)` with size `f32(m * 0.8f)`, `f32(m * 0.9f)` and `m`. The type, texture,
/// frontY, sideX and intensity are unused.
#[hook(addr = 0x70CCB0, conv = cdecl, name = "CShadows::RenderIndicatorShadow", expect = "83 EC 0C D9 44 24 2C",
       unverified = "samp-render-3", route = "L2", hazards = "none", recs = "world-cheats")]
pub fn shadows_render_indicator_shadow(id: u32, _ty: u32, _tex: u32, pos: GPtr<u32>, front_x: f32, _front_y: u32,
    _side_x: u32, side_y: f32, _intensity: u32) {
    let mut c = [pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read()];
    let neg = X87::from_f32(side_y).fchs().to_f32();
    let m = if !X87::from_f32(front_x).fcom(X87::from_f32(neg)).test_any(0x41) { front_x } else { neg };
    let cp = GPtr::from_raw(c.as_mut_ptr()).cast::<u8>();
    for s in [
        X87::from_f32(m).fmul_m(k(0x3F4C_CCCD)).to_f32(),
        X87::from_f32(m).fmul_m(k(0x3F66_6666)).to_f32(),
        m,
    ] {
        place_marker_set(id, 1, cp, s.to_bits(), 0xFF, 0, 0, 0xFF, 0x800, 0x3E4C_CCCD, 0);
    }
}

/// `CShadows::aPermanentShadows[48]` (0x38 each) at 0xC4AC30: position +0, type byte +0x32.
const PERMANENT_SHADOWS: usize = 0xC4_AC30;
/// `gFireManager` (0xB71F80)
const FIRE_MANAGER: usize = 0xB7_1F80;
/// The z scale 0x87271C (0.27).
const Z_SCALE: u32 = 0x3E8A_3D71;

/// `CShadows::GunShotSetsOilOnFire(const CVector& start, const CVector& end)` (0x707550, cdecl): with z scaled by
/// 0.27f (f32) on the shot line and on each point, the permanent shadow of type 4 or 8 nearest to the line
/// (`CCollision::DistToLine`, strictly below the best so far, starting at 1.0f; ties keep the earlier one) becomes
/// type 5 and `gFireManager.StartFire(its position, 1.8f, 0, nullptr, 2000, 0, 1)` starts a fire there.
#[hook(addr = 0x707550, conv = cdecl, name = "CShadows::GunShotSetsOilOnFire", expect = "83 EC 28 8B 44 24 2C",
       unverified = "samp-render-3", route = "L2", hazards = "none", recs = "weapon-combat")]
pub fn shadows_gun_shot_sets_oil_on_fire(start: GPtr<f32>, end: GPtr<f32>) {
    let scale = |v: f32| X87::from_f32(v).fmul_m(f32::from_bits(Z_SCALE)).to_f32();
    let mut s = [start.read(), start.wrapping_index(1).read(), scale(start.wrapping_index(2).read())];
    let mut e = [end.read(), end.wrapping_index(1).read(), scale(end.wrapping_index(2).read())];
    let mut best = 1.0f32;
    let mut found = -1i32;
    for i in 0..48 {
        let sh = gp::<u8>(PERMANENT_SHADOWS + 0x38 * i);
        let ty = rd::<u8>(sh, 0x32);
        if ty != 4 && ty != 8 {
            continue;
        }
        let mut p = [rd::<f32>(sh, 0), rd::<f32>(sh, 4), scale(rd::<f32>(sh, 8))];
        let d = X87::from_f64_exact(dist_to_line(
            GPtr::from_raw(s.as_mut_ptr()).cast(),
            GPtr::from_raw(e.as_mut_ptr()).cast(),
            GPtr::from_raw(p.as_mut_ptr()).cast(),
        ));
        if !d.fcom(X87::from_f32(best)).test_parity(5) {
            best = d.to_f32();
            found = i as i32;
        }
    }
    if found < 0 {
        return;
    }
    let sh = gp::<u8>(PERMANENT_SHADOWS + 0x38 * found as usize);
    wr::<u8>(sh, 0x32, 5);
    fire_manager_start_fire(gp(FIRE_MANAGER), rd::<u32>(sh, 0), rd::<u32>(sh, 4), rd::<u32>(sh, 8), 0x3FE6_6666, 0, 0,
        0x7D0, 0, 1);
}

/// `CBirds::aBirds[6]` (0x44 each) at 0xC6A8B0: position +0, active byte +0x3F; `CBirds::NumberOfBirds` 0xC6A8A4,
/// `bHasBirdBeenShot` 0xC6A8A0, `vecBirdShotAt` 0xC6AA48.
const BIRDS: usize = 0xC6_A8B0;

/// `CBirds::HandleGunShot(const CVector* start, const CVector* end)` (0x712E40, cdecl): every active bird hit by the
/// shot line (`CColLine(start, end)` against a sphere of radius 0.5f at the bird, `CCollision::TestLineSphere`) is
/// shot: `vecBirdShotAt` = its position, `bHasBirdBeenShot` = 1, inactive, `NumberOfBirds - 1`. The entry jumps to the
/// no-CD chunk 0x40126F, which only loads EDI = 0xC6A8EF (the first bird's active byte) and returns to 0x712E54.
#[hook(addr = 0x712E40, conv = cdecl, name = "CBirds::HandleGunShot", expect = "83 EC 34 53 8B 5C 24 3C 55",
       unverified = "samp-render-3", route = "L2", hazards = "uninit-locals", recs = "weapon-combat",
       note = "the sphere's bytes after the radius (+0x10..+0x13) are left unwritten, as in the original")]
pub fn birds_handle_gun_shot(start: GPtr<u8>, end: GPtr<u8>) {
    for i in 0..6 {
        let b = gp::<u8>(BIRDS + 0x44 * i);
        if rd::<u8>(b, 0x3F) == 0 {
            continue;
        }
        let mut line = [0u32; 8];
        let lp = GPtr::from_raw(line.as_mut_ptr()).cast::<u8>();
        col_line_ctor(lp, start, end);
        let mut sphere = [rd::<u32>(b, 0), rd::<u32>(b, 4), rd::<u32>(b, 8), 0x3F00_0000, 0];
        if test_line_sphere(lp, GPtr::from_raw(sphere.as_mut_ptr()).cast()) == 0 {
            continue;
        }
        gp::<u32>(0xC6_AA48).write(rd::<u32>(b, 0));
        gp::<u8>(0xC6_A8A0).write(1);
        gp::<u32>(0xC6_AA4C).write(rd::<u32>(b, 4));
        gp::<u32>(0xC6_AA50).write(rd::<u32>(b, 8));
        wr::<u8>(b, 0x3F, 0);
        let n = gp::<i32>(0xC6_A8A4);
        n.write(n.read().wrapping_sub(1));
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_SHADOWS_RENDER_INDICATOR_SHADOW,
    HOOK_SHADOWS_GUN_SHOT_SETS_OIL_ON_FIRE,
    HOOK_BIRDS_HANDLE_GUN_SHOT,
];
