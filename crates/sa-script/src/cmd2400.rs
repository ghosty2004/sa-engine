//! `CRunningScript::ProcessCommands2400To2499` (0x478000): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2400To2499.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x09A2, 0x09BA];

/// `CHud::bScriptDontDisplayAreaName` 0xBAA3F8 (owned by the HUD; written by address as the original does).
const SCRIPT_DONT_DISPLAY_AREA_NAME: usize = 0xBAA3F8;
/// `CClumpModelInfo::SetAtomicRendererCB` (0x4C5280), passed as a callback
const SET_ATOMIC_RENDERER_CB: u32 = 0x4C5280;
/// `CVisibilityPlugins::RenderFadingClumpCB` (0x733630), passed as the callback data
const RENDER_FADING_CLUMP_CB: u32 = 0x733630;

orig! {
    /// `CTheScripts::CleanUpThisObject(CObject*)` (0x4866C0)
    fn clean_up_this_object(o: GPtr<u8>) = cdecl 0x4866C0 => unported;
    /// `RpClumpForAllAtomics(RpClump*, RpAtomicCallBack, void*)` (0x749B70)
    fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
}

/// `CRunningScript::ProcessCommands2400To2499(int32 command)` (0x478000, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x478000, conv = thiscall, name = "CRunningScript::ProcessCommands2400To2499", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_2400_to_2499(s: Scr, command: u32) -> i8 {
    match command {
        // 09A2 REMOVE_OBJECT_ELEGANTLY (0x479107): `obj = OBJ(SP0)`; `CleanUpThisObject(obj)` (also when null); if obj:
        // `obj->m_pRwObject (+0x18)` -> `RpClumpForAllAtomics(rw, SetAtomicRendererCB, RenderFadingClumpCB)` (no type
        // check), then `+0x140 |= 0x400000`; with mission cleanup `RemoveEntityFromList(SP0, 3)`.
        0x09A2 => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            clean_up_this_object(o);
            if !o.is_null() {
                let rw = o.byte_add(0x18).cast::<GPtr<u8>>().read();
                if !rw.is_null() {
                    rp_clump_for_all_atomics(rw, SET_ATOMIC_RENDERER_CB, RENDER_FADING_CLUMP_CB);
                }
                let fl = o.byte_add(0x140).cast::<u32>();
                fl.write(fl.read() | 0x40_0000);
            }
            if uses_mission_cleanup(s) {
                mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 3);
            }
        }
        // 09BA DISPLAY_ZONE_NAMES (0x47970C): `CHud::bScriptDontDisplayAreaName = SP0 == 0`
        0x09BA => {
            collect(s, 1);
            GPtr::<u8>::from_addr(SCRIPT_DONT_DISPLAY_AREA_NAME).write(u8::from(sp(0) == 0));
        }
        _ => {
            return fallback(
                process_commands_2400_to_2499_original,
                "CRunningScript::ProcessCommands2400To2499",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2400_TO_2499];
