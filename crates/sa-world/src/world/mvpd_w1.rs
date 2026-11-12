//! WRITER-MVP-D (helper-world) WORLD ports, batch `mvpd-world-1`: `CWorld::Process` and the per-frame
//! managers it and the game loop drive. Ported from the 1.0 US bytes (the exe wins over gta-reversed).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::x87::X87;

use super::common::{ftol32, g, rd, wr, x, xi};

orig! {
    /// `RwHelper::GetEventGlobalGroup()` (0x4ABA50, cdecl)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGlobalGroup::AddEventsToPed(CPed*)` (0x4AB9C0, `ret 4`)
    fn add_events_to_ped(grp: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4AB9C0 => unported;
    /// `CEventGlobalGroup::AddEventsToGroup(CPedGroup*)` (0x4AB8A0, `ret 4`)
    fn add_events_to_group(grp: GPtr<u8>, pg: GPtr<u8>) = thiscall 0x4AB8A0 => unported;
    /// `CInformFriendsEventQueue::Process()` (0x4AC2E0, cdecl)
    fn inform_friends_process() = cdecl 0x4AC2E0 => unported;
    /// `CInformGroupEventQueue::Process()` (0x4B2AE0, cdecl)
    fn inform_group_process() = cdecl 0x4B2AE0 => unported;
    /// `CEventGroup::Flush(bool)` (0x4AB370, `ret 4`)
    fn event_group_flush(grp: GPtr<u8>, b: u32) = thiscall 0x4AB370 => unported;
    /// `CReferences::PruneAllReferencesInWorld()` (0x571CC0, cdecl)
    fn prune_all_references_in_world() = cdecl 0x571CC0 => unported;
    /// `CEntity::UpdateAnim()` (0x535F00)
    fn entity_update_anim(e: GPtr<u8>) = thiscall 0x535F00 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70, `ret 4`)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40, `ret 4`)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// entity virtual +0x0 (scalar deleting destructor)
    fn v_delete(e: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// entity virtual +0xC (`Remove`)
    fn v_remove(e: GPtr<u8>) = thiscall vslot 0xC => unported;
    /// entity virtual +0x28 (`ProcessControl`)
    fn v_process_control(e: GPtr<u8>) = thiscall vslot 0x28 => unported;
    /// entity virtual +0x2C (`ProcessCollision`)
    fn v_process_collision(e: GPtr<u8>) = thiscall vslot 0x2C => unported;
    /// entity virtual +0x30 (`ProcessShift`)
    fn v_process_shift(e: GPtr<u8>) = thiscall vslot 0x30 => unported;
    /// `CPhysical::RemoveFromMovingList()` (0x542860)
    fn remove_from_moving_list(e: GPtr<u8>) = thiscall 0x542860 => crate::physical::base::remove_from_moving_list;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => crate::world::cworld::find_player_ped_port;
    /// `CPopulation::RemovePed(CPed*)` (0x610F20, cdecl)
    fn population_remove_ped(ped: GPtr<u8>) = cdecl 0x610F20 => unported;
    /// `CObject::ProcessControlLogic()` (0x5A29A0)
    fn object_process_control_logic(o: GPtr<u8>) = thiscall 0x5A29A0 => unported;
    /// `CLoadMonitor::StartTimer(uint32)` (0x53D050, `ret 4`)
    fn load_monitor_start_timer(m: GPtr<u8>, t: u32) = thiscall 0x53D050 => unported;
    /// `CLoadMonitor::EndTimer(uint32)` (0x53D070, `ret 4`)
    fn load_monitor_end_timer(m: GPtr<u8>, t: u32) = thiscall 0x53D070 => unported;
    /// `CPhysical::ApplyMoveSpeed()` (0x542DD0)
    fn physical_apply_move_speed(e: GPtr<u8>) = thiscall 0x542DD0 => unported;
    /// `CPhysical::ApplyTurnSpeed()` (0x542E20)
    fn physical_apply_turn_speed(e: GPtr<u8>) = thiscall 0x542E20 => unported;
    /// `CVehicleRecording::SaveOrRetrieveDataForThisFrame()` (0x45A610, cdecl)
    fn vehicle_recording_save_or_retrieve() = cdecl 0x45A610 => unported;
    /// `IKChainManager_c::Update(float)` (0x6186D0, `ret 4`)
    fn ik_chain_manager_update(m: GPtr<u8>, dt: f32) = thiscall 0x6186D0 => unported;
    /// `CWorld::ProcessAttachedEntities()` (0x5647F0, cdecl)
    fn world_process_attached_entities() = cdecl 0x5647F0 => unported;
    /// `CPedIntelligence::ProcessAfterProcCol()` (0x6018F0)
    fn ped_intelligence_process_after_proc_col(i: GPtr<u8>) = thiscall 0x6018F0 => unported;
    /// `CPed::PositionAttachedPed()` (0x5DFDF0)
    fn ped_position_attached_ped(p: GPtr<u8>) = thiscall 0x5DFDF0 => unported;
    /// `CMessages::Process()` (0x69EE60, cdecl)
    fn messages_process() = cdecl 0x69EE60 => unported;
    /// `CPlayerInfo::Process(int32)` (0x56F8D0, `ret 4`)
    fn player_info_process(p: GPtr<u8>, id: i32) = thiscall 0x56F8D0 => unported;
    /// `CPedScriptedTaskRecord::Process()` (0x6085E0, cdecl)
    fn ped_scripted_task_record_process() = cdecl 0x6085E0 => unported;
    /// `CPedGroups::Process()` (0x5FC800, cdecl)
    fn ped_groups_process() = cdecl 0x5FC800 => unported;
    /// `CWorld::RemoveFallenPeds()` (0x565CB0, cdecl)
    fn world_remove_fallen_peds() = cdecl 0x565CB0 => unported;
    /// `CWorld::RemoveFallenCars()` (0x565E80, cdecl)
    fn world_remove_fallen_cars() = cdecl 0x565E80 => unported;
}

/// `CTimer::m_FrameCounter` (0xB7CB4C)
const FRAME_COUNTER: usize = 0xB7CB4C;
/// `CTimer::ms_fTimeStep` (0xB7CB5C)
const TIME_STEP: usize = 0xB7CB5C;
/// `CWorld::ms_listMovingEntityPtrs` head (0xB9ACC8)
const MOVING_LIST: usize = 0xB9ACC8;
/// `CWorld::ms_listObjectsWithControlCode` head (0xB9ACCC)
const CONTROL_CODE_LIST: usize = 0xB9ACCC;
/// `g_LoadMonitor` (0xB72978)
const LOAD_MONITOR: usize = 0xB72978;

/// The entity type (`+0x36 & 7`).
fn ent_type(e: GPtr<u8>) -> u8 {
    rd::<u8>(e, 0x36) & 7
}

/// The inlined `CEntity::UpdateRwMatrix` (0x446F90) followed by `CEntity::UpdateRwFrame` (0x532B00): with an
/// RW object, its frame matrix (`rw+4 -> +0x10`) from the entity matrix (+0x14) or the simple transform (+4).
fn update_rw_matrix_and_frame(e: GPtr<u8>) {
    let rw = rd::<GPtr<u8>>(e, 0x18);
    if !rw.is_null() {
        let m = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let mat = rd::<GPtr<u8>>(e, 0x14);
        if mat.is_null() {
            simple_transform_update_rw_matrix(e.byte_add(4), m);
        } else {
            matrix_update_rw_matrix(mat, m);
        }
    }
    entity_update_rw_frame(e);
}

/// Walks a `CPtrList` (node = `{item, next}`), reading `next` before the item's work, as the original does.
fn walk(head: usize, mut f: impl FnMut(GPtr<u8>)) {
    let mut node = g::<GPtr<u8>>(head).read();
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        f(e);
    }
}

/// The deletion / control step `CWorld::Process` runs for a moving entity (twice: all entities, then those with
/// flag `+0x1C` bit 6 again): flagged for removal (`+0x1C & 0x800`), non-peds are removed (`vslot 0xC`), taken
/// off the moving list when physical (type 2..4) and deleted (`vslot 0`); the local player ped is only removed
/// and taken off the list; other peds go through `CPopulation::RemovePed`. Otherwise `ProcessControl`
/// (`vslot 0x28`) and, when static (`+0x1C & 4`) or flag 0x40000, off the moving list.
fn process_control_or_remove(e: GPtr<u8>) {
    if rd::<u32>(e, 0x1C) & 0x800 != 0 {
        if ent_type(e) != 3 {
            v_remove(e);
            let t = ent_type(e);
            if t > 1 && t < 5 {
                remove_from_moving_list(e);
            }
            v_delete(e, 1);
        } else if find_player_ped(-1) == e {
            v_remove(e);
            let t = ent_type(e);
            if t > 1 && t < 5 {
                remove_from_moving_list(e);
            }
        } else {
            population_remove_ped(e);
        }
    } else {
        v_process_control(e);
        let f = rd::<u32>(e, 0x1C);
        if f & 4 != 0 || f & 0x40000 != 0 {
            remove_from_moving_list(e);
        }
    }
}

/// `CWorld::Process()` (0x5684A0, cdecl): the per-frame world update.
///
/// 1. Global events to every moving ped (`AddEventsToPed`) and to the 8 active ped groups (0xC09920, 0x2D4
///    bytes, active byte 0xC098E0+i), the inform queues, `CEventGroup::Flush(false)`; every 64 frames
///    `PruneAllReferencesInWorld`.
/// 2. Cutscene-only mode (`0xB7CD6D` and `CCutsceneMgr::ms_running` 0xB5F851): the 50 cutscene objects
///    (0xBC3F18) with a non-null `+0xB0` get `UpdateAnim`, `ProcessControl`, `ProcessCollision` and
///    their RW frame updated; nothing else runs.
/// 3. Otherwise: `UpdateAnim` for moving entities not flagged for removal, the control/removal pass (above),
///    again for entities with `+0x1C` bit 6 under `0xB7CD6E = 1`, `ProcessControlLogic` for the control-code
///    objects, then (load monitor timer 1) either the replay path (`CReplay::Mode` 0xA43088 == 1: every moving
///    entity flagged `+0x1C |= 0x20` and its frame updated) or the collision passes: one `ProcessCollision` pass
///    with `0xB7CD72 = 0`, four with 1, one more setting `+0x1C |= 0x10` around it, then two `ProcessShift`
///    passes (`0xB7CD6C` = 0 then 1); in the last one entities still not flagged 0x20 with status bits
///    (`+0x36 & 0xF8`) zero have their move speed (+0x44..+0x4C) scaled by `pow([0x8CDEEC], timestep)` and
///    get `ApplyMoveSpeed` / `ApplyTurnSpeed`. Entities with `+0x1C & 0x20` skip every pass.
/// 4. Vehicle recordings, `IKChainManager_c::Update(timestep * 0.02)`, attached entities, ped post-collision
///    (`ProcessAfterProcCol`; peds attached to `+0xFC` copy it to `+0x128` and are positioned), messages, both
///    players' `CPlayerInfo::Process`, scripted task records, ped groups, and on frame `& 7` == 1 / 5
///    `RemoveFallenPeds` / `RemoveFallenCars` (tail calls).
#[hook(addr = 0x5684A0, conv = cdecl, name = "CWorld::Process", expect = "56 8B 35 C8 AC B9 00",
       unverified = "mvpd-world-1", route = "L3-only", hazards = "heap-writes",
       recs = "grove-walk", note = "drives the whole frame: every call is a nested catalogued function")]
pub fn world_process() {
    walk(MOVING_LIST, |e| {
        if ent_type(e) == 3 {
            let grp = get_event_global_group();
            add_events_to_ped(grp, e);
        }
    });
    for i in 0..8usize {
        if g::<u8>(0xC098E0 + i).read() != 0 {
            let grp = get_event_global_group();
            add_events_to_group(grp, GPtr::from_addr(0xC09920 + 0x2D4 * i));
        }
    }
    inform_friends_process();
    inform_group_process();
    let grp = get_event_global_group();
    event_group_flush(grp, 0);
    if g::<u32>(FRAME_COUNTER).read() & 0x3F == 0 {
        prune_all_references_in_world();
    }

    if g::<u8>(0xB7CD6D).read() != 0 && g::<u8>(0xB5F851).read() != 0 {
        for i in 0..50usize {
            let e = g::<GPtr<u8>>(0xBC3F18 + 4 * i).read();
            if e.is_null() || rd::<u32>(e, 0xB0) == 0 {
                continue;
            }
            entity_update_anim(e);
            v_process_control(e);
            v_process_collision(e);
            update_rw_matrix_and_frame(e);
        }
        return;
    }

    walk(MOVING_LIST, |e| {
        if rd::<u32>(e, 0x1C) & 0x800 == 0 {
            entity_update_anim(e);
        }
    });
    walk(MOVING_LIST, process_control_or_remove);
    g::<u8>(0xB7CD6E).write(1);
    walk(MOVING_LIST, |e| {
        if (rd::<u32>(e, 0x1C) >> 6) & 1 != 0 {
            process_control_or_remove(e);
        }
    });
    g::<u8>(0xB7CD6E).write(0);
    walk(CONTROL_CODE_LIST, object_process_control_logic);
    load_monitor_start_timer(GPtr::from_addr(LOAD_MONITOR), 1);

    let not_skipped = |e: GPtr<u8>| (rd::<u32>(e, 0x1C) >> 5) & 1 == 0;
    let mark_10 = |e: GPtr<u8>| {
        let f = rd::<u32>(e, 0x1C);
        if (f >> 5) & 1 == 0 {
            wr::<u32>(e, 0x1C, f | 0x10);
            true
        } else {
            false
        }
    };
    if g::<u8>(0xA43088).read() == 1 {
        walk(MOVING_LIST, |e| {
            wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) | 0x20);
            update_rw_matrix_and_frame(e);
        });
    } else {
        g::<u8>(0xB7CD72).write(0);
        walk(MOVING_LIST, |e| {
            if not_skipped(e) {
                v_process_collision(e);
                update_rw_matrix_and_frame(e);
            }
        });
        g::<u8>(0xB7CD72).write(1);
        for _ in 0..4 {
            walk(MOVING_LIST, |e| {
                if not_skipped(e) {
                    v_process_collision(e);
                    update_rw_matrix_and_frame(e);
                }
            });
        }
        walk(MOVING_LIST, |e| {
            if mark_10(e) {
                v_process_collision(e);
                update_rw_matrix_and_frame(e);
                mark_10(e);
            }
        });
        g::<u8>(0xB7CD6C).write(0);
        walk(MOVING_LIST, |e| {
            if not_skipped(e) {
                v_process_shift(e);
                update_rw_matrix_and_frame(e);
                mark_10(e);
            }
        });
        g::<u8>(0xB7CD6C).write(1);
        walk(MOVING_LIST, |e| {
            if !not_skipped(e) {
                return;
            }
            v_process_shift(e);
            update_rw_matrix_and_frame(e);
            if !mark_10(e) || rd::<u8>(e, 0x36) & 0xF8 != 0 {
                return;
            }
            let k = X87::from_f64_exact(sa_core::orig::x87_binary(
                0x822130,
                f64::from(g::<f32>(0x8CDEEC).read()),
                f64::from(g::<f32>(TIME_STEP).read()),
            ));
            wr::<f32>(e, 0x44, k.fmul_m(rd::<f32>(e, 0x44)).to_f32());
            wr::<f32>(e, 0x48, k.fmul_m(rd::<f32>(e, 0x48)).to_f32());
            wr::<f32>(e, 0x4C, k.fmul_m(rd::<f32>(e, 0x4C)).to_f32());
            physical_apply_move_speed(e);
            physical_apply_turn_speed(e);
        });
    }
    load_monitor_end_timer(GPtr::from_addr(LOAD_MONITOR), 1);
    vehicle_recording_save_or_retrieve();
    let dt = x(g::<f32>(TIME_STEP).read()).fmul_m(g::<f32>(0x858B38).read()).to_f32();
    ik_chain_manager_update(GPtr::from_addr(0xC15448), dt);
    world_process_attached_entities();
    walk(MOVING_LIST, |e| {
        if ent_type(e) != 3 {
            return;
        }
        ped_intelligence_process_after_proc_col(rd::<GPtr<u8>>(e, 0x47C));
        let att = rd::<u32>(e, 0xFC);
        if att != 0 {
            wr::<u32>(e, 0x128, att);
            ped_position_attached_ped(e);
            update_rw_matrix_and_frame(e);
        }
    });
    messages_process();
    for i in 0..2usize {
        let p = GPtr::<u8>::from_addr(0xB7CD98 + 0x190 * i);
        if rd::<u32>(p, 0) != 0 {
            player_info_process(p, i as i32);
        }
    }
    ped_scripted_task_record_process();
    ped_groups_process();
    match g::<u32>(FRAME_COUNTER).read() & 7 {
        1 => world_remove_fallen_peds(),
        5 => world_remove_fallen_cars(),
        _ => {}
    }
}


orig! {
    /// `CPad::GetPad(int32)` (0x53FB70, cdecl)
    fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CReplay::Init()` (0x45E220, cdecl)
    fn replay_init() = cdecl 0x45E220 => unported;
    /// `CReplay::PlayBackThisFrame()` (0x4604A0, cdecl)
    fn replay_play_back_this_frame() = cdecl 0x4604A0 => unported;
    /// `CReplay::RecordThisFrame()` (0x45E300, cdecl)
    fn replay_record_this_frame_o() = cdecl 0x45E300 => crate::world::mvpd_w1::replay_record_this_frame;
    /// `CControllerConfigManager::GetIsKeyboardKeyJustDown(RsKeyCodes)` (0x52E450, `ret 4`)
    fn get_is_keyboard_key_just_down(m: GPtr<u8>, key: u32) -> u8 = thiscall 0x52E450 => unported;
    /// `CReplay::TriggerPlayback(eReplayCamMode, CVector fixedCamPos, bool loadScene)` (0x4600F0, cdecl)
    fn replay_trigger_playback(mode: u32, px: f32, py: f32, pz: f32, load_scene: u32) = cdecl 0x4600F0 => unported;
    /// `CReplay::SaveReplayToHD()` (0x45C340, cdecl)
    fn replay_save_replay_to_hd() = cdecl 0x45C340 => unported;
    /// `CReplay::PlayReplayFromHD()` (0x460390, cdecl)
    fn replay_play_replay_from_hd() = cdecl 0x460390 => unported;
    /// `CReplay::FinishPlayback()` (0x45F050, cdecl)
    fn replay_finish_playback() = cdecl 0x45F050 => unported;
    /// `CFont::SetScale(float, float)` (0x719380, cdecl)
    fn font_set_scale(w: f32, h: f32) = cdecl 0x719380 => unported;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610, cdecl)
    fn font_set_orientation(o: u32) = cdecl 0x719610 => unported;
    /// `CFont::SetBackground(bool, bool)` (0x7195C0, cdecl)
    fn font_set_background(a: u32, b: u32) = cdecl 0x7195C0 => unported;
    /// `CFont::SetCentreSize(float)` (0x7194E0, cdecl)
    fn font_set_centre_size(v: f32) = cdecl 0x7194E0 => unported;
    /// `CFont::SetProportional(bool)` (0x7195B0, cdecl)
    fn font_set_proportional(v: u32) = cdecl 0x7195B0 => unported;
    /// `CRGBA::CRGBA(uint8, uint8, uint8, uint8)` (0x7170C0, `ret 0x10`); returns `this`
    fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CFont::SetColor(CRGBA)` (0x719430, cdecl)
    fn font_set_color(c: u32) = cdecl 0x719430 => unported;
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490, cdecl)
    fn font_set_font_style(s: u32) = cdecl 0x719490 => unported;
    /// `CText::Get(const char*)` (0x6A0050, `ret 4`)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CFont::PrintString(float, float, const char*)` (0x71A700, cdecl)
    fn font_print_string(px: f32, py: f32, s: GPtr<u8>) = cdecl 0x71A700 => unported;
    /// `CReplay::MarkEverythingAsNew()` (0x45D430, cdecl)
    fn replay_mark_everything_as_new() = cdecl 0x45D430 => unported;
    /// `CMatrix::CopyOnlyMatrix(const CMatrix&)` (0x59ADD0, `ret 4`)
    fn matrix_copy_only_matrix(m: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59ADD0 => unported;
    /// `FindPlayerCoors(CVector* out, int32 playerId)` (0x56E010, cdecl); returns `out`
    fn find_player_coors(out: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x56E010 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `tReplayVehicleBlock::MakeVehicleUpdateData(CVehicle*, int32)` (0x45B730, `ret 8`)
    fn make_vehicle_update_data(b: GPtr<u8>, v: GPtr<u8>, i: i32) = thiscall 0x45B730 => unported;
    /// `tReplayBmxBlock::MakeBmxUpdateData(CBmx*, int32)` (0x45BF90, `ret 8`)
    fn make_bmx_update_data(b: GPtr<u8>, v: GPtr<u8>, i: i32) = thiscall 0x45BF90 => crate::world::mvpb_world::replay_make_bmx_update_data;
    /// `tReplayBikeBlock::MakeBikeUpdateData(CBike*, int32)` (0x45C070, `ret 8`)
    fn make_bike_update_data(b: GPtr<u8>, v: GPtr<u8>, i: i32) = thiscall 0x45C070 => crate::world::mvpb_world::replay_make_bike_update_data;
    /// `tReplayTrainBlock::MakeTrainUpdateData(CTrain*, int32)` (0x45CD10, `ret 8`)
    fn make_train_update_data(b: GPtr<u8>, v: GPtr<u8>, i: i32) = thiscall 0x45CD10 => unported;
    /// `CReplay::StoreClothesDesc(const CPedClothesDesc&, tReplayClothesBlock*)` (0x45C750, cdecl)
    fn replay_store_clothes_desc(desc: GPtr<u8>, out: GPtr<u8>) = cdecl 0x45C750 => crate::world::mvpb_world::replay_store_clothes_desc;
    /// `CReplay::StorePedUpdate(CPed*, int32)` (0x45C940, cdecl)
    fn replay_store_ped_update(p: GPtr<u8>, i: i32) = cdecl 0x45C940 => unported;
    /// `RpAnimBlendClumpGetMainAssociation(RpClump*, CAnimBlendAssociation**, float*)` (0x4D6910, cdecl)
    fn clump_get_main_association(c: GPtr<u8>, second: GPtr<GPtr<u8>>, blend: GPtr<f32>) -> GPtr<u8> = cdecl 0x4D6910 => unported;
    /// `RpAnimBlendClumpGetMainPartialAssociation(RpClump*)` (0x4D69A0, cdecl)
    fn clump_get_main_partial_association(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D69A0 => unported;
}

/// `CReplay::Mode` (0xA43088): 0 recording, 1 playback
const REPLAY_MODE: usize = 0xA43088;
/// `CReplay::Record.m_nOffset` (0x97FB70): write position in the current buffer
const REC_POS: usize = 0x97FB70;
/// `CReplay::Record.m_pBase` (0x97FB74): the current buffer
const REC_BASE: usize = 0x97FB74;
/// `ControlsManager` (0xB70198)
const CONTROLS_MANAGER: usize = 0xB70198;

/// `CReplay::Update()` (0x460500, cdecl): `Init` when a cutscene is processing (0xB5F852), pad 0's word
/// +0x10E is set, 0xBA67A4 or 0x96A7CC is set. Otherwise, unless the game is paused (`CTimer::m_UserPause` /
/// `m_CodePause` 0xB7CB48/0xB7CB49): playback mode plays the frame; recording mode records one when more
/// than 26 ms passed since 0x97FB40 (`CTimer::m_snTimeInMilliseconds` 0xB7CB84, unsigned difference). Then,
/// with 0xC3EFAB clear and the debug flag 0x8A6160 set, the keyboard keys 0x3E9/0x3EA/0x3EB trigger
/// playback / save to HD / play from HD while recording, and 0x3E9 or 0x3EB finish playback (all tail calls).
#[hook(addr = 0x460500, conv = cdecl, name = "CReplay::Update", expect = "A0 52 F8 B5 00",
       unverified = "mvpd-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn replay_update() {
    if g::<u8>(0xB5F852).read() != 0
        || rd::<u16>(pad_get_pad(0), 0x10E) != 0
        || g::<u8>(0xBA67A4).read() != 0
        || g::<u32>(0x96A7CC).read() != 0
    {
        replay_init();
        return;
    }
    if g::<u8>(0xB7CB48).read() != 0 || g::<u8>(0xB7CB49).read() != 0 {
        return;
    }
    match g::<u8>(REPLAY_MODE).read() {
        0 => {
            let now = g::<u32>(0xB7CB84).read();
            let last = g::<u32>(0x97FB40);
            if now.wrapping_sub(last.read()) > 0x1A {
                last.write(now);
                replay_record_this_frame_o();
            }
        }
        1 => replay_play_back_this_frame(),
        _ => {}
    }
    if g::<u8>(0xC3EFAB).read() != 0 || g::<u8>(0x8A6160).read() == 0 {
        return;
    }
    let cm = GPtr::<u8>::from_addr(CONTROLS_MANAGER);
    match g::<u8>(REPLAY_MODE).read() {
        0 => {
            if get_is_keyboard_key_just_down(cm, 0x3E9) != 0 {
                replay_trigger_playback(0, 0.0, 0.0, 0.0, 0);
            } else if get_is_keyboard_key_just_down(cm, 0x3EA) != 0 {
                replay_save_replay_to_hd();
            } else if get_is_keyboard_key_just_down(cm, 0x3EB) != 0 {
                replay_play_replay_from_hd();
            }
        }
        1 if get_is_keyboard_key_just_down(cm, 0x3E9) != 0 || get_is_keyboard_key_just_down(cm, 0x3EB) != 0 => {
            replay_finish_playback();
        }
        _ => {}
    }
}

/// `CReplay::Display()` (0x45C210, cdecl): outside recording mode the frame counter 0xA43240 advances
/// (`& 0xFFFF`); on frames with bit 5 set the font is set up (scale `width * 0.0015625 * 1.5`,
/// `height * 0.002232143 * 1.5` of `RsGlobal` 0xC17044/0xC17048, orientation 1, no background, centre size
/// `width - 20`, proportional, colour (255, 255, 200, 200) built through `CRGBA::CRGBA`, style 1) and in playback
/// mode the text of key 0x859C4C is printed at `(width / 10, height / 15)` (integer divisions).
#[hook(addr = 0x45C210, conv = cdecl, name = "CReplay::Display", expect = "51 A0 88 30 A4 00",
       route = "L2", hazards = "none")]
pub fn replay_display() {
    if g::<u8>(REPLAY_MODE).read() == 0 {
        return;
    }
    let ctr = g::<u32>(0xA43240);
    let c = ctr.read().wrapping_add(1) & 0xFFFF;
    ctr.write(c);
    if c & 0x20 == 0 {
        return;
    }
    let w = g::<i32>(0xC17044).read();
    let h = g::<i32>(0xC17048).read();
    let k = g::<f32>(0x858CE8).read();
    let sy = xi(h).fmul_m(g::<f32>(0x859524).read()).fmul_m(k).to_f32();
    let sx = xi(w).fmul_m(g::<f32>(0x859520).read()).fmul_m(k).to_f32();
    font_set_scale(sx, sy);
    font_set_orientation(1);
    font_set_background(0, 0);
    font_set_centre_size(xi(g::<i32>(0xC17044).read().wrapping_add(-0x14)).to_f32());
    font_set_proportional(1);
    let mut rgba = 0u32;
    let col = rgba_ctor(GPtr::from_raw(&mut rgba).cast(), 0xFF, 0xFF, 0xC8, 0xC8);
    font_set_color(rd::<u32>(col, 0));
    font_set_font_style(1);
    if g::<u8>(REPLAY_MODE).read() != 1 {
        return;
    }
    let s = text_get(GPtr::from_addr(0xC1B340), GPtr::from_addr(0x859C4C));
    let py = xi(g::<i32>(0xC17048).read() / 15).to_f32();
    let px = xi(g::<i32>(0xC17044).read() / 10).to_f32();
    font_print_string(px, py, s);
}

/// The replay buffer byte pointer at the current write position.
fn rec_ptr() -> GPtr<u8> {
    GPtr::from_addr(g::<u32>(REC_BASE).read().wrapping_add(g::<u32>(REC_POS).read()) as usize)
}

/// Advances the write position.
fn rec_advance(n: u32) {
    let p = g::<u32>(REC_POS);
    p.write(p.read().wrapping_add(n));
}

/// Pool slot `i` of a `CPool` (`{objects, flags, size}`) when it is live (flag byte >= 0) and non-null.
fn pool_live(pool: GPtr<u8>, i: i32, size: isize) -> Option<GPtr<u8>> {
    if (rd::<u8>(rd::<GPtr<u8>>(pool, 4), i as isize) as i8) < 0 {
        return None;
    }
    let e = rd::<GPtr<u8>>(pool, 0).byte_add(size * i as isize);
    if e.is_null() { None } else { Some(e) }
}

/// `CReplay::RecordThisFrame()` (0x45E300, cdecl).
///
/// First the frame's size is computed: 0x74 + per live vehicle with an RW object (pool 0xB74494, 0xA18 bytes,
/// type `+0x594`) 0x34 (types 0, 1, 2, 5, 11), 0x38 (3, 9, 10), 0x3C (4), 0x4C (6), 0 (7, 8) + per live ped with
/// an RW object (pool 0xB74490, 0x7C4 bytes) 0x34, plus 8 (and 0x78 for a player) if it was not recorded yet
/// (`+0x470 & 0x2000` clear) + 0x1C per active bullet trace (16 of 0x2C bytes at 0xC7C748, active byte +0x18).
/// When `pos + size + 0x14 > 0x18690` the buffer is closed (byte 0), its slot state (0x97FB7C+i) set to 1 and the
/// next of the 8 buffers (0x97FB88 + 0x186A0*i) opened (state 2, pos 0, first byte 0) and `MarkEverythingAsNew`
/// runs.
///
/// Then the blocks: 5 (camera matrix 0xB6F99C copied with `CMatrix::CopyOnlyMatrix`, player position through
/// three `FindPlayerCoors` calls, and whether the current player's `CPlayerInfo` +0xB0 is set), 6 (bytes
/// 0xB70153 / 0xB70152), 7 (bytes 0xC81320 / 0xC8131C and the dword 0xC8130C), 9 (game time), one block per
/// vehicle (`MakeVehicleUpdateData` with type 1 / 0x10 (+`+0x84C`) / 0x11 (+`+0x9C4`/`+0x9C8`), the bmx, bike and
/// train writers), per ped a type 3 block (index, model with 0x122..=0x12C stored as 7, `+0x598`) and the
/// player's clothes on its first record (then `+0x470 |= 0x2000`) followed by `StorePedUpdate`, type 0xA per
/// active bullet trace (index, 0x18 bytes), type 0xC (0xB6F084, 0xB6F154, bit 0/1 of +0xD from 0xC7C70C/0xC7C70D
/// with the other bits of the buffer byte kept, +0xC from 0xB72914), the end block 8 and a zero byte after it.
/// The vehicle count is read once for the size pass and on every iteration of the record passes.
#[hook(addr = 0x45E300, conv = cdecl, name = "CReplay::RecordThisFrame", expect = "83 EC 14 53 8B 1D 90 44 B7 00",
       unverified = "mvpd-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the replay buffers 0x97FB88.. (8 x 0x186A0 bytes)")]
pub fn replay_record_this_frame() {
    let peds = g::<GPtr<u8>>(0xB74490).read();
    let vehs = g::<GPtr<u8>>(0xB74494).read();
    let mut size: u32 = 0x74;
    let nveh = rd::<i32>(vehs, 8);
    for i in 0..nveh.max(0) {
        if rd::<u8>(rd::<GPtr<u8>>(vehs, 4), i as isize) & 0x80 != 0 {
            continue;
        }
        let v = rd::<GPtr<u8>>(vehs, 0).byte_add(0xA18 * i as isize);
        if v.is_null() || rd::<u32>(v, 0x18) == 0 {
            continue;
        }
        size += match rd::<u32>(v, 0x594) {
            0 | 1 | 2 | 5 | 11 => 0x34,
            3 | 9 | 10 => 0x38,
            4 => 0x3C,
            6 => 0x4C,
            _ => 0,
        };
    }
    let mut i = 0;
    while i < rd::<i32>(peds, 8) {
        if let Some(p) = pool_live(peds, i, 0x7C4)
            && rd::<u32>(p, 0x18) != 0
        {
            if rd::<u32>(p, 0x470) & 0x2000 == 0 {
                size += 8;
                if ped_is_player(p) != 0 {
                    size += 0x78;
                }
            }
            size += 0x34;
        }
        i += 1;
    }
    for j in 0..16usize {
        if g::<u8>(0xC7C760 + 0x2C * j).read() != 0 {
            size += 0x1C;
        }
    }
    let pos = g::<u32>(REC_POS).read();
    if pos.wrapping_add(size).wrapping_add(0x14) > 0x18690 {
        wr::<u8>(rec_ptr(), 0, 0);
        let slot = g::<u8>(0x97FB78);
        let cur = slot.read() as usize;
        g::<u8>(0x97FB7C + cur).write(1);
        let next = ((cur + 1) % 8) as u8;
        slot.write(next);
        g::<u8>(0x97FB7C + next as usize).write(2);
        let base = 0x97FB88u32 + 0x186A0 * next as u32;
        g::<u32>(REC_BASE).write(base);
        g::<u32>(REC_POS).write(0);
        g::<u8>(base as usize).write(0);
        replay_mark_everything_as_new();
    }

    // block 5: camera and player position
    let b = rec_ptr();
    wr::<u8>(b, 0, 5);
    matrix_copy_only_matrix(b.byte_add(4), GPtr::from_addr(0xB6F99C));
    let mut tmp = [0f32; 3];
    let t = GPtr::from_raw(&mut tmp).cast::<u8>();
    wr::<u32>(b, 0x4C, rd::<u32>(find_player_coors(t, -1), 0));
    wr::<u32>(b, 0x50, rd::<u32>(find_player_coors(t, -1), 4));
    wr::<u32>(b, 0x54, rd::<u32>(find_player_coors(t, -1), 8));
    let cur_player = g::<u8>(0xB7CD74).read() as usize;
    wr::<u8>(b, 1, (g::<u32>(0xB7CE48 + 0x190 * cur_player).read() != 0) as u8);
    rec_advance(0x58);
    // block 6
    let b = rec_ptr();
    wr::<u8>(b, 0, 6);
    wr::<u8>(b, 1, g::<u8>(0xB70153).read());
    wr::<u8>(b, 2, g::<u8>(0xB70152).read());
    rec_advance(4);
    // block 7
    let b = rec_ptr();
    wr::<u8>(b, 0, 7);
    wr::<u8>(b, 1, g::<u8>(0xC81320).read());
    wr::<u8>(b, 2, g::<u8>(0xC8131C).read());
    wr::<u32>(b, 4, g::<u32>(0xC8130C).read());
    rec_advance(8);
    // block 9: time
    let b = rec_ptr();
    wr::<u8>(b, 0, 9);
    wr::<u32>(b, 4, g::<u32>(0xB7CB84).read());
    rec_advance(8);

    let mut i = 0;
    while i < rd::<i32>(vehs, 8) {
        if let Some(v) = pool_live(vehs, i, 0xA18)
            && rd::<u32>(v, 0x18) != 0
        {
            let n = match rd::<u32>(v, 0x594) {
                0 | 1 | 2 | 5 | 11 => {
                    let b = rec_ptr();
                    wr::<u8>(b, 0, 1);
                    make_vehicle_update_data(b, v, i);
                    0x34
                }
                10 => {
                    make_bmx_update_data(rec_ptr(), v, i);
                    0x38
                }
                9 => {
                    make_bike_update_data(rec_ptr(), v, i);
                    0x38
                }
                4 => {
                    let b = rec_ptr();
                    wr::<u8>(b, 0, 0x11);
                    make_vehicle_update_data(b, v, i);
                    wr::<u32>(b, 0x38, rd::<u32>(v, 0x9C8));
                    wr::<u32>(b, 0x34, rd::<u32>(v, 0x9C4));
                    0x3C
                }
                3 => {
                    let b = rec_ptr();
                    wr::<u8>(b, 0, 0x10);
                    make_vehicle_update_data(b, v, i);
                    wr::<u32>(b, 0x34, rd::<u32>(v, 0x84C));
                    0x38
                }
                6 => {
                    make_train_update_data(rec_ptr(), v, i);
                    0x4C
                }
                _ => 0,
            };
            if n != 0 {
                rec_advance(n);
            }
        }
        i += 1;
    }

    let mut i = 0;
    while i < rd::<i32>(peds, 8) {
        if let Some(p) = pool_live(peds, i, 0x7C4)
            && rd::<u32>(p, 0x18) != 0
        {
            if rd::<u32>(p, 0x470) & 0x2000 == 0 {
                let b = rec_ptr();
                wr::<u8>(b, 0, 3);
                wr::<u8>(b, 1, i as u8);
                let m = rd::<i16>(p, 0x22);
                wr::<i16>(b, 2, m);
                if (0x122..=0x12C).contains(&m) {
                    wr::<u16>(b, 2, 7);
                }
                wr::<u8>(b, 4, rd::<u8>(p, 0x598));
                rec_advance(8);
                if ped_is_player(p) != 0 {
                    let desc = rd::<GPtr<u8>>(rd::<GPtr<u8>>(p, 0x480), 4);
                    replay_store_clothes_desc(desc, rec_ptr());
                    rec_advance(0x78);
                }
                wr::<u32>(p, 0x470, rd::<u32>(p, 0x470) | 0x2000);
            }
            replay_store_ped_update(p, i);
        }
        i += 1;
    }

    for j in 0..16usize {
        let e = GPtr::<u8>::from_addr(0xC7C748 + 0x2C * j);
        if rd::<u8>(e, 0x18) == 0 {
            continue;
        }
        let b = rec_ptr();
        wr::<u8>(b, 0, 0xA);
        wr::<u8>(b, 3, j as u8);
        for k in 0..6isize {
            wr::<u32>(b, 4 + 4 * k, rd::<u32>(e, 4 * k));
        }
        rec_advance(0x1C);
    }

    let b = rec_ptr();
    wr::<u8>(b, 0, 0xC);
    wr::<u32>(b, 4, g::<u32>(0xB6F084).read());
    wr::<u32>(b, 8, g::<u32>(0xB6F154).read());
    let f = (rd::<u8>(b, 0xD) & !1) | (g::<u8>(0xC7C70C).read() & 1);
    wr::<u8>(b, 0xD, f);
    let f = (f & !2) | ((g::<u8>(0xC7C70D).read() << 1) & 2);
    wr::<u8>(b, 0xD, f);
    wr::<u8>(b, 0xC, g::<u8>(0xB72914).read());
    rec_advance(0x10);
    wr::<u8>(rec_ptr(), 0, 8);
    rec_advance(4);
    wr::<u8>(rec_ptr(), 0, 0);
}

/// `std::clamp(v, lo, hi)` as compiled into `StorePedAnimation`: `hi` when `hi < v`, else `lo` when `lo > v`
/// (ordered), else `v` (NaN keeps `v`).
fn clamp_x(v: f32, lo: f32, hi: f32) -> X87 {
    if x(hi).fcom(x(v)).test_parity(5) {
        if x(lo).fcom(x(v)).test_any(0x41) { x(v) } else { x(lo) }
    } else {
        x(hi)
    }
}

/// `(uint8)_ftol(clamp(v, lo, hi) * k)`.
fn pack(v: f32, lo: f32, hi: f32, k: f32) -> u8 {
    ftol32(clamp_x(v, lo, hi).fmul_m(k)) as u8
}

/// `CReplay::StorePedAnimation(CPed*, CStoredAnimationState&)` (0x45B170, cdecl): the main and second
/// associations of the ped's clump (`RpAnimBlendClumpGetMainAssociation`) and the main partial association
/// (only with an animation id >= 0) are packed: id (+0x2C), time `clamp(+0x20, 0, 4) * 63.75`, speed
/// `clamp(+0x24, 0, 3) * 85`, blend `clamp(_, 0, 2) * 127.5` (second: the returned amount, partial: `+0x18`) and
/// group (+0xE). A missing main association stores id 3, speed 0x55; missing others store zeros. Kept original
/// detail: the second association's blend amount is returned into the caller-owned argument slot of `ped`
/// (modelled with a local; the slot is not written).
#[hook(addr = 0x45B170, conv = cdecl, name = "CReplay::StorePedAnimation", expect = "51 53 55 8B 6C 24 10",
       unverified = "mvpd-world-1", route = "L2", hazards = "stack-residue",
       note = "RpAnimBlendClumpGetMainAssociation writes the blend amount into the ped argument slot")]
pub fn replay_store_ped_animation(ped: GPtr<u8>, st: GPtr<u8>) {
    let mut second = GPtr::<u8>::NULL;
    let mut blend = 0f32;
    let a = clump_get_main_association(rd::<GPtr<u8>>(ped, 0x18), GPtr::from_raw(&mut second), GPtr::from_raw(&mut blend));
    let zero = g::<f32>(0x858B50).read();
    let two = g::<f32>(0x858CA0).read();
    let three = g::<f32>(0x858B3C).read();
    let four = g::<f32>(0x858B90).read();
    let kt = g::<f32>(0x859C20).read();
    let ks = g::<f32>(0x85989C).read();
    let kb = g::<f32>(0x859C1C).read();
    if a.is_null() {
        wr::<u16>(st, 0, 3);
        wr::<u8>(st, 2, 0);
        wr::<u8>(st, 3, 0x55);
        wr::<u8>(st, 4, 0);
    } else {
        wr::<u16>(st, 0, rd::<u16>(a, 0x2C));
        wr::<u8>(st, 2, pack(rd::<f32>(a, 0x20), zero, four, kt));
        wr::<u8>(st, 3, pack(rd::<f32>(a, 0x24), zero, three, ks));
        wr::<u8>(st, 4, rd::<u8>(a, 0xE));
    }
    let a = second;
    if a.is_null() {
        wr::<u16>(st, 6, 0);
        wr::<u8>(st, 8, 0);
        wr::<u8>(st, 9, 0);
        wr::<u8>(st, 0xA, 0);
        wr::<u8>(st, 0xB, 0);
    } else {
        wr::<u16>(st, 6, rd::<u16>(a, 0x2C));
        wr::<u8>(st, 8, pack(rd::<f32>(a, 0x20), zero, four, kt));
        wr::<u8>(st, 9, pack(rd::<f32>(a, 0x24), zero, three, ks));
        wr::<u8>(st, 0xA, pack(blend, zero, two, kb));
        wr::<u8>(st, 0xB, rd::<u8>(a, 0xE));
    }
    let a = clump_get_main_partial_association(rd::<GPtr<u8>>(ped, 0x18));
    if a.is_null() || rd::<i16>(a, 0x2C) < 0 {
        wr::<u16>(st, 0xC, 0);
        wr::<u8>(st, 0xE, 0);
        wr::<u8>(st, 0xF, 0);
        wr::<u8>(st, 0x10, 0);
        wr::<u8>(st, 0x11, 0);
    } else {
        wr::<u16>(st, 0xC, rd::<u16>(a, 0x2C));
        wr::<u8>(st, 0xE, pack(rd::<f32>(a, 0x20), zero, four, kt));
        wr::<u8>(st, 0xF, pack(rd::<f32>(a, 0x24), zero, three, ks));
        wr::<u8>(st, 0x10, pack(rd::<f32>(a, 0x18), zero, two, kb));
        wr::<u8>(st, 0x11, rd::<u8>(a, 0xE));
    }
}

orig! {
    /// `CHudColours::GetRGB(eHudColours)` (0x58FEA0, `ret 8`: hidden result pointer, colour); returns the pointer
    fn hud_colours_get_rgb(h: GPtr<u8>, out: GPtr<u8>, c: u32) -> GPtr<u8> = thiscall 0x58FEA0 => unported;
    /// `CFont::SetEdge(int8)` (0x719590, cdecl)
    fn font_set_edge(v: u32) = cdecl 0x719590 => unported;
    /// `CFont::SetDropColor(CRGBA)` (0x719510, cdecl)
    fn font_set_drop_color(c: u32) = cdecl 0x719510 => unported;
    /// `CMessages::InsertNumberInString(const char*, int32 x6, char* out)` (0x69DE90, cdecl)
    fn insert_number_in_string(s: GPtr<u8>, n1: i32, n2: i32, n3: i32, n4: i32, n5: i32, n6: i32, out: GPtr<u8>) = cdecl 0x69DE90 => unported;
    /// `CPtrListSingleLink::AddItem(void*)` (0x5335E0, `ret 4`)
    fn single_add_item(l: GPtr<u8>, item: GPtr<u8>) -> GPtr<u8> = thiscall 0x5335E0 => unported;
    /// `CPtrListDoubleLink::AddItem(void*)` (0x533670, `ret 4`)
    fn double_add_item(l: GPtr<u8>, item: GPtr<u8>) -> GPtr<u8> = thiscall 0x533670 => unported;
    /// `CPtrListSingleLink::DeleteItem(void*)` (0x533610, `ret 4`)
    fn single_delete_item(l: GPtr<u8>, item: GPtr<u8>) = thiscall 0x533610 => unported;
    /// `CPtrNodeSingleLink::operator delete(void*)` (0x552390, cdecl; catalogue name `delete`)
    fn single_node_delete(n: GPtr<u8>) = cdecl 0x552390 => unported;
    /// `CPtrNodeDoubleLink::operator delete(void*)` (0x5523D0, cdecl; catalogue name `delete`)
    fn double_node_delete(n: GPtr<u8>) = cdecl 0x5523D0 => unported;
    /// entity virtual +0x24 (`GetBoundRect`, hidden result pointer); returns the pointer
    fn v_get_bound_rect(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
}

/// `CGarages::PrintMessages()` (0x447790, cdecl): while `CTimer::m_snTimeInMilliseconds` is in
/// `[MessageStartTime 0x96C020, MessageEndTime 0x96C01C)` the garage message (key 0x96C014) is printed centred
/// at `(width / 2, height * 0.002232143 * 155)` with scale `(width * 0.0015625 * 0.5, height * 0.002232143 *
/// 1.4)`, proportional, no background, centre size `width - width * 0.0015625 * 230`, orientation 0, style 2,
/// HUD colour 3, edge 2 and drop colour (0, 0, 0, 255); the numbers 0x96C010 / 0x96C00C are inserted (into
/// 0xC1B100) when non-negative (0x96C00C >= 0: both, else 0x96C010 alone). Outside the window the key's first
/// byte is cleared.
#[hook(addr = 0x447790, conv = cdecl, name = "CGarages::PrintMessages", expect = "51 A1 84 CB B7 00",
       route = "L2", hazards = "none")]
pub fn garages_print_messages() {
    let now = g::<u32>(0xB7CB84).read();
    if now < g::<u32>(0x96C020).read() || now >= g::<u32>(0x96C01C).read() {
        g::<u8>(0x96C014).write(0);
        return;
    }
    let sw = g::<f32>(0x859520).read();
    let sh = g::<f32>(0x859524).read();
    let sy = xi(g::<i32>(0xC17048).read()).fmul_m(sh).fmul_m(g::<f32>(0x859994).read()).to_f32();
    let sx = xi(g::<i32>(0xC17044).read()).fmul_m(sw).fmul_m(g::<f32>(0x858B8C).read()).to_f32();
    font_set_scale(sx, sy);
    font_set_proportional(1);
    font_set_background(0, 0);
    let w = xi(g::<i32>(0xC17044).read());
    font_set_centre_size(w.fsub(x(sw).fmul(w).fmul_m(g::<f32>(0x858F80).read())).to_f32());
    font_set_orientation(0);
    font_set_font_style(2);
    let mut tmp = 0u32;
    let c = hud_colours_get_rgb(GPtr::from_addr(0xBAB22C), GPtr::from_raw(&mut tmp).cast(), 3);
    font_set_color(rd::<u32>(c, 0));
    font_set_edge(2);
    let mut drop = 0u32;
    let c = rgba_ctor(GPtr::from_raw(&mut drop).cast(), 0, 0, 0, 0xFF);
    font_set_drop_color(rd::<u32>(c, 0));
    let key = GPtr::<u8>::from_addr(0x96C014);
    let out = GPtr::<u8>::from_addr(0xC1B100);
    let text = GPtr::<u8>::from_addr(0xC1B340);
    let s = if g::<i32>(0x96C00C).read() >= 0 {
        let t = text_get(text, key);
        insert_number_in_string(t, g::<i32>(0x96C010).read(), g::<i32>(0x96C00C).read(), -1, -1, -1, -1, out);
        out
    } else if g::<i32>(0x96C010).read() >= 0 {
        let t = text_get(text, key);
        insert_number_in_string(t, g::<i32>(0x96C010).read(), -1, -1, -1, -1, -1, out);
        out
    } else {
        text_get(text, key)
    };
    let py = xi(g::<i32>(0xC17048).read()).fmul_m(sh).fmul_m(g::<f32>(0x859990).read()).to_f32();
    let px = xi(g::<i32>(0xC17044).read() / 2).to_f32();
    font_print_string(px, py, s);
}

/// The world-rect clamps `CEntity::Add(const CRect&)` / `Remove` apply to the four rect floats: [0] and [3]
/// below -3000 become -3000, [2] not below 3000 becomes 2999, [1] above 3000 becomes 2999 (NaN kept).
fn clamp_world_rect(r: &mut [f32; 4]) {
    let lo = g::<f32>(0x859A90).read();
    let hi = g::<f32>(0x859A94).read();
    if !x(r[0]).fcom(x(lo)).test_parity(5) {
        r[0] = f32::from_bits(0xC53B_8000);
    }
    if !x(r[2]).fcom(x(hi)).test_any(1) {
        r[2] = f32::from_bits(0x453B_7000);
    }
    if !x(r[3]).fcom(x(lo)).test_parity(5) {
        r[3] = f32::from_bits(0xC53B_8000);
    }
    if !x(r[1]).fcom(x(hi)).test_any(0x41) {
        r[1] = f32::from_bits(0x453B_7000);
    }
}

/// LOD sector index: `_ftol(v * 0.005 + 15)` (no floor).
fn lod_index(v: f32) -> i32 {
    ftol32(x(v).fmul_m(g::<f32>(0x858B4C).read()).fadd_m(g::<f32>(0x858B48).read()))
}

/// Sector index: `_ftol(floor(v * 0.02 + 60))` (the sum rounded to a double for `floor`).
fn sector_index(v: f32) -> i32 {
    let s = x(v).fmul_m(g::<f32>(0x858B38).read()).fadd_m(g::<f32>(0x858B34).read());
    ftol32(X87::from_f64_exact(crate::physical::base::floor_f64(s.to_f64())))
}

/// The list a non-building entity of type `t` goes to in cell (xs, ys), or `stale` (the original's register
/// value) for types outside 1..=5: repeat-sector vehicles/peds/objects (0xB992B8, `& 15`), sector dummies
/// (0xB7D0BC, clamped).
fn double_list(t: u8, xs: i32, ys: i32, stale: GPtr<u8>) -> GPtr<u8> {
    let rep = 0xB992B8 + 12 * ((((ys & 0xF) << 4) + (xs & 0xF)) as usize);
    match t {
        2 => GPtr::from_addr(rep),
        3 => GPtr::from_addr(rep + 4),
        4 => GPtr::from_addr(rep + 8),
        5 => GPtr::from_addr(0xB7D0BC + 8 * (ys.clamp(0, 0x77) * 0x78 + xs.clamp(0, 0x77)) as usize),
        _ => stale,
    }
}

/// `CEntity::Add(const CRect&)` (0x5347D0, `ret 4`): the clamped rect (see `clamp_world_rect`) is registered in
/// the LOD lists (0xB99EB8, 30x30, `+0x1C` bit 8 big buildings, unclamped indices) or, for other entities, per
/// sector cell: buildings in the sector's building list (single link), vehicles/peds/objects in the repeat
/// sector lists, dummies in the sector dummy list (double link). Kept original detail: a type outside 1..=5
/// adds to the list register left by the previous cell, initially the x0 index used as a pointer. The original
/// stores x1 (LOD path) / x0 (sector path) into the caller-owned rect argument slot (not modelled).
#[hook(addr = 0x5347D0, conv = thiscall, name = "CEntity::Add-rect", this = "CEntity", expect = "83 EC 18 8B 44 24 1C",
       unverified = "mvpd-world-1", route = "L2", hazards = "stack-residue",
       note = "writes its own (caller-owned) rect argument slot with a sector index")]
pub fn entity_add_rect(this: GPtr<u8>, rect: GPtr<u8>) {
    let mut r = [rd::<f32>(rect, 0), rd::<f32>(rect, 4), rd::<f32>(rect, 8), rd::<f32>(rect, 0xC)];
    clamp_world_rect(&mut r);
    if rd::<u32>(this, 0x1C) & 0x100 != 0 {
        let x0 = lod_index(r[0]);
        let x1 = lod_index(r[2]);
        let y1 = lod_index(r[1]);
        let y0 = lod_index(r[3]);
        if y0 > y1 {
            return;
        }
        for ys in y0..=y1 {
            if x0 <= x1 {
                for xs in x0..=x1 {
                    let l = 0xB99EB8u32.wrapping_add(4u32.wrapping_mul((ys.wrapping_mul(30)).wrapping_add(xs) as u32));
                    single_add_item(GPtr::from_addr(l as usize), this);
                }
            }
        }
        return;
    }
    let x0 = sector_index(r[0]);
    let y0 = sector_index(r[3]);
    let x1 = sector_index(r[2]);
    let y1 = sector_index(r[1]);
    if y0 > y1 {
        return;
    }
    let mut list = GPtr::<u8>::from_addr(x0 as u32 as usize);
    for ys in y0..=y1 {
        for xs in x0..=x1 {
            let t = rd::<u8>(this, 0x36) & 7;
            if t == 1 {
                let s = 0xB7D0B8 + 8 * (ys.clamp(0, 0x77) * 0x78 + xs.clamp(0, 0x77)) as usize;
                single_add_item(GPtr::from_addr(s), this);
            } else {
                list = double_list(t, xs, ys, list);
                double_add_item(list, this);
            }
        }
    }
}

/// The inlined `CPtrListDoubleLink::DeleteItem`: unlinks the first node holding `item` and frees it.
fn double_delete_inline(list: GPtr<u8>, item: GPtr<u8>) {
    let head = rd::<GPtr<u8>>(list, 0);
    let mut n = head;
    while !n.is_null() {
        if rd::<GPtr<u8>>(n, 0) == item {
            if head == n {
                wr::<GPtr<u8>>(list, 0, rd::<GPtr<u8>>(n, 4));
            }
            let prev = rd::<GPtr<u8>>(n, 8);
            if !prev.is_null() {
                wr::<GPtr<u8>>(prev, 4, rd::<GPtr<u8>>(n, 4));
            }
            let next = rd::<GPtr<u8>>(n, 4);
            if !next.is_null() {
                wr::<GPtr<u8>>(next, 8, rd::<GPtr<u8>>(n, 8));
            }
            double_node_delete(n);
            return;
        }
        n = rd::<GPtr<u8>>(n, 4);
    }
}

/// `CEntity::Remove()` (0x534AE0): the inverse of `Add(rect)` for the entity's `GetBoundRect` (vslot 0x24),
/// clamped as there: big buildings leave the LOD lists (inlined single-link delete), other entities their
/// sector lists (buildings through `CPtrListSingleLink::DeleteItem`, the rest with an inlined double-link
/// delete; the same stale-list quirk for types outside 1..=5).
#[hook(addr = 0x534AE0, conv = thiscall, name = "CEntity::Remove", this = "CEntity", expect = "83 EC 30 53 55 56 8B F1",
       route = "L2", hazards = "heap-writes")]
pub fn entity_remove(this: GPtr<u8>) {
    let mut rb = [0f32; 4];
    v_get_bound_rect(this, GPtr::from_raw(&mut rb).cast());
    let mut r = rb;
    clamp_world_rect(&mut r);
    if rd::<u32>(this, 0x1C) & 0x100 != 0 {
        let x0 = lod_index(r[0]);
        let x1 = lod_index(r[2]);
        let y1 = lod_index(r[1]);
        let y0 = lod_index(r[3]);
        if y0 > y1 {
            return;
        }
        for ys in y0..=y1 {
            if x0 > x1 {
                continue;
            }
            for xs in x0..=x1 {
                let l = 0xB99EB8u32.wrapping_add(4u32.wrapping_mul((ys.wrapping_mul(30)).wrapping_add(xs) as u32));
                let list = GPtr::<u8>::from_addr(l as usize);
                let head = rd::<GPtr<u8>>(list, 0);
                let mut prev = GPtr::<u8>::NULL;
                let mut n = head;
                while !n.is_null() {
                    if rd::<GPtr<u8>>(n, 0) == this {
                        if head == n {
                            wr::<GPtr<u8>>(list, 0, rd::<GPtr<u8>>(n, 4));
                        } else if !prev.is_null() {
                            wr::<GPtr<u8>>(prev, 4, rd::<GPtr<u8>>(n, 4));
                        }
                        single_node_delete(n);
                        break;
                    }
                    prev = n;
                    n = rd::<GPtr<u8>>(n, 4);
                }
            }
        }
        return;
    }
    let x0 = sector_index(r[0]);
    let y0 = sector_index(r[3]);
    let x1 = sector_index(r[2]);
    let y1 = sector_index(r[1]);
    if y0 > y1 {
        return;
    }
    let mut list = GPtr::<u8>::from_addr(x0 as u32 as usize);
    for ys in y0..=y1 {
        for xs in x0..=x1 {
            let t = rd::<u8>(this, 0x36) & 7;
            if t == 1 {
                let s = 0xB7D0B8 + 8 * (ys.clamp(0, 0x77) * 0x78 + xs.clamp(0, 0x77)) as usize;
                single_delete_item(GPtr::from_addr(s), this);
            } else {
                list = double_list(t, xs, ys, list);
                double_delete_inline(list, this);
            }
        }
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_WORLD_PROCESS,
    HOOK_REPLAY_UPDATE,
    HOOK_REPLAY_DISPLAY,
    HOOK_REPLAY_RECORD_THIS_FRAME,
    HOOK_REPLAY_STORE_PED_ANIMATION,
    HOOK_GARAGES_PRINT_MESSAGES,
    HOOK_ENTITY_ADD_RECT,
    HOOK_ENTITY_REMOVE,
];
