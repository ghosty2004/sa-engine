//! `CRunningScript::ProcessCommands2600To2699` (0x479DA0, batch `mvpn-script-2`): complete port (every case of the
//! 39-entry jump table 0A28..0A4E; anything else, 0A4F included, is the original default -1). Cases follow the 1.0 US
//! bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2600To2699.md` (gta-reversed's handlers as a reading aid).
//! Unverified (docs/verify/HOWTO.md).
//!
//! Not reproduced: the SEH state stores of 0A2E, and the uninitialised upper bytes of the `UpdateCompareFlag`
//! argument word (the callee reads the low byte; the port passes the byte zero-extended).
#![allow(clippy::unreadable_literal)]

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending (every jump-table case).
pub const PORTED: &[u16] = &[
    0x0A28, 0x0A29, 0x0A2A, 0x0A2B, 0x0A2C, 0x0A2D, 0x0A2E, 0x0A2F, 0x0A30, 0x0A31, 0x0A32, 0x0A33, 0x0A34, 0x0A35,
    0x0A36, 0x0A37, 0x0A38, 0x0A39, 0x0A3A, 0x0A3B, 0x0A3C, 0x0A3D, 0x0A3E, 0x0A3F, 0x0A40, 0x0A41, 0x0A42, 0x0A43,
    0x0A44, 0x0A45, 0x0A46, 0x0A47, 0x0A48, 0x0A49, 0x0A4A, 0x0A4B, 0x0A4C, 0x0A4D, 0x0A4E,
];

/// `CMenuManager` widescreen option (u8 0xBA6793), `FrontEndMenuManager.m_bMenuAccessWidescreen` (u8 0xBA677C), the
/// joypad option (u8 0xBA6818), the mouse vertical inversion option (u8 0xBA6745)
const MENU_WIDESCREEN: usize = 0xBA6793;
const MENU_ACCESS_WIDESCREEN: usize = 0xBA677C;
const MENU_USING_JOYPAD: usize = 0xBA6818;
const MENU_INVERT_MOUSE_Y: usize = 0xBA6745;
/// `CTheScripts`: subtitles / odd-job title before fade (u8 0xA44488 / 0xA44489), photo camera effect (0xA44490),
/// non-minigame help messages (u8 0xA444A7), `UsedObjectArray[].nModelIndex` (0xA44B88 - 0x1C * -id), debug counter
/// (0xA95190), `ScriptsForBrains` (0xA90CF0)
const DRAW_SUBTITLES_BEFORE_FADE: usize = 0xA44488;
const DRAW_ODDJOB_TITLE_BEFORE_FADE: usize = 0xA44489;
const PHOTO_CAMERA_EFFECT: usize = 0xA44490;
const DISPLAY_NON_MINIGAME_HELP: usize = 0xA444A7;
const USED_OBJECT_MODEL: u32 = 0xA44B88;
const DEBUG_STUFF: usize = 0xA95190;
const SCRIPTS_FOR_BRAINS: usize = 0xA90CF0;
/// `TheText` (0xC1B340), `CHud::m_pHelpMessageToPrint` (0xBAA480)
const THE_TEXT: usize = 0xC1B340;
const HELP_MESSAGE_TO_PRINT: usize = 0xBAA480;
/// Follow-node-route defaults: time 50000 (0x86FCAC), slow-down distance 3.0 (0x86FCA4), height change 2.0 (0x86FCA8)
const ROUTE_DEFAULT_TIME: usize = 0x86FCAC;
const ROUTE_SLOW_DOWN: usize = 0x86FCA4;
const ROUTE_HEIGHT_CHANGE: usize = 0x86FCA8;
/// `CVehicle::ms_forceVehicleLightsOff` (u8 0xC1CC18), `CGameLogic::bScriptCoopGameGoingOn` (u8 0x96A8A8), the pimp
/// cheat flag (u8 0x96918A)
const FORCE_LIGHTS_OFF: usize = 0xC1CC18;
const COOP_GAME: usize = 0x96A8A8;
const PIMP_CHEAT: usize = 0x96918A;
/// `TheCamera` in-car camera mode (0xB6F0DC)
const CAR_CAMERA_MODE: usize = 0xB6F0DC;
/// Railtrack resistance (f32 0x8D34AC) and its base (0x8D34B0)
const RAIL_RESISTANCE: usize = 0x8D34AC;
const RAIL_RESISTANCE_BASE: usize = 0x8D34B0;
/// Mouse movement of pad 0's new mouse state (0xB73424, 0xB73428)
const MOUSE_X: usize = 0xB73424;
const MOUSE_Y: usize = 0xB73428;

orig! {
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    fn intel_get_task_swim(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CPedIntelligence::GetTaskClimb()` (0x601180)
    fn intel_get_task_climb(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601180 => unported;
    /// `CText::Get(const char* key)` (0x6A0050)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CHud::HelpMessageDisplayed()` (0x588B50), AL
    fn hud_help_message_displayed() -> u8 = cdecl 0x588B50 => unported;
    /// `CMessages::StringCopy(char16* dst, const char16* src, uint16 size)` (0x69DB70)
    fn messages_string_copy(dst: GPtr<u8>, src: GPtr<u8>, size: u32) = cdecl 0x69DB70 => unported;
    /// `CMessages::InsertPlayerControlKeysInString(char16*)` (0x69E160)
    fn messages_insert_player_control_keys(s: GPtr<u8>) = cdecl 0x69E160 => unported;
    /// `CMessages::GetStringLength(const char16*)` (0x69DB50)
    fn messages_get_string_length(s: GPtr<u8>) -> u32 = cdecl 0x69DB50 => unported;
    /// `CMessages::StringCompare(const char16*, const char16*, uint16)` (0x69DBD0), AL
    fn messages_string_compare(a: GPtr<u8>, b: GPtr<u8>, len: u32) -> u8 = cdecl 0x69DBD0 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexFollowNodeRoute::CTaskComplexFollowNodeRoute(int32 moveState, const CVector& target, float radius,
    /// float slowDownDist, float heightChange, bool keepNodesHeadingAway, int32 time, bool useBlending)` (0x66EA30,
    /// `ret 0x20`)
    fn follow_node_route_ctor(t: GPtr<u8>, move_state: u32, target: GPtr<u8>, radius: u32, slow_down: u32,
        height_change: u32, keep_heading: u32, time: i32, blending: u32) -> GPtr<u8> = thiscall 0x66EA30 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 pedHandle, CTask*, int32 opcode)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: i32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => crate::script_tasks::give_ped_scripted_task;
    /// `CVehicle::Fix()` (vtable +0xC8)
    fn v_vehicle_fix(v: GPtr<u8>) = thiscall vslot 0xC8 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlayerPed::ForceGroupToNeverFollow(bool)` (0x60C800, `ret 4`)
    fn player_ped_force_group_to_never_follow(p: GPtr<u8>, b: u32) = thiscall 0x60C800 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160), ECX = the vehicle pool
    fn vehicle_get_ref(pool: GPtr<u8>, v: GPtr<u8>) -> i32 = thiscall 0x424160 => unported;
    /// `CGameLogic::SetUpSkip(CVector coors, float angle, bool afterMission, CEntity* vehicle, bool finishedByScript)`
    /// (0x4423C0, the vector by value)
    fn game_logic_set_up_skip(x: u32, y: u32, z: u32, angle: u32, after_mission: u32, veh: GPtr<u8>, by_script: u32) = cdecl 0x4423C0 => unported;
    /// `CGameLogic::IsSkipWaitingForScriptToFadeIn()` (0x4416C0), AL
    fn game_logic_is_skip_waiting_for_script_to_fade_in() -> u8 = cdecl 0x4416C0 => unported;
    /// `CConversations::SetUpConversationNode(const char* name, int32 yes, int32 no, int32 speech, int32 yesSpeech,
    /// int32 noSpeech)` (0x43A870)
    fn conversations_set_up_conversation_node(name: GPtr<u8>, a: i32, b: i32, c: i32, d: i32, e: i32) = cdecl 0x43A870 => unported;
    /// `CConversations::DoneSettingUpConversation(bool suppressSubtitles)` (0x43ADB0)
    fn conversations_done_setting_up_conversation(b: u32) = cdecl 0x43ADB0 => unported;
    /// `CCheat::ApplyCheat(eCheats)` (0x438370)
    fn apply_cheat(id: u32) = cdecl 0x438370 => unported;
    /// `CRunningScript::IsPedDead(CPed*)` (0x464D70), AL
    fn is_ped_dead(s: Scr, p: GPtr<u8>) -> u8 = thiscall 0x464D70 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `C3dMarkers::User3dMarkerSet(float x, float y, float z, int32 colour)` (0x720FD0)
    fn user_3d_marker_set(x: u32, y: u32, z: u32, colour: u32) -> i32 = cdecl 0x720FD0 => unported;
    /// `C3dMarkers::User3dMarkerDelete(int32)` (0x721090)
    fn user_3d_marker_delete(i: i32) = cdecl 0x721090 => unported;
    /// `CTaskComplexProstituteSolicit::GetRidOfPlayerProstitute()` (0x661D30)
    fn get_rid_of_player_prostitute() = cdecl 0x661D30 => unported;
    /// `CScriptsForBrains::SwitchAllObjectBrainsWithThisID(int8 id, bool)` (0x46A900, `ret 8`)
    fn switch_all_object_brains_with_this_id(b: GPtr<u8>, id: u32, on: u32) = thiscall 0x46A900 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
}

fn gp(a: usize) -> GPtr<u8> {
    GPtr::from_addr(a)
}
fn g8(a: usize) -> GPtr<u8> {
    GPtr::from_addr(a)
}
fn g32(a: usize) -> GPtr<u32> {
    GPtr::from_addr(a)
}

/// 0A3E: the last random ped (created by 1, no bit 11 at +0x1C, no bit 3 at +0x470, alive, no bit 8 at +0x46C, in no
/// group) inside the box, scanning the ped pool from the top; its handle or -1.
fn random_char_in_area(s: Scr) -> u32 {
    let (x, y, z) = (spf(0), spf(1), spf(2));
    let (rx, ry, rz) = (spf(3), spf(4), spf(5));
    let min_x = X87::from_f32(x).fsub_m(rx).to_f32();
    let min_y = X87::from_f32(y).fsub_m(ry).to_f32();
    let min_z = X87::from_f32(z).fsub_m(rz).to_f32();
    let max_x = X87::from_f32(rx).fadd_m(x).to_f32();
    let max_y = X87::from_f32(ry).fadd_m(y).to_f32();
    let max_z = X87::from_f32(rz).fadd_m(z).to_f32();
    let mut i = rd_u32(pool(PED_POOL), 8) as i32;
    while i != 0 {
        let p = pool(PED_POOL);
        i -= 1;
        if (rd_u8(GPtr::from_addr(rd_u32(p, 4) as usize), i as isize) as i8) < 0 {
            continue;
        }
        let ped: GPtr<u8> = GPtr::from_addr(rd_u32(p, 0).wrapping_add(i as u32 * 0x7C4) as usize);
        if ped.is_null()
            || rd_u8(ped, 0x484) != 1
            || rd_u32(ped, 0x1C) & 0x800 != 0
            || rd_u8(ped, 0x470) & 8 != 0
            || is_ped_dead(s, ped) != 0
            || rd_u32(ped, 0x46C) & 0x100 != 0
            || !get_peds_group(ped).is_null()
        {
            continue;
        }
        let m: GPtr<u8> = GPtr::from_addr(rd_u32(ped, 0x14) as usize);
        let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
        let (px, py, pz) = (rd_f32(pos, 0), rd_f32(pos, 4), rd_f32(pos, 8));
        if px >= min_x && px <= max_x && py >= min_y && py <= max_y && pz >= min_z && pz <= max_z {
            return ped_get_ref(pool(PED_POOL), ped) as u32;
        }
    }
    u32::MAX
}

fn rd_u32(p: GPtr<u8>, off: isize) -> u32 {
    p.byte_add(off).cast::<u32>().read()
}
fn rd_u8(p: GPtr<u8>, off: isize) -> u8 {
    p.byte_add(off).read()
}
fn rd_f32(p: GPtr<u8>, off: isize) -> f32 {
    p.byte_add(off).cast::<f32>().read()
}

/// `CRunningScript::ProcessCommands2600To2699(int32 command)` (0x479DA0, thiscall, `ret 4`): AL = 0 continue, -1
/// unknown (every case returns 0).
#[hook(addr = 0x479da0, conv = thiscall, name = "CRunningScript::ProcessCommands2600To2699", expect = "6A FF 64 A1 00 00 00 00",
       this = "CRunningScript", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "complete switch (39 cases); 0A2E allocates a follow-node-route task")]
pub fn process_commands_2600_to_2699(s: Scr, command: u32) -> i8 {
    match command {
        // 0A28 SET_SWIM_SPEED: the swim task's speed (+0x10) = SP1 when the ped and its swim task exist
        0x0A28 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let speed = sp(1);
            if !ped.is_null() {
                let intel = GPtr::from_addr(rd_u32(ped, 0x47C) as usize);
                if !intel_get_task_swim(intel).is_null() {
                    let t = intel_get_task_swim(GPtr::from_addr(rd_u32(ped, 0x47C) as usize));
                    t.byte_add(0x10).cast::<u32>().write(speed);
                }
            }
        }
        // 0A29 IS_PLAYER_CLIMBING: the player ped has a climb task (no null checks)
        0x0A29 => {
            collect(s, 1);
            let ped = GPtr::from_addr(rd_u32(player_info(sp(0)), 0) as usize);
            let t = intel_get_task_climb(GPtr::from_addr(rd_u32(ped, 0x47C) as usize));
            flag(s, u32::from(!t.is_null()));
        }
        // 0A2A IS_THIS_HELP_MESSAGE_BEING_DISPLAYED: with a help message on screen, the text with the control keys
        // inserted against `CHud::m_pHelpMessageToPrint` over its length
        0x0A2A => {
            let mut label = [0u8; 8];
            read_text_label(s, GPtr::from_raw(label.as_mut_ptr()), 8);
            let text = text_get(gp(THE_TEXT), GPtr::from_raw(label.as_mut_ptr()));
            let mut r = 0u8;
            if hud_help_message_displayed() != 0 {
                let mut buf = [0u16; 200];
                let b: GPtr<u8> = GPtr::from_raw(buf.as_mut_ptr()).cast();
                messages_string_copy(b, text, 0x190);
                messages_insert_player_control_keys(b);
                let len = messages_get_string_length(b);
                r = messages_string_compare(b, gp(HELP_MESSAGE_TO_PRINT), len);
            }
            flag(s, u32::from(r));
        }
        // 0A2B IS_WIDESCREEN_ON_IN_OPTIONS: the menu byte as the flag
        0x0A2B => flag(s, u32::from(g8(MENU_WIDESCREEN).read())),
        // 0A2C DRAW_SUBTITLES_BEFORE_FADE / 0A2D DRAW_ODDJOB_TITLE_BEFORE_FADE: the low byte of SP0
        0x0A2C => {
            collect(s, 1);
            g8(DRAW_SUBTITLES_BEFORE_FADE).write(sp(0) as u8);
        }
        0x0A2D => {
            collect(s, 1);
            g8(DRAW_ODDJOB_TITLE_BEFORE_FADE).write(sp(0) as u8);
        }
        // 0A2E TASK_FOLLOW_PATH_NODES_TO_COORD_WITH_RADIUS: time -1 → [0x86FCAC], -2 → -1
        0x0A2E => {
            collect(s, 7);
            let mut target = [sp(1), sp(2), sp(3)];
            let move_state = sp(4);
            let radius = sp(6);
            let time = match spi(5) {
                -1 => g32(ROUTE_DEFAULT_TIME).read() as i32,
                -2 => -1,
                t => t,
            };
            let t = task_new(0x60);
            let task = if t.is_null() {
                GPtr::NULL
            } else {
                follow_node_route_ctor(t, move_state, GPtr::from_raw(target.as_mut_ptr()).cast(), radius,
                                       g32(ROUTE_SLOW_DOWN).read(), g32(ROUTE_HEIGHT_CHANGE).read(), 1, time, 1)
            };
            give_ped_scripted_task(s, spi(0), task, command);
        }
        // 0A2F SET_PHOTO_CAMERA_EFFECT: SP0 != 0 ? 2 : 0
        0x0A2F => {
            collect(s, 1);
            g32(PHOTO_CAMERA_EFFECT).write(if sp(0) != 0 { 2 } else { 0 });
        }
        // 0A30 FIX_CAR: `Fix()`, health 1000 (no null check)
        0x0A30 => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            v_vehicle_fix(v);
            v.byte_add(0x4C0).cast::<u32>().write(0x447A_0000);
        }
        // 0A31 SET_PLAYER_GROUP_TO_FOLLOW_NEVER
        0x0A31 => {
            collect(s, 2);
            let b = u32::from(sp(1) != 0);
            player_ped_force_group_to_never_follow(find_player_ped(spi(0)), b);
        }
        // 0A32 IS_CHAR_ATTACHED_TO_ANY_CAR: the ped's attached-to entity (+0xFC) is a vehicle (type 2)
        0x0A32 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let e = GPtr::<u8>::from_addr(rd_u32(ped, 0xFC) as usize);
            flag(s, u32::from(!e.is_null() && rd_u8(e, 0x36) & 7 == 2));
        }
        // 0A33 STORE_CAR_CHAR_IS_ATTACHED_TO_NO_SAVE: its handle or -1
        0x0A33 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let e = GPtr::<u8>::from_addr(rd_u32(ped, 0xFC) as usize);
            let h = if !e.is_null() && rd_u8(e, 0x36) & 7 == 2 { vehicle_get_ref(pool(VEHICLE_POOL), e) } else { -1 };
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 0A34 SET_UP_SKIP_TO_BE_FINISHED_BY_SCRIPT, 0A38 SET_RENDER_PLAYER_WEAPON, 0A42 REMOVE_ALLUSER_3D_MARKERS:
        // empty cases
        0x0A34 | 0x0A38 | 0x0A42 => {}
        // 0A35 SET_UP_SKIP_FOR_VEHICLE_FINISHED_BY_SCRIPT
        0x0A35 => {
            collect(s, 5);
            let (x, y, z, angle) = (sp(0), sp(1), sp(2), sp(3));
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(4));
            game_logic_set_up_skip(x, y, z, angle, 0, v, 1);
        }
        // 0A36 IS_SKIP_WAITING_FOR_SCRIPT_TO_FADE_IN
        0x0A36 => flag(s, u32::from(game_logic_is_skip_waiting_for_script_to_fade_in())),
        // 0A37 FORCE_ALL_VEHICLE_LIGHTS_OFF: the low byte of SP0
        0x0A37 => {
            collect(s, 1);
            g8(FORCE_LIGHTS_OFF).write(sp(0) as u8);
        }
        // 0A39 GET_PLAYER_IN_CAR_CAMERA_MODE
        0x0A39 => {
            set_sp(0, g32(CAR_CAMERA_MODE).read());
            store(s, 1);
        }
        // 0A3A IS_LAST_BUILDING_MODEL_SHOT_BY_PLAYER: a negative model is a used-object index
        0x0A3A => {
            collect(s, 2);
            let info = player_info(sp(0));
            let mut model = sp(1) as i32;
            if model < 0 {
                model = GPtr::<i32>::from_addr(USED_OBJECT_MODEL.wrapping_sub(model.wrapping_mul(0x1C) as u32) as usize)
                    .read();
            }
            flag(s, u32::from(rd_u32(info, 0xA0) as i32 == model));
        }
        // 0A3B CLEAR_LAST_BUILDING_MODEL_SHOT_BY_PLAYER
        0x0A3B => {
            collect(s, 1);
            player_info(sp(0)).byte_add(0xA0).cast::<u32>().write(u32::MAX);
        }
        // 0A3C SET_UP_CONVERSATION_END_NODE_WITH_SCRIPTED_SPEECH: `SetUpConversationNode(label, 0, 0, -SP0, 0, 0)`
        0x0A3C => {
            let mut label = [0u8; 8];
            read_text_label(s, GPtr::from_raw(label.as_mut_ptr()), 8);
            collect(s, 1);
            conversations_set_up_conversation_node(GPtr::from_raw(label.as_mut_ptr()), 0, 0, spi(0).wrapping_neg(), 0, 0);
        }
        // 0A3D ACTIVATE_PIMP_CHEAT: cheat 0x5A, or the pimp flag cleared
        0x0A3D => {
            collect(s, 1);
            if sp(0) != 0 {
                apply_cheat(0x5A);
            } else {
                g8(PIMP_CHEAT).write(0);
            }
        }
        // 0A3E GET_RANDOM_CHAR_IN_AREA_OFFSET_NO_SAVE
        0x0A3E => {
            collect(s, 6);
            let h = random_char_in_area(s);
            set_sp(0, h);
            store(s, 1);
        }
        // 0A3F SET_SCRIPT_COOP_GAME
        0x0A3F => {
            collect(s, 1);
            g8(COOP_GAME).write(u8::from(sp(0) != 0));
        }
        // 0A40 CREATE_USER_3D_MARKER
        0x0A40 => {
            collect(s, 4);
            let r = user_3d_marker_set(sp(0), sp(1), sp(2), sp(3));
            set_sp(0, r as u32);
            store(s, 1);
        }
        // 0A41 REMOVE_USER_3D_MARKER
        0x0A41 => {
            collect(s, 1);
            user_3d_marker_delete(spi(0));
        }
        // 0A43 GET_RID_OF_PLAYER_PROSTITUTE
        0x0A43 => get_rid_of_player_prostitute(),
        // 0A44 DISPLAY_NON_MINIGAME_HELP_MESSAGES
        0x0A44 => {
            collect(s, 1);
            g8(DISPLAY_NON_MINIGAME_HELP).write(u8::from(sp(0) != 0));
        }
        // 0A45 SET_RAILTRACK_RESISTANCE_MULT: SP0 > 0.0 (double) → base * SP0 (x87, stored f32), else the base
        0x0A45 => {
            collect(s, 1);
            let m = X87::from_f32(spf(0));
            let base = GPtr::<f32>::from_addr(RAIL_RESISTANCE_BASE);
            if m.fcom(X87::from_f64_exact(0.0)).test_any(0x41) {
                g32(RAIL_RESISTANCE).write(g32(RAIL_RESISTANCE_BASE).read());
            } else {
                GPtr::<f32>::from_addr(RAIL_RESISTANCE).write(X87::from_f32(base.read()).fmul(m).to_f32());
            }
        }
        // 0A46 SWITCH_OBJECT_BRAINS: the zero-extended low byte of SP0
        0x0A46 => {
            collect(s, 2);
            let on = u32::from(sp(1) != 0);
            switch_all_object_brains_with_this_id(gp(SCRIPTS_FOR_BRAINS), sp(0) & 0xFF, on);
        }
        // 0A47 FINISH_SETTING_UP_CONVERSATION_NO_SUBTITLES
        0x0A47 => conversations_done_setting_up_conversation(1),
        // 0A48 ALLOW_PAUSE_IN_WIDESCREEN
        0x0A48 => {
            collect(s, 1);
            g8(MENU_ACCESS_WIDESCREEN).write(u8::from(sp(0) != 0));
        }
        // 0A49 IS_XBOX_VERSION, 0A4D IS_JAPANESE_VERSION: false
        0x0A49 | 0x0A4D => flag(s, 0),
        // 0A4A GET_PC_MOUSE_MOVEMENT: `CPad::GetPad(0)` twice (results unused), the mouse movement words
        0x0A4A => {
            pad_get_pad(0);
            set_sp(0, g32(MOUSE_X).read());
            pad_get_pad(0);
            set_sp(1, g32(MOUSE_Y).read());
            store(s, 2);
        }
        // 0A4B IS_PC_USING_JOYPAD, 0A4C IS_MOUSE_USING_VERTICAL_INVERSION
        0x0A4B => flag(s, u32::from(g8(MENU_USING_JOYPAD).read() != 0)),
        0x0A4C => flag(s, u32::from(g8(MENU_INVERT_MOUSE_Y).read() != 0)),
        // 0A4E IS_XBOX_PLAYER2_PRESSING_START / DO_DEBUG_STUFF: the debug counter
        0x0A4E => {
            let c = g32(DEBUG_STUFF);
            c.write(c.read().wrapping_add(1));
        }
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2600_TO_2699];
