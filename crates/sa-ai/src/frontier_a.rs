//! FRONTIER-2 turn 2, lane A (batch `frontier2-a2`): `AI` entries of the `tools/verify/frontier.py` list (task
//! constructors, the sunbathe / medic / on-fire helpers, the attack allocator, the gang drive-by's abort and anim
//! start), ported from the 1.0 US bytes (gta-reversed read as an aid; it reverses none of the drive-by bodies).
//! Entries that start with `push -1; jmp ISLAND` go through a no-CD protection island that only pushes the decoded
//! SEH handler (no observable effect); the medic's island decodes a float constant, read here from its address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::*;
use crate::bindings::task_simple_ctor;
use sa_anim::bindings::{anim_set_delete_callback, get_animation_block};
use sa_core::bindings::{
    anim_add_block_ref, general_get_radian_angle_between_points, general_limit_radian_angle,
    general_random_int_in_range, ped_get_bone_position, streaming_request_model,
};
use sa_math::fpu::{fcom_c0, fcom_ne, gt, le_or_nan, st0};
use sa_stream::bindings::get_animation_block_index;
use sa_world::bindings::{
    anim_blend_animation, anim_get_anim_block_name, damage_calc_compute_damage_response, damage_calc_ctor, damage_calc_dtor,
    get_player_info,
    vector_ne, wander_ctor,
};

use crate::bindings::clean_up_this_vehicle;
use crate::bindings::cleanup_remove_entity_from_list;
orig! {
    /// virtual `CEvent::GetEventType()` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CTaskAllocatorKillOnFoot::Constructor(CPed*, CPed*)` (0x69BB80, `ret 8`)
    fn kill_on_foot_allocator_ctor(t: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x69BB80 => unported;
    /// `CTaskAllocatorKillOnFoot::AllocateTasks(CPedGroupIntelligence*)` (0x69BCA0, `ret 4`)
    fn kill_on_foot_allocator_allocate_tasks(t: GPtr<u8>, group: GPtr<u8>) = thiscall 0x69BCA0 => unported;
    /// `CTaskAllocatorKillOnFoot::Destructor()` (0x69BBF0)
    fn kill_on_foot_allocator_dtor(t: GPtr<u8>) = thiscall 0x69BBF0 => unported;
    /// the sunbathe weather test (0x62F2D0, cdecl, the weather type in AX; generated port in gen_bodies)
    fn sunbathe_weather_ok(w: u32) -> u32 = cdecl 0x62F2D0 => unported;
    /// `CTaskSimpleGangDriveBy` anim start (0x627B20, `ret 4`, AL)
    fn drive_by_start_anim(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x627B20 => gang_drive_by_start_anim;
}

/// `CTaskSimpleJetPack::Constructor(const CVector* target, float cruiseHeight, int32 hoverTime, CEntity* target)`
/// (0x67B4E0, `ret 0x10`): vtable 0x8705C4; the flag bytes +8..+0xE and the words +0x10..+0x50, +0x5C, +0x64, +0x68
/// cleared; +0x54 / +0x58 the height and hover time; +0x60 the target entity (a registered reference); the target
/// position copied to +0x48 when given.
#[hook(addr = 0x67B4E0, conv = thiscall, name = "CTaskSimpleJetPack::Constructor", expect = "6A FF E9 3D C9 D8 FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn jet_pack_ctor(this: GPtr<u8>, pos: GPtr<u32>, height: u32, hover: u32, entity: GPtr<u8>) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x8705C4);
    for o in 8..0xD {
        wr::<u8>(this, o, 0);
    }
    for o in (0x30..=0x50).step_by(4) {
        wr::<u32>(this, o, 0);
    }
    wr::<u32>(this, 0x58, hover);
    wr::<u32>(this, 0x54, height);
    wr::<u32>(this, 0x5C, 0);
    wr::<GPtr<u8>>(this, 0x60, entity);
    wr::<u8>(this, 0xD, 0);
    wr::<u8>(this, 0xE, 0);
    for o in (0x10..=0x2C).step_by(4) {
        wr::<u32>(this, o, 0);
    }
    if !entity.is_null() {
        entity_register_reference(entity, this.byte_add(0x60));
    }
    if !pos.is_null() {
        wr::<u32>(this, 0x48, pos.read());
        wr::<u32>(this, 0x4C, pos.wrapping_index(1).read());
        wr::<u32>(this, 0x50, pos.wrapping_index(2).read());
    }
    wr::<u32>(this, 0x64, 0);
    wr::<u32>(this, 0x68, 0);
    this
}

/// `CTaskComplexArrestPed::Constructor(CPed* ped)` (0x68B990, `ret 4`): `CTaskComplex`, vtable 0x8709A8, +0x10 the
/// ped (a registered reference), +0x20 = 0.
#[hook(addr = 0x68B990, conv = thiscall, name = "CTaskComplexArrestPed::Constructor", expect = "6A FF E9 46 A1 D7 FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn arrest_ped_ctor(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<u32>(this, 0, 0x8709A8);
    wr::<GPtr<u8>>(this, 0x10, ped);
    wr::<u32>(this, 0x20, 0);
    if !ped.is_null() {
        entity_register_reference(ped, this.byte_add(0x10));
    }
    this
}

/// `CTaskComplexGoToPointAiming::Constructor(int32 moveState, const CVector& target, CEntity* aimAt, CVector aimPos,
/// float radius, float slowDownDist)` (0x668790, `ret 0x20`): `CTaskComplex`, vtable 0x86FE00, +0xC move state,
/// +0x10 the aim entity (a registered reference), +0x14 the aim position; then the inlined target setter: when the
/// target differs from +0x20 (`CVector::operator!=` 0x509760) or either float differs from +0x2C / +0x30 (unordered
/// counts as different; the fresh object's fields are compared as they are), the target and both floats are stored
/// and bit 0 of +0x34 set.
#[hook(addr = 0x668790, conv = thiscall, name = "CTaskComplexGoToPointAiming::Constructor", expect = "6A FF E9 BE D0 D9 FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "compares the fresh object's +0x20..+0x30 before writing them, as the original")]
#[allow(clippy::too_many_arguments)]
pub fn go_to_point_aiming_ctor(
    this: GPtr<u8>,
    move_state: u32,
    target: GPtr<u32>,
    aim: GPtr<u8>,
    ax: u32,
    ay: u32,
    az: u32,
    radius: f32,
    slow: f32,
) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<u32>(this, 0x14, ax);
    wr::<u32>(this, 0x18, ay);
    wr::<u32>(this, 0xC, move_state);
    wr::<u32>(this, 0, 0x86FE00);
    wr::<GPtr<u8>>(this, 0x10, aim);
    wr::<u32>(this, 0x1C, az);
    if !aim.is_null() {
        entity_register_reference(aim, this.byte_add(0x10));
    }
    let cur = this.byte_add(0x20);
    if vector_ne(cur, target.cast()) != 0
        || fcom_ne(x(rd::<f32>(this, 0x2C)), x(radius))
        || fcom_ne(x(rd::<f32>(this, 0x30)), x(slow))
    {
        wr::<u32>(cur, 0, target.read());
        wr::<u32>(cur, 4, target.wrapping_index(1).read());
        wr::<u32>(cur, 8, target.wrapping_index(2).read());
        wr::<f32>(this, 0x2C, radius);
        wr::<f32>(this, 0x30, slow);
        wr::<u8>(this, 0x34, rd::<u8>(this, 0x34) | 1);
    }
    this
}

/// `CTaskComplexFollowLeaderInFormation::Constructor(CPedGroup*, CPed* leader, const CVector& offset, float)` (0x6949A0,
/// `ret 0x10`): `CTaskComplex`, vtable 0x870C3C, +0xC the group, +0x10 the leader (a registered reference), +0x14 the
/// offset, +0x20 = 4, +0x24 the float.
#[hook(addr = 0x6949A0, conv = thiscall, name = "CTaskComplexFollowLeaderInFormation::Constructor", expect = "6A FF E9 1C CC D6 FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn follow_leader_in_formation_ctor(this: GPtr<u8>, group: u32, leader: GPtr<u8>, offset: GPtr<u32>, f: u32) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<u32>(this, 0xC, group);
    wr::<u32>(this, 0, 0x870C3C);
    wr::<GPtr<u8>>(this, 0x10, leader);
    wr::<u32>(this, 0x14, offset.read());
    wr::<u32>(this, 0x18, offset.wrapping_index(1).read());
    wr::<u32>(this, 0x1C, offset.wrapping_index(2).read());
    wr::<u32>(this, 0x20, 4);
    wr::<u32>(this, 0x24, f);
    if !leader.is_null() {
        entity_register_reference(leader, this.byte_add(0x10));
    }
    this
}

/// `CTaskComplexSunbathe::Constructor(CObject* towel, bool startStanding)` (0x631F80, `ret 8`): `CTaskComplex`, vtable
/// 0x86E0AC, +0xC the flag, +0xD..+0x10 / +0x1C / +0x1D cleared, +0x14 / +0x18 = 0, +0x34 the towel (a registered
/// reference; its byte +0x13C = 5); +0x24 / +0x2C the block and block index of "beach" (0x86E0A4), +0x28 / +0x30 of
/// "sunbathe" (0x86E098) (`GetAnimationBlock` / `GetAnimationBlockIndex`).
#[hook(addr = 0x631F80, conv = thiscall, name = "CTaskComplexSunbathe::Constructor", expect = "6A FF 68 F8 05 84 00",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn sunbathe_ctor(this: GPtr<u8>, towel: GPtr<u8>, standing: u8) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<u8>(this, 0xC, standing);
    wr::<u32>(this, 0, 0x86E0AC);
    for o in 0xD..=0x10 {
        wr::<u8>(this, o, 0);
    }
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 0x18, 0);
    wr::<u8>(this, 0x1C, 0);
    wr::<u8>(this, 0x1D, 0);
    wr::<GPtr<u8>>(this, 0x34, towel);
    if !towel.is_null() {
        entity_register_reference(towel, this.byte_add(0x34));
        wr::<u8>(rd::<GPtr<u8>>(this, 0x34), 0x13C, 5);
    }
    let beach = GPtr::from_addr(0x86E0A4);
    let sun = GPtr::from_addr(0x86E098);
    wr::<GPtr<u8>>(this, 0x24, get_animation_block(beach));
    wr::<i32>(this, 0x2C, get_animation_block_index(beach));
    wr::<GPtr<u8>>(this, 0x28, get_animation_block(sun));
    wr::<i32>(this, 0x30, get_animation_block_index(sun));
    this
}

/// `CTaskComplexWanderMedic::CTaskComplexWanderMedic(int32 moveState, uint8 dir, bool sensibly)` (0x658770, `ret 0xC`):
/// `CTaskComplexWander(moveState, dir, sensibly, 0.5)` (0x66F450; the island at 0x406A25 reads the float from
/// 0x86FCB0), vtable 0x86F48C.
/// Not hookable (the entry is a bare `jmp` into the island): phase B only.
#[sa_core::native(addr = 0x658770, conv = thiscall, name = "CTaskComplexWanderMedic::CTaskComplexWanderMedic")]
pub fn wander_medic_ctor(this: GPtr<u8>, move_state: u32, dir: u32, sensibly: u32) -> GPtr<u8> {
    wander_ctor(this, move_state, dir, sensibly, g::<u32>(0x86FCB0).read());
    wr::<u32>(this, 0, 0x86F48C);
    this
}

/// `CTaskComplexSunbathe::CanSunbathe()` (0x632140, cdecl): 1 between 10:00 and 17:59 (`CClock::ms_nGameClockHours`
/// 0xB70153) when `CWeather::OldWeatherType` (0xC8131C) is 1, 3, 14, 5, 10 or 18 or passes the weather test
/// 0x62F2D0, else 0 (EAX).
#[hook(addr = 0x632140, conv = cdecl, name = "CTaskComplexSunbathe::CanSunbathe", expect = "A0 53 01 B7 00 3C 0A",
       unverified = "frontier2-a2", route = "L2", hazards = "none")]
pub fn sunbathe_can_sunbathe() -> u32 {
    let h = g::<u8>(0xB70153).read();
    if !(10..0x12).contains(&h) {
        return 0;
    }
    let w = g::<u16>(0xC8131C).read();
    if matches!(w, 1 | 3 | 0xE | 5 | 0xA | 0x12) || sunbathe_weather_ok(u32::from(w)) as u8 != 0 {
        1
    } else {
        0
    }
}

/// `CTaskAllocatorAttack::AllocateTasks(CPedGroupIntelligence*)` (0x69C3F0, `ret 4`): +0x10 = now, +0x14 = 3000,
/// +0x18 = 1; then a stack `CTaskAllocatorKillOnFoot(+4, +8)` allocates the group's tasks and is destroyed.
#[hook(addr = 0x69C3F0, conv = thiscall, name = "CTaskAllocatorAttack::AllocateTasks", expect = "6A FF E9 61 4E D6 FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn allocator_attack_allocate_tasks(this: GPtr<u8>, group: GPtr<u8>) {
    wr::<u32>(this, 0x10, g::<u32>(0xB7CB84).read());
    wr::<u32>(this, 0x14, 0xBB8);
    wr::<u8>(this, 0x18, 1);
    let mut a = [0u32; 3];
    let ap: GPtr<u8> = GPtr::from_raw(a.as_mut_ptr()).cast();
    kill_on_foot_allocator_ctor(ap, rd::<u32>(this, 4), rd::<u32>(this, 8));
    kill_on_foot_allocator_allocate_tasks(ap, group);
    kill_on_foot_allocator_dtor(ap);
}

/// `CTaskComplexOnFire::ComputeFireDamage(CPed* ped, CPedDamageResponse& response)` (0x6333D0, cdecl): nothing for a
/// fire-proof ped (+0x40 bit 0x80000); else a stack `CPedDamageResponseCalculator(the fire's creator (+0x730 fire,
/// its +0x14) or null, TimeStep * ped +0x734 * 500 * 0.001, WEAPON_FLAMETHROWER, torso, false)` computes the response.
#[hook(addr = 0x6333D0, conv = cdecl, name = "CTaskComplexOnFire::ComputeFireDamage", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn on_fire_compute_fire_damage(ped: GPtr<u8>, response: GPtr<u8>) {
    if rd::<u32>(ped, 0x40) & 0x80000 != 0 {
        return;
    }
    let fire = rd::<GPtr<u8>>(ped, 0x730);
    let src = if fire.is_null() { 0 } else { rd::<u32>(fire, 0x14) };
    let dmg = x(g::<f32>(0xB7CB5C).read())
        .fmul_m(rd::<f32>(ped, 0x734))
        .fmul_m(g::<f32>(0x86DB40).read())
        .fmul_m(g::<f32>(0x858CDC).read())
        .to_f32();
    let mut calc = [0u32; 5];
    let cp: GPtr<u8> = GPtr::from_raw(calc.as_mut_ptr()).cast();
    damage_calc_ctor(cp, src, dmg.to_bits(), 0x25, 3, 0);
    damage_calc_compute_damage_response(cp, ped, response, 0);
    damage_calc_dtor(cp);
}

/// `CTaskComplexTreatAccident::ComputeHeading(CPed* medic)` (0x658AF0, `ret 4`, ST0): the midpoint of the injured
/// ped's (accident +0xC, its +0) bones 5 and 1 (`GetBonePosition`), each half `(a + b) * 0.5` stored as f32; the
/// heading is `LimitRadianAngle(GetRadianAngleBetweenPoints(mid.x, mid.y, medic.x, medic.y))`.
#[hook(addr = 0x658AF0, conv = thiscall, name = "CTaskComplexTreatAccident::ComputeHeading", expect = "83 EC 24 56 8B F1",
       unverified = "frontier2-a2", route = "L2", hazards = "none")]
pub fn treat_accident_compute_heading(this: GPtr<u8>, medic: GPtr<u8>) -> X87 {
    let (mut a, mut b) = ([0f32; 3], [0f32; 3]);
    let injured = || rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0);
    ped_get_bone_position(injured(), GPtr::from_raw(a.as_mut_ptr()).cast(), 5, 0);
    ped_get_bone_position(injured(), GPtr::from_raw(b.as_mut_ptr()).cast(), 1, 0);
    let mx = x(b[0]).fadd_m(a[0]).fmul_m(0.5).to_f32();
    let my = x(b[1]).fadd_m(a[1]).fmul_m(0.5).to_f32();
    let m = rd::<GPtr<u8>>(medic, 0x14);
    let pos = if m.is_null() { medic.byte_add(4) } else { m.byte_add(0x30) };
    let ang = general_get_radian_angle_between_points(mx, my, rd::<f32>(pos, 0), rd::<f32>(pos, 4));
    st0(general_limit_radian_angle(st0(ang).to_f32()))
}

/// `CTaskSimpleGangDriveBy::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x62D290, `ret 0xC`, AL). Any priority
/// other than 1 / 2: +0xF = 6, false. A damage event (type 9) without its +0x40 flag or with +0xC clear: false.
/// Otherwise +0xF = 7 and, when the anim start 0x627B20 succeeds, +0x10 = +0xF and the anim (+0x2C) gets a blend delta
/// of -1000 (priority 2) or bit 0 of its flags word +0x2E set; the player data (+0x480) gets bit 3 of +0x34; a player's
/// player info byte +0x153 = 1; an anim whose flag bit 0 is clear gets a blend delta of -8 when its delta is positive,
/// or zero with a positive blend amount, then the delete callback 0x4D1390 and +0x2C = 0; +8 = 1, true.
#[hook(addr = 0x62D290, conv = thiscall, name = "CTaskSimpleGangDriveBy::MakeAbortable", expect = "53 55 8B 6C 24 10",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes,ax-al-return")]
pub fn gang_drive_by_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if priority != 1 && priority != 2 {
        wr::<u8>(this, 0xF, 6);
        return 0;
    }
    if !event.is_null() && v_event_type(event) == 9 && (rd::<u8>(event, 0x40) == 0 || rd::<u8>(event, 0xC) == 0) {
        return 0;
    }
    wr::<u8>(this, 0xF, 7);
    if drive_by_start_anim(this, ped) != 0 {
        wr::<u8>(this, 0x10, rd::<u8>(this, 0xF));
        let a = rd::<GPtr<u8>>(this, 0x2C);
        if priority == 2 {
            wr::<u32>(a, 0x1C, 0xC47A0000);
        } else if rd::<u8>(a, 0x2E) & 1 == 0 {
            wr::<u16>(a, 0x2E, rd::<u16>(a, 0x2E) | 1);
        }
    }
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if !pd.is_null() {
        wr::<u32>(pd, 0x34, rd::<u32>(pd, 0x34) | 8);
    }
    if ped_is_player(ped) != 0 {
        wr::<u8>(get_player_info(ped), 0x153, 1);
    }
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if !a.is_null() {
        if rd::<u8>(a, 0x2E) & 1 == 0 {
            let d = x(rd::<f32>(a, 0x1C));
            if gt(d, x(0.0))
                || (!le_or_nan(x(rd::<f32>(a, 0x18)), x(0.0)) && !fcom_c0(x(rd::<f32>(a, 0x1C)), x(0.0)))
            {
                wr::<u32>(a, 0x1C, 0xC1000000);
            }
        }
        anim_set_delete_callback(rd::<GPtr<u8>>(this, 0x2C), 0x4D1390, 0);
        wr::<u32>(this, 0x2C, 0);
    }
    wr::<u8>(this, 8, 1);
    1
}

/// The gang drive-by's anim start (0x627B20, `ret 4`, AL). The anim group's block (+0x28; `ms_aAnimAssocGroups` 0xB4EA34
/// stride 0x14, else `GetAnimationBlock(GetAnimBlockName(group))`) not loaded: `CStreaming::RequestModel(25575 +
/// block index, 8)`, false. Else its reference is taken once (+9). With anim +0x24 0xBF and no anim: false. A new anim
/// (`BlendAnimation(clump, group, +0x24, 4.0)`, delete callback 0x621BE0 with the task) starts when there is none, the
/// timer word +0x16 is not positive and +0xC is set, or when the current anim has another id while the state +0xF is
/// 1..3 and +0x10 below 4; a replaced anim gets the delete callback 0x4D1390 and loses flag bit 0. State 3 with +0x11 at
/// most 1 draws +0x11 = `GetRandomNumberInRange(2, weapon info +0x20)`. After a replacement: the new anim's time 0.2667
/// (0x8D2E7C), its flag bit 0 cleared, +0x10 = 1, +0xF = 0 and, for a non-player, +0x16 = 100; else +0x10 = +0xF, +0xF =
/// 0. True when an anim exists with a blend amount above 0.9 and a blend delta not below 0 (NaN fails both).
#[hook(addr = 0x627B20, conv = thiscall, name = "FUN_00627b20", expect = "8B 15 34 EA B4 00",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes,ax-al-return")]
pub fn gang_drive_by_start_anim(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let grp = rd::<u32>(this, 0x28);
    let mut blk = g::<GPtr<u8>>(g::<u32>(0xB4EA34).read() as usize + grp as usize * 0x14).read();
    if blk.is_null() {
        blk = get_animation_block(anim_get_anim_block_name(grp));
    }
    let idx = (blk.addr() as i32).wrapping_sub(0xB5D4A0) >> 5;
    if rd::<u8>(blk, 0x10) != 1 {
        streaming_request_model(idx + 0x63E7, 8);
        return 0;
    }
    if rd::<u8>(this, 9) == 0 {
        anim_add_block_ref(idx);
        wr::<u8>(this, 9, 1);
    }
    let id = rd::<i32>(this, 0x24);
    if id == 0xBF && rd::<u32>(this, 0x2C) == 0 {
        return 0;
    }
    let cur = rd::<GPtr<u8>>(this, 0x2C);
    let start = if cur.is_null() {
        rd::<i16>(this, 0x16) <= 0 && rd::<u8>(this, 0xC) != 0
    } else {
        i32::from(rd::<i16>(cur, 0x2C)) != id && matches!(rd::<u8>(this, 0xF), 1..=3) && rd::<i8>(this, 0x10) < 4
    };
    if start {
        let mut replaced = false;
        if !cur.is_null() {
            anim_set_delete_callback(cur, 0x4D1390, 0);
            let a = rd::<GPtr<u8>>(this, 0x2C);
            wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFE);
            replaced = true;
        }
        let n = anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), rd::<u32>(this, 0x28), rd::<u32>(this, 0x24), 4.0);
        wr::<GPtr<u8>>(this, 0x2C, n);
        anim_set_delete_callback(n, 0x621BE0, this.addr() as u32);
        if rd::<u8>(this, 0xF) == 3 && rd::<i8>(this, 0x11) <= 1 {
            let hi = i32::from(rd::<i16>(rd::<GPtr<u8>>(this, 0x30), 0x20));
            wr::<u8>(this, 0x11, general_random_int_in_range(2, hi) as u8);
        }
        if replaced {
            assoc_set_current_time(rd::<GPtr<u8>>(this, 0x2C), g::<f32>(0x8D2E7C).read());
            let a = rd::<GPtr<u8>>(this, 0x2C);
            wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFE);
            wr::<u8>(this, 0x10, 1);
            wr::<u8>(this, 0xF, 0);
            if ped_is_player(ped) == 0 {
                wr::<u16>(this, 0x16, 100);
            }
        } else {
            wr::<u8>(this, 0x10, rd::<u8>(this, 0xF));
            wr::<u8>(this, 0xF, 0);
        }
    }
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if a.is_null()
        || le_or_nan(x(rd::<f32>(a, 0x18)), x(g::<f32>(0x858C20).read()))
        || fcom_c0(x(rd::<f32>(a, 0x1C)), x(0.0))
    {
        return 0;
    }
    1
}

/// `CTaskSimpleCreateCarAndGetIn::Destructor` (0x64CEA0): vtable 0x86F070; a created car (+0x18) is cleaned up
/// (`CTheScripts::CleanUpThisVehicle`) and removed from the mission cleanup list (0xA90850) by its pool handle (type
/// 1); then `CTaskSimple`'s destructor.
#[hook(addr = 0x64CEA0, conv = thiscall, name = "CTaskSimpleCreateCarAndGetIn::Destructor", expect = "6A FF 68 78 31 84 00",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn create_car_and_get_in_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86F070);
    let v = rd::<GPtr<u8>>(this, 0x18);
    if !v.is_null() {
        clean_up_this_vehicle(v);
        let h = sa_core::pool::pool_ref(sa_core::pool::VEHICLE_POOL.get(), rd::<GPtr<u8>>(this, 0x18).cast());
        cleanup_remove_entity_from_list(GPtr::from_addr(0xA90850), h, 1);
    }
    crate::bindings::task_simple_dtor(this);
}

// ---------------------------------------------------------------------------------------------------------
// Batch `frontier2-a3`: small direct entries (attractor lookup, office info, jet-pack drop, conversation node).

use sa_world::bindings::create_pickup_coors_close_to_coors;
orig! {
    /// `InteriorGroup_c::FindInteriorInfo(int32 type, InteriorInfo_t**, Interior_c**)` (0x594970, `ret 0xC`, AL)
    fn find_interior_info(g: GPtr<u8>, ty: i32, info: GPtr<u8>, interior: GPtr<u8>) -> u8 = thiscall 0x594970 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
}

use sa_core::bindings::make_upper_case;
use sa_core::crt::rand_at;
use sa_stream::bindings::generate_new_one;

/// `CPedAttractorManager::GetPedUsingEffect<CPedAttractor>(const C2dEffect*, const CEntity*, const
/// std::vector<CPedAttractor*>&)` (0x5EB740, `ret 0xC`; `this` unused): the first attractor of the vector with that
/// effect (+4) and entity (+8); its first ped (+0x20 vector) or null when it has none; null without a match.
#[hook(addr = 0x5EB740, conv = thiscall, name = "CPedAttractorManager::GetPedUsingEffect<CPedAttractor>", expect = "8B 44 24 0C 8B 50 04",
       unverified = "frontier2-a3", route = "L2", hazards = "none")]
pub fn attractor_manager_get_ped_using_effect(_this: GPtr<u8>, effect: u32, entity: u32, vec: GPtr<u8>) -> GPtr<u8> {
    let begin = rd::<u32>(vec, 4);
    let n = if begin == 0 { 0 } else { (rd::<u32>(vec, 8).wrapping_sub(begin) as i32) >> 2 };
    for i in 0..n {
        let a = g::<GPtr<u8>>(begin as usize + i as usize * 4).read();
        if rd::<u32>(a, 4) == effect && rd::<u32>(a, 8) == entity {
            let d = rd::<u32>(a, 0x20);
            if d == 0 || (rd::<u32>(a, 0x24).wrapping_sub(d) as i32) >> 2 == 0 {
                return GPtr::NULL;
            }
            return g::<GPtr<u8>>(d as usize).read();
        }
    }
    GPtr::NULL
}

/// `ftol((rand() & 0xFFFF) * 1/32768 * k)` with the `rand` return address `site`.
fn rand_scaled(site: u32, k: usize) -> i32 {
    let r = rand_at(site) & 0xFFFF;
    ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(k).read()))
}

/// `CTaskInteriorBeInOffice::GetInfoForPedToUse(int32* time)` (0x675300, `ret 4`): repeated until an info is found
/// (+0x14): `FindInteriorInfo(type, &+0x14, &+0x10)` on the interior group (+0xC) with type 7 when the current info's
/// type byte is 6, else 6. Type 6: `*time = -1` when `rand` scaled to 100 (0x675338) exceeds 20, else `5000 + rand
/// scaled to 25000` (0x675368, the constant is -25000); type 7: `5000 + rand scaled to 5000` (0x67539B).
#[hook(addr = 0x675300, conv = thiscall, name = "CTaskInteriorBeInOffice::GetInfoForPedToUse", expect = "53 8B 5C 24 08 55",
       unverified = "frontier2-a3", route = "L2", hazards = "heap-writes")]
pub fn interior_be_in_office_get_info_for_ped_to_use(this: GPtr<u8>, time: GPtr<i32>) {
    loop {
        let cur = rd::<GPtr<u8>>(this, 0x14);
        let ty = if !cur.is_null() && cur.read() == 6 { 7 } else { 6 };
        find_interior_info(rd::<GPtr<u8>>(this, 0xC), ty, this.byte_add(0x14), this.byte_add(0x10));
        if ty == 6 {
            if rand_scaled(0x67533D, 0x858628) > 0x14 {
                time.write(-1);
            } else {
                time.write(0x1388 - rand_scaled(0x67536D, 0x870214));
            }
        } else {
            time.write(0x1388 - rand_scaled(0x6753A0, 0x86E630));
        }
        if rd::<u32>(this, 0x14) != 0 {
            break;
        }
    }
}

/// `CTaskSimpleJetPack::DropJetPack(CPed* ped)` (0x67B660, `ret 4`): unless already dropped (+8), a jet-pack pickup
/// (model 370, type 5, ammo 0, `GenerateNewOne(pos, 370, 5, 0, 0, true, null)`) at `CreatePickupCoorsCloseToCoors` of
/// the ped's position; then the task's own `MakeAbortable(ped, 1, null)` and +0xD / +0xE / +0x10 cleared.
#[hook(addr = 0x67B660, conv = thiscall, name = "CTaskSimpleJetPack::DropJetPack", expect = "83 EC 0C 55 56 8B F1",
       unverified = "frontier2-a3", route = "L2", hazards = "heap-writes")]
pub fn jet_pack_drop_jet_pack(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(this, 8) == 0 {
        let p = sa_world::world::common::entity_pos(ped);
        let mut o = [0u32; 3];
        let op: GPtr<u8> = GPtr::from_raw(o.as_mut_ptr()).cast();
        create_pickup_coors_close_to_coors(rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), op, op.byte_add(4), op.byte_add(8));
        generate_new_one(f32::from_bits(o[0]), f32::from_bits(o[1]), f32::from_bits(o[2]), 0x172, 5, 0, 0, 1, GPtr::NULL);
    }
    v_make_abortable(this, ped, 1, GPtr::NULL);
    wr::<u8>(this, 0xD, 0);
    wr::<u8>(this, 0xE, 0);
    wr::<u32>(this, 0x10, 0);
}

/// `CConversations::IsConversationAtNode(const char* name, CPed* ped)` (0x43B000: `.HOODLUM` stub to 0x156C500, AL):
/// the ped's conversation (`m_aConversations` 0x9691D8, 14 entries of 0x1C, the ped at +8; none: read at null as the
/// original); false while its node (+4) is negative or its status (+0x14) is 1; else the name is upper-cased
/// (`MakeUpperCase`) and compared with the node's name (`m_aNodes` 0x969570, stride 0x18). The result is in AL; the
/// upper EAX bytes follow the original's paths (the node index on the early false, the compare's -1 / 1 / 0).
#[hook(addr = 0x43B000, conv = cdecl, name = "CConversations::IsConversationAtNode", expect = "E9 FB 14 13 01",
       unverified = "frontier2-a3", route = "L2", hazards = "ax-al-return")]
pub fn conversations_is_conversation_at_node(name: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let mut conv = GPtr::<u8>::NULL;
    for i in 0..14usize {
        if g::<GPtr<u8>>(0x9691E0 + i * 0x1C).read() == ped {
            conv = GPtr::from_addr(0x9691D8 + i * 0x1C);
            break;
        }
    }
    let node = rd::<i32>(conv, 4);
    if node < 0 || rd::<i32>(conv, 0x14) == 1 {
        return node as u32 & 0xFFFF_FF00;
    }
    make_upper_case(name);
    let mut a = name;
    let mut b = GPtr::<u8>::from_addr(0x969570 + rd::<i32>(conv, 4) as usize * 0x18);
    loop {
        let (ca, cb) = (a.read(), b.read());
        if ca != cb {
            // sbb eax, eax; sbb eax, -1: -1 below, 1 above
            return if ca < cb { 0xFFFF_FF00 } else { 0 };
        }
        if ca == 0 {
            return 1;
        }
        a = a.byte_add(1);
        b = b.byte_add(1);
    }
}

/// `CTaskSimpleJetPack::ApplyRollAndPitch(CPed* ped)` (0x67B720, `ret 4`): unless the ped's +0x46C bit 0 or 1 is set,
/// both thigh bones (`m_apBones` +0x4A4 / +0x4A8, their +0x10 quaternions) are rotated (`RtQuatRotate`, combine 2) by
/// +0x18 * 57.29578 about the axis at 0x8D2344 and +0x1C * 57.29578 about the axis at 0x8D2338 (each angle stored as
/// f32), and the ped's +0x470 bit 0x4000 set.
#[hook(addr = 0x67B720, conv = thiscall, name = "CTaskSimpleJetPack::ApplyRollAndPitch", expect = "56 8B 74 24 08 F6 86 6C 04 00 00 01",
       unverified = "frontier2-a3", route = "L2", hazards = "heap-writes")]
pub fn jet_pack_apply_roll_and_pitch(this: GPtr<u8>, ped: GPtr<u8>) {
    let f = rd::<u32>(ped, 0x46C);
    if f & 1 != 0 || (f >> 1) & 1 != 0 {
        return;
    }
    let deg = |o: isize| x(rd::<f32>(this, o)).fmul_m(g::<f32>(0x859878).read()).to_f32();
    for bone in [0x4A4isize, 0x4A8] {
        let q = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, bone), 0x10);
        sa_core::bindings::rt_quat_rotate(q, GPtr::from_addr(0x8D2344), deg(0x18), 2);
        sa_core::bindings::rt_quat_rotate(q, GPtr::from_addr(0x8D2338), deg(0x1C), 2);
    }
    wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x4000);
}

/// Phase B registrations of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_WANDER_MEDIC_CTOR];

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_JET_PACK_CTOR,
    HOOK_ARREST_PED_CTOR,
    HOOK_GO_TO_POINT_AIMING_CTOR,
    HOOK_FOLLOW_LEADER_IN_FORMATION_CTOR,
    HOOK_SUNBATHE_CTOR,
    HOOK_SUNBATHE_CAN_SUNBATHE,
    HOOK_ALLOCATOR_ATTACK_ALLOCATE_TASKS,
    HOOK_ON_FIRE_COMPUTE_FIRE_DAMAGE,
    HOOK_TREAT_ACCIDENT_COMPUTE_HEADING,
    HOOK_GANG_DRIVE_BY_MAKE_ABORTABLE,
    HOOK_GANG_DRIVE_BY_START_ANIM,
    HOOK_CREATE_CAR_AND_GET_IN_DTOR,
    HOOK_ATTRACTOR_MANAGER_GET_PED_USING_EFFECT,
    HOOK_INTERIOR_BE_IN_OFFICE_GET_INFO_FOR_PED_TO_USE,
    HOOK_JET_PACK_DROP_JET_PACK,
    HOOK_CONVERSATIONS_IS_CONVERSATION_AT_NODE,
    HOOK_JET_PACK_APPLY_ROLL_AND_PITCH,
];
