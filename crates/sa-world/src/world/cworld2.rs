//! CWorld functions of PORT-WORLD-2 (helper B): initialisation, sector constructors, the range scans
//! over one sector list, scan-code advance, pool sweeps and `FindPlayerHeading`. Ported from the 1.0 US
//! bytes (gta-reversed `World.cpp` as reading aid).
//!
//! Pools: `CPool` +0 objects, +4 flag bytes (bit 7 = free), +8 size; vehicles are 0xA18 bytes.
//! `CEntity`: +0x14 matrix (position at +0x30, right vector at +0x10/+0x14), +4 placement position,
//! +0x10 placement heading, +0x22 model index, +0x2C scan code. `CPtrList` node: +0 item, +4 next.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{g, rd, wr, x};
use super::cworld::{CURRENT_SCAN_CODE, PLAYERS, PLAYER_INFO_SIZE, PLAYER_IN_FOCUS};

sa_core::globals! {
    list GLOBALS;
}

/// `CPools::ms_pVehiclePool`
const VEHICLE_POOL: usize = 0xB74494;
const VEHICLE_SIZE: u32 = 0xA18;
/// `gCurCamColVars` (owner: CCamera)
const CUR_CAM_COL_VARS: usize = 0x8CCB80;

orig! {
    /// `CIplStore::Initialise` (0x405EC0)
    pub fn ipl_store_initialise() = cdecl 0x405EC0 => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    pub fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::CanBeDeleted` (0x5DF100)
    pub fn ped_can_be_deleted(ped: GPtr<u8>) -> u8 = thiscall 0x5DF100 => unported;
    /// `CPopulation::RemovePed(CPed*)` (0x610F20)
    pub fn population_remove_ped(ped: GPtr<u8>) = cdecl 0x610F20 => unported;
    /// CRT `printf` (0x821982) with three integer arguments
    pub fn printf4(fmt: GPtr<u8>, a: u32, b: i32, c: i32) -> i32 = cdecl 0x821982 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70)
    pub fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40)
    pub fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame` (0x532B00)
    pub fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CEntity::Add` (vtable slot 8)
    pub fn entity_add(e: GPtr<u8>) = thiscall vslot 0x8 => unported;
    /// `CPhysical::AddToMovingList` (0x542800)
    pub fn add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// `CEntity::Remove` (vtable slot 0xC)
    pub fn entity_remove(e: GPtr<u8>) = thiscall vslot 0xC => unported;
    /// `CPhysical::RemoveFromMovingList` (0x542860)
    pub fn remove_from_moving_list(e: GPtr<u8>) = thiscall 0x542860 => unported;
    /// `CPedIntelligence::ProcessAfterPreRender` (0x6019B0)
    pub fn process_after_pre_render(intel: GPtr<u8>) = thiscall 0x6019B0 => unported;
}

/// `CWorld::Initialise` (0x5631E0): the eight bytes 0xB7CD6C..0xB7CD73 (`bSecondShift`,
/// `bProcessCutsceneOnly`, `bForceProcessControl`, `bIncludeBikers`, `bIncludeCarTyres`,
/// `bIncludeDeadPeds`, `bNoMoreCollisionTorque`, `bDoingCarCollisions`), `pIgnoreEntity` and
/// `fWeaponSpreadRate` = 0, then tail call `CIplStore::Initialise`.
#[hook(addr = 0x5631E0, conv = cdecl, name = "CWorld::Initialise", expect = "33 C0 A2 73 CD B7 00")]
pub fn initialise() {
    for a in [0xB7CD73, 0xB7CD72, 0xB7CD71, 0xB7CD70, 0xB7CD6F, 0xB7CD6E, 0xB7CD6D, 0xB7CD6C] {
        g::<u8>(a).write(0);
    }
    g::<u32>(0xB7CD68).write(0);
    g::<u32>(0xB7CD64).write(0);
    ipl_store_initialise();
}

/// `CSector::CSector` (0x564040, thiscall): both lists empty; EAX = this.
#[hook(addr = 0x564040, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "8B C1 C7 00 00 00 00 00", this = "CSector")]
pub fn sector_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    this
}

/// `CWorld::CameraToIgnoreThisObject(CEntity*)` (0x563F40): false for a door model; else by the object
/// info's camera-avoid byte (+0x160 → +0x1E): 1 → `gCurCamColVars >= 10`, 2 → false, 3 →
/// `gCurCamColVars < 10`, 0 and above 3 → true.
#[hook(addr = 0x563F40, conv = cdecl, name = "CWorld::CameraToIgnoreThisObject", expect = "56 8B 74 24 08 0F BF 46")]
pub fn camera_to_ignore_this_object(e: GPtr<u8>) -> u8 {
    if super::garages::is_model_index_a_door(i32::from(rd::<i16>(e, 0x22))) != 0 {
        return 0;
    }
    let below = (g::<u8>(CUR_CAM_COL_VARS).read() < 10) as u8;
    match rd::<u8>(rd::<GPtr<u8>>(e, 0x160), 0x1E) {
        1 => (below == 0) as u8,
        2 => 0,
        3 => below,
        _ => 1,
    }
}

/// `CWorld::SetAllCarsCanBeDamaged(bool)` (0x5668F0): bit 5 of +0x42A (`bCanBeDamaged`) of every used
/// vehicle slot = bit 0 of the argument byte.
#[hook(addr = 0x5668F0, conv = cdecl, name = "CWorld::SetAllCarsCanBeDamaged", expect = "55 56 8B 35 94 44 B7 00")]
pub fn set_all_cars_can_be_damaged(enable: u8) {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let size = rd::<i32>(pool, 8);
    let mut i = 0;
    while i < size {
        let used = (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() as i8) >= 0;
        let v = rd::<GPtr<u8>>(pool, 0).byte_add((i as u32).wrapping_mul(VEHICLE_SIZE) as isize);
        if used && !v.is_null() {
            let b = rd::<u8>(v, 0x42A);
            wr::<u8>(v, 0x42A, ((enable << 5) ^ b) & 0x20 ^ b);
        }
        i += 1;
    }
}

/// `CWorld::StopAllLawEnforcersInTheirTracks` (0x566C10): move speed (+0x44) of every used vehicle with
/// bit 0 of +0x428 (`bIsLawEnforcer`) = 0, last slot first.
#[hook(addr = 0x566C10, conv = cdecl, name = "CWorld::StopAllLawEnforcersInTheirTracks", expect = "83 EC 0C 56 8B 35 94 44 B7 00")]
pub fn stop_all_law_enforcers_in_their_tracks() {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() & 0x80 != 0 {
            continue;
        }
        let v = rd::<GPtr<u8>>(pool, 0).byte_add(i.wrapping_mul(VEHICLE_SIZE) as isize);
        if v.is_null() || rd::<u8>(v, 0x428) & 1 == 0 {
            continue;
        }
        wr::<u32>(v, 0x44, 0);
        wr::<u32>(v, 0x48, 0);
        wr::<u32>(v, 0x4C, 0);
    }
}

fn player(id: i32) -> GPtr<u8> {
    g::<u8>(PLAYERS.wrapping_add((id as isize * PLAYER_INFO_SIZE as isize) as usize))
}

/// `CPlaceable::GetHeading`: `atan2(-right.x, right.y)` with a matrix, else the placement heading.
fn heading(e: GPtr<u8>) -> X87 {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        return x(rd::<f32>(e, 0x10));
    }
    x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
}

/// `FindPlayerHeading(int32 id)` (0x56E450): the remote vehicle's heading (`Players[id]` +0xB0, `id`
/// not resolved), else the vehicle of the player (`id < 0` → `PlayerInFocus`) when in one, else the
/// heading of `Players[id]`'s ped: `id` is not resolved there either (original bug: `Players[-1]` for
/// id -1), and a null ped is dereferenced.
/// Not registered: returns the raw FPATAN ST0, modelled at 53 bits (MATH-FPTAN); L2 compares the full ST0.
pub fn find_player_heading(id: i32) -> X87 {
    let remote = rd::<GPtr<u8>>(player(id), 0xB0);
    if !remote.is_null() {
        return heading(remote);
    }
    let r = if id < 0 { i32::from(PLAYER_IN_FOCUS.get()) } else { id };
    let ped = rd::<GPtr<u8>>(player(r), 0);
    if !ped.is_null() && rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let veh = rd::<GPtr<u8>>(ped, 0x58C);
        if !veh.is_null() {
            return heading(veh);
        }
    }
    heading(rd::<GPtr<u8>>(player(id), 0))
}

/// `CPlaceable::GetPosition()`: matrix + 0x30 when there is a matrix, else +4.
fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// The squared distance as the range scans compute it: 2D `dx² + dy²`, 3D `(dx² + dz²) + dy²`.
fn dist_sq(p: GPtr<u8>, e: GPtr<u8>, two_d: u8) -> X87 {
    let ep = pos_ptr(e);
    let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(ep, 0));
    let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(ep, 4));
    let dz = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(ep, 8));
    if two_d != 0 {
        dx.fmul(dx).fadd(dy.fmul(dy))
    } else {
        dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy))
    }
}

/// Appends `e` when within range (`test ah,5; jp` skips Greater and Unordered) and below `max`.
fn add_in_range(d: X87, r2: f32, e: GPtr<u8>, count: GPtr<i16>, max: i16, out: GPtr<GPtr<u8>>) {
    if d.fcom(x(r2)).test_parity(0x05) {
        return;
    }
    let n = count.read();
    if n >= max {
        return;
    }
    if !out.is_null() {
        out.wrapping_index(i32::from(n)).write(e);
    }
    count.write(n.wrapping_add(1));
}

/// `CWorld::FindObjectsInRangeSectorList(CPtrListDoubleLink&, const CVector&, float, bool, int16*,
/// int16, CEntity**)` (0x563500): every entity not yet scanned (scan code set) within `radius` (squared,
/// stored as f32) of `point` is appended while `*count < max` (the count is not advanced past max).
#[hook(addr = 0x563500, conv = cdecl, name = "CWorld::FindObjectsInRangeSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "D9 44 24 0C 8B 44 24 04")]
pub fn find_objects_in_range_sector_list(
    list: GPtr<u8>,
    point: GPtr<u8>,
    radius: f32,
    two_d: u8,
    count: GPtr<i16>,
    max: i16,
    out: GPtr<GPtr<u8>>,
) {
    let r2 = x(radius).fmul_m(radius).to_f32();
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let sc = CURRENT_SCAN_CODE.get();
        n = rd::<GPtr<u8>>(n, 4);
        if rd::<u16>(e, 0x2C) == sc {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        add_in_range(dist_sq(point, e, two_d), r2, e, count, max, out);
    }
}

/// `CWorld::FindObjectsOfTypeInRangeSectorList(uint32 model, CPtrListDoubleLink&, const CVector&, float,
/// bool, int16*, int16, CEntity**)` (0x5635C0): as `FindObjectsInRangeSectorList`, only entities whose
/// model (+0x22, sign-extended) is `model`; the scan code is set before the model test.
#[hook(addr = 0x5635C0, conv = cdecl, name = "CWorld::FindObjectsOfTypeInRangeSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "D9 44 24 10 8B 44 24 08")]
#[allow(clippy::too_many_arguments)]
pub fn find_objects_of_type_in_range_sector_list(
    model: i32,
    list: GPtr<u8>,
    point: GPtr<u8>,
    radius: f32,
    two_d: u8,
    count: GPtr<i16>,
    max: i16,
    out: GPtr<GPtr<u8>>,
) {
    let r2 = x(radius).fmul_m(radius).to_f32();
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let sc = CURRENT_SCAN_CODE.get();
        n = rd::<GPtr<u8>>(n, 4);
        if rd::<u16>(e, 0x2C) == sc {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        if i32::from(rd::<i16>(e, 0x22)) != model {
            continue;
        }
        add_in_range(dist_sq(point, e, two_d), r2, e, count, max, out);
    }
}

/// `CWorld::IncrementCurrentScanCode` / `SetNextScanCode` (stub 0x4072E0 → 0x1570050): below 0xFFFF the
/// scan code is incremented, else `ClearScanCodes` and 1.
#[hook(addr = 0x4072E0, conv = cdecl, name = "SetNextScanCode", expect = "E9 6B 8D 16 01")]
pub fn set_next_scan_code() {
    let sc = CURRENT_SCAN_CODE.get();
    if sc < 0xFFFF {
        CURRENT_SCAN_CODE.set(sc + 1);
        return;
    }
    super::cworld::clear_scan_codes();
    CURRENT_SCAN_CODE.set(1);
}

/// `CWorld::CastShadowSectorList(CPtrListDoubleLink&, ...)` (0x563390; the shadow work is compiled out):
/// every entity not yet scanned with bit 0 of +0x1C set gets the current scan code. Only the list
/// argument is read.
#[hook(addr = 0x563390, conv = cdecl, name = "CWorld::CastShadowSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "8B 44 24 04 8B 00 85 C0")]
pub fn cast_shadow_sector_list(list: GPtr<u8>) {
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let sc = CURRENT_SCAN_CODE.get();
        n = rd::<GPtr<u8>>(n, 4);
        if rd::<u16>(e, 0x2C) != sc && rd::<u8>(e, 0x1C) & 1 != 0 {
            wr::<u16>(e, 0x2C, sc);
        }
    }
}

/// `CWorld::TestForUnusedModels(CPtrListDoubleLink&, int32* counts)` (0x5639D0): every entity not yet
/// scanned gets the current scan code and `counts[model]` (+0x22, sign-extended) is incremented.
#[hook(addr = 0x5639D0, conv = cdecl, name = "CWorld::TestForUnusedModels<CPtrListDoubleLink<CPhysical*>>-InputArray", expect = "8B 44 24 04 8B 08 85 C9")]
pub fn test_for_unused_models_list(list: GPtr<u8>, counts: GPtr<i32>) {
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let sc = CURRENT_SCAN_CODE.get();
        if rd::<u16>(e, 0x2C) != sc {
            wr::<u16>(e, 0x2C, sc);
            let c = counts.wrapping_index(i32::from(rd::<i16>(e, 0x22)));
            c.write(c.read().wrapping_add(1));
        }
        n = rd::<GPtr<u8>>(n, 4);
    }
}

/// `CRepeatSector::CRepeatSector` (0x563190, thiscall): `eh vector constructor iterator` over the three
/// lists (4 bytes each, ctor 0x563180, dtor 0x5630F0); EAX = this.
#[hook(addr = 0x563190, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "56 68 F0 30 56 00", this = "CRepeatSector")]
pub fn repeat_sector_ctor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this, 4, 3, 0x563180, 0x5630F0);
    this
}

/// `CWorld::Remove(CEntity*)` (0x563280): the entity's `Remove` (vtable +0xC), then for a physical type
/// (2..4) tail jump to `CPhysical::RemoveFromMovingList`.
#[hook(addr = 0x563280, conv = cdecl, name = "CWorld::Remove", expect = "56 8B 74 24 08")]
pub fn remove(e: GPtr<u8>) {
    entity_remove(e);
    let t = rd::<u8>(e, 0x36) & 7;
    if t > 1 && t < 5 {
        remove_from_moving_list(e);
    }
}

/// `CWorld::ProcessPedsAfterPreRender` (0x563430): unless `CTimer::bSkipProcessThisFrame` (0xB7CB89),
/// `CPedIntelligence::ProcessAfterPreRender` of every ped (type 3) in the moving list (0xB9ACC8).
#[hook(addr = 0x563430, conv = cdecl, name = "CWorld::ProcessPedsAfterPreRender", expect = "A0 89 CB B7 00")]
pub fn process_peds_after_pre_render() {
    if g::<u8>(0xB7CB89).read() != 0 {
        return;
    }
    let mut n = g::<GPtr<u8>>(0xB9ACC8).read();
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        n = rd::<GPtr<u8>>(n, 4);
        if rd::<u8>(e, 0x36) & 7 == 3 {
            process_after_pre_render(rd::<GPtr<u8>>(e, 0x47C));
        }
    }
}

/// `CRepeatSector::~CRepeatSector` (0x5631B0, thiscall): `eh vector destructor iterator` over the three
/// lists (dtor 0x5630F0).
#[hook(addr = 0x5631B0, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "68 F0 30 56 00", this = "CRepeatSector")]
pub fn repeat_sector_dtor(this: GPtr<u8>) {
    eh_vector_dtor(this, 4, 3, 0x5630F0);
}

/// `CWorld::Add(CEntity*)` (0x563220): with an RW object (+0x18) its frame matrix (`[+0x18]+4` + 0x10)
/// is updated from the matrix (`CMatrix::UpdateRwMatrix`) or the placement
/// (`CSimpleTransform::UpdateRwMatrix`), then `UpdateRwFrame`, the entity's `Add` (vtable +8), and for
/// types other than 1 / 5 that are neither static (+0x1C bit 2) nor flag 0x40000, tail jump to
/// `CPhysical::AddToMovingList`.
#[hook(addr = 0x563220, conv = cdecl, name = "CWorld::Add", expect = "56 8B 74 24 08")]
pub fn add(e: GPtr<u8>) {
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
    entity_add(e);
    let t = rd::<u8>(e, 0x36) & 7;
    if t == 1 || t == 5 {
        return;
    }
    let f = rd::<u32>(e, 0x1C);
    if f & 4 != 0 || f & 0x40000 != 0 {
        return;
    }
    add_to_moving_list(e);
}

/// `CWorld::TestForBuildingsOnTopOfEachOther(CPtrListDoubleLink&)` (0x563950): the debug body with its
/// output compiled out; it only reads every pair's positions (and the outer model) of the list.
#[hook(addr = 0x563950, conv = cdecl, name = "CWorld::TestForBuildingsOnTopOfEachOther<CPtrListDoubleLink<CPhysical*>>-", expect = "8B 44 24 04 8B 08")]
pub fn test_for_buildings_on_top_of_each_other(list: GPtr<u8>) {
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let p = pos_ptr(e);
        let _ = (rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
        let next = rd::<GPtr<u8>>(n, 4);
        if !next.is_null() {
            let _ = rd::<u16>(e, 0x22);
            let mut m = next;
            while !m.is_null() {
                let p = pos_ptr(rd::<GPtr<u8>>(m, 0));
                let _ = (rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
                m = rd::<GPtr<u8>>(m, 4);
            }
        }
        n = next;
    }
}

/// `CWorld::PrintCarChanges` (0x566420): for every vehicle slot (last first) the model of a used
/// automobile (+0x590 = 0) or 0; when it differs from the stored one (0xB7CBA8[slot]) it is printed
/// (`printf(0x864F58, slot, old, new)`) and stored.
#[hook(addr = 0x566420, conv = cdecl, name = "CWorld::PrintCarChanges", expect = "55 8B 2D 94 44 B7 00")]
pub fn print_car_changes() {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        let used = (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize - 1).read() as i8) >= 0;
        i -= 1;
        let v = rd::<GPtr<u8>>(pool, 0).byte_add(i.wrapping_mul(VEHICLE_SIZE) as isize);
        let model = if used && !v.is_null() && rd::<u32>(v, 0x590) == 0 { i32::from(rd::<i16>(v, 0x22)) } else { 0 };
        let slot = g::<i32>(0xB7CBA8).wrapping_index(i as i32);
        let old = slot.read();
        if model != old {
            printf4(g(0x864F58), i, old, model);
            slot.write(model);
        }
    }
}

/// `CWorld::FindNearestObjectOfTypeSectorList(int32 model, CPtrListDoubleLink&, const CVector&, float,
/// bool b2D, CEntity** nearest, float* dist)` (0x565450): every entity not yet scanned gets the scan code;
/// those of the model (any model when negative) strictly nearer than `*dist` (2D `sqrt(dx² + dy²)`, 3D
/// `sqrt((dz² + dx²) + dy²)`) become the nearest. The radius is not read.
#[hook(addr = 0x565450, conv = cdecl, name = "CWorld::FindNearestObjectOfTypeSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "8B 44 24 08 8B 10")]
#[allow(clippy::too_many_arguments)]
pub fn find_nearest_object_of_type_sector_list(
    model: i32,
    list: GPtr<u8>,
    point: GPtr<u8>,
    _radius: f32,
    two_d: u8,
    nearest: GPtr<GPtr<u8>>,
    dist: GPtr<f32>,
) {
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let sc = CURRENT_SCAN_CODE.get();
        n = rd::<GPtr<u8>>(n, 4);
        if rd::<u16>(e, 0x2C) == sc {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        if i32::from(rd::<i16>(e, 0x22)) != model && model >= 0 {
            continue;
        }
        let ep = pos_ptr(e);
        let dx = x(rd::<f32>(point, 0)).fsub_m(rd::<f32>(ep, 0));
        let dy = x(rd::<f32>(point, 4)).fsub_m(rd::<f32>(ep, 4));
        let dz = x(rd::<f32>(point, 8)).fsub_m(rd::<f32>(ep, 8));
        let d = if two_d != 0 {
            dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt()
        } else {
            dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).fsqrt()
        };
        // test ah,5 / jp: kept only when Less
        if !d.fcom(x(dist.read())).test_parity(0x05) {
            dist.write(d.to_f32());
            nearest.write(e);
        }
    }
}

/// `CWorld::ClearPedsFromArea(float x1, float y1, float z1, float x2, float y2, float z2)` (0x5667F0): every
/// used ped (pool 0xB74490, 0x7C4 each; the size read once) that is not a player and can be deleted,
/// inside the box (bounds included; NaN outside), is removed (`CPopulation::RemovePed`).
#[hook(addr = 0x5667F0, conv = cdecl, name = "CWorld::ClearPedsFromArea", expect = "83 EC 10 53 8B 1D 90 44 B7 00")]
pub fn clear_peds_from_area(x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) {
    let pool = g::<GPtr<u8>>(0xB74490).read();
    let size = rd::<i32>(pool, 8);
    // test ah,1 / jne skips Less and Unordered; test ah,0x41 / jp skips Greater and Unordered
    let inside = |v: f32, lo: f32, hi: f32| !x(v).fcom(x(lo)).test_any(0x01) && !x(v).fcom(x(hi)).test_parity(0x41);
    for i in 0..size.max(0) {
        if (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() as i8) < 0 {
            continue;
        }
        let ped = rd::<GPtr<u8>>(pool, 0).byte_add((i as u32).wrapping_mul(0x7C4) as isize);
        if ped.is_null() || ped_is_player(ped) != 0 || ped_can_be_deleted(ped) == 0 {
            continue;
        }
        let p = pos_ptr(ped);
        let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
        if inside(px, x1, x2) && inside(py, y1, y2) && inside(pz, z1, z2) {
            population_remove_ped(ped);
        }
    }
}

/// `CWorld::TestForUnusedModels()` (0x566510): the debug model count (into a 20000-entry stack array,
/// output compiled out): the scan code is advanced (as `SetNextScanCode`) and every not yet scanned
/// entity of the 120×120 sectors' two lists gets it. The stack array (and its unchecked model index) is
/// not reproduced.
#[hook(addr = 0x566510, conv = cdecl, name = "CWorld::TestForUnusedModels-", expect = "55 8B EC 83 E4 F8")]
pub fn test_for_unused_models() {
    set_next_scan_code();
    for y in 0..0x78usize {
        for xx in 0..0x78usize {
            let s = super::cworld::SECTORS + (y * 0x78 + xx) * 8;
            for l in [s, s + 4] {
                let mut n = g::<GPtr<u8>>(l).read();
                while !n.is_null() {
                    let e = rd::<GPtr<u8>>(n, 0);
                    let sc = CURRENT_SCAN_CODE.get();
                    if rd::<u16>(e, 0x2C) != sc {
                        wr::<u16>(e, 0x2C, sc);
                    }
                    n = rd::<GPtr<u8>>(n, 4);
                }
            }
        }
    }
}

/// The cube test of the `...IntersectingCube...` scans: the entity's bounding sphere radius (model
/// info +0x14 → col model +0x24) around its position overlaps `[min, max]` on every axis (`test ah,1 /
/// jne` skips `pos + r` Less / Unordered than min; `test ah,0x41 / jp` skips `pos - r` Greater /
/// Unordered than max). `r_first`: the addition is `r + pos` (else `pos + r`).
fn intersects_cube(e: GPtr<u8>, min: GPtr<f32>, max: GPtr<f32>, r_first: bool) -> bool {
    let p = pos_ptr(e);
    let mi = g::<GPtr<u8>>(0xA9B0C8).wrapping_index(i32::from(rd::<i16>(e, 0x22))).read();
    let r = rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24);
    for k in 0..3 {
        let c = rd::<f32>(p, k as isize * 4);
        let hi = if r_first { x(r).fadd_m(c) } else { x(c).fadd_m(r) };
        if hi.fcom(x(min.wrapping_index(k).read())).test_any(0x01) {
            return false;
        }
        if x(c).fsub_m(r).fcom(x(max.wrapping_index(k).read())).test_parity(0x41) {
            return false;
        }
    }
    true
}

/// Appends `e` to `out` while `*count < max` (`out` may be null; the count still advances).
fn append_counted(e: GPtr<u8>, count: GPtr<i16>, max: i16, out: GPtr<GPtr<u8>>) {
    let n = count.read();
    if n >= max {
        return;
    }
    if !out.is_null() {
        out.wrapping_index(i32::from(n)).write(e);
    }
    count.write(n.wrapping_add(1));
}

/// `CWorld::FindObjectsIntersectingCubeSectorList(CPtrListDoubleLink&, const CVector& min, const CVector&
/// max, int16* count, int16 maxCount, CEntity** out)` (0x5650E0): every entity not yet scanned gets the
/// scan code; those whose bounding sphere overlaps the cube are appended.
#[hook(addr = 0x5650E0, conv = cdecl, name = "CWorld::FindObjectsIntersectingCubeSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "8B 44 24 04 53 8B 18")]
pub fn find_objects_intersecting_cube_sector_list(
    list: GPtr<u8>,
    min: GPtr<f32>,
    max: GPtr<f32>,
    count: GPtr<i16>,
    max_count: i16,
    out: GPtr<GPtr<u8>>,
) {
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let sc = CURRENT_SCAN_CODE.get();
        n = rd::<GPtr<u8>>(n, 4);
        if rd::<u16>(e, 0x2C) == sc {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        if intersects_cube(e, min, max, false) {
            append_counted(e, count, max_count, out);
        }
    }
}

/// `CWorld::FindMissionEntitiesIntersectingCubeSectorList(CPtrListDoubleLink&, const CVector& min, const
/// CVector& max, int16* count, int16 maxCount, CEntity** out, bool vehicles, bool peds)` (0x565300): as
/// `FindObjectsIntersectingCubeSectorList`, only mission entities: vehicles with created-by +0x4A4 = 2,
/// else peds with +0x484 = 2, else objects of type (+0x13C) 2 or 6.
#[hook(addr = 0x565300, conv = cdecl, name = "CWorld::FindMissionEntitiesIntersectingCubeSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "8B 44 24 04 53 8B 18")]
#[allow(clippy::too_many_arguments)]
pub fn find_mission_entities_intersecting_cube_sector_list(
    list: GPtr<u8>,
    min: GPtr<f32>,
    max: GPtr<f32>,
    count: GPtr<i16>,
    max_count: i16,
    out: GPtr<GPtr<u8>>,
    vehicles: u8,
    peds: u8,
) {
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let sc = CURRENT_SCAN_CODE.get();
        n = rd::<GPtr<u8>>(n, 4);
        if rd::<u16>(e, 0x2C) == sc {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        let mission = if vehicles != 0 {
            rd::<u8>(e, 0x4A4) == 2
        } else if peds != 0 {
            rd::<u8>(e, 0x484) == 2
        } else {
            matches!(rd::<u8>(e, 0x13C), 2 | 6)
        };
        if mission && intersects_cube(e, min, max, true) {
            append_counted(e, count, max_count, out);
        }
    }
}

/// `CWorld::FindUnsuspectingTargetCar(CVector pos, CVector playerPos)` (0x566C90): of the used random
/// (+0x4A4 = 1) automobiles (+0x594 = 0) with status (+0x36 >> 3) 2 or 3, last slot first, the one
/// strictly nearest in 2D to `pos` (below 9999.9, the distance kept as f32) that the player position is in
/// front of (`(dz·f.z + dy·f.y) + dx·f.x > 0` with the matrix forward vector +0x10; the matrix is
/// dereferenced unchecked); null when none.
#[hook(addr = 0x566C90, conv = cdecl, name = "CWorld::FindUnsuspectingTargetCar", expect = "83 EC 08 D9 05 90 4F 86 00")]
pub fn find_unsuspecting_target_car(px: f32, py: f32, _pz: f32, qx: f32, qy: f32, qz: f32) -> GPtr<u8> {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut best = x(f32::from_bits(0x461C_3F9A));
    let mut best_v = GPtr::NULL;
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        let used = (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize - 1).read() as i8) >= 0;
        i -= 1;
        let v = rd::<GPtr<u8>>(pool, 0).byte_add(i.wrapping_mul(VEHICLE_SIZE) as isize);
        if !used || v.is_null() || rd::<u32>(v, 0x594) != 0 || rd::<u8>(v, 0x4A4) != 1 {
            continue;
        }
        let st = rd::<u8>(v, 0x36) >> 3;
        if st != 2 && st != 3 {
            continue;
        }
        let m = rd::<GPtr<u8>>(v, 0x14);
        let p = pos_ptr(v);
        let dx = x(px).fsub_m(rd::<f32>(p, 0));
        let dy = x(py).fsub_m(rd::<f32>(p, 4));
        let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        let d32 = d.to_f32();
        // test ah,5 / jp: a candidate only when Less
        if d.fcom(best).test_parity(0x05) {
            continue;
        }
        let ax = x(qx).fsub_m(rd::<f32>(p, 0));
        let ay = x(qy).fsub_m(rd::<f32>(p, 4));
        let az = x(qz).fsub_m(rd::<f32>(p, 8));
        let dot = az.fmul_m(rd::<f32>(m, 0x18)).fadd(ay.fmul_m(rd::<f32>(m, 0x14))).fadd(ax.fmul_m(rd::<f32>(m, 0x10)));
        // test ah,0x41 / jne: only when Greater
        if dot.fcom(X87::zero()).test_any(0x41) {
            continue;
        }
        best = x(d32);
        best_v = v;
    }
    best_v
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_REPEAT_SECTOR_DTOR,
    HOOK_ADD,
    HOOK_TEST_FOR_BUILDINGS_ON_TOP_OF_EACH_OTHER,
    HOOK_PRINT_CAR_CHANGES,
    HOOK_FIND_NEAREST_OBJECT_OF_TYPE_SECTOR_LIST,
    HOOK_CLEAR_PEDS_FROM_AREA,
    HOOK_TEST_FOR_UNUSED_MODELS,
    HOOK_FIND_OBJECTS_INTERSECTING_CUBE_SECTOR_LIST,
    HOOK_FIND_MISSION_ENTITIES_INTERSECTING_CUBE_SECTOR_LIST,
    HOOK_FIND_UNSUSPECTING_TARGET_CAR,
    HOOK_REPEAT_SECTOR_CTOR,
    HOOK_REMOVE,
    HOOK_PROCESS_PEDS_AFTER_PRE_RENDER,
    HOOK_CAST_SHADOW_SECTOR_LIST,
    HOOK_TEST_FOR_UNUSED_MODELS_LIST,
    HOOK_INITIALISE,
    HOOK_SECTOR_CTOR,
    HOOK_CAMERA_TO_IGNORE_THIS_OBJECT,
    HOOK_SET_ALL_CARS_CAN_BE_DAMAGED,
    HOOK_STOP_ALL_LAW_ENFORCERS_IN_THEIR_TRACKS,
    HOOK_FIND_OBJECTS_IN_RANGE_SECTOR_LIST,
    HOOK_FIND_OBJECTS_OF_TYPE_IN_RANGE_SECTOR_LIST,
    HOOK_SET_NEXT_SCAN_CODE,
];
