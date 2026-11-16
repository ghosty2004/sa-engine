//! FRONTIER-2 turn 9, lane A (batch `frontier2-a12`): the last AI frontier virtuals and their callees, ported from
//! the 1.0 US bytes (gta-reversed read as an aid; where it differs the bytes win, noted per function).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x, xi, X87};
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
use sa_world::world::common::entity_pos;
use sa_world::world::hand_object::model_info;

/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS: usize = 0xB7CB84;

orig! {
    /// `CEventVehicleHitAndRun::CEventVehicleHitAndRun(CPed*, CVehicle*)` (0x4AE990, `ret 8`)
    fn hit_and_run_ctor(e: u32, ped: u32, car: u32) -> u32 = thiscall 0x4AE990 => crate::gen_ctors::g_event_vehicle_hit_and_run_constructor_4ae990;
    /// `CEventVehicleHitAndRun::~CEventVehicleHitAndRun()` (0x4AEA50)
    fn hit_and_run_dtor(e: GPtr<u8>) = thiscall 0x4AEA50 => crate::event_dtors::event_vehicle_hit_and_run_dtor;
}

/// `((v.x * v.x + v.y * v.y) + v.z * v.z)` then `fsqrt`: the inline magnitude of the vector at `v` (its components
/// loaded z, y, x).
fn magnitude(v: GPtr<u8>) -> X87 {
    let (vx, vy, vz) = (x(rd::<f32>(v, 0)), x(rd::<f32>(v, 4)), x(rd::<f32>(v, 8)));
    vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt()
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

pub const HOOKS: &[HookReg] = &[
    HOOK_KILL_PED_WITH_CAR_DAMAGE_CAR_BONNET,
    HOOK_KILL_PED_WITH_CAR_SHAKE_PAD,
    HOOK_KILL_PED_WITH_CAR_PROCESS_PED,
];
