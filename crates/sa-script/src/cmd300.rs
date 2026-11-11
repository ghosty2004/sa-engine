//! `CRunningScript::ProcessCommands300To399` (0x47C100): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands300To399.md`.
//!
//! No case of this switch returns 1; every ported case returns 0.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0137, 0x014E, 0x014F, 0x0151, 0x0158, 0x0159, 0x015A, 0x015B, 0x015D, 0x015F, 0x0160, 0x0161, 0x0164,
    0x0165, 0x0168, 0x0169, 0x016B, 0x016C, 0x016D, 0x016E, 0x0173, 0x0175, 0x017B, 0x0180, 0x0184, 0x0185,
    0x0186, 0x0187, 0x0188, 0x018A, 0x018B, 0x018C, 0x018D, 0x018E, 0x018F,
];

/// `.rdata` 0x858B50: 0.0f
const K_ZERO: u32 = 0x0000_0000;
/// `.rdata` 0x859E2C: 360.0f
const K_360: u32 = 0x43B4_0000;
/// `.rdata` 0x8595EC: 0.017453292f (degrees to radians)
const K_DEG2RAD: u32 = 0x3C8E_FA35;
/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;
/// `CTimer::ms_fTimeScale` (owned by sa-core's timer; written by address as the original does).
const TIME_SCALE: usize = 0xB7CB64;
/// `AudioEngine` (`CAudioEngine`) 0xB6BC90.
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `CTheScripts::UpsideDownCars` (`CUpsideDownCarCheck`) 0xA4892C.
const UPSIDE_DOWN_CARS: usize = 0xA4892C;
/// `CUserDisplay::OnscnTimer` (`COnscreenTimer`) 0xBA1788.
const ONSCN_TIMER: usize = 0xBA1788;
/// `CTheScripts::OnAMissionFlag` 0xA476AC.
const ON_A_MISSION_FLAG: usize = 0xA476AC;

orig! {
    /// `CRunningScript::CollectNextParameterWithoutIncreasingPC()` (0x464250)
    fn collect_next(s: Scr) -> i32 = thiscall 0x464250 => crate::vm::collect_next_parameter_without_increasing_pc;
    /// `CCamera::TakeControl(CEntity*, eCamMode, eSwitchType, int32 whoIsInControl)` (0x50C7C0)
    fn take_control(cam: GPtr<u8>, e: GPtr<u8>, mode: u32, switch: u32, who: i32) = thiscall 0x50C7C0 => unported;
    /// `CCamera::TakeControlNoEntity(const CVector&, eSwitchType, int32 whoIsInControl)` (0x50C8B0)
    fn take_control_no_entity(cam: GPtr<u8>, pos: GPtr<u32>, switch: u32, who: i32) = thiscall 0x50C8B0 => unported;
    /// `CCamera::SetCamPositionForFixedMode(const CVector&, const CVector&)` (0x50BEC0)
    fn set_cam_position_for_fixed_mode(cam: GPtr<u8>, pos: GPtr<u32>, up: GPtr<u32>) = thiscall 0x50BEC0 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0), AL
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CRadar::GetActualBlipArrayIndex(int32)` (0x582870); the result is discarded by every caller here
    fn get_actual_blip_array_index(h: i32) -> i32 = cdecl 0x582870 => unported;
    /// `CRadar::SetEntityBlip(eBlipType, int32 handle, uint32 colour, eBlipDisplay, const char* scriptName)` (0x5839A0)
    fn set_entity_blip(ty: i32, h: i32, colour: u32, display: u32, name: GPtr<u8>) -> i32 = cdecl 0x5839A0 => unported;
    /// `CRadar::ClearBlip(int32)` (0x587CE0)
    fn clear_blip(h: i32) = cdecl 0x587CE0 => unported;
    /// `CRadar::ChangeBlipColour(int32, uint32)` (0x583AB0)
    fn change_blip_colour(h: i32, c: u32) = cdecl 0x583AB0 => unported;
    /// `CRadar::ChangeBlipScale(int32, int32)` (0x583CC0)
    fn change_blip_scale(h: i32, scale: i32) = cdecl 0x583CC0 => unported;
    /// `CRadar::ChangeBlipDisplay(int32, eBlipDisplay)` (0x583D20)
    fn change_blip_display(h: i32, d: u32) = cdecl 0x583D20 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (read as f64: exact)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CRestart::AddHospitalRestartPoint(const CVector&, float, int32 town)` (0x460730)
    fn add_hospital_restart_point(pos: GPtr<u32>, angle: u32, town: i32) = cdecl 0x460730 => unported;
    /// `CRestart::AddPoliceRestartPoint(const CVector&, float, int32 town)` (0x460780)
    fn add_police_restart_point(pos: GPtr<u32>, angle: u32, town: i32) = cdecl 0x460780 => unported;
    /// `CRestart::OverrideNextRestart(const CVector&, float)` (0x4607D0)
    fn override_next_restart(pos: GPtr<u32>, angle: u32) = cdecl 0x4607D0 => unported;
    /// `COnscreenTimer::AddClock(uint32 varId, const char* gxt, bool countDown)` (0x44CD50)
    fn add_clock(t: GPtr<u8>, var: u32, gxt: GPtr<u8>, down: u32) = thiscall 0x44CD50 => unported;
    /// `CRadar::SetCoordBlip(eBlipType, CVector, uint32 colour, eBlipDisplay, const char*)` (0x583820);
    /// the by-value vector as three raw dwords
    fn set_coord_blip(ty: i32, x: u32, y: u32, z: u32, colour: u32, display: u32, name: GPtr<u8>) -> i32 = cdecl 0x583820 => unported;
    /// `CPlaceable::SetHeading(float)` (0x43E0C0)
    fn set_heading(e: GPtr<u8>, h: f32) = thiscall 0x43E0C0 => unported;
    /// `CEntity::UpdateRwMatrix()` (0x446F90)
    fn update_rw_matrix(e: GPtr<u8>) = thiscall 0x446F90 => unported;
    /// `CAudioEngine::ReportMissionAudioEvent(uint16, CVector&)` (0x507340)
    fn report_mission_audio_event(a: GPtr<u8>, id: u32, pos: GPtr<u32>) = thiscall 0x507340 => unported;
    /// `CRunningScript::GetIndexOfGlobalVariable()` (0x464700), AX
    fn global_index(s: Scr) -> u16 = thiscall 0x464700 => crate::vm::get_index_of_global_variable;
    /// `COnscreenTimer::ClearClock(uint32 varId)` (0x44CE60)
    fn clear_clock(t: GPtr<u8>, var: u32) = thiscall 0x44CE60 => unported;
    /// `COnscreenTimer::ClearCounter(uint32 varId)` (0x44CE80)
    fn clear_counter(t: GPtr<u8>, var: u32) = thiscall 0x44CE80 => unported;
    /// `CCamera::Restore()` (0x50B930)
    fn camera_restore(cam: GPtr<u8>) = thiscall 0x50B930 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::StartShake(int16 time, uint8 freq, uint32 shakeDelayMs)` (0x53F920)
    fn start_shake(pad: GPtr<u8>, time: u32, freq: u32, delay: u32) = thiscall 0x53F920 => unported;
    /// `CCamera::SetFadeColour(uint8, uint8, uint8)` (0x50BF00)
    fn set_fade_colour(cam: GPtr<u8>, r: u32, g: u32, b: u32) = thiscall 0x50BF00 => unported;
    /// `CCamera::GetFading()` (0x50ADE0), AL
    fn get_fading(cam: GPtr<u8>) -> u8 = thiscall 0x50ADE0 => unported;
    /// `CPed::SetAmmo(eWeaponType, uint32)` (0x5DF290)
    fn set_ammo(p: GPtr<u8>, ty: u32, ammo: u32) = thiscall 0x5DF290 => unported;
    /// `CUpsideDownCarCheck::HasCarBeenUpsideDownForAWhile(int32)` (0x463940), AL
    fn has_car_been_upside_down_for_a_while(c: GPtr<u8>, h: i32) -> u8 = thiscall 0x463940 => unported;
}

/// A local `CVector` (three raw f32 dwords) as the pointer the original passes.
#[inline]
fn vec_ptr(v: &[u32; 3]) -> GPtr<u32> {
    GPtr::from_raw(v.as_ptr().cast_mut())
}

/// The "-100 ground rule" as compiled in this switch: `fld z; fcom [-100.0]; fnstsw; test ah,0x41; jp
/// keep`; when taken not (`z <= -100`, ordered) `fstp st(0)` and ST0 = `FindGroundZForCoord(x, y)`; ST0
/// is then stored as f32 (an SNaN z comes back quieted by the `fld`).
#[inline]
fn ground_rule(x: u32, y: u32, z: u32) -> u32 {
    let st = X87::from_f32(f(z));
    let st = if st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        st
    } else {
        X87::from_f64_exact(find_ground_z_for_coord(x, y))
    };
    st.to_f32().to_bits()
}

/// The heading normalisation of 0173/0175, all in ST0: `fld a; fcom [0.0]; test ah,5; jp` → `fadd
/// [360.0]` when `a < 0`; `fcom [360.0]; test ah,0x41; jne` → `fsub [360.0]` when `a > 360`.
#[inline]
fn wrap_heading_deg(a: u32) -> X87 {
    let mut st = X87::from_f32(f(a));
    if !st.fcom(X87::constant(K_ZERO)).test_parity(5) {
        st = st.fadd_m(f(K_360));
    }
    if !st.fcom(X87::constant(K_360)).test_any(0x41) {
        st = st.fsub_m(f(K_360));
    }
    st
}

/// `CRunningScript::ProcessCommands300To399(int32 command)` (0x47C100, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x47C100, conv = thiscall, name = "CRunningScript::ProcessCommands300To399", expect = "8B 44 24 04 83 EC 7C", this = "CRunningScript", fallback)]
pub fn process_commands_300_to_399(s: Scr, command: u32) -> i8 {
    match command {
        // 0137 IS_CAR_MODEL (0x47C127): no null check; flag = (int16)m_nModelIndex (+0x22, `movsx`) == P1.
        0x0137 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let m = i32::from(veh.byte_add(0x22).cast::<u16>().read() as i16);
            set_flag(s, m == spi(1));
        }
        // 014E DISPLAY_ONSCREEN_TIMER (0x47C284): idx = GetIndexOfGlobalVariable(); CollectParameters(1);
        // OnscnTimer.AddClock((u16)idx, nullptr, P0 != 0).
        0x014E => {
            let idx = u32::from(global_index(s));
            collect(s, 1);
            add_clock(
                GPtr::from_addr(ONSCN_TIMER),
                idx,
                GPtr::NULL,
                u32::from(sp(0) != 0),
            );
        }
        // 014F CLEAR_ONSCREEN_TIMER (0x47C2D2) / 0151 CLEAR_ONSCREEN_COUNTER (0x47C2F1):
        // OnscnTimer.ClearClock / ClearCounter((u16)GetIndexOfGlobalVariable()).
        0x014F | 0x0151 => {
            let idx = u32::from(global_index(s));
            if command == 0x014F {
                clear_clock(GPtr::from_addr(ONSCN_TIMER), idx);
            } else {
                clear_counter(GPtr::from_addr(ONSCN_TIMER), idx);
            }
        }
        // 015A RESTORE_CAMERA (0x47C4C7)
        0x015A => camera_restore(GPtr::from_addr(THE_CAMERA)),
        // 015B SHAKE_PAD (0x47C4DB): CPad::GetPad(P0)->StartShake((u16)P1, (u8)P2, 0) (the narrow values
        // zero-extended; GetPad's `add esp,4` leaves the other three pushes as StartShake's arguments).
        0x015B => {
            collect(s, 3);
            let pad = get_pad(spi(0));
            start_shake(pad, sp(1) & 0xFFFF, sp(2) & 0xFF, 0);
        }
        // 0158 POINT_CAMERA_AT_CAR (0x47C3F9): null vehicle -> nothing; TakeControl(veh, (u16)P1,
        // (u16)P2, 1) (the narrow values are pushed zero-extended).
        0x0158 => {
            collect(s, 3);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            if !veh.is_null() {
                take_control(
                    GPtr::from_addr(THE_CAMERA),
                    veh,
                    sp(1) & 0xFFFF,
                    sp(2) & 0xFFFF,
                    1,
                );
            }
        }
        // 0159 POINT_CAMERA_AT_CHAR (0x47C446): null ped -> nothing; a player ped is replaced by
        // nullptr: TakeControl(IsPlayer() ? nullptr : ped, (u16)P1, (u16)P2, 1).
        0x0159 => {
            collect(s, 3);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            if !ped.is_null() {
                let e = if ped_is_player(ped) != 0 { GPtr::NULL } else { ped };
                take_control(GPtr::from_addr(THE_CAMERA), e, sp(1) & 0xFFFF, sp(2) & 0xFFFF, 1);
            }
        }
        // 015D SET_TIME_SCALE (0x47C518): raw dword copy into CTimer::ms_fTimeScale.
        0x015D => {
            collect(s, 1);
            GPtr::<u32>::from_addr(TIME_SCALE).write(sp(0));
        }
        // 015F SET_FIXED_CAMERA_POSITION (0x47C537): SetCamPositionForFixedMode(&(P0,P1,P2), &(P3,P4,P5)).
        0x015F => {
            collect(s, 6);
            let up = [sp(3), sp(4), sp(5)];
            let pos = [sp(0), sp(1), sp(2)];
            set_cam_position_for_fixed_mode(GPtr::from_addr(THE_CAMERA), vec_ptr(&pos), vec_ptr(&up));
        }
        // 0160 POINT_CAMERA_AT_POINT (0x47C598): -100 ground rule on z; TakeControlNoEntity(&(x,y,z),
        // (u16)P3, 1).
        0x0160 => {
            collect(s, 4);
            let (x, y) = (sp(0), sp(1));
            let z = ground_rule(x, y, sp(2));
            let pos = [x, y, z];
            take_control_no_entity(GPtr::from_addr(THE_CAMERA), vec_ptr(&pos), sp(3) & 0xFFFF, 1);
        }
        // 0161 ADD_BLIP_FOR_CAR_OLD (0x47C610): discarded GetActualBlipArrayIndex of the output operand;
        // SetEntityBlip(1 CAR, P0, P1, P2, m_szName); no scale call.
        0x0161 => {
            collect(s, 3);
            get_actual_blip_array_index(collect_next(s));
            let h = set_entity_blip(1, spi(0), sp(1), sp(2), s.byte_add(8));
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 0164 REMOVE_BLIP (0x47C660)
        0x0164 => {
            collect(s, 1);
            clear_blip(spi(0));
        }
        // 0165 CHANGE_BLIP_COLOUR (0x47C682)
        0x0165 => {
            collect(s, 2);
            change_blip_colour(spi(0), sp(1));
        }
        // 0168 CHANGE_BLIP_SCALE (0x47C764)
        0x0168 => {
            collect(s, 2);
            change_blip_scale(spi(0), spi(1));
        }
        // 0169 SET_FADING_COLOUR (0x47C78C): TheCamera.SetFadeColour((u8)P0, (u8)P1, (u8)P2).
        0x0169 => {
            collect(s, 3);
            set_fade_colour(
                GPtr::from_addr(THE_CAMERA),
                sp(0) & 0xFF,
                sp(1) & 0xFF,
                sp(2) & 0xFF,
            );
        }
        // 016B GET_FADING_STATUS (0x47C831): flag = TheCamera.GetFading() != 0.
        0x016B => {
            let b = get_fading(GPtr::from_addr(THE_CAMERA)) != 0;
            set_flag(s, b);
        }
        // 016C ADD_HOSPITAL_RESTART (0x47C85A): -100 ground rule; AddHospitalRestartPoint(&(x,y,z), P3,
        // P4). The original keeps P4 in ESI (`this` is not used afterwards).
        0x016C => {
            collect(s, 5);
            let (x, y) = (sp(0), sp(1));
            let z = ground_rule(x, y, sp(2));
            let pos = [x, y, z];
            add_hospital_restart_point(vec_ptr(&pos), sp(3), spi(4));
        }
        // 016D ADD_POLICE_RESTART (0x47C8DC): as 016C with AddPoliceRestartPoint(&(x,y,z), P3, P4).
        0x016D => {
            collect(s, 5);
            let (x, y) = (sp(0), sp(1));
            let z = ground_rule(x, y, sp(2));
            let pos = [x, y, z];
            add_police_restart_point(vec_ptr(&pos), sp(3), spi(4));
        }
        // 016E OVERRIDE_NEXT_RESTART (0x47C95E): CollectParameters(4); -100 ground rule;
        // OverrideNextRestart(&(x,y,z), P3).
        0x016E => {
            collect(s, 4);
            let (x, y) = (sp(0), sp(1));
            let z = ground_rule(x, y, sp(2));
            let pos = [x, y, z];
            override_next_restart(vec_ptr(&pos), sp(3));
        }
        // 0173 SET_CHAR_HEADING (0x47CB44): no null check. Heading wrapped in ST0; when the ped is in a
        // vehicle (`m_nPedFlags` bit 8 at +0x46C and `m_pVehicle` +0x58C != 0) nothing else; otherwise
        // `fmul [0.017453292]`, stored to m_fCurrentRotation (+0x558), m_fAimingRotation (+0x55C) and
        // passed to SetHeading, then UpdateRwMatrix. (The `fst [esp+0x10]` temp is a dead stack slot.)
        0x0173 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let st = wrap_heading_deg(sp(1));
            let in_vehicle = ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0
                && ped.byte_add(0x58C).cast::<u32>().read() != 0;
            if !in_vehicle {
                let r = st.fmul_m(f(K_DEG2RAD)).to_f32().to_bits();
                ped.byte_add(0x558).cast::<u32>().write(r);
                ped.byte_add(0x55C).cast::<u32>().write(r);
                set_heading(ped, f(r));
                update_rw_matrix(ped);
            }
        }
        // 0175 SET_CAR_HEADING (0x47CC64): no null check; wrapped heading `* 0.017453292` (ST0) to
        // SetHeading, then UpdateRwMatrix.
        0x0175 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let r = wrap_heading_deg(sp(1)).fmul_m(f(K_DEG2RAD)).to_f32();
            set_heading(veh, r);
            update_rw_matrix(veh);
        }
        // 017B SET_CHAR_AMMO (0x47CDC7): no null check; SetAmmo(P1, P2).
        0x017B => {
            collect(s, 3);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            set_ammo(ped, sp(1), sp(2));
        }
        // 0180 DECLARE_MISSION_FLAG (0x47CE00): the operand is read by hand: `m_IP += 1` (the type byte
        // is skipped unchecked), `OnAMissionFlag = *(u16*)m_IP` (zero-extended), `m_IP += 2`.
        0x0180 => {
            let ip = s.byte_add(crate::vm::IP).cast::<u32>();
            let p = ip.read().wrapping_add(1);
            ip.write(p);
            let v = GPtr::<u16>::from_addr(p as usize).read();
            ip.write(p.wrapping_add(2));
            GPtr::<u32>::from_addr(ON_A_MISSION_FLAG).write(u32::from(v));
        }
        // 0184 IS_CHAR_HEALTH_GREATER (0x47CE23) / 0185 IS_CAR_HEALTH_GREATER (0x47CE71): no null check.
        // `fild P1; fcomp [health]` (ped +0x540, vehicle +0x4C0); `test ah,5; jnp` keeps flag 1 only for
        // `(float)P1 < health` (equal, greater and unordered give 0).
        0x0184 | 0x0185 => {
            collect(s, 2);
            let (e, off) = if command == 0x0184 {
                (ped_at_ref(pool(PED_POOL), spi(0)), 0x540)
            } else {
                (vehicle_at_ref(pool(VEHICLE_POOL), spi(0)), 0x4C0)
            };
            let health = f(e.byte_add(off).cast::<u32>().read());
            let c = X87::from_i32(spi(1)).fcom(X87::from_f32(health));
            set_flag(s, !c.test_parity(5));
        }
        // 0186 ADD_BLIP_FOR_CAR (0x47CEBF), 0187 ADD_BLIP_FOR_CHAR (0x47CEFD), 0188 ADD_BLIP_FOR_OBJECT
        // (0x47CF25): discarded GetActualBlipArrayIndex of the output operand; h = SetEntityBlip(type, P0,
        // colour, 3 BOTH, m_szName) with (type, colour) = (1, 0) / (2, 1) / (3, 6); ChangeBlipScale(h, 3);
        // P0 = h; StoreParameters(1) (shared tail 0x47CFF2).
        0x0186..=0x0188 => {
            collect(s, 1);
            get_actual_blip_array_index(collect_next(s));
            let (ty, colour) = [(1, 0), (2, 1), (3, 6)][(command - 0x0186) as usize];
            let h = set_entity_blip(ty, spi(0), colour, 3, s.byte_add(8));
            change_blip_scale(h, 3);
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 018A ADD_BLIP_FOR_COORD (0x47CF4E): `fld z; fst local; fcomp -100.0; test ah,0x41; jp`: when
        // `z <= -100` (ordered) local z = FindGroundZForCoord(x, y); discarded GetActualBlipArrayIndex of
        // the output operand; h = SetCoordBlip(4 COORD, (x,y,z), 5, 3 BOTH, m_szName); ChangeBlipScale(h, 3);
        // P0 = h; StoreParameters(1).
        0x018A => {
            collect(s, 3);
            let (x, y) = (sp(0), sp(1));
            let z = ground_rule(x, y, sp(2));
            get_actual_blip_array_index(collect_next(s));
            let h = set_coord_blip(4, x, y, z, 5, 3, s.byte_add(8));
            change_blip_scale(h, 3);
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 018B CHANGE_BLIP_DISPLAY (0x47D00B)
        0x018B => {
            collect(s, 2);
            change_blip_display(spi(0), sp(1));
        }
        // 018C ADD_ONE_OFF_SOUND (0x47D034): AudioEngine.ReportMissionAudioEvent((u16)P3, &(P0,P1,P2)).
        0x018C => {
            collect(s, 4);
            let pos = [sp(0), sp(1), sp(2)];
            report_mission_audio_event(GPtr::from_addr(AUDIO_ENGINE), sp(3) & 0xFFFF, vec_ptr(&pos));
        }
        // 018D ADD_CONTINUOUS_SOUND (0x47D07D): 4 parameters read, ScriptParams[0] stored back unchanged
        // (no sound is created).
        0x018D => {
            collect(s, 4);
            store(s, 1);
        }
        // 018E REMOVE_SOUND (0x47D099): 1 parameter read, nothing else.
        0x018E => collect(s, 1),
        // 018F IS_CAR_STUCK_ON_ROOF (0x47D0AC): UpdateCompareFlag(UpsideDownCars.HasCarBeenUpsideDownForAWhile(P0) != 0).
        0x018F => {
            collect(s, 1);
            let b = has_car_been_upside_down_for_a_while(GPtr::from_addr(UPSIDE_DOWN_CARS), spi(0)) != 0;
            set_flag(s, b);
        }
        _ => {
            return fallback(
                process_commands_300_to_399_original,
                "CRunningScript::ProcessCommands300To399",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_300_TO_399];
