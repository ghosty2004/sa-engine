//! `FUN_006E61B0` (gta-reversed declares it as `CWaterLevel::TestLineAgainstWater(CVector, CVector,
//! CVector*)`, unreversed; batch `samp2-water-1`): the segment-against-water test `CWeapon::FireInstantHit`
//! uses for bullet splashes. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::{ftol32, g, k, rd, wr, x, xi};

/// `CWaterLevel::m_nBlocksToBeRenderedOutsideWorld`'s neighbour: the 12x12 block grid of water entries
/// (u16 each: bits 14-15 the kind, 1 = one quad, 3 = a list in 0xC215F8; bits 0-13 the index), at 0xC21B70.
const BLOCK_GRID: usize = 0xC2_1B70;
/// The entry lists (u16 each, a list ends at the first entry whose top two bits are 0), at 0xC215F8.
const BLOCK_LISTS: usize = 0xC2_15F8;
/// `CWaterLevel::m_aQuads` (10 bytes each: four vertex indices, flags), at 0xC21C90.
const QUADS: usize = 0xC2_1C90;
/// `CWaterLevel::m_aVertices` (20 bytes each, i16 x at +0 and y at +2), at 0xC22910.
const VERTICES: usize = 0xC2_2910;

/// Cell of a coordinate: `ftol(v * 0.002f + 6.0f)` (0x858F44, 0x858B44; the product kept at 80 bits).
fn cell(v: f32) -> i32 {
    ftol32(x(v).fmul_m(k(0x3B03_126F)).fadd_m(k(0x40C0_0000)))
}

/// `fild word` of a water vertex coordinate (`off` 0 = x, 2 = y).
fn vert(i: i16, off: usize) -> X87 {
    let a = (VERTICES as u32).wrapping_add((i32::from(i) * 20) as u32) as usize + off;
    xi(i32::from(g::<i16>(a).read()))
}

/// The crossing point of the segment with the plane z = 0 as the original builds it, written to `out`
/// (x first, then y, then z): `t = |s.z| / (zmax - zmin)` stored f32, `out.x = (f32)(d.x * t) + s.x`,
/// `out.y = d.y * t + s.y`, `out.z = (f32)(d.z) * t + s.z` (d = e - s; d.x / d.y kept at 80 bits, d.z
/// stored f32).
fn crossing(s: [f32; 3], e: [f32; 3], zmin: f32, zmax: f32, out: GPtr<u8>) {
    let dx = x(e[0]).fsub_m(s[0]);
    let dy = x(e[1]).fsub_m(s[1]);
    let dz = x(e[2]).fsub_m(s[2]).to_f32();
    let t = x(s[2]).fabs().fdiv(x(zmax).fsub_m(zmin)).to_f32();
    let dxt = dx.fmul_m(t).to_f32();
    let dyt = dy.fmul_m(t);
    let dzt = x(dz).fmul_m(t);
    let px = x(dxt).fadd_m(s[0]).to_f32();
    wr::<f32>(out, 0, px);
    let py = dyt.fadd_m(s[1]).to_f32();
    wr::<f32>(out, 4, py);
    wr::<f32>(out, 8, dzt.fadd_m(s[2]).to_f32());
}

/// The quad test: the segment crosses z = 0 (`s.z * e.z < 0`, else no), the crossing point is written to
/// `out`, and it lies in the quad's box: `v0.x <= p.x`, `!(v1.x < p.x)`, `v0.y <= p.y`, `!(v2.y < p.y)`
/// (vertex coordinates `fild`ed; `out` re-read after the write).
fn quad_hit(q: u32, s: [f32; 3], e: [f32; 3], zmin: f32, zmax: f32, out: GPtr<u8>) -> bool {
    if x(s[2]).fmul_m(e[2]).fcom(x(0.0)).test_parity(5) {
        return false;
    }
    crossing(s, e, zmin, zmax, out);
    let quad = QUADS + q as usize * 10;
    let v0 = g::<i16>(quad).read();
    let v1 = g::<i16>(quad + 2).read();
    let v2 = g::<i16>(quad + 4).read();
    let px = rd::<f32>(out, 0);
    if vert(v0, 0).fcom(x(px)).test_parity(0x41) {
        return false;
    }
    if vert(v1, 0).fcom(x(rd::<f32>(out, 0))).test_any(1) {
        return false;
    }
    if vert(v0, 2).fcom(x(rd::<f32>(out, 4))).test_parity(0x41) {
        return false;
    }
    !vert(v2, 2).fcom(x(rd::<f32>(out, 4))).test_any(1)
}

/// `FUN_006E61B0(CVector start, CVector end, CVector* out)` (cdecl, vectors by value; AL result, the upper
/// EAX bits are what the last callee left): true when the segment crosses a water quad's z = 0 plane.
///
/// The bounding box: `min = a < b ? a : b`, `max = a > b ? a : b` per axis (f32 compares; NaN picks the end
/// point). The cells `ftol(v * 0.002f + 6.0f)` of min y, max x, max y, min x (that order). For every cell x
/// from min to max (outer) and y from min to max (inner): inside the 12x12 grid (`0 <= x*12 < 144`,
/// `0 <= y < 12`) the grid entry `[x*12 + y]` decides: kind 1 tests its quad, kind 3 walks its list from
/// 0xC215F8 (only when the list's first entry has a kind) and tests every kind-1 entry, other kinds nothing;
/// a quad hit returns true. Outside the grid the flat sea counts: `zmin < 0` and `!(zmax <= 0)` (NaN fails
/// both), the crossing point is written to `out`, and it hits when its cells equal the loop's x and y.
/// The point is written for every tested candidate, so `out` holds the last one on a miss.
#[hook(addr = 0x6E61B0, conv = cdecl, name = "FUN_006e61b0", expect = "D9 44 24 0C 81 EC 88 00 00 00",
       unverified = "samp2-water-1", route = "L2", hazards = "none", recs = "weapon-combat",
       note = "gta-reversed's CWaterLevel::TestLineAgainstWater (unreversed); called by FireInstantHit for splashes")]
#[allow(clippy::too_many_arguments)]
pub fn test_line_against_water(sx: f32, sy: f32, sz: f32, ex: f32, ey: f32, ez: f32, out: GPtr<u8>) -> u8 {
    let s = [sx, sy, sz];
    let e = [ex, ey, ez];
    let lt = |a: f32, b: f32| !x(a).fcom(x(b)).test_parity(5);
    let le_or_nan = |a: f32, b: f32| x(a).fcom(x(b)).test_any(0x41);
    let zmin = if lt(sz, ez) { sz } else { ez };
    let ymin = if lt(sy, ey) { sy } else { ey };
    let xmin = if lt(sx, ex) { sx } else { ex };
    let zmax = if le_or_nan(sz, ez) { ez } else { sz };
    let ymax = if le_or_nan(sy, ey) { ey } else { sy };
    let xmax = if le_or_nan(sx, ex) { ex } else { sx };
    let cy0 = cell(ymin);
    let cx1 = cell(xmax);
    let cy1 = cell(ymax);
    let cx0 = cell(xmin);
    let mut cx = cx0;
    while cx <= cx1 {
        let row = cx.wrapping_mul(12);
        let mut cy = cy0;
        while cy <= cy1 {
            if (0..0x90).contains(&row) && (0..12).contains(&cy) {
                let v = g::<u16>(BLOCK_GRID + (cy + row) as usize * 2).read();
                match v >> 14 {
                    1 => {
                        if quad_hit(u32::from(v & 0x3FFF), s, e, zmin, zmax, out) {
                            return 1;
                        }
                    }
                    3 => {
                        let mut p = BLOCK_LISTS + usize::from(v & 0x3FFF) * 2;
                        if g::<u8>(p + 1).read() & 0xC0 != 0 {
                            loop {
                                let ent = g::<u16>(p).read();
                                if ent >> 14 == 1 && quad_hit(u32::from(ent & 0x3FFF), s, e, zmin, zmax, out) {
                                    return 1;
                                }
                                p += 2;
                                if g::<u8>(p + 1).read() & 0xC0 == 0 {
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            } else if !x(zmin).fcom(x(0.0)).test_parity(5) && !x(zmax).fcom(x(0.0)).test_any(0x41) {
                crossing(s, e, zmin, zmax, out);
                if cell(rd::<f32>(out, 0)) == cx && cell(rd::<f32>(out, 4)) == cy {
                    return 1;
                }
            }
            cy += 1;
        }
        cx += 1;
    }
    0
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_TEST_LINE_AGAINST_WATER];
