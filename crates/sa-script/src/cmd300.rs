//! `CRunningScript::ProcessCommands300To399` (0x47C100): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1). Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands300To399.md`.
//!
//! The heading reads (0172/0174/0176) take `CPlaceable::GetHeading`'s FPATAN result in ST0 as f64 (53
//! of 64 significand bits) before the 24-bit `fmul`; DRAW_SHADOW's FSIN/FCOS are modelled at 53 bits.
//! Both accepted project-wide (double rounding can differ in the last f32 bit, not observed).
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
    0x0137, 0x014B, 0x014C, 0x014E, 0x014F, 0x0151, 0x0154, 0x0158, 0x0159, 0x015A, 0x015B, 0x015D, 0x015F,
    0x0160, 0x0161, 0x0162, 0x0164, 0x0165, 0x0166, 0x0167, 0x0168, 0x0169, 0x016A, 0x016B, 0x016C, 0x016D,
    0x016E, 0x016F, 0x0172, 0x0173, 0x0174, 0x0175, 0x0176, 0x0177, 0x0179, 0x017B, 0x0180, 0x0181, 0x0184,
    0x0185, 0x0186, 0x0187, 0x0188, 0x0189, 0x018A, 0x018B, 0x018C, 0x018D, 0x018E, 0x018F,
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
/// `.rdata` 0x859F00: 0.015f (CREATE_CAR_GENERATOR's z lift)
const K_0_015: u32 = 0x3C75_C28F;
/// `.rdata` 0x858CDC: 0.001f (DO_FADE's ms to s)
const K_0_001: u32 = 0x3A83_126F;
/// `.rdata` 0x859878: 57.295776f (radians to degrees)
const K_RAD2DEG: u32 = 0x4265_2EE0;
/// `CTheCarGenerators::CarGeneratorArray` 0xC27AD0 (`CCarGenerator`, 0x20 bytes each).
const CAR_GENERATORS: usize = 0xC27AD0;
/// `CTheScripts::bScriptHasFadedOut` (byte) 0xA4448A.
const SCRIPT_HAS_FADED_OUT: usize = 0xA4448A;

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
    /// `CTheCarGenerators::CreateCarGenerator(CVector, float angle, int32 model, int16 colour1, int16
    /// colour2, uint8 forceSpawn, uint8 alarm, uint8 doorLock, uint16 minDelay, uint16 maxDelay, uint8
    /// iplId, bool owned)` (0x6F31A0); the vector and floats as raw dwords, narrow values zero-extended
    fn create_car_generator(x: u32, y: u32, z: u32, angle: u32, model: u32, c1: u32, c2: u32, force: u32, alarm: u32, lock: u32, min_delay: u32, max_delay: u32, ipl: u32, owned: u32) -> i32 = cdecl 0x6F31A0 => unported;
    /// `CCarGenerator::SwitchOff()` (0x6F2E30)
    fn car_generator_switch_off(g: GPtr<u8>) = thiscall 0x6F2E30 => unported;
    /// `CCarGenerator::SwitchOn()` (0x6F32C0)
    fn car_generator_switch_on(g: GPtr<u8>) = thiscall 0x6F32C0 => unported;
    /// Unnamed thiscall on a `CCarGenerator` taking the count (0x47C0E0, a `.HOODLUM` stub; gr's
    /// `SwitchOn(); m_nGenerateCount = count`)
    fn car_generator_47c0e0(g: GPtr<u8>, count: u32) = thiscall 0x47C0E0 => unported;
    /// `CTheZones::FindZone(const CVector*, uint64 label, eZoneType)` (0x572B80), the label as two
    /// stack dwords, AL
    fn find_zone(pos: GPtr<u32>, lo: u32, hi: u32, ty: u32) -> u8 = cdecl 0x572B80 => unported;
    /// `CCamera::Fade(float time, int16 direction)` (0x50AC20), the float as its raw dword
    fn camera_fade(cam: GPtr<u8>, time: u32, dir: u32) = thiscall 0x50AC20 => unported;
    /// `CShadows::StoreShadowToBeRendered(uint8 type, CVector* pos, float frontX, float frontY, float
    /// sideX, float sideY, int16 intensity, uint8 r, uint8 g, uint8 b)` (0x707930); floats as raw dwords
    fn store_shadow_to_be_rendered(ty: u32, pos: GPtr<u32>, fx: u32, fy: u32, sx: u32, sy: u32, intensity: u32, r: u32, g: u32, b: u32) = cdecl 0x707930 => unported;
    /// `CPlaceable::GetHeading()` (0x441DB0), ST0 (an FPATAN result: read as f64, i.e. 53 of its 64
    /// significand bits, as sa-ai's car_exit does)
    fn get_heading(e: GPtr<u8>) -> f64 = thiscall 0x441DB0 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CPhysical::GetHasCollidedWith(CEntity*)` (0x543540), AL
    fn get_has_collided_with(p: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x543540 => unported;
}

/// The ped's vehicle when `m_nPedFlags` bit 8 (+0x46C) is set and `m_pVehicle` (+0x58C) != 0, else the
/// ped (`GetPedOrItsVehicle`, inlined; no null check on the ped).
fn ped_or_its_vehicle(ped: GPtr<u8>) -> GPtr<u8> {
    if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
        let v = ped.byte_add(0x58C).cast::<GPtr<u8>>().read();
        if !v.is_null() {
            return v;
        }
    }
    ped
}

/// `CPlaceable::GetPosition()` inlined: `m_matrix` (+0x14) ? `&m_matrix->pos` (+0x30) : `&m_placement.pos`
/// (+0x4), copied as three raw dwords.
fn entity_pos(e: GPtr<u8>) -> [u32; 3] {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    let p = if m.is_null() { e.byte_add(0x4) } else { m.byte_add(0x30) }.cast::<u32>();
    [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()]
}

/// The heading tail of 0174/0176 (0x47CC03): `fmul [57.295776]; fst local; fcomp [0.0]` (the compare
/// on ST0), `+ 360` through the f32 local when below 0 (`test ah,5; jp`), then `fld local; fcomp
/// [360.0]` and `- 360` when above (`test ah,0x41; jne`). The GetHeading ST0 is read as f64 (53 bits),
/// then rounded to 24 bits by the `fmul` (accepted project-wide for FPATAN results).
fn heading_deg_tail(h: f64) -> u32 {
    let st = X87::from_f64_exact(h).fmul_m(f(K_RAD2DEG));
    let mut v = st.to_f32();
    if !st.fcom(X87::constant(K_ZERO)).test_parity(5) {
        v = X87::from_f32(v).fadd_m(f(K_360)).to_f32();
    }
    if !X87::from_f32(v).fcom(X87::constant(K_360)).test_any(0x41) {
        v = X87::from_f32(v).fsub_m(f(K_360)).to_f32();
    }
    v.to_bits()
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
#[hook(addr = 0x47C100, conv = thiscall, name = "CRunningScript::ProcessCommands300To399", expect = "8B 44 24 04 83 EC 7C", this = "CRunningScript")]
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
        // 014B CREATE_CAR_GENERATOR (0x47C169): CollectParameters(12); z += 0.015 (`fld; fadd; fstp`
        // through the local) when `z > -100` (`fcomp; test ah,0x41; jne`: ordered greater);
        // P0 = CreateCarGenerator((x,y,z), P3, P4, (u16)P5, (u16)P6, (u8)P7, (u8)P8, (u8)P9, (u16)P10,
        // (u16)P11, 0, 1); StoreParameters(1). gr: unregistered.
        0x014B => {
            collect(s, 12);
            let (x, y) = (sp(0), sp(1));
            let mut z = sp(2);
            if !X87::from_f32(f(z)).fcom(X87::constant(K_MINUS_100)).test_any(0x41) {
                z = X87::from_f32(f(z)).fadd_m(f(K_0_015)).to_f32().to_bits();
            }
            let h = create_car_generator(
                x,
                y,
                z,
                sp(3),
                sp(4),
                sp(5) & 0xFFFF,
                sp(6) & 0xFFFF,
                sp(7) & 0xFF,
                sp(8) & 0xFF,
                sp(9) & 0xFF,
                sp(10) & 0xFFFF,
                sp(11) & 0xFFFF,
                0,
                1,
            );
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 014C SWITCH_CAR_GENERATOR (0x47C22F): g = &CarGeneratorArray[(u16)P0]; P1 == 0 -> SwitchOff;
        // P1 > 100 (signed) -> SwitchOn only; otherwise (negative included) the `.HOODLUM` thiscall
        // 0x47C0E0(g, P1). gr: SwitchOn for any non-zero count, then the count when <= 100.
        0x014C => {
            collect(s, 2);
            let g = GPtr::<u8>::from_addr(CAR_GENERATORS.wrapping_add(((sp(0) & 0xFFFF) << 5) as usize));
            let n = spi(1);
            if n == 0 {
                car_generator_switch_off(g);
            } else if n > 100 {
                car_generator_switch_on(g);
            } else {
                car_generator_47c0e0(g, n as u32);
            }
        }
        // 0154 IS_CHAR_IN_ZONE (0x47C310): no null check. The 8-byte label is read into a buffer, then
        // copied byte by byte up to and including its NUL into a zeroed 8-byte local whose two dwords are
        // passed by value: FindZone(&pos, lo, hi, 0), pos = the ped's vehicle position when in one, else
        // the ped's. `CPed::IsPlayer()` is called and its result discarded (gr: no call). Divergence
        // note: the label buffer is a zeroed 16-byte buffer (the original's is not cleared; for an
        // 8-character label its copy loop runs on through stale stack bytes and past the 8-byte local
        // into the label buffer, a dead write; the two dwords passed are the same).
        0x0154 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let mut buf = [0u8; 16];
            read_text_label(s, GPtr::from_raw(buf.as_mut_ptr()), 8);
            let mut zone = [0u8; 8];
            for (d, &c) in zone.iter_mut().zip(buf.iter()) {
                *d = c;
                if c == 0 {
                    break;
                }
            }
            ped_is_player(ped);
            let pos = entity_pos(ped_or_its_vehicle(ped));
            let lo = u32::from_le_bytes([zone[0], zone[1], zone[2], zone[3]]);
            let hi = u32::from_le_bytes([zone[4], zone[5], zone[6], zone[7]]);
            let b = find_zone(vec_ptr(&pos), lo, hi, 0) != 0;
            set_flag(s, b);
        }
        // 0162 ADD_BLIP_FOR_CHAR_OLD, 0166 DIM_BLIP, 0181 DECLARE_MISSION_FLAG_FOR_CONTACT, 0189
        // ADD_BLIP_FOR_CONTACT_POINT (0x47D0E0, gr: unimplemented): straight to the epilogue, operands not
        // read, return 0.
        0x0162 | 0x0166 | 0x0181 | 0x0189 => {}
        // 0167 ADD_BLIP_FOR_COORD_OLD (0x47C6AA): CollectParameters(5); -100 ground rule on z; discarded
        // GetActualBlipArrayIndex of the output operand; P0 = SetCoordBlip(4 COORD, (x,y,z), P3, P4,
        // m_szName) (no scale call); StoreParameters(1). gr: unregistered.
        0x0167 => {
            collect(s, 5);
            let (x, y) = (sp(0), sp(1));
            let z = ground_rule(x, y, sp(2));
            get_actual_blip_array_index(collect_next(s));
            let h = set_coord_blip(4, x, y, z, sp(3), sp(4), s.byte_add(8));
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 016A DO_FADE (0x47C7C3): TheCamera.Fade(`fild P0; fmul [0.001]`, (u16)P1); then, with mission
        // cleanup only, bScriptHasFadedOut = (P1 != 1) (the full dword compared).
        0x016A => {
            collect(s, 2);
            let t = X87::from_i32(spi(0)).fmul_m(f(K_0_001)).to_f32().to_bits();
            camera_fade(GPtr::from_addr(THE_CAMERA), t, sp(1) & 0xFFFF);
            if uses_mission_cleanup(s) {
                GPtr::<u8>::from_addr(SCRIPT_HAS_FADED_OUT).write(u8::from(sp(1) != 1));
            }
        }
        // 016F DRAW_SHADOW (0x47C9DF): CollectParameters(10). When the angle P4 == 0 (`test ah,0x44;
        // jp`: ordered equal) front = (P5, 0); else front = (`fcos(P4) * P5`, `fsin(P4) * P5`), each
        // stored as f32 (FSIN/FCOS modelled at 53 bits, accepted project-wide).
        // StoreShadowToBeRendered((u8)P0, &(P1,P2,P3), -front.y (`fchs` of the f32), front.x, front.x,
        // front.y, (u16)P6, (u8)P7, (u8)P8, (u8)P9).
        0x016F => {
            collect(s, 10);
            let pos = [sp(1), sp(2), sp(3)];
            let a = X87::from_f32(spf(4));
            let (fx, fy) = if !a.fcom(X87::constant(K_ZERO)).test_parity(0x44) {
                (sp(5), 0)
            } else {
                (
                    a.fcos().fmul_m(spf(5)).to_f32().to_bits(),
                    a.fsin().fmul_m(spf(5)).to_f32().to_bits(),
                )
            };
            let nfy = X87::from_f32(f(fy)).fchs().to_f32().to_bits();
            store_shadow_to_be_rendered(
                sp(0) & 0xFF,
                vec_ptr(&pos),
                nfy,
                fx,
                fx,
                fy,
                sp(6) & 0xFFFF,
                sp(7) & 0xFF,
                sp(8) & 0xFF,
                sp(9) & 0xFF,
            );
        }
        // 0172 GET_CHAR_HEADING (0x47CAA9): no null check; GetHeading of the ped's vehicle when in one,
        // else of the ped (ST0 read as f64, FPATAN at 53 bits accepted), `fmul [57.295776]; fstp local`;
        // `+ 360` when local < 0, `- 360` when local > 360 (each through the f32 local); P0 = local;
        // StoreParameters(1).
        0x0172 => {
            collect(s, 1);
            let e = ped_or_its_vehicle(ped_at_ref(pool(PED_POOL), spi(0)));
            let mut v = X87::from_f64_exact(get_heading(e)).fmul_m(f(K_RAD2DEG)).to_f32();
            if !X87::from_f32(v).fcom(X87::constant(K_ZERO)).test_parity(5) {
                v = X87::from_f32(v).fadd_m(f(K_360)).to_f32();
            }
            if !X87::from_f32(v).fcom(X87::constant(K_360)).test_any(0x41) {
                v = X87::from_f32(v).fsub_m(f(K_360)).to_f32();
            }
            set_sp(0, v.to_bits());
            store(s, 1);
        }
        // 0174 GET_CAR_HEADING (0x47CBE1, gr: unregistered) / 0176 GET_OBJECT_HEADING (0x47CCCE, joins
        // 0174 at 0x47CBFC): no null check; P0 = heading_deg_tail(GetHeading(e)); StoreParameters(1).
        0x0174 | 0x0176 => {
            collect(s, 1);
            let e = if command == 0x0174 {
                vehicle_at_ref(pool(VEHICLE_POOL), spi(0))
            } else {
                object_at_ref(pool(OBJECT_POOL), spi(0))
            };
            set_sp(0, heading_deg_tail(get_heading(e)));
            store(s, 1);
        }
        // 0177 SET_OBJECT_HEADING (0x47CCEE): no null check; CWorld::Remove(obj); the heading wrapped in
        // ST0 (as 0173) `* 0.017453292` to SetHeading; UpdateRwMatrix; UpdateRwFrame; CWorld::Add(obj).
        0x0177 => {
            collect(s, 2);
            let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
            world_remove(obj);
            let r = wrap_heading_deg(sp(1)).fmul_m(f(K_DEG2RAD)).to_f32();
            set_heading(obj, r);
            update_rw_matrix(obj);
            update_rw_frame(obj);
            world_add(obj);
        }
        // 0179 IS_CHAR_TOUCHING_OBJECT (0x47CD71): no null checks; both lookups first; flag =
        // GetHasCollidedWith(ped's vehicle when in one, else the ped; obj) != 0.
        0x0179 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let obj = object_at_ref(pool(OBJECT_POOL), spi(1));
            let b = get_has_collided_with(ped_or_its_vehicle(ped), obj) != 0;
            set_flag(s, b);
        }
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_300_TO_399];
