//! `CPedGroupPlacer::PlaceFormationGroup` (batch `frontier2-a9`): places a gang group in the formation table's slots
//! behind its leader. Ported from the 1.0 US bytes (gta-reversed only calls the original). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::field::ptr_of as vp;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::fpu::{fld_fst, ge_or_nan, gt, le_or_nan};
use sa_world::world::common::entity_pos as pos_of;
use sa_world::world::common::find_player_ped;

use crate::bindings::{group_intel_process, group_intel_set_default_task_allocator, membership_process, tm_set_task};
use crate::common::*;
use sa_core::bindings::{general_get_radian_angle_between_points, membership_set_leader, world_get_is_line_of_sight_clear};
use sa_core::crt::rand_at;
use sa_pop::bindings::add_ped as population_add_ped;
use sa_pop::bindings::pdm_call as population_ped_creation_dist_multiplier;
use sa_pop::bindings::population_remove_ped;
use sa_world::bindings::{
    be_in_group_ctor, camera_is_sphere_visible, choose_gang_occupation, find_ground_z_for_3d_coord,
    is_position_clear_for_ped, membership_add_follower, ped_groups_add_group, placeable_set_posn, set_clump_alpha,
    set_posn, vector_add,
};

/// `TheCamera` (0xB6F028), `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8), `CPedGroups::ms_groups` (0xC09920, 0x2D4 each),
/// the formation group task allocator (0xC09908), the formation offsets (0xC196E8, 8 bytes each)
const THE_CAMERA: usize = 0xB6F028;
const MODEL_INFOS: usize = 0xA9B0C8;
const GROUPS: u32 = 0xC09920;
const FORMATION_ALLOCATOR: usize = 0xC09908;
const OFFSETS: usize = 0xC196E8;
/// 1.0 (0x858624), the creation distance factor (0x86C850)
const K_ONE: usize = 0x858624;
const K_CREATE_DIST: usize = 0x86C850;

/// `ChooseGangOccupation(type - 7)` and whether its model info has an RW object (+0x1C).
fn gang_model(ped_type: u32) -> (i32, bool) {
    let model = choose_gang_occupation(ped_type as i32 - 7);
    let mi = g::<GPtr<u8>>(MODEL_INFOS.wrapping_add((model as usize).wrapping_mul(4))).read();
    (model, rd::<u32>(mi, 0x1C) != 0)
}

/// A new ped of `ped_type` / `model` at `(o.x, f32 o.y, z)`.
fn add_at_origin(ped_type: u32, model: i32, origin: GPtr<u8>, z: f32) -> GPtr<u8> {
    let mut p = [rd::<f32>(origin, 0), fld_fst(rd::<f32>(origin, 4)), z];
    population_add_ped(ped_type, model as u32, GPtr::<u8>::cast::<f32>(vp(&mut p)), 0)
}

/// The sphere `(origin, radius)` visible to the camera with the player's 2D distance (f32 `sqrt`) below
/// `PedCreationDistMultiplier() * 42.5`: the placers' "the player would see the peds appear" test.
fn seen_near_player(origin: GPtr<u8>, radius: f32) -> bool {
    if camera_is_sphere_visible(GPtr::from_addr(THE_CAMERA), origin, radius) == 0 {
        return false;
    }
    let pp = pos_of(find_player_ped(-1));
    let dy = x(rd::<f32>(origin, 4)).fsub_m(rd::<f32>(pp, 4));
    let dx = x(rd::<f32>(origin, 0)).fsub_m(rd::<f32>(pp, 0));
    let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt().to_f32();
    let lim = population_ped_creation_dist_multiplier().fmul_m(g::<f32>(K_CREATE_DIST).read());
    gt(lim, x(d))
}

/// The group (`ms_groups + id * 0x2D4`) gets the default task allocator `allocator`, `peds[0]` as leader and then each
/// follower (membership and intelligence `Process()` after each), each with a `CTaskComplexBeInGroup(id, leader?)` in
/// slot 3 (task pool, NULL when that fails). Shared by PlaceFormationGroup and PlaceChatGroup.
fn build_group(group: i32, peds: &[GPtr<u8>], allocator: usize) {
    let grp = GPtr::<u8>::from_addr((GROUPS + (group as u32) * 0x2D4) as usize);
    let intel = grp.byte_add(0x30);
    group_intel_set_default_task_allocator(intel, GPtr::from_addr(allocator));
    let members = grp.byte_add(8);
    for (k, &ped) in peds.iter().enumerate() {
        if k == 0 {
            membership_set_leader(members, ped);
        } else {
            membership_add_follower(members, ped);
        }
        membership_process(members);
        group_intel_process(intel);
        let mem = task_new(0x28);
        let task = if mem.is_null() { GPtr::NULL } else { be_in_group_ctor(mem, group as u32, u32::from(k == 0)) };
        tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), task, 3, 0);
    }
}

/// `CPedGroupPlacer::PlaceFormationGroup(ePedType, uint32 numOfPeds, const CVector& origin,
/// ePedGroupDefaultTaskAllocatorType)` (0x5FC9B0, thiscall, `ret 0x10`; `this` and the allocator type unused), AL.
///
/// No free group (`AddGroup() < 0`) → 0. 0 also when the sphere (origin, 3) is visible and the player's 2D distance
/// (f32 `sqrt`) is below `PedCreationDistMultiplier() * k`, when `IsPositionClearForPed(origin, 3, -1, NULL, 1, 1, 1)`
/// fails, when `FindGroundZFor3DCoord(o.x, o.y, o.z + 1)` finds no ground, or when the leader cannot be made. `z =
/// ground + 1`, at least `o.z`. The leader: `ChooseGangOccupation(type - 7)` (no RW object → 0), `AddPed(type, model,
/// (o.x, o.y, z), false)`. Then for `i` in `1..n`: a model with an RW object, a ped added at the same point, moved to
/// leader + formation offset `i` (`(x, y, 0)`, through `fld` / `fstp`); from its position `p`, `FindGroundZFor3DCoord(
/// p.x, p.y, f32(p.z + 1))`: `z' = p.z` when above `ground + 1`, else `ground + 1`. No ground, `|z' - leader.z| > 1`
/// or no line of sight to the leader (buildings only) → `RemovePed`; else the ped is placed at `(p.x, p.y, z')`, kept
/// and its clump alpha set to 0. The group (ms_groups + id * 0x2D4) gets the formation allocator 0xC09908, the leader
/// and then each follower (membership and intelligence `Process()` after each), each with a `CTaskComplexBeInGroup(id,
/// leader?)` in slot 3 (task pool, NULL when that fails). Returns 1. The original's "no ped placed" path (remove the
/// placed peds, 0) cannot be reached: the leader always counts.
#[hook(addr = 0x5FC9B0, conv = thiscall, name = "CPedGroupPlacer::PlaceFormationGroup", expect = "6A FF 68 E6 E4 83 00",
       unverified = "frontier2-a9", route = "L2", hazards = "ax-al-return,heap-writes",
       note = "the ground flags live on the Rust stack; SEH frame not modelled")]
pub fn place_formation_group(_this: GPtr<u8>, ped_type: u32, n: i32, origin: GPtr<u8>, _alloc: u32) -> u8 {
    let group = ped_groups_add_group() as i32;
    if group < 0 {
        return 0;
    }
    if seen_near_player(origin, 3.0) {
        return 0;
    }
    if is_position_clear_for_ped(origin, 3.0, -1, GPtr::NULL, 1, 1, 1) == 0 {
        return 0;
    }
    let one = g::<f32>(K_ONE).read();
    let mut found = [0u8; 4];
    let gz_arg = x(rd::<f32>(origin, 8)).fadd_m(one).to_f32();
    let gz = find_ground_z_for_3d_coord(rd::<f32>(origin, 0), rd::<f32>(origin, 4), gz_arg,
                                        GPtr::from_raw(found.as_mut_ptr()), GPtr::NULL);
    let gz1 = X87::from_f64_exact(gz).fadd_m(one);
    if found[0] == 0 {
        return 0;
    }
    let z = if ge_or_nan(gz1, x(rd::<f32>(origin, 8))) { gz1.to_f32() } else { rd::<f32>(origin, 8) };
    let (model, ok) = gang_model(ped_type);
    if !ok {
        return 0;
    }
    let leader = add_at_origin(ped_type, model, origin, z);
    if leader.is_null() {
        return 0;
    }
    let mut placed = [GPtr::<u8>::NULL; 32];
    placed[0] = leader;
    let mut count = 1usize;
    for i in 1..n.max(1) {
        let (model, ok) = gang_model(ped_type);
        if !ok {
            continue;
        }
        let ped = add_at_origin(ped_type, model, origin, z);
        if ped.is_null() {
            continue;
        }
        let k = i as usize * 8;
        let oy = fld_fst(g::<f32>(OFFSETS + k + 4).read());
        let mut off = [fld_fst(g::<f32>(OFFSETS + k).read()), oy, 0.0];
        let mut out = [0f32; 3];
        let r = vector_add(vp(&mut out), pos_of(leader), vp(&mut off));
        placeable_set_posn(ped, r.cast());
        let p = pos_of(ped);
        let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
        let mut found = [0u8; 4];
        let gz = find_ground_z_for_3d_coord(px, py, x(pz).fadd_m(one).to_f32(), GPtr::from_raw(found.as_mut_ptr()),
                                            GPtr::NULL);
        let gz1 = X87::from_f64_exact(gz).fadd_m(one);
        let z2 = if le_or_nan(x(pz), gz1) { gz1.to_f32() } else { pz };
        if found[0] == 0 {
            population_remove_ped(ped);
            continue;
        }
        if gt(x(z2).fsub_m(rd::<f32>(pos_of(leader), 8)).fabs(), x(one)) {
            population_remove_ped(ped);
            continue;
        }
        let mut v = [px, py, z2];
        if world_get_is_line_of_sight_clear(vp(&mut v), pos_of(leader), 1, 0, 0, 0, 0, 0, 0) == 0 {
            population_remove_ped(ped);
            continue;
        }
        set_posn(ped, px, py, z2);
        placed[count] = ped;
        count += 1;
        set_clump_alpha(rd::<GPtr<u8>>(ped, 0x18), 0);
    }
    build_group(group, &placed[..count], FORMATION_ALLOCATOR);
    1
}

/// The chat group task allocator (0xC09910), 2pi (0x858CBC), 0.5 (0x858B8C), 1 / 32767 (0x858C7C), 0.4 (0x858EE8),
/// 0.2 (0x858CC4).
const CHAT_ALLOCATOR: usize = 0xC09910;
const K_TWO_PI: usize = 0x858CBC;
const K_HALF: usize = 0x858B8C;
const K_INV_RAND_MAX: usize = 0x858C7C;
const K_JITTER: usize = 0x858EE8;
const K_JITTER_HALF: usize = 0x858CC4;

/// `(rand() / 32767 * 0.4 - 0.2) * scale`, the rand call returning to `site`.
fn chat_jitter(site: u32, scale: f32) -> X87 {
    xi(rand_at(site)).fmul_m(g::<f32>(K_INV_RAND_MAX).read()).fmul_m(g::<f32>(K_JITTER).read())
        .fsub_m(g::<f32>(K_JITTER_HALF).read()).fmul_m(scale)
}

/// `CPedGroupPlacer::PlaceChatGroup(ePedType, uint32 numOfPeds, const CVector& origin,
/// ePedGroupDefaultTaskAllocatorType)` (0x5FCE80, thiscall, `ret 0x10`; `this` and the allocator type unused), AL.
/// gta-reversed only calls the original.
///
/// Fewer than 2 peds or no free group (`AddGroup() < 0`) → 0. The circle: `step = 2pi / n`, `r = sqrt(0.5 / (1 -
/// cos step))` (f32). 0 when the camera sees (origin, r) near the player (as PlaceFormationGroup with radius r) or
/// `IsPositionClearForPed(origin, r, -1, NULL, 1, 1, 1)` fails. For each `i` in `0..n`: angle `i * step + (rand / 32767
/// * 0.4 - 0.2) * step`, radius `r + (rand / 32767 * 0.4 - 0.2) * r`, the point `origin + (cos, sin) * radius`;
/// `FindGroundZFor3DCoord(x, y, o.z + 1) + 1`, at least `o.z` (no ground: skipped). Point `i = 0` is remembered as
/// the first point (an uninitialised stack vector until then). `ChooseGangOccupation(type - 7)` (no RW object:
/// skipped); `IsPositionClearForPed(p, model col radius, 9, list, 1, 1, 1)` (result unused): any listed entity that is
/// not an already placed ped blocks the point; for `i > 0` also no line of sight to the first point (buildings) or
/// `|z - first.z| >= 1` (NaN too). A free point gets `AddPed(type, model, p, false)` (NULL: `RemovePed(NULL)`, as the
/// bytes), heading `GetRadianAngleBetweenPoints(o.x, o.y, p.x, p.y)` (+0x558 and +0x55C) and clump alpha 0. No ped
/// placed → 0 (the original's removal loop runs over none). Else the group gets the chat allocator 0xC09910 with the
/// first placed ped as leader ([`build_group`]); 1. The original keeps eight placed peds on its stack; a ninth would
/// land in the entity list it clears for the next point.
#[hook(addr = 0x5FCE80, conv = thiscall, name = "CPedGroupPlacer::PlaceChatGroup", expect = "64 A1 00 00 00 00 6A FF 68 06 E5 83 00",
       unverified = "frontier2-a10", route = "L2", hazards = "x87-transcendental,ax-al-return,uninit-locals,heap-writes",
       note = "rand at 0x5FCF85 and 0x5FCFAC; the first point is an uninitialised stack vector until point 0 is placed (zeros here); SEH frame not modelled")]
pub fn place_chat_group(_this: GPtr<u8>, ped_type: u32, n: i32, origin: GPtr<u8>, _alloc: u32) -> u8 {
    if n < 2 {
        return 0;
    }
    let group = ped_groups_add_group() as i32;
    if group < 0 {
        return 0;
    }
    let step80 = xi(n).fdivr_m(g::<f32>(K_TWO_PI).read());
    let step = step80.to_f32();
    let one = g::<f32>(K_ONE).read();
    let r = step80.fcos().fsubr_m(one).fdivr_m(g::<f32>(K_HALF).read()).fsqrt().to_f32();
    if seen_near_player(origin, r) {
        return 0;
    }
    if is_position_clear_for_ped(origin, r, -1, GPtr::NULL, 1, 1, 1) == 0 {
        return 0;
    }
    let (ox, oy, oz) = (rd::<f32>(origin, 0), rd::<f32>(origin, 4), rd::<f32>(origin, 8));
    let mut placed = [GPtr::<u8>::NULL; 32];
    let mut count = 0usize;
    let mut first = [0f32; 3];
    for i in 0..n {
        let j1 = chat_jitter(0x5FCF85, step).to_f32();
        let j2 = chat_jitter(0x5FCFAC, r).to_f32();
        let a = xi(i).fmul_m(step).fadd_m(j1);
        let rad = x(j2).fadd_m(r);
        let cx = a.fcos().fmul(rad).to_f32();
        let sy = a.fsin().fmul(rad).to_f32();
        let px = x(cx).fadd_m(ox).to_f32();
        let py = x(sy).fadd_m(oy).to_f32();
        let mut found = [0u8; 4];
        let gz = find_ground_z_for_3d_coord(px, py, x(oz).fadd_m(one).to_f32(), GPtr::from_raw(found.as_mut_ptr()),
                                            GPtr::NULL);
        let gz1 = X87::from_f64_exact(gz).fadd_m(one);
        if found[0] == 0 {
            continue;
        }
        let pz = if ge_or_nan(gz1, x(oz)) { gz1.to_f32() } else { oz };
        let mut p = [px, py, pz];
        if i == 0 {
            first = p;
        }
        let (model, ok) = gang_model(ped_type);
        if !ok {
            continue;
        }
        let mi = g::<GPtr<u8>>(MODEL_INFOS.wrapping_add((model as usize).wrapping_mul(4))).read();
        let radius = rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24);
        let mut list = [GPtr::<u8>::NULL; 9];
        is_position_clear_for_ped(vp(&mut p), radius, 9, GPtr::from_raw(list.as_mut_ptr()).cast(), 1, 1, 1);
        let blocked = list.iter().any(|&e| !e.is_null() && !placed[..count].contains(&e));
        if blocked {
            continue;
        }
        if i != 0 {
            let los = world_get_is_line_of_sight_clear(vp(&mut p), vp(&mut first), 1, 0, 0, 0, 0, 0, 0) != 0;
            let near = !ge_or_nan(x(pz).fsub_m(first[2]).fabs(), x(one));
            if !los || !near {
                continue;
            }
        }
        let ped = population_add_ped(ped_type, model as u32, GPtr::<u8>::cast::<f32>(vp(&mut p)), 0);
        if ped.is_null() {
            population_remove_ped(GPtr::NULL);
            continue;
        }
        placed[count] = ped;
        count += 1;
        let h = general_get_radian_angle_between_points(ox, oy, px, py).to_f32();
        wr::<f32>(ped, 0x558, h);
        wr::<f32>(ped, 0x55C, h);
        set_clump_alpha(rd::<GPtr<u8>>(ped, 0x18), 0);
    }
    if count == 0 {
        return 0;
    }
    build_group(group, &placed[..count], CHAT_ALLOCATOR);
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_PLACE_FORMATION_GROUP, HOOK_PLACE_CHAT_GROUP];
