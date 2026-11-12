//! WRITER-NATIVE (batch `native-31`): `CPedGeometryAnalyser::CanPedJumpObstacle(const CPed&, const CEntity&,
//! const CVector& contactNormal, const CVector& contactPos)` (0x5F32D0), reached when peds bump into obstacles on
//! the native and web builds. Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes win). x87
//! arithmetic follows the original's instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `SurfaceInfos_c::IsShallowWater(surface)` (0x55E730) on `g_surfaceInfos` (0xB79538)
    fn surface_is_shallow_water(s: GPtr<u8>, surface: u32) -> u8 = thiscall 0x55E730 => unported;
    /// `CPedGroups::IsInPlayersGroup(CPed*)` (0x5F7F10)
    fn groups_is_in_players_group(ped: GPtr<u8>) -> u8 = cdecl 0x5F7F10 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, returns `out`)
    fn vector_mul_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00)
    fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x411A00 => unported;
    /// `CVector::operator*=(float)` (0x40FEF0)
    fn vector_mul_assign(v: GPtr<u8>, s: f32) -> GPtr<u8> = thiscall 0x40FEF0 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490)
    #[allow(clippy::too_many_arguments)]
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ig: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CWorld::FindGroundZFor3DCoord(float, float, float, bool*, CEntity**)` (0x5696C0, ST0)
    fn world_find_ground_z_for_3d_coord(x: f32, y: f32, z: f32, on: GPtr<u8>, e: GPtr<u8>) -> f64 = cdecl 0x5696C0 => unported;
}

/// `CVector::operator/=(float)` (0x411A30; not a catalogued function, inlined here): `inv = 1 / s` kept on the
/// x87 stack, each component `inv * c` stored.
fn vector_div_assign(v: GPtr<u8>, s: f32) {
    let inv = x(1.0).fdiv_m(s);
    for o in [0, 4, 8] {
        wr::<f32>(v, o, inv.fmul_m(rd::<f32>(v, o)).to_f32());
    }
}

/// `bool CPedGeometryAnalyser::CanPedJumpObstacle(const CPed& ped, const CEntity& entity, const CVector& normal,
/// const CVector& contactPos)` (0x5F32D0, cdecl, AL).
///
/// False for a temporary building (+0x1C bit 22), true on shallow water. `from` = the ped position, `dir` = its
/// forward vector. A steep contact (`normal.z <= 0.17`): `from.z -= 0.15` unless the ped is in the player's group.
/// Above 0.9: false. Between: `from.z += sphere0.centre.z - sphere0.radius * normal.z` (model collision), `m =
/// |normal.xy|`; up to 0.5 `dir += (dir * m) * radius`, above it `dir = -normal.xy / m`, `dir += (dir * m) *
/// radius`, `dir *= min(2 / m, 4)`. Then a line of sight (buildings and objects) from `from` to `from + dir`, and
/// the ground under `from + dir * 3`: true when found and less than 3 below that point. The original keeps the 2D
/// normal length and the ground flag in the ped argument slot.
#[hook(addr = 0x5F32D0, conv = cdecl, name = "CPedGeometryAnalyser::CanPedJumpObstacle-Contacted",
       expect = "83 EC 30 56 8B 74 24 38",
       unverified = "native-31", route = "L2", hazards = "x87-transcendental,ax-al-return,stack-residue",
       recs = "grove-walk", note = "fsqrt; CVector::operator/= at 0x411A30 inlined (not catalogued)")]
pub fn can_ped_jump_obstacle_contacted(ped: GPtr<u8>, entity: GPtr<u8>, normal: GPtr<f32>, _pos: GPtr<u8>) -> u8 {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let p = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let mut from = [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)];
    let fwd_row = m.byte_add(0x10);
    let mut dir = [rd::<f32>(fwd_row, 0), rd::<f32>(fwd_row, 4), rd::<f32>(fwd_row, 8)];
    let dirp: GPtr<u8> = GPtr::from_raw(dir.as_mut_ptr()).cast();
    if rd::<u32>(entity, 0x1C) & 0x40_0000 != 0 {
        return 0;
    }
    if surface_is_shallow_water(GPtr::from_addr(0xB7_9538), u32::from(rd::<u8>(ped, 0xBA))) != 0 {
        return 1;
    }
    let nz = x(normal.wrapping_index(2).read());
    if nz.fcom(x(f32::from_bits(0x3E2E_147B))).test_any(0x41) {
        if groups_is_in_players_group(ped) == 0 {
            from[2] = x(from[2]).fsub_m(f32::from_bits(0x3E19_999A)).to_f32();
        }
    } else {
        if !nz.fcom(x(f32::from_bits(0x3F66_6666))).test_any(0x41) {
            return 0;
        }
        let info = g::<GPtr<u8>>(0xA9_B0C8).wrapping_index(i32::from(rd::<i16>(ped, 0x22))).read();
        let spheres = rd::<GPtr<u8>>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(info, 0x14), 0x2C), 8);
        let r = rd::<f32>(spheres, 0xC);
        from[2] = x(r).fmul_m(normal.wrapping_index(2).read()).fsubr_m(rd::<f32>(spheres, 8)).fadd_m(from[2]).to_f32();
        let (nx, ny) = (x(normal.read()), x(normal.wrapping_index(1).read()));
        let mag = nx.fmul(nx).fadd(ny.fmul(ny)).fsqrt().to_f32();
        let mut t1 = [0f32; 3];
        let mut t2 = [0f32; 3];
        let t1p: GPtr<u8> = GPtr::from_raw(t1.as_mut_ptr()).cast();
        let t2p: GPtr<u8> = GPtr::from_raw(t2.as_mut_ptr()).cast();
        if !nz.fcom(x(0.5)).test_any(0x41) {
            wr::<f32>(dirp, 0, nx.fchs().to_f32());
            wr::<u32>(dirp, 8, 0);
            wr::<f32>(dirp, 4, ny.fchs().to_f32());
            vector_div_assign(dirp, mag);
            let a = vector_mul_scalar(t1p, dirp, mag);
            let b = vector_mul_scalar(t2p, a, r);
            vector_add_assign(dirp, b);
            let q = x(2.0).fdiv_m(mag);
            if x(4.0).fcom(q).test_parity(0x05) {
                vector_mul_assign(dirp, q.to_f32());
            } else {
                vector_mul_assign(dirp, 4.0);
            }
        } else {
            let a = vector_mul_scalar(t2p, dirp, mag);
            let b = vector_mul_scalar(t1p, a, r);
            vector_add_assign(dirp, b);
        }
    }
    let d = [rd::<f32>(dirp, 0), rd::<f32>(dirp, 4), rd::<f32>(dirp, 8)];
    let mut to = [
        x(from[0]).fadd_m(d[0]).to_f32(),
        x(from[1]).fadd_m(d[1]).to_f32(),
        x(d[2]).fadd_m(from[2]).to_f32(),
    ];
    let clear = world_get_is_line_of_sight_clear(
        GPtr::from_raw(from.as_mut_ptr()).cast(),
        GPtr::from_raw(to.as_mut_ptr()).cast(),
        1,
        0,
        0,
        1,
        0,
        0,
        0,
    );
    if clear == 0 {
        return 0;
    }
    let mut on_ground: u32 = 0;
    let px = x(d[0]).fmul_m(3.0).fadd_m(from[0]).to_f32();
    let py = x(d[1]).fmul_m(3.0).fadd_m(from[1]).to_f32();
    let pz = x(x(d[2]).fmul_m(3.0).to_f32()).fadd_m(from[2]).to_f32();
    let gz = world_find_ground_z_for_3d_coord(px, py, pz, GPtr::from_raw(&mut on_ground as *mut u32).cast(), GPtr::NULL);
    if on_ground as u8 == 0 {
        return 0;
    }
    u8::from(!X87::from_f64_exact(gz).fsubr_m(pz).fcom(x(3.0)).test_parity(0x05))
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CAN_PED_JUMP_OBSTACLE_CONTACTED];
