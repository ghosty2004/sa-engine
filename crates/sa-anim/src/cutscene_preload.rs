//! `CCutsceneMgr::LoadCutsceneData_preload` (batch `mvpn-anim-2`): reads `<name>.CUT` from `ANIM\CUTS.IMG`, parses
//! its sections and requests the cutscene models. Ported from the 1.0 US bytes (gta-reversed `CutsceneMgr.cpp`
//! `LoadCutsceneData_preload` / `LoadCutSceneFile` as a reading aid; the exe inlines the file parser and the bytes
//! win, see the doc comment). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

orig! {
    /// CRT `_stricmp` (0x8229B6)
    fn crt_stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `FindCutsceneAudioTrackId(const char*)` (0x5AFA50), AX (the caller pushes all of EAX)
    fn find_cutscene_audio_track_id(name: GPtr<u8>) -> u32 = cdecl 0x5AFA50 => crate::cutscene::find_cutscene_audio_track_id;
    /// `CAudioEngine::PreloadCutsceneTrack(int16, bool)` (0x507E30, `ret 8`)
    fn audio_preload_cutscene_track(e: GPtr<u8>, id: u32, b: u32) = thiscall 0x507E30 => unported;
    /// `CStreaming::RemoveModel(int32)` (0x4089A0)
    fn streaming_remove_model(id: i32) = cdecl 0x4089A0 => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
    /// `CCamera::SetNearClipScript(float)` (0x50BF90, `ret 4`)
    fn camera_set_near_clip_script(cam: GPtr<u8>, v: u32) = thiscall 0x50BF90 => unported;
    /// `CRubbish::SetVisibility(bool)` (0x7204C0)
    fn rubbish_set_visibility(b: u32) = cdecl 0x7204C0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWanted::ClearQdCrimes()` (0x561FE0)
    fn wanted_clear_qd_crimes(w: GPtr<u8>) = thiscall 0x561FE0 => unported;
    /// `CPlayerPed::ResetSprintEnergy()` (0x60A530)
    fn player_ped_reset_sprint_energy(p: GPtr<u8>) = thiscall 0x60A530 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPlayerInfo::MakePlayerSafe(bool, float)` (0x56E870, `ret 8`)
    fn player_info_make_player_safe(pi: GPtr<u8>, safe: u32, radius: f32) = thiscall 0x56E870 => unported;
    /// CRT `sprintf(buf, fmt, const char*)` (0x821BB5)
    fn crt_sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, s: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    /// `CDirectory::FindItem(const char* name, uint32& offset, uint32& size)` (0x5324A0, `ret 0xC`), AL
    fn directory_find_item(dir: GPtr<u8>, name: GPtr<u8>, offset: GPtr<u32>, size: GPtr<u32>) -> u8 = thiscall 0x5324A0 => unported;
    /// CRT `operator new[]` (0x821195)
    fn operator_new(n: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// CRT `free` (0x8213AE)
    fn crt_free(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// `RwStreamOpen(type, access, data)` (0x7ECEF0)
    fn rw_stream_open(ty: u32, access: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7ECEF0 => unported;
    /// `RwStreamSkip(stream, offset)` (0x7ECD00)
    fn rw_stream_skip(s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ECD00 => unported;
    /// `RwStreamRead(stream, buffer, length)` (0x7EC9D0)
    fn rw_stream_read(s: GPtr<u8>, buf: GPtr<u8>, n: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    /// `RwStreamClose(stream, data)` (0x7ECE20)
    fn rw_stream_close(s: GPtr<u8>, data: u32) -> i32 = cdecl 0x7ECE20 => unported;
    /// CRT `sscanf` (0x8220AD) with one target
    fn sscanf1(s: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `sscanf` (0x8220AD) with three targets
    fn sscanf3(s: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `sscanf` (0x8220AD) with four targets
    fn sscanf4(s: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `strncmp` (0x8214D0)
    fn crt_strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    /// CRT `strtok` (0x82244B)
    fn crt_strtok(s: GPtr<u8>, delim: GPtr<u8>) -> GPtr<u8> = cdecl 0x82244B => unported;
    /// CRT `_strlwr` (0x836C58)
    fn crt_strlwr(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x836C58 => unported;
    /// CRT `strncpy` (0x821F40)
    fn crt_strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// CRT `atol` (0x82258E)
    fn crt_atol(s: GPtr<u8>) -> i32 = cdecl 0x82258E => unported;
    /// CRT `atof` (0x823CEE), ST0
    fn crt_atof(s: GPtr<u8>) -> f64 = cdecl 0x823CEE => unported;
    /// `CTimeCycle::StartExtraColour(int32, bool)` (0x55FEC0)
    fn time_cycle_start_extra_colour(c: u32, b: u32) = cdecl 0x55FEC0 => unported;
    /// `CTimeCycle::StopExtraColour(bool)` (0x55FF20)
    fn time_cycle_stop_extra_colour(b: u32) = cdecl 0x55FF20 => unported;
    /// `CTaskSimpleCarSetPedOut::CTaskSimpleCarSetPedOut(CVehicle*, int32 door, bool)` (0x6478B0, `ret 0xC`)
    fn car_set_ped_out_ctor(t: GPtr<u8>, veh: GPtr<u8>, door: u32, b: u32) -> GPtr<u8> = thiscall 0x6478B0 => unported;
    /// `CTaskSimpleCarSetPedOut::ProcessPed(CPed*)` (0x647D10, `ret 4`)
    fn car_set_ped_out_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x647D10 => unported;
    /// `CTaskSimpleCarSetPedOut::~CTaskSimpleCarSetPedOut()` (0x647950)
    fn car_set_ped_out_dtor(t: GPtr<u8>) = thiscall 0x647950 => unported;
    /// `CIplStore::AddIplsNeededAtPosn(const CVector&)` (0x4045B0)
    fn ipl_store_add_ipls_needed_at_posn(p: GPtr<u8>) = cdecl 0x4045B0 => unported;
    /// `CStreaming::SetLoadVehiclesInLoadScene(bool)` (0x407A30)
    fn streaming_set_load_vehicles_in_load_scene(b: u32) = cdecl 0x407A30 => unported;
    /// `CStreaming::LoadScene(const CVector&)` (0x40EB70)
    fn streaming_load_scene(p: GPtr<u8>) = cdecl 0x40EB70 => unported;
    /// `CCutsceneMgr::LoadAnimationUncompressed(const char*)` (0x4D5AB0)
    fn load_animation_uncompressed(name: GPtr<u8>) = cdecl 0x4D5AB0 => crate::cutscene::load_animation_uncompressed;
    /// `CCutsceneMgr::LoadCutsceneData_postload()` (0x5AFBC0)
    fn load_cutscene_data_postload() = cdecl 0x5AFBC0 => unported;
    /// `CModelInfo::GetModelInfo(const char*, int32* index)` (0x4C5940)
    fn model_info_get_model_info(name: GPtr<u8>, idx: GPtr<i32>) -> GPtr<u8> = cdecl 0x4C5940 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: u32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::RequestSpecialModel(int32, const char*, int32)` (0x409D10)
    fn streaming_request_special_model(id: i32, name: GPtr<u8>, flags: u32) = cdecl 0x409D10 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    fn streaming_load_all_requested_models(b: u32) = cdecl 0x40EA10 => unported;
}

/// `CCutsceneMgr` state: name (0xBC3F0C), load status (0xB5F84C), offset (CVector, 0xBC4034); the finale's name
/// (0x8689B4)
const NAME: usize = 0xBC3F0C;
const LOAD_STATUS: usize = 0xB5F84C;
const OFFSET: usize = 0xBC4034;
const FINALE: usize = 0x8689B4;
/// Saved extra colour / on flag / densities and their sources
const PREV_EXTRA: usize = 0xBC4000;
const PREV_EXTRA_ON: usize = 0xBC4004;
const PREV_CAR_DENSITY: usize = 0xBC1D68;
const PREV_PED_DENSITY: usize = 0xBC1D6C;
const EXTRA_COLOUR: usize = 0xB79E44;
const EXTRA_COLOUR_ON: usize = 0xB7C484;
const CAR_DENSITY: usize = 0x8A5B20;
const PED_DENSITY: usize = 0x8D2530;
/// Counters reset at the start: texts (0xBC3FEC), current text (0xBC3FF0), load objects (0xBC3FE8), uncompressed
/// anims (0xBC3FF4), bone attachments (0xBC2A30), hidden entities (0xBC3FF8), particle effects (0xBC3FFC); the
/// finishing flag (u8 0xBC1CF8)
const TEXT_COUNT: usize = 0xBC3FEC;
const TEXT_CURR: usize = 0xBC3FF0;
const OBJ_NAME_COUNT: usize = 0xBC3FE8;
const UNCOMPRESSED_COUNT: usize = 0xBC3FF4;
const ATTACH_COUNT: usize = 0xBC2A30;
const HIDDEN_COUNT: usize = 0xBC3FF8;
const FX_COUNT: usize = 0xBC3FFC;
const FINISHING: usize = 0xBC1CF8;
/// Tables: uncompressed anim names (0xBC2CC0, 8 x 0x20), model indices (0xBC31C0, 50 dwords), load object names
/// (0xBC38C8, 0x20 each), load anim names (0xBC3288, 0x20 each), repeat flags (u8 0xBC2A34), text keys (0xBC2FC0, 8
/// each), text start times (0xBC2EC0), durations (0xBC2DC0), bone attachments (0xBC2A68, 12 each), hidden items
/// (0xBC20D0, 0x2C each), particle effects (name at 0xBC1D70, 0x6C each; system +0x20)
const UNCOMPRESSED_NAMES: usize = 0xBC2CC0;
const MODELS: usize = 0xBC31C0;
const OBJ_NAMES: usize = 0xBC38C8;
const ANIM_NAMES: usize = 0xBC3288;
const REPEAT: usize = 0xBC2A34;
const TEXT_KEYS: usize = 0xBC2FC0;
const TEXT_START: usize = 0xBC2EC0;
const TEXT_DURATION: usize = 0xBC2DC0;
const ATTACHMENTS: usize = 0xBC2A68;
const HIDDEN_ITEMS: usize = 0xBC20D0;
const FX_ENTRIES: usize = 0xBC1D70;
/// Objects appended by `AppendToNextCutscene`: count (0xB5F858), object names (0xB5F208), anim names (0xB5EBC8)
const APPEND_COUNT: usize = 0xB5F858;
const APPEND_OBJECTS: usize = 0xB5F208;
const APPEND_ANIMS: usize = 0xB5EBC8;
/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8), `CStreaming::ms_aInfoForModel[].m_nLoadState` (0x8E4CC0 + 0x10)
const MODEL_INFOS: usize = 0xA9B0C8;
const LOAD_STATE: usize = 0x8E4CD0;
/// `AudioEngine` (0xB6BC90), `TheCamera` (0xB6F028), `CWorld::PlayerInFocus` (u8 0xB7CD74), `CWorld::Players`
/// (0xB7CD98, 0x190 each), `gString` (0xB71670), `CCutsceneMgr::ms_pCutsceneDir` (0xB5F848)
const AUDIO_ENGINE: usize = 0xB6BC90;
const THE_CAMERA: usize = 0xB6F028;
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
const PLAYERS: usize = 0xB7CD98;
const G_STRING: usize = 0xB71670;
const CUTSCENE_DIR: usize = 0xB5F848;
/// Strings: "&*%" (0x868A4C), "%s.CUT" (0x868A44), "ANIM\CUTS.IMG" (0x85EEB0), "end" (0x8671EC), section names
/// ("info" 0x868A3C, "model" 0x868A34, "text" 0x862EE8, "uncompress" 0x868A28, "attach" 0x868A20, "remove" 0x868A18,
/// "peffect" 0x868A10, "extracol" 0x868A04), "offset" (0x8689FC), "%d" (0x859508), "%f %f %f" (0x8689F0), " ,"
/// (0x8689EC), "%d,%d,%s" (0x8689E0), "%d,%d,%d" (0x8689D4), "%f,%f,%f,%s" (0x8689C8), "," (0x8689C4), "csplay"
/// (0x8689BC)
const S_BAD_KEY: usize = 0x868A4C;
const S_CUT_FMT: usize = 0x868A44;
const S_CUTS_IMG: usize = 0x85EEB0;
const S_END: usize = 0x8671EC;
const S_SECTIONS: [(usize, usize); 8] = [
    (0x868A3C, 1),
    (0x868A34, 2),
    (0x862EE8, 3),
    (0x868A28, 4),
    (0x868A20, 5),
    (0x868A18, 6),
    (0x868A10, 7),
    (0x868A04, 8),
];
const S_OFFSET: usize = 0x8689FC;
const S_D: usize = 0x859508;
const S_FFF: usize = 0x8689F0;
const S_SPACE_COMMA: usize = 0x8689EC;
const S_DDS: usize = 0x8689E0;
const S_DDD: usize = 0x8689D4;
const S_FFFS: usize = 0x8689C8;
const S_COMMA: usize = 0x8689C4;
const S_CSPLAY: usize = 0x8689BC;

fn gp(a: usize) -> GPtr<u8> {
    GPtr::from_addr(a)
}
fn gu32(a: usize) -> GPtr<u32> {
    g::<u32>(a)
}
fn lp<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b.as_mut_ptr()).cast()
}
fn lb<const N: usize>(b: &mut [u8; N]) -> GPtr<u8> {
    GPtr::from_raw(b.as_mut_ptr())
}
/// `strcpy(dst, src)` as the original's inline byte loop.
fn copy_str(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut k = 0isize;
    loop {
        let c = rd::<u8>(src, k);
        wr::<u8>(dst, k, c);
        if c == 0 {
            break;
        }
        k += 1;
    }
}
fn c_strlen(s: GPtr<u8>) -> usize {
    let mut n = 0isize;
    while rd::<u8>(s, n) != 0 {
        n += 1;
    }
    n as usize
}
/// `repe cmpsb` over `strlen(lit) + 1` bytes: the line equals the literal.
fn line_is(line: GPtr<u8>, lit: usize) -> bool {
    let l = gp(lit);
    let n = c_strlen(l) as isize + 1;
    (0..n).all(|k| rd::<u8>(line, k) == rd::<u8>(l, k))
}

/// `CCutsceneMgr::LoadCutsceneData_preload()` (0x5B05A0, cdecl).
///
/// Outside the finale the cutscene track is preloaded when it has an id (`PreloadCutsceneTrack(id, true)`). The
/// cutscene object models 300..=319 without references are removed when loaded and get the key of "&*%" (loaded or
/// not: the bytes set the key on every unreferenced model). Saved: extra colour and on flag, car and ped densities;
/// reset: offset, the text / object / uncompressed / attachment / hidden / effect counters, the finishing flag, the 8
/// uncompressed names, the 50 model indices. Near clip 0.1, rubbish hidden, the player's queued crimes cleared (the
/// wanted pointer of its player data, null when there is none), the player made invisible, sprint energy reset, pad 0
/// "safe for cutscene", `MakePlayerSafe(true, 10000)`. `gString = "<name>.CUT"`; not in the cutscene directory →
/// `LoadCutsceneData_postload()` only. Otherwise the file is read whole (`new[size]`, `ANIM\CUTS.IMG` stream) and
/// parsed line by line (lines end at `\n`, `\r` or NUL; empty lines skipped; "end" closes a section). Sections:
/// info (`offset x y z`: a player in a vehicle is set out of it with a stack `CTaskSimpleCarSetPedOut(veh, 0, true)`
/// (+0x11 = 1) `ProcessPed`, the offset stored, `AddIplsNeededAtPosn`, `LoadScene` without vehicles), model
/// (`id, obj, anim...`: one load object per anim, lower-cased, the first new (model 2), the others repeats (3)),
/// text (`start,duration,KEY` upper-cased), uncompress, attach (3 ints), remove (`x,y,z,name`), peffect (name, 3
/// ints, attach name, 6 floats; system 0, flags 0), extracol (the first non-zero index starts extra colour `n - 1`;
/// 0 clears the started flag). The buffer is freed; no extra colour → `StopExtraColour(false)`. Appended objects join
/// as new models (2). No objects → `LoadCutsceneData_postload()` only. Each object: "csplay" → model 1; new → its
/// model index (requested with 0x1C) or the next free special model 300+ (`RequestSpecialModel(.., name, 0x1C)`, the
/// counter skipping loaded slots); repeat → the previous index. `LoadAllRequestedModels(true)`, load status 1.
///
/// Not reproduced: the SEH frame and the original's stack slot reuse (the special-model counter starts from the
/// local it shares with `GetModelInfo`'s index output, which is 0 at that point).
#[hook(addr = 0x5b05a0, conv = cdecl, name = "CCutsceneMgr::LoadCutsceneData_preload", expect = "55 8B EC 83 E4 F8",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads ANIM\\CUTS.IMG through RwStream (file effects: split); once per cutscene load")]
pub fn cutscene_load_cutscene_data_preload() {
    if crt_stricmp(gp(NAME), gp(FINALE)) != 0 {
        let id = find_cutscene_audio_track_id(gp(NAME));
        if id as u16 != 0xFFFF {
            audio_preload_cutscene_track(gp(AUDIO_ENGINE), id, 1);
        }
    }
    for i in 300usize..=319 {
        let mi = g::<GPtr<u8>>(MODEL_INFOS + 4 * i).read();
        if rd::<i16>(mi, 8) > 0 {
            continue;
        }
        if g::<u8>(LOAD_STATE + 0x14 * i).read() == 1 {
            streaming_remove_model(i as i32);
        }
        let mi = g::<GPtr<u8>>(MODEL_INFOS + 4 * i).read();
        let k = get_uppercase_key(gp(S_BAD_KEY));
        wr::<u32>(mi, 4, k);
    }
    gu32(PREV_EXTRA).write(gu32(EXTRA_COLOUR).read());
    g::<u8>(PREV_EXTRA_ON).write(u8::from(gu32(EXTRA_COLOUR_ON).read() != 0));
    gu32(PREV_CAR_DENSITY).write(gu32(CAR_DENSITY).read());
    gu32(PREV_PED_DENSITY).write(gu32(PED_DENSITY).read());
    for k in 0..3 {
        gu32(OFFSET + 4 * k).write(0);
    }
    for c in [TEXT_COUNT, TEXT_CURR, OBJ_NAME_COUNT, UNCOMPRESSED_COUNT, ATTACH_COUNT, HIDDEN_COUNT, FX_COUNT] {
        gu32(c).write(0);
    }
    g::<u8>(FINISHING).write(0);
    for k in 0..8 {
        g::<u8>(UNCOMPRESSED_NAMES + 0x20 * k).write(0);
    }
    for k in 0..50 {
        gu32(MODELS + 4 * k).write(0);
    }
    camera_set_near_clip_script(gp(THE_CAMERA), 0x3DCC_CCCD);
    rubbish_set_visibility(0);
    let player = find_player_ped(-1);
    let info = gp(PLAYERS + 0x190 * g::<u8>(PLAYER_IN_FOCUS).read() as usize);
    let data = rd::<GPtr<u8>>(player, 0x480);
    wanted_clear_qd_crimes(if data.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(data, 0) });
    wr::<u32>(player, 0x1C, rd::<u32>(player, 0x1C) & !0x80);
    player_ped_reset_sprint_energy(player);
    let pad = pad_get_pad(0);
    wr::<u8>(pad, 0x10E, rd::<u8>(pad, 0x10E) | 0x80);
    player_info_make_player_safe(info, 1, 10000.0);
    crt_sprintf1(gp(G_STRING), gp(S_CUT_FMT), gp(NAME));
    let mut offset = 0u32;
    let mut size = 0u32;
    if directory_find_item(g::<GPtr<u8>>(CUTSCENE_DIR).read(), gp(G_STRING), GPtr::from_raw(&mut offset),
                           GPtr::from_raw(&mut size)) == 0
    {
        load_cutscene_data_postload();
        return;
    }
    let offset = offset << 11;
    let size = size << 11;
    let buf = operator_new(size);
    let stream = rw_stream_open(2, 1, gp(S_CUTS_IMG));
    rw_stream_skip(stream, offset);
    rw_stream_read(stream, buf, size);
    rw_stream_close(stream, 0);
    let extra_set = parse(buf);
    crt_free(buf);
    if !extra_set {
        time_cycle_stop_extra_colour(0);
    }

    let appended = g::<i32>(APPEND_COUNT).read();
    let mut count = g::<i32>(OBJ_NAME_COUNT).read();
    if appended > 0 {
        let first = count;
        for k in 0..appended {
            gu32(MODELS + 4 * (first + k) as usize).write(2);
        }
        count = first + appended;
        gu32(OBJ_NAME_COUNT).write(count as u32);
        for k in 0..appended as usize {
            let slot = (first as usize + k) * 0x20;
            copy_str(gp(OBJ_NAMES + slot), gp(APPEND_OBJECTS + 0x20 * k));
            copy_str(gp(ANIM_NAMES + slot), gp(APPEND_ANIMS + 0x20 * k));
        }
    }
    gu32(APPEND_COUNT).write(0);
    if count == 0 {
        load_cutscene_data_postload();
        return;
    }
    if count > 0 {
        let mut special: i32 = 0;
        let mut i = 0usize;
        while (i as i32) < g::<i32>(OBJ_NAME_COUNT).read() {
            let name = gp(OBJ_NAMES + 0x20 * i);
            let m = gu32(MODELS + 4 * i);
            if crt_stricmp(name, gp(S_CSPLAY)) == 0 {
                m.write(1);
            } else if m.read() == 2 {
                let mut idx = 0i32;
                if !model_info_get_model_info(name, GPtr::from_raw(&mut idx)).is_null() {
                    m.write(idx as u32);
                    streaming_request_model(idx, 0x1C);
                } else {
                    let id = special + 300;
                    m.write(id as u32);
                    streaming_request_special_model(id, name, 0x1C);
                    special += 1;
                    while g::<u8>(LOAD_STATE + 0x14 * (special + 300) as usize).read() == 1 {
                        special += 1;
                    }
                }
            } else if m.read() == 3 {
                m.write(gu32((MODELS + 4 * i).wrapping_sub(4)).read());
            }
            i += 1;
        }
    }
    streaming_load_all_requested_models(1);
    gu32(LOAD_STATUS).write(1);
}

/// The inlined file parser (0x5B0815..0x5B0FEB): sections 0 (headers) .. 8, see the hook's doc comment. Returns
/// whether an extra colour was started. The port's line and field buffers are larger than the original's stack
/// slots (an overlong field overwrites neighbouring locals in the original).
fn parse(buf: GPtr<u8>) -> bool {
    let mut cur = buf;
    let mut section = 0u32;
    let mut extra_set = false;
    let mut line_buf = [0u8; 0x800];
    let line = lb(&mut line_buf);
    loop {
        while matches!(cur.read(), b'\n' | b'\r') {
            cur = cur.byte_add(1);
        }
        let mut c = cur.read();
        if c == 0 {
            break;
        }
        let mut n = 0isize;
        if c != b'\n' {
            loop {
                if c == b'\r' || c == 0 {
                    break;
                }
                wr::<u8>(line, n, c);
                n += 1;
                c = rd::<u8>(cur, 1);
                cur = cur.byte_add(1);
                if c == b'\n' {
                    break;
                }
            }
        }
        wr::<u8>(line, n, 0);
        if line_is(line, S_END) {
            section = 0;
            continue;
        }
        match section {
            0 => {
                for (s, v) in S_SECTIONS {
                    if line_is(line, s) {
                        section = v as u32;
                    }
                }
            }
            1 => section_info(line),
            2 => section_model(line),
            3 => section_text(line),
            4 => load_animation_uncompressed(crt_strtok(line, gp(S_SPACE_COMMA))),
            5 => {
                let (mut a, mut b, mut c3) = (0u32, 0u32, 0u32);
                sscanf3(line, gp(S_DDD), GPtr::from_raw(&mut a).cast(), GPtr::from_raw(&mut b).cast(),
                        GPtr::from_raw(&mut c3).cast());
                let n = gu32(ATTACH_COUNT).read();
                let e = gp(ATTACHMENTS + 12 * n as usize);
                wr::<u32>(e, 0, a);
                wr::<u32>(e, 4, b);
                wr::<u32>(e, 8, c3);
                gu32(ATTACH_COUNT).write(n.wrapping_add(1));
            }
            6 => {
                let (mut x, mut y, mut z) = (0u32, 0u32, 0u32);
                let mut name = [0u8; 0x400];
                sscanf4(line, gp(S_FFFS), GPtr::from_raw(&mut x).cast(), GPtr::from_raw(&mut y).cast(),
                        GPtr::from_raw(&mut z).cast(), lb(&mut name));
                let n = gu32(HIDDEN_COUNT).read();
                let e = gp(HIDDEN_ITEMS + 0x2C * n as usize);
                wr::<u32>(e, 0, x);
                wr::<u32>(e, 4, y);
                wr::<u32>(e, 8, z);
                copy_str(e.byte_add(0xC), lb(&mut name));
                gu32(HIDDEN_COUNT).write(n.wrapping_add(1));
            }
            7 => section_peffect(line),
            8 if !extra_set => {
                let mut v = 0i32;
                sscanf1(line, gp(S_D), GPtr::from_raw(&mut v).cast());
                if v != 0 {
                    time_cycle_start_extra_colour((v - 1) as u32, 0);
                    extra_set = true;
                }
            }
            _ => {}
        }
    }
    extra_set
}

/// `info`: `offset x y z` (parsed from line + 7).
fn section_info(line: GPtr<u8>) {
    if crt_strncmp(line, gp(S_OFFSET), 6) != 0 {
        return;
    }
    let (mut x, mut y, mut z) = (0u32, 0u32, 0u32);
    sscanf3(line.byte_add(7), gp(S_FFF), GPtr::from_raw(&mut x).cast(), GPtr::from_raw(&mut y).cast(),
            GPtr::from_raw(&mut z).cast());
    let player = find_player_ped(-1);
    if rd::<u32>(player, 0x46C) & 0x100 != 0 {
        let veh = rd::<GPtr<u8>>(player, 0x58C);
        if !veh.is_null() {
            let mut task = [0u32; 8];
            let t = lp(&mut task);
            car_set_ped_out_ctor(t, veh, 0, 1);
            wr::<u8>(t, 0x11, 1);
            car_set_ped_out_process_ped(t, player);
            car_set_ped_out_dtor(t);
        }
    }
    gu32(OFFSET).write(x);
    gu32(OFFSET + 4).write(y);
    gu32(OFFSET + 8).write(z);
    ipl_store_add_ipls_needed_at_posn(gp(OFFSET));
    streaming_set_load_vehicles_in_load_scene(0);
    let mut at = [x, y, z];
    streaming_load_scene(lp(&mut at));
    streaming_set_load_vehicles_in_load_scene(1);
}

/// `model`: `id, object, anim, anim...` (strtok on " ,").
fn section_model(line: GPtr<u8>) {
    let delim = gp(S_SPACE_COMMA);
    let mut id = 0u32;
    sscanf1(crt_strtok(line, delim), gp(S_D), GPtr::from_raw(&mut id).cast());
    let mut obj_buf = [0u8; 0x400];
    let obj = lb(&mut obj_buf);
    copy_str(obj, crt_strtok(GPtr::NULL, delim));
    let mut tok = crt_strtok(GPtr::NULL, delim);
    crt_strlwr(obj);
    let mut first = true;
    let mut anim_buf = [0u8; 0x400];
    let anim = lb(&mut anim_buf);
    while !tok.is_null() {
        wr::<u8>(anim, 0, 0);
        copy_str(anim, tok);
        crt_strlwr(anim);
        let n = g::<u32>(OBJ_NAME_COUNT).read() as usize;
        copy_str(gp(OBJ_NAMES + 0x20 * n), obj);
        copy_str(gp(ANIM_NAMES + 0x20 * n), anim);
        if first {
            first = false;
            gu32(MODELS + 4 * n).write(2);
            g::<u8>(REPEAT + n).write(0);
        } else {
            g::<u8>(REPEAT + n).write(1);
            gu32(MODELS + 4 * n).write(3);
        }
        gu32(OBJ_NAME_COUNT).write(n as u32 + 1);
        tok = crt_strtok(GPtr::NULL, delim);
    }
}

/// `text`: `start,duration,KEY` (the key upper-cased in place).
fn section_text(line: GPtr<u8>) {
    let (mut start, mut duration) = (0u32, 0u32);
    let mut key_buf = [0u8; 0x400];
    let key = lb(&mut key_buf);
    sscanf3(line, gp(S_DDS), GPtr::from_raw(&mut start).cast(), GPtr::from_raw(&mut duration).cast(), key);
    if c_strlen(key) != 0 {
        let mut i = 0u8;
        loop {
            let c = rd::<u8>(key, i as isize);
            if c.is_ascii_lowercase() {
                wr::<u8>(key, i as isize, c - 0x20);
            }
            i = i.wrapping_add(1);
            if i as usize >= c_strlen(key) {
                break;
            }
        }
    }
    let n = gu32(TEXT_COUNT).read() as usize;
    copy_str(gp(TEXT_KEYS + 8 * n), key);
    gu32(TEXT_START + 4 * n).write(start);
    gu32(TEXT_DURATION + 4 * n).write(duration);
    gu32(TEXT_COUNT).write(n as u32 + 1);
}

/// `peffect`: `name,a,b,c,attach,f1..f6` (strtok on ","; the floats `atof` rounded to f32).
fn section_peffect(line: GPtr<u8>) {
    let delim = gp(S_COMMA);
    let mut name_buf = [0u8; 0x20];
    let mut attach_buf = [0u8; 0x20];
    let name = lb(&mut name_buf);
    let attach = lb(&mut attach_buf);
    crt_strncpy(name, crt_strtok(line, delim), 0x1F);
    let a = crt_atol(crt_strtok(GPtr::NULL, delim));
    let b = crt_atol(crt_strtok(GPtr::NULL, delim));
    let c = crt_atol(crt_strtok(GPtr::NULL, delim));
    crt_strncpy(attach, crt_strtok(GPtr::NULL, delim), 0x1F);
    let mut f = [0f32; 6];
    for v in f.iter_mut() {
        *v = crt_atof(crt_strtok(GPtr::NULL, delim)) as f32;
    }
    let n = gu32(FX_COUNT).read() as usize;
    let e = gp(FX_ENTRIES + 0x6C * n);
    wr::<u32>(e, 0x20, 0);
    wr::<i32>(e, 0x24, a);
    wr::<i32>(e, 0x28, b);
    wr::<i32>(e, 0x2C, c);
    copy_str(e.byte_add(0x30), attach);
    for (k, v) in f.iter().enumerate() {
        wr::<f32>(e, 0x50 + 4 * k as isize, *v);
    }
    wr::<u8>(e, 0x68, 0);
    wr::<u8>(e, 0x69, 0);
    copy_str(e, name);
    gu32(FX_COUNT).write(n as u32 + 1);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CUTSCENE_LOAD_CUTSCENE_DATA_PRELOAD];
