//! WRITER-WORLD-2 (batch `world6-2`): the three "theme" cheats that restream peds and redress the
//! player: `CCheat::BeachPartyCheat` (entry `nop; jmp 0x404790`, a protection chunk holding the
//! body's first instruction `mov dl, [0x969159]` that jumps back to 0x439236), `FunhouseCheat` and
//! `CountrysideInvasionCheat`. Ported from the 1.0 US bytes; gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd};

orig! {
    /// `CStreaming::StreamPedsIntoRandomSlots(int32 models[8])` (0x40BDA0)
    fn stream_peds_into_random_slots(models: GPtr<u8>) = cdecl 0x40BDA0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPedClothesDesc::SetTextureAndModel(const char* texture, const char* model, eClothesTexturePart)` (0x5A8080)
    fn set_texture_and_model(d: GPtr<u8>, tex: u32, model: u32, part: u32) = thiscall 0x5A8080 => unported;
    /// `CClothes::RebuildPlayer(CPlayerPed*, bool)` (0x5A82C0)
    fn rebuild_player(ped: GPtr<u8>, b: u32) = cdecl 0x5A82C0 => unported;
    /// `CWeather::ForceWeatherNow(int16)` (0x72A4F0)
    fn force_weather_now(w: u32) = cdecl 0x72A4F0 => unported;
    /// `CCheat::HandleSpecialCheats(int32)` (0x439A10)
    fn handle_special_cheats(id: i32) = cdecl 0x439A10 => unported;
    /// `CStreaming::ReclassifyLoadedCars()` (0x40AFA0)
    fn reclassify_loaded_cars() = cdecl 0x40AFA0 => unported;
    /// `CCheat::BeachPartyCheat()` (0x439230)
    fn beach_party_cheat_call() = cdecl 0x439230 => beach_party_cheat;
    /// `CCheat::FunhouseCheat()` (0x439720)
    fn funhouse_cheat_call() = cdecl 0x439720 => funhouse_cheat;
}

/// `CCheat::m_aCheatsActive[...]`: beach party (0x969159), funhouse (0x969176), countryside
/// invasion (0x96917B), and the two flags the countryside cheat clears (0x96915E, 0x96915F).
const BEACH_PARTY_ON: usize = 0x96_9159;
const FUNHOUSE_ON: usize = 0x96_9176;
const COUNTRYSIDE_ON: usize = 0x96_917B;
/// The funhouse post effect flag (0xC402BA).
const FUNHOUSE_FX: usize = 0xC4_02BA;

/// Toggles a cheat's state byte (`test dl, dl; sete al`), returning the new state.
fn toggle(addr: usize) -> bool {
    let on = g::<u8>(addr).read() == 0;
    g::<u8>(addr).write(u8::from(on));
    on
}

/// `StreamPedsIntoRandomSlots` with an 8-model array built on the stack.
fn stream_peds(mut models: [i32; 8]) {
    stream_peds_into_random_slots(GPtr::from_raw(&mut models as *mut [i32; 8]).cast());
}

/// `RebuildPlayer(FindPlayerPed(-1), false)` unless the player is driving (+0x530 == 50).
fn rebuild_unless_driving() {
    if rd::<u32>(find_player_ped(-1), 0x530) != 0x32 {
        rebuild_player(find_player_ped(-1), 0);
    }
}

/// The player's clothes descriptor (`FindPlayerPed(-1)->m_pPlayerData->m_pPedClothesDesc`).
fn clothes() -> GPtr<u8> {
    rd::<GPtr<u8>>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 4)
}

/// `CCheat::BeachPartyCheat()` (0x439230): toggles the cheat; when it turns on: streams the beach
/// peds (models 18, 45, 138, 139, 140, the other three slots -2), dresses the player ("torso" with
/// no model, "shortskhaki"/"shorts", "flipflop", "glasses04dark"/"glasses04") and rebuilds the
/// player unless driving (+0x530 == 50), forces weather 0, applies special cheat 41 and finally
/// tail-calls `CStreaming::ReclassifyLoadedCars`.
#[hook(addr = 0x439230, conv = cdecl, name = "CCheat::BeachPartyCheat",
       expect = "90 E9 5A B5 FC FF",
       unverified = "world6-2", route = "L2", hazards = "heap-writes",
       recs = "world-cheats",
       note = "entry nop; jmp 0x404790 (chunk = mov dl, [0x969159]; jmp 0x439236), relocated whole; ends in a jmp to 0x40AFA0")]
pub fn beach_party_cheat() {
    if !toggle(BEACH_PARTY_ON) {
        return;
    }
    stream_peds([0x12, 0x2D, 0x8A, 0x8B, 0x8C, -2, -2, -2]);
    set_texture_and_model(clothes(), 0x85_90CC, 0, 0);
    set_texture_and_model(clothes(), 0x85_90B8, 0x85_90C4, 2);
    set_texture_and_model(clothes(), 0x85_90AC, 0x85_90AC, 3);
    set_texture_and_model(clothes(), 0x85_9090, 0x85_90A0, 0xF);
    rebuild_unless_driving();
    force_weather_now(0);
    handle_special_cheats(0x29);
    reclassify_loaded_cars();
}

/// `CCheat::FunhouseCheat()` (0x439720): toggles the cheat; on: streams models 167, 205, 264 (the
/// other slots -2), dresses the player (torso/torso, "legsheart"/"legs", "timberhike"/"bask1",
/// 0x8590F0/0x8590F8 part 15, 0x8590DC/0x8590E8 part 1), rebuilds the player unless driving,
/// special cheat 70, `ReclassifyLoadedCars`, weather 0 and sets the funhouse effect flag
/// (0xC402BA); off: clears the flag.
#[hook(addr = 0x439720, conv = cdecl, name = "CCheat::FunhouseCheat",
       expect = "8A 15 76 91 96 00",
       unverified = "world6-2", route = "L2", hazards = "heap-writes",
       recs = "world-cheats")]
pub fn funhouse_cheat() {
    if !toggle(FUNHOUSE_ON) {
        g::<u8>(FUNHOUSE_FX).write(0);
        return;
    }
    stream_peds([0xA7, 0xCD, 0x108, -2, -2, -2, -2, -2]);
    set_texture_and_model(clothes(), 0x85_90CC, 0x85_90CC, 0);
    set_texture_and_model(clothes(), 0x85_9118, 0x85_9124, 2);
    set_texture_and_model(clothes(), 0x85_9104, 0x85_9110, 3);
    set_texture_and_model(clothes(), 0x85_90F0, 0x85_90F8, 0xF);
    set_texture_and_model(clothes(), 0x85_90DC, 0x85_90E8, 1);
    rebuild_unless_driving();
    handle_special_cheats(0x46);
    reclassify_loaded_cars();
    force_weather_now(0);
    g::<u8>(FUNHOUSE_FX).write(1);
}

/// `CCheat::CountrysideInvasionCheat()` (0x439F60): toggles the cheat; on: streams models 196..198,
/// 157..161, dresses the player ("timberfawn"/"bask1" part 3, "captruck" part 16, "countrytr" part
/// 17), rebuilds the player unless driving, switches the beach party and funhouse cheats off
/// through their own handlers and clears 0x96915E / 0x96915F, then tail-calls
/// `ReclassifyLoadedCars`.
#[hook(addr = 0x439F60, conv = cdecl, name = "CCheat::CountrysideInvasionCheat",
       expect = "8A 15 7B 91 96 00",
       unverified = "world6-2", route = "L2", hazards = "heap-writes",
       recs = "world-cheats")]
pub fn countryside_invasion_cheat() {
    if !toggle(COUNTRYSIDE_ON) {
        return;
    }
    stream_peds([0xC4, 0xC5, 0xC6, 0x9D, 0x9E, 0x9F, 0xA0, 0xA1]);
    set_texture_and_model(clothes(), 0x85_9144, 0x85_9110, 3);
    set_texture_and_model(clothes(), 0x85_9138, 0x85_9138, 0x10);
    set_texture_and_model(clothes(), 0x85_912C, 0x85_912C, 0x11);
    rebuild_unless_driving();
    if g::<u8>(BEACH_PARTY_ON).read() != 0 {
        beach_party_cheat_call();
    }
    if g::<u8>(0x96_915E).read() != 0 {
        g::<u8>(0x96_915E).write(0);
    }
    if g::<u8>(0x96_915F).read() != 0 {
        g::<u8>(0x96_915F).write(0);
    }
    if g::<u8>(FUNHOUSE_ON).read() != 0 {
        funhouse_cheat_call();
    }
    reclassify_loaded_cars();
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_BEACH_PARTY_CHEAT, HOOK_FUNHOUSE_CHEAT, HOOK_COUNTRYSIDE_INVASION_CHEAT];
