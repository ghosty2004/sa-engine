//! `CRunningScript::ProcessCommands200To299` (0x469390): partial port (PORT-SCRIPT-2, ADR-002). Opcodes
//! in [`PORTED`] run in Rust; every other opcode (and the default, -1) runs the original body through
//! the fallback. Cases follow the 1.0 US bytes (jump tables 0x469E4C/0x469DD4) and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands200To299.md`.
//!
//! Dead stack writes not ported: CREATE_OBJECT keeps its z coordinate in the command-argument slot
//! (`fst [esp+0x50]`), which is dead after `ret 4`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x00EC, 0x00ED, 0x00EE, 0x00EF, 0x00F0, 0x00F1, 0x00F2, 0x00F3, 0x00F4, 0x00FE, 0x00FF, 0x0100, 0x0101,
    0x0102, 0x0103, 0x0104, 0x0105, 0x0106, 0x0107, 0x0108, 0x0109,
];

/// `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8.
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CTheScripts::UsedObjectArray[0].nModelIndex` + 0x1C*0 as the negative-model lookup addresses it
/// (`0xA44B88 - id*0x1C`, id < 0).
const USED_OBJECT_MODEL: usize = 0xA44B88;
/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;
/// `CRunningScript::m_IsExternal` (+0xC7) and `m_ExternalType` (+0xC9).
const IS_EXTERNAL: isize = 0xC7;
const EXTERNAL_TYPE: isize = 0xC9;
/// `CObject::m_nObjectType` (+0x13C).
const OBJECT_TYPE: isize = 0x13C;
/// `CPlayerInfo::m_nMoney` (+0xB8).
const PLAYER_MONEY: isize = 0xB8;

orig! {
    /// `CRunningScript::LocateCharCharCommand(int32 command)` (0x4870F0)
    pub fn locate_char_char_command(s: Scr, command: u32) = thiscall 0x4870F0 => unported;
    /// `CObject::Create(int32 model, bool)` (0x5A1F60)
    pub fn object_create(model: i32, flag: i32) -> GPtr<u8> = cdecl 0x5A1F60 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80)
    pub fn set_orientation(e: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x439A80 => unported;
    /// `CEntity::UpdateRwMatrix()` (0x446F90)
    pub fn update_rw_matrix(e: GPtr<u8>) = thiscall 0x446F90 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    pub fn update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CBaseModelInfo` virtual slot 3 (+0x0C, `AsLodAtomicModelInfoPtr`)
    pub fn model_info_vslot_0c(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x0C => unported;
    /// `CEntity::SetupBigBuilding()` (0x533150)
    pub fn setup_big_building(e: GPtr<u8>) = thiscall 0x533150 => unported;
    /// `CObject` virtual slot 0 (scalar deleting destructor, `delete` flag 1)
    pub fn object_delete(o: GPtr<u8>, flags: i32) = thiscall vslot 0x00 => unported;
}

/// `CRunningScript::ProcessCommands200To299(int32 command)` (0x469390, thiscall, `ret 4`): AL = 0
/// continue, -1 unknown.
#[hook(addr = 0x469390, conv = thiscall, name = "CRunningScript::ProcessCommands200To299", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_200_to_299(s: Scr, command: u32) -> i8 {
    match command {
        // 00EC..00F1, 00FE..0103 LOCATE_(STOPPED_)CHAR_{ANY_MEANS,ON_FOOT,IN_CAR}_{2D,3D} (case
        // 0x469751): `LocateCharCommand(command)`, return 0.
        0x00EC..=0x00F1 | 0x00FE..=0x0103 => {
            locate_char_command(s, command);
            0
        }
        // 00F2..00F4, 0104..0106 LOCATE_CHAR_*_CHAR_{2D,3D} (case 0x469760): `LocateCharCharCommand`.
        0x00F2..=0x00F4 | 0x0104..=0x0106 => {
            locate_char_char_command(s, command);
            0
        }
        0x0107 => {
            create_object(s);
            0
        }
        0x0108 => {
            delete_object(s);
            0
        }
        // 0109 ADD_SCORE (case 0x469922): `Players[SP0].m_nMoney += SP1`.
        0x0109 => {
            collect(s, 2);
            let m = player_info(sp(0)).byte_add(PLAYER_MONEY).cast::<u32>();
            m.write(m.read().wrapping_add(sp(1)));
            0
        }
        _ => fallback(
            process_commands_200_to_299_original,
            "CRunningScript::ProcessCommands200To299",
            s,
            command,
        ),
    }
}

/// 0107 CREATE_OBJECT (case 0x46976F): model (negative = `UsedObjectArray[-id]`), x, y, z. Sets the
/// model info's byte +0x0C to 0xFF, creates the object, object type 2 (mission) unless the script is
/// external or has an external type, ground z when `z <= -100` (ordered), + the distance from the
/// centre of mass to the base (x87, z as memory operand), places it, zero orientation, RW matrix and
/// frame, `SetupBigBuilding` when the model info is an LOD atomic, clears space, adds it to the world,
/// stores the handle, and registers it with the mission cleanup (type 3).
fn create_object(s: Scr) {
    collect(s, 4);
    let mut model = spi(0);
    if model < 0 {
        // imul eax,eax,0x1C; mov ecx,0xA44B88; sub ecx,eax; mov eax,[ecx]
        model =
            GPtr::<i32>::from_addr(USED_OBJECT_MODEL.wrapping_sub(model.wrapping_mul(0x1C) as u32 as usize))
                .read();
    }
    let mi =
        GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS.wrapping_add((model as u32).wrapping_mul(4) as usize))
            .read();
    mi.byte_add(0x0C).write(0xFF);
    let obj = object_create(model, 0);
    let ty = if s.byte_add(IS_EXTERNAL).read() == 0 && s.byte_add(EXTERNAL_TYPE).read() == 0xFF {
        2
    } else {
        6
    };
    obj.byte_add(OBJECT_TYPE).write(ty);
    let mut z = spf(3);
    let y = spf(2);
    let x = spf(1);
    // fcomp -100.0; test ah,0x41; jp skip: ground when z <= -100 (ordered)
    if z <= f(K_MINUS_100) {
        z = X87::from_f64_exact(crate::cmd0::find_ground_z_for_coord(x, y)).to_f32();
    }
    let d = X87::from_f64_exact(crate::cmd0::get_distance_from_centre_of_mass_to_base_of_model(
        obj,
    ));
    let z = d.fadd_m(z).to_f32();
    crate::cmd0::set_posn(obj, x, y, z);
    set_orientation(obj, 0.0, 0.0, 0.0);
    update_rw_matrix(obj);
    update_rw_frame(obj);
    if !model_info_vslot_0c(mi).is_null() {
        setup_big_building(obj);
    }
    let pos = [x, y, z];
    crate::cmd0::clear_space_for_mission_entity(GPtr::from_raw(pos.as_ptr().cast_mut()), obj);
    world_add(obj);
    let h = object_get_ref(pool(OBJECT_POOL), obj);
    set_sp(0, h as u32);
    store(s, 1);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), spi(0), 3);
    }
}

/// 0108 DELETE_OBJECT (case 0x4698C2): removes the object from the world, removes references to it
/// and deletes it (virtual destructor, flag 1) when the handle is valid; then, with mission cleanup,
/// removes `SP0` from the cleanup list (type 3) whether or not the handle was valid.
fn delete_object(s: Scr) {
    collect(s, 1);
    let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
    if !obj.is_null() {
        world_remove(obj);
        world_remove_references_to_deleted_object(obj);
        object_delete(obj, 1);
    }
    if uses_mission_cleanup(s) {
        mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 3);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_200_TO_299];
