//! `CRunningScript::ProcessCommands100To199` (0x466DE0): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1; the common tail only restores the SEH frame through the stub 0x4021E1). Cases
//! follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands100To199.md`.
//!
//! Divergence notes: the 8-byte text labels (00BA, 00BB, 00BC) are read into a zeroed 16-byte buffer (the
//! original's is stale stack bytes past the label); bools pushed with stale upper bytes are passed as 0/1.
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
pub const PORTED: &[u16] = &[
    0x0064, 0x0065, 0x0066, 0x0067, 0x0068, 0x0069, 0x006A, 0x006B, 0x006C, 0x006D, 0x006E, 0x006F, 0x0070,
    0x0071, 0x0072, 0x0073, 0x0074, 0x0075, 0x0076, 0x0077, 0x0078, 0x0079, 0x007A, 0x007B, 0x007C, 0x007D,
    0x007E, 0x007F, 0x0080, 0x0081, 0x0082, 0x0083, 0x0084, 0x0085, 0x0086, 0x0087, 0x0088, 0x0089, 0x008A,
    0x008B, 0x008C, 0x008D, 0x008E, 0x008F, 0x0090, 0x0091, 0x0092, 0x0093, 0x0094, 0x0095, 0x0096, 0x0097,
    0x0098, 0x0099, 0x009A, 0x009B, 0x00A0, 0x00A1, 0x00A3, 0x00A4, 0x00A5, 0x00A6, 0x00A7, 0x00A8, 0x00A9,
    0x00AA, 0x00AB, 0x00AD, 0x00AE, 0x00AF, 0x00B0, 0x00B1, 0x00BA, 0x00BB, 0x00BC, 0x00BE, 0x00BF, 0x00C0,
    0x00C1, 0x00C2, 0x00C3, 0x00C4,
];

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

/// `CTimer::ms_fTimeStep` 0xB7CB5C.
const TIME_STEP: usize = 0xB7CB5C;
/// `.rdata` 0x859D5C: 1/65536 (0x37800000)
const K_INV_65536: u32 = 0x3780_0000;

orig! {
    /// CRT `rand()` (0x821B1E)
    fn crt_rand() -> u32 = cdecl 0x821B1E => unported;
}

orig! {
    /// `CTheScripts::RemoveThisPed(CPed*)` (0x486240)
    fn remove_this_ped(p: GPtr<u8>) = cdecl 0x486240 => unported;
    /// `CRunningScript::SetCharCoordinates(CPed*, float x, float y, float z, bool, bool)` (0x464DC0);
    /// floats raw
    fn set_char_coordinates(s: Scr, p: GPtr<u8>, x: u32, y: u32, z: u32, a: u32, b: u32) = thiscall 0x464DC0 => unported;
    /// `CPlaceable::IsWithinArea(float x1, float y1, float x2, float y2)` (0x54F200), AL; floats raw
    fn is_within_area_2d(e: GPtr<u8>, x1: u32, y1: u32, x2: u32, y2: u32) -> u8 = thiscall 0x54F200 => unported;
    /// `CPlaceable::IsWithinArea(float x1, float y1, float z1, float x2, float y2, float z2)` (0x54F2B0), AL
    fn is_within_area_3d(e: GPtr<u8>, x1: u32, y1: u32, z1: u32, x2: u32, y2: u32, z2: u32) -> u8 = thiscall 0x54F2B0 => unported;
    /// `CTheScripts::HighlightImportantArea(uint32 id, float x1, float y1, float x2, float y2, float z)`
    /// (0x485E00); floats raw
    fn highlight_important_area(id: u32, x1: u32, y1: u32, x2: u32, y2: u32, z: u32) = cdecl 0x485E00 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats raw, ST0 (exact as f64)
    fn ground_z(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CEntity::GetDistanceFromCentreOfMassToBaseOfModel()` (0x536BE0, called directly), ST0 (exact as f64)
    fn distance_to_base(e: GPtr<u8>) -> f64 = thiscall 0x536BE0 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystemGotoCoors(CVehicle*, const CVector&, bool, bool)` (0x42F870), AL
    fn join_car_with_road_system_goto_coors(v: GPtr<u8>, pos: GPtr<u32>, a: u32, b: u32) -> u8 = cdecl 0x42F870 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystem(CVehicle*)` (0x42F5A0)
    fn join_car_with_road_system(v: GPtr<u8>) = cdecl 0x42F5A0 => unported;
    /// Unnamed thiscall at 0x41BDD0 (a `.HOODLUM` stub) taking a bool, called on the vehicle with `true`
    /// after the car-mission commands set the autopilot
    fn vehicle_41bdd0(v: GPtr<u8>, b: u32) = thiscall 0x41BDD0 => unported;
    /// Unnamed thiscall at 0x463490 (a `.HOODLUM` stub) on the vehicle's `CAutoPilot` (+0x390) taking
    /// the mission byte (presumably `CAutoPilot::SetCarMission`)
    fn autopilot_463490(ap: GPtr<u8>, mission: u32) = thiscall 0x463490 => unported;
    /// `CCarCtrl::SetCoordsOfScriptCar(CVehicle*, float x, float y, float z, bool, bool)` (0x4342A0); floats raw
    fn set_coords_of_script_car(v: GPtr<u8>, x: u32, y: u32, z: u32, a: u32, b: u32) = cdecl 0x4342A0 => unported;
    /// `CText::Get(const char*)` (0x6A0050), ECX = `TheText`
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CMessages::AddBigMessage(const char*, uint32 time, uint16 style)` (0x69F2B0)
    fn add_big_message(text: GPtr<u8>, time: u32, style: u32) = cdecl 0x69F2B0 => unported;
    /// `CMessages::AddMessageQ(const char*, uint32 time, uint16 flag, bool previousBrief)` (0x69F0B0)
    fn add_message_q(text: GPtr<u8>, time: u32, flag: u32, brief: u32) = cdecl 0x69F0B0 => unported;
    /// `CMessages::AddMessageJump(const char*, uint32 time, uint16 flag, bool previousBrief)` (0x69F1E0)
    fn add_message_jump(text: GPtr<u8>, time: u32, flag: u32, brief: u32) = cdecl 0x69F1E0 => unported;
    /// `CClock::GetGameClockMinutesUntil(uint8 h, uint8 m)` (0x52CEB0)
    fn get_game_clock_minutes_until(h: u32, m: u32) -> u32 = cdecl 0x52CEB0 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40), AL; the float raw
    fn is_sphere_visible(cam: GPtr<u8>, c: GPtr<u32>, r: u32) -> u8 = thiscall 0x420D40 => unported;
}

/// `CTimer::m_snTimeInMilliseconds` 0xB7CB84.
const TIME_MS: usize = 0xB7CB84;
/// `TheText` 0xC1B340.
const THE_TEXT: usize = 0xC1B340;
/// `CTheScripts::bAddNextMessageToPreviousBriefs` (byte) 0xA4448B.
const ADD_NEXT_TO_PREVIOUS_BRIEFS: usize = 0xA4448B;
/// Byte 0xBA678C (the "~z~" subtitle switch PRINT / PRINT_NOW test; `FrontEndMenuManager.m_bShowSubtitles`).
const SHOW_SUBTITLES: usize = 0xBA678C;
/// `CClock::ms_nGameClockHours` / `ms_nGameClockMinutes` (bytes) 0xB70153 / 0xB70152.
const CLOCK_HOURS: usize = 0xB70153;
const CLOCK_MINUTES: usize = 0xB70152;
/// `.rdata` 0x858B8C: 0.5f; 0x858B34: 60.0f
const K_HALF: u32 = 0x3F00_0000;
const K_60: u32 = 0x4270_0000;

/// `m_matrix ? &m_matrix->pos : &m_placement.pos` of an entity.
#[inline]
fn entity_pos(e: GPtr<u8>) -> GPtr<u32> {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }.cast::<u32>()
}

/// The ped itself, or its vehicle when "in vehicle" (+0x46C bit 8) and `m_pVehicle` (+0x58C) is set.
#[inline]
fn ped_or_vehicle(ped: GPtr<u8>) -> GPtr<u8> {
    if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
        let v = ped.byte_add(0x58C).cast::<GPtr<u8>>().read();
        if !v.is_null() {
            return v;
        }
    }
    ped
}

/// `m_pCurrentIP (+0x14) + this` (the id the area/marker helpers take).
#[inline]
fn script_id(s: Scr) -> u32 {
    s.byte_add(crate::vm::IP).cast::<u32>().read().wrapping_add(s.addr() as u32)
}

/// IS_*_IN_AREA_2D after the lookup: flag = `e->IsWithinArea(P1, P2, P3, P4) != 0`; with P5 != 0
/// HighlightImportantArea(id, P1, P2, P3, P4, -100.0f).
fn in_area_2d(s: Scr, e: GPtr<u8>) {
    let b = is_within_area_2d(e, sp(1), sp(2), sp(3), sp(4)) != 0;
    set_flag(s, b);
    if sp(5) != 0 {
        highlight_important_area(script_id(s), sp(1), sp(2), sp(3), sp(4), 0xC2C8_0000);
    }
}

/// IS_*_IN_AREA_3D after the lookup: flag = `e->IsWithinArea(P1..P6) != 0`; with P7 != 0
/// HighlightImportantArea(id, P1, P2, P4, P5, (f32)((P6 + P3) * 0.5)) (`fld; fadd; fmul`, PC=24).
fn in_area_3d(s: Scr, e: GPtr<u8>) {
    let b = is_within_area_3d(e, sp(1), sp(2), sp(3), sp(4), sp(5), sp(6)) != 0;
    set_flag(s, b);
    if sp(7) != 0 {
        let z = X87::from_f32(spf(6)).fadd_m(spf(3)).fmul_m(f(K_HALF)).to_f32().to_bits();
        highlight_important_area(script_id(s), sp(1), sp(2), sp(4), sp(5), z);
    }
}

/// The car-mission tail of 00A7/00A8: `vehicle_41bdd0(veh, true)`; `m_autoPilot.m_nCruiseSpeed`
/// (+0x3D0) = max(byte, 1) (unsigned); `m_autoPilot.m_nTimeToStartMission` (+0x3AC) = CTimer time.
fn car_mission_tail(v: GPtr<u8>) {
    vehicle_41bdd0(v, 1);
    let p = v.byte_add(0x3D0);
    let c = p.read();
    p.write(if c > 1 { c } else { 1 });
    v.byte_add(0x3AC).cast::<u32>().write(GPtr::<u32>::from_addr(TIME_MS).read());
}

/// An 8-byte text label read with `ReadTextLabelFromScript` into a zeroed 16-byte buffer, then
/// `TheText.Get` (divergence note: the original's buffer is not cleared).
fn label_text(s: Scr) -> GPtr<u8> {
    let mut buf = [0u8; 16];
    let b = GPtr::from_raw(buf.as_mut_ptr());
    read_text_label(s, b, 8);
    text_get(GPtr::from_addr(THE_TEXT), b)
}

/// PRINT / PRINT_NOW: nothing printed when the text starts with "~z~" (not null) and byte 0xBA678C is
/// 0; otherwise `add(text, P0, (u16)P1, byte 0xA4448B != 0)`; byte 0xA4448B = 1 either way.
fn print_with(s: Scr, add: fn(GPtr<u8>, u32, u32, u32)) {
    let t = label_text(s);
    collect(s, 2);
    let subtitle = !t.is_null() && t.read() == b'~' && t.byte_add(1).read() == b'z' && t.byte_add(2).read() == b'~';
    let brief = GPtr::<u8>::from_addr(ADD_NEXT_TO_PREVIOUS_BRIEFS);
    if !subtitle || GPtr::<u8>::from_addr(SHOW_SUBTITLES).read() != 0 {
        add(t, sp(0), sp(1) & 0xFFFF, u32::from(brief.read() != 0));
    }
    brief.write(1);
}

/// Two `GetPointerToScriptVariable` operands, `a` (the destination) first.
#[inline]
fn two_vars(s: Scr, ta: i32, tb: i32) -> (GPtr<u32>, GPtr<u32>) {
    let a = var_ptr(s, ta);
    let b = var_ptr(s, tb);
    (a, b)
}

/// `fld x; fop m; fstp` with `m` a memory operand (PC=24).
#[inline]
fn fop(x: u32, m: u32, op: fn(X87, f32) -> X87) -> u32 {
    op(X87::from_f32(f(x)), f(m)).to_f32().to_bits()
}

/// `cdq; idiv` (faults on a zero divisor and on INT_MIN / -1, as the original).
#[inline]
fn idiv(a: u32, b: u32) -> u32 {
    let (a, b) = (a as i32, b as i32);
    assert!(b != 0 && !(a == i32::MIN && b == -1), "ProcessCommands100To199: idiv fault");
    (a / b) as u32
}

/// `CTimer::ms_fTimeStep` as an x87 load.
#[inline]
fn time_step() -> X87 {
    X87::from_f32(f(GPtr::<u32>::from_addr(TIME_STEP).read()))
}

/// `CRunningScript::ProcessCommands100To199(int32 command)` (0x466DE0, thiscall, `ret 4`): AL = 0
/// continue, -1 unknown.
#[hook(addr = 0x466DE0, conv = thiscall, name = "CRunningScript::ProcessCommands100To199", expect = "64 A1 00 00 00 00", this = "CRunningScript")]
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
        // Variable operands are `GetPointerToScriptVariable(t)` (t = 2 global, 1 local) in the order the bytes
        // fetch them; `a` is the destination. Integer ops wrap; float ops are `fld; fop m32; fstp` (PC=24)
        // with the operand order of the bytes.
        // 0064 SUB_INT_VAR_FROM_INT_LVAR (0x466E13, a=L, b=G) / 0066 SUB_INT_LVAR_FROM_INT_VAR (0x466E6F,
        // a=G, b=L): `*a -= *b`.
        0x0064 | 0x0066 => {
            let (a, b) = if command == 0x0064 { two_vars(s, 1, 2) } else { two_vars(s, 2, 1) };
            a.write(a.read().wrapping_sub(b.read()));
            0
        }
        // 0065 SUB_FLOAT_VAR_FROM_FLOAT_LVAR (0x466E40, a=L, b=G) / 0067 SUB_FLOAT_LVAR_FROM_FLOAT_VAR
        // (0x466E9C, a=G, b=L): `fld a; fsub b`.
        0x0065 | 0x0067 => {
            let (a, b) = if command == 0x0065 { two_vars(s, 1, 2) } else { two_vars(s, 2, 1) };
            a.write(fop(a.read(), b.read(), X87::fsub_m));
            0
        }
        // 0068 MULT_INT_VAR_BY_INT_VAR (0x466ECB, G,G) / 006A _LVAR_BY_INT_LVAR (0x466F2A, L,L) / 006C
        // _VAR_BY_INT_LVAR (0x466F89, G,L) / 006E _LVAR_BY_INT_VAR (0x466FE8, L,G): `*a *= *b` (imul).
        0x0068 | 0x006A | 0x006C | 0x006E => {
            let (ta, tb) = [(2, 2), (1, 1), (2, 1), (1, 2)][((command - 0x68) / 2) as usize];
            let (a, b) = two_vars(s, ta, tb);
            a.write(a.read().wrapping_mul(b.read()));
            0
        }
        // 0069 MULT_FLOAT_VAR_BY_FLOAT_VAR (0x466EFB, G,G) and 006F _LVAR_BY_FLOAT_VAR (0x467018, L,G): `fld b;
        // fmul a`; 006B _LVAR_BY_FLOAT_LVAR (0x466F5A, L,L) and 006D _VAR_BY_FLOAT_LVAR (0x466FB9, G,L): `fld a;
        // fmul b`.
        0x0069 | 0x006B | 0x006D | 0x006F => {
            let (ta, tb) = [(2, 2), (1, 1), (2, 1), (1, 2)][((command - 0x69) / 2) as usize];
            let (a, b) = two_vars(s, ta, tb);
            let r = if command == 0x0069 || command == 0x006F {
                fop(b.read(), a.read(), X87::fmul_m)
            } else {
                fop(a.read(), b.read(), X87::fmul_m)
            };
            a.write(r);
            0
        }
        // 0070 DIV_INT_VAR_BY_INT_VAR (0x467047, G,G) / 0072 _LVAR_BY_INT_LVAR (0x4670A8, L,L) / 0074
        // _VAR_BY_INT_LVAR (0x4670E4, G,L) / 0076 _LVAR_BY_INT_VAR (0x467123, L,G): `*a = *a / *b` (`cdq; idiv`,
        // faults kept).
        0x0070 | 0x0072 | 0x0074 | 0x0076 => {
            let (ta, tb) = [(2, 2), (1, 1), (2, 1), (1, 2)][((command - 0x70) / 2) as usize];
            let (a, b) = two_vars(s, ta, tb);
            a.write(idiv(a.read(), b.read()));
            0
        }
        // 0071 DIV_FLOAT_VAR_BY_FLOAT_VAR (0x467079, G,G) / 0073 (0x4670B5, L,L) / 0075 (0x4670F4, G,L) / 0077
        // (0x46712A, L,G): `fld a; fdiv b`.
        0x0071 | 0x0073 | 0x0075 | 0x0077 => {
            let (ta, tb) = [(2, 2), (1, 1), (2, 1), (1, 2)][((command - 0x71) / 2) as usize];
            let (a, b) = two_vars(s, ta, tb);
            a.write(fop(a.read(), b.read(), X87::fdiv_m));
            0
        }
        // 0078 ADD_TIMED_VAL_TO_FLOAT_VAR (0x467159, G) / 0079 _LVAR (0x467192, L): `a = var(t)`;
        // `CollectParameters(1)`; `fld TimeStep; fmul P0; fadd a`. 007E SUB_TIMED_VAL_FROM_FLOAT_VAR (0x4671E3,
        // G) / 007F _LVAR (0x46720A, L): the same with `fsubr a` (a - ts * P0).
        0x0078 | 0x0079 | 0x007E | 0x007F => {
            let a = var_ptr(s, if command & 1 == 0 { 2 } else { 1 });
            collect(s, 1);
            let t = time_step().fmul_m(spf(0));
            let r = if command < 0x7E { t.fadd_m(f(a.read())) } else { t.fsubr_m(f(a.read())) };
            a.write(r.to_f32().to_bits());
            0
        }
        // 007A ADD_TIMED_FLOAT_VAR_TO_FLOAT_VAR (0x467196, G,G) / 007B _LVAR_TO_.._LVAR (0x4671A3, L,L) / 007C
        // _VAR_TO_.._LVAR (0x4671A7, a=L, b=G) / 007D _LVAR_TO_.._VAR (0x4671C5, a=G, b=L): `fld TimeStep; fmul
        // b; fadd a`. 0080..0083 SUB_TIMED_* (0x46720E, 0x46721B, 0x46721F, 0x46722C; same operand pairs):
        // `fsubr a`.
        0x007A..=0x007D | 0x0080..=0x0083 => {
            let k = if command < 0x80 { command - 0x7A } else { command - 0x80 };
            let (ta, tb) = [(2, 2), (1, 1), (1, 2), (2, 1)][k as usize];
            let (a, b) = two_vars(s, ta, tb);
            let t = time_step().fmul_m(f(b.read()));
            let r = if command < 0x80 { t.fadd_m(f(a.read())) } else { t.fsubr_m(f(a.read())) };
            a.write(r.to_f32().to_bits());
            0
        }
        // 0084..008B SET_*_TO_* (0x46724F, 0x46727C, 0x4672A9, 0x4672C4, 0x4672F1, 0x46731E, 0x467339,
        // 0x467366): a raw dword copy `*a = *b`.
        0x0084..=0x008B => {
            let (ta, tb) = [(2, 2), (1, 1), (2, 2), (1, 1), (2, 1), (1, 2), (2, 1), (1, 2)][(command - 0x84) as usize];
            let (a, b) = two_vars(s, ta, tb);
            a.write(b.read());
            0
        }
        // 008C/008E/0090/0092 CSET_*_INT_TO_*_FLOAT (0x467393, 0x4673CE, 0x467409, 0x467444): `fld b;
        // _ftol2` (0x821B40 as `X87::ftol`, low dword). 008D/008F/0091/0093 CSET_*_FLOAT_TO_*_INT (0x4673B3,
        // 0x4673EE, 0x467429, 0x467464): `fild b; fstp a`. Operand pairs (a, b): 8C/8D (G,G), 8E/8F (L,G),
        // 90/91 (G,L), 92/93 (L,L).
        0x008C..=0x0093 => {
            let (ta, tb) = [(2, 2), (1, 2), (2, 1), (1, 1)][((command - 0x8C) / 2) as usize];
            let (a, b) = two_vars(s, ta, tb);
            let r = if command & 1 == 0 {
                X87::from_f32(f(b.read())).ftol() as u32
            } else {
                X87::from_i32(b.read() as i32).to_f32().to_bits()
            };
            a.write(r);
            0
        }
        // 0094 ABS_VAR_INT (0x46747F) / 0095 ABS_LVAR_INT (0x4674A7): `neg` when negative (wrapping).
        0x0094 | 0x0095 => {
            let a = var_ptr(s, if command == 0x0094 { 2 } else { 1 });
            let v = a.read() as i32;
            a.write(if v < 0 { v.wrapping_neg() as u32 } else { v as u32 });
            0
        }
        // 0096 ABS_VAR_FLOAT (0x4674AB) / 0097 ABS_LVAR_FLOAT (0x4674E0): `fld; fcomp 0.0; fld; test ah,5; jp`:
        // `fchs` only when less than 0 (ordered); always written back through `fld; fstp` (an sNaN comes
        // back quiet).
        0x0096 | 0x0097 => {
            let a = var_ptr(s, if command == 0x0096 { 2 } else { 1 });
            let v = X87::from_f32(f(a.read()));
            let v = if v.fcom(X87::zero()) == sa_math::x87::Cmp::Less { v.fchs() } else { v };
            a.write(v.to_f32().to_bits());
            0
        }
        // 0098 GENERATE_RANDOM_FLOAT (0x4674E4): `a = var(2)`; `rand()` four times, the last value `fild`ed,
        // `fmul 1/65536`, `fstp a`. gr: unregistered.
        0x0098 => {
            let a = var_ptr(s, 2);
            sa_core::orig::at_site(0x4674F4, crt_rand);
            sa_core::orig::at_site(0x4674F9, crt_rand);
            sa_core::orig::at_site(0x4674FE, crt_rand);
            let r = sa_core::orig::at_site(0x467503, crt_rand);
            a.write(X87::from_i32(r as i32).fmul_m(f(K_INV_65536)).to_f32().to_bits());
            0
        }
        // 0099 GENERATE_RANDOM_INT (0x467528): `*var(2) = rand()`.
        0x0099 => {
            let a = var_ptr(s, 2);
            a.write(sa_core::orig::at_site(0x467538, crt_rand));
            0
        }
        // 009B DELETE_CHAR (0x467789): RemoveThisPed(PedPool.GetAtRef(P0)); with mission cleanup
        // RemoveEntityFromList(P0, 2).
        0x009B => {
            collect(s, 1);
            remove_this_ped(ped_at_ref(pool(PED_POOL), spi(0)));
            if uses_mission_cleanup(s) {
                mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 2);
            }
            0
        }
        // 00A0 GET_CHAR_COORDINATES (0x4677E2): ped not null-checked; the position of its vehicle when "in
        // vehicle" and `m_pVehicle` set, else its own; P0..P2 = pos; StoreParameters(3).
        0x00A0 => {
            collect(s, 1);
            let p = entity_pos(ped_or_vehicle(ped_at_ref(pool(PED_POOL), spi(0))));
            let v = [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()];
            set_sp(0, v[0]);
            set_sp(1, v[1]);
            set_sp(2, v[2]);
            store(s, 3);
            0
        }
        // 00A1 SET_CHAR_COORDINATES (0x467875): this->SetCharCoordinates(PedPool.GetAtRef(P0), P1, P2, P3,
        // true, true).
        0x00A1 => {
            collect(s, 4);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            set_char_coordinates(s, ped, sp(1), sp(2), sp(3), 1, 1);
            0
        }
        // 00A3 IS_CHAR_IN_AREA_2D (0x4678C5) / 00A4 IS_CHAR_IN_AREA_3D (0x4679A6): the ped's vehicle when "in
        // vehicle" and set, else the ped.
        0x00A3 => {
            collect(s, 6);
            let e = ped_or_vehicle(ped_at_ref(pool(PED_POOL), spi(0)));
            in_area_2d(s, e);
            0
        }
        0x00A4 => {
            collect(s, 8);
            let e = ped_or_vehicle(ped_at_ref(pool(PED_POOL), spi(0)));
            in_area_3d(s, e);
            0
        }
        // 00A7 CAR_GOTO_COORDINATES (0x467B83): vehicle not null-checked; z = `fld P3; fst` (ground z
        // `fstp`ed when `<= -100`, ordered) then `GetDistanceFromCentreOfMassToBaseOfModel() fadd z`;
        // `r = JoinCarWithRoadSystemGotoCoors(veh, &(P1, P2, z), false, false)`; unless the mission byte
        // (+0x3BA) is 0x39/0x3A it becomes 9 (r) or 8; status (+0x36) = 3 (`& 7 | 0x18`); the car-mission tail.
        // gr: unregistered.
        0x00A7 => {
            collect(s, 4);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let z = if spf(3) <= f(K_MINUS_100) {
                X87::from_f64_exact(ground_z(sp(1), sp(2))).to_f32()
            } else {
                fld_fst(spf(3))
            };
            let z = X87::from_f64_exact(distance_to_base(v)).fadd_m(z).to_f32().to_bits();
            let pos = [sp(1), sp(2), z];
            let r = join_car_with_road_system_goto_coors(v, GPtr::from_raw(pos.as_ptr().cast_mut()), 0, 0) != 0;
            let m = v.byte_add(0x3BA);
            let mv = m.read();
            if mv != 0x39 && mv != 0x3A {
                m.write(if r { 9 } else { 8 });
            }
            let st = v.byte_add(0x36);
            st.write((st.read() & 7) | 0x18);
            car_mission_tail(v);
            0
        }
        // 00A8 CAR_WANDER_RANDOMLY (0x467C83): JoinCarWithRoadSystem(veh); mission byte = 1 unless 0x39/0x3A;
        // the car-mission tail. gr: unregistered.
        0x00A8 => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            join_car_with_road_system(v);
            let m = v.byte_add(0x3BA);
            let mv = m.read();
            if mv != 0x39 && mv != 0x3A {
                m.write(1);
            }
            car_mission_tail(v);
            0
        }
        // 00A9 CAR_SET_IDLE (0x467CFF): mission byte (+0x3BA) = 0 unless 0x39/0x3A. gr: unregistered.
        0x00A9 => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let m = v.byte_add(0x3BA);
            let mv = m.read();
            if mv != 0x39 && mv != 0x3A {
                m.write(0);
            }
            0
        }
        // 00AA GET_CAR_COORDINATES (0x467D48): P0..P2 = the vehicle's position; StoreParameters(3).
        // gr: unregistered.
        0x00AA => {
            collect(s, 1);
            let p = entity_pos(vehicle_at_ref(pool(VEHICLE_POOL), spi(0)));
            let v = [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()];
            set_sp(0, v[0]);
            set_sp(1, v[1]);
            set_sp(2, v[2]);
            store(s, 3);
            0
        }
        // 00AB SET_CAR_COORDINATES (0x467DB5): SetCoordsOfScriptCar(veh, P1, P2, P3, false, true).
        // gr: unregistered.
        0x00AB => {
            collect(s, 4);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_coords_of_script_car(v, sp(1), sp(2), sp(3), 0, 1);
            0
        }
        // 00AD SET_CAR_CRUISE_SPEED (0x467E05): cruise byte (+0x3D0) = `(u8)_ftol(P1)`; then `limit =
        // (f32)(m_pHandlingData (+0x384)->m_transmissionData.m_fMaxVelocity (+0x88) * 60.0f)`; the byte is
        // rewritten as `(u8)_ftol(fild byte < limit ? byte : limit)` (`fcom; test ah,5; jnp`, the int kept
        // only when less, ordered). gr: unregistered.
        0x00AD => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let c = X87::from_f32(spf(1)).ftol() as u8;
            v.byte_add(0x3D0).write(c);
            let h = v.byte_add(0x384).cast::<GPtr<u8>>().read();
            let limit = X87::from_f32(f(h.byte_add(0x88).cast::<u32>().read())).fmul_m(f(K_60)).to_f32();
            let st = X87::from_i32(i32::from(c));
            let st = if st.fcom(X87::from_f32(limit)) == sa_math::x87::Cmp::Less { st } else { X87::from_f32(limit) };
            v.byte_add(0x3D0).write(st.ftol() as u8);
            0
        }
        // 00AE SET_CAR_DRIVING_STYLE (0x467E85): no null check; +0x3B9 = (u8)P1.
        0x00AE => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            v.byte_add(0x3B9).write(sp(1) as u8);
            0
        }
        // 00AF SET_CAR_MISSION (0x467EC1): 0x463490(&veh->m_autoPilot (+0x390), (u8)P1); +0x3AC = CTimer time;
        // vehicle_41bdd0(veh, true). gr: unregistered.
        0x00AF => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            autopilot_463490(v.byte_add(0x390), sp(1) & 0xFF);
            v.byte_add(0x3AC).cast::<u32>().write(GPtr::<u32>::from_addr(TIME_MS).read());
            vehicle_41bdd0(v, 1);
            0
        }
        // 00B0 IS_CAR_IN_AREA_2D (0x467F1B) / 00B1 IS_CAR_IN_AREA_3D (0x467FBD). gr: unregistered.
        0x00B0 => {
            collect(s, 6);
            in_area_2d(s, vehicle_at_ref(pool(VEHICLE_POOL), spi(0)));
            0
        }
        0x00B1 => {
            collect(s, 8);
            in_area_3d(s, vehicle_at_ref(pool(VEHICLE_POOL), spi(0)));
            0
        }
        // 00BA PRINT_BIG (0x46807B): label (8) → TheText.Get; `CollectParameters(2)`; AddBigMessage(text, P0,
        // P1 - 1). Divergence note: zeroed label buffer.
        0x00BA => {
            let t = label_text(s);
            collect(s, 2);
            add_big_message(t, sp(0), sp(1).wrapping_sub(1));
            0
        }
        // 00BB PRINT (0x4680CF, AddMessageQ) / 00BC PRINT_NOW (0x46815B, AddMessageJump): see `print_with`.
        // Divergence notes: zeroed label buffer; the `setne` bool passed as 0/1.
        0x00BB => {
            print_with(s, add_message_q);
            0
        }
        0x00BC => {
            print_with(s, add_message_jump);
            0
        }
        // 00BF GET_TIME_OF_DAY (0x468201): P0 = hours, P1 = minutes (bytes, zero-extended); StoreParameters(2).
        0x00BF => {
            set_sp(0, u32::from(GPtr::<u8>::from_addr(CLOCK_HOURS).read()));
            set_sp(1, u32::from(GPtr::<u8>::from_addr(CLOCK_MINUTES).read()));
            store(s, 2);
            0
        }
        // 00C1 GET_MINUTES_TO_TIME_OF_DAY (0x468272): P0 = GetGameClockMinutesUntil((u8)P0, (u8)P1);
        // StoreParameters(1).
        0x00C1 => {
            collect(s, 2);
            let r = get_game_clock_minutes_until(sp(0) & 0xFF, sp(1) & 0xFF);
            set_sp(0, r);
            store(s, 1);
            0
        }
        // 00C2 IS_POINT_ON_SCREEN (0x4682B7): (P0, P1, z) with z = P2 through `fld; fcom -100; jp` (ground z when
        // `<= -100`, ordered), `fstp`ed; flag = TheCamera.IsSphereVisible(&v, P3) != 0.
        0x00C2 => {
            collect(s, 4);
            let z = if spf(2) <= f(K_MINUS_100) {
                X87::from_f64_exact(ground_z(sp(0), sp(1))).to_f32().to_bits()
            } else {
                fld_fst(spf(2)).to_bits()
            };
            let v = [sp(0), sp(1), z];
            let b = is_sphere_visible(GPtr::from_addr(THE_CAMERA), GPtr::from_raw(v.as_ptr().cast_mut()), sp(3)) != 0;
            set_flag(s, b);
            0
        }
        // 00C3 DEBUG_ON / 00C4 DEBUG_OFF (0x46833A): straight to the epilogue, 0 without operands.
        0x00C3 | 0x00C4 => 0,
        _ => -1,
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
