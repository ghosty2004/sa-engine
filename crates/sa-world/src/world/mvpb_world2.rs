//! WRITER-MVP-B (helperB) WORLD ports, part 2: `CWorld::ProcessAttachedEntities`, `RemoveFallenPeds`,
//! `RemoveFallenCars`. Ported from the 1.0 US bytes (the exe wins over gta-reversed).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{g, rd, wr, x};

/// `ThePaths`
const THE_PATHS: usize = 0x96F050;

orig! {
    /// `CPhysical::PositionAttachedEntity` (0x546FF0)
    fn position_attached_entity(e: GPtr<u8>) = thiscall 0x546FF0 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70, `ret 4`)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40, `ret 4`)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CPhysical::AddToMovingList` (0x542800)
    fn add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `CPhysical::RemoveFromMovingList` (0x542860)
    fn remove_from_moving_list(e: GPtr<u8>) = thiscall 0x542860 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPopulation::RemovePed(CPed*)` (0x610F20, cdecl)
    fn population_remove_ped(p: GPtr<u8>) = cdecl 0x610F20 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0, cdecl)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CPathFind::FindNodeClosestToCoors(CVector, uint8, float, bool, bool, bool, bool, bool)` (0x44F460, `ret 0x2C`,
    /// hidden return pointer first)
    fn find_node_closest_to_coors(pf: GPtr<u8>, out: GPtr<u32>, x: u32, y: u32, z: u32, ty: u32, d: f32, a: u32, b: u32, c: u32, e: u32, f: u32) -> GPtr<u32> = thiscall 0x44F460 => unported;
    /// `CEntity::Add` (vtable +0x8)
    fn v_add(e: GPtr<u8>) = thiscall vslot 0x8 => unported;
    /// `CEntity::Remove` (vtable +0xC)
    fn v_remove(e: GPtr<u8>) = thiscall vslot 0xC => unported;
    /// `CPlaceable::Teleport(CVector, bool)` (vtable +0x38)
    fn v_teleport(e: GPtr<u8>, x: f32, y: f32, z: f32, b: u32) = thiscall vslot 0x38 => unported;
    /// scalar deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CGarage::IsPointInsideGarage(CVector)` (0x448740, `ret 0xC`)
    fn garage_is_point_inside(g: GPtr<u8>, x: u32, y: u32, z: u32) -> u8 = thiscall 0x448740 => unported;
    /// `CGarage::IsPointInsideGarage(CVector, float)` (0x4487D0, `ret 0x10`)
    fn garage_is_point_inside_r(g: GPtr<u8>, x: f32, y: f32, z: f32, r: f32) -> u8 = thiscall 0x4487D0 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280, cdecl)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CEntity::GetColModel` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CPlaceable::AllocateMatrix` (0x54F560)
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CCompressedMatrixNotAligned::CompressFromFullMatrix(CMatrix&)` (0x59BAD0, `ret 4`)
    fn compress_from_full_matrix(c: GPtr<u8>, m: GPtr<u8>) = thiscall 0x59BAD0 => unported;
    /// `CReplay::StorePedAnimation(CPed*, CStoredAnimationState&)` (0x45B170, cdecl)
    fn replay_store_ped_animation(p: GPtr<u8>, s: GPtr<u8>) = cdecl 0x45B170 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `operator*(CVector* out, const CMatrix*, const CVector*)` (0x59C890, cdecl)
    fn matrix_mul_vec(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<f32> = cdecl 0x59C890 => unported;
}

/// The inline `UpdateRwMatrix` + `UpdateRwFrame` pair.
fn update_rw(e: GPtr<u8>) {
    let rw = rd::<GPtr<u8>>(e, 0x18);
    if !rw.is_null() {
        let m = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let mat = rd::<GPtr<u8>>(e, 0x14);
        if mat.is_null() {
            simple_transform_update_rw_matrix(e.byte_add(4), m);
        } else {
            matrix_update_rw_matrix(mat, m);
        }
    }
    entity_update_rw_frame(e);
}

/// Calls `f` for every used slot of the pool at `pool_cell` (objects of `size` bytes), from the last slot down.
fn for_pool_desc(pool_cell: usize, size: u32, mut f: impl FnMut(GPtr<u8>)) {
    let pool = g::<GPtr<u8>>(pool_cell).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        let flags = rd::<GPtr<u8>>(pool, 4);
        let fl = rd::<u8>(flags, (i - 1) as isize);
        i -= 1;
        if fl & 0x80 != 0 {
            continue;
        }
        let e: GPtr<u8> = GPtr::from_addr(rd::<u32>(pool, 0).wrapping_add(size.wrapping_mul(i as u32)) as usize);
        if e.is_null() {
            continue;
        }
        f(e);
    }
}

fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CWorld::ProcessAttachedEntities` (0x5647F0, static): every attached vehicle (+0xFC; pool walked from the end)
/// gets +0x128 = the entity it is attached to, `PositionAttachedEntity` and its RW matrix/frame updated; every
/// attached object is removed from the world (and the moving list when physical), positioned, updated twice,
/// re-added, and put back on the moving list unless it is a building/dummy or static (+0x1C bits 4 / 0x40000).
#[hook(addr = 0x5647F0, conv = cdecl, name = "CWorld::ProcessAttachedEntities", expect = "53 8B 1D 94 44 B7 00",
       unverified = "mvpb-world-2", route = "L2", hazards = "heap-writes")]
pub fn world_process_attached_entities() {
    for_pool_desc(0xB74494, 0xA18, |v| {
        let a = rd::<u32>(v, 0xFC);
        if a == 0 {
            return;
        }
        wr::<u32>(v, 0x128, a);
        position_attached_entity(v);
        update_rw(v);
    });
    for_pool_desc(0xB7449C, 0x19C, |o| {
        if rd::<u32>(o, 0xFC) == 0 {
            return;
        }
        v_remove(o);
        let t = rd::<u8>(o, 0x36) & 7;
        if t > 1 && t < 5 {
            remove_from_moving_list(o);
        }
        position_attached_entity(o);
        update_rw(o);
        update_rw(o);
        v_add(o);
        let t = rd::<u8>(o, 0x36) & 7;
        if t == 1 || t == 5 {
            return;
        }
        let f = rd::<u32>(o, 0x1C);
        if f & 4 != 0 || f & 0x40000 != 0 {
            return;
        }
        add_to_moving_list(o);
    });
}

/// True when the entity's z is below -100 (x87 compare, unordered false).
fn fell(e: GPtr<u8>) -> bool {
    !x(rd::<f32>(entity_pos(e), 8)).fcom(x(-100.0)).test_parity(5)
}

/// `FindNodeClosestToCoors(pos, type, 999999.9, false x5)`; returns (area, node).
fn closest_node(e: GPtr<u8>, ty: u32) -> (u16, u16) {
    let p = entity_pos(e);
    let mut out = 0u32;
    find_node_closest_to_coors(
        GPtr::from_addr(THE_PATHS),
        GPtr::from_raw(&mut out),
        rd::<u32>(p, 0),
        rd::<u32>(p, 4),
        rd::<u32>(p, 8),
        ty,
        999_999.9,
        0,
        0,
        0,
        0,
        0,
    );
    (out as u16, (out >> 16) as u16)
}

/// The node's position (`x/8`, `y/8`, `z/8 + dz`, x87 PC24).
fn node_pos(area: u16, node: u16, dz: f32) -> (f32, f32, f32) {
    let base = rd::<u32>(g::<u8>(0x96F854), 4 * area as isize);
    let n: GPtr<u8> = GPtr::from_addr(base.wrapping_add(0x1C * node as u32) as usize);
    let z = X87::from_i32(rd::<i16>(n, 0xC) as i32).fmul_m(0.125);
    let y = X87::from_i32(rd::<i16>(n, 0xA) as i32).fmul_m(0.125);
    let xx = X87::from_i32(rd::<i16>(n, 8) as i32).fmul_m(0.125);
    (xx.to_f32(), y.to_f32(), z.fadd_m(dz).to_f32())
}

fn clear_move_speed(e: GPtr<u8>) {
    wr::<u32>(e, 0x44, 0);
    wr::<u32>(e, 0x48, 0);
    wr::<u32>(e, 0x4C, 0);
}

/// `CWorld::RemoveFallenPeds` (0x565CB0, static): every ped below z = -100 (pool walked from the end): random peds
/// (+0x484 = 1) that are not players are removed (`CPopulation::RemovePed`); others are teleported to 2 above the
/// closest ped node (type 1), or, without one, their matrix z is set to -95 (matrix not null-checked, as the
/// original); their move speed is cleared.
#[hook(addr = 0x565CB0, conv = cdecl, name = "CWorld::RemoveFallenPeds", expect = "83 EC 2C 53 8B 1D 90 44 B7 00",
       unverified = "mvpb-world-2", route = "L2", hazards = "heap-writes")]
pub fn world_remove_fallen_peds() {
    for_pool_desc(0xB74490, 0x7C4, |p| {
        if !fell(p) {
            return;
        }
        if rd::<u8>(p, 0x484) == 1 && ped_is_player(p) == 0 {
            population_remove_ped(p);
            return;
        }
        let (area, node) = closest_node(p, 1);
        if area == 0xFFFF {
            wr::<u32>(rd::<GPtr<u8>>(p, 0x14), 0x38, 0xC2BE_0000);
        } else {
            let (nx, ny, nz) = node_pos(area, node, 2.0);
            v_teleport(p, nx, ny, nz, 0);
        }
        clear_move_speed(p);
    });
}

/// `CWorld::RemoveFallenCars` (0x565E80, static): every vehicle below z = -100: mission vehicles (+0x4A4 = 2)
/// without flag 0x20000000 in +0x40, the player's vehicle and vehicles driven by a player are teleported (closest
/// car node + 3, or without one the same x/y at z = 0); random (1) and parked (3) vehicles are removed from the
/// world and the moving list and deleted; the rest are left alone. Teleported vehicles lose their move speed.
#[hook(addr = 0x565E80, conv = cdecl, name = "CWorld::RemoveFallenCars", expect = "83 EC 2C 53 8B 1D 94 44 B7 00",
       unverified = "mvpb-world-2", route = "L2", hazards = "heap-writes")]
pub fn world_remove_fallen_cars() {
    for_pool_desc(0xB74494, 0xA18, |v| {
        if !fell(v) {
            return;
        }
        let keep = (rd::<u8>(v, 0x4A4) == 2 && rd::<u32>(v, 0x40) & 0x2000_0000 == 0)
            || v.addr() == find_player_vehicle(-1, 0).addr()
            || {
                let d = rd::<GPtr<u8>>(v, 0x460);
                !d.is_null() && ped_is_player(d) != 0
            };
        if !keep {
            let c = rd::<u8>(v, 0x4A4);
            if c == 1 || c == 3 {
                v_remove(v);
                let t = rd::<u8>(v, 0x36) & 7;
                if t > 1 && t < 5 {
                    remove_from_moving_list(v);
                }
                v_delete(v, 1);
            }
            return;
        }
        let (area, node) = closest_node(v, 0);
        if area == 0xFFFF {
            let p = entity_pos(v);
            let px = f32::from_bits(rd::<u32>(p, 0));
            let py = x(rd::<f32>(p, 4)).to_f32();
            v_teleport(v, px, py, 0.0, 1);
        } else {
            let (nx, ny, nz) = node_pos(area, node, 3.0);
            v_teleport(v, nx, ny, nz, 1);
        }
        clear_move_speed(v);
    });
}

/// `CGarage::IsEntityEntirelyOutside(CEntity*, float margin)` (0x448D30, `ret 8`): AL = 0 when the entity's
/// position is strictly inside the garage box widened by the margin (x87 compares, unordered counts as outside);
/// otherwise AL = 0 as soon as one collision sphere (centre transformed by the entity matrix, allocated first when
/// missing; radius + margin) touches the garage, else 1 (also without col data).
#[hook(addr = 0x448D30, conv = thiscall, name = "CGarage::IsEntityEntirelyOutside", this = "CGarage",
       expect = "83 EC 10 53 8B 5C 24 18", unverified = "mvpb-world-3", route = "L2", hazards = "heap-writes",
       note = "the original reuses the operator* argument slots as the next call's arguments (stack-identical)")]
pub fn garage_is_entity_entirely_outside(this: GPtr<u8>, e: GPtr<u8>, margin: f32) -> u8 {
    let px = rd::<f32>(entity_pos(e), 0);
    let py = rd::<f32>(entity_pos(e), 4);
    let inside = !x(rd::<f32>(this, 0x28)).fsub_m(margin).fcom(x(px)).test_parity(5)
        && !x(margin).fadd_m(rd::<f32>(this, 0x2C)).fcom(x(px)).test_any(0x41)
        && !x(rd::<f32>(this, 0x30)).fsub_m(margin).fcom(x(py)).test_parity(5)
        && !x(margin).fadd_m(rd::<f32>(this, 0x34)).fcom(x(py)).test_any(0x41);
    if inside {
        return 0;
    }
    let cd = rd::<GPtr<u8>>(entity_get_col_model(e), 0x2C);
    if cd.is_null() {
        return 1;
    }
    let mut i: i16 = 0;
    while i < rd::<i16>(cd, 0) {
        if rd::<u32>(e, 0x14) == 0 {
            allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let sphere = rd::<GPtr<u8>>(cd, 8).byte_add(0x14 * i as isize);
        let mut out = [0.0f32; 3];
        let c = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()), rd::<GPtr<u8>>(e, 0x14), sphere);
        let r = x(margin).fadd_m(rd::<f32>(rd::<GPtr<u8>>(cd, 8), 0x14 * i as isize + 0xC)).to_f32();
        if garage_is_point_inside_r(this, c.read(), c.wrapping_index(1).read(), c.wrapping_index(2).read(), r) != 0 {
            return 0;
        }
        i += 1;
    }
    1
}

/// `CGarage::TidyUpGarage` (0x449C50): every automobile/bike (+0x590 = 0 or 9) of the vehicle pool (walked from
/// the last slot down to slot 1: slot 0 is never visited, as in the original) inside the garage that is wrecked
/// (status 5) or upside down (matrix up.z < 0.5) is removed from the world and deleted.
#[hook(addr = 0x449C50, conv = thiscall, name = "CGarage::TidyUpGarage", this = "CGarage", expect = "83 EC 08 53 55 56",
       unverified = "mvpb-world-3", route = "L2", hazards = "heap-writes")]
pub fn garage_tidy_up_garage(this: GPtr<u8>) {
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let mut i = rd::<i32>(pool, 8).wrapping_sub(1);
    while i != 0 {
        let fl = rd::<u8>(rd::<GPtr<u8>>(pool, 4), i as isize);
        if fl & 0x80 == 0 {
            let v: GPtr<u8> = GPtr::from_addr(rd::<u32>(pool, 0).wrapping_add(0xA18u32.wrapping_mul(i as u32)) as usize);
            if !v.is_null() {
                let st = rd::<u32>(v, 0x590);
                if st == 0 || st == 9 {
                    let m = rd::<GPtr<u8>>(v, 0x14);
                    let p = entity_pos(v);
                    if garage_is_point_inside(this, rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)) != 0
                        && (rd::<u8>(v, 0x36) & 0xF8 == 0x28 || !x(rd::<f32>(m, 0x28)).fcom(x(0.5)).test_parity(5))
                    {
                        world_remove(v);
                        v_delete(v, 1);
                    }
                }
            }
        }
        i -= 1;
    }
}

/// `CReplay::StorePedUpdate(CPed*, int32 index)` (0x45C940, cdecl): writes a 0x34-byte ped block (type 4) at the
/// replay write position: index, heading (`(int8)(+0x558 * 40.764328)`), compressed matrix (+0x18), +0x4D4 word,
/// `(int8)(+0x12C * 100)`, flag bits from +0x1C (bit 7 -> 0, bit 30 -> 1, bit 31 -> 2; bit 0 cleared for the
/// player while 0xB6EC20 is set; the byte is written three times as the original), the vehicle slot + 1 of a ped
/// in a vehicle (else 0), +0x740 word, the animation state (+4), then advances the position by 0x34.
#[hook(addr = 0x45C940, conv = cdecl, name = "CReplay::StorePedUpdate", expect = "A1 70 FB 97 00",
       unverified = "mvpb-world-4", route = "L2", hazards = "none")]
pub fn replay_store_ped_update(ped: GPtr<u8>, idx: i32) {
    let b: GPtr<u8> = GPtr::from_addr(g::<u32>(0x97FB70).read().wrapping_add(g::<u32>(0x97FB74).read()) as usize);
    wr::<u8>(b, 1, idx as u8);
    wr::<u8>(b, 0, 4);
    wr::<u8>(b, 2, x(rd::<f32>(ped, 0x558)).fmul_m(f32::from_bits(0x4223_0EAC)).ftol() as u8);
    compress_from_full_matrix(b.byte_add(0x18), rd::<GPtr<u8>>(ped, 0x14));
    wr::<u16>(b, 0x2E, rd::<u16>(ped, 0x4D4));
    wr::<u8>(b, 0x30, x(rd::<f32>(ped, 0x12C)).fmul_m(100.0).ftol() as u8);
    let f = rd::<u32>(ped, 0x1C);
    let a = (rd::<u8>(b, 0x31) & !1) | ((f >> 7) & 1) as u8;
    wr::<u8>(b, 0x31, a);
    let f = rd::<u32>(ped, 0x1C);
    let a = (a & !2) | (((f >> 30) as u8) << 1 & 2);
    wr::<u8>(b, 0x31, a);
    let f = rd::<u32>(ped, 0x1C);
    let a = (a & !4) | (((f >> 31) as u8) << 2 & 4);
    wr::<u8>(b, 0x31, a);
    if ped.addr() == find_player_ped(-1).addr() && g::<u8>(0xB6EC20).read() != 0 {
        wr::<u8>(b, 0x31, rd::<u8>(b, 0x31) & 0xFE);
    }
    let veh = rd::<u32>(ped, 0x58C);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 && veh != 0 {
        let pool = g::<GPtr<u8>>(0xB74494).read();
        let i = (veh.wrapping_sub(rd::<u32>(pool, 0)) as i32) / 0xA18;
        let fl = rd::<u8>(rd::<GPtr<u8>>(pool, 4), i as isize) as i32;
        let h = (i.wrapping_shl(8).wrapping_add(fl)) >> 8;
        wr::<u8>(b, 3, (h as u8).wrapping_add(1));
    } else {
        wr::<u8>(b, 3, 0);
    }
    wr::<u16>(b, 0x2C, rd::<u16>(ped, 0x740));
    replay_store_ped_animation(ped, b.byte_add(4));
    let p = g::<u32>(0x97FB70);
    p.write(p.read().wrapping_add(0x34));
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_REPLAY_STORE_PED_UPDATE,
    HOOK_GARAGE_IS_ENTITY_ENTIRELY_OUTSIDE,
    HOOK_GARAGE_TIDY_UP_GARAGE,
    HOOK_WORLD_PROCESS_ATTACHED_ENTITIES,
    HOOK_WORLD_REMOVE_FALLEN_PEDS,
    HOOK_WORLD_REMOVE_FALLEN_CARS,
];
