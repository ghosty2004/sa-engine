//! The four unnamed `CTaskSimpleFight` helpers `CTaskSimpleFight::ProcessPed` (0x629920, `fight.rs`) calls
//! (PORT-AI-5, helper B): the anim starter 0x623B10, the strike / hit test 0x6240B0 (also called by the
//! pistol whip of `CTaskSimpleUseGun::ProcessPed`), and the two attack choosers 0x624710 (player) and
//! 0x624A40 (AI). Ported from the 1.0 US bytes (the exe wins over the specs in
//! `docs/re/specs/AI/_unattributed-J4.md`; gta-reversed only declares them). Every call to another
//! catalogued function goes through `orig!` at the original address.
//!
//! `CTaskSimpleFight`: +8 finished, +0xA anim block referenced, +0xC anim group, +0x14 byte, +0x15 chain
//! count, +0x18 target, +0x1C `m_pAnim`, +0x20 `m_pIdleAnim`, +0x24 combo set, +0x25 current move, +0x26
//! next command, +0x27 last command. Combo data `CTaskSimpleFight::m_aComboData[set - 4]` at 0xC170D0
//! (stride 0x88): +0 anim group, +0x30 f32 radius[5], +0x44 f32 ground-loop time, +0x50 u8 hit level[5],
//! +0x84 u8 flags (low byte), +0x85 u8 flags (high byte) (the `CMeleeInfo` layout of PORT-AI-2).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTaskSimpleFight::m_aComboData` (0xC170D0, stride 0x88).
const COMBO_DATA: u32 = 0xC170D0;
/// The fight anim callback (`FUN_0061dae0`), passed as the original address.
const FIGHT_ANIM_CB: u32 = 0x61DAE0;
/// `CDefaultAnimCallback::DefaultAnimCB`.
const DEFAULT_ANIM_CB: u32 = 0x4D1390;
/// `CModelInfo::ms_modelInfoPtrs`.
const MODEL_INFO_PTRS: u32 = 0xA9B0C8;
/// `CAnimManager::ms_aAnimAssocGroups` (pointer, stride 0x14).
const ANIM_ASSOC_GROUPS: usize = 0xB4EA34;
/// `CAnimManager::ms_aAnimBlocks` (stride 0x20).
const ANIM_BLOCKS: u32 = 0xB5D4A0;
/// `CWorld::m_aTempColPts` (CColPoint[], +0 position, +0x10 normal, +0x23 / +0x24 surface bytes).
const TEMP_COL_PTS: u32 = 0xB9ACD0;
/// The `CColModel` the fight collision is set up in (`FightSetUpCol` 0x61D5F0).
const FIGHT_COL_MODEL: u32 = 0xC17824;
/// `.data` tunable read by 0x624710: maximum chain count (initial value 2).
const MAX_CHAIN: usize = 0x8D2E48;
/// `.data` tunable read by 0x624710: facing search range (initial value 2.0).
const FACE_RANGE: usize = 0x8D2E8C;

/// An f32 constant of `.rdata`.
fn c(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// `fabs` of an x87 value (clears the sign bit, NaN payload kept).
fn fabs(v: X87) -> X87 {
    X87::from_f64_exact(v.to_f64().abs())
}

/// Address of `m_aComboData[set - 4] + off` (`movsx; sub 4; imul 0x88`, 32-bit wrapping).
fn combo(set: i8, off: u32) -> usize {
    (i32::from(set) - 4)
        .wrapping_mul(0x88)
        .wrapping_add((COMBO_DATA + off) as i32) as u32 as usize
}

/// Anim group of a combo set (`m_aComboData[set - 4]` +0).
fn combo_group(set: i8) -> u32 {
    g::<u32>(combo(set, 0)).read()
}

/// The combo flags of the current set (+0x84), masked with the ped's style flags (+0x72E) for sets 5..7.
fn combo_flags(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let set = rd::<i8>(this, 0x24);
    let mut f = g::<u8>(combo(set, 0x84)).read();
    if set > 4 && set <= 7 {
        f &= rd::<u8>(ped, 0x72E);
    }
    f
}

/// Entity position: matrix +0x30, else the placement at +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// Pointer to a Rust stack buffer (stands in for an object the original builds on its own stack).
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

orig! {
    // ---- CTaskSimpleFight (unnamed) ----
    /// `FUN_0061d6f0(CPed*)` (0x61D6F0, `ret 4`; target on the ground test)
    fn fight_61d6f0(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x61D6F0 => unported;
    /// `FUN_0061c9b0(CPed*)` (0x61C9B0, `ret 4`)
    fn fight_61c9b0(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61C9B0 => unported;
    /// `FUN_0061d5f0(float radius)` (0x61D5F0, `ret 4`; sets up the fight col model)
    fn fight_set_up_col(t: GPtr<u8>, radius: u32) = thiscall 0x61D5F0 => unported;
    /// `FUN_0061cba0(CPed*, CPed* victim, CVector* pos, CVector* dir, int32)` (0x61CBA0, `ret 0x14`; hit a ped)
    fn fight_hit_ped(t: GPtr<u8>, ped: GPtr<u8>, victim: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>, a: i32) -> GPtr<u8> = thiscall 0x61CBA0 => unported;
    /// `FUN_0061d0b0(CPed*, CVehicle*, CVector* pos, CVector* normal, int16, uint8)` (0x61D0B0, `ret 0x18`; hit a car)
    fn fight_hit_car(t: GPtr<u8>, ped: GPtr<u8>, v: GPtr<u8>, pos: u32, normal: u32, a: u32, b: u32) = thiscall 0x61D0B0 => unported;
    /// `FUN_0061d400(CPed*, CObject*, CVector* pos, CVector* normal, int16, uint8)` (0x61D400, `ret 0x18`; hit an object)
    fn fight_hit_obj(t: GPtr<u8>, ped: GPtr<u8>, o: GPtr<u8>, pos: u32, normal: u32, a: u32, b: u32) = thiscall 0x61D400 => unported;

    // ---- other catalogued functions ----
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00, `ret 4`)
    fn ped_set_move_state(p: GPtr<u8>, state: u32) = thiscall 0x5DEC00 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::GetWantedLevel()` (0x41BE60)
    fn player_get_wanted_level(p: GPtr<u8>) -> i32 = thiscall 0x41BE60 => unported;
    /// `CPedIntelligence::GetTaskDuck(bool)` (0x6010A0, `ret 4`)
    fn intel_get_task_duck(i: GPtr<u8>, ignore: u32) -> GPtr<u8> = thiscall 0x6010A0 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn weapon_info_get(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CStats::GetFatAndMuscleModifier(int32)` (0x559AF0, float in ST0)
    fn stats_get_fat_and_muscle_modifier(a: i32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0)
    fn anim_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CAnimBlendAssociation::SetDeleteCallback(cb, data)` (0x4CEBC0)
    fn anim_set_delete_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBC0 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80, `ret 4`; the float passed as its bits)
    fn anim_set_current_time(a: GPtr<u8>, t: u32) = thiscall 0x4CEA80 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn anim_remove_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    fn anim_add_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::GetAnimBlockName(AssocGroupId)` (0x4D3A30)
    fn anim_get_anim_block_name(group: u32) -> u32 = cdecl 0x4D3A30 => unported;
    /// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940)
    fn anim_get_animation_block(name: u32) -> GPtr<u8> = cdecl 0x4D3940 => unported;
    /// `CStreaming::RequestModel(int32 id, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association_id(clump: u32, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn world_find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CGlass::BreakGlassPhysically(CVector pos (by value), float radius)` (0x71CF50)
    fn glass_break_glass_physically(x: u32, y: u32, z: u32, r: u32) = cdecl 0x71CF50 => unported;
    /// `CWorld::FindObjectsInRange(const CVector&, float, bool 2D, int16* count, int16 max, CEntity**,
    /// bool buildings, bool vehicles, bool peds, bool objects, bool dummies)` (0x564A20)
    fn world_find_objects_in_range(pos: GPtr<u8>, r: u32, b2d: u32, count: GPtr<u8>, max: u32, out: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32) = cdecl 0x564A20 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0, `ret 4`)
    fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BCF0 => unported;
    /// `CMatrix::SetTranslateOnly(float, float, float)` (0x59AF80, `ret 0xC`)
    fn matrix_set_translate_only(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF80 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CEntity::GetBoundCentre(CVector& out)` (0x534250, `ret 4`; returns `&out`)
    fn entity_get_bound_centre(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x534250 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> u32 = thiscall 0x535300 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CCollision::ProcessColModels(const CMatrix&, CColModel&, const CMatrix&, CColModel&, CColPoint*,
    /// CColPoint*, float*, bool)` (0x4185C0)
    fn collision_process_col_models(m1: GPtr<u8>, c1: u32, m2: GPtr<u8>, c2: u32, cps: u32, lcps: u32, dists: u32, b: u32) -> i32 = cdecl 0x4185C0 => unported;
    /// `CPedModelInfo::AnimatePedColModelSkinnedWorld(RpClump*)` (0x4C7170, `ret 4`)
    fn ped_mi_animate_col_model_skinned_world(mi: GPtr<u8>, clump: u32) -> GPtr<u8> = thiscall 0x4C7170 => unported;
    /// `CEventSoundQuiet::Constructor(CEntity*, float volume, uint32 time, const CVector&)` (0x5E05B0, `ret 0x10`)
    fn event_sound_quiet_ctor(e: GPtr<u8>, ent: GPtr<u8>, vol: u32, time: i32, pos: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E05B0 => unported;
    /// `FUN_005dea00` (0x5DEA00): `CEventSoundQuiet` destructor body
    fn event_sound_quiet_dtor(e: GPtr<u8>) = thiscall 0x5DEA00 => unported;
    /// `RwHelper::GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

/// Common tail of 0x623B10: the last command (+0x27) becomes the next one (+0x26), which is cleared.
fn start_anim_end(this: GPtr<u8>) {
    wr::<u8>(this, 0x27, rd::<u8>(this, 0x26));
    wr::<u8>(this, 0x26, 0);
}

/// `FUN_00623b10(CPed*, int32 move)` (0x623B10, `ret 8`; `CTaskSimpleFight::StartAnim` in the spec). A negative
/// `move` only clears +0x26. Otherwise the current anim gets the default delete callback and is dropped, then the
/// next command (+0x26, signed, unsigned compare against 0x12; byte table 0x624098 -> jump table 0x624084)
/// selects: 0 idle, 1 / 15..18 move, 2 block, 3..10 plain (non-player) attack, 11..14 combo attack; the tail copies
/// +0x26 to +0x27 and clears +0x26 (idle also clears +0x24 first; move sets +0x26 = 0x10 before the tail).
#[hook(addr = 0x623B10, conv = thiscall, name = "FUN_00623b10", expect = "8B 44 24 08 85 C0")]
pub fn fight_start_anim(this: GPtr<u8>, ped: GPtr<u8>, mv: i32) {
    if mv < 0 {
        wr::<u8>(this, 0x26, 0);
        return;
    }
    let a = rd::<GPtr<u8>>(this, 0x1C);
    if !a.is_null() {
        anim_set_delete_callback(a, DEFAULT_ANIM_CB, 0);
        wr::<u32>(this, 0x1C, 0);
    }
    let cmd = rd::<i8>(this, 0x26);
    if i32::from(cmd) as u32 > 0x12 {
        start_anim_end(this);
        return;
    }
    let clump = || rd::<u32>(ped, 0x18);
    match cmd {
        0 => start_anim_idle(this, ped),
        2 => {
            // BLOCK (0x623EF7)
            let flags_hi = g::<u8>(combo(rd::<i8>(this, 0x24), 0x85)).read();
            wr::<u8>(this, 0x25, 0);
            wr::<u8>(this, 0x14, 0);
            if flags_hi & 2 == 0 {
                wr::<u8>(this, 0x24, 4);
            }
            let a = anim_blend_animation(clump(), combo_group(rd::<i8>(this, 0x24)), 0xDE, 0x4100_0000);
            wr::<GPtr<u8>>(this, 0x1C, a);
            anim_set_finish_callback(a, FIGHT_ANIM_CB, this.addr() as u32);
            start_anim_end(this);
        }
        3..=10 => {
            // PLAIN (0x623E21)
            let pl = ped_is_player(ped);
            if pl != 0 {
                start_anim_end(this);
                return;
            }
            wr::<u8>(this, 0x14, pl);
            let c = rd::<u8>(this, 0x26);
            wr::<u8>(this, 0x24, 1);
            let id = match c {
                7 => {
                    wr::<u8>(this, 0x25, 0);
                    0x2D
                }
                9 => {
                    wr::<u8>(this, 0x25, 2);
                    0x2E
                }
                _ => {
                    let m = match c {
                        8 => 1,
                        10 => 2,
                        _ => c.wrapping_sub(3),
                    };
                    wr::<u8>(this, 0x25, m);
                    (i32::from(rd::<i8>(this, 0x25)) + 0x29) as u32
                }
            };
            let a = anim_blend_animation(clump(), 0, id, 0x4100_0000);
            wr::<GPtr<u8>>(this, 0x1C, a);
            if rd::<u8>(this, 0x26) == 3 {
                anim_set_delete_callback(rd::<GPtr<u8>>(this, 0x1C), FIGHT_ANIM_CB, this.addr() as u32);
                let a = rd::<GPtr<u8>>(this, 0x1C);
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
            } else {
                let a = rd::<GPtr<u8>>(this, 0x1C);
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFD);
                let a = rd::<GPtr<u8>>(this, 0x1C);
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xDF);
                let a = rd::<GPtr<u8>>(this, 0x1C);
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 8);
                anim_set_finish_callback(rd::<GPtr<u8>>(this, 0x1C), FIGHT_ANIM_CB, this.addr() as u32);
            }
            start_anim_end(this);
        }
        11..=14 => {
            // ATTACK (0x623B59)
            let group = combo_group(rd::<i8>(this, 0x24));
            let m = mv as u8;
            wr::<u8>(this, 0x25, m);
            wr::<u8>(this, 0x14, 1);
            let id = (i32::from(m as i8) + 0xD6) as u32;
            let a = anim_blend_animation(clump(), group, id, 0x4100_0000);
            wr::<GPtr<u8>>(this, 0x1C, a);
            anim_set_finish_callback(a, FIGHT_ANIM_CB, this.addr() as u32);
            if rd::<u8>(this, 0x25) == 3 {
                let a = rd::<GPtr<u8>>(this, 0x1C);
                // fcomp [0.0]; test ah, 0x44; jnp: skipped only when the time equals 0
                if x(rd::<f32>(a, 0x20)).fcom(x(c(0x858B50))).test_parity(0x44) {
                    let t = g::<u32>(combo(rd::<i8>(this, 0x24), 0x44)).read();
                    anim_set_current_time(a, t);
                }
            }
            if rd::<u32>(ped, 0x480) != 0 {
                let v = stats_get_fat_and_muscle_modifier(3);
                wr::<f32>(rd::<GPtr<u8>>(this, 0x1C), 0x24, X87::from_f64_exact(v).to_f32());
                if rd::<i8>(this, 0x25) < 4 {
                    ped_set_move_state(ped, 1);
                }
            }
            start_anim_end(this);
        }
        _ => start_anim_move(this, ped, cmd as u8),
    }
}

/// IDLE case of 0x623B10 (0x623C1C).
fn start_anim_idle(this: GPtr<u8>, ped: GPtr<u8>) {
    wr::<u8>(this, 0x24, 0);
    wr::<u8>(this, 0x25, 0);
    wr::<u8>(this, 0x14, 0);
    if rd::<i32>(ped, 0x534) >= 4 && ped_is_player(ped) != 0 {
        let c26 = rd::<u8>(this, 0x26);
        wr::<u8>(this, 8, 1);
        wr::<u8>(this, 0x27, c26);
        wr::<u8>(this, 0x26, 0);
        return;
    }
    let slot = i32::from(rd::<i8>(ped, 0x718)).wrapping_mul(0x1C);
    let ty = rd::<u32>(ped, (slot + 0x5A0) as isize);
    let mut k = rd::<i8>(weapon_info_get(ty, 1), 0x6E);
    if k == 4 {
        k = rd::<i8>(ped, 0x72D);
    }
    let set: i8;
    if g::<u8>(combo(k, 0x85)).read() & 4 == 0 {
        set = 4;
    } else {
        let grp = combo_group(k);
        let load = if grp == 0x21 {
            false
        } else {
            let cur = rd::<u32>(this, 0xC);
            if grp != cur {
                if rd::<u8>(this, 0xA) != 0 {
                    let groups = g::<u32>(ANIM_ASSOC_GROUPS).read();
                    let blk = g::<u32>(groups.wrapping_add(cur.wrapping_mul(0x14)) as usize).read();
                    anim_remove_block_ref((blk.wrapping_sub(ANIM_BLOCKS) as i32) >> 5);
                    wr::<u8>(this, 0xA, 0);
                }
                wr::<u32>(this, 0xC, combo_group(k));
                true
            } else {
                rd::<u8>(this, 0xA) == 0
            }
        };
        if load {
            // LOAD (0x623CEC)
            let grp = rd::<u32>(this, 0xC);
            let groups = g::<u32>(ANIM_ASSOC_GROUPS).read();
            let mut b = g::<GPtr<u8>>(groups.wrapping_add(grp.wrapping_mul(0x14)) as usize).read();
            if b.is_null() {
                b = anim_get_animation_block(anim_get_anim_block_name(grp));
            }
            let loaded = rd::<u8>(b, 0x10);
            let idx = ((b.addr() as u32).wrapping_sub(ANIM_BLOCKS) as i32) >> 5;
            if loaded != 0 {
                anim_add_block_ref(idx);
                wr::<u8>(this, 0xA, 1);
                set = k;
            } else {
                streaming_request_model(idx.wrapping_add(0x63E7), 8);
                let c26 = rd::<i8>(this, 0x26);
                if (11..=14).contains(&c26) {
                    wr::<u8>(this, 0x26, 11);
                }
                set = 4;
            }
        } else {
            set = k;
        }
    }
    // 0x623D2E
    let idle = rd::<GPtr<u8>>(this, 0x20);
    wr::<i8>(this, 0x24, set);
    if idle.is_null() {
        let clump = rd::<u32>(ped, 0x18);
        let a = anim_blend_animation(clump, combo_group(set), 0xDF, 0x4100_0000);
        wr::<GPtr<u8>>(this, 0x20, a);
        anim_set_delete_callback(a, FIGHT_ANIM_CB, this.addr() as u32);
    } else if !x(rd::<f32>(idle, 0x18)).fcom(x(c(0x858624))).test_parity(5)
        && !x(rd::<f32>(idle, 0x1C)).fcom(x(c(0x858B50))).test_parity(0x41)
    {
        // blend amount < 1.0 and blend delta <= 0.0 (each `jp` leaves on NaN)
        let clump = rd::<u32>(ped, 0x18);
        let a = anim_blend_animation(clump, combo_group(set), 0xDF, 0x4100_0000);
        wr::<GPtr<u8>>(this, 0x20, a);
    }
    if ped_is_player(ped) != 0 {
        let pd = rd::<GPtr<u8>>(ped, 0x480).byte_add(0xC);
        wr::<u32>(pd, 0, 0);
        wr::<u32>(pd, 4, 0);
        fight_61c9b0(this, ped);
    }
    let c26 = rd::<u8>(this, 0x26);
    wr::<u8>(this, 0x24, 0);
    wr::<u8>(this, 0x27, c26);
    wr::<u8>(this, 0x26, 0);
}

/// MOVE case of 0x623B10 (0x623F68): commands 1 and 15..18 (`cmd`).
fn start_anim_move(this: GPtr<u8>, ped: GPtr<u8>, cmd: u8) {
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if !pd.is_null() {
        wr::<u32>(pd, 0xC, 0);
        wr::<u32>(pd, 0x10, 0);
        fight_61c9b0(this, ped);
        if cmd == 0x11 {
            wr::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x14, 0x4000_0000);
        } else if cmd == 0x10 {
            wr::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x14, 0x3F80_0000);
        }
    } else {
        let st = match cmd {
            0x11 => 6,
            0x10 => 4,
            _ => 1,
        };
        ped_set_move_state(ped, st);
        wr::<u32>(ped, 0x538, rd::<u32>(ped, 0x534));
    }
    let group = || rd::<u32>(ped, 0x4D4);
    let clump = || rd::<u32>(ped, 0x18);
    match cmd {
        0x11 => {
            let g_ = group();
            anim_blend_animation(clump(), g_, 1, 0x4100_0000);
        }
        0x10 => {
            let g_ = group();
            anim_blend_animation(clump(), g_, 0, 0x4100_0000);
        }
        0x0F => {
            anim_blend_animation(clump(), group(), 3, 0x4080_0000);
        }
        _ => {
            if cmd == 0x12
                && rd::<u32>(ped, 0x46C) & 0x0400_0000 != 0
                && !intel_get_task_duck(rd::<GPtr<u8>>(ped, 0x47C), 1).is_null()
            {
                anim_blend_animation(clump(), 0, 0x37, 0x4080_0000);
            } else {
                anim_blend_animation(clump(), group(), 3, 0x4000_0000);
            }
        }
    }
    let idle = rd::<GPtr<u8>>(this, 0x20);
    if !idle.is_null() {
        wr::<u8>(idle, 0x2E, rd::<u8>(idle, 0x2E) & 0xFE);
    } else {
        wr::<u8>(this, 8, 1);
    }
    wr::<u8>(this, 0x26, 0x10);
    start_anim_end(this);
}

/// `FUN_006240b0(CPed*, CVector* pos)` (0x6240B0, `ret 8`; `CTaskSimpleFight::FightStrike` in the spec). Returns
/// AL = 0. Hit test of the current move (radius `fR = m_aComboData[+0x24 - 4].radius[(i8)+0x25]`): glass, then the
/// 16 scanned peds, the 16 scanned vehicles and (player only) up to 16 objects in 5 units of `pos`; peds are
/// tested sphere by sphere (pushing `pos` forward by 1.5 * fR along the ped's forward axis up to twice for hit
/// levels >= 4), vehicles and objects with `CCollision::ProcessColModels` against the fight col model. A miss by
/// the player raises a quiet sound event; the result position is stored at ped +0x720.
///
/// Stack objects are Rust locals: the `CMatrix` copy (0x48), the second `CMatrix` whose attach fields the
/// original zeroes and only destroys (0x48, all zero), the object list, the bound-centre outputs, the hit
/// direction vector (zeroed and reused as the sound event position, as the original reuses the slot), and the
/// `CEventSoundQuiet` (the original's slot is 0x2C bytes; the buffer is 0x40). The two `int16` arguments of the
/// car / object hit handlers are pushed with stale upper halves (the callee-clobbered ECX / EDX after
/// `ProcessColModels`); the port passes them zero-extended.
#[hook(addr = 0x6240B0, conv = thiscall, name = "FUN_006240b0", expect = "6A FF 68 11 FD 83 00")]
pub fn fight_strike(this: GPtr<u8>, ped: GPtr<u8>, pos: GPtr<u8>) -> u8 {
    let slot_off = || (i32::from(rd::<i8>(ped, 0x718)).wrapping_mul(0x1C) + 0x5A0) as isize;
    weapon_info_get(rd::<u32>(ped, slot_off()), 1);
    let set = rd::<i8>(this, 0x24);
    let cd = combo(set, 0);
    let radius_addr = || cd.wrapping_add((i32::from(rd::<i8>(this, 0x25)) * 4 + 0x30) as usize);
    let mut missed = true;
    // the second CMatrix: +0x40 / +0x44 zeroed by the original, never otherwise written
    let mut m2 = [0u32; 0x12];
    if world_find_player_ped(-1) == ped && rd::<u32>(ped, slot_off()) != 0 {
        let fr = g::<u32>(radius_addr()).read();
        glass_break_glass_physically(rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8), fr);
    }
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let mut n_obj = [0u32; 1];
    let mut hit_ped = GPtr::<u8>::NULL;
    let mut objs = [0u32; 16];
    if ped_is_player(ped) != 0 {
        world_find_objects_in_range(
            pos,
            0x40A0_0000,
            1,
            buf(&mut n_obj),
            0x10,
            buf(&mut objs),
            0,
            0,
            0,
            1,
            0,
        );
    }
    fight_set_up_col(this, g::<u32>(radius_addr()).read());
    let mut m = [0u32; 0x12];
    matrix_copy_ctor(buf(&mut m), rd::<GPtr<u8>>(ped, 0x14));
    matrix_set_translate_only(
        buf(&mut m),
        rd::<u32>(pos, 0),
        rd::<u32>(pos, 4),
        rd::<u32>(pos, 8),
    );
    let mut d = [0u32; 3];
    let count = |n: &[u32; 1]| i32::from(n[0] as u16 as i16) + 0x20;
    let mut i: i32 = 0;
    while i < count(&n_obj) {
        let (p, v, o) = if i < 16 {
            (
                rd::<GPtr<u8>>(intel, (0x130 + i * 4) as isize),
                GPtr::NULL,
                GPtr::NULL,
            )
        } else if i < 32 {
            (
                GPtr::NULL,
                rd::<GPtr<u8>>(intel, (0xA0 + i * 4) as isize),
                GPtr::NULL,
            )
        } else {
            // objs[i - 32]; FindObjectsInRange writes at most 16 entries
            let e = objs.get((i - 32) as usize).copied().unwrap_or(0);
            (GPtr::NULL, GPtr::NULL, GPtr::from_addr(e as usize))
        };
        let e = if !p.is_null() {
            p
        } else if !v.is_null() {
            v
        } else {
            o
        };
        i += 1;
        if e.is_null() {
            continue;
        }
        strike_entity(
            this,
            ped,
            pos,
            cd,
            e,
            p,
            v,
            o,
            &mut m,
            &mut d,
            &mut missed,
            &mut hit_ped,
        );
    }
    if missed && ped_is_player(ped) != 0 {
        d = [0; 3];
        let mut ev = [0u32; 16];
        event_sound_quiet_ctor(buf(&mut ev), ped, 0x4220_0000, -1, buf(&mut d));
        let grp = get_event_global_group();
        event_group_add(grp, buf(&mut ev), 0);
        event_sound_quiet_dtor(buf(&mut ev));
    }
    if rd::<u8>(this, 0x24) == 7
        && rd::<u8>(this, 0x25) == 1
        && !rd::<GPtr<u8>>(this, 0x1C).is_null()
        && (hit_ped.is_null() || clump_get_association_id(rd::<u32>(hit_ped, 0x18), 0xDC).is_null())
    {
        wr::<u32>(rd::<GPtr<u8>>(this, 0x1C), 0x1C, 0xC080_0000);
        let a = rd::<GPtr<u8>>(this, 0x1C);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFE);
        let a = rd::<GPtr<u8>>(this, 0x1C);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
    }
    wr::<u32>(ped, 0x720, rd::<u32>(pos, 0));
    wr::<u32>(ped, 0x724, rd::<u32>(pos, 4));
    wr::<u32>(ped, 0x728, rd::<u32>(pos, 8));
    matrix_dtor(buf(&mut m));
    matrix_dtor(buf(&mut m2));
    0
}

/// One entity of the 0x6240B0 loop (0x62427D-0x6245B4). `p` / `v` / `o` are the ped / vehicle / object slot
/// values of this iteration (only one is non-null, `e`).
#[allow(clippy::too_many_arguments)]
fn strike_entity(
    this: GPtr<u8>,
    ped: GPtr<u8>,
    pos: GPtr<u8>,
    cd: usize,
    e: GPtr<u8>,
    p: GPtr<u8>,
    v: GPtr<u8>,
    o: GPtr<u8>,
    m: &mut [u32; 0x12],
    d: &mut [u32; 3],
    missed: &mut bool,
    hit_ped: &mut GPtr<u8>,
) {
    let mv = || i32::from(rd::<i8>(this, 0x25));
    let radius = |k: i32| g::<f32>(cd.wrapping_add((k * 4 + 0x30) as usize)).read();
    let hit_level = |k: i32| g::<u8>(cd.wrapping_add((k + 0x50) as usize)).read();
    let model_info = |e: GPtr<u8>| {
        let idx = i32::from(rd::<i16>(e, 0x22));
        g::<GPtr<u8>>(MODEL_INFO_PTRS.wrapping_add((idx * 4) as u32) as usize).read()
    };
    let col = rd::<GPtr<u8>>(model_info(e), 0x14);
    let k = mv();
    let mut r = f32_of(x(rd::<f32>(col, 0x24)).fadd_m(radius(k)));
    let eligible = if !p.is_null() {
        if hit_level(k) >= 4 {
            r = f32_of(x(radius(k)).fmul_m(c(0x858B8C)).fadd_m(r));
        }
        rd::<u8>(p, 0x1C) & 1 != 0
            || ped_is_alive(p) == 0
            || (rd::<u32>(p, 0x46C) & 0x100 != 0 && {
                let veh = rd::<GPtr<u8>>(p, 0x58C);
                !veh.is_null() && rd::<i32>(veh, 0x590) == 9
            })
    } else {
        false
    };
    let eligible = eligible
        || (!v.is_null() && rd::<i32>(v, 0x590) == 0)
        || (!o.is_null() && rd::<u8>(o, 0x1C) & 1 != 0);
    if !eligible {
        return;
    }
    let mut centre = [0u32; 3];
    let sum = if !p.is_null() {
        let cp = entity_get_bound_centre(e, buf(&mut centre));
        let dy = x(rd::<f32>(cp, 4)).fsub_m(rd::<f32>(pos, 4));
        let dx = x(rd::<f32>(cp, 0)).fsub_m(rd::<f32>(pos, 0));
        dx.fmul(dx).fadd(dy.fmul(dy))
    } else {
        let cp = entity_get_bound_centre(e, buf(&mut centre));
        let dz = x(rd::<f32>(cp, 8)).fsub_m(rd::<f32>(pos, 8));
        let dy = x(rd::<f32>(cp, 4)).fsub_m(rd::<f32>(pos, 4));
        let dx = x(rd::<f32>(cp, 0)).fsub_m(rd::<f32>(pos, 0));
        dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz))
    };
    // fld r; fmul r; fxch; fcompp; test ah, 5; jp: next unless sum < r*r
    if sum.fcom(x(r).fmul_m(r)).test_parity(5) {
        return;
    }
    if !p.is_null() {
        let mi = model_info(p);
        let clump = rd::<u32>(p, 0x18);
        let mut tries = 0i32;
        let cdata = rd::<GPtr<u8>>(ped_mi_animate_col_model_skinned_world(mi, clump), 0x2C);
        loop {
            let n = i32::from(rd::<i16>(cdata, 0));
            if n > 0 {
                let fr_k = mv();
                let mut s = rd::<GPtr<u8>>(cdata, 8);
                for _ in 0..n {
                    let dz = x(rd::<f32>(s, 8)).fsub_m(rd::<f32>(pos, 8));
                    let dy = x(rd::<f32>(s, 4)).fsub_m(rd::<f32>(pos, 4));
                    let dx = x(rd::<f32>(s, 0)).fsub_m(rd::<f32>(pos, 0));
                    let dx32 = f32_of(dx);
                    d[0] = dx32.to_bits();
                    d[1] = f32_of(dy).to_bits();
                    d[2] = f32_of(dz).to_bits();
                    let sum = x(dx32).fmul_m(dx32).fadd(dy.fmul(dy)).fadd(dz.fmul(dz));
                    let t = x(rd::<f32>(s, 0xC)).fadd_m(radius(fr_k));
                    // fcomp; test ah, 5; jnp HIT: sum < t*t
                    if !sum.fcom(t.fmul(t)).test_parity(5) {
                        let h = fight_hit_ped(this, ped, p, pos, buf(d), 3);
                        if !h.is_null() {
                            *hit_ped = h;
                        }
                        *missed = false;
                        return;
                    }
                    s = s.byte_add(0x14);
                }
            }
            let k = mv();
            if hit_level(k) < 4 {
                return;
            }
            let mat = rd::<GPtr<u8>>(ped, 0x14);
            let f15 = c(0x858CE8);
            let z15 = x(rd::<f32>(mat, 0x18)).fmul_m(f15);
            let y15 = x(rd::<f32>(mat, 0x14)).fmul_m(f15);
            let x15 = x(rd::<f32>(mat, 0x10)).fmul_m(f15);
            tries += 1;
            let x15 = f32_of(x15);
            let fr = radius(k);
            let zv = f32_of(z15.fmul_m(fr));
            let yv = y15.fmul_m(fr);
            let xv = x(x15).fmul_m(fr);
            wr::<f32>(pos, 0, f32_of(xv.fadd_m(rd::<f32>(pos, 0))));
            wr::<f32>(pos, 4, f32_of(yv.fadd_m(rd::<f32>(pos, 4))));
            wr::<f32>(pos, 8, f32_of(x(zv).fadd_m(rd::<f32>(pos, 8))));
            if tries >= 2 {
                return;
            }
        }
    }
    // vehicle / object (0x6244FC)
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        placeable_allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    let em = rd::<GPtr<u8>>(e, 0x14);
    let ecol = entity_get_col_model(e);
    let n = collision_process_col_models(buf(m), FIGHT_COL_MODEL, em, ecol, TEMP_COL_PTS, 0, 0, 0);
    if n <= 0 {
        return;
    }
    let surf_b = u32::from(g::<u8>(0xB9ACF4).read());
    let surf_a = u32::from(g::<u8>(0xB9ACF3).read());
    if !v.is_null() {
        fight_hit_car(this, ped, v, TEMP_COL_PTS, 0xB9ACE0, surf_b, surf_a);
    } else if !o.is_null() {
        fight_hit_obj(this, ped, o, TEMP_COL_PTS, 0xB9ACE0, surf_b, surf_a);
    }
}

/// `FUN_00624710(CPed*)` (0x624710, `ret 4`; `CTaskSimpleFight::ChooseAttackPlayer` in the spec; the entry
/// continues through the island 0x40377E that sets the no-target flag). Returns the next move in AX (callers read
/// AX only; the normal path leaves the upper half of EAX stale, the port zero-extends; the two early returns set
/// EAX = -1 / 3). Without a target (+0x18) the ped turns towards the nearest living scanned ped within
/// [0x8D2E8C] (by heading difference; `ped +0x55C` written when one was found).
///
/// Ported, not registered (PORT-AI-5): split weapon-combat (ai5v4) 13 of 19 pairs differ in the upper half of EAX
/// (orig 0x2106xxxx, rust 0): the original returns `mov ax, bp` over a stale EAX (the ped pointer loaded at
/// 0x6249A4 on the loop path). Hook attribute was `addr = 0x624710, conv = thiscall, expect = "83 EC 0C 53 55"`.
pub fn fight_choose_attack_player(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let no_target = rd::<u32>(this, 0x18) == 0;
    let mut r: i16 = -1;
    let c26 = rd::<i8>(this, 0x26);
    if !((11..=14).contains(&c26) && rd::<i8>(this, 0x24) >= 4) {
        r = 1;
    }
    let f = combo_flags(this, ped);
    if r < 0 {
        let anim = rd::<GPtr<u8>>(this, 0x1C);
        let cur = if anim.is_null() { 0 } else { rd::<u8>(this, 0x25) };
        if !anim.is_null() && cur != 3 && cur != 4 {
            ped_set_move_state(ped, 1);
            let max: i16 = if f & 2 != 0 {
                2
            } else if f & 1 != 0 {
                1
            } else {
                0
            };
            if fight_61d6f0(this, ped) != 0 {
                r = -1;
            } else {
                let cn = rd::<i8>(this, 0x26);
                let cl = rd::<i8>(this, 0x27);
                let mut same = cn == cl;
                if !same {
                    let chain = rd::<i8>(this, 0x15);
                    if i32::from(chain) > g::<i32>(MAX_CHAIN).read() {
                        same = true;
                    } else {
                        wr::<i8>(this, 0x15, chain.wrapping_add(1));
                        let t = (i32::from(rd::<i8>(this, 0x25)) - i32::from(cl) + i32::from(cn)) as u8;
                        r = if t & 1 != 0 {
                            0
                        } else if t & 2 != 0 {
                            1
                        } else {
                            2
                        };
                        if r > max {
                            r = 0;
                        }
                    }
                }
                if same {
                    r = i16::from(rd::<i8>(this, 0x25)).wrapping_add(1);
                    if r > max {
                        r = -1;
                    }
                }
            }
        } else if rd::<i32>(ped, 0x534) > 4 {
            if f & 8 == 0 {
                wr::<u8>(this, 0x24, 4);
            }
            r = 4;
        } else if fight_61d6f0(this, ped) != 0 {
            if !rd::<GPtr<u8>>(this, 0x1C).is_null() && rd::<u8>(this, 0x26) != rd::<u8>(this, 0x27) {
                return 0xFFFF_FFFF;
            }
            if f & 4 == 0 {
                wr::<u8>(this, 0x24, 4);
            } else {
                let t = rd::<GPtr<u8>>(this, 0x18);
                if !t.is_null() && rd::<u8>(t, 0x36) & 7 == 3 && rd::<u32>(t, 0x46C) & 0x0400_0000 != 0 {
                    wr::<u8>(this, 0x24, 4);
                }
            }
            return 3;
        } else {
            wr::<u8>(this, 0x15, 0);
            r = 0;
        }
    }
    // FACE (0x6248B4)
    if no_target {
        let list = rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x130);
        let mut best = -1000.0f32;
        let mut best_abs = 1000.0f32;
        for i in 0..16isize {
            let q = rd::<GPtr<u8>>(list, i * 4);
            if q.is_null() || ped_is_alive(q) == 0 {
                continue;
            }
            let pp = ent_pos(ped);
            let qp = ent_pos(rd::<GPtr<u8>>(list, i * 4));
            let dx = x(rd::<f32>(qp, 0)).fsub_m(rd::<f32>(pp, 0));
            let dy = x(rd::<f32>(qp, 4)).fsub_m(rd::<f32>(pp, 4));
            let dz = x(rd::<f32>(qp, 8)).fsub_m(rd::<f32>(pp, 8));
            let sum = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
            let range = g::<f32>(FACE_RANGE).read();
            let rr = x(range).fmul_m(g::<f32>(FACE_RANGE).read());
            // fcompp (ST0 = R*R); test ah, 0x41; jne: next unless R*R > sum
            if rr.fcom(sum).test_any(0x41) {
                continue;
            }
            let pp = ent_pos(ped);
            let qp = ent_pos(q);
            let dy = x(rd::<f32>(qp, 4)).fsub_m(rd::<f32>(pp, 4));
            let ndx = x(rd::<f32>(qp, 0)).fsub_m(rd::<f32>(pp, 0)).fchs();
            let a = ndx.fpatan(dy);
            let mut diff = a.fsub_m(rd::<f32>(ped, 0x558));
            if !diff.fcom(x(c(0x858CB8))).test_any(0x41) {
                diff = diff.fsub_m(c(0x858CBC));
            } else if !diff.fcom(x(c(0x858CC0))).test_parity(5) {
                diff = diff.fadd_m(c(0x858CBC));
            }
            let ad = fabs(diff);
            if !ad.fcom(x(best_abs)).test_parity(5) {
                best_abs = f32_of(ad);
                best = f32_of(a);
            }
        }
        if !x(best).fcom(x(c(0x859004))).test_any(0x41) {
            wr::<f32>(ped, 0x55C, best);
        }
    }
    u32::from(r as u16)
}

/// `FUN_00624a40(CPed*)` (0x624A40, `ret 4`; `CTaskSimpleFight::ChooseAttackAI` in the spec). Returns the move
/// in AX (callers read AX only; the original leaves the upper half of EAX stale, the port zero-extends): one
/// `rand()` (return address 0x624A71) scaled to [0, 1]; > 0.8 with flag 2 -> 2 (unless flag 0x40 and the target
/// is a wanted player ped with health > 20, which falls to the 0.5 test); otherwise target on the ground with
/// flag 4 -> 3; then > 0.5 with flag 1 -> 1; else 0.
///
/// Ported, not registered (PORT-AI-5): split weapon-combat (ai5v4) 16 of 21 pairs differ in the upper half of EAX
/// (orig 0x2106xxxx, rust 0): the original returns AX over a stale EAX. Hook attribute was `addr = 0x624A40, conv =
/// thiscall, expect = "53 56 8B F1 8A 46 24"`.
pub fn fight_choose_attack_ai(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let f = combo_flags(this, ped);
    let rv = at_site(0x624A71, rand);
    let r = xi(rv).fmul_m(c(0x858C7C));
    let r32 = f32_of(r);
    if !r.fcom(x(c(0x858C98))).test_any(0x41) && f & 2 != 0 {
        let t = rd::<GPtr<u8>>(this, 0x18);
        let wanted_target = f & 0x40 != 0
            && !t.is_null()
            && rd::<u8>(t, 0x36) & 7 == 3
            && rd::<u32>(t, 0x480) != 0
            && player_get_wanted_level(t) > 0
            && !x(rd::<f32>(t, 0x540)).fcom(x(c(0x858BA4))).test_any(0x41);
        if !wanted_target {
            return 2;
        }
    } else if fight_61d6f0(this, ped) != 0 && f & 4 != 0 {
        return 3;
    }
    if !x(r32).fcom(x(c(0x858B8C))).test_any(0x41) && f & 1 != 0 {
        return 1;
    }
    0
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_FIGHT_START_ANIM,
    HOOK_FIGHT_STRIKE,
];
