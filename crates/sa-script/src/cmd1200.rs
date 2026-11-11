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
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x04B0, 0x04B1, 0x04B2, 0x04B3, 0x04B4, 0x04B5, 0x04B6, 0x04B7, 0x04B8, 0x04B9, 0x04BA, 0x04BB, 0x04BC, 0x04BD,
    0x04BE, 0x04BF, 0x04C0, 0x04C1, 0x04C4, 0x04C5, 0x04C7, 0x04C8, 0x04CE, 0x04D0, 0x04D1, 0x04D2, 0x04D3,
    0x04D5, 0x04D6, 0x04D7, 0x04D8, 0x04D9, 0x04DA, 0x04DB, 0x04DD, 0x04DF, 0x04E0, 0x04E1, 0x04E2, 0x04E3,
    0x04E4, 0x04E5, 0x04E6, 0x04E7, 0x04E9, 0x04EA, 0x04EB, 0x04ED, 0x04EE, 0x04EF, 0x04F0, 0x04F1, 0x04F4,
    0x04F5, 0x04F7, 0x04F8, 0x04F9, 0x04FA, 0x04FC, 0x04FE, 0x0500, 0x0501, 0x0506, 0x0507, 0x0508, 0x0509,
    0x050A, 0x050B, 0x050E, 0x050F, 0x0512, 0x0513,
];

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
    /// `CScriptResourceManager::RemoveFromResourceManager(int32, uint32 type, CRunningScript*)` (0x470510), AL
    fn remove_from_resource_manager(m: GPtr<u8>, id: i32, ty: u32, s: Scr) -> u8 = thiscall 0x470510 => unported;
    /// Unnamed cdecl taking the animation block index (0x48B570; REMOVE_ANIMATION's removal)
    fn remove_anim_48b570(idx: i32) = cdecl 0x48B570 => unported;
    /// `CPickups::ModelForWeapon(eWeaponType)` (0x454AC0)
    fn model_for_weapon(ty: u32) -> i32 = cdecl 0x454AC0 => unported;
    /// `CRoadBlocks::RegisterScriptRoadBlock(CVector, CVector, bool)` (0x460DF0), vectors as raw dwords
    fn register_script_road_block(a: u32, b: u32, c: u32, d: u32, e: u32, g: u32, flag: u32) = cdecl 0x460DF0 => unported;
    /// Unnamed `CVector operator*(const CVector&, float)` (0x40FEC0; hidden-pointer result), the float raw
    fn vector_mul_float(out: GPtr<u32>, v: GPtr<u8>, k: u32) -> GPtr<u32> = cdecl 0x40FEC0 => unported;
    /// `CPed::AttachPedToEntity(CEntity*, CVector offset, uint16 position, float angleLimit, eWeaponType)`
    /// (0x5E7CB0); the vector and float as raw dwords
    fn attach_ped_to_entity(p: GPtr<u8>, e: GPtr<u8>, x: u32, y: u32, z: u32, pos: u32, angle: u32, weapon: u32) = thiscall 0x5E7CB0 => unported;
    /// `CAutomobile::CloseAllDoors()` (0x6A4520)
    fn close_all_doors(v: GPtr<u8>) = thiscall 0x6A4520 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0 (an FSQRT result under PC=24: exact as f64)
    fn vector_magnitude(v: GPtr<u32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CText::Get(const char* key)` (0x6A0050)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CHud::SetHelpMessage(const char*, bool quick, bool permanent, bool addToBrief)` (0x588BE0)
    fn set_help_message(t: GPtr<u8>, a: u32, b: u32, c: u32) = cdecl 0x588BE0 => unported;
    /// `CMessages::AddToPreviousBriefArray(const char*, int32 n1..n6, const char*)` (0x69DD50)
    fn add_to_previous_brief_array(t: GPtr<u8>, n1: u32, n2: u32, n3: u32, n4: u32, n5: u32, n6: u32, s: u32) = cdecl 0x69DD50 => unported;
    /// `CHud::SetHelpMessageWithNumber(const char*, int32, bool, bool)` (0x588E30)
    fn set_help_message_with_number(t: GPtr<u8>, n: u32, a: u32, b: u32) = cdecl 0x588E30 => unported;
    /// `COnscreenTimer::AddCounter(uint32 varId, uint16 type, const char* gxt, uint16 index)` (0x44CDA0)
    fn add_counter(t: GPtr<u8>, var: u32, ty: u32, gxt: GPtr<u8>, index: u32) = thiscall 0x44CDA0 => unported;
    /// `CRunningScript::GetIndexOfGlobalVariable()` (0x464700), AX
    fn global_index(s: Scr) -> u16 = thiscall 0x464700 => crate::vm::get_index_of_global_variable;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
    /// `CTaskSimpleDuckToggle::CTaskSimpleDuckToggle(int32)` (0x62F690)
    fn duck_toggle_ctor(t: GPtr<u8>, on: u32) -> GPtr<u8> = thiscall 0x62F690 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 ped, CTask*, int32 opcode)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: i32, task: GPtr<u8>, op: u32) = thiscall 0x465C20 => unported;
    /// `CSetPieces::AddOne(uint8 type, CVector2D, CVector2D, CVector2D, CVector2D, CVector2D, CVector2D)`
    /// (0x499500); the 2D vectors as twelve raw dwords
    fn set_pieces_add_one(ty: u32, a: u32, b: u32, c: u32, d: u32, e: u32, g: u32, h: u32, i: u32, j: u32, k: u32, l: u32, m: u32) = cdecl 0x499500 => unported;
    /// `CVehicle` virtual slot +0xB0 (`BurstTyre(uint8 tyre, bool applyEffects)`)
    fn vehicle_burst_tyre(v: GPtr<u8>, tyre: u32, fx: u32) = thiscall vslot 0xB0 => unported;
    /// `Multiply3x3(const CMatrix&, const CVector&)` (0x59C790; hidden-pointer result)
    fn multiply_3x3(out: GPtr<u32>, m: GPtr<u8>, v: GPtr<u32>) -> GPtr<u32> = cdecl 0x59C790 => unported;
    /// Unnamed `CVector::operator+=(const CVector&)` (0x411A00)
    fn vector_add_assign(v: GPtr<u32>, o: GPtr<u8>) = thiscall 0x411A00 => unported;
    /// `CAutomobile::SetHeliOrientation(float)` (0x6A2450), the float as its raw dword
    fn set_heli_orientation(v: GPtr<u8>, a: u32) = thiscall 0x6A2450 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), raw dwords, ST0 (exact as f64)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CPathFind::FindNthNodeClosestToCoors(CVector, uint8 type, float maxDist, bool, bool, int32 n,
    /// bool, bool, CNodeAddress*)` (0x44F8C0), the `CNodeAddress` through the hidden pointer (EAX)
    fn find_nth_node_closest_to_coors(paths: GPtr<u8>, out: GPtr<u32>, x: u32, y: u32, z: u32, ty: u32, max: u32, a: u32, b: u32, n: u32, c: u32, d: u32, e: u32) -> GPtr<u32> = thiscall 0x44F8C0 => unported;
    /// `CPathFind::FindNodePairClosestToCoors(CVector, uint8 type, CNodeAddress* n1, CNodeAddress* n2, float*
    /// orientation, float minDist, float maxDist, bool, bool, bool)` (0x44FEE0, `ret 0x30`), raw dwords
    fn find_node_pair_closest_to_coors(paths: GPtr<u8>, x: u32, y: u32, z: u32, ty: u32, n1: GPtr<u32>, n2: GPtr<u32>, orient: GPtr<u32>, min: u32, max: u32, a: u32, b: u32, c: u32) = thiscall 0x44FEE0 => unported;
    /// `CPathFind::FindNodeCoorsForScript(CNodeAddress, bool* found)` (0x4505E0), hidden-pointer vector
    fn find_node_coors_for_script(paths: GPtr<u8>, out: GPtr<u32>, addr: u32, found: GPtr<u8>) -> GPtr<u32> = thiscall 0x4505E0 => unported;
    /// `CCoronas::RegisterCorona(uint32 id, CEntity*, uint8 r, g, b, a, const CVector*, float radius,
    /// float farClip, uint8 type, uint8 flare, bool reflection, bool obstacles, int32, float angle, bool
    /// longDistance, float nearClip, uint8 fadeState, float fadeSpeed, bool onlyFromBelow, bool)`
    /// (0x6FC580); floats as raw dwords
    fn register_corona(id: u32, e: u32, r: u32, g: u32, b: u32, a: u32, pos: GPtr<u32>, radius: u32, far: u32, ty: u32, flare: u32, refl: u32, obst: u32, unused: u32, angle: u32, long: u32, near: u32, fade: u32, speed: u32, below: u32, delay: u32) = cdecl 0x6FC580 => unported;
    /// `CRoadBlocks::ClearScriptRoadBlocks()` (0x460EC0)
    fn clear_script_road_blocks() = cdecl 0x460EC0 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CAutomobile::ClearHeliOrientation()` (0x6A2460)
    fn clear_heli_orientation(v: GPtr<u8>) = thiscall 0x6A2460 => unported;
    /// `CAutomobile::TellPlaneToGoToCoors(float x, float y, float z, float, float)` (0x6A2470), raw dwords
    fn tell_plane_to_go_to_coors(v: GPtr<u8>, x: u32, y: u32, z: u32, a: u32, b: u32) = thiscall 0x6A2470 => unported;
    /// `CPlayerInfo::BlowUpRCBuggy(bool)` (0x56EA30)
    fn blow_up_rc_buggy(p: GPtr<u8>, explode: u32) = thiscall 0x56EA30 => unported;
    /// `CAutomobile::PopBoot()` (0x6AF910)
    fn pop_boot(v: GPtr<u8>) = thiscall 0x6AF910 => unported;
    /// `CAEPedSpeechAudioEntity::SetCJMood(int16, uint32, int16, int16, int16)` (0x4E3ED0)
    fn set_cj_mood(a: u32, b: u32, c: u32, d: u32, e: u32) = cdecl 0x4E3ED0 => unported;
    /// `CRunningScript::LocateObjectCommand(int32 command)` (0x487D10)
    fn locate_object_command(s: Scr, command: u32) = thiscall 0x487D10 => unported;
    /// `CRunningScript::ObjectInAreaCheckCommand(int32 command)` (0x489150)
    fn object_in_area_check_command(s: Scr, command: u32) = thiscall 0x489150 => unported;
    /// `CTimeCycle::StartExtraColour(int32, bool)` (0x55FEC0)
    fn start_extra_colour(c: u32, keep: u32) = cdecl 0x55FEC0 => unported;
    /// `CTimeCycle::StopExtraColour(bool)` (0x55FF20)
    fn stop_extra_colour(keep: u32) = cdecl 0x55FF20 => unported;
}

/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;
/// `.rdata` 0x8595EC: 0.017453292f
const K_DEG2RAD: u32 = 0x3C8E_FA35;
/// `.rdata` 0x85991C: 90.0f
const K_90: u32 = 0x42B4_0000;
/// `.rdata` 0x858CBC: 6.2831855f
const K_2PI: u32 = 0x40C9_0FDB;
/// 999999.0f (`push 0x497423FE`)
const K_999999: u32 = 0x4974_23FE;
/// `ThePaths` 0x96F050, `TheText` 0xC1B340, `CUserDisplay::OnscnTimer` 0xBA1788.
const THE_PATHS: usize = 0x96F050;
const THE_TEXT: usize = 0xC1B340;
const ONSCN_TIMER: usize = 0xBA1788;
/// `CWanted::MaximumWantedLevel` 0x8CDEE4.
const MAX_WANTED_LEVEL: usize = 0x8CDEE4;
/// `CTheScripts::bDisplayNonMiniGameHelpMessages` 0xA444A7, `bMiniGameInProgress` 0xA444A8,
/// `bAddNextMessageToPreviousBriefs` 0xA4448B.
const DISPLAY_NON_MINIGAME_HELP: usize = 0xA444A7;
const MINIGAME_IN_PROGRESS: usize = 0xA444A8;
const ADD_NEXT_MESSAGE_TO_PREVIOUS_BRIEFS: usize = 0xA4448B;
/// `CVehicleModelInfo::ms_compsToUse` (2 bytes) 0x8A6458.
const COMPS_TO_USE: usize = 0x8A6458;

/// The "-100 ground rule" (`fld z; fcom [-100.0]; test ah,0x41; jp keep`, else `fstp st(0)` and ST0 =
/// `FindGroundZForCoord(x, y)`; ST0 stored as f32).
fn ground_rule(x: u32, y: u32, z: u32) -> u32 {
    let st = X87::from_f32(f(z));
    let st = if st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        st
    } else {
        X87::from_f64_exact(find_ground_z_for_coord(x, y))
    };
    st.to_f32().to_bits()
}

/// `.rdata` 0x859044: 0.016666668f
const K_ONE_SIXTIETH: u32 = 0x3C88_8889;
/// `CWorld::PlayerInFocus` (byte) 0xB7CD74.
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
/// Byte 0xC1CC01 (SET_ENABLE_RC_DETONATE_ON_CONTACT's flag).
const RC_DETONATE_ON_CONTACT_DISABLED: usize = 0xC1CC01;

/// Sets or clears `mask` in the dword at `e + off` (`or` / `and ~mask`).
fn set_bits(e: GPtr<u8>, off: isize, mask: u32, on: bool) {
    let p = e.byte_add(off).cast::<u32>();
    let v = p.read();
    p.write(if on { v | mask } else { v & !mask });
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
        // 04B0..04B7 IS_INT_(L)VAR_GREATER_(OR_EQUAL_)THAN_CONSTANT / IS_CONSTANT_GREATER_(OR_EQUAL_)THAN_INT_
        // (L)VAR (0x48B5E4..0x48B72C): the `*_VAR_*_CONSTANT` forms read the variable pointer
        // (GetPointerToScriptVariable 2 global / 1 local) before CollectParameters(1), the
        // `CONSTANT_*` forms after it; flag = signed `>` / `>=` (`setg` / `setge`).
        0x04B0..=0x04B7 => {
            let ty = if command & 1 == 0 { 2 } else { 1 };
            let var_first = command & 2 == 0;
            let p = if var_first {
                let p = var_ptr(s, ty);
                collect(s, 1);
                p
            } else {
                collect(s, 1);
                var_ptr(s, ty)
            };
            let (v, c) = (p.read() as i32, spi(0));
            let (a, b) = if var_first { (v, c) } else { (c, v) };
            set_flag(s, if command < 0x04B4 { a > b } else { a >= b });
            0
        }
        // 04B8 GET_CHAR_WEAPON_IN_SLOT (0x48B75A): no null check; w = ped + P1 * 0x1C (P1 is 1-based):
        // P0 = type (+0x584), P1 = total ammo (+0x590), P2 = CPickups::ModelForWeapon(type);
        // StoreParameters(3).
        0x04B8 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let w = ped.byte_add(spi(1).wrapping_mul(0x1C) as isize);
            let ty = w.byte_add(0x584).cast::<u32>().read();
            set_sp(0, ty);
            set_sp(1, w.byte_add(0x590).cast::<u32>().read());
            set_sp(2, model_for_weapon(ty) as u32);
            store(s, 3);
            0
        }
        // 04BC, 04BE, 04BF, 04C7, 04F5, 0507, 050B (0x48CC39; gr: nop/unimplemented, 050B a handler): straight
        // to the epilogue, operands not read, return 0.
        0x04BC | 0x04BE | 0x04BF | 0x04C7 | 0x04F5 | 0x0507 | 0x050B => 0,
        // 04E2 SHUT_PLAYER_UP (0x48C206, gr: nop): CollectParameters(2), nothing else.
        0x04E2 => {
            collect(s, 2);
            0
        }
        // 04BD MARK_CAR_AS_CONVOY_CAR (0x48B9B0) / 04DF SET_HELI_STABILISER (0x48C165), gr: unregistered: no
        // null check; byte +0x42B bit 0x08 / 0x10 = P1 != 0.
        0x04BD | 0x04DF => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0)).byte_add(0x42B);
            let m = if command == 0x04BD { 0x08 } else { 0x10 };
            let b = v.read();
            v.write(if sp(1) != 0 { b | m } else { b & !m });
            0
        }
        // 04B9 GET_CLOSEST_STRAIGHT_ROAD (0x48B7B5, gr: unregistered): CollectParameters(5);
        // `ThePaths.FindNodePairClosestToCoors((P0,P1,P2), 0, &n1, &n2, &d, P3, P4, 1, 1, 0)`; n1.areaId (low
        // word) == 0xFFFF -> fail; else P0..2 = FindNodeCoorsForScript(n1, null), c2 = FindNodeCoorsForScript(n2,
        // &found); found -> P3..5 = c2, P6 = d, flag 1; fail -> P0..6 = 0, flag 0; StoreParameters(7),
        // UpdateCompareFlag. The prologue's `n1 = n2 = 0xFFFF` (low words) and the stale high words / `d` never
        // reach an output: 0x44FEE0 has two exits, the success one (0x4501AA..0x4502E0) writes n1, n2 and the
        // orientation as whole dwords, the failure one (0x450300) writes n1.low = n2.low = 0xFFFF and d = 0, and
        // the case reads n2 and d only after n1.low != 0xFFFF; 0x4505E0 writes `*found` on both of its paths.
        0x04B9 => {
            collect(s, 5);
            let paths = GPtr::<u8>::from_addr(THE_PATHS);
            let mut n1 = [0xFFFFu32; 1];
            let mut n2 = [0xFFFFu32; 1];
            let mut d = [0u32; 1];
            find_node_pair_closest_to_coors(
                paths,
                sp(0),
                sp(1),
                sp(2),
                0,
                GPtr::from_raw(n1.as_mut_ptr()),
                GPtr::from_raw(n2.as_mut_ptr()),
                GPtr::from_raw(d.as_mut_ptr()),
                sp(3),
                sp(4),
                1,
                1,
                0,
            );
            let mut ok = false;
            if n1[0] & 0xFFFF != 0xFFFF {
                let mut c1 = [0u32; 3];
                find_node_coors_for_script(paths, GPtr::from_raw(c1.as_mut_ptr()), n1[0], GPtr::NULL);
                set_sp(0, c1[0]);
                set_sp(1, c1[1]);
                set_sp(2, c1[2]);
                let mut c2 = [0u32; 3];
                let mut found = [0u8; 1];
                let r = find_node_coors_for_script(
                    paths,
                    GPtr::from_raw(c2.as_mut_ptr()),
                    n2[0],
                    GPtr::from_raw(found.as_mut_ptr()),
                );
                let c2 = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
                if found[0] != 0 {
                    set_sp(3, c2[0]);
                    set_sp(4, c2[1]);
                    set_sp(5, c2[2]);
                    set_sp(6, d[0]);
                    ok = true;
                }
            }
            if !ok {
                for i in 0..7 {
                    set_sp(i, 0);
                }
            }
            store(s, 7);
            set_flag(s, ok);
            0
        }
        // 04BA SET_CAR_FORWARD_SPEED (0x48B900, gr: unregistered): no null checks (`m_matrix` neither);
        // m_vecMoveSpeed (+0x44) = forward (`m_matrix + 0x10`) * (`P1 * 0.016666668`, x87, f32) through the
        // unnamed `CVector operator*(const CVector&, float)` 0x40FEC0; then, when the handling's model flags
        // (`m_pHandlingData` +0x384 -> +0xCC) have 0x2000000 and the vehicle is an automobile (+0x590 ==
        // 0), dword +0x84C = 0.22f.
        0x04BA => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let k = X87::from_f32(spf(1)).fmul_m(f(K_ONE_SIXTIETH)).to_f32().to_bits();
            let fwd = v.byte_add(0x14).cast::<GPtr<u8>>().read().byte_add(0x10);
            let mut out = [0u32; 3];
            let r = vector_mul_float(GPtr::from_raw(out.as_mut_ptr()), fwd, k);
            let ms = v.byte_add(0x44).cast::<u32>();
            for i in 0..3 {
                ms.wrapping_index(i).write(r.wrapping_index(i).read());
            }
            let h = v.byte_add(0x384).cast::<GPtr<u8>>().read();
            if h.byte_add(0xCC).cast::<u32>().read() & 0x200_0000 != 0 && v.byte_add(0x590).cast::<u32>().read() == 0 {
                v.byte_add(0x84C).cast::<u32>().write(0x3E61_47AE);
            }
            0
        }
        // 04C0 CREATE_SCRIPT_ROADBLOCK (0x48B9FB, gr: unregistered): CollectParameters(7);
        // CRoadBlocks::RegisterScriptRoadBlock((P0,P1,P2), (P3,P4,P5), P6 != 0).
        0x04C0 => {
            collect(s, 7);
            register_script_road_block(sp(0), sp(1), sp(2), sp(3), sp(4), sp(5), u32::from(sp(6) != 0));
            0
        }
        // 04C4 GET_OFFSET_FROM_CHAR_IN_WORLD_COORDS (0x48BA97): no null checks (`m_matrix` neither); v =
        // Multiply3x3(m_matrix, (P1, P2, P3)); `v += GetPosition()` (matrix pos when `m_matrix`, else the
        // placement, through the unnamed `CVector::operator+=` 0x411A00); P0..P2 = v; StoreParameters(3).
        0x04C4 => {
            collect(s, 4);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let off = [sp(1), sp(2), sp(3)];
            let mut out = [0u32; 3];
            let r = multiply_3x3(
                GPtr::from_raw(out.as_mut_ptr()),
                ped.byte_add(0x14).cast::<GPtr<u8>>().read(),
                GPtr::from_raw(off.as_ptr().cast_mut()),
            );
            let mut v = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
            let m = ped.byte_add(0x14).cast::<GPtr<u8>>().read();
            let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
            vector_add_assign(GPtr::from_raw(v.as_mut_ptr()), pos);
            set_sp(0, v[0]);
            set_sp(1, v[1]);
            set_sp(2, v[2]);
            store(s, 3);
            0
        }
        // 04D0 SET_HELI_ORIENTATION (0x48BCC6, gr: unregistered): no null check. In ST0: `(P1 + 90) *
        // 0.017453292`; while below 0 (ordered) `+= 6.2831855`; while above 6.2831855 (ordered) `-=` it;
        // SetHeliOrientation(the f32 of ST0; the original pushes its `fst` copy, the same value).
        0x04D0 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let mut st = X87::from_f32(spf(1)).fadd_m(f(K_90)).fmul_m(f(K_DEG2RAD));
            if !st.fcom(X87::zero()).test_parity(5) {
                loop {
                    st = st.fadd_m(f(K_2PI));
                    if st.fcom(X87::zero()).test_parity(5) {
                        break;
                    }
                }
            }
            if !st.fcom(X87::constant(K_2PI)).test_any(0x41) {
                loop {
                    st = st.fsub_m(f(K_2PI));
                    if st.fcom(X87::constant(K_2PI)).test_any(0x41) {
                        break;
                    }
                }
            }
            set_heli_orientation(v, st.to_f32().to_bits());
            0
        }
        // 04D3 GET_NTH_CLOSEST_CAR_NODE (0x48BDED, gr: unregistered): -100 ground rule on z; a =
        // ThePaths.FindNthNodeClosestToCoors((x,y,z), 0, 999999.0, 0, 1, P3 - 1, 0, 0, 0);
        // FindNodeCoorsForScript(a, &found): found -> P0..P2 = the vector, flag 1; else P0..P2 = 0, flag 0;
        // StoreParameters(3) before UpdateCompareFlag.
        0x04D3 => {
            collect(s, 4);
            let (x, y) = (sp(0), sp(1));
            let z = ground_rule(x, y, sp(2));
            let paths = GPtr::from_addr(THE_PATHS);
            let mut ret = [0u32; 1];
            let a = find_nth_node_closest_to_coors(
                paths,
                GPtr::from_raw(ret.as_mut_ptr()),
                x,
                y,
                z,
                0,
                K_999999,
                0,
                1,
                sp(3).wrapping_sub(1),
                0,
                0,
                0,
            )
            .read();
            let mut out = [0u32; 3];
            let mut found = [0u8; 1];
            find_node_coors_for_script(paths, GPtr::from_raw(out.as_mut_ptr()), a, GPtr::from_raw(found.as_mut_ptr()));
            let b = found[0] != 0;
            let v = if b { out } else { [0; 3] };
            set_sp(0, v[0]);
            set_sp(1, v[1]);
            set_sp(2, v[2]);
            store(s, 3);
            set_flag(s, b);
            0
        }
        // 04D5 DRAW_WEAPONSHOP_CORONA (0x48BEF2, gr: unregistered): CollectParameters(9); -100 ground rule on
        // z; CCoronas::RegisterCorona(this + m_IP, null, (u8)P6, (u8)P7, (u8)P8, 255, &(x,y,z), P3, 150.0,
        // (u8)P4, (u8)P5, 1, 0, 0, 0.0, 0, 0.2, 0, 15.0, 0, 0).
        0x04D5 => {
            collect(s, 9);
            let (x, y) = (sp(0), sp(1));
            let pos = [x, y, ground_rule(x, y, sp(2))];
            let id = s.byte_add(crate::vm::IP).cast::<u32>().read().wrapping_add(s.addr() as u32);
            register_corona(
                id,
                0,
                sp(6) & 0xFF,
                sp(7) & 0xFF,
                sp(8) & 0xFF,
                0xFF,
                GPtr::from_raw(pos.as_ptr().cast_mut()),
                sp(3),
                0x4316_0000,
                sp(4) & 0xFF,
                sp(5) & 0xFF,
                1,
                0,
                0,
                0,
                0,
                0x3E4C_CCCD,
                0,
                0x4170_0000,
                0,
                0,
            );
            0
        }
        // 04EB TASK_TOGGLE_DUCK (0x48C2F8, gr: unregistered): t = CTask::operator new(0xC) and, when not
        // null, CTaskSimpleDuckToggle(P1); GivePedScriptedTask(P0, t, command).
        0x04EB => {
            collect(s, 2);
            let t = crate::cmd0::task_new(0xC);
            let t = if t.is_null() { GPtr::NULL } else { duck_toggle_ctor(t, sp(1)) };
            give_ped_scripted_task(s, spi(0), t, command);
            0
        }
        // 04F4 ATTACH_CHAR_TO_OBJECT (0x48C49D): angle `P6 * 0.017453292` computed first; then the object
        // and the ped lookups (no null checks); AttachPedToEntity(obj, (P2, P3, P4) raw, (u16)P5, angle, P7).
        0x04F4 => {
            collect(s, 8);
            let a = X87::from_f32(spf(6)).fmul_m(f(K_DEG2RAD)).to_f32().to_bits();
            let obj = object_at_ref(pool(OBJECT_POOL), spi(1));
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            attach_ped_to_entity(ped, obj, sp(2), sp(3), sp(4), sp(5) & 0xFFFF, a, sp(7));
            0
        }
        // 04F7 DISPLAY_NTH_ONSCREEN_COUNTER_WITH_STRING (0x48C52F, gr: unregistered): idx =
        // GetIndexOfGlobalVariable(); CollectParameters(2); the 8-byte label (zeroed 16-byte buffer) ->
        // TheText.Get (result discarded); OnscnTimer.AddCounter((u16)idx, (u16)P0, label, P1 - 1).
        0x04F7 => {
            let idx = u32::from(global_index(s));
            collect(s, 2);
            let mut buf = [0u8; 16];
            let l = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, l, 8);
            text_get(GPtr::from_addr(THE_TEXT), l);
            add_counter(GPtr::from_addr(ONSCN_TIMER), idx, sp(0) & 0xFFFF, l, sp(1).wrapping_sub(1));
            0
        }
        // 04F8 ADD_SET_PIECE (0x48C589): CollectParameters(13); CSetPieces::AddOne((u8)P0, P1..P12).
        0x04F8 => {
            collect(s, 13);
            set_pieces_add_one(
                sp(0) & 0xFF,
                sp(1),
                sp(2),
                sp(3),
                sp(4),
                sp(5),
                sp(6),
                sp(7),
                sp(8),
                sp(9),
                sp(10),
                sp(11),
                sp(12),
            );
            0
        }
        // 04FE BURST_CAR_TYRE (0x48C768, gr: unregistered): no null check. Bike (+0x590 == 9): P1 2 -> 0 and
        // 3 -> 1 written back to ScriptParams[1], then vslot +0xB0 (BurstTyre)(P1, 1); others: (u8)P1.
        0x04FE => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            if v.byte_add(0x590).cast::<u32>().read() == 9 {
                let mut t = sp(1);
                if t == 2 || t == 3 {
                    t -= 2;
                    set_sp(1, t);
                }
                vehicle_burst_tyre(v, t, 1);
            } else {
                vehicle_burst_tyre(v, sp(1) & 0xFF, 1);
            }
            0
        }
        // 0500 IS_PLAYER_WEARING (0x48C7D7): k = Players[P0].m_PlayerData.m_pPedClothesDesc (+8)
        // `->m_anTextureKeys[P1]` (+0x28, read first); flag = k == CKeyGen::GetUppercaseKey(16-byte label).
        0x0500 => {
            collect(s, 2);
            let cd = player_info(sp(0)).byte_add(8).cast::<GPtr<u8>>().read();
            let k = cd.byte_add(0x28 + (sp(1).wrapping_mul(4) as i32) as isize).cast::<u32>().read();
            let buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_ptr().cast_mut());
            read_text_label(s, b, 16);
            let key = get_uppercase_key(b);
            set_flag(s, k == key);
            0
        }
        // 0506 SET_CAR_MODEL_COMPONENTS (0x48C9F4, gr: unregistered): CollectParameters(3);
        // CVehicleModelInfo::ms_compsToUse[0..1] (0x8A6458) = (u8)P1, (u8)P2.
        0x0506 => {
            collect(s, 3);
            GPtr::<u8>::from_addr(COMPS_TO_USE).write(sp(1) as u8);
            GPtr::<u8>::from_addr(COMPS_TO_USE + 1).write(sp(2) as u8);
            0
        }
        // 0508 CLOSE_ALL_CAR_DOORS (0x48CA1C, gr: unregistered): no null check; CAutomobile::CloseAllDoors().
        0x0508 => {
            collect(s, 1);
            close_all_doors(vehicle_at_ref(pool(VEHICLE_POOL), spi(0)));
            0
        }
        // 0509 GET_DISTANCE_BETWEEN_COORDS_2D (0x48CA44): dx = (x1 - x2) registers, stored f32; dy = y1 -
        // y2 (memory); P0 = fsqrt(dy*dy + dx*dx (memory)); StoreParameters(1).
        0x0509 => {
            collect(s, 4);
            let dx = X87::from_f32(spf(0)).fsub(X87::from_f32(spf(2))).to_f32();
            let dy = X87::from_f32(spf(1)).fsub_m(spf(3));
            let r = dy.fmul(dy).fadd(X87::from_f32(dx).fmul_m(dx)).fsqrt();
            set_sp(0, r.to_f32().to_bits());
            store(s, 1);
            0
        }
        // 050A GET_DISTANCE_BETWEEN_COORDS_3D (0x48CA89): (x1 - x2) registers, (y1 - y2), (z1 - z2) memory
        // operands, each stored f32; P0 = CVector::Magnitude of them; StoreParameters(1).
        0x050A => {
            collect(s, 6);
            let dx = X87::from_f32(spf(0)).fsub(X87::from_f32(spf(3))).to_f32().to_bits();
            let dy = X87::from_f32(spf(1)).fsub_m(spf(4)).to_f32().to_bits();
            let dz = X87::from_f32(spf(2)).fsub_m(spf(5)).to_f32().to_bits();
            let v = [dx, dy, dz];
            let m = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(v.as_ptr().cast_mut())));
            set_sp(0, m.to_f32().to_bits());
            store(s, 1);
            0
        }
        // 050E SORT_OUT_OBJECT_COLLISION_WITH_CAR (0x48CB16, gr: unregistered): vehicle P1 looked up first;
        // no null checks; object P0 +0x128 = the vehicle.
        0x050E => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            o.byte_add(0x128).cast::<GPtr<u8>>().write(v);
            0
        }
        // 050F GET_MAX_WANTED_LEVEL (0x48CB51, gr: unregistered): no operands; P0 = CWanted::MaximumWantedLevel
        // (0x8CDEE4); StoreParameters(1).
        0x050F => {
            set_sp(0, GPtr::<u32>::from_addr(MAX_WANTED_LEVEL).read());
            store(s, 1);
            0
        }
        // 0512 PRINT_HELP_FOREVER (0x48CB6D) / 0513 PRINT_HELP_FOREVER_WITH_NUMBER (0x48CBE2): t =
        // TheText.Get(label) (zeroed 16-byte buffer); 0513 then CollectParameters(1). Shown when
        // bDisplayNonMiniGameHelpMessages || the script's byte +0xC8 || !bMiniGameInProgress: 0512
        // SetHelpMessage(t, 0, 1, 0) and, when bAddNextMessageToPreviousBriefs, AddToPreviousBriefArray(t,
        // -1 x6, 0); 0513 SetHelpMessageWithNumber(t, P0, 0, 1). 0512 then sets
        // bAddNextMessageToPreviousBriefs = 1 (shown or not); 0513 does not.
        0x0512 | 0x0513 => {
            let mut buf = [0u8; 16];
            let l = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, l, 8);
            let t = text_get(GPtr::from_addr(THE_TEXT), l);
            if command == 0x0513 {
                collect(s, 1);
            }
            let show = GPtr::<u8>::from_addr(DISPLAY_NON_MINIGAME_HELP).read() != 0
                || s.byte_add(0xC8).read() != 0
                || GPtr::<u8>::from_addr(MINIGAME_IN_PROGRESS).read() == 0;
            let prev = GPtr::<u8>::from_addr(ADD_NEXT_MESSAGE_TO_PREVIOUS_BRIEFS);
            if command == 0x0512 {
                if show {
                    set_help_message(t, 0, 1, 0);
                    if prev.read() != 0 {
                        let m = 0xFFFF_FFFF;
                        add_to_previous_brief_array(t, m, m, m, m, m, m, 0);
                    }
                }
                prev.write(1);
            } else if show {
                set_help_message_with_number(t, sp(0), 0, 1);
            }
            0
        }
        // 04C1 CLEAR_ALL_SCRIPT_ROADBLOCKS (0x48BA8B, gr: unregistered)
        0x04C1 => {
            clear_script_road_blocks();
            0
        }
        // 04C5 HAS_CHAR_BEEN_PHOTOGRAPHED (0x48BB50): flag = ped && +0x470 bit 0x40000, which is cleared.
        0x04C5 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let mut b = false;
            if !ped.is_null() {
                let p = ped.byte_add(0x470).cast::<u32>();
                let v = p.read();
                if v & 0x40000 != 0 {
                    b = true;
                    p.write(v & !0x40000);
                }
            }
            set_flag(s, b);
            0
        }
        // 04C8 IS_CHAR_IN_FLYING_VEHICLE (0x48BBA6): no null checks (the vehicle pointer neither); flag = in
        // a vehicle && (GetVehicleAppearance() == 5 || (called again) == 3).
        0x04C8 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let mut b = false;
            if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
                let v = || ped.byte_add(0x58C).cast::<GPtr<u8>>().read();
                b = get_vehicle_appearance(v()) == 5 || get_vehicle_appearance(v()) == 3;
            }
            set_flag(s, b);
            0
        }
        // 04D1 CLEAR_HELI_ORIENTATION (0x48BD7B): no null check.
        0x04D1 => {
            collect(s, 1);
            clear_heli_orientation(vehicle_at_ref(pool(VEHICLE_POOL), spi(0)));
            0
        }
        // 04D2 PLANE_GOTO_COORDS (0x48BDA3, gr: unregistered): no null check; TellPlaneToGoToCoors(P1..P5).
        0x04D2 => {
            collect(s, 6);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            tell_plane_to_go_to_coors(v, sp(1), sp(2), sp(3), sp(4), sp(5));
            0
        }
        // 04D6 SET_ENABLE_RC_DETONATE_ON_CONTACT (0x48BFC3, gr: unregistered): byte 0xC1CC01 = P0 == 0.
        0x04D6 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(RC_DETONATE_ON_CONTACT_DISABLED).write(u8::from(sp(0) == 0));
            0
        }
        // 04D8 SET_CHAR_DROWNS_IN_WATER (0x48C02B): no null check; +0x470 bit 0x100000 = P1 != 0.
        0x04D8 => {
            collect(s, 2);
            set_bits(ped_at_ref(pool(PED_POOL), spi(0)), 0x470, 0x10_0000, sp(1) != 0);
            0
        }
        // 04D9 SET_OBJECT_RECORDS_COLLISIONS (0x48C07B, gr: unregistered): no null check; +0x40 bit
        // 0x10000000 = P1 != 0.
        0x04D9 => {
            collect(s, 2);
            set_bits(object_at_ref(pool(OBJECT_POOL), spi(0)), 0x40, 0x1000_0000, sp(1) != 0);
            0
        }
        // 04DA HAS_OBJECT_COLLIDED_WITH_ANYTHING (0x48C0C3, gr: unregistered): no null check; flag = byte
        // +0xB9 != 0.
        0x04DA => {
            collect(s, 1);
            let b = object_at_ref(pool(OBJECT_POOL), spi(0)).byte_add(0xB9).read() != 0;
            set_flag(s, b);
            0
        }
        // 04DB REMOVE_RC_BUGGY (0x48C109): no operands; Players[(u8)PlayerInFocus].BlowUpRCBuggy(false).
        0x04DB => {
            let i = u32::from(GPtr::<u8>::from_addr(PLAYER_IN_FOCUS).read());
            blow_up_rc_buggy(player_info(i), 0);
            0
        }
        // 04DD GET_CHAR_ARMOUR (0x48C12A): no null check; P0 = `_ftol2(m_fArmour (+0x548))` (low dword);
        // StoreParameters(1).
        0x04DD => {
            collect(s, 1);
            let a = ped_at_ref(pool(PED_POOL), spi(0)).byte_add(0x548).cast::<u32>().read();
            set_sp(0, X87::from_f32(f(a)).ftol() as u32);
            store(s, 1);
            0
        }
        // 04E0 SET_CAR_STRAIGHT_LINE_DISTANCE (0x48C1B0, gr: unregistered): no null check; byte +0x3DD =
        // (u8)P1.
        0x04E0 => {
            collect(s, 2);
            vehicle_at_ref(pool(VEHICLE_POOL), spi(0)).byte_add(0x3DD).write(sp(1) as u8);
            0
        }
        // 04E1 POP_CAR_BOOT (0x48C1DD, gr: unregistered): no null check; CAutomobile::PopBoot().
        0x04E1 => {
            collect(s, 1);
            pop_boot(vehicle_at_ref(pool(VEHICLE_POOL), spi(0)));
            0
        }
        // 04E3 SET_PLAYER_MOOD (0x48C216): CollectParameters(3) (P0 unused);
        // CAEPedSpeechAudioEntity::SetCJMood((u16)P1, P2, -1, -1, -1).
        0x04E3 => {
            collect(s, 3);
            set_cj_mood(sp(1) & 0xFFFF, sp(2), 0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_FFFF);
            0
        }
        // 04E5/04E6 LOCATE_OBJECT_2D/3D (0x48C28F): LocateObjectCommand(command); 04E9/04EA
        // IS_OBJECT_IN_AREA_2D/3D (0x48C2E9): ObjectInAreaCheckCommand(command). gr: unregistered.
        0x04E5 | 0x04E6 => {
            locate_object_command(s, command);
            0
        }
        0x04E9 | 0x04EA => {
            object_in_area_check_command(s, command);
            0
        }
        // 04E7 IS_OBJECT_IN_WATER (0x48C29E, gr: unregistered): flag = obj && +0x40 bit 0x100.
        0x04E7 => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let b = !o.is_null() && o.byte_add(0x40).cast::<u32>().read() & 0x100 != 0;
            set_flag(s, b);
            0
        }
        // 04EF REMOVE_ANIMATION (0x48C3D5, gr: unregistered): 16-byte label; idx =
        // GetAnimationBlockIndex(label); when ScriptResourceManager.RemoveFromResourceManager(idx, 1,
        // this) the unnamed 0x48B570(idx) (the IFP removal).
        0x04EF => {
            let buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_ptr().cast_mut());
            read_text_label(s, b, 16);
            let idx = get_animation_block_index(b);
            if remove_from_resource_manager(GPtr::from_addr(SCRIPT_RESOURCE_MANAGER), idx, 1, s) != 0 {
                remove_anim_48b570(idx);
            }
            0
        }
        // 04F0 IS_CHAR_WAITING_FOR_WORLD_COLLISION (0x48C418) / 04F1 IS_CAR_WAITING_FOR_WORLD_COLLISION
        // (0x48C45C, gr: unregistered): no null check; flag = +0x1C bit 0x40000.
        0x04F0 | 0x04F1 => {
            collect(s, 1);
            let e = if command == 0x04F0 {
                ped_at_ref(pool(PED_POOL), spi(0))
            } else {
                vehicle_at_ref(pool(VEHICLE_POOL), spi(0))
            };
            let b = e.byte_add(0x1C).cast::<u32>().read() & 0x40000 != 0;
            set_flag(s, b);
            0
        }
        // 04F9 SET_EXTRA_COLOURS (0x48C65A) / 04FA CLEAR_EXTRA_COLOURS (0x48C69D), gr: unregistered:
        // CTimeCycle::StartExtraColour(P0 - 1, P1 != 0) / StopExtraColour(P0 != 0).
        0x04F9 => {
            collect(s, 2);
            start_extra_colour(sp(0).wrapping_sub(1), u32::from(sp(1) != 0));
            0
        }
        0x04FA => {
            collect(s, 1);
            stop_extra_colour(u32::from(sp(0) != 0));
            0
        }
        // 04FC GET_WHEELIE_STATS (0x48C6D1, gr: unregistered): P0..P5 = the six dwords at Players[P0]
        // +0x118..+0x12C; StoreParameters(6); then the six are zeroed.
        0x04FC => {
            collect(s, 1);
            let p = player_info(sp(0)).byte_add(0x118).cast::<u32>();
            for i in 0..6 {
                set_sp(i, p.wrapping_index(i).read());
            }
            store(s, 6);
            for i in 0..6 {
                p.wrapping_index(i).write(0);
            }
            0
        }
        // 0501 SET_PLAYER_CAN_DO_DRIVE_BY (0x48C832): Players[P0] byte +0x153 = P1 != 0.
        0x0501 => {
            collect(s, 2);
            player_info(sp(0)).byte_add(0x153).write(u8::from(sp(1) != 0));
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
