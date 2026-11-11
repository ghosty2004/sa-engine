//! `CRunningScript::LocateCharCommand` and `LocateCharObjectCommand` (batch `mvpk-script-2`): the LOCATE_CHAR_*
//! script conditions.
//! Ported from the 1.0 US bytes (gta-reversed `RunningScript.cpp` as a reading aid; the bytes win). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
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

orig! {
    /// `CTheScripts::IsPedStopped(CPed*)` (0x486110)
    fn is_ped_stopped(p: GPtr<u8>) -> u8 = cdecl 0x486110 => unported;
    /// `CTheScripts::HighlightImportantArea(uint32 id, float x1, float y1, float x2, float y2, float z)` (0x485E00)
    fn highlight_important_area(id: u32, x1: u32, y1: u32, x2: u32, y2: u32, z: u32) = cdecl 0x485E00 => unported;
    /// `CTheScripts::DrawDebugSquare(float x1, float y1, float x2, float y2)` (0x486840)
    fn draw_debug_square(x1: u32, y1: u32, x2: u32, y2: u32) = cdecl 0x486840 => unported;
}

fn rd<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}
fn x(v: f32) -> X87 {
    X87::from_f32(v)
}
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
/// An entity's position (matrix +0x30, else the placement +4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}
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
       this = "CRunningScript", unverified = "mvpk-script-2", route = "L2", hazards = "stack-residue,uninit-locals",
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

pub const HOOKS: &[HookReg] = &[HOOK_RUNNING_SCRIPT_LOCATE_CHAR_COMMAND, HOOK_RUNNING_SCRIPT_LOCATE_CHAR_OBJECT_COMMAND];
