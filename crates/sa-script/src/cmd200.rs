//! `CRunningScript::ProcessCommands200To299` (0x469390): complete port (PORT-SCRIPT-2 partial, completed
//! in PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1). Cases follow the 1.0 US bytes (jump tables 0x469E4C/0x469DD4) and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands200To299.md`.
//!
//! Dead stack writes not ported: CREATE_OBJECT keeps its z coordinate in the command-argument slot
//! (`fst [esp+0x50]`), which is dead after `ret 4`; the `setcc`/flag byte cases write the 0/1 flag into
//! that slot before pushing it; CREATE_CHAR_INSIDE_CAR's SEH state and `operator new` result slots.

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
    0x00D6, 0x00D7, 0x00D8, 0x00D9, 0x00DB, 0x00DD, 0x00DF, 0x00E1, 0x00E2, 0x00EC, 0x00ED, 0x00EE, 0x00EF,
    0x00F0, 0x00F1, 0x00F2, 0x00F3, 0x00F4, 0x00FE, 0x00FF, 0x0100, 0x0101, 0x0102, 0x0103, 0x0104, 0x0105,
    0x0106, 0x0107, 0x0108, 0x0109, 0x010A, 0x010B, 0x010C, 0x010D, 0x010E, 0x010F, 0x0110, 0x0111, 0x0112,
    0x0114, 0x0117, 0x0118, 0x0119, 0x0122, 0x0129,
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
/// `CPlayerInfo::m_nPlayerState` (byte +0xDC).
const PLAYER_STATE: isize = 0xDC;
/// `CRunningScript::m_bDeathArrestEnabled` (+0xD3) and `m_bDeathArrestExecuted` (+0xD4).
const DEATH_ARREST_ENABLED: isize = 0xD3;
const DEATH_ARREST_EXECUTED: isize = 0xD4;
/// `CTheScripts::StoreVehicleIndex` 0xA447EC (int32) and `StoreVehicleWasRandomlyCreated` (byte) 0xA447F0.
const STORE_VEHICLE_INDEX: usize = 0xA447EC;
const STORE_VEHICLE_WAS_RANDOM: usize = 0xA447F0;
/// `CPopulation::ms_nTotalMissionPeds` 0xC0EC24.
const TOTAL_MISSION_PEDS: usize = 0xC0EC24;
/// `CTheScripts::ScriptSpace` 0xA49960.
const SCRIPT_SPACE: usize = 0xA49960;
/// `CPed::m_nPedFlags` first dword (+0x46C; bit 8 = bInVehicle) and `m_pVehicle` (+0x58C).
const PED_FLAGS: isize = 0x46C;
const PED_VEHICLE: isize = 0x58C;

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
    /// `CMissionCleanup::Process()` (0x468560)
    fn mission_cleanup_process(m: GPtr<u8>) = thiscall 0x468560 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160), ECX = the vehicle pool
    fn vehicle_get_ref(pool: GPtr<u8>, v: GPtr<u8>) -> i32 = thiscall 0x424160 => unported;
    /// `CCarCtrl::RemoveFromInterestingVehicleList(CVehicle*)` (0x423ED0)
    fn remove_from_interesting_vehicle_list(v: GPtr<u8>) = cdecl 0x423ED0 => unported;
    /// `CVehicle::SetVehicleCreatedBy(int32)` (0x6D5D70)
    fn set_vehicle_created_by(v: GPtr<u8>, by: i32) = thiscall 0x6D5D70 => unported;
    /// `CRunningScript::GetPadState(uint16 pad, uint16 button)` (0x485B10), AX
    fn get_pad_state(s: Scr, pad: u32, button: u32) -> u16 = thiscall 0x485B10 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetHorn()` (0x53FEE0), AL
    fn get_horn(pad: GPtr<u8>) -> u8 = thiscall 0x53FEE0 => unported;
    /// `CPlayerPed::SetWantedLevel(int32)` (0x609F10)
    fn set_wanted_level(p: GPtr<u8>, level: u32) = thiscall 0x609F10 => unported;
    /// `CPlayerPed::SetWantedLevelNoDrop(int32)` (0x609F30)
    fn set_wanted_level_no_drop(p: GPtr<u8>, level: u32) = thiscall 0x609F30 => unported;
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230)
    fn find_player_wanted(i: u32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CPed::GrantAmmo(eWeaponType, uint32)` (0x5DF220)
    fn grant_ammo(p: GPtr<u8>, ty: u32, ammo: u32) = thiscall 0x5DF220 => unported;
    /// `CRunningScript::IsPedDead(CPed*)` (0x464D70), AL
    fn is_ped_dead(s: Scr, p: GPtr<u8>) -> u8 = thiscall 0x464D70 => unported;
    /// `CTaskSimpleCarSetPedInAsDriver::CTaskSimpleCarSetPedInAsDriver(CVehicle*, CTaskUtilityLineUpPedWithCar*)` (0x6470E0)
    fn set_ped_in_as_driver_ctor(t: GPtr<u8>, veh: GPtr<u8>, line_up: GPtr<u8>) -> GPtr<u8> = thiscall 0x6470E0 => unported;
    /// `CTaskSimpleCarSetPedInAsDriver::ProcessPed(CPed*)` (0x64B950), AL
    fn set_ped_in_as_driver_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x64B950 => unported;
    /// `CTaskSimpleCarSetPedInAsDriver::~CTaskSimpleCarSetPedInAsDriver()` (0x647170)
    fn set_ped_in_as_driver_dtor(t: GPtr<u8>) = thiscall 0x647170 => unported;
}

/// `CPed::m_pVehicle` when `m_nPedFlags` bit 8 (in vehicle) is set, else null (`test ch,1`, then the
/// field read; no null check on the ped).
fn vehicle_if_in_one(ped: GPtr<u8>) -> GPtr<u8> {
    if ped.byte_add(PED_FLAGS).cast::<u32>().read() & 0x100 != 0 {
        ped.byte_add(PED_VEHICLE).cast::<GPtr<u8>>().read()
    } else {
        GPtr::NULL
    }
}

/// `CRunningScript::ProcessCommands200To299(int32 command)` (0x469390, thiscall, `ret 4`): AL = 0
/// continue, -1 unknown.
#[hook(addr = 0x469390, conv = thiscall, name = "CRunningScript::ProcessCommands200To299", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_200_to_299(s: Scr, command: u32) -> i8 {
    match command {
        // 00D6 ANDOR (case 0x4693CF): `m_AndOrState` (+0xD0, word) = (u16)SP0; 0 -> m_CondResult = 0;
        // 1..8 -> state + 1, m_CondResult = 1; 21..28 -> state + 1, m_CondResult = 0; any other value
        // returns 0 with the state left at SP0 and m_CondResult unchanged (gr: unreachable).
        0x00D6 => {
            collect(s, 1);
            let v = sp(0) as u16;
            let st = s.byte_add(crate::vm::AND_OR_STATE).cast::<u16>();
            let cond = s.byte_add(crate::vm::COND_RESULT);
            st.write(v);
            if v == 0 {
                cond.write(0);
            } else if (1..=8).contains(&v) {
                st.write(v + 1);
                cond.write(1);
            } else if (0x15..=0x1C).contains(&v) {
                st.write(v + 1);
                cond.write(0);
            }
            0
        }
        // 00D7 LAUNCH_MISSION (case 0x46943A): `StartNewScript(&ScriptSpace[SP0])` (result discarded).
        0x00D7 => {
            collect(s, 1);
            crate::cmd0::start_new_script(GPtr::from_addr(sp(0).wrapping_add(SCRIPT_SPACE as u32) as usize));
            0
        }
        // 00D8 MISSION_HAS_FINISHED (case 0x46945E): no operands; `MissionCleanUp.Process()` when the
        // script uses the mission cleanup. gr: unregistered.
        0x00D8 => {
            if uses_mission_cleanup(s) {
                mission_cleanup_process(GPtr::from_addr(MISSION_CLEANUP));
            }
            0
        }
        0x00D9 => {
            store_car_char_is_in(s);
            0
        }
        // 00DB IS_CHAR_IN_CAR (case 0x4695A9): no null check on the ped; flag = in a vehicle && it is the
        // vehicle `GetAtRef(SP1)` (both lookups done first).
        0x00DB => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let v = vehicle_if_in_one(ped);
            set_flag(s, !v.is_null() && v == veh);
            0
        }
        // 00DD IS_CHAR_IN_MODEL (case 0x46960E): no null check; flag = in a vehicle && (int16)its
        // `m_nModelIndex` (+0x22, `movsx`) == SP1.
        0x00DD => {
            collect(s, 2);
            let v = vehicle_if_in_one(ped_at_ref(pool(PED_POOL), spi(0)));
            let b = !v.is_null() && i32::from(v.byte_add(0x22).cast::<u16>().read() as i16) == spi(1);
            set_flag(s, b);
            0
        }
        // 00DF IS_CHAR_IN_ANY_CAR (case 0x469667): no null check; flag = in a vehicle && m_pVehicle != 0.
        0x00DF => {
            collect(s, 1);
            let v = vehicle_if_in_one(ped_at_ref(pool(PED_POOL), spi(0)));
            set_flag(s, !v.is_null());
            0
        }
        // 00E1 IS_BUTTON_PRESSED (case 0x4696B8): flag = GetPadState((u16)SP0, (u16)SP1) != 0 (AX) &&
        // `CPad::GetPad(0)->JustOutOfFrontEnd` (byte +0x117) == 0.
        0x00E1 => {
            collect(s, 2);
            let r = get_pad_state(s, sp(0) & 0xFFFF, sp(1) & 0xFFFF);
            let b = r != 0 && get_pad(0).byte_add(0x117).read() == 0;
            set_flag(s, b);
            0
        }
        // 00E2 GET_PAD_STATE (case 0x469715): SP0 = (int16)GetPadState((u16)SP0, (u16)SP1) (`movsx`);
        // StoreParameters(1). gr: unregistered.
        0x00E2 => {
            collect(s, 2);
            let r = get_pad_state(s, sp(0) & 0xFFFF, sp(1) & 0xFFFF);
            set_sp(0, i32::from(r as i16) as u32);
            store(s, 1);
            0
        }
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
        // 010A IS_SCORE_GREATER (case 0x46994E): flag = (int32)Players[SP0].m_nMoney > SP1 (`setg`).
        0x010A => {
            collect(s, 2);
            let m = player_info(sp(0)).byte_add(PLAYER_MONEY).cast::<u32>().read() as i32;
            set_flag(s, m > spi(1));
            0
        }
        // 010B STORE_SCORE (case 0x46998A): SP0 = Players[SP0].m_nMoney; StoreParameters(1).
        0x010B => {
            collect(s, 1);
            set_sp(0, player_info(sp(0)).byte_add(PLAYER_MONEY).cast::<u32>().read());
            store(s, 1);
            0
        }
        // 010C GIVE_REMOTE_CONTROLLED_CAR_TO_PLAYER (case 0x469DB9): straight to the epilogue, operands not
        // read, returns 0.
        0x010C => 0,
        // 010D ALTER_WANTED_LEVEL (case 0x4699BB) / 010E ALTER_WANTED_LEVEL_NO_DROP (case 0x4699E8):
        // Players[SP0].m_pPed->SetWantedLevel / SetWantedLevelNoDrop(SP1) (no null check).
        0x010D | 0x010E => {
            collect(s, 2);
            let ped = player_info(sp(0)).cast::<GPtr<u8>>().read();
            if command == 0x010D {
                set_wanted_level(ped, sp(1));
            } else {
                set_wanted_level_no_drop(ped, sp(1));
            }
            0
        }
        // 010F IS_WANTED_LEVEL_GREATER (case 0x469A15): flag = (int32)FindPlayerWanted(SP0)->m_nWantedLevel
        // (+0x2C) > SP1 (`setg`).
        0x010F => {
            collect(s, 2);
            let w = find_player_wanted(sp(0)).byte_add(0x2C).cast::<u32>().read() as i32;
            set_flag(s, w > spi(1));
            0
        }
        // 0110 CLEAR_WANTED_LEVEL (case 0x469A52): Players[SP0].m_pPed->SetWantedLevel(0).
        0x0110 => {
            collect(s, 1);
            set_wanted_level(player_info(sp(0)).cast::<GPtr<u8>>().read(), 0);
            0
        }
        // 0111 SET_DEATHARREST_STATE (case 0x469A7B): m_bDeathArrestEnabled (+0xD3) = SP0 == 1. gr:
        // unregistered.
        0x0111 => {
            collect(s, 1);
            s.byte_add(DEATH_ARREST_ENABLED).write(u8::from(sp(0) == 1));
            0
        }
        // 0112 HAS_DEATHARREST_BEEN_EXECUTED (case 0x469A9B): no operands; flag = m_bDeathArrestExecuted
        // (+0xD4) != 0. gr: unregistered.
        0x0112 => {
            let b = s.byte_add(DEATH_ARREST_EXECUTED).read() != 0;
            set_flag(s, b);
            0
        }
        // 0114 ADD_AMMO_TO_CHAR (case 0x469AA8): no null check; GrantAmmo(SP1, SP2).
        0x0114 => {
            collect(s, 3);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            grant_ammo(ped, sp(1), sp(2));
            0
        }
        // 0117 IS_PLAYER_DEAD (case 0x469ADE): flag = Players[SP0].m_nPlayerState (byte +0xDC) == 1.
        0x0117 => {
            collect(s, 1);
            let b = player_info(sp(0)).byte_add(PLAYER_STATE).read() == 1;
            set_flag(s, b);
            0
        }
        // 0118 IS_CHAR_DEAD (case 0x469B13): null ped -> 1; else IsPedDead(ped) != 0.
        0x0118 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let b = ped.is_null() || is_ped_dead(s, ped) != 0;
            set_flag(s, b);
            0
        }
        // 0119 IS_CAR_DEAD (case 0x469B6D): null vehicle or status WRECKED (`(byte +0x36 & 0xF8) == 0x28`)
        // -> 1; else bit 6 of byte +0x42B (bIsDrowning). gr: unregistered.
        0x0119 => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let b = v.is_null()
                || v.byte_add(0x36).read() & 0xF8 == 0x28
                || (v.byte_add(0x42B).read() >> 6) & 1 != 0;
            set_flag(s, b);
            0
        }
        // 0122 IS_PLAYER_PRESSING_HORN (case 0x469BD1): no null check on Players[SP0].m_pPed; when its
        // `m_nPedState` (+0x530) == 0x32 (DRIVING), flag = CPad::GetPad(SP0)->GetHorn() != 0, else 0.
        0x0122 => {
            collect(s, 1);
            let ped = player_info(sp(0)).cast::<GPtr<u8>>().read();
            let b = if ped.byte_add(0x530).cast::<u32>().read() == 0x32 {
                get_horn(get_pad(spi(0))) != 0
            } else {
                false
            };
            set_flag(s, b);
            0
        }
        0x0129 => {
            create_char_inside_car(s);
            0
        }
        _ => -1,
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

/// 00D9 STORE_CAR_CHAR_IS_IN (case 0x46947D): no null check on the ped; `veh` = the ped's vehicle when
/// in one, else null, and `h = VehiclePool->GetRef(veh)` is computed for null too (gr throws). When `h !=
/// StoreVehicleIndex` and the script uses the mission cleanup: the stored vehicle (`GetAtRef`, no `-1`
/// check, gr: bugfix) is removed from the interesting list and, when created by a mission (2) and
/// `StoreVehicleWasRandomlyCreated`, set back to random (1), unlocked (byte +0x428 &= 0xF7) and removed
/// from the cleanup (type 1); `StoreVehicleIndex = h`; then by `veh->m_nCreatedBy` (+0x4A4, read
/// without a null check): 1/3 -> SetVehicleCreatedBy(2), the random flag = 1 (written before the call),
/// cleanup add (type 1); 2/4 -> the random flag = 0; others nothing. SP0 = StoreVehicleIndex;
/// StoreParameters(1).
fn store_car_char_is_in(s: Scr) {
    collect(s, 1);
    let ped = ped_at_ref(pool(PED_POOL), spi(0));
    let veh = vehicle_if_in_one(ped);
    let vp = pool(VEHICLE_POOL);
    let h = vehicle_get_ref(vp, veh);
    let idx = GPtr::<i32>::from_addr(STORE_VEHICLE_INDEX);
    let random = GPtr::<u8>::from_addr(STORE_VEHICLE_WAS_RANDOM);
    let cleanup = GPtr::from_addr(MISSION_CLEANUP);
    if h != idx.read() && uses_mission_cleanup(s) {
        let old = vehicle_at_ref(vp, idx.read());
        if !old.is_null() {
            remove_from_interesting_vehicle_list(old);
            if old.byte_add(0x4A4).read() == 2 && random.read() != 0 {
                set_vehicle_created_by(old, 1);
                let fl = old.byte_add(0x428);
                fl.write(fl.read() & 0xF7);
                mission_cleanup_remove(cleanup, idx.read(), 1);
            }
        }
        idx.write(h);
        match veh.byte_add(0x4A4).read() {
            1 | 3 => {
                set_vehicle_created_by(veh, 2);
                random.write(1);
                mission_cleanup_add(cleanup, idx.read(), 1);
            }
            2 | 4 => random.write(0),
            _ => {}
        }
    }
    set_sp(0, idx.read() as u32);
    store(s, 1);
}

/// 0129 CREATE_CHAR_INSIDE_CAR (case 0x469C3A): vehicle `GetAtRef(SP0)` (no null check), ped type SP1,
/// model SP2 (fixed up by `GetCorrectPedModelIndexForEmergencyServiceType(SP1, &SP2)`). Type 6 ->
/// `CCopPed(SP2)` (size 0x7C4), 0x12/0x13 -> `CEmergencyPed(SP1, SP2)`, else `CCivilianPed(SP1, SP2)`
/// (0x79C; a null allocation leaves the ped null as in the original). Created by mission (2), bit 31 of
/// +0x46C cleared; a stack `CTaskSimpleCarSetPedInAsDriver(veh, nullptr)` (0x1C bytes) with
/// `m_bIsWarpingPedIntoCar` (+0x18) = 1 set after the constructor (as 036A; gr passes it to the
/// constructor), ProcessPed(ped), destructor; CWorld::Add; mission ped count + 1; handle stored;
/// cleanup add (type 2) with mission cleanup. Unlike CREATE_CHAR (009A) no primary task, position or
/// 0x40000 flag. SEH state stores are not ported.
fn create_char_inside_car(s: Scr) {
    collect(s, 3);
    let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
    crate::cmd100::get_correct_ped_model_index(s, spi(1), param(2));
    let ped = match spi(1) {
        6 => {
            let p = crate::cmd100::ped_new(0x7C4);
            if p.is_null() {
                GPtr::NULL
            } else {
                crate::cmd100::cop_ped_ctor(p, sp(2))
            }
        }
        0x12 | 0x13 => {
            let p = crate::cmd100::ped_new(0x79C);
            if p.is_null() {
                GPtr::NULL
            } else {
                crate::cmd100::emergency_ped_ctor(p, sp(1), sp(2))
            }
        }
        _ => {
            let p = crate::cmd100::ped_new(0x79C);
            if p.is_null() {
                GPtr::NULL
            } else {
                crate::cmd100::civilian_ped_ctor(p, sp(1), sp(2))
            }
        }
    };
    crate::cmd0::set_char_created_by(ped, 2);
    let fl = ped.byte_add(PED_FLAGS).cast::<u32>();
    fl.write(fl.read() & 0x7FFF_FFFF);
    let mut task = [0u32; 7];
    let t = GPtr::from_raw(task.as_mut_ptr()).cast::<u8>();
    set_ped_in_as_driver_ctor(t, veh, GPtr::NULL);
    t.byte_add(0x18).write(1);
    set_ped_in_as_driver_process_ped(t, ped);
    set_ped_in_as_driver_dtor(t);
    world_add(ped);
    let n = GPtr::<u32>::from_addr(TOTAL_MISSION_PEDS);
    n.write(n.read().wrapping_add(1));
    let h = ped_get_ref(pool(PED_POOL), ped);
    set_sp(0, h as u32);
    store(s, 1);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), spi(0), 2);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_200_TO_299];
