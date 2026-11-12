//! PORT-VEH-3 (batch `world3-1`, SA:MP-executed rows): `CGameLogic::PassTime`,
//! `CGameLogic::SortOutStreamingAndMemory` and `CGameLogic::Remove2ndPlayerIfPresent`. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{g, rd, wr};

orig! {
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    fn stats_increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CClock::SetGameClock(uint8, uint8, uint8)` (0x52D150)
    fn set_game_clock(h: u32, m: u32, day: u32) = cdecl 0x52D150 => unported;
    /// `CPickups::PassTime(uint32)` (0x455200)
    fn pickups_pass_time(ms: u32) = cdecl 0x455200 => unported;
    /// `CTimer::Stop()` (0x561AA0)
    fn timer_stop() = cdecl 0x561AA0 => unported;
    /// `CTimer::Update()` (0x561B10)
    fn timer_update() = cdecl 0x561B10 => unported;
    /// `CTimeCycle::FindFarClipForCoors(CVector)` (0x5616E0), ST0
    fn find_far_clip_for_coors(x: u32, y: u32, z: u32) -> f64 = cdecl 0x5616E0 => unported;
    /// `RwCameraSetFarClipPlane(RwCamera*, float)` (0x7EE2A0)
    fn rw_camera_set_far_clip_plane(cam: GPtr<u8>, far: f32) -> GPtr<u8> = cdecl 0x7EE2A0 => unported;
    /// `CStreaming::FlushRequestList()` (0x40E4E0)
    fn flush_request_list() = cdecl 0x40E4E0 => unported;
    /// `CStreaming::DeleteRwObjectsAfterDeath(const CVector&)` (0x409210)
    fn delete_rw_objects_after_death(p: GPtr<u8>) = cdecl 0x409210 => unported;
    /// `CStreaming::RemoveUnusedModelsInLoadedList()` (0x407AC0)
    fn remove_unused_models_in_loaded_list() = cdecl 0x407AC0 => unported;
    /// `CGame::DrasticTidyUpMemory(bool)` (0x53C810)
    fn drastic_tidy_up_memory(b: u32) = cdecl 0x53C810 => unported;
    /// `CStreaming::LoadSceneCollision(const CVector&)` (0x40ED80)
    fn load_scene_collision(p: GPtr<u8>) = cdecl 0x40ED80 => unported;
    /// `CRenderer::RequestObjectsInDirection(const CVector&, float, int32)` (0x555CB0)
    fn request_objects_in_direction(p: GPtr<u8>, angle: u32, flags: u32) = cdecl 0x555CB0 => unported;
    /// `CStreaming::LoadScene(const CVector&)` (0x40EB70)
    fn load_scene(p: GPtr<u8>) = cdecl 0x40EB70 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// virtual slot 0: the scalar deleting destructor
    fn entity_vdelete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CClothes::RebuildPlayer(CPlayerPed*, bool)` (0x5A82C0)
    fn clothes_rebuild_player(ped: GPtr<u8>, ignore: u32) = cdecl 0x5A82C0 => unported;
}

/// `CGameLogic::PassTime(uint32 minutes)` (0x4414C0): advances `CClock` (minutes 0xB70152, hours
/// 0xB70153, week day 0xB7014E wrapping 7 -> 1) with one `IncrementStat(134, 1.0)` per day passed,
/// sets the clock, then `CPickups::PassTime(minutes * 1000)` (a tail call).
#[hook(addr = 0x4414C0, conv = cdecl, name = "CGameLogic::PassTime",
       expect = "8B 4C 24 04 55",
       unverified = "world3-1", route = "L2", hazards = "none",
       note = "the original stores minutes*1000 into its own argument slot for the tail call")]
pub fn pass_time(minutes: u32) {
    let mut h = i32::from(g::<u8>(0xB7_0153).read());
    let mut m = i32::from(g::<u8>(0xB7_0152).read()).wrapping_add(minutes as i32);
    let mut day = i32::from(g::<u8>(0xB7_014E).read());
    if m >= 60 {
        let q = (m as u32) / 60;
        m = m.wrapping_sub((q as i32).wrapping_mul(60));
        h = h.wrapping_add(q as i32);
    }
    if h >= 24 {
        let q = (h as u32) / 24;
        h = h.wrapping_sub((q as i32).wrapping_mul(24));
        for _ in 0..q {
            day += 1;
            if day > 7 {
                day = 1;
            }
            stats_increment_stat(0x86, 1.0);
        }
    }
    set_game_clock(h as u32, m as u32, day as u32);
    pickups_pass_time(minutes.wrapping_mul(1000));
}

/// `CGameLogic::SortOutStreamingAndMemory(const CVector&, float)` (0x441440): with the timer stopped,
/// the far clip plane of `Scene.m_pRwCamera` (0xB6F97C) from the time cycle at `pos`, the request
/// list flushed, objects after death deleted, unused models removed, a drastic memory tidy-up, the
/// scene collision and the objects in the direction `angle` requested, the scene loaded; then
/// `CTimer::Update` (a tail call).
#[hook(addr = 0x441440, conv = cdecl, name = "CGameLogic::SortOutStreamingAndMemory",
       expect = "51 56 E8 59 06 12 00",
       unverified = "world3-1", route = "split", hazards = "heap-writes",
       note = "loads models synchronously (file I/O) and frees memory; restarts the timer")]
pub fn sort_out_streaming_and_memory(pos: GPtr<u8>, angle: u32) {
    timer_stop();
    let far = X87::from_f64_exact(find_far_clip_for_coors(rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)))
        .to_f32();
    rw_camera_set_far_clip_plane(g::<GPtr<u8>>(0xB6_F97C).read(), far);
    flush_request_list();
    delete_rw_objects_after_death(pos);
    remove_unused_models_in_loaded_list();
    drastic_tidy_up_memory(1);
    load_scene_collision(pos);
    request_objects_in_direction(pos, angle, 0x20);
    load_scene(pos);
    timer_update();
}

/// `CGameLogic::Remove2ndPlayerIfPresent()` (0x4413C0): when `CWorld::Players[1].m_pPed` (0xB7CF28)
/// exists it leaves the world and is deleted, the slot cleared, player 1's clothes rebuilt and its
/// +0x474 bits 27/28 cleared; `CGameLogic::n2PlayerPedInFocus` (0x8A5E50) = 2 in every case.
/// The entry is `jmp 0x4011B9`, a protection fragment outside `.HOODLUM` that decodes the `Players[1]`
/// ped into EAX (`[[0x1560604] + [0x156E74D]]` = 0xB7CF28) and jumps back to 0x4413C5; a lone 5-byte
/// `jmp` that is not a `.HOODLUM` stub cannot be hooked in phase A, so this is a phase B native.
#[sa_core::native(addr = 0x4413C0, conv = cdecl, name = "CGameLogic::Remove2ndPlayerIfPresent")]
pub fn remove_2nd_player_if_present() {
    let p = g::<GPtr<u8>>(0xB7_CF28).read();
    if !p.is_null() {
        world_remove(p);
        let q = g::<GPtr<u8>>(0xB7_CF28).read();
        if !q.is_null() {
            entity_vdelete(q, 1);
        }
        g::<u32>(0xB7_CF28).write(0);
        clothes_rebuild_player(g::<GPtr<u8>>(0xB7_CD98).read(), 0);
        let p0 = g::<GPtr<u8>>(0xB7_CD98).read();
        wr::<u32>(p0, 0x474, rd::<u32>(p0, 0x474) & 0xE7FF_FFFF);
    }
    g::<u32>(0x8A_5E50).write(2);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_PASS_TIME,
    HOOK_SORT_OUT_STREAMING_AND_MEMORY,
];

/// This module's phase B natives.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_REMOVE_2ND_PLAYER_IF_PRESENT];
