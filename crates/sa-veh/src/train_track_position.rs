//! PORT-VEH-4 (batch `veh4-3`): `CTrain::FindPositionOnTrackFromCoors`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).
//!
//! The entry's protection detour (`nop; nop; jmp 0x403C30`) is the relocated `movzx eax, byte [esi +
//! 0x5C0]` (the track index) and jumps straight back to 0x6F6CCE.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use crate::common::{f32_of, g, jne_mask, jp5, model_info, rd, wr, x, xi};

orig! {
    /// `CTrainNode::GetLightingFromCollision()` (0x6F5F80, AL)
    fn train_node_get_lighting(node: GPtr<u8>) -> u8 = thiscall 0x6F5F80 => unported;
}

/// `NumTrackNodes[track]` (0xC38014).
fn num_nodes(track: usize) -> i32 {
    g::<i32>(0xC3_8014 + 4 * track).read()
}

/// `pTrackNodes[track]` (0xC38024): 10-byte `CTrainNode`s (x, y, z as int16 eighths, +6 distance in
/// thirds of a unit, +8 lighting, +9 lighting-valid).
fn nodes(track: usize) -> GPtr<u8> {
    g::<GPtr<u8>>(0xC3_8024 + 4 * track).read()
}

/// A node coordinate (int16 / 8).
fn coord(node: GPtr<u8>, o: isize) -> f32 {
    f32_of(xi(i32::from(rd::<i16>(node, o))).fmul_m(0.125))
}

/// The node's lighting as a day/night blend (`lo * (1/30) * (1 - mix) + hi * (1/30) * mix`, the mix at
/// 0x8D12C0).
fn node_light(node: GPtr<u8>) -> X87 {
    let l = train_node_get_lighting(node);
    let mix = x(g::<f32>(0x8D_12C0).read());
    let c = f32::from_bits(0x3D08_8889);
    let lo = xi(i32::from(l & 0xF)).fmul_m(c);
    let hi = xi(i32::from(l >> 4)).fmul_m(c);
    lo.fmul(x(1.0).fsub(mix)).fadd(hi.fmul(mix))
}

/// `CTrain::FindPositionOnTrackFromCoors()` (0x6F6CC0): walks the train's track (+0x5C0) for the
/// segment whose projection parameter `t` of the train position lies in (0.001, 1.001) and whose
/// projected point is within 3 units (xy): the track position (+0x5A8) becomes the interpolated node
/// distance (`/3`) minus half the model's col-model length (plus the track length when negative), the
/// speed (+0x5A4) the length of the move speed, negated when the direction flag (+0x5B8 bit 6) equals
/// "moving along the segment". Segments in range but too far set the brightness (+0x12C) from the two
/// nodes' lighting.
#[hook(addr = 0x6F6CC0, conv = thiscall, name = "CTrain::FindPositionOnTrackFromCoors", this = "CTrain",
       expect = "83 EC 50 53 56 8B F1",
       unverified = "veh4-3", route = "L2", hazards = "x87-transcendental",
       note = "FSQRT of the segment lengths; the body's `nop; nop; jmp 0x403C30` detour is a relocated movzx; the lighting query may run a vertical line test and cache it in the node")]
pub fn train_find_position_on_track_from_coors(this: GPtr<u8>) {
    let track = rd::<u8>(this, 0x5C0) as usize;
    if num_nodes(track) <= 0 {
        return;
    }
    let mut i = 0i32;
    loop {
        let track = rd::<u8>(this, 0x5C0) as usize;
        let next = (i + 1) % num_nodes(track);
        let base = nodes(track);
        let ni = base.byte_add(10 * i as isize);
        let nn = base.byte_add(10 * next as isize);
        let (p0x, p0y) = (coord(ni, 0), coord(ni, 2));
        let (p1x, p1y) = (coord(nn, 0), coord(nn, 2));
        let m = rd::<GPtr<u8>>(this, 0x14);
        let pp = if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) };
        let (px, py) = (rd::<f32>(pp, 0), rd::<f32>(pp, 4));
        let dx = x(p0x).fsub_m(p1x);
        let dy = x(p0y).fsub_m(p1y);
        let len = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        let ex = x(p1x).fsub_m(p0x);
        let ey = x(p1y).fsub_m(p0y);
        let (exf, eyf) = (f32_of(ex), f32_of(ey));
        let ax = x(px).fsub_m(p0x);
        let ay = x(py).fsub_m(p0y);
        let t = f32_of(ay.fmul(ey).fadd(ax.fmul(ex)).fdiv(len.fmul(len)));
        if !jne_mask(x(t), x(f32::from_bits(0x3A83_126F)), 0x41) && !jp5(x(t), x(f32::from_bits(0x3F80_20C5))) {
            let sx = f32_of(x(exf).fmul_m(t));
            let sy = x(eyf).fmul_m(t);
            let z = x(t).fmul_m(0.0);
            let qx = f32_of(x(sx).fadd_m(p0x));
            let qy = f32_of(sy.fadd_m(p0y));
            let ddx = x(qx).fsub_m(px);
            let ddy = x(qy).fsub_m(py);
            let d = z.fmul(z).fadd(ddx.fmul(ddx)).fadd(ddy.fmul(ddy)).fsqrt();
            if d.fcom(x(3.0)) == Cmp::Less {
                found(this, i, next, t, (p0x, p0y), (p1x, p1y));
                return;
            }
            let l1 = f32_of(node_light(ni));
            let l2 = node_light(nodes(rd::<u8>(this, 0x5C0) as usize).byte_add(10 * next as isize));
            wr::<f32>(this, 0x12C, f32_of(l2.fsub_m(l1).fmul_m(t).fadd_m(l1)));
        }
        i += 1;
        if i >= num_nodes(rd::<u8>(this, 0x5C0) as usize) {
            return;
        }
    }
}

/// The segment found (0x6F6F6D).
fn found(this: GPtr<u8>, i: i32, next: i32, t: f32, p0: (f32, f32), p1: (f32, f32)) {
    let track = rd::<u8>(this, 0x5C0) as usize;
    let base = nodes(track);
    let third = f32::from_bits(0x3EAA_AAAB);
    let a = xi(i32::from(rd::<u16>(base.byte_add(10 * i as isize), 6))).fmul_m(third);
    let b = xi(i32::from(rd::<u16>(base.byte_add(10 * next as isize), 6))).fmul_m(third);
    let pos = a.fadd(b.fsub(a).fmul_m(t));
    wr::<f32>(this, 0x5A8, f32_of(pos));
    let col = rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(this, 0x22))), 0x14);
    let half = x(rd::<f32>(col, 0x10)).fsub_m(rd::<f32>(col, 4)).fmul_m(0.5);
    let pos = pos.fsub(half);
    wr::<f32>(this, 0x5A8, f32_of(pos));
    if pos.fcom(x(0.0)) == Cmp::Less {
        let total = g::<f32>(0xC3_7FEC + 4 * track).read();
        wr::<f32>(this, 0x5A8, f32_of(pos.fadd_m(total)));
    }
    let (mx, my, mz) = (rd::<f32>(this, 0x44), rd::<f32>(this, 0x48), rd::<f32>(this, 0x4C));
    let speed = x(mz).fmul(x(mz)).fadd(x(my).fmul(x(my))).fadd(x(mx).fmul_m(mx)).fsqrt();
    wr::<f32>(this, 0x5A4, f32_of(speed));
    let ex = x(p1.0).fsub_m(p0.0);
    let ey = x(p1.1).fsub_m(p0.1);
    let dot = ey.fmul_m(my).fadd(x(mz).fmul_m(0.0)).fadd(ex.fmul_m(mx));
    let along = !jne_mask(dot, x(0.0), 0x41);
    let flag = (rd::<u8>(this, 0x5B8) >> 6) & 1 != 0;
    if flag == along {
        wr::<f32>(this, 0x5A4, f32_of(speed.fchs()));
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_TRAIN_FIND_POSITION_ON_TRACK_FROM_COORS];
