//! WRITER-CLIMB (batch `climb-3`): `CTaskSimpleClimb::ProcessPed` (0x680DC0), the per-frame climb: the ped is
//! pulled to the handhold offset of its height, the anim heights advance on the pad's jump input, and the task
//! finishes (stand-up / vault) or aborts. Ported from the 1.0 US bytes; gta-reversed (`TaskSimpleClimb.cpp`) as a
//! reading aid (its frame-rate speed fix is NOTSA and not ported). x87 arithmetic follows the original's
//! instruction order.
//!
//! Task fields: +8 finished, +9 change anim, +0xA change position, +0xB force climb, +0xC invalid climb, +0xD
//! height for the anim, +0xE height for the position, +0x14 handhold heading, +0x18 handhold position, +0x24
//! climb entity, +0x28 get-to-position counter (word), +0x2C anim.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::MakeAbortable(CPed*, eAbortPriority, const CEvent*)` (vtable slot 0x18)
    fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskSimpleClimb::StartAnim(CPed*)` (0x67DBE0)
    fn climb_start_anim_c(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x67DBE0 => crate::climb_anim::climb_start_anim;
    /// `CTaskSimpleClimb::StartSpeech(CPed*)` (0x67A320)
    fn climb_start_speech(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x67A320 => unported;
    /// `CTaskSimpleClimb::TestForVault(CPed*, const CVector&, float)` (0x6804D0)
    fn climb_test_for_vault_c(t: GPtr<u8>, ped: GPtr<u8>, p: GPtr<u8>, a: f32) -> u8 = thiscall 0x6804D0 => crate::climb_scan::climb_test_for_vault;
    /// `CTaskSimpleClimb::TestForStandUp(CPed*, const CVector&, float)` (0x680570)
    fn climb_test_for_stand_up_c(t: GPtr<u8>, ped: GPtr<u8>, p: GPtr<u8>, a: f32) -> u8 = thiscall 0x680570 => crate::climb_scan::climb_test_for_stand_up;
    /// `CMatrix * CVector` (0x59C890, `(CVector* out, const CMatrix*, const CVector*)`)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, hidden result first)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CPhysical::GetSpeed(CVector point)` (0x542CE0, hidden result first, `ret 0x10`)
    fn physical_get_speed(p: GPtr<u8>, out: GPtr<u8>, x: u32, y: u32, z: u32) -> GPtr<u8> = thiscall 0x542CE0 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn placeable_set_posn(p: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4241C0 => unported;
    /// `operator/(const CVector&, float)` (0x4119D0, hidden result first)
    fn vector_div_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x4119D0 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00)
    fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) = thiscall 0x411A00 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CVector::operator*=(float)` (0x40FEF0)
    fn vector_scale_assign(v: GPtr<u8>, s: f32) = thiscall 0x40FEF0 => unported;
    /// `operator*(float, const CVector&)` (0x40FE90, hidden result first)
    fn scalar_mul_vector(out: GPtr<u8>, s: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn physical_apply_move_force(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x5429F0 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn player_get_pad(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::GetJump()` (0x540750), `JumpJustDown()` (0x540770), `GetExitVehicle()` (0x5400D0)
    fn pad_get_jump(pad: GPtr<u8>) -> u8 = thiscall 0x540750 => unported;
    fn pad_jump_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540770 => unported;
    fn pad_get_exit_vehicle(pad: GPtr<u8>) -> u8 = thiscall 0x5400D0 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80)
    fn placeable_set_orientation(e: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x439A80 => unported;
    /// `CEventSoundQuiet(CEntity*, float, uint32, CVector)` (0x5E05B0)
    fn event_sound_quiet_ctor(e: GPtr<u8>, ent: GPtr<u8>, volume: f32, time: u32, pos: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E05B0 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventSoundQuiet::~CEventSoundQuiet()` body (0x5DEA00)
    fn event_sound_quiet_dtor(e: GPtr<u8>) = thiscall 0x5DEA00 => unported;
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C)
const TIME_STEP: usize = 0xB7_CB5C;

fn c(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn buf(v: &mut [u32]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for k in 0..3 {
        wr::<u32>(dst, 4 * k, rd::<u32>(src, 4 * k));
    }
}

/// `blend == 1.0` as `test ah, 0x44; jp`: equal only.
fn blend_is_one(a: GPtr<u8>) -> bool {
    !x(rd::<f32>(a, 0x18)).fcom(x(c(0x85_8624))).test_parity(0x44)
}

/// Clears the ped's ignored-collision entity (+0x128) when it is the climb entity.
fn release_ignored(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u32>(ped, 0x128) == rd::<u32>(this, 0x24) {
        wr::<u32>(ped, 0x128, 0);
    }
}

fn make_abortable(this: GPtr<u8>, ped: GPtr<u8>) {
    task_make_abortable(this, ped, 1, 0);
}

/// `bool CTaskSimpleClimb::ProcessPed(CPed* ped)` (0x680DC0, `ret 4`, SEH frame).
///
/// Finished (+8): releases the ped's ignored-collision entity, true. No climb entity, an object that is neither
/// static (+0x1C bit 2 / bit 18) nor collision-force free (+0x40 bit 2), or a train (+0x594 == 6) faster than
/// 0.1: `MakeAbortable(ped, 1, null)`, true. A vehicle becomes the ped's ignored-collision entity. Without an anim
/// or on +9: `StartAnim`, `StartSpeech`. The handhold (a physical entity's local position and heading through its
/// matrix, allocated when missing) and, once the anim with +0xA reaches blend 1, the position height takes the anim
/// height. With a position height (1 / 2 hanging, 3 / 5 at the edge, 4 stand-up, 6 vault offsets) the target is
/// `hand + (-sin(a) h, cos(a) h, v)`; a moving physical entity contributes `GetSpeed(target - its position)`. A new
/// height snaps the ped there with that speed; otherwise the ped is pulled (`|d|^2 < 0.1`: `d / step`; else `d /
/// 4 / step` capped at 0.2) plus the entity speed: a paused stand anim at height 3 vaults (5) when `TestForVault`
/// passes, else plays; a slow pull past 1000 ms (500 on the idle anim) marks the climb invalid (+0xC), aborts and
/// pushes the ped back (`forward * mass * -0.1`). A new height 4 / 6 finishes: in-air / landing flags cleared,
/// speed `forward * 0.05 - (0, 0, 0.01)` standing (4) or `0.02 forward - (0, 0, 0.05)` not standing (6), the
/// player's quiet sound event (50), the anim fading at -8, true. At blend 1 the anim heights advance by anim id
/// (jump: at its end on force / jump, restarting the anim; idle: force, no pad or jump; pull / stand: at the end;
/// jump-b: at its end; others: at the end on jump-just-down or force). Exit-vehicle aborts; at heights 3 / 5 with
/// the anim playing a failed `TestForStandUp` aborts; else anim height 3 below position height 3 vaults when
/// `TestForVault` passes. Rotation +0x558 / +0x55C and the orientation take the handhold heading; false.
#[hook(addr = 0x680DC0, conv = thiscall, name = "CTaskSimpleClimb::ProcessPed",
       expect = "6A FF 64 A1 00 00 00 00",
       unverified = "climb-3", route = "L2", hazards = "x87-transcendental,heap-writes,ax-al-return,uninit-locals",
       recs = "grove-walk",
       note = "fsin / fcos / fpatan; a position height outside 1..6 reads uninitialised offsets (0 here)")]
pub fn climb_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 {
        release_ignored(this, ped);
        return 1;
    }
    let e = rd::<GPtr<u8>>(this, 0x24);
    if e.is_null() {
        make_abortable(this, ped);
        return 1;
    }
    let ty = rd::<u8>(e, 0x36) & 7;
    if ty == 4 {
        let f = rd::<u32>(e, 0x1C);
        if f & 4 == 0 && f & 0x4_0000 == 0 && rd::<u8>(e, 0x40) & 4 == 0 {
            make_abortable(this, ped);
            return 1;
        }
    }
    if ty == 2 && rd::<u32>(e, 0x594) == 6 {
        let (vx, vy, vz) = (x(rd::<f32>(e, 0x44)), x(rd::<f32>(e, 0x48)), x(rd::<f32>(e, 0x4C)));
        let m = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt();
        if !m.fcom(x(c(0x85_8B1C))).test_any(0x41) {
            make_abortable(this, ped);
            return 1;
        }
    }
    if ty == 2 {
        wr::<GPtr<u8>>(ped, 0x128, e);
    }
    if rd::<u32>(this, 0x2C) == 0 || rd::<u8>(this, 9) != 0 {
        climb_start_anim_c(this, ped);
        climb_start_speech(this, ped);
    }
    // the handhold
    let mut heading = rd::<f32>(this, 0x14);
    let mut hand = [rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C), rd::<u32>(this, 0x20)];
    let e = rd::<GPtr<u8>>(this, 0x24);
    let ety = rd::<u8>(e, 0x36) & 7;
    if ety > 1 && ety < 5 {
        if rd::<u32>(e, 0x14) == 0 {
            placeable_allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let mut out = [0u32; 3];
        let r = matrix_mul_vec(buf(&mut out), rd::<GPtr<u8>>(e, 0x14), buf(&mut hand));
        hand = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
        let e = rd::<GPtr<u8>>(this, 0x24);
        let m = rd::<GPtr<u8>>(e, 0x14);
        let h = if m.is_null() {
            x(rd::<f32>(e, 0x10))
        } else {
            x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
        };
        heading = h.fadd_m(heading).to_f32();
    }
    let hand_p = buf(&mut hand);
    let mut new_height = false;
    if rd::<u8>(this, 0xA) != 0 {
        let a = rd::<GPtr<u8>>(this, 0x2C);
        if !a.is_null() && blend_is_one(a) {
            new_height = true;
            wr::<u8>(this, 0xA, 0);
            wr::<u8>(this, 0xE, rd::<u8>(this, 0xD));
        }
    }
    let hp = rd::<i8>(this, 0xE);
    if hp > 0 {
        let (horz, vert) = match hp {
            1 | 2 => (c(0x8D_2F1C), c(0x8D_2F20)),
            3 | 5 => (c(0x8D_2F24), c(0xC1_8F78)),
            4 => (c(0x8D_2F28), c(0x8D_2F2C)),
            6 => (c(0x8D_2F30), c(0xC1_8F7C)),
            _ => (0.0, 0.0),
        };
        let (hx, hy, hz) = (f32::from_bits(hand[0]), f32::from_bits(hand[1]), f32::from_bits(hand[2]));
        let mut target = [
            x(hx).fsub(x(heading).fsin().fmul_m(horz)).to_f32(),
            x(heading).fcos().fmul_m(horz).fadd_m(hy).to_f32(),
            x(hz).fadd(x(vert)).to_f32(),
        ];
        let target_p: GPtr<u8> = GPtr::from_raw(target.as_mut_ptr()).cast();
        let pm = rd::<GPtr<u8>>(ped, 0x14);
        let pp = if pm.is_null() { ped.byte_add(4) } else { pm.byte_add(0x30) };
        let mut speed = [0u32; 3];
        let dz = x(target[2]).fsub_m(rd::<f32>(pp, 8)).to_f32();
        let dy = x(target[1]).fsub_m(rd::<f32>(pp, 4)).to_f32();
        let dx = x(target[0]).fsub_m(rd::<f32>(pp, 0)).to_f32();
        let mut d = [dx.to_bits(), dy.to_bits(), dz.to_bits()];
        let sq = x(dx).fmul_m(dx).fadd(x(dy).fmul_m(dy)).fadd(x(dz).fmul_m(dz)).to_f32();
        let ef = rd::<u32>(e, 0x1C);
        if ef & 4 == 0 && ef & 0x4_0000 == 0 && ety > 1 && ety < 5 {
            let em = rd::<GPtr<u8>>(e, 0x14);
            let ep = if em.is_null() { e.byte_add(4) } else { em.byte_add(0x30) };
            let mut rel = [0u32; 3];
            let r = vector_sub(buf(&mut rel), target_p, ep);
            let (rx, ry, rz) = (rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8));
            let mut out = [0u32; 3];
            let s = physical_get_speed(e, buf(&mut out), rx, ry, rz);
            speed = [rd::<u32>(s, 0), rd::<u32>(s, 4), rd::<u32>(s, 8)];
        }
        let move_speed = ped.byte_add(0x44);
        if new_height {
            placeable_set_posn(ped, target_p);
            copy3(move_speed, buf(&mut speed));
        } else if !x(sq).fcom(x(c(0x85_8B1C))).test_parity(0x05) {
            // close: settle at the target
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & !1);
            let mut out = [0u32; 3];
            let r = vector_div_scalar(buf(&mut out), buf(&mut d), c(TIME_STEP));
            copy3(move_speed, r);
            vector_add_assign(move_speed, buf(&mut speed));
            let a = rd::<GPtr<u8>>(this, 0x2C);
            if rd::<u8>(a, 0x2E) & 1 == 0 && rd::<u8>(this, 0xD) == 3 {
                if climb_test_for_vault_c(this, ped, hand_p, heading) != 0 {
                    wr::<u8>(this, 0xD, 5);
                    wr::<u8>(this, 9, 1);
                } else {
                    let a = rd::<GPtr<u8>>(this, 0x2C);
                    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
                }
            }
        } else {
            // far: a quarter of the way per step, capped at 0.2
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & !1);
            let q = c(0x85_8C84);
            let mut dq = [
                x(dx).fmul_m(q).to_f32().to_bits(),
                x(dy).fmul_m(q).to_f32().to_bits(),
                x(dz).fmul_m(q).to_f32().to_bits(),
            ];
            let mut out = [0u32; 3];
            let r = vector_div_scalar(buf(&mut out), buf(&mut dq), c(TIME_STEP));
            copy3(move_speed, r);
            let mag = X87::from_f64_exact(vector_magnitude(move_speed));
            let cap = c(0x85_8CC4);
            if !mag.fcom(x(cap)).test_any(0x41) {
                vector_scale_assign(move_speed, x(cap).fdiv(mag).to_f32());
            }
            vector_add_assign(move_speed, buf(&mut speed));
            let a = rd::<GPtr<u8>>(this, 0x2C);
            if rd::<u8>(a, 0x2E) & 1 == 0 || rd::<u16>(a, 0x2C) == 0x81 {
                let step = c(TIME_STEP);
                let ms = ftol32(x(step).fmul_m(c(0x85_8B38)).fmul_m(c(0x85_8C4C))) as u16;
                let n = rd::<u16>(this, 0x28).wrapping_add(ms);
                wr::<u16>(this, 0x28, n);
                let n = n as i16;
                if n > 1000 || (rd::<u16>(a, 0x2C) == 0x81 && n > 500) {
                    wr::<u8>(this, 0xC, 1);
                    task_make_abortable(this, ped, 1, 0);
                    let f = x(rd::<f32>(ped, 0x8C)).fmul_m(c(0x85_8EF4)).to_f32();
                    let mut out = [0u32; 3];
                    let r = scalar_mul_vector(buf(&mut out), f, rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10));
                    physical_apply_move_force(ped, rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8));
                }
            }
        }
    }
    if new_height && matches!(rd::<u8>(this, 0xE), 4 | 6) {
        wr::<u8>(this, 8, 1);
        let mut flags = rd::<u32>(ped, 0x46C) & 0xFFFF_F9FF;
        wr::<u32>(ped, 0x46C, flags);
        let fwd = rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10);
        let ms = ped.byte_add(0x44);
        if rd::<u8>(this, 0xE) == 6 {
            let mut out = [0u32; 3];
            let r = scalar_mul_vector(buf(&mut out), c(0x8D_2F90), fwd);
            let z = x(c(0x8D_2F8C)).fadd_m(rd::<f32>(r, 8)).to_f32();
            let (rx, ry) = (rd::<u32>(r, 0), rd::<u32>(r, 4));
            wr::<u32>(ms, 0, rx);
            wr::<u32>(ms, 4, ry);
            wr::<f32>(ms, 8, z);
            flags = (flags & !1) | 2;
        } else {
            let k = c(0x85_8C28);
            let z = x(rd::<f32>(fwd, 8)).fmul_m(k).fsub_m(c(0x85_8C58)).to_f32();
            let y = x(rd::<f32>(fwd, 4)).fmul_m(k).to_f32();
            let xx = x(rd::<f32>(fwd, 0)).fmul_m(k).to_f32();
            flags |= 3;
            wr::<f32>(ms, 0, xx);
            wr::<f32>(ms, 4, y);
            wr::<f32>(ms, 8, z);
        }
        wr::<u32>(ped, 0x46C, flags);
        if ped_is_player(ped) != 0 {
            let mut pos = [0u32; 3];
            let mut ev = [0u32; 16];
            let evp = buf(&mut ev);
            event_sound_quiet_ctor(evp, ped, 50.0, 0xFFFF_FFFF, buf(&mut pos));
            event_group_add(get_event_global_group(), evp, 0);
            event_sound_quiet_dtor(evp);
        }
        let a = rd::<GPtr<u8>>(this, 0x2C);
        if !a.is_null() {
            wr::<u32>(a, 0x1C, 0xC100_0000);
        }
        release_ignored(this, ped);
        return 1;
    }
    let pad = if ped_is_player(ped) != 0 { player_get_pad(ped) } else { GPtr::NULL };
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if !a.is_null() && blend_is_one(a) {
        let total = || x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10));
        let at_end = || !total().fcom(x(rd::<f32>(a, 0x20))).test_parity(0x44);
        let past_end = || !total().fcom(x(rd::<f32>(a, 0x28)).fadd_m(rd::<f32>(a, 0x20))).test_parity(0x41);
        let advance = || {
            wr::<u8>(this, 0xD, rd::<u8>(this, 0xD).wrapping_add(1));
            wr::<u8>(this, 9, 1);
        };
        match i32::from(rd::<i16>(a, 0x2C)) - 0x80 {
            0 => {
                if past_end() {
                    if rd::<u8>(this, 0xB) != 0 || (!pad.is_null() && pad_get_jump(pad) != 0) {
                        advance();
                    }
                    climb_start_anim_c(this, ped);
                }
            }
            1 => {
                if rd::<u8>(this, 0xB) != 0 || pad.is_null() || pad_get_jump(pad) != 0 {
                    advance();
                }
            }
            2 | 3 => {
                if at_end() {
                    advance();
                }
            }
            5 => {
                if past_end() {
                    advance();
                }
            }
            _ => {
                if at_end() && ((!pad.is_null() && pad_jump_just_down(pad) != 0) || rd::<u8>(this, 0xB) != 0) {
                    advance();
                }
            }
        }
    }
    if !pad.is_null() && pad_get_exit_vehicle(pad) != 0 {
        make_abortable(this, ped);
    } else {
        let hp = rd::<i8>(this, 0xE);
        let a = rd::<GPtr<u8>>(this, 0x2C);
        if (hp == 3 || hp == 5) && !a.is_null() && rd::<u8>(a, 0x2E) & 1 != 0 {
            if climb_test_for_stand_up_c(this, ped, hand_p, heading) == 0 {
                make_abortable(this, ped);
            }
        } else if rd::<u8>(this, 0xD) == 3 && hp < 3 && climb_test_for_vault_c(this, ped, hand_p, heading) != 0 {
            wr::<u8>(this, 0xD, 5);
        }
    }
    wr::<f32>(ped, 0x55C, heading);
    wr::<f32>(ped, 0x558, heading);
    placeable_set_orientation(ped, 0.0, 0.0, heading);
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_CLIMB_PROCESS_PED];
