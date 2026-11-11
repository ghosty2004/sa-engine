//! `CRunningScript::ProcessCommands800To899` (0x481300): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands800To899.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0323, 0x0330, 0x0331, 0x0332, 0x0335, 0x0338, 0x033F, 0x0341, 0x0342, 0x0343, 0x0344, 0x0345, 0x0348,
    0x0349, 0x0350, 0x0362, 0x0366, 0x036A, 0x0373, 0x0382,
];

/// `CGarages::RespraysAreFree` (byte) 0x96C009.
const RESPRAYS_ARE_FREE: usize = 0x96C009;
/// `CTheScripts::NumberOfIntroTextLinesThisFrame` (u16) 0xA44B68.
const NUM_INTRO_TEXT_LINES: usize = 0xA44B68;
/// `CTheScripts::IntroTextLines` (`tScriptText`, 0x44 bytes each) 0xA913E8.
const INTRO_TEXT_LINES: u32 = 0xA913E8;

/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;

orig! {
    /// `CPed::SetStayInSamePlace(bool)` (0x481090)
    fn set_stay_in_same_place(p: GPtr<u8>, stay: u32) = thiscall 0x481090 => unported;
    /// Unnamed `CObject` method returning AL (0x46A2F0, a `.HOODLUM` stub; HAS_OBJECT_BEEN_DAMAGED's callee)
    fn object_has_been_damaged(o: GPtr<u8>) -> u8 = thiscall 0x46A2F0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (read as f64: exact)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CPedIntelligence::FlushImmediately(bool)` (0x601640)
    fn flush_immediately(i: GPtr<u8>, set_primary_default: u32) = thiscall 0x601640 => unported;
    /// `CEntity::GetDistanceFromCentreOfMassToBaseOfModel()` (0x536BE0), ST0
    fn get_distance_from_centre_of_mass_to_base_of_model(e: GPtr<u8>) -> f64 = thiscall 0x536BE0 => unported;
    /// `CEntity::Teleport(CVector, bool)` (vtable slot 0x38); the by-value vector as three raw dwords
    fn teleport(e: GPtr<u8>, x: u32, y: u32, z: u32, reset_rotation: u32) = thiscall vslot 0x38 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    fn clear_space_for_mission_entity(pos: GPtr<u32>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// `CTaskSimpleCarSetPedInAsDriver::CTaskSimpleCarSetPedInAsDriver(CVehicle*, CTaskUtilityLineUpPedWithCar*)` (0x6470E0)
    fn set_ped_in_as_driver_ctor(t: GPtr<u8>, veh: GPtr<u8>, line_up: GPtr<u8>) -> GPtr<u8> = thiscall 0x6470E0 => unported;
    /// `CTaskSimpleCarSetPedInAsDriver::ProcessPed(CPed*)` (0x64B950)
    fn set_ped_in_as_driver_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x64B950 => unported;
    /// `CTaskSimpleCarSetPedInAsDriver::~CTaskSimpleCarSetPedInAsDriver()` (0x647170)
    fn set_ped_in_as_driver_dtor(t: GPtr<u8>) = thiscall 0x647170 => unported;
    /// `CCamera::SetCameraDirectlyBehindForFollowPed_CamOnAString()` (0x50BD40)
    fn set_camera_directly_behind_for_follow_ped_cam_on_a_string(cam: GPtr<u8>) = thiscall 0x50BD40 => unported;
}

/// The current intro text line: `IntroTextLines[NumberOfIntroTextLinesThisFrame]` (`movzx` of the u16,
/// `imul 0x44`), re-read at every access as compiled.
#[inline]
fn text_line() -> GPtr<u8> {
    let n = u32::from(GPtr::<u16>::from_addr(NUM_INTRO_TEXT_LINES).read());
    GPtr::from_addr(INTRO_TEXT_LINES.wrapping_add(n.wrapping_mul(0x44)) as usize)
}

/// `dword [e+off]` bit `mask` = `v != 0` (read, or/and, write).
#[inline]
fn set_dword_flag(e: GPtr<u8>, off: isize, mask: u32, v: u32) {
    let p = e.byte_add(off).cast::<u32>();
    let old = p.read();
    p.write(if v != 0 { old | mask } else { old & !mask });
}

/// The common return-0 tail 0x4835B1 (via the .text stub 0x4039D5): `~CMatrix` on the function-scope
/// `CMatrix` at [esp+0x9C], whose attach fields (+0x40, +0x44) the prologue zeroes (so the destructor
/// frees nothing). The port runs the call on a zeroed local of the same size (0x48 bytes).
#[inline]
fn common_tail() {
    let m = [0u32; 0x12];
    matrix_dtor(GPtr::from_raw(m.as_ptr().cast_mut()).cast());
}

/// `CRunningScript::ProcessCommands800To899(int32 command)` (0x481300, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x481300, conv = thiscall, name = "CRunningScript::ProcessCommands800To899", expect = "6A FF 68 A8 83 83 00", this = "CRunningScript", fallback)]
pub fn process_commands_800_to_899(s: Scr, command: u32) -> i8 {
    match command {
        // 0323 ANCHOR_BOAT (0x481435): no null check; byte +0x5AC bit 2 (0x4) = P1 != 0.
        0x0323 => {
            collect(s, 2);
            let boat = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let p = boat.byte_add(0x5AC);
            let b = p.read();
            p.write(if sp(1) != 0 { b | 0x4 } else { b & 0xFB });
        }
        // 0330 SET_PLAYER_NEVER_GETS_TIRED (0x48175F) / 0331 SET_PLAYER_FAST_RELOAD (0x48178E):
        // CWorld::Players[P0] byte +0x14C / +0x14D = P1 != 0.
        0x0330 | 0x0331 => {
            collect(s, 2);
            let off = if command == 0x0330 { 0x14C } else { 0x14D };
            player_info(sp(0)).byte_add(off).write(u8::from(sp(1) != 0));
        }
        // 0332 SET_CHAR_BLEEDING (0x4817BD): no null check; `m_nPedFlags` dword +0x46C bit 0x40000 = P1 != 0.
        0x0332 => {
            collect(s, 2);
            set_dword_flag(ped_at_ref(pool(PED_POOL), spi(0)), 0x46C, 0x40000, sp(1));
        }
        // 0335 SET_FREE_RESPRAYS (0x481808): CGarages::RespraysAreFree = P0 != 0.
        0x0335 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(RESPRAYS_ARE_FREE).write(u8::from(sp(0) != 0));
        }
        // 0338 SET_CAR_VISIBLE (0x481880): no null check; entity flags +0x1C bit 0x80 (bIsVisible) = P1 != 0.
        0x0338 => {
            collect(s, 2);
            set_dword_flag(vehicle_at_ref(pool(VEHICLE_POOL), spi(0)), 0x1C, 0x80, sp(1));
        }
        // 033F SET_TEXT_SCALE (0x481AAE): line +0 / +4 = P0 / P1 (raw dwords).
        0x033F => {
            collect(s, 2);
            let l = text_line();
            l.cast::<u32>().write(sp(0));
            l.byte_add(4).cast::<u32>().write(sp(1));
        }
        // 0341 SET_TEXT_JUSTIFY (0x481B47), 0342 SET_TEXT_CENTRE (0x481B84), 0345 SET_TEXT_BACKGROUND
        // (0x481C08), 0348 SET_TEXT_PROPORTIONAL (0x481C45): line byte +0xC / +0xD / +0xE / +0x1C = P0 != 0.
        0x0341 | 0x0342 | 0x0345 | 0x0348 => {
            collect(s, 1);
            let off = if command == 0x0341 {
                0xC
            } else if command == 0x0342 {
                0xD
            } else if command == 0x0345 {
                0xE
            } else {
                0x1C
            };
            let v = u8::from(sp(0) != 0);
            text_line().byte_add(off).write(v);
        }
        // 0343 SET_TEXT_WRAPX (0x481BC1), 0344 SET_TEXT_CENTRE_SIZE (0x481BE5), 0349 SET_TEXT_FONT
        // (0x481C82): line dword +0x10 / +0x14 / +0x28 = P0.
        0x0343 | 0x0344 | 0x0349 => {
            collect(s, 1);
            let off = if command == 0x0343 {
                0x10
            } else if command == 0x0344 {
                0x14
            } else {
                0x28
            };
            text_line().byte_add(off).cast::<u32>().write(sp(0));
        }
        // 0350 SET_CHAR_STAY_IN_SAME_PLACE (0x482B4D): no null check; SetStayInSamePlace(P1 != 0).
        0x0350 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            set_stay_in_same_place(ped, u32::from(sp(1) != 0));
        }
        // 0366 HAS_OBJECT_BEEN_DAMAGED (0x4831F8): no null check; flag = 0x46A2F0(obj) != 0.
        0x0366 => {
            collect(s, 1);
            let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
            let b = object_has_been_damaged(obj) != 0;
            set_flag(s, b);
        }
        // 0382 SET_OBJECT_COLLISION (0x483580): no null check; entity flags +0x1C bit 0 (bUsesCollision) = P1 != 0.
        0x0382 => {
            collect(s, 2);
            set_dword_flag(object_at_ref(pool(OBJECT_POOL), spi(0)), 0x1C, 0x1, sp(1));
        }
        // 0362 WARP_CHAR_FROM_CAR_TO_COORD (0x482EA3): no null check. `fld P3; fst z; fcomp -100.0; test
        // ah,0x41; jp`: when `z <= -100` (ordered) z = FindGroundZForCoord(P1, P2). Then
        // `m_pIntelligence (+0x47C)->FlushImmediately(true)`; z = GetDistanceFromCentreOfMassToBaseOfModel()
        // `fadd [z]`; `Teleport((P1, P2, z), false)` (EDI = 0 for the whole switch); ClearSpaceForMissionEntity.
        0x0362 => {
            collect(s, 4);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let st = X87::from_f32(f(sp(3)));
            let (x, y) = (sp(1), sp(2));
            let mut z = st.to_f32().to_bits();
            if !st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
                z = X87::from_f64_exact(find_ground_z_for_coord(x, y))
                    .to_f32()
                    .to_bits();
            }
            flush_immediately(ped.byte_add(0x47C).cast::<GPtr<u8>>().read(), 1);
            let d = X87::from_f64_exact(get_distance_from_centre_of_mass_to_base_of_model(ped));
            let z = d.fadd_m(f(z)).to_f32().to_bits();
            teleport(ped, x, y, z, 0);
            let pos = [x, y, z];
            clear_space_for_mission_entity(GPtr::from_raw(pos.as_ptr().cast_mut()), ped);
        }
        // 036A WARP_CHAR_INTO_CAR (0x483239): no null checks. `m_pIntelligence->FlushImmediately(false)`;
        // a stack `CTaskSimpleCarSetPedInAsDriver(veh, nullptr)` (0x1C bytes) with
        // `m_bIsWarpingPedIntoCar` (+0x18) = 1 set after the constructor (gr passes it to the constructor);
        // ProcessPed(ped); destructor. The SEH state stores around the object are not ported.
        0x036A => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            flush_immediately(ped.byte_add(0x47C).cast::<GPtr<u8>>().read(), 0);
            let mut task = [0u32; 7];
            let t = GPtr::from_raw(task.as_mut_ptr()).cast::<u8>();
            set_ped_in_as_driver_ctor(t, veh, GPtr::NULL);
            t.byte_add(0x18).write(1);
            set_ped_in_as_driver_process_ped(t, ped);
            set_ped_in_as_driver_dtor(t);
        }
        // 0373 SET_CAMERA_BEHIND_PLAYER (0x48331B)
        0x0373 => set_camera_directly_behind_for_follow_ped_cam_on_a_string(GPtr::from_addr(THE_CAMERA)),
        _ => {
            return fallback(
                process_commands_800_to_899_original,
                "CRunningScript::ProcessCommands800To899",
                s,
                command,
            );
        }
    }
    common_tail();
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_800_TO_899];
