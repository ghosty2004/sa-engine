//! `CRunningScript::ProcessCommands1600To1699` (0x493FE0): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1600To1699.md`.
//!
//! Every ported case returns 0 (shared `xor al,al` tails 0x495DA9 / 0x495DDD, epilogue 0x495DE3 -> 0x4059EC).
//! Degree-to-radian products the original stores as f32 temporaries in stack slots (`fmul [0x8595EC]; fstp`)
//! are computed with `X87` (PC=24, memory operand) and rounded to f32 the same way.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x064C, 0x0650, 0x0681, 0x0682, 0x0685, 0x069A, 0x069B];

/// `.rdata` 0x8595EC: 0.017453292f (degrees to radians)
const K_DEG2RAD: u32 = 0x3C8E_FA35;
/// `.rdata` 0x858C1C: -1.0f
const K_MINUS_1: u32 = 0xBF80_0000;
/// `CTheScripts::ScriptEffectSystemArray[0].m_pFxSystem` (8-byte entries; the case reads `[i*8 + 0xA44114]`).
const SCRIPT_EFFECT_FX: usize = 0xA44114;

orig! {
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: i32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    fn fx_play(fx: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `CTheScripts::RemoveScriptEffectSystem(int32)` (0x492FD0)
    fn remove_script_effect_system(h: i32) = cdecl 0x492FD0 => unported;
    /// `CPhysical::AttachEntityToEntity(CPhysical*, CVector offset, CVector rotation)` (0x54D570; both vectors by value)
    fn attach_entity_to_entity(e: GPtr<u8>, target: GPtr<u8>, ox: u32, oy: u32, oz: u32, rx: f32, ry: f32, rz: f32) = thiscall 0x54D570 => unported;
    /// `CPhysical::DettachEntityFromEntity(float x, float y, float z, bool useCollision)` (0x5442F0)
    fn dettach_entity_from_entity(e: GPtr<u8>, x: f32, y: f32, z: u32, use_collision: u32) = thiscall 0x5442F0 => unported;
}

/// `fld SP[i]; fmul [0x8595EC]; fstp dword`
#[inline]
fn deg2rad(v: f32) -> f32 {
    X87::from_f32(v).fmul_m(f(K_DEG2RAD)).to_f32()
}

/// The script effect system slot of handle `h` (`GetActualScriptThingIndex(h, 1)`), or `None` for a negative
/// index or a null `FxSystem_c*` (both go to the `xor al,al` tail 0x495DDD).
#[inline]
fn fx_slot(h: i32) -> Option<GPtr<u8>> {
    let i = get_actual_script_thing_index(h, 1);
    if i < 0 {
        return None;
    }
    let fx = GPtr::<GPtr<u8>>::from_addr(SCRIPT_EFFECT_FX.wrapping_add((i as u32).wrapping_mul(8) as usize))
        .read();
    if fx.is_null() { None } else { Some(fx) }
}

/// `CollectParameters(8)`; `obj = OBJ(SP0)`, `target` from the given lookup on SP1 (called after the object
/// lookup); `obj->AttachEntityToEntity(target, CVector(SP2, SP3, SP4), CVector(SP5, SP6, SP7) * deg2rad)`. No null
/// checks (0681 0x495425, 069A 0x495BAF, 069B 0x495C6A: the same code with a different target pool).
fn attach_object(s: Scr, target: fn(u32) -> GPtr<u8>) {
    collect(s, 8);
    let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
    let t = target(sp(1));
    let (rx, ry, rz) = (deg2rad(spf(5)), deg2rad(spf(6)), deg2rad(spf(7)));
    attach_entity_to_entity(obj, t, sp(2), sp(3), sp(4), rx, ry, rz);
}

/// `CRunningScript::ProcessCommands1600To1699(int32 command)` (0x493FE0, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x493FE0, conv = thiscall, name = "CRunningScript::ProcessCommands1600To1699", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_1600_to_1699(s: Scr, command: u32) -> i8 {
    match command {
        // 064C PLAY_FX_SYSTEM (0x494323)
        0x064C => {
            collect(s, 1);
            if let Some(fx) = fx_slot(spi(0)) {
                fx_play(fx);
            }
        }
        // 0650 KILL_FX_SYSTEM (0x494402): the handle (EDI) is SP0 as collected; tail 0x4943D7
        0x0650 => {
            collect(s, 1);
            let h = spi(0);
            if let Some(fx) = fx_slot(h) {
                fx_kill(fx);
                remove_script_effect_system(h);
                if uses_mission_cleanup(s) {
                    mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), h, 4);
                }
            }
        }
        // 0681 ATTACH_OBJECT_TO_CAR (0x495425)
        0x0681 => attach_object(s, |h| vehicle_at_ref(pool(VEHICLE_POOL), h as i32)),
        // 0682 DETACH_OBJECT (0x4954E2)
        0x0682 => detach_object(s),
        // 0685 IS_OBJECT_ATTACHED (0x4956D3): `obj && obj->m_pAttachedTo (+0xFC)`
        0x0685 => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let b = !o.is_null() && !o.byte_add(0xFC).cast::<GPtr<u8>>().read().is_null();
            set_flag(s, b);
        }
        // 069A ATTACH_OBJECT_TO_OBJECT (0x495BAF): the object pool pointer is loaded once (ESI) for both lookups
        0x069A => {
            collect(s, 8);
            let p = pool(OBJECT_POOL);
            let obj = object_at_ref(p, spi(0));
            let t = object_at_ref(p, spi(1));
            let (rx, ry, rz) = (deg2rad(spf(5)), deg2rad(spf(6)), deg2rad(spf(7)));
            attach_entity_to_entity(obj, t, sp(2), sp(3), sp(4), rx, ry, rz);
        }
        // 069B ATTACH_OBJECT_TO_CHAR (0x495C6A)
        0x069B => attach_object(s, |h| ped_at_ref(pool(PED_POOL), h as i32)),
        _ => {
            return fallback(
                process_commands_1600_to_1699_original,
                "CRunningScript::ProcessCommands1600To1699",
                s,
                command,
            );
        }
    }
    0
}

/// 0x0682 DETACH_OBJECT (0x4954E2): `CollectParameters(5)`; `e = OBJ(SP0)`; `ny = SP2 * -1.0f` (x87, stored f32);
/// if `e && e->m_pAttachedTo (+0xFC)`: `e->DettachEntityFromEntity(SP1 * deg2rad, ny * deg2rad, SP3 (raw), SP4 != 0)`.
fn detach_object(s: Scr) {
    collect(s, 5);
    let e = object_at_ref(pool(OBJECT_POOL), spi(0));
    let ny = X87::from_f32(spf(2)).fmul_m(f(K_MINUS_1)).to_f32();
    if e.is_null() || e.byte_add(0xFC).cast::<GPtr<u8>>().read().is_null() {
        return;
    }
    let z = sp(3);
    let use_collision = u32::from(sp(4) != 0);
    let y = deg2rad(ny);
    let x = deg2rad(spf(1));
    dettach_entity_from_entity(e, x, y, z, use_collision);
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1600_TO_1699];
