//! `CSkidmarks::RegisterOne(index, pos, dirX, dirY, type, bool* blood, length)` (0x720930, batch
//! `samp2-render-1`), with `CSkidmark::RegisterNewPart` / `Init` / `GetNextFree` inlined. Ported from the
//! 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, x};

orig! {
    /// `CCamera::IsSphereVisible(const CVector& center, float radius)` (0x420D40, thiscall)
    fn camera_is_sphere_visible(cam: GPtr<u8>, center: GPtr<u8>, radius: f32) -> u8 = thiscall 0x420D40 => unported;
}

/// `CSkidmarks::m_aSkidmarks` (32 x 0x158 at 0xC79AA8): 16 positions +0, side x [16] +0xC0, side y [16]
/// +0x100, id +0x140, last update +0x144, fade start +0x148, fade end +0x14C, type +0x150, parts u16 +0x154,
/// state +0x156 (0 free, 1 updating, 2 fading), active +0x157.
const MARKS: usize = 0xC7_9AA8;

fn mark(i: usize) -> usize {
    MARKS + i * 0x158
}

fn fv(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn now() -> u32 {
    sa_core::timer::TIME_MS.get()
}

/// `(x, y) / |(x, y)|` as the original keeps it: with a positive (or NaN) length `(y_mem * inv, x_reg *
/// inv)` where `inv = (f32)(1 / len)`; otherwise `(1.0, x_reg)` (not normalised).
fn unit(xr: X87, ym: f32) -> (X87, X87) {
    let len = xr.fmul(xr).fadd(x(ym).fmul_m(ym)).fsqrt();
    if len.fcom(x(0.0)).test_parity(0x41) {
        let inv = x(1.0).fdiv(len).to_f32();
        (x(inv).fmul_m(ym), xr.fmul_m(inv))
    } else {
        (x(1.0), xr)
    }
}

/// `CSkidmarks::RegisterOne(uint32 index, const CVector& pos, float dirX, float dirY, eSkidmarkType type,
/// bool* bloodState, float length)` (0x720930, cdecl). Nothing during replay playback (0xA43088 == 1).
///
/// A mark updating with this id: when its bloodiness (`type == 3`) differs from `*blood` it starts fading
/// (state 2, fade 10 s to 20 s from now). Else it is active; within 100 ms (unsigned) of its last update
/// only the newest position is replaced. Otherwise the update time is now; a full mark (15 parts) fades and
/// clears `*blood`; else part n = parts + 1 gets the position and the side vector: `(a, b) = unit(prev.x -
/// pos.x, pos.y - prev.y)`, `(c, d) = unit(dirY, dirX)` (same rule, x87 kept), `k = |a*c + b*d| + 1`, side
/// x = `k * a * length * 0.5` (k at 80 bits), side y = `(f32)k * b * length * 0.5`; part 1 copies its side
/// to part 0; past 8 parts `*blood` is cleared.
///
/// Otherwise a free mark, else the inactive one with the oldest update (unsigned, below 0xFFFFFFFF) whose
/// bounding sphere (centre `(p0 + pn) * 0.5`, radius `|p0 - centre|`) the camera cannot see; none: `*blood
/// = 0`. Init: id, state 1, p0 = pos, side 0, active, 0 parts, update time now - 1000, type 3 when `*blood`
/// else `type`.
#[hook(addr = 0x720930, conv = cdecl, name = "CSkidmarks::RegisterOne-", expect = "A0 88 30 A4 00",
       unverified = "samp2-render-1", route = "L2", hazards = "none", recs = "driving")]
#[allow(clippy::too_many_arguments)]
pub fn register_one(index: u32, pos: GPtr<f32>, dir_x: f32, dir_y: f32, ty: u32, blood: GPtr<u8>, length: f32) {
    if g::<u8>(0xA4_3088).read() == 1 {
        return;
    }
    let p: [f32; 3] = [pos.wrapping_index(0).read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read()];
    if let Some(i) = (0..32).find(|&i| g::<u8>(mark(i) + 0x156).read() == 1 && g::<u32>(mark(i) + 0x140).read() == index) {
        let m = mark(i);
        let t = now();
        if u32::from(g::<u32>(m + 0x150).read() == 3) != u32::from(blood.read()) {
            g::<u8>(m + 0x156).write(2);
            g::<u32>(m + 0x148).write(t.wrapping_add(10_000));
            g::<u32>(m + 0x14C).write(t.wrapping_add(20_000));
            return;
        }
        g::<u8>(m + 0x157).write(1);
        if t.wrapping_sub(g::<u32>(m + 0x144).read()) <= 100 {
            let n = g::<i16>(m + 0x154).read() as i32;
            g::<[f32; 3]>((m as i32 + n * 12) as usize).write(p);
            return;
        }
        let n = g::<i16>(m + 0x154).read();
        g::<u32>(m + 0x144).write(t);
        if n >= 0xF {
            g::<u32>(m + 0x144).write(t);
            g::<u8>(m + 0x156).write(2);
            g::<u32>(m + 0x148).write(t.wrapping_add(10_000));
            g::<u32>(m + 0x14C).write(t.wrapping_add(20_000));
            blood.write(0);
            return;
        }
        let n = n + 1;
        g::<i16>(m + 0x154).write(n);
        let n = n as i32;
        let cur = (m as i32 + n * 12) as usize;
        g::<[f32; 3]>(cur).write(p);
        let dy = x(fv(cur + 4)).fsub_m(fv(cur - 8)).to_f32();
        let dxn = x(fv(cur - 12)).fsub_m(fv(cur));
        let (a, b) = unit(dxn, dy);
        let (c, d) = unit(x(dir_y), dir_x);
        let s = a.fmul(c).fadd(b.fmul(d)).fabs().fadd_m(1.0);
        let kk = s.to_f32();
        g::<f32>((m as i32 + 0xC0 + n * 4) as usize).write(s.fmul(a).fmul_m(length).fmul_m(0.5).to_f32());
        g::<f32>((m as i32 + 0x100 + n * 4) as usize).write(x(kk).fmul(b).fmul_m(length).fmul_m(0.5).to_f32());
        if n == 1 {
            g::<u32>(m + 0xC0).write(g::<u32>(m + 0xC4).read());
            g::<u32>(m + 0x100).write(g::<u32>(m + 0x104).read());
        }
        if n > 8 {
            blood.write(0);
        }
        return;
    }
    let mut slot = (0..32).find(|&i| g::<u8>(mark(i) + 0x156).read() == 0);
    if slot.is_none() {
        let mut oldest = u32::MAX;
        let mut best = -1i32;
        for i in 0..32 {
            let m = mark(i);
            if g::<u8>(m + 0x157).read() != 0 || g::<u32>(m + 0x144).read() >= oldest {
                continue;
            }
            let n = g::<i16>(m + 0x154).read() as i32;
            let e = (m as i32 + n * 12) as usize;
            let sx = x(fv(e)).fadd_m(fv(m));
            let sy = x(fv(e + 4)).fadd_m(fv(m + 4));
            let sz = x(fv(e + 8)).fadd_m(fv(m + 8)).to_f32();
            let mut c = [sx.fmul_m(0.5).to_f32(), sy.fmul_m(0.5).to_f32(), x(sz).fmul_m(0.5).to_f32()];
            let dx = x(fv(m)).fsub_m(c[0]);
            let dy = x(fv(m + 4)).fsub_m(c[1]);
            let dz = x(fv(m + 8)).fsub_m(c[2]);
            let r = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt().to_f32();
            if camera_is_sphere_visible(g(0xB6_F028), GPtr::<f32>::from_raw(c.as_mut_ptr()).cast(), r) == 0 {
                oldest = g::<u32>(m + 0x144).read();
                best = i as i32;
            }
        }
        if best >= 0 {
            slot = Some(best as usize);
        }
    }
    let Some(i) = slot else {
        blood.write(0);
        return;
    };
    let m = mark(i);
    g::<u32>(m + 0x140).write(index);
    g::<u8>(m + 0x156).write(1);
    g::<[f32; 3]>(m).write(p);
    g::<u32>(m + 0xC0).write(0);
    g::<u32>(m + 0x100).write(0);
    g::<u8>(m + 0x157).write(1);
    g::<i16>(m + 0x154).write(0);
    g::<u32>(m + 0x144).write(now().wrapping_add(0xFFFF_FC18));
    g::<u32>(m + 0x150).write(if blood.read() != 0 { 3 } else { ty });
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_REGISTER_ONE];
