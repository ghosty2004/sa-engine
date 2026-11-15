//! `CTaskComplexKillCriminal::CreateFirstSubTask` (PORT-AI-5, helper B). Ported from the 1.0 US bytes (the exe wins
//! over `docs/re/specs/AI/CTaskComplexKillCriminal.md`; gta-reversed does not reverse it). Every call to another
//! catalogued function goes through `orig!` at the original address; tasks come from the original task pool.
//!
//! `CTaskComplexKillCriminal`: +8 sub-task, +0xC criminal, +0x10 cop (registered reference), +0x18 randomize,
//! +0x1B / +0x1C / +0x1D saved driving style / mission / cruise speed, +0x1E set up. `CCopPed`: +0x7A8 partner,
//! +0x7C0 is-the-driver. `CVehicle`: +0x3B9 / +0x3BA / +0x3D0 autopilot bytes, +0x460 driver, +0x464 passengers,
//! +0x484 passenger count, +0x488 maximum passengers, +0x590 vehicle type.
#![allow(clippy::unreadable_literal)]

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// Ambient-crime enable flag (script 09D2).
const AMBIENT_CRIME: usize = 0xB72994;
/// `g_InterestingEvents`
const INTERESTING_EVENTS: usize = 0xC0B058;
/// The vtable the original stores before calling the event base destructor 0x4AF890.
const EVENT_BASE_VTBL: u32 = 0x858E68;

use crate::bindings::acquaintance_ped_hate_ctor as event_ped_hate_ctor;
use sa_world::bindings::entity_register_reference;
use sa_world::bindings::find_player_ped as world_find_player_ped;
use sa_core::bindings::general_random_int_in_range;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::event_acquaintance_ped_dtor as event_base_dtor;
use crate::bindings::event_group_add;
use sa_world::bindings::intel_find_task_by_type;
use sa_world::bindings::ped_set_current_weapon;
use sa_world::bindings::set_ped_decision_maker_type as intel_set_ped_decision_maker_type;
use sa_world::bindings::simple_car_drive_ctor;
use sa_core::bindings::ped_give_weapon;
use sa_world::bindings::car_drive_mission_ctor;
use sa_world::bindings::cop_add_criminal_to_kill;
use crate::bindings::interesting_events_add;
use sa_world::bindings::kill_ped_on_foot_ctor;
use sa_world::bindings::leave_car_ctor;
orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CCopPed::SetPartner(CCopPed*)` (0x5DDE80, `ret 4`)
    fn cop_set_partner(cop: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5DDE80 => unported;
}

fn sub_task(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 8)
}

/// "reuse T": the current sub-task when it has type `ty` (one `GetTaskType` call), else null.
fn reuse(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    let s = sub_task(this);
    if !s.is_null() && v_task_type(s) == ty {
        sub_task(this)
    } else {
        GPtr::NULL
    }
}

fn criminal(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0xC)
}

fn cop(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x10)
}

use sa_world::world::cworld::in_vehicle;
/// A stack `CEventAcquaintancePedHate(about)` with task id `task` (+0xE) added to `target`'s event group
/// (intelligence +0x68), then destroyed (vtable 0x858E68, base destructor 0x4AF890). The original's slot is 0x18
/// bytes; the Rust buffer is 0x20.
fn send_hate(about: GPtr<u8>, target_intel: impl FnOnce() -> GPtr<u8>, task: u16) {
    let mut ev = [0u32; 8];
    let e: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 8]).cast();
    event_ped_hate_ctor(e.addr() as u32, about.addr() as u32);
    wr::<u16>(e, 0xE, task);
    let intel = target_intel();
    event_group_add(intel.byte_add(0x68), e, 0);
    wr::<u32>(e, 0, EVENT_BASE_VTBL);
    event_base_dtor(e.cast::<u32>());
}

/// `GiveWeapon(22, 1000, true)` then `SetCurrentWeapon(22)`, the target re-read for the second call.
fn arm(p: impl Fn() -> GPtr<u8>) {
    ped_give_weapon(p(), 0x16, 0x3E8, 1);
    ped_set_current_weapon(p(), 0x16);
}

/// `CTaskComplexKillCriminal::CreateFirstSubTask(CPed* cop)` (0x68DC60, `ret 4`; the tail continues through the
/// island 0x407651). Null when there is no criminal, it is a player, the player is wanted, ambient crime is off,
/// the ped is not a cop, a player sits in the criminal's vehicle, or (randomized, criminal +0x478 bit 0x800
/// clear) the criminal is not in a vehicle with +0x42E bit 3 or `GetRandomNumberInRange(0, 3)` != 0. Then the
/// cop is registered, the criminal added to its kill list and a sub-task chosen from the cop's / criminal's
/// vehicles (leave car, car-drive mission 0x35 / 0x12 / 0x37, simple car drive, kill on foot; an existing
/// sub-task of the chosen type is reused); a driver cop whose partner already hunts somebody else gets a hate
/// event sent to the partner and the function returns null. The tail arms the criminal (and its passengers;
/// `rand()` at 0x68E3D9 / 0x68E403, return addresses 0x68E3DE / 0x68E408) and sets the cop's flags.
///
/// Original behaviour kept: a null `m_pPlayerData` of the player is read as address 0x2C; returning null after the
/// reference registration leaves the cop registered; the failed randomize path stores the criminal's model index
/// into its own dead argument slot (not reproduced: never read). The `CEventAcquaintancePedHate` objects live on
/// the original's stack (Rust buffers here).
#[hook(addr = 0x68DC60, conv = thiscall, name = "CTaskComplexKillCriminal::CreateFirstSubTask", expect = "6A FF 64 A1 00 00 00 00")]
pub fn kill_criminal_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let c = criminal(this);
    if c.is_null() || ped_is_player(c) != 0 {
        return GPtr::NULL;
    }
    let pd = rd::<GPtr<u8>>(world_find_player_ped(-1), 0x480);
    let wanted = if pd.is_null() {
        GPtr::NULL
    } else {
        rd::<GPtr<u8>>(pd, 0)
    };
    if rd::<u32>(wanted, 0x2C) != 0 || g::<u8>(AMBIENT_CRIME).read() == 0 || rd::<i32>(ped, 0x598) != 6 {
        return GPtr::NULL;
    }
    let cveh = rd::<GPtr<u8>>(criminal(this), 0x58C);
    if !cveh.is_null() {
        let drv = rd::<GPtr<u8>>(cveh, 0x460);
        if !drv.is_null() && ped_is_player(drv) != 0 {
            return GPtr::NULL;
        }
        let veh = || rd::<GPtr<u8>>(criminal(this), 0x58C);
        if rd::<u8>(veh(), 0x488) != 0 {
            let mut i = 0i32;
            loop {
                let p = rd::<GPtr<u8>>(veh(), (0x464 + i * 4) as isize);
                if !p.is_null() && ped_is_player(p) != 0 {
                    return GPtr::NULL;
                }
                i += 1;
                if i >= i32::from(rd::<u8>(veh(), 0x488)) {
                    break;
                }
            }
        }
    }
    let cop_ref = this.byte_add(0x10);
    wr::<GPtr<u8>>(cop_ref, 0, ped);
    entity_register_reference(ped, cop_ref);
    if rd::<u8>(this, 0x18) != 0 {
        let c = criminal(this);
        if rd::<u32>(c, 0x478) & 0x800 == 0 {
            if !in_vehicle(c) {
                return GPtr::NULL;
            }
            let v = rd::<GPtr<u8>>(c, 0x58C);
            if v.is_null() || rd::<u8>(v, 0x42E) & 8 == 0 {
                return GPtr::NULL;
            }
            if general_random_int_in_range(0, 3) != 0 {
                return GPtr::NULL;
            }
        }
    }
    cop_add_criminal_to_kill(cop(this), criminal(this));
    let c = criminal(this);
    if in_vehicle(c) {
        intel_set_ped_decision_maker_type(rd::<GPtr<u8>>(c, 0x47C), 6);
    }
    let mut task: Option<GPtr<u8>> = None;
    let k = cop(this);
    if in_vehicle(k) && !rd::<GPtr<u8>>(k, 0x58C).is_null() {
        let drv = rd::<GPtr<u8>>(rd::<GPtr<u8>>(k, 0x58C), 0x460);
        if drv == ped {
            // 6a (0x68DE09)
            wr::<u8>(k, 0x7C0, 1);
            let k = cop(this);
            if rd::<u32>(k, 0x7A8) == 0 {
                let veh = rd::<GPtr<u8>>(k, 0x58C);
                if rd::<u8>(veh, 0x488) != 0 {
                    let mut slot = veh.byte_add(0x464);
                    let mut i = 0i32;
                    loop {
                        let p = rd::<GPtr<u8>>(slot, 0);
                        if !p.is_null() && rd::<i32>(p, 0x598) == 6 {
                            cop_set_partner(k, p);
                            break;
                        }
                        i += 1;
                        slot = slot.byte_add(4);
                        if i >= i32::from(rd::<u8>(rd::<GPtr<u8>>(cop(this), 0x58C), 0x488)) {
                            break;
                        }
                    }
                }
            }
            if rd::<u32>(cop(this), 0x7A8) != 0 {
                send_hate(
                    criminal(this),
                    || rd::<GPtr<u8>>(rd::<GPtr<u8>>(cop(this), 0x7A8), 0x47C),
                    0x451,
                );
            }
        } else {
            // 6b (0x68DEF3)
            if !drv.is_null() && rd::<i32>(drv, 0x598) == 6 {
                cop_set_partner(k, drv);
                wr::<u8>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(cop(this), 0x58C), 0x460), 0x7C0, 1);
            }
            let k = cop(this);
            let partner = rd::<GPtr<u8>>(k, 0x7A8);
            if !partner.is_null() {
                if rd::<u8>(partner, 0x7C0) != 0 {
                    let t = intel_find_task_by_type(rd::<GPtr<u8>>(partner, 0x47C), 0x451);
                    if !t.is_null() && rd::<GPtr<u8>>(t, 0xC) == criminal(this) {
                        wr::<u8>(cop(this), 0x7C0, 0);
                        let k = cop(this);
                        wr::<u32>(k, 0x46C, rd::<u32>(k, 0x46C) | 0x2000_0000);
                    } else {
                        send_hate(
                            criminal(this),
                            || rd::<GPtr<u8>>(rd::<GPtr<u8>>(cop(this), 0x7A8), 0x47C),
                            0x451,
                        );
                        return GPtr::NULL;
                    }
                }
            } else {
                // 0x68DFCD
                wr::<u8>(k, 0x7C0, 1);
                let r = reuse(this, 0x2C0);
                let t = if !r.is_null() {
                    r
                } else {
                    let mem = task_new(0x34);
                    if mem.is_null() {
                        GPtr::NULL
                    } else {
                        leave_car_ctor(mem, rd::<GPtr<u8>>(ped, 0x58C), 0, 0, 1, 0)
                    }
                };
                if !t.is_null() {
                    task = Some(t);
                }
            }
        }
        if task.is_none() {
            task = Some(step7(this, ped));
        }
    } else {
        // 8 (0x68E273)
        let partner = rd::<GPtr<u8>>(k, 0x7A8);
        if partner.is_null()
            || !x(rd::<f32>(partner, 0x540))
                .fcom(x(g::<f32>(0x858B50).read()))
                .test_parity(0x41)
        {
            wr::<u8>(k, 0x7C0, 1);
        }
        task = Some(kill_on_foot(this, ped));
    }
    let task = task.unwrap_or(GPtr::NULL);
    tail(this, ped, task)
}

/// Step 7 of 0x68DC60 (0x68E036): the sub-task from the criminal's vehicle state.
fn step7(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let c = criminal(this);
    if !in_vehicle(c) || rd::<GPtr<u8>>(c, 0x58C).is_null() {
        return kill_on_foot(this, ped);
    }
    if rd::<u8>(cop(this), 0x7C0) != 0 {
        let r = reuse(this, 0x2D4);
        if !r.is_null() {
            return r;
        }
        let pveh = rd::<GPtr<u8>>(ped, 0x58C);
        if pveh.is_null() {
            return GPtr::NULL;
        }
        let c = criminal(this);
        if in_vehicle(c) && !rd::<GPtr<u8>>(c, 0x58C).is_null() {
            let mission = if rd::<i32>(pveh, 0x590) == 9 { 0x35 } else { 0x12 };
            let mem = task_new(0x2C);
            if mem.is_null() {
                return GPtr::NULL;
            }
            let cv = rd::<GPtr<u8>>(criminal(this), 0x58C);
            let speed = xi(i32::from(rd::<u8>(cv, 0x3D0))).fadd_m(g::<f32>(0x85862C).read());
            car_drive_mission_ctor(
                mem,
                rd::<GPtr<u8>>(ped, 0x58C),
                cv,
                mission as u32,
                2,
                f32_of(speed).to_bits(),
            )
        } else {
            // 0x68E17D (unreachable: the criminal was checked to be in a vehicle above)
            let mem = task_new(0x2C);
            if mem.is_null() {
                return GPtr::NULL;
            }
            car_drive_mission_ctor(mem, rd::<GPtr<u8>>(ped, 0x58C), criminal(this), 0x37, 2, 0x41A0_0000)
        }
    } else {
        let r = reuse(this, 0x2C5);
        if !r.is_null() {
            return r;
        }
        let mem = task_new(0x60);
        if mem.is_null() {
            return GPtr::NULL;
        }
        simple_car_drive_ctor(mem, rd::<GPtr<u8>>(ped, 0x58C), GPtr::NULL, 0)
    }
}

/// Kill on foot (0x68E225 / 0x68E297): reuse a 0x3E8 sub-task (no weapon change), else a new
/// `CTaskComplexKillPedOnFoot(criminal, -1, 0, 0, 0, true)` and `SetCurrentWeapon(22)` (also when the allocation
/// failed).
fn kill_on_foot(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let r = reuse(this, 0x3E8);
    if !r.is_null() {
        return r;
    }
    let mem = task_new(0x38);
    let t = if mem.is_null() {
        GPtr::NULL
    } else {
        kill_ped_on_foot_ctor(mem, criminal(this), -1, 0, 0, 0, 1)
    };
    ped_set_current_weapon(ped, 0x16);
    t
}

/// TAIL of 0x68DC60 (0x68E2DA, through the island 0x407651).
fn tail(this: GPtr<u8>, ped: GPtr<u8>, task: GPtr<u8>) -> GPtr<u8> {
    let pveh = rd::<GPtr<u8>>(ped, 0x58C);
    if !pveh.is_null() && rd::<u8>(cop(this), 0x7C0) != 0 {
        wr::<u8>(this, 0x1B, rd::<u8>(pveh, 0x3B9));
        wr::<u8>(this, 0x1C, rd::<u8>(rd::<GPtr<u8>>(ped, 0x58C), 0x3BA));
        wr::<u8>(this, 0x1D, rd::<u8>(rd::<GPtr<u8>>(ped, 0x58C), 0x3D0));
        wr::<u8>(this, 0x1E, 1);
    }
    let cv = rd::<GPtr<u8>>(criminal(this), 0x58C);
    let crim = || criminal(this);
    if !cv.is_null() {
        if rd::<u8>(cv, 0x42E) & 8 != 0 {
            if rd::<u8>(cv, 0x484) != 0 {
                arm(crim);
                let veh = || rd::<GPtr<u8>>(criminal(this), 0x58C);
                if rd::<u8>(veh(), 0x488) != 0 {
                    let mut i = 0i32;
                    loop {
                        let p = rd::<GPtr<u8>>(veh(), (0x464 + i * 4) as isize);
                        if !p.is_null() {
                            arm(|| p);
                            cop_add_criminal_to_kill(cop(this), p);
                        }
                        i += 1;
                        if i >= i32::from(rd::<u8>(veh(), 0x488)) {
                            break;
                        }
                    }
                }
            } else if at_site(0x68E3DE, rand) & 1 != 0 {
                arm(crim);
            }
        }
    } else if at_site(0x68E408, rand) & 1 != 0 {
        arm(crim);
        send_hate(ped, || rd::<GPtr<u8>>(criminal(this), 0x47C), 0x3E8);
    }
    let now = g::<u32>(TIME_MS).read();
    let flags = rd::<u32>(ped, 0x470);
    let pveh = rd::<GPtr<u8>>(ped, 0x58C);
    wr::<u32>(ped, 0x54C, now.wrapping_add(300000));
    wr::<u32>(ped, 0x470, flags | 0x40);
    wr::<u32>(ped, 0x790, 0x3E99_999A);
    if !pveh.is_null() {
        wr::<u16>(pveh, 0x4A6, 0xFF);
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        wr::<u8>(v, 0x42C, rd::<u8>(v, 0x42C) | 0x40);
    }
    let v = rd::<GPtr<u8>>(ped, 0x58C);
    if !v.is_null() && in_vehicle(ped) {
        wr::<u8>(v, 0x42D, rd::<u8>(v, 0x42D) | 0x80);
    }
    interesting_events_add(g(INTERESTING_EVENTS), 0x19, ped);
    task
}

// ---------------------------------------------------------------------------------------------------------
// FRONTIER-2 (batch `frontier2-3`): ControlSubTask 0x68E950 and the two members it calls, from the 1.0 US bytes
// (gta-reversed reverses neither ControlSubTask nor CreateFirstSubTask; its ChangeTarget / FindNextCriminalToKill
// differ: see the docs below).

use sa_core::bindings::{entity_clean_up_old_reference, vector_magnitude, vector_magnitude_sqr};
use sa_world::bindings::{
    enter_car_as_driver_ctor, enter_car_as_passenger_ctor, find_player_wanted, gang_drive_by_ctor, tm_set_task,
    vector_sub, wanted_can_cop_join_pursuit,
};
use crate::bindings::event_handler_current_type;
use sa_world::world::common::entity_pos;
use sa_math::fpu::{gt, lt};

orig! {
    /// virtual `CTask::MakeAbortable(CPed*, eAbortPriority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CPed::SetPedDefaultDecisionMaker()` (0x5E06E0)
    fn set_ped_default_decision_maker(p: GPtr<u8>) = thiscall 0x5E06E0 => unported;
    /// `CTaskComplexKillCriminal::FindNextCriminalToKill(CPed*, bool)` (0x68C3C0, `ret 8`)
    fn find_next_criminal_call(this: GPtr<u8>, ped: GPtr<u8>, any: u8) -> GPtr<u8> = thiscall 0x68C3C0 => kill_criminal_find_next_criminal_to_kill;
    /// `CTaskComplexKillCriminal::ChangeTarget(CPed*)` (0x68C6E0, `ret 4`, AL)
    fn change_target_call(this: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x68C6E0 => kill_criminal_change_target;
    /// virtual `CEvent::GetEventType()` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// virtual `CEvent::GetSourceEntity()` (vtable +0x28)
    fn v_source_entity(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
}

/// `fld health (+0x540); fcomp 0; test ah, 0x41; jp alive`: dead for health < 0 or == 0 (NaN counts as alive).
fn dead(p: GPtr<u8>) -> bool {
    !x(rd::<f32>(p, 0x540)).fcom(x(0.0)).test_parity(0x41)
}

/// `fcomp 0; test ah, 0x41; jne`: alive only for health > 0 (ordered).
fn alive_gt(p: GPtr<u8>) -> bool {
    gt(x(rd::<f32>(p, 0x540)), x(0.0))
}

/// `(a - b)` through `VectorSub` (0x40FE60) into a local; `SquaredMagnitude` (0x406DA0) of it.
fn sub_sq(a: GPtr<u8>, b: GPtr<u8>) -> X87 {
    let mut t = [0f32; 3];
    let v = vector_sub(GPtr::from_raw(t.as_mut_ptr()).cast(), a, b);
    vector_magnitude_sqr(v)
}

/// `(a - b)` through `VectorSub`; `CVector::Magnitude` (0x4082C0) of it.
fn sub_mag(a: GPtr<u8>, b: GPtr<u8>) -> X87 {
    let mut t = [0f32; 3];
    let v = vector_sub(GPtr::from_raw(t.as_mut_ptr()).cast(), a, b);
    X87::from_f64_exact(vector_magnitude(v.cast()))
}

fn veh_of(p: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(p, 0x58C)
}

/// `(x - p)^2` of two positions as 0x68C3C0 computes it: `(dz^2 + dy^2) + dx^2`.
fn dist_sq(a: GPtr<u8>, b: GPtr<u8>) -> X87 {
    let (a, b) = (a.cast::<f32>(), b.cast::<f32>());
    let dx = x(a.read()).fsub_m(b.read());
    let dy = x(a.wrapping_index(1).read()).fsub_m(b.wrapping_index(1).read());
    let dz = x(a.wrapping_index(2).read()).fsub_m(b.wrapping_index(2).read());
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx))
}

/// `CTaskComplexKillCriminal::FindNextCriminalToKill(CPed* ped, bool any)` (0x68C3C0, `ret 8`): the reference
/// distance is the squared 3D distance from `ped` to the current criminal (living, and only when `!any`), else 0.
/// Each of the cop's five criminals (+0x7AC..+0x7BC) that is living and not the current criminal is taken when it is
/// nearer than the reference or the reference is 0; the reference is never updated, so the last such entry wins
/// (gta-reversed returns the nearest).
#[hook(addr = 0x68C3C0, conv = thiscall, name = "CTaskComplexKillCriminal::FindNextCriminalToKill",
       expect = "51 8A 44 24 0C 55", unverified = "frontier2-3", route = "L2", hazards = "none")]
pub fn kill_criminal_find_next_criminal_to_kill(this: GPtr<u8>, ped: GPtr<u8>, any: u8) -> GPtr<u8> {
    let mut best = GPtr::NULL;
    let mut reference = 0f32;
    let c = criminal(this);
    if any == 0 && !c.is_null() && alive_gt(c) {
        reference = dist_sq(entity_pos(c), entity_pos(ped)).to_f32();
    }
    let cp = cop(this);
    for i in 0..5 {
        let cand = rd::<GPtr<u8>>(cp, 0x7AC + 4 * i);
        if cand.is_null() || !alive_gt(cand) || cand == criminal(this) {
            continue;
        }
        let d = dist_sq(entity_pos(cand), entity_pos(ped));
        // fcomp ref; test ah, 5; jnp take; else fld ref; fcomp 0; test ah, 0x44; jp skip
        if lt(d, x(reference)) || !x(reference).fcom(x(0.0)).test_parity(0x44) {
            best = cand;
        }
    }
    best
}

/// `CTaskComplexKillCriminal::ChangeTarget(CPed* ped)` (0x68C6E0, `ret 4`, AL): true at once when `ped` is the
/// criminal. False when `ped` is null or not living (`health > 0` or NaN passes), the criminal sits in a vehicle,
/// a kill-on-foot sub-task (0x3E8) refuses `MakeAbortable(ped, 1, null)`, or `ped` is not a living entry of the
/// cop's five criminals. Otherwise the criminal reference moves to `ped`, a partner in a vehicle running
/// `CTaskComplexKillCriminal` (0x451) gets `ChangeTarget(ped)` too, +0x19 is cleared and the result is true.
#[hook(addr = 0x68C6E0, conv = thiscall, name = "CTaskComplexKillCriminal::ChangeTarget",
       expect = "53 8B D9 8B 4B 0C", unverified = "frontier2-3", route = "L2", hazards = "heap-writes")]
pub fn kill_criminal_change_target(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let c = criminal(this);
    if ped == c {
        return 1;
    }
    if ped.is_null() || dead(ped) {
        return 0;
    }
    if !c.is_null() && in_vehicle(c) {
        return 0;
    }
    let s = sub_task(this);
    if !s.is_null() && v_task_type(s) == 0x3E8 && v_make_abortable(sub_task(this), ped, 1, GPtr::NULL) == 0 {
        return 0;
    }
    let cp = cop(this);
    let mut i = 0;
    while i < 5 {
        let e = rd::<GPtr<u8>>(cp, 0x7AC + 4 * i);
        if e == ped && alive_gt(e) {
            let slot = this.byte_add(0xC);
            let old = criminal(this);
            if !old.is_null() {
                entity_clean_up_old_reference(old, slot);
            }
            wr::<GPtr<u8>>(slot, 0, e);
            entity_register_reference(e, slot);
            break;
        }
        i += 1;
    }
    if i == 5 {
        return 0;
    }
    let partner = rd::<GPtr<u8>>(cop(this), 0x7A8);
    if !partner.is_null() && in_vehicle(partner) {
        let t = intel_find_task_by_type(rd::<GPtr<u8>>(partner, 0x47C), 0x451);
        if !t.is_null() {
            change_target_call(t, ped);
        }
    }
    wr::<u8>(this, 0x19, 0);
    1
}

/// `CTaskComplexKillCriminal::ControlSubTask(CPed* cop)` (0x68E950, `ret 4`; the epilogue passes the island
/// 0x401331, and one block the island 0x4018F2, which loads the criminal's vehicle). Null when the criminal is a
/// player, of ped type 6 / 0x12 / 0x13 / >= 0x18, created by a mission, when the player is wanted and the cop may
/// join the pursuit and the sub-task aborts, or when ambient crime (0xB72994) is off. A passenger cop whose partner
/// is gone becomes the driver (`SetPartner(null)`, leave car 0x2C0 when in a vehicle). Then a sub-task type is chosen
/// ([`passenger_choice`], [`driver_choice`], [`criminal_gone`]; 0x516 when the cop is dead), a bike cop is turned into
/// a chase (mission 0x35 behind the criminal's vehicle or 0x37 after the criminal), the criminal vehicle's
/// passengers get drive-bys at the cop when it is within 60, and the chosen task (0xC8: none) replaces the sub-task
/// if that aborts and is of another type: 0x2BC / 0x2BD enter the cop's vehicle, 0x2C0 leave it, 0x2C5 simple car
/// drive, 0x2D4 car-drive mission, 0x3E8 kill on foot, 0x3FE drive-by (both with weapon 22), 0x516 finished (the
/// criminal gets its default decision maker; null).
#[hook(addr = 0x68E950, conv = thiscall, name = "CTaskComplexKillCriminal::ControlSubTask",
       expect = "6A FF 68 16 71 84 00", unverified = "frontier2-3", route = "L2", hazards = "heap-writes",
       note = "the CEventAcquaintancePedHate lives on the original's stack (a Rust buffer here); the SEH frame is not reproduced")]
pub fn kill_criminal_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub0 = sub_task(this);
    let mut chosen: i32 = 0xC8;
    if !criminal(this).is_null() {
        if ped_is_player(criminal(this)) != 0 {
            return GPtr::NULL;
        }
        let c = criminal(this);
        let pt = rd::<i32>(c, 0x598);
        if matches!(pt, 6 | 0x12 | 0x13) || pt >= 0x18 || rd::<u8>(c, 0x484) == 2 {
            return GPtr::NULL;
        }
    }
    let pd = rd::<GPtr<u8>>(world_find_player_ped(-1), 0x480);
    let wanted = if pd.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(pd, 0) };
    if rd::<u32>(wanted, 0x2C) != 0
        && wanted_can_cop_join_pursuit(find_player_wanted(-1), ped) != 0
        && v_make_abortable(sub_task(this), ped, 1, GPtr::NULL) != 0
    {
        return GPtr::NULL;
    }
    if g::<u8>(AMBIENT_CRIME).read() == 0 {
        return GPtr::NULL;
    }
    let partner = rd::<GPtr<u8>>(cop(this), 0x7A8);
    if rd::<u8>(cop(this), 0x7C0) == 0 && (partner.is_null() || dead(partner)) {
        wr::<u8>(cop(this), 0x7C0, 1);
        cop_set_partner(cop(this), GPtr::NULL);
        if in_vehicle(ped) && !veh_of(ped).is_null() {
            chosen = 0x2C0;
        }
    }
    if dead(ped) {
        chosen = 0x516;
    } else if criminal(this).is_null() || dead(criminal(this)) {
        criminal_gone(this, ped, partner, &mut chosen);
    } else {
        if rd::<u8>(cop(this), 0x7C0) == 0 {
            passenger_choice(this, ped, partner, &mut chosen);
        } else {
            driver_choice(this, ped, partner, &mut chosen);
        }
        bike_chase(this, ped, &mut chosen);
    }
    drive_bys_at_cop(this, ped);
    if chosen == 0xC8 || v_make_abortable(sub_task(this), ped, 1, GPtr::NULL) == 0 {
        return sub0;
    }
    let s = sub_task(this);
    if !s.is_null() && v_task_type(s) == chosen {
        return sub_task(this);
    }
    create_chosen(this, ped, chosen)
}

/// The cop is a passenger (0x68EAD9): kill on foot (0x3E8) unless the partner sits in a vehicle or is entering one as
/// its driver (task 0x2BD) and the cop has a vehicle it may use (+0x1A clear): out of it → enter as passenger
/// (0x2BC); in it with the partner in it too and the criminal within 60 of it → drive-by (0x3FE); else simple car
/// drive (0x2C5).
fn passenger_choice(this: GPtr<u8>, ped: GPtr<u8>, partner: GPtr<u8>, chosen: &mut i32) {
    *chosen = 0x3E8;
    if !in_vehicle(partner) && intel_find_task_by_type(rd::<GPtr<u8>>(partner, 0x47C), 0x2BD).is_null() {
        return;
    }
    let v = veh_of(ped);
    if v.is_null() || rd::<u8>(this, 0x1A) != 0 {
        return;
    }
    if !in_vehicle(ped) {
        *chosen = 0x2BC;
        return;
    }
    if in_vehicle(partner) && lt(sub_mag(entity_pos(criminal(this)), entity_pos(v)), x(g::<f32>(0x858B34).read())) {
        *chosen = 0x3FE;
        return;
    }
    *chosen = 0x2C5;
}

/// The cop drives (0x68EBB4): its vehicle's siren goes on. A criminal in a vehicle: `flag` = that vehicle's health
/// below 250. With the cop in its vehicle: during a car-drive mission (0x2D4) or drive-by (0x3FE) sub-task, the
/// get-out timer (+0x14) restarts at 1.0 while the criminal's vehicle moves faster than 0.12, else counts down by
/// `timestep * 0.02`; at or below 0, unless the criminal is handling event 0x24, within 15 (squared 225) of the
/// criminal's vehicle → kill on foot and `FindNextCriminalToKill(ped, false)` / `ChangeTarget`; during a simple
/// car drive (0x2C5) with the partner gone or in a vehicle → car-drive mission. With the cop out of its vehicle:
/// the criminal's vehicle slower than 0.2 and the criminal within 6 of the cop or of a partner on foot → the hate
/// event; else (cop may use its vehicle, no enter-car sub-task) the criminal's vehicle not slower than 0.2 or further
/// than 20 → enter the vehicle (0x2BC / 0x2BD). Then `flag` sends the hate event. A criminal on foot: with the cop
/// in its vehicle, beyond 15 → car-drive mission when the partner is gone or in a vehicle; within → kill on foot
/// (on chase missions 0x36 / 0x37 only within 16, 5 on a bike); out of it, with its vehicle and allowed to use it,
/// the criminal beyond 25 or the vehicle beyond sqrt(250) → the next criminal (kill on foot) or enter as driver.
fn driver_choice(this: GPtr<u8>, ped: GPtr<u8>, partner: GPtr<u8>, chosen: &mut i32) {
    let v = veh_of(ped);
    if !v.is_null() && in_vehicle(ped) {
        wr::<u8>(v, 0x42D, rd::<u8>(v, 0x42D) | 0x80);
    }
    let c = criminal(this);
    if in_vehicle(c) && !veh_of(c).is_null() {
        let cv = veh_of(c);
        let flag = lt(x(rd::<f32>(cv, 0x4C0)), x(g::<f32>(0x859F80).read()));
        let mut hate = false;
        if in_vehicle(ped) && !veh_of(ped).is_null() {
            if v_task_type(sub_task(this)) == 0x2D4 || v_task_type(sub_task(this)) == 0x3FE {
                let c = criminal(this);
                if !lt(X87::from_f64_exact(vector_magnitude(veh_of(c).byte_add(0x44).cast())), x(g::<f32>(0x85A6F0).read())) {
                    wr::<f32>(this, 0x14, 1.0);
                } else {
                    let t = x(g::<f32>(0xB7CB5C).read()).fmul_m(g::<f32>(0x858B38).read()).fsubr_m(rd::<f32>(this, 0x14));
                    wr::<f32>(this, 0x14, t.to_f32());
                    // fcomp 0; test ah, 0x41; jp: positive (or NaN) keeps waiting
                    if !t.fcom(x(0.0)).test_parity(0x41)
                        && event_handler_current_type(rd::<GPtr<u8>>(c, 0x47C).byte_add(0x34)) != 0x24
                        && lt(sub_sq(entity_pos(veh_of(criminal(this))), entity_pos(veh_of(ped))), x(g::<f32>(0x8599D4).read()))
                    {
                        *chosen = 0x3E8;
                        let n = find_next_criminal_call(this, ped, 0);
                        if !n.is_null() {
                            change_target_call(this, n);
                        }
                    }
                }
            } else if v_task_type(sub_task(this)) == 0x2C5 && (partner.is_null() || dead(partner) || in_vehicle(partner)) {
                *chosen = 0x2D4;
            }
        } else if lt(X87::from_f64_exact(vector_magnitude(cv.byte_add(0x44).cast())), x(g::<f32>(0x858CC4).read())) {
            let near = |p: GPtr<u8>| lt(sub_sq(entity_pos(c), entity_pos(p)), x(g::<f32>(0x863204).read()));
            hate = near(ped) || (!partner.is_null() && !in_vehicle(partner) && near(partner));
        } else {
            let pv = veh_of(ped);
            if !pv.is_null()
                && rd::<u8>(this, 0x1A) == 0
                && v_task_type(sub_task(this)) != 0x2BD
                && v_task_type(sub_task(this)) != 0x2BC
            {
                let c = criminal(this);
                let d = sub_sq(entity_pos(veh_of(c)), entity_pos(veh_of(ped))).to_f32();
                let moving = !X87::from_f64_exact(vector_magnitude(veh_of(c).byte_add(0x44).cast()))
                    .fcom(x(g::<f32>(0x858CC4).read()))
                    .c0();
                if moving || gt(x(d), x(g::<f32>(0x85A700).read())) {
                    *chosen = 0x2BC + i32::from(rd::<u8>(cop(this), 0x7C0) != 0);
                }
            }
        }
        if hate || flag {
            hate_event(this, ped);
        }
        return;
    }
    let pv = veh_of(ped);
    if in_vehicle(ped) && !pv.is_null() {
        let d = sub_sq(entity_pos(c), entity_pos(pv));
        let gone = partner.is_null() || dead(partner) || in_vehicle(partner);
        if gt(d, x(g::<f32>(0x8599D4).read())) {
            if gone {
                *chosen = 0x2D4;
            }
            return;
        }
        let k = if rd::<i32>(pv, 0x590) == 9 { g::<f32>(0x858C80).read() } else { g::<f32>(0x8599D0).read() };
        let m = rd::<u8>(pv, 0x3BA);
        if (m != 0x37 && m != 0x36) || lt(d, x(k)) {
            *chosen = 0x3E8;
        }
        return;
    }
    if rd::<u8>(this, 0x1A) != 0 || veh_of(ped).is_null() {
        return;
    }
    let far = gt(sub_sq(entity_pos(c), entity_pos(ped)), x(g::<f32>(0x85A6E8).read()))
        || gt(sub_sq(entity_pos(veh_of(ped)), entity_pos(ped)), x(g::<f32>(0x859F80).read()));
    if far {
        let n = find_next_criminal_call(this, ped, 0);
        *chosen = if !n.is_null() && change_target_call(this, n) != 0 { 0x3E8 } else { 0x2BD };
    }
}

/// The hate event (0x68EF56): a stack `CEventAcquaintancePedHate(cop)` with task id (+0xE) 0x2C0 when the criminal's
/// vehicle health is below 250, else 0x406 (0x38F only for a null weapon slot address, never), added to the
/// criminal's event group; the criminal gets its default decision maker; the vehicle's driver (not the criminal, not a
/// player) gets the event, and every passenger (not the criminal, not a player) the event and its default decision
/// maker. Then the event is destroyed (vtable 0x858E68, base destructor).
fn hate_event(this: GPtr<u8>, ped: GPtr<u8>) {
    let mut ev = [0u32; 8];
    let e: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 8]).cast();
    event_ped_hate_ctor(e.addr() as u32, ped.addr() as u32);
    let c = criminal(this);
    let task: u16 = if lt(x(rd::<f32>(veh_of(c), 0x4C0)), x(g::<f32>(0x859F80).read())) {
        0x2C0
    } else {
        let slot = (c.addr() as u32).wrapping_add(0x5A0).wrapping_add((i32::from(rd::<i8>(c, 0x718)) * 0x1C) as u32);
        if slot != 0 { 0x406 } else { 0x38F }
    };
    wr::<u16>(e, 0xE, task);
    event_group_add(rd::<GPtr<u8>>(c, 0x47C).byte_add(0x68), e, 0);
    set_ped_default_decision_maker(criminal(this));
    let c = criminal(this);
    let drv = rd::<GPtr<u8>>(veh_of(c), 0x460);
    if c != drv && !drv.is_null() && ped_is_player(drv) == 0 {
        let drv = rd::<GPtr<u8>>(veh_of(criminal(this)), 0x460);
        event_group_add(rd::<GPtr<u8>>(drv, 0x47C).byte_add(0x68), e, 0);
    }
    if rd::<u8>(veh_of(criminal(this)), 0x488) != 0 {
        let mut i = 0i32;
        loop {
            let c = criminal(this);
            let p = rd::<GPtr<u8>>(veh_of(c), (0x464 + i * 4) as isize);
            if !p.is_null() && p != c && ped_is_player(p) == 0 {
                event_group_add(rd::<GPtr<u8>>(p, 0x47C).byte_add(0x68), e, 0);
                set_ped_default_decision_maker(p);
            }
            i += 1;
            if i >= i32::from(rd::<u8>(veh_of(criminal(this)), 0x488)) {
                break;
            }
        }
    }
    wr::<u32>(e, 0, EVENT_BASE_VTBL);
    event_base_dtor(e.cast::<u32>());
}

/// The criminal is gone or dead (0x68F476): unless the sub-task kills on foot (0x3E8) or another criminal is found
/// (`FindNextCriminalToKill(ped, true)`, `ChangeTarget`) or the sub-task enters a vehicle (0x2BC / 0x2BD): out of its
/// vehicle with permission → enter it (0x2BC / 0x2BD); otherwise finished (0x516), except in a vehicle during a
/// simple car drive (0x2C5) with a living partner on foot.
fn criminal_gone(this: GPtr<u8>, ped: GPtr<u8>, partner: GPtr<u8>, chosen: &mut i32) {
    if v_task_type(sub_task(this)) == 0x3E8 {
        return;
    }
    let n = find_next_criminal_call(this, ped, 1);
    if !n.is_null() && change_target_call(this, n) != 0 {
        return;
    }
    if v_task_type(sub_task(this)) == 0x2BC || v_task_type(sub_task(this)) == 0x2BD {
        return;
    }
    if !veh_of(ped).is_null() && !in_vehicle(ped) && rd::<u8>(this, 0x1A) == 0 {
        *chosen = 0x2BC + i32::from(rd::<u8>(cop(this), 0x7C0) != 0);
        return;
    }
    if in_vehicle(ped)
        && v_task_type(sub_task(this)) == 0x2C5
        && !partner.is_null()
        && !dead(partner)
        && !in_vehicle(partner)
    {
        return;
    }
    *chosen = 0x516;
}

/// A cop on a bike (vehicle type +0x590 == 9, 0x68F2A1): during a car-drive mission (0x2D4) with nothing or the same
/// chosen and the criminal within 60 → drive-by. With a drive-by chosen or running, the bike chases (status 3, style
/// 2): the criminal's vehicle (mission 0x35, cruise speed `ftol(its speed + 10)`, +0x3C8 the same as f32) or the
/// criminal itself (mission 0x37, cruise speed 20); the autopilot target (+0x41C) is set without a reference.
fn bike_chase(this: GPtr<u8>, ped: GPtr<u8>, chosen: &mut i32) {
    let pv = veh_of(ped);
    if pv.is_null() || rd::<i32>(pv, 0x590) != 9 {
        return;
    }
    let ty = v_task_type(sub_task(this));
    let before = *chosen;
    let mut go = false;
    if ty == 0x2D4
        && (before == 0xC8 || before == 0x2D4)
        && lt(sub_mag(entity_pos(criminal(this)), entity_pos(veh_of(ped))), x(g::<f32>(0x858B34).read()))
    {
        *chosen = 0x3FE;
        go = true;
    }
    if !go && before != 0x3FE && v_task_type(sub_task(this)) != 0x3FE {
        return;
    }
    let v = || veh_of(ped);
    let to_physics = |v: GPtr<u8>| wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x18);
    let c = criminal(this);
    if in_vehicle(c) && !veh_of(c).is_null() {
        to_physics(v());
        wr::<u8>(v(), 0x3BA, 0x35);
        let s = ftol32(xi(i32::from(rd::<u8>(veh_of(criminal(this)), 0x3D0))).fadd_m(g::<f32>(0x85862C).read()));
        wr::<u8>(v(), 0x3D0, s as u8);
        wr::<f32>(v(), 0x3C8, xi(i32::from(rd::<u8>(v(), 0x3D0))).to_f32());
        wr::<u8>(v(), 0x3B9, 2);
        wr::<GPtr<u8>>(v(), 0x41C, veh_of(criminal(this)));
    } else {
        to_physics(v());
        wr::<u8>(v(), 0x3BA, 0x37);
        wr::<u8>(v(), 0x3D0, 0x14);
        wr::<f32>(v(), 0x3C8, xi(i32::from(rd::<u8>(v(), 0x3D0))).to_f32());
        wr::<u8>(v(), 0x3B9, 2);
        wr::<GPtr<u8>>(v(), 0x41C, criminal(this));
    }
}

/// 0x68F53C: a criminal in a vehicle with passengers (+0x484) within 60 of the cop: every passenger in the vehicle
/// without a drive-by (0x3FE) gets `CTaskSimpleGangDriveBy(cop, null, 70, 70, 8, false)` as primary task (slot 3).
fn drive_bys_at_cop(this: GPtr<u8>, ped: GPtr<u8>) {
    let c = criminal(this);
    if c.is_null() || !in_vehicle(c) {
        return;
    }
    let cv = veh_of(c);
    if cv.is_null()
        || rd::<u8>(cv, 0x484) == 0
        || !lt(sub_mag(entity_pos(veh_of(criminal(this))), entity_pos(ped)), x(g::<f32>(0x858B34).read()))
        || rd::<u8>(cv, 0x488) == 0
    {
        return;
    }
    let mut i = 0i32;
    loop {
        let p = rd::<GPtr<u8>>(veh_of(criminal(this)), (0x464 + i * 4) as isize);
        if !p.is_null() && in_vehicle(p) && intel_find_task_by_type(rd::<GPtr<u8>>(p, 0x47C), 0x3FE).is_null() {
            let mem = task_new(0x44);
            let t = if mem.is_null() {
                GPtr::NULL
            } else {
                gang_drive_by_ctor(mem, ped, GPtr::NULL, 0x428C_0000, 0x46, 8, 0)
            };
            tm_set_task(rd::<GPtr<u8>>(p, 0x47C).byte_add(4), t, 3, 0);
        }
        i += 1;
        if i >= i32::from(rd::<u8>(veh_of(criminal(this)), 0x488)) {
            break;
        }
    }
}

/// The new sub-task of 0x68F6D0 (switch on the chosen type).
fn create_chosen(this: GPtr<u8>, ped: GPtr<u8>, chosen: i32) -> GPtr<u8> {
    match chosen {
        0x2BC | 0x2BD | 0x2C0 | 0x2C5 => {
            let size = match chosen {
                0x2BC | 0x2BD => 0x50,
                0x2C0 => 0x34,
                _ => 0x60,
            };
            let mem = task_new(size);
            if mem.is_null() {
                return GPtr::NULL;
            }
            let v = veh_of(ped);
            match chosen {
                0x2BC => enter_car_as_passenger_ctor(mem, v, 0, 0),
                0x2BD => enter_car_as_driver_ctor(mem, v),
                0x2C0 => leave_car_ctor(mem, v, 0, 0, 1, 0),
                _ => simple_car_drive_ctor(mem, v, GPtr::NULL, 0),
            }
        }
        0x2D4 => {
            let pv = veh_of(ped);
            if pv.is_null() {
                return GPtr::NULL;
            }
            let c = criminal(this);
            if in_vehicle(c) && !veh_of(c).is_null() {
                let mission = if rd::<i32>(pv, 0x590) == 9 { 0x35 } else { 0x12 };
                let mem = task_new(0x2C);
                if mem.is_null() {
                    return GPtr::NULL;
                }
                let cv = veh_of(criminal(this));
                let speed = xi(i32::from(rd::<u8>(cv, 0x3D0))).fadd_m(g::<f32>(0x85862C).read());
                car_drive_mission_ctor(mem, veh_of(ped), cv, mission, 2, f32_of(speed).to_bits())
            } else {
                let mem = task_new(0x2C);
                if mem.is_null() {
                    return GPtr::NULL;
                }
                car_drive_mission_ctor(mem, veh_of(ped), criminal(this), 0x37, 2, 0x41A0_0000)
            }
        }
        0x3E8 | 0x3FE => {
            let mem = task_new(if chosen == 0x3E8 { 0x38 } else { 0x44 });
            let t = if mem.is_null() {
                GPtr::NULL
            } else if chosen == 0x3E8 {
                kill_ped_on_foot_ctor(mem, criminal(this), -1, 0, 0, 0, 1)
            } else {
                gang_drive_by_ctor(mem, criminal(this), GPtr::NULL, 0x428C_0000, 0x46, 8, 0)
            };
            ped_set_current_weapon(ped, 0x16);
            t
        }
        0x516 => {
            let c = criminal(this);
            if !c.is_null() {
                set_ped_default_decision_maker(c);
            }
            GPtr::NULL
        }
        _ => GPtr::NULL,
    }
}

/// Registrations of this module.
/// `CTaskComplexKillCriminal::MakeAbortable(CPed*, eAbortPriority, const CEvent*)` (0x68DAD0, `ret 0xC`, AL;
/// FRONTIER-2 part 4, crossing 4000). With an event: its type `t` and source `s` (vtable +4 / +0x28, the source read
/// again for every use). Coming from the criminal (`s == criminal`, criminal set), a damage-like event (9, 0x24, 0x29,
/// 0x49, 0x1F, 0xF) is not obeyed: the event's +4 counter is incremented and false returned. From anyone else, 0x24 /
/// 0x49 are not obeyed either; 9 / 0x29 / 0x1F / 0xF from a ped that is not the player are not obeyed and, when the cop
/// (+0x10) adds it to its criminals to kill and the type is 9 or 0x29, the task changes target to it (without a
/// criminal, or when the criminal is nearer than 25 to the ped: squared distance < 625). Any other event, or none: the
/// sub-task's `MakeAbortable(ped, priority, event)`.
#[hook(addr = 0x68DAD0, conv = thiscall, name = "CTaskComplexKillCriminal::MakeAbortable", expect = "83 EC 0C 53 55 56 8B 74 24 24",
       unverified = "frontier2-8", route = "L2", hazards = "heap-writes")]
pub fn kill_criminal_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 {
    let sub_abort = || v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, prio, ev);
    if ev.is_null() {
        return sub_abort();
    }
    let ty = v_event_type(ev);
    let criminal = rd::<GPtr<u8>>(this, 0xC);
    let not_obeyed = || {
        wr::<i32>(ev, 4, rd::<i32>(ev, 4).wrapping_add(1));
        0
    };
    if v_source_entity(ev) == criminal && !criminal.is_null() {
        return if matches!(ty, 9 | 0x24 | 0x29 | 0x49 | 0x1F | 0xF) { not_obeyed() } else { sub_abort() };
    }
    if matches!(ty, 0x24 | 0x49) {
        return not_obeyed();
    }
    if !matches!(ty, 9 | 0x29 | 0x1F | 0xF) {
        return sub_abort();
    }
    'change: {
        let s = v_source_entity(ev);
        if s.is_null() || rd::<u8>(v_source_entity(ev), 0x36) & 7 != 3 || ped_is_player(v_source_entity(ev)) != 0 {
            break 'change;
        }
        let cop = rd::<GPtr<u8>>(this, 0x10);
        if cop.is_null() || cop_add_criminal_to_kill(cop, v_source_entity(ev)) == 0 || !matches!(ty, 9 | 0x29) {
            break 'change;
        }
        let c = rd::<GPtr<u8>>(this, 0xC);
        if !c.is_null() && !lt(sub_sq(entity_pos(c), entity_pos(ped)), x(sa_core::field::gf(0x85A6E8))) {
            break 'change;
        }
        change_target_call(this, v_source_entity(ev));
    }
    not_obeyed()
}

// ---------------------------------------------------------------------------------------------------------
// FRONTIER-2 turn 2 lane A (batch `frontier2-a1`): CreateNextSubTask 0x68E4F0 (crossing 4000's last block) and the
// CreateSubTask 0x68C050 it calls, from the 1.0 US bytes.

orig! {
    /// `CTaskComplexKillCriminal::CreateSubTask(int, CPed*, bool)` (0x68C050, `ret 0xC`)
    fn create_sub_task_call(this: GPtr<u8>, ty: i32, ped: GPtr<u8>, force: u8) -> GPtr<u8> = thiscall 0x68C050 => kill_criminal_create_sub_task;
}

/// `CTaskComplexKillCriminal::CreateSubTask(int type, CPed* ped, bool force)` (0x68C050, `ret 0xC`; the tail runs
/// through the island 0x40198B, which only restores the SEH link). Without `force`, a current sub-task of type `type`
/// is returned as is (one `GetTaskType` call); else the task of [`create_chosen`] (the same switch the original
/// inlines into ControlSubTask 0x68E950).
#[hook(addr = 0x68C050, conv = thiscall, name = "CTaskComplexKillCriminal::CreateSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-a1", route = "L2", hazards = "heap-writes",
       note = "allocates from the task pool; the SEH frame is not reproduced")]
pub fn kill_criminal_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>, force: u8) -> GPtr<u8> {
    if force == 0 {
        let s = sub_task(this);
        if !s.is_null() && v_task_type(s) == ty {
            return sub_task(this);
        }
    }
    create_chosen(this, ped, ty)
}

/// `CreateSubTask(ty, ped, false)` then `+0x19 = 1` (the 0x68E929 tail of the kill-on-foot case).
fn finish_with(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let t = create_sub_task_call(this, ty, ped, 0);
    wr::<u8>(this, 0x19, 1);
    t
}

/// `CTaskComplexKillCriminal::CreateNextSubTask(CPed* ped)` (0x68E4F0, `ret 4`). The cop's partner (cop +0x7A8) is read
/// before the sub-task's `GetTaskType`. By the finished sub-task's type (+0x19 finished, +0x1A cannot get in the car):
/// - 0x2BD enter as driver: not in a vehicle → +0x1A = 1, kill on foot (0x3E8) while not finished and the criminal
///   lives (health > 0), else finished (0x516). In a vehicle: `p` = no partner, a dead one or one in a vehicle; with a
///   living criminal and not finished: a criminal in its vehicle → car-drive mission (0x2D4) when `p`, else simple car
///   drive (0x2C5); otherwise kill on foot when the cop has no vehicle or the criminal is within 25 of it (squared
///   distance not above 625, NaN included), else the same drive choice. Otherwise the next criminal (any = true) is
///   targeted with a forced kill on foot when `ChangeTarget` accepts it, else the siren flag of the cop's vehicle
///   (+0x42D bit 7) is cleared (in a vehicle) and the task finishes (`p`) or drives (0x2C5).
/// - 0x2BC enter as passenger: in a vehicle → 0x2C5; else +0x1A = 1 and kill on foot / finished as above.
/// - 0x2C0 leave car: kill on foot without a vehicle or with +0x1A set; else enter as driver (0x2BD) unless the
///   criminal lives, is not finished, is not in its vehicle and is within 25 of the cop.
/// - 0x3FE drive-by: 0x2D4 for the driver cop (+0x7C0), else 0x2C5.
/// - 0x3E8 kill on foot: the living criminal or the next one; a forced kill on foot when `ChangeTarget` accepts it.
///   Else +0x19 = 1 after the new sub-task: finished (+0x1A set or no vehicle), enter as passenger (0x2BC) for a
///   passenger cop whose partner lives, else the cop becomes the driver (+0x7C0 = 1, `SetPartner(NULL)`) and enters as
///   driver (0x2BD).
/// - any other type: null.
///
/// gta-reversed differs: it tests `bInVehicle` instead of the vehicle pointer in the leave-car case and does not set
/// +0x1A in the enter-as-driver case (the bytes do).
#[hook(addr = 0x68E4F0, conv = thiscall, name = "CTaskComplexKillCriminal::CreateNextSubTask", expect = "83 EC 18 53 55 56 8B F1",
       unverified = "frontier2-a1", route = "L2", hazards = "heap-writes")]
pub fn kill_criminal_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let partner = rd::<GPtr<u8>>(cop(this), 0x7A8);
    let cs = |ty: i32| create_sub_task_call(this, ty, ped, 0);
    match v_task_type(sub_task(this)) {
        0x2BD => {
            if !in_vehicle(ped) {
                let fin = rd::<u8>(this, 0x19);
                wr::<u8>(this, 0x1A, 1);
                let c = criminal(this);
                return cs(if fin == 0 && !c.is_null() && alive_gt(c) { 0x3E8 } else { 0x516 });
            }
            let p = partner.is_null() || dead(partner) || in_vehicle(partner);
            let c = criminal(this);
            if rd::<u8>(this, 0x19) == 0 && !c.is_null() && !dead(c) {
                if !(in_vehicle(c) && !veh_of(c).is_null()) {
                    let pv = veh_of(ped);
                    if pv.is_null() {
                        return cs(0x3E8);
                    }
                    if !gt(sub_sq(entity_pos(pv), entity_pos(c)), x(g::<f32>(0x85A6E8).read())) {
                        return cs(0x3E8);
                    }
                }
                return cs(if p { 0x2D4 } else { 0x2C5 });
            }
            let n = find_next_criminal_call(this, ped, 1);
            if !n.is_null() && change_target_call(this, n) != 0 {
                return create_sub_task_call(this, 0x3E8, ped, 1);
            }
            if in_vehicle(ped) {
                let v = veh_of(ped);
                if !v.is_null() {
                    wr::<u8>(v, 0x42D, rd::<u8>(v, 0x42D) & 0x7F);
                }
            }
            cs(if p { 0x516 } else { 0x2C5 })
        }
        0x2BC => {
            if in_vehicle(ped) {
                return cs(0x2C5);
            }
            let fin = rd::<u8>(this, 0x19);
            wr::<u8>(this, 0x1A, 1);
            let c = criminal(this);
            cs(if fin == 0 && !c.is_null() && alive_gt(c) { 0x3E8 } else { 0x516 })
        }
        0x2C0 => {
            if veh_of(ped).is_null() || rd::<u8>(this, 0x1A) != 0 {
                return cs(0x3E8);
            }
            let c = criminal(this);
            if rd::<u8>(this, 0x19) == 0
                && !c.is_null()
                && !dead(c)
                && !(in_vehicle(c) && !veh_of(c).is_null())
                && !gt(sub_sq(entity_pos(ped), entity_pos(c)), x(g::<f32>(0x85A6E8).read()))
            {
                return cs(0x3E8);
            }
            cs(0x2BD)
        }
        0x3FE => cs(if rd::<u8>(cop(this), 0x7C0) != 0 { 0x2D4 } else { 0x2C5 }),
        0x3E8 => {
            let c = criminal(this);
            let n = if !c.is_null() && alive_gt(c) { c } else { find_next_criminal_call(this, ped, 1) };
            if !n.is_null() && change_target_call(this, n) != 0 {
                return create_sub_task_call(this, 0x3E8, ped, 1);
            }
            if rd::<u8>(this, 0x1A) != 0 || veh_of(ped).is_null() {
                return finish_with(this, 0x516, ped);
            }
            let k = cop(this);
            if rd::<u8>(k, 0x7C0) == 0 {
                if !partner.is_null() && !dead(partner) {
                    return finish_with(this, 0x2BC, ped);
                }
                wr::<u8>(k, 0x7C0, 1);
                cop_set_partner(cop(this), GPtr::NULL);
            }
            finish_with(this, 0x2BD, ped)
        }
        _ => GPtr::NULL,
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_KILL_CRIMINAL_CREATE_FIRST_SUB_TASK,
    HOOK_KILL_CRIMINAL_FIND_NEXT_CRIMINAL_TO_KILL,
    HOOK_KILL_CRIMINAL_CHANGE_TARGET,
    HOOK_KILL_CRIMINAL_CONTROL_SUB_TASK,
    HOOK_KILL_CRIMINAL_MAKE_ABORTABLE,
    HOOK_KILL_CRIMINAL_CREATE_SUB_TASK,
    HOOK_KILL_CRIMINAL_CREATE_NEXT_SUB_TASK,
];
