//! CPopulation leftovers (WRITER-POP-4, lead): `AddToPopulation`. Ported from the 1.0 US bytes; gta-reversed and
//! `docs/re/specs/POP/CPopulation.md` as reading aids.

use sa_core::abi::{Fp, HookReg};
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, jne_mask, jp5, model_info, rand_at, rd, wr, x, xi};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

/// `ThePaths` (0x96F050).
const THE_PATHS: usize = 0x96F050;
/// `TheCamera` (0xB6F028).
const THE_CAMERA: usize = 0xB6F028;

use sa_world::bindings::camera_is_sphere_visible;
use crate::bindings::cull_zones_fewer_peds as fewer_peds;
use sa_world::bindings::is_sunbather;
use sa_world::bindings::entity_update_rw_frame as update_rw_frame;
use sa_core::bindings::object_new;
use sa_core::bindings::random_float_in_range as grn_f;
use sa_core::bindings::stricmp;
use sa_core::bindings::task_new;
use sa_core::bindings::tm_set_task_secondary as set_task_secondary;
use sa_world::bindings::world_add;
use sa_world::bindings::find_player_wanted;
use sa_stream::bindings::get_anim_group_name;
use sa_world::bindings::brains_check_if_new_entity_needs_script as check_if_new_entity_needs_script;
use sa_world::bindings::find_ground_z_for_3d_coord;
use sa_world::bindings::find_player_centre_of_world;
use sa_world::bindings::la_riots_active_here;
use sa_world::bindings::population_remove_all_random_peds as remove_all_random_peds;
use sa_world::bindings::set_clump_alpha;
use sa_world::bindings::set_ped_decision_maker_type;
use sa_world::bindings::set_posn;
use sa_world::bindings::update_rw_matrix;
use sa_world::bindings::wander_get_wander_task_by_ped_type as get_wander_task_by_ped_type;
use crate::bindings::dont_create_civilians;
use crate::bindings::generate_peds_at_attractors;
use sa_core::bindings::general_random_int_in_range as grn_i;
use sa_core::bindings::get_weapon_info;
use sa_stream::bindings::load_all_requested_models;
use sa_core::bindings::object_ctor_5a1d70 as object_ctor;
use sa_stream::bindings::col_load_collision as load_collision;
use crate::bindings::add_ped;
use crate::bindings::frenzy_on_going;
use crate::bindings::gang_war_fighting_going_on;
use sa_world::bindings::hold_entity_ctor;
use sa_world::bindings::is_position_clear_for_ped;
use sa_world::bindings::placeable_set_heading as set_heading;
use crate::bindings::population_remove_ped as remove_ped;
use sa_world::bindings::sunbathe_ctor;
use sa_world::bindings::task_wander_criminal_ctor as wander_criminal_ctor;
use sa_world::bindings::tm_set_task as set_task;
use sa_world::bindings::vector_add as vec_add;
use sa_world::bindings::vector_add_assign as vec_add_assign;
use sa_world::bindings::vector_mul_scalar as vec_mul;
use sa_world::bindings::world_process_vertical_line as process_vertical_line;
use sa_world::bindings::find_player_ped;
use crate::bindings::pdm_call;
use sa_core::bindings::cross_product;
orig! {
    /// `CPopCycle::FindNewPedType(ePedType*, int32*, bool, bool)` (0x60FBD0)
    fn find_new_ped_type(t: GPtr<i32>, m: GPtr<i32>, no_gangs: u8, no_cops: u8) -> u8 = cdecl 0x60FBD0 => crate::population2::find_new_ped_type;
    /// `CPopulation::ChooseCivilianCoupleOccupations(int32&, int32&)` (0x613180)
    fn choose_civilian_couple_occupations(h: GPtr<i32>, w: GPtr<i32>) = cdecl 0x613180 => crate::population2::choose_civilian_couple_occupations;
    /// `FUN_00610DB0` (0x610DB0): a random gang size
    fn random_gang_size() -> i32 = cdecl 0x610DB0 => crate::population2::random_gang_size;
    /// `CPathFind::GeneratePedCreationCoors(...)` (0x44E790, thiscall)
    fn generate_ped_creation_coors(this: GPtr<u8>, px: f32, py: f32, min1: f32, max1: f32, min2: f32, max2: f32, out: GPtr<f32>, a: GPtr<u32>, b: GPtr<u32>, orient: GPtr<f32>, low: u8, m: GPtr<u8>) -> u8 = thiscall 0x44E790 => crate::pathfind::generate_ped_creation_coors;
    /// `FUN_0044DA30(CNodeAddress, CNodeAddress, int16, float*, float*)` (0x44DA30, thiscall)
    fn add_wander_offset_between_nodes(this: GPtr<u8>, a: u32, b: u32, seed: u32, ox: GPtr<f32>, oy: GPtr<f32>) = thiscall 0x44DA30 => crate::pathfind::add_wander_offset_between_nodes;
    /// `CPopulation::PlaceGangMembers(ePedType, int32, const CVector&)` (0x6117D0)
    fn place_gang_members(t: i32, n: i32, pos: GPtr<f32>) -> u8 = cdecl 0x6117D0 => crate::population::place_gang_members;
    /// `CPed::GetWalkAnimSpeed()` (0x5E04B0, thiscall, ST0)
    fn get_walk_anim_speed(p: GPtr<u8>) -> f64 = thiscall 0x5E04B0 => unported;
    /// `CTaskComplexBeInCouple::CTaskComplexBeInCouple(CPed*, bool, bool, bool, float)` (0x6836F0)
    fn be_in_couple_ctor(this: GPtr<u8>, partner: GPtr<u8>, leader: u8, a: u8, b: u8, dist: f32) -> GPtr<u8> = thiscall 0x6836F0 => unported;
    /// `CPopulation::IsSkateable(const CVector&)` (0x6114C0)
    fn is_skateable(pos: GPtr<f32>) -> u8 = cdecl 0x6114C0 => crate::population2::is_skateable;
    /// `CPopulation::FindDistanceToNearestPedOfType(ePedType, CVector)` (0x6143E0, ST0)
    fn find_distance_to_nearest_ped_of_type(t: i32, px: f32, py: f32, pz: f32) -> f64 = cdecl 0x6143E0 => fdtnpot_abi;
    /// `SurfaceInfos_c::IsBeach(uint32)` (0x55E750, thiscall on `g_surfaceInfos` 0xB79538)
    fn is_beach(this: GPtr<u8>, surface: u32) -> u8 = thiscall 0x55E750 => unported;
    /// `CTaskComplexSunbathe::CanSunbathe()` (0x632140)
    fn can_sunbathe() -> u8 = cdecl 0x632140 => unported;
    /// `CWaterLevel::CreateBeachToy(const CVector&, eBeachToy)` (0x6EABA0)
    fn create_beach_toy(pos: GPtr<f32>, toy: i32) -> GPtr<u8> = cdecl 0x6EABA0 => crate::misc4::create_beach_toy;
    /// `CPathFind::GeneratePedCreationCoors_Interior(float, float, CVector*, CNodeAddress*, CNodeAddress*, float*)` (0x44ECA0)
    fn generate_ped_creation_coors_interior(this: GPtr<u8>, px: f32, py: f32, out: GPtr<f32>, a: GPtr<u32>, b: GPtr<u32>, orient: GPtr<f32>) -> u8 = thiscall 0x44ECA0 => crate::pathfind::generate_ped_creation_coors_interior;
}

/// The `orig!` routing target of 0x6143E0.
#[cfg_attr(sa_phase_a, allow(dead_code))]
fn fdtnpot_abi(t: i32, px: f32, py: f32, pz: f32) -> f64 {
    crate::population2::find_distance_to_nearest_ped_of_type(t, px, py, pz).to_f64()
}

/// `PedCreationDistMultiplier()` in ST0.
fn pdm() -> X87 {
    X87::from_f64_exact(pdm_call().to_f64())
}

/// `GetRandomNumberInRange<float>(lo, hi)` stored as f32.
fn rnd_f(lo: f32, hi: f32) -> f32 {
    X87::from_f64_exact(grn_f(lo, hi).to_f64()).to_f32()
}

use sa_world::world::common::entity_pos_f as entity_pos;

/// `fcomp` + `fnstsw ax` + `test ah, mask` + `jnp` taken.
fn jnp(a: X87, b: X87, mask: u8) -> bool {
    !a.fcom(b).test_parity(mask)
}

/// `CModelInfo::ms_modelInfoPtrs[m]->m_pColModel->boundSphere.radius` (+0x14, +0x24).
fn col_radius(m: i32) -> f32 {
    rd::<f32>(rd::<GPtr<u8>>(model_info(m), 0x14), 0x24)
}

/// `ms_aInfoForModel[GetWeaponInfo(w, 1)->m_nModelId].m_nLoadState == 1` (0x8E4CD0, stride 20).
fn weapon_model_loaded(w: i32) -> bool {
    let m = rd::<i32>(get_weapon_info(w, 1), 0xC);
    g::<u8>(0x8E4CD0usize.wrapping_add(m.wrapping_mul(20) as usize)).read() == 1
}

/// `(sqrt(dx*dx + dy*dy))` stored f32, with `d = a - b` (dy first on the stack, dx*dx + dy*dy summed).
fn dist2d(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dy = x(ay).fsub_m(by);
    let dx = x(ax).fsub_m(bx);
    dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt().to_f32()
}

/// The two locals the original may read uninitialised (`[entry ESP - 0x10C]` pedType, `[entry ESP - 0xE8]`
/// model): their values when the port returns, recorded as the original's stack writes.
struct Slots {
    ped_type: i32,
    model: i32,
}

/// The 32-bit residue at `entry ESP + off`.
fn residue32(off: i32) -> i32 {
    let mut v = 0u32;
    for k in 0..4 {
        v |= u32::from(sa_core::abi::footprint_residue_u8(off + k, 0x1000)) << (8 * k);
    }
    v as i32
}

/// `CPopulation::AddToPopulation(float r1, float r2, float r3, float r4)` (0x614720, SEH frame, AL), from the bytes
/// (docs/re/specs/POP/CPopulation.md has the step-by-step reading this follows):
/// - 0 when `DontCreateCivilians()` or the cheat byte 0x96917A; else `room = totalPeds < lim` (x87 `fcom`, PF test)
///   is the result of every later exit. `createCops` from the focus player's wanted level (> 2, fewer cops than its
///   +0x19 byte, no gang-war fight, ped flag 0x100 of +0x46C clear) and the car counts (law cars >= +0x1A, or
///   `P->+0x130 * CarDensityMultiplier <= NumRandomCars` with `jnp`, or the car sum >= MaxNumberOfCarsInUse); then
///   r1 / r2 become `PDM * 42.5` / `PDM * 50.5`. Riots with density > 0.1 and no cop also set it.
/// - cops: type 6, cop model 0, count 1; else `FindNewPedType` (both bools `ctr.z > 950`), the couple roll for
///   types 4 / 5 (`GetRandomNumberInRange(0, 1) > 0.9`), `m_AllRandomPedsThisType`, the gang size for 7..16.
///   `FindNewPedType` may return 1 writing nothing: the type / model then come from the stack residue (read
///   through the footprint snapshot, and their final values recorded as the original's writes).
/// - `GeneratePedCreationCoors` (gang ranges + 30), the spawn-rate test against `rand() & 15` (rand 0x614B71),
///   the wander offset with a second `rand()` (0x614B8B), gangs to `PlaceGangMembers`.
/// - couples: visibility / distance, clear-position and ground tests, the two `AddPed`s, the walk speed tests and
///   the `CTaskComplexBeInCouple` pair (leader = the slower walker), the wife moved by the offset at 0xC19708 and
///   both removed when the ground is missing or a third entity is in the way.
/// - single / group peds: the cop weapon / model loaded checks, `z += 0.7`, a `rand()` (0x61514C) per member, the
///   offset around the leader (rand 0x61519A / 0x6151AD for the signs), the clear-position test, ground snap for
///   all but the last member, the visibility / skater / sunbather-hour / dealer-distance / Grove-box tests, AddPed,
///   then sunbathers (rand 0x6153D1; towel, beach toys rand 0x61568F; task rand 0x61572E), riot behaviour (rand
///   0x6157B9: criminal wander or the TV-carrying task) or the brain script check, the leader and the clump alpha.
#[hook(addr = 0x614720, conv = cdecl, name = "CPopulation::AddToPopulation", expect = "6A FF 68 8E EF 83 00 64",
       footprint = 0x1000,
       route = "split", hazards = "heap-writes,uninit-locals,stack-residue,x87-transcendental",
       recs = "grove-walk,driving,car-ai,police-gang,world-cheats",
       note = "every frame from CPopulation::Update; creates peds, tasks, objects and beach toys (split: no double run); pedType/model locals can be residue when FindNewPedType writes nothing (SPLIT_STACKFILL); fsqrt distances")]
pub fn add_to_population(r1: f32, r2: f32, r3: f32, r4: f32) -> u8 {
    let mut s = Slots {
        ped_type: residue32(-0x10C),
        model: residue32(-0xE8),
    };
    let mut touched = false;
    let r = add_to_population_body(r1, r2, r3, r4, &mut s, &mut touched);
    if touched {
        sa_core::abi::stack_footprint(&[
            (-0x10C, Fp::Val(s.ped_type as u32)),
            (-0xE8, Fp::Val(s.model as u32)),
        ]);
    }
    r
}

#[allow(clippy::too_many_lines, clippy::needless_late_init, clippy::collapsible_match)]
fn add_to_population_body(r1: f32, r2: f32, r3: f32, r4: f32, s: &mut Slots, touched: &mut bool) -> u8 {
    let (mut r1, mut r2) = (r1, r2);
    let pif = g::<u8>(0xB7CD74).read();
    let player = 0xB7CD98 + usize::from(pif) * 0x190;
    let mut husband = [-1i32];
    let mut wife = [-1i32];
    let mut create_cops = false;
    let mut room = 0u8;
    let mut leader = GPtr::<u8>::NULL;
    if dont_create_civilians() != 0 || g::<u8>(0x96917A).read() != 0 {
        return 0;
    }
    let c = GPtr::<u8>::cast::<f32>(find_player_centre_of_world(i32::from(pif)));
    let ctr = [c.read(), c.wrapping_index(1).read(), c.wrapping_index(2).read()];
    let ped = || g::<GPtr<u8>>(player).read();
    let wanted = || {
        let w = rd::<GPtr<GPtr<u8>>>(ped(), 0x480);
        if w.is_null() { GPtr::NULL } else { w.read() }
    };
    if rd::<i32>(wanted(), 0x2C) > 2
        && g::<i32>(0xC0EC68).read() < i32::from(rd::<u8>(wanted(), 0x19))
        && gang_war_fighting_going_on() == 0
        && rd::<u32>(ped(), 0x46C) & 0x100 == 0
    {
        let law = g::<i32>(0x969098).read();
        let cops = if law >= i32::from(rd::<u8>(wanted(), 0x1A)) {
            true
        } else {
            let random_cars = xi(g::<i32>(0x969094).read());
            let want = xi(i32::from(g::<u16>(player + 0x130).read())).fmul_m(g::<f32>(0x8A5B20).read());
            if jnp(want, random_cars, 0x41) {
                true
            } else {
                let sum = g::<i32>(0x9690AC)
                    .read()
                    .wrapping_add(g::<i32>(0x9690A8).read())
                    .wrapping_add(g::<i32>(0x9690A0).read())
                    .wrapping_add(g::<i32>(0x96909C).read())
                    .wrapping_add(g::<i32>(0x969094).read())
                    .wrapping_add(law);
                sum >= g::<i32>(0x8A5B24).read()
            }
        };
        if cops {
            create_cops = true;
            r1 = pdm().fmul_m(42.5).to_f32();
            r2 = pdm().fmul_m(50.5).to_f32();
        }
    }
    if la_riots_active_here() != 0
        && !jne_mask(x(g::<f32>(0x8D2530).read()), x(f32::from_bits(0x3DCC_CCCD)), 0x41)
        && g::<i32>(0xC0EC68).read() < 1
    {
        create_cops = true;
    }
    let mut max_peds = xi(g::<i32>(0x8D2538).read()).to_f32();
    frenzy_on_going();
    let t = x(g::<f32>(0xC0BC40).read())
        .fadd_m(g::<f32>(0xC0BC44).read())
        .fadd_m(g::<f32>(0xC0BC48).read())
        .fadd_m(g::<f32>(0xC0E978).read());
    if x(max_peds).fcom(t).test_parity(5) {
        max_peds = t.to_f32();
    }
    if g::<i32>(0xB72914).read() != 0 {
        max_peds = xi(g::<i32>(0x8D253C).read()).to_f32();
    }
    let f = if fewer_peds() != 0 {
        x(f32::from_bits(0x3F19_999A))
    } else {
        x(1.0)
    };
    let lim = f.fmul_m(g::<f32>(0x8D2530).read()).fmul_m(max_peds);
    let total = xi(g::<i32>(0xC0EC28).read());
    let less = !total.fcom(lim).test_parity(5);
    if less {
        room = 1;
    }
    if !create_cops && !less {
        return room;
    }
    let count;
    *touched = true;
    if create_cops {
        s.ped_type = 6;
        s.model = 0;
        count = 1;
    } else {
        let z = u8::from(!jne_mask(x(ctr[2]), x(950.0), 0x41));
        let mut pt = [s.ped_type];
        let mut mdl = [s.model];
        let ok = find_new_ped_type(
            GPtr::from_raw(pt.as_mut_ptr()),
            GPtr::from_raw(mdl.as_mut_ptr()),
            z,
            z,
        );
        s.ped_type = pt[0];
        s.model = mdl[0];
        if ok == 0 {
            return room;
        }
        if s.ped_type == 4 || s.ped_type == 5 {
            let r = X87::from_f64_exact(grn_f(0.0, 1.0).to_f64());
            if !jne_mask(r, x(f32::from_bits(0x3F66_6666)), 0x41) {
                choose_civilian_couple_occupations(
                    GPtr::from_raw(husband.as_mut_ptr()),
                    GPtr::from_raw(wife.as_mut_ptr()),
                );
                if husband[0] == -1 || wife[0] == -1 {
                    return room;
                }
                s.ped_type = rd::<i32>(model_info(husband[0]), 0x28);
            }
        }
        let all = g::<i32>(0x8D2534).read();
        if all > 0 {
            s.ped_type = all;
        }
        count = if (7..=16).contains(&s.ped_type) {
            random_gang_size()
        } else {
            1
        };
    }
    let pt = s.ped_type;
    let model = s.model;
    let gang = (7..=16).contains(&pt);
    let (big1, big2) = if gang {
        (x(r1).fadd_m(30.0).to_f32(), x(r2).fadd_m(30.0).to_f32())
    } else {
        (r1, r2)
    };
    let low = u8::from(pt == 6 && rd::<i32>(find_player_wanted(-1), 0x2C) > 0);
    let mut cpos = [0f32; 3];
    let mut a1 = [0x0000_FFFFu32];
    let mut a2 = [0x0000_FFFFu32];
    let mut orient = [0f32];
    if generate_ped_creation_coors(
        g::<u8>(THE_PATHS),
        ctr[0],
        ctr[1],
        big1,
        big2,
        r3,
        r4,
        GPtr::from_raw(cpos.as_mut_ptr()),
        GPtr::from_raw(a1.as_mut_ptr()),
        GPtr::from_raw(a2.as_mut_ptr()),
        GPtr::from_raw(orient.as_mut_ptr()),
        low,
        GPtr::NULL,
    ) == 0
    {
        return room;
    }
    let spawn = |a: u32| {
        let nodes = g::<GPtr<u8>>(0x96F854 + (a & 0xFFFF) as usize * 4).read();
        rd::<u8>(nodes, ((a >> 16) * 0x1C + 0x1A) as isize) & 0xF
    };
    let p = spawn(a2[0]).min(spawn(a1[0]));
    if (rand_at(0x614B71) & 0xF) > i32::from(p) {
        return room;
    }
    let seed = rand_at(0x614B8B);
    let cp = GPtr::from_raw(cpos.as_mut_ptr());
    add_wander_offset_between_nodes(
        g::<u8>(THE_PATHS),
        a1[0],
        a2[0],
        seed as u32,
        cp,
        cp.wrapping_index(1),
    );
    if gang {
        place_gang_members(pt, count, cp);
        return room;
    }
    if husband[0] >= 0 && wife[0] >= 0 {
        add_couple(husband[0], wife[0], cpos);
        return room;
    }
    if count <= 0 {
        return room;
    }
    let mut i = 0i32;
    loop {
        if pt == 6 {
            match model {
                4 => {
                    if g::<u8>(0x8E6328).read() != 1 || !weapon_model_loaded(0x1D) {
                        return room;
                    }
                }
                2 => {
                    if g::<u8>(0x8E6314).read() != 1 || !weapon_model_loaded(0x1C) {
                        return room;
                    }
                }
                5 => {
                    if g::<u8>(0x8E633C).read() != 1
                        || !weapon_model_loaded(0x1D)
                        || !weapon_model_loaded(0x10)
                    {
                        return room;
                    }
                }
                _ => {}
            }
        } else if rd::<u32>(model_info(model), 0x1C) == 0 {
            return room;
        }
        cpos[2] = x(cpos[2]).fadd_m(f32::from_bits(0x3F33_3333)).to_f32();
        if i < count {
            rand_at(0x61514C);
            if !leader.is_null() {
                let k = xi(i);
                let hi = x(1.0).fadd(k).fmul_m(0.75).to_f32();
                let lo = k.fmul_m(0.75).to_f32();
                let mut a = rnd_f(lo, hi);
                let b = rnd_f(lo, hi);
                if rand_at(0x61519A) & 1 != 0 {
                    a = x(a).fchs().to_f32();
                }
                let bx = if rand_at(0x6151AD) & 1 != 0 {
                    x(b).fchs()
                } else {
                    x(b)
                };
                cpos[0] = x(a).fadd_m(entity_pos(leader).read()).to_f32();
                cpos[1] = bx.fadd_m(entity_pos(leader).wrapping_index(1).read()).to_f32();
            }
        }
        if is_position_clear_for_ped(cp.cast::<u8>(), -1.0, -1, GPtr::<u32>::cast::<u8>(GPtr::NULL), 1, 1, 1) == 0 {
            return room;
        }
        let next = i.wrapping_add(1);
        if next < count {
            let mut found = [0u8];
            let gz = X87::from_f64_exact(find_ground_z_for_3d_coord(
                cpos[0],
                cpos[1],
                x(cpos[2]).fadd_m(2.0).to_f32(),
                GPtr::from_raw(found.as_mut_ptr()),
                GPtr::NULL,
            ))
            .fadd_m(f32::from_bits(0x3F33_3333));
            if found[0] == 0 {
                return room;
            }
            if jne_mask(x(cpos[2]), gz, 0x41) {
                cpos[2] = gz.to_f32();
            }
        }
        let mut ok = 1u8;
        let visible = camera_is_sphere_visible(g::<u8>(THE_CAMERA), cp.cast::<u8>(), 2.0) != 0;
        if visible
            && !jne_mask(
                pdm().fmul_m(42.5),
                x(dist2d(cpos[0], cpos[1], ctr[0], ctr[1])),
                0x41,
            )
        {
            ok = 0;
        } else if rd::<i32>(model_info(model), 0x2C) == 0x28 {
            ok = is_skateable(cp);
        }
        if is_sunbather(model) != 0 {
            let h = g::<u8>(0xB70153).read();
            if !(8..=0x13).contains(&h) {
                ok = 0;
            }
        }
        if pt == 0x11 {
            if ok == 0 {
                return room;
            }
            let d = X87::from_f64_exact(find_distance_to_nearest_ped_of_type(
                pt, cpos[0], cpos[1], cpos[2],
            ));
            if jnp(d, x(20.0), 5) {
                return room;
            }
        }
        if ok == 0 {
            return room;
        }
        if (7..=16).contains(&pt)
            && pt != 8
            && !jne_mask(x(cpos[0]), x(2400.0), 0x41)
            && !jp5(x(cpos[0]), x(2540.0))
            && !jne_mask(x(cpos[1]), x(-1730.0), 0x41)
            && jnp(x(cpos[1]), x(-1625.0), 5)
        {
            return room;
        }
        let p = add_ped(pt as u32, model as u32, cp, 1);
        if !(is_sunbather(model) != 0 && rand_at(0x6153D1) & 3 != 0 && sunbathe(p, &mut cpos)) {
            others(p, pt);
        }
        if i == 0 {
            leader = p;
        }
        set_clump_alpha(rd(p, 0x18), 0);
        if next > count || count <= 1 || next >= count {
            return room;
        }
        i = next;
    }
}

/// The couple branch of `AddToPopulation` (0x614BDC..0x615049); `q` = the creation point.
fn add_couple(husband: i32, wife: i32, mut q: [f32; 3]) {
    if la_riots_active_here() != 0 {
        return;
    }
    let qp = GPtr::from_raw(q.as_mut_ptr());
    if camera_is_sphere_visible(g::<u8>(THE_CAMERA), qp.cast::<u8>(), 1.5) != 0 {
        let pp = entity_pos(find_player_ped(-1));
        let d = dist2d(q[0], q[1], pp.read(), pp.wrapping_index(1).read());
        if !jne_mask(pdm().fmul_m(42.5), x(d), 0x41) {
            return;
        }
    }
    if is_position_clear_for_ped(qp.cast::<u8>(), col_radius(husband), -1, GPtr::<u32>::cast::<u8>(GPtr::NULL), 1, 1, 1) == 0 {
        return;
    }
    let mut found = [0u8];
    let gz = X87::from_f64_exact(find_ground_z_for_3d_coord(
        q[0],
        q[1],
        x(q[2]).fadd_m(1.0).to_f32(),
        GPtr::from_raw(found.as_mut_ptr()),
        GPtr::NULL,
    ))
    .fadd_m(1.0);
    if found[0] == 0 {
        return;
    }
    let z = if jne_mask(x(q[2]), gz, 0x41) {
        gz.to_f32()
    } else {
        q[2]
    };
    if rd::<u32>(model_info(husband), 0x1C) == 0 {
        return;
    }
    let mut at = [q[0], q[1], z];
    let h = add_ped(4, husband as u32, GPtr::from_raw(at.as_mut_ptr()), 1);
    if h.is_null() {
        return;
    }
    set_clump_alpha(rd(h, 0x18), 0);
    if rd::<u32>(model_info(wife), 0x1C) == 0 {
        return;
    }
    let mut at = [q[0], q[1], z];
    let w = add_ped(5, wife as u32, GPtr::from_raw(at.as_mut_ptr()), 1);
    if w.is_null() {
        return;
    }
    let sw = X87::from_f64_exact(get_walk_anim_speed(w)).to_f32();
    let sh = X87::from_f64_exact(get_walk_anim_speed(h)).to_f32();
    if jnp(x(sw), x(0.75), 5) || jnp(x(sh), x(0.75), 5) {
        return;
    }
    if !jne_mask(x(sw).fsub_m(sh).fabs(), x(f32::from_bits(0x3EE6_6666)), 0x41) {
        return;
    }
    let couple = |owner: GPtr<u8>, partner: GPtr<u8>, is_leader: u8| {
        let t = task_new(0x20);
        let t = if t.is_null() {
            GPtr::NULL
        } else {
            be_in_couple_ctor(t, partner, is_leader, 1, 1, 10.0)
        };
        set_task(rd::<GPtr<u8>>(owner, 0x47C).byte_add(4), t, 3, 0);
    };
    if !jne_mask(x(sw), x(sh), 0x41) {
        couple(h, w, 0);
        couple(w, h, 1);
    } else {
        couple(w, h, 0);
        couple(h, w, 1);
    }
    let mut off = [
        g::<f32>(0xC19708).read(),
        x(g::<f32>(0xC1970C).read()).to_f32(),
        0.0,
    ];
    let mut wp = [0f32; 3];
    vec_add(
        GPtr::<f32>::cast::<u8>(GPtr::from_raw(wp.as_mut_ptr())),
        GPtr::<f32>::cast::<u8>(entity_pos(h)),
        GPtr::<f32>::cast::<u8>(GPtr::from_raw(off.as_mut_ptr())),
    );
    let mut found = [0u8];
    let gz = X87::from_f64_exact(find_ground_z_for_3d_coord(
        wp[0],
        wp[1],
        x(wp[2]).fadd_m(1.0).to_f32(),
        GPtr::from_raw(found.as_mut_ptr()),
        GPtr::NULL,
    ))
    .fadd_m(1.0);
    if found[0] == 0 {
        remove_ped(h);
        remove_ped(w);
        return;
    }
    let z = if jne_mask(x(wp[2]), gz, 0x41) {
        gz.to_f32()
    } else {
        wp[2]
    };
    set_posn(w, wp[0], wp[1], z);
    let mut ents = [0u32; 3];
    is_position_clear_for_ped(
        GPtr::<f32>::cast::<u8>(GPtr::from_raw(wp.as_mut_ptr())),
        col_radius(wife),
        3,
        GPtr::<u32>::cast::<u8>(GPtr::from_raw(ents.as_mut_ptr())),
        1,
        1,
        1,
    );
    for &e in &ents {
        if e != 0 && e != h.addr() as u32 && e != w.addr() as u32 {
            remove_ped(h);
            remove_ped(w);
            return;
        }
    }
    set_clump_alpha(rd(w, 0x18), 0);
}

/// The sunbather branch (0x6153D9..0x6157A5) after its `rand() & 3` test: false when it falls through to the
/// other-peds code (no ground entity, wrong surface, no sunbathing, a blocking entity), true when done (task set,
/// or no ground / too steep under the ped: nothing more).
fn sunbathe(ped: GPtr<u8>, cpos: &mut [f32; 3]) -> bool {
    let mut line = [cpos[0], cpos[1], x(cpos[2]).fadd_m(2.0).to_f32()];
    let zlow = x(cpos[2]).fsub_m(2.0).to_f32();
    let mut cp = [0u32; 11];
    let mut ent = [0u32];
    process_vertical_line(
        GPtr::<f32>::cast::<u8>(GPtr::from_raw(line.as_mut_ptr())),
        zlow,
        GPtr::<u32>::cast::<u8>(GPtr::from_raw(cp.as_mut_ptr())),
        GPtr::<u32>::cast::<u8>(GPtr::from_raw(ent.as_mut_ptr())),
        1,
        0,
        0,
        0,
        0,
        0,
        GPtr::NULL,
    );
    if ent[0] == 0 {
        return false;
    }
    let surface = (cp[8] >> 24) as u8;
    let cheat = || g::<u8>(0x969159).read() != 0;
    if is_beach(g::<u8>(0xB79538), u32::from(surface)) == 0 && surface != 0x22 && surface != 0xA0 && !cheat()
    {
        return false;
    }
    if can_sunbathe() == 0 && !cheat() {
        return false;
    }
    let mut ents = [0u32; 2];
    let mut ok = true;
    if is_position_clear_for_ped(
        GPtr::<f32>::cast::<u8>(GPtr::from_raw(cpos.as_mut_ptr())),
        3.0,
        2,
        GPtr::<u32>::cast::<u8>(GPtr::from_raw(ents.as_mut_ptr())),
        1,
        1,
        1,
    ) != 0
    {
        for &e in &ents {
            if e != 0 && e != ped.addr() as u32 {
                ok = false;
            }
        }
    }
    let towel = surface != 0xA0;
    if !ok {
        return false;
    }
    let pp = entity_pos(ped);
    let (px, py, pz) = (
        pp.read(),
        pp.wrapping_index(1).read(),
        pp.wrapping_index(2).read(),
    );
    let mut line2 = [px, py, x(pz).fadd_m(10.0).to_f32()];
    let mut cp2 = [0f32; 11];
    let mut ent2 = [0u32];
    if process_vertical_line(
        GPtr::<f32>::cast::<u8>(GPtr::from_raw(line2.as_mut_ptr())),
        -10.0,
        GPtr::<u32>::cast::<u8>(GPtr::from_raw(cp2.as_mut_ptr()).cast()),
        GPtr::<u32>::cast::<u8>(GPtr::from_raw(ent2.as_mut_ptr())),
        1,
        0,
        0,
        0,
        1,
        0,
        GPtr::NULL,
    ) == 0
    {
        return true;
    }
    let (nx, ny, nz) = (cp2[4], cp2[5], cp2[6]);
    if jne_mask(
        x(nx).fadd_m(ny).fmul_m(0.0).fadd_m(nz),
        x(f32::from_bits(0x3F73_3333)),
        0x41,
    ) {
        return true;
    }
    let hd = X87::from_f64_exact(grn_f(0.0, 1.0).to_f64())
        .fmul_m(f32::from_bits(0x40C9_0FDB))
        .to_f32();
    wr::<f32>(ped, 0x55C, hd);
    wr::<f32>(ped, 0x558, hd);
    set_heading(ped, hd);
    let mut toy = GPtr::<u8>::NULL;
    if towel {
        let mut tp = [px, py, x(cp2[2]).fadd_m(f32::from_bits(0x3D23_D70A)).to_f32()];
        toy = create_beach_toy(GPtr::from_raw(tp.as_mut_ptr()), 11);
        if !toy.is_null() {
            set_heading(toy, hd);
            let m = rd::<GPtr<u8>>(toy, 0x14);
            wr::<f32>(m, 0x20, nx);
            wr::<f32>(m, 0x24, ny);
            wr::<f32>(m, 0x28, nz);
            let m: GPtr<f32> = rd::<GPtr<u8>>(toy, 0x14).cast();
            let mut tmp = [0f32; 3];
            cross_product(
                GPtr::<f32>::cast::<u8>(GPtr::from_raw(tmp.as_mut_ptr())),
                GPtr::<f32>::cast::<u8>(m.wrapping_index(8)),
                GPtr::<f32>::cast::<u8>(m.wrapping_index(4)),
            );
            for (k, v) in tmp.iter().enumerate() {
                m.wrapping_index(k as i32).write(*v);
            }
            let m: GPtr<f32> = rd::<GPtr<u8>>(toy, 0x14).cast();
            cross_product(GPtr::<f32>::cast::<u8>(GPtr::from_raw(tmp.as_mut_ptr())), GPtr::<f32>::cast::<u8>(m.wrapping_index(8)), m.cast::<u8>());
            for (k, v) in tmp.iter().enumerate() {
                m.wrapping_index(4 + k as i32).write(*v);
            }
            update_rw_matrix(toy);
            update_rw_frame(toy);
            if rand_at(0x61568F) & 3 == 0 {
                let mut bpos = tp;
                let bp = GPtr::from_raw(bpos.as_mut_ptr());
                let m: GPtr<f32> = rd::<GPtr<u8>>(toy, 0x14).cast();
                let r = rnd_f(-0.5, 0.5);
                let mut t1 = [0f32; 3];
                let o1 = GPtr::<u8>::cast::<f32>(vec_mul(GPtr::<f32>::cast::<u8>(GPtr::from_raw(t1.as_mut_ptr())), m.cast::<u8>(), r));
                vec_add_assign(bp.cast::<u8>(), o1.cast::<u8>());
                let m: GPtr<f32> = rd::<GPtr<u8>>(toy, 0x14).cast();
                let r = rnd_f(-1.0, 1.0);
                let mut t2 = [0f32; 3];
                let o2 = GPtr::<u8>::cast::<f32>(vec_mul(GPtr::<f32>::cast::<u8>(GPtr::from_raw(t2.as_mut_ptr())), GPtr::<f32>::cast::<u8>(m.wrapping_index(4)), r));
                vec_add_assign(bp.cast::<u8>(), o2.cast::<u8>());
                create_beach_toy(bp, 6);
            }
        }
    }
    let flag = u8::from(rand_at(0x61572E) & 3 == 0);
    let t = task_new(0x38);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        sunbathe_ctor(t, toy, flag)
    };
    set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 3, 0);
    true
}

/// The other-peds branch (0x6157A7..0x61590C): riot behaviour or the brain script check.
fn others(ped: GPtr<u8>, pt: i32) {
    if la_riots_active_here() == 0 {
        check_if_new_entity_needs_script(g::<u8>(0xA90CF0), ped, 0, 0);
        return;
    }
    if rand_at(0x6157B9) & 3 != 0 {
        let t = task_new(0x38);
        let t = if t.is_null() {
            GPtr::NULL
        } else {
            let dir = grn_i(0, 8);
            wander_criminal_ctor(t, 6, dir as u32, 1)
        };
        set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4, 0);
        return;
    }
    if pt == 6 {
        return;
    }
    let o = object_new(0x17C);
    let o = if o.is_null() {
        GPtr::NULL
    } else {
        object_ctor(o, i32::from(g::<u16>(0x8CD5A8).read()), 1)
    };
    world_add(o);
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let w = get_wander_task_by_ped_type(ped);
    set_task(intel.byte_add(4), w, 4, 0);
    let mut v = [0.0f32, f32::from_bits(0x3EE6_6666), f32::from_bits(0x3EB3_3333)];
    let t = task_new(0x3C);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        hold_entity_ctor(t, o, GPtr::<f32>::cast::<u8>(GPtr::from_raw(v.as_mut_ptr())), 1, 1, 0x13E, 0x51, 0)
    };
    set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4);
}

/// The first anim group index `< CAnimManager::ms_numAnimAssocDefinitions` (0xB4EA28, re-read every step) whose
/// name equals `name` (`_stricmp`), or that count when none does.
fn find_anim_group(name: usize) -> (i32, bool) {
    let mut i = 0;
    while i < g::<i32>(0xB4EA28).read() {
        if stricmp(get_anim_group_name(i as u32), g::<u8>(name)) == 0 {
            return (i, true);
        }
        i += 1;
    }
    (i, false)
}

/// `CPopulation::PopulateInterior(int32 numPeds, CVector pos)` (0x616470), from the bytes: nothing below z 900
/// (`test ah, 5; jnp`); `RemoveAllRandomPeds`, `LoadCollision(pos, true)`, `LoadAllRequestedModels(false)`; with
/// `numPeds > 0`, `GeneratePedsAtAttractors(pos, 0, 150, 0, 150, 7, ftol(numPeds * 0.95))` and then up to 25
/// tries while peds remain: `GeneratePedCreationCoors_Interior` (the two node-address area words reset to
/// 0xFFFF each try), `FindNewPedType(&type, &model, true, true)`, `z += 0.9` (stored), `AddPed(type, model, &c,
/// true)` (not null-checked), decision maker 7, and a ped in the "jogger" anim group (0x86D290) moved to "man"
/// (0x86D28C) when that group exists. `FindNewPedType` can succeed writing nothing: type / model are then the
/// previous try's, or the stack residue on the first (`[entry ESP - 0x14]` / `- 0x18`, footprint snapshot).
#[hook(addr = 0x616470, conv = cdecl, name = "CPopulation::PopulateInterior", expect = "D9 44 24 10 83 EC 24 D8",
       footprint = 0x1000,
       unverified = "popw4-pop-1", route = "split", hazards = "heap-writes,uninit-locals,stack-residue",
       note = "on entering interiors (script / entry-exit); creates peds; type/model locals can be residue")]
pub fn populate_interior(num_peds: i32, px: f32, py: f32, pz: f32) {
    if jnp(x(pz), x(900.0), 5) {
        return;
    }
    let mut ty = residue32(-0x14);
    let mut model = residue32(-0x18);
    remove_all_random_peds();
    load_collision(px, py, pz, 1);
    load_all_requested_models(0);
    if num_peds <= 0 {
        return;
    }
    let n = crate::common::ftol32(xi(num_peds).fmul_m(f32::from_bits(0x3F73_3333)));
    let mut left = num_peds.wrapping_sub(generate_peds_at_attractors(
        px, py, pz, 0.0, 150.0, 0.0, 150.0, 7, n,
    ));
    let mut tries = 0;
    let mut touched = false;
    while left > 0 && tries < 25 {
        tries += 1;
        let mut a = [0x0000_FFFFu32];
        let mut b = [0x0000_FFFFu32];
        let mut c = [0f32; 3];
        let mut orient = [0f32];
        if generate_ped_creation_coors_interior(
            g::<u8>(THE_PATHS),
            px,
            py,
            GPtr::from_raw(c.as_mut_ptr()),
            GPtr::from_raw(b.as_mut_ptr()),
            GPtr::from_raw(a.as_mut_ptr()),
            GPtr::from_raw(orient.as_mut_ptr()),
        ) == 0
        {
            continue;
        }
        let mut t = [ty];
        let mut m = [model];
        let ok = find_new_ped_type(
            GPtr::from_raw(t.as_mut_ptr()),
            GPtr::from_raw(m.as_mut_ptr()),
            1,
            1,
        );
        ty = t[0];
        model = m[0];
        touched = true;
        if ok == 0 {
            continue;
        }
        c[2] = x(c[2]).fadd_m(f32::from_bits(0x3F66_6666)).to_f32();
        let ped = add_ped(ty as u32, model as u32, GPtr::from_raw(c.as_mut_ptr()), 1);
        left -= 1;
        set_ped_decision_maker_type(rd(ped, 0x47C), 7);
        let (jogger, _) = find_anim_group(0x86D290);
        if rd::<i32>(ped, 0x4D4) == jogger {
            let (man, found) = find_anim_group(0x86D28C);
            if found {
                wr::<i32>(ped, 0x4D4, man);
            }
        }
    }
    if touched {
        sa_core::abi::stack_footprint(&[(-0x14, Fp::Val(ty as u32)), (-0x18, Fp::Val(model as u32))]);
    }
}

use sa_core::bindings::matrix_update_rw_matrix as matrix_update_rw;
use sa_world::bindings::automobile_place_on_road_properly as place_on_road_properly;
use sa_world::bindings::intel_process;
use crate::bindings::join_car_with_road_system;
use sa_world::bindings::kill_ped_on_foot_ctor;
use sa_world::bindings::ped_new;
use sa_world::bindings::ped_set_current_weapon as set_current_weapon;
use sa_world::bindings::simple_transform_update_rw_matrix;
use sa_world::bindings::uses_siren;
use sa_world::bindings::vehicle_new;
use sa_core::bindings::automobile_ctor;
use sa_core::bindings::matrix_set_rotate_z_only;
use sa_core::bindings::ped_give_weapon as give_weapon;
use sa_core::bindings::world_get_is_line_of_sight_clear as get_is_line_of_sight_clear;
use sa_stream::bindings::get_default_cop_car_model;
use sa_world::bindings::cop_ped_ctor;
use sa_world::bindings::event_group_add;
use sa_world::bindings::event_script_command_ctor;
use sa_world::bindings::event_script_command_dtor;
use crate::bindings::find_nth_node_closest_to_coors;
use sa_world::bindings::find_objects_in_range;
use sa_core::bindings::vector_normalise;
orig! {
    /// virtual slot 0x10 of a `CEntity`: `SetIsStatic(bool)`
    fn entity_set_is_static(e: GPtr<u8>, b: u8) = thiscall vslot 0x10 => unported;
}

/// The cars of `CreateWaitingCoppers` (0x613499..0x6136C4): `n` police cars (model `GetDefaultCopCarModel(1)`,
/// nothing when <= 0) on up to three nodes (`FindNthNodeClosestToCoors(pos, 0, 30, ..., 3, ..., true, &nodes[1])`,
/// the address array built by the `CNodeAddress` constructor: area words 0xFFFF), each placed at its node, facing
/// the point (`m.fwd = normalised (dx, dy, 0)`, `m.right = (-fwd.y, fwd.x, 0)`), put on the road, not static, its
/// RW matrix updated, joined to the road system, a siren car's +0x42D bit 7 on `rand() & 1` (0x6136AB), added.
fn create_waiting_cop_cars(mut n: i32, px: f32, py: f32, pz: f32) {
    let model = get_default_cop_car_model(1);
    if model <= 0 {
        return;
    }
    let mut nodes = [0x0000_FFFFu32; 3];
    let mut ret = [0u32];
    let r = find_nth_node_closest_to_coors(
        g::<u8>(THE_PATHS),
        GPtr::from_raw(ret.as_mut_ptr()),
        px,
        py,
        pz,
        0,
        30.0,
        0,
        0,
        3,
        0,
        1,
        GPtr::from_raw(nodes.as_mut_ptr()).wrapping_index(1),
    );
    nodes[0] = r.read();
    for &a in &nodes {
        if n == 0 || a as u16 == 0xFFFF {
            continue;
        }
        let v = vehicle_new(0x988);
        let v = if v.is_null() {
            GPtr::NULL
        } else {
            automobile_ctor(v, model as u32, 1, 1)
        };
        wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x20);
        let base = g::<GPtr<u8>>(0x96F854 + (a & 0xFFFF) as usize * 4).read();
        let node = base.byte_add(((a >> 16) * 0x1C) as isize);
        let coord = |off: isize| xi(i32::from(rd::<i16>(node, off))).fmul_m(0.125).to_f32();
        let (nz, ny, nx) = (coord(0xC), coord(0xA), coord(8));
        let m = rd::<GPtr<u8>>(v, 0x14);
        if m.is_null() {
            wr::<f32>(v, 4, nx);
            wr::<f32>(v, 8, ny);
            wr::<f32>(v, 0xC, nz);
        } else {
            wr::<f32>(m, 0x30, nx);
            wr::<f32>(rd(v, 0x14), 0x34, ny);
            wr::<f32>(rd(v, 0x14), 0x38, nz);
        }
        let pos = entity_pos(v);
        let dy = x(py).fsub_m(pos.wrapping_index(1).read());
        let dx = x(px).fsub_m(pos.read());
        let mut d = [dx.to_f32(), dy.to_f32(), 0.0];
        vector_normalise(GPtr::<f32>::cast::<[f32;3]>(GPtr::from_raw(d.as_mut_ptr())));
        let m = rd::<GPtr<u8>>(v, 0x14);
        wr::<f32>(m, 0x10, d[0]);
        wr::<f32>(m, 0x14, d[1]);
        wr::<f32>(m, 0x18, d[2]);
        let rx = x(d[1]).fchs().to_f32();
        let m = rd::<GPtr<u8>>(v, 0x14);
        wr::<f32>(m, 0, rx);
        wr::<f32>(m, 4, d[0]);
        wr::<u32>(m, 8, 0);
        place_on_road_properly(v);
        entity_set_is_static(v, 0);
        let rw = rd::<GPtr<u8>>(v, 0x18);
        if !rw.is_null() {
            let f = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
            let m = rd::<GPtr<u8>>(v, 0x14);
            if m.is_null() {
                simple_transform_update_rw_matrix(v.byte_add(4), f);
            } else {
                matrix_update_rw(m, f);
            }
        }
        join_car_with_road_system(v);
        if uses_siren(v) != 0 && rand_at(0x6136AB) & 1 != 0 {
            wr::<u8>(v, 0x42D, rd::<u8>(v, 0x42D) | 0x80);
        }
        world_add(v);
        n -= 1;
    }
}

/// `CPopulation::CreateWaitingCoppers(CVector pos, float angle)` (0x6133F0, SEH frame), from the bytes: `pos.z +=
/// 1.0` is stored into its own argument slot (`[entry ESP + 0xC]`, recorded as a footprint write). By the focus
/// player's wanted level (jump table 0x6139E4): 1 → 1 cop, 2 → 2, 3 / 4 → 4 / 5 cops and 1 car, 5 / 6 → 6 / 7
/// cops and 2 cars (other levels: nothing). Then 20 tries (i = 0.., j = 0, -1, ..) while cops remain: radius
/// `rand() * (1/32767) * 2 + 8` (rand 0x6136E5), angle `rand() * (1/32767) * (i * 0.2 - j * 0.2) + j * 0.2 +
/// angle` (rand 0x613712, 80-bit), the point `(sin * r + x, cos * r + y, z)` (FSIN / FCOS); with a clear line of
/// sight from `pos` and no vehicle / ped within 1.0, the ground z + 1 (stored even when not found), then a
/// `CCopPed(0)` there facing `pos` (FPATAN of the normalised direction), added, the shotgun (0x16, 30000) when the
/// wanted level is > 1, and a `CTaskComplexKillPedOnFoot(player)` (flags +0xC |= 0x30, task used unchecked) given
/// through a `CEventScriptCommand(3)` and `CPedIntelligence::Process`.
#[hook(addr = 0x6133F0, conv = cdecl, name = "CPopulation::CreateWaitingCoppers", expect = "64 A1 00 00 00 00 D9 44",
       footprint = 0x1000,
       unverified = "popw4-pop-1", route = "split", hazards = "heap-writes,x87-transcendental,stack-residue",
       note = "script / road-block entry; creates cop peds and police cars; writes its own z argument slot")]
pub fn create_waiting_coppers(px: f32, py: f32, pz: f32, angle: f32) {
    let za = x(pz).fadd_m(1.0).to_f32();
    let w = rd::<GPtr<GPtr<u8>>>(find_player_ped(-1), 0x480);
    let wanted = if w.is_null() { GPtr::NULL } else { w.read() };
    let (mut cops, cars) = match rd::<i32>(wanted, 0x2C) {
        1 => (1, 0),
        2 => (2, 0),
        3 => (4, 1),
        4 => (5, 1),
        5 => (6, 2),
        6 => (7, 2),
        _ => (0, 0),
    };
    if cars != 0 {
        create_waiting_cop_cars(cars, px, py, za);
    }
    let inv = f32::from_bits(0x3800_0100);
    let mut arg = [px, py, za];
    let (mut i, mut j) = (0i32, 0i32);
    loop {
        if cops != 0 {
            let r = rand_at(0x6136E5);
            let rf = xi(r).fmul_m(inv);
            let radius = rf.fadd(rf).fadd_m(8.0).to_f32();
            let lo = xi(j).fmul_m(f32::from_bits(0x3E4C_CCCD)).to_f32();
            let r2 = rand_at(0x613712);
            let diff = xi(i).fmul_m(f32::from_bits(0x3E4C_CCCD)).fsub_m(lo);
            let a = xi(r2).fmul_m(inv).fmul(diff).fadd_m(lo).fadd_m(angle);
            let mut p = [
                a.fsin().fmul_m(radius).fadd_m(px).to_f32(),
                a.fcos().fmul_m(radius).fadd_m(py).to_f32(),
                za,
            ];
            let pp = GPtr::from_raw(p.as_mut_ptr());
            if get_is_line_of_sight_clear(GPtr::<f32>::cast::<u8>(GPtr::from_raw(arg.as_mut_ptr())), pp.cast::<u8>(), 1, 1, 0, 1, 0, 0, 0) != 0 {
                let mut cnt = [0i16; 2];
                let mut ents = [0u32];
                find_objects_in_range(
                    pp.cast::<u8>(),
                    1.0,
                    1,
                    GPtr::<i16>::cast::<u8>(GPtr::from_raw(cnt.as_mut_ptr())),
                    1,
                    GPtr::<u32>::cast::<u8>(GPtr::from_raw(ents.as_mut_ptr())),
                    0,
                    1,
                    1,
                    0,
                    0,
                );
                if cnt[0] == 0 {
                    let mut found = [0u8];
                    let gz = X87::from_f64_exact(find_ground_z_for_3d_coord(
                        p[0],
                        p[1],
                        p[2],
                        GPtr::from_raw(found.as_mut_ptr()),
                        GPtr::NULL,
                    ))
                    .fadd_m(1.0);
                    p[2] = gz.to_f32();
                    if found[0] != 0 {
                        spawn_waiting_cop(&p, px, py, za);
                        cops -= 1;
                    }
                }
            }
        }
        i += 1;
        j -= 1;
        if j <= -20 {
            break;
        }
    }
    sa_core::abi::stack_footprint(&[(0xC, Fp::Val(za.to_bits()))]);
}

/// One cop of `CreateWaitingCoppers` (0x613810..0x6139A9) at `p`, facing `(px, py, pz)`.
fn spawn_waiting_cop(p: &[f32; 3], px: f32, py: f32, pz: f32) {
    let c = ped_new(0x7C4);
    let cop = if c.is_null() {
        GPtr::NULL
    } else {
        cop_ped_ctor(c, 0)
    };
    let m = rd::<GPtr<u8>>(cop, 0x14);
    if m.is_null() {
        wr::<f32>(cop, 8, x(p[1]).to_f32());
        wr::<f32>(cop, 4, p[0]);
        wr::<f32>(cop, 0xC, x(p[2]).to_f32());
    } else {
        wr::<f32>(m, 0x30, p[0]);
        wr::<f32>(rd(cop, 0x14), 0x34, x(p[1]).to_f32());
        wr::<f32>(rd(cop, 0x14), 0x38, x(p[2]).to_f32());
    }
    let mut v = [
        x(px).fsub_m(p[0]).to_f32(),
        x(py).fsub_m(p[1]).to_f32(),
        x(pz).fsub_m(p[2]).to_f32(),
    ];
    vector_normalise(GPtr::<f32>::cast::<[f32;3]>(GPtr::from_raw(v.as_mut_ptr())));
    let h = x(v[0]).fchs().fpatan(x(v[1])).to_f32();
    wr::<f32>(cop, 0x55C, h);
    wr::<f32>(cop, 0x558, h);
    let m = rd::<GPtr<u8>>(cop, 0x14);
    if m.is_null() {
        wr::<f32>(cop, 0x10, h);
    } else {
        matrix_set_rotate_z_only(m, h);
    }
    world_add(cop);
    let w = rd::<GPtr<GPtr<u8>>>(find_player_ped(-1), 0x480);
    let wanted = if w.is_null() { GPtr::NULL } else { w.read() };
    if rd::<i32>(wanted, 0x2C) > 1 {
        give_weapon(cop, 0x16, 30000, 1);
        set_current_weapon(cop, 0x16);
    }
    let player = find_player_ped(-1);
    let t = task_new(0x38);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        kill_ped_on_foot_ctor(t, player, -1, 0, 0, 0, 2)
    };
    wr::<u8>(t, 0xC, rd::<u8>(t, 0xC) | 0x30);
    let mut ev = [0u32; 8];
    let evp = GPtr::from_raw(ev.as_mut_ptr());
    event_script_command_ctor(evp.cast::<u8>(), 3, t, 0);
    event_group_add(rd::<GPtr<u8>>(cop, 0x47C).byte_add(0x68), evp.cast::<u8>(), 0);
    intel_process(rd(cop, 0x47C));
    event_script_command_dtor(evp);
}

/// `CPopulation::PlaceCouple(ePedType, int32 husbandModel, ePedType, int32 wifeModel, CVector pos)` (0x613D60,
/// SEH frame): only for the types 4 and 5; then the same code as the couple branch of `AddToPopulation` (the bytes
/// match instruction for instruction: riots, visibility / distance, clear position, ground, the two `AddPed(4 /
/// 5)`, walk speeds, the `CTaskComplexBeInCouple` pair, the wife's offset position, removal on a missing ground or
/// a third entity, clump alphas).
#[hook(addr = 0x613D60, conv = cdecl, name = "CPopulation::PlaceCouple", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "popw4-pop-1", route = "split", hazards = "heap-writes,x87-transcendental",
       note = "script command; creates two peds and their tasks; fsqrt distance")]
pub fn place_couple(t1: i32, m1: i32, t2: i32, m2: i32, px: f32, py: f32, pz: f32) {
    if t1 == 4 && t2 == 5 {
        add_couple(m1, m2, [px, py, pz]);
    }
}

use sa_core::bindings::entity_register_reference as register_reference;
use sa_world::bindings::world_remove;
use sa_world::bindings::entity_get_col_model;
use sa_world::bindings::camera_is_sphere_visible_m;
use sa_world::bindings::die_ctor as task_die_ctor;
use sa_world::bindings::process_col_models;
orig! {
    /// virtual slot 0 of a `CEntity`: the deleting destructor
    fn entity_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

/// `CPopulation::AddDeadPedInFrontOfCar(const CVector& pos, CVehicle* vehicle)` (0x612CD0, SEH frame, EAX), from the
/// bytes: when `pos` (radius 2) is visible from the camera (matrix 0xB6FA74, or the mirror matrix 0xB6FABC when
/// byte 0xB6F998 is set) and closer to the player ped than `PDM * 42.5` (2D, FSQRT): NULL. Ground z + 1 at `pos.z +
/// 1` (NULL without ground), raised to at least `pos.z` (`fcom; test ah, 5; jp`); NULL when model 7 is not loaded.
/// `AddPed(4, 7, &(pos.x, pos.y, z), false)` (not null-checked), a `CTaskComplexDie(0, 0, 0xF, 4.0, 0, ...)` given
/// through `CEventScriptCommand(4)`, ped flags +0x470 |= 0x400000, +0x756 = 0, +0x590 = vehicle (registered
/// reference). `IsPositionClearForPed(pos, 2, 3, ents)`: when false, any entity other than the vehicle and the ped
/// removes the ped (NULL). Then a collision of the vehicle's and the ped's col models (32 points on the stack)
/// deletes the ped (`CWorld::Remove`, deleting destructor; NULL); else its clump alpha 0 and the ped.
#[hook(addr = 0x612CD0, conv = cdecl, name = "CPopulation::AddDeadPedInFrontOfCar", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "popw4-pop-1", route = "split", hazards = "heap-writes,x87-transcendental",
       note = "car-crash accidents (CAccidentManager); creates a dead ped; fsqrt distance")]
pub fn add_dead_ped_in_front_of_car(pos: GPtr<f32>, vehicle: GPtr<u8>) -> GPtr<u8> {
    let cam = g::<u8>(THE_CAMERA);
    let visible = camera_is_sphere_visible_m(cam, pos.cast::<u8>(), 2.0, g::<u8>(0xB6FA74)) != 0
        || (g::<u8>(0xB6F998).read() != 0
            && camera_is_sphere_visible_m(cam, pos.cast::<u8>(), 2.0, g::<u8>(0xB6FABC)) != 0);
    if visible {
        let pp = entity_pos(find_player_ped(-1));
        let d = dist2d(
            pos.read(),
            pos.wrapping_index(1).read(),
            pp.read(),
            pp.wrapping_index(1).read(),
        );
        if !jne_mask(pdm().fmul_m(42.5), x(d), 0x41) {
            return GPtr::NULL;
        }
    }
    let (px, py, pz) = (
        pos.read(),
        pos.wrapping_index(1).read(),
        pos.wrapping_index(2).read(),
    );
    let mut found = [0u8];
    let gz = X87::from_f64_exact(find_ground_z_for_3d_coord(
        px,
        py,
        x(pz).fadd_m(1.0).to_f32(),
        GPtr::from_raw(found.as_mut_ptr()),
        GPtr::NULL,
    ))
    .fadd_m(1.0);
    if found[0] == 0 {
        return GPtr::NULL;
    }
    let z = if jp5(gz, x(pz)) { gz } else { x(pz) };
    if rd::<u32>(model_info(7), 0x1C) == 0 {
        return GPtr::NULL;
    }
    let mut at = [px, x(py).to_f32(), z.to_f32()];
    let ped = add_ped(4, 7, GPtr::from_raw(at.as_mut_ptr()), 0);
    let t = task_new(0x28);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        task_die_ctor(t, 0, 0, 0xF, 4.0, 0.0, 0, 0, 0, 0)
    };
    let mut ev = [0u32; 8];
    let evp = GPtr::from_raw(ev.as_mut_ptr());
    event_script_command_ctor(evp.cast::<u8>(), 4, t, 0);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp.cast::<u8>(), 0);
    wr::<u16>(ped, 0x756, 0);
    wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x40_0000);
    wr::<GPtr<u8>>(ped, 0x590, vehicle);
    register_reference(vehicle, ped.byte_add(0x590));
    let mut ents = [0u32; 3];
    if is_position_clear_for_ped(pos.cast::<u8>(), 2.0, 3, GPtr::<u32>::cast::<u8>(GPtr::from_raw(ents.as_mut_ptr())), 1, 1, 1) == 0 {
        for &e in &ents {
            if e != 0 && e != vehicle.addr() as u32 && e != ped.addr() as u32 {
                remove_ped(ped);
                event_script_command_dtor(evp);
                return GPtr::NULL;
            }
        }
    }
    let ped_col = rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(ped, 0x22))), 0x14);
    let ped_m = rd::<GPtr<u8>>(ped, 0x14);
    let veh_m = rd::<GPtr<u8>>(vehicle, 0x14);
    let veh_col = entity_get_col_model(vehicle);
    let mut pts = [0u32; 0x580 / 4];
    if process_col_models(
        veh_m,
        veh_col,
        ped_m,
        ped_col,
        GPtr::<u32>::cast::<u8>(GPtr::from_raw(pts.as_mut_ptr())),
        GPtr::NULL,
        GPtr::NULL,
        0,
    ) != 0
    {
        world_remove(ped);
        entity_delete(ped, 1);
        event_script_command_dtor(evp);
        return GPtr::NULL;
    }
    set_clump_alpha(rd(ped, 0x18), 0);
    event_script_command_dtor(evp);
    ped
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[
    HOOK_ADD_TO_POPULATION,
    HOOK_POPULATE_INTERIOR,
    HOOK_CREATE_WAITING_COPPERS,
    HOOK_PLACE_COUPLE,
    HOOK_ADD_DEAD_PED_IN_FRONT_OF_CAR,
];
