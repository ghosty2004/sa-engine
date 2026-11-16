//! FRONTIER-2 turn 9, lane A (batch `frontier2-a12`): the last AI frontier virtuals and their callees, ported from
//! the 1.0 US bytes (gta-reversed read as an aid; where it differs the bytes win, noted per function).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, new_task, rd, task_create_next_sub_task, task_get_type, task_make_abortable, wr, x, xi, X87};
use crate::bindings::{
    fight_ctor, fight_dtor_orig, gang_hassle_ped_ctor, ped_is_playing_hand_signal, play_hand_signal_anim_ctor_61b2b0,
    should_load_gang_anims, tm_get_task_secondary, tm_set_task_secondary,
};
use crate::cover_update::speed as magnitude;
use crate::fighthelp::{fight_hit_car, fight_set_up_col};
use sa_anim::bindings::{anim_add_block_ref, anim_set_delete_callback, assoc_set_finish_callback};
use crate::bindings::task_timer_is_out_of_time;
use sa_core::bindings::{
    general_get_radian_angle_between_points, general_limit_radian_angle, matrix_mul_vec, matrix_set_rotate_z_only,
};
use sa_world::bindings::{anim_blend_animation, get_peds_interior_group, ped_set_move_state};
use sa_anim::bindings::ik_abort_look_at;
use crate::frontier_a_t5::interior_group_find_closest_info;
use sa_core::bindings::{clump_get_association, general_random_int_in_range, matrix_dtor};
use sa_stream::bindings::{anim_remove_block_ref, entity_is_visible, get_animation_block_index, streaming_request_model};
use sa_world::bindings::{
    entity_get_col_model, ik_is_looking, ik_look_at, kill_ped_on_foot_ctor, matrix_copy_ctor, placeable_get_heading,
    process_col_models, run_anim_ctor, shake_fist_ctor,
};
use sa_core::bindings::vector_normalise;
use sa_core::crt::rand_at;
use sa_math::fpu::{ftol32, ge_or_nan, gt, le_or_nan};
use sa_math::vector::CVector;
use sa_pop::bindings::{automobile_remove_bonnet_in_ped_collision, localisation_knock_down_peds};
use sa_world::bindings::{
    audio_add_event, damage_calc_compute_damage_response, damage_calc_ctor, damage_calc_dtor, event_damage_affects_ped,
    event_damage_ctor, event_damage_dtor, event_group_add, find_player_vehicle, get_event_global_group,
    pad_start_shake, physical_apply_move_force_5429f0, physical_apply_turn_force,
};
use sa_input::bindings::get_pad;
use crate::bindings::compute_ped_hit_side;
use sa_world::bindings::{ped_placement_find_z_coor_for_ped, ped_say, placeable_set_posn};
use sa_world::world::common::entity_pos;
use sa_world::world::hand_object::model_info;

/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS: usize = 0xB7CB84;

orig! {
    /// `CEventVehicleHitAndRun::CEventVehicleHitAndRun(CPed*, CVehicle*)` (0x4AE990, `ret 8`)
    fn hit_and_run_ctor(e: u32, ped: u32, car: u32) -> u32 = thiscall 0x4AE990 => crate::gen_ctors::g_event_vehicle_hit_and_run_constructor_4ae990;
    /// `CEventVehicleHitAndRun::~CEventVehicleHitAndRun()` (0x4AEA50)
    fn hit_and_run_dtor(e: GPtr<u8>) = thiscall 0x4AEA50 => crate::event_dtors::event_vehicle_hit_and_run_dtor;
    /// `CRopes::FindCoorsAlongRope(uint32 id, float along, CVector* pos, CVector* speed)` (0x555E40, sa-render)
    fn ropes_find_coors_along_rope(id: u32, along: f32, pos: GPtr<f32>, speed: GPtr<f32>) -> u8 = cdecl 0x555E40 => unported;
    /// `InteriorManager_c::AreAnimsLoaded(int32 block)` (0x5980F0, static, `ret 4`)
    fn are_anims_loaded(block: i32) -> u8 = stdcall 0x5980F0 => sa_world::world::interior_wall_fillers::are_anims_loaded;
    /// `CTaskGangHassleVehicle::GetTargetHeading(CPed*)` (0x65FDD0, `ret 4`, ST0)
    fn target_heading(t: GPtr<u8>, ped: GPtr<u8>) -> X87 = reg("args = ecx stack; pop") 0x65FDD0 => crate::gang6::gang_hassle_vehicle_get_target_heading;
    /// `CTaskComplexUseSwatRope::CreateSubTask(int32 type, CPed*)` (0x659620, phase B native)
    fn use_swat_rope_create_sub_task(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x659620 => crate::frontier_a_subtasks::use_swat_rope_create_sub_task;
}

/// `(a.z * b.z + a.y * b.y) + a.x * b.x` with `a` as three f32 and `b` read at `bp`.
fn dot_zyx(a: [f32; 3], bp: GPtr<u8>) -> X87 {
    x(a[2]).fmul_m(rd::<f32>(bp, 8)).fadd(x(a[1]).fmul_m(rd::<f32>(bp, 4))).fadd(x(a[0]).fmul_m(rd::<f32>(bp, 0)))
}

// ------------------------------------------------------------------------------------------- kill ped with car

/// `CTaskSimpleKillPedWithCar::DamageCarBonnet(const CPed*)` (0x653CF0, `ret 4`). Only for an automobile (car +0x594
/// = 0) whose bonnet `RemoveBonnetInPedCollision` throws off: `rand() * 0x858C7C > 0.5` picks the side. The bonnet's
/// speed is the **ped's** speed (gta-reversed adds to the bonnet's own) `± right * 0.1 + up * 0.5`, with the original's
/// f32 stack slots (the right x / z sums and up.z rounded, the y sum not); then `ApplyTurnForce(up * 10, forward)`.
#[hook(addr = 0x653CF0, conv = thiscall, name = "CTaskSimpleKillPedWithCar::DamageCarBonnet", expect = "83 EC 34 56 57 8B F9",
       unverified = "frontier2-a12", route = "L2", hazards = "heap-writes",
       note = "the bonnet object comes from SpawnFlyingComponent (pool, world lists)")]
pub fn kill_ped_with_car_damage_car_bonnet(this: GPtr<u8>, ped: GPtr<u8>) {
    let car = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(car, 0x594) != 0 {
        return;
    }
    let obj = automobile_remove_bonnet_in_ped_collision(car);
    if obj.is_null() {
        return;
    }
    let plus = gt(xi(rand_at(0x653D17)).fmul_m(g::<f32>(0x858C7C).read()), x(g::<f32>(0x858B8C).read()));
    let m = rd::<GPtr<u8>>(car, 0x14);
    let half = g::<f32>(0x858B8C).read();
    let tenth = g::<f32>(0x858B1C).read();
    let (ux, uy) = (x(rd::<f32>(m, 0x20)).fmul_m(half), x(rd::<f32>(m, 0x24)).fmul_m(half));
    let uz = x(rd::<f32>(m, 0x28)).fmul_m(half).to_f32();
    let (rx, ry) = (x(rd::<f32>(m, 0)).fmul_m(tenth), x(rd::<f32>(m, 4)).fmul_m(tenth));
    let rz = x(rd::<f32>(m, 8)).fmul_m(tenth).to_f32();
    let (psx, psy, psz) = (rd::<f32>(ped, 0x44), rd::<f32>(ped, 0x48), rd::<f32>(ped, 0x4C));
    let (sx, sy, sz) = if plus {
        (rx.fadd_m(psx).to_f32(), ry.fadd_m(psy), x(rz).fadd_m(psz).to_f32())
    } else {
        (x(psx).fsub(rx).to_f32(), x(psy).fsub(ry), x(psz).fsub_m(rz).to_f32())
    };
    wr::<f32>(obj, 0x44, x(sx).fadd(ux).to_f32());
    wr::<f32>(obj, 0x48, sy.fadd(uy).to_f32());
    wr::<f32>(obj, 0x4C, x(sz).fadd_m(uz).to_f32());
    let m = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x14);
    let ten = g::<f32>(0x85862C).read();
    let up = |o| x(rd::<f32>(m, o)).fmul_m(ten).to_f32();
    let force = CVector { x: up(0x20), y: up(0x24), z: up(0x28) };
    let fwd = CVector { x: rd::<f32>(m, 0x10), y: rd::<f32>(m, 0x14), z: rd::<f32>(m, 0x18) };
    physical_apply_turn_force(obj, force, fwd);
}

/// `FUN_006563d0` (0x6563D0, thiscall, no arguments; KillPedWithCar's pad shake). Only when the task's car (+8) is
/// `FindPlayerVehicle(-1, 0)`: `v = |speed| * f32(1 / mass) * 200000 + 80`, capped at 250 (`test ah, 41h`: a NaN
/// keeps the sum), `f = (u8)ftol(v)`; `GetPad(0)->StartShake(40000 / f, f, 0)`. The frequency dword's upper bytes
/// are the f32 `1 / mass` stack slot it was stored over (the callee reads the low byte); `f = 0` divides by zero
/// as the original does.
#[hook(addr = 0x6563D0, conv = thiscall, name = "FUN_006563d0", expect = "51 56 8B 71 08 6A 00 6A FF",
       unverified = "frontier2-a12", route = "L2", hazards = "heap-writes")]
pub fn kill_ped_with_car_shake_pad(this: GPtr<u8>) {
    let car = rd::<GPtr<u8>>(this, 8);
    if find_player_vehicle(-1, 0) != car {
        return;
    }
    let inv = x(g::<f32>(0x858624).read()).fdiv_m(rd::<f32>(car, 0x8C)).to_f32();
    let sum = || magnitude(car.byte_add(0x44)).fmul_m(inv).fmul_m(g::<f32>(0x86C688).read()).fadd_m(g::<f32>(0x858A14).read());
    let cap = g::<f32>(0x859F80).read();
    let v = if le_or_nan(sum(), x(cap)) { sum() } else { x(cap) };
    let f = ftol32(v) as u8;
    let freq = (inv.to_bits() & 0xFFFF_FF00) | u32::from(f);
    let time = 40000 / i32::from(f);
    pad_start_shake(get_pad(0), time as u32, freq, 0);
}

/// `CTaskSimpleKillPedWithCar::ProcessPed(CPed*)` (0x6564A0, `ret 4`, AL = 1). `d = ped - car` (f32), its components
/// along the car's right / forward / up rows, `forward · speed` and the car's (model info) bounding box; the hit side
/// `ComputePedHitSide(ped, car)`. Three cases, by the bytes (gta-reversed differs in the marked points):
/// * `|d·right| > max.x * 0.99` (a side hit, direction 1 when `d·right <= 0`, else 3): when `|d·forward| <
///   |0.85 * (forward · speed > 0 ? max.y : min.y)|` (gta-reversed: `>= 0`) the ped is run over: weapon 50, the ped's
///   speed `0.9 * car speed` with z = 0, the crunch audio (0x77) when peds may be knocked down, hit side 1 / 3 → 0.
/// * `d·up > 0.1` (onto the bonnet, direction 4): the ped's speed points along the car's speed with the size
///   `(height - ped z) / (span / |speed|)`; the height uses max.z (gta-reversed: max.x for a lifted back) and the span
///   for `forward.z > 0.1` is `max.y` when moving forward, else `max.y - min.y` (gta-reversed tests `max.y <= 0`);
///   then `DamageCarBonnet`.
/// * else (direction 5, gta-reversed: 4): run over as the side case, with the hit side reset before the audio.
///
/// Then a `CEventVehicleHitAndRun` to the global group, a `CEventDamage(car, now, weapon, 9, hit side, 0, in-vehicle
/// dword)` with the piece by direction (1 → 5, 3 → 6, 4 → 4, 5 → 9) and its 1000-damage response, added to the ped;
/// the pad shake 0x6563D0; the ped's standing flag cleared, +0x470 bit 4 = direction 4; the car pushed with
/// `normalise(d) * min(1, mass * 0x864F98) * -100`.
#[hook(addr = 0x6564A0, conv = thiscall, name = "CTaskSimpleKillPedWithCar::ProcessPed", expect = "6A FF 64 A1 00 00 00 00 68 08 3A 84 00",
       unverified = "frontier2-a12", route = "L2", hazards = "heap-writes,ax-al-return",
       note = "the three events live on the Rust stack; SEH frame not modelled; EAX's upper bytes come from the last dtor")]
pub fn kill_ped_with_car_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let car = rd::<GPtr<u8>>(this, 8);
    let pp = entity_pos(ped);
    let cp = entity_pos(car);
    let d: [f32; 3] = core::array::from_fn(|i| x(rd::<f32>(pp, 4 * i as isize)).fsub_m(rd::<f32>(cp, 4 * i as isize)).to_f32());
    let m = rd::<GPtr<u8>>(car, 0x14);
    let lx = dot_zyx(d, m).to_f32();
    let ly = dot_zyx(d, m.byte_add(0x10)).to_f32();
    let lz = dot_zyx(d, m.byte_add(0x20)).to_f32();
    let fwd: [f32; 3] = core::array::from_fn(|i| rd::<f32>(m, 0x10 + 4 * i as isize));
    let fdot = dot_zyx(fwd, car.byte_add(0x44)).to_f32();
    let col = rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(car, 0x22))), 0x14);
    let (min_y, max_x, max_y, max_z) = (rd::<f32>(col, 4), rd::<f32>(col, 0xC), rd::<f32>(col, 0x10), rd::<f32>(col, 0x14));
    let mut hit = compute_ped_hit_side(ped, car);
    let zero = x(g::<f32>(0x858B50).read());
    let tenth = g::<f32>(0x858B1C).read();
    let moving_fwd = gt(x(fdot), zero);
    let run_over = |ped: GPtr<u8>| {
        let k = g::<f32>(0x858C20).read();
        for o in [0x44, 0x48] {
            wr::<f32>(ped, o, x(rd::<f32>(car, o)).fmul_m(k).to_f32());
        }
        wr::<u32>(ped, 0x4C, 0);
    };
    let crunch = || {
        if localisation_knock_down_peds() != 0 {
            audio_add_event(rd::<GPtr<u8>>(this, 8).byte_add(0x138), 0x77, 0.0);
        }
    };
    let mut weapon = 0x31;
    let dir: u32;
    if !ge_or_nan(x(max_x).fmul_m(g::<f32>(0x862CD0).read()), x(lx).fabs()) {
        dir = if gt(x(lx), zero) { 3 } else { 1 };
        let lim = x(if moving_fwd { max_y } else { min_y }).fmul_m(g::<f32>(0x862E1C).read()).fabs();
        if !ge_or_nan(x(ly).fabs(), lim) {
            weapon = 0x32;
            run_over(ped);
            crunch();
            if hit == 1 || hit == 3 {
                hit = 0;
            }
        }
    } else if !le_or_nan(x(lz), x(tenth)) {
        dir = 4;
        let fz = rd::<f32>(m, 0x18);
        let up_z = rd::<f32>(m, 0x28);
        let pos_z = rd::<f32>(cp, 8);
        let (height, span) = if !ge_or_nan(x(fz), x(g::<f32>(0x858FDC).read())) {
            let h = x(min_y).fmul_m(fz).fadd(x(max_z).fmul_m(up_z)).fadd_m(pos_z);
            (h, if moving_fwd { x(max_y).fsub_m(min_y) } else { x(min_y).fchs() })
        } else if !le_or_nan(x(fz), x(tenth)) {
            let h = x(max_y).fmul_m(fz).fadd(x(max_z).fmul_m(up_z)).fadd_m(pos_z);
            (h, if moving_fwd { x(max_y) } else { x(max_y).fsub_m(min_y) })
        } else {
            let h = x(max_z).fmul_m(up_z).fadd_m(pos_z);
            (h, if moving_fwd { x(max_y) } else { x(min_y).fchs() })
        };
        let t = height.fsub_m(rd::<f32>(entity_pos(ped), 8)).fdiv(span.fdiv(magnitude(car.byte_add(0x44)))).to_f32();
        let mut n: [f32; 3] = core::array::from_fn(|i| rd::<f32>(car, 0x44 + 4 * i as isize));
        vector_normalise(GPtr::from_raw(n.as_mut_ptr()).cast());
        for (i, c) in n.iter().enumerate() {
            wr::<f32>(ped, 0x44 + 4 * i as isize, x(*c).fmul_m(t).to_f32());
        }
        kill_ped_with_car_damage_car_bonnet(this, ped);
    } else {
        dir = 5;
        weapon = 0x32;
        run_over(ped);
        if hit == 1 || hit == 3 {
            hit = 0;
        }
        crunch();
    }
    let mut hr = [0u32; 5];
    let hrp: GPtr<u8> = GPtr::from_raw(hr.as_mut_ptr()).cast();
    hit_and_run_ctor(hrp.addr() as u32, ped.addr() as u32, rd::<u32>(this, 8));
    event_group_add(get_event_global_group(), hrp, 0);
    let mut calc = [0u32; 5];
    let cp_: GPtr<u8> = GPtr::from_raw(calc.as_mut_ptr()).cast();
    damage_calc_ctor(cp_, rd::<u32>(this, 8), g::<u32>(0x86F130).read(), weapon, 9, 0);
    let mut ev = [0u32; 0x11];
    let ep: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    let in_veh = (rd::<u32>(ped, 0x46C) >> 8) & 0xFFFF_FF01;
    event_damage_ctor(ep, rd::<GPtr<u8>>(this, 8), g::<u32>(TIME_MS).read(), weapon, 9, hit, 0, in_veh);
    match dir {
        1 => wr::<u32>(ep, 0x20, 5),
        3 => wr::<u32>(ep, 0x20, 6),
        4 => wr::<u32>(ep, 0x20, 4),
        5 => wr::<u32>(ep, 0x20, 9),
        _ => {}
    }
    if event_damage_affects_ped(ep, ped) != 0 {
        damage_calc_compute_damage_response(cp_, ped, ep.byte_add(0x38), 1);
    } else {
        wr::<u8>(ep, 0x42, 1);
    }
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ep, 0);
    kill_ped_with_car_shake_pad(this);
    let f470 = rd::<u32>(ped, 0x470);
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & !1);
    wr::<u32>(ped, 0x470, if dir == 4 { f470 | 0x10 } else { f470 & !0x10 });
    let mut v = d;
    vector_normalise(GPtr::from_raw(v.as_mut_ptr()).cast());
    let car = rd::<GPtr<u8>>(this, 8);
    let one = g::<f32>(0x858624).read();
    let f = x(rd::<f32>(car, 0x8C)).fmul_m(g::<f32>(0x864F98).read());
    let s = if ge_or_nan(x(one), f) { f } else { x(one) }.fmul_m(g::<f32>(0x859014).read());
    let force = CVector { x: x(v[0]).fmul(s).to_f32(), y: x(v[1]).fmul(s).to_f32(), z: x(v[2]).fmul(s).to_f32() };
    physical_apply_move_force_5429f0(car, force);
    event_damage_dtor(ep);
    damage_calc_dtor(cp_);
    hit_and_run_dtor(hrp);
    1
}

// ------------------------------------------------------------------------------------------- swat rope

/// `CTaskComplexUseSwatRope::ControlSubTask(CPed*)` (0x65A460, `ret 4`). On a heli (+0x14 bit 0) that is gone (+0x18
/// null), crashing (mission +0x3BA = 0x3A) or at health (+0x4C0) `< 0` / `= 0` (a NaN keeps going): when the sub-task
/// aborts (`MakeAbortable(ped, 1, NULL)`) the ped's +0x1C bit 0 is set, its standing flag cleared, its speed zeroed,
/// NULL. A pause (0xCA) or abseil (0x1A7) sub-task: the ped's ground z (`FindZCoorForPed` on its position); with
/// `z - 2 < ground` the sub-task is aborted into `CreateSubTask(0xC8)`; else +0x10 += `timeStep * 0.003` and the
/// rope point (`FindCoorsAlongRope(+0xC, +0x10, &p, NULL)`) becomes the ped's position, +0x55C = +0x558 - `timeStep
/// * 0.05`, z speed -0.03, `Say(0xB1)`; when the rope is not found the abort path runs (gta-reversed keeps the
/// sub-task). Otherwise the sub-task.
#[hook(addr = 0x65A460, conv = thiscall, name = "CTaskComplexUseSwatRope::ControlSubTask", expect = "83 EC 18 56 8B 74 24 20 57 8B",
       unverified = "frontier2-a12", route = "L2", hazards = "heap-writes")]
pub fn use_swat_rope_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if rd::<u8>(this, 0x14) & 1 != 0 {
        let heli = rd::<GPtr<u8>>(this, 0x18);
        let gone = heli.is_null()
            || rd::<u8>(heli, 0x3BA) == 0x3A
            || !x(rd::<f32>(heli, 0x4C0)).fcom(x(g::<f32>(0x858B50).read())).test_parity(0x41);
        if gone && task_make_abortable(sub(), ped, 1, GPtr::NULL) != 0 {
            wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) | 1);
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & !1);
            for o in [0x44, 0x48, 0x4C] {
                wr::<u32>(ped, o, 0);
            }
            return GPtr::NULL;
        }
    }
    let s = sub();
    let ty = task_get_type(s);
    if ty != 0xCA && ty != 0x1A7 {
        return s;
    }
    let mut ground: [f32; 3] = core::array::from_fn(|i| rd::<f32>(entity_pos(ped), 4 * i as isize));
    ped_placement_find_z_coor_for_ped(GPtr::from_raw(ground.as_mut_ptr()));
    let low = !x(rd::<f32>(entity_pos(ped), 8)).fsub_m(g::<f32>(0x86F44C).read()).fcom(x(ground[2])).test_parity(5);
    if !low {
        let ts = g::<f32>(0xB7CB5C).read();
        let t = x(ts).fmul_m(g::<f32>(0x86F450).read()).fadd_m(rd::<f32>(this, 0x10)).to_f32();
        wr::<f32>(this, 0x10, t);
        let mut p = [0f32; 3];
        if ropes_find_coors_along_rope(rd::<u32>(this, 0xC), t, GPtr::from_raw(p.as_mut_ptr()), GPtr::NULL) != 0 {
            placeable_set_posn(ped, GPtr::from_raw(p.as_mut_ptr()));
            let ts = g::<f32>(0xB7CB5C).read();
            wr::<f32>(ped, 0x55C, x(ts).fmul_m(g::<f32>(0x86F458).read()).fadd_m(rd::<f32>(ped, 0x558)).to_f32());
            wr::<u32>(ped, 0x4C, g::<u32>(0x86F454).read());
            ped_say(ped, 0xB1, 0, 1.0, 0, 0, 0);
            return s;
        }
    }
    if task_make_abortable(sub(), ped, 1, GPtr::NULL) != 0 {
        return use_swat_rope_create_sub_task(this, 0xC8, ped);
    }
    s
}

// ------------------------------------------------------------------------------------------- gang hassle vehicle

/// `+0x1C²` (plus `add`) against the range (`fld o; fld st0; fmul st1; [fadd add]; fld range; fcompp; test ah, 41h`):
/// the range is above it (ordered).
fn range_above(this: GPtr<u8>, range: f32, add: Option<f32>) -> bool {
    let o = x(rd::<f32>(this, 0x1C));
    let o2 = o.fmul(o);
    let o2 = match add {
        Some(a) => o2.fadd_m(a),
        None => o2,
    };
    !x(range).fcom(o2).test_any(0x41)
}

/// `range` against `+0x1C²` with `test ah, 41h; jp` not taken (0x663B69): exactly one of C0 / C3 set, `range <= o²`.
fn range_at_most(this: GPtr<u8>, range: f32) -> bool {
    let o = x(rd::<f32>(this, 0x1C));
    !x(range).fcom(o.fmul(o)).test_parity(0x41)
}

/// `CTaskGangHassleVehicle::ControlSubTask(CPed*)` (0x6637C0, `ret 4`; gta-reversed only calls the original). The
/// gang anim block ref (+0x30) follows `ShouldLoadGangAnims` (requested until loaded); no vehicle (+0x10): the
/// sub-task (not a 0x3E8) is aborted into NULL. A timer (+0x40, restarted by +0x41; +0x38 start, +0x3C length) that
/// ran out with +0x18: a `CTaskComplexKillPedOnFoot(+0x34, -1, 0, 0, 0, 1)` unless the sub-task already is 0x3E8.
/// A driverless vehicle with a target (+0x34): the sub-task (not 0x4BC) aborted into `CTaskGangHasslePed(+0x34,
/// +0x18 ? 2 : 1, 12000, 20000)`, +0x31 = 3. The range is 100, or the 0x39D sub-task's +0x50; a visible ped under
/// range 4 not already looking may look at the driver (60 %, 1000..3000 ms). For a 0x39D / 0x38A sub-task: vehicle
/// health under 250 → abort into `CreateNextSubTask`; then the state +0x31: 0 → maybe a shake-fist secondary task,
/// state 1 once in range (`range <= +0x1C²`), `Say(0x101)`; 1 → the ped turns to `GetTargetHeading`, back to 0 out
/// of range (0x39D only), 2 when within 0.05; 2 → 0 out of range + 0.05, 1 when off the heading by 0.1 or more, a
/// random line (0xFE / 0xFF / 0x100), then with the anims loaded and the ped state below 6: no secondary task →
/// a random 0..200 picks a gang anim, a hand signal or state 0 (0x663CD9); a playing gang anim at its hit time →
/// the car is hit (0x663ECB). Returns the sub-task (+8).
#[hook(addr = 0x6637C0, conv = thiscall, name = "CTaskGangHassleVehicle::ControlSubTask", expect = "6A FF 64 A1 00 00 00 00 68 8E 4B 84 00",
       unverified = "frontier2-a12", route = "L2", hazards = "heap-writes",
       note = "the fight task and matrix live on the Rust stack; SEH frame not modelled; FightHitCar's int16 piece argument has stale upper bytes in the original (movzx cx)")]
pub fn gang_hassle_vehicle_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if !sub().is_null() && task_get_type(sub()) != 0x3E8 && rd::<u32>(this, 0x10) == 0 {
        return if task_make_abortable(sub(), ped, 1, GPtr::NULL) != 0 { GPtr::NULL } else { sub() };
    }
    let gangs = || get_animation_block_index(GPtr::from_addr(0x86F92C));
    if rd::<u8>(this, 0x30) == 0 {
        if should_load_gang_anims() != 0 {
            let i = gangs();
            if g::<u8>(0xB5D4A0usize.wrapping_add((i as u32).wrapping_shl(5) as usize) + 0x10).read() == 1 {
                anim_add_block_ref(i);
                wr::<u8>(this, 0x30, 1);
            } else {
                streaming_request_model(i.wrapping_add(0x63E7), 8);
            }
        }
    } else if should_load_gang_anims() == 0 {
        anim_remove_block_ref(gangs() as u32);
        wr::<u8>(this, 0x30, 0);
    }
    let now = || g::<u32>(TIME_MS).read();
    if rd::<u8>(this, 0x40) != 0 {
        if rd::<u8>(this, 0x41) != 0 {
            wr::<u32>(this, 0x38, now());
            wr::<u8>(this, 0x41, 0);
        }
        let out = now() >= rd::<u32>(this, 0x3C).wrapping_add(rd::<u32>(this, 0x38));
        if out && rd::<u8>(this, 0x18) != 0 {
            if task_get_type(sub()) == 0x3E8 {
                return sub();
            }
            let target = rd::<GPtr<u8>>(this, 0x34);
            return new_task(0x38, |m| kill_ped_on_foot_ctor(m, target, -1, 0, 0, 0, 1));
        }
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    if !veh.is_null()
        && rd::<u32>(veh, 0x460) == 0
        && rd::<u32>(this, 0x34) != 0
        && task_get_type(sub()) != 0x4BC
        && task_make_abortable(sub(), ped, 1, GPtr::NULL) != 0
    {
        wr::<u8>(this, 0x31, 3);
        let kind = if rd::<u8>(this, 0x18) != 0 { 2 } else { 1 };
        let target = rd::<GPtr<u8>>(this, 0x34);
        return new_task(0x2C, |m| gang_hassle_ped_ctor(m, target, kind, 12000, 20000));
    }
    let mut range = f32::from_bits(0x42C8_0000);
    if task_get_type(sub()) == 0x39D {
        range = rd::<f32>(sub(), 0x50);
    }
    if entity_is_visible(ped) != 0
        && !x(range).fcom(x(g::<f32>(0x858B90).read())).test_parity(5)
        && ik_is_looking(GPtr::from_addr(0xC15448), ped) == 0
        && general_random_int_in_range(0, 100) > 60
    {
        let t = xi(general_random_int_in_range(1000, 3000));
        let driver = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x460);
        if !driver.is_null() {
            ik_look_at(GPtr::from_addr(0xC15448), GPtr::from_addr(0x86FC14), ped, driver, ftol32(t), 5, GPtr::NULL, 1,
                       f32::from_bits(0x3E19_999A), 0x1F4, 3, 0);
        }
    }
    if sub().is_null() {
        return sub();
    }
    if task_get_type(sub()) != 0x39D && task_get_type(sub()) != 0x38A {
        return sub();
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    if !x(rd::<f32>(veh, 0x4C0)).fcom(x(g::<f32>(0x859F80).read())).test_parity(5)
        && task_make_abortable(sub(), ped, 1, GPtr::NULL) != 0
    {
        return task_create_next_sub_task(this, ped);
    }
    let say = |id: u32| {
        ped_say(ped, id, 0, 1.0, 0, 0, 0);
    };
    let tm = || rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
    match rd::<u8>(this, 0x31) {
        0 => {
            if tm_get_task_secondary(tm(), 4).is_null() && general_random_int_in_range(0, 100) > 60 {
                let t = new_task(0x10, shake_fist_ctor);
                tm_set_task_secondary(tm(), t, 4);
            }
            if range_at_most(this, range) {
                wr::<u8>(this, 0x31, 1);
            }
            say(0x101);
        }
        1 => {
            let h = target_heading(this, ped).to_f32();
            wr::<f32>(ped, 0x55C, h);
            let hd = X87::from_f64_exact(placeable_get_heading(ped)).to_f32();
            if task_get_type(sub()) != 0x39D {
                return sub();
            }
            if range_above(this, range, None) {
                wr::<u8>(this, 0x31, 0);
            } else if !x(hd).fsub_m(h).fabs().fcom(x(g::<f32>(0x858C28).read())).test_parity(5) {
                wr::<u8>(this, 0x31, 2);
            }
        }
        2 => {
            if task_get_type(sub()) == 0x39D && range_above(this, range, Some(g::<f32>(0x858C28).read())) {
                wr::<u8>(this, 0x31, 0);
            }
            let h = target_heading(this, ped).to_f32();
            let off = X87::from_f64_exact(placeable_get_heading(ped)).fsub_m(h).fabs();
            if !off.fcom(x(g::<f32>(0x858B1C).read())).test_any(1) {
                wr::<u8>(this, 0x31, 1);
            }
            match general_random_int_in_range(0, 3) {
                0 => say(0xFE),
                1 => say(0xFF),
                2 => say(0x100),
                _ => {}
            }
            if rd::<u8>(this, 0x30) == 0 || rd::<i32>(ped, 0x534) >= 6 {
                return sub();
            }
            if tm_get_task_secondary(tm(), 4).is_null() {
                gang_hassle_pick_anim(this, ped);
            } else {
                gang_hassle_hit_car(this, ped, range);
            }
        }
        _ => {}
    }
    sub()
}

/// 0x663CD9: no secondary task: `r = (float)GetRandomNumberInRange(0, 200)` picks the secondary task (each
/// threshold a `test ah, 41h` "above"): > 166 / 133 / 100 the gang anims 0x128 / 0x129 / 0x12A, > 70 anim `0x117 +
/// ftol((rand() & 0xFFFF) * 0x858B14 * 8)` (rand after `new`), > 60 a hand signal (none while one plays), > 40
/// state 0, else nothing. The task (or NULL when `new` fails) becomes secondary task 4.
fn gang_hassle_pick_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let r = x(xi(general_random_int_in_range(0, 200)).to_f32());
    let above = |c: usize| !r.fcom(x(g::<f32>(c).read())).test_any(0x41);
    let run = |anim: u32| new_task(0x20, |m| run_anim_ctor(m, 0x34, anim, 4.0, 0));
    let t = if above(0x86FC10) {
        run(0x128)
    } else if above(0x86FC0C) {
        run(0x129)
    } else if above(0x858628) {
        run(0x12A)
    } else if above(0x858CE0) {
        new_task(0x20, |m| {
            let v = xi(rand_at(0x663E04) & 0xFFFF).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x859000).read());
            run_anim_ctor(m, 0x34, (ftol32(v) as u32).wrapping_add(0x117), 4.0, 0)
        })
    } else if above(0x858B34) {
        if ped_is_playing_hand_signal(ped) != 0 {
            return;
        }
        new_task(0x18, |m| play_hand_signal_anim_ctor_61b2b0(m, u32::MAX, 4.0))
    } else {
        if above(0x858A10) {
            wr::<u8>(this, 0x31, 0);
        }
        return;
    };
    tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4);
}

/// 0x663ECB: a secondary task plays. Out of range the sub-task is asked to abort (result unused). The first of the
/// gang anims 0x128 (hit time 0.5) / 0x129 (0.7) / 0x12A (0.5) on the ped's clump, when it crosses its hit time this
/// frame (`hit < time` and `time - delta <= hit`, as the bytes' NaN handling), hits the car: `ApplyTurnForce((0, 0,
/// mass * 0.02), point)` with the point by hassle position (+0x14: 0 / 2 minus right, 1 / 3 right, 4 minus forward,
/// 5 forward, else the force itself), a stack `CTaskSimpleFight(car, 0xB, 20000)` whose sphere (radius 0.5, at the
/// ped's matrix moved by its forward vector) is tested against the car (`ProcessColModels` into 0xB9ACD0) and
/// `FightHitCar` on contact, the car's health restored after, the 0x6C audio event.
fn gang_hassle_hit_car(this: GPtr<u8>, ped: GPtr<u8>, range: f32) {
    if range_above(this, range, None) {
        task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL);
    }
    let clump = || rd::<GPtr<u8>>(ped, 0x18);
    let Some((assoc, k)) = [(0x128, 0x858B8C), (0x129, 0x858CB0), (0x12A, 0x858B8C)]
        .into_iter()
        .map(|(a, k)| (clump_get_association(clump(), a), k))
        .find(|(a, _)| !a.is_null())
    else {
        return;
    };
    let k = x(g::<f32>(k).read());
    if k.fcom(x(rd::<f32>(assoc, 0x20))).test_parity(5) {
        return;
    }
    if x(rd::<f32>(assoc, 0x20)).fsub_m(rd::<f32>(assoc, 0x28)).fcom(k).test_parity(0x41) {
        return;
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    let f = x(rd::<f32>(veh, 0x8C)).fmul_m(g::<f32>(0x858B38).read()).to_f32();
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let row = |o: isize, neg: bool| {
        let c = |i: isize| if neg { x(rd::<f32>(m, o + i)).fchs().to_f32() } else { rd::<f32>(m, o + i) };
        CVector { x: c(0), y: c(4), z: c(8) }
    };
    let point = match rd::<i32>(this, 0x14) {
        0 | 2 => row(0, true),
        1 | 3 => row(0, false),
        4 => row(0x10, true),
        5 => row(0x10, false),
        _ => CVector { x: 0.0, y: 0.0, z: f },
    };
    physical_apply_turn_force(veh, CVector { x: 0.0, y: 0.0, z: f }, point);
    let health = rd::<u32>(rd::<GPtr<u8>>(this, 0x10), 0x4C0);
    let mut fight = [0u32; 10];
    let fp: GPtr<u8> = GPtr::from_raw(fight.as_mut_ptr()).cast();
    fight_ctor(fp, rd::<GPtr<u8>>(this, 0x10), 0xB, 20000);
    let mut mat = [0u32; 18];
    let mp: GPtr<u8> = GPtr::from_raw(mat.as_mut_ptr()).cast();
    matrix_copy_ctor(mp, rd::<GPtr<u8>>(ped, 0x14));
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    for i in 0..3 {
        wr::<f32>(mp, 0x30 + 4 * i, x(rd::<f32>(mp, 0x30 + 4 * i)).fadd_m(rd::<f32>(pm, 0x10 + 4 * i)).to_f32());
    }
    fight_set_up_col(fp, 0x3F00_0000);
    wr::<u8>(fp, 0x24, 4);
    wr::<u8>(fp, 0x25, 1);
    wr::<u8>(fp, 0x27, 0xB);
    let veh = rd::<GPtr<u8>>(this, 0x10);
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let cm = entity_get_col_model(veh);
    let n = process_col_models(mp, GPtr::from_addr(0xC17824), vm, cm, GPtr::from_addr(0xB9ACD0), GPtr::NULL, GPtr::NULL, 0);
    if n > 0 {
        let piece = u32::from(g::<u8>(0xB9ACF4).read());
        let surface = u32::from(g::<u8>(0xB9ACF3).read());
        fight_hit_car(fp, ped, rd::<GPtr<u8>>(this, 0x10), 0xB9ACD0, 0xB9ACE0, piece, surface);
    }
    wr::<u32>(rd::<GPtr<u8>>(this, 0x10), 0x4C0, health);
    audio_add_event(rd::<GPtr<u8>>(this, 0x10).byte_add(0x138), 0x6C, 0.0);
    matrix_dtor(mp);
    fight_dtor_orig(fp);
}

// ------------------------------------------------------------------------------------------- interior tasks

/// The anim tasks' "only script peds" acquaintance scanner state (intelligence +0x238: +0xC = 1, +0xD / +0xE = 0;
/// gta-reversed `SetOnlyScriptPedAllowed`).
fn scanner_only_script_peds(ped: GPtr<u8>) {
    let s = rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x238);
    wr::<u8>(s, 0xC, 1);
    wr::<u8>(s, 0xD, 0);
    wr::<u8>(s, 0xE, 0);
}

/// The interior anim tasks' shared step towards their interior info (+0xC; 0x677606 in 0x6772E0, 0x67705B in
/// 0x676D30): `d = info pos (+4) - ped pos` (f32), `len = sqrt((dz * dz + dy * dy) + dx * dx)` (dz's product uses the
/// unrounded difference once), `step = len < 0.02 ? len : 0.02` (a NaN takes 0.02); the ped's anim shift (+0x4D8 /
/// +0x4DC) = the step vector `n = (dx / len * step, f32(dy / len) * step, f32(dz / len) * step)` along its matrix's
/// right and forward rows (`(r.z * n.z + r.y * n.y) + n.x * r.x`), then +0x55C = `LimitRadianAngle(f32(
/// GetRadianAngleBetweenPoints(info +0x10, info +0x14, 0, 0)))`. The original stores `len` over its own ped
/// argument slot.
fn interior_step_to_info(this: GPtr<u8>, ped: GPtr<u8>) {
    let info = rd::<GPtr<u8>>(this, 0xC);
    let pp = entity_pos(ped);
    let dx = x(rd::<f32>(info, 4)).fsub_m(rd::<f32>(pp, 0)).to_f32();
    let dy = x(rd::<f32>(info, 8)).fsub_m(rd::<f32>(pp, 4)).to_f32();
    let dz80 = x(rd::<f32>(info, 0xC)).fsub_m(rd::<f32>(pp, 8));
    let dz = dz80.to_f32();
    let len80 = dz80.fmul_m(dz).fadd(x(dy).fmul_m(dy)).fadd(x(dx).fmul_m(dx)).fsqrt();
    let len = len80.to_f32();
    let min = g::<f32>(0x858B38).read();
    let step = if len80.fcom(x(min)).test_parity(5) { x(min) } else { x(len) };
    let inv = x(g::<f32>(0x858624).read()).fdiv_m(len);
    let nx = x(dx).fmul(inv).fmul(step);
    let ny = x(x(dy).fmul(inv).to_f32()).fmul(step);
    let nz = x(x(dz).fmul(inv).to_f32()).fmul(step);
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let along = |o: isize| {
        x(rd::<f32>(m, o + 8)).fmul(nz).fadd(x(rd::<f32>(m, o + 4)).fmul(ny)).fadd(nx.fmul_m(rd::<f32>(m, o))).to_f32()
    };
    wr::<f32>(ped, 0x4D8, along(0));
    wr::<f32>(ped, 0x4DC, along(0x10));
    let info = rd::<GPtr<u8>>(this, 0xC);
    let a = general_get_radian_angle_between_points(rd::<f32>(info, 0x10), rd::<f32>(info, 0x14), 0.0, 0.0).to_f32();
    wr::<f32>(ped, 0x55C, general_limit_radian_angle(a) as f32);
}

/// `SetDeleteCallback(default)` on the task's anim (+0x14), then a new `BlendAnimation(clump, 0x54, anim, delta)`
/// as the task's anim with the finish callback `cb` (data = the task).
fn interior_blend(this: GPtr<u8>, ped: GPtr<u8>, anim: u32, delta: f32, cb: u32, drop_old: bool) {
    if drop_old {
        anim_set_delete_callback(rd::<GPtr<u8>>(this, 0x14), 0x4D1390, 0);
    }
    let a = anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), 0x54, anim, delta);
    wr::<GPtr<u8>>(this, 0x14, a);
    assoc_set_finish_callback(a, cb, this.addr() as u32);
}

/// The interior anim tasks' timer start (`+8 >= 0`: +0x1C = now, +0x20 = +8, +0x24 = 1).
fn interior_start_timer(this: GPtr<u8>) {
    let d = rd::<i32>(this, 8);
    if d >= 0 {
        wr::<u32>(this, 0x1C, g::<u32>(TIME_MS).read());
        wr::<i32>(this, 0x20, d);
        wr::<u8>(this, 0x24, 1);
    }
}

/// `CTaskInteriorLieInBed::FinishAnimCB` (0x675FC0).
const LIE_IN_BED_FINISH_CB: u32 = 0x675FC0;

/// `CTaskInteriorLieInBed::ProcessPed(CPed*)` (0x6772E0, `ret 4`, AL). Base anim +0x2C (get in), +1 the loop, +2
/// get out; anim +0x14, previous anim +0x18, the get-out timer +0x1C, +0x28 "place the ped", +0x29 finished, +0x2A
/// aborting, +0x11 start in the loop. `SetMoveState(1)`; finished with the get-out anim gone, or aborting without
/// the int_house anims: the scanner set to script peds only, 1. Aborting: get-in → blend delta (+0x1C) -8; loop
/// (not being placed) → get-out blended (1000), 0; get-out → anim +0x24 = 3. No anim: without the anims 0; first
/// time (+0x18 = -1) the scanner cleared, then the loop (timer started, placed) or get-in (blend 4, not placed);
/// after get-in the loop; 0. With an anim: placing moves the ped to its matrix times the bed offset (0xC18C54 for
/// anims 0x152 / 0x153, 0xC18C60 for 0x155 / 0x156, else its own position; z kept) and, for the loop, turns it by pi
/// (`LimitRadianAngle`, the matrix's Z rotation or +0x10); the timer out → get-out; in get-in the step towards the
/// interior info. 0.
#[hook(addr = 0x6772E0, conv = thiscall, name = "CTaskInteriorLieInBed::ProcessPed", expect = "83 EC 18 53 55 56 8B F1 8B 46 14",
       unverified = "frontier2-a13", route = "L2", hazards = "heap-writes,ax-al-return,stack-residue",
       note = "the original stores floats over its own ped argument slot; AL result")]
pub fn interior_lie_in_bed_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let anim = rd::<GPtr<u8>>(this, 0x14);
    let cur = if anim.is_null() { -1 } else { i32::from(rd::<i16>(anim, 0x2C)) };
    ped_set_move_state(ped, 1);
    let base = rd::<i32>(this, 0x2C);
    if rd::<u8>(this, 0x29) != 0 && clump_get_association(rd::<GPtr<u8>>(ped, 0x18), (base + 2) as u32).is_null() {
        scanner_only_script_peds(ped);
        return 1;
    }
    if rd::<u8>(this, 0x2A) != 0 {
        if are_anims_loaded(0) == 0 {
            scanner_only_script_peds(ped);
            return 1;
        }
        if cur == base {
            wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x1C, 0xC100_0000);
        } else if cur == base + 1 {
            if rd::<u8>(this, 0x28) == 0 {
                interior_blend(this, ped, (base + 2) as u32, 1000.0, LIE_IN_BED_FINISH_CB, true);
                wr::<u8>(this, 0x28, 1);
                return 0;
            }
        } else if cur == base + 2 {
            wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x24, 0x4040_0000);
        }
    }
    if rd::<GPtr<u8>>(this, 0x14).is_null() {
        if are_anims_loaded(0) == 0 {
            return 0;
        }
        let prev = rd::<i32>(this, 0x18);
        if prev == -1 {
            let s = rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x238);
            for o in [0xC, 0xD, 0xE] {
                wr::<u8>(s, o, 0);
            }
            if rd::<u8>(this, 0x11) == 0 {
                interior_blend(this, ped, base as u32, 4.0, LIE_IN_BED_FINISH_CB, false);
                return 0;
            }
        } else if prev != base {
            return 0;
        }
        interior_start_timer(this);
        interior_blend(this, ped, (base + 1) as u32, 1000.0, LIE_IN_BED_FINISH_CB, false);
        wr::<u8>(this, 0x28, 1);
        return 0;
    }
    if rd::<u8>(this, 0x28) != 0 {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let mut p: [f32; 3] = core::array::from_fn(|i| rd::<f32>(entity_pos(ped), 4 * i as isize));
        let z = p[2];
        let off = match cur {
            0x152 | 0x153 => Some(0xC18C54),
            0x155 | 0x156 => Some(0xC18C60),
            _ => None,
        };
        if let Some(off) = off {
            let mut o = [0f32; 3];
            matrix_mul_vec(GPtr::from_raw(o.as_mut_ptr()).cast(), m, GPtr::from_addr(off));
            p = o;
        }
        p[2] = z;
        placeable_set_posn(ped, GPtr::from_raw(p.as_mut_ptr()));
        wr::<u8>(this, 0x28, 0);
        if cur == base + 1 {
            let h = x(rd::<f32>(ped, 0x558)).fadd_m(g::<f32>(0x858CB8).read()).to_f32();
            let h = general_limit_radian_angle(h) as f32;
            wr::<f32>(ped, 0x558, h);
            wr::<f32>(ped, 0x55C, h);
            let m = rd::<GPtr<u8>>(ped, 0x14);
            if m.is_null() {
                wr::<f32>(ped, 0x10, h);
            } else {
                matrix_set_rotate_z_only(m, h);
            }
        }
    }
    if rd::<u8>(this, 0x24) != 0
        && task_timer_is_out_of_time(this.byte_add(0x1C)) != 0
        && i32::from(rd::<i16>(rd::<GPtr<u8>>(this, 0x14), 0x2C)) != base + 2
    {
        interior_blend(this, ped, (base + 2) as u32, 1000.0, LIE_IN_BED_FINISH_CB, true);
        wr::<u8>(this, 0x28, 1);
    }
    if i32::from(rd::<i16>(rd::<GPtr<u8>>(this, 0x14), 0x2C)) == rd::<i32>(this, 0x2C) {
        interior_step_to_info(this, ped);
    }
    0
}

/// vt_870314's finish callback (0x675DD0).
const SIT_IN_CHAIR_FINISH_CB: u32 = 0x675DD0;

/// `vt_870314::ProcessPed(CPed*)` (0x676D30, `ret 4`, AL; the house chair task, look-at name "TaskSitInChair"). The
/// anims are fixed: 0x157 sit down, 0x159 the loop, 0x158 stand up; the fields as `CTaskInteriorLieInBed`'s (start
/// in the loop: +0x10). Finished (+0x29) with 0x158 gone, or aborting without the int_house anims: 1 (no scanner
/// change). Aborting: sit-down → blend delta -8; loop (not being placed) → stand-up blended, 0; stand-up → anim
/// +0x24 = 3. No anim (with the anims loaded): the loop (timer, placed) on the first run with +0x10 or after the sit
/// down, the sit-down (blend 4) on the first run. With an anim: placing, for the loop, turns the ped by pi first, then
/// moves it to its matrix times 0xC18C78 (loop and stand-up) or its own position, z kept; the timer out → stand-up;
/// sitting down steps towards the interior info. Every path but the two early ones ends in the look-at: during the
/// loop a ped not already looking (2 % per frame, `rand(0, 1000) > 980`) looks at the closest interior info of its
/// group within 10 for 10..20 s; otherwise a running look-at is aborted (250 ms). 0.
#[hook(addr = 0x676D30, conv = thiscall, name = "vt_870314::ProcessPed", expect = "83 EC 20 53 55 56 8B F1 8B 46 14",
       unverified = "frontier2-a13", route = "L2", hazards = "heap-writes,ax-al-return,stack-residue",
       note = "the original stores floats and a null info pointer over its own ped argument slot; AL result; the 0x6771B2 island compares the anim id")]
pub fn interior_sit_in_chair_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let anim = rd::<GPtr<u8>>(this, 0x14);
    let cur = if anim.is_null() { -1 } else { i32::from(rd::<i16>(anim, 0x2C)) };
    ped_set_move_state(ped, 1);
    if rd::<u8>(this, 0x29) != 0 && clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0x158).is_null() {
        return 1;
    }
    if rd::<u8>(this, 0x2A) != 0 {
        if are_anims_loaded(0) == 0 {
            return 1;
        }
        match cur {
            0x157 => wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x1C, 0xC100_0000),
            0x159 => {
                if rd::<u8>(this, 0x28) == 0 {
                    interior_blend(this, ped, 0x158, 1000.0, SIT_IN_CHAIR_FINISH_CB, true);
                    wr::<u8>(this, 0x28, 1);
                    return 0;
                }
            }
            0x158 => wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x24, 0x4040_0000),
            _ => {}
        }
    }
    if rd::<GPtr<u8>>(this, 0x14).is_null() {
        if are_anims_loaded(0) != 0 {
            let prev = rd::<i32>(this, 0x18);
            if (prev == -1 && rd::<u8>(this, 0x10) != 0) || prev == 0x157 {
                interior_start_timer(this);
                interior_blend(this, ped, 0x159, 1000.0, SIT_IN_CHAIR_FINISH_CB, false);
                wr::<u8>(this, 0x28, 1);
            } else if prev == -1 {
                interior_blend(this, ped, 0x157, 4.0, SIT_IN_CHAIR_FINISH_CB, false);
            }
        }
    } else {
        if rd::<u8>(this, 0x28) != 0 {
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let mut p: [f32; 3] = core::array::from_fn(|i| rd::<f32>(entity_pos(ped), 4 * i as isize));
            let z = p[2];
            if cur == 0x159 || cur == 0x158 {
                let mut o = [0f32; 3];
                matrix_mul_vec(GPtr::from_raw(o.as_mut_ptr()).cast(), m, GPtr::from_addr(0xC18C78));
                p = o;
            }
            if cur == 0x159 {
                let h = x(rd::<f32>(ped, 0x558)).fadd_m(g::<f32>(0x858CB8).read()).to_f32();
                let h = general_limit_radian_angle(h) as f32;
                wr::<f32>(ped, 0x55C, h);
                wr::<f32>(ped, 0x558, h);
                let m = rd::<GPtr<u8>>(ped, 0x14);
                if m.is_null() {
                    wr::<f32>(ped, 0x10, h);
                } else {
                    matrix_set_rotate_z_only(m, h);
                }
            }
            p[2] = z;
            placeable_set_posn(ped, GPtr::from_raw(p.as_mut_ptr()));
            wr::<u8>(this, 0x28, 0);
        }
        if rd::<u8>(this, 0x24) != 0
            && task_timer_is_out_of_time(this.byte_add(0x1C)) != 0
            && rd::<i16>(rd::<GPtr<u8>>(this, 0x14), 0x2C) != 0x158
        {
            interior_blend(this, ped, 0x158, 1000.0, SIT_IN_CHAIR_FINISH_CB, true);
            wr::<u8>(this, 0x28, 1);
        }
        if rd::<i16>(rd::<GPtr<u8>>(this, 0x14), 0x2C) == 0x157 {
            interior_step_to_info(this, ped);
        }
    }
    let ik = GPtr::<u8>::from_addr(0xC15448);
    let a = rd::<GPtr<u8>>(this, 0x14);
    if !a.is_null() && rd::<i16>(a, 0x2C) == 0x159 {
        if ik_is_looking(ik, ped) != 0 || general_random_int_in_range(0, 1000) <= 0x3D4 {
            return 0;
        }
        let grp = get_peds_interior_group(GPtr::from_addr(0xBAF670), ped);
        if grp.is_null() {
            return 0;
        }
        let pp = entity_pos(ped);
        let mut info = GPtr::<u8>::NULL;
        let mut interior = GPtr::<u8>::NULL;
        let mut dist = 0f32;
        interior_group_find_closest_info(grp, 0, rd::<f32>(pp, 0), rd::<f32>(pp, 4), rd::<f32>(pp, 8), 10.0,
            GPtr::from_raw(&mut info), GPtr::from_raw(&mut interior), GPtr::from_raw(&mut dist));
        if info.is_null() {
            return 0;
        }
        let t = general_random_int_in_range(10000, 20000);
        ik_look_at(ik, GPtr::from_addr(0x870380), ped, GPtr::NULL, t, -1, info.byte_add(4), 0, 0.25, 0x1F4, 3, 0);
        return 0;
    }
    if ik_is_looking(ik, ped) != 0 {
        ik_abort_look_at(ik, ped, 0xFA);
    }
    0
}

pub const HOOKS: &[HookReg] = &[
    HOOK_KILL_PED_WITH_CAR_DAMAGE_CAR_BONNET,
    HOOK_KILL_PED_WITH_CAR_SHAKE_PAD,
    HOOK_KILL_PED_WITH_CAR_PROCESS_PED,
    HOOK_USE_SWAT_ROPE_CONTROL_SUB_TASK,
    HOOK_GANG_HASSLE_VEHICLE_CONTROL_SUB_TASK,
    HOOK_INTERIOR_LIE_IN_BED_PROCESS_PED,
    HOOK_INTERIOR_SIT_IN_CHAIR_PROCESS_PED,
];
