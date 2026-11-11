//! Hand-ported `CTaskSimpleFight` / `CMeleeInfo` and `CTaskSimpleDuck` (PORT-AI-2, helper B). Ported from the 1.0 US bytes (the exe wins over gta-reversed and the specs).
//! Every call to another catalogued function goes through `orig!` at the original address; task objects
//! keep their original vtables and come from the original task pool (`common::task_new`).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CAnimBlendAssociation` finish/delete callback that does nothing (`CDefaultAnimCallback::DefaultAnimCB`).
const DEFAULT_ANIM_CB: u32 = 0x4D1390;
/// `CTaskSimpleDuck::DeleteDuckAnimCB` (0x692550), passed as the original address.
const DELETE_DUCK_ANIM_CB: u32 = 0x692550;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// The `0.0f` constant the comparisons read.
const F_ZERO: usize = 0x858B50;

orig! {
    // ---- virtual slots ----
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CEvent::GetEventType` (CEvent vtable +4)
    fn v_event_get_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEvent::GetSourceEntity` (CEvent vtable +0x28)
    fn v_event_get_source_entity(e: GPtr<u8>) -> u32 = thiscall vslot 0x28 => unported;

    // ---- task bases ----
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CTaskSimple::~CTaskSimple` (0x61A3A0)
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;

    // ---- this module's own functions (called through the original address) ----
    /// `CTaskSimpleDuck::Destructor` (0x692030)
    fn duck_dtor_orig(t: GPtr<u8>) = thiscall 0x692030 => unported;
    /// `CTaskSimpleDuck::SetMoveAnim(CPed*)` (0x6939F0, `ret 4`)
    fn duck_set_move_anim_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6939F0 => unported;
    /// `CTaskSimpleFight::~CTaskSimpleFight` (0x61C530)
    fn fight_dtor_orig(t: GPtr<u8>) = thiscall 0x61C530 => unported;

    // ---- other catalogued functions ----
    /// `CTaskSimpleDuck::AbortBecauseOfOtherDuck(CPed*)` (0x692340, `ret 4`)
    fn duck_abort_because_of_other_duck(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x692340 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn task_manager_get_simplest_active_task(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn task_manager_get_task_secondary(m: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CPedIntelligence::GetTaskUseGun()` (0x600F70)
    fn ped_intelligence_get_task_use_gun(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CTaskSimpleUseGun::ClearAnim(CPed*)` (0x61E190, `ret 4`)
    fn use_gun_clear_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61E190 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association_id(clump: u32, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0)
    fn anim_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CAnimBlendAssociation::SetDeleteCallback(cb, data)` (0x4CEBC0)
    fn anim_set_delete_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBC0 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::GetWeaponSkill()` (0x5E6580, current weapon)
    fn ped_get_weapon_skill(p: GPtr<u8>) -> u32 = thiscall 0x5E6580 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn weapon_info_get(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CEntity::CleanUpOldReference(CEntity** ref)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity** ref)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn anim_remove_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    fn anim_add_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::GetAnimBlockName(AssocGroupId)` (0x4D3A30)
    fn anim_get_anim_block_name(group: u32) -> GPtr<u8> = cdecl 0x4D3A30 => unported;
    /// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940)
    fn anim_get_animation_block(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D3940 => unported;
    /// `CStreaming::RequestModel(int32 id, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00, `ret 4`)
    fn ped_set_move_state(p: GPtr<u8>, state: u32) = thiscall 0x5DEC00 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80, `ret 4`; the float passed as its bits)
    fn anim_set_current_time(a: GPtr<u8>, t: u32) = thiscall 0x4CEA80 => unported;
    /// `CAEPedAudioEntity::AddAudioEvent(event, volume, speed, entity, a4, a5, a6)` (0x4E2BB0, `ret 0x1C`)
    fn ped_audio_add_audio_event(e: GPtr<u8>, ev: u32, vol: u32, speed: u32, a3: u32, a4: u32, a5: u32, a6: u32) = thiscall 0x4E2BB0 => unported;
    /// `CAEPedWeaponAudioEntity::AddAudioEvent(int32)` (0x4E69F0, `ret 4`)
    fn ped_weapon_audio_add_audio_event(e: GPtr<u8>, ev: u32) = thiscall 0x4E69F0 => unported;
    /// `CMatrix::operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, hidden result pointer first)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, hidden result pointer first)
    fn vector_mul_float(out: GPtr<u8>, v: GPtr<u8>, f: u32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00, `ret 4`)
    fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x411A00 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn player_get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::GetTarget()` (0x540670)
    fn pad_get_target(p: GPtr<u8>) -> u8 = thiscall 0x540670 => unported;
    /// `FUN_0061c7f0(CPed*, int8 command)` (0x61C7F0, `ret 8`; a `CTaskSimpleFight` method choosing the
    /// next combo move; reads only the low byte of `command`)
    fn fight_61c7f0(t: GPtr<u8>, ped: GPtr<u8>, command: u32) -> u8 = thiscall 0x61C7F0 => unported;
    /// `FUN_0061c9b0(CPed*)` (0x61C9B0, `ret 4`)
    fn fight_61c9b0(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61C9B0 => unported;
    /// `FUN_00624710(CPed*)` (0x624710, `ret 4`; returns an int16 in AX)
    fn fight_624710(t: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x624710 => unported;
    /// `FUN_00624a40(CPed*)` (0x624A40, `ret 4`; returns an int16 in AX)
    fn fight_624a40(t: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x624A40 => unported;
    /// `FUN_00623b10(CPed*, int32)` (0x623B10, `ret 8`)
    fn fight_623b10(t: GPtr<u8>, ped: GPtr<u8>, a: i32) = thiscall 0x623B10 => unported;
    /// `FUN_006240b0(CPed*, const CVector*)` (0x6240B0, `ret 8`; the melee hit test)
    fn fight_6240b0(t: GPtr<u8>, ped: GPtr<u8>, pos: GPtr<u8>) = thiscall 0x6240B0 => unported;

    // ---- LoadMeleeData ----
    /// `CFileMgr::OpenFile(const char* name, const char* mode)` (0x538900)
    fn file_mgr_open_file(name: u32, mode: u32) -> u32 = cdecl 0x538900 => unported;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    fn file_mgr_close_file(f: u32) -> i32 = cdecl 0x5389D0 => unported;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    fn file_loader_load_line(f: u32) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// CRT `_strncmp` (0x8214D0)
    fn crt_strncmp(a: GPtr<u8>, b: u32, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    /// CRT `_sscanf` (0x8220AD) with two outputs
    fn crt_sscanf2(s: GPtr<u8>, fmt: u32, a0: GPtr<u8>, a1: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `_sscanf` (0x8220AD) with three outputs
    fn crt_sscanf3(s: GPtr<u8>, fmt: u32, a0: GPtr<u8>, a1: GPtr<u8>, a2: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `_sscanf` (0x8220AD) with four outputs
    fn crt_sscanf4(s: GPtr<u8>, fmt: u32, a0: GPtr<u8>, a1: GPtr<u8>, a2: GPtr<u8>, a3: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `_sscanf` (0x8220AD) with nine outputs
    fn crt_sscanf9(s: GPtr<u8>, fmt: u32, a0: GPtr<u8>, a1: GPtr<u8>, a2: GPtr<u8>, a3: GPtr<u8>, a4: GPtr<u8>, a5: GPtr<u8>, a6: GPtr<u8>, a7: GPtr<u8>, a8: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// `CAnimManager::GetAnimGroupName(AssocGroupId)` (0x4D3A20)
    fn anim_get_anim_group_name(group: i32) -> GPtr<u8> = cdecl 0x4D3A20 => unported;
    /// `FUN_005bd360(const char*)` (0x5BD360): hit level of a melee data token
    fn melee_hit_level_of(s: GPtr<u8>) -> u8 = cdecl 0x5BD360 => unported;
    /// `FUN_005bd3b0(int32)` (0x5BD3B0): sound id of a melee data number (int16 in AX)
    fn melee_sound_of(a: i32) -> u32 = cdecl 0x5BD3B0 => unported;
}

/// `x87 fld [a]; fcomp [0.0]` helper: true when the `test ah, mask` branch is taken.
fn cmp_zero(v: f32, mask: u8) -> bool {
    x(v).fcom(x(g::<f32>(F_ZERO).read())).test_any(mask)
}

// ======================================================================================================
// CTaskSimpleDuck
// ======================================================================================================

/// `CTaskSimpleDuck::CTaskSimpleDuck(eDuckControlType, uint16 lengthOfDuck, int16 shotWhizzingCounter)`
/// (0x691FC0, `ret 0xC`; continues through the island 0x40A43C that writes the vtable). `CTaskSimple` base;
/// bytes +0x1A = +0x1B = 1, +0x18 = +0x19 = 0; vtable 0x870BC4; +0x1C = +0x20 = 0; +0x24 = control type;
/// +0x25 = 0xFF; +8 = `CTimer::m_snTimeInMilliseconds`; +0x10 = +0x14 = 0; word +0xC / +0xE = the two
/// 16-bit arguments.
#[hook(addr = 0x691FC0, conv = thiscall, name = "CTaskSimpleDuck::Constructor", expect = "56 8B F1 E8 C8 83 F8 FF")]
pub fn duck_ctor(this: GPtr<u8>, control: u32, length: u32, whizz: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u8>(this, 0x1A, 1);
    wr::<u8>(this, 0x1B, 1);
    wr::<u8>(this, 0x18, 0);
    wr::<u8>(this, 0x19, 0);
    wr::<u32>(this, 0, 0x870BC4);
    wr::<u32>(this, 0x1C, 0);
    wr::<u32>(this, 0x20, 0);
    wr::<u8>(this, 0x24, control as u8);
    wr::<u8>(this, 0x25, 0xFF);
    wr::<u32>(this, 8, g::<u32>(TIME_MS).read());
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x14, 0);
    wr::<u16>(this, 0xC, length as u16);
    wr::<u16>(this, 0xE, whizz as u16);
    this
}

/// `CTaskSimpleDuck::GetTaskType` (0x692020): `mov eax, 0x19F; ret` -> 415.
#[hook(addr = 0x692020, conv = thiscall, name = "CTaskSimpleDuck::GetTaskType", expect = "B8 9F 01 00 00")]
pub fn duck_get_task_type(_this: GPtr<u8>) -> u32 {
    0x19F
}

/// Shared tail of the two anim blocks of the duck destructor: when the association's blendAmount (+0x18)
/// is above 0 and its blendDelta (+0x1C) not below 0 (and, for the first one, flag bit 4 of +0x2E is
/// set) the blendDelta becomes -8.
fn duck_dtor_blend_out(a: GPtr<u8>, need_flag: bool) {
    if cmp_zero(rd::<f32>(a, 0x18), 0x41) || cmp_zero(rd::<f32>(a, 0x1C), 0x01) {
        return;
    }
    if need_flag && (rd::<u8>(a, 0x2E) >> 4) & 1 == 0 {
        return;
    }
    wr::<u32>(a, 0x1C, 0xC100_0000);
}

/// `CTaskSimpleDuck::Destructor` (0x692030). vtable 0x870BC4; for the down/up anim (+0x10) then the move
/// anim (+0x14): resets its finish callback to the default one and, while it is blending in (amount > 0,
/// delta >= 0; for +0x10 only with flag bit 4 of +0x2E), sets its blendDelta to -8; then `~CTaskSimple`.
#[hook(addr = 0x692030, conv = thiscall, name = "CTaskSimpleDuck::Destructor", expect = "6A FF 68 38 73 84 00")]
pub fn duck_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870BC4);
    if !rd::<GPtr<u8>>(this, 0x10).is_null() {
        anim_set_finish_callback(rd::<GPtr<u8>>(this, 0x10), DEFAULT_ANIM_CB, 0);
        duck_dtor_blend_out(rd::<GPtr<u8>>(this, 0x10), true);
    }
    if !rd::<GPtr<u8>>(this, 0x14).is_null() {
        anim_set_finish_callback(rd::<GPtr<u8>>(this, 0x14), DEFAULT_ANIM_CB, 0);
        duck_dtor_blend_out(rd::<GPtr<u8>>(this, 0x14), false);
    }
    task_simple_dtor(this);
}

/// `CTaskSimpleDuck` scalar deleting destructor (0x6939D0, catalogued as `CTaskSimple::~CTaskSimple`):
/// `CTaskSimpleDuck::Destructor` (0x692030), then `CTask::operator delete` when bit 0 of `flags` is set;
/// returns `this`.
#[hook(addr = 0x6939D0, conv = thiscall, name = "CTaskSimple::~CTaskSimple", expect = "56 8B F1 E8 58 E6 FF FF")]
pub fn duck_deleting_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    duck_dtor_orig(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// Common exit of `MakeAbortable` when the task finishes: clears the ped's "ducking" flag (bit 26 of
/// +0x46C), sets +0x1A and returns true.
fn duck_abort_done(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFBFF_FFFF);
    wr::<u8>(this, 0x1A, 1);
    1
}

/// `CTaskSimpleDuck::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x692100).
/// Immediate (2): the down/up anim (+0x10) gets blendDelta -1000 when flag bit 4 of +0x2E is set, else a
/// `BlendAnimation(clump, ped+0x4D4, 3, 1000)` is started (its result unused); its finish callback is reset
/// and +0x10 cleared; the move anim (+0x14) gets blendDelta -1000, finish callback reset, cleared; done.
/// Urgent (1) with a whizz counter (+0xE, int16) > -1 and an event of type 0x31 whose word +0xE is 0x137
/// and which has a source entity: done. Otherwise: false when the move anim's id (+0x2C) is 0x39/0x3B;
/// delta = -8 (urgent) / -4; a down anim that is blending in (amount > 0, delta >= 0) gets the delta when
/// flag bit 4 is set and `BlendAnimation(clump, ped+0x4D4, 3, -delta)` with ped+0x538 = 1; urgent clears
/// it (finish callback reset). A move anim that is blending in gets the delta and loses flag bit 0 of +0x2E
/// when urgent or its id is 0x38/0x3A; urgent clears it. The use-gun task's anim is cleared
/// (`GetTaskUseGun` twice). Urgent: +0x18 = 1, done; else +0x19 = 1, false.
#[hook(addr = 0x692100, conv = thiscall, name = "CTaskSimpleDuck::MakeAbortable", expect = "55 8B 6C 24 0C")]
pub fn duck_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if priority == 2 {
        let a = rd::<GPtr<u8>>(this, 0x10);
        if !a.is_null() {
            if (rd::<u8>(a, 0x2E) >> 4) & 1 != 0 {
                wr::<u32>(a, 0x1C, 0xC47A_0000);
            } else {
                anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, 0x447A_0000);
            }
            anim_set_finish_callback(rd::<GPtr<u8>>(this, 0x10), DEFAULT_ANIM_CB, 0);
            wr::<u32>(this, 0x10, 0);
        }
        let m = rd::<GPtr<u8>>(this, 0x14);
        if !m.is_null() {
            wr::<u32>(m, 0x1C, 0xC47A_0000);
            anim_set_finish_callback(rd::<GPtr<u8>>(this, 0x14), DEFAULT_ANIM_CB, 0);
            wr::<u32>(this, 0x14, 0);
        }
        return duck_abort_done(this, ped);
    }
    if priority == 1
        && rd::<i16>(this, 0xE) > -1
        && !event.is_null()
        && v_event_get_type(event) == 0x31
        && rd::<u16>(event, 0xE) == 0x137
        && v_event_get_source_entity(event) != 0
    {
        return duck_abort_done(this, ped);
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    if !m.is_null() {
        let id = rd::<u16>(m, 0x2C);
        if id == 0x39 || id == 0x3B {
            return 0;
        }
    }
    let delta: u32 = if priority == 1 { 0xC100_0000 } else { 0xC080_0000 };
    let a = rd::<GPtr<u8>>(this, 0x10);
    if !a.is_null() {
        if !cmp_zero(rd::<f32>(a, 0x18), 0x41) && !cmp_zero(rd::<f32>(a, 0x1C), 0x01) {
            if (rd::<u8>(a, 0x2E) >> 4) & 1 != 0 {
                wr::<u32>(a, 0x1C, delta);
            }
            // fld delta; fchs; fstp: an exact sign flip.
            let neg = f32_of(x(f32::from_bits(delta)).fchs()).to_bits();
            anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, neg);
            wr::<u32>(ped, 0x538, 1);
        }
        if priority == 1 {
            anim_set_finish_callback(rd::<GPtr<u8>>(this, 0x10), DEFAULT_ANIM_CB, 0);
            wr::<u32>(this, 0x10, 0);
        }
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    if !m.is_null() {
        if !cmp_zero(rd::<f32>(m, 0x18), 0x41) && !cmp_zero(rd::<f32>(m, 0x1C), 0x01) {
            let id = rd::<u16>(m, 0x2C);
            if priority == 1 || id == 0x38 || id == 0x3A {
                wr::<u32>(m, 0x1C, delta);
                let m2 = rd::<GPtr<u8>>(this, 0x14);
                wr::<u8>(m2, 0x2E, rd::<u8>(m2, 0x2E) & 0xFE);
            }
        }
        if priority == 1 {
            anim_set_finish_callback(rd::<GPtr<u8>>(this, 0x14), DEFAULT_ANIM_CB, 0);
            wr::<u32>(this, 0x14, 0);
        }
    }
    if !ped_intelligence_get_task_use_gun(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        let t = ped_intelligence_get_task_use_gun(rd::<GPtr<u8>>(ped, 0x47C));
        use_gun_clear_anim(t, ped);
    }
    if priority == 1 {
        wr::<u8>(this, 0x18, 1);
        return duck_abort_done(this, ped);
    }
    wr::<u8>(this, 0x19, 1);
    0
}

/// `CTaskSimpleDuck::ControlDuckMove(float moveX, float moveY)` (0x6923F0, `ret 8`; the floats are passed
/// as their bits). Sets +0x1B; nothing more while the move direction (+0x1C) is +-1. Otherwise moves the
/// move speed (+0x20) towards `moveY` by at most `timeStep * [0x858CA8]` (`diff > step`: += step;
/// `diff < -step`: -= step; else = moveY). When `|moveY| < [0x858B1C]` and `|moveX| > [0x858C20]`, +0x20 = 0
/// and +0x1C = 1 (moveX > 0) or -1. Returns true.
#[hook(addr = 0x6923F0, conv = thiscall, name = "CTaskSimpleDuck::ControlDuckMove", expect = "8B 41 1C 3D 00 00 80 3F")]
pub fn duck_control_duck_move(this: GPtr<u8>, move_x: u32, move_y: u32) -> u8 {
    let dir = rd::<u32>(this, 0x1C);
    wr::<u8>(this, 0x1B, 1);
    if dir == 0x3F80_0000 || dir == 0xBF80_0000 {
        return 1;
    }
    let mx = f32::from_bits(move_x);
    let my = f32::from_bits(move_y);
    let step = x(g::<f32>(TIME_STEP).read()).fmul_m(g::<f32>(0x858CA8).read());
    let diff = x(my).fsub_m(rd::<f32>(this, 0x20));
    if !diff.fcom(step).test_any(0x41) {
        wr::<f32>(this, 0x20, f32_of(step.fadd_m(rd::<f32>(this, 0x20))));
    } else if !diff.fcom(step.fchs()).test_parity(0x05) {
        wr::<f32>(this, 0x20, f32_of(x(rd::<f32>(this, 0x20)).fsub(step)));
    } else {
        wr::<u32>(this, 0x20, move_y);
    }
    // fld; fabs: the absolute value of a loaded f32 is exact.
    if x(my.abs()).fcom(x(g::<f32>(0x858B1C).read())).test_parity(0x05) {
        return 1;
    }
    if x(mx.abs()).fcom(x(g::<f32>(0x858C20).read())).test_any(0x41) {
        return 1;
    }
    wr::<u32>(this, 0x20, 0);
    if !cmp_zero(mx, 0x41) {
        wr::<u32>(this, 0x1C, 0x3F80_0000);
    } else {
        wr::<u32>(this, 0x1C, 0xBF80_0000);
    }
    1
}

/// `CTaskSimpleDuck::DeleteDuckAnimCB(CAnimBlendAssociation*, void* task)` (0x692550, cdecl). Nothing
/// without an association or a task. By the anim id (+0x2C): 0x37 / 0x8D (down): +0x10 = 0 and
/// +0x18 = 1 when the move anim (+0x14) is null or +0x19 is clear; 0x39 / 0x3B: +0x1C = 0, then as
/// 0x38 / 0x3A: +0x14 = 0 and +0x18 = 1 when +0x19 is set.
#[hook(addr = 0x692550, conv = cdecl, name = "CTaskSimpleDuck::DeleteDuckAnimCB", expect = "8B 44 24 08 33 D2")]
pub fn duck_delete_duck_anim_cb(assoc: GPtr<u8>, data: GPtr<u8>) {
    if data.is_null() || assoc.is_null() {
        return;
    }
    // Jump table 0x6925A0 indexed through the byte table 0x6925B0 (id - 0x37, 0..=0x56).
    match i32::from(rd::<i16>(assoc, 0x2C)).wrapping_sub(0x37) {
        0 | 0x56 => {
            let move_null = rd::<u32>(data, 0x14) == 0;
            wr::<u32>(data, 0x10, 0);
            if move_null || rd::<u8>(data, 0x19) == 0 {
                wr::<u8>(data, 0x18, 1);
            }
        }
        1 | 3 => {
            wr::<u32>(data, 0x14, 0);
            if rd::<u8>(data, 0x19) != 0 {
                wr::<u8>(data, 0x18, 1);
            }
        }
        2 | 4 => {
            wr::<u32>(data, 0x1C, 0);
            wr::<u32>(data, 0x14, 0);
            if rd::<u8>(data, 0x19) != 0 {
                wr::<u8>(data, 0x18, 1);
            }
        }
        _ => {}
    }
}

/// `CTaskSimpleDuck::CanPedDuck(CPed*)` (0x692610, static). False for a player whose state (+0x534) is
/// >= 6. Otherwise reads the current weapon's info (`GetWeaponInfo(type, GetWeaponSkill())`, the weapon
/// type read before the skill call): true for fire type (+0) 0 (melee) or 5, weapon type 0x29, or flag
/// bit 13 of +0x18; else false.
#[hook(addr = 0x692610, conv = cdecl, name = "CTaskSimpleDuck::CanPedDuck", expect = "56 8B 74 24 08")]
pub fn duck_can_ped_duck(ped: GPtr<u8>) -> u8 {
    if ped_is_player(ped) != 0 && rd::<i32>(ped, 0x534) >= 6 {
        return 0;
    }
    let slot = |p: GPtr<u8>| (i32::from(rd::<i8>(p, 0x718)) * 0x1C) as isize;
    let ty = rd::<u32>(ped, 0x5A0 + slot(ped));
    let skill = ped_get_weapon_skill(ped);
    let info = weapon_info_get(ty, skill);
    let fire = rd::<u32>(info, 0);
    if fire == 0
        || fire == 5
        || rd::<u32>(ped, 0x5A0 + slot(ped)) == 0x29
        || (rd::<u32>(info, 0x18) >> 13) & 1 != 0
    {
        return 1;
    }
    0
}

/// `CTaskSimpleDuck::SetMoveAnim(CPed*)` (0x6939F0, `ret 4`).
/// Turning (+0x1C != 0, NaN included): a move anim (+0x14) other than 0x38/0x3A keeps playing (return);
/// a 0x38/0x3A one gets the default finish callback; then blends anim 0x3B (+0x1C > 0) or 0x39 (group 0,
/// delta 8) into +0x14 with `DeleteDuckAnimCB` as its *finish* callback.
/// Not turning: speed (+0x20) > 0: unless a move anim other than 0x38 plays, (default finish callback on a
/// 0x38 one and) blend 0x3A into +0x14 with `DeleteDuckAnimCB` as delete callback; speed < 0: the same with
/// 0x3A / 0x38. Then the move anim's speed (+0x24) = min(|speed| as above, [0x870A90]).
/// Speed 0 (or NaN): a 0x38/0x3A move anim loses flag bit 0 of +0x2E and gets blendDelta -4.
#[hook(addr = 0x6939F0, conv = thiscall, name = "CTaskSimpleDuck::SetMoveAnim", expect = "56 8B F1 D9 46 1C")]
pub fn duck_set_move_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    // fcomp; test ah, 0x44; jnp: taken only for Equal.
    if x(rd::<f32>(this, 0x1C))
        .fcom(x(g::<f32>(F_ZERO).read()))
        .test_parity(0x44)
    {
        let m = rd::<GPtr<u8>>(this, 0x14);
        if !m.is_null() {
            let id = rd::<u16>(m, 0x2C);
            if id != 0x38 && id != 0x3A {
                return;
            }
            anim_set_finish_callback(m, DEFAULT_ANIM_CB, 0);
        }
        let anim = if cmp_zero(rd::<f32>(this, 0x1C), 0x41) {
            0x39
        } else {
            0x3B
        };
        let r = anim_blend_animation(rd::<u32>(ped, 0x18), 0, anim, 0x4100_0000);
        wr::<GPtr<u8>>(this, 0x14, r);
        anim_set_finish_callback(r, DELETE_DUCK_ANIM_CB, this.addr() as u32);
        return;
    }
    let speed = if !cmp_zero(rd::<f32>(this, 0x20), 0x41) {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let blend = if m.is_null() {
            true
        } else if rd::<u16>(m, 0x2C) != 0x38 {
            false
        } else {
            anim_set_finish_callback(m, DEFAULT_ANIM_CB, 0);
            true
        };
        if blend {
            let r = anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x3A, 0x4100_0000);
            wr::<GPtr<u8>>(this, 0x14, r);
            anim_set_delete_callback(r, DELETE_DUCK_ANIM_CB, this.addr() as u32);
        }
        x(rd::<f32>(this, 0x20))
    } else if !x(rd::<f32>(this, 0x20))
        .fcom(x(g::<f32>(F_ZERO).read()))
        .test_parity(0x05)
    {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let blend = if m.is_null() {
            true
        } else if rd::<u16>(m, 0x2C) != 0x3A {
            false
        } else {
            anim_set_finish_callback(m, DEFAULT_ANIM_CB, 0);
            true
        };
        if blend {
            let r = anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x38, 0x4100_0000);
            wr::<GPtr<u8>>(this, 0x14, r);
            anim_set_delete_callback(r, DELETE_DUCK_ANIM_CB, this.addr() as u32);
        }
        x(rd::<f32>(this, 0x20)).fchs()
    } else {
        let m = rd::<GPtr<u8>>(this, 0x14);
        if !m.is_null() {
            let id = rd::<u16>(m, 0x2C);
            if id == 0x38 || id == 0x3A {
                wr::<u8>(m, 0x2E, rd::<u8>(m, 0x2E) & 0xFE);
                wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x1C, 0xC080_0000);
            }
        }
        return;
    };
    // fld [0x870A90]; fcomp st(1); test ah, 0x41; jne keep: the limit replaces the speed when greater.
    let lim = x(g::<f32>(0x870A90).read());
    let v = if !lim.fcom(speed).test_any(0x41) {
        x(g::<f32>(0x870A90).read())
    } else {
        speed
    };
    wr::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x24, f32_of(v));
}

/// `CTaskSimpleDuck::ProcessPed(CPed*)` (0x694390). See the inline comments; returns true when the task
/// finished.
#[hook(addr = 0x694390, conv = thiscall, name = "CTaskSimpleDuck::ProcessPed", expect = "53 55 56 8B F1")]
pub fn duck_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    // Finished (+0x18) or health (+0x540) below [0x858624] (jnp after `test ah, 5`: Less only).
    if rd::<u8>(this, 0x18) != 0
        || !x(rd::<f32>(ped, 0x540))
            .fcom(x(g::<f32>(0x858624).read()))
            .test_parity(0x05)
    {
        if rd::<u8>(this, 0x19) == 0 {
            v_make_abortable(this, ped, 1, GPtr::NULL);
        }
        if rd::<u8>(this, 0x1A) == 0 {
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFBFF_FFFF);
        }
        return 1;
    }
    if rd::<u8>(this, 0x1A) != 0 {
        // First run: resolve a primary and a secondary duck task.
        let mut primary = task_manager_get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
        if !primary.is_null() && v_get_task_type(primary) != 0x19F {
            primary = GPtr::NULL;
        }
        let mut secondary = task_manager_get_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 1);
        if !secondary.is_null() && v_get_task_type(secondary) != 0x19F {
            secondary = GPtr::NULL;
        }
        if secondary == this && !primary.is_null() {
            duck_abort_because_of_other_duck(primary, ped);
        } else if primary == this && !secondary.is_null() {
            v_make_abortable(this, ped, 1, GPtr::NULL);
            return 0;
        }
        if !ped_intelligence_get_task_use_gun(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
            let t = ped_intelligence_get_task_use_gun(rd::<GPtr<u8>>(ped, 0x47C));
            use_gun_clear_anim(t, ped);
        }
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 0x0400_0000);
        wr::<u8>(this, 0x1A, 0);
    } else if rd::<u32>(ped, 0x46C) & 0x0400_0000 == 0 {
        wr::<u8>(this, 0x18, 1);
    }
    if rd::<u8>(this, 0x19) != 0 {
        return 0;
    }
    if rd::<u8>(this, 0x1B) == 0 {
        // No control this frame: count down +0x25; when it was 0, ask to abort (urgent).
        let c = rd::<u8>(this, 0x25);
        wr::<u8>(this, 0x25, c.wrapping_sub(1));
        if c == 0 && v_make_abortable(this, ped, 1, GPtr::NULL) != 0 {
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFBFF_FFFF);
            return 1;
        }
        return 0;
    }
    let len = rd::<u16>(this, 0xC);
    wr::<u8>(this, 0x25, 4);
    if len != 0 && g::<u32>(TIME_MS).read() > u32::from(len).wrapping_add(rd::<u32>(this, 8)) {
        v_make_abortable(this, ped, 0, GPtr::NULL);
    }
    if rd::<i16>(this, 0xE) > 0 {
        let v = x(g::<f32>(TIME_STEP).read())
            .fmul_m(g::<f32>(0x858B38).read())
            .fmul_m(g::<f32>(0x858C4C).read());
        let n = ftol32(v) as u16;
        wr::<u16>(this, 0xE, rd::<u16>(this, 0xC).wrapping_sub(n));
    }
    let a = rd::<GPtr<u8>>(this, 0x10);
    if a.is_null() {
        let w = clump_get_association_id(rd::<u32>(ped, 0x18), 0xA);
        if !w.is_null() {
            wr::<u32>(w, 0x1C, 0xC000_0000);
        }
        let (anim, other) = if rd::<u8>(this, 0x24) != 0 {
            (0x37, 0x8D)
        } else {
            (0x8D, 0x37)
        };
        let r = anim_blend_animation(rd::<u32>(ped, 0x18), 0, anim, 0x4080_0000);
        wr::<GPtr<u8>>(this, 0x10, r);
        anim_set_delete_callback(r, DELETE_DUCK_ANIM_CB, this.addr() as u32);
        let o = clump_get_association_id(rd::<u32>(ped, 0x18), other);
        if !o.is_null() && !cmp_zero(rd::<f32>(o, 0x18), 0x41) && !cmp_zero(rd::<f32>(o, 0x1C), 0x01) {
            if (rd::<u8>(o, 0x2E) >> 4) & 1 != 0 {
                wr::<u32>(o, 0x1C, 0xC080_0000);
            } else {
                anim_blend_animation(rd::<u32>(ped, 0x18), 0, 3, 0x4080_0000);
            }
        }
    } else if !x(rd::<f32>(a, 0x18))
        .fcom(x(g::<f32>(0x862CD0).read()))
        .test_any(0x41)
    {
        let ct = rd::<u8>(this, 0x24);
        if ct == 2 || ct == 4 {
            duck_set_move_anim_orig(this, ped);
        }
    }
    if rd::<u8>(this, 0x24) == 2 {
        wr::<u8>(this, 0x1B, 0);
    }
    0
}

// ======================================================================================================
// CTaskSimpleFight
// ======================================================================================================

/// `CTaskSimpleFight::CTaskSimpleFight(CEntity* target, int32 command, uint32 idleTime)` (0x61C470,
/// `ret 0xC`). `CTaskSimple` base; +0x24 = +0x25 = 0xFF; vtable 0x86D684; bytes +8 = 0, +9 = 1, +0xA = 0;
/// +0xC = 0x21 (no anim group); word +0x12 = 0; +0x14 = +0x15 = 0; +0x18 = target; +0x1C = +0x20 = 0;
/// +0x26 = command (low byte); +0x27 = 0; registers the target reference; word +0x10 = min(idleTime, 60000).
#[hook(addr = 0x61C470, conv = thiscall, name = "CTaskSimpleFight::CTaskSimpleFight", expect = "6A FF 68 18 F3 83 00")]
pub fn fight_ctor(this: GPtr<u8>, target: GPtr<u8>, command: u32, idle_time: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u8>(this, 0x24, 0xFF);
    wr::<u8>(this, 0x25, 0xFF);
    wr::<u32>(this, 0, 0x86D684);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 9, 1);
    wr::<u8>(this, 0xA, 0);
    wr::<u32>(this, 0xC, 0x21);
    wr::<u16>(this, 0x12, 0);
    wr::<u8>(this, 0x14, 0);
    wr::<u8>(this, 0x15, 0);
    wr::<GPtr<u8>>(this, 0x18, target);
    wr::<u32>(this, 0x1C, 0);
    wr::<u32>(this, 0x20, 0);
    wr::<u8>(this, 0x26, command as u8);
    wr::<u8>(this, 0x27, 0);
    if !target.is_null() {
        entity_register_reference(target, this.byte_add(0x18));
    }
    wr::<u16>(this, 0x10, idle_time.min(0xEA60) as u16);
    this
}

/// `CTaskSimpleFight::GetTaskType` (0x61C520): `mov eax, 0x3F8; ret` -> 1016.
#[hook(addr = 0x61C520, conv = thiscall, name = "CTaskSimpleFight::GetTaskType", expect = "B8 F8 03 00 00")]
pub fn fight_get_task_type(_this: GPtr<u8>) -> u32 {
    0x3F8
}

/// `CTaskSimpleFight::~CTaskSimpleFight` (0x61C530). vtable 0x86D684; cleans up the target reference
/// (+0x18); resets the delete callbacks of the anims at +0x1C and +0x20 to the default one; when an anim
/// block reference is held (+0xA) for a group (+0xC) other than 0x21, releases it
/// (`RemoveAnimBlockRef((group[+0xC].block - ms_aAnimBlocks) >> 5)`, groups at [0xB4EA34], 0x14 bytes each)
/// and clears +0xA; then `~CTaskSimple`.
#[hook(addr = 0x61C530, conv = thiscall, name = "CTaskSimpleFight::~CTaskSimpleFight", expect = "6A FF 68 38 F3 83 00")]
pub fn fight_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D684);
    let t = rd::<GPtr<u8>>(this, 0x18);
    if !t.is_null() {
        entity_clean_up_old_reference(t, this.byte_add(0x18));
    }
    if !rd::<GPtr<u8>>(this, 0x1C).is_null() {
        anim_set_delete_callback(rd::<GPtr<u8>>(this, 0x1C), DEFAULT_ANIM_CB, 0);
    }
    if !rd::<GPtr<u8>>(this, 0x20).is_null() {
        anim_set_delete_callback(rd::<GPtr<u8>>(this, 0x20), DEFAULT_ANIM_CB, 0);
    }
    if rd::<u8>(this, 0xA) != 0 {
        let grp = rd::<u32>(this, 0xC);
        if grp != 0x21 {
            let groups = g::<GPtr<u8>>(0xB4EA34).read();
            let block = rd::<u32>(groups, grp.wrapping_mul(0x14) as i32 as isize);
            anim_remove_block_ref((block.wrapping_sub(0xB5D4A0) as i32) >> 5);
            wr::<u8>(this, 0xA, 0);
        }
    }
    task_simple_dtor(this);
}

/// `CTaskSimpleFight::~CTaskSimpleFight` (0x6239D0): scalar deleting destructor: the destructor (0x61C530),
/// then `CTask::operator delete` when bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6239D0, conv = thiscall, name = "CTaskSimpleFight::~CTaskSimpleFight", expect = "56 8B F1 E8 58 8B FF FF")]
pub fn fight_deleting_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    fight_dtor_orig(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleFight::ControlFight(CEntity* target, int8 command)` (0x61C5E0, `ret 8`). Sets +9; a new
/// target replaces +0x18 (old reference cleaned up, new one registered when non-null); the command
/// (+0x26) is raised to `command` (signed byte compare). Returns true.
#[hook(addr = 0x61C5E0, conv = thiscall, name = "CTaskSimpleFight::ControlFight", expect = "53 8B 5C 24 08")]
pub fn fight_control_fight(this: GPtr<u8>, target: GPtr<u8>, command: u32) -> u8 {
    let old = rd::<GPtr<u8>>(this, 0x18);
    wr::<u8>(this, 9, 1);
    if target != old {
        if !old.is_null() {
            entity_clean_up_old_reference(old, this.byte_add(0x18));
        }
        wr::<GPtr<u8>>(this, 0x18, target);
        if !target.is_null() {
            entity_register_reference(target, this.byte_add(0x18));
        }
    }
    let c = command as u8 as i8;
    if c > rd::<i8>(this, 0x26) {
        wr::<i8>(this, 0x26, c);
    }
    1
}

/// `CMeleeInfo` table (`CTaskSimpleFight::m_aComboData`, 0x88 bytes per combo, combo ids start at 4).
const MELEE_INFO: u32 = 0xC170D0;
/// `CTaskSimpleFight::m_aHitOffset` (CVector per hit level, at 0xC177D0).
const MELEE_HIT_OFFSETS: u32 = 0xC177D0;
/// `CAnimManager::ms_aAnimAssocGroups` (pointer to the 0x14-byte groups; dword 0 is the anim block).
const ANIM_ASSOC_GROUPS: usize = 0xB4EA34;
/// `CAnimManager::ms_aAnimBlocks` (0x20 bytes per block)
const ANIM_BLOCKS: u32 = 0xB5D4A0;

/// The melee info of a (signed byte) combo id.
fn melee_info(combo: u8) -> GPtr<u8> {
    let i = i32::from(combo as i8).wrapping_sub(4).wrapping_mul(0x88);
    GPtr::from_addr(MELEE_INFO.wrapping_add(i as u32) as usize)
}

/// The ped's current weapon type (`m_aWeapons[m_nActiveWeaponSlot].m_eWeaponType`).
fn ped_weapon_type(ped: GPtr<u8>) -> u32 {
    rd::<u32>(ped, 0x5A0 + (i32::from(rd::<i8>(ped, 0x718)) * 0x1C) as isize)
}

/// `CTaskSimpleFight::ProcessPed(CPed*)` (0x629920). Returns true only from the "finished" branch (+8).
#[hook(addr = 0x629920, conv = thiscall, name = "CTaskSimpleFight::ProcessPed", expect = "83 EC 18 55 56")]
pub fn fight_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    // ---- finished: blend the idle anim (+0x20) out, back to the ped's move anims ----
    if rd::<u8>(this, 8) != 0 {
        if !rd::<GPtr<u8>>(this, 0x20).is_null() {
            anim_set_delete_callback(rd::<GPtr<u8>>(this, 0x20), DEFAULT_ANIM_CB, 0);
            let a = rd::<GPtr<u8>>(this, 0x20);
            if !cmp_zero(rd::<f32>(a, 0x18), 0x41) && !cmp_zero(rd::<f32>(a, 0x1C), 0x01) {
                anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, 0x4100_0000);
            }
            wr::<u32>(this, 0x20, 0);
        }
        return 1;
    }
    // ---- idle timer (+0x12, ms) ----
    if rd::<u8>(this, 0x24) != 0 && rd::<u8>(this, 9) != 0 {
        wr::<u16>(this, 0x12, 0);
    } else {
        let v = x(g::<f32>(TIME_STEP).read())
            .fmul_m(g::<f32>(0x858B38).read())
            .fmul_m(g::<f32>(0x858C4C).read());
        let n = ftol32(v) as u16;
        wr::<u16>(this, 0x12, rd::<u16>(this, 0x12).wrapping_add(n));
    }
    // Not controlled this frame: nothing to do without a command (signed < 1), or while the anim with id
    // 0x2F plays as the attack anim. These exits leave +9 untouched.
    if rd::<u8>(this, 9) == 0 {
        if rd::<i8>(this, 0x26) < 1 {
            return 0;
        }
        let a = rd::<GPtr<u8>>(this, 0x1C);
        if !a.is_null() && rd::<u16>(a, 0x2C) == 0x2F {
            return 0;
        }
    }
    // ---- no idle anim: finish on an end command, else start the fight idle of the combo set ----
    if rd::<u32>(this, 0x20) == 0 {
        let c = rd::<u8>(this, 0x27);
        if c == 1 || c == 0xF || c == 0x10 || c == 0x11 {
            wr::<u8>(this, 8, 1);
        } else if rd::<u32>(this, 0x1C) == 0 && (rd::<i32>(ped, 0x534) <= 4 || ped_is_player(ped) == 0) {
            fight_start_idle(this, ped);
        }
    }
    // 0x629B9B: keep the combo set's anim block referenced.
    if rd::<u32>(this, 0xC) != 0x21 && rd::<u8>(this, 0xA) == 0 {
        fight_61c7f0(this, ped, 0xFFFF_FFFF);
    }
    if rd::<u32>(this, 0x1C) == 0 {
        // ---- no attack anim: maybe start one ----
        if ped_is_player(ped) != 0
            && rd::<u16>(this, 0x12) > rd::<u16>(this, 0x10)
            && rd::<u8>(this, 0x26) == 0
            && rd::<u8>(this, 0x24) == 0
        {
            wr::<u8>(this, 0x26, 1);
        }
        let c = rd::<u8>(this, 0x26);
        if (c != 0 || rd::<u8>(this, 0x24) != 0) && rd::<u8>(this, 0x27) != 0x10 {
            // The original pushes EAX with AL = +0x26 (upper bytes left over from `CPed::IsPlayer`); the
            // callee reads only the low byte.
            let r = fight_61c7f0(this, ped, u32::from(c));
            wr::<u8>(this, 0x24, r);
            if ped_is_player(ped) != 0 {
                let c = rd::<i8>(this, 0x26);
                if (3..=6).contains(&c) {
                    fight_61c9b0(this, ped);
                } else {
                    let v = i32::from(fight_624710(this, ped) as u16 as i16);
                    fight_623b10(this, ped, v);
                }
            } else {
                let v = i32::from(fight_624a40(this, ped) as u16 as i16);
                fight_623b10(this, ped, v);
            }
        }
    } else {
        fight_attack(this, ped);
    }
    // ---- 0x629FF7: face the target, else the camera front while a player aims ----
    let t = rd::<GPtr<u8>>(this, 0x18);
    if !t.is_null() {
        let pm = rd::<GPtr<u8>>(ped, 0x14);
        let pp = if pm.is_null() {
            ped.byte_add(4)
        } else {
            pm.byte_add(0x30)
        };
        let tm = rd::<GPtr<u8>>(t, 0x14);
        let tp = if tm.is_null() {
            t.byte_add(4)
        } else {
            tm.byte_add(0x30)
        };
        let dx = x(rd::<f32>(tp, 0)).fsub_m(rd::<f32>(pp, 0));
        let dy = x(rd::<f32>(tp, 4)).fsub_m(rd::<f32>(pp, 4));
        // fxch; fchs; fxch; fpatan: atan2(-dx, dy).
        wr::<f32>(ped, 0x55C, f32_of(dx.fchs().fpatan(dy)));
    } else if ped_is_player(ped) != 0
        && g::<u8>(0xB6EC2E).read() != 0
        && pad_get_target(player_get_pad_from_player(ped)) != 0
    {
        let cx = g::<f32>(0xB6F32C).read();
        let cy = g::<f32>(0xB6F330).read();
        wr::<f32>(ped, 0x55C, f32_of(x(cx).fchs().fpatan(x(cy))));
    }
    wr::<u8>(this, 9, 0);
    0
}

/// 0x629A3C..0x629B9B of `CTaskSimpleFight::ProcessPed`: picks the combo set of the current weapon
/// (`GetWeaponInfo(type, 1)` +0x6E; 4 = the ped's own fighting style +0x72D), references or requests its
/// anim block (falling back to combo 4 while it streams, and clamping a pending command 0xB..0xE to 0xB),
/// then blends the idle anim 0xDF of that set (delta 4, delete callback 0x61DAE0), sets move state 1 and
/// +0x538 = 1; +0x24 (written with the combo first) and +0x27 end as 0.
fn fight_start_idle(this: GPtr<u8>, ped: GPtr<u8>) {
    let info = weapon_info_get(ped_weapon_type(ped), 1);
    let mut set = rd::<u8>(info, 0x6E);
    if set == 4 {
        set = rd::<u8>(ped, 0x72D);
    }
    let m = melee_info(set);
    let combo = if rd::<u8>(m, 0x85) & 4 == 0 {
        4
    } else if rd::<u32>(m, 0) == 0x21 {
        set
    } else {
        let load = if rd::<u32>(m, 0) != rd::<u32>(this, 0xC) {
            if rd::<u8>(this, 0xA) != 0 {
                let grp = rd::<u32>(this, 0xC);
                let block = rd::<u32>(
                    g::<GPtr<u8>>(ANIM_ASSOC_GROUPS).read(),
                    grp.wrapping_mul(0x14) as i32 as isize,
                );
                anim_remove_block_ref((block.wrapping_sub(ANIM_BLOCKS) as i32) >> 5);
                wr::<u8>(this, 0xA, 0);
            }
            wr::<u32>(this, 0xC, rd::<u32>(m, 0));
            true
        } else {
            rd::<u8>(this, 0xA) == 0
        };
        if !load {
            set
        } else {
            let grp = rd::<u32>(this, 0xC);
            let mut block = rd::<GPtr<u8>>(
                g::<GPtr<u8>>(ANIM_ASSOC_GROUPS).read(),
                grp.wrapping_mul(0x14) as i32 as isize,
            );
            if block.is_null() {
                block = anim_get_animation_block(anim_get_anim_block_name(grp));
            }
            let idx = ((block.addr() as u32).wrapping_sub(ANIM_BLOCKS) as i32) >> 5;
            if rd::<u8>(block, 0x10) != 0 {
                anim_add_block_ref(idx);
                wr::<u8>(this, 0xA, 1);
                set
            } else {
                streaming_request_model(idx.wrapping_add(0x63E7), 8);
                let c = rd::<i8>(this, 0x26);
                if (0xB..=0xE).contains(&c) {
                    wr::<u8>(this, 0x26, 0xB);
                }
                4
            }
        }
    };
    let m = melee_info(combo);
    wr::<u8>(this, 0x24, combo);
    let r = anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(m, 0), 0xDF, 0x4080_0000);
    wr::<GPtr<u8>>(this, 0x20, r);
    anim_set_delete_callback(r, 0x61DAE0, this.addr() as u32);
    ped_set_move_state(ped, 1);
    wr::<u32>(ped, 0x538, 1);
    wr::<u8>(this, 0x24, 0);
    wr::<u8>(this, 0x27, 0);
}

/// 0x629C4E..0x629FF7 of `CTaskSimpleFight::ProcessPed`: an attack anim (+0x1C) is playing.
fn fight_attack(this: GPtr<u8>, ped: GPtr<u8>) {
    let m = melee_info(rd::<u8>(this, 0x24));
    if ped_weapon_type(ped) == 9 {
        ped_weapon_audio_add_audio_event(ped.byte_add(0x394), 0x9A);
    }
    if rd::<i8>(this, 0x24) < 4 {
        // Generic (non-combo) moves.
        let st = rd::<u8>(this, 0x27);
        if st == 8 || st == 0xA {
            let a = rd::<GPtr<u8>>(this, 0x1C);
            let ratio = x(rd::<f32>(a, 0x20)).fdiv_m(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10));
            if !ratio.fcom(x(g::<f32>(0x8D2E94).read())).test_any(0x41)
                && !x(rd::<f32>(a, 0x1C))
                    .fcom(x(g::<f32>(0x859C3C).read()))
                    .test_any(0x41)
            {
                wr::<u32>(a, 0x1C, 0xC080_0000);
                return;
            }
        }
        if st == 3 {
            if rd::<u8>(this, 0x26) == 3 {
                wr::<u8>(this, 0x26, 0);
            } else {
                let a = rd::<GPtr<u8>>(this, 0x1C);
                if !x(rd::<f32>(a, 0x1C))
                    .fcom(x(g::<f32>(0x859C3C).read()))
                    .test_any(0x41)
                {
                    wr::<u32>(a, 0x1C, 0xC080_0000);
                }
            }
        }
        return;
    }
    if rd::<u8>(this, 0x27) == 2 {
        if rd::<u8>(this, 0x26) == 2 {
            let a = rd::<GPtr<u8>>(this, 0x1C);
            if rd::<u8>(a, 0x2E) & 1 != 0 {
                // Loops the anim back to +0x48 when it crosses +0x48 or +0x4C this frame
                // (time < lim && time + delta >= lim).
                let cross = |lim: f32| {
                    !x(rd::<f32>(a, 0x20)).fcom(x(lim)).test_parity(0x05)
                        && !x(rd::<f32>(a, 0x28))
                            .fadd_m(rd::<f32>(a, 0x20))
                            .fcom(x(lim))
                            .test_any(0x01)
                };
                if cross(rd::<f32>(m, 0x48)) || cross(rd::<f32>(m, 0x4C)) {
                    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFE);
                    anim_set_current_time(rd::<GPtr<u8>>(this, 0x1C), rd::<u32>(m, 0x48));
                }
            }
        } else {
            let a = rd::<GPtr<u8>>(this, 0x1C);
            if rd::<u8>(a, 0x2E) & 1 == 0
                && !cmp_zero(rd::<f32>(a, 0x18), 0x41)
                && !cmp_zero(rd::<f32>(a, 0x1C), 0x01)
            {
                wr::<u32>(a, 0x1C, 0xC080_0000);
            }
            if rd::<i8>(this, 0x26) >= 0xB {
                anim_set_delete_callback(rd::<GPtr<u8>>(this, 0x1C), DEFAULT_ANIM_CB, 0);
                wr::<u32>(this, 0x1C, 0);
            }
        }
        if rd::<u8>(this, 0x26) == 2 {
            wr::<u8>(this, 0x26, 0);
        }
        return;
    }
    let a = rd::<GPtr<u8>>(this, 0x1C);
    if x(rd::<f32>(a, 0x18))
        .fcom(x(g::<f32>(0x858C20).read()))
        .test_any(0x41)
        || cmp_zero(rd::<f32>(a, 0x1C), 0x01)
    {
        return;
    }
    let i = isize::from(rd::<i8>(this, 0x25));
    let hit_time = rd::<f32>(m, 8 + i * 4);
    // The strike frame: current time (+0x20) past the strike time, previous time (+0x20 - +0x28) before it.
    if !x(rd::<f32>(a, 0x20)).fcom(x(hit_time)).test_any(0x41)
        && !x(rd::<f32>(a, 0x28))
            .fsubr_m(rd::<f32>(a, 0x20))
            .fcom(x(hit_time))
            .test_parity(0x05)
    {
        let audio = ped.byte_add(0x138);
        ped_audio_add_audio_event(audio, 0x3C, 0, 0x3F80_0000, 0, 0, rd::<u32>(m, 0x5C + i * 4), 0);
        if rd::<u8>(this, 0x24) == 5 {
            let j = rd::<i8>(this, 0x25);
            if (0..=2).contains(&j) {
                let j = isize::from(j);
                let p = g::<u32>((0x8D2E3C + j * 4) as usize).read();
                ped_audio_add_audio_event(audio, 0x3C, 0, 0x3F80_0000, 0, 0, rd::<u32>(m, 0x5C + j * 4), p);
            }
        }
        let c = rd::<i8>(this, 0x26);
        if (0xB..=0xE).contains(&c) {
            wr::<u8>(this, 0x26, 0);
        }
        let lvl = rd::<u8>(m, 0x50 + isize::from(rd::<i8>(this, 0x25)));
        if lvl == 7 {
            return;
        }
        // Stack CVectors of the original: the hit position (esp+0x10) and the speed term (esp+0x1C).
        let mut pos = [0u32; 3];
        let pp: GPtr<u8> = GPtr::from_raw(&mut pos as *mut [u32; 3]).cast();
        let off = GPtr::from_addr(MELEE_HIT_OFFSETS.wrapping_add(u32::from(lvl) * 0xC) as usize);
        matrix_mul_vec(pp, rd::<GPtr<u8>>(ped, 0x14), off);
        if rd::<u8>(this, 0x25) == 4 {
            let mut tmp = [0u32; 3];
            let tp: GPtr<u8> = GPtr::from_raw(&mut tmp as *mut [u32; 3]).cast();
            let r = vector_mul_float(tp, ped.byte_add(0x44), g::<u32>(TIME_STEP).read());
            vector_add_assign(pp, r);
        }
        fight_6240b0(this, ped, pp);
        return;
    }
    // 0x629EF7: past the chain time (+0x1C[i]) with a pending combo command 0xB..0xE: chain the next move.
    if x(rd::<f32>(a, 0x20))
        .fcom(x(rd::<f32>(m, 0x1C + i * 4)))
        .test_any(0x01)
    {
        return;
    }
    let c = rd::<i8>(this, 0x26);
    if !(0xB..=0xE).contains(&c) {
        return;
    }
    let k = rd::<u8>(this, 0x25);
    if k == 0 || k == 1 {
        // The original pushes EAX with AL = +0x26 (upper bytes left over); the callee reads only the low byte.
        let r = fight_61c7f0(this, ped, u32::from(c as u8));
        fight_chain(this, ped, r);
    } else if k == 3 {
        if ped_is_player(ped) != 0 && fight_624710(this, ped) as u16 == 3 {
            fight_623b10(this, ped, 3);
        }
    } else if k == 4 {
        if rd::<u8>(this, 0x24) == 0xC && ped_is_player(ped) != 0 {
            let j = isize::from(rd::<i8>(this, 0x25));
            let t = x(rd::<f32>(m, 8 + j * 4)).fsub_m(g::<f32>(0x858C58).read());
            anim_set_current_time(rd::<GPtr<u8>>(this, 0x1C), f32_of(t).to_bits());
        } else {
            let r = fight_61c7f0(this, ped, u32::from(rd::<u8>(this, 0x26)));
            fight_chain(this, ped, r);
        }
    }
}

/// Common tail after `FUN_0061c7f0` chose the next move: +0x24 = its result; a player continues with
/// `FUN_00623b10(ped, (int16) FUN_00624710(ped))`, other peds with `FUN_00623b10(ped, +0x25 + 1)`.
fn fight_chain(this: GPtr<u8>, ped: GPtr<u8>, r: u8) {
    wr::<u8>(this, 0x24, r);
    if ped_is_player(ped) != 0 {
        let v = i32::from(fight_624710(this, ped) as u16 as i16);
        fight_623b10(this, ped, v);
    } else {
        fight_623b10(this, ped, i32::from(rd::<i8>(this, 0x25)).wrapping_add(1));
    }
}

/// `CTaskSimpleFight::LoadMeleeData()` (0x5BEDC0, static; exits through the island 0x40204B).
///
/// Defaults: 13 `CMeleeInfo` entries from 0xC170D0 (0x88 bytes): +0 = 0x21, +4 = 1.5, and for i in 0..5
/// +8[i] = +0x1C[i] = 100, +0x30[i] = 1, byte +0x50[i] = 7, byte +0x55[i] = 0, +0x5C[i] = +0x70[i] = 0;
/// +0x44 = 0, +0x48 = +0x4C = 100, word +0x84 = 0. Hit offsets 0xC177D0: 7 CVectors (0, 0.75, 0).
/// Then parses `DATA\melee.dat` line by line (`#`/empty lines skipped, `END_MELEE_DATA` stops) with blocks
/// `START_COMBO` / `START_LEVELS` closed by `END_COMBO` (which also advances the combo after a combo
/// block). Level lines: `%s %f %f %f` -> hit offset[n] (no bound). Combo line n: 1 `%s %s` anim group by
/// name -> +0; 2 `%s %f` -> +4; 3..7 `%s %f %f %f %s %d %d %d %f` -> attack i = n - 3: +8[i] = f1/30,
/// +0x1C[i] = f2/30 (x87 `fmul [0x858F10]`), +0x30[i] = f3 (raw), byte +0x50[i] = `FUN_005bd360(s)`,
/// byte +0x55[i] = d1, +0x5C[i] = (int16) `FUN_005bd3b0(d2)`, +0x70[i] = (int16) `FUN_005bd3b0(d3)`,
/// +0x44 = f4/30 when f4 > 0; 8 `%s %f %f` -> +0x48 / +0x4C (/30); 9 `%s %x` -> word +0x84.
///
/// Stack object: the original's 0x90-byte frame (locals at esp+0x10..0x90) is a Rust buffer with the same
/// offsets, so `sscanf` targets, stale values kept between lines and `%s` overflows land as in the
/// original. Slots the original leaves uninitialised start as zero here (they are read only after a
/// `sscanf` that failed to fill them, i.e. for malformed lines); the hit-offset scratch slots
/// esp+0x44/0x48/0x4C start as 0 / 0.75 / 0 exactly as the original's dead CVector temporaries leave them.
#[hook(addr = 0x5BEDC0, conv = cdecl, name = "CTaskSimpleFight::LoadMeleeData", expect = "81 EC 80 00 00 00")]
pub fn fight_load_melee_data() {
    // ---- defaults ----
    let mut m = MELEE_INFO + 4;
    while m < 0xC177BC {
        let p = |o: u32| GPtr::<u8>::from_addr(m.wrapping_add(o) as usize);
        wr::<u32>(p(0), (-4i32) as isize, 0x21);
        wr::<u32>(p(0), 0, 0x3FC0_0000);
        for i in 0..5u32 {
            let e = p(0x18 + i * 4);
            wr::<u32>(e, -0x14, 0x42C8_0000);
            wr::<u32>(e, 0, 0x42C8_0000);
            wr::<u32>(e, 0x14, 0x3F80_0000);
            wr::<u8>(p(i), 0x4C, 7);
            wr::<u8>(p(i), 0x51, 0);
            wr::<u32>(e, 0x40, 0);
            wr::<u32>(e, 0x54, 0);
        }
        wr::<u32>(p(0), 0x40, 0);
        wr::<u32>(p(0), 0x44, 0x42C8_0000);
        wr::<u32>(p(0), 0x48, 0x42C8_0000);
        wr::<u16>(p(0), 0x80, 0);
        m += 0x88;
    }
    for a in [0xC177D4, 0xC177E0, 0xC177EC, 0xC177F8, 0xC17804, 0xC17810] {
        g::<u32>(a).write(0x3F40_0000);
    }
    for a in [
        0xC177D0, 0xC177D8, 0xC177DC, 0xC177E4, 0xC177E8, 0xC177F0, 0xC177F4, 0xC177FC, 0xC17800, 0xC17808,
        0xC1780C, 0xC17814, 0xC17818,
    ] {
        g::<u32>(a).write(0);
    }
    g::<u32>(0xC1781C).write(0x3F40_0000);
    g::<u32>(0xC17820).write(0);

    // ---- the original's stack frame (offsets relative to esp after the prologue) ----
    let mut frame = [0u32; 0x24];
    let fr: GPtr<u8> = GPtr::from_raw(&mut frame as *mut [u32; 0x24]).cast();
    let slot = |o: isize| fr.byte_add(o);
    wr::<u32>(fr, 0x44, 0);
    wr::<u32>(fr, 0x48, 0x3F40_0000);
    wr::<u32>(fr, 0x4C, 0);

    let file = file_mgr_open_file(0x86AAB4, 0x859C6C);
    let mut level_or_line: u32 = 0; // EBX
    let mut ofs: u32 = 0; // EBP: byte offset of the current combo
    wr::<u32>(fr, 0x40, file);
    wr::<u32>(fr, 0x14, 0);
    wr::<u8>(fr, 0x12, 0);
    wr::<u8>(fr, 0x13, 0);
    let mut line = file_loader_load_line(file);
    if !line.is_null() {
        wr::<u32>(fr, 0x18, 0);
        while !line.is_null() {
            let c0 = rd::<u8>(line, 0);
            if c0 != b'#' && c0 != 0 {
                if crt_strncmp(line, 0x86AAA4, 0xE) == 0 {
                    break;
                }
                load_melee_line(line, fr, &mut level_or_line, &mut ofs, slot);
            }
            line = file_loader_load_line(rd::<u32>(fr, 0x40));
        }
    }
    file_mgr_close_file(rd::<u32>(fr, 0x40));
}

/// One non-comment line of `LoadMeleeData` (0x5BEFAA..0x5BF2C3).
fn load_melee_line(
    line: GPtr<u8>,
    fr: GPtr<u8>,
    n: &mut u32,
    ofs: &mut u32,
    slot: impl Fn(isize) -> GPtr<u8>,
) {
    if rd::<u8>(fr, 0x12) == 0 && rd::<u8>(fr, 0x13) == 0 {
        if crt_strncmp(line, 0x86AA98, 0xB) == 0 {
            wr::<u8>(fr, 0x12, 1);
        } else if crt_strncmp(line, 0x86AA88, 0xC) == 0 {
            wr::<u8>(fr, 0x13, 1);
        }
        return;
    }
    if crt_strncmp(line, 0x86AA7C, 9) == 0 {
        if rd::<u8>(fr, 0x12) != 0 {
            wr::<u32>(fr, 0x18, rd::<u32>(fr, 0x18).wrapping_add(0x22));
            *ofs = ofs.wrapping_add(0x88);
        }
        *n = 0;
        wr::<u8>(fr, 0x12, 0);
        wr::<u8>(fr, 0x13, 0);
        return;
    }
    let base = *ofs;
    let melee = |o: u32| GPtr::<u8>::from_addr(MELEE_INFO.wrapping_add(base).wrapping_add(o) as usize);
    if rd::<u8>(fr, 0x13) != 0 {
        crt_sscanf4(line, 0x86AA70, slot(0x50), slot(0x44), slot(0x48), slot(0x4C));
        let v = GPtr::<u8>::from_addr(n.wrapping_mul(0xC).wrapping_add(MELEE_HIT_OFFSETS) as usize);
        wr::<u32>(v, 0, rd::<u32>(fr, 0x44));
        wr::<u32>(v, 4, rd::<u32>(fr, 0x48));
        wr::<u32>(v, 8, rd::<u32>(fr, 0x4C));
        *n = n.wrapping_add(1);
        return;
    }
    *n = n.wrapping_add(1);
    let k = || g::<f32>(0x858F10).read();
    match n.wrapping_sub(1) {
        0 => {
            crt_sscanf2(line, 0x869860, slot(0x50), slot(0x70));
            let mut grp: i32 = 0;
            while grp < g::<i32>(0xB4EA28).read() {
                let name = anim_get_anim_group_name(grp);
                // Inlined strcmp(esp+0x70, name) == 0.
                let mut i = 0isize;
                let equal = loop {
                    let a = rd::<u8>(fr, 0x70 + i);
                    if a != rd::<u8>(name, i) {
                        break false;
                    }
                    if a == 0 {
                        break true;
                    }
                    i += 1;
                };
                if equal {
                    wr::<i32>(melee(0), 0, grp);
                    break;
                }
                grp += 1;
            }
        }
        1 => {
            crt_sscanf2(line, 0x86AA68, slot(0x50), slot(0x28));
            wr::<u32>(melee(4), 0, rd::<u32>(fr, 0x28));
        }
        2..=6 => {
            wr::<u32>(fr, 0x1C, 0);
            crt_sscanf9(
                line,
                0x86AA4C,
                slot(0x50),
                slot(0x20),
                slot(0x24),
                slot(0x2C),
                slot(0x70),
                slot(0x30),
                slot(0x34),
                slot(0x38),
                slot(0x1C),
            );
            let idx = n.wrapping_sub(3);
            wr::<u32>(fr, 0x14, idx);
            // ECX = (esp+0x18 + idx) * 4: the dword offset of the attack inside the table.
            let e = |o: u32| {
                GPtr::<u8>::from_addr(
                    rd::<u32>(fr, 0x18)
                        .wrapping_add(idx)
                        .wrapping_mul(4)
                        .wrapping_add(MELEE_INFO + o) as usize,
                )
            };
            wr::<f32>(e(8), 0, f32_of(x(rd::<f32>(fr, 0x20)).fmul_m(k())));
            wr::<u32>(e(0x30), 0, rd::<u32>(fr, 0x2C));
            wr::<f32>(e(0x1C), 0, f32_of(x(rd::<f32>(fr, 0x24)).fmul_m(k())));
            let lvl = melee_hit_level_of(slot(0x70));
            wr::<u8>(melee(0x50 + idx), 0, lvl);
            wr::<u8>(melee(0x55 + idx), 0, rd::<u8>(fr, 0x30));
            let s1 = melee_sound_of(rd::<i32>(fr, 0x34));
            wr::<i32>(e(0x5C), 0, i32::from(s1 as u16 as i16));
            let s2 = melee_sound_of(rd::<i32>(fr, 0x38));
            wr::<i32>(e(0x70), 0, i32::from(s2 as u16 as i16));
            if !cmp_zero(rd::<f32>(fr, 0x1C), 0x41) {
                wr::<f32>(melee(0x44), 0, f32_of(x(rd::<f32>(fr, 0x1C)).fmul_m(k())));
            }
        }
        7 => {
            crt_sscanf3(line, 0x86AA40, slot(0x50), slot(0x20), slot(0x24));
            wr::<f32>(melee(0x48), 0, f32_of(x(rd::<f32>(fr, 0x20)).fmul_m(k())));
            wr::<f32>(melee(0x4C), 0, f32_of(x(rd::<f32>(fr, 0x24)).fmul_m(k())));
        }
        8 => {
            crt_sscanf2(line, 0x86AA38, slot(0x50), slot(0x3C));
            wr::<u16>(melee(0x84), 0, rd::<u16>(fr, 0x3C));
        }
        _ => {}
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_DUCK_CTOR,
    HOOK_DUCK_GET_TASK_TYPE,
    HOOK_DUCK_DTOR,
    HOOK_DUCK_DELETING_DTOR,
    HOOK_DUCK_MAKE_ABORTABLE,
    HOOK_DUCK_CONTROL_DUCK_MOVE,
    HOOK_DUCK_DELETE_DUCK_ANIM_CB,
    HOOK_DUCK_CAN_PED_DUCK,
    HOOK_DUCK_SET_MOVE_ANIM,
    HOOK_DUCK_PROCESS_PED,
    HOOK_FIGHT_CTOR,
    HOOK_FIGHT_GET_TASK_TYPE,
    HOOK_FIGHT_DTOR,
    HOOK_FIGHT_DELETING_DTOR,
    HOOK_FIGHT_CONTROL_FIGHT,
    HOOK_FIGHT_PROCESS_PED,
    HOOK_FIGHT_LOAD_MELEE_DATA,
];
