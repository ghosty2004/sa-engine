//! `CRunningScript::ProcessCommands1200To1299` (0x48B590): partial port (PORT-SCRIPT-2, ADR-002).
//! Opcodes in [`PORTED`] run in Rust; every other opcode (and the default, -1) runs the original body
//! through the fallback. Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands1200To1299.md`.
//!
//! Text-label buffers: the original's 16-byte stack buffer is not cleared before
//! `ReadTextLabelFromScript`, which copies `min(length, 16)` bytes; the port's buffer starts zeroed.
//! They differ only for a label that leaves its terminator to stale stack bytes (not observed).

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x04BB, 0x04CE, 0x04D7, 0x04E4, 0x04ED, 0x04EE];

/// `CGame::currArea` 0xB72914.
const CURR_AREA: usize = 0xB72914;
/// `CScriptResourceManager ScriptResourceManager` 0xA485A8.
const SCRIPT_RESOURCE_MANAGER: usize = 0xA485A8;

orig! {
    /// `CStreaming::RemoveBuildingsNotInArea(int32)` (0x4094B0)
    pub fn remove_buildings_not_in_area(area: i32) = cdecl 0x4094B0 => unported;
    /// `CColStore::RequestCollision(const CVector&, int32 area)` (0x410C00)
    pub fn request_collision(pos: GPtr<f32>, area: i32) = cdecl 0x410C00 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    pub fn get_animation_block_index(name: GPtr<u8>) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940)
    pub fn get_animation_block(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D3940 => unported;
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0)
    pub fn request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CScriptResourceManager::AddToResourceManager(int32, uint32 type, CRunningScript*)` (0x4704B0)
    pub fn add_to_resource_manager(m: GPtr<u8>, id: i32, ty: u32, s: Scr) = thiscall 0x4704B0 => unported;
}

/// `CRunningScript::ProcessCommands1200To1299(int32 command)` (0x48B590, thiscall, `ret 4`): AL = 0
/// continue, -1 unknown.
#[hook(addr = 0x48B590, conv = thiscall, name = "CRunningScript::ProcessCommands1200To1299", expect = "6A FF 68 BA 84 83 00", this = "CRunningScript", fallback)]
pub fn process_commands_1200_to_1299(s: Scr, command: u32) -> i8 {
    match command {
        // 04BB SET_AREA_VISIBLE (case 0x48B98D): `CGame::currArea = SP0;
        // CStreaming::RemoveBuildingsNotInArea(SP0)`. gr: unregistered.
        0x04BB => {
            collect(s, 1);
            let a = spi(0);
            GPtr::<i32>::from_addr(CURR_AREA).write(a);
            remove_buildings_not_in_area(a);
            0
        }
        // 04CE ADD_SHORT_RANGE_SPRITE_BLIP_FOR_COORD (case 0x48BC0A): type 4 (coord), colour 5.
        0x04CE => {
            short_range_coord_blip(s, 4, 5);
            0
        }
        // 04D7 FREEZE_CHAR_POSITION (case 0x48BFE3): `ped->m_nPhysicalFlags` (+0x40) bit 0x2000
        // (bDontApplySpeed) = SP1 != 0; the ped pointer is not null-checked (as in the original).
        0x04D7 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let fl = ped.byte_add(0x40).cast::<u32>();
            let v = fl.read();
            fl.write(if sp(1) != 0 { v | 0x2000 } else { v & !0x2000 });
            0
        }
        // 04E4 REQUEST_COLLISION (case 0x48C244): `CColStore::RequestCollision({SP0, SP1, 0},
        // CGame::currArea)`. gr: unregistered.
        0x04E4 => {
            collect(s, 2);
            let area = GPtr::<i32>::from_addr(CURR_AREA).read();
            let v = [sp(0), sp(1), 0u32];
            request_collision(GPtr::from_raw(v.as_ptr().cast_mut()).cast(), area);
            0
        }
        // 04ED REQUEST_ANIMATION (case 0x48C351): `idx = GetAnimationBlockIndex(label)`;
        // `RequestModel(idx + 25575, 4)` (IFP streaming ids; idx not checked for -1);
        // `ScriptResourceManager.AddToResourceManager(idx, 1, this)`. gr: unregistered.
        0x04ED => {
            let buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_ptr().cast_mut());
            read_text_label(s, b, 16);
            let idx = get_animation_block_index(b);
            request_model(idx.wrapping_add(0x63E7), 4);
            add_to_resource_manager(GPtr::from_addr(SCRIPT_RESOURCE_MANAGER), idx, 1, s);
            0
        }
        // 04EE HAS_ANIMATION_LOADED (case 0x48C391): flag = `GetAnimationBlock(label)->bLoaded (+0x10)
        // == 1` (no null check). gr: unregistered.
        0x04EE => {
            let buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_ptr().cast_mut());
            read_text_label(s, b, 16);
            let blk = get_animation_block(b);
            set_flag(s, blk.byte_add(0x10).read() == 1);
            0
        }
        _ => fallback(
            process_commands_1200_to_1299_original,
            "CRunningScript::ProcessCommands1200To1299",
            s,
            command,
        ),
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1200_TO_1299];
