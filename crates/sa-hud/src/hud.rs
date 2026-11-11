//! `CHud` message and name state, `CHudColours` lookups, `CLocalisation` and `CUserDisplay` (PORT-HUD).
//! Ported from the 1.0 US bytes; gta-reversed `Hud.cpp`, `HudColours.cpp`, `Localisation.cpp`,
//! `UserDisplay.cpp` as reading aids. The drawing functions (`CHud::Draw*`) stay original in this pass.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `CHud::m_pVehicleNameToPrint`
    pub static VEHICLE_NAME_TO_PRINT @ 0xBAA444: u32 = 0;
    /// `CHud::m_VehicleState`
    pub static VEHICLE_STATE @ 0xBAA448: u32 = 0;
    /// `CHud::m_VehicleFadeTimer`
    pub static VEHICLE_FADE_TIMER @ 0xBAA44C: u32 = 0;
    /// `CHud::m_VehicleNameTimer`
    pub static VEHICLE_NAME_TIMER @ 0xBAA450: u32 = 0;
    /// `CHud::m_pVehicleName`
    pub static VEHICLE_NAME @ 0xBAA458: u32 = 0;
    /// `CHud::m_fHelpMessageTime`
    pub static HELP_MESSAGE_TIME @ 0xBAA460: u32 = 0;
    /// `CHud::m_bHelpMessagePermanent`
    pub static HELP_MESSAGE_PERMANENT @ 0xBAA464: u8 = 0;
    /// `CHud::m_fHelpMessageStatUpdateValue`
    pub static HELP_MESSAGE_STAT_UPDATE_VALUE @ 0xBAA468: u32 = 0;
    /// `CHud::m_nHelpMessageMaxStatValue`
    pub static HELP_MESSAGE_MAX_STAT_VALUE @ 0xBAA46C: u16 = 0;
    /// `CHud::m_nHelpMessageStatId`
    pub static HELP_MESSAGE_STAT_ID @ 0xBAA470: u16 = 0;
    /// `CHud::m_bHelpMessageQuick`
    pub static HELP_MESSAGE_QUICK @ 0xBAA472: u8 = 0;
    /// `CHud::m_nHelpMessageState`
    pub static HELP_MESSAGE_STATE @ 0xBAA474: u32 = 0;
    /// `CHud::m_nHelpMessageFadeTimer`
    pub static HELP_MESSAGE_FADE_TIMER @ 0xBAA478: u32 = 0;
    /// `CHud::m_nHelpMessageTimer`
    pub static HELP_MESSAGE_TIMER @ 0xBAA47C: u32 = 0;
    /// `CHud::m_pHelpMessageToPrint[400]`
    pub static HELP_MESSAGE_TO_PRINT @ 0xBAA480: [u8; 400] = [0; 400];
    /// `CHud::m_pLastHelpMessage[400]`
    pub static LAST_HELP_MESSAGE @ 0xBAA610: [u8; 400] = [0; 400];
    /// `CHud::m_pHelpMessage[400]`
    pub static HELP_MESSAGE @ 0xBAA7A0: [u8; 400] = [0; 400];
    /// `CHud::m_ZoneState`
    pub static ZONE_STATE @ 0xBAA930: u32 = 0;
    /// `CHud::m_ZoneNameTimer`
    pub static ZONE_NAME_TIMER @ 0xBAA938: u32 = 0;
    /// `CHud::m_Message[400]`
    pub static MESSAGE @ 0xBAB040: [u8; 400] = [0; 400];
    /// `CHud::m_pZoneName`
    pub static ZONE_NAME @ 0xBAB1D8: u32 = 0;
    /// `CHud::OddJob2OffTimer`
    pub static ODD_JOB2_OFF_TIMER @ 0xBAA398: u32 = 0;
    /// `CHud::OddJob2On`
    pub static ODD_JOB2_ON @ 0xBAB1E0: u16 = 0;
    /// `CHud::BigMessageX[7]`
    pub static BIG_MESSAGE_X @ 0xBAA3DC: [f32; 7] = [0.0; 7];
    /// `CLocalisation::nastyGame`
    pub static NASTY_GAME @ 0xB9B7EC: u8 = 0;
    /// `CLocalisation::germanGame`
    pub static GERMAN_GAME @ 0xB9B7ED: u8 = 0;
    /// `CLocalisation::frenchGame`
    pub static FRENCH_GAME @ 0xB9B7EE: u8 = 0;
    /// `CUserDisplay::PlaceName` (`CPlaceName`: +0 zone pointer, +4 int16 timer)
    pub static PLACE_NAME @ 0xBA18F4: [u8; 8] = [0; 8];
    /// `CCredits::CreditsStartTime`
    pub static CREDITS_START_TIME @ 0xC6E978: u32 = 0;
    /// `CCredits::bCreditsGoing`
    pub static CREDITS_ON @ 0xC6E97C: u8 = 0;
    /// `CUserDisplay::CurrentVehicle` (`CCurrentVehicle`: +0 vehicle pointer)
    pub static CURRENT_VEHICLE @ 0xBA18FC: u32 = 0;
}

/// `CHud::m_BigMessage[7][0x80]` (0xBAACC0; byte 0xBAAD40 is declared by sa-script as `BIG_MESSAGE_1`, so the
/// array is reached through its address).
pub const BIG_MESSAGE: usize = 0xBAACC0;
/// `CHud::LastBigMessage[7][0x80]`
pub const LAST_BIG_MESSAGE: usize = 0xBAA940;
/// `CUserDisplay::OnscnTimer` (`COnscreenTimer`, owned by the timer module)
pub const ONSCN_TIMER: usize = 0xBA1788;
/// `CWorld::Players[]` (stride 0x190); `m_nMaxHealth` at +0x14F
const PLAYERS: usize = 0xB7CD98;

orig! {
    /// `COnscreenTimer::Init()` (0x44CBC0)
    pub fn onscreen_timer_init(this: GPtr<u8>) = thiscall 0x44CBC0 => unported;
    /// `COnscreenTimer::Process()` (0x44CD30)
    pub fn onscreen_timer_process(this: GPtr<u8>) = thiscall 0x44CD30 => unported;
    /// `CPlaceName::Process()` (0x571F20)
    pub fn place_name_process(this: GPtr<u8>) = thiscall 0x571F20 => unported;
    /// `CCurrentVehicle::Process()` (0x572040)
    pub fn current_vehicle_process(this: GPtr<u8>) = thiscall 0x572040 => unported;
    /// `CRGBA::CRGBA(uint8 r, uint8 g, uint8 b, uint8 a)` (0x7170C0)
    pub fn rgba_ctor(this: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
}

fn big(style: u32, i: u32) -> GPtr<u8> {
    g::<u8>(BIG_MESSAGE + (style as usize) * 0x80 + i as usize)
}

fn last_big(style: u32, i: u32) -> GPtr<u8> {
    g::<u8>(LAST_BIG_MESSAGE + (style as usize) * 0x80 + i as usize)
}

/// `fld [x]; fcomp [0.0]; test ah, 0x44; jp` not taken: `x == 0.0` (ordered).
fn is_zero(v: f32) -> bool {
    !x(v).fcom(x(0.0)).test_parity(0x44)
}

/// `CHud::SetVehicleName(const GxtChar*)` (0x588F50)
#[hook(addr = 0x588F50, conv = cdecl, name = "CHud::SetVehicleName", expect = "8B 44 24 04 A3 58 A4 BA 00")]
pub fn set_vehicle_name(name: u32) {
    VEHICLE_NAME.set(name);
}

/// `CHud::HelpMessageDisplayed()` (0x588B50): help state != 0 (`setne al`; EAX = state with AL replaced).
#[hook(addr = 0x588B50, conv = cdecl, name = "CHud::HelpMessageDisplayed", expect = "A1 74 A4 BA 00")]
pub fn help_message_displayed() -> u8 {
    u8::from(HELP_MESSAGE_STATE.get() != 0)
}

/// `CHud::ResetWastedText()` (0x589070): big messages 2 and 0 off (X 0, first byte 0).
#[hook(addr = 0x589070, conv = cdecl, name = "CHud::ResetWastedText", expect = "32 C0 C7 05 E4 A3 BA 00 00 00 00 00")]
pub fn reset_wasted_text() {
    let mut bx = BIG_MESSAGE_X.get();
    bx[2] = 0.0;
    BIG_MESSAGE_X.set(bx);
    big(2, 0).write(0);
    let mut bx = BIG_MESSAGE_X.get();
    bx[0] = 0.0;
    BIG_MESSAGE_X.set(bx);
    big(0, 0).write(0);
}

/// `CHud::SetZoneName(const GxtChar*, bool force)` (0x588BB0): unless forced, only outside interiors
/// (`CGame::currArea` 0xB72914 == 0) and with no zone name being shown.
#[hook(addr = 0x588BB0, conv = cdecl, name = "CHud::SetZoneName", expect = "8A 44 24 08 84 C0")]
pub fn set_zone_name(name: u32, force: u8) {
    if force == 0 && (sa_core::game::CURR_AREA.get() != 0 || ZONE_STATE.get() != 0) {
        return;
    }
    ZONE_NAME.set(name);
}

/// `CHud::GetYPosBasedOnHealth(uint8 player, float y, int8 offset)` (0x588B60): with a maximum health
/// below 101, `y - RsGlobal.maximumHeight * (1/448) * offset`; otherwise `y` (ST0).
#[hook(addr = 0x588B60, conv = cdecl, name = "CHud::GetYPosBasedOnHealth", expect = "0F B6 44 24 04")]
pub fn get_y_pos_based_on_health(player: u8, y: f32, offset: i8) -> X87 {
    let max_health = g::<u8>(PLAYERS + usize::from(player) * 0x190 + 0x14F).read();
    if xi(i32::from(max_health)).fcom(x(101.0)).test_parity(0x05) {
        return x(y);
    }
    xi(sa_core::platform::RS_MAXIMUM_HEIGHT.get())
        .fmul_m(f32::from_bits(0x3B12_4925))
        .fmul(xi(i32::from(offset)))
        .fsubr_m(y)
}

/// `CHud::SetMessage(const GxtChar*)` (0x588F60): copies up to 400 characters into `m_Message`, then a
/// terminator at the stop index (index 400 when the text fills the buffer: one byte past it, as the bytes).
#[hook(addr = 0x588F60, conv = cdecl, name = "CHud::SetMessage", expect = "53 56 8B 74 24 0C")]
pub fn set_message(text: GPtr<u8>) {
    let m = MESSAGE.ptr().cast::<u8>();
    if text.is_null() {
        m.write(0);
        m.write(0);
        return;
    }
    let mut i: u16 = 0;
    loop {
        let c: u8 = rd(text, i as isize);
        if c == 0 {
            break;
        }
        wr(m, i as isize, c);
        i += 1;
        if i >= 0x190 {
            break;
        }
    }
    wr::<u8>(m, i as isize, 0);
}

/// `CHud::SetBigMessage(GxtChar* text, eMessageStyle style)` (0x588FC0): only while that style's X is 0.
/// Style 5 also keeps `LastBigMessage[5]` and restarts the odd-job-2 timer when the text changes; the other
/// styles clear the caller's text (`text[0] = 0`, as the bytes). Both terminate the copied text and
/// `LastBigMessage[style]` at the stop index.
#[hook(addr = 0x588FC0, conv = cdecl, name = "CHud::SetBigMessage", expect = "66 8B 4C 24 08")]
pub fn set_big_message(text: GPtr<u8>, style: u16) {
    let s = u32::from(style);
    if !is_zero(g::<f32>(0xBAA3DC + s as usize * 4).read()) {
        return;
    }
    let mut i: u16 = 0;
    if style == 5 {
        loop {
            let c: u8 = rd(text, i as isize);
            if c == 0 {
                break;
            }
            if c != last_big(5, u32::from(i)).read() {
                ODD_JOB2_OFF_TIMER.set(0);
                ODD_JOB2_ON.set(0);
            }
            big(5, u32::from(i)).write(rd(text, i as isize));
            last_big(5, u32::from(i)).write(rd(text, i as isize));
            i += 1;
            if i >= 0x80 {
                break;
            }
        }
    } else {
        loop {
            let c: u8 = rd(text, i as isize);
            if c == 0 {
                break;
            }
            big(s, u32::from(i)).write(c);
            i += 1;
            if i >= 0x80 {
                break;
            }
        }
        text.write(0);
    }
    big(s, u32::from(i)).write(0);
    last_big(s, u32::from(i)).write(0);
}

/// `CHud::GetRidOfAllHudMessages(bool keepFadingOut)` (0x588A50): clears the help, zone and vehicle name
/// state and `m_Message`, then every big message whose X is 0, except styles 1 and 4 when the flag is set.
#[hook(addr = 0x588A50, conv = cdecl, name = "CHud::GetRidOfAllHudMessages", expect = "53 33 DB 56 57")]
pub fn get_rid_of_all_hud_messages(keep: u8) {
    HELP_MESSAGE_TO_PRINT.set([0; 400]);
    LAST_HELP_MESSAGE.set([0; 400]);
    HELP_MESSAGE.set([0; 400]);
    ZONE_NAME_TIMER.set(0);
    ZONE_NAME.set(0);
    ZONE_STATE.set(0);
    HELP_MESSAGE_TIMER.set(0);
    HELP_MESSAGE_FADE_TIMER.set(0);
    HELP_MESSAGE_STATE.set(0);
    HELP_MESSAGE_QUICK.set(0);
    HELP_MESSAGE_STAT_ID.set(0);
    HELP_MESSAGE_MAX_STAT_VALUE.set(1000);
    HELP_MESSAGE_STAT_UPDATE_VALUE.set(0);
    HELP_MESSAGE_PERMANENT.set(0);
    HELP_MESSAGE_TIME.set(0x3F80_0000);
    VEHICLE_NAME.set(0);
    VEHICLE_NAME_TO_PRINT.set(0);
    VEHICLE_NAME_TIMER.set(0);
    VEHICLE_FADE_TIMER.set(0);
    VEHICLE_STATE.set(0);
    MESSAGE.set([0; 400]);
    for k in 0..7u32 {
        if !is_zero(BIG_MESSAGE_X.get()[k as usize]) {
            continue;
        }
        if keep != 0 && (k == 1 || k == 4) {
            continue;
        }
        for i in 0..0x80 {
            big(k, i).write(0);
        }
    }
}

/// `CHudColours::GetRGB(CRGBA* out, uint8 colour)` (0x58FEA0, thiscall, hidden return pointer):
/// `CRGBA(r, g, b, a)` of the table entry; returns `out`.
#[hook(addr = 0x58FEA0, conv = thiscall, name = "CHudColours::GetRGB", expect = "0F B6 44 24 08", this = "CHudColours")]
pub fn get_rgb(this: GPtr<u8>, out: GPtr<u8>, colour: u32) -> GPtr<u8> {
    let e = this.byte_add(isize::from(colour as u8) * 4);
    let c = |k| u32::from(rd::<u8>(e, k));
    rgba_ctor(out, c(0), c(1), c(2), c(3));
    out
}

/// `CHudColours::GetRGBA(CRGBA* out, uint8 colour, uint8 alpha)` (0x58FEE0, thiscall, hidden return
/// pointer): `CRGBA(r, g, b, alpha)` (the alpha argument word is passed on unchanged); returns `out`.
#[hook(addr = 0x58FEE0, conv = thiscall, name = "CHudColours::GetRGBA", expect = "0F B6 44 24 08", this = "CHudColours")]
pub fn get_rgba(this: GPtr<u8>, out: GPtr<u8>, colour: u32, alpha: u32) -> GPtr<u8> {
    let e = this.byte_add(isize::from(colour as u8) * 4);
    let c = |k| u32::from(rd::<u8>(e, k));
    rgba_ctor(out, c(0), c(1), c(2), alpha);
    out
}

/// `CLocalisation::Initialise()` (0x56D180): nasty game on, German and French off.
#[hook(addr = 0x56D180, conv = cdecl, name = "CLocalisation::Initialise", expect = "32 C0 C6 05 EC B7 B9 00 01")]
pub fn localisation_initialise() {
    NASTY_GAME.set(1);
    GERMAN_GAME.set(0);
    FRENCH_GAME.set(0);
}

/// `CLocalisation::Blood()` (0x56D230): `mov al, 1` (the rest of EAX is the caller's). Not hooked: the
/// function is 3 bytes (`mov al, 1; ret`), too short for the 5-byte patch.
pub fn localisation_blood() -> u8 {
    1
}

/// `CLocalisation::KickingWhenDown()` (0x56D270): `mov al, 1`. Not hooked (3 bytes, as `Blood`).
pub fn localisation_kicking_when_down() -> u8 {
    1
}

/// `CLocalisation::SetNormalGame()` (0x56D1A0): nasty on, French and German off.
#[hook(addr = 0x56D1A0, conv = cdecl, name = "CLocalisation::SetNormalGame", expect = "32 C0 C6 05 EC B7 B9 00 01")]
pub fn localisation_set_normal_game() {
    NASTY_GAME.set(1);
    FRENCH_GAME.set(0);
    GERMAN_GAME.set(0);
}

/// `CLocalisation::SetFrenchGame()` (0x56D1C0): French on, German off, nasty on.
#[hook(addr = 0x56D1C0, conv = cdecl, name = "CLocalisation::SetFrenchGame", expect = "B0 01 A2 EE B7 B9 00")]
pub fn localisation_set_french_game() {
    FRENCH_GAME.set(1);
    GERMAN_GAME.set(0);
    NASTY_GAME.set(1);
}

/// `CLocalisation::SetGermanGame()` (0x56D1E0): German on, French and nasty off.
#[hook(addr = 0x56D1E0, conv = cdecl, name = "CLocalisation::SetGermanGame", expect = "32 C0 C6 05 ED B7 B9 00 01")]
pub fn localisation_set_german_game() {
    GERMAN_GAME.set(1);
    FRENCH_GAME.set(0);
    NASTY_GAME.set(0);
}

/// `CLocalisation::GermanGame()` (0x56D200): `[0xB9B7ED] != 0` (`setne al`; EAX bits 8..31 are
/// the caller's).
#[hook(addr = 0x56D200, conv = cdecl, name = "CLocalisation::GermanGame", expect = "A0 ED B7 B9 00")]
pub fn localisation_german_game() -> u8 {
    u8::from(GERMAN_GAME.get() != 0)
}

/// `CLocalisation::FrenchGame()` (0x56D210): `[0xB9B7EE] != 0` (`setne al`; EAX bits 8..31 are
/// the caller's).
#[hook(addr = 0x56D210, conv = cdecl, name = "CLocalisation::FrenchGame", expect = "A0 EE B7 B9 00")]
pub fn localisation_french_game() -> u8 {
    u8::from(FRENCH_GAME.get() != 0)
}

/// `CLocalisation::Metric()` (0x56D220): `[0xBA67CC] != 0` (`setne al`; EAX bits 8..31 are
/// the caller's).
#[hook(addr = 0x56D220, conv = cdecl, name = "CLocalisation::Metric", expect = "A0 CC 67 BA 00")]
pub fn localisation_metric() -> u8 {
    u8::from(sa_core::platform::PREFS_LANGUAGE.get() != 0)
}

/// `CLocalisation::ScreamsFromKills()` (0x56D250): `[0xB9B7ED] == 0` (`sete al`; EAX bits 8..31 are
/// the caller's).
#[hook(addr = 0x56D250, conv = cdecl, name = "CLocalisation::ScreamsFromKills", expect = "A0 ED B7 B9 00")]
pub fn localisation_screams_from_kills() -> u8 {
    u8::from(GERMAN_GAME.get() == 0)
}

/// `CLocalisation::ShootLimbs()` (0x56D280): `[0xB9B7EC] != 0` (`setne al`; EAX bits 8..31 are
/// the caller's).
#[hook(addr = 0x56D280, conv = cdecl, name = "CLocalisation::ShootLimbs", expect = "A0 EC B7 B9 00")]
pub fn localisation_shoot_limbs() -> u8 {
    u8::from(NASTY_GAME.get() != 0)
}

/// `CLocalisation::KnockDownPeds()` (0x56D290): `[0xB9B7EC] != 0` (`setne al`; EAX bits 8..31 are
/// the caller's).
#[hook(addr = 0x56D290, conv = cdecl, name = "CLocalisation::KnockDownPeds", expect = "A0 EC B7 B9 00")]
pub fn localisation_knock_down_peds() -> u8 {
    u8::from(NASTY_GAME.get() != 0)
}

/// `CLocalisation::KillFrenzy()` (0x56D2A0): `[0xB9B7EC] != 0` (`setne al`; EAX bits 8..31 are
/// the caller's).
#[hook(addr = 0x56D2A0, conv = cdecl, name = "CLocalisation::KillFrenzy", expect = "A0 EC B7 B9 00")]
pub fn localisation_kill_frenzy() -> u8 {
    u8::from(NASTY_GAME.get() != 0)
}

/// `CLocalisation::StealFromDeadPed()` (0x56D2B0): `[0xB9B7EC] != 0` (`setne al`; EAX bits 8..31 are
/// the caller's).
#[hook(addr = 0x56D2B0, conv = cdecl, name = "CLocalisation::StealFromDeadPed", expect = "A0 EC B7 B9 00")]
pub fn localisation_steal_from_dead_ped() -> u8 {
    u8::from(NASTY_GAME.get() != 0)
}

/// `CLocalisation::KillPeds()` (0x56D2C0): `[0xB9B7EC] != 0` (`setne al`; EAX bits 8..31 are
/// the caller's).
#[hook(addr = 0x56D2C0, conv = cdecl, name = "CLocalisation::KillPeds", expect = "A0 EC B7 B9 00")]
pub fn localisation_kill_peds() -> u8 {
    u8::from(NASTY_GAME.get() != 0)
}

/// `CLocalisation::PedsOnFire()` (0x56D2D0): `[0xB9B7EC] != 0` (`setne al`; EAX bits 8..31 are
/// the caller's).
#[hook(addr = 0x56D2D0, conv = cdecl, name = "CLocalisation::PedsOnFire", expect = "A0 EC B7 B9 00")]
pub fn localisation_peds_on_fire() -> u8 {
    u8::from(NASTY_GAME.get() != 0)
}

/// `CLocalisation::Porn()` (0x56D240) / `Prostitutes()` (0x56D260): `mov al, 1; ret` like `Blood`; not
/// hooked (3 bytes).
pub fn localisation_porn() -> u8 {
    1
}

/// `CHudColours::SetRGBAValue(uint8 colour, uint8 r, uint8 g, uint8 b, uint8 a)` (0x58FD20, thiscall).
#[hook(addr = 0x58FD20, conv = thiscall, name = "CHudColours::SetRGBAValue", expect = "0F B6 44 24 04", this = "CHudColours")]
pub fn set_rgba_value(this: GPtr<u8>, colour: u8, r: u8, g: u8, b: u8, a: u8) {
    let e = this.byte_add(isize::from(colour) * 4);
    wr(e, 0, r);
    wr(e, 1, g);
    wr(e, 2, b);
    wr(e, 3, a);
}

/// `CHudColours::GetIntColour(uint8 colour)` (0x58FD50, thiscall; the entry jumps to a 0x401FA0 chunk):
/// `(g << 16 | 0xFF) + (b << 8 | 0xFF) + (r << 24 | 0xFF)`, added as the bytes add them (so the low byte
/// is 0xFD and the carries reach the colour bytes).
/// Not hooked: protection code reads the entry bytes as data (`sbb ecx, [0x58FD50]` at 0x403CCE; sa-hook
/// `every_hook_installs_on_the_exe` / keycells reject it).
pub fn get_int_colour(this: GPtr<u8>, colour: u8) -> u32 {
    let e = this.byte_add(isize::from(colour) * 4);
    let r = u32::from(rd::<u8>(e, 0));
    let gg = u32::from(rd::<u8>(e, 1));
    let b = u32::from(rd::<u8>(e, 2));
    ((gg << 16) | 0xFF)
        .wrapping_add((b << 8) | 0xFF)
        .wrapping_add((r << 24) | 0xFF)
}

/// `CCredits::Start()` (0x7170E0): credits on, start time = `CTimer::m_snTimeInMilliseconds`.
#[hook(addr = 0x7170E0, conv = cdecl, name = "CCredits::Start", expect = "A1 84 CB B7 00")]
pub fn credits_start() {
    CREDITS_ON.set(1);
    CREDITS_START_TIME.set(sa_core::timer::TIME_MS.get());
}

/// `CCredits::Stop()` (0x717100): credits off.
#[hook(addr = 0x717100, conv = cdecl, name = "CCredits::Stop", expect = "C6 05 7C E9 C6 00 00")]
pub fn credits_stop() {
    CREDITS_ON.set(0);
}

/// `CUserDisplay::Init()` (0x571EE0): place name zone and timer 0, `OnscnTimer.Init()`, current vehicle 0.
#[hook(addr = 0x571EE0, conv = cdecl, name = "CUserDisplay::Init", expect = "B9 88 17 BA 00")]
pub fn user_display_init() {
    let p: GPtr<u8> = PLACE_NAME.ptr().cast();
    wr::<u32>(p, 0, 0);
    wr::<u16>(p, 4, 0);
    onscreen_timer_init(g(ONSCN_TIMER));
    CURRENT_VEHICLE.set(0);
}

/// `CUserDisplay::Process()` (0x5720A0): place name, on-screen timer, current vehicle.
#[hook(addr = 0x5720A0, conv = cdecl, name = "CUserDisplay::Process", expect = "B9 F4 18 BA 00")]
pub fn user_display_process() {
    place_name_process(PLACE_NAME.ptr().cast());
    onscreen_timer_process(g(ONSCN_TIMER));
    current_vehicle_process(CURRENT_VEHICLE.ptr().cast());
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SET_VEHICLE_NAME,
    HOOK_HELP_MESSAGE_DISPLAYED,
    HOOK_RESET_WASTED_TEXT,
    HOOK_SET_ZONE_NAME,
    HOOK_GET_Y_POS_BASED_ON_HEALTH,
    HOOK_SET_MESSAGE,
    HOOK_SET_BIG_MESSAGE,
    HOOK_GET_RID_OF_ALL_HUD_MESSAGES,
    HOOK_GET_RGB,
    HOOK_GET_RGBA,
    HOOK_LOCALISATION_INITIALISE,
    HOOK_LOCALISATION_SET_NORMAL_GAME,
    HOOK_LOCALISATION_SET_FRENCH_GAME,
    HOOK_LOCALISATION_SET_GERMAN_GAME,
    HOOK_LOCALISATION_GERMAN_GAME,
    HOOK_LOCALISATION_FRENCH_GAME,
    HOOK_LOCALISATION_METRIC,
    HOOK_LOCALISATION_SCREAMS_FROM_KILLS,
    HOOK_LOCALISATION_SHOOT_LIMBS,
    HOOK_LOCALISATION_KNOCK_DOWN_PEDS,
    HOOK_LOCALISATION_KILL_FRENZY,
    HOOK_LOCALISATION_STEAL_FROM_DEAD_PED,
    HOOK_LOCALISATION_KILL_PEDS,
    HOOK_LOCALISATION_PEDS_ON_FIRE,
    HOOK_SET_RGBA_VALUE,
    HOOK_CREDITS_START,
    HOOK_CREDITS_STOP,
    HOOK_USER_DISPLAY_INIT,
    HOOK_USER_DISPLAY_PROCESS,
];
