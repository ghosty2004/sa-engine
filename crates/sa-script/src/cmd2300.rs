//! `CRunningScript::ProcessCommands2300To2399` (0x4762D0): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2300To2399.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x0920, 0x0925, 0x092F, 0x0930, 0x0936, 0x0948];

/// `TheCamera.m_bCameraPersistTrack` (0xB6F028 + 0xCEF).
const CAMERA_PERSIST_TRACK: usize = 0xB6FD17;
/// `TheCamera.m_bCameraPersistPosition` (0xB6F028 + 0xCEE).
const CAMERA_PERSIST_POSITION: usize = 0xB6FD16;

orig! {
    /// `CCamera::VectorTrackLinear(CVector& to, CVector& from, float duration, bool ease)` (0x50D1D0)
    fn vector_track_linear(cam: GPtr<u8>, to: GPtr<u32>, from: GPtr<u32>, duration: f32, ease: u32) = thiscall 0x50D1D0 => unported;
    /// `CCamera::VectorMoveLinear(CVector& to, CVector& from, float duration, bool ease)` (0x50D160)
    fn vector_move_linear(cam: GPtr<u8>, to: GPtr<u32>, from: GPtr<u32>, duration: f32, ease: u32) = thiscall 0x50D160 => unported;
    /// `CCamera::InitialiseScriptableComponents()` (0x50D2D0)
    fn initialise_scriptable_components(cam: GPtr<u8>) = thiscall 0x50D2D0 => unported;
    /// `CExplosion::AddExplosion(CEntity* victim, CPed* creator, eExplosionType, CVector pos, uint32 lifetime,
    /// bool makeSound, float camShake, bool visibility)` (0x736A50; the vector by value)
    fn add_explosion(victim: u32, creator: u32, ty: u32, x: u32, y: u32, z: u32, lifetime: u32, make_sound: u32, cam_shake: u32, visibility: u32) = cdecl 0x736A50 => unported;
}

/// The VECTOR_TRACK / VECTOR_MOVE argument shape (0920 0x476BDE, 0936 0x476F9D): `CollectParameters(8)`; `from =
/// (SP0, SP1, SP2)`, `to = (SP3, SP4, SP5)`; `duration = (float)SP6` (`fild`: an integer operand, gr reads a float
/// for 0936); `ease = SP7 != 0` (pushed with stale upper bytes of ECX = SP0; the callee reads the low byte, the port
/// passes 0/1). The callee gets `(&to, &from, duration, ease)`.
fn vector_args(s: Scr) -> ([u32; 3], [u32; 3], f32, u32) {
    collect(s, 8);
    let from = [sp(0), sp(1), sp(2)];
    let to = [sp(3), sp(4), sp(5)];
    let duration = X87::from_i32(spi(6)).to_f32();
    (to, from, duration, u32::from(sp(7) != 0))
}

/// `CRunningScript::ProcessCommands2300To2399(int32 command)` (0x4762D0, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x4762D0, conv = thiscall, name = "CRunningScript::ProcessCommands2300To2399", expect = "81 EC 84 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_2300_to_2399(s: Scr, command: u32) -> i8 {
    match command {
        // 0920 CAMERA_SET_VECTOR_TRACK (0x476BDE)
        0x0920 => {
            let (to, from, d, ease) = vector_args(s);
            let (tp, fp) = (
                GPtr::from_raw(to.as_ptr().cast_mut()),
                GPtr::from_raw(from.as_ptr().cast_mut()),
            );
            vector_track_linear(GPtr::from_addr(THE_CAMERA), tp, fp, d, ease);
        }
        // 0925 CAMERA_RESET_NEW_SCRIPTABLES (0x476CF8)
        0x0925 => initialise_scriptable_components(GPtr::from_addr(THE_CAMERA)),
        // 092F CAMERA_PERSIST_TRACK (0x476EE9): `TheCamera.m_bCameraPersistTrack = SP0 != 0`
        0x092F => {
            collect(s, 1);
            GPtr::<u8>::from_addr(CAMERA_PERSIST_TRACK).write(u8::from(sp(0) != 0));
        }
        // 0930 CAMERA_PERSIST_POS (0x476F10): `TheCamera.m_bCameraPersistPosition = SP0 != 0`
        0x0930 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(CAMERA_PERSIST_POSITION).write(u8::from(sp(0) != 0));
        }
        // 0936 CAMERA_SET_VECTOR_MOVE (0x476F9D): the duration is `fild SP6` (gr: a float parameter)
        0x0936 => {
            let (to, from, d, ease) = vector_args(s);
            let (tp, fp) = (
                GPtr::from_raw(to.as_ptr().cast_mut()),
                GPtr::from_raw(from.as_ptr().cast_mut()),
            );
            vector_move_linear(GPtr::from_addr(THE_CAMERA), tp, fp, d, ease);
        }
        // 0948 ADD_EXPLOSION_VARIABLE_SHAKE (0x4774DA): `CollectParameters(5)`; `AddExplosion(null, null, SP3,
        // CVector(SP0, SP1, SP2), 0, true, SP4 (raw f32), false)`
        0x0948 => {
            collect(s, 5);
            add_explosion(0, 0, sp(3), sp(0), sp(1), sp(2), 0, 1, sp(4), 0);
        }
        _ => {
            return fallback(
                process_commands_2300_to_2399_original,
                "CRunningScript::ProcessCommands2300To2399",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2300_TO_2399];
