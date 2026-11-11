//! `CRunningScript::ProcessCommands100To199` (0x466DE0): partial port (PORT-SCRIPT-2, ADR-002). Opcodes
//! in [`PORTED`] run in Rust; every other opcode (and the default, -1) runs the original body through
//! the fallback. Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands100To199.md`.
//!
//! Dead stack writes not ported: CREATE_CHAR's EH state (`[esp+0x48]`) and the `operator new` result
//! slot (`[esp+0x50]`, the command-argument slot) are only read by the switch's unwind funclet.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x009A, 0x00A5, 0x00A6, 0x00BE, 0x00C0];

/// `.rdata` 0x858624: 1.0f
const K_ONE: u32 = 0x3F80_0000;
/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;
/// `CPopulation::ms_nTotalMissionPeds` 0xC0EC24.
const TOTAL_MISSION_PEDS: usize = 0xC0EC24;

orig! {
    /// `CRunningScript::GetCorrectPedModelIndexForEmergencyServiceType(ePedType, int32*)` (0x464F50)
    pub fn get_correct_ped_model_index(s: Scr, ped_type: i32, model: GPtr<u32>) = thiscall 0x464F50 => unported;
    /// `CPed::operator new(uint32)` (0x5E4720)
    pub fn ped_new(size: u32) -> GPtr<u8> = cdecl 0x5E4720 => unported;
    /// `CCopPed::CCopPed(eCopType)` (0x5DDC60), returns this
    pub fn cop_ped_ctor(p: GPtr<u8>, cop_type: u32) -> GPtr<u8> = thiscall 0x5DDC60 => unported;
    /// `CCivilianPed::CCivilianPed(ePedType, uint32 model)` (0x5DDB70), returns this
    pub fn civilian_ped_ctor(p: GPtr<u8>, ped_type: u32, model: u32) -> GPtr<u8> = thiscall 0x5DDB70 => unported;
    /// `CEmergencyPed::CEmergencyPed(ePedType, uint32 model)` (0x5DE340), returns this
    pub fn emergency_ped_ctor(p: GPtr<u8>, ped_type: u32, model: u32) -> GPtr<u8> = thiscall 0x5DE340 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32 time, bool looped, bool hold, float blend)` (0x62F310)
    pub fn task_simple_stand_still_ctor(t: GPtr<u8>, time: i32, looped: i32, hold: i32, blend: f32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    pub fn set_posn_v(e: GPtr<u8>, pos: GPtr<f32>) = thiscall 0x4241C0 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80)
    pub fn set_orientation(e: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x439A80 => unported;
    /// `CCarCtrl::CreateCarForScript(int32 model, CVector pos, bool mission)` (0x431F80): the vector
    /// is passed by value (three stack words)
    pub fn create_car_for_script(model: u32, x: u32, y: u32, z: u32, mission: u32) -> GPtr<u8> = cdecl 0x431F80 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160), ECX = the vehicle pool
    pub fn vehicle_get_ref(pool: GPtr<u8>, v: GPtr<u8>) -> i32 = thiscall 0x424160 => unported;
    /// `CVehicle` virtual slot 0 (scalar deleting destructor, flag 1)
    pub fn vehicle_delete(v: GPtr<u8>, flags: i32) = thiscall vslot 0x00 => unported;
    /// `CMessages::ClearMessages(bool)` (0x69DCD0)
    pub fn clear_messages(flag: i32) = cdecl 0x69DCD0 => unported;
    /// `CClock::SetGameClock(uint8 h, uint8 m, uint8 day)` (0x52D150)
    pub fn set_game_clock(h: u32, m: u32, day: u32) = cdecl 0x52D150 => unported;
}

/// `CRunningScript::ProcessCommands100To199(int32 command)` (0x466DE0, thiscall, `ret 4`): AL = 0
/// continue, -1 unknown.
#[hook(addr = 0x466DE0, conv = thiscall, name = "CRunningScript::ProcessCommands100To199", expect = "64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_100_to_199(s: Scr, command: u32) -> i8 {
    match command {
        0x009A => {
            create_char(s);
            0
        }
        // 00A5 CREATE_CAR (case 0x467AB3): `CCarCtrl::CreateCarForScript(SP0, {SP1, SP2, SP3},
        // m_UsesMissionCleanup)`, handle stored (shared tail 0x468294). gr: unregistered.
        0x00A5 => {
            collect(s, 4);
            let mission = u32::from(s.byte_add(crate::vm::USES_MISSION_CLEANUP).read());
            let v = create_car_for_script(sp(0), sp(1), sp(2), sp(3), mission);
            let h = vehicle_get_ref(pool(VEHICLE_POOL), v);
            set_sp(0, h as u32);
            store(s, 1);
            0
        }
        // 00A6 DELETE_CAR (case 0x467B1A): remove from the world, remove references, delete (vslot 0,
        // flag 1) when the handle is valid; with mission cleanup remove SP0 (type 1) regardless.
        // gr: unregistered.
        0x00A6 => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            if !v.is_null() {
                world_remove(v);
                world_remove_references_to_deleted_object(v);
                vehicle_delete(v, 1);
            }
            if uses_mission_cleanup(s) {
                mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 1);
            }
            0
        }
        // 00BE CLEAR_PRINTS (case 0x4681E2): `CMessages::ClearMessages(false)`.
        0x00BE => {
            clear_messages(0);
            0
        }
        // 00C0 SET_TIME_OF_DAY (case 0x468238): `CClock::SetGameClock((uint8)SP0, (uint8)SP1, 0)`.
        0x00C0 => {
            collect(s, 2);
            set_game_clock(sp(0) & 0xFF, sp(1) & 0xFF, 0);
            0
        }
        _ => fallback(
            process_commands_100_to_199_original,
            "CRunningScript::ProcessCommands100To199",
            s,
            command,
        ),
    }
}

/// 009A CREATE_CHAR (case 0x46754F): ped type SP0, model SP1 (fixed up by
/// `GetCorrectPedModelIndexForEmergencyServiceType`), x/y/z SP2..SP4. Type 6 → `CCopPed(SP1)`, 0x12/0x13 →
/// `CEmergencyPed(SP0, SP1)`, else `CCivilianPed(SP0, SP1)` (each after `CPed::operator new`, size 0x7C4 /
/// 0x79C; a null allocation leaves the ped null as in the original). Primary task 4 =
/// `CTaskSimpleStandStill(999999, true, false, 8.0)`, created by mission script (2), bit 31 of +0x46C
/// cleared, z = (ground z when z <= -100, ordered, else z) + 1.0 in x87, `SetPosn`, zero orientation,
/// clear space, flag 0x40000 in +0x1C with mission cleanup, world add, mission ped count + 1, handle
/// stored, mission cleanup entry (type 2).
fn create_char(s: Scr) {
    collect(s, 5);
    get_correct_ped_model_index(s, spi(0), param(1));
    let ped = match spi(0) {
        6 => {
            let p = ped_new(0x7C4);
            if p.is_null() {
                GPtr::NULL
            } else {
                cop_ped_ctor(p, sp(1))
            }
        }
        0x12 | 0x13 => {
            let p = ped_new(0x79C);
            if p.is_null() {
                GPtr::NULL
            } else {
                emergency_ped_ctor(p, sp(0), sp(1))
            }
        }
        _ => {
            let p = ped_new(0x79C);
            if p.is_null() {
                GPtr::NULL
            } else {
                civilian_ped_ctor(p, sp(0), sp(1))
            }
        }
    };
    let t = crate::cmd0::task_new(0x20);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        task_simple_stand_still_ctor(t, 999_999, 1, 0, 8.0)
    };
    let intel = ped.byte_add(0x47C).cast::<GPtr<u8>>().read();
    crate::cmd0::task_manager_set_task(intel.byte_add(4), t, 4, 0);
    crate::cmd0::set_char_created_by(ped, 2);
    let fl = ped.byte_add(0x46C).cast::<u32>();
    fl.write(fl.read() & 0x7FFF_FFFF);
    let z = spf(4);
    let y = sp(3);
    let x = sp(2);
    // fld z; fcom -100.0; test ah,0x41; jp keep: ground when z <= -100 (ordered); then fadd 1.0
    let st0 = if z <= f(K_MINUS_100) {
        X87::from_f64_exact(crate::cmd0::find_ground_z_for_coord(f(x), f(y)))
    } else {
        X87::from_f32(z)
    };
    let z = st0.fadd_m(f(K_ONE)).to_f32();
    let pos = [f(x), f(y), z];
    let pos_p = GPtr::from_raw(pos.as_ptr().cast_mut());
    set_posn_v(ped, pos_p);
    set_orientation(ped, 0.0, 0.0, 0.0);
    crate::cmd0::clear_space_for_mission_entity(pos_p, ped);
    if uses_mission_cleanup(s) {
        let fl = ped.byte_add(0x1C).cast::<u32>();
        fl.write(fl.read() | 0x40000);
    }
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

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_100_TO_199];
