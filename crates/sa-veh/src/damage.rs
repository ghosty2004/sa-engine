//! `CDamageManager` (0x18 bytes: +0 wheel damage effect f32, +4 engine status, +5 4 wheel statuses,
//! +9 6 door statuses, +0xF unused, +0x10 lights (4 x 2 bits), +0x14 panels (7 x 4 bits; the aeroplane
//! components 12.. reuse the panel word with 2-bit fields)). Ported from the 1.0 US bytes; gta-reversed
//! `DamageManager.cpp` as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, rand_at, rand_frac, rd, wr};

/// `CDamageManager` thiscall `this`.
type Dm = GPtr<u8>;

orig! {
    /// `CDoor::Open(float)` (0x6F4790)
    pub fn door_open(door: GPtr<u8>, ratio: f32) = thiscall 0x6F4790 => unported;
    /// `CDamageManager::GetComponentGroup(tComponent, tComponentGroup*, uint8*)` (0x6C2040)
    pub fn get_component_group_call(dm: GPtr<u8>, comp: i32, group: GPtr<i32>, sub: GPtr<u8>) -> u8 = thiscall 0x6C2040 => get_component_group;
    /// `CDamageManager::ProgressWheelDamage(uint8)` (0x6C2440)
    pub fn progress_wheel_damage_call(dm: GPtr<u8>, wheel: u32) -> u8 = thiscall 0x6C2440 => progress_wheel_damage;
    /// `CDamageManager::ProgressDoorDamage(uint8, CAutomobile*)` (0x6C2320)
    pub fn progress_door_damage_call(dm: GPtr<u8>, door: u32, car: GPtr<u8>) -> u8 = thiscall 0x6C2320 => progress_door_damage;
    /// `CDamageManager::ProgressPanelDamage(uint8)` (0x6C23C0)
    pub fn progress_panel_damage_call(dm: GPtr<u8>, panel: u32) -> u8 = thiscall 0x6C23C0 => progress_panel_damage;
    /// `CDamageManager::SetLightStatus(eLights, uint32)` (0x6C2100)
    pub fn set_light_status_call(dm: GPtr<u8>, light: u32, status: u32) = thiscall 0x6C2100 => set_light_status;
    /// `CDamageManager::ResetDamageStatus()` (0x6C20E0)
    pub fn reset_damage_status_call(dm: GPtr<u8>) = thiscall 0x6C20E0 => reset_damage_status;
    /// `CAutomobile::SetPanelDamage(int32, bool)` (0x6B1350)
    pub fn car_set_panel_damage(car: GPtr<u8>, panel: u32, no_fly: u32) = thiscall 0x6B1350 => unported;
    /// `CAutomobile::SetBumperDamage(int32, bool)` (0x6B1480)
    pub fn car_set_bumper_damage(car: GPtr<u8>, panel: u32, no_fly: u32) = thiscall 0x6B1480 => unported;
    /// `CAutomobile::SetDoorDamage(int32, bool)` (0x6B1600)
    pub fn car_set_door_damage(car: GPtr<u8>, door: u32, no_fly: u32) = thiscall 0x6B1600 => unported;
}

/// `CDamageManager::Init()` (0x6A0520): `rep stosd` of 6 zero dwords; returns `this`.
#[hook(addr = 0x6A0520, conv = thiscall, name = "CDamageManager::Init", expect = "8B D1 57 33 C0", this = "CDamageManager")]
pub fn init(dm: Dm) -> GPtr<u8> {
    for k in 0..6 {
        wr::<u32>(dm, 4 * k, 0);
    }
    dm
}

/// `CDamageManager::ResetDamageStatus()` (0x6C20E0): engine, wheels, doors, lights and panels to 0
/// (the wheel damage effect at +0 and the byte at +0xF stay).
#[hook(addr = 0x6C20E0, conv = thiscall, name = "CDamageManager::ResetDamageStatus", expect = "33 C0 88 41 04", this = "CDamageManager")]
pub fn reset_damage_status(dm: Dm) {
    wr::<u8>(dm, 4, 0);
    wr::<u32>(dm, 5, 0);
    wr::<u32>(dm, 9, 0);
    wr::<u16>(dm, 0xD, 0);
    wr::<u32>(dm, 0x10, 0);
    wr::<u32>(dm, 0x14, 0);
}

/// `CDamageManager::ResetDamageStatusAndWheelDamage()` (0x6A04E0): ResetDamageStatus, then the wheel
/// damage effect 0.5; returns `this`.
#[hook(addr = 0x6A04E0, conv = thiscall, name = "CDamageManager::ResetDamageStatusAndWheelDamage", expect = "56 8B F1 E8 F8 1B 02 00", this = "CDamageManager")]
pub fn reset_damage_status_and_wheel_damage(dm: Dm) -> GPtr<u8> {
    reset_damage_status_call(dm);
    wr::<u32>(dm, 0, 0x3F00_0000);
    dm
}

/// `CDamageManager::GetEngineStatus()` (0x6C22C0).
#[hook(addr = 0x6C22C0, conv = thiscall, name = "CDamageManager::GetEngineStatus", expect = "0F B6 41 04 C3", this = "CDamageManager")]
pub fn get_engine_status(dm: Dm) -> u32 {
    u32::from(rd::<u8>(dm, 4))
}

/// `CDamageManager::SetEngineStatus(uint32)` (0x6C22A0): clamped to 250 (unsigned compare).
#[hook(addr = 0x6C22A0, conv = thiscall, name = "CDamageManager::SetEngineStatus", expect = "8B 44 24 04 3D FA 00 00 00", this = "CDamageManager")]
pub fn set_engine_status(dm: Dm, status: u32) {
    wr::<u8>(dm, 4, if status > 0xFA { 0xFA } else { status as u8 });
}

/// `CDamageManager::ProgressEngineDamage(float)` (0x6C23B0): `xor al, al` (EAX's upper bytes are the
/// caller's; callers test AL).
#[hook(addr = 0x6C23B0, conv = thiscall, name = "CDamageManager::ProgressEngineDamage", expect = "32 C0 C2 04 00", this = "CDamageManager")]
pub fn progress_engine_damage(_dm: Dm, _unused: u32) -> u8 {
    0
}

/// `CDamageManager::GetWheelStatus(eCarWheel)` (0x6C21B0): no range check.
#[hook(addr = 0x6C21B0, conv = thiscall, name = "CDamageManager::GetWheelStatus", expect = "8B 44 24 04 0F B6 44 08 05", this = "CDamageManager")]
pub fn get_wheel_status(dm: Dm, wheel: i32) -> u32 {
    u32::from(rd::<u8>(dm, wheel as isize + 5))
}

/// `CDamageManager::SetWheelStatus(eCarWheel, eCarWheelStatus)` (0x6C21A0): no range check.
#[hook(addr = 0x6C21A0, conv = thiscall, name = "CDamageManager::SetWheelStatus", expect = "8A 44 24 08 8B 54 24 04", this = "CDamageManager")]
pub fn set_wheel_status(dm: Dm, wheel: i32, status: u32) {
    wr::<u8>(dm, wheel as isize + 5, status as u8);
}

/// `CDamageManager::ProgressWheelDamage(uint8)` (0x6C2440): status 2 stays (false), else +1 (true).
#[hook(addr = 0x6C2440, conv = thiscall, name = "CDamageManager::ProgressWheelDamage", expect = "0F B6 44 24 04", this = "CDamageManager")]
pub fn progress_wheel_damage(dm: Dm, wheel: u32) -> u8 {
    let off = (wheel & 0xFF) as isize + 5;
    let s = rd::<u8>(dm, off);
    if s == 2 {
        return 0;
    }
    wr::<u8>(dm, off, s.wrapping_add(1));
    1
}

/// `CDamageManager::GetDoorStatus(eDoors)` (0x6C2230, tail at 0x401860): doors 0..5, else 4 (missing).
#[hook(addr = 0x6C2230, conv = thiscall, name = "CDamageManager::GetDoorStatus", expect = "8B 44 24 04 85 C0", this = "CDamageManager")]
pub fn get_door_status(dm: Dm, door: i32) -> u32 {
    if (0..6).contains(&door) {
        u32::from(rd::<u8>(dm, door as isize + 9))
    } else {
        4
    }
}

/// `CDamageManager::SetDoorStatus(eDoors, eDoorStatus)` (0x6C21C0): doors 0..5 only.
#[hook(addr = 0x6C21C0, conv = thiscall, name = "CDamageManager::SetDoorStatus-", expect = "8B 44 24 04 85 C0", this = "CDamageManager")]
pub fn set_door_status(dm: Dm, door: i32, status: u32) {
    if (0..6).contains(&door) {
        wr::<u8>(dm, door as isize + 9, status as u8);
    }
}

/// Byte offset of the door status a car-node component 8..11 maps to (`GetDoorStatus_Component` /
/// `SetDoorStatus_Component` jump tables 0x6C2288 / 0x6C221C: 8 -> +0xC, 9 -> +0xE, 10 -> +0xB, 11 -> +0xD).
fn door_component_offset(comp: i32) -> Option<isize> {
    match comp.wrapping_sub(8) {
        0 => Some(0xC),
        1 => Some(0xE),
        2 => Some(0xB),
        3 => Some(0xD),
        _ => None,
    }
}

/// `CDamageManager::GetDoorStatus_Component(tComponent)` (0x6C2250): components 8..11, else 4.
#[hook(addr = 0x6C2250, conv = thiscall, name = "CDamageManager::GetDoorStatus_Component", expect = "8B 44 24 04 83 C0 F8", this = "CDamageManager")]
pub fn get_door_status_component(dm: Dm, comp: i32) -> u32 {
    door_component_offset(comp).map_or(4, |o| u32::from(rd::<u8>(dm, o)))
}

/// `CDamageManager::SetDoorStatus_Component(tComponent, eDoorStatus)` (0x6C21E0): components 8..11.
#[hook(addr = 0x6C21E0, conv = thiscall, name = "CDamageManager::SetDoorStatus_Component", expect = "8B 44 24 04 83 C0 F8", this = "CDamageManager")]
pub fn set_door_status_component(dm: Dm, comp: i32, status: u32) {
    if let Some(o) = door_component_offset(comp) {
        wr::<u8>(dm, o, status as u8);
    }
}

/// `CDamageManager::GetLightStatus(eLights)` (0x6C2130): `(lights >> (2 * light as u8)) & 3`.
#[hook(addr = 0x6C2130, conv = thiscall, name = "CDamageManager::GetLightStatus", expect = "8B C1 8B 40 10", this = "CDamageManager")]
pub fn get_light_status(dm: Dm, light: u32) -> u32 {
    let cl = (light as u8).wrapping_shl(1);
    rd::<u32>(dm, 0x10).wrapping_shr(u32::from(cl)) & 3
}

/// `CDamageManager::SetLightStatus(eLights, uint32)` (0x6C2100).
#[hook(addr = 0x6C2100, conv = thiscall, name = "CDamageManager::SetLightStatus", expect = "8A 44 24 04 D0 E0", this = "CDamageManager")]
pub fn set_light_status(dm: Dm, light: u32, status: u32) {
    let cl = u32::from((light as u8).wrapping_shl(1));
    let v = (!3u32.wrapping_shl(cl) & rd::<u32>(dm, 0x10)) | status.wrapping_shl(cl);
    wr::<u32>(dm, 0x10, v);
}

/// `CDamageManager::GetPanelStatus(ePanels)` (0x6C2180): `(panels >> (4 * panel as u8)) & 0xF`.
#[hook(addr = 0x6C2180, conv = thiscall, name = "CDamageManager::GetPanelStatus", expect = "8B C1 8B 40 14", this = "CDamageManager")]
pub fn get_panel_status(dm: Dm, panel: u32) -> u32 {
    let cl = (panel as u8).wrapping_shl(2);
    rd::<u32>(dm, 0x14).wrapping_shr(u32::from(cl)) & 0xF
}

/// `CDamageManager::SetPanelStatus(ePanels, uint32)` (0x6C2150).
#[hook(addr = 0x6C2150, conv = thiscall, name = "CDamageManager::SetPanelStatus", expect = "8A 44 24 04 C0 E0 02", this = "CDamageManager")]
pub fn set_panel_status(dm: Dm, panel: u32, status: u32) {
    let cl = u32::from((panel as u8).wrapping_shl(2));
    let v = (!0xFu32.wrapping_shl(cl) & rd::<u32>(dm, 0x14)) | status.wrapping_shl(cl);
    wr::<u32>(dm, 0x14, v);
}

/// Shift count of an aeroplane component (`(comp as u8 - 12) * 2`, 8-bit).
fn aero_shift(comp: u32) -> u8 {
    (comp as u8).wrapping_sub(0xC).wrapping_shl(1)
}

/// `CDamageManager::GetAeroplaneCompStatus(uint8)` (0x6C2300).
#[hook(addr = 0x6C2300, conv = thiscall, name = "CDamageManager::GetAeroplaneCompStatus", expect = "8B C1 8B 40 14", this = "CDamageManager")]
pub fn get_aeroplane_comp_status(dm: Dm, comp: u32) -> u32 {
    rd::<u32>(dm, 0x14).wrapping_shr(u32::from(aero_shift(comp))) & 3
}

/// `CDamageManager::SetAeroplaneCompStatus(uint8, uint32)` (0x6C22D0).
#[hook(addr = 0x6C22D0, conv = thiscall, name = "CDamageManager::SetAeroplaneCompStatus", expect = "8A 44 24 04 2C 0C", this = "CDamageManager")]
pub fn set_aeroplane_comp_status(dm: Dm, comp: u32, status: u32) {
    let cl = u32::from(aero_shift(comp));
    let v = (!3u32.wrapping_shl(cl) & rd::<u32>(dm, 0x14)) | status.wrapping_shl(cl);
    wr::<u32>(dm, 0x14, v);
}

/// `CDamageManager::ProgressAeroplaneDamage(uint8)` (0x6C2460): status 2 stays (false), else +1.
/// The false path leaves EAX = the shifted panel word with AL = 0 (callers test AL).
#[hook(addr = 0x6C2460, conv = thiscall, name = "CDamageManager::ProgressAeroplaneDamage", expect = "8B D1 8A 4C 24 04", this = "CDamageManager")]
pub fn progress_aeroplane_damage(dm: Dm, comp: u32) -> u8 {
    let cl = u32::from(aero_shift(comp));
    let s = (rd::<u32>(dm, 0x14).wrapping_shr(cl) as u8) & 3;
    if s == 2 {
        return 0;
    }
    let v = (!3u32.wrapping_shl(cl) & rd::<u32>(dm, 0x14)) | u32::from(s + 1).wrapping_shl(cl);
    wr::<u32>(dm, 0x14, v);
    1
}

/// `CDamageManager::ProgressPanelDamage(uint8)` (0x6C23C0): status 3 stays; status 2 advances only when
/// `rand() & 7 == 0` (panel 4, the windscreen: `rand() & 1 == 0`); else +1.
#[hook(addr = 0x6C23C0, conv = thiscall, name = "CDamageManager::ProgressPanelDamage", expect = "8A 44 24 04 53", this = "CDamageManager")]
pub fn progress_panel_damage(dm: Dm, panel: u32) -> u8 {
    let p = panel as u8;
    let cl = u32::from(p.wrapping_shl(2));
    let s = (rd::<u32>(dm, 0x14).wrapping_shr(cl) as u8) & 0xF;
    if s == 2 {
        if p == 4 {
            if rand_at(0x6C23E7) & 1 != 0 {
                return 0;
            }
        } else if rand_at(0x6C23F7) & 7 != 0 {
            return 0;
        }
    } else if s == 3 {
        return 0;
    }
    let v = (!0xFu32.wrapping_shl(cl) & rd::<u32>(dm, 0x14)) | u32::from(s.wrapping_add(1)).wrapping_shl(cl);
    wr::<u32>(dm, 0x14, v);
    1
}

/// `CDamageManager::ProgressDoorDamage(uint8, CAutomobile*)` (0x6C2320): doors 0..5 not missing (4):
/// status 0/1 -> open the door (`CDoor::Open(0.0)` of the car's door +0x5B8 + 0x18 * door) and set 2;
/// 2 -> 3; 3 -> 4 only when `rand() & 7 == 0`; above 4 the status is rewritten unchanged (true).
#[hook(addr = 0x6C2320, conv = thiscall, name = "CDamageManager::ProgressDoorDamage", expect = "53 56 0F B6 74 24 0C", this = "CDamageManager")]
pub fn progress_door_damage(dm: Dm, door: u32, car: GPtr<u8>) -> u8 {
    let d = door & 0xFF;
    if d >= 6 {
        return 0;
    }
    let off = d as isize + 9;
    let s = rd::<u8>(dm, off);
    let next = match s {
        4 => return 0,
        0 | 1 => {
            door_open(car.byte_add(((d * 3 + 0xB7) * 8) as isize), 0.0);
            2
        }
        2 => 3,
        3 => {
            if rand_at(0x6C2381) & 7 != 0 {
                return 0;
            }
            4
        }
        s => s,
    };
    wr::<u8>(dm, off, next);
    1
}

/// `CDamageManager::GetComponentGroup(tComponent, tComponentGroup*, uint8*)` (0x6C2040): `*sub = 0xFE`
/// first; 11.. -> (comp >= 16 ? 0 : 5, comp - 11); 5 -> (3, 0); 6..10 -> (comp == 6 ? 4 : 2, comp - 5);
/// 1..4 -> (1, comp - 1); 0 -> (6, 0); negative -> false.
#[hook(addr = 0x6C2040, conv = thiscall, name = "CDamageManager::GetComponentGroup", expect = "8B 44 24 04 83 F8 0B", this = "CDamageManager")]
pub fn get_component_group(_dm: Dm, comp: i32, group: GPtr<i32>, sub: GPtr<u8>) -> u8 {
    sub.write(0xFE);
    let (g, s) = if comp >= 0xB {
        (if comp >= 0x10 { 0 } else { 5 }, (comp as u8).wrapping_sub(0xB))
    } else if comp == 5 {
        (3, 0)
    } else if comp > 5 {
        (if comp == 6 { 4 } else { 2 }, (comp as u8).wrapping_sub(5))
    } else if comp >= 1 {
        (1, (comp as u8).wrapping_sub(1))
    } else if comp == 0 {
        (6, 0)
    } else {
        return 0;
    };
    group.write(g);
    sub.write(s);
    1
}

/// `CDamageManager::FuckCarCompletely(bool skipWheels)` (0x6C25D0). Without `skipWheels` one random
/// wheel status byte is set to 2 at `this + 5 - n` with n = ftol(rand fraction * -3.0 (0x859024)) =
/// -5..0, i.e. one of +5..+10: the 4 wheels or doors 0/1 (which the next store overwrites). Doors all 4 (missing); panels 5 and 6 are
/// progressed three times (the rand calls happen), then lights and panels are cleared and the engine
/// status set to 250.
#[hook(addr = 0x6C25D0, conv = thiscall, name = "CDamageManager::FuckCarCompletely", expect = "8A 44 24 04 84 C0", this = "CDamageManager")]
pub fn fuck_car_completely(dm: Dm, skip_wheels: u32) {
    if skip_wheels as u8 == 0 {
        let r = rand_at(0x6C25E2);
        let n = ftol32(rand_frac(r).fmul_m(crate::common::g::<f32>(0x859024).read()));
        wr::<u8>(dm, 5isize.wrapping_sub(n as isize), 2);
    }
    wr::<u32>(dm, 9, 0x0404_0404);
    wr::<u16>(dm, 0xD, 0x0404);
    for _ in 0..3 {
        let s = (rd::<u32>(dm, 0x14) >> 0x14) as u8 & 0xF;
        if !(s == 2 && rand_at(0x6C2633) & 7 != 0) && s != 3 {
            let v = (u32::from(s.wrapping_add(1)) << 0x14) | (rd::<u32>(dm, 0x14) & 0xFF0F_FFFF);
            wr::<u32>(dm, 0x14, v);
        }
        let s = rd::<u8>(dm, 0x17) & 0xF;
        if !(s == 2 && rand_at(0x6C2664) & 7 != 0) && s != 3 {
            let v = (u32::from(s.wrapping_add(1)) << 0x18) | (rd::<u32>(dm, 0x14) & 0xF0FF_FFFF);
            wr::<u32>(dm, 0x14, v);
        }
    }
    wr::<u32>(dm, 0x10, 0);
    wr::<u32>(dm, 0x14, 0);
    wr::<u8>(dm, 4, 0xFA);
}

/// `CDamageManager::ApplyDamage(CAutomobile*, tComponent, float intensity, float factor)` (0x6C24B0).
/// Only for cars whose `+0x868` flags have bit 4 (damage allowed). The component's group and
/// sub-index come from GetComponentGroup, which writes them into the stack slots of `car` (the
/// sub-index byte, so the dword passed on keeps the pointer's upper bytes) and `comp` (the group).
/// `intensity * groupMultiplier[group] (0x8D32A0)` (times 0.6 for component 15) must exceed 150.
#[hook(addr = 0x6C24B0, conv = thiscall, name = "CDamageManager::ApplyDamage", expect = "55 8B 6C 24 08", this = "CDamageManager")]
pub fn apply_damage(dm: Dm, car: GPtr<u8>, comp: i32, intensity: f32, _factor: f32) -> u8 {
    use crate::common::{jne_mask, x};
    if rd::<u8>(car, 0x868) & 0x10 == 0 {
        return 0;
    }
    // the two stack slots GetComponentGroup writes: arg0 (sub byte) and arg1 (group dword)
    let mut slots: [u32; 2] = [car.addr() as u32, comp as u32];
    let tmp = GPtr::from_raw(&raw mut slots);
    get_component_group_call(dm, comp, tmp.cast::<u8>().byte_add(4).cast(), tmp.cast());
    let [sub_slot, group_slot] = tmp.read();
    let group = group_slot as i32;
    let mul = sa_world::world::common::g::<f32>(0x8D32A0)
        .wrapping_index(group)
        .read();
    let mut d = x(intensity).fmul_m(mul);
    if comp == 0xF {
        d = d.fmul_m(f32::from_bits(0x3F19_999A));
    }
    if jne_mask(d, x(150.0), 0x41) {
        return 0;
    }
    let sub = sub_slot as u8;
    match group_slot {
        0 | 5 => {
            if group_slot == 5 {
                set_light_status_call(dm, u32::from(sub), 1);
            }
            if progress_panel_damage_call(dm, sub_slot) != 0 {
                if group == 0 {
                    car_set_panel_damage(car, u32::from(sub), 0);
                } else {
                    car_set_bumper_damage(car, u32::from(sub), 0);
                }
            }
        }
        1 => {
            progress_wheel_damage_call(dm, sub_slot);
        }
        2..=4 if progress_door_damage_call(dm, sub_slot, car) != 0 => {
            car_set_door_damage(car, u32::from(sub), 0);
        }
        _ => {}
    }
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT,
    HOOK_RESET_DAMAGE_STATUS,
    HOOK_RESET_DAMAGE_STATUS_AND_WHEEL_DAMAGE,
    HOOK_GET_ENGINE_STATUS,
    HOOK_SET_ENGINE_STATUS,
    HOOK_PROGRESS_ENGINE_DAMAGE,
    HOOK_GET_WHEEL_STATUS,
    HOOK_SET_WHEEL_STATUS,
    HOOK_PROGRESS_WHEEL_DAMAGE,
    HOOK_GET_DOOR_STATUS,
    HOOK_SET_DOOR_STATUS,
    HOOK_GET_DOOR_STATUS_COMPONENT,
    HOOK_SET_DOOR_STATUS_COMPONENT,
    HOOK_GET_LIGHT_STATUS,
    HOOK_SET_LIGHT_STATUS,
    HOOK_GET_PANEL_STATUS,
    HOOK_SET_PANEL_STATUS,
    HOOK_GET_AEROPLANE_COMP_STATUS,
    HOOK_SET_AEROPLANE_COMP_STATUS,
    HOOK_PROGRESS_AEROPLANE_DAMAGE,
    HOOK_PROGRESS_PANEL_DAMAGE,
    HOOK_PROGRESS_DOOR_DAMAGE,
    HOOK_GET_COMPONENT_GROUP,
    HOOK_FUCK_CAR_COMPLETELY,
    HOOK_APPLY_DAMAGE,
];
