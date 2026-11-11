//! WRITER-MVP-B (helperB) WORLD ports: `CEntity` (destructor, `UpdateRwMatrix`, `Render`, `RenderEffects`),
//! `CWorld::FindObjectsInRange` / `FindObjectsOfTypeInRange`, replay block writers (`RecordPedDeleted`,
//! `RecordVehicleDeleted`, BMX/bike update data, `StoreClothesDesc`) and two garage helpers. Ported from the
//! 1.0 US bytes (the exe wins over gta-reversed).
//!
//! The `.HOODLUM` bodies of `CEntity::UpdateRwMatrix`, `RecordPedDeleted` and `RecordVehicleDeleted` call through
//! an obfuscated return-address trick whose target the protection computes from data; evaluated statically it is
//! `CMatrix::UpdateRwMatrix` (0x59AD70) resp. `CReplay::GoToNextBlock` (0x45E2A0), called directly here.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{ftol32, g, rd, wr, x};
use super::cworld::CURRENT_SCAN_CODE;
use crate::physical::base::floor_f64;

orig! {
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70, `ret 4`)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40, `ret 4`)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CBaseModelInfo::Get2dEffect(int32)` (0x4C4C70, `ret 4`)
    fn get_2d_effect(mi: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4C4C70 => unported;
    /// `CCustomRoadsignMgr::RenderRoadsignAtomic(RpAtomic*, const CVector&)` (0x6FF350, cdecl)
    fn render_roadsign_atomic(a: GPtr<u8>, cam: GPtr<u8>) = cdecl 0x6FF350 => unported;
    /// `CTagManager::IsTag(const CEntity*)` (0x49CCE0, cdecl)
    fn tag_manager_is_tag(e: GPtr<u8>) -> u8 = cdecl 0x49CCE0 => unported;
    /// `CTagManager::RenderTagForPC(RpAtomic*)` (0x49CE40, cdecl)
    fn tag_manager_render_tag_for_pc(a: GPtr<u8>) = cdecl 0x49CE40 => unported;
    /// `RwEngineInstance->dOpenDevice.fpRenderStateGet(state, void*)` (table +0x24)
    fn rw_render_state_get(state: u32, out: GPtr<u32>) -> u32 = cdecl table 0xC97B24 0x24 => unported;
    /// `RwEngineInstance->dOpenDevice.fpRenderStateSet(state, value)` (table +0x20)
    fn rw_render_state_set(state: u32, v: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;
    /// `atomic->renderCallBack(atomic)` (the pointer at RpAtomic +0x48)
    fn atomic_render_callback(f: u32, a: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// `RpClumpRender(RpClump*)` (0x749B20, cdecl)
    fn rp_clump_render(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B20 => unported;
    /// `CStreaming::RenderEntity(CLink<CEntity*>*)` (0x4096D0, cdecl)
    fn streaming_render_entity(l: GPtr<u8>) = cdecl 0x4096D0 => unported;
    /// `CEntity::RenderEffects` (0x5342B0)
    fn entity_render_effects_o(e: GPtr<u8>) = thiscall 0x5342B0 => crate::world::mvpb_world::entity_render_effects;
    /// `CEntity::DeleteRwObject` (0x534030)
    fn entity_delete_rw_object(e: GPtr<u8>) = thiscall 0x534030 => unported;
    /// `CEntity::ResolveReferences` (0x571A40)
    fn entity_resolve_references(e: GPtr<u8>) = thiscall 0x571A40 => crate::world::entity::resolve_references;
    /// `CPlaceable::~CPlaceable` (0x54F490)
    fn placeable_dtor(e: GPtr<u8>) = thiscall 0x54F490 => unported;
    /// `CReplay::GoToNextBlock` (0x45E2A0)
    fn replay_go_to_next_block() = cdecl 0x45E2A0 => unported;
    /// `tReplayVehicleBlock::MakeVehicleUpdateData(CVehicle*, int32)` (0x45B730, `ret 8`)
    fn make_vehicle_update_data(b: GPtr<u8>, v: GPtr<u8>, i: i32) = thiscall 0x45B730 => unported;
    /// vehicle virtual +0xBC (returns a vector pointer: the steering/lean data the replay stores)
    fn v_vehicle_bc(v: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0xBC => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0, cdecl)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CGarage::IsEntityEntirelyOutside(CEntity*, float)` (0x448D30, `ret 8`)
    fn garage_is_entity_entirely_outside(g: GPtr<u8>, e: GPtr<u8>, m: f32) -> u32 = thiscall 0x448D30 => unported;
    /// `CWorld::ClearScanCodes` (0x563470)
    fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CWorld::FindObjectsInRangeSectorList` (0x563500, cdecl)
    fn find_in_range_list(list: GPtr<u8>, p: GPtr<u8>, r: f32, two_d: u8, n: GPtr<i16>, max: i16, out: GPtr<GPtr<u8>>) = cdecl 0x563500 => crate::world::cworld2::find_objects_in_range_sector_list;
    /// `CWorld::FindObjectsOfTypeInRangeSectorList` (0x5635C0, cdecl)
    fn find_of_type_in_range_list(m: i32, list: GPtr<u8>, p: GPtr<u8>, r: f32, two_d: u8, n: GPtr<i16>, max: i16, out: GPtr<GPtr<u8>>) = cdecl 0x5635C0 => crate::world::cworld2::find_objects_of_type_in_range_sector_list;
}

/// `CEntity::UpdateRwMatrix` (0x446F90 -> .HOODLUM 0x1569BF0): with an RW object, its parent frame's matrix
/// (+0x10) is updated from the entity matrix (+0x14) or, without one, from the simple transform (+4).
#[hook(addr = 0x446F90, conv = thiscall, name = "CEntity::UpdateRwMatrix", this = "CEntity", expect = "E9 5B 2C 12 01",
       route = "L2", hazards = "heap-writes")]
pub fn entity_update_rw_matrix(this: GPtr<u8>) {
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if rw.is_null() {
        return;
    }
    let m = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
    let mat = rd::<GPtr<u8>>(this, 0x14);
    if mat.is_null() {
        simple_transform_update_rw_matrix(this.byte_add(4), m);
    } else {
        matrix_update_rw_matrix(mat, m);
    }
}

/// `CEntity::RenderEffects` (0x5342B0): entities flagged with 2d effects (+0x1F bit 0) render every road-sign
/// effect (type 7) of their model at the camera position (`TheCamera` entity matrix position or its vector).
#[hook(addr = 0x5342B0, conv = thiscall, name = "CEntity::RenderEffects", this = "CEntity", expect = "F6 41 1F 01 74 53",
       route = "L2", hazards = "none")]
pub fn entity_render_effects(this: GPtr<u8>) {
    if rd::<u8>(this, 0x1F) & 1 == 0 {
        return;
    }
    let mi = rd::<GPtr<u8>>(g::<u8>(0xA9B0C8), 4 * rd::<i16>(this, 0x22) as isize);
    let n = rd::<u8>(mi, 0xD) as i32;
    for i in 0..n {
        let e = get_2d_effect(mi, i);
        if rd::<u8>(e, 0xC) != 7 {
            continue;
        }
        let ce = g::<GPtr<u8>>(0xB6F03C).read();
        let cam = if ce.is_null() { GPtr::from_addr(0xB6F02C) } else { ce.byte_add(0x30) };
        render_roadsign_atomic(rd::<GPtr<u8>>(e, 0x2C), cam);
    }
}

/// `CEntity::Render` (0x534310): tags render through `CTagManager::RenderTagForPC`; the two models at
/// 0x8CD644/0x8CD648 render with render state 0x1E forced to 0 (restored after); the entity is flagged as being
/// rendered (+0x1C bit 0x2000) while its atomic (render callback) or clump renders, then the streaming link is
/// touched (`CStreaming::RenderEntity(+0x28)`) and `RenderEffects` runs.
#[hook(addr = 0x534310, conv = thiscall, name = "CEntity::Render", this = "CEntity", expect = "51 56 8B F1 8B 46 18",
       unverified = "mvpb-world-1", route = "L2", hazards = "heap-writes")]
pub fn entity_render(this: GPtr<u8>) {
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if rw.is_null() {
        return;
    }
    if rd::<u8>(rw, 0) == 1 && tag_manager_is_tag(this) != 0 {
        tag_manager_render_tag_for_pc(rd::<GPtr<u8>>(this, 0x18));
        return;
    }
    let special = |this: GPtr<u8>| {
        let id = rd::<i16>(this, 0x22) as i32;
        id == g::<u16>(0x8CD644).read() as i32 || id == g::<u16>(0x8CD648).read() as i32
    };
    let mut saved = 0u32;
    if special(this) {
        rw_render_state_get(0x1E, GPtr::from_raw(&mut saved));
        rw_render_state_set(0x1E, 0);
    }
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x2000);
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if rd::<u8>(rw, 0) == 1 {
        atomic_render_callback(rd::<u32>(rw, 0x48), rw);
    } else {
        rp_clump_render(rw);
    }
    streaming_render_entity(rd::<GPtr<u8>>(this, 0x28));
    entity_render_effects_o(this);
    if special(this) {
        rw_render_state_set(0x1E, saved);
    }
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & 0xFFFF_DFFF);
}

/// `CEntity::~CEntity` (0x535E90): vtable 0x863928, the LOD's child count (+0x30 -> +0x34) decremented,
/// `DeleteRwObject`, `ResolveReferences`, then the `CPlaceable` destructor.
#[hook(addr = 0x535E90, conv = thiscall, name = "CEntity::Destructor", this = "CEntity", expect = "6A FF E9 BE 10 ED FF",
       unverified = "mvpb-world-1", route = "L2", hazards = "heap-writes,entry-protection-read")]
pub fn entity_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x863928);
    let lod = rd::<GPtr<u8>>(this, 0x30);
    if !lod.is_null() {
        wr::<u8>(lod, 0x34, rd::<u8>(lod, 0x34).wrapping_sub(1));
    }
    entity_delete_rw_object(this);
    entity_resolve_references(this);
    placeable_dtor(this);
}

/// Shared body of `RecordPedDeleted` / `RecordVehicleDeleted`: only while recording (mode byte 0xA43088 = 0),
/// a 4-byte block `{type, 0, pool index (word)}` at the replay write position (moving to the next block when it
/// would pass 0x18690), then the next byte is zeroed.
fn record_deleted(e: GPtr<u8>, ty: u8, pool: usize, size: i32) {
    if g::<u8>(0xA43088).read() != 0 {
        return;
    }
    let pos = g::<u32>(0x97FB70);
    if pos.read().wrapping_add(4) > 0x18690 {
        replay_go_to_next_block();
    }
    let base = g::<u32>(0x97FB74);
    let b: GPtr<u8> = GPtr::from_addr(base.read().wrapping_add(pos.read()) as usize);
    wr::<u8>(b, 0, ty);
    let objs = rd::<u32>(g::<GPtr<u8>>(pool).read(), 0);
    let idx = ((e.addr() as u32).wrapping_sub(objs) as i32) / size;
    wr::<u16>(b, 2, idx as u16);
    let p = pos.read().wrapping_add(4);
    pos.write(p);
    wr::<u8>(GPtr::from_addr(p.wrapping_add(base.read()) as usize), 0, 0);
}

/// `CReplay::RecordPedDeleted(CPed*)` (0x45EC20 -> .HOODLUM 0x1561870, cdecl): block type 0xE with the ped's pool
/// index (0x7C4-byte peds, pool 0xB74490).
#[hook(addr = 0x45EC20, conv = cdecl, name = "CReplay::RecordPedDeleted", expect = "E9 4B 2C 10 01",
       unverified = "mvpb-world-1", route = "L2", hazards = "none")]
pub fn replay_record_ped_deleted(ped: GPtr<u8>) {
    record_deleted(ped, 0xE, 0xB74490, 0x7C4);
}

/// `CReplay::RecordVehicleDeleted(CVehicle*)` (0x45EBB0 -> .HOODLUM 0x156DAB0, cdecl): block type 0xD with the
/// vehicle's pool index (0xA18-byte vehicles, pool 0xB74494).
#[hook(addr = 0x45EBB0, conv = cdecl, name = "CReplay::RecordVehicleDeleted", expect = "E9 FB EE 10 01",
       unverified = "mvpb-world-1", route = "L2", hazards = "none")]
pub fn replay_record_vehicle_deleted(v: GPtr<u8>) {
    record_deleted(v, 0xD, 0xB74494, 0xA18);
}

fn two_wheel_update_data(this: GPtr<u8>, v: GPtr<u8>, i: i32, ty: u8) {
    wr::<u8>(this, 0, ty);
    make_vehicle_update_data(this, v, i);
    let a = v_vehicle_bc(v);
    wr::<u8>(this, 0x34, ftol32(x(rd::<f32>(a, 8)).fmul_m(50.0)) as u8);
    let a = v_vehicle_bc(v);
    wr::<u8>(this, 0x35, ftol32(x(rd::<f32>(a, 4)).fmul_m(50.0)) as u8);
}

/// `tReplayBmxBlock::MakeBmxUpdateData(CBmx*, int32)` (0x45BF90, `ret 8`): type 0xF, the vehicle data, then
/// `(int8)(v[+8] * 50)` and `(int8)(v[+4] * 50)` of the vector vslot 0xBC returns (+0x34/+0x35).
#[hook(addr = 0x45BF90, conv = thiscall, name = "tReplayBmxBlock::MakeBmxUpdateData", expect = "8B 44 24 08 56 57",
       unverified = "mvpb-world-1", route = "L2", hazards = "none")]
pub fn replay_make_bmx_update_data(this: GPtr<u8>, v: GPtr<u8>, i: i32) {
    two_wheel_update_data(this, v, i, 0xF);
}

/// `tReplayBikeBlock::MakeBikeUpdateData(CBike*, int32)` (0x45C070, `ret 8`): as the BMX block with type 2.
#[hook(addr = 0x45C070, conv = thiscall, name = "tReplayBikeBlock::MakeBikeUpdateData", expect = "8B 44 24 08 56 57",
       unverified = "mvpb-world-1", route = "L2", hazards = "none")]
pub fn replay_make_bike_update_data(this: GPtr<u8>, v: GPtr<u8>, i: i32) {
    two_wheel_update_data(this, v, i, 2);
}

/// `CReplay::StoreClothesDesc(const CPedClothesDesc&, tReplayClothesBlock*)` (0x45C750, cdecl): type 0x13, the
/// 10 model keys (+4) and 18 texture keys (+0x2C) copied, and the two floats (+0x70/+0x74) truncated to words
/// (+0x74/+0x76).
#[hook(addr = 0x45C750, conv = cdecl, name = "CReplay::StoreClothesDesc", expect = "53 56 8B 74 24 10",
       unverified = "mvpb-world-1", route = "L2", hazards = "none")]
pub fn replay_store_clothes_desc(desc: GPtr<u8>, out: GPtr<u8>) {
    wr::<u8>(out, 0, 0x13);
    for k in 0..28isize {
        wr::<u32>(out, 4 + 4 * k, rd::<u32>(desc, 4 * k));
    }
    wr::<u16>(out, 0x74, ftol32(x(rd::<f32>(desc, 0x70))) as u16);
    wr::<u16>(out, 0x76, ftol32(x(rd::<f32>(desc, 0x74))) as u16);
}

/// `thunk_FUN_01569180(float margin)` (0x448E50 -> .HOODLUM 0x1569180, `ret 4`): `CGarage::IsEntityEntirelyOutside`
/// for the player's vehicle when there is one, else for the player ped.
#[hook(addr = 0x448E50, conv = thiscall, name = "thunk_FUN_01569180", this = "CGarage", expect = "E9 2B 03 12 01",
       route = "L2", hazards = "none")]
pub fn garage_is_player_entirely_outside(this: GPtr<u8>, margin: f32) -> u32 {
    if !find_player_vehicle(-1, 0).is_null() {
        let v = find_player_vehicle(-1, 0);
        garage_is_entity_entirely_outside(this, v, margin)
    } else {
        let p = find_player_ped(-1);
        garage_is_entity_entirely_outside(this, p, margin)
    }
}

/// `FUN_00447d80(float x, float y)` (0x447D80, `ret 8`, float in ST0): the squared 2D distance from the point to
/// the garage's box (x range +0x28..+0x2C, y range +0x30..+0x34; 0 inside on an axis): `dy*dy + dx*dx` in x87.
#[hook(addr = 0x447D80, conv = thiscall, name = "FUN_00447d80", this = "CGarage", expect = "D9 44 24 04 D8 59 28",
       route = "L2", hazards = "none")]
pub fn garage_dist_sq_to_box(this: GPtr<u8>, px: f32, py: f32) -> X87 {
    let axis = |v: f32, lo: f32, hi: f32| {
        if !x(v).fcom(x(lo)).test_parity(5) {
            x(v).fsub_m(lo)
        } else if !x(v).fcom(x(hi)).test_any(0x41) {
            x(v).fsub_m(hi)
        } else {
            X87::zero()
        }
    };
    let dx = axis(px, rd::<f32>(this, 0x28), rd::<f32>(this, 0x2C));
    let dy = axis(py, rd::<f32>(this, 0x30), rd::<f32>(this, 0x34));
    dy.fmul(dy).fadd(dx.fmul(dx))
}

/// `_ftol(floor((v * 0.02 + 60)))` with `v` an x87 value (PC 24), as the range scans compute sector indices.
fn sector_index(v: X87) -> i32 {
    let s = v.fmul_m(g::<f32>(0x858B38).read()).fadd_m(g::<f32>(0x858B34).read());
    X87::from_f64_exact(floor_f64(s.to_f64())).ftol() as u32 as i32
}

/// The sector rectangle the range scans visit (x0, y0, x1, y1), and the scan-code advance they do first.
fn scan_rect(pos: GPtr<u8>, r: f32) -> (i32, i32, i32, i32) {
    let x0 = sector_index(x(rd::<f32>(pos, 0)).fsub_m(r));
    let y0 = sector_index(x(rd::<f32>(pos, 4)).fsub_m(r));
    let x1 = sector_index(x(r).fadd_m(rd::<f32>(pos, 0)));
    let y1 = sector_index(x(r).fadd_m(rd::<f32>(pos, 4)));
    let sc = CURRENT_SCAN_CODE.get();
    if sc < 0xFFFF {
        CURRENT_SCAN_CODE.set(sc + 1);
    } else {
        world_clear_scan_codes();
        CURRENT_SCAN_CODE.set(1);
    }
    (x0, y0, x1, y1)
}

/// The sector (0xB7D0B8, 8 bytes, clamped to 0..119) and repeat sector (0xB992B8, 12 bytes, & 15) of a cell.
fn cell(xs: i32, ys: i32) -> (GPtr<u8>, GPtr<u8>) {
    let cx = xs.clamp(0, 0x77);
    let cy = ys.clamp(0, 0x77);
    let sector = GPtr::from_addr(0xB7D0B8 + 8 * (cy * 0x78 + cx) as usize);
    let rep = GPtr::from_addr(0xB992B8 + 12 * (((ys & 0xF) << 4) + (xs & 0xF)) as usize);
    (sector, rep)
}

/// `CWorld::FindObjectsInRange(const CVector&, float, bool, int16*, int16, CEntity**, bool buildings, bool
/// vehicles, bool peds, bool objects, bool dummies)` (0x564A20, cdecl): advances the scan code, zeroes the count
/// and scans the selected lists (sector buildings, repeat-sector vehicles/peds/objects, sector dummies) of every
/// cell of the radius' sector rectangle (y outer, x inner). Kept original detail: the last x sector index is
/// stored in the caller's first argument slot (not modelled).
#[hook(addr = 0x564A20, conv = cdecl, name = "CWorld::FindObjectsInRange", expect = "83 EC 18 53 55 56 57",
       unverified = "mvpb-world-1", route = "L2", hazards = "none",
       note = "overwrites its own (caller-owned) pos argument slot with the end x sector")]
#[allow(clippy::too_many_arguments)]
pub fn world_find_objects_in_range(
    pos: GPtr<u8>,
    r: f32,
    two_d: u8,
    count: GPtr<i16>,
    max: i16,
    out: GPtr<GPtr<u8>>,
    b: u8,
    v: u8,
    p: u8,
    o: u8,
    d: u8,
) {
    let (x0, y0, x1, y1) = scan_rect(pos, r);
    count.write(0);
    for ys in y0..=y1 {
        for xs in x0..=x1 {
            let (sector, rep) = cell(xs, ys);
            if b != 0 {
                find_in_range_list(sector, pos, r, two_d, count, max, out);
            }
            if v != 0 {
                find_in_range_list(rep, pos, r, two_d, count, max, out);
            }
            if p != 0 {
                find_in_range_list(rep.byte_add(4), pos, r, two_d, count, max, out);
            }
            if o != 0 {
                find_in_range_list(rep.byte_add(8), pos, r, two_d, count, max, out);
            }
            if d != 0 {
                find_in_range_list(sector.byte_add(4), pos, r, two_d, count, max, out);
            }
        }
    }
}

/// `CWorld::FindObjectsOfTypeInRange(uint32 model, const CVector&, float, bool, int16*, int16, CEntity**, bool
/// buildings, bool vehicles, bool peds, bool objects, bool dummies)` (0x564C70, cdecl): as `FindObjectsInRange`
/// with the model filter. Kept original detail: the last x sector index is stored in the caller's pos argument
/// slot (not modelled).
#[hook(addr = 0x564C70, conv = cdecl, name = "CWorld::FindObjectsOfTypeInRange", expect = "83 EC 18 53 55 56 57",
       unverified = "mvpb-world-1", route = "L2", hazards = "none",
       note = "overwrites its own (caller-owned) pos argument slot with the end x sector")]
#[allow(clippy::too_many_arguments)]
pub fn world_find_objects_of_type_in_range(
    model: i32,
    pos: GPtr<u8>,
    r: f32,
    two_d: u8,
    count: GPtr<i16>,
    max: i16,
    out: GPtr<GPtr<u8>>,
    b: u8,
    v: u8,
    p: u8,
    o: u8,
    d: u8,
) {
    let (x0, y0, x1, y1) = scan_rect(pos, r);
    count.write(0);
    for ys in y0..=y1 {
        for xs in x0..=x1 {
            let (sector, rep) = cell(xs, ys);
            if b != 0 {
                find_of_type_in_range_list(model, sector, pos, r, two_d, count, max, out);
            }
            if v != 0 {
                find_of_type_in_range_list(model, rep, pos, r, two_d, count, max, out);
            }
            if p != 0 {
                find_of_type_in_range_list(model, rep.byte_add(4), pos, r, two_d, count, max, out);
            }
            if o != 0 {
                find_of_type_in_range_list(model, rep.byte_add(8), pos, r, two_d, count, max, out);
            }
            if d != 0 {
                find_of_type_in_range_list(model, sector.byte_add(4), pos, r, two_d, count, max, out);
            }
        }
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ENTITY_UPDATE_RW_MATRIX,
    HOOK_ENTITY_RENDER_EFFECTS,
    HOOK_ENTITY_RENDER,
    HOOK_ENTITY_DTOR,
    HOOK_REPLAY_RECORD_PED_DELETED,
    HOOK_REPLAY_RECORD_VEHICLE_DELETED,
    HOOK_REPLAY_MAKE_BMX_UPDATE_DATA,
    HOOK_REPLAY_MAKE_BIKE_UPDATE_DATA,
    HOOK_REPLAY_STORE_CLOTHES_DESC,
    HOOK_GARAGE_IS_PLAYER_ENTIRELY_OUTSIDE,
    HOOK_GARAGE_DIST_SQ_TO_BOX,
    HOOK_WORLD_FIND_OBJECTS_IN_RANGE,
    HOOK_WORLD_FIND_OBJECTS_OF_TYPE_IN_RANGE,
];
