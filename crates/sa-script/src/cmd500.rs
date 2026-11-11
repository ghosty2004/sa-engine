//! `CRunningScript::ProcessCommands500To599` (0x47E090): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1). Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands500To599.md`.
//!
//! Divergence notes: the 8-byte text labels (01F9, 0217, 021B, 023C) are read into a zeroed 16-byte buffer
//! and 0245's 16-byte label into a zeroed 17-byte one; the original's stack buffers are not cleared, so
//! a label that fills its buffer is followed by stale stack bytes in the C-string callee. Dead stack
//! writes not ported: the EH state slot of 0223 and the flag byte in the command-argument slot.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x01F4, 0x01F5, 0x01F6, 0x01F7, 0x01F9, 0x01FA, 0x01FB, 0x0202, 0x0203, 0x0204, 0x0205, 0x0206, 0x0207,
    0x0208, 0x0209, 0x020A, 0x020B, 0x020C, 0x020D, 0x0213, 0x0214, 0x0215, 0x0216, 0x0217, 0x021B, 0x021C,
    0x0221, 0x0223, 0x0224, 0x0226, 0x0227, 0x0229, 0x022A, 0x022B, 0x0231, 0x0235, 0x0236, 0x0237, 0x023B,
    0x023C, 0x023D, 0x0241, 0x0244, 0x0245, 0x0247, 0x0248, 0x0249, 0x024F, 0x0250, 0x0253, 0x0254, 0x0256,
];

/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;
/// `.rdata` 0x858B8C: 0.5f
const K_HALF: u32 = 0x3F00_0000;
/// `CTheScripts::UsedObjectArray[0].nModelIndex` 0xA44B88 (0x1C bytes per entry; a negative model
/// operand `m` reads `0xA44B88 - m * 0x1C`).
const USED_OBJECT_MODEL: u32 = 0xA44B88;
/// `CStreaming::ms_aInfoForModel[0].m_LoadState` (0x8E4CC0 + 0x10, 0x14 bytes per model).
const MODEL0_LOAD_STATE: u32 = 0x8E4CD0;
/// `CTheScripts::ScriptResourceManager` 0xA485A8.
const SCRIPT_RESOURCE_MANAGER: usize = 0xA485A8;

orig! {
    /// `CRestart::CancelOverrideRestart()` (0x460800)
    fn cancel_override_restart() = cdecl 0x460800 => unported;
    /// `CDarkel::ReadStatus()` (0x43D1E0), AX
    fn darkel_read_status() -> u16 = cdecl 0x43D1E0 => unported;
    /// `CRunningScript::LocateCharCarCommand(int32 command)` (0x487420)
    fn locate_char_car_command(s: Scr, command: u32) = thiscall 0x487420 => unported;
    /// `CGeneral::GetRandomNumberInRange(float, float)` (0x41BD90), floats as raw dwords, ST0 (a PC=24
    /// result: exact as f64)
    fn random_float_in_range(lo: u32, hi: u32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_int_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CVehicle::BlowUpCar(CEntity*, bool)` (vtable slot 0xA4)
    fn blow_up_car(v: GPtr<u8>, culprit: GPtr<u8>, in_cutscene: u32) = thiscall vslot 0xA4 => unported;
    /// `CGangs::SetGangWeapons(int16 gang, int32, int32, int32)` (0x5DE550)
    fn set_gang_weapons(gang: u32, w1: i32, w2: i32, w3: i32) = cdecl 0x5DE550 => unported;
    /// `CPlayerInfo::IsPlayerInRemoteMode()` (0x56DAB0), AL
    fn is_player_in_remote_mode(info: GPtr<u8>) -> u8 = thiscall 0x56DAB0 => unported;
    /// Unnamed cdecl function taking `const CVector*` (0x47E070, a `.HOODLUM` stub; SET_CUTSCENE_OFFSET's
    /// callee, presumably `CCutsceneMgr::SetCutsceneOffset`)
    fn set_cutscene_offset(v: GPtr<u32>) = cdecl 0x47E070 => unported;
    /// `CClock::StoreClock()` (0x52D020)
    fn store_clock() = cdecl 0x52D020 => unported;
    /// `CClock::RestoreClock()` (0x52D070)
    fn restore_clock() = cdecl 0x52D070 => unported;
    /// `CRunningScript::CollectNextParameterWithoutIncreasingPC()` (0x464250)
    fn collect_next(s: Scr) -> i32 = thiscall 0x464250 => crate::vm::collect_next_parameter_without_increasing_pc;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (read as f64: exact)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CPickups::GetActualPickupIndex(int32)` (0x4552A0); the result is discarded here
    fn get_actual_pickup_index(h: i32) -> i32 = cdecl 0x4552A0 => unported;
    /// `CPickups::GenerateNewOne(CVector pos, uint32 model, uint8 type, uint32 ammo, uint32 money,
    /// bool isEmpty, const char* message)` (0x456F20); the by-value vector as three raw dwords
    fn generate_new_one(x: u32, y: u32, z: u32, model: u32, ty: u32, ammo: u32, money: u32, empty: u32, msg: u32) -> i32 = cdecl 0x456F20 => unported;
    /// `CPickups::IsPickUpPickedUp(int32)` (0x454B40), AL
    fn is_pick_up_picked_up(h: i32) -> u8 = cdecl 0x454B40 => unported;
    /// `CPickups::RemovePickUp(int32)` (0x4573D0)
    fn remove_pick_up(h: i32) = cdecl 0x4573D0 => unported;
    /// `CStreaming::RequestModel(int32 model, int32 flags)` (0x4087E0)
    fn request_model(model: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::SetMissionDoesntRequireModel(int32)` (0x409C90)
    fn set_mission_doesnt_require_model(model: i32) = cdecl 0x409C90 => unported;
    /// `CScriptResourceManager::AddToResourceManager(int32, uint32 type, CRunningScript*)` (0x4704B0)
    fn add_to_resource_manager(m: GPtr<u8>, model: i32, ty: u32, s: Scr) = thiscall 0x4704B0 => unported;
    /// `CScriptResourceManager::RemoveFromResourceManager(int32, uint32 type, CRunningScript*)` (0x470510), AL
    fn remove_from_resource_manager(m: GPtr<u8>, model: i32, ty: u32, s: Scr) -> u8 = thiscall 0x470510 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CText::Get(const char*)` (0x6A0050), ECX = `TheText`
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CDarkel::StartFrenzy(eWeaponType, int32 time, uint16 kills, int32 model, uint16* text, int32,
    /// int32, int32, bool standard, bool headshot)` (0x43D3B0)
    fn start_frenzy(w: u32, time: u32, kills: u32, m1: u32, text: GPtr<u8>, m2: u32, m3: u32, m4: u32, standard: u32, head: u32) = cdecl 0x43D3B0 => unported;
    /// `CExplosion::AddExplosion(CEntity* victim, CEntity* creator, eExplosionType, CVector pos,
    /// uint32 time, uint8 makeSound, float camShake, uint8 visible)` (0x736A50); the vector and the
    /// float as raw dwords
    fn add_explosion(victim: u32, creator: u32, ty: u32, x: u32, y: u32, z: u32, time: u32, sound: u32, shake: u32, visible: u32) = cdecl 0x736A50 => unported;
    /// `CAutomobile::SetTaxiLight(bool)` (0x6A3740)
    fn set_taxi_light(v: GPtr<u8>, on: u32) = thiscall 0x6A3740 => unported;
    /// `CMessages::AddBigMessageQ(const char*, uint32 time, uint16 style)` (0x69F370)
    fn add_big_message_q(text: GPtr<u8>, time: u32, style: u32) = cdecl 0x69F370 => unported;
    /// `CGarages::GetGarageNumberByName(const char*)` (0x447680); the caller tests the low word
    fn get_garage_number_by_name(name: GPtr<u8>) -> u32 = cdecl 0x447680 => unported;
    /// `CGarages::SetTargetCarForMissionGarage(int16 garage, CVehicle*)` (0x447C40)
    fn set_target_car_for_mission_garage(garage: u32, veh: GPtr<u8>) = cdecl 0x447C40 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: u32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexDie::CTaskComplexDie(eWeaponType, AssocGroupId, AnimationId, float blendDelta,
    /// float animSpeed, bool stealth, bool falling, int32 dir, bool railing)` (0x630040); floats as raw
    /// dwords
    fn task_complex_die_ctor(t: GPtr<u8>, w: u32, group: u32, anim: u32, blend: u32, speed: u32, stealth: u32, falling: u32, dir: u32, railing: u32) -> GPtr<u8> = thiscall 0x630040 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32 primaryTaskIndex, CTask*, bool)` (0x4B0A00)
    fn event_script_command_ctor(e: GPtr<u8>, index: u32, task: GPtr<u8>, affects_dead: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50, unnamed in the catalogue)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0), AL
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CWorld::FindPlayerSlotWithPedPointer(void*)` (0x563FA0)
    fn find_player_slot_with_ped_pointer(p: GPtr<u8>) -> u32 = cdecl 0x563FA0 => unported;
    /// `CPathFind::SwitchPedRoadsOffInArea(float x1, float x2, float y1, float y2, float z1, float z2,
    /// bool off, bool)` (0x452F00), ECX = `ThePaths`; floats as raw dwords
    fn switch_ped_roads_off_in_area(paths: GPtr<u8>, x1: u32, x2: u32, y1: u32, y2: u32, z1: u32, z2: u32, off: u32, b: u32) = thiscall 0x452F00 => unported;
    /// `CPhysical::GetHasCollidedWith(CEntity*)` (0x543540), AL
    fn get_has_collided_with(p: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x543540 => unported;
    /// `CStreaming::RequestSpecialChar(int32 slot, const char*, int32 flags)` (0x40B450)
    fn request_special_char(slot: u32, name: GPtr<u8>, flags: u32) = cdecl 0x40B450 => unported;
    /// `CStreaming::HasSpecialCharLoaded(int32 slot)` (0x407F00), AL
    fn has_special_char_loaded(slot: u32) -> u8 = cdecl 0x407F00 => unported;
    /// `CAnimManager::GetAnimGroupName(AssocGroupId)` (0x4D3A20)
    fn get_anim_group_name(i: u32) -> GPtr<u8> = cdecl 0x4D3A20 => unported;
    /// `CCoronas::RegisterCorona(uint32 id, CEntity*, uint8 r, uint8 g, uint8 b, uint8 a, const CVector&,
    /// float radius, float farClip, uint8 type, uint8 flare, bool reflection, bool obstacles, int32,
    /// float angle, bool longDistance, float nearClip, uint8 fade, float fadeSpeed, bool onlyFromBelow,
    /// bool reflectionDelay)` (0x6FC580); floats as raw dwords
    fn register_corona(id: u32, attach: u32, r: u32, g: u32, b: u32, a: u32, pos: GPtr<u32>, radius: u32, far_clip: u32, ty: u32, flare: u32, refl: u32, obst: u32, unused: u32, angle: u32, long_dist: u32, near_clip: u32, fade: u32, fade_speed: u32, below: u32, refl_delay: u32) = cdecl 0x6FC580 => unported;
    /// `CWorld::StopAllLawEnforcersInTheirTracks()` (0x566C10)
    fn stop_all_law_enforcers_in_their_tracks() = cdecl 0x566C10 => unported;
}

/// `TheText` 0xC1B340.
const THE_TEXT: usize = 0xC1B340;
/// `ThePaths` 0x96F050.
const THE_PATHS: usize = 0x96F050;
/// `CAnimManager::ms_numAnimAssocDefinitions` (int) 0xB4EA28.
const NUM_ANIM_ASSOC_DEFINITIONS: usize = 0xB4EA28;
/// `.rdata` 0x858C24: 0.3f
const K_0_3: u32 = 0x3E99_999A;
/// `.rdata` 0x859A90: -3000.0f
const K_MINUS_3000: u32 = 0xC53B_8000;
/// `.rdata` 0x859F1C: 12001.0f
const K_12001: u32 = 0x463B_8400;
/// `.rdata` 0x858C54: 4294967296.0f (the unsigned correction after `fild`)
const K_2_POW_32: u32 = 0x4F80_0000;
/// "DUMMY" (`.rdata` 0x859EE0, START_KILL_FRENZY's no-text key), NUL-terminated.
const DUMMY: &[u8; 6] = b"DUMMY\0";

/// An 8-byte text label read with `ReadTextLabelFromScript` into a zeroed 16-byte buffer (divergence
/// note: the original's stack buffer is not cleared, so an 8-character label is followed by stale stack
/// bytes when a callee reads it as a C string).
#[inline]
fn label8(s: Scr, buf: &mut [u8; 16]) -> GPtr<u8> {
    let b = GPtr::from_raw(buf.as_mut_ptr());
    read_text_label(s, b, 8);
    b
}

/// The `m_matrix` (+0x14) up-vector z (+0x28) of the vehicle `SP0` names (no null checks), as the
/// IS_CAR_UPSIDEDOWN / IS_CAR_UPRIGHT cases copy it.
#[inline]
fn car_up_z() -> f32 {
    let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
    let m = veh.byte_add(0x14).cast::<GPtr<u8>>().read();
    f(m.byte_add(0x28).cast::<u32>().read())
}

/// SWITCH_PED_ROADS_ON/OFF (022A 0x47EAE0 / 022B 0x47EBD1): `CollectParameters(6)`; each pair
/// (P0,P3), (P1,P4), (P2,P5) is ordered by `fld a; fcomp b; test ah,0x41; jne` (swap only when `a > b`,
/// ordered; the swapped value goes through `fld; fstp`, exact since it compared greater); then
/// `ThePaths.SwitchPedRoadsOffInArea(xmin, xmax, ymin, ymax, zmin, zmax, off, false)`.
fn switch_ped_roads(s: Scr, off: u32) {
    collect(s, 6);
    let pair = |a: u32, b: u32| {
        if f(a) > f(b) { (b, a) } else { (a, b) }
    };
    let (x1, x2) = pair(sp(0), sp(3));
    let (y1, y2) = pair(sp(1), sp(4));
    let (z1, z2) = pair(sp(2), sp(5));
    switch_ped_roads_off_in_area(GPtr::from_addr(THE_PATHS), x1, x2, y1, y2, z1, z2, off, 0);
}

/// A model operand as these cases decode it: `test v,v; jge keep; imul v,0x1C; mov r,0xA44B88; sub r,v;
/// mov v,[r]` (a negative value indexes `CTheScripts::UsedObjectArray` backwards).
#[inline]
fn model_operand(v: u32) -> u32 {
    if (v as i32) < 0 {
        GPtr::<u32>::from_addr(USED_OBJECT_MODEL.wrapping_sub(v.wrapping_mul(0x1C)) as usize).read()
    } else {
        v
    }
}

/// `CRunningScript::ProcessCommands500To599(int32 command)` (0x47E090, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x47E090, conv = thiscall, name = "CRunningScript::ProcessCommands500To599", expect = "64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_500_to_599(s: Scr, command: u32) -> i8 {
    match command {
        // 01F4 IS_CAR_UPSIDEDOWN (0x47E0D1): no null checks; flag = `m_matrix (+0x14)->up.z (+0x28) <= 0.3f`
        // (`fcomp [0x858C24]; test ah,0x41; jnp` = less or equal, ordered). gr: unregistered.
        0x01F4 => {
            collect(s, 1);
            let z = car_up_z();
            set_flag(s, z <= f(K_0_3));
        }
        // 01F5 GET_PLAYER_CHAR (0x47E150): P0 = PedPool.GetRef(CWorld::Players[P0].m_pPed).
        0x01F5 => {
            collect(s, 1);
            let ped = player_info(sp(0)).cast::<GPtr<u8>>().read();
            let h = ped_get_ref(pool(PED_POOL), ped);
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 01F6 CANCEL_OVERRIDE_RESTART (0x47E19B)
        0x01F6 => cancel_override_restart(),
        // 01F7 SET_POLICE_IGNORE_PLAYER (0x47E1B6): `wanted = m_pPlayerData (+0x480) ? m_pPlayerData->m_Wanted
        // : nullptr` (the null path still writes `[0x1E]`, as the exe); P1 != 0: `m_bPoliceBackOff` (+0x1E
        // bit 0) set, then StopAllLawEnforcersInTheirTracks(); else cleared.
        0x01F7 => {
            collect(s, 2);
            let ped = player_info(sp(0)).cast::<GPtr<u8>>().read();
            let pd = ped.byte_add(0x480).cast::<GPtr<u8>>().read();
            let wanted = if pd.is_null() { GPtr::NULL } else { pd.cast::<GPtr<u8>>().read() };
            let b = wanted.byte_add(0x1E);
            if sp(1) != 0 {
                b.write(b.read() | 1);
                stop_all_law_enforcers_in_their_tracks();
            } else {
                b.write(b.read() & 0xFE);
            }
        }
        // 01F9 START_KILL_FRENZY (0x47E263): label (8) read first; text = `_stricmp(label, "DUMMY") ?
        // TheText.Get(label) : nullptr`; then `CollectParameters(8)`; StartFrenzy(P0, P1, (u16)P2, P3, text,
        // P4, P5, P6, P7 != 0, false). gr: unregistered. Divergence notes: zeroed label buffer; the `setne
        // al` bool is pushed with stale upper bytes, passed as 0/1.
        0x01F9 => {
            let mut buf = [0u8; 16];
            let b = label8(s, &mut buf);
            let text = if stricmp(b, GPtr::from_raw(DUMMY.as_ptr().cast_mut())) == 0 {
                GPtr::NULL
            } else {
                text_get(GPtr::from_addr(THE_TEXT), b)
            };
            collect(s, 8);
            start_frenzy(sp(0), sp(1), sp(2) & 0xFFFF, sp(3), text, sp(4), sp(5), sp(6), u32::from(sp(7) != 0), 0);
        }
        // 01FA READ_KILL_FRENZY_STATUS (0x47E304): P0 = (u16)CDarkel::ReadStatus() (`movzx`).
        0x01FA => {
            let st = darkel_read_status();
            set_sp(0, u32::from(st));
            store(s, 1);
        }
        // 01FB SQRT (0x47E331): `fld P0; fsqrt; fstp P0` (PC=24).
        0x01FB => {
            collect(s, 1);
            set_sp(0, X87::from_f32(spf(0)).fsqrt().to_f32().to_bits());
            store(s, 1);
        }
        // 0202..0207 LOCATE_CHAR_*_CAR_2D/3D (0x47E367): LocateCharCarCommand(command) (ECX holds the
        // command from the prologue).
        0x0202..=0x0207 => locate_char_car_command(s, command),
        // 0208 GENERATE_RANDOM_FLOAT_IN_RANGE (0x47E385): P0 = GetRandomNumberInRange(P0, P1) (`fstp`).
        0x0208 => {
            collect(s, 2);
            let r = random_float_in_range(sp(0), sp(1));
            set_sp(0, X87::from_f64_exact(r).to_f32().to_bits());
            store(s, 1);
        }
        // 0209 GENERATE_RANDOM_INT_IN_RANGE (0x47E3C8)
        0x0209 => {
            collect(s, 2);
            let r = random_int_in_range(spi(0), spi(1));
            set_sp(0, r as u32);
            store(s, 1);
        }
        // 020A LOCK_CAR_DOORS (0x47E40A): null -> nothing; `m_eDoorLock` (+0x4F8) = P1.
        0x020A => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            if !veh.is_null() {
                veh.byte_add(0x4F8).cast::<u32>().write(sp(1));
            }
        }
        // 020B EXPLODE_CAR (0x47E44F): no null check; virtual BlowUpCar(nullptr, false).
        0x020B => {
            collect(s, 1);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            blow_up_car(veh, GPtr::NULL, 0);
        }
        // 020C ADD_EXPLOSION (0x47E48D): AddExplosion(nullptr, nullptr, P3, (P0, P1, P2), 0, 1, -1.0f, 0).
        // gr: unregistered.
        0x020C => {
            collect(s, 4);
            add_explosion(0, 0, sp(3), sp(0), sp(1), sp(2), 0, 1, 0xBF80_0000, 0);
        }
        // 020D IS_CAR_UPRIGHT (0x47E500): no null checks; flag = `m_matrix->up.z >= 0.0f` (`fcomp
        // [0x858B50]; test ah,1; je`, ordered). gr: unregistered.
        0x020D => {
            collect(s, 1);
            let z = car_up_z();
            set_flag(s, z >= 0.0);
        }
        // 0213 CREATE_PICKUP (0x47E57A): P0 model (negative: UsedObjectArray), P1 type, P2..P4 x, y,
        // z. `fld z; fst local; fcomp -100.0; test ah,0x41; jp`: when `z <= -100` (ordered) local z =
        // FindGroundZForCoord(x, y) `fadd [0.5]`. Discarded GetActualPickupIndex of the output operand;
        // P0 = GenerateNewOne((x, y, z), model, (u8)P1, 0, 0, 0, nullptr); StoreParameters(1).
        0x0213 => {
            collect(s, 5);
            let model = model_operand(sp(0));
            let (x, y) = (sp(2), sp(3));
            let st = X87::from_f32(f(sp(4)));
            let mut z = st.to_f32().to_bits();
            if !st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
                z = X87::from_f64_exact(find_ground_z_for_coord(x, y))
                    .fadd_m(f(K_HALF))
                    .to_f32()
                    .to_bits();
            }
            get_actual_pickup_index(collect_next(s));
            let h = generate_new_one(x, y, z, model, sp(1) & 0xFF, 0, 0, 0, 0);
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 0214 HAS_PICKUP_BEEN_COLLECTED (0x47E662): flag = IsPickUpPickedUp(P0) != 0 (`setne` into the
        // dead command-argument slot, then pushed).
        0x0214 => {
            collect(s, 1);
            let b = is_pick_up_picked_up(spi(0)) != 0;
            set_flag(s, b);
        }
        // 0215 REMOVE_PICKUP (0x47E6AB)
        0x0215 => {
            collect(s, 1);
            remove_pick_up(spi(0));
        }
        // 0216 SET_TAXI_LIGHTS (0x47E6D9): no null check; CAutomobile::SetTaxiLight(P1 != 0). gr: unregistered.
        0x0216 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_taxi_light(veh, u32::from(sp(1) != 0));
        }
        // 0217 PRINT_BIG_Q (0x47E73A): label (8) first, `TheText.Get(label)`, then `CollectParameters(2)`;
        // AddBigMessageQ(text, P0, P1 - 1). Divergence note: zeroed label buffer.
        0x0217 => {
            let mut buf = [0u8; 16];
            let b = label8(s, &mut buf);
            let text = text_get(GPtr::from_addr(THE_TEXT), b);
            collect(s, 2);
            add_big_message_q(text, sp(0), sp(1).wrapping_sub(1));
        }
        // 021B SET_TARGET_CAR_FOR_MISSION_GARAGE (0x47E78F): label (8); `g = GetGarageNumberByName(label)`;
        // `CollectParameters(1)`; nothing when `(int16)g < 0`; else SetTargetCarForMissionGarage(g, P0 < 0 ?
        // nullptr : VehiclePool.GetAtRef(P0)) (EAX pushed whole). gr: unregistered. Divergence note:
        // zeroed label buffer.
        0x021B => {
            let mut buf = [0u8; 16];
            let b = label8(s, &mut buf);
            let g = get_garage_number_by_name(b);
            collect(s, 1);
            if (g as i16) >= 0 {
                let veh = if spi(0) < 0 {
                    GPtr::NULL
                } else {
                    vehicle_at_ref(pool(VEHICLE_POOL), spi(0))
                };
                set_target_car_for_mission_garage(g, veh);
            }
        }
        // 021C IS_CAR_IN_MISSION_GARAGE / 0231 SET_SCRIPT_POLICE_HELI_TO_CHASE_CHAR / 0250 DRAW_LIGHT
        // (0x47F21F): the jump table goes straight to the epilogue: 0 without reading the operands (gr: 0250
        // is a handler that reads them).
        0x021C | 0x0231 | 0x0250 => {}
        // 0221 APPLY_BRAKES_TO_PLAYERS_CAR (0x47E814): `GetPad(P0)->bApplyBrakes` (+0x118) = P1 != 0.
        0x0221 => {
            collect(s, 2);
            let on = sp(1) != 0;
            get_pad(sp(0)).byte_add(0x118).write(u8::from(on));
        }
        // 0223 SET_CHAR_HEALTH (0x47E87D): ped not null-checked. P1 == 0 (integer test): `new
        // CTaskComplexDie(0, 0, 0xF, 4.0f, 0, false, false, 0, false)` (null when new fails), a stack
        // `CEventScriptCommand(3, task, false)` added to `m_pIntelligence (+0x47C)->m_eventGroup (+0x68)`
        // (`Add(&ev, false)`), then its destructor 0x4B0A50. P1 != 0: a player gets `fild min(P1,
        // Players[FindPlayerSlotWithPedPointer(ped)].m_nMaxHealth (+0x14F, u8))` (signed int compare); any
        // other ped `m_fHealth (+0x540) = fild P1`, and `m_fMaxHealth (+0x544) = fild P1` when its bits
        // are exactly 100.0f (`cmp dword, 0x42C80000`). gr: a float operand compared with 0.0f, float min.
        0x0223 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            if spi(1) == 0 {
                let mem = task_new(0x28);
                let task = if mem.is_null() {
                    GPtr::NULL
                } else {
                    task_complex_die_ctor(mem, 0, 0, 0xF, 0x4080_0000, 0, 0, 0, 0, 0)
                };
                let mut ev = [0u32; 6];
                let e = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
                event_script_command_ctor(e, 3, task, 0);
                let intel = ped.byte_add(0x47C).cast::<GPtr<u8>>().read();
                event_group_add(intel.byte_add(0x68), e, 0);
                event_script_command_dtor(e);
            } else if ped_is_player(ped) != 0 {
                let slot = find_player_slot_with_ped_pointer(ped);
                let max = i32::from(player_info(slot).byte_add(0x14F).read());
                let h = if max < spi(1) { max } else { spi(1) };
                ped.byte_add(0x540).cast::<u32>().write(X87::from_i32(h).to_f32().to_bits());
            } else {
                let h = X87::from_i32(spi(1)).to_f32().to_bits();
                ped.byte_add(0x540).cast::<u32>().write(h);
                let mx = ped.byte_add(0x544).cast::<u32>();
                if mx.read() == 0x42C8_0000 {
                    mx.write(h);
                }
            }
        }
        // 0224 SET_CAR_HEALTH (0x47E9E0): `fild P1; fstp local` (int -> f32, round to nearest) before
        // the lookup; no null check; `m_fHealth` (+0x4C0) = local.
        0x0224 => {
            collect(s, 2);
            let h = X87::from_i32(spi(1)).to_f32().to_bits();
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            veh.byte_add(0x4C0).cast::<u32>().write(h);
        }
        // 0226 GET_CHAR_HEALTH (0x47EA2B) / 0227 GET_CAR_HEALTH (0x47EA74, shared tail 0x47EA4B): no null
        // check; P0 = `_ftol2(health)` (ped +0x540, vehicle +0x4C0; EAX = the low dword).
        0x0226 | 0x0227 => {
            collect(s, 1);
            let (e, off) = if command == 0x0226 {
                (ped_at_ref(pool(PED_POOL), spi(0)), 0x540)
            } else {
                (vehicle_at_ref(pool(VEHICLE_POOL), spi(0)), 0x4C0)
            };
            let health = f(e.byte_add(off).cast::<u32>().read());
            set_sp(0, X87::from_f32(health).ftol() as u32);
            store(s, 1);
        }
        // 0229 CHANGE_CAR_COLOUR (0x47EA97): no null check; primary/secondary colour bytes +0x434/+0x435 =
        // (u8)P1, (u8)P2.
        0x0229 => {
            collect(s, 3);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            veh.byte_add(0x434).write(sp(1) as u8);
            veh.byte_add(0x435).write(sp(2) as u8);
        }
        // 022A SWITCH_PED_ROADS_ON (0x47EAE0) / 022B SWITCH_PED_ROADS_OFF (0x47EBD1)
        0x022A => switch_ped_roads(s, 0),
        0x022B => switch_ped_roads(s, 1),
        // 0235 SET_GANG_PED_MODELS (0x47ECC2) / 0236 SET_GANG_CAR_MODEL (0x47ECE1): operands read only.
        0x0235 => collect(s, 3),
        0x0236 => collect(s, 2),
        // 0237 SET_GANG_WEAPONS (0x47ED00): CGangs::SetGangWeapons((u16)P0, P1, P2, P3).
        0x0237 => {
            collect(s, 4);
            set_gang_weapons(sp(0) & 0xFFFF, spi(1), spi(2), spi(3));
        }
        // 023B IS_CHAR_TOUCHING_OBJECT_ON_FOOT (0x47ED44): both lookups first (no null checks); flag = 0
        // when "in vehicle" (+0x46C bit 8 and `m_pVehicle` +0x58C != 0, tested inline), else
        // `ped->GetHasCollidedWith(obj) != 0`.
        0x023B => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let obj = object_at_ref(pool(OBJECT_POOL), spi(1));
            let in_car = ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0
                && ped.byte_add(0x58C).cast::<u32>().read() != 0;
            let b = !in_car && get_has_collided_with(ped, obj) != 0;
            set_flag(s, b);
        }
        // 023C LOAD_SPECIAL_CHARACTER (0x47EDA9): `CollectParameters(1)`; `SP0 -= 1` (written back); label
        // (8), its 8 bytes lower-cased in place ('A'..='Z' + 0x20); RequestSpecialChar(SP0, label, 0xC);
        // ScriptResourceManager.AddToResourceManager(SP0 + 290, 2, this). gr: unregistered. Divergence
        // note: zeroed label buffer.
        0x023C => {
            collect(s, 1);
            set_sp(0, sp(0).wrapping_sub(1));
            let mut buf = [0u8; 16];
            let b = label8(s, &mut buf);
            for i in 0..8 {
                let p = b.byte_add(i);
                let c = p.read();
                if c.is_ascii_uppercase() {
                    p.write(c + 0x20);
                }
            }
            request_special_char(sp(0), b, 0xC);
            add_to_resource_manager(
                GPtr::from_addr(SCRIPT_RESOURCE_MANAGER),
                sp(0).wrapping_add(0x122) as i32,
                2,
                s,
            );
        }
        // 023D HAS_SPECIAL_CHARACTER_LOADED (0x47EE2D): `SP0 -= 1` (written back); flag =
        // HasSpecialCharLoaded(SP0) != 0. gr: unregistered.
        0x023D => {
            collect(s, 1);
            set_sp(0, sp(0).wrapping_sub(1));
            let b = has_special_char_loaded(sp(0)) != 0;
            set_flag(s, b);
        }
        // 0241 IS_PLAYER_IN_REMOTE_MODE (0x47EE83): flag = CWorld::Players[P0].IsPlayerInRemoteMode() != 0.
        0x0241 => {
            collect(s, 1);
            let b = is_player_in_remote_mode(player_info(sp(0))) != 0;
            set_flag(s, b);
        }
        // 0244 SET_CUTSCENE_OFFSET (0x47EEB8): 0x47E070(&local (P0, P1, P2)).
        0x0244 => {
            collect(s, 3);
            let v = [sp(0), sp(1), sp(2)];
            set_cutscene_offset(GPtr::from_raw(v.as_ptr().cast_mut()));
        }
        // 0245 SET_ANIM_GROUP_FOR_CHAR (0x47EF01): ped looked up first (no null check); label (16); `i` (u16)
        // runs from 0 while `i < ms_numAnimAssocDefinitions` (signed) until `_stricmp(label,
        // GetAnimGroupName(i)) == 0` (found: `i` kept); `m_nAnimGroup` (+0x4D4) = i (a miss stores the
        // count). gr: commented out. Divergence note: zeroed 17-byte buffer (a 16-character label has its
        // terminator in stale stack bytes in the original).
        0x0245 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let mut buf = [0u8; 17];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 16);
            let n = GPtr::<i32>::from_addr(NUM_ANIM_ASSOC_DEFINITIONS);
            let mut i: u16 = 0;
            if n.read() > 0 {
                loop {
                    if stricmp(b, get_anim_group_name(u32::from(i))) == 0 {
                        break;
                    }
                    i = i.wrapping_add(1);
                    if i32::from(i) >= n.read() {
                        break;
                    }
                }
            }
            ped.byte_add(0x4D4).cast::<u32>().write(u32::from(i));
        }
        // 0247 REQUEST_MODEL (0x47EF90): RequestModel(m, 0xC) (KEEP_IN_MEMORY | MISSION_REQUIRED);
        // ScriptResourceManager.AddToResourceManager(m, 2, this).
        0x0247 => {
            collect(s, 1);
            let m = model_operand(sp(0)) as i32;
            request_model(m, 0xC);
            add_to_resource_manager(GPtr::from_addr(SCRIPT_RESOURCE_MANAGER), m, 2, s);
        }
        // 0248 HAS_MODEL_LOADED (0x47EFDE): flag = ms_aInfoForModel[m].m_LoadState == 1 (LOADED).
        0x0248 => {
            collect(s, 1);
            let m = model_operand(sp(0));
            let st =
                GPtr::<u8>::from_addr(MODEL0_LOAD_STATE.wrapping_add(m.wrapping_mul(20)) as usize).read();
            set_flag(s, st == 1);
        }
        // 0249 MARK_MODEL_AS_NO_LONGER_NEEDED (0x47F03E): SetMissionDoesntRequireModel(m) only when
        // RemoveFromResourceManager(m, 2, this) returns true.
        0x0249 => {
            collect(s, 1);
            let m = model_operand(sp(0)) as i32;
            if remove_from_resource_manager(GPtr::from_addr(SCRIPT_RESOURCE_MANAGER), m, 2, s) != 0 {
                set_mission_doesnt_require_model(m);
            }
        }
        // 024F DRAW_CORONA (0x47F08E): `CollectParameters(9)`; z = P2, or FindGroundZForCoord(P0, P1) when
        // `z <= -100` (ordered; `fld; fcom; jp`, the kept value goes through `fstp`); id =
        // `(uint32)_ftol((y - -3000) * 12001) + (x - -3000)` then `_ftol` again (x87 PC=24: `fsub; fmul`,
        // `_ftol2` 0x821B40 as `X87::ftol` low dword, a negative first result `fild`ed and corrected by
        // `fadd 2^32`, then `faddp`), `+ m_pCurrentIP (+0x14) + this`; RegisterCorona(id, nullptr, (u8)P6,
        // (u8)P7, (u8)P8, 255, &(x, y, z), P3, 450.0f, (u8)P4, (u8)P5, true, false, 0, 0.0f, false, 1.5f,
        // 0, 15.0f, false, false). gr: unregistered.
        0x024F => {
            collect(s, 9);
            let (x, y) = (sp(0), sp(1));
            let z = if spf(2) <= f(K_MINUS_100) {
                X87::from_f64_exact(find_ground_z_for_coord(x, y)).to_f32().to_bits()
            } else {
                fld_fst(spf(2)).to_bits()
            };
            let a = X87::from_f32(f(y)).fsub_m(f(K_MINUS_3000)).fmul_m(f(K_12001)).ftol() as u32;
            let mut st1 = X87::from_i32(a as i32);
            if (a as i32) < 0 {
                st1 = st1.fadd_m(f(K_2_POW_32));
            }
            let st0 = X87::from_f32(f(x)).fsub_m(f(K_MINUS_3000));
            let b = st1.fadd(st0).ftol() as u32;
            let ip = s.byte_add(crate::vm::IP).cast::<u32>().read();
            let id = b.wrapping_add(ip).wrapping_add(s.addr() as u32);
            let pos = [x, y, z];
            register_corona(
                id,
                0,
                sp(6) & 0xFF,
                sp(7) & 0xFF,
                sp(8) & 0xFF,
                0xFF,
                GPtr::from_raw(pos.as_ptr().cast_mut()),
                sp(3),
                0x43E1_0000,
                sp(4) & 0xFF,
                sp(5) & 0xFF,
                1,
                0,
                0,
                0,
                0,
                0x3FC0_0000,
                0,
                0x4170_0000,
                0,
                0,
            );
        }
        // 0253 STORE_CLOCK (0x47F1AC) / 0254 RESTORE_CLOCK (0x47F1C7)
        0x0253 => store_clock(),
        0x0254 => restore_clock(),
        // 0256 IS_PLAYER_PLAYING (0x47F1E2): flag = CWorld::Players[P0].m_nPlayerState (+0xDC) == 0.
        0x0256 => {
            collect(s, 1);
            let st = player_info(sp(0)).byte_add(0xDC).read();
            set_flag(s, st == 0);
        }
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_500_TO_599];
