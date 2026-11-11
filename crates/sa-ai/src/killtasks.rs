//! `CTaskComplexKillCriminal::CreateFirstSubTask` (PORT-AI-5, helper B). Ported from the 1.0 US bytes (the exe wins
//! over `docs/re/specs/AI/CTaskComplexKillCriminal.md`; gta-reversed does not reverse it). Every call to another
//! catalogued function goes through `orig!` at the original address; tasks come from the original task pool.
//!
//! `CTaskComplexKillCriminal`: +8 sub-task, +0xC criminal, +0x10 cop (registered reference), +0x18 randomize,
//! +0x1B / +0x1C / +0x1D saved driving style / mission / cruise speed, +0x1E set up. `CCopPed`: +0x7A8 partner,
//! +0x7C0 is-the-driver. `CVehicle`: +0x3B9 / +0x3BA / +0x3D0 autopilot bytes, +0x460 driver, +0x464 passengers,
//! +0x484 passenger count, +0x488 maximum passengers, +0x590 vehicle type.
#![allow(clippy::unreadable_literal)]

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

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn world_find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn general_random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CCopPed::AddCriminalToKill(CPed*)` (0x5DDEB0, `ret 4`)
    fn cop_add_criminal_to_kill(cop: GPtr<u8>, c: GPtr<u8>) = thiscall 0x5DDEB0 => unported;
    /// `CCopPed::SetPartner(CCopPed*)` (0x5DDE80, `ret 4`)
    fn cop_set_partner(cop: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5DDE80 => unported;
    /// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50, `ret 4`)
    fn intel_set_ped_decision_maker_type(i: GPtr<u8>, t: i32) = thiscall 0x600B50 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, t: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CEventAcquaintancePedHate::Constructor2(CPed*)` (0x420E70, `ret 4`)
    fn event_ped_hate_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x420E70 => unported;
    /// `FUN_004af890` (0x4AF890): the event base destructor the original calls after resetting the vtable
    fn event_base_dtor(e: GPtr<u8>) = thiscall 0x4AF890 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CTaskComplexLeaveCar` constructor `(CVehicle*, int32, int32, bool, bool)` (0x63B8C0, `ret 0x14`)
    fn leave_car_ctor(t: GPtr<u8>, v: GPtr<u8>, a: i32, b: i32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskComplexCarDriveMission::Constructor(CVehicle*, CEntity*, int32 mission, int32 style, float speed)`
    /// (0x63CC30, `ret 0x14`)
    fn car_drive_mission_ctor(t: GPtr<u8>, v: GPtr<u8>, e: GPtr<u8>, mission: i32, style: i32, speed: u32) -> GPtr<u8> = thiscall 0x63CC30 => unported;
    /// `CTaskSimpleCarDrive::Constructor(CVehicle*, CTaskUtilityLineUpPedWithCar*, bool)` (0x63C340, `ret 0xC`)
    fn simple_car_drive_ctor(t: GPtr<u8>, v: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x63C340 => unported;
    /// `CTaskComplexKillPedOnFoot::Constructor(CPed*, int32, uint32, uint32, int32, bool)` (0x620E30, `ret 0x18`)
    fn kill_ped_on_foot_ctor(t: GPtr<u8>, p: GPtr<u8>, a: i32, b: u32, c: u32, d: i32, e: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    /// `CPed::SetCurrentWeapon(eWeaponType)` (0x5E6280, `ret 4`)
    fn ped_set_current_weapon(p: GPtr<u8>, w: i32) = thiscall 0x5E6280 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool)` (0x5E6080, `ret 0xC`)
    fn ped_give_weapon(p: GPtr<u8>, w: i32, ammo: u32, b: u32) -> u32 = thiscall 0x5E6080 => unported;
    /// `CInterestingEvents::Add(int32 type, CEntity*)` (0x602590, `ret 8`)
    fn interesting_events_add(ie: GPtr<u8>, ty: i32, e: GPtr<u8>) = thiscall 0x602590 => unported;
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

fn in_vehicle(p: GPtr<u8>) -> bool {
    rd::<u32>(p, 0x46C) & 0x100 != 0
}

/// A stack `CEventAcquaintancePedHate(about)` with task id `task` (+0xE) added to `target`'s event group
/// (intelligence +0x68), then destroyed (vtable 0x858E68, base destructor 0x4AF890). The original's slot is 0x18
/// bytes; the Rust buffer is 0x20.
fn send_hate(about: GPtr<u8>, target_intel: impl FnOnce() -> GPtr<u8>, task: u16) {
    let mut ev = [0u32; 8];
    let e: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 8]).cast();
    event_ped_hate_ctor(e, about);
    wr::<u16>(e, 0xE, task);
    let intel = target_intel();
    event_group_add(intel.byte_add(0x68), e, 0);
    wr::<u32>(e, 0, EVENT_BASE_VTBL);
    event_base_dtor(e);
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
                mission,
                2,
                f32_of(speed).to_bits(),
            )
        } else {
            // 0x68E17D (unreachable: the criminal was checked to be in a vehicle above)
            let mem = task_new(0x2C);
            if mem.is_null() {
                return GPtr::NULL;
            }
            car_drive_mission_ctor(
                mem,
                rd::<GPtr<u8>>(ped, 0x58C),
                criminal(this),
                0x37,
                2,
                0x41A0_0000,
            )
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
        simple_car_drive_ctor(mem, rd::<GPtr<u8>>(ped, 0x58C), 0, 0)
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

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_KILL_CRIMINAL_CREATE_FIRST_SUB_TASK];
