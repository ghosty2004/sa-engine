//! `CRunningScript::LocateCharCommand` and `LocateCharObjectCommand` (batch `mvpk-script-2`): the LOCATE_CHAR_*
//! script conditions.
//! Ported from the 1.0 US bytes (gta-reversed `RunningScript.cpp` as a reading aid; the bytes win). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;
use sa_math::x87::X87;

use crate::vm::{collect_parameters_call, update_compare_flag_call, Scr};

/// `CTheScripts::ScriptParams` (0xA43C78)
const SCRIPT_PARAMS: usize = 0xA43C78;
/// `CPools::ms_pPedPool` (0xB74490; 0x7C4 per ped)
const PED_POOL: usize = 0xB74490;
/// `CTheScripts::DbgFlag` (0x859CF8): draw the 2D areas as debug squares
const DBG_FLAG: usize = 0x859CF8;
/// -100.0 (the 2D area's highlight z)
const K_MINUS_100: u32 = 0xC2C8_0000;

use crate::bindings::draw_debug_square;
use crate::bindings::highlight_important_area;
use crate::bindings::is_ped_stopped;

use sa_core::field::rd;
use sa_math::fpu::x;
fn param_u32(i: usize) -> u32 {
    GPtr::<u32>::from_addr(SCRIPT_PARAMS + 4 * i).read()
}
fn param_f32(i: usize) -> f32 {
    f32::from_bits(param_u32(i))
}
/// The pool slot of `handle` (index = handle >> 8, its flags byte must equal the low byte of the handle), else NULL.
fn pool_at(cell: usize, size: i32, handle: i32) -> GPtr<u8> {
    let p = GPtr::<GPtr<u8>>::from_addr(cell).read();
    let idx = handle >> 8;
    if rd::<GPtr<u8>>(p, 4).byte_add(idx as isize).read() != handle as u8 {
        return GPtr::NULL;
    }
    rd::<GPtr<u8>>(p, 0).byte_add(idx.wrapping_mul(size) as isize)
}
use sa_world::world::common::entity_pos;
/// `c - r <= p` and `r + c >= p` with the original's compares (`test ah, 0x41` / `jp` and `test ah, 1`; NaN fails).
fn within(c: f32, r: f32, p: f32) -> bool {
    !x(c).fsub_m(r).fcom(x(p)).test_parity(0x41) && !x(r).fadd_m(c).fcom(x(p)).c0()
}

/// `CRunningScript::LocateCharCommand(int32 command)` (0x486D80, thiscall, `ret 4`). x87 compares.
///
/// Commands 0xFE..0x103 are 3D (8 parameters: char, x, y, z, rx, ry, rz, show), the others 2D (6: char, x, y, rx, ry,
/// show). The char (`ped pool` handle, no NULL check) or, in a vehicle (+0x46C bit 8, +0x58C), its vehicle gives the
/// position p. The "stopped" commands (0xEF..0xF1, 0x101..0x103) need `IsPedStopped(ped)`, else false. Otherwise the
/// box `x - rx <= p.x <= rx + x`, y likewise, and in 3D z; inside → by `command - 0xEC`: any (0xEC/0xEF/0xFE/0x101),
/// on foot (0xED/0xF0/0xFF/0x102: not in a vehicle), in a vehicle (0xEE/0xF1/0x100/0x103); others false.
/// `UpdateCompareFlag(result)`. Show set → `HighlightImportantArea(this + IP (+0x14), f32(x - rx), f32(y - ry), f32(x +
/// rx), f32(y + ry), 3D ? z : -100)`. `CTheScripts::DbgFlag` and 2D → `DrawDebugSquare` of the same rectangle.
///
/// The original keeps the 3D flag in its own `command` argument slot (popped by `ret 4`) and passes the result as a
/// byte over an uninitialised stack dword.
#[hook(addr = 0x486d80, conv = thiscall, name = "CRunningScript::LocateCharCommand", expect = "83 EC 2C 53 55",
       this = "CRunningScript", route = "L2", hazards = "stack-residue,uninit-locals",
       recs = "grove-walk", note = "UpdateCompareFlag gets the result byte over an uninitialised dword (upper bytes)")]
pub fn running_script_locate_char_command(this: Scr, command: i32) {
    let is3d = (0xFE..=0x103).contains(&command);
    collect_parameters_call(this, if is3d { 8 } else { 6 });
    let ped = pool_at(PED_POOL, 0x7C4, param_u32(0) as i32);
    let in_veh = rd::<u32>(ped, 0x46C) & 0x100 != 0;
    let pos = if in_veh && !rd::<GPtr<u8>>(ped, 0x58C).is_null() { entity_pos(rd::<GPtr<u8>>(ped, 0x58C)) } else { entity_pos(ped) };
    let p = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
    let t = command.wrapping_sub(0xEF) as u32;
    let stopped_cmd = t <= 0x14 && !(3..=17).contains(&t);
    let stopped_fail = stopped_cmd && is_ped_stopped(ped) == 0;
    let (px, py) = (param_f32(1), param_f32(2));
    let (z, rx, ry, rz, show) = if is3d {
        (param_f32(3), param_f32(4), param_f32(5), param_f32(6), param_u32(7))
    } else {
        (0.0, param_f32(3), param_f32(4), 0.0, param_u32(5))
    };
    let mut result = false;
    if !stopped_fail && within(px, rx, p[0]) && within(py, ry, p[1]) && (!is3d || within(z, rz, p[2])) {
        let in_vehicle = rd::<u32>(ped, 0x46C) & 0x100 != 0;
        result = match command.wrapping_sub(0xEC) {
            0 | 3 | 18 | 21 => true,
            1 | 4 | 19 | 22 => !in_vehicle,
            2 | 5 | 20 | 23 => in_vehicle,
            _ => false,
        };
    }
    update_compare_flag_call(this, u32::from(result));
    let (x1, y1) = (x(px).fsub_m(rx).to_f32(), x(py).fsub_m(ry).to_f32());
    let (x2, y2) = (x(rx).fadd_m(px).to_f32(), x(ry).fadd_m(py).to_f32());
    if show != 0 {
        let id = (this.addr() as u32).wrapping_add(rd::<u32>(this, 0x14));
        highlight_important_area(id, x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits(), if is3d { z.to_bits() } else { K_MINUS_100 });
    }
    if GPtr::<u8>::from_addr(DBG_FLAG).read() != 0 && !is3d {
        draw_debug_square(x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits());
    }
}

/// `CPools::ms_pObjectPool` (0xB7449C; 0x19C per object)
const OBJECT_POOL: usize = 0xB7449C;

/// `CRunningScript::LocateCharObjectCommand(int32 command)` (0x487720, thiscall, `ret 4`). x87 compares.
///
/// Commands 0x474..0x476 are 3D (6 parameters: char, object, rx, ry, rz, show), the others 2D (5: char, object, rx,
/// ry, show). The char and the object come from their pools (no NULL checks); the char's position p is its vehicle's
/// when in one (+0x46C bit 8 and +0x58C), the object's position o. Inside: `p.x` not below `f32(o.x - rx)` and `o.x +
/// rx` not below `p.x`; `o.y - ry` not above `p.y` and `o.y + ry` not below it; in 3D z likewise with rz (NaN
/// fails). Inside → by `command - 0x471`: any (0, 3), on foot (1, 4: not in a vehicle), in a vehicle (2, 5).
/// `UpdateCompareFlag(result)`. Show set → `HighlightImportantArea(this + IP (+0x14), f32(o.x - rx), f32(o.y - ry),
/// f32(o.x + rx), f32(o.y + ry), 3D ? o.z : -100)`. `CTheScripts::DbgFlag` and 2D → `DrawDebugSquare` of the same
/// rectangle.
///
/// The original passes the result as a byte over an uninitialised stack dword.
#[hook(addr = 0x487720, conv = thiscall, name = "CRunningScript::LocateCharObjectCommand", expect = "8B 44 24 04 83 EC 30",
       this = "CRunningScript", unverified = "mvpk-script-2", route = "L2", hazards = "uninit-locals",
       recs = "grove-walk", note = "UpdateCompareFlag gets the result byte over an uninitialised dword (upper bytes)")]
pub fn running_script_locate_char_object_command(this: Scr, command: i32) {
    let is3d = (0x474..=0x476).contains(&command);
    collect_parameters_call(this, if is3d { 6 } else { 5 });
    let ped = pool_at(PED_POOL, 0x7C4, param_u32(0) as i32);
    let obj = pool_at(OBJECT_POOL, 0x19C, param_u32(1) as i32);
    let in_vehicle = rd::<u32>(ped, 0x46C) & 0x100 != 0;
    let pos = if in_vehicle && !rd::<GPtr<u8>>(ped, 0x58C).is_null() { entity_pos(rd::<GPtr<u8>>(ped, 0x58C)) } else { entity_pos(ped) };
    let p = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
    let op = entity_pos(obj);
    let o = [rd::<f32>(op, 0), rd::<f32>(op, 4), rd::<f32>(op, 8)];
    let (rx, ry) = (param_f32(2), param_f32(3));
    let (rz, show) = if is3d { (param_f32(4), param_u32(5)) } else { (0.0, param_u32(4)) };
    let x1 = x(o[0]).fsub_m(rx).to_f32();
    let inside = !x(p[0]).fcom(x(x1)).c0()
        && !x(o[0]).fadd_m(rx).fcom(x(p[0])).c0()
        && within(o[1], ry, p[1])
        && (!is3d || within(o[2], rz, p[2]));
    let result = inside
        && match command.wrapping_sub(0x471) {
            0 | 3 => true,
            1 | 4 => !in_vehicle,
            2 | 5 => in_vehicle,
            _ => false,
        };
    update_compare_flag_call(this, u32::from(result));
    let y1 = x(o[1]).fsub_m(ry).to_f32();
    let x2 = x(o[0]).fadd_m(rx).to_f32();
    let y2 = x(o[1]).fadd_m(ry).to_f32();
    if show != 0 {
        let id = (this.addr() as u32).wrapping_add(rd::<u32>(this, 0x14));
        highlight_important_area(id, x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits(), if is3d { o[2].to_bits() } else { K_MINUS_100 });
    }
    if GPtr::<u8>::from_addr(DBG_FLAG).read() != 0 && !is3d {
        draw_debug_square(x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits());
    }
}

// ---- SCRIPT-5: the remaining LOCATE helpers (char/char, char/car, car, object) and `IsVehicleStopped` ----

/// `CPools::ms_pVehiclePool` (0xB74494; 0xA18 per vehicle)
const VEHICLE_POOL: usize = 0xB74494;

use crate::bindings::is_vehicle_stopped as the_scripts_is_vehicle_stopped_call;
/// `CTimer::ms_fTimeStep` (0xB7CB5C)
const TIME_STEP: usize = 0xB7CB5C;
/// `CTimer::ms_fOldTimeStep` (0xB7CB54)
const OLD_TIME_STEP: usize = 0xB7CB54;
/// `.rdata` 0x858C58: 0.01f
const K_0_01: u32 = 0x3C23_D70A;

/// A char's in-vehicle bit (+0x46C bit 8, read once) and its position: the vehicle's (+0x58C) when the bit is set and
/// the vehicle is non-null, else its own.
fn char_pos(ped: GPtr<u8>) -> (bool, [f32; 3]) {
    let in_veh = rd::<u32>(ped, 0x46C) & 0x100 != 0;
    let v = rd::<GPtr<u8>>(ped, 0x58C);
    let pos = if in_veh && !v.is_null() { entity_pos(v) } else { entity_pos(ped) };
    (in_veh, [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)])
}

fn pos3(e: GPtr<u8>) -> [f32; 3] {
    let pos = entity_pos(e);
    [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)]
}

/// The script-side "vehicle stopped" test shared by `IsVehicleStopped` and the inlined copy in `LocateCarCommand`:
/// `m = ms_fTimeStep < ms_fOldTimeStep ? ms_fTimeStep : ms_fOldTimeStep` (`test ah, 5; jp`: NaN takes the old step),
/// then `speed (+0xD4) <= 0.01f * m` with the product kept on the x87 stack (`test ah, 0x41; jp` = false).
fn vehicle_stopped(v: GPtr<u8>) -> bool {
    let ts = GPtr::<f32>::from_addr(TIME_STEP).read();
    let old = GPtr::<f32>::from_addr(OLD_TIME_STEP).read();
    let m = if x(ts).fcom(x(old)).test_parity(5) { old } else { ts };
    let km = X87::from_f32(f32::from_bits(K_0_01)).fmul_m(m);
    !x(rd::<f32>(v, 0xD4)).fcom(km).test_parity(0x41)
}

/// The common tail of `LocateCharCharCommand` / `LocateCharCarCommand`: char position `p` (its in-vehicle bit
/// `in_veh`), the other entity's position `o`; parameters from slot 2: rx, ry, (3D: rz), show. `kind` comes from the
/// command's byte/jump table: 0 any, 1 on foot, 2 in a vehicle, other values false.
///
/// Inside (x87, NaN fails): `p.x` not below the stored `f32(o.x - rx)`, `o.x + rx` not below `p.x`, `o.y - ry` not
/// above `p.y` (`test ah, 0x41; jp`), `o.y + ry` not below `p.y`, in 3D z likewise with rz. The result byte goes to
/// `UpdateCompareFlag` (over an uninitialised dword). Show → `HighlightImportantArea(this + IP, x1, f32(o.y - ry),
/// f32(o.x + rx), f32(o.y + ry), 3D ? o.z : -100)`; `DbgFlag` and 2D → `DrawDebugSquare` of the same rectangle.
fn locate_char_vs(this: Scr, is3d: bool, in_veh: bool, p: [f32; 3], o: [f32; 3], kind: u8) {
    let (rx, ry) = (param_f32(2), param_f32(3));
    let (rz, show) = if is3d { (param_f32(4), param_u32(5)) } else { (0.0, param_u32(4)) };
    let x1 = x(o[0]).fsub_m(rx).to_f32();
    let inside = !x(p[0]).fcom(x(x1)).c0()
        && !x(o[0]).fadd_m(rx).fcom(x(p[0])).c0()
        && !x(o[1]).fsub_m(ry).fcom(x(p[1])).test_parity(0x41)
        && !x(o[1]).fadd_m(ry).fcom(x(p[1])).c0()
        && (!is3d || (!x(o[2]).fsub_m(rz).fcom(x(p[2])).test_parity(0x41) && !x(o[2]).fadd_m(rz).fcom(x(p[2])).c0()));
    let result = inside
        && match kind {
            0 => true,
            1 => !in_veh,
            2 => in_veh,
            _ => false,
        };
    update_compare_flag_call(this, u32::from(result));
    let y2 = x(o[1]).fadd_m(ry).to_f32();
    let x2 = x(o[0]).fadd_m(rx).to_f32();
    let y1 = x(o[1]).fsub_m(ry).to_f32();
    if show != 0 {
        let id = (this.addr() as u32).wrapping_add(rd::<u32>(this, 0x14));
        highlight_important_area(id, x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits(), if is3d { o[2].to_bits() } else { K_MINUS_100 });
    }
    if GPtr::<u8>::from_addr(DBG_FLAG).read() != 0 && !is3d {
        draw_debug_square(x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits());
    }
}

/// `CRunningScript::LocateCharCharCommand(int32 command)` (0x4870F0, thiscall, `ret 4`).
///
/// Commands 0x104..0x106 are 3D (6 parameters: char, char, rx, ry, rz, show), the others 2D (5). Both chars come from
/// the ped pool (no NULL checks; both handles resolved before either position is read); the first one's position is
/// tested against the second one's (each a vehicle position when in one). Kind by `command - 0xF2` through the byte
/// table 0x487408 (0..20): 0/18 any, 1/19 on foot, 2/20 in a vehicle, the rest false. See [`locate_char_vs`].
#[hook(addr = 0x4870f0, conv = thiscall, name = "CRunningScript::LocateCharCharCommand", expect = "8B 44 24 04 83 EC 30",
       this = "CRunningScript", unverified = "script5-1", route = "L2", hazards = "uninit-locals",
       recs = "grove-walk", note = "UpdateCompareFlag gets the result byte over an uninitialised dword (upper bytes)")]
pub fn running_script_locate_char_char_command(this: Scr, command: i32) {
    let is3d = (0x104..=0x106).contains(&command);
    collect_parameters_call(this, if is3d { 6 } else { 5 });
    let a = pool_at(PED_POOL, 0x7C4, param_u32(0) as i32);
    let b = pool_at(PED_POOL, 0x7C4, param_u32(1) as i32);
    let (in_veh, p) = char_pos(a);
    let (_, o) = char_pos(b);
    let kind = match command.wrapping_sub(0xF2) as u32 {
        0 | 18 => 0,
        1 | 19 => 1,
        2 | 20 => 2,
        _ => 3,
    };
    locate_char_vs(this, is3d, in_veh, p, o, kind);
}

/// `CRunningScript::LocateCharCarCommand(int32 command)` (0x487420, thiscall, `ret 4`).
///
/// Commands 0x205..0x207 are 3D (6 parameters: char, car, rx, ry, rz, show), the others 2D (5). The char from the ped
/// pool, the car from the vehicle pool (no NULL checks); the char's position (its vehicle's when in one) is tested
/// against the car's. Kind by `command - 0x202` through the jump table 0x487704 (0..5): any, on foot, in a vehicle,
/// twice. See [`locate_char_vs`].
#[hook(addr = 0x487420, conv = thiscall, name = "CRunningScript::LocateCharCarCommand", expect = "8B 44 24 04 83 EC 30",
       this = "CRunningScript", unverified = "script5-1", route = "L2", hazards = "uninit-locals",
       recs = "grove-walk", note = "UpdateCompareFlag gets the result byte over an uninitialised dword (upper bytes)")]
pub fn running_script_locate_char_car_command(this: Scr, command: i32) {
    let is3d = (0x205..=0x207).contains(&command);
    collect_parameters_call(this, if is3d { 6 } else { 5 });
    let ped = pool_at(PED_POOL, 0x7C4, param_u32(0) as i32);
    let veh = pool_at(VEHICLE_POOL, 0xA18, param_u32(1) as i32);
    let (in_veh, p) = char_pos(ped);
    let o = pos3(veh);
    let kind = match command.wrapping_sub(0x202) as u32 {
        0 | 3 => 0,
        1 | 4 => 1,
        2 | 5 => 2,
        _ => 3,
    };
    locate_char_vs(this, is3d, in_veh, p, o, kind);
}

/// `CRunningScript::LocateCarCommand(int32 command)` (0x487A20, thiscall, `ret 4`).
///
/// Commands 0x1AF/0x1B0 are 3D (8 parameters: car, x, y, z, rx, ry, rz, show), the others 2D (6: car, x, y, rx, ry,
/// show); the original keeps the 3D flag in its own argument slot. The car from the vehicle pool (no NULL check).
/// 0x1AE and 0x1B0 ("stopped") first need [`vehicle_stopped`] (inlined in the original), else false without reading
/// the car's position. Inside: `x - rx <= p.x <= rx + x`, y likewise, in 3D z ([`within`]). `UpdateCompareFlag`.
/// Show → `HighlightImportantArea(this + IP, f32(x - rx), f32(y - ry), f32(rx + x), f32(ry + y), 3D ? z : -100)`;
/// `DbgFlag` and 2D → `DrawDebugSquare` of the same rectangle.
#[hook(addr = 0x487a20, conv = thiscall, name = "CRunningScript::LocateCarCommand", expect = "83 EC 28 53 56",
       this = "CRunningScript", unverified = "script5-1", route = "L2", hazards = "uninit-locals,stack-residue",
       recs = "grove-walk", note = "UpdateCompareFlag gets the result byte over an uninitialised dword (upper bytes)")]
pub fn running_script_locate_car_command(this: Scr, command: i32) {
    let is3d = (0x1AF..=0x1B0).contains(&command);
    collect_parameters_call(this, if is3d { 8 } else { 6 });
    let veh = pool_at(VEHICLE_POOL, 0xA18, param_u32(0) as i32);
    let not_stopped = (command == 0x1AE || command == 0x1B0) && !vehicle_stopped(veh);
    let (px, py) = (param_f32(1), param_f32(2));
    let (z, rx, ry, rz, show) = if is3d {
        (param_f32(3), param_f32(4), param_f32(5), param_f32(6), param_u32(7))
    } else {
        (0.0, param_f32(3), param_f32(4), 0.0, param_u32(5))
    };
    let mut result = false;
    if !not_stopped {
        let p = pos3(veh);
        result = within(px, rx, p[0]) && within(py, ry, p[1]) && (!is3d || within(z, rz, p[2]));
    }
    update_compare_flag_call(this, u32::from(result));
    let y2 = x(ry).fadd_m(py).to_f32();
    let x2 = x(rx).fadd_m(px).to_f32();
    let y1 = x(py).fsub_m(ry).to_f32();
    let x1 = x(px).fsub_m(rx).to_f32();
    if show != 0 {
        let id = (this.addr() as u32).wrapping_add(rd::<u32>(this, 0x14));
        highlight_important_area(id, x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits(), if is3d { z.to_bits() } else { K_MINUS_100 });
    }
    if GPtr::<u8>::from_addr(DBG_FLAG).read() != 0 && !is3d {
        draw_debug_square(x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits());
    }
}

/// `CRunningScript::LocateObjectCommand(int32 command)` (0x487D10, thiscall, `ret 4`).
///
/// Command 0x4E6 is 3D (8 parameters: object, x, y, z, rx, ry, rz, show), every other one 2D (6: object, x, y, rx,
/// ry, show); the original keeps ry in its own argument slot. The object from the object pool (no NULL check), its
/// position p. Inside: `p.x` not below the stored `f32(x - rx)` and `rx + x` not below `p.x`; y and (3D) z as in
/// [`within`]. `UpdateCompareFlag`. Show → `HighlightImportantArea(this + IP, x1, f32(y - ry), f32(rx + x), f32(ry +
/// y), 3D ? z : -100)`; `DbgFlag` and 2D → `DrawDebugSquare` of the same rectangle.
#[hook(addr = 0x487d10, conv = thiscall, name = "CRunningScript::LocateObjectCommand", expect = "8B 44 24 04 83 EC 28",
       this = "CRunningScript", unverified = "script5-1", route = "L2", hazards = "uninit-locals,stack-residue",
       recs = "grove-walk", note = "UpdateCompareFlag gets the result byte over an uninitialised dword (upper bytes)")]
pub fn running_script_locate_object_command(this: Scr, command: i32) {
    let is3d = command == 0x4E6;
    collect_parameters_call(this, if is3d { 8 } else { 6 });
    let obj = pool_at(OBJECT_POOL, 0x19C, param_u32(0) as i32);
    let (px, py) = (param_f32(1), param_f32(2));
    let (z, rx, ry, rz, show) = if is3d {
        (param_f32(3), param_f32(4), param_f32(5), param_f32(6), param_u32(7))
    } else {
        (0.0, param_f32(3), param_f32(4), 0.0, param_u32(5))
    };
    let p = pos3(obj);
    let x1 = x(px).fsub_m(rx).to_f32();
    let result = !x(p[0]).fcom(x(x1)).c0()
        && !x(rx).fadd_m(px).fcom(x(p[0])).c0()
        && within(py, ry, p[1])
        && (!is3d || within(z, rz, p[2]));
    update_compare_flag_call(this, u32::from(result));
    let y2 = x(ry).fadd_m(py).to_f32();
    let x2 = x(rx).fadd_m(px).to_f32();
    let y1 = x(py).fsub_m(ry).to_f32();
    if show != 0 {
        let id = (this.addr() as u32).wrapping_add(rd::<u32>(this, 0x14));
        highlight_important_area(id, x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits(), if is3d { z.to_bits() } else { K_MINUS_100 });
    }
    if GPtr::<u8>::from_addr(DBG_FLAG).read() != 0 && !is3d {
        draw_debug_square(x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits());
    }
}

/// `CTheScripts::IsVehicleStopped(CVehicle*)` (0x4861F0, cdecl): [`vehicle_stopped`] (AL; the original leaves the FPU
/// status word in AH).
#[hook(addr = 0x4861f0, conv = cdecl, name = "CTheScripts::IsVehicleStopped", expect = "8B 44 24 04 D9 80 D4 00 00 00",
       preserve = "ecx", unverified = "script5-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "AL only; the original's AH holds the fnstsw bits; ECX kept (caller 0x488F2F reads it)")]
pub fn the_scripts_is_vehicle_stopped(v: GPtr<u8>) -> u8 {
    u8::from(vehicle_stopped(v))
}

use sa_ped::bindings::ped_is_player;
use sa_core::bindings::clump_get_association;

/// `CTheScripts::IsPedStopped(CPed*)` (0x486110, cdecl). The flags load (`mov eax, [esi+0x46C]`) sits behind a `jmp`
/// to 0x409A95 and back.
///
/// In a vehicle (+0x46C bit 8, +0x58C non-null): stopped when `ms_fTimeStep * 0.01f` (x87) is not below the vehicle's
/// speed (+0xD4; NaN → false). Otherwise: a player with any of the animations 6, 7, 0x74, 0x76 on its clump (+0x18)
/// is moving; then false for a move state (+0x534) >= 4, flags (re-read) with bit 9 or 10, or without bit 0; else
/// stopped when both +0x4D8 and +0x4DC equal 0.0 (`test ah, 0x44; jp`).
#[hook(addr = 0x486110, conv = cdecl, name = "CTheScripts::IsPedStopped", expect = "56 8B 74 24 08 90 E9",
       unverified = "script5-10", route = "L2", hazards = "none", recs = "grove-walk",
       note = "AL only; the original's upper EAX holds the flags, a status word or a callee's result")]
pub fn the_scripts_is_ped_stopped(ped: GPtr<u8>) -> u8 {
    let flags = rd::<u32>(ped, 0x46C);
    let v = rd::<GPtr<u8>>(ped, 0x58C);
    if flags & 0x100 != 0 && !v.is_null() {
        let speed = rd::<f32>(v, 0xD4);
        let k = x(GPtr::<f32>::from_addr(TIME_STEP).read()).fmul_m(f32::from_bits(K_0_01));
        return u8::from(!k.fcom(x(speed)).c0());
    }
    if ped_is_player(ped) != 0 {
        for id in [6, 7, 0x74, 0x76] {
            if !clump_get_association(rd::<GPtr<u8>>(ped, 0x18), id).is_null() {
                return 0;
            }
        }
    }
    if rd::<i32>(ped, 0x534) >= 4 {
        return 0;
    }
    let flags = rd::<u32>(ped, 0x46C);
    if flags & 0x600 != 0 || flags & 1 == 0 {
        return 0;
    }
    let zero = |off: isize| !x(rd::<f32>(ped, off)).fcom(x(0.0)).test_parity(0x44);
    u8::from(zero(0x4D8) && zero(0x4DC))
}

/// The original's `fcom`-based box test of `IS_*_IN_AREA`: `p` not below `lo` (`test ah, 1; jne` fails) and not above
/// `hi` (`test ah, 0x41; jp` fails); NaN fails.
fn in_span(p: f32, lo: f32, hi: f32) -> bool {
    !x(p).fcom(x(lo)).c0() && !x(p).fcom(x(hi)).test_parity(0x41)
}

/// `a > b` (ordered; `fcom; test ah, 0x41; jne` not taken): the swap test of the area corners.
fn swap_if_greater(a: &mut f32, b: &mut f32) {
    if !x(*a).fcom(x(*b)).test_any(0x41) {
        core::mem::swap(a, b);
    }
}

/// The shared part of the `IS_*_IN_AREA_2D/3D` checks (0x488B50, 0x488EC0, 0x489150) after the entity position `p`
/// is known: the corners from the parameters (2D: x1, y1, x2, y2, show from parameter 1; 3D: x1, y1, z1, x2, y2, z2,
/// show), each pair ordered (z first in 3D, then x, then y; swapped only when the first is greater). Unless `failed`,
/// the result is `in_span` on every axis and then `pass()`. `UpdateCompareFlag(result)` (a byte over an uninitialised
/// dword). Show set → `HighlightImportantArea(this + IP, x1, y1, x2, y2, 3D ? f32((z2 + z1) * 0.5) : -100)`.
/// `CTheScripts::DbgFlag` and 2D → `DrawDebugSquare(x1, y1, x2, y2)`.
fn area_check(this: Scr, is3d: bool, p: [f32; 3], failed: bool, pass: impl Fn() -> bool) {
    let (mut x1, mut y1) = (param_f32(1), param_f32(2));
    let (mut x2, mut y2, mut z1, mut z2, show);
    if is3d {
        z1 = param_f32(3);
        x2 = param_f32(4);
        y2 = param_f32(5);
        z2 = param_f32(6);
        swap_if_greater(&mut z1, &mut z2);
        show = param_u32(7);
    } else {
        x2 = param_f32(3);
        y2 = param_f32(4);
        show = param_u32(5);
        z1 = 0.0;
        z2 = 0.0;
    }
    swap_if_greater(&mut x1, &mut x2);
    swap_if_greater(&mut y1, &mut y2);
    let result = !failed
        && in_span(p[0], x1, x2)
        && in_span(p[1], y1, y2)
        && (!is3d || in_span(p[2], z1, z2))
        && pass();
    update_compare_flag_call(this, u32::from(result));
    if show != 0 {
        let id = (this.addr() as u32).wrapping_add(rd::<u32>(this, 0x14));
        let z = if is3d { x(z2).fadd_m(z1).fmul_m(0.5).to_f32().to_bits() } else { K_MINUS_100 };
        highlight_important_area(id, x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits(), z);
    }
    if GPtr::<u8>::from_addr(DBG_FLAG).read() != 0 && !is3d {
        draw_debug_square(x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits());
    }
}

/// `CRunningScript::CharInAreaCheckCommand(int32 command)` (0x488B50, thiscall, `ret 4`; unreversed in gta-reversed).
/// x87 compares.
///
/// Commands 0xA4 and 0x1A6..0x1AA are 3D (8 parameters: char, x1, y1, z1, x2, y2, z2, show), the others 2D (6: char,
/// x1, y1, x2, y2, show). The char from the ped pool (NULL for a stale handle, not checked) or, in a vehicle (+0x46C bit
/// 8, +0x58C), its vehicle gives the position p. The "stopped" commands (0x1A3..0x1A5, 0x1A8..0x1AA; byte table
/// 0x488EA4) fail unless `IsPedStopped(ped)`. Inside the box (`area_check`) → by command: any (0xA3, 0xA4, 0x1A3,
/// 0x1A8), on foot (0x1A1, 0x1A4, 0x1A6, 0x1A9: not in a vehicle), in a vehicle (0x1A2, 0x1A5, 0x1A7, 0x1AA); others
/// false.
#[hook(addr = 0x488B50, conv = thiscall, name = "CRunningScript::CharInAreaCheckCommand", expect = "83 EC 2C 53 55 56",
       this = "CRunningScript", unverified = "frontier2-c2", route = "L2", hazards = "uninit-locals",
       recs = "grove-walk", note = "UpdateCompareFlag gets the result byte over an uninitialised dword (upper bytes)")]
pub fn running_script_char_in_area_check_command(this: Scr, command: i32) {
    let is3d = command == 0xA4 || (0x1A6..=0x1AA).contains(&command);
    collect_parameters_call(this, if is3d { 8 } else { 6 });
    let ped = pool_at(PED_POOL, 0x7C4, param_u32(0) as i32);
    let in_vehicle = rd::<u32>(ped, 0x46C) & 0x100 != 0;
    let pos = if in_vehicle && !rd::<GPtr<u8>>(ped, 0x58C).is_null() { entity_pos(rd::<GPtr<u8>>(ped, 0x58C)) } else { entity_pos(ped) };
    let p = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
    let t = command.wrapping_sub(0x1A3) as u32;
    let failed = t <= 7 && [0u8, 0, 0, 1, 1, 0, 0, 0][t as usize] == 0 && is_ped_stopped(ped) == 0;
    area_check(this, is3d, p, failed, || {
        let on_foot = rd::<u32>(ped, 0x46C) & 0x100 == 0;
        match command {
            0xA3 | 0xA4 | 0x1A3 | 0x1A8 => true,
            0x1A1 | 0x1A4 | 0x1A6 | 0x1A9 => on_foot,
            0x1A2 | 0x1A5 | 0x1A7 | 0x1AA => !on_foot,
            _ => false,
        }
    });
}

/// `CRunningScript::CarInAreaCheckCommand(int32 command)` (0x488EC0, thiscall, `ret 4`; unreversed in gta-reversed).
/// x87 compares. Commands 0xB1 and 0x1AC are 3D (8 parameters), the others 2D (6); the car from the vehicle pool (no
/// NULL check) gives the position. The "stopped" commands 0x1AB / 0x1AC fail unless `IsVehicleStopped(car)` (checked
/// before the corners are read). Inside the box → true (`area_check`).
#[hook(addr = 0x488EC0, conv = thiscall, name = "CRunningScript::CarInAreaCheckCommand", expect = "83 EC 28 53 55 56 57",
       this = "CRunningScript", unverified = "frontier2-c2", route = "L2", hazards = "uninit-locals",
       recs = "grove-walk", note = "UpdateCompareFlag gets the result byte over an uninitialised dword (upper bytes)")]
pub fn running_script_car_in_area_check_command(this: Scr, command: i32) {
    let is3d = command == 0xB1 || command == 0x1AC;
    collect_parameters_call(this, if is3d { 8 } else { 6 });
    let car = pool_at(VEHICLE_POOL, 0xA18, param_u32(0) as i32);
    let failed = (0x1AB..=0x1AC).contains(&command) && the_scripts_is_vehicle_stopped_call(car) == 0;
    let pos = entity_pos(car);
    let p = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
    area_check(this, is3d, p, failed, || true);
}

/// `CRunningScript::ObjectInAreaCheckCommand(int32 command)` (0x489150, thiscall, `ret 4`; unreversed in
/// gta-reversed). x87 compares. Command 0x4EA is 3D (8 parameters), the others 2D (6); the object from the object pool
/// (no NULL check) gives the position. Inside the box → true (`area_check`).
#[hook(addr = 0x489150, conv = thiscall, name = "CRunningScript::ObjectInAreaCheckCommand", expect = "8B 44 24 04 83 EC 28",
       this = "CRunningScript", unverified = "frontier2-c2", route = "L2", hazards = "uninit-locals",
       recs = "grove-walk", note = "UpdateCompareFlag gets the result byte over an uninitialised dword (upper bytes)")]
pub fn running_script_object_in_area_check_command(this: Scr, command: i32) {
    let is3d = command == 0x4EA;
    collect_parameters_call(this, if is3d { 8 } else { 6 });
    let obj = pool_at(OBJECT_POOL, 0x19C, param_u32(0) as i32);
    let pos = entity_pos(obj);
    let p = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
    area_check(this, is3d, p, false, || true);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RUNNING_SCRIPT_CHAR_IN_AREA_CHECK_COMMAND,
    HOOK_RUNNING_SCRIPT_CAR_IN_AREA_CHECK_COMMAND,
    HOOK_RUNNING_SCRIPT_OBJECT_IN_AREA_CHECK_COMMAND,
    HOOK_RUNNING_SCRIPT_LOCATE_CHAR_COMMAND,
    HOOK_RUNNING_SCRIPT_LOCATE_CHAR_OBJECT_COMMAND,
    HOOK_RUNNING_SCRIPT_LOCATE_CHAR_CHAR_COMMAND,
    HOOK_RUNNING_SCRIPT_LOCATE_CHAR_CAR_COMMAND,
    HOOK_RUNNING_SCRIPT_LOCATE_CAR_COMMAND,
    HOOK_RUNNING_SCRIPT_LOCATE_OBJECT_COMMAND,
    HOOK_THE_SCRIPTS_IS_VEHICLE_STOPPED,
    HOOK_THE_SCRIPTS_IS_PED_STOPPED,
];
