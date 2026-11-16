//! Replay vehicle playback (FRONTIER-2 lane C turn 6): `tReplayVehicleBlock::ExtractVehicleUpdateData` (0x45BA90) and
//! `ApplyPanelDamageToCar` (0x59CA10). Ported from the 1.0 US bytes; gta-reversed is a reading aid (it inlines the
//! panel function, reads the speed / steer / wheel bytes unsigned and calls `SetAreaCode`; the bytes win).
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::car_set_panel_damage as set_bumper_damage;
use crate::common::{g, rd, wr, x, xi};
use sa_core::bindings::{decompress_into_full_matrix, matrix_assign};
use sa_world::bindings::{
    automobile_set_panel_damage, damage_get_door_status, dm_set_door_status, matrix_copy_ctor, matrix_dtor,
    set_door_damage, world_add, world_remove,
};

orig! {
    /// `CMatrix::ScaleAll(float)` (0x45AF40, `ret 4`; its loop counter set in the fragment 0x401BA5)
    fn matrix_scale_all(m: GPtr<u8>, f: u32) = thiscall 0x45AF40 => unported;
    /// `CMatrix::operator+=(const CMatrix&)` (0x59ADF0)
    fn matrix_add_assign(m: GPtr<u8>, rhs: GPtr<u8>) -> GPtr<u8> = thiscall 0x59ADF0 => unported;
    /// `ApplyPanelDamageToCar(uint32 panels, CAutomobile*, bool)` (0x59CA10)
    fn apply_panel_damage_to_car_call(panels: u32, car: GPtr<u8>, flag: u32) = cdecl 0x59CA10 => apply_panel_damage_to_car;
}

/// `CGame::currArea` (0xB72914).
const CURR_AREA: usize = 0xB72914;

/// `ApplyPanelDamageToCar(uint32 panels, CAutomobile* car, bool flag)` (0x59CA10, cdecl): for each of the seven 4-bit
/// panel states of the damage manager's panel word (+0x5B4) that differs from `panels`, the nibble is copied; panels 0,
/// 1 and 4 then call `SetPanelDamage(i, flag)` (0x6B1480), panels 5 and 6 `SetBumperDamage(i, flag)` (0x6B1350),
/// panels 2 and 3 nothing.
#[hook(addr = 0x59CA10, conv = cdecl, name = "ApplyPanelDamageToCar", expect = "53 8B 5C 24 10 56",
       unverified = "frontier2-c12", route = "L2", hazards = "heap-writes")]
pub fn apply_panel_damage_to_car(panels: u32, car: GPtr<u8>, flag: u32) {
    for i in 0..7u32 {
        let mask = 0xFu32 << (4 * i);
        let cur = rd::<u32>(car, 0x5B4);
        if (cur ^ panels) & mask == 0 {
            continue;
        }
        wr::<u32>(car, 0x5B4, ((cur ^ panels) & mask) ^ cur);
        match i {
            0 | 1 | 4 => automobile_set_panel_damage(car, i, flag),
            5 | 6 => set_bumper_damage(car, i, flag),
            _ => {}
        }
    }
}

/// `int8 * c` as `fild m32; fmul m32; fstp m32` (the byte sign-extended into a stack dword).
fn scaled(b: i8, c: usize) -> u32 {
    xi(i32::from(b)).fmul_m(g::<f32>(c).read()).to_f32().to_bits()
}

/// `tReplayVehicleBlock::ExtractVehicleUpdateData(CVehicle* veh, float interpolation)` (0x45BA90, thiscall, `ret 8`;
/// the entry jumps through the protection fragment 0x401D62 that builds the SEH frame). Nothing for a null vehicle.
/// Else the matrix becomes `veh * (1 - t) + decompressed(block +4) * t` (two stack `CMatrix`: `DecompressIntoFullMatrix`,
/// the copy constructor, `ScaleAll` ×2, `operator+=`, `operator=` into `veh->m_matrix`, both destroyed at the end);
/// turn speed (+0x50) = 0; health (+0x4C0) = `byte +2 * 4`; gas pedal (+0x49C) = `byte +3 * [0x858C58]`; an automobile
/// (+0x590 == 0) gets `ApplyPanelDamageToCar(block +0x1C, veh, 1)`; move speed (+0x44) = `int8 +0x20..+0x22 *
/// [0x859C48]`; flags +0x1C take bit 7 from bit 1 of the flags byte +0x2F, bit 30 from its bit 2 and bit 31 from its
/// bit 3; the Rhino (model 0x1B0) zeroes the steer angle +0x494 and sets the doom rotation +0x94C = `int8 +0x23 *
/// [0x859C44]`, others steer = `int8 +0x23 * [0x858B38]`. An automobile then gets the suspension compressions +0x7D4
/// (`int8 +0x24.. * [0x858B38]`), wheel rotations +0x828 (`int8 +0x28.. * [0x859C44]`), the front door angles +0x5F4 /
/// +0x60C (`int8 +0x18 / +0x19 * [0x859C40]`, copied to the previous angles +0x5F8 / +0x610; a non-zero byte sets the
/// door's open flag +0x5AB / +0x5AC) and, for each of the six bits of +0x2C whose door status (`GetDoorStatus`) is not
/// 4, `SetDoorStatus(i, 4)` and `SetDoorDamage(i, 1)`. Every vehicle: engine-on bit 4 of +0x428 = not engine-broken
/// (bit 1 of +0x42D); an automobile's +0x961 = 4; area (+0x2F) = `CGame::currArea`; `CWorld::Remove` and `Add`; bit 29
/// of the physical flags +0x40 from bit 0 of +0x2F.
#[hook(addr = 0x45BA90, conv = thiscall, name = "tReplayVehicleBlock::ExtractVehicleUpdateData",
       expect = "90 E9 CC 62 FA FF", unverified = "frontier2-c12", route = "L2",
       hazards = "heap-writes,uninit-locals,entry-protection-read",
       note = "the two stack CMatrix keep uninitialised padding words; the entry bytes are read by protection code")]
pub fn extract_vehicle_update_data(this: GPtr<u8>, veh: GPtr<u8>, t: u32) {
    if veh.is_null() {
        return;
    }
    let mut ma = [0u32; 0x48 / 4];
    let mut mb = [0u32; 0x48 / 4];
    let a = GPtr::from_raw(ma.as_mut_ptr().cast::<u8>());
    let b = GPtr::from_raw(mb.as_mut_ptr().cast::<u8>());
    wr::<u32>(a, 0x40, 0);
    wr::<u32>(a, 0x44, 0);
    decompress_into_full_matrix(this.byte_add(4), a);
    matrix_copy_ctor(b, rd::<GPtr<u8>>(veh, 0x14));
    matrix_scale_all(b, x(g::<f32>(0x858624).read()).fsub_m(f32::from_bits(t)).to_f32().to_bits());
    matrix_scale_all(a, t);
    matrix_add_assign(b, a);
    matrix_assign(rd::<GPtr<u8>>(veh, 0x14), b);
    wr::<u32>(veh, 0x50, 0);
    wr::<u32>(veh, 0x54, 0);
    wr::<u32>(veh, 0x58, 0);
    let health = i32::from(rd::<u8>(this, 2)) << 2;
    wr::<u32>(veh, 0x4C0, xi(health).to_f32().to_bits());
    let gas = i32::from(rd::<u8>(this, 3));
    wr::<u32>(veh, 0x49C, xi(gas).fmul_m(g::<f32>(0x858C58).read()).to_f32().to_bits());
    if rd::<u32>(veh, 0x590) == 0 {
        apply_panel_damage_to_car_call(rd::<u32>(this, 0x1C), veh, 1);
    }
    for k in 0..3 {
        wr::<u32>(veh, 0x44 + 4 * k, scaled(rd::<i8>(this, 0x20 + k), 0x859C48));
    }
    let pf = u32::from(rd::<u8>(this, 0x2F));
    let mut f = rd::<u32>(veh, 0x1C);
    f ^= ((pf << 6) ^ f) & 0x80;
    wr::<u32>(veh, 0x1C, f);
    f ^= ((pf << 28) ^ f) & 0x4000_0000;
    wr::<u32>(veh, 0x1C, f);
    wr::<u32>(veh, 0x1C, (((pf << 28) ^ f) & 0x7FFF_FFFF) ^ (pf << 28));
    if rd::<u16>(veh, 0x22) == 0x1B0 {
        wr::<u32>(veh, 0x494, 0);
        wr::<u32>(veh, 0x94C, scaled(rd::<i8>(this, 0x23), 0x859C44));
    } else {
        wr::<u32>(veh, 0x494, scaled(rd::<i8>(this, 0x23), 0x858B38));
    }
    if rd::<u32>(veh, 0x590) == 0 {
        for k in 0..4 {
            wr::<u32>(veh, 0x7D4 + 4 * k, scaled(rd::<i8>(this, 0x24 + k), 0x858B38));
            wr::<u32>(veh, 0x828 + 4 * k, scaled(rd::<i8>(this, 0x28 + k), 0x859C44));
        }
        wr::<u32>(veh, 0x5F4, scaled(rd::<i8>(this, 0x18), 0x859C40));
        wr::<u32>(veh, 0x5F8, rd::<u32>(veh, 0x5F4));
        wr::<u32>(veh, 0x60C, scaled(rd::<i8>(this, 0x19), 0x859C40));
        wr::<u32>(veh, 0x610, rd::<u32>(veh, 0x60C));
        if rd::<u8>(this, 0x18) != 0 {
            wr::<u8>(veh, 0x5AB, 1);
        }
        if rd::<u8>(this, 0x19) != 0 {
            wr::<u8>(veh, 0x5AC, 1);
        }
        let dm = veh.byte_add(0x5A0);
        for door in 0..6u32 {
            if rd::<u8>(this, 0x2C) & (1 << door) != 0 && damage_get_door_status(dm, door as i32) != 4 {
                dm_set_door_status(dm, door, 4);
                set_door_damage(veh, door, 1);
            }
        }
    }
    let mut eng = rd::<u8>(veh, 0x428);
    if rd::<u8>(veh, 0x42D) & 2 != 0 {
        eng &= 0xEF;
    } else {
        eng |= 0x10;
    }
    wr::<u8>(veh, 0x428, eng);
    if rd::<u32>(veh, 0x590) == 0 {
        wr::<u8>(veh, 0x961, 4);
    }
    wr::<u8>(veh, 0x2F, g::<u8>(CURR_AREA).read());
    world_remove(veh);
    world_add(veh);
    let pf = u32::from(rd::<u8>(this, 0x2F));
    let p = rd::<u32>(veh, 0x40);
    wr::<u32>(veh, 0x40, (((pf << 29) ^ p) & 0x2000_0000) ^ p);
    matrix_dtor(b);
    matrix_dtor(a);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_APPLY_PANEL_DAMAGE_TO_CAR, HOOK_EXTRACT_VEHICLE_UPDATE_DATA];
