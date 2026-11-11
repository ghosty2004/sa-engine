//! `CRunningScript::ProcessCommands600To699` (0x47F370): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1). Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands600To699.md`.
//!
//! Divergence note (0299/02B9): the 8-byte garage name is read into a zeroed 16-byte buffer; the
//! original's buffer (`[esp+0x18]`) is not cleared, so an 8-character name without a terminator is
//! followed by stale stack bytes in `GetGarageNumberByName`. Differs only for 8-character names.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0293, 0x0294, 0x0296, 0x0297, 0x0298, 0x0299, 0x029B, 0x02A0, 0x02A3, 0x02A4, 0x02A5, 0x02A7, 0x02A8,
    0x02A9, 0x02AA, 0x02AB, 0x02AC, 0x02B9,
];

/// `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8.
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CTheScripts::UsedObjectArray[0].nModelIndex` as the negative-model lookup addresses it
/// (`0xA44B88 - id*0x1C`, id < 0).
const USED_OBJECT_MODEL: usize = 0xA44B88;

/// `CTheScripts::ScriptResourceManager` 0xA485A8.
const SCRIPT_RESOURCE_MANAGER: usize = 0xA485A8;

/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;

orig! {
    /// `CScriptResourceManager::RemoveFromResourceManager(int32, uint32 type, CRunningScript*)` (0x470510), AL
    fn remove_from_resource_manager(m: GPtr<u8>, model: i32, ty: u32, s: Scr) -> u8 = thiscall 0x470510 => unported;
    /// `CStreaming::SetMissionDoesntRequireSpecialChar(int32 slot)` (0x40B490)
    fn set_mission_doesnt_require_special_char(slot: i32) = cdecl 0x40B490 => unported;
    /// `CDarkel::ResetModelsKilledByPlayer(int32 player)` (0x43D6A0)
    fn reset_models_killed_by_player(player: i32) = cdecl 0x43D6A0 => unported;
    /// `CDarkel::QueryModelsKilledByPlayer(int32 model, int32 player)` (0x43D6C0), AX
    fn query_models_killed_by_player(model: i32, player: i32) -> u16 = cdecl 0x43D6C0 => unported;
    /// `CTheScripts::IsPedStopped(CPed*)` (0x486110), AL
    fn is_ped_stopped(p: GPtr<u8>) -> u8 = cdecl 0x486110 => unported;
    /// `CRunningScript::CollectNextParameterWithoutIncreasingPC()` (0x464250)
    fn collect_next(s: Scr) -> i32 = thiscall 0x464250 => crate::vm::collect_next_parameter_without_increasing_pc;
    /// `CCamera::SetWideScreenOn()` (0x50C140)
    fn set_wide_screen_on(cam: GPtr<u8>) = thiscall 0x50C140 => unported;
    /// `CCamera::SetWideScreenOff()` (0x50C150)
    fn set_wide_screen_off(cam: GPtr<u8>) = thiscall 0x50C150 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (read as f64: exact)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CRadar::GetActualBlipArrayIndex(int32)` (0x582870); the result is discarded here
    fn get_actual_blip_array_index(h: i32) -> i32 = cdecl 0x582870 => unported;
    /// `CRadar::SetCoordBlip(eBlipType, CVector, uint32 colour, eBlipDisplay, const char*)` (0x583820);
    /// the by-value vector as three raw dwords
    fn set_coord_blip(ty: i32, x: u32, y: u32, z: u32, colour: u32, display: u32, name: GPtr<u8>) -> i32 = cdecl 0x583820 => unported;
    /// `CRadar::SetBlipSprite(int32, int32)` (0x583D70)
    fn set_blip_sprite(h: i32, sprite: i32) = cdecl 0x583D70 => unported;
    /// `CGarages::GetGarageNumberByName(const char*)` (0x447680), AX
    fn get_garage_number_by_name(name: GPtr<u8>) -> u16 = cdecl 0x447680 => unported;
    /// `CGarages::ActivateGarage(int16)` (0x447CD0)
    fn activate_garage(n: i32) = cdecl 0x447CD0 => unported;
    /// `CGarages::DeActivateGarage(int16)` (0x447CB0)
    fn deactivate_garage(n: i32) = cdecl 0x447CB0 => unported;
    /// `CObject::Create(int32 model, bool)` (0x5A1F60)
    fn object_create(model: i32, flag: i32) -> GPtr<u8> = cdecl 0x5A1F60 => unported;
    /// `CPlaceable::SetPosn(float, float, float)` (0x420B80), floats as raw dwords
    fn set_posn(e: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x420B80 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80)
    fn set_orientation(e: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x439A80 => unported;
    /// `CEntity::UpdateRwMatrix()` (0x446F90)
    fn update_rw_matrix(e: GPtr<u8>) = thiscall 0x446F90 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CBaseModelInfo` virtual slot 3 (+0x0C, `AsLodAtomicModelInfoPtr`)
    fn model_info_vslot_0c(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x0C => unported;
    /// `CEntity::SetupBigBuilding()` (0x533150)
    fn setup_big_building(e: GPtr<u8>) = thiscall 0x533150 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    fn clear_space_for_mission_entity(pos: GPtr<u32>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
}

/// 0299 ACTIVATE_GARAGE / 02B9 DEACTIVATE_GARAGE body: `ReadTextLabelFromScript(buf, 8)`;
/// `n = (int16)GetGarageNumberByName(buf)` (`movsx`); `n >= 0` → (De)ActivateGarage(n).
fn garage_by_name(s: Scr, activate: bool) {
    let mut buf = [0u8; 16];
    let b = GPtr::from_raw(buf.as_mut_ptr());
    read_text_label(s, b, 8);
    let n = i32::from(get_garage_number_by_name(b) as i16);
    if n >= 0 {
        if activate {
            activate_garage(n);
        } else {
            deactivate_garage(n);
        }
    }
}

/// 029B CREATE_OBJECT_NO_OFFSET (0x47F4F1): as 0107 CREATE_OBJECT without the base-of-model offset.
/// `model < 0` → `UsedObjectArray[-model]`; `mi->m_nAlpha = 0xFF`; `CObject::Create(model, false)`
/// (no null check); object type 2 (mission) for a non-external script, else 6; `fld z; fst; fcomp
/// -100.0`: z = ground z when `z <= -100` (ordered); SetPosn(x, y, z), SetOrientation(0, 0, 0),
/// UpdateRwMatrix, UpdateRwFrame, SetupBigBuilding when the model info is a LOD atomic;
/// ClearSpaceForMissionEntity(&(x, y, z)), CWorld::Add, P0 = GetRef, StoreParameters(1), mission
/// cleanup type 3. x and y are passed as raw dwords (`mov` loads); z went through `fld`/`fst`.
fn create_object_no_offset(s: Scr) {
    collect(s, 4);
    let mut model = spi(0);
    if model < 0 {
        model =
            GPtr::<i32>::from_addr(USED_OBJECT_MODEL.wrapping_sub(model.wrapping_mul(0x1C) as u32 as usize))
                .read();
    }
    let mi =
        GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS.wrapping_add((model as u32).wrapping_mul(4) as usize))
            .read();
    mi.byte_add(0x0C).write(0xFF);
    let obj = object_create(model, 0);
    let ty = if s.byte_add(0xC7).read() == 0 && s.byte_add(0xC9).read() == 0xFF {
        2
    } else {
        6
    };
    obj.byte_add(0x13C).write(ty);
    let st = X87::from_f32(f(sp(3)));
    let (x, y) = (sp(1), sp(2));
    let mut z = st.to_f32().to_bits();
    if !st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        z = X87::from_f64_exact(find_ground_z_for_coord(x, y))
            .to_f32()
            .to_bits();
    }
    set_posn(obj, x, y, z);
    set_orientation(obj, 0, 0, 0);
    update_rw_matrix(obj);
    update_rw_frame(obj);
    if !model_info_vslot_0c(mi).is_null() {
        setup_big_building(obj);
    }
    let mut pos = [x, y, z];
    clear_space_for_mission_entity(GPtr::from_raw(pos.as_mut_ptr()), obj);
    world_add(obj);
    let h = object_get_ref(pool(OBJECT_POOL), obj);
    set_sp(0, h as u32);
    store(s, 1);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), spi(0), 3);
    }
}

/// 02A7 / 02A8 body: `fld z; fst local; fcomp -100.0; test ah,0x41; jp`: when `z <= -100` (ordered)
/// local z = FindGroundZForCoord(x, y) (`fstp`); discarded GetActualBlipArrayIndex of the output
/// operand; h = SetCoordBlip(ty, (x, y, z), colour, 3 BOTH, m_szName); SetBlipSprite(h, P3); P0 = h;
/// StoreParameters(1) (shared tail 0x47F7FB).
fn sprite_blip_for_coord(s: Scr, ty: i32, colour: u32) {
    collect(s, 4);
    let (x, y) = (sp(0), sp(1));
    let st = X87::from_f32(f(sp(2)));
    let mut z = st.to_f32().to_bits();
    if !st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        z = X87::from_f64_exact(find_ground_z_for_coord(x, y))
            .to_f32()
            .to_bits();
    }
    get_actual_blip_array_index(collect_next(s));
    let h = set_coord_blip(ty, x, y, z, colour, 3, s.byte_add(8));
    set_blip_sprite(h, spi(3));
    set_sp(0, h as u32);
    store(s, 1);
}

/// `e->m_nPhysicalFlags` (+0x40) bit `mask` = `v != 0` (one read-modify-write per flag, as compiled).
#[inline]
fn set_physical_flag(e: GPtr<u8>, mask: u32, v: u32) {
    let p = e.byte_add(0x40).cast::<u32>();
    let old = p.read();
    p.write(if v != 0 { old | mask } else { old & !mask });
}

/// 02AB / 02AC body (shared tail 0x47F8AA): bullet (0x40000) = P1, fire (0x80000) = P2, explosion
/// (0x800000) = P3, collision (0x100000) = P4, melee (0x200000) = P5. No null check.
fn set_proofs(e: GPtr<u8>) {
    set_physical_flag(e, 0x40000, sp(1));
    set_physical_flag(e, 0x80000, sp(2));
    set_physical_flag(e, 0x800000, sp(3));
    set_physical_flag(e, 0x100000, sp(4));
    set_physical_flag(e, 0x200000, sp(5));
}

/// `CRunningScript::ProcessCommands600To699(int32 command)` (0x47F370, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x47F370, conv = thiscall, name = "CRunningScript::ProcessCommands600To699", expect = "8B 44 24 04 83 EC 2C", this = "CRunningScript")]
pub fn process_commands_600_to_699(s: Scr, command: u32) -> i8 {
    match command {
        // 0293 GET_CONTROLLER_MODE (0x47F399): P0 = 0; StoreParameters(1).
        0x0293 => {
            set_sp(0, 0);
            store(s, 1);
        }
        // 0294 SET_CAN_RESPRAY_CAR (0x47F3B8): no null check; byte +0x868 bit 1 (0x2) = P1 == 0.
        0x0294 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let p = veh.byte_add(0x868);
            let b = p.read();
            p.write(if sp(1) != 0 { b & 0xFD } else { b | 0x2 });
        }
        // 0296 UNLOAD_SPECIAL_CHARACTER (0x47F40D): ScriptParams[0] is decremented in place; when
        // ScriptResourceManager.RemoveFromResourceManager(P0 + 0x122, 2, this) returns true,
        // SetMissionDoesntRequireSpecialChar(P0) (re-read).
        0x0296 => {
            collect(s, 1);
            let slot = sp(0).wrapping_sub(1);
            set_sp(0, slot);
            let m = slot.wrapping_add(0x122) as i32;
            if remove_from_resource_manager(GPtr::from_addr(SCRIPT_RESOURCE_MANAGER), m, 2, s) != 0 {
                set_mission_doesnt_require_special_char(spi(0));
            }
        }
        // 0297 RESET_NUM_OF_MODELS_KILLED_BY_PLAYER (0x47F457)
        0x0297 => {
            collect(s, 1);
            reset_models_killed_by_player(spi(0));
        }
        // 0298 GET_NUM_OF_MODELS_KILLED_BY_PLAYER (0x47F47A): P0 = (int16)QueryModelsKilledByPlayer(P1, P0)
        // (`movsx`).
        0x0298 => {
            collect(s, 2);
            let n = query_models_killed_by_player(spi(1), spi(0));
            set_sp(0, i32::from(n as i16) as u32);
            store(s, 1);
        }
        // 0299 ACTIVATE_GARAGE (0x47F4B6)
        0x0299 => garage_by_name(s, true),
        // 029B CREATE_OBJECT_NO_OFFSET (0x47F4F1)
        0x029B => create_object_no_offset(s),
        // 02A0 IS_CHAR_STOPPED (0x47F63A): flag = CTheScripts::IsPedStopped(GetAtRef(P0)) != 0.
        0x02A0 => {
            collect(s, 1);
            let b = is_ped_stopped(ped_at_ref(pool(PED_POOL), spi(0))) != 0;
            set_flag(s, b);
        }
        // 02A3 SWITCH_WIDESCREEN (0x47F684)
        0x02A3 => {
            collect(s, 1);
            let cam = GPtr::from_addr(THE_CAMERA);
            if sp(0) != 0 {
                set_wide_screen_on(cam);
            } else {
                set_wide_screen_off(cam);
            }
        }
        // 02A4 ADD_SPRITE_BLIP_FOR_CAR / 02A5 ADD_SPRITE_BLIP_FOR_CHAR: the jump table sends both to the
        // "return 0" epilogue 0x47F9A1 (no operand read; gr: unimplemented, -1).
        0x02A4 | 0x02A5 => {}
        // 02A7 ADD_SPRITE_BLIP_FOR_CONTACT_POINT (0x47F6BD): SetCoordBlip(5 CONTACT_POINT, .., colour 2, 3).
        0x02A7 => sprite_blip_for_coord(s, 5, 2),
        // 02A8 ADD_SPRITE_BLIP_FOR_COORD (0x47F75D): SetCoordBlip(4 COORD, .., colour 5, 3).
        0x02A8 => sprite_blip_for_coord(s, 4, 5),
        // 02A9 SET_CHAR_ONLY_DAMAGED_BY_PLAYER (0x47F821) / 02AA SET_CAR_ONLY_DAMAGED_BY_PLAYER
        // (0x47F85E, shared tail 0x47F83C): physical flag 0x400000 = P1; no null check.
        0x02A9 | 0x02AA => {
            collect(s, 2);
            let e = if command == 0x02A9 {
                ped_at_ref(pool(PED_POOL), spi(0))
            } else {
                vehicle_at_ref(pool(VEHICLE_POOL), spi(0))
            };
            set_physical_flag(e, 0x400000, sp(1));
        }
        // 02AB SET_CHAR_PROOFS (0x47F88F)
        0x02AB => {
            collect(s, 6);
            set_proofs(ped_at_ref(pool(PED_POOL), spi(0)));
        }
        // 02AC SET_CAR_PROOFS (0x47F908)
        0x02AC => {
            collect(s, 6);
            set_proofs(vehicle_at_ref(pool(VEHICLE_POOL), spi(0)));
        }
        // 02B9 DEACTIVATE_GARAGE (0x47F976)
        0x02B9 => garage_by_name(s, false),
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_600_TO_699];
