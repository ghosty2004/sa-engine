//! `CPedGroupPlacer::PlaceRandomGroup` (batch `mvpm-ai-3`): places a wandering gang group in a ring around a point.
//! Ported from the 1.0 US bytes (gta-reversed only calls the original). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::hook;
use sa_world::world::common::find_player_ped;

use crate::common::*;

use crate::bindings::group_intel_process;
use crate::bindings::membership_process;
use sa_pop::bindings::wander_gang_ctor;
use sa_core::bindings::membership_set_leader;
use sa_pop::bindings::add_ped as population_add_ped;
use sa_world::bindings::be_in_group_ctor;
use sa_world::bindings::choose_gang_occupation as population_choose_gang_occupation;
use sa_world::bindings::membership_add_follower;
use sa_world::bindings::ped_groups_add_group;
use sa_pop::bindings::population_remove_ped;
use crate::bindings::tm_set_task;
use sa_core::bindings::world_get_is_line_of_sight_clear;
use crate::bindings::group_intel_set_default_task_allocator;
use sa_core::bindings::general_get_radian_angle_between_points;
use sa_world::bindings::camera_is_sphere_visible;
use sa_world::bindings::find_ground_z_for_3d_coord as world_find_ground_z_for_3d_coord;
use sa_world::bindings::is_position_clear_for_ped as ped_placement_is_position_clear;
use sa_world::bindings::set_clump_alpha as visibility_set_clump_alpha;
use sa_pop::bindings::pdm_call as population_ped_creation_dist_multiplier;

/// 2*pi (0x858CBC), 1.0 (0x858624), 0.5 (0x858B8C), 1/32767 (0x858C7C)
const K_TWO_PI: usize = 0x858CBC;
const K_ONE: usize = 0x858624;
const K_HALF: usize = 0x858B8C;
const K_RAND_SCALE: usize = 0x858C7C;
/// The jitter scale (0x858EE8) and offset (0x858CC4): `rand * 1/32767 * s - o`
const K_JITTER_SCALE: usize = 0x858EE8;
const K_JITTER_OFFSET: usize = 0x858CC4;
/// The creation distance factor (0x86C850)
const K_CREATE_DIST: usize = 0x86C850;
/// The wander direction scale (0x858B14, 0x859000) and the wander float argument (0x86FCB0)
const K_DIR1: usize = 0x858B14;
const K_DIR2: usize = 0x859000;
const WANDER_ARG: usize = 0x86FCB0;
/// `TheCamera` (0xB6F028), `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8), `CPedGroups::ms_groups` (0xC09920, 0x2D4 each),
/// the random group task allocator (0xC09918)
const THE_CAMERA: usize = 0xB6F028;
const MODEL_INFOS: u32 = 0xA9B0C8;
const GROUPS: u32 = 0xC09920;
const RANDOM_ALLOCATOR: u32 = 0xC09918;

use sa_core::field::gf as kf;
use sa_core::field::ptr_of as vp;

/// `CPedGroupPlacer::PlaceRandomGroup(ePedType, uint32 numOfPeds, const CVector& origin, ePedGroupDefaultTaskAllocatorType)`
/// (0x5FD330, thiscall, `ret 0x10`; `this` unused), AL.
///
/// Fewer than 2 peds or no free group (`AddGroup() < 0`) → 0. `step = f32(2pi / n)`, `r = f32(sqrt(0.5 / (1 - cos(step))))`
/// (FCOS). 0 when the sphere (origin, r) is visible and the player's 2D distance is within `PedCreationDistMultiplier()
/// * k`, or when `IsPositionClearForPed(origin, r, -1, NULL, 1, 1, 1)` fails. For each `i < n`: angle jitter `ja =
/// f32((rand() * 1/32767 * s - o) * step)`, radial jitter `jr = f32((rand() * 1/32767 * s - o) * r)`, `θ = i * step +
/// ja`, `ρ = jr + r`, `d = (f32(cos θ * ρ), f32(sin θ * ρ))` (FCOS, FSIN); `FindGroundZFor3DCoord(d.x + o.x, d.y + o.y,
/// o.z + 1)`; without ground the point is skipped; `z = ground + 1`, at least `o.z` (`test ah, 5; jp`); `P = (f32(d.x
/// + o.x), f32(d.y + o.y), f32(z))`, the first one also kept as `Q`. The model is `ChooseGangOccupation(type - 7)`; one
/// without RW object (model info +0x1C) is skipped. `IsPositionClearForPed(P, col radius, 9, list, 1, 1, 1)`: any entity
/// in the list that is not a ped placed so far blocks it. After the first point it must also have a clear line of sight
/// to `Q` (buildings only) and `|P.z - Q.z| < 1`. Then `AddPed(type, model, P, false)`: a null ped calls
/// `RemovePed(NULL)` (original); else the ped is kept, its current and aimed rotation (+0x558, +0x55C) face away from the
/// origin (`GetRadianAngleBetweenPoints(o, P)`) and its clump alpha is 0. No ped placed → 0 (nothing to remove). Else
/// the group (ms_groups + id * 0x2D4) gets the random task allocator; ped 0 is the leader, the rest followers (each
/// followed by membership and intelligence `Process()`); each ped gets a `CTaskComplexWanderGang(4, ftol((rand() & 0xFFFF)
/// * k1 * k2), 5000, true, [0x86FCB0])` in slot 4 and a `CTaskComplexBeInGroup(id, false)` in slot 3 (task pool, NULL
/// when that fails). Returns 1.
#[hook(addr = 0x5fd330, conv = thiscall, name = "CPedGroupPlacer::PlaceRandomGroup", expect = "64 A1 00 00 00 00",
       unverified = "mvpm-ai-3", route = "L2", hazards = "x87-transcendental,ax-al-return,heap-writes,stack-residue", recs = "grove-walk",
       note = "FCOS/FSIN through sa_math::x87; rand() through at_site (0x5FD436, 0x5FD45D, 0x5FD73E); the entity list and the ground flag live on the Rust stack; RemovePed(NULL) after a failed AddPed is the original's")]
pub fn place_random_group(_this: GPtr<u8>, ped_type: u32, n: i32, origin: GPtr<u8>, _alloc: u32) -> u8 {
    if n < 2 {
        return 0;
    }
    let group = ped_groups_add_group() as i32;
    if group < 0 {
        return 0;
    }
    let step = X87::from_i32(n).fdivr_m(kf(K_TWO_PI));
    let step_f = step.to_f32();
    let r = step.fcos().fsubr_m(kf(K_ONE)).fdivr_m(kf(K_HALF)).fsqrt().to_f32();
    let (ox, oy, oz) = (rd::<f32>(origin, 0), rd::<f32>(origin, 4), rd::<f32>(origin, 8));
    if camera_is_sphere_visible(GPtr::from_addr(THE_CAMERA), origin, r) != 0 {
        let pl = find_player_ped(-1);
        let pm = rd::<GPtr<u8>>(pl, 0x14);
        let pp = if pm.is_null() { pl.byte_add(4) } else { pm.byte_add(0x30) };
        let dy = x(oy).fsub_m(rd::<f32>(pp, 4));
        let dx = x(ox).fsub_m(rd::<f32>(pp, 0));
        let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt().to_f32();
        let lim = X87::from_f64_exact(population_ped_creation_dist_multiplier().to_f64()).fmul_m(kf(K_CREATE_DIST));
        if !lim.fcom(x(d)).test_any(0x41) {
            return 0;
        }
    }
    if ped_placement_is_position_clear(origin, r, -1, GPtr::NULL, 1, 1, 1) == 0 {
        return 0;
    }
    let mut placed = [GPtr::<u8>::NULL; 32];
    let mut count: usize = 0;
    let mut q = [0f32; 3];
    let ks = kf(K_RAND_SCALE);
    for i in 0..n {
        let r1 = at_site(0x5FD436, rand);
        let ja = X87::from_i32(r1).fmul_m(ks).fmul_m(kf(K_JITTER_SCALE)).fsub_m(kf(K_JITTER_OFFSET)).fmul_m(step_f).to_f32();
        let r2 = at_site(0x5FD45D, rand);
        let mut found = [0u8; 4];
        let jr = X87::from_i32(r2).fmul_m(ks).fmul_m(kf(K_JITTER_SCALE)).fsub_m(kf(K_JITTER_OFFSET)).fmul_m(r).to_f32();
        let th = X87::from_i32(i).fmul_m(step_f).fadd_m(ja);
        let rho = x(jr).fadd_m(r);
        let dx = th.fcos().fmul(rho).to_f32();
        let dy = th.fsin().fmul(rho).to_f32();
        let gz_arg = x(oz).fadd_m(kf(K_ONE)).to_f32();
        let gy = x(dy).fadd_m(oy).to_f32();
        let gx = x(dx).fadd_m(ox).to_f32();
        let gz = world_find_ground_z_for_3d_coord(gx, gy, gz_arg, GPtr::from_raw(found.as_mut_ptr()), GPtr::NULL);
        let mut z = X87::from_f64_exact(gz).fadd_m(kf(K_ONE));
        if found[0] == 0 {
            continue;
        }
        if !z.fcom(x(oz)).test_parity(5) {
            z = x(oz);
        }
        let mut p = [x(dx).fadd_m(ox).to_f32(), x(dy).fadd_m(oy).to_f32(), z.to_f32()];
        if i == 0 {
            q = p;
        }
        let model = population_choose_gang_occupation(ped_type as i32 - 7);
        let mi = g::<GPtr<u8>>((MODEL_INFOS as i32).wrapping_add(model.wrapping_mul(4)) as u32 as usize).read();
        if rd::<u32>(mi, 0x1C) == 0 {
            continue;
        }
        let mut list = [0u32; 9];
        let radius = rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24);
        ped_placement_is_position_clear(vp(&mut p), radius, 9, GPtr::from_raw(list.as_mut_ptr()).cast(), 1, 1, 1);
        let blocked = list.iter().any(|&e| e != 0 && !placed[..count].iter().any(|pp| pp.addr() as u32 == e));
        let (los, near) = if i != 0 {
            let los = world_get_is_line_of_sight_clear(vp(&mut p), vp(&mut q), 1, 0, 0, 0, 0, 0, 0);
            let near = !x(p[2]).fsub_m(q[2]).fabs().fcom(x(kf(K_ONE))).test_parity(5);
            (los, near)
        } else {
            (1, true)
        };
        if blocked || los == 0 || !near {
            continue;
        }
        let ped = population_add_ped(ped_type, model as u32, GPtr::<u8>::cast::<f32>(vp(&mut p)), 0);
        if ped.is_null() {
            population_remove_ped(GPtr::NULL);
            continue;
        }
        placed[count] = ped;
        count += 1;
        let a = X87::from_f64_exact(general_get_radian_angle_between_points(ox, oy, p[0], p[1]).to_f64()).to_f32();
        wr::<f32>(ped, 0x558, a);
        wr::<f32>(ped, 0x55C, a);
        visibility_set_clump_alpha(GPtr::from_addr(rd::<u32>(ped, 0x18) as usize), 0);
    }
    if count < 1 {
        return 0;
    }
    let grp = GPtr::<u8>::from_addr((GROUPS + (group as u32) * 0x2D4) as usize);
    let intel = grp.byte_add(0x30);
    group_intel_set_default_task_allocator(intel, GPtr::from_addr(RANDOM_ALLOCATOR as usize));
    let members = grp.byte_add(8);
    for (k, &ped) in placed[..count].iter().enumerate() {
        if k == 0 {
            membership_set_leader(members, ped);
        } else {
            membership_add_follower(members, ped);
        }
        membership_process(members);
        group_intel_process(intel);
        let mem = task_new(0x38);
        let task = if mem.is_null() {
            GPtr::NULL
        } else {
            let r = at_site(0x5FD73E, rand) & 0xFFFF;
            let dir = X87::from_i32(r).fmul_m(kf(K_DIR1)).fmul_m(kf(K_DIR2)).ftol() as u32 as i32;
            wander_gang_ctor(mem, 4, dir, 0x1388, 1, g::<u32>(WANDER_ARG).read())
        };
        tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), task, 4, 0);
        let mem = task_new(0x28);
        let task = if mem.is_null() { GPtr::NULL } else { be_in_group_ctor(mem, group as u32, 0) };
        tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), task, 3, 0);
    }
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_PLACE_RANDOM_GROUP];
