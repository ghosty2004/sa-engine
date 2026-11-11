//! `CHud` drawing orchestration (PORT-HUD-2): `CHud::Draw`, `DrawAfterFade` and the per-element draw
//! functions. Ported from the 1.0 US bytes; gta-reversed `Hud.cpp` as reading aid (the bytes win).
//!
//! Only the orchestration is Rust: conditions, coordinates, x87 arithmetic in the original order, global
//! reads and writes, timers and state. Every call into RenderWare (`RwEngineInstance` render states),
//! `CSprite2d`, `CSprite`, `CFont`, `CRadar`, `CMessages` and any other function stays an `orig!` call to its
//! original address. Foreign globals are read by address (`g::<T>(addr)`); the CHud statics this module
//! writes are declared in [`GLOBALS`].

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, rd, x, xi};

sa_core::globals! {
    list GLOBALS_DECL;
    /// `CHud::bDrawingVitalStats`
    pub static DRAWING_VITAL_STATS @ 0xBAB1DE: u8 = 0;
    /// `CHud::HelpTripSkipShown`
    pub static HELP_TRIP_SKIP_SHOWN @ 0xBAB229: u8 = 0;
    /// `CHud::m_WantedState`
    pub static WANTED_STATE @ 0xBAA414: u32 = 0;
    /// `CHud::m_WantedFadeTimer`
    pub static WANTED_FADE_TIMER @ 0xBAA418: i32 = 0;
    /// `CHud::m_WantedTimer`
    pub static WANTED_TIMER @ 0xBAA41C: i32 = 0;
    /// `CHud::m_LastWanted`
    pub static LAST_WANTED @ 0xBAA420: i32 = 0;
    /// Unnamed CHud byte (0xBAB228) `DrawWanted` writes: 1 when the wanted level equals `m_LastWanted` this
    /// frame, 0 when it changed.
    pub static WANTED_LEVEL_UNCHANGED @ 0xBAB228: u8 = 0;
}

/// This module's globals.
pub const GLOBALS: &[GlobalDesc] = GLOBALS_DECL;

// Foreign globals (read by address).
/// `CReplay::Mode` (1 = playback)
const REPLAY_MODE: usize = 0xA43088;
/// `CWeapon::ms_bTakePhoto`
const TAKE_PHOTO: usize = 0xC8A7C1;
/// `FrontEndMenuManager.m_bActivateMenuNextFrame`
const MENU_ACTIVATE_NEXT_FRAME: usize = 0xBA677B;
/// `FrontEndMenuManager.m_bMenuActive`
const MENU_ACTIVE: usize = 0xBA67A4;
/// `FrontEndMenuManager.m_bHudOn`
const MENU_HUD_ON: usize = 0xBA6769;
/// `gbCineyCamProcessedOnFrame`
const CINEY_CAM_FRAME: usize = 0xB6EC40;
/// `CTimer::m_FrameCounter`
const FRAME_COUNTER: usize = 0xB7CB4C;
/// `CTimer::m_UserPause`
const USER_PAUSE: usize = 0xB7CB49;
/// `TheCamera.m_bWideScreenOn`
const WIDE_SCREEN_ON: usize = 0xB6F065;
/// `CEntryExitManager::ms_exitEnterState`
const EXIT_ENTER_STATE: usize = 0x96A7CC;
/// `FrontEndMenuManager.m_nRadarMode` (1 = blips only, 2 = off)
const MENU_RADAR_MODE: usize = 0xBA676C;
/// `CHud::m_ItemToFlash` (int16)
const ITEM_TO_FLASH: usize = 0xBAB1DC;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CWorld::PlayerInFocus` (byte)
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
/// `CWorld::Players[]` (stride 0x190); `m_nMaxHealth` at +0x14F
const PLAYERS: usize = 0xB7CD98;
/// `CCutsceneMgr::ms_running`
const CUTSCENE_RUNNING: usize = 0xB5F851;
/// `CGarages::MessageIDString[0]`
const GARAGES_MESSAGE_ID: usize = 0x96C014;
/// `CHud::m_BigMessage[4][0]`
const BIG_MESSAGE_4: usize = 0xBAAEC0;
/// `CTheScripts::IntroTextLines[0x60]` (0x44 bytes each) + 4: the loop pointer starts at `Scale.y`
const INTRO_TEXT_LINES_4: usize = 0xA913EC;
/// `CHud::m_BigMessage[1]` (mission title) and `[2]` (busted / wasted)
const BIG_MESSAGE_1: usize = 0xBAAD40;
const BIG_MESSAGE_2: usize = 0xBAADC0;
/// `CHud::BigMessageAlpha[1]`, `[2]`
const BIG_MESSAGE_ALPHA_1: usize = 0xBAA3A8;
const BIG_MESSAGE_ALPHA_2: usize = 0xBAA3AC;
/// `CHud::BigMessageInUse[1]`
const BIG_MESSAGE_IN_USE_1: usize = 0xBAA3C4;
/// `CHud::BigMessageX[1]`, `[2]`
const BIG_MESSAGE_X_1: usize = 0xBAA3E0;
const BIG_MESSAGE_X_2: usize = 0xBAA3E4;
/// `CHud::m_ZoneFadeTimer`
const ZONE_FADE_TIMER: usize = 0xBAA934;
/// `DrawBustedWastedMessage`'s function-local static Y (float) and its guard bit (bit 0 of 0xBAB224)
const BUSTED_Y: usize = 0xBAB220;
const BUSTED_Y_GUARD: usize = 0xBAB224;
/// `CHud::m_BigMessage[3]`, `[5]`
const BIG_MESSAGE_3: usize = 0xBAAE40;
const BIG_MESSAGE_5: usize = 0xBAAF40;
/// `CHud::BigMessageAlpha[0]`, `BigMessageInUse[0]`, `BigMessageX[0]`
const BIG_MESSAGE_ALPHA_0: usize = 0xBAA3A4;
const BIG_MESSAGE_IN_USE_0: usize = 0xBAA3C0;
const BIG_MESSAGE_X_0: usize = 0xBAA3DC;
/// `DrawSuccessFailedMessage`'s function-local static Y (float) and its guard bit (bit 0 of 0xBAB21C)
const SUCCESS_Y: usize = 0xBAB218;
const SUCCESS_Y_GUARD: usize = 0xBAB21C;
/// `CHud::m_Message[0]`
const MESSAGE_0: usize = 0xBAB040;
/// `FrontEndMenuManager.m_bShowSubtitles`
const MENU_SHOW_SUBTITLES: usize = 0xBA678C;
/// Unnamed byte `DrawSubtitles` keeps (0xBAB214): 1 after a wide-screen frame; the next normal frame
/// clears `m_Message[0]`.
const SUBTITLES_WIDESCREEN: usize = 0xBAB214;
/// `CTheScripts::bUseMessageFormatting`, `MessageWidth`, `MessageCentre`
const SCRIPTS_USE_MESSAGE_FORMATTING: usize = 0xA44B66;
const SCRIPTS_MESSAGE_WIDTH: usize = 0xA44B60;
const SCRIPTS_MESSAGE_CENTRE: usize = 0xA44B64;
/// `TheCamera`
const THE_CAMERA: usize = 0xB6F028;
/// `CTheScripts::bPlayerIsOffTheMap`
const SCRIPTS_PLAYER_OFF_THE_MAP: usize = 0xA444A4;
/// `CGangWars::bGangWarsActive`
const GANG_WARS_ACTIVE: usize = 0x96AB90;
/// `CPopCycle::m_pCurrZoneInfo`
const CURR_ZONE_INFO: usize = 0xC0BC68;
/// `CHud::m_ZoneToPrint`, `m_pLastZoneName`
const ZONE_TO_PRINT: usize = 0xBAB1D0;
const LAST_ZONE_NAME: usize = 0xBAB1D4;
/// `CTheScripts::bDrawOddJobTitleBeforeFade`
const SCRIPTS_ODD_JOB_TITLE_BEFORE_FADE: usize = 0xA44489;
/// `CHud::m_BigMessage[6]`
const BIG_MESSAGE_6: usize = 0xBAAFC0;
/// `CHud::OddJob2OffTimer` (float), `OddJob2XOffset` (float), `OddJob2Timer` (uint16), `OddJob2On` (int16)
const ODD_JOB2_OFF_TIMER: usize = 0xBAA398;
const ODD_JOB2_X_OFFSET: usize = 0xBAA39C;
const ODD_JOB2_TIMER: usize = 0xBAA3A0;
const ODD_JOB2_ON: usize = 0xBAB1E0;
/// `CHud::bScriptForceDisplayWithCounters`
const FORCE_DISPLAY_WITH_COUNTERS: usize = 0xBAA3FA;
/// `CWorld::Players[1].m_pPed` and `Players[1].m_nMaxHealth`
const PLAYER2_PED: usize = 0xB7CF28;
const PLAYER2_MAX_HEALTH: usize = 0xB7D077;
/// `CUserDisplay::OnscnTimer` (`COnscreenTimer` 0xBA1788): clock key (+4), clock text (+0xE), clock
/// enabled (+0x38), the four counters (+0x44, 0x44 bytes: +4 key, +0xE type, +0x10 text, +0x3A enabled,
/// +0x3B flash, +0x3C colour) and the display flag (+0x150)
const TIMER_CLOCK_KEY: usize = 0xBA178C;
const TIMER_CLOCK_TEXT: usize = 0xBA1796;
const TIMER_CLOCK_ENABLED: usize = 0xBA17C0;
const TIMER_COUNTERS: usize = 0xBA17CC;
const TIMER_DISPLAY: usize = 0xBA18D8;
/// `CHud::TimerMainCounterHideState` (int16), `TimerMainCounterWasDisplayed`,
/// `TimerCounterHideState[4]` (int16), `TimerCounterWasDisplayed[4]` (bytes)
const TIMER_MAIN_HIDE_STATE: usize = 0xBAA388;
const TIMER_MAIN_WAS_DISPLAYED: usize = 0xBAA38A;
const TIMER_COUNTER_HIDE_STATE: usize = 0xBAA38C;
const TIMER_COUNTER_WAS_DISPLAYED: usize = 0xBAA394;
/// `AudioEngine`
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `FrontEndMenuManager.m_bWidescreenOn`
const MENU_WIDESCREEN_ON: usize = 0xBA6793;
/// Help box layout globals (.data floats, 200.0 and 150.0 after `ReInitialise`)
const HELP_BOX_WIDTH: usize = 0x8D0934;
const HELP_BOX_Y: usize = 0x8D0938;
/// `CPedGroups::ms_groups[0]` + 8 (`CPedGroupMembership`; 0x2D4 per group)
const PED_GROUP_MEMBERSHIP: usize = 0xC09928;
/// `CTheScripts::bDisplayHud`
const SCRIPTS_DISPLAY_HUD: usize = 0xA444A0;
/// `CTheScripts::bDrawSubtitlesBeforeFade`
const SCRIPTS_SUBTITLES_BEFORE_FADE: usize = 0xA44488;
/// `CCutsceneMgr::ms_cutsceneProcessing`
const CUTSCENE_PROCESSING: usize = 0xB5F852;
/// `CHud::bScriptDontDisplayAreaName`
const DONT_DISPLAY_AREA_NAME: usize = 0xBAA3F8;
/// `CHud::bScriptDontDisplayVehicleName`
const DONT_DISPLAY_VEHICLE_NAME: usize = 0xBAA3F9;
/// `CHud::bScriptDontDisplayRadar`
const DONT_DISPLAY_RADAR: usize = 0xBAA3FB;
/// `CHud::m_bDraw3dMarkers`
const DRAW_3D_MARKERS: usize = 0xBAA45C;
/// `CHud::m_BigMessage[STYLE_MIDDLE][0]` (first byte of the array)
const BIG_MESSAGE_0: usize = 0xBAACC0;
/// `CMenuSystem::num_menus_in_use`
const NUM_MENUS_IN_USE: usize = 0xBA82E3;
/// `TheText` (`CText`)
const THE_TEXT: usize = 0xC1B340;
/// `CHud::Sprites[6]` (`CSprite2d`, 4 bytes each; [0] = fist)
const SPRITES: usize = 0xBAB1FC;
/// `CHud::Sprites[5]` (trip skip icon)
const SPRITE_SKIP_ICON: usize = 0xBAB210;
/// `CHud::Sprites[3]` (radar disc) and `[4]` (plane radar ring)
const SPRITE_RADAR_DISC: usize = 0xBAB208;
const SPRITE_RADAR_RING_PLANE: usize = 0xBAB20C;
/// `HudColour` (`CHudColours`)
const HUD_COLOUR: usize = 0xBAB22C;
/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CTxdStore::ms_pTxdPool`
const TXD_POOL: usize = 0xC8800C;
/// `gString` (char[]) and `gGxtString` (GxtChar[])
const G_STRING: usize = 0xB71670;
const G_GXT_STRING: usize = 0xC1B100;

// .rdata strings
/// `"SKIP_1"`
const STR_SKIP_1: usize = 0x866CB0;
/// `"hud"`
const STR_HUD: usize = 0x86696C;
/// `"FEC_TSK"`
const STR_FEC_TSK: usize = 0x866560;
/// `"MODELS\\HUD.TXD"`
const STR_HUD_TXD: usize = 0x86A664;
/// `CHud::Sprites` name / mask table (6 x {name, mask} pointers, .data)
const SPRITE_NAMES: usize = 0x8D128C;
/// `"STAT00%d"`, `"STAT0%d"`, `"STAT%d"`, `"+"`
const STR_STAT00: usize = 0x864300;
const STR_STAT0: usize = 0x8642F8;
const STR_STAT: usize = 0x8642F0;
const STR_PLUS: usize = 0x866C18;
/// `"ICON"`
const STR_ICON: usize = 0x866C54;
/// `"%d"`
const STR_FMT_D: usize = 0x859508;
/// `"%d-%d"`
const STR_FMT_D_D: usize = 0x866BBC;

// .rdata float constants (bits)
/// `1/640` (0x859520)
const K_1_640: u32 = 0x3ACC_CCCD;
/// `1/448` (0x859524)
const K_1_448: u32 = 0x3B12_4925;
/// `0.5` (0x858B8C)
const K_HALF: u32 = 0x3F00_0000;
/// `47.0` (0x866C4C)
const K_47: u32 = 0x423C_0000;
/// `58.0` (0x866C50)
const K_58: u32 = 0x4268_0000;
/// `0.02` (0x858B38)
const K_0_02: u32 = 0x3CA3_D70A;
/// `-1000.0` (0x859948)
const K_M1000: u32 = 0xC47A_0000;
/// `1000.0` (0x858C4C)
const K_1000: u32 = 0x447A_0000;
/// `0.001` (0x858CDC)
const K_0_001: u32 = 0x3A83_126F;
/// `10000.0` (0x859AA4)
const K_10000: u32 = 0x461C_4000;
/// `255.0` (0x859AAC)
const K_255: u32 = 0x437F_0000;
/// `1.21` (0x866C64)
const K_1_21: u32 = 0x3F9A_E148;
/// `0.605` (0x866C60)
const K_0_605: u32 = 0x3F1A_E148;
/// `29.0` (0x863210)
const K_29: u32 = 0x41E8_0000;
/// `114.0` (0x866C5C)
const K_114: u32 = 0x42E4_0000;
/// `101.0` (0x866BB4)
const K_101: u32 = 0x42CA_0000;
/// `12.0` (0x858CCC)
const K_12: u32 = 0x4140_0000;
/// `0.8` (0x858C98)
const K_0_8: u32 = 0x3F4C_CCCD;
/// `1.2` (0x858F08)
const K_1_2: u32 = 0x3F99_999A;
/// `1.0` (0x858624)
const K_1: u32 = 0x3F80_0000;
/// `100.0` (0x858628)
const K_100: u32 = 0x42C8_0000;
/// `9.0` (0x85EED4)
const K_9: u32 = 0x4110_0000;
/// `62.0` (0x86503C)
const K_62: u32 = 0x4278_0000;
/// `109.0` (0x866BB8)
const K_109: u32 = 0x42DA_0000;
/// `448.0` (0x8652B0)
const K_448: u32 = 0x43E0_0000;
/// `1.3` (0x859918)
const K_1_3: u32 = 0x3FA6_6666;
/// `2^32` (0x858C54)
const K_2_32: u32 = 0x4F80_0000;
/// `120.0` (0x858BB0)
const K_120: u32 = 0x42F0_0000;
/// `115.0` (0x863194)
const K_115: u32 = 0x42E6_0000;
/// `30.0` (0x858CA4)
const K_30: u32 = 0x41F0_0000;
/// `0.4` (0x858EE8)
const K_0_4: u32 = 0x3ECC_CCCD;
/// `2.1` (0x859974)
const K_2_1: u32 = 0x4006_6666;
/// `15.0` (0x858B48)
const K_15: u32 = 0x4170_0000;
/// `590.0` (0x866C28)
const K_590: u32 = 0x4413_8000;
/// `1.8` as a double (0x866C38, `fmul qword`)
const D_1_8: u64 = 0x3FFC_CCCC_CCCC_CCCD;
/// `1.3` as a double (0x866C30, `fmul qword`)
const D_1_3: u64 = 0x3FF4_CCCC_CCCC_CCCD;
/// `54.0` (0x858C94)
const K_54: u32 = 0x4258_0000;
/// `85.0` (0x85989C)
const K_85: u32 = 0x42AA_0000;
/// `64.0` (0x859A44)
const K_64: u32 = 0x4280_0000;
/// `60.0` (0x858B34)
const K_60: u32 = 0x4270_0000;
/// `0.58` (0x866C24)
const K_0_58: u32 = 0x3F14_7AE1;
/// `80.0` (0x858A14)
const K_80: u32 = 0x42A0_0000;
/// `1.22` (0x866C20)
const K_1_22: u32 = 0x3F9C_28F6;
/// `105.0` (0x866C1C)
const K_105: u32 = 0x42D2_0000;
/// `8.0` (0x859000)
const K_8: u32 = 0x4100_0000;
/// `140.0` (0x865374)
const K_140: u32 = 0x430C_0000;
/// `3000.0` (0x859A94)
const K_3000: u32 = 0x453B_8000;
/// `1.5` (0x858CE8)
const K_1_5: u32 = 0x3FC0_0000;
/// `32.0` (0x85950C)
const K_32: u32 = 0x4200_0000;
/// `1.9` (0x866C0C)
const K_1_9: u32 = 0x3FF3_3333;
/// `180.0` (0x85A994)
const K_180: u32 = 0x4334_0000;
/// `1.35` (0x866C48)
const K_1_35: u32 = 0x3FAC_CCCD;
/// `0.6` (0x858CC8)
const K_0_6: u32 = 0x3F19_999A;
/// `350.0` (0x858A4C)
const K_350: u32 = 0x43AF_0000;
/// `1.8` (0x859BB4)
const K_1_8: u32 = 0x3FE6_6666;
/// `500.0` (0x858B58)
const K_500: u32 = 0x43FA_0000;
/// `155.0` (0x859990)
const K_155: u32 = 0x431B_0000;
/// `1/6` (0x85F0A0)
const K_1_6: u32 = 0x3E2A_AAAB;
/// `0.2` (0x858CC4)
const K_0_2: u32 = 0x3E4C_CCCD;
/// `-380.0` (0x866C44)
const K_M380: u32 = 0xC3BE_0000;
/// `217.0` (0x866C40)
const K_217: u32 = 0x4359_0000;
/// `148.0` (0x859518)
const K_148: u32 = 0x4314_0000;
/// `72.0` (0x858C90)
const K_72: u32 = 0x4290_0000;
/// `90.0` (0x85991C)
const K_90: u32 = 0x42B4_0000;
/// `61.0` (0x866C10)
const K_61: u32 = 0x4274_0000;
/// `0.01` (0x858C58)
const K_0_01: u32 = 0x3C23_D70A;
/// `6.0` (0x858B44)
const K_6: u32 = 0x40C0_0000;
/// `34.0` (0x86322C)
const K_34: u32 = 0x4208_0000;
/// `28.0` (0x859AD8)
const K_28: u32 = 0x41E0_0000;
/// `1.1` (0x858F14)
const K_1_1: u32 = 0x3F8C_CCCD;
/// `0.52` (0x8665BC)
const K_0_52: u32 = 0x3F05_1EB8;
/// `75.0` (0x858C8C)
const K_75: u32 = 0x4296_0000;
/// `150.0` (0x858A28)
const K_150: u32 = 0x4316_0000;
/// `65.0` (0x866C14)
const K_65: u32 = 0x4282_0000;
/// `0.7` (0x858CB0)
const K_0_7: u32 = 0x3F33_3333;
/// `0.3` (0x858C24)
const K_0_3: u32 = 0x3E99_999A;
/// `640.0` (0x85951C)
const K_640: u32 = 0x4420_0000;
/// `9999.0` (0x863C20)
const K_9999: u32 = 0x461C_3C00;
/// `pi/4` (0x859AB0)
const K_PI_4: u32 = 0x3F49_0FDB;
/// `pi/2` (0x858FE4)
const K_PI_2: u32 = 0x3FC9_0FDB;
/// `0.0` (0x858B50)
const K_ZERO: u32 = 0;
/// `2.0` (0x858CA0)
const K_2: u32 = 0x4000_0000;
/// `4.0` (0x858B90)
const K_4: u32 = 0x4080_0000;
/// `5.0` (0x858C80)
const K_5: u32 = 0x40A0_0000;
/// `10.0` (0x85862C)
const K_10: u32 = 0x4120_0000;
/// `18.0` (0x859008)
const K_18: u32 = 0x4190_0000;
/// `20.0` (0x858BA4)
const K_20: u32 = 0x41A0_0000;
/// `25.0` (0x858FE8)
const K_25: u32 = 0x41C8_0000;
/// `40.0` (0x858A10)
const K_40: u32 = 0x4220_0000;
/// `76.0` (0x866B74)
const K_76: u32 = 0x4298_0000;
/// `94.0` (0x866B78)
const K_94: u32 = 0x42BC_0000;
/// `104.0` (0x866B70)
const K_104: u32 = 0x42D0_0000;
/// `200.0` (0x858A48)
const K_200: u32 = 0x4348_0000;
/// `0.005` (0x858B4C)
const K_0_005: u32 = 0x3BA3_D70A;
/// `1/950` (0x866C08)
const K_1_950: u32 = 0x3A89_F874;
/// `1.0` (pushed as an immediate)
const ONE: u32 = 0x3F80_0000;

fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}

fn b(addr: usize) -> u8 {
    g::<u8>(addr).read()
}

fn max_w() -> X87 {
    xi(g::<i32>(RS_MAX_WIDTH).read())
}

fn max_h() -> X87 {
    xi(g::<i32>(RS_MAX_HEIGHT).read())
}

orig! {
    /// `RwRenderStateSet` through `RwEngineInstance` (`mov r, [0xC97B24]; call [r+0x20]`)
    fn rw_render_state_set(state: u32, value: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;

    /// `CHud::DrawCrossHairs()` (0x58E020)
    fn draw_cross_hairs_call() = cdecl 0x58E020 => crate::hud_draw3::draw_cross_hairs;
    /// `CHud::DrawPlayerInfo()` (0x58EAF0)
    fn draw_player_info_call() = cdecl 0x58EAF0 => crate::hud_draw3::draw_player_info;
    /// `CHud::DrawWanted()` (0x58D9A0)
    fn draw_wanted_call() = cdecl 0x58D9A0 => draw_wanted;
    /// `CHud::DrawVehicleName()` (0x58AEA0)
    fn draw_vehicle_name_call() = cdecl 0x58AEA0 => draw_vehicle_name;
    /// `CHud::DrawMissionTimers()` (0x58B180)
    fn draw_mission_timers_call() = cdecl 0x58B180 => draw_mission_timers;
    /// `CHud::DrawVitalStats()` (0x589650)
    fn draw_vital_stats_call() = cdecl 0x589650 => crate::hud_draw3::draw_vital_stats;
    /// `CHud::DrawRadar()` (0x58A330)
    fn draw_radar_call() = cdecl 0x58A330 => draw_radar;
    /// `CHud::DrawTripSkip()` (0x58A160)
    fn draw_trip_skip_call() = cdecl 0x58A160 => draw_trip_skip;
    /// `CHud::SetHelpMessage(const GxtChar*, bool quick, bool permanent, bool addToBrief)` (0x588BE0)
    fn set_help_message_call(text: GPtr<u8>, quick: u8, permanent: u8, brief: u8) = cdecl 0x588BE0 => set_help_message;
    /// `CHud::DrawScriptText(bool beforeFade)` (0x58C080)
    fn draw_script_text_call(before_fade: u32) = cdecl 0x58C080 => draw_script_text;
    /// `CHud::DrawSubtitles()` (0x58C250)
    fn draw_subtitles_call() = cdecl 0x58C250 => draw_subtitles;
    /// `CHud::DrawHelpText()` (0x58B6E0)
    fn draw_help_text_call() = cdecl 0x58B6E0 => draw_help_text;
    /// `CHud::DrawOddJobMessage(bool)` (0x58CC80)
    fn draw_odd_job_message_call(v: u8) = cdecl 0x58CC80 => draw_odd_job_message;
    /// `CHud::DrawSuccessFailedMessage()` (0x58C6A0)
    fn draw_success_failed_message_call() = cdecl 0x58C6A0 => draw_success_failed_message;
    /// `CHud::DrawBustedWastedMessage()` (0x58CA50)
    fn draw_busted_wasted_message_call() = cdecl 0x58CA50 => draw_busted_wasted_message;
    /// `CHud::DrawAreaName()` (0x58AA50)
    fn draw_area_name_call() = cdecl 0x58AA50 => draw_area_name;
    /// `CHud::DrawMissionTitle()` (0x58D240)
    fn draw_mission_title_call() = cdecl 0x58D240 => draw_mission_title;

    /// `CRadar::DrawMap()` (0x586B00)
    fn radar_draw_map() = cdecl 0x586B00 => unported;
    /// `CRadar::DrawBlips()` (0x588050)
    fn radar_draw_blips() = cdecl 0x588050 => crate::radar_draw2::draw_blips;
    /// `eh vector constructor iterator(ptr, size, count, ctor)` (0x403D60, stdcall)
    fn vector_ctor_iterator(p: GPtr<u8>, size: u32, count: u32, ctor: u32) = stdcall 0x403D60 => unported;
    /// `std::min<float>(a, b)` (0x404330): ST0 = `a` when `a < b`, else `b`
    fn min_f_st0(a: f32, bb: f32) -> f64 = cdecl 0x404330 => unported;
    /// `CSprite2d::Draw(x1, y1, x2, y2, x3, y3, x4, y4, const CRGBA&)` (0x728520)
    fn sprite2d_draw_quad(this: GPtr<u8>, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32, colour: GPtr<u8>) = thiscall 0x728520 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    fn sprite2d_draw_rect_static(rect: GPtr<u8>, colour: GPtr<u8>) = cdecl 0x727B60 => unported;

    /// `FindPlayerWanted(int32)` (0x56E230)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => sa_world::world::cworld::find_player_wanted;
    /// `CHud::GetYPosBasedOnHealth(uint8, float, int8)` (0x588B60, ST0)
    fn get_y_pos_based_on_health_st0(player: u8, y: f32, offset: i8) -> f64 = cdecl 0x588B60 => get_y_pos_based_on_health_st0_rust;

    /// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0, ST0)
    fn stats_get_fat_and_muscle_modifier_st0(stat: u32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CSprite2d::DrawBarChart(x, y, width, height, progress, progressAdd, drawPercentage, drawBlackBorder,
    /// CRGBA colour, CRGBA addColour)` (0x728640)
    fn sprite2d_draw_bar_chart(px: f32, py: f32, w: u32, h: u32, progress: f32, add: u32, pct: u32, border: u32, colour: u32, add_colour: u32) = cdecl 0x728640 => unported;
    /// `CHudColours::GetRGB(CRGBA* out, uint8 colour)` (0x58FEA0)
    fn hud_colours_get_rgb(this: GPtr<u8>, out: GPtr<u8>, colour: u32) -> GPtr<u8> = thiscall 0x58FEA0 => crate::hud::get_rgb;

    /// `CMessages::StringCopy(GxtChar* dst, const GxtChar* src, uint16 len)` (0x69DB70)
    fn msg_string_copy(dst: GPtr<u8>, src: GPtr<u8>, len: u32) = cdecl 0x69DB70 => crate::messages::string_copy;
    /// `CMessages::StringCompare(const GxtChar*, const GxtChar*, uint16)` (0x69DBD0)
    fn msg_string_compare(a: GPtr<u8>, bb: GPtr<u8>, len: u32) -> u8 = cdecl 0x69DBD0 => crate::messages::string_compare;
    /// `CMessages::GetStringLength(const GxtChar*)` (0x69DB50)
    fn msg_get_string_length(s: GPtr<u8>) -> u32 = cdecl 0x69DB50 => crate::messages::get_string_length;
    /// `CMessages::InsertPlayerControlKeysInString(GxtChar*)` (0x69E160)
    fn msg_insert_player_control_keys(s: GPtr<u8>) = cdecl 0x69E160 => crate::messages::insert_player_control_keys_in_string;
    /// `CMessages::AddToPreviousBriefArray(text, n1..n6, string)` (0x69DD50)
    fn msg_add_to_previous_brief_array(text: GPtr<u8>, n1: i32, n2: i32, n3: i32, n4: i32, n5: i32, n6: i32, st: GPtr<u8>) = cdecl 0x69DD50 => crate::messages::add_to_previous_brief_array;
    /// `CMessages::InsertNumberInString(text, n1..n6, out)` (0x69DE90)
    fn msg_insert_number_in_string(text: GPtr<u8>, n1: i32, n2: i32, n3: i32, n4: i32, n5: i32, n6: i32, out: GPtr<u8>) = cdecl 0x69DE90 => crate::messages::insert_number_in_string;
    /// `CSprite2d::Delete()` (0x727240)
    fn sprite2d_delete(this: GPtr<u8>) = thiscall 0x727240 => unported;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => sa_stream::txdstore::find_txd_slot_name;
    /// `CTxdStore::RemoveTxdSlot(int32)` (0x731CD0)
    fn remove_txd_slot(slot: i32) = cdecl 0x731CD0 => sa_stream::txdstore::remove_txd_slot;

    /// `CTheScripts::DrawScriptSpritesAndRectangles(bool)` (0x464980)
    fn draw_script_sprites_and_rectangles(before_fade: u32) = cdecl 0x464980 => unported;
    /// `CFont::SetJustify(bool)` (0x719600)
    fn font_set_justify(v: u32) = cdecl 0x719600 => crate::font::set_justify;
    /// `CFont::SetWrapx(float)` (0x7194D0)
    fn font_set_wrapx(v: u32) = cdecl 0x7194D0 => crate::font::set_wrapx;
    /// `CFont::SetBackgroundColor(CRGBA)` (0x7195E0)
    fn font_set_background_color(c: u32) = cdecl 0x7195E0 => crate::font::set_background_color;
    /// `CFont::SetDropShadowPosition(int8)` (0x719570)
    fn font_set_drop_shadow_position(v: u32) = cdecl 0x719570 => crate::font::set_drop_shadow_position;
    /// `CFont::SetRightJustifyWrap(float)` (0x7194F0)
    fn font_set_right_justify_wrap(v: u32) = cdecl 0x7194F0 => crate::font::set_right_justify_wrap;
    /// `CFont::PrintStringFromBottom(float x, float y, const GxtChar*)` (0x71A820)
    fn font_print_string_from_bottom(px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A820 => unported;

    /// `CFont::GetNumberLines(float x, float y, const GxtChar*)` (0x71A5E0; the caller tests AX)
    fn font_get_number_lines(px: f32, py: f32, text: GPtr<u8>) -> u32 = cdecl 0x71A5E0 => crate::font::get_number_lines;

    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390)
    fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CFont::SetScaleForCurrentLanguage(float w, float h)` (0x7193A0)
    fn font_set_scale_for_current_language(w: f32, h: u32) = cdecl 0x7193A0 => crate::font::set_scale_for_current_language;

    /// `CFont::SetSlant(float)` (0x719420)
    fn font_set_slant(v: u32) = cdecl 0x719420 => crate::font::set_slant;

    /// `CCamera::GetFading()` (0x50ADE0)
    fn camera_get_fading(this: GPtr<u8>) -> u8 = thiscall 0x50ADE0 => unported;
    /// `CCamera::GetScreenFadeStatus()` (0x50AE20)
    fn camera_get_screen_fade_status(this: GPtr<u8>) -> u32 = thiscall 0x50AE20 => unported;

    /// CRT `atol` (0x82258E)
    fn crt_atol(s: GPtr<u8>) -> i32 = cdecl 0x82258E => unported;

    /// `CTxdStore::AddTxdSlot(const char*)` (0x731C80)
    fn add_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731C80 => sa_stream::txdstore::add_txd_slot;
    /// `CTxdStore::LoadTxd(int32, const char*)` (0x7320B0)
    fn load_txd(slot: i32, name: GPtr<u8>) -> u8 = cdecl 0x7320B0 => sa_stream::txdstore::load_txd_filename;
    /// `CTxdStore::AddRef(int32)` (0x731A00)
    fn txd_add_ref(slot: i32) = cdecl 0x731A00 => sa_stream::txdstore::add_ref;
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    fn push_current_txd() = cdecl 0x7316A0 => sa_stream::txdstore::push_current_txd;
    /// `CTxdStore::SetCurrentTxd(int32)` (0x7319C0)
    fn set_current_txd(slot: i32) = cdecl 0x7319C0 => sa_stream::txdstore::set_current_txd;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    fn pop_current_txd() = cdecl 0x7316B0 => sa_stream::txdstore::pop_current_txd;
    /// `CSprite2d::SetTexture(const char* name, const char* mask)` (0x7272B0)
    fn sprite2d_set_texture(this: GPtr<u8>, name: GPtr<u8>, mask: GPtr<u8>) = thiscall 0x7272B0 => unported;
    /// `CHud::ReInitialise()` (0x588880)
    fn re_initialise_call() = cdecl 0x588880 => re_initialise;
    /// MSVC `eh vector constructor iterator(ptr, size, count, ctor, dtor)` (0x821D30, stdcall)
    fn eh_vector_ctor_iterator(p: GPtr<u8>, size: u32, count: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;

    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float volume, float speed)` (0x506EA0)
    fn audio_report_frontend_event(this: GPtr<u8>, event: u32, volume: u32, speed: u32) = thiscall 0x506EA0 => unported;
    /// `CFont::SetAlphaFade(float)` (0x719500)
    fn font_set_alpha_fade(v: u32) = cdecl 0x719500 => crate::font::set_alpha_fade;
    /// `CFont::GetStringWidth(const GxtChar*, bool full, bool scriptText)` (0x71A0E0, ST0)
    fn font_get_string_width_st0(text: GPtr<u8>, full: u32, script: u32) -> f64 = cdecl 0x71A0E0 => unported;
    /// `CStats::GetStatValue(eStats)` (0x558E40, ST0)
    fn stats_get_stat_value_st0(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CPedGroupMembership::CountMembersExcludingLeader()` (0x5F6AA0)
    fn group_count_members_excluding_leader(this: GPtr<u8>) -> i32 = thiscall 0x5F6AA0 => unported;
    /// `std::max<float>(a, b)` (0x420800): ST0 = `a` when `a > b`, else `b`
    fn max_f_st0(a: f32, bb: f32) -> f64 = cdecl 0x420800 => unported;

    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetDisplayVitalStats(CPed*)` (0x5408B0; the caller tests AX)
    fn pad_get_display_vital_stats(pad: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x5408B0 => unported;
    /// `CGameLogic::SkipCanBeActivated()` (0x4415C0)
    fn skip_can_be_activated() -> u8 = cdecl 0x4415C0 => unported;
    /// `CText::Get(const char*)` (0x6A0050)
    fn text_get(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => crate::text::text_get;
    /// `CRadar::Draw3dMarkers()` (0x585BF0)
    fn radar_draw_3d_markers() = cdecl 0x585BF0 => crate::radar_draw2::draw_3d_markers;
    /// `CMenuSystem::Process(uint8 menu)` (0x582630)
    fn menu_system_process(menu: i32) = cdecl 0x582630 => unported;
    /// `CDarkel::FrenzyOnGoing()` (0x43D1F0)
    fn darkel_frenzy_on_going() -> u8 = cdecl 0x43D1F0 => unported;

    /// `CPed::GetWeaponSkill()` (0x5E6580)
    fn ped_get_weapon_skill(ped: GPtr<u8>) -> u32 = thiscall 0x5E6580 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn get_weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CKeyGen::AppendStringToKey(uint32, const char*)` (0x53CF70)
    fn append_string_to_key(key: u32, s: GPtr<u8>) -> u32 = cdecl 0x53CF70 => sa_core::keygen::append_string_to_key;
    /// `RwTexDictionaryFindHashNamedTexture(RwTexDictionary*, uint32)` (0x734E50)
    fn rw_tex_dictionary_find_hash_named_texture(dict: GPtr<u8>, hash: u32) -> GPtr<u8> = cdecl 0x734E50 => unported;
    /// `CSprite::RenderOneXLUSprite(x, y, z, halfW, halfH, r, g, b, intensity, rhw, a, uDir, vDir)` (0x70D000)
    fn sprite_render_one_xlu_sprite(px: f32, py: f32, pz: u32, hw: f32, hh: f32, r: u32, gg: u32, bb: u32, intensity: u32, rhw: u32, a: u32, u_dir: u32, v_dir: u32) = cdecl 0x70D000 => unported;
    /// `CSprite2d::Draw(const CRect&, const CRGBA&)` (0x728350)
    fn sprite2d_draw_rect(this: GPtr<u8>, rect: GPtr<u8>, colour: GPtr<u8>) = thiscall 0x728350 => unported;
    /// `CHudColours::GetRGBA(CRGBA* out, uint8 colour, uint8 alpha)` (0x58FEE0)
    fn hud_colours_get_rgba(this: GPtr<u8>, out: GPtr<u8>, colour: u32, alpha: u32) -> GPtr<u8> = thiscall 0x58FEE0 => crate::hud::get_rgba;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0)
    fn rgba_ctor(this: GPtr<u8>, r: u32, gg: u32, bb: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;

    /// CRT `sprintf(buf, fmt, a)` (0x821BB5)
    fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: i32) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `sprintf(buf, fmt, a, b)` (0x821BB5)
    fn sprintf2(buf: GPtr<u8>, fmt: GPtr<u8>, a: i32, bb: i32) -> i32 = cdecl 0x821BB5 => unported;
    /// `AsciiToGxtChar(const char*, GxtChar*)` (0x718600)
    fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => unported;

    /// `CFont::SetBackground(bool, bool)` (0x7195C0)
    fn font_set_background(on: u32, outline: u32) = cdecl 0x7195C0 => crate::font::set_background;
    /// `CFont::SetScale(float w, float h)` (0x719380)
    fn font_set_scale(w: u32, h: u32) = cdecl 0x719380 => crate::font::set_scale;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610)
    fn font_set_orientation(v: u32) = cdecl 0x719610 => crate::font::set_orientation;
    /// `CFont::SetCentreSize(float)` (0x7194E0)
    fn font_set_centre_size(v: u32) = cdecl 0x7194E0 => crate::font::set_centre_size;
    /// `CFont::SetProportional(bool)` (0x7195B0)
    fn font_set_proportional(v: u32) = cdecl 0x7195B0 => crate::font::set_proportional;
    /// `CFont::SetEdge(int8)` (0x719590)
    fn font_set_edge(v: u32) = cdecl 0x719590 => crate::font::set_edge;
    /// `CFont::SetDropColor(CRGBA)` (0x719510)
    fn font_set_drop_color(c: u32) = cdecl 0x719510 => crate::font::set_drop_color;
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490)
    fn font_set_font_style(v: u32) = cdecl 0x719490 => crate::font::set_font_style;
    /// `CFont::SetColor(CRGBA)` (0x719430)
    fn font_set_color(c: u32) = cdecl 0x719430 => crate::font::set_color;
    /// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71A700)
    fn font_print_string(px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A700 => unported;
}

/// A local buffer as a game pointer (the original's stack temporaries).
fn local<const N: usize>(buf: &mut [u8; N]) -> GPtr<u8> {
    GPtr::from_raw(buf.as_mut_ptr())
}

/// A `CRect` {left, bottom, right, top} as the original lays it out on its stack.
fn rect_bytes(l: f32, bt: f32, r: f32, t: f32) -> [u8; 16] {
    let mut o = [0u8; 16];
    o[0..4].copy_from_slice(&l.to_le_bytes());
    o[4..8].copy_from_slice(&bt.to_le_bytes());
    o[8..12].copy_from_slice(&r.to_le_bytes());
    o[12..16].copy_from_slice(&t.to_le_bytes());
    o
}

fn fpv() -> GPtr<u8> {
    sa_world::world::common::find_player_vehicle(-1, 0)
}

/// The ped's active `CWeapon` (`ped + 0x5A0 + m_nActiveWeaponSlot * 0x1C`, slot = signed byte +0x718).
fn active_weapon(ped: GPtr<u8>) -> GPtr<u8> {
    let slot = i32::from(rd::<i8>(ped, 0x718));
    ped.byte_add(0x5A0 + slot.wrapping_mul(0x1C) as isize)
}

/// `CHud::Draw()` (0x58FAE0): render states, then the HUD elements in the original order.
#[hook(addr = 0x58FAE0, conv = cdecl, name = "CHud::Draw", expect = "80 3D 88 30 A4 00 01")]
pub fn draw() {
    if b(REPLAY_MODE) == 1
        || b(TAKE_PHOTO) != 0
        || b(MENU_ACTIVATE_NEXT_FRAME) != 0
        || g::<u32>(CINEY_CAM_FRAME).read() == g::<u32>(FRAME_COUNTER).read()
    {
        return;
    }
    rw_render_state_set(9, 1);
    rw_render_state_set(0xC, 0);
    rw_render_state_set(0xE, 0);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 6);
    rw_render_state_set(0xC, 1);
    rw_render_state_set(9, 2);
    rw_render_state_set(2, 3);
    rw_render_state_set(1, 0);
    rw_render_state_set(7, 1);
    rw_render_state_set(0x1D, 5);
    rw_render_state_set(0x1E, 0);

    if b(WIDE_SCREEN_ON) == 0 {
        draw_cross_hairs_call();
        if b(MENU_HUD_ON) != 0 && b(SCRIPTS_DISPLAY_HUD) != 0 {
            draw_player_info_call();
            draw_wanted_call();
        }
        if b(DONT_DISPLAY_VEHICLE_NAME) == 0 {
            draw_vehicle_name_call();
        }
        draw_mission_timers_call();
    }

    if b(DONT_DISPLAY_RADAR) == 0 && b(WIDE_SCREEN_ON) == 0 {
        let ped = sa_world::world::common::find_player_ped(-1);
        let pad = get_pad(0);
        if pad_get_display_vital_stats(pad, ped) as u16 != 0
            && sa_world::world::common::find_player_vehicle(-1, 0).is_null()
        {
            DRAWING_VITAL_STATS.set(1);
            draw_vital_stats_call();
        } else {
            DRAWING_VITAL_STATS.set(0);
            draw_radar_call();
        }
        if skip_can_be_activated() != 0 && DRAWING_VITAL_STATS.get() == 0 {
            draw_trip_skip_call();
            if HELP_TRIP_SKIP_SHOWN.get() == 0 {
                let text = text_get(g(THE_TEXT), g(STR_SKIP_1));
                set_help_message_call(text, 1, 0, 0);
                HELP_TRIP_SKIP_SHOWN.set(1);
            }
        } else {
            HELP_TRIP_SKIP_SHOWN.set(0);
        }
    }

    if b(DRAW_3D_MARKERS) == 1 && b(WIDE_SCREEN_ON) == 0 {
        radar_draw_3d_markers();
    }

    if b(USER_PAUSE) != 0 {
        return;
    }
    if b(BIG_MESSAGE_0) == 0 {
        if b(NUM_MENUS_IN_USE) != 0 {
            menu_system_process(-99);
        }
        draw_script_text_call(1);
    }
    if b(SCRIPTS_SUBTITLES_BEFORE_FADE) != 0 {
        draw_subtitles_call();
    }
    draw_help_text_call();
    draw_odd_job_message_call(1);
    draw_success_failed_message_call();
    draw_busted_wasted_message_call();
}

/// `CHud::DrawAfterFade()` (0x58D490). The area name is skipped while the player's vehicle is a plane or a
/// heli (`m_nVehicleSubType` +0x594 == 4 or 3); `FindPlayerVehicle(-1, false)` is called again for each test,
/// as the bytes.
#[hook(addr = 0x58D490, conv = cdecl, name = "CHud::DrawAfterFade", expect = "A1 24 7B C9 00")]
pub fn draw_after_fade() {
    rw_render_state_set(9, 1);
    rw_render_state_set(2, 3);
    rw_render_state_set(0xC, 0);
    if b(USER_PAUSE) != 0 || b(REPLAY_MODE) == 1 || b(TAKE_PHOTO) != 0 {
        return;
    }
    let fpv = || sa_world::world::common::find_player_vehicle(-1, 0);
    let skip_area = !fpv().is_null() && (rd::<u32>(fpv(), 0x594) == 4 || rd::<u32>(fpv(), 0x594) == 3);
    if !skip_area
        && b(CUTSCENE_PROCESSING) == 0
        && b(MENU_ACTIVE) == 0
        && b(WIDE_SCREEN_ON) == 0
        && b(DONT_DISPLAY_AREA_NAME) == 0
    {
        draw_area_name_call();
    }
    if b(BIG_MESSAGE_0) == 0 {
        draw_script_text_call(0);
    }
    if b(SCRIPTS_SUBTITLES_BEFORE_FADE) == 0 {
        draw_subtitles_call();
    }
    draw_mission_title_call();
    draw_odd_job_message_call(0);
}

/// `CHud::DrawWeaponIcon(CPed*, int32 x, int32 y, float alpha)` (0x58D7D0). The txd pool slot is read
/// without a null check of the entry pointer: a free slot gives entry 0 and the original reads `[0]` (a
/// fault), as the port does.
#[hook(addr = 0x58D7D0, conv = cdecl, name = "CHud::DrawWeaponIcon", expect = "A1 24 7B C9 00")]
pub fn draw_weapon_icon(ped: GPtr<u8>, px: i32, py: i32, alpha: f32) {
    rw_render_state_set(9, 2);
    let ty = rd::<u32>(active_weapon(ped), 0);
    let model = rd::<i32>(get_weapon_info(ty, 1), 0xC);
    if model <= 0 {
        // Fist: CRect {left, bottom, right, top}
        let x0 = xi(px);
        let y0 = xi(py);
        let right = max_w().fmul_m(k(K_1_640)).fmul_m(k(K_47)).fadd(x0);
        let bottom = max_h().fmul_m(k(K_1_448)).fmul_m(k(K_58)).fadd(y0);
        let mut rect = [0u8; 16];
        rect[0..4].copy_from_slice(&f32_of(x0).to_le_bytes());
        rect[4..8].copy_from_slice(&f32_of(bottom).to_le_bytes());
        rect[8..12].copy_from_slice(&f32_of(right).to_le_bytes());
        rect[12..16].copy_from_slice(&f32_of(y0).to_le_bytes());
        let a = ftol32(x(alpha)) as u32;
        let mut rgba = [0u8; 4];
        let c = rgba_ctor(local(&mut rgba), 0xFF, 0xFF, 0xFF, a);
        sprite2d_draw_rect(g(SPRITES), local(&mut rect), c);
        return;
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + model as usize * 4).read();
    let txd = i32::from(rd::<i16>(mi, 0xA));
    let pool = g::<GPtr<u8>>(TXD_POOL).read();
    let flags = rd::<GPtr<u8>>(pool, 4);
    let entry = if rd::<i8>(flags, txd as isize) < 0 {
        GPtr::<u8>::NULL
    } else {
        rd::<GPtr<u8>>(pool, 0).byte_add(txd as isize * 12)
    };
    let dict = rd::<GPtr<u8>>(entry, 0);
    if dict.is_null() {
        return;
    }
    let key = append_string_to_key(rd::<u32>(mi, 4), g(STR_ICON));
    let tex = rw_tex_dictionary_find_hash_named_texture(dict, key);
    if tex.is_null() {
        return;
    }
    rw_render_state_set(6, 0);
    rw_render_state_set(1, rd::<u32>(tex, 0));
    let hh = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_58)).fmul_m(k(K_HALF)));
    let hw = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_47)).fmul_m(k(K_HALF)));
    let sy = f32_of(xi(py).fadd_m(hh));
    let sx = f32_of(xi(px).fadd_m(hw));
    sprite_render_one_xlu_sprite(sx, sy, ONE, hw, hh, 0xFF, 0xFF, 0xFF, 0xFF, ONE, 0xFF, 0, 0);
    rw_render_state_set(8, 0);
}

/// `CHud::DrawAmmo(CPed*, int32 x, int32 y, float alpha)` (0x5893B0). `"%d-%d"` of (total - clip, clip)
/// capped at 9999 (the flamethrower, type 37, shows both divided by 10); only the total for weapons with
/// a clip size outside 2..999. The text is printed only when `(float)(total - clip) < 9999`, no frenzy runs,
/// the weapon type is not 0, 40, 10..15 or 46, its fire type is not 5 and its slot is above 1. Font edge 0
/// on every exit after the setup.
#[hook(addr = 0x5893B0, conv = cdecl, name = "CHud::DrawAmmo", expect = "53 56 57 8B 7C 24 10")]
pub fn draw_ammo(ped: GPtr<u8>, px: i32, py: i32, alpha: f32) {
    let w = active_weapon(ped);
    let total = rd::<i32>(w, 0xC);
    let ty = rd::<u32>(w, 0);
    let skill = ped_get_weapon_skill(ped);
    let clip_size = i32::from(rd::<i16>(get_weapon_info(ty, skill), 0x20));
    if clip_size <= 1 || clip_size >= 1000 {
        sprintf1(g(G_STRING), g(STR_FMT_D), total);
    } else {
        let w = active_weapon(ped);
        let clip = rd::<i32>(w, 8);
        if rd::<u32>(w, 0) == 0x25 {
            let a = total.wrapping_sub(clip) / 10;
            sprintf2(g(G_STRING), g(STR_FMT_D_D), a.min(9999), clip / 10);
        } else {
            let a = total.wrapping_sub(clip);
            sprintf2(g(G_STRING), g(STR_FMT_D_D), a.min(9999), clip);
        }
    }
    ascii_to_gxt_char(g(G_STRING), g(G_GXT_STRING));
    font_set_background(0, 0);
    let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_0_7)));
    let wd = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_3)));
    font_set_scale(wd.to_bits(), h.to_bits());
    font_set_orientation(0);
    font_set_centre_size(f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_640))).to_bits());
    font_set_proportional(1);
    font_set_edge(1);
    let mut rgba = [0u8; 4];
    let c = rgba_ctor(local(&mut rgba), 0, 0, 0, 0xFF);
    font_set_drop_color(rd::<u32>(c, 0));
    font_set_font_style(1);
    let rest = total.wrapping_sub(rd::<i32>(active_weapon(ped), 8));
    let v = xi(rest);
    // `fld 9999; fcomp st1; test ah, 5; jnp skip`: skip when 9999 < v; then `fcomp 9999; test ah, 0x44;
    // jnp skip`: skip when v == 9999.
    let c1 = x(k(K_9999)).fcom(v);
    if !c1.test_parity(0x05) {
        font_set_edge(0);
        return;
    }
    if !v.fcom(x(k(K_9999))).test_parity(0x44) || darkel_frenzy_on_going() != 0 {
        font_set_edge(0);
        return;
    }
    let ty = rd::<u32>(active_weapon(ped), 0);
    if ty == 0 || ty == 0x28 || (ty as i32 >= 10 && (ty as i32) < 0x10) || ty == 0x2E {
        font_set_edge(0);
        return;
    }
    if rd::<u32>(get_weapon_info(ty, 1), 0) == 5 {
        font_set_edge(0);
        return;
    }
    let ty = rd::<u32>(active_weapon(ped), 0);
    if rd::<i32>(get_weapon_info(ty, 1), 0x14) <= 1 {
        font_set_edge(0);
        return;
    }
    let a = ftol32(x(alpha)) as u32;
    let mut out = [0u8; 4];
    let c = hud_colours_get_rgba(g(HUD_COLOUR), local(&mut out), 3, a);
    font_set_color(rd::<u32>(c, 0));
    let fy = f32_of(xi(py));
    let fx = f32_of(xi(px));
    font_print_string(fx, fy, g(G_GXT_STRING));
    font_set_edge(0);
}

/// Draws `sprite` over `rect` with `CRGBA(r, g, b, a)` (a stack `CRect` and `CRGBA`, `CSprite2d::Draw`).
fn draw_sprite_rect(sprite: usize, mut rect: [u8; 16], c: [u32; 4]) {
    let mut rgba = [0u8; 4];
    let cp = rgba_ctor(local(&mut rgba), c[0], c[1], c[2], c[3]);
    sprite2d_draw_rect(g(sprite), local(&mut rect), cp);
}

/// `CSprite2d::DrawRect(rect, CRGBA(r, g, b, a))` with stack temporaries.
fn draw_rect(mut rect: [u8; 16], c: [u32; 4]) {
    let mut rgba = [0u8; 4];
    let cp = rgba_ctor(local(&mut rgba), c[0], c[1], c[2], c[3]);
    sprite2d_draw_rect_static(local(&mut rect), cp);
}

/// `CHud::DrawRadar()` (0x58A330). Byte findings: the plane ring angle is `-atan2(-right.z, up.z) - pi/4`
/// plus `i * pi/2` per corner (gr: `PI - atan2`); the altimeter shows for planes **and helis** that are not
/// the Vortex (gr: the Vortex test only for helis), or with a parachute; the disc rects are the original's.
#[hook(addr = 0x58A330, conv = cdecl, name = "CHud::DrawRadar", expect = "A1 CC A7 96 00")]
pub fn draw_radar() {
    let ees = g::<u32>(EXIT_ENTER_STATE).read();
    if ees == 1 || ees == 2 || g::<u32>(MENU_RADAR_MODE).read() == 2 {
        return;
    }
    if g::<u16>(ITEM_TO_FLASH).read() == 8 && b(FRAME_COUNTER) & 8 == 0 {
        return;
    }
    rw_render_state_set(9, 2);
    rw_render_state_set(7, 1);
    radar_draw_map();
    if g::<u32>(MENU_RADAR_MODE).read() == 1 {
        radar_draw_blips();
        return;
    }

    // Plane radar ring.
    if !fpv().is_null() && rd::<u32>(fpv(), 0x594) == 4 && rd::<u16>(fpv(), 0x22) != 0x21B {
        let m1 = rd::<GPtr<u8>>(fpv(), 0x14);
        let m2 = rd::<GPtr<u8>>(fpv(), 0x14);
        let nrz = f32_of(x(rd::<f32>(m2, 8)).fchs());
        let atz = rd::<f32>(m1, 0x28);
        let mut verts = [0u8; 32];
        vector_ctor_iterator(local(&mut verts), 8, 4, 0x43E0E0);
        let a = x(nrz).fpatan(x(atz)).fchs().fsub_m(k(K_PI_4));
        let sx = max_w().fmul_m(k(K_1_640));
        let c78sx = x(k(K_94)).fmul(sx);
        let l8 = f32_of(sx.fmul_m(k(K_18)).fsubr(c78sx));
        let lc = f32_of(c78sx.fmul_m(k(K_HALF)));
        let x0 = sx.fmul_m(k(K_40));
        let mh = max_h();
        let sy = x(k(K_1_448)).fmul(mh);
        let l4 = f32_of(x(k(K_76)).fmul(sy));
        let l10 = f32_of(x(k(K_18)).fmul(sy).fsubr_m(l4));
        let y0 = mh.fsub(sy.fmul_m(k(K_104)));
        let hc = x(l4).fmul_m(k(K_HALF));
        let mut v = [0f32; 8];
        for i in 0..4 {
            let t = xi(i as i32).fmul_m(k(K_PI_2)).fadd(a);
            let sn = t.fsin();
            let cs = t.fcos();
            let vx = x(k(K_ZERO)).fmul(cs).fadd(sn).fmul_m(l8).fadd(x0).fadd_m(lc);
            let vy = sn.fmul_m(k(K_ZERO)).fsubr(cs).fmul_m(l10).fadd(hc).fadd(y0);
            v[i * 2] = f32_of(vx);
            v[i * 2 + 1] = f32_of(vy);
        }
        for (i, f) in v.iter().enumerate() {
            verts[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
        }
        let mut rgba = [0u8; 4];
        let cp = rgba_ctor(local(&mut rgba), 0xFF, 0xFF, 0xFF, 0xFF);
        sprite2d_draw_quad(g(SPRITE_RADAR_RING_PLANE), v[2], v[3], v[0], v[1], v[4], v[5], v[6], v[7], cp);
    }

    // Altimeter: planes and helis other than the Vortex, or a parachute in hand.
    let air = if fpv().is_null() {
        false
    } else if rd::<u32>(fpv(), 0x594) == 4 || rd::<u32>(fpv(), 0x594) == 3 {
        rd::<u16>(fpv(), 0x22) != 0x21B
    } else {
        false
    };
    if air || rd::<u32>(active_weapon(sa_world::world::common::find_player_ped(-1)), 0) == 0x2E {
        let sx = max_w().fmul_m(k(K_1_640));
        let x40 = x(k(K_40)).fmul(sx);
        let r0 = f32_of(sx.fmul_m(k(K_20)).fsubr(x40));
        let mh = max_h();
        let sy = x(k(K_1_448)).fmul(mh);
        let syf = f32_of(sy);
        let bf = f32_of(mh.fsub(sy.fmul_m(k(K_104))));
        let r2 = f32_of(sx.fmul_m(k(K_10)).fsubr(x40));
        let r1 = f32_of(x(syf).fmul_m(k(K_76)).fadd_m(bf));
        draw_rect(rect_bytes(r0, r1, r2, bf), [10, 10, 10, 100]);

        let ent = if !fpv().is_null() { fpv() } else { sa_world::world::common::find_player_ped(-1) };
        let m = rd::<GPtr<u8>>(ent, 0x14);
        let pos = if m.is_null() { ent.byte_add(4) } else { m.byte_add(0x30) };
        let z = x(rd::<f32>(pos, 8));
        let le = z.fcom(x(k(K_200))).test_any(0x41);
        let h76 = max_h().fmul_m(k(K_1_448)).fmul_m(k(K_76));
        let zk = z.fmul_m(k(if le { K_0_005 } else { K_1_950 }));
        let vv = f32_of(h76.fmul(zk));
        rw_render_state_set(1, 0);
        let sx = max_w().fmul_m(k(K_1_640));
        let x40f = f32_of(x(k(K_40)).fmul(sx));
        let r0 = f32_of(sx.fmul_m(k(K_25)).fsubr_m(x40f));
        let mh = max_h();
        let sy = x(k(K_1_448)).fmul(mh);
        let h76f = f32_of(x(k(K_76)).fmul(sy));
        let t = f32_of(sy.fmul_m(k(K_104)).fsubr(mh).fadd_m(h76f));
        let r3x = min_f(h76f, vv).fsubr_m(t);
        let r3 = f32_of(r3x);
        let r2 = f32_of(x(x40f).fsub_m(k(K_5)));
        let r1 = f32_of(r3x.fadd_m(k(K_2)));
        draw_rect(rect_bytes(r0, r1, r2, r3), [200, 200, 200, 200]);
    }

    let black = [0, 0, 0, 0xFF];
    // Disc 1
    {
        let sx = max_w().fmul_m(k(K_1_640));
        let x40 = x(k(K_40)).fmul(sx);
        let r0 = f32_of(sx.fmul_m(k(K_4)).fsubr(x40));
        let mh = max_h();
        let sy = x(k(K_1_448)).fmul(mh);
        let syf = f32_of(sy);
        let bf = f32_of(sy.fmul_m(k(K_104)).fsubr(mh));
        let r3 = f32_of(x(syf).fmul_m(k(K_4)).fsubr_m(bf));
        let r2 = f32_of(sx.fmul_m(k(K_94)).fmul_m(k(K_HALF)).fadd(x40));
        let r1 = f32_of(x(syf).fmul_m(k(K_76)).fmul_m(k(K_HALF)).fadd_m(bf));
        draw_sprite_rect(SPRITE_RADAR_DISC, rect_bytes(r0, r1, r2, r3), black);
    }
    // Disc 2
    {
        let sx = max_w().fmul_m(k(K_1_640));
        let x40 = x(k(K_40)).fmul(sx);
        let s78 = sx.fmul_m(k(K_94));
        let r0 = f32_of(sx.fmul_m(k(K_4)).fadd(x40).fadd(s78));
        let mh = max_h();
        let sy = x(k(K_1_448)).fmul(mh);
        let syf = f32_of(sy);
        let bf = f32_of(sy.fmul_m(k(K_104)).fsubr(mh));
        let r3 = f32_of(x(syf).fmul_m(k(K_4)).fsubr_m(bf));
        let r2 = f32_of(s78.fmul_m(k(K_HALF)).fadd(x40));
        let r1 = f32_of(x(syf).fmul_m(k(K_76)).fmul_m(k(K_HALF)).fadd_m(bf));
        draw_sprite_rect(SPRITE_RADAR_DISC, rect_bytes(r0, r1, r2, r3), black);
    }
    // Disc 3
    {
        let sx = max_w().fmul_m(k(K_1_640));
        let x40 = x(k(K_40)).fmul(sx);
        let r0 = f32_of(sx.fmul_m(k(K_4)).fsubr(x40));
        let mh = max_h();
        let sy = x(k(K_1_448)).fmul(mh);
        let bf = f32_of(x(k(K_104)).fmul(sy).fsubr(mh));
        let h76f = f32_of(x(k(K_76)).fmul(sy));
        let r3 = f32_of(sy.fmul_m(k(K_4)).fadd_m(bf).fadd_m(h76f));
        let r2 = f32_of(sx.fmul_m(k(K_94)).fmul_m(k(K_HALF)).fadd(x40));
        let r1 = f32_of(x(h76f).fmul_m(k(K_HALF)).fadd_m(bf));
        draw_sprite_rect(SPRITE_RADAR_DISC, rect_bytes(r0, r1, r2, r3), black);
    }
    // Disc 4
    {
        let sx = max_w().fmul_m(k(K_1_640));
        let x40 = x(k(K_40)).fmul(sx);
        let s78 = sx.fmul_m(k(K_94));
        let r0 = f32_of(sx.fmul_m(k(K_4)).fadd(x40).fadd(s78));
        let mh = max_h();
        let sy = x(k(K_1_448)).fmul(mh);
        let bf = f32_of(x(k(K_104)).fmul(sy).fsubr(mh));
        let h76f = f32_of(x(k(K_76)).fmul(sy));
        let r3 = f32_of(sy.fmul_m(k(K_4)).fadd_m(bf).fadd_m(h76f));
        let r2 = f32_of(s78.fmul_m(k(K_HALF)).fadd(x40));
        let r1 = f32_of(x(h76f).fmul_m(k(K_HALF)).fadd_m(bf));
        draw_sprite_rect(SPRITE_RADAR_DISC, rect_bytes(r0, r1, r2, r3), black);
    }
    radar_draw_blips();
}

/// `ftol(CTimer::ms_fTimeStep * 0.02 * c)`: the fade step the wanted stars use.
fn wanted_step(c: u32) -> i32 {
    ftol32(x(g::<f32>(TIME_STEP).read()).fmul_m(k(K_0_02)).fmul_m(k(c)))
}

/// `CHud::DrawWanted()` (0x58D9A0). The fade state machine (states 0..5 in `m_WantedState`), then six
/// stars right to left: lit stars (level > i) bright for 2 s after the level changed and then blinking
/// with `frame & 4`; stars of the level before parole (`+0x30`) flash in 0.8x colour 6; the others are
/// drawn dim (0.7 alpha, 1.2 scale). The lit star's Y is `GetYPosBasedOnHealth` inlined (`< 101`); the
/// alpha argument words carry the upper bytes of `m_WantedTimer` (`mov bl, al`), as the bytes.
#[hook(addr = 0x58D9A0, conv = cdecl, name = "CHud::DrawWanted", expect = "83 EC 4C 53 55")]
pub fn draw_wanted() {
    let level = rd::<i32>(find_player_wanted(-1), 0x2C);
    let parole = rd::<i32>(find_player_wanted(-1), 0x30);
    let mut timer = WANTED_TIMER.get();
    let mut fade = WANTED_FADE_TIMER.get();
    let mut state = WANTED_STATE.get();
    let mut l18 = timer;
    let mut alpha = x(k(K_255));
    let changed = LAST_WANTED.get() != level;
    if changed {
        match state {
            0 => {
                fade = 0;
                timer = 5;
                state = 2;
                l18 = 5;
            }
            1 | 3 => {
                timer = 5;
                state = 2;
                l18 = 5;
            }
            _ => {}
        }
    }
    if state != 0 && state != 5 {
        match state {
            1 => {
                fade = 1000;
                alpha = x(k(K_255));
                if !xi(l18).fcom(x(k(K_10000))).test_any(0x41) {
                    state = 3;
                    fade = 3000;
                }
            }
            2 => {
                fade = fade.wrapping_add(wanted_step(K_1000));
                if !xi(fade).fcom(x(k(K_1000))).test_any(0x41) {
                    fade = 1000;
                    state = 1;
                }
                alpha = xi(fade).fmul_m(k(K_0_001)).fmul_m(k(K_255));
            }
            3 => {
                fade = fade.wrapping_add(wanted_step(K_M1000));
                if !xi(fade).fcom(x(k(K_ZERO))).test_parity(0x05) {
                    fade = 0;
                    state = 0;
                }
                alpha = xi(fade).fmul_m(k(K_0_001)).fmul_m(k(K_255));
            }
            _ => {}
        }
        timer = timer.wrapping_add(wanted_step(K_1000));
    }
    let neg = !alpha.fcom(x(k(K_ZERO))).test_parity(0x05);
    WANTED_STATE.set(state);
    WANTED_TIMER.set(timer);
    WANTED_FADE_TIMER.set(fade);
    if neg {
        alpha = x(k(K_ZERO));
    } else if !alpha.fcom(x(k(K_255))).test_any(0x41) {
        alpha = x(k(K_255));
    }
    if changed {
        LAST_WANTED.set(level);
        WANTED_LEVEL_UNCHANGED.set(0);
    } else {
        WANTED_LEVEL_UNCHANGED.set(1);
    }
    let alpha_f = f32_of(alpha);
    if state == 0 {
        return;
    }
    font_set_background(0, 0);
    let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_21)));
    let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_605)));
    font_set_scale(w.to_bits(), h.to_bits());
    font_set_orientation(2);
    font_set_proportional(1);
    font_set_font_style(0);
    let mut star_ascii = [0u8; 0x14];
    star_ascii[0] = b']';
    let mut star = [0u8; 0x14];
    ascii_to_gxt_char(local(&mut star_ascii), local(&mut star));
    let mw = max_w();
    let mut px = f32_of(x(k(K_1_640)).fmul(mw).fmul_m(k(K_29)).fsubr(mw));
    if !((level > 0 && WANTED_LEVEL_UNCHANGED.get() != 0) || parole > 0) {
        return;
    }
    let ebx = (timer as u32 & 0xFFFF_FF00) | u32::from(ftol32(x(alpha_f)) as u8);
    for i in 0..6i32 {
        font_set_edge(1);
        let mut drop = [0u8; 4];
        let c = rgba_ctor(local(&mut drop), 0, 0, 0, ebx);
        font_set_drop_color(rd::<u32>(c, 0));
        let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_21)));
        let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_605)));
        font_set_scale(w.to_bits(), h.to_bits());
        let mut lit = false;
        if level > i {
            let t = g::<u32>(TIME_MS).read();
            let changed_at = rd::<u32>(find_player_wanted(-1), 0xC).wrapping_add(2000);
            if t > changed_at || b(FRAME_COUNTER) & 4 != 0 {
                lit = true;
            }
        }
        if lit {
            let mut col = [0u8; 4];
            let c = hud_colours_get_rgba(g(HUD_COLOUR), local(&mut col), 6, ebx);
            font_set_color(rd::<u32>(c, 0));
            let sy = max_h().fmul_m(k(K_1_448));
            let yc = f32_of(x(k(K_114)).fmul(sy));
            let focus = usize::from(b(PLAYER_IN_FOCUS));
            let max_health = g::<u8>(PLAYERS + focus * 0x190 + 0x14F).read();
            let py = if !xi(i32::from(max_health)).fcom(x(k(K_101))).test_parity(0x05) {
                f32_of(sy.fmul_m(k(K_12)).fsubr_m(yc))
            } else {
                yc
            };
            font_print_string(px, py, local(&mut star));
        } else if parole > i && b(FRAME_COUNTER) & 4 != 0 {
            let cb = ftol32(xi(i32::from(b(HUD_COLOUR + 6 * 4 + 2))).fmul_m(k(K_0_8))) as u32;
            let cg = ftol32(xi(i32::from(b(HUD_COLOUR + 6 * 4 + 1))).fmul_m(k(K_0_8))) as u32;
            let cr = ftol32(xi(i32::from(b(HUD_COLOUR + 6 * 4))).fmul_m(k(K_0_8))) as u32;
            let mut col = [0u8; 4];
            let c = rgba_ctor(local(&mut col), cr, cg, cb, ebx);
            font_set_color(rd::<u32>(c, 0));
            let focus = b(PLAYER_IN_FOCUS);
            let y = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_114)));
            let py = f32_of(get_y_pos_based_on_health(focus, y, 12));
            font_print_string(px, py, local(&mut star));
        } else if level <= i {
            font_set_edge(0);
            let a = ftol32(x(alpha_f).fmul_m(k(K_0_7))) as u32;
            let mut col = [0u8; 4];
            let c = rgba_ctor(local(&mut col), 0, 0, 0, a);
            font_set_color(rd::<u32>(c, 0));
            let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_21)).fmul_m(k(K_1_2)));
            let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_605)).fmul_m(k(K_1_2)));
            font_set_scale(w.to_bits(), h.to_bits());
            let focus = b(PLAYER_IN_FOCUS);
            let syf = f32_of(max_h().fmul_m(k(K_1_448)));
            let y = f32_of(x(syf).fmul_m(k(K_114)));
            let yp = get_y_pos_based_on_health(focus, y, 12);
            let py = f32_of(yp.fsub(x(syf).fdouble()));
            font_print_string(px, py, local(&mut star));
        }
        px = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_18)).fsubr_m(px));
    }
    font_set_edge(0);
}

/// `m_ItemToFlash == item && !(frame & 8)`: the flashing HUD item is hidden this frame.
fn flashed_off(item: u16) -> bool {
    g::<u16>(ITEM_TO_FLASH).read() == item && b(FRAME_COUNTER) & 8 == 0
}

/// `CPlayerInfo` of player `id` (`CWorld::Players[id]`, stride 0x190).
fn player_info(id: i32) -> GPtr<u8> {
    g::<u8>(PLAYERS.wrapping_add(id.wrapping_mul(0x190) as usize))
}

/// The two `CRGBA` stack temporaries of the bar functions: `CRGBA(0, 0, 0, 0)` (the add colour), then
/// `HudColour.GetRGB(colour)`; returns their dwords (colour, add colour).
fn bar_colours(colour: u32) -> (u32, u32) {
    let mut a = [0u8; 4];
    let ap = rgba_ctor(local(&mut a), 0, 0, 0, 0);
    let mut bb = [0u8; 4];
    let bp = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut bb), colour);
    (rd::<u32>(bp, 0), rd::<u32>(ap, 0))
}

/// `CHud::RenderArmorBar(int32 player, int32 x, int32 y)` (0x5890A0): only with armour above 1.0.
#[hook(addr = 0x5890A0, conv = cdecl, name = "CHud::RenderArmorBar", expect = "83 EC 0C 66 83 3D DC B1 BA 00 03")]
pub fn render_armor_bar(id: i32, px: i32, py: i32) {
    if flashed_off(3) {
        return;
    }
    let pi = player_info(id);
    if x(rd::<f32>(rd::<GPtr<u8>>(pi, 0), 0x548)).fcom(x(k(K_1))).test_any(0x41) {
        return;
    }
    let (colour, add) = bar_colours(4);
    let ped = rd::<GPtr<u8>>(pi, 0);
    let max_armour = i32::from(rd::<u8>(pi, 0x150));
    let progress = f32_of(xi(max_armour).fdivr_m(rd::<f32>(ped, 0x548)).fmul_m(k(K_100)));
    let h = ftol32(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_9))) as u32;
    let w = ftol32(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_62))) as u32;
    let fy = f32_of(xi(py));
    let fx = f32_of(xi(px));
    sprite2d_draw_bar_chart(fx, fy, w, h, progress, 0, 0, 1, colour, add);
}

/// `CHud::RenderBreathBar(int32 player, int32 x, int32 y)` (0x589190): breath (`m_pPlayerData` +0x44) over
/// `GetFatAndMuscleModifier(8)`.
#[hook(addr = 0x589190, conv = cdecl, name = "CHud::RenderBreathBar", expect = "83 EC 0C 66 83 3D DC B1 BA 00 0A")]
pub fn render_breath_bar(id: i32, px: i32, py: i32) {
    if flashed_off(10) {
        return;
    }
    let ped = rd::<GPtr<u8>>(player_info(id), 0);
    let data = rd::<GPtr<u8>>(ped, 0x480);
    let ratio = f32_of(stats_get_fat_and_muscle_modifier(8).fdivr_m(rd::<f32>(data, 0x44)));
    let (colour, add) = bar_colours(3);
    let progress = f32_of(x(ratio).fmul_m(k(K_100)));
    let h = ftol32(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_9))) as u32;
    let w = ftol32(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_62))) as u32;
    let fy = f32_of(xi(py));
    let fx = f32_of(xi(px));
    sprite2d_draw_bar_chart(fx, fy, w, h, progress, 0, 0, 1, colour, add);
}

/// `CHud::RenderHealthBar(int32 player, int32 x, int32 y)` (0x589270): blinks with `frame & 8` below 10
/// health (`(int16)ftol(health) < 10`); the bar is `width = ftol(sx * maxHealth * 109 / modifier(10))` wide
/// and right-aligned at `x + sx * 109 - (uint16)width`.
#[hook(addr = 0x589270, conv = cdecl, name = "CHud::RenderHealthBar", expect = "83 EC 0C 66 83 3D DC B1 BA 00 04")]
pub fn render_health_bar(id: i32, px: i32, py: i32) {
    let frame = b(FRAME_COUNTER);
    if g::<u16>(ITEM_TO_FLASH).read() == 4 && frame & 8 == 0 {
        return;
    }
    let pi = player_info(id);
    let hp = ftol32(x(rd::<f32>(rd::<GPtr<u8>>(pi, 0), 0x540))) as i16;
    if hp < 10 && frame & 8 == 0 {
        return;
    }
    let max_health = i32::from(rd::<u8>(pi, 0x14F));
    let w1 = f32_of(max_w().fmul_m(k(K_1_640)).fmul(xi(max_health)).fmul_m(k(K_109)));
    let width = ftol32(stats_get_fat_and_muscle_modifier(10).fdivr_m(w1)) as u32;
    let ped = rd::<GPtr<u8>>(pi, 0);
    let max_health = i32::from(rd::<u8>(pi, 0x14F));
    let progress = f32_of(x(rd::<f32>(ped, 0x540)).fmul_m(k(K_100)).fdiv(xi(max_health)));
    let (colour, add) = bar_colours(0);
    let h = ftol32(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_9))) as u32;
    let fy = f32_of(xi(py));
    let fx = f32_of(
        max_w()
            .fmul_m(k(K_1_640))
            .fmul_m(k(K_109))
            .fsub(xi(i32::from(width as u16)))
            .fadd(xi(px)),
    );
    sprite2d_draw_bar_chart(fx, fy, width, h, progress, 0, 0, 1, colour, add);
}

/// The common gate of the help message setters: no `m_BigMessage[4]`, no garage message, no replay playback
/// and no cutscene (`SetHelpMessage` tests `CCutsceneMgr::ms_running` 0xB5F851, the other two
/// `ms_cutsceneProcessing` 0xB5F852, as the bytes).
fn help_blocked(cutscene: usize) -> bool {
    b(BIG_MESSAGE_4) != 0 || b(GARAGES_MESSAGE_ID) != 0 || b(REPLAY_MODE) == 1 || b(cutscene) != 0
}

fn help_buf(a: &sa_core::mem::Global<[u8; 400]>) -> GPtr<u8> {
    a.ptr().cast()
}

/// Tail of `SetHelpMessage` / `SetHelpMessageWithNumber`: a permanent message goes straight to the print
/// and last buffers (state 1), else state 0.
fn help_finish(permanent: u8) {
    use crate::hud::{HELP_MESSAGE, HELP_MESSAGE_TO_PRINT, LAST_HELP_MESSAGE};
    if permanent != 0 {
        crate::hud::HELP_MESSAGE_STATE.set(1);
        msg_string_copy(help_buf(&HELP_MESSAGE_TO_PRINT), help_buf(&HELP_MESSAGE), 0x190);
        msg_string_copy(help_buf(&LAST_HELP_MESSAGE), help_buf(&HELP_MESSAGE), 0x190);
    } else {
        crate::hud::HELP_MESSAGE_STATE.set(0);
    }
}

fn help_flags(quick: u8, permanent: u8) {
    crate::hud::HELP_MESSAGE_PERMANENT.set(permanent);
    crate::hud::HELP_MESSAGE_QUICK.set(quick);
    crate::hud::HELP_MESSAGE_STAT_ID.set(0);
    crate::hud::HELP_MESSAGE_MAX_STAT_VALUE.set(1000);
    crate::hud::HELP_MESSAGE_STAT_UPDATE_VALUE.set(0);
}

/// `CHud::SetHelpMessage(const GxtChar* text, bool quick, bool permanent, bool addToBrief)` (0x588BE0).
/// An unchanged text is ignored while a help message shows; a null text clears the help and print buffers.
#[hook(addr = 0x588BE0, conv = cdecl, name = "CHud::SetHelpMessage", expect = "A0 C0 AE BA 00")]
pub fn set_help_message(text: GPtr<u8>, quick: u8, permanent: u8, brief: u8) {
    use crate::hud::{HELP_MESSAGE, HELP_MESSAGE_TO_PRINT, LAST_HELP_MESSAGE};
    if help_blocked(CUTSCENE_RUNNING) {
        return;
    }
    HELP_MESSAGE_TO_PRINT.set([0; 400]);
    LAST_HELP_MESSAGE.set([0; 400]);
    HELP_MESSAGE.set([0; 400]);
    msg_string_copy(help_buf(&HELP_MESSAGE), text, 0x190);
    msg_insert_player_control_keys(help_buf(&HELP_MESSAGE));
    if crate::hud::HELP_MESSAGE_STATE.get() != 0
        && msg_string_compare(help_buf(&HELP_MESSAGE), help_buf(&HELP_MESSAGE_TO_PRINT), 0x190) != 0
    {
        return;
    }
    LAST_HELP_MESSAGE.set([0; 400]);
    if text.is_null() {
        help_buf(&HELP_MESSAGE).write(0);
        help_buf(&HELP_MESSAGE_TO_PRINT).write(0);
    }
    help_finish(permanent);
    if brief != 0 {
        msg_add_to_previous_brief_array(text, -1, -1, -1, -1, -1, -1, GPtr::NULL);
    }
    help_flags(quick, permanent);
}

/// `CHud::SetHelpMessageStatUpdate(eStatUpdateState state, uint16 stat, float value, float max)` (0x588D40).
/// The three buffers are cleared first, so the "same text" test compares two empty buffers and returns
/// whenever a help message is shown (as the bytes). The message is `"+"` (state != 0) or `"-"`.
#[hook(addr = 0x588D40, conv = cdecl, name = "CHud::SetHelpMessageStatUpdate", expect = "A0 C0 AE BA 00")]
pub fn set_help_message_stat_update(state: u8, stat: u16, value: u32, max: f32) {
    use crate::hud::{HELP_MESSAGE, HELP_MESSAGE_TO_PRINT, LAST_HELP_MESSAGE};
    if help_blocked(CUTSCENE_PROCESSING) {
        return;
    }
    HELP_MESSAGE_TO_PRINT.set([0; 400]);
    LAST_HELP_MESSAGE.set([0; 400]);
    HELP_MESSAGE.set([0; 400]);
    if crate::hud::HELP_MESSAGE_STATE.get() != 0
        && msg_string_compare(help_buf(&HELP_MESSAGE), help_buf(&HELP_MESSAGE_TO_PRINT), 0x190) != 0
    {
        return;
    }
    LAST_HELP_MESSAGE.set([0; 400]);
    crate::hud::HELP_MESSAGE_STATE.set(0);
    crate::hud::HELP_MESSAGE_QUICK.set(0);
    crate::hud::HELP_MESSAGE_PERMANENT.set(0);
    crate::hud::HELP_MESSAGE_STAT_ID.set(stat);
    crate::hud::HELP_MESSAGE_STAT_UPDATE_VALUE.set(value);
    crate::hud::HELP_MESSAGE_MAX_STAT_VALUE.set(ftol32(x(max)) as u16);
    let sign: u16 = if state == 0 { 0x2D } else { 0x2B };
    g::<u16>(G_STRING).write(sign);
    ascii_to_gxt_char(g(G_STRING), help_buf(&HELP_MESSAGE));
}

/// `CHud::SetHelpMessageWithNumber(const GxtChar* text, int32 number, bool quick, bool permanent)`
/// (0x588E30). Unlike `SetHelpMessage` it neither clears the print buffers nor tests for the same text; the
/// `GetStringLength` result is unused (as the bytes).
#[hook(addr = 0x588E30, conv = cdecl, name = "CHud::SetHelpMessageWithNumber", expect = "A0 C0 AE BA 00")]
pub fn set_help_message_with_number(text: GPtr<u8>, number: i32, quick: u8, permanent: u8) {
    use crate::hud::{HELP_MESSAGE, LAST_HELP_MESSAGE};
    if help_blocked(CUTSCENE_PROCESSING) {
        return;
    }
    let mut buf = [0u8; 0x320];
    msg_insert_number_in_string(text, number, -1, -1, -1, -1, -1, local(&mut buf));
    msg_get_string_length(local(&mut buf));
    msg_string_copy(help_buf(&HELP_MESSAGE), local(&mut buf), 0x190);
    msg_insert_player_control_keys(help_buf(&HELP_MESSAGE));
    LAST_HELP_MESSAGE.set([0; 400]);
    help_finish(permanent);
    help_flags(quick, permanent);
}

/// `CHud::Shutdown()` (0x588850): deletes the six HUD sprites and removes the `"hud"` txd slot.
#[hook(addr = 0x588850, conv = cdecl, name = "CHud::Shutdown", expect = "56 BE FC B1 BA 00")]
pub fn shutdown() {
    for i in 0..6 {
        sprite2d_delete(g(SPRITES + i * 4));
    }
    remove_txd_slot(find_txd_slot(g(STR_HUD)));
}

fn w32(addr: usize, v: u32) {
    g::<u32>(addr).write(v);
}
fn w16(addr: usize, v: u16) {
    g::<u16>(addr).write(v);
}
fn w8(addr: usize, v: u8) {
    g::<u8>(addr).write(v);
}

/// `CHud::ReInitialise()` (0x588880): clears the message buffers and the HUD state; the energy-lost time and
/// the displayed score take the focused player's values (`Players[focus]` +0xE4 and +0xBC); 0x8D0934 = 200.0
/// and 0x8D0938 = 150.0. One `mov ecx, 0x64` sits in a protection chunk (0x401A64).
#[hook(addr = 0x588880, conv = cdecl, name = "CHud::ReInitialise", expect = "57 33 D2 33 C0")]
pub fn re_initialise() {
    use crate::hud::{
        BIG_MESSAGE, BIG_MESSAGE_X, HELP_MESSAGE, HELP_MESSAGE_FADE_TIMER, HELP_MESSAGE_MAX_STAT_VALUE,
        HELP_MESSAGE_PERMANENT, HELP_MESSAGE_QUICK, HELP_MESSAGE_STATE, HELP_MESSAGE_STAT_ID,
        HELP_MESSAGE_STAT_UPDATE_VALUE, HELP_MESSAGE_TIME, HELP_MESSAGE_TIMER, HELP_MESSAGE_TO_PRINT,
        LAST_HELP_MESSAGE, MESSAGE, ODD_JOB2_OFF_TIMER, ODD_JOB2_ON, VEHICLE_FADE_TIMER, VEHICLE_NAME,
        VEHICLE_NAME_TIMER, VEHICLE_NAME_TO_PRINT, VEHICLE_STATE, ZONE_NAME, ZONE_NAME_TIMER, ZONE_STATE,
    };
    HELP_MESSAGE_TO_PRINT.set([0; 400]);
    LAST_HELP_MESSAGE.set([0; 400]);
    HELP_MESSAGE.set([0; 400]);
    MESSAGE.set([0; 400]);
    for i in 0..0x380 {
        g::<u8>(BIG_MESSAGE + i).write(0);
    }
    HELP_MESSAGE_STAT_UPDATE_VALUE.set(0);
    BIG_MESSAGE_X.set([0.0; 7]);
    w32(0xBAA38C, 0); // TimerCounterHideState[0..2]
    w32(0xBAA390, 0); // TimerCounterHideState[2..4]
    w32(0xBAA39C, 0); // OddJob2XOffset
    ODD_JOB2_OFF_TIMER.set(0);
    let pi = PLAYERS + usize::from(b(PLAYER_IN_FOCUS)) * 0x190;
    let energy = g::<u32>(pi + 0xE4).read();
    let score = g::<u32>(pi + 0xBC).read();
    w32(0xBAA440, energy); // m_LastTimeEnergyLost
    w8(0xBAA45D, 1); // m_Wants_To_Draw_Hud
    w8(DRAW_3D_MARKERS, 1);
    ZONE_NAME_TIMER.set(0);
    ZONE_NAME.set(0);
    w32(0xBAB1D4, 0); // m_pLastZoneName
    ZONE_STATE.set(0);
    HELP_MESSAGE_TIMER.set(0);
    HELP_MESSAGE_FADE_TIMER.set(0);
    HELP_MESSAGE_STATE.set(0);
    HELP_MESSAGE_QUICK.set(0);
    HELP_MESSAGE_STAT_ID.set(0);
    HELP_MESSAGE_MAX_STAT_VALUE.set(1000);
    HELP_MESSAGE_PERMANENT.set(0);
    HELP_MESSAGE_TIME.set(0x3F80_0000);
    w32(0x8D0934, 0x4348_0000);
    VEHICLE_NAME.set(0);
    w32(0xBAA454, 0); // m_pLastVehicleName
    VEHICLE_NAME_TO_PRINT.set(0);
    VEHICLE_NAME_TIMER.set(0);
    VEHICLE_FADE_TIMER.set(0);
    VEHICLE_STATE.set(0);
    w32(0xBAA394, 0); // TimerCounterWasDisplayed[0..2]
    w8(0xBAA38A, 0); // TimerMainCounterWasDisplayed
    w16(0xBAA388, 0); // TimerMainCounterHideState
    w8(DONT_DISPLAY_RADAR, 0);
    w8(0xBAA3FA, 0); // bScriptForceDisplayWithCounters
    w8(DONT_DISPLAY_VEHICLE_NAME, 0);
    w8(DONT_DISPLAY_AREA_NAME, 0);
    w16(ITEM_TO_FLASH, 0xFFFF);
    w32(0x8D0938, 0x4316_0000);
    ODD_JOB2_ON.set(0);
    w16(0xBAA3A0, 0); // OddJob2Timer
    w32(0xBAA43C, 0); // m_EnergyLostTimer
    w32(0xBAA438, 0); // m_EnergyLostFadeTimer
    w32(0xBAA434, 5); // m_EnergyLostState
    w32(0xBAA430, score); // m_LastDisplayScore
    w32(0xBAA42C, 0); // m_DisplayScoreTimer
    w32(0xBAA428, 0); // m_DisplayScoreFadeTimer
    w32(0xBAA424, 5); // m_DisplayScoreState
    LAST_WANTED.set(0);
    WANTED_TIMER.set(0);
    WANTED_FADE_TIMER.set(0);
    WANTED_STATE.set(5);
    w32(0xBAA410, 0); // m_LastWeapon
    w32(0xBAA40C, 0); // m_WeaponTimer
    w32(0xBAA408, 0); // m_WeaponFadeTimer
    w32(0xBAA404, 5); // m_WeaponState
    w32(0xBAA400, 1); // bDrawClock
    w32(0xBAA3FC, 0); // m_LastBreathTime
}

/// `(float)(uint32)ftol(CTimer::ms_fTimeStep * 0.02 * 1000)`: `fild` of the result, `+ 2^32` when negative.
fn step_ms_unsigned() -> X87 {
    let v = ftol32(x(g::<f32>(TIME_STEP).read()).fmul_m(k(K_0_02)).fmul_m(k(K_1000)));
    let r = xi(v);
    if v < 0 {
        r.fadd_m(k(K_2_32))
    } else {
        r
    }
}

/// `CHud::DrawScriptText(bool beforeFade)` (0x58C080): script sprites and rectangles, then the 0x60
/// `IntroTextLines` whose key is set and whose before-fade byte equals the argument's low byte.
#[hook(addr = 0x58C080, conv = cdecl, name = "CHud::DrawScriptText", expect = "81 EC 90 01 00 00")]
pub fn draw_script_text(before_fade: u32) {
    draw_script_sprites_and_rectangles(before_fade);
    for i in 0..0x60usize {
        let e = g::<u8>(INTRO_TEXT_LINES_4 + i * 0x44);
        if rd::<u8>(e, 0x30) == 0 || rd::<u8>(e, 0x1F) != before_fade as u8 {
            continue;
        }
        let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(rd::<f32>(e, 0)).fmul_m(k(K_HALF)));
        let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(rd::<f32>(e, -4)));
        font_set_scale(w.to_bits(), h.to_bits());
        font_set_color(rd::<u32>(e, 4));
        font_set_justify(u32::from(rd::<u8>(e, 8)));
        let orient = if rd::<u8>(e, 0x20) != 0 {
            2
        } else if rd::<u8>(e, 9) != 0 {
            0
        } else {
            1
        };
        font_set_orientation(orient);
        font_set_wrapx(f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(rd::<f32>(e, 0xC))).to_bits());
        font_set_centre_size(f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(rd::<f32>(e, 0x10))).to_bits());
        font_set_background(u32::from(rd::<u8>(e, 0xA)), 0);
        font_set_background_color(rd::<u32>(e, 0x14));
        font_set_proportional(u32::from(rd::<u8>(e, 0x18)));
        font_set_drop_color(rd::<u32>(e, 0x19));
        let edge = rd::<u8>(e, 0x1E);
        if edge != 0 {
            font_set_edge(u32::from(edge));
        } else {
            font_set_drop_shadow_position(u32::from(rd::<u8>(e, 0x1D)));
        }
        font_set_font_style(u32::from(rd::<u8>(e, 0x24)));
        let text = text_get(g(THE_TEXT), e.byte_add(0x30));
        let mut buf = [0u8; 0x190];
        msg_insert_number_in_string(text, rd::<i32>(e, 0x38), rd::<i32>(e, 0x3C), -1, -1, -1, -1, local(&mut buf));
        msg_insert_player_control_keys(local(&mut buf));
        let mh = max_h();
        let mw = max_w();
        let py = f32_of(x(k(K_448)).fsub_m(rd::<f32>(e, 0x2C)).fmul(mh.fmul_m(k(K_1_448))).fsubr(mh));
        let px = f32_of(x(k(K_640)).fsub_m(rd::<f32>(e, 0x28)).fmul(mw.fmul_m(k(K_1_640))).fsubr(mw));
        font_print_string(px, py, local(&mut buf));
        font_set_edge(0);
    }
}

/// `CHud::DrawMissionTitle()` (0x58D240). The first frame of a title (X == 0) only starts it: in-use
/// -60, X 1, zone state and fade timer 0 and `SetHelpMessage(nullptr, true, false, false)`. Then the title
/// slides in (in-use += 0.3 * step while `maxWidth - 20 > in-use`), holds until X reaches 120 and fades
/// out; it is printed from the bottom with edge 2.
#[hook(addr = 0x58D240, conv = cdecl, name = "CHud::DrawMissionTitle", expect = "51 A0 40 AD BA 00")]
pub fn draw_mission_title() {
    if b(BIG_MESSAGE_1) == 0 {
        g::<u32>(BIG_MESSAGE_X_1).write(0);
        return;
    }
    if !x(g::<f32>(BIG_MESSAGE_X_1).read()).fcom(x(k(K_ZERO))).test_parity(0x44) {
        g::<u32>(BIG_MESSAGE_IN_USE_1).write(0xC270_0000);
        g::<u32>(BIG_MESSAGE_X_1).write(0x3F80_0000);
        g::<u32>(0xBAA930).write(0);
        g::<u32>(ZONE_FADE_TIMER).write(0);
        set_help_message_call(GPtr::NULL, 1, 0, 0);
        return;
    }
    font_set_background(0, 0);
    font_set_proportional(1);
    font_set_right_justify_wrap(0);
    font_set_orientation(2);
    font_set_font_style(3);
    let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_3)));
    let w = f32_of(max_w().fmul_m(k(K_1_640)));
    font_set_scale(w.to_bits(), h.to_bits());
    let lim = g::<i32>(RS_MAX_WIDTH).read().wrapping_add(-20);
    let ts = x(g::<f32>(TIME_STEP).read());
    if !xi(lim).fcom(x(g::<f32>(BIG_MESSAGE_IN_USE_1).read())).test_any(0x41) {
        let v = step_ms_unsigned().fmul_m(k(K_0_3));
        g::<u32>(BIG_MESSAGE_ALPHA_1).write(0x437F_0000);
        let n = v.fadd_m(g::<f32>(BIG_MESSAGE_IN_USE_1).read());
        g::<f32>(BIG_MESSAGE_IN_USE_1).write(f32_of(n));
    } else {
        let nx = ts.fadd_m(g::<f32>(BIG_MESSAGE_X_1).read());
        g::<f32>(BIG_MESSAGE_X_1).write(f32_of(nx));
        if !nx.fcom(x(k(K_120))).test_any(0x01) {
            g::<u32>(BIG_MESSAGE_X_1).write(0x42F0_0000);
            let v = step_ms_unsigned().fsubr_m(g::<f32>(BIG_MESSAGE_ALPHA_1).read());
            g::<f32>(BIG_MESSAGE_ALPHA_1).write(f32_of(v));
        }
        if !x(g::<f32>(BIG_MESSAGE_ALPHA_1).read()).fcom(x(k(K_ZERO))).test_parity(0x41) {
            g::<u32>(BIG_MESSAGE_ALPHA_1).write(0);
            g::<u8>(BIG_MESSAGE_1).write(0);
            g::<u32>(BIG_MESSAGE_X_1).write(0);
        }
    }
    font_set_edge(2);
    let a = ftol32(x(g::<f32>(BIG_MESSAGE_ALPHA_1).read())) as u32;
    let mut tmp = [0u8; 4];
    let c = rgba_ctor(local(&mut tmp), 0, 0, 0, a);
    font_set_drop_color(rd::<u32>(c, 0));
    let a = ftol32(x(g::<f32>(BIG_MESSAGE_ALPHA_1).read())) as u32;
    let c = hud_colours_get_rgba(g(HUD_COLOUR), local(&mut tmp), 6, a);
    font_set_color(rd::<u32>(c, 0));
    let mh = max_h();
    let mw = max_w();
    let py = f32_of(mh.fmul_m(k(K_1_448)).fmul_m(k(K_115)).fsubr(mh));
    let px = f32_of(x(k(K_1_640)).fmul(mw).fmul_m(k(K_20)).fsubr(mw));
    font_print_string_from_bottom(px, py, g(BIG_MESSAGE_1));
    font_set_edge(0);
}

/// `maxHeight / 2 - maxHeight * (1/448) * 30` (the busted / wasted Y).
fn busted_y() -> f32 {
    let mh = g::<i32>(RS_MAX_HEIGHT).read();
    f32_of(xi(mh / 2).fsub(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_30))))
}

/// `CHud::DrawBustedWastedMessage()` (0x58CA50). The Y is a function-local static (0xBAB220, guard bit 0 of
/// 0xBAB224), set once and again on the first frame of each message (both arms of the `m_BigMessage[0]`
/// test compute the same value, as the bytes). The alpha rises by `0.4 * step` up to 255.
#[hook(addr = 0x58CA50, conv = cdecl, name = "CHud::DrawBustedWastedMessage", expect = "51 A0 C0 AD BA 00")]
pub fn draw_busted_wasted_message() {
    if b(BIG_MESSAGE_2) == 0 {
        g::<u32>(BIG_MESSAGE_X_2).write(0);
        return;
    }
    let guard = g::<u32>(BUSTED_Y_GUARD).read();
    if guard & 1 == 0 {
        g::<u32>(BUSTED_Y_GUARD).write(guard | 1);
        g::<f32>(BUSTED_Y).write(busted_y());
    }
    if !x(g::<f32>(BIG_MESSAGE_X_2).read()).fcom(x(k(K_ZERO))).test_parity(0x44) {
        let vs = crate::hud::VEHICLE_STATE.get();
        g::<u32>(BIG_MESSAGE_X_2).write(0x3F80_0000);
        g::<u32>(BIG_MESSAGE_ALPHA_2).write(0);
        if vs != 0 {
            crate::hud::VEHICLE_STATE.set(0);
        }
        if crate::hud::ZONE_STATE.get() != 0 {
            crate::hud::ZONE_STATE.set(0);
        }
        g::<f32>(BUSTED_Y).write(busted_y());
        return;
    }
    let a = step_ms_unsigned().fmul_m(k(K_0_4)).fadd_m(g::<f32>(BIG_MESSAGE_ALPHA_2).read());
    g::<f32>(BIG_MESSAGE_ALPHA_2).write(f32_of(a));
    if !a.fcom(x(k(K_255))).test_any(0x41) {
        g::<u32>(BIG_MESSAGE_ALPHA_2).write(0x437F_0000);
    }
    font_set_background(0, 0);
    let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_2_1)));
    let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_2_1)));
    font_set_scale(w.to_bits(), h.to_bits());
    font_set_proportional(1);
    font_set_justify(0);
    font_set_orientation(0);
    font_set_font_style(0);
    font_set_edge(3);
    let al = ftol32(x(g::<f32>(BIG_MESSAGE_ALPHA_2).read())) as u32;
    let mut tmp = [0u8; 4];
    let c = rgba_ctor(local(&mut tmp), 0, 0, 0, al);
    font_set_drop_color(rd::<u32>(c, 0));
    let al = ftol32(x(g::<f32>(BIG_MESSAGE_ALPHA_2).read())) as u32;
    let c = hud_colours_get_rgba(g(HUD_COLOUR), local(&mut tmp), 4, al);
    font_set_color(rd::<u32>(c, 0));
    let py = g::<f32>(BUSTED_Y).read();
    let px = f32_of(xi(g::<i32>(RS_MAX_WIDTH).read() / 2));
    font_print_string_from_bottom(px, py, g(BIG_MESSAGE_2));
}

/// `fmul qword [c]`: a double constant as a register operand (exact; one rounding to 24 bits).
fn dq(bits: u64) -> X87 {
    X87::from_f64_exact(f64::from_bits(bits))
}

/// `(float)(maxHeight / 2) - maxHeight * (1/448) * 10`
fn success_y() -> X87 {
    let mh = g::<i32>(RS_MAX_HEIGHT).read();
    xi(mh / 2).fsub(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_10)))
}

/// `CHud::DrawSuccessFailedMessage()` (0x58C6A0). The Y is a function-local static (0xBAB218, guard bit 0
/// of 0xBAB21C). On the first frame of a message (X == 0) only the Y is set: `+ 25 * sy` while message 3 or 5
/// shows, the base Y while message 2 shows, else `- 15 * sy` when the text needs more than one line. Then
/// the text slides in (in-use and alpha += 0.3 * step), holds until X reaches 120 and fades out.
#[hook(addr = 0x58C6A0, conv = cdecl, name = "CHud::DrawSuccessFailedMessage", expect = "51 F6 05 1C B2 BA 00 01")]
pub fn draw_success_failed_message() {
    let guard = g::<u32>(SUCCESS_Y_GUARD).read();
    if guard & 1 == 0 {
        g::<u32>(SUCCESS_Y_GUARD).write(guard | 1);
        g::<f32>(SUCCESS_Y).write(f32_of(success_y()));
    }
    if b(BIG_MESSAGE_0) == 0 {
        g::<u32>(BIG_MESSAGE_X_0).write(0);
        return;
    }
    if !x(g::<f32>(BIG_MESSAGE_X_0).read()).fcom(x(k(K_ZERO))).test_parity(0x44) {
        g::<u32>(BIG_MESSAGE_IN_USE_0).write(0xC270_0000);
        g::<u32>(BIG_MESSAGE_X_0).write(0x3F80_0000);
        g::<u32>(BIG_MESSAGE_ALPHA_0).write(0);
        let mh = g::<i32>(RS_MAX_HEIGHT).read();
        if b(BIG_MESSAGE_3) != 0 || b(BIG_MESSAGE_5) != 0 {
            let sy = max_h().fmul_m(k(K_1_448));
            let y = xi(mh / 2).fsub(sy.fmul_m(k(K_10))).fadd(sy.fmul_m(k(K_25)));
            g::<f32>(SUCCESS_Y).write(f32_of(y));
            return;
        }
        if b(BIG_MESSAGE_2) == 0 {
            let mw = g::<i32>(RS_MAX_WIDTH).read();
            let ly = f32_of(success_y());
            let lx = f32_of(xi(mw / 2));
            let lines = font_get_number_lines(lx, ly, g(BIG_MESSAGE_0));
            if lines as i16 > 1 {
                let mh = g::<i32>(RS_MAX_HEIGHT).read();
                let sy = max_h().fmul_m(k(K_1_448));
                let y = xi(mh / 2).fsub(sy.fmul_m(k(K_10))).fsub(sy.fmul_m(k(K_15)));
                g::<f32>(SUCCESS_Y).write(f32_of(y));
                return;
            }
        }
        g::<f32>(SUCCESS_Y).write(f32_of(success_y()));
        return;
    }
    font_set_background(0, 0);
    let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul(dq(D_1_8)));
    let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul(dq(D_1_3)));
    font_set_scale(w.to_bits(), h.to_bits());
    font_set_proportional(1);
    font_set_justify(0);
    font_set_orientation(0);
    font_set_centre_size(f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_590))).to_bits());
    font_set_font_style(3);
    font_set_edge(2);
    let al = ftol32(x(g::<f32>(BIG_MESSAGE_ALPHA_0).read())) as u32;
    let mut tmp = [0u8; 4];
    let c = rgba_ctor(local(&mut tmp), 0, 0, 0, al);
    font_set_drop_color(rd::<u32>(c, 0));
    let al = ftol32(x(g::<f32>(BIG_MESSAGE_ALPHA_0).read())) as u32;
    let c = hud_colours_get_rgba(g(HUD_COLOUR), local(&mut tmp), 6, al);
    font_set_color(rd::<u32>(c, 0));
    let mw = g::<i32>(RS_MAX_WIDTH).read();
    let ts = x(g::<f32>(TIME_STEP).read());
    if !xi(mw.wrapping_sub(20)).fcom(x(g::<f32>(BIG_MESSAGE_IN_USE_0).read())).test_any(0x41) {
        let v = step_ms_unsigned().fmul_m(k(K_0_3));
        let iu = x(g::<f32>(BIG_MESSAGE_IN_USE_0).read()).fadd(v);
        g::<f32>(BIG_MESSAGE_IN_USE_0).write(f32_of(iu));
        let a = v.fadd_m(g::<f32>(BIG_MESSAGE_ALPHA_0).read());
        g::<f32>(BIG_MESSAGE_ALPHA_0).write(f32_of(a));
        if !a.fcom(x(k(K_255))).test_any(0x41) {
            g::<u32>(BIG_MESSAGE_ALPHA_0).write(0x437F_0000);
        }
    } else {
        let nx = ts.fadd_m(g::<f32>(BIG_MESSAGE_X_0).read());
        g::<f32>(BIG_MESSAGE_X_0).write(f32_of(nx));
        if !nx.fcom(x(k(K_120))).test_any(0x01) {
            g::<u32>(BIG_MESSAGE_X_0).write(0x42F0_0000);
            let v = step_ms_unsigned().fmul_m(k(K_0_3)).fsubr_m(g::<f32>(BIG_MESSAGE_ALPHA_0).read());
            g::<f32>(BIG_MESSAGE_ALPHA_0).write(f32_of(v));
        }
        if !x(g::<f32>(BIG_MESSAGE_ALPHA_0).read()).fcom(x(k(K_ZERO))).test_parity(0x41) {
            g::<u32>(BIG_MESSAGE_ALPHA_0).write(0);
            g::<u8>(BIG_MESSAGE_0).write(0);
        }
    }
    let py = g::<f32>(SUCCESS_Y).read();
    let px = f32_of(xi(mw / 2));
    font_print_string(px, py, g(BIG_MESSAGE_0));
}

/// `CHud::DrawTripSkip()` (0x58A160): the skip icon and the `FEC_TSK` text next to it.
#[hook(addr = 0x58A160, conv = cdecl, name = "CHud::DrawTripSkip", expect = "83 EC 1C DB 05 44 70 C1 00")]
pub fn draw_trip_skip() {
    let sx = max_w().fmul_m(k(K_1_640));
    let x0 = x(k(K_54)).fmul(sx);
    let left = f32_of(x0);
    let mh = max_h();
    let sy = x(k(K_1_448)).fmul(mh);
    let syf = f32_of(sy);
    let y0 = mh.fsub(sy.fmul_m(k(K_104))).fsub(x(syf).fmul_m(k(K_85)));
    let top = f32_of(y0);
    let right = f32_of(sx.fmul_m(k(K_64)).fadd(x0));
    let bottom = f32_of(x(syf).fmul_m(k(K_64)).fadd_m(top));
    draw_sprite_rect(SPRITE_SKIP_ICON, rect_bytes(left, bottom, right, top), [0xFF, 0xFF, 0xFF, 0xFF]);

    font_set_background(0, 0);
    let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_0_7)));
    let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_3)));
    font_set_scale(w.to_bits(), h.to_bits());
    font_set_orientation(0);
    font_set_centre_size(f32_of(max_w()).to_bits());
    font_set_proportional(1);
    font_set_edge(1);
    let mut tmp = [0u8; 4];
    let c = rgba_ctor(local(&mut tmp), 0, 0, 0, 0xFF);
    font_set_drop_color(rd::<u32>(c, 0));
    font_set_font_style(2);
    let c = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut tmp), 4);
    font_set_color(rd::<u32>(c, 0));
    let mhf = f32_of(max_h());
    let syf = f32_of(max_h().fmul_m(k(K_1_448)));
    let sxf = f32_of(max_w().fmul_m(k(K_1_640)));
    let text = text_get(g(THE_TEXT), g(STR_FEC_TSK));
    let py = f32_of(
        x(syf)
            .fmul_m(k(K_104))
            .fsubr_m(mhf)
            .fsub(x(syf).fmul_m(k(K_85)))
            .fadd(x(syf).fmul_m(k(K_64)))
            .fsub(x(syf).fdouble()),
    );
    let px = f32_of(x(sxf).fmul_m(k(K_64)).fmul_m(k(K_HALF)).fadd(x(sxf).fmul_m(k(K_54))));
    font_print_string(px, py, text);
}

/// The subtitle centre size and X of the normal layout (`DrawSubtitles`): `sx = maxW / 640`,
/// `P = 8 sx`, `R = 140 sx + P`, centre size `maxW - 20 sx - P - R` (register order), and the X from the
/// stored f32 values `(mwf - 20 sxf - Pf - Rf) * 0.5 + Rf`.
fn subtitle_centre_size() -> X87 {
    let mw = max_w();
    let sx = x(k(K_1_640)).fmul(mw);
    let p = x(k(K_8)).fmul(sx);
    let q = sx.fmul_m(k(K_20)).fsubr(mw).fsub(p);
    let r = sx.fmul_m(k(K_140)).fadd(p);
    q.fsub(r)
}

/// `(maxH - 105 sy) - 2 sy` with `sy = maxH / 448` (register order).
fn subtitle_y() -> f32 {
    let mh = max_h();
    let sy = x(k(K_1_448)).fmul(mh);
    let t = x(k(K_105)).fmul(sy).fsubr(mh);
    f32_of(t.fsub(sy.fdouble()))
}

/// The stored f32 values of the normal-layout X: (mwf, sxf, Pf, Rf).
fn subtitle_x_parts() -> (f32, f32, f32, f32) {
    let mw = max_w();
    let mwf = f32_of(mw);
    let sx = mw.fmul_m(k(K_1_640));
    let sxf = f32_of(sx);
    let pf = f32_of(sx.fmul_m(k(K_8)));
    let rf = f32_of(x(sxf).fmul_m(k(K_140)).fadd_m(pf));
    (mwf, sxf, pf, rf)
}

/// `CHud::DrawSubtitles()` (0x58C250): `m_Message` (the current brief) at the bottom of the screen. Busted /
/// wasted hides it unless a co-op game runs; vehicle and zone names are pushed to state 3. In wide screen it
/// is centred lower (and skipped in cutscenes unless subtitles are on); after a wide-screen frame the next
/// normal frame clears `m_Message[0]` before printing (as the bytes). With script message formatting the
/// script's width and centre are used; while the vital stats show, the text is scaled by 0.8 and moved 40
/// right.
#[hook(addr = 0x58C250, conv = cdecl, name = "CHud::DrawSubtitles", expect = "A0 40 B0 BA 00 83 EC 10")]
pub fn draw_subtitles() {
    if b(MESSAGE_0) == 0 {
        return;
    }
    if b(BIG_MESSAGE_2) != 0 && is_coop_game_going_on() == 0 {
        return;
    }
    if crate::hud::VEHICLE_STATE.get() != 0 {
        crate::hud::VEHICLE_STATE.set(3);
    }
    if crate::hud::ZONE_STATE.get() != 0 {
        crate::hud::ZONE_STATE.set(3);
    }
    font_set_background(0, 0);
    let mut tmp = [0u8; 4];
    let c = rgba_ctor(local(&mut tmp), 0, 0, 0, 0x80);
    font_set_background_color(rd::<u32>(c, 0));
    font_set_orientation(0);
    font_set_proportional(1);
    font_set_drop_shadow_position(0);
    font_set_font_style(1);
    let c = rgba_ctor(local(&mut tmp), 0xE1, 0xE1, 0xE1, 0xFF);
    font_set_color(rd::<u32>(c, 0));
    font_set_drop_shadow_position(2);
    let c = rgba_ctor(local(&mut tmp), 0, 0, 0, 0xFF);
    font_set_drop_color(rd::<u32>(c, 0));

    let (px, py);
    if b(WIDE_SCREEN_ON) != 0 {
        let show = b(MENU_SHOW_SUBTITLES);
        w8(SUBTITLES_WIDESCREEN, 1);
        if show == 0 && b(CUTSCENE_RUNNING) != 0 {
            font_set_drop_shadow_position(0);
            return;
        }
        let mw = max_w();
        font_set_centre_size(f32_of(x(k(K_1_640)).fmul(mw).fmul_m(k(K_60)).fsubr(mw)).to_bits());
        let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_2)));
        let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_58)));
        font_set_scale(w.to_bits(), h.to_bits());
        let mh = max_h();
        py = f32_of(x(k(K_1_448)).fmul(mh).fmul_m(k(K_80)).fsubr(mh));
        px = f32_of(xi(g::<i32>(RS_MAX_WIDTH).read() / 2));
    } else {
        if b(SUBTITLES_WIDESCREEN) != 0 {
            w8(MESSAGE_0, 0);
        }
        w8(SUBTITLES_WIDESCREEN, 0);
        let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_22))).to_bits();
        let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_58)));
        font_set_scale_for_current_language(w, h);
        if b(SCRIPTS_USE_MESSAGE_FORMATTING) != 0 {
            let width = i32::from(g::<u16>(SCRIPTS_MESSAGE_WIDTH).read());
            font_set_centre_size(f32_of(max_w().fmul_m(k(K_1_640)).fmul(xi(width))).to_bits());
            let centre = i32::from(g::<u16>(SCRIPTS_MESSAGE_CENTRE).read());
            py = subtitle_y();
            px = f32_of(max_w().fmul_m(k(K_1_640)).fmul(xi(centre)));
        } else if DRAWING_VITAL_STATS.get() != 0 {
            let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_22))).to_bits();
            let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_58)).fmul_m(k(K_0_8)));
            font_set_scale_for_current_language(w, h);
            font_set_centre_size(f32_of(subtitle_centre_size().fmul_m(k(K_0_8))).to_bits());
            let (mwf, sxf, pf, rf) = subtitle_x_parts();
            py = subtitle_y();
            px = f32_of(
                x(sxf)
                    .fmul_m(k(K_20))
                    .fsubr_m(mwf)
                    .fsub_m(pf)
                    .fsub_m(rf)
                    .fmul_m(k(K_HALF))
                    .fadd_m(rf)
                    .fadd(x(sxf).fmul_m(k(K_40))),
            );
        } else {
            font_set_centre_size(f32_of(subtitle_centre_size()).to_bits());
            let (mwf, sxf, pf, rf) = subtitle_x_parts();
            py = subtitle_y();
            px = f32_of(x(sxf).fmul_m(k(K_20)).fsubr_m(mwf).fsub_m(pf).fsub_m(rf).fmul_m(k(K_HALF)).fadd_m(rf));
        }
    }
    font_print_string(px, py, g(MESSAGE_0));
    font_set_drop_shadow_position(0);
}

/// `CHud::DrawVehicleName()` (0x58AEA0). State machine in `m_VehicleState` (1 shown, 2 fading in, 3 fading
/// out, 4 fading out for a new name), then the name at the bottom right while no brief shows.
/// Partial port (ADR-002): with a state outside 0..=4 the original's switch default leaves its alpha local
/// uninitialised (stack garbage), so that path runs the original body. No writer stores such a state.
#[hook(addr = 0x58AEA0, conv = cdecl, name = "CHud::DrawVehicleName", expect = "51 8B 15 58 A4 BA 00", fallback)]
pub fn draw_vehicle_name() {
    use crate::hud::{VEHICLE_FADE_TIMER, VEHICLE_NAME, VEHICLE_NAME_TIMER, VEHICLE_NAME_TO_PRINT, VEHICLE_STATE};
    const LAST_VEHICLE_NAME: usize = 0xBAA454;
    let name = VEHICLE_NAME.get();
    if name == 0 {
        VEHICLE_STATE.set(0);
        VEHICLE_NAME_TIMER.set(0);
        VEHICLE_FADE_TIMER.set(0);
        w32(LAST_VEHICLE_NAME, 0);
        return;
    }
    let mut state = VEHICLE_STATE.get() as i32;
    if !(0..=4).contains(&state) {
        draw_vehicle_name_original();
        return;
    }
    let mut last = g::<u32>(LAST_VEHICLE_NAME).read();
    if name != last {
        if state == 0 {
            let zs = crate::hud::ZONE_STATE.get();
            state = 2;
            VEHICLE_STATE.set(2);
            VEHICLE_NAME_TIMER.set(0);
            VEHICLE_FADE_TIMER.set(0);
            VEHICLE_NAME_TO_PRINT.set(name);
            if zs == 1 || zs == 2 {
                crate::hud::ZONE_STATE.set(3);
            }
        } else {
            state = 4;
            VEHICLE_STATE.set(4);
            VEHICLE_NAME_TIMER.set(0);
        }
        last = name;
        w32(LAST_VEHICLE_NAME, name);
    }
    if state == 0 {
        return;
    }
    let fade_alpha = || f32_of(xi(VEHICLE_FADE_TIMER.get() as i32).fmul_m(k(K_0_001)).fmul_m(k(K_255)));
    let alpha = match state {
        1 => {
            if !xi(VEHICLE_NAME_TIMER.get() as i32).fcom(x(k(K_3000))).test_any(0x41) {
                VEHICLE_STATE.set(3);
                VEHICLE_FADE_TIMER.set(1000);
            }
            k(K_255)
        }
        2 => {
            let f = (VEHICLE_FADE_TIMER.get() as i32).wrapping_add(wanted_step(K_1000));
            VEHICLE_FADE_TIMER.set(f as u32);
            if !xi(f).fcom(x(k(K_1000))).test_any(0x41) {
                VEHICLE_FADE_TIMER.set(1000);
                VEHICLE_STATE.set(1);
            }
            fade_alpha()
        }
        3 => {
            let f = (VEHICLE_FADE_TIMER.get() as i32).wrapping_add(wanted_step(K_M1000));
            VEHICLE_FADE_TIMER.set(f as u32);
            if f < 0 {
                VEHICLE_STATE.set(0);
                VEHICLE_FADE_TIMER.set(0);
            }
            fade_alpha()
        }
        _ => {
            let f = (VEHICLE_FADE_TIMER.get() as i32).wrapping_add(wanted_step(K_M1000));
            VEHICLE_FADE_TIMER.set(f as u32);
            if f < 0 {
                VEHICLE_NAME_TIMER.set(0);
                VEHICLE_STATE.set(2);
                VEHICLE_NAME_TO_PRINT.set(last);
                VEHICLE_FADE_TIMER.set(0);
            }
            fade_alpha()
        }
    };
    if b(MESSAGE_0) != 0 {
        return;
    }
    VEHICLE_NAME_TIMER.set(VEHICLE_NAME_TIMER.get().wrapping_add(wanted_step(K_1000) as u32));
    font_set_proportional(1);
    font_set_background(0, 0);
    let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_5))).to_bits();
    let w = f32_of(max_w().fmul_m(k(K_1_640)));
    font_set_scale_for_current_language(w, h);
    font_set_orientation(2);
    font_set_right_justify_wrap(0);
    font_set_font_style(2);
    font_set_edge(2);
    let a = u32::from(ftol32(x(alpha)) as u8);
    let mut tmp = [0u8; 4];
    let c = hud_colours_get_rgba(g(HUD_COLOUR), local(&mut tmp), 1, a);
    font_set_color(rd::<u32>(c, 0));
    let c = rgba_ctor(local(&mut tmp), 0, 0, 0, a);
    font_set_drop_color(rd::<u32>(c, 0));
    if b(SCRIPTS_DISPLAY_HUD) != 0 {
        let mh = max_h();
        let mw = max_w();
        let py = f32_of(mh.fmul_m(k(K_1_448)).fmul_m(k(K_104)).fsubr(mh));
        let px = f32_of(x(k(K_1_640)).fmul(mw).fmul_m(k(K_32)).fsubr(mw));
        font_print_string(px, py, g(VEHICLE_NAME_TO_PRINT.get() as usize));
    }
    font_set_slant(0);
}

/// `CHud::DrawAreaName()` (0x58AA50). State machine in `m_ZoneState` (1 shown, 2 fading in, 3 fading out,
/// 4 switching); the fades stop while the camera fades, and a fully faded-out screen (status 2) forces the
/// shown alpha. A new name in state 0 only starts while the HUD shows (and the player is on the map) or during
/// an enter/exit. While a brief or big message 1 / 2 shows the name is pushed to state 3. In gang wars the
/// zone's territory colour is used when all three components are non-zero.
#[hook(addr = 0x58AA50, conv = cdecl, name = "CHud::DrawAreaName", expect = "51 8B 15 D8 B1 BA 00")]
pub fn draw_area_name() {
    use crate::hud::{ZONE_NAME, ZONE_NAME_TIMER, ZONE_STATE};
    let name = ZONE_NAME.get();
    if name == 0 {
        return;
    }
    let mut state = ZONE_STATE.get();
    let mut last = g::<u32>(LAST_ZONE_NAME).read();
    if name != last {
        match state {
            0 => {
                let ees = g::<u32>(EXIT_ENTER_STATE).read();
                let start = (b(SCRIPTS_PLAYER_OFF_THE_MAP) == 0 && b(SCRIPTS_DISPLAY_HUD) != 0) || ees == 1 || ees == 2;
                if start {
                    let vs = crate::hud::VEHICLE_STATE.get();
                    state = 2;
                    ZONE_STATE.set(2);
                    ZONE_NAME_TIMER.set(0);
                    w32(ZONE_FADE_TIMER, 0);
                    w32(ZONE_TO_PRINT, name);
                    if vs == 1 || vs == 2 {
                        crate::hud::VEHICLE_STATE.set(3);
                    }
                }
            }
            1..=3 => {
                state = 4;
                ZONE_STATE.set(4);
                ZONE_NAME_TIMER.set(0);
            }
            4 => ZONE_NAME_TIMER.set(0),
            _ => {}
        }
        last = name;
        w32(LAST_ZONE_NAME, name);
    }
    if state == 0 {
        return;
    }
    let cam = g::<u8>(THE_CAMERA);
    let fade = || g::<i32>(ZONE_FADE_TIMER).read();
    let fade_alpha = || f32_of(xi(g::<i32>(ZONE_FADE_TIMER).read()).fmul_m(k(K_0_001)).fmul_m(k(K_255)));
    let mut alpha = k(K_255);
    match state {
        1 => {
            w32(ZONE_FADE_TIMER, 1000);
            if !xi(ZONE_NAME_TIMER.get() as i32).fcom(x(k(K_3000))).test_any(0x41) {
                ZONE_STATE.set(3);
                w32(ZONE_FADE_TIMER, 1000);
            }
        }
        2 => {
            if camera_get_fading(cam) == 0 && camera_get_screen_fade_status(cam) != 2 {
                w32(ZONE_FADE_TIMER, fade().wrapping_add(wanted_step(K_1000)) as u32);
            }
            if !xi(fade()).fcom(x(k(K_1000))).test_any(0x41) {
                w32(ZONE_FADE_TIMER, 1000);
                ZONE_STATE.set(1);
            }
            if camera_get_screen_fade_status(cam) == 2 {
                ZONE_STATE.set(3);
                w32(ZONE_FADE_TIMER, 1000);
            } else {
                alpha = fade_alpha();
            }
        }
        3 => {
            if camera_get_fading(cam) == 0 && camera_get_screen_fade_status(cam) != 2 {
                w32(ZONE_FADE_TIMER, fade().wrapping_add(wanted_step(K_M1000)) as u32);
            }
            if !xi(fade()).fcom(x(k(K_ZERO))).test_parity(0x05) {
                w32(ZONE_FADE_TIMER, 0);
                ZONE_STATE.set(0);
            }
            if camera_get_screen_fade_status(cam) == 2 {
                w32(ZONE_FADE_TIMER, 1000);
            } else {
                alpha = fade_alpha();
            }
        }
        4 => {
            let f = fade().wrapping_add(wanted_step(K_M1000));
            w32(ZONE_FADE_TIMER, f as u32);
            if !xi(f).fcom(x(k(K_ZERO))).test_parity(0x05) {
                w32(ZONE_FADE_TIMER, 0);
                ZONE_STATE.set(2);
                w32(ZONE_TO_PRINT, last);
            }
            alpha = fade_alpha();
        }
        _ => {}
    }
    if b(MESSAGE_0) != 0
        || x(g::<f32>(BIG_MESSAGE_X_1).read()).fcom(x(k(K_ZERO))).test_parity(0x44)
        || x(g::<f32>(BIG_MESSAGE_X_2).read()).fcom(x(k(K_ZERO))).test_parity(0x44)
    {
        ZONE_STATE.set(3);
        return;
    }
    ZONE_NAME_TIMER.set(ZONE_NAME_TIMER.get().wrapping_add(wanted_step(K_1000) as u32));
    font_set_proportional(1);
    font_set_background(0, 0);
    let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_9))).to_bits();
    let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_1_2)));
    font_set_scale_for_current_language(w, h);
    font_set_edge(2);
    font_set_orientation(2);
    font_set_right_justify_wrap(f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_180))).to_bits());
    let a = u32::from(ftol32(x(alpha)) as u8);
    let mut tmp = [0u8; 4];
    let c = rgba_ctor(local(&mut tmp), 0, 0, 0, a);
    font_set_drop_color(rd::<u32>(c, 0));
    font_set_font_style(0);
    let zi = g::<GPtr<u8>>(CURR_ZONE_INFO).read();
    let colour = if b(GANG_WARS_ACTIVE) != 0
        && !zi.is_null()
        && rd::<u8>(zi, 0xB) != 0
        && rd::<u8>(zi, 0xC) != 0
        && rd::<u8>(zi, 0xD) != 0
    {
        let (cb, cg, cr) = (rd::<u8>(zi, 0xD), rd::<u8>(zi, 0xC), rd::<u8>(zi, 0xB));
        let mut t2 = [0u8; 4];
        let c = rgba_ctor(local(&mut t2), u32::from(cr), u32::from(cg), u32::from(cb), a);
        rd::<u32>(c, 0)
    } else {
        let mut t2 = [0u8; 4];
        let c = hud_colours_get_rgba(g(HUD_COLOUR), local(&mut t2), 3, a);
        rd::<u32>(c, 0)
    };
    font_set_color(colour);
    let mh = max_h();
    let sy = x(k(K_1_448)).fmul(mh);
    let to_print = g::<u32>(ZONE_TO_PRINT).read();
    let mwf = f32_of(max_w());
    let py = f32_of(x(k(K_104)).fmul(sy).fsubr(mh).fadd(sy.fmul_m(k(K_76))));
    let px = f32_of(x(mwf).fmul_m(k(K_1_640)).fmul_m(k(K_32)).fsubr_m(mwf));
    font_print_string_from_bottom(px, py, g(to_print as usize));
    font_set_slant(0);
}

/// `CHud::DrawFadeState(DRAW_FADE_STATE item, int32 forceFadingIn)` (0x58D580, ST0): the fade state
/// machine of `DrawWanted` for item 0 wanted (0xBAA414/18/1C), 1 energy lost (0xBAA434/38/3C), 2 score
/// (0xBAA424/28/2C) or 3 weapon (0xBAA404/08/0C) as (state, fade timer, timer); returns the alpha clamped
/// to [0, 255]. For an item above 3 the original takes state, fade timer and timer from its own `item`
/// argument slot and stores nothing (as the bytes).
#[hook(addr = 0x58D580, conv = cdecl, name = "CHud::DrawFadeState", expect = "D9 05 AC 9A 85 00")]
pub fn draw_fade_state(item: u32, force: i32) -> X87 {
    let base = match item {
        0 => Some(0xBAA414usize),
        1 => Some(0xBAA434),
        2 => Some(0xBAA424),
        3 => Some(0xBAA404),
        _ => None,
    };
    let (mut state, mut fade, mut timer) = match base {
        Some(a) => (g::<i32>(a).read(), g::<i32>(a + 4).read(), g::<i32>(a + 8).read()),
        None => (item as i32, item as i32, item as i32),
    };
    let mut l = timer;
    let mut alpha = x(k(K_255));
    if force != 0 {
        match state {
            0 => {
                fade = 0;
                timer = 5;
                state = 2;
                l = 5;
            }
            1 | 3 => {
                timer = 5;
                state = 2;
                l = 5;
            }
            _ => {}
        }
    }
    if state != 0 && state != 5 {
        match state {
            1 => {
                fade = 1000;
                alpha = x(k(K_255));
                if !xi(l).fcom(x(k(K_10000))).test_any(0x41) {
                    state = 3;
                    fade = 3000;
                }
            }
            2 => {
                fade = fade.wrapping_add(wanted_step(K_1000));
                if !xi(fade).fcom(x(k(K_1000))).test_any(0x41) {
                    fade = 1000;
                    state = 1;
                }
                alpha = xi(fade).fmul_m(k(K_0_001)).fmul_m(k(K_255));
            }
            3 => {
                fade = fade.wrapping_add(wanted_step(K_M1000));
                if !xi(fade).fcom(x(k(K_ZERO))).test_parity(0x05) {
                    fade = 0;
                    state = 0;
                }
                alpha = xi(fade).fmul_m(k(K_0_001)).fmul_m(k(K_255));
            }
            _ => {}
        }
        timer = timer.wrapping_add(wanted_step(K_1000));
    }
    if let Some(a) = base {
        g::<i32>(a).write(state);
        g::<i32>(a + 8).write(timer);
        g::<i32>(a + 4).write(fade);
    }
    if !alpha.fcom(x(k(K_ZERO))).test_parity(0x05) {
        x(k(K_ZERO))
    } else if !alpha.fcom(x(k(K_255))).test_any(0x41) {
        x(k(K_255))
    } else {
        alpha
    }
}

/// The font setup shared by the odd-job texts: background off, (justify off), scale for the current
/// language `(maxW/640 * sw, maxH/448 * sh)`, left orientation, proportional, centre size `maxW/640 * cs`,
/// font style, edge 2, black drop colour and `HudColour.GetRGB(colour)`.
fn odd_job_font(justify: bool, sw: Option<u32>, sh: u32, cs: u32, style: u32, colour: u32) {
    font_set_background(0, 0);
    if justify {
        font_set_justify(0);
    }
    let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(sh))).to_bits();
    let w = match sw {
        Some(c) => f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(c))),
        None => f32_of(max_w().fmul_m(k(K_1_640))),
    };
    font_set_scale_for_current_language(w, h);
    font_set_orientation(0);
    font_set_proportional(1);
    font_set_centre_size(f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(cs))).to_bits());
    font_set_font_style(style);
    font_set_edge(2);
    let mut tmp = [0u8; 4];
    let c = rgba_ctor(local(&mut tmp), 0, 0, 0, 0xFF);
    font_set_drop_color(rd::<u32>(c, 0));
    let c = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut tmp), colour);
    font_set_color(rd::<u32>(c, 0));
}

/// `((float)(maxW / 2), maxH/448 * c)`
fn odd_job_pos(c: u32) -> (f32, f32) {
    let py = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(c)));
    let px = f32_of(xi(g::<i32>(RS_MAX_WIDTH).read() / 2));
    (px, py)
}

/// `CHud::DrawOddJobMessage(bool flag)` (0x58CC80): big message 4 (from the bottom, when `flag` equals
/// `bDrawOddJobTitleBeforeFade` and no mission title shows); with `flag` set also messages 6 and 3 and
/// the sliding message 5 (odd job 2). The odd-job-2 X offset is animated (states 0..3 in `OddJob2On`) but
/// not used for printing, as the bytes. One `mov al, [0xBAAD40]` sits in a protection chunk (0x409704).
#[hook(addr = 0x58CC80, conv = cdecl, name = "CHud::DrawOddJobMessage", expect = "A0 89 44 A4 00 53")]
pub fn draw_odd_job_message(flag: u8) {
    if flag == b(SCRIPTS_ODD_JOB_TITLE_BEFORE_FADE) && b(BIG_MESSAGE_1) == 0 && b(BIG_MESSAGE_4) != 0 {
        odd_job_font(true, Some(K_0_6), K_1_35, K_350, 2, 6);
        let (px, py) = odd_job_pos(K_140);
        font_print_string_from_bottom(px, py, g(BIG_MESSAGE_4));
    }
    if flag == 0 {
        return;
    }
    if b(BIG_MESSAGE_6) != 0 {
        odd_job_font(true, None, K_1_8, K_500, 3, 3);
        let (px, py) = odd_job_pos(K_60);
        font_print_string(px, py, g(BIG_MESSAGE_6));
    }
    if b(BIG_MESSAGE_3) != 0 {
        odd_job_font(true, Some(K_0_6), K_1_35, K_500, 2, 6);
        let (px, py) = odd_job_pos(K_155);
        font_print_string(px, py, g(BIG_MESSAGE_3));
    }
    if !x(g::<f32>(ODD_JOB2_OFF_TIMER).read()).fcom(x(k(K_ZERO))).test_any(0x41) {
        let v = step_ms_unsigned().fsubr_m(g::<f32>(ODD_JOB2_OFF_TIMER).read());
        g::<f32>(ODD_JOB2_OFF_TIMER).write(f32_of(v));
    }
    if b(BIG_MESSAGE_5) == 0 {
        return;
    }
    if x(g::<f32>(ODD_JOB2_OFF_TIMER).read()).fcom(x(k(K_ZERO))).test_parity(0x41) {
        return;
    }
    match g::<i16>(ODD_JOB2_ON).read() {
        0 => {
            g::<u32>(ODD_JOB2_X_OFFSET).write(0x43BE_0000);
            g::<i16>(ODD_JOB2_ON).write(1);
        }
        1 => {
            let xo = g::<f32>(ODD_JOB2_X_OFFSET).read();
            if x(xo).fcom(x(k(K_2))).test_any(0x41) {
                g::<i16>(ODD_JOB2_ON).write(2);
                g::<u16>(ODD_JOB2_TIMER).write(0);
            } else {
                let mut v = x(xo).fmul_m(k(K_1_6));
                if !v.fcom(x(k(K_40))).test_any(0x41) {
                    v = x(k(K_40));
                }
                let n = x(g::<f32>(ODD_JOB2_X_OFFSET).read()).fsub(v);
                g::<f32>(ODD_JOB2_X_OFFSET).write(f32_of(n));
            }
        }
        2 => {
            let t = g::<u16>(ODD_JOB2_TIMER).read().wrapping_add(wanted_step(K_1000) as u16);
            g::<u16>(ODD_JOB2_TIMER).write(t);
            if t as i16 > 0x5DC {
                g::<i16>(ODD_JOB2_ON).write(3);
            }
        }
        3 => {
            let mut v = x(g::<f32>(ODD_JOB2_X_OFFSET).read()).fmul_m(k(K_0_2));
            if !v.fcom(x(k(K_30))).test_parity(0x05) {
                v = x(k(K_30));
            }
            let n = v.fsubr_m(g::<f32>(ODD_JOB2_X_OFFSET).read());
            g::<f32>(ODD_JOB2_X_OFFSET).write(f32_of(n));
            if !n.fcom(x(k(K_M380))).test_parity(0x05) {
                g::<i16>(ODD_JOB2_ON).write(0);
                g::<u32>(ODD_JOB2_OFF_TIMER).write(0x459C_4000);
            }
        }
        _ => {}
    }
    if b(BIG_MESSAGE_1) != 0 {
        return;
    }
    odd_job_font(false, Some(K_0_6), K_1_35, K_500, 2, 4);
    let (px, py) = odd_job_pos(K_217);
    font_print_string(px, py, g(BIG_MESSAGE_5));
}

/// `GetYPosBasedOnHealth(1, (float)GetYPosBasedOnHealth(focus, y, 12), 12)` (the chained call of
/// `DrawMissionTimers`; the focus is passed as the byte the original loads into BL).
fn timers_y_chain(y: f32) -> f32 {
    let focus = b(PLAYER_IN_FOCUS);
    let y1 = f32_of(get_y_pos_based_on_health(focus, y, 12));
    f32_of(get_y_pos_based_on_health(1, y1, 12))
}

/// Advances a flash hide state: 0 stays 0, else +1 and back to 0 above 50 (int16).
fn advance_hide_state(addr: usize) {
    let v = g::<i16>(addr).read();
    if v != 0 {
        let n = v.wrapping_add(1);
        g::<i16>(addr).write(n);
        if n > 0x32 {
            g::<i16>(addr).write(0);
        }
    }
}

/// `x = maxW - 32 * sx - 90 * sx` (register order) for the timer and counter keys.
fn timers_key_x() -> f32 {
    let mw = max_w();
    let sx = x(k(K_1_640)).fmul(mw);
    f32_of(x(k(K_32)).fmul(sx).fsubr(mw).fsub(sx.fmul_m(k(K_90))))
}

/// `x = maxW - 32 * sx`
fn timers_text_x() -> f32 {
    let mw = max_w();
    f32_of(x(k(K_1_640)).fmul(mw).fmul_m(k(K_32)).fsubr(mw))
}

/// `CHud::DrawMissionTimers()` (0x58B180): the on-screen clock and the four counters (`OnscnTimer`).
/// Byte findings: the base Y is `GetYPosBasedOnHealth` chained for the focused player and player 1;
/// without the clock it is recomputed with the inlined health test of **player 2's** max health
/// (0xB7D077); a counter's row offset is `20 * sy * i * sy` (`sy` applied twice); bar counters print
/// `atol(text)` as `(int16)value * 0.01 * 100` percent.
#[hook(addr = 0x58B180, conv = cdecl, name = "CHud::DrawMissionTimers", expect = "A0 C0 AE BA 00 81 EC F0 00 00 00")]
pub fn draw_mission_timers() {
    if b(BIG_MESSAGE_4) != 0 && b(FORCE_DISPLAY_WITH_COUNTERS) == 0 {
        return;
    }
    if b(GARAGES_MESSAGE_ID) != 0 {
        return;
    }
    let syf = f32_of(max_h().fmul_m(k(K_1_448)));
    let yb = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_148)));
    let mut ya = timers_y_chain(yb);
    let mut yb2 = timers_y_chain(f32_of(x(syf).fmul_m(k(K_20)).fadd_m(yb)));
    if g::<u32>(PLAYER2_PED).read() != 0 {
        let v = x(syf).fmul_m(k(K_72));
        ya = f32_of(x(ya).fadd(v));
        yb2 = f32_of(v.fadd_m(yb2));
    }
    font_set_proportional(1);
    font_set_background(0, 0);
    let h = f32_of(max_h().fmul_m(k(K_1_448)));
    let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_HALF)));
    font_set_scale(w.to_bits(), h.to_bits());
    font_set_orientation(2);
    font_set_right_justify_wrap(0);
    font_set_font_style(2);
    font_set_wrapx(f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_640))).to_bits());
    font_set_edge(2);
    let mut tmp = [0u8; 4];
    let c = rgba_ctor(local(&mut tmp), 0, 0, 0, 0xFF);
    font_set_drop_color(rd::<u32>(c, 0));
    let p2 = g::<u32>(PLAYER2_PED).read();
    let clock = b(TIMER_CLOCK_ENABLED);
    if p2 != 0 && clock == 0 {
        w8(TIMER_MAIN_WAS_DISPLAYED, 0);
    }
    for i in 0..4 {
        if b(TIMER_COUNTERS + 0x3A + i * 0x44) == 0 {
            w8(TIMER_COUNTER_WAS_DISPLAYED + i, 0);
        }
    }
    if b(TIMER_DISPLAY) != 1 {
        return;
    }
    if clock == 1 {
        if b(TIMER_MAIN_WAS_DISPLAYED) == 0 {
            g::<i16>(TIMER_MAIN_HIDE_STATE).write(1);
        }
        w8(TIMER_MAIN_WAS_DISPLAYED, 1);
        advance_hide_state(TIMER_MAIN_HIDE_STATE);
        let c = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut tmp), 4);
        font_set_color(rd::<u32>(c, 0));
        if b(FRAME_COUNTER) & 4 != 0 || g::<i16>(TIMER_MAIN_HIDE_STATE).read() == 0 {
            let mut buf = [0u8; 0xC0];
            ascii_to_gxt_char(g(TIMER_CLOCK_TEXT), local(&mut buf));
            font_print_string(timers_text_x(), ya, local(&mut buf));
            if b(TIMER_CLOCK_KEY) != 0 {
                let text = text_get(g(THE_TEXT), g(TIMER_CLOCK_KEY));
                font_print_string(timers_key_x(), ya, text);
            }
        }
    } else {
        let focus = b(PLAYER_IN_FOCUS);
        let syf2 = f32_of(max_h().fmul_m(k(K_1_448)));
        let mut r = get_y_pos_based_on_health(focus, f32_of(x(syf2).fmul_m(k(K_148))), 12);
        let mh2 = i32::from(b(PLAYER2_MAX_HEALTH));
        if !xi(mh2).fcom(x(k(K_101))).test_parity(0x05) {
            r = r.fsub(x(syf2).fmul_m(k(K_12)));
        }
        yb2 = f32_of(r);
        if p2 != 0 {
            yb2 = f32_of(x(syf2).fmul_m(k(K_72)).fadd(r));
        }
    }
    for i in 0..4usize {
        let e = TIMER_COUNTERS + i * 0x44;
        if b(e + 0x3A) != 1 {
            continue;
        }
        let hide = TIMER_COUNTER_HIDE_STATE + i * 2;
        if b(TIMER_COUNTER_WAS_DISPLAYED + i) == 0 && b(e + 0x3B) == 1 {
            g::<i16>(hide).write(1);
        }
        w8(TIMER_COUNTER_WAS_DISPLAYED + i, 1);
        advance_hide_state(hide);
        if b(FRAME_COUNTER) & 4 == 0 && g::<i16>(hide).read() != 0 {
            continue;
        }
        let colour = u32::from(b(e + 0x3C));
        let c = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut tmp), colour);
        font_set_color(rd::<u32>(c, 0));
        let sy = max_h().fmul_m(k(K_1_448));
        let yi = f32_of(x(k(K_20)).fmul(sy).fmul(xi(i as i32)).fmul(sy).fadd_m(yb2));
        if g::<u16>(e + 0xE).read() != 1 {
            let mut buf = [0u8; 0xC0];
            ascii_to_gxt_char(g(e + 0x10), local(&mut buf));
            font_print_string(timers_text_x(), yi, local(&mut buf));
        } else {
            let v = crt_atol(g(e + 0x10));
            let mut add = [0u8; 4];
            let ap = rgba_ctor(local(&mut add), 0, 0, 0, 0);
            let mut col = [0u8; 4];
            let cp = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut col), colour);
            let sy = max_h().fmul_m(k(K_1_448));
            let mw = max_w();
            let sx = x(k(K_1_640)).fmul(mw);
            let c61 = x(k(K_61)).fmul(sx);
            let progress = f32_of(xi(i32::from(v as i16)).fmul_m(k(K_0_01)).fmul_m(k(K_100)));
            let bh = ftol32(sy.fmul_m(k(K_9))) as u32;
            let bw = ftol32(c61) as u32;
            let by = f32_of(sy.fmul_m(k(K_6)).fadd_m(yi));
            let bx = f32_of(sx.fmul_m(k(K_32)).fsubr(mw).fsub(c61));
            sprite2d_draw_bar_chart(bx, by, bw, bh, progress, 0, 0, 1, rd::<u32>(cp, 0), rd::<u32>(ap, 0));
        }
        if b(e + 4) != 0 {
            let text = text_get(g(THE_TEXT), g(e + 4));
            font_print_string(timers_key_x(), yi, text);
        }
    }
}

/// `CHud::Initialise()` (0x5BA850): loads `MODELS\HUD.TXD` into a new `"hud"` slot, sets the six sprites
/// from the name / mask table at 0x8D128C, then `ReInitialise` (a tail jump in the original).
#[hook(addr = 0x5BA850, conv = cdecl, name = "CHud::Initialise", expect = "53 56 57 68 6C 69 86 00")]
pub fn initialise() {
    let slot = add_txd_slot(g(STR_HUD));
    load_txd(slot, g(STR_HUD_TXD));
    txd_add_ref(slot);
    push_current_txd();
    set_current_txd(slot);
    for i in 0..6usize {
        let name = g::<u32>(SPRITE_NAMES + i * 8).read();
        let mask = g::<u32>(SPRITE_NAMES + i * 8 + 4).read();
        sprite2d_set_texture(g(SPRITES + i * 4), g(name as usize), g(mask as usize));
    }
    pop_current_txd();
    re_initialise_call();
}

/// `CHudColours::CHudColours()` (0x58FDA0, thiscall): constructs the 15 `CRGBA` entries through the
/// runtime's `eh vector constructor iterator` (ctor 0x463360, dtor 0x43CE70), then the colour table;
/// returns `this`. Entry 14's alpha (+0x3B) is 0xFF and its blue (+0x3A) 0, as the bytes.
#[hook(addr = 0x58FDA0, conv = thiscall, name = "CHudColours::Constructor", expect = "56 68 70 CE 43 00", this = "CHudColours")]
pub fn hud_colours_constructor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor_iterator(this, 4, 0xF, 0x463360, 0x43CE70);
    const TABLE: [u8; 0x3C] = [
        0xB4, 0x19, 0x1D, 0xFF, 0x36, 0x68, 0x2C, 0xFF, 0x32, 0x3C, 0x7F, 0xFF, 0xAC, 0xCB, 0xF1, 0xFF, //
        0xE1, 0xE1, 0xE1, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x90, 0x62, 0x10, 0xFF, 0xA8, 0x6E, 0xFC, 0xFF, //
        0x96, 0x96, 0x96, 0xFF, 0x68, 0x0F, 0x11, 0xFF, 0x26, 0x47, 0x1F, 0xFF, 0xE2, 0xC0, 0x63, 0xFF, //
        0x4A, 0x5A, 0x6B, 0xFF, 0x14, 0x19, 0xC8, 0xFF, 0xFF, 0xFF, 0x00, 0xFF,
    ];
    for (i, v) in TABLE.iter().enumerate() {
        this.byte_add(i as isize).write(*v);
    }
    this
}

/// `(f - ftol(step)) ...`: the help box fade step, `2 * ftol(CTimer::ms_fTimeStep * 0.02 * 1000)`.
fn help_fade_step() -> i32 {
    wanted_step(K_1000)
}

/// `CHud::DrawHelpText()` (0x58B6E0): the help box (state machine in `m_nHelpMessageState`: 1 shown,
/// 2 fading in, 3 fading out, 4 switching) and the stat-update bar. Byte findings: a new message plays
/// front-end audio event 0x20 and sizes the display time as `GetNumberLines + 3` seconds; the box wrap and
/// Y follow the globals 0x8D0934 / 0x8D0938; the stat bar's Y is `(155 - [0x8D0938]) * 0.6 + 28 sy`
/// (no `sy` on the first term, unlike the texts), its percentage is `max(value / max * 100, 2)` with
/// `max(update / max * 100, 3)` as the progress-add byte, no border; the alpha fade is reset to 255 on
/// exit.
#[hook(addr = 0x58B6E0, conv = cdecl, name = "CHud::DrawHelpText", expect = "A0 A0 A7 BA 00 83 EC 18")]
pub fn draw_help_text() {
    use crate::hud::{
        HELP_MESSAGE, HELP_MESSAGE_FADE_TIMER, HELP_MESSAGE_MAX_STAT_VALUE, HELP_MESSAGE_PERMANENT,
        HELP_MESSAGE_QUICK, HELP_MESSAGE_STATE, HELP_MESSAGE_STAT_ID, HELP_MESSAGE_STAT_UPDATE_VALUE,
        HELP_MESSAGE_TIME, HELP_MESSAGE_TIMER, HELP_MESSAGE_TO_PRINT, LAST_HELP_MESSAGE,
    };
    if help_buf(&HELP_MESSAGE).read() == 0 {
        HELP_MESSAGE_STATE.set(0);
        return;
    }
    if msg_string_compare(help_buf(&HELP_MESSAGE), help_buf(&LAST_HELP_MESSAGE), 0x190) == 0 {
        match HELP_MESSAGE_STATE.get() {
            0 => {
                HELP_MESSAGE_STATE.set(2);
                HELP_MESSAGE_TIMER.set(0);
                HELP_MESSAGE_FADE_TIMER.set(0);
                msg_string_copy(help_buf(&HELP_MESSAGE_TO_PRINT), help_buf(&HELP_MESSAGE), 0x190);
                font_set_orientation(1);
                font_set_justify(0);
                let sx = max_w().fmul_m(k(K_1_640));
                let wx = x(k(K_34)).fmul(sx).fadd(sx.fmul_m(k(K_200))).fsub(sx.fmul_m(k(K_4)));
                font_set_wrapx(f32_of(wx).to_bits());
                font_set_font_style(1);
                font_set_background(1, 1);
                font_set_drop_shadow_position(0);
                let ly = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_28)));
                let lx = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_34)));
                let lines = font_get_number_lines(lx, ly, help_buf(&HELP_MESSAGE_TO_PRINT));
                let secs = i32::from(lines as i16).wrapping_add(3);
                HELP_MESSAGE_TIME.set(f32_of(xi(secs)).to_bits());
                font_set_wrapx(f32_of(max_w()).to_bits());
                audio_report_frontend_event(g(AUDIO_ENGINE), 0x20, 0, 0x3F80_0000);
            }
            1..=4 => {
                HELP_MESSAGE_STATE.set(4);
                HELP_MESSAGE_TIMER.set(5);
            }
            _ => {}
        }
        msg_string_copy(help_buf(&LAST_HELP_MESSAGE), help_buf(&HELP_MESSAGE), 0x190);
    }
    let state = HELP_MESSAGE_STATE.get();
    if state == 0 {
        return;
    }
    let fade = || HELP_MESSAGE_FADE_TIMER.get() as i32;
    let fade_alpha = || f32_of(xi(HELP_MESSAGE_FADE_TIMER.get() as i32).fmul_m(k(K_0_001)).fmul_m(k(K_200)));
    let mut alpha = k(K_200);
    match state {
        1 => {
            HELP_MESSAGE_FADE_TIMER.set(600);
            if HELP_MESSAGE_PERMANENT.get() == 0 {
                let t = xi(HELP_MESSAGE_TIMER.get() as i32);
                let lim = x(f32::from_bits(HELP_MESSAGE_TIME.get())).fmul_m(k(K_1000));
                let expired = if !t.fcom(lim).test_any(0x41) {
                    true
                } else {
                    HELP_MESSAGE_QUICK.get() != 0 && !t.fcom(x(k(K_3000))).test_any(0x41)
                };
                if expired {
                    HELP_MESSAGE_STATE.set(3);
                    HELP_MESSAGE_FADE_TIMER.set(600);
                }
            }
        }
        2 => {
            if b(WIDE_SCREEN_ON) == 0 {
                let f = fade().wrapping_add(help_fade_step().wrapping_mul(2));
                HELP_MESSAGE_FADE_TIMER.set(f as u32);
                if !xi(f).fcom(x(k(K_ZERO))).test_any(0x41) {
                    HELP_MESSAGE_FADE_TIMER.set(0);
                    HELP_MESSAGE_STATE.set(1);
                }
                alpha = fade_alpha();
            }
        }
        3 => {
            let f = fade().wrapping_add(help_fade_step().wrapping_neg().wrapping_mul(2));
            HELP_MESSAGE_FADE_TIMER.set(f as u32);
            if !xi(f).fcom(x(k(K_ZERO))).test_parity(0x05) || b(WIDE_SCREEN_ON) != 0 {
                HELP_MESSAGE_FADE_TIMER.set(0);
                HELP_MESSAGE_STATE.set(0);
            }
            alpha = fade_alpha();
        }
        4 => {
            let f = fade().wrapping_add(help_fade_step().wrapping_neg().wrapping_mul(2));
            HELP_MESSAGE_FADE_TIMER.set(f as u32);
            if !xi(f).fcom(x(k(K_ZERO))).test_parity(0x05) {
                HELP_MESSAGE_FADE_TIMER.set(0);
                HELP_MESSAGE_STATE.set(2);
                msg_string_copy(help_buf(&HELP_MESSAGE_TO_PRINT), help_buf(&LAST_HELP_MESSAGE), 0x190);
            }
            alpha = fade_alpha();
        }
        _ => {}
    }
    if b(CUTSCENE_RUNNING) != 0 {
        return;
    }
    HELP_MESSAGE_TIMER.set(HELP_MESSAGE_TIMER.get().wrapping_add(wanted_step(K_1000) as u32));
    font_set_alpha_fade(alpha.to_bits());
    font_set_proportional(1);
    let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_1))).to_bits();
    let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_52)));
    font_set_scale_for_current_language(w, h);
    let stat = HELP_MESSAGE_STAT_ID.get();
    let mut tmp = [0u8; 4];
    if stat == 0 {
        if b(BIG_MESSAGE_0) != 0 || b(BIG_MESSAGE_4) != 0 || b(GARAGES_MESSAGE_ID) != 0 {
            font_set_alpha_fade(0x437F_0000);
            return;
        }
        font_set_orientation(1);
        font_set_justify(0);
        let sx = max_w().fmul_m(k(K_1_640));
        let s200 = x(k(K_200)).fmul(sx);
        let gw = g::<f32>(HELP_BOX_WIDTH).read();
        let wx = if sx.fmul_m(gw).fcom(s200).test_parity(0x44) {
            x(gw).fsub_m(k(K_4)).fmul(sx).fadd(sx.fmul_m(k(K_34)))
        } else {
            sx.fmul_m(k(K_34)).fadd(s200).fsub(sx.fmul_m(k(K_4)))
        };
        font_set_wrapx(f32_of(wx).to_bits());
        font_set_font_style(1);
        font_set_background(1, 1);
        font_set_drop_shadow_position(0);
        let a = ftol32(x(alpha)) as u32;
        let c = rgba_ctor(local(&mut tmp), 0, 0, 0, a);
        font_set_background_color(rd::<u32>(c, 0));
        let c = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut tmp), 4);
        font_set_color(rd::<u32>(c, 0));
        let mut off: u8 = 0;
        if b(WIDE_SCREEN_ON) != 0 && b(MENU_WIDESCREEN_ON) == 0 {
            off = 0x38;
        }
        let v = i32::from(off) + 0x96;
        let sy = max_h().fmul_m(k(K_1_448));
        let py = f32_of(
            xi(v)
                .fsub_m(g::<f32>(HELP_BOX_Y).read())
                .fmul_m(k(K_0_6))
                .fmul(sy)
                .fadd(sy.fmul_m(k(K_28))),
        );
        let px = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_34)));
        font_print_string(px, py, help_buf(&HELP_MESSAGE_TO_PRINT));
        font_set_wrapx(f32_of(max_w()).to_bits());
        font_set_alpha_fade(0x437F_0000);
        return;
    }
    if b(WIDE_SCREEN_ON) != 0 {
        font_set_alpha_fade(0x437F_0000);
        return;
    }
    let fmt = if stat < 10 {
        STR_STAT00
    } else if stat < 100 {
        STR_STAT0
    } else {
        STR_STAT
    };
    sprintf1(g(G_STRING), g(fmt), i32::from(stat));
    font_set_orientation(1);
    font_set_justify(0);
    font_set_wrapx(f32_of(max_w()).to_bits());
    font_set_font_style(1);
    font_set_background(1, 1);
    font_set_drop_shadow_position(0);
    let a8 = u32::from(ftol32(x(alpha)) as u8);
    let c = rgba_ctor(local(&mut tmp), 0, 0, 0, a8);
    font_set_background_color(rd::<u32>(c, 0));
    let c = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut tmp), 4);
    font_set_color(rd::<u32>(c, 0));
    let sxf = f32_of(max_w().fmul_m(k(K_1_640)));
    let name = text_get(g(THE_TEXT), g(G_STRING));
    let sw = font_get_string_width(name, 1, 0);
    let x1 = f32_of(sw.fadd(x(sxf).fmul_m(k(K_34))).fadd(x(sxf).fmul_m(k(K_10))));
    font_set_wrapx(f32_of(x(sxf).fmul_m(k(K_75)).fadd_m(x1)).to_bits());
    let syf = f32_of(max_h().fmul_m(k(K_1_448)));
    let name = text_get(g(THE_TEXT), g(G_STRING));
    let py = f32_of(
        x(k(K_150))
            .fsub_m(g::<f32>(HELP_BOX_Y).read())
            .fmul_m(k(K_0_6))
            .fmul_m(syf)
            .fadd(x(syf).fmul_m(k(K_28))),
    );
    let px = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_34)));
    font_print_string(px, py, name);
    ascii_to_gxt_char(g(STR_PLUS), g(G_GXT_STRING));
    let value = if HELP_MESSAGE_STAT_ID.get() == 0x150 {
        let ped = sa_world::world::common::find_player_ped(-1);
        let group = rd::<i32>(rd::<GPtr<u8>>(ped, 0x480), 0x38);
        let n = group_count_members_excluding_leader(g(PED_GROUP_MEMBERSHIP.wrapping_add(group.wrapping_mul(0x2D4) as usize)));
        xi(n)
    } else {
        stats_get_stat_value(u32::from(HELP_MESSAGE_STAT_ID.get()))
    };
    let value = f32_of(value);
    let plus = help_buf(&HELP_MESSAGE_TO_PRINT).read() == b(G_GXT_STRING);
    let mut c1 = [0u8; 4];
    let add = hud_colours_get_rgba(g(HUD_COLOUR), local(&mut c1), if plus { 1 } else { 0 }, a8);
    let mut c2 = [0u8; 4];
    let col = hud_colours_get_rgba(g(HUD_COLOUR), local(&mut c2), 4, a8);
    let max_stat = i32::from(HELP_MESSAGE_MAX_STAT_VALUE.get());
    let (add_c, col_c) = (rd::<u32>(add, 0), rd::<u32>(col, 0));
    let inv = f32_of(xi(max_stat).fdivr_m(k(K_1)));
    let syf = f32_of(max_h().fmul_m(k(K_1_448)));
    let upd = f32::from_bits(HELP_MESSAGE_STAT_UPDATE_VALUE.get());
    let pa = f32_of(x(inv).fmul_m(upd).fmul_m(k(K_100)));
    let progress_add = ftol32(max_f(pa, f32::from_bits(0x4040_0000))) as u32;
    let pv = f32_of(x(inv).fmul_m(value).fmul_m(k(K_100)));
    let progress = f32_of(max_f(pv, f32::from_bits(0x4000_0000)));
    let bh = ftol32(x(syf).fmul_m(k(K_12))) as u32;
    let bw = ftol32(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_62))) as u32;
    let by = f32_of(x(k(K_155)).fsub_m(g::<f32>(HELP_BOX_Y).read()).fmul_m(k(K_0_6)).fadd(x(syf).fmul_m(k(K_28))));
    sprite2d_draw_bar_chart(x1, by, bw, bh, progress, progress_add, 0, 0, col_c, add_c);
    let sy = max_h().fmul_m(k(K_1_448));
    let py = f32_of(
        x(k(K_150))
            .fsub_m(g::<f32>(HELP_BOX_Y).read())
            .fmul_m(k(K_0_6))
            .fmul(sy)
            .fadd(sy.fmul_m(k(K_28))),
    );
    let px = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_65)).fadd_m(x1));
    font_print_string(px, py, help_buf(&HELP_MESSAGE_TO_PRINT));
    font_set_wrapx(f32_of(max_w()).to_bits());
    font_set_alpha_fade(0x437F_0000);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_DRAW, HOOK_DRAW_AFTER_FADE, HOOK_DRAW_WEAPON_ICON, HOOK_DRAW_AMMO, HOOK_DRAW_RADAR, HOOK_DRAW_WANTED,
    HOOK_RENDER_ARMOR_BAR,
    HOOK_RENDER_BREATH_BAR,
    HOOK_RENDER_HEALTH_BAR,
    HOOK_SET_HELP_MESSAGE,
    HOOK_SET_HELP_MESSAGE_STAT_UPDATE,
    HOOK_SET_HELP_MESSAGE_WITH_NUMBER,
    HOOK_SHUTDOWN,
    HOOK_RE_INITIALISE,
    HOOK_DRAW_SCRIPT_TEXT,
    HOOK_DRAW_MISSION_TITLE,
    HOOK_DRAW_BUSTED_WASTED_MESSAGE,
    HOOK_DRAW_SUCCESS_FAILED_MESSAGE,
    HOOK_DRAW_TRIP_SKIP,
    HOOK_DRAW_SUBTITLES,
    HOOK_DRAW_VEHICLE_NAME,
    HOOK_DRAW_AREA_NAME,
    HOOK_DRAW_FADE_STATE,
    HOOK_DRAW_ODD_JOB_MESSAGE,
    HOOK_DRAW_MISSION_TIMERS,
    HOOK_INITIALISE,
    HOOK_HUD_COLOURS_CONSTRUCTOR,
    HOOK_DRAW_HELP_TEXT,
];

/// [`min_f_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
fn min_f(a: f32, bb: f32) -> X87 {
    X87::from_f64_exact(min_f_st0(a, bb))
}

/// The Rust path of [`get_y_pos_based_on_health`] as an ST0 value.
#[allow(dead_code)]
fn get_y_pos_based_on_health_st0_rust(player: u8, y: f32, offset: i8) -> f64 {
    crate::hud::get_y_pos_based_on_health(player, y, offset).to_f64()
}

/// [`get_y_pos_based_on_health_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
fn get_y_pos_based_on_health(player: u8, y: f32, offset: i8) -> X87 {
    X87::from_f64_exact(get_y_pos_based_on_health_st0(player, y, offset))
}

/// [`stats_get_fat_and_muscle_modifier_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
fn stats_get_fat_and_muscle_modifier(stat: u32) -> X87 {
    X87::from_f64_exact(stats_get_fat_and_muscle_modifier_st0(stat))
}

/// [`font_get_string_width_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
fn font_get_string_width(text: GPtr<u8>, full: u32, script: u32) -> X87 {
    X87::from_f64_exact(font_get_string_width_st0(text, full, script))
}

/// [`stats_get_stat_value_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
fn stats_get_stat_value(stat: u32) -> X87 {
    X87::from_f64_exact(stats_get_stat_value_st0(stat))
}

/// [`max_f_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
fn max_f(a: f32, bb: f32) -> X87 {
    X87::from_f64_exact(max_f_st0(a, bb))
}
