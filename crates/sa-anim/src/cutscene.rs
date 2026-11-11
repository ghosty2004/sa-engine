//! ANIM: `CCutsceneMgr` (PORT-ANIM-INPUT, fork D): the bookkeeping and glue functions. Ported from
//! the 1.0 US bytes; gta-reversed `CutsceneMgr.cpp` as a reading aid,
//! `docs/re/specs/ANIM/CCutsceneMgr*.md`. See `docs/arch/anim-input-port.md`.
//!
//! Deferred (they mostly drive unported streaming / world / ped / camera / HUD code, see the port
//! doc): `LoadCutsceneData_preload` / `_loading` / `_postload`, `DeleteCutsceneData_overlay`,
//! `Update_overlay`, `SetupCutsceneToStart`, `FinishCutscene`, `CreateCutsceneObject`,
//! `HideRequestedObjects`, `RemoveEverythingBecauseCutsceneDoesntFitInMemory`,
//! `LoadEverythingBecauseCutsceneDeletedAllOfIt`, `CCutsceneObject::CCutsceneObject`.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::{Cmp, X87};

use crate::common::{g, rd, wr, x};
use crate::ik::{get_anim_hierarchy_from_skin_clump, rp_hanim_id_get_index, rw_v3d_length};

sa_core::globals! {
    list GLOBALS;
    /// `CCutsceneMgr::ms_cutsceneAssociations` (`CAnimBlendAssocGroup`, 0x14)
    pub static CUTSCENE_ASSOCIATIONS @ 0xBC4020: [u8; 0x14] = [0; 0x14];
}

/// `CCutsceneMgr::ms_cutsceneAssociations` (0xBC4020).
pub fn cutscene_associations() -> GPtr<u8> {
    CUTSCENE_ASSOCIATIONS.ptr().cast()
}

// CCutsceneMgr globals (byte addresses)
const PCUTSCENE_DIR: usize = 0xB5F848; // ms_pCutsceneDir
const LOAD_STATUS: usize = 0xB5F84C; // ms_cutsceneLoadStatus (int32)
const ANIM_LOADED: usize = 0xB5F850; // ms_animLoaded
const RUNNING: usize = 0xB5F851; // ms_running
const CUTSCENE_PROCESSING: usize = 0xB5F852; // ms_cutsceneProcessing
const WAS_SKIPPED: usize = 0xB5F854; // ms_wasCutsceneSkipped
const HAS_FILE_IO_STARTED: usize = 0xB5F855; // ms_hasFileInfo...
const B5F853: usize = 0xB5F853; // ms_useLodMultiplier
const NUM_APPENDED: usize = 0xB5F858; // ms_numAppendObjectNames
const RESTORE_EVERYTHING: usize = 0xB5F85C; // restoreEverythingAfterCutscene
const APPEND_ANIM_NAMES: usize = 0xB5EBC8; // ms_cAppendAnimName[][32]
const APPEND_OBJECT_NAMES: usize = 0xB5F208; // ms_cAppendObjectName[][32]
const UNCOMPRESSED_ANIMS: usize = 0xBC2CC0; // ms_aUncompressedCutsceneAnims[][32]
const NUM_UNCOMPRESSED: usize = 0xBC3FF4; // ms_numUncompressedCutsceneAnims
const CUTSCENE_TIMER: usize = 0xBC3F08; // ms_cutsceneTimer (float)
const CUTSCENE_NAME: usize = 0xBC3F0C; // ms_cutsceneName[8]
const DATA_FILE_LOADED: usize = 0xBC4006; // dataFileLoaded
const BC4005: usize = 0xBC4005; // the "keep zone models" flag LoadCutsceneData_overlay tests
const BC3FE0: usize = 0xBC3FE0; // ms_cutscenePlayStatus... set to 1 by StartCutscene
const CUT_OFFSET: usize = 0xBC4034; // ms_cutsceneOffset
const CAMERA: usize = 0xB6F028; // TheCamera
const CAM_MATRIX: usize = 0xB6F03C;
const CAM_PLACEMENT_POS: usize = 0xB6F02C;
const BAAD40: usize = 0xBAAD40; // cleared by SkipCutscene
const CUTS_DIR: usize = 0x85EEB0; // "ANIM\\CUTS.DIR"
const AUDIO_TRACKS: usize = 0x8D0AA8; // {name, trackId}[], null-terminated
/// `CTempColModels::ms_colModelCutObj` radius of model 300 (`+0x24` of the 0x30-byte models at
/// 0x968A30 - 300 * 0x30 ... indexed by model id).
const CUTOBJ_COLMODELS: usize = 0x968A30;
const ANIM_CLUMP_OFFSET: usize = 0xB5F878; // the RpAnimBlendClumpData plugin offset

orig! {
    /// `operator new` (0x82119A)
    pub fn operator_new2(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `operator delete` / `free` (0x8214BD)
    pub fn crt_free2(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CDirectory::CDirectory(capacity)` (0x5322A0)
    pub fn directory_ctor(d: GPtr<u8>, capacity: i32) -> GPtr<u8> = thiscall 0x5322A0 => unported;
    /// `CDirectory::~CDirectory()` (0x5322D0)
    pub fn directory_dtor(d: GPtr<u8>) = thiscall 0x5322D0 => unported;
    /// `CDirectory::ReadDirFile(name)` (0x532350)
    pub fn directory_read_dir_file(d: GPtr<u8>, name: GPtr<u8>) = thiscall 0x532350 => unported;
    /// CRT `_strlwr` (0x836C58)
    pub fn crt_strlwr(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x836C58 => unported;
    /// CRT `_stricmp` (0x8229B6)
    pub fn crt_stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CCutsceneMgr::Update_overlay()` (0x5B1720)
    pub fn update_overlay() = cdecl 0x5B1720 => unported;
    /// `CPad::GetPad(n)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `FUN_00746070` (the last skip test: skip when it returns 0)
    pub fn skip_check_746070() -> u8 = cdecl 0x746070 => unported;
    /// `FindPlayerPed(id)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CClothes::RebuildPlayerIfNeeded(player)` (0x5A8390)
    pub fn clothes_rebuild_player_if_needed(p: GPtr<u8>) = cdecl 0x5A8390 => unported;
    /// `CClothes::RebuildCutscenePlayer(player, model)` (0x5A8270)
    pub fn clothes_rebuild_cutscene_player(p: GPtr<u8>, model: i32) = cdecl 0x5A8270 => unported;
    /// `CStreaming::RequestModel(model, flags)` (0x4087E0)
    pub fn streaming_request_model(model: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::LoadAllRequestedModels(priority)` (0x40EA10)
    pub fn streaming_load_all_requested_models(prio: u32) = cdecl 0x40EA10 => unported;
    /// `CStreaming::SetMissionDoesntRequireModel(model)` (0x409C90)
    pub fn streaming_set_mission_doesnt_require_model(model: i32) = cdecl 0x409C90 => unported;
    /// `CStreaming::RemoveCurrentZonesModels()` (0x40B080)
    pub fn streaming_remove_current_zones_models() = cdecl 0x40B080 => unported;
    /// `CStreaming::RemoveUnusedModelsInLoadedList()` (0x407AC0)
    pub fn streaming_remove_unused_models_in_loaded_list() = cdecl 0x407AC0 => unported;
    /// `CStreaming::ImGonnaUseStreamingMemory()` (0x407BE0)
    pub fn streaming_im_gonna_use_streaming_memory() = cdecl 0x407BE0 => unported;
    /// `CStreaming::IHaveUsedStreamingMemory()` (0x407BF0)
    pub fn streaming_i_have_used_streaming_memory() = cdecl 0x407BF0 => unported;
    /// `CGame::DrasticTidyUpMemory(bool)` (0x53C810)
    pub fn game_drastic_tidy_up_memory(b: u32) = cdecl 0x53C810 => unported;
    /// `CTimer::Suspend()` (0x5619D0)
    pub fn timer_suspend() = cdecl 0x5619D0 => unported;
    /// `CTimer::Resume()` (0x561A00)
    pub fn timer_resume() = cdecl 0x561A00 => unported;
    /// `CCutsceneMgr::LoadCutsceneData_preload()` (0x5B05A0)
    pub fn load_cutscene_data_preload() = cdecl 0x5B05A0 => unported;
    /// `CCutsceneMgr::LoadCutsceneData_overlay(name)` (0x5B13F0)
    pub fn call_load_cutscene_data_overlay(name: GPtr<u8>) = cdecl 0x5B13F0 => load_cutscene_data_overlay;
    /// `CCutsceneMgr::DeleteCutsceneData_overlay()` (0x5AFD60)
    pub fn delete_cutscene_data_overlay() = cdecl 0x5AFD60 => unported;
    /// `CCutsceneMgr::LoadEverythingBecauseCutsceneDeletedAllOfIt()` (0x4D5C10)
    pub fn load_everything_because_cutscene_deleted_all_of_it() = cdecl 0x4D5C10 => unported;
    /// `CCutsceneMgr::FinishCutscene()` (0x5B04D0)
    pub fn finish_cutscene() = cdecl 0x5B04D0 => unported;
    /// `CAnimBlendAssocGroup::GetAnimation(name)` (0x4CE040)
    pub fn assoc_group_get_animation(gr: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x4CE040 => unported;
    /// `CAnimBlendAssocGroup::CopyAnimation(name)` (0x4CE0B0)
    pub fn assoc_group_copy_animation(gr: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x4CE0B0 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(time)` (0x4CEA80)
    pub fn assoc_set_current_time(a: GPtr<u8>, t: f32) = thiscall 0x4CEA80 => unported;
    /// `RpAnimBlendClumpFindFrame(clump, name)` (0x4D62A0)
    pub fn rp_anim_blend_clump_find_frame(clump: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D62A0 => unported;
    /// `_rpAtomicResyncInterpolatedSphere(atomic)` (0x7491F0)
    pub fn rp_atomic_resync_interpolated_sphere(a: GPtr<u8>) = cdecl 0x7491F0 => unported;
    /// `RwV3dTransformPoints(out, in, n, matrix)` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<f32>, inp: GPtr<f32>, n: i32, m: GPtr<u8>) -> GPtr<f32> = cdecl 0x7EDD90 => unported;
    /// `RpClumpForAllAtomics(clump, callback, data)` (0x749B70)
    pub fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `CCamera::GetPositionAlongSpline()` (0x50AF80): ST0
    pub fn camera_get_position_along_spline(cam: GPtr<u8>) -> f64 = thiscall 0x50AF80 => unported;
    /// `CCamera::SetCamCutSceneOffSet(const CVector&)` (0x50BD20)
    pub fn camera_set_cam_cut_scene_offset(cam: GPtr<u8>, v: GPtr<u8>) = thiscall 0x50BD20 => unported;
    /// `CCamera::TakeControlWithSpline(mode)` (0x50CAE0)
    pub fn camera_take_control_with_spline(cam: GPtr<u8>, mode: i32) = thiscall 0x50CAE0 => unported;
    /// `CCamera::SetWideScreenOn()` (0x50C140)
    pub fn camera_set_wide_screen_on(cam: GPtr<u8>) = thiscall 0x50C140 => unported;
    /// `CHud::SetHelpMessage(text, quick, permanent, addToBrief)` (0x588BE0)
    pub fn hud_set_help_message(text: u32, quick: u32, permanent: u32, brief: u32) = cdecl 0x588BE0 => unported;
    /// `CPlantMgr::PreUpdateOnceForNewCameraPos(pos)` (0x5DCF30)
    pub fn plant_mgr_pre_update_once_for_new_camera_pos(pos: GPtr<u8>) = cdecl 0x5DCF30 => unported;
}

/// `strcpy` as the inlined byte loops copy (including the terminator); returns the source pointer
/// one past the terminator (EAX after the loop).
fn strcpy(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut i = 0;
    loop {
        let c = rd::<u8>(src, i);
        wr::<u8>(dst, i, c);
        i += 1;
        if c == 0 {
            break;
        }
    }
}

/// `CCutsceneMgr::Initialise()` (0x4D5A20): load status 0, the flags 0xB5F850..0xB5F855 cleared,
/// `ms_pCutsceneDir = new CDirectory(0x200)` (null if the allocation fails). The SEH frame around
/// the constructor is not built (stack only).
#[hook(addr = 0x4D5A20, conv = cdecl, name = "CCutsceneMgr::Initialise", expect = "6A FF 68 DB BD 83 00")]
pub fn initialise() {
    g::<u32>(LOAD_STATUS).write(0);
    g::<u8>(RUNNING).write(0);
    g::<u8>(ANIM_LOADED).write(0);
    g::<u8>(CUTSCENE_PROCESSING).write(0);
    g::<u8>(B5F853).write(0);
    g::<u8>(WAS_SKIPPED).write(0);
    g::<u8>(HAS_FILE_IO_STARTED).write(0);
    let p = operator_new2(0x10);
    let d = if p.is_null() { GPtr::NULL } else { directory_ctor(p, 0x200) };
    g::<u32>(PCUTSCENE_DIR).write(d.addr() as u32);
}

/// `CCutsceneMgr::LoadAnimationUncompressed(name)` (0x4D5AB0): copy the name into
/// `ms_aUncompressedCutsceneAnims[n++]` and clear the first byte of the next entry.
#[hook(addr = 0x4D5AB0, conv = cdecl, name = "CCutsceneMgr::LoadAnimationUncompressed", expect = "8B 44 24 04 56")]
pub fn load_animation_uncompressed(name: GPtr<u8>) {
    let n = g::<i32>(NUM_UNCOMPRESSED).read();
    strcpy(g::<u8>(UNCOMPRESSED_ANIMS).byte_add((n << 5) as isize), name);
    let n = n + 1;
    g::<i32>(NUM_UNCOMPRESSED).write(n);
    wr::<u8>(g(UNCOMPRESSED_ANIMS), (n << 5) as isize, 0);
}

/// `CCutsceneMgr::Update()` (0x4D5D00): tail-jump `Update_overlay` while a cutscene is loading or
/// loaded (`ms_cutsceneLoadStatus != 0`).
#[hook(addr = 0x4D5D00, conv = cdecl, name = "CCutsceneMgr::Update", expect = "A1 4C F8 B5 00")]
pub fn update() {
    if g::<u32>(LOAD_STATUS).read() != 0 {
        update_overlay();
    }
}

/// `NewState.w != 0 && OldState.w == 0` of a pad word at `new` / `old`.
fn edge(new: GPtr<u16>, old: GPtr<u16>) -> bool {
    new.read() != 0 && old.read() == 0
}

/// `CCutsceneMgr::IsCutsceneSkipButtonBeingPressed()` (0x4D5D10): pad 0 cross, mouse LMB, Enter,
/// the key at +0x25A of the key states and Space "just pressed" (each preceded by the original's
/// `GetPad(0)` calls), else the 0x746070 test returning 0. AL.
#[hook(addr = 0x4D5D10, conv = cdecl, name = "CCutsceneMgr::IsCutsceneSkipButtonBeingPressed", expect = "6A 00 E8 59 9E 06 00")]
pub fn is_cutscene_skip_button_being_pressed() -> u8 {
    let pad = get_pad(0);
    if edge(pad.byte_add(0x20).cast(), pad.byte_add(0x50).cast()) {
        return 1;
    }
    get_pad(0);
    if g::<u8>(0xB73418).read() != 0 && g::<u8>(0xB7342C).read() == 0 {
        return 1;
    }
    get_pad(0);
    if edge(g(0xB733CC), g(0xB7315C)) || edge(g(0xB733EA), g(0xB7317A)) {
        return 1;
    }
    get_pad(0);
    if edge(g(0xB731E8), g(0xB72F78)) {
        return 1;
    }
    u8::from(skip_check_746070() == 0)
}

/// `CCutsceneMgr::AppendToNextCutscene(object, anim)` (0x4D5DB0): copy and lower-case both names
/// into slot `ms_numAppendObjectNames`, then increment it (EAX = the new count).
#[hook(addr = 0x4D5DB0, conv = cdecl, name = "CCutsceneMgr::AppendToNextCutscene", expect = "8B 44 24 04 56")]
pub fn append_to_next_cutscene(object: GPtr<u8>, anim: GPtr<u8>) -> i32 {
    let n = g::<i32>(NUM_APPENDED).read();
    let d = g::<u8>(APPEND_OBJECT_NAMES).byte_add((n << 5) as isize);
    strcpy(d, object);
    crt_strlwr(d);
    let n = g::<i32>(NUM_APPENDED).read();
    let d = g::<u8>(APPEND_ANIM_NAMES).byte_add((n << 5) as isize);
    strcpy(d, anim);
    crt_strlwr(d);
    let n = g::<i32>(NUM_APPENDED).read() + 1;
    g::<i32>(NUM_APPENDED).write(n);
    n
}

/// The player rebuild both `BuildCutscenePlayer` and `LoadCutsceneData` start with.
fn rebuild_player() {
    let p = find_player_ped(-1);
    clothes_rebuild_player_if_needed(p);
    streaming_request_model(1, 0x1C);
    streaming_load_all_requested_models(1);
    clothes_rebuild_cutscene_player(p, 1);
}

/// `CCutsceneMgr::BuildCutscenePlayer()` (0x4D5E20): rebuild the player's clothes, load model 1
/// (`CSPLAY`) and rebuild the cutscene player from it.
#[hook(addr = 0x4D5E20, conv = cdecl, name = "CCutsceneMgr::BuildCutscenePlayer", expect = "56 6A FF E8 E8 83 09 00")]
pub fn build_cutscene_player() {
    rebuild_player();
}

/// `CCutsceneMgr::RemoveCutscenePlayer()` (0x4D5E50): `SetMissionDoesntRequireModel(1)`.
#[hook(addr = 0x4D5E50, conv = cdecl, name = "CCutsceneMgr::RemoveCutscenePlayer", expect = "6A 01 E8 39 3E F3 FF")]
pub fn remove_cutscene_player() {
    streaming_set_mission_doesnt_require_model(1);
}

/// `CCutsceneMgr::Shutdown()` (0x4D5E60): `delete ms_pCutsceneDir` (the pointer is not cleared).
#[hook(addr = 0x4D5E60, conv = cdecl, name = "CCutsceneMgr::Shutdown", expect = "8B 0D 48 F8 B5 00")]
pub fn shutdown() {
    let d = g::<GPtr<u8>>(PCUTSCENE_DIR).read();
    if !d.is_null() {
        directory_dtor(d);
        crt_free2(d);
    }
}

/// `CCutsceneMgr::LoadCutsceneData(name)` (0x4D5E80): the player rebuild, `CUTS.DIR` read when the
/// directory is empty, then tail-jump `LoadCutsceneData_overlay(name)`.
#[hook(addr = 0x4D5E80, conv = cdecl, name = "CCutsceneMgr::LoadCutsceneData", expect = "56 6A FF E8 88 83 09 00")]
pub fn load_cutscene_data(name: GPtr<u8>) {
    rebuild_player();
    let d = g::<GPtr<u8>>(PCUTSCENE_DIR).read();
    if rd::<u32>(d, 8) == 0 {
        directory_read_dir_file(d, g(CUTS_DIR));
    }
    call_load_cutscene_data_overlay(name);
}

/// `CCutsceneMgr::DeleteCutsceneData()` (0x4D5ED0): `DeleteCutsceneData_overlay`,
/// `SetMissionDoesntRequireModel(1)`, and when `restoreEverythingAfterCutscene`, tail-jump
/// `LoadEverythingBecauseCutsceneDeletedAllOfIt`.
#[hook(addr = 0x4D5ED0, conv = cdecl, name = "CCutsceneMgr::DeleteCutsceneData", expect = "E8 8B 9E 0D 00")]
pub fn delete_cutscene_data() {
    delete_cutscene_data_overlay();
    streaming_set_mission_doesnt_require_model(1);
    if g::<u8>(RESTORE_EVERYTHING).read() != 0 {
        load_everything_because_cutscene_deleted_all_of_it();
    }
}

/// `FindCutsceneAudioTrackId(name)` (0x5AFA50): the track id (`_stricmp` match in the table at
/// 0x8D0AA8) zero-extended (the `_stricmp` result 0 is the upper half), or 0x0000FFFF (`or ax,
/// 0xFFFF` over the table's null terminator).
#[hook(addr = 0x5AFA50, conv = cdecl, name = "FindCutsceneAudioTrackId", expect = "A1 A8 0A 8D 00")]
pub fn find_cutscene_audio_track_id(name: GPtr<u8>) -> u32 {
    let mut e = g::<u8>(AUDIO_TRACKS);
    loop {
        let s = rd::<GPtr<u8>>(e, 0);
        if s.is_null() {
            return 0xFFFF;
        }
        if crt_stricmp(name, s) == 0 {
            return u32::from(rd::<u16>(e, 4));
        }
        e = e.byte_add(8);
    }
}

/// `CCutsceneMgr::CalculateBoundingSphereRadiusCB(RpAtomic*, void* data)` (0x5B0130): the atomic's
/// bounding sphere (resynced when flag 2 of +0x4C is set, twice as the inlined getter does), its
/// centre through every frame matrix up to (not including) the root, `|centre| + radius`; stored
/// to `*(float*)data` when larger. Returns the atomic. (The original keeps the radius in its own
/// argument slot: stack only.)
#[hook(addr = 0x5B0130, conv = cdecl, name = "CCutsceneMgr::CalculateBoundingSphereRadiusCB", expect = "83 EC 0C 57 8B 7C 24 14")]
pub fn calculate_bounding_sphere_radius_cb(atomic: GPtr<u8>, data: GPtr<f32>) -> GPtr<u8> {
    if rd::<u8>(atomic, 0x4C) & 2 != 0 {
        rp_atomic_resync_interpolated_sphere(atomic);
    }
    let radius = rd::<f32>(atomic, 0x28);
    if rd::<u8>(atomic, 0x4C) & 2 != 0 {
        rp_atomic_resync_interpolated_sphere(atomic);
    }
    let mut c = [rd::<f32>(atomic, 0x1C), rd::<f32>(atomic, 0x20), rd::<f32>(atomic, 0x24)];
    let mut f = rd::<GPtr<u8>>(atomic, 4);
    while !rd::<GPtr<u8>>(f, 4).is_null() {
        let cp = GPtr::from_raw(c.as_mut_ptr());
        rw_v3d_transform_points(cp, cp, 1, f.byte_add(0x10));
        f = rd::<GPtr<u8>>(f, 4);
    }
    let v = X87::from_f64_exact(rw_v3d_length(GPtr::from_raw(c.as_mut_ptr()))).fadd_m(radius);
    if v.fcom(x(data.read())) == Cmp::Greater {
        data.write(v.to_f32());
    }
    atomic
}

/// `CCutsceneMgr::UpdateCutsceneObjectBoundingBox(clump, modelId)` (0x5B01E0): the radius of the
/// cut-object col model of `modelId` (0x968A30 + (id - 300) * 0x30), at least 20.0, enlarged to
/// the clump's atomics (`CalculateBoundingSphereRadiusCB`), then the col model gets min(grown,
/// start clamped to >= 20) (sic, see below); bounding box ±radius. (The original
/// keeps the start radius in its `modelId` argument slot: stack only.)
#[hook(addr = 0x5B01E0, conv = cdecl, name = "CCutsceneMgr::UpdateCutsceneObjectBoundingBox", expect = "83 EC 10 8B 44 24 18")]
pub fn update_cutscene_object_bounding_box(clump: GPtr<u8>, model: i32) {
    let cm = g::<u8>(CUTOBJ_COLMODELS).byte_add((model.wrapping_mul(0x30).wrapping_sub(0x3840)) as isize);
    let mut r = rd::<f32>(cm, 0x24);
    let mut slot = r;
    rp_clump_for_all_atomics(clump, 0x5B0130, GPtr::from_raw(&mut r as *mut f32).cast());
    if x(slot).fcom(x(20.0)) == Cmp::Less {
        slot = 20.0;
    }
    // 0x5B0230..0x5B0241: `fld r; fcom slot; test ah, 0x41; jne keep` keeps `r` when r <= slot (or
    // unordered) and loads `slot` otherwise: the result is the SMALLER value (gta-reversed stores the
    // grown radius; the bytes do not)
    let big = if x(r).fcom(x(slot)) == Cmp::Greater { x(slot) } else { x(r) };
    wr::<f32>(cm, 0x24, big.to_f32());
    let n = big.fchs().to_f32().to_bits();
    let p = big.to_f32().to_bits();
    for k in 0..3 {
        wr::<u32>(cm, 4 * k, n);
    }
    for k in 0..3 {
        wr::<u32>(cm, 0xC + 4 * k, p);
    }
}

/// `CCutsceneMgr::AddCutsceneHead(object, anim)` (0x5B0380): returns 0 (`xor eax, eax; ret`). Not
/// hookable (3 bytes).
pub fn add_cutscene_head(_object: GPtr<u8>, _anim: i32) -> u32 {
    0
}

/// `CCutsceneMgr::SetHeadAnim(anim, head)` (0x5B0440): `ret`. Not hookable.
pub fn set_head_anim(_anim: GPtr<u8>, _head: GPtr<u8>) {}

/// `CCutsceneMgr::SetCutsceneAnim(animName, CObject* object)` (0x5B0390): a copy of the cutscene
/// group's animation (compressed hierarchies are kept compressed), time 0, flags `& ~1 | 0x40`,
/// linked at the head of the object clump's association list; when kept compressed, the clump's
/// frame data get flag 0x40.
#[hook(addr = 0x5B0390, conv = cdecl, name = "CCutsceneMgr::SetCutsceneAnim", expect = "8B 44 24 08 56")]
pub fn set_cutscene_anim(name: GPtr<u8>, object: GPtr<u8>) {
    let clump = rd::<GPtr<u8>>(object, 0x18);
    let st = assoc_group_get_animation(cutscene_associations(), name);
    if st.is_null() {
        return;
    }
    let h = rd::<GPtr<u8>>(st, 0x10);
    if rd::<u8>(h, 0xA) != 0 {
        wr::<u8>(h, 0xB, 1);
    }
    streaming_im_gonna_use_streaming_memory();
    let a = assoc_group_copy_animation(cutscene_associations(), name);
    streaming_i_have_used_streaming_memory();
    assoc_set_current_time(a, 0.0);
    wr::<u16>(a, 0x2E, (rd::<u16>(a, 0x2E) & 0xFFFE) | 0x40);
    let cd = rd::<GPtr<u8>>(clump, g::<i32>(ANIM_CLUMP_OFFSET).read() as isize);
    let link = a.byte_add(4);
    let first = rd::<GPtr<u8>>(cd, 0);
    if !first.is_null() {
        wr::<u32>(first, 4, link.addr() as u32);
    }
    wr::<u32>(link, 0, rd::<u32>(cd, 0));
    wr::<u32>(link, 4, cd.addr() as u32);
    wr::<u32>(cd, 0, link.addr() as u32);
    if rd::<u8>(rd::<GPtr<u8>>(a, 0x14), 0xB) != 0 {
        let fr = rd::<GPtr<u8>>(cd, 0x10);
        wr::<u8>(fr, 0, rd::<u8>(fr, 0) | 0x40);
    }
}

/// `CCutsceneMgr::SetCutsceneAnimToLoop(animName)` (0x5B0420): the static association's flags
/// (+0xA) |= 2 (no null check).
#[hook(addr = 0x5B0420, conv = cdecl, name = "CCutsceneMgr::SetCutsceneAnimToLoop", expect = "8B 44 24 04 50")]
pub fn set_cutscene_anim_to_loop(name: GPtr<u8>) {
    let a = assoc_group_get_animation(cutscene_associations(), name);
    wr::<u8>(a, 0xA, rd::<u8>(a, 0xA) | 2);
}

/// `CCutsceneMgr::AttachObjectToBone(CObject* attachment, CObject* object, int32 bone)` (0x5B0450):
/// `attachment->m_pAttachToObject (+0x180) = object`, `+0x17C = RpHAnimIDGetIndex(skin hierarchy
/// of object, bone)`.
#[hook(addr = 0x5B0450, conv = cdecl, name = "CCutsceneMgr::AttachObjectToBone", expect = "56 8B 74 24 0C")]
pub fn attach_object_to_bone(attachment: GPtr<u8>, object: GPtr<u8>, bone: i32) {
    let h = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(object, 0x18));
    let i = rp_hanim_id_get_index(h, bone);
    wr::<u32>(attachment, 0x180, object.addr() as u32);
    wr::<i32>(attachment, 0x17C, i);
}

/// `CCutsceneMgr::AttachObjectToFrame(CObject* attachment, CEntity* object, const char* frame)`
/// (0x5B0480): `+0x180 = 0`, `+0x17C = &frame->ltm`... the found frame's +0x10 (its modelling
/// matrix pointer area) as the original loads it.
#[hook(addr = 0x5B0480, conv = cdecl, name = "CCutsceneMgr::AttachObjectToFrame", expect = "8B 4C 24 0C 8B 44 24 08")]
pub fn attach_object_to_frame(attachment: GPtr<u8>, object: GPtr<u8>, frame: GPtr<u8>) {
    let f = rp_anim_blend_clump_find_frame(rd::<GPtr<u8>>(object, 0x18), frame);
    let v = rd::<u32>(f, 0x10);
    wr::<u32>(attachment, 0x180, 0);
    wr::<u32>(attachment, 0x17C, v);
}

/// `CCutsceneMgr::AttachObjectToParent(CObject* attachment, CEntity* object)` (0x5B04B0): `+0x180 =
/// 0`, `+0x17C = object's RW object's parent (+4)`.
#[hook(addr = 0x5B04B0, conv = cdecl, name = "CCutsceneMgr::AttachObjectToParent", expect = "8B 44 24 08 8B 48 18")]
pub fn attach_object_to_parent(attachment: GPtr<u8>, object: GPtr<u8>) {
    let v = rd::<u32>(rd::<GPtr<u8>>(object, 0x18), 4);
    wr::<u32>(attachment, 0x180, 0);
    wr::<u32>(attachment, 0x17C, v);
}

/// `CCutsceneMgr::GetCutsceneTimeInMilleseconds()` (0x5B0550): tail-jump `_ftol(ms_cutsceneTimer *
/// 1000.0)` (EDX:EAX).
#[hook(addr = 0x5B0550, conv = cdecl, name = "CCutsceneMgr::GetCutsceneTimeInMilleseconds", expect = "D9 05 08 3F BC 00")]
pub fn get_cutscene_time_in_milleseconds() -> u64 {
    x(g::<f32>(CUTSCENE_TIMER).read()).fmul_m(1000.0).ftol()
}

/// `CCutsceneMgr::HasCutsceneFinished()` (0x5B0570): without a loaded data file true; else
/// `TheCamera.GetPositionAlongSpline() == 1.0` exactly. AL.
#[hook(addr = 0x5B0570, conv = cdecl, name = "CCutsceneMgr::HasCutsceneFinished", expect = "A0 06 40 BC 00")]
pub fn has_cutscene_finished() -> u8 {
    if g::<u8>(DATA_FILE_LOADED).read() == 0 {
        return 1;
    }
    let p = X87::from_f64_exact(camera_get_position_along_spline(g(CAMERA)));
    u8::from(p.fcom(x(1.0)) == Cmp::Equal)
}

/// `CCutsceneMgr::LoadCutsceneData_overlay(name)` (0x5B13F0): `CTimer::Suspend`, processing on,
/// not skipped, the current zone models removed unless 0xBC4005, `CUTS.DIR` re-read, unused models
/// removed, `DrasticTidyUpMemory(true)`, the name copied to `ms_cutsceneName`,
/// `LoadCutsceneData_preload`, tail-jump `CTimer::Resume`.
#[hook(addr = 0x5B13F0, conv = cdecl, name = "CCutsceneMgr::LoadCutsceneData_overlay", expect = "E8 DB 05 FB FF")]
pub fn load_cutscene_data_overlay(name: GPtr<u8>) {
    timer_suspend();
    let keep = g::<u8>(BC4005).read();
    g::<u8>(CUTSCENE_PROCESSING).write(1);
    g::<u8>(WAS_SKIPPED).write(0);
    if keep == 0 {
        streaming_remove_current_zones_models();
    }
    let d = g::<GPtr<u8>>(PCUTSCENE_DIR).read();
    wr::<u32>(d, 8, 0);
    directory_read_dir_file(g::<GPtr<u8>>(PCUTSCENE_DIR).read(), g(CUTS_DIR));
    streaming_remove_unused_models_in_loaded_list();
    game_drastic_tidy_up_memory(1);
    strcpy(g(CUTSCENE_NAME), name);
    load_cutscene_data_preload();
    timer_resume();
}

/// `CCutsceneMgr::StartCutscene()` (0x5B1460): play status 1; with a data file: camera cut-scene
/// offset, spline control (mode 2), wide screen, help message cleared, plants pre-updated for the
/// camera position.
#[hook(addr = 0x5B1460, conv = cdecl, name = "CCutsceneMgr::StartCutscene", expect = "A0 06 40 BC 00")]
pub fn start_cutscene() {
    let loaded = g::<u8>(DATA_FILE_LOADED).read();
    g::<u32>(BC3FE0).write(1);
    if loaded == 0 {
        return;
    }
    camera_set_cam_cut_scene_offset(g(CAMERA), g(CUT_OFFSET));
    camera_take_control_with_spline(g(CAMERA), 2);
    camera_set_wide_screen_on(g(CAMERA));
    hud_set_help_message(0, 1, 0, 0);
    let m = g::<GPtr<u8>>(CAM_MATRIX).read();
    let pos = if m.is_null() { g(CAM_PLACEMENT_POS) } else { m.byte_add(0x30) };
    plant_mgr_pre_update_once_for_new_camera_pos(pos);
}

/// `CCutsceneMgr::SkipCutscene()` (0x5B1700): byte 0xBAAD40 = 0, `ms_wasCutsceneSkipped = 1`,
/// tail-jump `FinishCutscene`.
#[hook(addr = 0x5B1700, conv = cdecl, name = "CCutsceneMgr::SkipCutscene", expect = "C6 05 40 AD BA 00 00")]
pub fn skip_cutscene() {
    g::<u8>(BAAD40).write(0);
    g::<u8>(WAS_SKIPPED).write(1);
    finish_cutscene();
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INITIALISE,
    HOOK_LOAD_ANIMATION_UNCOMPRESSED,
    HOOK_UPDATE,
    HOOK_IS_CUTSCENE_SKIP_BUTTON_BEING_PRESSED,
    HOOK_APPEND_TO_NEXT_CUTSCENE,
    HOOK_BUILD_CUTSCENE_PLAYER,
    HOOK_REMOVE_CUTSCENE_PLAYER,
    HOOK_SHUTDOWN,
    HOOK_LOAD_CUTSCENE_DATA,
    HOOK_DELETE_CUTSCENE_DATA,
    HOOK_FIND_CUTSCENE_AUDIO_TRACK_ID,
    HOOK_CALCULATE_BOUNDING_SPHERE_RADIUS_CB,
    HOOK_UPDATE_CUTSCENE_OBJECT_BOUNDING_BOX,
    HOOK_SET_CUTSCENE_ANIM,
    HOOK_SET_CUTSCENE_ANIM_TO_LOOP,
    HOOK_ATTACH_OBJECT_TO_BONE,
    HOOK_ATTACH_OBJECT_TO_FRAME,
    HOOK_ATTACH_OBJECT_TO_PARENT,
    HOOK_GET_CUTSCENE_TIME_IN_MILLESECONDS,
    HOOK_HAS_CUTSCENE_FINISHED,
    HOOK_LOAD_CUTSCENE_DATA_OVERLAY,
    HOOK_START_CUTSCENE,
    HOOK_SKIP_CUTSCENE,
];
