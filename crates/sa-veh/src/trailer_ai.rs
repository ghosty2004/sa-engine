//! FRONTIER-2 (batch `frontier2-1`): `CTrailer::ProcessAI` (0x6CF590) and `CTrailer::ScanForTowLink` (0x6CF030), reached
//! by the crossing scenario's trailers. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::bindings::{matrix_attach, matrix_dtor, matrix_mul_vec, matrix_set_translate, matrix_update_rw};
use sa_core::field::gf;
use sa_core::{hook, orig};
use sa_math::fpu::{gt, le_or_nan, lt};
use sa_math::x87::X87;
use sa_pop::bindings::fabsf;
use sa_math::vector::CVector;
use sa_pop::bindings::dm_set_wheel_status;
use sa_world::bindings::{apply_spring_collision_alt, apply_spring_dampening, find_objects_in_range, get_wheel_status, physical_get_speed};

use crate::common::*;
use sa_world::world::common::placeable_heading as heading;
use sa_core::mem::GPtr;

use crate::bindings::{set_component_rotation, automobile_process_suspension, is_playback_going_on_for_car, process_car_wheel_pair};
orig! {
    /// `CAutomobile::PreRender()` (0x6AAB50)
    fn automobile_pre_render(v: GPtr<u8>) = thiscall 0x6AAB50 => unported;
    /// `CAutomobile::ProcessAI(uint32& extraHandlingFlags)` (0x6B4800)
    fn automobile_process_ai(v: GPtr<u8>, flags: GPtr<u8>) -> u8 = thiscall 0x6B4800 => unported;
    /// `CTrailer::ScanForTowLink()` (0x6CF030)
    fn scan_for_tow_link_o(v: GPtr<u8>) = thiscall 0x6CF030 => scan_for_tow_link;
    /// virtual `CVehicle::GetTowHitchPos(CVector&, bool, CVehicle*)` (vtable +0xEC)
    fn v_get_tow_hitch_pos(v: GPtr<u8>, out: GPtr<u8>, b: u32, other: GPtr<u8>) -> u8 = thiscall vslot 0xEC => unported;
    /// virtual `CVehicle::GetTowBarPos(CVector&, bool, CVehicle*)` (vtable +0xF0)
    fn v_get_tow_bar_pos(v: GPtr<u8>, out: GPtr<u8>, b: u32, other: GPtr<u8>) -> u8 = thiscall vslot 0xF0 => unported;
    /// virtual `CVehicle::SetTowLink(CVehicle*, bool)` (vtable +0xF4)
    fn v_set_tow_link(v: GPtr<u8>, tractor: GPtr<u8>, b: u32) -> u8 = thiscall vslot 0xF4 => unported;
}

/// `CTrailer::ProcessAI(uint32& flags)` (0x6CF590, `ret 4`): `CAutomobile::ProcessAI` first (its result is returned).
/// Abandoned (status 4): a trailer with tow ratio (+0x9EC) above 0.5 or exactly -1000 scans for a tow link. Towed
/// (status 10) by +0x4C4: a baggage trailer (-1000) takes the heading difference to its tractor, wrapped by 2pi once
/// (+0x494); another copies the tractor's +0x4A0.
#[hook(addr = 0x6CF590, conv = thiscall, name = "CTrailer::ProcessAI", expect = "8B 44 24 04 53 56 50 8B F1 E8",
       unverified = "frontier2-1", route = "L2", hazards = "x87-transcendental,heap-writes")]
pub fn trailer_process_ai(this: GPtr<u8>, flags: GPtr<u8>) -> u8 {
    let r = automobile_process_ai(this, flags);
    let status = rd::<u8>(this, 0x36) >> 3;
    if status == 4 {
        if !le_or_nan(x(rd::<f32>(this, 0x9EC)), x(gf(0x85_8B8C))) || rd::<u32>(this, 0x9EC) == 0xC47A_0000 {
            scan_for_tow_link_o(this);
        }
        return r;
    }
    let tractor = rd::<GPtr<u8>>(this, 0x4C4);
    if status != 0xA || tractor.is_null() {
        return r;
    }
    if rd::<u32>(this, 0x9EC) != 0xC47A_0000 {
        wr(this, 0x4A0, x(rd::<f32>(tractor, 0x4A0)).to_f32());
        return r;
    }
    let mut a = heading(tractor);
    let b = heading(this);
    let pi = gf(0x85_8CB8);
    if !le_or_nan(a, x(pi).fadd(b)) {
        a = a.fsub_m(gf(0x85_8CBC));
    } else if lt(a, b.fsub_m(pi)) {
        a = a.fadd_m(gf(0x85_8CBC));
    }
    wr(this, 0x494, a.fsub(b).to_f32());
    r
}

/// `CTrailer::ScanForTowLink()` (0x6CF030): untowed, not in a recording playback, not model 0x262, with a tow hitch:
/// among up to 16 vehicles within 10 of the hitch (2D), the first other one whose tow bar is within `[0x8D3470]` in 2D
/// and `[0x8D3474]` in |z| becomes the tractor (`SetTowLink(it, false)`).
#[hook(addr = 0x6CF030, conv = thiscall, name = "CTrailer::ScanForTowLink", expect = "83 EC 68 57 8B F9 8B 87",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes")]
pub fn scan_for_tow_link(this: GPtr<u8>) {
    if rd::<u32>(this, 0x4C4) != 0 || is_playback_going_on_for_car(this) != 0 || rd::<u16>(this, 0x22) == 0x262 {
        return;
    }
    let mut hitch = [0f32; 3];
    let mut bar = [0f32; 3];
    let hp: GPtr<u8> = GPtr::from_raw(hitch.as_mut_ptr()).cast();
    if v_get_tow_hitch_pos(this, hp, 0, this) == 0 {
        return;
    }
    let mut count: u32 = 0;
    let mut found = [0u32; 16];
    find_objects_in_range(hp, 10.0, 1, GPtr::from_raw(&mut count).cast(), 0x10, GPtr::from_raw(found.as_mut_ptr()).cast(),
                          0, 1, 0, 0, 0);
    let n = count as u16 as i16;
    let mut i: i16 = 0;
    while i < n {
        let v = GPtr::<u8>::from_addr(found[i as usize] as usize);
        if v != this && v_get_tow_bar_pos(v, GPtr::from_raw(bar.as_mut_ptr()).cast(), 0, this) != 0 {
            let dx = x(hitch[0]).fsub_m(bar[0]);
            let dy = x(hitch[1]).fsub_m(bar[1]);
            let dz = x(hitch[2]).fsub_m(bar[2]).to_f32();
            let r = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
            if lt(r, x(gf(0x8D_3470))) && lt(X87::from_f64_exact(fabsf(dz)), x(gf(0x8D_3474))) {
                v_set_tow_link(this, v, 0);
                return;
            }
        }
        i += 1;
    }
}

/// `CTrailer::GetTowHitchPos(CVector& out, bool always, CVehicle* tractor)` (0x6CEEA0, `ret 0xC`): never for a
/// tractor of model 0x20D. The model's hitch dummy (vehicle structure +0x6C) when not all zero, else (only when
/// `always`) `(0, colModel.max.y + 1, 0.5 - +0x898)`; transformed by the trailer's matrix; true.
#[hook(addr = 0x6CEEA0, conv = thiscall, name = "CTrailer::GetTowHitchPos", expect = "0F BF 41 22 8B 14 85 C8 B0 A9 00",
       unverified = "frontier2-1", route = "L2", hazards = "none")]
pub fn trailer_get_tow_hitch_pos(this: GPtr<u8>, out: GPtr<u8>, always: u32, tractor: GPtr<u8>) -> u8 {
    let mi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * rd::<i16>(this, 0x22) as usize).read();
    let d = rd::<GPtr<u8>>(mi, 0x5C).byte_add(0x6C);
    for k in 0..3isize {
        wr(out, 4 * k, rd::<u32>(d, 4 * k));
    }
    if !tractor.is_null() && rd::<u16>(tractor, 0x22) == 0x20D {
        return 0;
    }
    let zero = x(gf(0x85_8B50));
    let nonzero = (0..3isize).any(|k| x(rd::<f32>(out, 4 * k)).fcom(zero).test_parity(0x44));
    if !nonzero {
        if always as u8 == 0 {
            return 0;
        }
        wr::<u32>(out, 0, 0);
        wr(out, 4, x(rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x10)).fadd_m(gf(0x85_8624)).to_f32());
        wr(out, 8, x(gf(0x85_8B8C)).fsub_m(rd::<f32>(this, 0x898)).to_f32());
    }
    let mut t = [0u32; 3];
    let r = matrix_mul_vec(GPtr::from_raw(t.as_mut_ptr()).cast(), rd::<GPtr<u8>>(this, 0x14), out);
    for k in 0..3isize {
        wr(out, 4 * k, rd::<u32>(r, 4 * k));
    }
    1
}

/// `-up` of the trailer's matrix (`up * [0x858C1C]`, z first as the original loads it).
fn down(this: GPtr<u8>) -> [f32; 3] {
    let m = rd::<GPtr<u8>>(this, 0x14);
    let k = gf(0x85_8C1C);
    let z = x(rd::<f32>(m, 0x28)).fmul_m(k);
    let y = x(rd::<f32>(m, 0x24)).fmul_m(k);
    let xx = x(rd::<f32>(m, 0x20)).fmul_m(k);
    [xx.to_f32(), y.to_f32(), z.to_f32()]
}

/// `CTrailer::ProcessSuspension()` (0x6CF6A0).
///
/// `CAutomobile::ProcessSuspension` first; a baggage trailer (+0x9EC == -1000) stops there. For each of the two
/// supports whose ratio (+0x9E0) is below the towed ratio (+0x9F0) while that is above `[0x858B1C]`: a spring
/// (`ApplySpringCollisionAlt([0x8D3464], -up, contact - pos, ratio / towed, 1, normal, impulse)`; the colpoints at
/// +0x988, 0x2C each), then the contact speeds (`GetSpeed`), then the dampening (`[0x8D3468]`, along `-normal` when
/// the normal's z is above `[0x858F9C]`, else `-up`). Not towed (status 10 / 11): both wheels' statuses are saved,
/// set to 0 (contact, its speed again) or 2, `ProcessCarWheelPair(0, 1, 0, speeds, points, [0xC1CB28] * traction
/// (handling +0x28) * [0x858C84], 0, 1000, true)`, and restored.
#[hook(addr = 0x6CF6A0, conv = thiscall, name = "CTrailer::ProcessSuspension", expect = "83 EC 5C 53 55 56 57 8B",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes")]
pub fn trailer_process_suspension(this: GPtr<u8>) {
    automobile_process_suspension(this);
    if rd::<u32>(this, 0x9EC) == 0xC47A_0000 {
        return;
    }
    let mut impulse = [0f32; 2];
    let mut points = [[0f32; 3]; 2];
    let mut speeds = [[0f32; 3]; 2];
    let active = |i: usize| {
        let r2 = x(rd::<f32>(this, 0x9F0));
        lt(x(rd::<f32>(this, 0x9E0 + 4 * i as isize)), r2) && !le_or_nan(r2, x(gf(0x85_8B1C)))
    };
    let pos = |this: GPtr<u8>| {
        let m = rd::<GPtr<u8>>(this, 0x14);
        if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) }
    };
    for i in 0..2usize {
        if !active(i) {
            continue;
        }
        let comp = x(rd::<f32>(this, 0x9E0 + 4 * i as isize)).fdiv_m(rd::<f32>(this, 0x9F0)).to_f32();
        let mut d = down(this);
        let cp = this.byte_add(0x988 + 0x2C * i as isize);
        let p = pos(this);
        points[i] = [
            x(rd::<f32>(cp, 0)).fsub_m(rd::<f32>(p, 0)).to_f32(),
            x(rd::<f32>(cp, 4)).fsub_m(rd::<f32>(p, 4)).to_f32(),
            x(rd::<f32>(cp, 8)).fsub_m(rd::<f32>(p, 8)).to_f32(),
        ];
        apply_spring_collision_alt(this, gf(0x8D_3464), GPtr::from_raw(d.as_mut_ptr()).cast(),
                                   GPtr::from_raw(points[i].as_mut_ptr()).cast(), comp, 1.0, cp.byte_add(0x10),
                                   GPtr::from_raw(&mut impulse[i]));
    }
    let speed_at = |pt: [f32; 3]| {
        let mut out = CVector::default();
        let r: GPtr<u8> = physical_get_speed(this, GPtr::from_raw(&mut out), CVector { x: pt[0], y: pt[1], z: pt[2] }).cast();
        [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)]
    };
    for i in 0..2usize {
        if active(i) {
            speeds[i] = speed_at(points[i]);
        }
    }
    for i in 0..2usize {
        if !active(i) {
            continue;
        }
        let mut d = down(this);
        let n = this.byte_add(0x998 + 0x2C * i as isize);
        if !le_or_nan(x(rd::<f32>(n, 8)), x(gf(0x85_8F9C))) {
            d = [
                x(rd::<f32>(n, 0)).fchs().to_f32(),
                x(rd::<f32>(n, 4)).fchs().to_f32(),
                x(rd::<f32>(n, 8)).fchs().to_f32(),
            ];
        }
        apply_spring_dampening(this, gf(0x8D_3468), impulse[i], GPtr::from_raw(d.as_mut_ptr()).cast(),
                               GPtr::from_raw(points[i].as_mut_ptr()).cast(), GPtr::from_raw(speeds[i].as_mut_ptr()).cast());
    }
    let status = rd::<u8>(this, 0x36) >> 3;
    if status == 0xA || status == 0xB {
        return;
    }
    let dm = this.byte_add(0x5A0);
    let mut saved = [0u8; 2];
    for i in 0..2usize {
        saved[i] = get_wheel_status(dm, i as u32) as u8;
        if active(i) {
            speeds[i] = speed_at(points[i]);
            dm_set_wheel_status(dm, i as i32, 0);
        } else {
            dm_set_wheel_status(dm, i as i32, 2);
        }
    }
    let traction = x(gf(0xC1_CB28)).fmul_m(rd::<f32>(rd::<GPtr<u8>>(this, 0x384), 0x28)).fmul_m(gf(0x85_8C84)).to_f32();
    process_car_wheel_pair(this, 0, 1, 0, GPtr::from_raw(speeds.as_mut_ptr()).cast(), GPtr::from_raw(points.as_mut_ptr()).cast(),
                           traction.to_bits(), 0, 0x447A_0000, 1);
    dm_set_wheel_status(dm, 0, saved[0]);
    dm_set_wheel_status(dm, 1, saved[1]);
}

/// `CTrailer::PreRender()` (0x6CFAC0).
///
/// `CAutomobile::PreRender` first. With the support node (+0x698), more than four col lines and the trailer moving
/// (any move / turn speed component non-zero) or not on fake physics (+0xB8): the node's matrix is attached and its z
/// set to `min(((s * (end.z - start.z)) + start.z) * vr - (1 - vr) * h, +0x9E8)` of support line 4, with `s =
/// min((ratio1 + ratio0) * 0.5, +0x9EC)`, `vr = pos.y / start.y`, `h` = +0x898; `UpdateRW`. The farm trailer (0x262)
/// turns its propeller (+0x958, less 2pi once when above 2pi) by `clamp(moveSpeed . forward, 0, 0.1) * timeStep`
/// (`SetComponentRotation(+0x69C, x, angle, true)`). The temporary matrix is destroyed.
#[hook(addr = 0x6CFAC0, conv = thiscall, name = "CTrailer::PreRender", expect = "6A FF 68 78 86 84 00 64",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes", note = "the SEH frame is not reproduced")]
pub fn trailer_pre_render(this: GPtr<u8>) {
    automobile_pre_render(this);
    let mi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * rd::<i16>(this, 0x22) as usize).read();
    let cd = rd::<GPtr<u8>>(rd::<GPtr<u8>>(mi, 0x14), 0x2C);
    let mut mat = [0u32; 18];
    let mp: GPtr<u8> = GPtr::from_raw(mat.as_mut_ptr()).cast();
    let node = rd::<GPtr<u8>>(this, 0x698);
    if !node.is_null() && rd::<i8>(cd, 6) > 4 {
        let zero = x(gf(0x85_8B50));
        let moving = [0x44isize, 0x48, 0x4C, 0x50, 0x54, 0x58].iter().any(|&o| x(rd::<f32>(this, o)).fcom(zero).test_parity(0x44));
        if moving || rd::<u8>(this, 0xB8) == 0 {
            matrix_attach(mp, node.byte_add(0x10), 0);
            let mut s = x(rd::<f32>(this, 0x9E4)).fadd_m(rd::<f32>(this, 0x9E0)).fmul_m(gf(0x85_8B8C));
            let (px, py) = (rd::<f32>(mp, 0x30), rd::<f32>(mp, 0x34));
            if !le_or_nan(s, x(rd::<f32>(this, 0x9EC))) {
                s = x(rd::<f32>(this, 0x9EC));
            }
            let l = rd::<GPtr<u8>>(cd, 0x10);
            let vr = x(py).fdiv_m(rd::<f32>(l, 0x84));
            let sd = x(rd::<f32>(l, 0x98)).fsub_m(rd::<f32>(l, 0x88)).fmul(s);
            let t = sd.fadd_m(rd::<f32>(l, 0x88)).fmul(vr).fsub(x(gf(0x85_8624)).fsub(vr).fmul_m(rd::<f32>(this, 0x898)));
            let mut z = t.to_f32().to_bits();
            if !le_or_nan(t, x(rd::<f32>(this, 0x9E8))) {
                z = rd::<u32>(this, 0x9E8);
            }
            matrix_set_translate(mp, px, py, f32::from_bits(z));
            matrix_update_rw(mp);
        }
    }
    if rd::<u16>(this, 0x22) == 0x262 {
        let two_pi = gf(0x85_8CBC);
        if !le_or_nan(x(rd::<f32>(this, 0x958)), x(two_pi)) {
            wr(this, 0x958, x(rd::<f32>(this, 0x958)).fsub_m(two_pi).to_f32());
        }
        let m = rd::<GPtr<u8>>(this, 0x14);
        let dot = || {
            x(rd::<f32>(this, 0x4C))
                .fmul_m(rd::<f32>(m, 0x18))
                .fadd(x(rd::<f32>(this, 0x48)).fmul_m(rd::<f32>(m, 0x14)))
                .fadd(x(rd::<f32>(this, 0x44)).fmul_m(rd::<f32>(m, 0x10)))
        };
        let k = x(gf(0x85_8B1C));
        let zero = x(gf(0x85_8B50));
        let v = if !gt(dot(), k) && !zero.fcom(dot()).test_any(0x41) {
            zero
        } else if gt(dot(), k) {
            k
        } else {
            dot()
        };
        let a = v.fmul_m(gf(0xB7_CB5C)).fsubr_m(rd::<f32>(this, 0x958)).to_f32();
        wr(this, 0x958, a);
        set_component_rotation(this, rd::<GPtr<u8>>(this, 0x69C), 0, a, 1);
    }
    matrix_dtor(mp);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_TRAILER_PRE_RENDER,
    HOOK_TRAILER_PROCESS_AI,
    HOOK_SCAN_FOR_TOW_LINK,
    HOOK_TRAILER_GET_TOW_HITCH_POS,
    HOOK_TRAILER_PROCESS_SUSPENSION,
];
