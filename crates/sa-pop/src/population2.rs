//! `CPopulation` drivers and `CPopCycle` (PORT-POP-2): population update, management, ped creation, file loaders. Ported from the 1.0 US bytes;
//! gta-reversed and `docs/re/specs/POP/*.md` as reading aids.
//!
//! Pool walks follow the original: the pool pointer (and, where the original caches it, its size) is read
//! once; the byte map and the object array are re-read on every slot. Distances are computed in the x87 order
//! of each function (`dist_zyx`: `sqrt((dz*dz + dy*dy) + dx*dx)`).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{find_player_ped, ftol32, g, jne_mask, jp5, model_info, rd, wr, x};
use crate::popcycle::{PERC_ROWS, find_player_centre_of_world};
use crate::population::{CURRENT_WORLD_ZONE, PED_COUNTS, delete_entity, world_remove};

sa_core::globals! {
    list GLOBALS;
    /// `CPopCycle::m_NumOther_Cars` 0xC0BC30, `m_NumCops_Cars` +4, `m_NumGangs_Cars` +8, `m_NumDealers_Cars` +0xC,
    /// `m_NumOther_Peds` +0x10, `m_NumCops_Peds` +0x14, `m_NumGangs_Peds` +0x18, `m_fPercOther` +0x1C,
    /// `m_fPercCops` +0x20, `m_fPercGangs` +0x24, `m_fPercDealers` +0x28
    pub static POPCYCLE_NUMS @ 0xC0BC30: [f32; 11] = [0.0; 11];
    /// `CPopCycle::m_nPercCops` 0xC0E018, `m_nPercGang` 0xC0E1F8, `m_nPercDealers` 0xC0E3D8, `m_nMaxNumCars`
    /// 0xC0E5B8, `m_nMaxNumPeds` 0xC0E798 (each `uint8[12][2][20]`)
    pub static PERC_TABLES @ 0xC0E018: [u8; 5 * PERC_ROWS] = [0; 5 * PERC_ROWS];
    /// `CPopCycle::m_NumDealers_Peds`
    pub static NUM_DEALERS_PEDS @ 0xC0E978: f32 = 0.0;
}

/// `CTimer::m_snTimeInMilliseconds`
fn time_ms() -> u32 {
    sa_core::timer::TIME_MS.get()
}

/// `CWorld::PlayerInFocus`
fn player_in_focus() -> i32 {
    i32::from(sa_world::world::cworld::PLAYER_IN_FOCUS.get())
}

/// `CGame::currArea` (0xB72914)
fn curr_area() -> i32 {
    sa_core::game::CURR_AREA.get()
}

/// `TheCamera.m_fGenerationDistMultiplier` (0xB6F11C)
fn cam_gen_dist_multiplier() -> f32 {
    g::<f32>(0xB6F11C).read()
}

/// A `ModelIndices` cell (`MI_*`, u16 zero-extended).
fn mi_cell(addr: u32) -> i32 {
    i32::from(sa_stream::modelinfo::model_indices::cell(addr).read())
}

/// The `CPopulation` counter at 1.0 US address `addr` (0xC0EC24..0xC0EC70).
fn counter(addr: usize) -> GPtr<u32> {
    PED_COUNTS.ptr().cast::<u8>().byte_add((addr - 0xC0EC24) as isize).cast()
}

/// `CPlaceable::GetPosition()`: matrix +0x30 when the entity has a matrix, else the placement at +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m: GPtr<u8> = rd(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn sub3(a: GPtr<u8>, b: GPtr<u8>) -> (X87, X87, X87) {
    (
        x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0)),
        x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4)),
        x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8)),
    )
}

/// `|a - b|` as `sqrt((dz*dz + dy*dy) + dx*dx)`.
fn dist_zyx(a: GPtr<u8>, b: GPtr<u8>) -> X87 {
    let (dx, dy, dz) = sub3(a, b);
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// The pool header pointer stored at `cell` (`CPools::ms_p*Pool`).
fn pool_at(cell: usize) -> GPtr<u8> {
    g::<GPtr<u8>>(cell).read()
}

const PED_POOL: usize = 0xB74490;
const OBJECT_POOL: usize = 0xB7449C;
const DUMMY_POOL: usize = 0xB744A0;

/// Slot `i` of `pool` (its byte map and objects re-read), or null when free.
fn pool_slot(pool: GPtr<u8>, i: i32, stride: i32) -> GPtr<u8> {
    let map: GPtr<u8> = rd(pool, 4);
    if (map.wrapping_index(i).read() as i8) < 0 {
        return GPtr::NULL;
    }
    let objs: GPtr<u8> = rd(pool, 0);
    objs.byte_add(i.wrapping_mul(stride) as isize)
}

orig! {
    /// `CPopulation::ConvertToRealObject(CDummyObject*)` (0x614580)
    pub fn convert_to_real_object_call(d: GPtr<u8>) = cdecl 0x614580 => convert_to_real_object;
    /// `CPopulation::ConvertToDummyObject(CObject*)` (0x614670)
    pub fn convert_to_dummy_object_call(o: GPtr<u8>) = cdecl 0x614670 => convert_to_dummy_object;
    /// `CPopulation::TestSafeForRealObject(CDummyObject*)` (0x6123A0)
    pub fn test_safe_for_real_object_call(d: GPtr<u8>) -> u8 = cdecl 0x6123A0 => test_safe_for_real_object;
    /// `CPopulation::TestRoomForDummyObject(CObject*)` (0x612320)
    pub fn test_room_for_dummy_object_call(o: GPtr<u8>) -> u8 = cdecl 0x612320 => test_room_for_dummy_object;
    /// `CPopulation::ManageObject(CObject*, const CVector&)` (0x615DC0)
    pub fn manage_object_call(o: GPtr<u8>, c: GPtr<u8>) = cdecl 0x615DC0 => manage_object;
    /// `CPopulation::ManageDummy(CDummy*, const CVector&)` (0x616000)
    pub fn manage_dummy_call(d: GPtr<u8>, c: GPtr<u8>) = cdecl 0x616000 => manage_dummy;
    /// `CPopulation::ManagePed(CPed*, const CVector&)` (0x611FC0)
    pub fn manage_ped_call(p: GPtr<u8>, c: GPtr<u8>) = cdecl 0x611FC0 => manage_ped;
    /// `CPopulation::ManagePopulation()` (0x616190)
    pub fn manage_population_call() = cdecl 0x616190 => manage_population;
    /// `CPopulation::RemovePedsIfThePoolGetsFull()` (0x616300)
    pub fn remove_peds_if_the_pool_gets_full_call() = cdecl 0x616300 => remove_peds_if_the_pool_gets_full;
    /// `CPopulation::GeneratePedsAtStartOfGame()` (0x615C90)
    pub fn generate_peds_at_start_of_game_call() = cdecl 0x615C90 => generate_peds_at_start_of_game;
    /// `CPopulation::RemovePed(CPed*)` (0x610F20)
    pub fn remove_ped_call(p: GPtr<u8>) = cdecl 0x610F20 => crate::population::remove_ped;
    /// `CPopulation::PedCreationDistMultiplier()` (0x6116C0), ST0 as f32 (exact: a value in [1, 1.5])
    pub fn ped_creation_dist_multiplier_call() -> f32 = cdecl 0x6116C0 => pcdm_f32;
    /// `CPopulation::FindDummyDistForModel(int32)` (0x610ED0), ST0 as f32 (exact: 80 or 750)
    pub fn find_dummy_dist_for_model_call(m: i32) -> f32 = cdecl 0x610ED0 => fddfm_f32;
    /// `CPopulation::AddToPopulation(float, float, float, float)` (0x614720)
    pub fn add_to_population_call(a: f32, b: f32, c: f32, d: f32) -> u8 = cdecl 0x614720 => unported;
    /// `CPopulation::GeneratePedsAtAttractors(CVector, float, float, float, float, int32, int32)` (0x615970)
    pub fn generate_peds_at_attractors_call(px: f32, py: f32, pz: f32, a: f32, b: f32, c: f32, d: f32, dm: i32, n: i32) -> i32 = cdecl 0x615970 => generate_peds_at_attractors;

    /// `CWorld::Add(CEntity*)` (0x563220)
    pub fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CDummyObject::CreateObject()` (0x59EAC0)
    pub fn dummy_create_object(d: GPtr<u8>) -> GPtr<u8> = thiscall 0x59EAC0 => unported;
    /// `CDummyObject::UpdateFromObject(CObject*)` (0x59EB70)
    pub fn dummy_update_from_object(d: GPtr<u8>, o: GPtr<u8>) = thiscall 0x59EB70 => unported;
    /// `CEntity::ResolveReferences()` (0x571A40)
    pub fn entity_resolve_references(e: GPtr<u8>) = thiscall 0x571A40 => unported;
    /// `CObject::SetRelatedDummy(CDummyObject*)` (0x59F160)
    pub fn object_set_related_dummy(o: GPtr<u8>, d: GPtr<u8>) = thiscall 0x59F160 => unported;
    /// `CGlass::IsObjectGlass(CEntity*)` (0x46A760)
    pub fn glass_is_object_glass(e: GPtr<u8>) -> u8 = cdecl 0x46A760 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800)
    pub fn physical_add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// virtual slot 0x10 of a `CEntity`: `SetIsStatic(bool)`
    pub fn entity_set_is_static(e: GPtr<u8>, v: u8) = thiscall vslot 0x10 => unported;
    /// virtual slot 4 of a `CBaseModelInfo`: `AsAtomicModelInfoPtr()`
    pub fn mi_as_atomic(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x04 => unported;
    /// `CEntity::GetBoundCentre(CVector* out)` (0x534250), returns `out`
    pub fn entity_get_bound_centre_vec(e: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x534250 => unported;
    /// `CEntity::GetBoundCentre(CVector& out)` (0x534290)
    pub fn entity_get_bound_centre_ref(e: GPtr<u8>, out: GPtr<f32>) = thiscall 0x534290 => unported;
    /// `CWorld::FindObjectsKindaColliding(const CVector&, float, bool, int16*, int16, CEntity**, bool, bool, bool, bool, bool)` (0x568B80)
    pub fn world_find_objects_kinda_colliding(pos: GPtr<f32>, r: f32, b2d: u8, cnt: GPtr<i16>, max: i32, out: GPtr<u8>, bld: u8, veh: u8, ped: u8, obj: u8, dum: u8) = cdecl 0x568B80 => unported;
    /// `CObject::CanBeDeleted()` (0x59F120)
    pub fn object_can_be_deleted(o: GPtr<u8>) -> u8 = thiscall 0x59F120 => unported;
    /// `CEntity::GetIsOnScreen()` (0x534540)
    pub fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `CEntity::IsVisible()` (0x536BC0)
    pub fn entity_is_visible(e: GPtr<u8>) -> u8 = thiscall 0x536BC0 => unported;
    /// `CVisibilityPlugins::GetClumpAlpha(RpClump*)` (0x732B20)
    pub fn get_clump_alpha(c: GPtr<u8>) -> i32 = cdecl 0x732B20 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    pub fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::CanBeDeleted()` (0x5DF100)
    pub fn ped_can_be_deleted(p: GPtr<u8>) -> u8 = thiscall 0x5DF100 => unported;
    /// `CGangWars::GangWarFightingGoingOn()` (0x443AC0)
    pub fn gang_war_fighting_going_on_call() -> u8 = cdecl 0x443AC0 => crate::gangs::gang_war_fighting_going_on;
    /// `CPedIntelligence::GetTaskHold(bool)` (0x600FF0)
    pub fn intel_get_task_hold(intel: GPtr<u8>, b: u8) -> GPtr<u8> = thiscall 0x600FF0 => unported;
    /// virtual slot 0x10 of a `CTask`: `GetTaskType()`
    pub fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    pub fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    pub fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CEntity::GetIsTouching(const CVector&, float)` (0x5344B0)
    pub fn entity_get_is_touching(e: GPtr<u8>, c: GPtr<f32>, r: f32) -> u8 = thiscall 0x5344B0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    pub fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CCollision::ProcessColModels(...)` (0x4185C0), 8 cdecl arguments
    pub fn process_col_models(ma: GPtr<u8>, ca: GPtr<u8>, mb: GPtr<u8>, cb: GPtr<u8>, pts: GPtr<u8>, lpts: GPtr<u8>, ld: GPtr<u8>, b: u8) -> i32 = cdecl 0x4185C0 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    pub fn request_model(m: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::GetDefaultCabDriverModel()` (0x407D50)
    pub fn get_default_cab_driver_model() -> i32 = cdecl 0x407D50 => unported;
    /// CRT `floor(double)` (0x8219F0)
    pub fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => sa_world::physical::base::floor_f64;
}

/// `PedCreationDistMultiplier` as the f32 the `orig!` declaration returns.
pub fn pcdm_f32() -> f32 {
    crate::population::ped_creation_dist_multiplier().to_f32()
}

/// `FindDummyDistForModel` as the f32 the `orig!` declaration returns.
pub fn fddfm_f32(m: i32) -> f32 {
    crate::population::find_dummy_dist_for_model(m).to_f32()
}

/// The near threshold `ManageDummy` / `ManageObject` compare with: 750.0 for the two model indices at
/// 0x8CD6AC / 0x8CD6B0, else 80.0.
fn dummy_threshold(model: i32) -> X87 {
    if model == mi_cell(0x8CD6AC) || model == mi_cell(0x8CD6B0) {
        x(750.0)
    } else {
        x(80.0)
    }
}

/// `CPopulation::ManageDummy(CDummy*, const CVector& centre)` (0x616000): a visible dummy in the current
/// area (or area 13) nearer than its threshold becomes a real object.
#[hook(addr = 0x616000, conv = cdecl, name = "CPopulation::ManageDummy", expect = "8B 54 24 04 8A 42 2F")]
pub fn manage_dummy(d: GPtr<u8>, centre: GPtr<u8>) {
    let area = rd::<u8>(d, 0x2F);
    if i32::from(area) != curr_area() && area != 0xD {
        return;
    }
    if (rd::<u8>(d, 0x1C) as i8) >= 0 {
        return;
    }
    let dist = dist_zyx(ent_pos(d), centre);
    let model = i32::from(rd::<i16>(d, 0x22));
    if jp5(dist, dummy_threshold(model)) {
        return;
    }
    convert_to_real_object_call(d);
}

/// `CPopulation::TestRoomForDummyObject(CObject*)` (0x612320): no vehicle or ped kinda-colliding with the
/// bounding sphere of the object's model around its dummy's bound centre.
#[hook(addr = 0x612320, conv = cdecl, name = "CPopulation::TestRoomForDummyObject", expect = "83 EC 18 56 8B 74 24 20")]
pub fn test_room_for_dummy_object(obj: GPtr<u8>) -> u8 {
    let mut out = [0f32; 3];
    let r = entity_get_bound_centre_vec(rd(obj, 0x170), GPtr::from_raw(out.as_mut_ptr()));
    let mut pos = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
    let col: GPtr<u8> = rd(model_info(i32::from(rd::<i16>(obj, 0x22))), 0x14);
    let radius = rd::<f32>(col, 0x24);
    let mut n: i16 = 0;
    world_find_objects_kinda_colliding(
        GPtr::from_raw(pos.as_mut_ptr()),
        radius,
        0,
        GPtr::from_raw(&mut n),
        2,
        GPtr::NULL,
        0,
        1,
        1,
        0,
        0,
    );
    u8::from(n == 0)
}

/// `_ftol(floor((double)(v * 0.02f + 60.0f)))` of an x87 value (the world sector of a coordinate).
fn sector(v: X87) -> i32 {
    let s = v.fmul_m(f32::from_bits(0x3CA3_D70A)).fadd_m(60.0);
    ftol32(X87::from_f64_exact(crt_floor(s.to_f64())))
}

/// `CPopulation::TestSafeForRealObject(CDummyObject*)` (0x6123A0): false when an entity of the
/// repeat-sector vehicle lists around the dummy's bounding sphere (not yet scanned this pass) touches it and
/// its col model collides with the dummy's (`ProcessColModels > 0`). The sector bounds are clamped to
/// [0, 120] (each one computed twice when in range, as the original).
#[hook(addr = 0x6123A0, conv = cdecl, name = "CPopulation::TestSafeForRealObject", expect = "83 EC 2C 53 55")]
pub fn test_safe_for_real_object(d: GPtr<u8>) -> u8 {
    let col: GPtr<u8> = rd(model_info(i32::from(rd::<i16>(d, 0x22))), 0x14);
    let lo = |k: isize| x(rd::<f32>(ent_pos(d), k)).fsub_m(rd::<f32>(col, 0x24));
    let x0 = if sector(lo(0)) > 0 { sector(lo(0)) } else { 0 };
    let y0 = if sector(lo(4)) > 0 { sector(lo(4)) } else { 0 };
    let x1 = {
        let h = || x(rd::<f32>(col, 0x24)).fadd_m(rd::<f32>(ent_pos(d), 0));
        if sector(h()) < 120 { sector(h()) } else { 120 }
    };
    let y1 = {
        let h = || x(rd::<f32>(ent_pos(d), 4)).fadd_m(rd::<f32>(col, 0x24));
        if sector(h()) < 120 { sector(h()) } else { 120 }
    };
    let radius =
        rd::<f32>(rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(d, 0x22))), 0x14), 0x24);
    let mut centre = [0f32; 3];
    entity_get_bound_centre_ref(d, GPtr::from_raw(centre.as_mut_ptr()));
    // the static-local guard of the CColPoint array at 0xC0FD00
    let guard = g::<u32>(0xC10280);
    if guard.cast::<u8>().read() & 1 == 0 {
        guard.write(guard.read() | 1);
    }
    let scan = g::<u16>(0xB7CD78);
    let mut y = y0;
    while y <= y1 {
        let mut xs = x0;
        while xs <= x1 {
            let idx = ((xs & 0xF) + ((y & 0xF) << 4)).wrapping_mul(3);
            let mut node: GPtr<u8> = g::<GPtr<u8>>(0xB992B8).wrapping_index(idx).read();
            while !node.is_null() {
                let e: GPtr<u8> = rd(node, 0);
                node = rd(node, 4);
                if rd::<u16>(e, 0x2C) == scan.read() {
                    continue;
                }
                if entity_get_is_touching(e, GPtr::from_raw(centre.as_mut_ptr()), radius) == 0 {
                    continue;
                }
                wr(e, 0x2C, scan.read());
                if rd::<GPtr<u8>>(e, 0x14).is_null() {
                    placeable_allocate_matrix(e);
                    simple_transform_update_matrix(e.byte_add(4), rd(e, 0x14));
                }
                let mb: GPtr<u8> = rd(e, 0x14);
                if rd::<GPtr<u8>>(d, 0x14).is_null() {
                    placeable_allocate_matrix(d);
                    simple_transform_update_matrix(d.byte_add(4), rd(d, 0x14));
                }
                let ma: GPtr<u8> = rd(d, 0x14);
                let cb = entity_get_col_model(e);
                if process_col_models(ma, col, mb, cb, GPtr::from_addr(0xC0FD00), GPtr::NULL, GPtr::NULL, 0) > 0 {
                    return 0;
                }
            }
            xs += 1;
        }
        y += 1;
    }
    1
}

/// `CPopulation::ConvertToRealObject(CDummyObject*)` (0x614580).
#[hook(addr = 0x614580, conv = cdecl, name = "CPopulation::ConvertToRealObject", expect = "83 EC 0C 57 8B 7C 24 14")]
pub fn convert_to_real_object(d: GPtr<u8>) {
    if test_safe_for_real_object_call(d) == 0 {
        return;
    }
    let obj = dummy_create_object(d);
    if obj.is_null() {
        return;
    }
    world_remove(d);
    wr(d, 0x1C, rd::<u32>(d, 0x1C) & 0xFFFF_FF7F);
    entity_resolve_references(d);
    object_set_related_dummy(obj, d);
    world_add(obj);
    let glass = glass_is_object_glass(obj) != 0;
    if glass {
        let mi = model_info(i32::from(rd::<i16>(obj, 0x22)));
        if u32::from(rd::<u16>(mi, 0x12)) & 0x7800 != 0x2800 {
            wr(obj, 0x1C, rd::<u32>(obj, 0x1C) & 0xFFFF_FF7F);
            return;
        }
    }
    let model = i32::from(rd::<i16>(obj, 0x22));
    if model != mi_cell(0x8CD608) && (rd::<u32>(obj, 0x40) >> 25) & 1 == 0 {
        return;
    }
    entity_set_is_static(obj, 0);
    wr::<u32>(obj, 0x44, 0);
    wr::<u32>(obj, 0x48, 0);
    wr::<u32>(obj, 0x4C, 0xBA83_126F);
    wr(obj, 0x40, rd::<u32>(obj, 0x40) | 0x0800_0000);
    physical_add_to_moving_list(obj);
}

/// `CPopulation::ConvertToDummyObject(CObject*)` (0x614670). The `(flags & 0x7800)` test that clears the
/// dummy's visibility runs without a dummy check, as the original (it would fault on a null dummy).
#[hook(addr = 0x614670, conv = cdecl, name = "CPopulation::ConvertToDummyObject", expect = "56 8B 74 24 08")]
pub fn convert_to_dummy_object(obj: GPtr<u8>) {
    let d: GPtr<u8> = rd(obj, 0x170);
    if !d.is_null() {
        if test_room_for_dummy_object_call(obj) == 0 {
            return;
        }
        wr(d, 0x1C, rd::<u32>(d, 0x1C) | 0x80);
        dummy_update_from_object(d, obj);
    }
    if rd::<u8>(obj, 0x36) & 7 == 4 {
        let a = mi_as_atomic(model_info(i32::from(rd::<i16>(obj, 0x22))));
        if !a.is_null() {
            let f = rd::<u16>(a, 0x12) & 0x7800;
            if f == 0x2000 || f == 0x2800 {
                let mi = model_info(i32::from(rd::<i16>(obj, 0x22)));
                if u32::from(rd::<u16>(mi, 0x12)) & 0x7800 != 0x2800 {
                    wr(d, 0x1C, rd::<u32>(d, 0x1C) & 0xFFFF_FF7F);
                }
            }
        }
    }
    world_remove(obj);
    delete_entity(obj, 1);
    if !d.is_null() {
        world_add(d);
    }
}

/// `CWorld::Remove` then the scalar deleting destructor.
fn delete(e: GPtr<u8>) {
    world_remove(e);
    delete_entity(e, 1);
}

/// `CPopulation::ManageObject(CObject*, const CVector& centre)` (0x615DC0).
#[hook(addr = 0x615DC0, conv = cdecl, name = "CPopulation::ManageObject", expect = "51 56 8B 74 24 0C")]
pub fn manage_object(obj: GPtr<u8>, centre: GPtr<u8>) {
    if object_can_be_deleted(obj) == 0 {
        return;
    }
    let dist = dist_zyx(ent_pos(obj), centre);
    if rd::<u8>(obj, 0x13C) == 3 {
        let m = i32::from(rd::<i16>(obj, 0x22));
        if m == mi_cell(0x8CD57C) || m == mi_cell(0x8CD580) || m == mi_cell(0x8CD584) || m == mi_cell(0x8CD638) {
            // fcomp 120.0; test ah, 0x41; jne keep
            if !jne_mask(dist, x(120.0), 0x41) {
                delete(obj);
            }
            return;
        }
        let mu = m as u32;
        if mu >= mi_cell(0x8CD68C) as u32 && mu <= mi_cell(0x8CD698) as u32 {
            if jne_mask(dist, x(64.5), 0x41) {
                if jne_mask(dist, x(35.0), 0x41) {
                    return;
                }
                if entity_get_is_on_screen(obj) != 0 {
                    return;
                }
            }
            delete(obj);
            return;
        }
        if !jne_mask(dist, x(54.5), 0x41) {
            delete(obj);
            return;
        }
        if !jne_mask(dist, x(25.0), 0x41) && entity_get_is_on_screen(obj) == 0 {
            delete(obj);
            return;
        }
        if time_ms() > rd::<u32>(obj, 0x150) {
            delete(obj);
            return;
        }
        let rw: GPtr<u8> = rd(obj, 0x18);
        if rw.is_null() || rw.read() != 2 || rd::<u32>(obj, 0x140) & 0x40_0000 == 0 {
            return;
        }
        if get_clump_alpha(rw) != 0 && entity_is_visible(obj) != 0 {
            return;
        }
        delete(obj);
        return;
    }
    let dd: GPtr<u8> = rd(obj, 0x170);
    let dummy_dist = if dd.is_null() { 100000.0 } else { dist_zyx(ent_pos(dd), centre).to_f32() };
    let model = i32::from(rd::<i16>(obj, 0x22));
    // fcomp st(1) with the threshold; test ah, 0x41; jne keep
    if jne_mask(dist, dummy_threshold(model), 0x41) {
        return;
    }
    let v = find_dummy_dist_for_model_call(model);
    if jp5(x(v), x(dummy_dist)) {
        return;
    }
    convert_to_dummy_object_call(obj);
}

/// `CPopulation::ManagePed(CPed*, const CVector& centre)` (0x611FC0).
#[hook(addr = 0x611FC0, conv = cdecl, name = "CPopulation::ManagePed", expect = "51 56 8B 74 24 0C")]
pub fn manage_ped(ped: GPtr<u8>, centre: GPtr<u8>) {
    if ped_is_player(ped) != 0 || ped_can_be_deleted(ped) == 0 {
        return;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return;
    }
    let att: GPtr<u8> = rd(ped, 0xFC);
    if !att.is_null() && rd::<u8>(att, 0x36) & 7 == 2 {
        return;
    }
    let age = time_ms().wrapping_sub(rd::<u32>(ped, 0x750)) as i32;
    if rd::<i32>(ped, 0x530) == 0x37 {
        // FrenzyOnGoing is called whenever age <= 30000; GangWarFightingGoingOn only past the frenzy test
        let set = age > 30000
            || (crate::misc::frenzy_on_going_call() != 0 && age > 15000)
            || (age > 8000 && gang_war_fighting_going_on_call() != 0);
        if set {
            wr(ped, 0x470, rd::<u32>(ped, 0x470) | 8);
        }
    }
    if rd::<u8>(ped, 0x470) & 8 != 0 && get_clump_alpha(rd(ped, 0x18)) == 0 {
        remove_ped_call(ped);
        return;
    }
    let p = ent_pos(ped);
    let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(centre, 0));
    let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(centre, 4));
    let ty = rd::<i32>(ped, 0x598);
    let mut d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().fmul_m(rd::<f32>(ped, 0x790)).to_f32();
    if (7..=16).contains(&ty) {
        d = x(d).fsub_m(30.0).to_f32();
    } else if rd::<u32>(ped, 0x470) & 0x40_0000 != 0 && !rd::<GPtr<u8>>(ped, 0x590).is_null() {
        d = 0.0;
    }
    let mut far = false;
    let m = x(ped_creation_dist_multiplier_call()).fmul_m(cam_gen_dist_multiplier()).fmul_m(65.0);
    if !jp5(m, x(d)) {
        far = true;
    } else if rd::<u8>(ped, 0x470) & 0x40 == 0 {
        let m = x(ped_creation_dist_multiplier_call()).fmul_m(cam_gen_dist_multiplier()).fmul_m(54.5);
        if !jp5(m, x(d)) {
            far = true;
        }
    }
    if !far {
        let m = x(ped_creation_dist_multiplier_call()).fmul_m(25.0);
        if jp5(m, x(d)) {
            let t = time_ms();
            let add = if rd::<i32>(ped, 0x598) == 6 { 10000 } else { 4000 };
            wr(ped, 0x54C, t.wrapping_add(add));
            return;
        }
        let vis = entity_get_is_on_screen(ped);
        let t = time_ms();
        if t <= rd::<u32>(ped, 0x54C) {
            if vis == 0 {
                return;
            }
        } else if vis == 0 {
            let cam = u32::from(g::<u8>(0xB6F081).read()).wrapping_mul(0x238) as usize;
            let mode = g::<u16>(0xB6F1A8 + cam).read();
            if mode == 7 || mode == 0x27 || mode == 0x2E {
                return;
            }
            if g::<u8>(0xB6F1A4 + cam).read() != 0
                || g::<u8>(0xB6F1A5 + cam).read() != 0
                || g::<u8>(0xB6F1A3 + cam).read() != 0
            {
                return;
            }
            far = true;
        }
        if !far {
            let add = if rd::<i32>(ped, 0x598) == 6 { 10000 } else { 4000 };
            wr(ped, 0x54C, t.wrapping_add(add));
            return;
        }
    }
    if entity_get_is_on_screen(ped) == 0 {
        remove_ped_call(ped);
        return;
    }
    wr(ped, 0x470, rd::<u32>(ped, 0x470) | 8);
}

/// `CPopulation::ManageAllPopulation()` (0x6160A0): every object (descending, except the one the player
/// holds) then every dummy (descending) against the player's centre of world.
#[hook(addr = 0x6160A0, conv = cdecl, name = "CPopulation::ManageAllPopulation", expect = "83 EC 0C 0F B6 05 74 CD B7 00")]
pub fn manage_all_population() {
    let c = find_player_centre_of_world(player_in_focus());
    let mut centre = [c.cast::<u32>().read(), c.cast::<u32>().wrapping_index(1).read(), c.cast::<u32>().wrapping_index(2).read()];
    let cp: GPtr<u8> = GPtr::from_raw(centre.as_mut_ptr()).cast();
    let mut held = GPtr::<u8>::NULL;
    let t = intel_get_task_hold(rd(find_player_ped(0), 0x47C), 0);
    if !t.is_null() && task_get_type(t) == 0x133 {
        let e: GPtr<u8> = rd(t, 8);
        if rd::<u8>(e, 0x36) & 7 == 4 {
            held = e;
        }
    }
    let pool = pool_at(OBJECT_POOL);
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        let o = pool_slot(pool, i, 0x19C);
        if !o.is_null() && o != held {
            manage_object_call(o, cp);
        }
    }
    let pool = pool_at(DUMMY_POOL);
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        let d = pool_slot(pool, i, 0x38);
        if !d.is_null() {
            manage_dummy_call(d, cp);
        }
    }
}

/// `CPopulation::ManagePopulation()` (0x616190): this frame's 1/32 batch of the object and dummy pools
/// (ascending, signed `/ 32` bounds), then every ped (descending; the RE-SPEC-3 pool-order deviation).
#[hook(addr = 0x616190, conv = cdecl, name = "CPopulation::ManagePopulation", expect = "83 EC 1C A1 90 44 B7 00")]
pub fn manage_population() {
    let peds = pool_at(PED_POOL);
    let dummies = pool_at(DUMMY_POOL);
    let objects = pool_at(OBJECT_POOL);
    let b = (sa_core::timer::FRAME_COUNTER.get() & 0x1F) as i32;
    let c = find_player_centre_of_world(player_in_focus());
    let mut centre = [c.cast::<u32>().read(), c.cast::<u32>().wrapping_index(1).read(), c.cast::<u32>().wrapping_index(2).read()];
    let cp: GPtr<u8> = GPtr::from_raw(centre.as_mut_ptr()).cast();
    let n = rd::<i32>(objects, 8);
    let (lo, hi) = (b.wrapping_mul(n) / 32, (b + 1).wrapping_mul(n) / 32);
    for i in lo..hi {
        let o = pool_slot(objects, i, 0x19C);
        if !o.is_null() {
            manage_object_call(o, cp);
        }
    }
    let n = rd::<i32>(dummies, 8);
    let (lo, hi) = (b.wrapping_mul(n) / 32, (b + 1).wrapping_mul(n) / 32);
    for i in lo..hi {
        let d = pool_slot(dummies, i, 0x38);
        if !d.is_null() {
            manage_dummy_call(d, cp);
        }
    }
    let mut i = rd::<i32>(peds, 8);
    while i != 0 {
        i -= 1;
        let p = pool_slot(peds, i, 0x7C4);
        if !p.is_null() {
            manage_ped_call(p, cp);
        }
    }
}

/// `CPopulation::RemovePedsIfThePoolGetsFull()` (0x616300): every 8th frame (`& 7 == 5`), with fewer than 8
/// free ped slots, delete the deletable ped nearest the camera (`sqrt((dz*dz + dx*dx) + dy*dy)`, strict `<`
/// over a descending walk: ties keep the highest index; RE-SPEC-3).
#[hook(addr = 0x616300, conv = cdecl, name = "CPopulation::RemovePedsIfThePoolGetsFull", expect = "A1 4C CB B7 00")]
pub fn remove_peds_if_the_pool_gets_full() {
    if sa_core::timer::FRAME_COUNTER.get() & 7 != 5 {
        return;
    }
    let pool = pool_at(PED_POOL);
    let n = rd::<i32>(pool, 8);
    let mut used = 0;
    let map: GPtr<u8> = rd(pool, 4);
    for i in 0..n.max(0) {
        if (map.wrapping_index(i).read() as i8) >= 0 {
            used += 1;
        }
    }
    if n - used >= 8 {
        return;
    }
    let mut best = f32::from_bits(0x4B18_9680);
    let mut cand = GPtr::<u8>::NULL;
    let mut i = n;
    while i != 0 {
        i -= 1;
        let p = pool_slot(pool, i, 0x7C4);
        if p.is_null() || ped_can_be_deleted(p) == 0 {
            continue;
        }
        let pp = ent_pos(p);
        let cam = sa_world::world::common::cam_pos().cast::<u8>();
        let (dx, dy, dz) = sub3(cam, pp);
        let d = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).fsqrt();
        if !jp5(d, x(best)) {
            best = d.to_f32();
            cand = p;
        }
    }
    if !cand.is_null() {
        delete(cand);
    }
}

/// `CPopulation::ConvertAllObjectsToDummyObjects()` (0x616420): every game object (`+0x13C == 1`), descending.
#[hook(addr = 0x616420, conv = cdecl, name = "CPopulation::ConvertAllObjectsToDummyObjects", expect = "8B 0D 9C 44 B7 00")]
pub fn convert_all_objects_to_dummy_objects() {
    let mut pool = pool_at(OBJECT_POOL);
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        let o = pool_slot(pool, i, 0x19C);
        if !o.is_null() && rd::<u8>(o, 0x13C) == 1 {
            convert_to_dummy_object_call(o);
            pool = pool_at(OBJECT_POOL);
        }
    }
}

/// The ped totals: civilians (female + male), gangs (sum of the 10), and the total `extra + cop + gang + civ`.
fn update_totals(extra: u32) {
    let civ = counter(0xC0EC6C).read().wrapping_add(counter(0xC0EC70).read());
    let mut gang = 0u32;
    for k in 0..10 {
        gang = gang.wrapping_add(counter(0xC0EC3C + 4 * k).read());
    }
    let total = extra.wrapping_add(counter(0xC0EC68).read()).wrapping_add(gang).wrapping_add(civ);
    counter(0xC0EC30).write(civ);
    counter(0xC0EC2C).write(gang);
    counter(0xC0EC28).write(total);
}

/// `PedCreationDistMultiplier() * k` stored as an f32 argument.
fn pcdm_times(k: f32) -> f32 {
    x(ped_creation_dist_multiplier_call()).fmul_m(k).to_f32()
}

/// `CPopulation::GeneratePedsAtStartOfGame()` (0x615C90): 100 rounds of the totals (here the total is
/// `emergency - car passengers + cop + gang + civ`) and `AddToPopulation(10, m*50.5, 10, m*50.5)`, then
/// `GeneratePedsAtAttractors` around the centre of world.
#[hook(addr = 0x615C90, conv = cdecl, name = "CPopulation::GeneratePedsAtStartOfGame", expect = "56 57 BE 64 00 00 00")]
pub fn generate_peds_at_start_of_game() {
    for _ in 0..100 {
        update_totals(counter(0xC0EC64).read().wrapping_sub(counter(0xC0EC34).read()));
        let a = pcdm_times(50.5);
        let b = pcdm_times(50.5);
        add_to_population_call(10.0, b, 10.0, a);
    }
    let c = find_player_centre_of_world(player_in_focus());
    let a = pcdm_times(50.5);
    let b = pcdm_times(50.5);
    let p = c.cast::<f32>();
    generate_peds_at_attractors_call(p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read(), 10.0, b, 10.0, a, -1, 1);
}

/// The four `AddToPopulation` / `GeneratePedsAtAttractors` radii of `Update`, in call order:
/// `m*25`, `m*25 - 10`, `m*cam*50.5`, `m*cam*42.5`.
fn update_radii() -> (f32, f32, f32, f32) {
    let a = pcdm_times(25.0);
    let b = x(ped_creation_dist_multiplier_call()).fmul_m(25.0).fsub_m(10.0).to_f32();
    let c = x(ped_creation_dist_multiplier_call()).fmul_m(cam_gen_dist_multiplier()).fmul_m(50.5).to_f32();
    let d = x(ped_creation_dist_multiplier_call()).fmul_m(cam_gen_dist_multiplier()).fmul_m(42.5).to_f32();
    (a, b, c, d)
}

/// `CPopulation::Update(bool generate)` (0x616650): world zone from the weather region (other values keep
/// it), then (not during a replay) manage, prune, the start countdown or the totals and the generation.
#[hook(addr = 0x616650, conv = cdecl, name = "CPopulation::Update", expect = "0F BF 05 14 13 C8 00")]
pub fn update(generate: u8) {
    match g::<i16>(0xC81314).read() {
        0 | 1 | 4 => CURRENT_WORLD_ZONE.set(0),
        2 => CURRENT_WORLD_ZONE.set(1),
        3 => CURRENT_WORLD_ZONE.set(2),
        _ => {}
    }
    if g::<u8>(0xA43088).read() == 1 {
        return;
    }
    manage_population_call();
    remove_peds_if_the_pool_gets_full_call();
    let cd = crate::population::COUNT_DOWN_TO_PEDS_AT_START.get();
    if cd != 0 {
        crate::population::COUNT_DOWN_TO_PEDS_AT_START.set(cd - 1);
        if cd - 1 == 0 {
            generate_peds_at_start_of_game_call();
        }
        return;
    }
    update_totals(counter(0xC0EC64).read());
    if g::<u8>(0xB5F852).read() != 0 || generate == 0 {
        return;
    }
    let (a, b, c, d) = update_radii();
    if add_to_population_call(d, c, b, a) == 0 {
        return;
    }
    let cw = find_player_centre_of_world(player_in_focus());
    let dm = i32::from(curr_area() != 0) * 8 - 1;
    let (a, b, c, d) = update_radii();
    let p = cw.cast::<f32>();
    generate_peds_at_attractors_call(p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read(), d, c, b, a, dm, 1);
}

/// `CPopulation::LoadSpecificDriverModelsForCar(int32 carModel)` (0x6117F0): requests the drivers of the
/// special vehicles (jump table 0x611888 / byte table 0x6118A8, flags 10).
#[hook(addr = 0x6117F0, conv = cdecl, name = "CPopulation::LoadSpecificDriverModelsForCar", expect = "8B 44 24 04 05 67 FE FF FF")]
pub fn load_specific_driver_models_for_car(car: i32) {
    match car {
        409 => request_model(0xFF, 10),
        420 | 438 => {
            let m = get_default_cab_driver_model();
            request_model(m, 10);
        }
        423 => request_model(0x108, 10),
        428 => request_model(0x47, 10),
        448 => request_model(0x9B, 10),
        463 => {
            request_model(0xF7, 10);
            request_model(0xF8, 10);
        }
        481 => request_model(0x17, 10),
        _ => {}
    }
}

/// A string of the exe's `.rdata` (pushed as an immediate address by the original).
fn s(addr: usize) -> GPtr<u8> {
    GPtr::from_addr(addr)
}

orig! {
    /// `CFileMgr::ChangeDir(const char*)` (0x538730)
    pub fn change_dir(p: GPtr<u8>) -> i32 = cdecl 0x538730 => sa_core::filemgr::change_dir;
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    pub fn set_dir(p: GPtr<u8>) -> i32 = cdecl 0x5387D0 => sa_core::filemgr::set_dir;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    pub fn load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// `CModelInfo::GetModelInfo(const char*, int32*)` (0x4C5940)
    pub fn get_model_info(name: GPtr<u8>, idx: GPtr<i32>) -> GPtr<u8> = cdecl 0x4C5940 => unported;
    /// CRT `sscanf` (0x8220AD) with the 24 int outputs of `CPopCycle::Initialise`
    pub fn sscanf24(
        l: GPtr<u8>, fmt: GPtr<u8>,
        a0: GPtr<i32>, a1: GPtr<i32>, a2: GPtr<i32>, a3: GPtr<i32>, a4: GPtr<i32>, a5: GPtr<i32>,
        a6: GPtr<i32>, a7: GPtr<i32>, a8: GPtr<i32>, a9: GPtr<i32>, a10: GPtr<i32>, a11: GPtr<i32>,
        a12: GPtr<i32>, a13: GPtr<i32>, a14: GPtr<i32>, a15: GPtr<i32>, a16: GPtr<i32>, a17: GPtr<i32>,
        a18: GPtr<i32>, a19: GPtr<i32>, a20: GPtr<i32>, a21: GPtr<i32>, a22: GPtr<i32>, a23: GPtr<i32>
    ) -> i32 = cdecl 0x8220AD => unported;
    /// `CGameLogic::LaRiotsActiveHere()` (0x441C10)
    pub fn la_riots_active_here() -> u8 = cdecl 0x441C10 => unported;
    /// `CPopCycle::GetCurrentPercOther_Peds()` (0x610310)
    pub fn get_current_perc_other_peds_call() -> i32 = cdecl 0x610310 => crate::popcycle::get_current_perc_other_peds;
}

use sa_stream::fileloader::call::{close_file, open_file, read_line, strncpy};

/// `LoadPedGroups` / `LoadCarGroups`: one group per data line (`,` and `\r` become spaces; tokens up to
/// `#`, NUL or `slots` fields), model names resolved with `CModelInfo::GetModelInfo`; unknown names are
/// skipped; the rest of a stored row is filled with 2000. A ped line is stored once it has a token, a car
/// line only when it resolved a model. The line scan for `\n` is bounded by the 0x400 buffer here (the
/// original runs on past it for a last line without a newline).
fn load_groups(file: usize, mut counts: GPtr<u16>, groups: GPtr<u16>, slots: i32, peds: bool) {
    change_dir(s(0x86A91C));
    let f = open_file(s(file), s(0x85A53C));
    change_dir(s(0x859F7C));
    let mut line = [0u8; 0x402];
    let mut name = [0u8; 0x402];
    let mut idx: i32 = 0;
    let mut row = 0i32;
    let lp = GPtr::from_raw(line.as_mut_ptr());
    while read_line(f, lp, 0x400) {
        let mut count = 0i32;
        let mut any = false;
        let mut i = 0usize;
        if line[0] != b'\n' {
            loop {
                if line[i] == b',' || line[i] == b'\r' {
                    line[i] = b' ';
                }
                if i + 1 >= 0x400 {
                    break;
                }
                let c = line[i + 1];
                i += 1;
                if c == b'\n' {
                    break;
                }
            }
        }
        line[i] = 0;
        let mut pos = 0usize;
        let mut fields = 0;
        loop {
            if line[pos] <= 0x20 {
                while line[pos] != 0 {
                    let c = line[pos + 1];
                    pos += 1;
                    if c > 0x20 {
                        break;
                    }
                }
            }
            if line[pos] == b'#' || line[pos] <= 0x20 {
                break;
            }
            let mut e = pos;
            loop {
                e += 1;
                if line[e] <= 0x20 {
                    break;
                }
            }
            let len = e - pos;
            strncpy(GPtr::from_raw(name.as_mut_ptr()), lp.byte_add(pos as isize), len as u32);
            name[len] = 0;
            if !get_model_info(GPtr::from_raw(name.as_mut_ptr()), GPtr::from_raw(&mut idx)).is_null() {
                groups.wrapping_index(row + count).write(idx as u16);
                count += 1;
            }
            fields += 1;
            any = true;
            pos = e;
            if fields >= slots {
                break;
            }
        }
        let store = if peds { any } else { count > 0 };
        if store {
            counts.write(count as u16);
            let mut k = count;
            while k < slots {
                groups.wrapping_index(row + k).write(0x7D0);
                k += 1;
            }
            counts = counts.wrapping_index(1);
            row += slots;
        }
    }
    close_file(f);
}

/// `CPopulation::LoadPedGroups()` (0x5BCFE0): `DATA\PEDGRP.DAT`, 21 models per group.
#[hook(addr = 0x5BCFE0, conv = cdecl, name = "CPopulation::LoadPedGroups", expect = "81 EC 10 05 00 00")]
pub fn load_ped_groups() {
    use crate::population::{NUM_PEDS_IN_GROUP, PED_GROUPS};
    load_groups(0x86A910, NUM_PEDS_IN_GROUP.ptr().cast(), PED_GROUPS.ptr().cast(), 21, true);
}

/// `CPopulation::LoadCarGroups()` (0x5BD1A0): `DATA\CARGRP.DAT`, 23 models per group.
#[hook(addr = 0x5BD1A0, conv = cdecl, name = "CPopulation::LoadCarGroups", expect = "81 EC 10 05 00 00")]
pub fn load_car_groups() {
    use crate::population::{CAR_GROUPS, NUM_CARS_IN_GROUP};
    load_groups(0x86A924, NUM_CARS_IN_GROUP.ptr().cast(), CAR_GROUPS.ptr().cast(), 23, false);
}

/// Byte `i` of the `PERC_TABLES` table `t` (0 cops, 1 gang, 2 dealers, 3 max cars, 4 max peds).
fn perc_table(t: i32, i: i32) -> GPtr<u8> {
    PERC_TABLES.ptr().cast::<u8>().wrapping_index(t * PERC_ROWS as i32 + i)
}

/// `CPopCycle::Initialise()` (0x5BC090): `DATA\POPCYCLE.DAT` (see `docs/re/specs/POP/CPopCycle.deviations.md`):
/// 24 `%d` per line (the int locals keep their previous values when sscanf fails; zero before the first
/// line here, uninitialised stack in the original), low bytes stored, every group row rescaled with
/// `_ftol(p * (100.0f / sum))` and the deficit added to the largest entry (last on ties); then the current
/// time / week / zone info / zone type reset to 0, 0, NULL, -1.
#[hook(addr = 0x5BC090, conv = cdecl, name = "CPopCycle::Initialise", expect = "81 EC 84 00 00 00")]
pub fn popcycle_initialise() {
    use crate::popcycle::{
        CURR_ZONE_INFO, CURRENT_TIME_INDEX, CURRENT_TIME_OF_WEEK, CURRENT_ZONE_TYPE, PERC_OTHER, PERC_TYPE_GROUP,
    };
    set_dir(s(0x86A784));
    let f = open_file(s(0x86A7D8), s(0x859C6C));
    set_dir(s(0x858B54));
    let mut v = [0i32; 24];
    for zone in 0..20i32 {
        for week in 0..2i32 {
            for time in 0..12i32 {
                let mut l = load_line(f);
                while !l.is_null() && (l.read() == b'/' || l.read() == 0) {
                    l = load_line(f);
                }
                let a = GPtr::from_raw(v.as_mut_ptr());
                let o = |k: i32| a.wrapping_index(k);
                sscanf24(
                    l,
                    s(0x86A790),
                    o(0),
                    o(1),
                    o(2),
                    o(3),
                    o(4),
                    o(5),
                    o(6),
                    o(7),
                    o(8),
                    o(9),
                    o(10),
                    o(11),
                    o(12),
                    o(13),
                    o(14),
                    o(15),
                    o(16),
                    o(17),
                    o(18),
                    o(19),
                    o(20),
                    o(21),
                    o(22),
                    o(23),
                );
                let u = zone + week * 20 + time * 40;
                perc_table(4, u).write(v[0] as u8);
                perc_table(3, u).write(v[1] as u8);
                perc_table(2, u).write(v[2] as u8);
                perc_table(1, u).write(v[3] as u8);
                perc_table(0, u).write(v[4] as u8);
                PERC_OTHER.ptr().cast::<u8>().wrapping_index(u).write(v[5] as u8);
                let p = PERC_TYPE_GROUP
                    .ptr()
                    .cast::<u8>()
                    .wrapping_index(zone * 18 + week * 360 + time * 720);
                for j in 0..18 {
                    p.wrapping_index(j).write(v[6 + j as usize] as u8);
                }
                let mut sum = 0i32;
                for j in 0..18 {
                    sum = sum.wrapping_add(i32::from(p.wrapping_index(j).read()));
                }
                let k = crate::common::xi(sum).fdivr_m(100.0);
                for j in 0..18 {
                    let q = crate::common::xi(i32::from(p.wrapping_index(j).read())).fmul(k);
                    p.wrapping_index(j).write(ftol32(q) as u8);
                }
                let (mut nsum, mut max, mut best) = (0i32, 0i32, 0i32);
                for j in 0..18 {
                    let b = i32::from(p.wrapping_index(j).read());
                    nsum = nsum.wrapping_add(b);
                    if b >= max {
                        max = b;
                        best = j;
                    }
                }
                let c = 100u8.wrapping_sub(nsum as u8);
                let cell = p.wrapping_index(best);
                cell.write(cell.read().wrapping_add(c));
            }
        }
    }
    CURRENT_TIME_INDEX.set(0);
    CURRENT_TIME_OF_WEEK.set(0);
    CURR_ZONE_INFO.set(GPtr::NULL);
    CURRENT_ZONE_TYPE.set(-1);
    close_file(f);
}

/// `CPopCycle` float `k` of `POPCYCLE_NUMS` (0 other cars .. 10 perc dealers).
fn pn(k: i32) -> GPtr<f32> {
    POPCYCLE_NUMS.ptr().cast::<f32>().wrapping_index(k)
}
const N_OTHER_CARS: i32 = 0;
const N_COPS_CARS: i32 = 1;
const N_GANGS_CARS: i32 = 2;
const N_DEALERS_CARS: i32 = 3;
const N_OTHER_PEDS: i32 = 4;
const N_COPS_PEDS: i32 = 5;
const N_GANGS_PEDS: i32 = 6;
const P_OTHER: i32 = 7;
const P_COPS: i32 = 8;
const P_GANGS: i32 = 9;
const P_DEALERS: i32 = 10;

/// `fld m; fstp m` of a float global (quiets a signalling NaN, as the original's reload does).
fn reload(p: GPtr<f32>) {
    p.write(x(p.read()).to_f32());
}

/// `CPopCycle::UpdatePercentages()` (0x610770): the dealer / gang / cop / other shares of the current
/// zone (renormalised to 1.0 when they exceed it, as the bytes do), the per-zone cop floors (jump table
/// 0x610BC8), and the ped / car numbers from the `uint8` tables; `gfLaRiotsLightMult` (0x8CD060) follows
/// the riots. The riot test compares `1 - m_fDNBalanceParam` (0x8D12C0) with 0.5, as the bytes do.
#[hook(addr = 0x610770, conv = cdecl, name = "CPopCycle::UpdatePercentages", expect = "83 EC 1C 8B 0D 68 BC C0 00")]
pub fn update_percentages() {
    use crate::common::xi;
    let zi = crate::popcycle::CURR_ZONE_INFO.get();
    let k01 = f32::from_bits(0x3C23_D70A);
    let v = xi(i32::from(rd::<u8>(zi, 0xA))).fmul_m(k01);
    if jp5(x(f32::from_bits(0x3DCC_CCCD)), v) {
        pn(P_DEALERS).write(v.to_f32());
    } else {
        pn(P_DEALERS).write(f32::from_bits(0x3DCC_CCCD));
    }
    let mut sum = 0i32;
    for k in 0..10 {
        sum += i32::from(rd::<u8>(zi, k));
    }
    let v2 = xi(sum).fmul_m(k01);
    let mut l810 = true;
    if jp5(x(0.5), v2) {
        pn(P_GANGS).write(v2.to_f32());
        if !jp5(v2, x(f32::from_bits(0x3E19_999A))) {
            l810 = false;
            if jne_mask(x(f32::from_bits(0x3CA3_D70A)), x(pn(P_GANGS).read()), 0x41) {
                pn(P_COPS).write(pn(P_GANGS).read());
            } else {
                pn(P_COPS).write(f32::from_bits(0x3CA3_D70A));
            }
        }
    } else {
        pn(P_GANGS).write(0.5);
    }
    if l810 {
        let t = x(f32::from_bits(0x3E99_999A)).fsub_m(pn(P_GANGS).read());
        if jne_mask(x(f32::from_bits(0x3CF5_C28F)), t, 0x41) {
            pn(P_COPS).write(t.to_f32());
        } else {
            pn(P_COPS).write(f32::from_bits(0x3CF5_C28F));
        }
    }
    // `fld cops; fcomp c; test ah, 0x41; jne`: cops <= c stores c, else cops is reloaded (a floor, not a cap)
    let cap = |c: f32| {
        if jne_mask(x(pn(P_COPS).read()), x(c), 0x41) {
            pn(P_COPS).write(x(c).to_f32());
        } else {
            reload(pn(P_COPS));
        }
    };
    match (i32::from(rd::<u8>(zi, 0xF)) & 0x1F) - 4 {
        0 | 10 | 12 => cap(f32::from_bits(0x3DCC_CCCD)),
        1 => cap(f32::from_bits(0x3D4C_CCCD)),
        4 | 13 => pn(P_COPS).write(0.0),
        _ => {}
    }
    let tot = x(pn(P_COPS).read()).fadd_m(pn(P_GANGS).read()).fadd_m(pn(P_DEALERS).read());
    if jne_mask(tot, x(1.0), 0x41) {
        let o = x(1.0)
            .fsub_m(pn(P_DEALERS).read())
            .fsub_m(pn(P_GANGS).read())
            .fsub_m(pn(P_COPS).read());
        pn(P_OTHER).write(o.to_f32());
    } else {
        let k = tot.fdivr_m(1.0);
        pn(P_OTHER).write(0.0);
        pn(P_DEALERS).write(x(pn(P_DEALERS).read()).fmul(k).to_f32());
        pn(P_GANGS).write(x(pn(P_GANGS).read()).fmul(k).to_f32());
        pn(P_COPS).write(k.fmul_m(pn(P_COPS).read()).to_f32());
    }
    let row = || {
        let week = crate::popcycle::CURRENT_TIME_OF_WEEK.get();
        let time = crate::popcycle::CURRENT_TIME_INDEX.get();
        crate::popcycle::CURRENT_ZONE_TYPE
            .get()
            .wrapping_add(week.wrapping_add(time.wrapping_mul(2)).wrapping_mul(20))
    };
    let idx = row();
    let dealers_n = i32::from(perc_table(2, idx).read());
    let gang_n = i32::from(perc_table(1, idx).read());
    let cops_n = i32::from(perc_table(0, idx).read());
    let other_n = i32::from(crate::popcycle::PERC_OTHER.ptr().cast::<u8>().wrapping_index(idx).read());
    let dn = xi(dealers_n).fmul_m(pn(P_DEALERS).read()).fmul_m(k01).to_f32();
    let gn = xi(gang_n).fmul_m(pn(P_GANGS).read()).fmul_m(k01).to_f32();
    let cn = xi(cops_n).fmul_m(pn(P_COPS).read()).fmul_m(k01).to_f32();
    let on = xi(other_n).fmul_m(pn(P_OTHER).read()).fmul_m(k01).to_f32();
    let op = get_current_perc_other_peds_call();
    let idx = row();
    let opn = xi(op).fmul_m(pn(P_OTHER).read()).fmul_m(k01).to_f32();
    let mut max_peds = i32::from(perc_table(4, idx).read());
    if la_riots_active_here() != 0 && max_peds <= 20 {
        max_peds = 20;
    }
    let q = x(1.0).fsub_m(g::<f32>(0x8D12C0).read()).to_f32();
    let mult = g::<f32>(0x8CD060);
    if la_riots_active_here() != 0 && !jne_mask(x(q), x(0.5), 0x41) {
        let m = x(mult.read()).fsub_m(k01);
        mult.write(m.to_f32());
        if !jp5(m, x(f32::from_bits(0x3F19_999A))) {
            mult.write(f32::from_bits(0x3F19_999A));
        }
    } else {
        let m = x(mult.read()).fadd_m(k01);
        mult.write(m.to_f32());
        if !jne_mask(m, x(1.0), 0x41) {
            mult.write(1.0);
        }
    }
    let mp = xi(max_peds);
    NUM_DEALERS_PEDS.set(x(dn).fmul(mp).to_f32());
    pn(N_GANGS_PEDS).write(x(gn).fmul(mp).to_f32());
    pn(N_COPS_PEDS).write(x(cn).fmul(mp).to_f32());
    pn(N_OTHER_PEDS).write(mp.fmul_m(opn).to_f32());
    let idx = row();
    let mc = xi(i32::from(perc_table(3, idx).read()));
    pn(N_DEALERS_CARS).write(x(dn).fmul(mc).to_f32());
    pn(N_GANGS_CARS).write(x(gn).fmul(mc).to_f32());
    pn(N_COPS_CARS).write(x(cn).fmul(mc).to_f32());
    pn(N_OTHER_CARS).write(mc.fmul_m(on).to_f32());
    if la_riots_active_here() != 0 {
        for k in [N_DEALERS_CARS, N_GANGS_CARS, N_COPS_CARS, N_OTHER_CARS] {
            pn(k).write(x(pn(k).read()).fmul_m(0.75).to_f32());
        }
    }
}

orig! {
    /// `CWorld::FindObjectsInRange(const CVector&, float, bool, int16*, int16, CEntity**, bool, bool, bool, bool, bool)` (0x564A20)
    pub fn world_find_objects_in_range(pos: GPtr<f32>, r: f32, b2d: u8, cnt: GPtr<i16>, max: i32, out: GPtr<GPtr<u8>>, bld: u8, veh: u8, ped: u8, obj: u8, dum: u8) = cdecl 0x564A20 => unported;
    /// `CEntity::GetRandom2dEffect(int32 type, bool mustBeFree)` (0x533410)
    pub fn entity_get_random_2d_effect(e: GPtr<u8>, ty: i32, free: u8) -> GPtr<u8> = thiscall 0x533410 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890), result through the hidden out pointer
    pub fn matrix_mul_vec(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C890 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40)
    pub fn camera_is_sphere_visible(cam: GPtr<u8>, c: GPtr<f32>, r: f32) -> u8 = thiscall 0x420D40 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    pub fn get_random_number_in_range_i(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CStreaming::GetDefaultCopModel()` (0x407C00)
    pub fn get_default_cop_model() -> i32 = cdecl 0x407C00 => unported;
    /// `CPopulation::PedMICanBeCreatedAtThisAttractor(int32, const char*)` (0x6110E0)
    pub fn ped_mi_can_be_created_at_this_attractor(mi: i32, name: GPtr<u8>) -> u8 = cdecl 0x6110E0 => unported;
    /// `CPopulation::ChooseCivilianOccupation(bool, bool, int32, int32, int32, bool, bool, bool, const char*)` (0x612F90)
    pub fn choose_civilian_occupation_call(male: u8, female: u8, race: i32, exclude: i32, stat: i32, a: u8, b: u8, c: u8, name: GPtr<u8>) -> i32 = cdecl 0x612F90 => choose_civilian_occupation;
    /// `CPopulation::AddPedAtAttractor(int32, C2dEffect*, CVector, CEntity*, int32)` (0x614210)
    pub fn add_ped_at_attractor(mi: i32, eff: GPtr<u8>, px: f32, py: f32, pz: f32, ent: GPtr<u8>, dm: i32) -> u8 = cdecl 0x614210 => unported;
}

/// `CPopulation::GeneratePedsAtAttractors(CVector pos, float minR, float maxR, float minRClose, float
/// maxRClose, int32 decisionMaker, int32 numPeds)` (0x615970): up to 12 passes over the buildings and
/// objects within `maxR` (count initialised to -1 before `FindObjectsInRange`); each entity with an RW
/// object in the current area offers a random free ped-attractor effect (types 0/1/2/8 only between 9h and
/// 19h; flag 1 needs an object with `+0x143 & 1`), whose world position must lie strictly between the
/// radii (`minR` / `maxR` when the camera sees a 2.0 sphere there, else the close pair). In a police
/// station a 0..100 roll <= 70 tries the default cop model (and then sets the decision maker to 1, as the
/// bytes do); else a civilian for the effect's script name. A successful `AddPedAtAttractor` counts,
/// returns at once with decision maker -1, else decrements `numPeds` and restarts the pass counter.
#[hook(addr = 0x615970, conv = cdecl, name = "CPopulation::GeneratePedsAtAttractors", expect = "81 EC 2C 08 00 00")]
#[allow(clippy::too_many_arguments)]
pub fn generate_peds_at_attractors(
    px: f32,
    py: f32,
    pz: f32,
    min_r: f32,
    max_r: f32,
    min_r_close: f32,
    max_r_close: f32,
    mut dm: i32,
    mut num: i32,
) -> i32 {
    let mut created = 0i32;
    if num == 0 {
        return 0;
    }
    let mut pos = [px, py, pz];
    let mut ents = [GPtr::<u8>::NULL; 0x200];
    let mut cnt: [i16; 2] = [-1, -1];
    world_find_objects_in_range(
        GPtr::from_raw(pos.as_mut_ptr()),
        max_r,
        0,
        GPtr::from_raw(cnt.as_mut_ptr()),
        0x200,
        GPtr::from_raw(ents.as_mut_ptr()),
        1,
        0,
        0,
        1,
        0,
    );
    if num <= 0 {
        return created;
    }
    let mut tries = 0i32;
    loop {
        if tries >= 12 {
            return created;
        }
        tries += 1;
        let mut i = 0i32;
        while i < i32::from(cnt[0]) {
            if num == 0 {
                return created;
            }
            'next: {
                let e = ents[i as usize];
                if rd::<GPtr<u8>>(e, 0x18).is_null() || curr_area() != i32::from(rd::<u8>(e, 0x2F)) {
                    break 'next;
                }
                let eff = entity_get_random_2d_effect(e, 3, 1);
                if eff.is_null() {
                    break 'next;
                }
                let t = rd::<u8>(eff, 0x34);
                if t == 0 || t == 1 || t == 2 || t == 8 {
                    let h = i32::from(sa_core::light::CLOCK_HOURS.get());
                    if h <= 8 || h >= 20 {
                        break 'next;
                    }
                }
                if rd::<u8>(eff, 0x37) & 1 != 0 && (rd::<u8>(e, 0x36) & 7 != 4 || rd::<u8>(e, 0x143) & 1 == 0) {
                    break 'next;
                }
                let mut local = [rd::<u32>(eff, 0), rd::<u32>(eff, 4), rd::<u32>(eff, 8)];
                if rd::<GPtr<u8>>(e, 0x14).is_null() {
                    placeable_allocate_matrix(e);
                    simple_transform_update_matrix(e.byte_add(4), rd(e, 0x14));
                }
                let mut out = [0f32; 3];
                let lp: GPtr<f32> = GPtr::from_raw(local.as_mut_ptr()).cast();
                let w = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()), rd(e, 0x14), lp);
                let wp = [w.cast::<u32>().read(), w.cast::<u32>().wrapping_index(1).read(), w.cast::<u32>().wrapping_index(2).read()];
                local = wp;
                let dx = x(px).fsub_m(f32::from_bits(local[0]));
                let dy = x(py).fsub_m(f32::from_bits(local[1]));
                let dz = x(pz).fsub_m(f32::from_bits(local[2]));
                let dist = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt().to_f32();
                let (lo, hi) = if camera_is_sphere_visible(GPtr::from_addr(0xB6F028), lp, 2.0) != 0 {
                    (min_r, max_r)
                } else {
                    (min_r_close, max_r_close)
                };
                // fld dist; fcomp lo; test ah, 0x41; jne next
                if jne_mask(x(dist), x(lo), 0x41) {
                    break 'next;
                }
                // fld dist; fcomp hi; test ah, 5; jp next
                if jp5(x(dist), x(hi)) {
                    break 'next;
                }
                let name = eff.byte_add(0x38);
                let mut mi = None;
                if POLICE_STATION.read() != 0 && get_random_number_in_range_i(0, 100) <= 70 {
                    let cop = get_default_cop_model();
                    if ped_mi_can_be_created_at_this_attractor(cop, name) != 0 {
                        mi = Some(get_default_cop_model());
                        dm = 1;
                    }
                }
                let mi = match mi {
                    Some(m) => m,
                    None => choose_civilian_occupation_call(0, 0, -1, -1, -1, 1, 1, 1, name),
                };
                if mi <= 0 || mi == 7 {
                    break 'next;
                }
                if add_ped_at_attractor(
                    mi,
                    eff,
                    f32::from_bits(local[0]),
                    f32::from_bits(local[1]),
                    f32::from_bits(local[2]),
                    e,
                    dm,
                ) == 0
                {
                    break 'next;
                }
                created += 1;
                if dm == -1 {
                    return created;
                }
                num -= 1;
                tries = 0;
            }
            i += 1;
        }
        if num <= 0 {
            return created;
        }
    }
}

/// `CPopulation::bInPoliceStation` (0xC0FCB6, in `population::POP_FLAGS`)
const POLICE_STATION: PopFlag = PopFlag(4);

/// A byte of `CPopulation`'s flag block at 0xC0FCB2.
struct PopFlag(i32);

impl PopFlag {
    fn read(&self) -> u8 {
        crate::population::POP_FLAGS.ptr().cast::<u8>().wrapping_index(self.0).read()
    }
}

/// `FUN_00610DB0` (`CPedGroupPlacer`'s gang size, called by `AddToPopulation`): `min + _ftol((rand() & 0xFFFF)
/// * (1/32768) * (max - min))` with `ms_minGangSize` / `ms_maxGangSize` (0x86C6C4 / 0x86C6C8, read from memory).
#[hook(addr = 0x610DB0, conv = cdecl, name = "FUN_00610db0", expect = "51 56 8B 35 C8 C6 86 00")]
pub fn random_gang_size() -> i32 {
    let max = g::<i32>(0x86C6C8).read();
    let min = g::<i32>(0x86C6C4).read();
    let r = crate::common::rand_at(0x610DC4);
    let v = crate::common::rand_frac(r).fmul(crate::common::xi(max.wrapping_sub(min)));
    ftol32(v).wrapping_add(min)
}

orig! {
    /// `CStreaming::SetModelIsDeletable(int32)` (0x409C10)
    pub fn set_model_is_deletable(m: i32) = cdecl 0x409C10 => unported;
    /// `CStreaming::SetModelTxdIsDeletable(int32)` (0x409C70)
    pub fn set_model_txd_is_deletable(m: i32) = cdecl 0x409C70 => unported;
}

/// `CPopulation::FindSpecificDriverModelForCar_ToUse(int32 carModel)` (0x611900): the driver of the special
/// vehicles (jump table 0x611964 / byte table 0x611984); 463 picks 247 / 248 with
/// `GetRandomNumberInRange(0, 3)` (2 gives -1); else -1.
#[hook(addr = 0x611900, conv = cdecl, name = "CPopulation::FindSpecificDriverModelForCar_ToUse", expect = "8B 44 24 04 05 67 FE FF FF")]
pub fn find_specific_driver_model_for_car_to_use(car: i32) -> i32 {
    match car {
        409 => 0xFF,
        420 | 438 => get_default_cab_driver_model(),
        423 => 0x108,
        428 => 0x47,
        448 => 0x9B,
        463 => match get_random_number_in_range_i(0, 3) {
            0 => 0xF7,
            1 => 0xF8,
            _ => -1,
        },
        481 => 0x17,
        _ => -1,
    }
}

/// `CPopulation::RemoveSpecificDriverModelsForCar(int32 carModel)` (0x6119D0): `SetModelIsDeletable` then
/// `SetModelTxdIsDeletable` for the drivers `LoadSpecificDriverModelsForCar` requests (jump table 0x611AB0).
#[hook(addr = 0x6119D0, conv = cdecl, name = "CPopulation::RemoveSpecificDriverModelsForCar", expect = "8B 44 24 04 05 67 FE FF FF")]
pub fn remove_specific_driver_models_for_car(car: i32) {
    let both = |m: i32| {
        set_model_is_deletable(m);
        set_model_txd_is_deletable(m);
    };
    match car {
        409 => both(0xFF),
        420 | 438 => both(get_default_cab_driver_model()),
        423 => both(0x108),
        428 => both(0x47),
        448 => both(0x9B),
        463 => {
            both(0xF7);
            both(0xF8);
        }
        481 => both(0x17),
        _ => {}
    }
}

/// `CPopulation::FindDistanceToNearestPedOfType(ePedType, CVector pos)` (0x6143E0): the smallest
/// `sqrt((dz*dz + dy*dy) + dx*dx)` (stored as f32, strict `<`) over the peds of that type, descending; starts
/// at 10000000.0. The pool's byte map pointer is read once (as the original).
#[hook(addr = 0x6143E0, conv = cdecl, name = "CPopulation::FindDistanceToNearestPedOfType", expect = "51 D9 05 70 D2 86 00")]
pub fn find_distance_to_nearest_ped_of_type(ty: i32, px: f32, py: f32, pz: f32) -> X87 {
    let mut best = x(10_000_000.0);
    let pool = pool_at(PED_POOL);
    let mut i = rd::<i32>(pool, 8);
    if i == 0 {
        return best;
    }
    let map: GPtr<u8> = rd(pool, 4);
    while i != 0 {
        i -= 1;
        if (map.wrapping_index(i).read() as i8) < 0 {
            continue;
        }
        let objs: GPtr<u8> = rd(pool, 0);
        let p = objs.byte_add(i.wrapping_mul(0x7C4) as isize);
        if p.is_null() || rd::<i32>(p, 0x598) != ty {
            continue;
        }
        let pp = ent_pos(p);
        let dx = x(px).fsub_m(rd::<f32>(pp, 0));
        let dy = x(py).fsub_m(rd::<f32>(pp, 4));
        let dz = x(pz).fsub_m(rd::<f32>(pp, 8));
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
        let tmp = d.to_f32();
        if !jp5(d, best) {
            best = x(tmp);
        }
    }
    best
}

/// `CPopulation::ChooseCivilianCoupleOccupations(int32& husband, int32& wife)` (0x613180): see
/// `docs/re/specs/POP/CPopulation.deviations.md` (the wife call gets the husband's model and stat type;
/// `*wife` is not reset when the husband choice fails; both become -1 when exactly one is a skater).
#[hook(addr = 0x613180, conv = cdecl, name = "CPopulation::ChooseCivilianCoupleOccupations", expect = "83 EC 08 66 83 3D 14 13 C8 00 02")]
pub fn choose_civilian_couple_occupations(husband: GPtr<i32>, wife: GPtr<i32>) {
    let (mut a, mut b) = (1u8, 0u8);
    if g::<i16>(0xC81314).read() == 2 && crate::common::rand_at(0x61319C) & 0x10 != 0 {
        if crate::common::rand_at(0x6131A5) & 0x20 != 0 {
            (a, b) = (1, 1);
        } else {
            (a, b) = (0, 0);
        }
    }
    let h = choose_civilian_occupation_call(a, b, -1, -1, -1, 0, 1, 0, GPtr::NULL);
    husband.write(h);
    if h >= 0 {
        let stat = rd::<i32>(model_info(h), 0x2C);
        wife.write(choose_civilian_occupation_call(b, a, -1, h, stat, 0, 1, 0, GPtr::NULL));
    }
    let w = wife.read();
    if w >= 0 {
        let s1 = rd::<i32>(model_info(husband.read()), 0x2C);
        let s2 = rd::<i32>(model_info(w), 0x2C);
        if (s1 == 40) == (s2 == 40) {
            return;
        }
    }
    wife.write(-1);
    husband.write(-1);
}

/// `CPopulation::ArePedStatsCompatible(int32 a, int32 b)` (0x610F50): false when either is 0..3 or 26..42,
/// or when either is 4..10; with `a` 18 or 24 only `b` 18 or 24 is compatible.
#[hook(addr = 0x610F50, conv = cdecl, name = "CPopulation::ArePedStatsCompatible", expect = "8B 44 24 04 85 C0")]
pub fn are_ped_stats_compatible(a: i32, b: i32) -> u8 {
    let bad = |v: i32| (0..=3).contains(&v) || (26..=42).contains(&v);
    if bad(a) || bad(b) {
        return 0;
    }
    if (4..=10).contains(&a) || (4..=10).contains(&b) {
        return 0;
    }
    if (a == 18 || a == 24) && b != 18 && b != 24 {
        return 0;
    }
    1
}

orig! {
    /// `CPopCycle::PedIsAcceptableInCurrentZone(int32)` (0x610720)
    pub fn ped_is_acceptable_in_current_zone_call(m: i32) -> u8 = cdecl 0x610720 => crate::popcycle::ped_is_acceptable_in_current_zone;
    /// `CPopulation::ArePedStatsCompatible(int32, int32)` (0x610F50)
    pub fn are_ped_stats_compatible_call(a: i32, b: i32) -> u8 = cdecl 0x610F50 => are_ped_stats_compatible;
    /// `CPopulation::IsSunbather(int32)` (0x611760)
    pub fn is_sunbather_call(m: i32) -> u8 = cdecl 0x611760 => crate::population::is_sunbather;
}

/// `CStreaming::ms_aInfoForModel[m].m_nLoadState` (0x8E4CD0 + m * 0x14)
fn load_state(m: i32) -> u8 {
    g::<u8>(0x8E4CD0).byte_add(m.wrapping_mul(0x14) as isize).read()
}

/// `CPopulation::ChooseCivilianOccupation(bool male, bool female, int32 race, int32 exclude, int32 stat, bool
/// needFlag, bool tryHard, bool attractor, const char* attractorName)` (0x612F90): passes `i = 0..n` over the
/// 8 loaded ped slots, taking only models whose reference count is `i` (n = 3, 5 inside with more than 20
/// interior peds, 7 when `tryHard` is false); the first model passing every filter wins (sunbathers are
/// skipped while `CWeather::WetRoads >= 0.1`). Fallback 7 (without
/// `tryHard`) or -1. The entry's `currArea` read goes through a protection stub (0x407E99) that decodes
/// 0xB72914; the port reads `CGame::currArea` directly.
/// Not hookable: the protection reads the patched entry as a code key (`xor esi, [0x612F94]` at 0x401732).
#[allow(clippy::too_many_arguments)]
pub fn choose_civilian_occupation(
    male: u8,
    female: u8,
    race: i32,
    exclude: i32,
    stat: i32,
    need_flag: u8,
    try_hard: u8,
    attractor: u8,
    name: GPtr<u8>,
) -> i32 {
    let mut n = 3;
    if curr_area() != 0 && crate::population::NUMBER_OF_PEDS_IN_USE_INTERIOR.get() as i32 > 20 {
        n = 5;
    }
    if try_hard == 0 {
        n = 7;
    }
    let loaded = sa_stream::streaming::PEDS_LOADED.ptr().cast::<i32>();
    for pass in 0..n {
        for slot in 0..8 {
            let m = loaded.wrapping_index(slot).read();
            if m < 0 || load_state(m) != 1 {
                continue;
            }
            let mi = model_info(m);
            if i32::from(rd::<i16>(mi, 8)) != pass || m == exclude {
                continue;
            }
            if ped_is_acceptable_in_current_zone_call(m) == 0 && curr_area() == 0 {
                continue;
            }
            if need_flag != 0 && rd::<u8>(model_info(m), 0x31) & 0x10 == 0 {
                continue;
            }
            if male != 0 && rd::<i32>(model_info(m), 0x28) != 4 {
                continue;
            }
            if female != 0 && rd::<i32>(model_info(m), 0x28) != 5 {
                continue;
            }
            if race >= 0 && race != rd::<i32>(model_info(m), 0x24) {
                continue;
            }
            if attractor != 0 {
                let t = rd::<i32>(model_info(m), 0x28);
                if (17..=22).contains(&t) || ped_mi_can_be_created_at_this_attractor(m, name) == 0 {
                    continue;
                }
            }
            if curr_area() != 0 && (6..=22).contains(&rd::<i32>(model_info(m), 0x28)) {
                continue;
            }
            if stat > -1 && are_ped_stats_compatible_call(rd::<i32>(model_info(m), 0x2C), stat) == 0 {
                continue;
            }
            // fld WetRoads; fcomp 0.1; test ah, 5; jnp -> take it (WetRoads < 0.1)
            if !jp5(x(g::<f32>(0xC81324).read()), x(f32::from_bits(0x3DCC_CCCD))) || is_sunbather_call(m) == 0 {
                return m;
            }
        }
    }
    if try_hard == 0 { 7 } else { -1 }
}

/// `CPopulation::ChooseCivilianOccupationForVehicle(bool male, CVehicle*)` (0x613260): two passes; in each,
/// reference counts 0..3 then any (4) over the 8 loaded ped slots. Unless one of the cheats at 0x969157,
/// 0x969158, 0x969159, 0x96915A, 0x96915C, 0x96915D, 0x969176, 0x96917B is on, the model must be acceptable
/// in the zone and allow the vehicle's class (`1 << (int8)mi->+0x4D` in `+0x30`). Pass 0 skips models
/// already driving or riding the vehicle (+0x460 .. +0x46C); pass 1 takes the first match. Fallback 7.
#[hook(addr = 0x613260, conv = cdecl, name = "CPopulation::ChooseCivilianOccupationForVehicle", expect = "51 53 55 56 57")]
pub fn choose_civilian_occupation_for_vehicle(male: u8, veh: GPtr<u8>) -> i32 {
    let class = i32::from(rd::<i8>(model_info(i32::from(rd::<i16>(veh, 0x22))), 0x4D));
    let loaded = sa_stream::streaming::PEDS_LOADED.ptr().cast::<i32>();
    const CHEATS: [usize; 8] = [0x969157, 0x969158, 0x969159, 0x96915A, 0x96915C, 0x96915D, 0x969176, 0x96917B];
    for pass in 0..2 {
        for rc in 0..=4 {
            for slot in 0..8 {
                let m = loaded.wrapping_index(slot).read();
                if m < 0 || load_state(m) != 1 {
                    continue;
                }
                if rc != 4 && i32::from(rd::<i16>(model_info(m), 8)) != rc {
                    continue;
                }
                if CHEATS.iter().all(|&c| g::<u8>(c).read() == 0) {
                    if ped_is_acceptable_in_current_zone_call(m) == 0 {
                        continue;
                    }
                    let mask = u32::from(rd::<u16>(model_info(m), 0x30));
                    if 1u32.wrapping_shl(class as u32) & mask == 0 {
                        continue;
                    }
                }
                if male != 0 && rd::<i32>(model_info(m), 0x28) != 4 {
                    continue;
                }
                if pass == 1 {
                    return m;
                }
                let uses = |off: isize| {
                    let p: GPtr<u8> = rd(veh, off);
                    !p.is_null() && i32::from(rd::<i16>(p, 0x22)) == m
                };
                let mut used = uses(0x460);
                used |= uses(0x464);
                used |= uses(0x468);
                if uses(0x46C) {
                    continue;
                }
                if !used {
                    return m;
                }
            }
        }
    }
    7
}

orig! {
    /// `CPopulation::FindSpecificDriverModelForCar_ToUse(int32)` (0x611900)
    pub fn find_specific_driver_model_for_car_to_use_call(car: i32) -> i32 = cdecl 0x611900 => find_specific_driver_model_for_car_to_use;
    /// `CPopulation::ChooseCivilianOccupationForVehicle(bool, CVehicle*)` (0x613260)
    pub fn choose_civilian_occupation_for_vehicle_call(male: u8, veh: GPtr<u8>) -> i32 = cdecl 0x613260 => choose_civilian_occupation_for_vehicle;
    /// `CGangs::ChooseGangPedModel(int16)` (0x5DE5A0)
    pub fn choose_gang_ped_model_call(gang: i16) -> i32 = cdecl 0x5DE5A0 => crate::gangs::choose_gang_ped_model;
    /// `CStreaming::GetDefaultMedicModel()` (0x407D20)
    pub fn get_default_medic_model() -> i32 = cdecl 0x407D20 => unported;
    /// `CStreaming::GetDefaultFiremanModel()` (0x407D40)
    pub fn get_default_fireman_model() -> i32 = cdecl 0x407D40 => unported;
    /// `CPopulation::AddPed(ePedType, int32, const CVector&, bool)` (0x612710)
    pub fn add_ped_call(ty: i32, mi: i32, pos: GPtr<f32>, wander: u8) -> GPtr<u8> = cdecl 0x612710 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToEnterAsPassenger(const CVehicle*, int32)` (0x64F190)
    pub fn compute_target_door_to_enter_as_passenger(veh: GPtr<u8>, seat: i32) -> i32 = cdecl 0x64F190 => unported;
    /// `CCarEnterExit::SetPedInCarDirect(CPed*, CVehicle*, int32, bool)` (0x650280)
    pub fn set_ped_in_car_direct(ped: GPtr<u8>, veh: GPtr<u8>, door: i32, driver: u8) = cdecl 0x650280 => unported;
    /// `CPopulation::UpdatePedCount(CPed*, uint8)` (0x611570)
    pub fn update_ped_count_call(ped: GPtr<u8>, ty: u8) = cdecl 0x611570 => crate::population::update_ped_count;
}

/// `m_nPedType` (+0x28) of a ped model.
fn ped_type_of(m: i32) -> i32 {
    rd::<i32>(model_info(m), 0x28)
}

/// `CPopulation::AddPedInCar(CVehicle*, bool driver, int32 gangPedType, int32 seat, bool male, bool criminal)`
/// (0x613A00): the vehicle's specific driver (when driving and loaded), else by model (jump table 0x613BE0 /
/// byte table 0x613C04: 407 fireman 0x13, 416 medic 0x12, cop types for 427 / 430 / 432 / 433 / 490 / 497 / 523
/// / 596..599 with the cop model indices 0..5 straight to `AddPed`, 570 a civilian), else a gang member for
/// `gangPedType` 14..23 or a civilian for the vehicle; a model without +0x1C becomes 7 on the default path.
/// `AddPed` at the vehicle position, `SetPedInCarDirect`, and a criminal is recounted as type 0x14.
/// `FindPlayerCoors` is called (its result unused), as the original.
#[hook(addr = 0x613A00, conv = cdecl, name = "CPopulation::AddPedInCar", expect = "83 EC 18 53 56")]
pub fn add_ped_in_car(veh: GPtr<u8>, driver: u8, gang: i32, seat: i32, male: u8, criminal: u8) -> GPtr<u8> {
    let mut player = [0f32; 3];
    crate::common::find_player_coors(GPtr::from_raw(player.as_mut_ptr()), -1);
    let vp = ent_pos(veh);
    let mut pos = [rd::<u32>(vp, 0), rd::<u32>(vp, 4), rd::<u32>(vp, 8)];
    let spec = find_specific_driver_model_for_car_to_use_call(i32::from(rd::<i16>(veh, 0x22)));
    let (ty, m) = 'pick: {
        let (mut ty, mut m, checked) = if driver != 0 && spec >= 0 && load_state(spec) == 1 {
            (ped_type_of(spec), spec, true)
        } else {
            match i32::from(rd::<i16>(veh, 0x22)) {
                407 => (0x13, get_default_fireman_model(), true),
                416 => (0x12, get_default_medic_model(), true),
                427 => break 'pick (6, 2),
                430 | 497 | 596..=599 => break 'pick (6, 0),
                432 | 433 => break 'pick (6, 5),
                490 => break 'pick (6, 4),
                523 => break 'pick (6, 1),
                570 => {
                    let mut m = choose_civilian_occupation_call(0, 0, -1, -1, -1, 0, 1, 0, GPtr::NULL);
                    if m == -1 {
                        m = 7;
                    }
                    (ped_type_of(m), m, true)
                }
                _ => {
                    if (14..=23).contains(&gang) {
                        (gang - 7, choose_gang_ped_model_call((gang - 14) as i16), false)
                    } else {
                        let mut m = choose_civilian_occupation_for_vehicle_call(male, veh);
                        if m < 0 {
                            m = 7;
                        }
                        (ped_type_of(m), m, false)
                    }
                }
            }
        };
        if m < 0 {
            m = 7;
        }
        if !checked && rd::<GPtr<u8>>(model_info(m), 0x1C).is_null() {
            ty = ped_type_of(7);
            m = 7;
        }
        (ty, m)
    };
    let ped = add_ped_call(ty, m, GPtr::from_raw(pos.as_mut_ptr()).cast(), 0);
    let door = if seat >= 0 { compute_target_door_to_enter_as_passenger(veh, seat) } else { 0 };
    set_ped_in_car_direct(ped, veh, door, driver);
    if criminal != 0 {
        update_ped_count_call(ped, 1);
        wr::<i32>(ped, 0x598, 0x14);
        update_ped_count_call(ped, 0);
    }
    ped
}

/// `CPopCycle::PickGangToCreateMembersOf()` (0x60F8D0): with the gang cheat (0x96915B) `7 -
/// _ftol(frac(rand()) * -8.0)`; else 0 when the zone's gang densities sum to <= 0, otherwise `7 + g` for the
/// gang with the largest `density[g] / sum - ms_nNumGang[g] / m_NumGangs_Peds` (strictly larger, f32 kept;
/// gang 0 only when that value is > 0).
#[hook(addr = 0x60F8D0, conv = cdecl, name = "CPopCycle::PickGangToCreateMembersOf", expect = "A0 5B 91 96 00")]
pub fn pick_gang_to_create_members_of() -> i32 {
    use crate::common::xi;
    if g::<u8>(0x96915B).read() != 0 {
        let r = crate::common::rand_at(0x60F8E1);
        return 7i32.wrapping_sub(ftol32(crate::common::rand_frac(r).fmul_m(-8.0)));
    }
    let zi = crate::popcycle::CURR_ZONE_INFO.get();
    let mut s = xi(i32::from(rd::<u8>(zi, 0)));
    for k in 1..10 {
        s = s.fadd(xi(i32::from(rd::<u8>(zi, k))));
    }
    let sum = s.to_f32();
    // fcomp 0.0; test ah, 0x41; jp -> go on (sum > 0 or unordered)
    if !s.fcom(x(0.0)).test_parity(0x41) {
        return 0;
    }
    let inv1 = x(1.0).fdiv_m(sum).to_f32();
    let inv2 = x(1.0).fdiv_m(pn(N_GANGS_PEDS).read()).to_f32();
    let mut best = x(0.0);
    let mut gang = 0;
    for k in 0..10 {
        let a = xi(i32::from(rd::<u8>(zi, k))).fmul_m(inv1);
        let b = xi(counter(0xC0EC3C + 4 * k as usize).read() as i32).fmul_m(inv2);
        let v = a.fsub(b);
        if !jne_mask(v, best, 0x41) {
            best = x(v.to_f32());
            gang = k;
        }
    }
    gang as i32 + 7
}

orig! {
    /// `CPopulation::ChoosePolicePedOccupation()` (0x610F40)
    pub fn choose_police_ped_occupation() -> i32 = cdecl 0x610F40 => unported;
    /// `CPopulation::ChooseGangOccupation(int32)` (0x611550)
    pub fn choose_gang_occupation(gang: i32) -> i32 = cdecl 0x611550 => unported;
    /// `CPopCycle::PickGangToCreateMembersOf()` (0x60F8D0)
    pub fn pick_gang_to_create_members_of_call() -> i32 = cdecl 0x60F8D0 => pick_gang_to_create_members_of;
}

/// `fcom m; fnstsw; test ah, 0x44; jp` not taken: ST0 == m (ordered).
fn x87_eq(a: X87, b: f32) -> bool {
    !a.fcom(x(b)).test_parity(0x44)
}

/// `x < 2.0` (`fcomp 2.0; test ah, 5; jp` not taken) then `x = (rand() * 3.0518509e-05) * x`.
fn scale_chance(v: X87, slot: &mut f32, site: u32) {
    if !jp5(v, x(2.0)) {
        let r = crate::common::rand_at(site);
        *slot = crate::common::xi(r).fmul_m(f32::from_bits(0x3800_0100)).fmul_m(*slot).to_f32();
    }
}

/// `CPopCycle::FindNewPedType(ePedType* outType, int32* outMI, bool noGangs, bool noCops)` (0x60FBD0): see
/// `docs/re/specs/POP/CPopCycle.deviations.md` (police-station roll, the four chances, the max chain with
/// f32 reloads, the dealer search keeping the last loaded model and writing type 0x11 / -1 first, the gang
/// branch skipping `ChooseGangOccupation` when no gang is picked, return 1 with nothing written when no
/// chance equals the maximum).
#[hook(addr = 0x60FBD0, conv = cdecl, name = "CPopCycle::FindNewPedType", expect = "A1 68 BC C0 00")]
pub fn find_new_ped_type(out_type: GPtr<i32>, out_mi: GPtr<i32>, no_gangs: u8, no_cops: u8) -> u8 {
    use crate::common::xi;
    use crate::population::POP_FLAGS;
    let flag = |k: i32| POP_FLAGS.ptr().cast::<u8>().wrapping_index(k).read();
    let zi = crate::popcycle::CURR_ZONE_INFO.get();
    if zi.is_null() {
        return 0;
    }
    if flag(4) != 0 {
        let r = crate::common::rand_at(0x60FBF0);
        if ftol32(crate::common::rand_frac(r).fmul_m(100.0)) < 70 {
            out_type.write(6);
            out_mi.write(choose_police_ped_occupation());
            return 1;
        }
    }
    let mut dealers = xi(counter(0xC0EC38).read() as i32).fsubr_m(NUM_DEALERS_PEDS.get()).to_f32();
    let mut s = xi(counter(0xC0EC3C).read() as i32);
    for k in 1..10 {
        s = s.fadd(xi(counter(0xC0EC3C + 4 * k).read() as i32));
    }
    let mut gang = x(pn(N_GANGS_PEDS).read()).fsub(s).to_f32();
    if flag(1) != 0 {
        gang = 50.0;
    }
    if flag(0) != 0 || no_gangs != 0 {
        gang = -10.0;
    }
    let mut cops = xi(counter(0xC0EC68).read() as i32).fsubr_m(pn(N_COPS_PEDS).read()).to_f32();
    if gang_war_fighting_going_on_call() != 0 {
        cops = -10.0;
    }
    if flag(2) != 0 || no_cops != 0 {
        cops = -10.0;
    }
    let zi = crate::popcycle::CURR_ZONE_INFO.get();
    if !zi.is_null() && (rd::<u8>(zi, 0xF) as i8) < 0 {
        cops = -10.0;
    }
    let civ_n = counter(0xC0EC6C).read().wrapping_add(counter(0xC0EC70).read()) as i32;
    let civ_x = xi(civ_n).fsubr_m(pn(N_OTHER_PEDS).read());
    let mut civ = civ_x.to_f32();
    scale_chance(civ_x, &mut civ, 0x60FD35);
    scale_chance(x(cops), &mut cops, 0x60FD61);
    scale_chance(x(dealers), &mut dealers, 0x60FD8D);
    scale_chance(x(gang), &mut gang, 0x60FDB9);
    if curr_area() != 0 {
        dealers = -10.0;
    }
    loop {
        // each `fld a; fcomp b; test ah, 0x41; jne` keeps b when a <= b (or unordered)
        let max2 = |a: f32, b: f32| if jne_mask(x(a), x(b), 0x41) { b } else { a };
        let m1 = max2(dealers, gang);
        let m2 = max2(m1, cops);
        let hi = x(max2(m2, civ));
        if !hi.fcom(x(0.0)).test_parity(0x41) {
            return 0;
        }
        if x87_eq(hi, dealers) {
            out_type.write(0x11);
            out_mi.write(-1);
            let grp_cell = dealer_group;
            let n = |grp: i32| i32::from(crate::population::NUM_PEDS_IN_GROUP.ptr().cast::<i16>().wrapping_index(grp).read());
            let grp = grp_cell();
            if n(grp) > 0 {
                let mut i = 0;
                loop {
                    let m = i32::from(
                        crate::population::PED_GROUPS
                            .ptr()
                            .cast::<u16>()
                            .wrapping_index(grp.wrapping_mul(21).wrapping_add(i))
                            .read(),
                    );
                    if load_state(m) == 1 {
                        out_mi.write(m);
                    }
                    i += 1;
                    if i >= n(grp_cell()) {
                        break;
                    }
                }
                if out_mi.read() >= 0 {
                    return 1;
                }
            }
            dealers = 0.0;
            continue;
        }
        if x87_eq(hi, gang) {
            let t = pick_gang_to_create_members_of_call();
            out_type.write(t);
            if t == 0 {
                out_mi.write(-1);
            } else {
                let m = choose_gang_occupation(t - 7);
                out_mi.write(m);
                if m >= 0 {
                    return 1;
                }
            }
            if flag(1) != 0 {
                return 0;
            }
            gang = 0.0;
            continue;
        }
        if x87_eq(hi, cops) {
            out_type.write(6);
            out_mi.write(choose_police_ped_occupation());
            return 1;
        }
        if x87_eq(hi, civ) {
            let m = choose_civilian_occupation_call(0, 0, -1, -1, -1, 0, 1, 0, GPtr::NULL);
            out_mi.write(m);
            if m < 0 || m == 7 {
                return 0;
            }
            out_type.write(ped_type_of(m));
        }
        return 1;
    }
}

/// `m_TranslationArray[28].ids[0]` (0x8D2690): the dealers' ped group.
fn dealer_group() -> i32 {
    crate::population::TRANSLATION_ARRAY.ptr().cast::<i32>().wrapping_index(84).read()
}

/// `CPopCycle::PlayerKilledADealer()` (0x610490): the current zone info's dealer strength (+0xA) drops by
/// one when above 0.
#[hook(addr = 0x610490, conv = cdecl, name = "CPopCycle::PlayerKilledADealer", expect = "8B 0D 68 BC C0 00")]
pub fn player_killed_a_dealer() {
    let zi = crate::popcycle::CURR_ZONE_INFO.get();
    if zi.is_null() {
        return;
    }
    let s = rd::<u8>(zi, 0xA);
    if s > 0 {
        wr(zi, 0xA, s - 1);
    }
}

/// `CPopCycle::UpdateIsGangArea()` (0x6106D0): `m_bCurrentZoneIsGangArea = sum(gang densities) > 20`.
#[hook(addr = 0x6106D0, conv = cdecl, name = "CPopCycle::UpdateIsGangArea", expect = "8B 0D 68 BC C0 00")]
pub fn update_is_gang_area() {
    let zi = crate::popcycle::CURR_ZONE_INFO.get();
    let mut sum = 0i32;
    for k in 0..10 {
        sum += i32::from(rd::<u8>(zi, k));
    }
    crate::popcycle::CURRENT_ZONE_IS_GANG_AREA.set(u8::from(sum > 20));
}

orig! {
    /// `CCullZones::FewerPeds()` (0x72DD90)
    pub fn cull_zones_fewer_peds() -> u8 = cdecl 0x72DD90 => unported;
}

/// `CPopulation::FindPedDensityMultiplierCullZone()` (0x610F00): 0.6 in a "fewer peds" cull zone, else 1.0.
#[hook(addr = 0x610F00, conv = cdecl, name = "CPopulation::FindPedDensityMultiplierCullZone", expect = "E8 8B CE 11 00")]
pub fn find_ped_density_multiplier_cull_zone() -> X87 {
    if cull_zones_fewer_peds() != 0 { x(f32::from_bits(0x3F19_999A)) } else { x(1.0) }
}

/// `CPopulation::PedMICanBeCreatedAtAttractor(int32)` (0x6110C0): not for ped types 17..22.
#[hook(addr = 0x6110C0, conv = cdecl, name = "CPopulation::PedMICanBeCreatedAtAttractor", expect = "8B 44 24 04 8B 0C 85 C8 B0 A9 00")]
pub fn ped_mi_can_be_created_at_attractor(m: i32) -> u8 {
    u8::from(!(17..=22).contains(&ped_type_of(m)))
}

/// `CPopulation::PedMICanBeCreatedInInterior(int32)` (0x611450): not for ped types 6..22.
#[hook(addr = 0x611450, conv = cdecl, name = "CPopulation::PedMICanBeCreatedInInterior", expect = "8B 44 24 04 8B 0C 85 C8 B0 A9 00")]
pub fn ped_mi_can_be_created_in_interior(m: i32) -> u8 {
    u8::from(!(6..=22).contains(&ped_type_of(m)))
}

/// `CPopulation::IsMale(int32)` (0x611470): ped type 4 (EAX 0/1).
#[hook(addr = 0x611470, conv = cdecl, name = "CPopulation::IsMale", expect = "8B 44 24 04 8B 0C 85 C8 B0 A9 00")]
pub fn is_male(m: i32) -> u8 {
    u8::from(ped_type_of(m) == 4)
}

/// `CPopulation::IsFemale(int32)` (0x611490): ped type 5 (EAX 0/1).
#[hook(addr = 0x611490, conv = cdecl, name = "CPopulation::IsFemale", expect = "8B 44 24 04 8B 0C 85 C8 B0 A9 00")]
pub fn is_female(m: i32) -> u8 {
    u8::from(ped_type_of(m) == 5)
}

/// `CPopulation::AddExistingPedInCar(CPed*, CVehicle*)` (0x611560): returns the ped.
/// Not hookable: the protection reads the entry as data (`add eax, 0x611561` at 0x1426A2E / 0x1515006).
pub fn add_existing_ped_in_car(ped: GPtr<u8>, _veh: GPtr<u8>) -> GPtr<u8> {
    ped
}

/// `CPopulation::CanSolicitPlayerInCar(int32)` (0x611790): ped type 22 (AL; the rest of EAX is the model
/// info pointer's high bytes in the original).
#[hook(addr = 0x611790, conv = cdecl, name = "CPopulation::CanSolicitPlayerInCar", expect = "8B 44 24 04 8B 0C 85 C8 B0 A9 00")]
pub fn can_solicit_player_in_car(m: i32) -> u8 {
    u8::from(ped_type_of(m) == 0x16)
}

/// `CPopulation::CanJeerAtStripper(int32)` (0x6117B0): ped type 4 (AL only, as above).
#[hook(addr = 0x6117B0, conv = cdecl, name = "CPopulation::CanJeerAtStripper", expect = "8B 44 24 04 8B 0C 85 C8 B0 A9 00")]
pub fn can_jeer_at_stripper(m: i32) -> u8 {
    u8::from(ped_type_of(m) == 4)
}

/// `CPopulation::FindPedMultiplierMotorway()` (0x611B80): 1.0 (the `m_bMoreCarsAndFewerPeds` read is unused).
#[hook(addr = 0x611B80, conv = cdecl, name = "CPopulation::FindPedMultiplierMotorway", expect = "A0 B5 FC C0 00")]
pub fn find_ped_multiplier_motorway() -> X87 {
    x(1.0)
}

/// `CPopulation::Shutdown()` (0x610EC0, a lone `ret`): not hookable (1 byte).
pub fn shutdown() {}

/// `CPopulation::MoveCarsAndPedsOutOfAbandonedZones()` (0x6116A0, a lone `ret`): not hookable (1 byte).
pub fn move_cars_and_peds_out_of_abandoned_zones() {}

/// `CPopulation::DealWithZoneChange(eLevelName, eLevelName, bool)` (0x6116B0, a lone `ret`): not hookable.
pub fn deal_with_zone_change(_old: i32, _new: i32, _force: u8) {}

/// `CPopulation::IsSecurityGuard(int32)` (0x6114B0, `xor al, al; ret`): not hookable (3 bytes).
pub fn is_security_guard(_m: i32) -> u8 {
    0
}

/// `CPopulation::CanSolicitPlayerOnFoot(int32)` (0x611780, `xor al, al; ret`): not hookable (3 bytes).
pub fn can_solicit_player_on_foot(_m: i32) -> u8 {
    0
}

/// `CPopulation::IsCorrectTimeOfDayForEffect(const C2dEffect*)` (0x611B20): attractor types 0/1/2/8 only
/// between 9h and 19h (EAX 0/1 there; AL = 1 for the other types).
#[hook(addr = 0x611B20, conv = cdecl, name = "CPopulation::IsCorrectTimeOfDayForEffect", expect = "8B 44 24 04 8A 40 34")]
pub fn is_correct_time_of_day_for_effect(eff: GPtr<u8>) -> u8 {
    let t = rd::<u8>(eff, 0x34);
    if t == 0 || t == 1 || t == 2 || t == 8 {
        let h = i32::from(sa_core::light::CLOCK_HOURS.get());
        return u8::from(h > 8 && h < 20);
    }
    1
}

orig! {
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x6, CStoredCollPoly*)` (0x5674E0)
    pub fn world_process_vertical_line(o: GPtr<f32>, z: f32, cp: GPtr<u8>, ent: GPtr<GPtr<u8>>, b: u8, v: u8, p: u8, ob: u8, d: u8, see: u8, poly: GPtr<u8>) -> u8 = cdecl 0x5674E0 => unported;
    /// `SurfaceInfos_c::IsSkateable(uint32)` (0x55E7D0)
    pub fn surface_is_skateable(si: GPtr<u8>, surface: u32) -> u8 = thiscall 0x55E7D0 => unported;
}

/// `CPopulation::IsSkateable(const CVector&)` (0x6114C0): a building hit by the vertical line from z + 2 to
/// z - 2 whose surface (colpoint +0x23) is skateable (`g_surfaceInfos` 0xB79538).
#[hook(addr = 0x6114C0, conv = cdecl, name = "CPopulation::IsSkateable", expect = "83 EC 3C 8B 44 24 40")]
pub fn is_skateable(pos: GPtr<f32>) -> u8 {
    let mut start = [
        pos.read(),
        x(pos.wrapping_index(1).read()).to_f32(),
        x(pos.wrapping_index(2).read()).fadd_m(2.0).to_f32(),
    ];
    let z_end = x(pos.wrapping_index(2).read()).fsub_m(2.0).to_f32();
    let mut cp = [0u8; 0x2C];
    let mut ent = GPtr::<u8>::NULL;
    world_process_vertical_line(
        GPtr::from_raw(start.as_mut_ptr()),
        z_end,
        GPtr::from_raw(cp.as_mut_ptr()),
        GPtr::from_raw(&mut ent),
        1,
        0,
        0,
        0,
        0,
        0,
        GPtr::NULL,
    );
    if ent.is_null() {
        return 0;
    }
    u8::from(surface_is_skateable(GPtr::from_addr(0xB79538), u32::from(cp[0x23])) != 0)
}

/// `CPopulation::FindNumberOfPedsWeCanPlaceOnBenches()` (0x612240): `_ftol(_ftol(min(MaxNumberOfPedsInUse,
/// m_NumOther_Peds)) * PedDensityMultiplier * cullZoneFactor)` (the interior count inside), minus the
/// civilians, plus 2.
#[hook(addr = 0x612240, conv = cdecl, name = "CPopulation::FindNumberOfPedsWeCanPlaceOnBenches", expect = "51 DB 05 38 25 8D 00")]
pub fn find_number_of_peds_we_can_place_on_benches() -> i32 {
    let v = crate::common::xi(sa_core::inifile::MAX_NUMBER_OF_PEDS_IN_USE.get() as i32);
    let other = x(pn(N_OTHER_PEDS).read());
    let v = if jp5(v, other) { other } else { v };
    let n = ftol32(v);
    let f = if cull_zones_fewer_peds() != 0 { x(f32::from_bits(0x3F19_999A)) } else { x(1.0) };
    let mut m = ftol32(crate::common::xi(n).fmul_m(crate::population::PED_DENSITY_MULTIPLIER.get()).fmul(f));
    if curr_area() != 0 {
        m = crate::population::NUMBER_OF_PEDS_IN_USE_INTERIOR.get() as i32;
    }
    m.wrapping_sub(counter(0xC0EC6C).read() as i32)
        .wrapping_sub(counter(0xC0EC70).read() as i32)
        .wrapping_add(2)
}

/// `CPopulation::RemoveAllRandomPeds()` (0x6122C0): every deletable ped, descending.
#[hook(addr = 0x6122C0, conv = cdecl, name = "CPopulation::RemoveAllRandomPeds", expect = "55 8B 2D 90 44 B7 00")]
pub fn remove_all_random_peds() {
    let pool = pool_at(PED_POOL);
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        let p = pool_slot(pool, i, 0x7C4);
        if !p.is_null() && ped_can_be_deleted(p) != 0 {
            delete(p);
        }
    }
}

/// `FUN_00611F20` (thiscall, `ret 8`): `this + 8 * k` with `k` from the (-1/0/1, -1/0/1) pair: (-1,-1) 5,
/// (-1,0) 3, (-1,1) 0, (0,-1) 6, (0,1) 1, (1,-1) 7, (1,0) 4, (1,1) 2, anything else 0.
#[hook(addr = 0x611F20, conv = thiscall, name = "FUN_00611f20", expect = "8B 44 24 04 33 D2")]
pub fn offset_by_direction(this: GPtr<u8>, a: i32, b: i32) -> GPtr<u8> {
    let k = match (a, b) {
        (-1, -1) => 5,
        (-1, 0) => 3,
        (0, -1) => 6,
        (0, 1) => 1,
        (1, -1) => 7,
        (1, 0) => 4,
        (1, 1) => 2,
        _ => 0,
    };
    this.byte_add(8 * k)
}

/// `thunk_FUN_01560e20` (stub 0x406E80 -> 0x1560E20): `m_PedGroups[m_TranslationArray[group * 3 + zone]][slot]`.
#[hook(addr = 0x406E80, conv = cdecl, name = "thunk_FUN_01560e20", expect = "E9 9B 9F 15 01")]
pub fn ped_group_model(group: i32, slot: i32, zone: i32) -> i32 {
    let pg = crate::population::TRANSLATION_ARRAY
        .ptr()
        .cast::<i32>()
        .wrapping_index(group.wrapping_mul(3).wrapping_add(zone))
        .read();
    i32::from(
        crate::population::PED_GROUPS
            .ptr()
            .cast::<u16>()
            .wrapping_index(pg.wrapping_mul(21).wrapping_add(slot))
            .read(),
    )
}

/// `CPopCycle::IsPedAppropriateForCurrentZone(int32)` (0x610150): the model's race must be allowed by the
/// zone (`zi->+0x10 & 0xF`, byte test), and the model must belong to a ped group of a popcycle group with a
/// non-zero share in the current row (zone type from `zi->+0xF`).
#[hook(addr = 0x610150, conv = cdecl, name = "CPopCycle::IsPedAppropriateForCurrentZone", expect = "8B 44 24 04 8B 0C 85 C8 B0 A9 00")]
pub fn is_ped_appropriate_for_current_zone(m: i32) -> u8 {
    use crate::popcycle::{CURR_ZONE_INFO, CURRENT_TIME_INDEX, CURRENT_TIME_OF_WEEK, PERC_TYPE_GROUP};
    use crate::population::{NUM_PEDS_IN_GROUP, PED_GROUPS, TRANSLATION_ARRAY};
    let race = u32::from(rd::<u8>(model_info(m), 0x3A));
    let zi = CURR_ZONE_INFO.get();
    if race != 0 {
        let bit = 1u32.wrapping_shl(race - 1) as u8;
        if (rd::<u8>(zi, 0x10) & 0xF) & bit == 0 {
            return 0;
        }
    }
    let row = (i32::from(rd::<u8>(zi, 0xF)) & 0x1F)
        .wrapping_add(CURRENT_TIME_OF_WEEK.get().wrapping_add(CURRENT_TIME_INDEX.get().wrapping_mul(2)).wrapping_mul(20));
    let p = PERC_TYPE_GROUP.ptr().cast::<u8>().wrapping_index(row.wrapping_mul(18));
    let zone = CURRENT_WORLD_ZONE.get() as i32;
    for gidx in 0..18 {
        if p.wrapping_index(gidx).read() == 0 {
            continue;
        }
        let pg = TRANSLATION_ARRAY.ptr().cast::<i32>().wrapping_index(gidx * 3 + zone).read();
        let n = i32::from(NUM_PEDS_IN_GROUP.ptr().cast::<i16>().wrapping_index(pg).read());
        let rowp = PED_GROUPS.ptr().cast::<u16>().wrapping_index(pg.wrapping_mul(21));
        for i in 0..n.max(0) {
            if i32::from(rowp.wrapping_index(i).read()) == m {
                return 1;
            }
        }
    }
    0
}

orig! {
    /// `CStreaming::GetDefaultCopCarModel(int32)` (0x407C50)
    pub fn get_default_cop_car_model(ignore_lvpd: i32) -> i32 = cdecl 0x407C50 => unported;
}

/// The `i16` model at byte offset `off` of the loaded car group at `addr` (0xC0E9F8 appropriate, 0xC0E9C8
/// inappropriate).
fn loaded_model(addr: usize, off: i32) -> i32 {
    let base = crate::population::LOADED_CAR_GROUPS.ptr().cast::<u8>();
    i32::from(base.byte_add((addr - 0xC0E998) as isize + off as isize).cast::<i16>().read())
}

/// `CPopulation::PickRiotRoadBlockCar()` (0x6144B0): a random gang car group to start from (`_ftol(frac *
/// 10)`), then the appropriate and the inappropriate loaded cars scanned forwards for the first model whose
/// class (+0x4D) is not 5, 7 or 8 (empty -1 entries are not skipped: the `jl` tests the byte offset, as the
/// bytes do), else `GetDefaultCopCarModel(1)`.
#[hook(addr = 0x6144B0, conv = cdecl, name = "CPopulation::PickRiotRoadBlockCar", expect = "51 56 57 E8 66 D6 20 00")]
pub fn pick_riot_road_block_car() -> i32 {
    let r = crate::common::rand_at(0x6144B8);
    let base = ftol32(crate::common::rand_frac(r).fmul_m(10.0));
    for i in 0..10i32 {
        let g = i.wrapping_add(base) % 10;
        let m = crate::population::pick_random_car(crate::population::loaded_gang_cars(g), 0, 0);
        if m >= 0 {
            return m;
        }
    }
    for addr in [0xC0E9F8, 0xC0E9C8] {
        let mut off = 0;
        while off < 0x2E {
            let m = loaded_model(addr, off);
            let class = rd::<u8>(model_info(m), 0x4D);
            if class != 5 && class != 7 && class != 8 {
                return m;
            }
            off += 2;
        }
    }
    get_default_cop_car_model(1)
}

/// `CPopulation::PlaceMallPedsAsStationaryGroup(const CVector&)` (0x613CD0): in area 4 only, a civilian's ped
/// type placed as a group of `min + _ftol(frac(rand()) * (max - min))` (`CPedGroupPlacer` sizes at 0x86C6C4 /
/// 0x86C6C8) with allocator 2; the placer `this` is 4 zero bytes here (uninitialised in the original).
#[hook(addr = 0x613CD0, conv = cdecl, name = "CPopulation::PlaceMallPedsAsStationaryGroup", expect = "A1 14 29 B7 00")]
pub fn place_mall_peds_as_stationary_group(pos: GPtr<f32>) {
    if curr_area() != 4 {
        return;
    }
    let m = choose_civilian_occupation_call(0, 0, -1, -1, -1, 0, 1, 0, GPtr::NULL);
    let ty = ped_type_of(m);
    let max = g::<i32>(0x86C6C8).read();
    let min = g::<i32>(0x86C6C4).read();
    let r = crate::common::rand_at(0x613D15);
    let f = crate::common::rand_frac(r);
    let n = ftol32(crate::common::xi(max.wrapping_sub(min)).fmul(f)).wrapping_add(min);
    let mut placer = [0u8; 4];
    crate::population::ped_group_placer_place_group(GPtr::from_raw(placer.as_mut_ptr()), ty, n, pos, 2);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_MANAGE_DUMMY,
    HOOK_TEST_ROOM_FOR_DUMMY_OBJECT,
    HOOK_TEST_SAFE_FOR_REAL_OBJECT,
    HOOK_CONVERT_TO_REAL_OBJECT,
    HOOK_CONVERT_TO_DUMMY_OBJECT,
    HOOK_MANAGE_OBJECT,
    HOOK_MANAGE_PED,
    HOOK_MANAGE_ALL_POPULATION,
    HOOK_MANAGE_POPULATION,
    HOOK_REMOVE_PEDS_IF_THE_POOL_GETS_FULL,
    HOOK_CONVERT_ALL_OBJECTS_TO_DUMMY_OBJECTS,
    HOOK_GENERATE_PEDS_AT_START_OF_GAME,
    HOOK_UPDATE,
    HOOK_LOAD_SPECIFIC_DRIVER_MODELS_FOR_CAR,
    HOOK_LOAD_PED_GROUPS,
    HOOK_LOAD_CAR_GROUPS,
    HOOK_POPCYCLE_INITIALISE,
    HOOK_GENERATE_PEDS_AT_ATTRACTORS,
    HOOK_RANDOM_GANG_SIZE,
    HOOK_FIND_SPECIFIC_DRIVER_MODEL_FOR_CAR_TO_USE,
    HOOK_REMOVE_SPECIFIC_DRIVER_MODELS_FOR_CAR,
    HOOK_FIND_DISTANCE_TO_NEAREST_PED_OF_TYPE,
    HOOK_CHOOSE_CIVILIAN_COUPLE_OCCUPATIONS,
    HOOK_ARE_PED_STATS_COMPATIBLE,
    HOOK_CHOOSE_CIVILIAN_OCCUPATION_FOR_VEHICLE,
    HOOK_ADD_PED_IN_CAR,
    HOOK_PICK_GANG_TO_CREATE_MEMBERS_OF,
    HOOK_PLAYER_KILLED_A_DEALER,
    HOOK_UPDATE_IS_GANG_AREA,
    HOOK_UPDATE_PERCENTAGES,
    HOOK_FIND_NEW_PED_TYPE,
    HOOK_FIND_PED_DENSITY_MULTIPLIER_CULL_ZONE,
    HOOK_PED_MI_CAN_BE_CREATED_AT_ATTRACTOR,
    HOOK_PED_MI_CAN_BE_CREATED_IN_INTERIOR,
    HOOK_IS_MALE,
    HOOK_IS_FEMALE,
    HOOK_CAN_SOLICIT_PLAYER_IN_CAR,
    HOOK_CAN_JEER_AT_STRIPPER,
    HOOK_FIND_PED_MULTIPLIER_MOTORWAY,
    HOOK_IS_CORRECT_TIME_OF_DAY_FOR_EFFECT,
    HOOK_IS_SKATEABLE,
    HOOK_FIND_NUMBER_OF_PEDS_WE_CAN_PLACE_ON_BENCHES,
    HOOK_REMOVE_ALL_RANDOM_PEDS,
    HOOK_OFFSET_BY_DIRECTION,
    HOOK_PED_GROUP_MODEL,
    HOOK_IS_PED_APPROPRIATE_FOR_CURRENT_ZONE,
    HOOK_PICK_RIOT_ROAD_BLOCK_CAR,
    HOOK_PLACE_MALL_PEDS_AS_STATIONARY_GROUP,
];
