//! The per-frame cutscene driver (batch `mvpm-anim-2`): `CCutsceneMgr::Update_overlay`,
//! `CCutsceneMgr::SetupCutsceneToStart` and `CCutsceneMgr::LoadCutsceneData_loading`. Ported from the 1.0 US bytes
//! (gta-reversed `CutsceneMgr.cpp` as a reading aid; the bytes win: the effect stop and text tests are strict unsigned
//! compares). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// `CIplStore::AddIplsNeededAtPosn(const CVector&)` (0x4045B0)
    fn ipl_store_add_ipls_needed_at_posn(p: GPtr<u8>) = cdecl 0x4045B0 => unported;
    /// `CTimer::Suspend()` (0x5619D0)
    fn timer_suspend() = cdecl 0x5619D0 => unported;
    /// `CTimer::Resume()` (0x561A00)
    fn timer_resume() = cdecl 0x561A00 => unported;
    /// `CCutsceneMgr::LoadCutsceneData_loading()` (0x5B11C0)
    fn call_load_cutscene_data_loading() = cdecl 0x5B11C0 => crate::cutscene_update::cutscene_load_cutscene_data_loading;
    /// `CCutsceneMgr::LoadCutsceneData_postload()` (0x5AFBC0)
    fn cutscene_load_cutscene_data_postload() = cdecl 0x5AFBC0 => unported;
    /// `CCutsceneMgr::CreateCutsceneObject(int32 model)` (0x5B02A0)
    fn cutscene_create_cutscene_object(model: i32) -> u32 = cdecl 0x5B02A0 => unported;
    /// `CCutsceneMgr::SetCutsceneAnim(const char* name, CObject*)` (0x5B0390)
    fn cutscene_set_cutscene_anim(name: GPtr<u8>, obj: u32) = cdecl 0x5B0390 => unported;
    /// `Fx_c::CreateMatFromVec(RwMatrix* out, CVector* pos, CVector* dir)` (0x49E950, `ret 0xC`)
    fn fx_create_mat_from_vec(fx: GPtr<u8>, out: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>) = thiscall 0x49E950 => unported;
    /// `GetFirstAtomic(RpClump*)` (0x734820)
    fn get_first_atomic(c: u32) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RpSkinGeometryGetSkin(RpGeometry*)` (0x7C7550)
    fn rp_skin_geometry_get_skin(g: u32) -> u32 = cdecl 0x7C7550 => unported;
    /// CRT `atol` (0x82258E)
    fn crt_atol(s: GPtr<u8>) -> i32 = cdecl 0x82258E => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    fn get_anim_hierarchy_from_skin_clump(c: u32) -> u32 = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0)
    fn hanim_id_get_index(h: u32, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    fn hanim_hierarchy_get_matrix_array(h: u32) -> u32 = cdecl 0x7C5120 => unported;
    /// `CClumpModelInfo::GetFrameFromName(RpClump*, const char*)` (0x4C5400)
    fn clump_mi_get_frame_from_name(c: u32, name: GPtr<u8>) -> u32 = cdecl 0x4C5400 => unported;
    /// `RwFrameGetLTM(RwFrame*)` (0x7F0990)
    fn rw_frame_get_ltm(f: u32) -> u32 = cdecl 0x7F0990 => unported;
    /// `FxManager_c::CreateFxSystem(const char* name, RwMatrix* m, RwMatrix* attach, bool)` (0x4A9BB0, `ret 0x10`)
    fn fx_manager_create_fx_system(m: GPtr<u8>, name: GPtr<u8>, mat: GPtr<u8>, attach: u32, b: u32) -> u32 = thiscall 0x4A9BB0 => unported;
    /// `CCutsceneMgr::SetupCutsceneToStart()` (0x5B14D0)
    fn call_setup_cutscene_to_start() = cdecl 0x5B14D0 => crate::cutscene_update::cutscene_setup_cutscene_to_start;
    /// `CCutsceneMgr::HideRequestedObjects()` (0x5AFAD0)
    fn cutscene_hide_requested_objects() = cdecl 0x5AFAD0 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn crt_stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CAudioEngine::DisableEffectsLoading()` (0x506E80)
    fn audio_disable_effects_loading(e: GPtr<u8>) = thiscall 0x506E80 => unported;
    /// `CAEPedSpeechAudioEntity::DisableAllPedSpeech()` (0x4E3EB0)
    fn ped_speech_disable_all() = cdecl 0x4E3EB0 => unported;
    /// `CAudioEngine::PlayPreloadedCutsceneTrack()` (0x507070)
    fn audio_play_preloaded_cutscene_track(e: GPtr<u8>) = thiscall 0x507070 => unported;
    /// `FxSystem_c::Stop()` (0x4AA390)
    fn fx_system_stop(f: GPtr<u8>) = thiscall 0x4AA390 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    fn fx_system_play(f: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `CText::Get(const char* key)` (0x6A0050, `ret 4`)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CMessages::AddMessageJump(const char*, uint32 time, uint16 flag, bool)` (0x69F1E0)
    fn messages_add_message_jump(text: GPtr<u8>, time: u32, flag: u32, b: u32) = cdecl 0x69F1E0 => unported;
    /// `CCutsceneMgr::UpdateCutsceneObjectBoundingBox(RpClump*, int32 model)` (0x5B01E0)
    fn cutscene_update_object_bounding_box(clump: u32, model: i32) = cdecl 0x5B01E0 => unported;
    /// `CCamera::GetCutSceneFinishTime()` (0x50AD90)
    fn camera_get_cut_scene_finish_time(c: GPtr<u8>) -> u32 = thiscall 0x50AD90 => unported;
    /// `CCamera::Fade(float time, int16 direction)` (0x50AC20, `ret 8`)
    fn camera_fade(c: GPtr<u8>, time: f32, dir: u32) = thiscall 0x50AC20 => unported;
    /// `CCutsceneMgr::IsCutsceneSkipButtonBeingPressed()` (0x4D5D10), AL
    fn cutscene_is_skip_button_being_pressed() -> u8 = cdecl 0x4D5D10 => unported;
    /// `CCutsceneMgr::FinishCutscene()` (0x5B04D0)
    fn cutscene_finish_cutscene() = cdecl 0x5B04D0 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*)` (0x4D15E0)
    fn clump_get_first_association(c: u32) -> GPtr<u8> = cdecl 0x4D15E0 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CEntity::UpdateRpHAnim()` (0x532B20)
    fn entity_update_rp_hanim(e: GPtr<u8>) = thiscall 0x532B20 => unported;
    /// `CTimer::Update()` (0x561B10)
    fn timer_update() = cdecl 0x561B10 => unported;
}

/// 1/1024 (0x85C6D4): the compressed key frame translation scale
const K_TRANS: usize = 0x85C6D4;
/// 1.0 (0x858624)
const K_ONE: usize = 0x858624;

/// `CCutsceneMgr` state: offset (CVector, 0xBC4034), load status (0xB5F84C), play status (0xBC3FE0), running (u8
/// 0xB5F851), timer seconds (0xBC3F08), cutscene name (0xBC3F0C), data file loaded (u8 0xBC4006), finishing (u8
/// 0xBC1CF8)
const OFFSET: usize = 0xBC4034;
const LOAD_STATUS: usize = 0xB5F84C;
const PLAY_STATUS: usize = 0xBC3FE0;
const RUNNING: usize = 0xB5F851;
const TIMER: usize = 0xBC3F08;
const NAME: usize = 0xBC3F0C;
const DATA_FILE_LOADED: usize = 0xBC4006;
const FINISHING: usize = 0xBC1CF8;
/// The finale's name ("finale", 0x8689B4)
const FINALE: usize = 0x8689B4;
/// Particle effects: count (0xBC3FFC), array (0xBC1D90, 0x6C each: +0 system, +4 start, +8 end, +0x48 playing, +0x49
/// stopped)
const FX_COUNT: usize = 0xBC3FFC;
const FX: u32 = 0xBC1D90;
/// Texts: current (0xBC3FF0), count (0xBC3FEC), start times (0xBC2EC0), durations (0xBC2DC0), keys (0xBC2FC0, 8 each)
const TEXT_CURR: usize = 0xBC3FF0;
const TEXT_COUNT: usize = 0xBC3FEC;
const TEXT_START: u32 = 0xBC2EC0;
const TEXT_DURATION: u32 = 0xBC2DC0;
const TEXT_KEYS: u32 = 0xBC2FC0;
/// Cutscene objects: count (0xBC3FE4), array (0xBC3F18)
const OBJ_COUNT: usize = 0xBC3FE4;
const OBJS: u32 = 0xBC3F18;
/// `AudioEngine` (0xB6BC90), `TheText` (0xC1B340), `TheCamera` (0xB6F028), active cam (u8 0xB6F081), cam mode (u16
/// 0xB6F1A8 + 0x238 * active)
const AUDIO_ENGINE: usize = 0xB6BC90;
const THE_TEXT: usize = 0xC1B340;
const THE_CAMERA: usize = 0xB6F028;
const ACTIVE_CAM: usize = 0xB6F081;
const CAM_MODE: u32 = 0xB6F1A8;
/// `CTimer::ms_fTimeStepNonClipped` (0xB7CB58), 1/50 (0x858B38), 1000 (0x858C4C)
const TIME_STEP_NON_CLIPPED: usize = 0xB7CB58;
const K_STEP_SECONDS: usize = 0x858B38;
const K_MS: usize = 0x858C4C;
/// `CPad` skip flags written when the skip button finishes the cutscene (0xBAAD40, 0xB5F854)
const SKIP_A: usize = 0xBAAD40;
const SKIP_B: usize = 0xB5F854;

fn gu32(a: usize) -> GPtr<u32> {
    GPtr::from_addr(a)
}
fn gu8(a: usize) -> GPtr<u8> {
    GPtr::from_addr(a)
}

/// `CCutsceneMgr::Update_overlay()` (0x5B1720, cdecl).
///
/// `AddIplsNeededAtPosn(offset)`. Load status 1: `LoadCutsceneData_loading()` between `CTimer::Suspend` / `Resume`.
/// Status 2: the play status steps 1 (set up: `SetupCutsceneToStart`, `HideRequestedObjects`, and unless the cutscene
/// is the finale (`_stricmp`) the audio effects loading and ped speech are disabled and the preloaded track plays) → 2 →
/// 3 → 4 → 0. While running: `timer = f32(timeStepNonClipped * 1/50 + timer)`, `t = _ftol(timer * 1000)` (from the
/// register); each effect (count re-read) with a system: playing, an end time other than -1 and `t > start + end`
/// (unsigned) → `Stop()`, not playing, stopped; then not playing, `t >= start` (unsigned) and not stopped → `Play()`,
/// playing. The next text when `curr < count` and `t > start[curr]` (unsigned): `AddMessageJump(TheText.Get(key),
/// duration, 1, true)`, `curr++`. Every cutscene object with a model in 0x12C..=0x13F updates its bounding box. With
/// the data file loaded, outside the finale, the active cam in mode 0x11 and load status 2: once `t + 1000 >
/// GetCutSceneFinishTime()` (unsigned) the camera fades (`Fade(1.0, 0)`, once, flag 0xBC1CF8); the skip button
/// clears 0xBAAD40, sets 0xB5F854 and finishes the cutscene.
#[hook(addr = 0x5b1720, conv = cdecl, name = "CCutsceneMgr::Update_overlay", expect = "68 34 40 BC 00",
       unverified = "mvpm-anim-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn cutscene_update_overlay() {
    ipl_store_add_ipls_needed_at_posn(GPtr::from_addr(OFFSET));
    match gu32(LOAD_STATUS).read() {
        1 => {
            timer_suspend();
            call_load_cutscene_data_loading();
            timer_resume();
            return;
        }
        2 => {}
        _ => return,
    }
    let ps = gu32(PLAY_STATUS);
    match ps.read() {
        1 => {
            call_setup_cutscene_to_start();
            cutscene_hide_requested_objects();
            if crt_stricmp(GPtr::from_addr(NAME), GPtr::from_addr(FINALE)) != 0 {
                let e: GPtr<u8> = GPtr::from_addr(AUDIO_ENGINE);
                audio_disable_effects_loading(e);
                ped_speech_disable_all();
                audio_play_preloaded_cutscene_track(e);
            }
            ps.write(ps.read().wrapping_add(1));
        }
        2 | 3 => ps.write(ps.read().wrapping_add(1)),
        4 => ps.write(0),
        _ => {}
    }
    if gu8(RUNNING).read() == 0 {
        return;
    }
    let tm = GPtr::<f32>::from_addr(TIMER);
    let s = X87::from_f32(GPtr::<f32>::from_addr(TIME_STEP_NON_CLIPPED).read())
        .fmul_m(GPtr::<f32>::from_addr(K_STEP_SECONDS).read())
        .fadd_m(tm.read());
    tm.write(s.to_f32());
    let t = s.fmul_m(GPtr::<f32>::from_addr(K_MS).read()).ftol() as u32;
    let mut i = 0u32;
    while (i as i32) < gu32(FX_COUNT).read() as i32 {
        let fx = GPtr::<u8>::from_addr((FX + i * 0x6C) as usize);
        let sys = fx.cast::<GPtr<u8>>().read();
        let start = fx.byte_add(4).cast::<u32>().read();
        let playing = fx.byte_add(0x48);
        let stopped = fx.byte_add(0x49);
        if playing.read() != 0 {
            let end = fx.byte_add(8).cast::<u32>().read();
            if end != u32::MAX && !sys.is_null() && t > start.wrapping_add(end) {
                fx_system_stop(sys);
                playing.write(0);
                stopped.write(1);
            }
        }
        let sys = fx.cast::<GPtr<u8>>().read();
        if !sys.is_null() && playing.read() == 0 && t >= fx.byte_add(4).cast::<u32>().read() && stopped.read() == 0 {
            fx_system_play(sys);
            playing.write(1);
        }
        i += 1;
    }
    let curr = gu32(TEXT_CURR).read();
    if (curr as i32) < gu32(TEXT_COUNT).read() as i32 && t > gu32((TEXT_START + curr * 4) as usize).read() {
        let text = text_get(GPtr::from_addr(THE_TEXT), GPtr::from_addr((TEXT_KEYS + curr * 8) as usize));
        let c = gu32(TEXT_CURR).read();
        messages_add_message_jump(text, gu32((TEXT_DURATION + c * 4) as usize).read(), 1, 1);
        gu32(TEXT_CURR).write(gu32(TEXT_CURR).read().wrapping_add(1));
    }
    let mut k = 0u32;
    while (k as i32) < gu32(OBJ_COUNT).read() as i32 {
        let obj = gu32((OBJS + k * 4) as usize).read();
        let o = GPtr::<u8>::from_addr(obj as usize);
        let model = i32::from(o.byte_add(0x22).cast::<i16>().read());
        if (0x12C..=0x13F).contains(&model) {
            cutscene_update_object_bounding_box(o.byte_add(0x18).cast::<u32>().read(), model);
        }
        k += 1;
    }
    if gu8(DATA_FILE_LOADED).read() == 0 || crt_stricmp(GPtr::from_addr(NAME), GPtr::from_addr(FINALE)) == 0 {
        return;
    }
    let active = u32::from(gu8(ACTIVE_CAM).read());
    if GPtr::<u16>::from_addr((CAM_MODE + active * 0x238) as usize).read() != 0x11 || gu32(LOAD_STATUS).read() != 2 {
        return;
    }
    let cam: GPtr<u8> = GPtr::from_addr(THE_CAMERA);
    let finish = camera_get_cut_scene_finish_time(cam);
    if t.wrapping_add(1000) > finish && gu8(FINISHING).read() == 0 {
        gu8(FINISHING).write(1);
        camera_fade(cam, 1.0, 0);
    }
    if cutscene_is_skip_button_being_pressed() != 0 {
        gu8(SKIP_A).write(0);
        gu8(SKIP_B).write(1);
        cutscene_finish_cutscene();
    }
}

fn wf(p: GPtr<u8>, off: isize, v: f32) {
    p.byte_add(off).cast::<f32>().write(v);
}
fn wu(p: GPtr<u8>, off: isize, v: u32) {
    p.byte_add(off).cast::<u32>().write(v);
}

/// `CCutsceneMgr::SetupCutsceneToStart()` (0x5B14D0, cdecl).
///
/// `offset.z = f32(offset.z + 1)`. The cutscene objects from the last to the first: with a first animation
/// association, an object attached to a frame (+0x17C) only gets association flags +0x2E `&= ~0x40, |= 1`; otherwise
/// the first sequence's first key frame translation `t` (uncompressed: f32 at key +0x14; compressed: i16 at +0xA/+0xC/+0xE
/// * 1/1024, kept on the FPU) gives `p = (f32(offset.x + t.x), f32(t.y + offset.y), f32(f32(t.z) + offset.z))`, written to
/// the matrix position (or the placement) and, recomputed, to the world position +0x184; then flag bit 0. Without an
/// association the offset itself is written (x as a dword, y and z through the FPU) to both. Each object is added to the
/// world and a clump object (RW type byte 2) updates its hierarchy (`UpdateRpHAnim`). Then `CTimer::Update()` twice,
/// timer 0, running = 1.
#[hook(addr = 0x5b14d0, conv = cdecl, name = "CCutsceneMgr::SetupCutsceneToStart", expect = "D9 05 3C 40 BC 00",
       unverified = "mvpm-anim-2", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the cutscene objects (object pool) and their animation associations")]
pub fn cutscene_setup_cutscene_to_start() {
    let off = GPtr::<u8>::from_addr(OFFSET);
    let rf = |o: isize| off.byte_add(o).cast::<f32>().read();
    wf(off, 8, X87::from_f32(rf(8)).fadd_m(GPtr::<f32>::from_addr(K_ONE).read()).to_f32());
    let mut i = gu32(OBJ_COUNT).read();
    while i != 0 {
        let first = GPtr::<u8>::from_addr(gu32((OBJS + (i - 1) * 4) as usize).read() as usize);
        i -= 1;
        let assoc = clump_get_first_association(first.byte_add(0x18).cast::<u32>().read());
        let obj = GPtr::<u8>::from_addr(gu32((OBJS + i * 4) as usize).read() as usize);
        if !assoc.is_null() {
            let flags = assoc.byte_add(0x2E);
            if obj.byte_add(0x17C).cast::<u32>().read() != 0 {
                flags.write(flags.read() & 0xBF);
                flags.write(flags.read() | 1);
            } else {
                let hier = assoc.byte_add(0x14).cast::<GPtr<u8>>().read();
                let seq = hier.byte_add(4).cast::<GPtr<u8>>().read();
                let keys = seq.byte_add(8).cast::<GPtr<u8>>().read();
                let (tx, ty, tz) = if hier.byte_add(0xA).read() == 0 {
                    let k = keys.byte_add(0x14).cast::<f32>();
                    (X87::from_f32(k.read()), X87::from_f32(k.byte_add(4).read()), k.byte_add(8).read())
                } else {
                    let kt = GPtr::<f32>::from_addr(K_TRANS).read();
                    let c = |o: isize| X87::from_i32(i32::from(keys.byte_add(o).cast::<i16>().read())).fmul_m(kt);
                    (c(0xA), c(0xC), c(0xE).to_f32())
                };
                let px = X87::from_f32(rf(0)).fadd(tx).to_f32();
                let py = ty.fadd_m(rf(4)).to_f32();
                let pz = X87::from_f32(tz).fadd_m(rf(8)).to_f32();
                let m = obj.byte_add(0x14).cast::<GPtr<u8>>().read();
                if m.is_null() {
                    wf(obj, 4, px);
                    wf(obj, 8, py);
                    wf(obj, 0xC, pz);
                } else {
                    wf(m, 0x30, px);
                    wf(obj.byte_add(0x14).cast::<GPtr<u8>>().read(), 0x34, py);
                    wf(obj.byte_add(0x14).cast::<GPtr<u8>>().read(), 0x38, pz);
                }
                let wx = X87::from_f32(rf(0)).fadd(tx).to_f32();
                let wy = ty.fadd_m(rf(4)).to_f32();
                let wz = X87::from_f32(tz).fadd_m(rf(8)).to_f32();
                let wp = GPtr::<u8>::from_addr(gu32((OBJS + i * 4) as usize).read() as usize).byte_add(0x184);
                wf(wp, 0, wx);
                wf(wp, 4, wy);
                wf(wp, 8, wz);
                flags.write(flags.read() | 1);
            }
        } else {
            let oz = X87::from_f32(rf(8)).to_f32();
            let oy = X87::from_f32(rf(4)).to_f32();
            let ox = off.cast::<u32>().read();
            let m = obj.byte_add(0x14).cast::<GPtr<u8>>().read();
            if m.is_null() {
                wf(obj, 8, oy);
                wu(obj, 4, ox);
                wf(obj, 0xC, oz);
            } else {
                wu(m, 0x30, ox);
                wf(obj.byte_add(0x14).cast::<GPtr<u8>>().read(), 0x34, oy);
                wf(obj.byte_add(0x14).cast::<GPtr<u8>>().read(), 0x38, oz);
            }
            let wp = GPtr::<u8>::from_addr(gu32((OBJS + i * 4) as usize).read() as usize).byte_add(0x184);
            for k in 0..3isize {
                wu(wp, 4 * k, off.byte_add(4 * k).cast::<u32>().read());
            }
        }
        let o = GPtr::<u8>::from_addr(gu32((OBJS + i * 4) as usize).read() as usize);
        world_add(o);
        let o = GPtr::<u8>::from_addr(gu32((OBJS + i * 4) as usize).read() as usize);
        let rw = o.byte_add(0x18).cast::<GPtr<u8>>().read();
        if rw.read() == 2 {
            entity_update_rp_hanim(o);
        }
    }
    timer_update();
    timer_update();
    gu32(TIMER).write(0);
    gu8(RUNNING).write(1);
}

/// Cutscene models: count (0xBC3FE8), model ids (0xBC31C0), "no new object" flags (0xBC2A34), animation names
/// (0xBC3288, 0x20 each); `CStreaming::ms_aInfoForModel` load state (0x8E4CD0 + 0x14 * model)
const MODEL_COUNT: usize = 0xBC3FE8;
const MODELS: u32 = 0xBC31C0;
const SHARED_FLAGS: u32 = 0xBC2A34;
const ANIM_NAMES: u32 = 0xBC3288;
const STREAM_INFO_STATE: u32 = 0x8E4CD0;
/// Attachments: count (0xBC2A30), entries (0xBC2A68, 0xC: object, attached-to object, bone id)
const ATTACH_COUNT: usize = 0xBC2A30;
const ATTACHES: u32 = 0xBC2A68;
/// `g_fx` (Fx_c, 0xA9AE00) and `g_fxMan` (FxManager_c, 0xA9AE80)
const G_FX: usize = 0xA9AE00;
const G_FX_MAN: usize = 0xA9AE80;
/// The effect array as `LoadCutsceneData_loading` sees it (0xBC1D70, 0x6C each: +0 name, +0x20 system, +0x2C attach
/// object (1-based), +0x30 bone or frame name, +0x50 position, +0x5C direction)
const FX_ENTRIES: u32 = 0xBC1D70;

/// `CCutsceneMgr::LoadCutsceneData_loading()` (0x5B11C0, cdecl).
///
/// Waits (returns) while any cutscene model in 0x12C..=0x13F is not loaded (streaming state != 1). Then
/// `LoadCutsceneData_postload()`; for each model without the shared flag a new `CreateCutsceneObject(model)` (the last
/// one created stays current), and a model with an animation name gets `SetCutsceneAnim(name, current)` (the current
/// object of a shared first model is an uninitialised local in the original; NULL here). Each effect: its matrix from
/// `Fx_c::CreateMatFromVec(position, direction)`; an attach object index in 1..=count gives the attach matrix: the bone
/// matrix (`atol(name)` as the bone id) of a skinned clump, else the LTM of the frame named so (none when missing);
/// `system = CreateFxSystem(name, matrix, attach, true)`. Each attachment: child +0x180 = parent object, +0x17C = the
/// parent's bone index (`RpHAnimIDGetIndex`).
#[hook(addr = 0x5b11c0, conv = cdecl, name = "CCutsceneMgr::LoadCutsceneData_loading", expect = "8B 15 E8 3F BC 00",
       unverified = "mvpm-anim-2", route = "L2", hazards = "uninit-locals,heap-writes", recs = "grove-walk",
       note = "the current object starts as an uninitialised local in the original (NULL here); the effect matrix lives on the Rust stack")]
pub fn cutscene_load_cutscene_data_loading() {
    let n = gu32(MODEL_COUNT).read() as i32;
    for i in 0..n.max(0) {
        let m = gu32((MODELS + i as u32 * 4) as usize).read() as i32;
        if (0x12C..=0x13F).contains(&m) && gu8((STREAM_INFO_STATE as i32 + m * 0x14) as usize).read() != 1 {
            return;
        }
    }
    cutscene_load_cutscene_data_postload();
    let mut current: u32 = 0;
    let mut i = 0u32;
    while (i as i32) < gu32(MODEL_COUNT).read() as i32 {
        if gu8((SHARED_FLAGS + i) as usize).read() == 0 {
            current = cutscene_create_cutscene_object(gu32((MODELS + i * 4) as usize).read() as i32);
        }
        let name = GPtr::<u8>::from_addr((ANIM_NAMES + i * 0x20) as usize);
        if name.read() != 0 {
            cutscene_set_cutscene_anim(name, current);
        }
        i += 1;
    }
    let mut k = 0u32;
    while (k as i32) < gu32(FX_COUNT).read() as i32 {
        let e = GPtr::<u8>::from_addr((FX_ENTRIES + k * 0x6C) as usize);
        let rq = |o: isize| e.byte_add(o).cast::<u32>().read();
        let mut pos = [rq(0x50), rq(0x54), X87::from_f32(f32::from_bits(rq(0x58))).to_f32().to_bits()];
        let mut dir = [rq(0x5C), X87::from_f32(f32::from_bits(rq(0x60))).to_f32().to_bits(), X87::from_f32(f32::from_bits(rq(0x64))).to_f32().to_bits()];
        let mut mat = [0u32; 16];
        let mp: GPtr<u8> = GPtr::from_raw(mat.as_mut_ptr()).cast();
        fx_create_mat_from_vec(GPtr::from_addr(G_FX), mp, GPtr::from_raw(pos.as_mut_ptr()).cast(), GPtr::from_raw(dir.as_mut_ptr()).cast());
        let mut attach: u32 = 0;
        let idx = rq(0x2C) as i32;
        if idx > 0 && idx < gu32(OBJ_COUNT).read() as i32 + 1 {
            let obj = gu32((OBJS as i32 + (idx - 1) * 4) as usize).read();
            let clump = GPtr::<u32>::from_addr(obj as usize + 0x18).read();
            let atomic = get_first_atomic(clump);
            let skinned = !atomic.is_null() && rp_skin_geometry_get_skin(atomic.byte_add(0x18).cast::<u32>().read()) != 0;
            if skinned {
                let bone = crt_atol(e.byte_add(0x30));
                let h = get_anim_hierarchy_from_skin_clump(clump);
                let bi = hanim_id_get_index(h, bone);
                attach = hanim_hierarchy_get_matrix_array(h).wrapping_add((bi as u32) << 6);
            } else {
                let f = clump_mi_get_frame_from_name(clump, e.byte_add(0x30));
                if f != 0 {
                    attach = rw_frame_get_ltm(f);
                }
            }
        }
        let sys = fx_manager_create_fx_system(GPtr::from_addr(G_FX_MAN), e, mp, attach, 1);
        e.byte_add(0x20).cast::<u32>().write(sys);
        k += 1;
    }
    let mut a = 0u32;
    while (a as i32) < gu32(ATTACH_COUNT).read() as i32 {
        let en = GPtr::<u32>::from_addr((ATTACHES + a * 0xC) as usize);
        let parent = gu32((OBJS + en.read() * 4) as usize).read();
        let child = gu32((OBJS + en.byte_add(4).read() * 4) as usize).read();
        let bone = en.byte_add(8).read() as i32;
        let h = get_anim_hierarchy_from_skin_clump(GPtr::<u32>::from_addr(parent as usize + 0x18).read());
        let bi = hanim_id_get_index(h, bone);
        GPtr::<u32>::from_addr(child as usize + 0x180).write(parent);
        GPtr::<i32>::from_addr(child as usize + 0x17C).write(bi);
        a += 1;
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_CUTSCENE_UPDATE_OVERLAY,
    HOOK_CUTSCENE_SETUP_CUTSCENE_TO_START,
    HOOK_CUTSCENE_LOAD_CUTSCENE_DATA_LOADING,
];
