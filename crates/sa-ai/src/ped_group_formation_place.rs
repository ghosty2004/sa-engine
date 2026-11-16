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
use sa_core::bindings::{membership_set_leader, world_get_is_line_of_sight_clear};
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
    if camera_is_sphere_visible(GPtr::from_addr(THE_CAMERA), origin, 3.0) != 0 {
        let pp = pos_of(find_player_ped(-1));
        let dy = x(rd::<f32>(origin, 4)).fsub_m(rd::<f32>(pp, 4));
        let dx = x(rd::<f32>(origin, 0)).fsub_m(rd::<f32>(pp, 0));
        let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt().to_f32();
        let lim = population_ped_creation_dist_multiplier().fmul_m(g::<f32>(K_CREATE_DIST).read());
        if gt(lim, x(d)) {
            return 0;
        }
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
    let grp = GPtr::<u8>::from_addr((GROUPS + (group as u32) * 0x2D4) as usize);
    let intel = grp.byte_add(0x30);
    group_intel_set_default_task_allocator(intel, GPtr::from_addr(FORMATION_ALLOCATOR));
    let members = grp.byte_add(8);
    for (k, &ped) in placed[..count].iter().enumerate() {
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
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_PLACE_FORMATION_GROUP];
