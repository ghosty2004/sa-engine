//! WRITER-MVP-D (helper-world) WORLD ports, batch `mvpd-world-3`: occlusion, area clearing and the per-frame
//! managers. Ported from the 1.0 US bytes (the exe wins over gta-reversed).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{ftol32, g, rd, wr, x};

orig! {
    /// `CEntity::GetBoundCentre(CVector&)` (0x534290, `ret 4`)
    fn entity_get_bound_centre(e: GPtr<u8>, out: GPtr<u8>) = thiscall 0x534290 => unported;
    /// `CalcScreenCoors(const CVector&, CVector*, float*, float*)` (0x71DA00, cdecl)
    fn calc_screen_coors_wh(i: GPtr<u8>, o: GPtr<u8>, w: GPtr<u8>, h: GPtr<u8>) -> u8 = cdecl 0x71DA00 => unported;
    /// `CalcScreenCoors(const CVector&, CVector*)` (0x71DAB0, cdecl)
    fn calc_screen_coors(i: GPtr<u8>, o: GPtr<u8>) -> u8 = cdecl 0x71DAB0 => unported;
    /// `CActiveOccluder::IsPointWithinOcclusionArea(float, float, float)` (0x71E580, `ret 0xC`)
    fn occluder_is_point_within_area(o: GPtr<u8>, x: f32, y: f32, r: f32) -> u8 = thiscall 0x71E580 => unported;
    /// `CActiveOccluder::IsPointBehindOccluder(CVector, float)` (0x71FA40, `ret 0x10`)
    fn occluder_is_point_behind(o: GPtr<u8>, x: f32, y: f32, z: f32, r: f32) -> u8 = thiscall 0x71FA40 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(p: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn placeable_get_matrix(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `CMatrix::operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, hidden result pointer)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
}

/// The model's collision model (`CModelInfo::ms_modelInfoPtrs[model]` +0x14).
fn col_model(e: GPtr<u8>) -> GPtr<u8> {
    let mi = g::<GPtr<u8>>(0xA9B0C8 + 4 * rd::<i16>(e, 0x22) as isize as usize).read();
    rd::<GPtr<u8>>(mi, 0x14)
}

/// The inlined `GetMatrix`: allocates and fills the matrix from the simple transform when there is none.
fn ensure_matrix(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        placeable_allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// One corner test: the corner on screen, inside the occluder's area and behind it.
fn corner_hidden(occ: GPtr<u8>, p: &mut [f32; 3]) -> bool {
    let mut q = [0f32; 3];
    if calc_screen_coors(GPtr::from_raw(p).cast(), GPtr::from_raw(&mut q).cast()) == 0 {
        return false;
    }
    if occluder_is_point_within_area(occ, q[0], q[1], 0.0) == 0 {
        return false;
    }
    occluder_is_point_behind(occ, p[0], p[1], p[2], 0.0) != 0
}

/// `matrix * v` into a fresh vector.
fn transform(m: GPtr<u8>, mut v: [f32; 3]) -> [f32; 3] {
    let mut out = [0f32; 3];
    let r = matrix_mul_vec(GPtr::from_raw(&mut out).cast(), m, GPtr::from_raw(&mut v).cast());
    [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)]
}

/// `CEntity::IsEntityOccluded()` (0x71FAE0, bool in AL): with active occluders (`NumActiveOccluders` 0xC73CC0,
/// 0xAC-byte `ActiveOccluders` at 0xC78610) and the bound centre on screen, the screen radius is `sphere radius
/// * max(w, h) * 0x858C20`. An occluder nearer than the centre's depth minus the radius (`int16` +0x78) hides
/// the entity when the centre's screen circle is within its area and the centre is behind it; otherwise, when
/// the centre point is within its area, the bounding box corners min, max, (min.x, max.y, max.z) and (max.x,
/// min.y, min.z) must all be hidden (all four are transformed; later tests are skipped once one fails); then
/// boxes not larger than 60 in x or y or not larger than 0x858CA4 in z are occluded, else (min.x, min.y,
/// max.z) and (max.x, min.y, max.z) must be hidden too.
#[hook(addr = 0x71FAE0, conv = thiscall, name = "CEntity::IsEntityOccluded", this = "CEntity", expect = "81 EC AC 00 00 00",
       route = "L2", hazards = "none")]
pub fn entity_is_entity_occluded(this: GPtr<u8>) -> u8 {
    let mut c = [0f32; 3];
    entity_get_bound_centre(this, GPtr::from_raw(&mut c).cast());
    let n = || g::<i32>(0xC73CC0).read();
    if n() == 0 {
        return 0;
    }
    let mut s = [0f32; 3];
    let (mut w, mut h) = (0f32, 0f32);
    if calc_screen_coors_wh(
        GPtr::from_raw(&mut c).cast(),
        GPtr::from_raw(&mut s).cast(),
        GPtr::from_raw(&mut w).cast(),
        GPtr::from_raw(&mut h).cast(),
    ) == 0
    {
        return 0;
    }
    let m = if x(w).fcom(x(h)).test_any(0x41) { h } else { w };
    let r = x(rd::<f32>(col_model(this), 0x24)).fmul_m(m).fmul_m(g::<f32>(0x858C20).read()).to_f32();
    let mut i = 0;
    while i < n() {
        let occ = GPtr::<u8>::from_addr(0xC78610 + 0xAC * i as usize);
        i += 1;
        let col = col_model(this);
        let depth = x(s[2]).fsub_m(rd::<f32>(col, 0x24));
        if X87::from_i32(rd::<i16>(occ, 0x78) as i32).fcom(depth).test_parity(5) {
            continue;
        }
        if occluder_is_point_within_area(occ, s[0], s[1], r) != 0
            && occluder_is_point_behind(occ, c[0], c[1], c[2], rd::<f32>(col, 0x24)) != 0
        {
            return 1;
        }
        if occluder_is_point_within_area(occ, s[0], s[1], 0.0) == 0 {
            continue;
        }
        let cv = |o: isize| rd::<f32>(col_model(this), o);
        // min, max, (min.x, max.y, max.z), (max.x, min.y, min.z): every corner is transformed, the test
        // stops at the first one that is not hidden
        let mut ok = true;
        for (a, b, c) in [(0, 4, 8), (0xC, 0x10, 0x14), (0, 0x10, 0x14), (0xC, 4, 8)] {
            let mut p = transform(ensure_matrix(this), [cv(a), cv(b), cv(c)]);
            if ok {
                ok = corner_hidden(occ, &mut p);
            }
        }
        if !ok {
            continue;
        }
        let col = col_model(this);
        let sixty = x(g::<f32>(0x858B34).read());
        let big_x = !x(rd::<f32>(col, 0xC)).fsub_m(rd::<f32>(col, 0)).fcom(sixty).test_any(0x41);
        if !big_x && x(rd::<f32>(col, 0x10)).fsub_m(rd::<f32>(col, 4)).fcom(sixty).test_any(0x41) {
            return 1;
        }
        if x(rd::<f32>(col, 0x14)).fsub_m(rd::<f32>(col, 8)).fcom(x(g::<f32>(0x858CA4).read())).test_any(0x41) {
            return 1;
        }
        let mut p = transform(placeable_get_matrix(this), [cv(0), cv(4), cv(0x14)]);
        let ok = corner_hidden(occ, &mut p);
        let mut p = transform(placeable_get_matrix(this), [cv(0xC), cv(4), cv(0x14)]);
        if ok && corner_hidden(occ, &mut p) {
            return 1;
        }
    }
    0
}

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::CanBeDeleted()` (0x5DF100)
    fn ped_can_be_deleted(p: GPtr<u8>) -> u8 = thiscall 0x5DF100 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80, cdecl)
    fn ped_groups_get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::IsFollower(const CPed*)` (0x5F69E0, `ret 4`)
    fn membership_is_follower(m: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x5F69E0 => unported;
    /// `CPopulation::RemovePed(CPed*)` (0x610F20, cdecl)
    fn population_remove_ped(p: GPtr<u8>) = cdecl 0x610F20 => unported;
    /// `CVehicle::CanBeDeleted()` (0x6D1180)
    fn vehicle_can_be_deleted(v: GPtr<u8>) -> u8 = thiscall 0x6D1180 => unported;
    /// `CGarages::IsPointWithinHideOutGarage(const CVector&)` (0x448900, cdecl)
    fn garages_is_point_within_hide_out_garage(p: GPtr<u8>) -> u8 = cdecl 0x448900 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CVehicle::RemovePassenger(CPed*)` (0x6D1610, `ret 4`)
    fn vehicle_remove_passenger(v: GPtr<u8>, p: GPtr<u8>) = thiscall 0x6D1610 => unported;
    /// `CCarCtrl::IsThisVehicleInteresting(CVehicle*)` (0x423EA0, cdecl)
    fn car_ctrl_is_this_vehicle_interesting(v: GPtr<u8>) -> u8 = cdecl 0x423EA0 => unported;
    /// `CGarages::StoreCarInNearestImpoundingGarage(CVehicle*)` (0x44A3C0, cdecl)
    fn garages_store_car_in_nearest_impounding_garage(v: GPtr<u8>) = cdecl 0x44A3C0 => unported;
    /// `CCarCtrl::RemoveFromInterestingVehicleList(CVehicle*)` (0x423ED0, cdecl)
    fn car_ctrl_remove_from_interesting_vehicle_list(v: GPtr<u8>) = cdecl 0x423ED0 => unported;
    /// entity virtual +0xC (`Remove`)
    fn v_remove(e: GPtr<u8>) = thiscall vslot 0xC => unported;
    /// entity virtual +0x0 (scalar deleting destructor)
    fn v_delete(e: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CPhysical::RemoveFromMovingList()` (0x542860)
    fn remove_from_moving_list(e: GPtr<u8>) = thiscall 0x542860 => crate::physical::base::remove_from_moving_list;
    /// `CObject::DeleteAllTempObjectsInArea(CVector, float)` (0x5A1980, cdecl)
    fn object_delete_all_temp_objects_in_area(x: f32, y: f32, z: f32, r: f32) = cdecl 0x5A1980 => unported;
    /// `CFireManager::ExtinguishPoint(CVector, float)` (0x539450, `ret 0x10`)
    fn fire_manager_extinguish_point(fm: GPtr<u8>, x: f32, y: f32, z: f32, r: f32) = thiscall 0x539450 => unported;
    /// `CWorld::ExtinguishAllCarFiresInArea(CVector, float)` (0x566950, cdecl)
    fn world_extinguish_all_car_fires_in_area(x: f32, y: f32, z: f32, r: f32) = cdecl 0x566950 => unported;
    /// `CExplosion::RemoveAllExplosionsInArea(CVector, float)` (0x7369E0, cdecl)
    fn explosion_remove_all_explosions_in_area(x: f32, y: f32, z: f32, r: f32) = cdecl 0x7369E0 => unported;
    /// `CProjectileInfo::RemoveAllProjectiles()` (0x7399B0, cdecl)
    fn projectile_info_remove_all_projectiles() = cdecl 0x7399B0 => unported;
    /// `CShadows::TidyUpShadows()` (0x707770, cdecl)
    fn shadows_tidy_up_shadows() = cdecl 0x707770 => unported;
    /// `CPickups::RemoveUnnecessaryPickups(const CVector&, float)` (0x4563A0, cdecl)
    fn pickups_remove_unnecessary_pickups(p: GPtr<u8>, r: f32) = cdecl 0x4563A0 => unported;
}

/// The entity's position (matrix +0x30 or the simple transform at +4).
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `r * r > dx * dx + dy * dy` in the original order (`(pos.x - p.x)^2 + (pos.y - p.y)^2` summed as `x + y`
/// after squaring y first); NaN is outside.
fn within_2d(pos: GPtr<u8>, p: GPtr<u8>, r: f32) -> bool {
    let a = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(p, 4));
    let b = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(p, 0));
    let s = b.fmul(b).fadd(a.fmul(a));
    !x(r).fmul_m(r).fcom(s).test_any(0x41)
}

/// `CWorld::ClearExcitingStuffFromArea(const CVector& point, float radius, bool removeProjectilesAndShadows)`
/// (0x56A0D0, cdecl): removes (`CPopulation::RemovePed`) deletable non-player peds within the 2D radius that
/// are not followers of the player's group; deletes vehicles within it that carry no follower, are not the
/// player's last vehicle (`player +0x584`) of class 5, are not flagged `+0x428 & 8`, can be deleted and are not
/// in a hide-out garage (driver and passengers removed, interesting cars impounded and dropped from the list,
/// `Remove`, off the moving list, deleted); then temp objects, fires, car fires and explosions in the area are
/// cleared, optionally all projectiles and shadows, and unnecessary pickups removed. Both pool counts are read
/// once.
#[hook(addr = 0x56A0D0, conv = cdecl, name = "CWorld::ClearExcitingStuffFromArea", expect = "83 EC 1C E9 77 6F E9 FF",
       route = "L2", hazards = "heap-writes",
       note = "the vehicle pool pointer is read through an obfuscated SecuROM fragment (0x40104F)")]
pub fn world_clear_exciting_stuff_from_area(point: GPtr<u8>, r: f32, projectiles: u8) {
    let vehs = g::<GPtr<u8>>(0xB74494).read();
    let peds = g::<GPtr<u8>>(0xB74490).read();
    let live = |pool: GPtr<u8>, i: i32, size: isize| -> Option<GPtr<u8>> {
        if (rd::<u8>(rd::<GPtr<u8>>(pool, 4), i as isize) as i8) < 0 {
            return None;
        }
        let e = rd::<GPtr<u8>>(pool, 0).byte_add(size * i as isize);
        if e.is_null() { None } else { Some(e) }
    };
    let n = rd::<i32>(peds, 8);
    for i in 0..n.max(0) {
        let Some(p) = live(peds, i, 0x7C4) else { continue };
        if ped_is_player(p) != 0 || ped_can_be_deleted(p) == 0 || !within_2d(ent_pos(p), point, r) {
            continue;
        }
        let grp = ped_groups_get_peds_group(find_player_ped(-1));
        if !grp.is_null() && membership_is_follower(grp.byte_add(8), p) != 0 {
            continue;
        }
        population_remove_ped(p);
    }
    let n = rd::<i32>(vehs, 8);
    'veh: for i in 0..n.max(0) {
        let Some(v) = live(vehs, i, 0xA18) else { continue };
        let grp = ped_groups_get_peds_group(find_player_ped(-1));
        let mut j = 0;
        while j < rd::<u8>(v, 0x488) as i32 {
            let pass = rd::<GPtr<u8>>(v, 0x464 + 4 * j as isize);
            if !pass.is_null() && !grp.is_null() && membership_is_follower(grp.byte_add(8), pass) != 0 {
                continue 'veh;
            }
            j += 1;
        }
        if v == rd::<GPtr<u8>>(find_player_ped(-1), 0x584) && rd::<u32>(v, 0x590) == 5 {
            continue;
        }
        if !within_2d(ent_pos(v), point, r) || rd::<u8>(v, 0x428) & 8 != 0 || vehicle_can_be_deleted(v) == 0 {
            continue;
        }
        if garages_is_point_within_hide_out_garage(ent_pos(v)) != 0 {
            continue;
        }
        let driver = rd::<GPtr<u8>>(v, 0x460);
        if !driver.is_null() {
            population_remove_ped(driver);
            let d = rd::<GPtr<u8>>(v, 0x460);
            if !d.is_null() {
                entity_clean_up_old_reference(d, v.byte_add(0x460));
            }
            wr::<u32>(v, 0x460, 0);
        }
        let mut j = 0;
        while j < rd::<u8>(v, 0x488) as i32 {
            let pass = rd::<GPtr<u8>>(v, 0x464 + 4 * j as isize);
            if !pass.is_null() {
                vehicle_remove_passenger(v, pass);
                population_remove_ped(pass);
            }
            j += 1;
        }
        if car_ctrl_is_this_vehicle_interesting(v) != 0 {
            garages_store_car_in_nearest_impounding_garage(v);
        }
        car_ctrl_remove_from_interesting_vehicle_list(v);
        v_remove(v);
        let t = rd::<u8>(v, 0x36) & 7;
        if t > 1 && t < 5 {
            remove_from_moving_list(v);
        }
        v_delete(v, 1);
    }
    let (px, py, pz) = (rd::<f32>(point, 0), rd::<f32>(point, 4), rd::<f32>(point, 8));
    object_delete_all_temp_objects_in_area(px, py, pz, r);
    let (px, py, pz) = (rd::<f32>(point, 0), rd::<f32>(point, 4), rd::<f32>(point, 8));
    fire_manager_extinguish_point(GPtr::from_addr(0xB71F80), px, py, pz, r);
    let (px, py, pz) = (rd::<f32>(point, 0), rd::<f32>(point, 4), rd::<f32>(point, 8));
    world_extinguish_all_car_fires_in_area(px, py, pz, r);
    let (px, py, pz) = (rd::<f32>(point, 0), rd::<f32>(point, 4), rd::<f32>(point, 8));
    explosion_remove_all_explosions_in_area(px, py, pz, r);
    if projectiles != 0 {
        projectile_info_remove_all_projectiles();
        shadows_tidy_up_shadows();
    }
    pickups_remove_unnecessary_pickups(point, r);
}

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70, cdecl)
    fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390, cdecl)
    fn game_logic_is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CQuadTreeNode::GetAllMatching(const CRect&, CPtrListSingleLink&)` (0x5528C0, `ret 8`)
    fn quad_tree_get_all_matching_rect(t: GPtr<u8>, r: GPtr<u8>, l: GPtr<u8>) = thiscall 0x5528C0 => unported;
    /// `CQuadTreeNode::GetAllMatching(const CVector2D&, CPtrListSingleLink&)` (0x552930, `ret 8`)
    fn quad_tree_get_all_matching_vec(t: GPtr<u8>, v: GPtr<u8>, l: GPtr<u8>) = thiscall 0x552930 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float, const CMatrix*)` (0x420C40, `ret 0xC`)
    fn camera_is_sphere_visible(c: GPtr<u8>, p: GPtr<u8>, r: f32, m: GPtr<u8>) -> u8 = thiscall 0x420C40 => unported;
    /// `CClock::GetIsTimeInRange(uint8, uint8)` (0x52CEE0, cdecl)
    fn clock_get_is_time_in_range(a: u32, b: u32) -> u8 = cdecl 0x52CEE0 => unported;
    /// `C3dMarkers::PlaceMarkerCone(uint32 id, const CVector&, float size, uint8 r, g, b, a, uint16 period,
    /// float fraction, int16 type, bool rotate)` (0x726D40, cdecl)
    fn markers_place_marker_cone(id: GPtr<u8>, p: GPtr<u8>, size: f32, r: u32, g: u32, b: u32, a: u32, period: u32, frac: f32, ty: u32, rot: u32) = cdecl 0x726D40 => unported;
    /// `CPtrListSingleLink::Flush()` (0x552400)
    fn single_list_flush(l: GPtr<u8>) = thiscall 0x552400 => unported;
    /// `CEntryExit::TransitionFinished(CPed*)` (0x4404A0, `ret 4`)
    fn entry_exit_transition_finished(ee: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4404A0 => unported;
    /// `CEntryExit::IsInArea(const CVector&)` (0x43E460, `ret 4`)
    fn entry_exit_is_in_area(ee: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x43E460 => unported;
    /// `CEntryExit::TransitionStarted(CPed*)` (0x43FFD0, `ret 4`)
    fn entry_exit_transition_started(ee: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x43FFD0 => unported;
}

/// The camera position the original reads (`TheCamera` entity +0x30 when set, else 0xB6F02C).
fn cam_position() -> GPtr<u8> {
    let e = g::<GPtr<u8>>(0xB6F03C).read();
    if e.is_null() { GPtr::from_addr(0xB6F02C) } else { e.byte_add(0x30) }
}

/// `CEntryExitManager::Update()` (0x440D10, cdecl, SEH frame for the local lists).
///
/// Markers are blocked during cutscene processing (0xB5F852), with pad 0 +0x10E set, in a coop game, in
/// replay playback or with 0x96A7C8 set. In state 0 (0x96A7CC) the entry-exits in the camera rect (camera
/// position plus `0xB6F9AC/0xB6F9B0 * k`, +-k, `k` = 0x858CA4) of the quad tree 0x96A7D0 that are enabled
/// (+0x31 bit 6), whose area matches (linked +0x38 ones in the current area 0xB72914, unlinked ones outside it),
/// visible (camera matrices 0xB6FA74 or, with 0xB6F998, 0xB6FABC, radius 1), not blocked, in their time range
/// (+0x34/+0x35) and within the 2D distance limit 0x859600 (squared) get a marker cone at their centre
/// (z + 1). Then a running transition (0x96A7D4) is finished, or the player's (vehicle's) position starts the
/// transition of the first enabled entry-exit containing it (unless blocked); with none found, state 3 becomes
/// 4 and any other state 0.
#[hook(addr = 0x440D10, conv = cdecl, name = "CEntryExitManager::Update", expect = "64 A1 00 00 00 00",
       route = "L2", hazards = "heap-writes",
       note = "the original's SEH frame only guards the local lists' destructors")]
pub fn entry_exit_manager_update() {
    let blocked = g::<u8>(0xB5F852).read() != 0
        || rd::<u16>(pad_get_pad(0), 0x10E) != 0
        || game_logic_is_coop_game_going_on() != 0
        || g::<u8>(0xA43088).read() == 1
        || g::<u8>(0x96A7C8).read() != 0;
    let tree = || g::<GPtr<u8>>(0x96A7D0).read();
    let cam_obj = GPtr::<u8>::from_addr(0xB6F028);
    if g::<u32>(0x96A7CC).read() == 0 {
        let mut list: u32 = 0;
        let k = g::<f32>(0x858CA4).read();
        let y = x(g::<f32>(0xB6F9B0).read()).fmul_m(k);
        let v0 = x(g::<f32>(0xB6F9AC).read()).fmul_m(k).to_f32();
        let cp = cam_position();
        let y = y.fadd_m(rd::<f32>(cp, 4));
        let xx = x(v0).fadd_m(rd::<f32>(cp, 0));
        let mut rect = [xx.fsub_m(k).to_f32(), y.fadd_m(k).to_f32(), xx.fadd_m(k).to_f32(), y.fsub_m(k).to_f32()];
        let lp = GPtr::from_raw(&mut list).cast::<u8>();
        quad_tree_get_all_matching_rect(tree(), GPtr::from_raw(&mut rect).cast(), lp);
        let mut node = GPtr::<u8>::from_addr(list as usize);
        while !node.is_null() {
            let ee = rd::<GPtr<u8>>(node, 0);
            node = rd::<GPtr<u8>>(node, 4);
            if rd::<u8>(ee, 0x31) & 0x40 == 0 {
                continue;
            }
            let linked = rd::<u32>(ee, 0x38) != 0;
            let here = rd::<u8>(ee, 0x32) as u32 == g::<u32>(0xB72914).read();
            if linked != here {
                continue;
            }
            let half = g::<f32>(0x858B8C).read();
            let mut s = [
                x(rd::<f32>(ee, 0x10)).fadd_m(rd::<f32>(ee, 8)).fmul_m(half).to_f32(),
                x(rd::<f32>(ee, 0x14)).fadd_m(rd::<f32>(ee, 0xC)).fmul_m(half).to_f32(),
                rd::<f32>(ee, 0x18),
            ];
            let sp = GPtr::from_raw(&mut s).cast::<u8>();
            if camera_is_sphere_visible(cam_obj, sp, 1.0, GPtr::from_addr(0xB6FA74)) == 0
                && (g::<u8>(0xB6F998).read() == 0
                    || camera_is_sphere_visible(cam_obj, sp, 1.0, GPtr::from_addr(0xB6FABC)) == 0)
            {
                continue;
            }
            if blocked || clock_get_is_time_in_range(rd::<u8>(ee, 0x34) as u32, rd::<u8>(ee, 0x35) as u32) == 0 {
                continue;
            }
            let cp = cam_position();
            let a = x(rd::<f32>(sp, 4)).fsub_m(rd::<f32>(cp, 4));
            let b = x(rd::<f32>(sp, 0)).fsub_m(rd::<f32>(cp, 0));
            if b.fmul(b).fadd(a.fmul(a)).fcom(x(g::<f32>(0x859600).read())).test_parity(5) {
                continue;
            }
            wr::<f32>(sp, 8, x(rd::<f32>(sp, 8)).fadd_m(g::<f32>(0x858624).read()).to_f32());
            markers_place_marker_cone(ee, sp, 2.0, 0xFF, 0xFF, 0, 0xFF, 0x400, 1.0, 5, 0);
        }
        single_list_flush(lp);
    }
    let ped = find_player_ped(-1);
    let cur = g::<GPtr<u8>>(0x96A7D4);
    if !cur.read().is_null() {
        if entry_exit_transition_finished(cur.read(), ped) != 0 {
            cur.write(GPtr::NULL);
        }
        return;
    }
    let mut list: u32 = 0;
    let src = if rd::<u32>(ped, 0x46C) & 0x100 != 0 { rd::<GPtr<u8>>(ped, 0x58C) } else { ped };
    let pos = ent_pos(src);
    let mut p = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
    let mut xy = [p[0], p[1]];
    let lp = GPtr::from_raw(&mut list).cast::<u8>();
    quad_tree_get_all_matching_vec(tree(), GPtr::from_raw(&mut xy).cast(), lp);
    let mut found = false;
    let mut node = GPtr::<u8>::from_addr(list as usize);
    while !node.is_null() {
        let ee = rd::<GPtr<u8>>(node, 0);
        if entry_exit_is_in_area(ee, GPtr::from_raw(&mut p).cast()) != 0 && rd::<u8>(ee, 0x31) & 0x40 != 0 {
            found = true;
            if !blocked && entry_exit_transition_started(ee, ped) != 0 {
                cur.write(ee);
                single_list_flush(lp);
                return;
            }
        }
        node = rd::<GPtr<u8>>(node, 4);
    }
    if !found {
        let st = g::<u32>(0x96A7CC);
        st.write(if st.read() == 3 { 4 } else { 0 });
    }
    single_list_flush(lp);
}

orig! {
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0, cdecl)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CCamera::RestoreWithJumpCut()` (0x50BAB0)
    fn camera_restore_with_jump_cut(c: GPtr<u8>) = thiscall 0x50BAB0 => unported;
    /// `CStats::IncrementStat(eStats, float)` (0x55C180, cdecl)
    fn stats_increment_stat(s: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float volume, float speed)` (0x506EA0, `ret 0xC`)
    fn audio_report_frontend_audio_event(a: GPtr<u8>, ev: u32, vol: f32, speed: f32) = thiscall 0x506EA0 => unported;
    /// `CText::Get(const char*)` (0x6A0050, `ret 4`)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CMessages::AddBigMessageQ(const char*, uint32, eMessageStyle)` (0x69F370, cdecl)
    fn messages_add_big_message_q(s: GPtr<u8>, t: u32, style: u32) = cdecl 0x69F370 => unported;
    /// `CHud::SetHelpMessage(const char*, bool, bool, bool)` (0x588BE0, cdecl)
    fn hud_set_help_message(s: GPtr<u8>, a: u32, b: u32, c: u32) = cdecl 0x588BE0 => unported;
    /// `CMessages::AddBigMessageWithNumber(const char*, uint32, eMessageStyle, int32 x6)` (0x69E5F0, cdecl)
    fn messages_add_big_message_with_number(s: GPtr<u8>, t: u32, style: u32, n1: i32, n2: i32, n3: i32, n4: i32, n5: i32, n6: i32) = cdecl 0x69E5F0 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn vehicle_get_vehicle_appearance(v: GPtr<u8>) -> u32 = thiscall 0x6D1080 => unported;
    /// `CVector::Magnitude()` (0x4082C0, float in ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `thunk_FUN_0156acc0(int32)` (0x49C3B0, `ret 4`): the stunt-jump pool's slot `i`
    fn stunt_jump_pool_at(pool: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x49C3B0 => unported;
    /// `CBoundingBox::IsPointWithin(const CVector&)` (0x49C300, `ret 4`)
    fn bounding_box_is_point_within(b: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x49C300 => unported;
    /// `CCamera::SetCamPositionForFixedMode(const CVector&, const CVector&)` (0x50BEC0, `ret 8`)
    fn camera_set_cam_position_for_fixed_mode(c: GPtr<u8>, p: GPtr<u8>, up: GPtr<u8>) = thiscall 0x50BEC0 => unported;
    /// `CCamera::TakeControl(CEntity*, eCamMode, eSwitchType, int32)` (0x50C7C0, `ret 0x10`)
    fn camera_take_control(c: GPtr<u8>, e: GPtr<u8>, mode: u32, sw: u32, who: u32) = thiscall 0x50C7C0 => unported;
    /// `CVehicle::PickRandomPassenger()` (0x6D2A10)
    fn vehicle_pick_random_passenger(v: GPtr<u8>) -> GPtr<u8> = thiscall 0x6D2A10 => unported;
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, off: u32, vol: f32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
}

/// `CStuntJumpManager` statics: pool 0xA9A888, active jump 0xA9A88C, active flag 0xA9A890, hit-reward flag
/// 0xA9A891, timer 0xA9A894, state 0xA9A898, jump count 0xA9A89C, completed count 0xA9A8A0.
const SJ_ACTIVE: usize = 0xA9A88C;
const SJ_TIMER: usize = 0xA9A894;
const SJ_STATE: usize = 0xA9A898;

/// `_ftol(timestep * 0.02 * 1000)`: the frame time in ms.
fn frame_ms() -> i32 {
    ftol32(x(g::<f32>(0xB7CB5C).read()).fmul_m(g::<f32>(0x858B38).read()).fmul_m(g::<f32>(0x858C4C).read()))
}

/// `CStuntJumpManager::Update()` (0x49C490, cdecl): nothing without a pool or in replay playback. State 0
/// (jumps enabled 0xA9A890): the player driving his own airborne vehicle (+0xB9 = 0, not appearance 3/4/5, speed
/// `|move| * 0x858B40 >= 0x858BA4`) inside a jump's start box (pool slots by `thunk_FUN_0156acc0`, all 256
/// visited) starts it: state 1, timer 0, reward flag clear, first-time stat 0x90, time scale 0.3, fixed camera
/// at the jump's +0x30 and `TakeControl(vehicle, 15, 2, 1)`. State 1: landing (+0xB9 set after 100 ms) or
/// failing (player info +0xDC, out of the vehicle, wrecked status, +0x42B bit 6 or +0x40 bit 8) moves to
/// state 2 with the timer restarted; the landing box (+0x18) sets the reward flag; passing 1000 ms makes a random
/// passenger say 0x25. State 2: after 300 ms the camera is restored (time scale 1); a first rewarded jump marks
/// +0x40, counts it (stat 0x91), pays `+0x3C` (10000 for the last one) into the player's money (+0xB8) with
/// frontend sound 0x2B and messages; then state 0 and no active jump.
#[hook(addr = 0x49C490, conv = cdecl, name = "CStuntJumpManager::Update", expect = "A1 88 A8 A9 00",
       route = "L2", hazards = "heap-writes")]
pub fn stunt_jump_manager_update() {
    if g::<GPtr<u8>>(0xA9A888).read().is_null() || g::<u8>(0xA43088).read() == 1 {
        return;
    }
    let ped = find_player_ped(-1);
    let pinfo = GPtr::<u8>::from_addr(0xB7CD98 + 0x190 * g::<u8>(0xB7CD74).read() as usize);
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    let state = g::<u32>(SJ_STATE);
    let active = g::<GPtr<u8>>(SJ_ACTIVE);
    let timer = g::<i32>(SJ_TIMER);
    let text = GPtr::<u8>::from_addr(0xC1B340);
    let cam = GPtr::<u8>::from_addr(0xB6F028);
    match state.read() {
        0 => {
            if g::<u8>(0xA9A890).read() == 0
                || rd::<u8>(pinfo, 0xDC) != 0
                || rd::<u32>(ped, 0x46C) & 0x100 == 0
                || veh.is_null()
                || vehicle_get_vehicle_appearance(veh) == 4
                || rd::<GPtr<u8>>(veh, 0x460) != ped
                || vehicle_get_vehicle_appearance(veh) == 5
                || vehicle_get_vehicle_appearance(veh) == 3
                || rd::<u8>(veh, 0xB9) != 0
            {
                return;
            }
            let sp = X87::from_f64_exact(vector_magnitude(veh.byte_add(0x44))).fmul_m(g::<f32>(0x858B40).read());
            if !sp.fcom(x(g::<f32>(0x858BA4).read())).test_parity(5) {
                return;
            }
            let mut pool = g::<GPtr<u8>>(0xA9A888).read();
            for i in 0..0x100 {
                if (rd::<u8>(rd::<GPtr<u8>>(pool, 4), i as isize) as i8) < 0 {
                    continue;
                }
                let jump = stunt_jump_pool_at(pool, i);
                if bounding_box_is_point_within(jump, ent_pos(veh)) == 0 {
                    continue;
                }
                state.write(1);
                active.write(jump);
                timer.write(0);
                g::<u8>(0xA9A891).write(0);
                if rd::<u8>(jump, 0x41) == 0 {
                    wr::<u8>(jump, 0x41, 1);
                    stats_increment_stat(0x90, 1.0);
                }
                let mut up = [0f32; 3];
                g::<f32>(0xB7CB64).write(f32::from_bits(0x3E99_999A));
                camera_set_cam_position_for_fixed_mode(cam, active.read().byte_add(0x30), GPtr::from_raw(&mut up).cast());
                camera_take_control(cam, veh, 0xF, 2, 1);
                pool = g::<GPtr<u8>>(0xA9A888).read();
            }
        }
        1 => {
            let a = active.read();
            if a.is_null() {
                state.write(0);
                return;
            }
            let mut fail = false;
            let landed = rd::<u8>(veh, 0xB9) != 0 && timer.read() >= 100;
            if !landed {
                if rd::<u8>(pinfo, 0xDC) != 0 {
                    fail = true;
                }
                if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
                    fail = true;
                }
                if rd::<u8>(veh, 0x36) & 0xF8 == 0x28 || rd::<u8>(veh, 0x42B) & 0x40 != 0 || rd::<u32>(veh, 0x40) & 0x100 != 0 {
                    fail = true;
                }
                if bounding_box_is_point_within(a.byte_add(0x18), ent_pos(veh)) != 0 {
                    g::<u8>(0xA9A891).write(1);
                }
            }
            let old = if landed || fail {
                state.write(2);
                0
            } else {
                timer.read()
            };
            let now = old.wrapping_add(frame_ms());
            timer.write(now);
            if now > 1000 && old <= 1000 && !find_player_vehicle(-1, 0).is_null() {
                let p = vehicle_pick_random_passenger(find_player_vehicle(-1, 0));
                if !p.is_null() {
                    ped_say(p, 0x25, 0, 1.0, 0, 0, 0);
                }
            }
        }
        2 => {
            let t = timer.read().wrapping_add(frame_ms());
            timer.write(t);
            if t < 0x12C {
                return;
            }
            g::<f32>(0xB7CB64).write(1.0);
            camera_restore_with_jump_cut(cam);
            let a = active.read();
            if g::<u8>(0xA9A891).read() != 0 && rd::<u8>(a, 0x40) == 0 {
                wr::<u8>(a, 0x40, 1);
                let done = g::<i32>(0xA9A8A0);
                done.write(done.read().wrapping_add(1));
                stats_increment_stat(0x91, 1.0);
                let reward = if done.read() == g::<i32>(0xA9A89C).read() { 10000 } else { rd::<i32>(active.read(), 0x3C) };
                wr::<i32>(pinfo, 0xB8, rd::<i32>(pinfo, 0xB8).wrapping_add(reward));
                audio_report_frontend_audio_event(GPtr::from_addr(0xB6BC90), 0x2B, 0.0, 1.0);
                let s = text_get(text, GPtr::from_addr(0x85A57C));
                if !s.is_null() {
                    messages_add_big_message_q(s, 0x1388, 4);
                }
                if g::<i32>(0xA9A8A0).read() == g::<i32>(0xA9A89C).read() {
                    let s = text_get(text, GPtr::from_addr(0x85A574));
                    if !s.is_null() {
                        hud_set_help_message(s, 0, 0, 0);
                    }
                }
                let s = text_get(text, GPtr::from_addr(0x85A56C));
                if !s.is_null() {
                    messages_add_big_message_with_number(s, 0x1770, 5, reward, -1, -1, -1, -1, -1);
                }
            }
            state.write(0);
            active.write(GPtr::NULL);
        }
        _ => {}
    }
}

orig! {
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn task_manager_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// task virtual +0x10 (`GetTaskType`)
    fn v_task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `InteriorManager_c::GetVisibleEffects(InteriorEffectInfo_t*, int32 max)` (0x598D80, `ret 8`)
    fn interior_manager_get_visible_effects(m: GPtr<u8>, buf: GPtr<u8>, max: i32) -> i32 = thiscall 0x598D80 => unported;
    /// `InteriorManager_c::PruneVisibleEffects(InteriorEffectInfo_t*, int32 n, int32 max, float dist)` (0x598A60)
    fn interior_manager_prune_visible_effects_o(m: GPtr<u8>, buf: GPtr<u8>, n: i32, max: i32, d: f32) = thiscall 0x598A60 => crate::world::mvpd_w3::interior_manager_prune_visible_effects;
    /// `InteriorGroup_c::Exit()` (0x595290)
    fn interior_group_exit(gr: GPtr<u8>) = thiscall 0x595290 => unported;
    /// `FUN_004a8e30(ListItem_c*)` (0x4A8E30, `ret 4`): `List_c::RemoveItem`
    fn list_remove_item(l: GPtr<u8>, i: GPtr<u8>) = thiscall 0x4A8E30 => unported;
    /// `FUN_004a8df0(ListItem_c*)` (0x4A8DF0, `ret 4`): `List_c::AddItem`
    fn list_add_item(l: GPtr<u8>, i: GPtr<u8>) = thiscall 0x4A8DF0 => unported;
    /// `FUN_004a8e70()` (0x4A8E70): `List_c::RemoveHead`
    fn list_remove_head(l: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8E70 => unported;
    /// `InteriorGroup_c::Init(CEntity*, int32)` (0x5947E0, `ret 8`)
    fn interior_group_init(gr: GPtr<u8>, e: GPtr<u8>, id: i32) = thiscall 0x5947E0 => unported;
    /// `Interior_c::Init(C2dEffect*)` (0x593BF0, `ret 4`)
    fn interior_init(i: GPtr<u8>, fx: GPtr<u8>) = thiscall 0x593BF0 => unported;
    /// `InteriorGroup_c::AddInterior(Interior_c*)` (0x594840, `ret 4`)
    fn interior_group_add_interior(gr: GPtr<u8>, i: GPtr<u8>) = thiscall 0x594840 => unported;
    /// `InteriorGroup_c::Setup()` (0x595320)
    fn interior_group_setup(gr: GPtr<u8>) = thiscall 0x595320 => unported;
    /// `InteriorGroup_c::Update()` (0x5968E0)
    fn interior_group_update(gr: GPtr<u8>) = thiscall 0x5968E0 => unported;
}

/// One `InteriorEffectInfo_t` (0x50 bytes): entity +0, effect count +4, effects +8 (8), ids +0x28 (8), squared
/// distance +0x48, pruned flag +0x4C.
const IEI: isize = 0x50;

/// `InteriorManager_c::Update()` (0x598F50, bool in AL): nothing (false) while disabled (+0x4750). In an interior
/// (player ped area +0x2F) with interiors enabled (+0x4751) and an active task other than 0x2C5, up to 32 visible
/// effects are gathered and pruned (max 8 effects, 20 units). Active groups (+0x428C) no longer matching an
/// unpruned visible effect (entity and the first effect's +0x11 id) exit and go back to the free list (+0x4298);
/// each unpruned visible effect without a group gets one (`Init(entity, id)`, +0x30 from +0x473C) with one
/// interior per effect from the free interiors (+0x3CA0): seed `_ftol(z*y*x of the entity position) + id`,
/// effect +0x10 at +0x14, area, group; then `Setup` and the time stamp +0x4754. Returns whether any interior was
/// created, after updating every active group.
#[hook(addr = 0x598F50, conv = thiscall, name = "InteriorManager_c::Update", this = "InteriorManager_c", expect = "81 EC 24 0A 00 00",
       unverified = "mvpd-world-3", route = "L2", hazards = "heap-writes")]
pub fn interior_manager_update(this: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x4750) != 0 {
        return 0;
    }
    let mut created = 0u8;
    let mut buf = [0u8; 0xA00];
    let fx = GPtr::from_raw(&mut buf).cast::<u8>();
    let mut n = 0;
    let ped = find_player_ped(-1);
    if rd::<u8>(ped, 0x2F) != 0 && rd::<u8>(this, 0x4751) != 0 {
        let tm = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
        if !task_manager_get_active_task(tm).is_null() {
            let t = task_manager_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
            if v_task_get_type(t) != 0x2C5 {
                n = interior_manager_get_visible_effects(this, fx, 0x20);
            }
        }
    }
    interior_manager_prune_visible_effects_o(this, fx, n, 8, 20.0);
    let active = this.byte_add(0x428C);
    let free_groups = this.byte_add(0x4298);
    let matches = |gr: GPtr<u8>, e: GPtr<u8>| {
        rd::<GPtr<u8>>(gr, 8) == rd::<GPtr<u8>>(e, 0) && rd::<u8>(gr, 0xD) == rd::<u8>(rd::<GPtr<u8>>(e, 8), 0x11)
    };
    let mut gr = rd::<GPtr<u8>>(active, 0);
    while !gr.is_null() {
        let next = rd::<GPtr<u8>>(gr, 4);
        let keep = (0..n).any(|k| {
            let e = fx.byte_add(IEI * k as isize);
            matches(gr, e) && rd::<u8>(e, 0x4C) == 0
        });
        if !keep {
            interior_group_exit(gr);
            list_remove_item(active, gr);
            list_add_item(free_groups, gr);
        }
        gr = next;
    }
    for j in 0..n.max(0) {
        let e = fx.byte_add(IEI * j as isize);
        if rd::<u8>(e, 0x4C) != 0 {
            continue;
        }
        let mut found = false;
        let mut gr = rd::<GPtr<u8>>(active, 0);
        while !gr.is_null() {
            if matches(gr, e) {
                found = true;
            }
            gr = rd::<GPtr<u8>>(gr, 4);
        }
        if found {
            continue;
        }
        let gr = list_remove_head(free_groups);
        let ent = rd::<GPtr<u8>>(e, 0);
        interior_group_init(gr, ent, rd::<u8>(rd::<GPtr<u8>>(e, 8), 0x11) as i8 as i32);
        wr::<u32>(gr, 0x30, rd::<u32>(this, 0x473C));
        list_add_item(active, gr);
        let cnt = rd::<i32>(e, 4);
        for i in 0..cnt.max(0) {
            let intr = list_remove_head(this.byte_add(0x3CA0));
            if intr.is_null() {
                continue;
            }
            let p = ent_pos(ent);
            let seed = ftol32(x(rd::<f32>(p, 8)).fmul_m(rd::<f32>(p, 4)).fmul_m(rd::<f32>(p, 0)));
            let f = rd::<GPtr<u8>>(e, 8 + 4 * i as isize);
            wr::<GPtr<u8>>(intr, 0x14, f.byte_add(0x10));
            wr::<i32>(intr, 8, seed.wrapping_add(rd::<i32>(e, 0x28 + 4 * i as isize)));
            wr::<u32>(intr, 0x10, rd::<u8>(ent, 0x2F) as u32);
            wr::<GPtr<u8>>(intr, 0xC, gr);
            interior_init(intr, f);
            interior_group_add_interior(gr, intr);
            created = 1;
        }
        interior_group_setup(gr);
        wr::<u32>(this, 0x4754, g::<u32>(0xB7CB84).read());
    }
    let mut gr = rd::<GPtr<u8>>(active, 0);
    while !gr.is_null() {
        interior_group_update(gr);
        gr = rd::<GPtr<u8>>(gr, 4);
    }
    created
}

/// `InteriorManager_c::PruneVisibleEffects(InteriorEffectInfo_t*, int32 n, int32 max, float dist)` (0x598A60,
/// `ret 0x10`): when the effects' total count exceeds `max`, each entry's +0x48 becomes the smallest squared 2D
/// camera distance of its effects (`entity matrix * effect`, from 1e8), the entries are sorted ascending by it
/// (exchange sort, whole 0x50-byte swaps, ordered `<`) and entries are taken while the running count stays
/// within `max` and their distance is not above `dist^2` (f32); the rest are flagged (+0x4C).
#[hook(addr = 0x598A60, conv = thiscall, name = "InteriorManager_c::PruneVisibleEffects", this = "InteriorManager_c", expect = "55 8B EC 83 E4 F8",
       route = "L2", hazards = "none")]
pub fn interior_manager_prune_visible_effects(_this: GPtr<u8>, fx: GPtr<u8>, n: i32, max: i32, dist: f32) {
    let max_sq = x(dist).fmul_m(dist).to_f32();
    let e = |k: i32| fx.byte_add(IEI * k as isize);
    let total = (0..n.max(0)).fold(0i32, |a, k| a.wrapping_add(rd::<i32>(e(k), 4)));
    if total <= max {
        return;
    }
    for k in 0..n.max(0) {
        let ek = e(k);
        wr::<u32>(ek, 0x48, 0x4CBE_BC20);
        let mut i = 0;
        while i < rd::<i32>(ek, 4) {
            let ent = rd::<GPtr<u8>>(ek, 0);
            if rd::<GPtr<u8>>(ent, 0x14).is_null() {
                placeable_allocate_matrix(ent);
                simple_transform_update_matrix(ent.byte_add(4), rd::<GPtr<u8>>(ent, 0x14));
            }
            let mut out = [0f32; 3];
            let r = matrix_mul_vec(GPtr::from_raw(&mut out).cast(), rd::<GPtr<u8>>(ent, 0x14), rd::<GPtr<u8>>(ek, 8 + 4 * i as isize));
            let (px, py) = (rd::<f32>(r, 0), rd::<f32>(r, 4));
            let cp = cam_position();
            let a = x(rd::<f32>(cp, 0)).fsub_m(px);
            let b = x(rd::<f32>(cp, 4)).fsub_m(py);
            let s = b.fmul(b).fadd(a.fmul(a));
            if !s.fcom(x(rd::<f32>(ek, 0x48))).test_parity(5) {
                wr::<f32>(ek, 0x48, s.to_f32());
            }
            i += 1;
        }
    }
    for i in 0..(n - 1).max(0) {
        for j in i + 1..n {
            if !x(rd::<f32>(e(j), 0x48)).fcom(x(rd::<f32>(e(i), 0x48))).test_parity(5) {
                for w in 0..0x14isize {
                    let t = rd::<u32>(e(i), 4 * w);
                    wr::<u32>(e(i), 4 * w, rd::<u32>(e(j), 4 * w));
                    wr::<u32>(e(j), 4 * w, t);
                }
            }
        }
    }
    let mut acc = 0i32;
    for k in 0..n.max(0) {
        let ek = e(k);
        let s = rd::<i32>(ek, 4).wrapping_add(acc);
        if s <= max && !x(rd::<f32>(ek, 0x48)).fcom(x(max_sq)).test_parity(0x41) {
            acc = s;
        } else {
            wr::<u8>(ek, 0x4C, 1);
        }
    }
}

orig! {
    /// `FUN_004a8ec0()` (0x4A8EC0): `List_c::GetNumItems`
    fn list_get_num_items(l: GPtr<u8>) -> i32 = thiscall 0x4A8EC0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40, `ret 8`)
    fn camera_is_sphere_visible_r(c: GPtr<u8>, p: GPtr<u8>, r: f32) -> u8 = thiscall 0x420D40 => unported;
    /// `CWaterLevel::GetWaterDepth(const CVector&, float* depth, float* level, float* bed)` (0x6EA960, cdecl)
    fn water_level_get_water_depth(p: GPtr<u8>, depth: GPtr<u8>, level: GPtr<u8>, bed: GPtr<u8>) -> u8 = cdecl 0x6EA960 => unported;
    /// `WaterCreatureManager_c::GetRandomWaterCreatureId()` (0x6E4040)
    fn wcm_get_random_water_creature_id(m: GPtr<u8>) -> i32 = thiscall 0x6E4040 => unported;
    /// `WaterCreatureManager_c::CanAddWaterCreatureAtPos(int32, CVector)` (0x6E4510, `ret 0x10`)
    fn wcm_can_add_water_creature_at_pos(m: GPtr<u8>, id: i32, x: f32, y: f32, z: f32) -> u8 = thiscall 0x6E4510 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180, cdecl)
    fn general_get_random_number_in_range_int(a: i32, b: i32) -> i32 = cdecl 0x407180 => unported;
    /// `WaterCreature_c::Init(int32 type, CVector* pos, WaterCreature_c* leader, float level, float depth)` (0x6E4160)
    fn water_creature_init(w: GPtr<u8>, id: i32, pos: GPtr<u8>, leader: GPtr<u8>, level: f32, depth: f32) -> u8 = thiscall 0x6E4160 => unported;
    /// `WaterCreature_c::Update(float)` (0x6E4670, `ret 4`)
    fn water_creature_update(w: GPtr<u8>, dt: f32) = thiscall 0x6E4670 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280, cdecl)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
}

/// `rand() * (1/32768) * 2 - 1` at a call site.
fn rand_signed(site: u32) -> f32 {
    let r = sa_core::orig::at_site(site, sa_core::crt::rand);
    let v = X87::from_i32(r).fmul_m(g::<f32>(0x858C7C).read());
    v.fadd(v).fsub_m(g::<f32>(0x858624).read()).to_f32()
}

/// `WaterCreatureManager_c::Update(float timeStep)` (0x6E4F10, `ret 4`): when the player's stat byte
/// (`player data +0x8E`) is above 50, more than 1000 ms passed since +0x1A18 and free creatures (+0x1A00) exist,
/// a spawn is tried (time stamp first): a random horizontal direction (normalised) scaled by `rand * 15 + 25`
/// from the camera, not visible (radius 3), over water deeper than 0x863214, of a random creature type that may
/// be added there; `GetRandomNumberInRange(info.min, info.max)` creatures (at most the free count, info table
/// 0x8D3698, 0x2C bytes) are initialised (the first is the leader; a failed one ends the loop and goes back
/// to the free list). Then every active creature (+0x1A0C) updates, and those flagged (+0xD) move from the
/// global manager's active list (0xC1F93C) to its free list (0xC1F930), their object is removed from the world
/// and deleted, and the counter 0xBB4A70 decremented.
#[hook(addr = 0x6E4F10, conv = thiscall, name = "WaterCreatureManager_c::Update",
       expect = "83 EC 48 53 55 56 57", unverified = "mvpd-world-3", route = "L2", hazards = "heap-writes")]
pub fn water_creature_manager_update(this: GPtr<u8>, dt: f32) {
    let free = this.byte_add(0x1A00);
    let stat = rd::<u8>(rd::<GPtr<u8>>(find_player_ped(0), 0x480), 0x8E);
    'spawn: {
        if stat <= 0x32 {
            break 'spawn;
        }
        let now = g::<i32>(0xB7CB84).read();
        if now.wrapping_sub(rd::<i32>(this, 0x1A18)) <= 1000 || list_get_num_items(free) <= 0 {
            break 'spawn;
        }
        wr::<i32>(this, 0x1A18, now);
        let mut v = [rand_signed(0x6E4F78), 0.0, 0.0];
        v[1] = rand_signed(0x6E4F97);
        let vp = GPtr::from_raw(&mut v).cast::<u8>();
        vector_normalise(vp);
        let r = sa_core::orig::at_site(0x6E4FC7, sa_core::crt::rand);
        let s = X87::from_i32(r)
            .fmul_m(g::<f32>(0x858C7C).read())
            .fmul_m(g::<f32>(0x858B48).read())
            .fadd_m(g::<f32>(0x858FE8).read());
        for k in 0..3isize {
            wr::<f32>(vp, 4 * k, s.fmul_m(rd::<f32>(vp, 4 * k)).to_f32());
        }
        let cp = cam_position();
        for k in 0..3isize {
            wr::<f32>(vp, 4 * k, x(rd::<f32>(vp, 4 * k)).fadd_m(rd::<f32>(cp, 4 * k)).to_f32());
        }
        if camera_is_sphere_visible_r(GPtr::from_addr(0xB6F028), vp, 3.0) != 0 {
            break 'spawn;
        }
        let (mut depth, mut level) = (0f32, 0f32);
        if water_level_get_water_depth(vp, GPtr::from_raw(&mut depth).cast(), GPtr::from_raw(&mut level).cast(), GPtr::NULL) == 0 {
            break 'spawn;
        }
        if x(depth).fcom(x(g::<f32>(0x863214).read())).test_any(0x41) {
            break 'spawn;
        }
        let id = wcm_get_random_water_creature_id(this);
        if wcm_can_add_water_creature_at_pos(this, id, rd::<f32>(vp, 0), rd::<f32>(vp, 4), rd::<f32>(vp, 8)) == 0 {
            break 'spawn;
        }
        let info = GPtr::<u8>::from_addr((0x8D3698 + 0x2C * id as isize) as usize);
        let mut num = general_get_random_number_in_range_int(rd::<u8>(info, 4) as i32, rd::<u8>(info, 5) as i32);
        if list_get_num_items(free) < num {
            num = list_get_num_items(free);
        }
        let mut leader = GPtr::<u8>::NULL;
        let mut i = 0;
        while i < num {
            let wc = list_remove_head(this.byte_add(0x1A00));
            if water_creature_init(wc, id, vp, leader, level, depth) != 0 {
                list_add_item(this.byte_add(0x1A0C), wc);
            } else {
                num = i;
                list_add_item(this.byte_add(0x1A00), wc);
            }
            if leader.is_null() {
                leader = wc;
            }
            i += 1;
        }
    }
    let mut wc = rd::<GPtr<u8>>(this, 0x1A0C);
    while !wc.is_null() {
        let next = rd::<GPtr<u8>>(wc, 4);
        water_creature_update(wc, dt);
        wc = next;
    }
    let mut wc = rd::<GPtr<u8>>(this, 0x1A0C);
    while !wc.is_null() {
        let next = rd::<GPtr<u8>>(wc, 4);
        if rd::<u8>(wc, 0xD) != 0 {
            list_remove_item(GPtr::from_addr(0xC1F93C), wc);
            list_add_item(GPtr::from_addr(0xC1F930), wc);
            world_remove(rd::<GPtr<u8>>(wc, 8));
            let o = rd::<GPtr<u8>>(wc, 8);
            if !o.is_null() {
                v_delete(o, 1);
            }
            wr::<u32>(wc, 8, 0);
            let c = g::<u16>(0xBB4A70);
            c.write(c.read().wrapping_sub(1));
        }
        wc = next;
    }
}

orig! {
    /// `CPhysical::ProcessControl()` (0x5485E0)
    fn physical_process_control(p: GPtr<u8>) = thiscall 0x5485E0 => unported;
    /// `CEntity::UpdateRpHAnim()` (0x532B20)
    fn entity_update_rp_h_anim(e: GPtr<u8>) = thiscall 0x532B20 => unported;
    /// `GetAnimHierarchyFromClump(RpClump*)` (0x734B10, cdecl)
    fn get_anim_hierarchy_from_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734B10 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120, cdecl)
    fn rp_h_anim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `RwFrameGetLTM(RwFrame*)` (0x7F0990, cdecl)
    fn rw_frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool)` (0x59C050, `ret 8`)
    fn matrix_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59C050 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, `ret 4`)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `GetFirstAtomic(RpClump*)` (0x734820, cdecl)
    fn get_first_atomic(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RpSkinGeometryGetSkin(RpGeometry*)` (0x7C7550, cdecl)
    fn rp_skin_geometry_get_skin(g: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7550 => unported;
    /// `CRealTimeShadowManager::DoShadowThisFrame(CPhysical*)` (0x706BA0, `ret 4`)
    fn shadow_manager_do_shadow_this_frame(m: GPtr<u8>, e: GPtr<u8>) = thiscall 0x706BA0 => unported;
    /// `CShadows::StoreShadowForPedObject(CEntity*, float x6)` (0x707B40, cdecl)
    fn shadows_store_shadow_for_ped_object(e: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, f: u32, g: u32) = cdecl 0x707B40 => unported;
    /// `CPed::ShoulderBoneRotation(RpClump*)` (0x5DF560, cdecl)
    fn ped_shoulder_bone_rotation(c: GPtr<u8>) = cdecl 0x5DF560 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x7, CStoredCollPoly*)`
    /// (0x5674E0, cdecl)
    fn world_process_vertical_line(o: GPtr<u8>, z: f32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, ob: u32, d: u32, s: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
}

/// The attached cutscene object's matrix (`+0x17C` bone index on the `+0x180` entity's hierarchy, or `+0x17C`
/// as an RW frame whose LTM is used), assigned through a temporary `CMatrix(RwMatrix*, false)`.
fn cutscene_object_attach_matrix(this: GPtr<u8>, with_anim_update: bool) {
    let mut tmp = [0u32; 0x12];
    let t = GPtr::from_raw(&mut tmp).cast::<u8>();
    let owner = rd::<GPtr<u8>>(this, 0x180);
    let rw = if !owner.is_null() {
        if with_anim_update {
            entity_update_rp_h_anim(owner);
        }
        let h = get_anim_hierarchy_from_clump(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x180), 0x18));
        rp_h_anim_hierarchy_get_matrix_array(h).byte_add(rd::<i32>(this, 0x17C).wrapping_shl(6) as isize)
    } else {
        rw_frame_get_ltm(rd::<GPtr<u8>>(this, 0x17C))
    };
    matrix_ctor_rw(t, rw, 0);
    matrix_assign(rd::<GPtr<u8>>(this, 0x14), t);
    matrix_dtor(t);
}

/// `CCutsceneObject::ProcessControl()` (0x5B1B90, SEH frame for the temporary matrix): an object attached to a
/// bone (+0x17C and +0x180) whose `+0x1C` bit 6 is clear only sets it. Otherwise `CPhysical::ProcessControl`;
/// attached objects follow the bone (`UpdateRpHAnim` of the owner, bit 6 cleared) or frame. Free objects convert
/// their move speed to units per frame (`* 100` below a time step of 0.01, else `/ timestep`), zero it when
/// `+0x40` has bit 0x2000 or 0x20, else integrate the offset +0x190 by `timestep * speed` and place the matrix at
/// `+0x190 + +0x184`.
#[hook(addr = 0x5B1B90, conv = thiscall, name = "CCutsceneObject::ProcessControl", this = "CCutsceneObject",
       expect = "64 A1 00 00 00 00", unverified = "mvpd-world-3", route = "L2", hazards = "heap-writes")]
pub fn cutscene_object_process_control(this: GPtr<u8>) {
    if rd::<u32>(this, 0x17C) != 0 && rd::<u32>(this, 0x180) != 0 {
        let f = rd::<u32>(this, 0x1C);
        if (f >> 6) & 1 == 0 {
            wr::<u32>(this, 0x1C, f | 0x40);
            return;
        }
    }
    physical_process_control(this);
    if rd::<u32>(this, 0x17C) != 0 {
        let attached = rd::<u32>(this, 0x180) != 0;
        cutscene_object_attach_matrix(this, true);
        if attached {
            wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & !0x40);
        }
        return;
    }
    let ts = g::<f32>(0xB7CB5C).read();
    if !x(ts).fcom(x(g::<f32>(0x858C58).read())).test_parity(5) {
        let k = g::<f32>(0x858628).read();
        for o in [0x44, 0x48, 0x4C] {
            wr::<f32>(this, o, x(rd::<f32>(this, o)).fmul_m(k).to_f32());
        }
    } else {
        let k = x(g::<f32>(0x858624).read()).fdiv_m(ts);
        for o in [0x44, 0x48, 0x4C] {
            wr::<f32>(this, o, k.fmul_m(rd::<f32>(this, o)).to_f32());
        }
    }
    let fl = rd::<u32>(this, 0x40);
    if fl & 0x2000 != 0 || fl & 0x20 != 0 {
        wr::<u32>(this, 0x4C, 0);
        wr::<u32>(this, 0x48, 0);
        wr::<u32>(this, 0x44, 0);
        return;
    }
    let ts = g::<f32>(0xB7CB5C).read();
    let dx = x(ts).fmul_m(rd::<f32>(this, 0x44));
    let dy = x(ts).fmul_m(rd::<f32>(this, 0x48));
    let dz = x(ts).fmul_m(rd::<f32>(this, 0x4C)).to_f32();
    wr::<f32>(this, 0x190, dx.fadd_m(rd::<f32>(this, 0x190)).to_f32());
    wr::<f32>(this, 0x194, dy.fadd_m(rd::<f32>(this, 0x194)).to_f32());
    wr::<f32>(this, 0x198, x(dz).fadd_m(rd::<f32>(this, 0x198)).to_f32());
    let px = x(rd::<f32>(this, 0x190)).fadd_m(rd::<f32>(this, 0x184)).to_f32();
    let py = x(rd::<f32>(this, 0x194)).fadd_m(rd::<f32>(this, 0x188)).to_f32();
    let pz = x(rd::<f32>(this, 0x198)).fadd_m(rd::<f32>(this, 0x18C)).to_f32();
    wr::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x30, px);
    wr::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x34, py);
    wr::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x38, pz);
}

/// `CCutsceneObject::PreRender()` (0x5B1E00, SEH frame for the temporary matrices): attached objects take the
/// bone / frame matrix (as `ProcessControl`, without the owner's anim update) and skinned clumps copy the anim
/// data's root position into the geometry's morph target bounding-sphere centre; clumps update their RpHAnim;
/// the real-time shadow is requested, objects without +0x134 store a ped-object shadow with the current
/// `CTimeCycle` shadow values (index 0xB79FD0), model 1 rotates its shoulder bones (`+0x1C |= 0x800000`); a
/// building-only vertical line from `pos + 0.5` down to `pos.z + 0.5 - 0x858C80` blends the hit's lighting
/// (DN balance 0x8D12C0, factor `timestep * 0x858B1C`) into +0x12C.
#[hook(addr = 0x5B1E00, conv = thiscall, name = "CCutsceneObject::PreRender", this = "CCutsceneObject",
       expect = "64 A1 00 00 00 00", unverified = "mvpd-world-3", route = "L2", hazards = "heap-writes")]
pub fn cutscene_object_pre_render(this: GPtr<u8>) {
    if rd::<u32>(this, 0x17C) != 0 {
        cutscene_object_attach_matrix(this, false);
        let clump = rd::<GPtr<u8>>(this, 0x18);
        if rd::<u8>(clump, 0) == 2 {
            let a = get_first_atomic(clump);
            if !a.is_null() && !rp_skin_geometry_get_skin(rd::<GPtr<u8>>(a, 0x18)).is_null() {
                let a = get_first_atomic(clump);
                let data = rd::<GPtr<u8>>(clump, g::<u32>(0xB5F878).read() as isize);
                let src = rd::<GPtr<u8>>(rd::<GPtr<u8>>(data, 0x10), 0x10).byte_add(0x10);
                let dst = rd::<GPtr<u8>>(rd::<GPtr<u8>>(a, 0x18), 0x5C).byte_add(4);
                wr::<u32>(dst, 0, rd::<u32>(src, 0));
                wr::<u32>(dst, 4, rd::<u32>(src, 4));
                wr::<u32>(dst, 8, rd::<u32>(src, 8));
            }
        }
    }
    if rd::<u8>(rd::<GPtr<u8>>(this, 0x18), 0) == 2 {
        entity_update_rp_h_anim(this);
    }
    shadow_manager_do_shadow_this_frame(GPtr::from_addr(0xC40350), this);
    if rd::<u32>(this, 0x134) == 0 {
        let i = 4 * g::<u32>(0xB79FD0).read() as usize;
        let t = |a: usize| g::<u32>(a + i).read();
        shadows_store_shadow_for_ped_object(this, t(0xB79E90), t(0xB79E50), t(0xB79F90), t(0xB79F50), t(0xB79F10), t(0xB79ED0));
    }
    if rd::<i16>(this, 0x22) == 1 {
        ped_shoulder_bone_rotation(rd::<GPtr<u8>>(this, 0x18));
        wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x80_0000);
    }
    let p = ent_pos(this);
    let mut o = [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)];
    o[2] = x(o[2]).fadd_m(g::<f32>(0x858B8C).read()).to_f32();
    let z = x(o[2]).fsub_m(g::<f32>(0x858C80).read()).to_f32();
    let mut cp = [0u8; 0x2C];
    let mut hit = 0u32;
    let cpp = GPtr::from_raw(&mut cp).cast::<u8>();
    if world_process_vertical_line(GPtr::from_raw(&mut o).cast(), z, cpp, GPtr::from_raw(&mut hit).cast(), 1, 0, 0, 0, 0, 0, 0) == 0 {
        return;
    }
    let l = rd::<u8>(cpp, 0x25);
    let b = x(g::<f32>(0x8D12C0).read());
    let t = x(g::<f32>(0xB7CB5C).read()).fmul_m(g::<f32>(0x858B1C).read());
    let kf = g::<f32>(0x858F10).read();
    let one = g::<f32>(0x858624).read();
    let day = X87::from_i32((l & 0xF) as i32).fmul_m(kf).fmul(x(one).fsub(b));
    let night = X87::from_i32((l >> 4) as i32).fmul_m(kf).fmul(b);
    let lt = day.fadd(night).fmul(t);
    let keep = x(one).fsub(t).fmul_m(rd::<f32>(this, 0x12C));
    wr::<f32>(this, 0x12C, lt.fadd(keep).to_f32());
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ENTITY_IS_ENTITY_OCCLUDED, HOOK_WORLD_CLEAR_EXCITING_STUFF_FROM_AREA, HOOK_ENTRY_EXIT_MANAGER_UPDATE, HOOK_STUNT_JUMP_MANAGER_UPDATE, HOOK_INTERIOR_MANAGER_UPDATE, HOOK_INTERIOR_MANAGER_PRUNE_VISIBLE_EFFECTS, HOOK_WATER_CREATURE_MANAGER_UPDATE, HOOK_CUTSCENE_OBJECT_PROCESS_CONTROL, HOOK_CUTSCENE_OBJECT_PRE_RENDER];
