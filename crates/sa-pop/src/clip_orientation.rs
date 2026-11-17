//! `CCarCtrl::ClitargetOrientationToLink` (0x422760, batch `frontier2-a16`), ported from the 1.0 US bytes
//! (gta-reversed only calls the original).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::fpu::st0;
use sa_math::x87::X87;

use crate::common::{g, get_atan_of_xy, rd, x, xi};
use sa_world::world::common::entity_pos;

/// `ThePaths.m_pNaviNodes[64]` (0x96F854) and the navi links per area (0x96F974).
const NAVI_NODES: usize = 0x96_F854;
const NAVI_LINKS: usize = 0x96_F974;
/// 0x858C58 0.01, 0x858B8C 0.5, 0x858EEC 1/86.4, 0x858C24 0.3, 0x858C50 5.4, 0x858B50 0, 0x858CB8 pi, 0x858CBC 2pi,
/// 0x858CC0 -pi
const F_0_01: f32 = f32::from_bits(0x3C23_D70A);
const F_0_5: f32 = 0.5;
const F_WIDTH: f32 = f32::from_bits(0x3C3D_A12F);
const F_0_3: f32 = f32::from_bits(0x3E99_999A);
const F_5_4: f32 = f32::from_bits(0x40AC_CCCD);
const F_PI: f32 = f32::from_bits(0x4049_0FDB);
const F_2PI: f32 = f32::from_bits(0x40C9_0FDB);
const F_MPI: f32 = f32::from_bits(0xC049_0FDB);

/// `fcom m; test ah, 0x41; jne`: `v <= m` (or unordered).
fn le(v: X87, m: f32) -> bool {
    v.fcom(x(m)).test_any(0x41)
}

/// `fcom m; test ah, 5; jp`: `v >= m` (or unordered).
fn ge(v: X87, m: f32) -> bool {
    v.fcom(x(m)).test_parity(0x05)
}

/// The angle to `(px, py)` from the vehicle's position (`GetATanOfXY(px - pos.x, py - pos.y)`, ST0).
fn angle_from(veh: GPtr<u8>, px: f32, py: X87) -> X87 {
    let pos = entity_pos(veh);
    let dy = py.fsub_m(rd::<f32>(pos, 4)).to_f32();
    let dx = x(px).fsub_m(rd::<f32>(pos, 0)).to_f32();
    st0(get_atan_of_xy(dx, dy))
}

/// `CCarCtrl::ClitargetOrientationToLink(CVehicle*, CCarPathLinkAddress link, int8 dir, float* orientation, float x,
/// float y)` (0x422760, cdecl). Nothing when the link's navi area is not loaded. The link's direction (+8 / +9,
/// int8 * 0.01 * dir) gives the two lane edges: with lanes on one side only, both at `lanes * 0.5`; otherwise the
/// `dir`'s own lanes (dir 0: left (+0xB bits 0-2), else right (bits 3-5)) plus `width (+0xA) / 86.4`, and the other
/// side's lane count; each minus 0.3 and scaled by 5.4 across the direction from `(x, y)`. The angles to both edge
/// points minus the orientation, each wrapped into [-pi, pi] (the first stored as f32, the second kept extended):
/// when both have the same sign the one closer to 0 is added to the orientation, which is then wrapped into
/// [0, 2pi]. The original also computes (and drops) the angle to `(x, y)` first.
#[hook(addr = 0x422760, conv = cdecl, name = "CCarCtrl::ClitargetOrientationToLink",
       expect = "E9 04 F7 FD FF",
       unverified = "frontier2-a16", route = "L2", hazards = "x87-transcendental,stack-residue",
       note = "entry jmp 0x401E69 (chunk = movzx eax, word [esp+8]; jmp 0x422765), relocated whole; the original stores locals over its link argument slot")]
pub fn clip_target_orientation_to_link(veh: GPtr<u8>, link: u32, dir: u32, orient: GPtr<f32>, px: f32, py: f32) {
    let link = link & 0xFFFF;
    let area = (link >> 10) as usize;
    if g::<u32>(NAVI_NODES + 4 * area).read() == 0 {
        return;
    }
    let nl = GPtr::<u8>::from_addr((g::<u32>(NAVI_LINKS + 4 * area).read() + (link & 0x3FF) * 0xE) as usize);
    let d = xi(i32::from(dir as u8 as i8));
    let fx = xi(i32::from(rd::<i8>(nl, 8))).fmul_m(F_0_01).fmul(d).to_f32();
    let fy = xi(i32::from(rd::<i8>(nl, 9))).fmul_m(F_0_01).fmul(d).to_f32();
    let _ = angle_from(veh, px, x(py));
    let lanes = rd::<u8>(nl, 0xB);
    let left = i32::from(lanes & 7);
    let right = i32::from((lanes >> 3) & 7);
    let (a, b) = if left == 0 {
        let f = xi(right).fmul_m(F_0_5).to_f32();
        (f, x(f))
    } else if right == 0 {
        let f = xi(left).fmul_m(F_0_5).to_f32();
        (f, x(f))
    } else {
        let (own, other) = if dir as u8 == 0 { (left, right) } else { (right, left) };
        let own = xi(own).to_f32();
        (xi(i32::from(rd::<u8>(nl, 0xA))).fmul_m(F_WIDTH).fadd_m(own).to_f32(), xi(other))
    };
    let am = x(a).fsub_m(F_0_3);
    let bm = b.fsub_m(F_0_3).to_f32();
    let p1x = x(fy).fmul(am).fmul_m(F_5_4).fadd_m(px).to_f32();
    let p1y = am.fmul_m(fx).fmul_m(F_5_4).fsubr_m(py);
    let p2x = x(bm).fmul_m(fy).fmul_m(F_5_4).fsubr_m(px).to_f32();
    let p2y = x(bm).fmul_m(fx).fmul_m(F_5_4).fadd_m(py).to_f32();
    let a1 = angle_from(veh, p1x, p1y).to_f32();
    let a2 = angle_from(veh, p2x, x(p2y));
    let o = orient.read();
    let mut d1s = x(a1).fsub_m(o).to_f32();
    let mut d2 = a2.fsub_m(o);
    let mut d1 = x(d1s);
    if !le(d1, F_PI) {
        while !le(d1, F_PI) {
            d1 = d1.fsub_m(F_2PI);
        }
        d1s = d1.to_f32();
    }
    if !ge(d1, F_MPI) {
        while !ge(d1, F_MPI) {
            d1 = d1.fadd_m(F_2PI);
        }
        d1s = d1.to_f32();
    }
    while !le(d2, F_PI) {
        d2 = d2.fsub_m(F_2PI);
    }
    while !ge(d2, F_MPI) {
        d2 = d2.fadd_m(F_2PI);
    }
    let pick = if !ge(d1, 0.0) {
        if ge(d2, 0.0) {
            None
        } else if d1.fcom(d2).test_any(0x41) {
            Some(d2)
        } else {
            Some(x(d1s))
        }
    } else if le(d1, 0.0) || le(d2, 0.0) {
        None
    } else if d1.fcom(d2).test_parity(0x05) {
        Some(d2)
    } else {
        Some(x(d1s))
    };
    if let Some(v) = pick {
        orient.write(v.fadd_m(o).to_f32());
    }
    let v = x(orient.read());
    if !ge(v, 0.0) {
        let mut v = v;
        while !ge(v, 0.0) {
            v = v.fadd_m(F_2PI);
        }
        orient.write(v.to_f32());
    }
    let v = x(orient.read());
    if !le(v, F_2PI) {
        let mut v = v;
        while !le(v, F_2PI) {
            v = v.fsub_m(F_2PI);
        }
        orient.write(v.to_f32());
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CLIP_TARGET_ORIENTATION_TO_LINK];
