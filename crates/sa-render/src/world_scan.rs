//! `CWorldScan::ScanWorld` (batch `mvpm-render-2`): the sector rasteriser behind `CRenderer::ScanWorld`,
//! `GetObjectsInFrustum`, `RequestObjectsInFrustum` and `CWaterLevel::ScanThroughBlocks`. Ported instruction by
//! instruction from the 1.0 US bytes (0x72CAE0-0x72D5CF; gta-reversed only calls the original;
//! `docs/re/specs/CALLED-ONLY/CWorldScan.md` describes the phases). Unverified (docs/verify/HOWTO.md).
//!
//! The original keeps its state in an aligned stack frame whose slots are reused for unrelated values (the
//! `used[]` bytes become a float temporary, the left-edge vertex index becomes the left edge x, the
//! extra-list search writes `count - 1` into the `xLo` slot). The port keeps the same frame as a byte array
//! ([`Frame`], offsets relative to ESP after the three register pushes) so every such reuse happens as in
//! the original.
//!
//! x87 (PC=24): every subtraction, product and quotient goes through [`X87`]; `floor` (0x8219F0) and `ceil`
//! (0x823820) are exact on the doubles they get, `_ftol` (0x821B40) truncates.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// `CGeneral::GetATanOfXY(float x, float y)` (0x53CC70, ST0)
    fn general_get_atan_of_xy(x: f32, y: f32) -> f64 = cdecl 0x53CC70 => unported;
    /// The scan callback `void (*)(int32 x, int32 y)` (cdecl, the address passed in)
    fn call_scan(f: u32, x: i32, y: i32) = cdecl dyn => unported;
}

/// `CWorldScan` extra-rectangle count (i32, 0xC815B8), its Y list (i32[4], 0xC81598) and X list (i32[4], 0xC815A8)
const EXTRA_COUNT: usize = 0xC815B8;
const EXTRA_Y: u32 = 0xC81598;
const EXTRA_X: u32 = 0xC815A8;
/// 0.0f (0x858B50)
const K_ZERO: usize = 0x858B50;
/// 2*pi as f32 (0x858CBC, 6.2831855)
const K_TWO_PI: usize = 0x858CBC;

// Frame slots (ESP after `push ebx; push esi; push edi`).
const X_HI: usize = 0x0C;
const X_LO: usize = 0x10;
const SLOPE_L: usize = 0x14;
const Y_MAX_SLOPE_R: usize = 0x18; // yMax while finding the bounds, then the right slope
const L_CUR_XL: usize = 0x1C; // the left walker's vertex, then the left edge x
const HULL_N: usize = 0x20;
const X_R: usize = 0x24;
const ROW: usize = 0x28;
const L_NEXT: usize = 0x2C;
const S30: usize = 0x30; // dedupe outer pointer, hull `prev` angle, walker counters, the right walker's vertex
const S34: usize = 0x34; // dedupe outer index, `best` angle, the last row
const S38: usize = 0x38; // `used[5]` bytes, then a float / double temporary and the column
const HULL: usize = 0x40; // CVector2D H[]

/// The original's stack frame (0x40 bytes of slots, then room for the hull points).
struct Frame([u8; 0x40 + 8 * 16]);

impl Frame {
    fn i(&self, o: usize) -> i32 {
        i32::from_le_bytes(self.0[o..o + 4].try_into().unwrap())
    }
    fn set_i(&mut self, o: usize, v: i32) {
        self.0[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn f(&self, o: usize) -> f32 {
        f32::from_bits(self.i(o) as u32)
    }
    fn set_f(&mut self, o: usize, v: f32) {
        self.set_i(o, v.to_bits() as i32);
    }
    fn hx(&self, k: i32) -> f32 {
        self.f(HULL + 8 * k as usize)
    }
    fn hy(&self, k: i32) -> f32 {
        self.f(HULL + 8 * k as usize + 4)
    }
    /// `fld dword; fstp qword [esp+0x38]`: the f32 widened into the 0x38..0x3F temporary (the original then passes
    /// the two dwords to `floor`).
    fn store_double(&mut self, v: f32) -> f64 {
        let d = f64::from(X87::from_f32(v).to_f32());
        self.0[S38..S38 + 8].copy_from_slice(&d.to_le_bytes());
        d
    }
}

/// CRT `floor` (0x8219F0) on a double: exact.
fn floor_d(v: f64) -> f64 {
    if v.is_nan() || v.is_infinite() || v.abs() >= 4_503_599_627_370_496.0 {
        return v;
    }
    let t = v as i64 as f64;
    if t > v { t - 1.0 } else { t }
}
/// CRT `ceil` (0x823820) on a double: exact.
fn ceil_d(v: f64) -> f64 {
    if v.is_nan() || v.is_infinite() || v.abs() >= 4_503_599_627_370_496.0 {
        return v;
    }
    let t = v as i64 as f64;
    if t < v { t + 1.0 } else { t }
}
/// `_ftol` (0x821B40) of an integral double in ST0.
fn ftol(v: f64) -> i32 {
    X87::from_f64_exact(v).ftol() as u32 as i32
}
/// `_ftol(floor((double)v))` of an x87 value (≤ 24 significant bits: the qword store is exact).
fn floor_i(v: X87) -> i32 {
    ftol(floor_d(v.to_f64()))
}
fn floor_f(v: f32) -> i32 {
    floor_i(X87::from_f32(v))
}
fn xf(v: f32) -> X87 {
    X87::from_f32(v)
}
fn k(a: usize) -> f32 {
    GPtr::<f32>::from_addr(a).read()
}
/// `fcom` C0 alone (`test ah, 5; jp` falls through): `a < b`, ordered.
fn less(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(5)
}
/// `test ah, 1` set: `a < b` or unordered.
fn c0(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(1)
}
/// `test ah, 0x41` clear: `a > b`, ordered.
fn greater(a: X87, b: X87) -> bool {
    !a.fcom(b).test_any(0x41)
}

/// The extra-rectangle list search after `scan(x, row)` (0x72D10B-0x72D1A5): the first entry equal to (x, row)
/// is removed (both lists shift down with `rep movsd`), the count drops by one and the search stops (ECX = 4 then
/// `inc`). The slot `xLo` receives `count - 1` while searching (original slot reuse).
fn remove_extra(fr: &mut Frame, x: i32, row: i32) {
    let cnt = GPtr::<i32>::from_addr(EXTRA_COUNT);
    let mut eax = cnt.read();
    let mut ecx: i32 = 0;
    if eax <= 0 {
        return;
    }
    fr.set_i(X_LO, eax - 1);
    loop {
        let xs = GPtr::<i32>::from_addr((EXTRA_X as i32).wrapping_add(ecx.wrapping_mul(4)) as u32 as usize);
        let ys = GPtr::<i32>::from_addr((EXTRA_Y as i32).wrapping_add(ecx.wrapping_mul(4)) as u32 as usize);
        if xs.read() == x && ys.read() == row {
            let last = fr.i(X_LO);
            if ecx < last {
                let n = (last - ecx) as u32;
                for t in 0..n {
                    let d = GPtr::<u32>::from_addr((EXTRA_Y + (ecx as u32 + t) * 4) as usize);
                    d.write(GPtr::<u32>::from_addr((EXTRA_Y + (ecx as u32 + t + 1) * 4) as usize).read());
                }
                for t in 0..n {
                    let d = GPtr::<u32>::from_addr((EXTRA_X + (ecx as u32 + t) * 4) as usize);
                    d.write(GPtr::<u32>::from_addr((EXTRA_X + (ecx as u32 + t + 1) * 4) as usize).read());
                }
                eax = cnt.read();
            }
            eax -= 1;
            fr.set_i(X_LO, fr.i(X_LO) - 1);
            cnt.write(eax);
            ecx = 4;
        }
        ecx += 1;
        if ecx >= eax {
            return;
        }
    }
}

/// `CWorldScan::ScanWorld(CVector2D* points, int32 count, void (*scan)(int32 x, int32 y))` (0x72CAE0, cdecl).
///
/// 1. De-duplicates the caller's points in place (exact x and y compares; later points shift down, the count
///    argument slot is decremented).
/// 2. Gift-wraps them: start at the first strict minimum of y; from the current point every other point (used ones
///    included) gets `a = GetATanOfXY(dx, dy) - prev` (differences stored as f32) brought into [0, 2pi) by adding /
///    subtracting 2pi (f32 constant); the smallest `a` (strict, stored as f32 `best`) is next. A point already used
///    closes the hull; else `prev = f32(best + prev)` and the point is appended. The first pass's fallback index is
///    the dword of the `used[]` bytes (only reached without any candidate).
/// 3. Fewer than 3 hull points: return (the extra list is kept).
/// 4. Rasterises the hull row by row from `floor(min y)` to `floor(max y)` with two edge walkers (left: index - 1,
///    right: index + 1, both skipping vertices on the same row), calling `scan(x, row)` for `x = xLo ..= xHi` and
///    removing a matching extra-list entry after each call.
/// 5. Calls `scan` for every remaining extra-list entry and clears the count.
#[hook(addr = 0x72cae0, conv = cdecl, name = "CWorldScan::ScanWorld", expect = "55 8B EC 83 E4 F8",
       unverified = "mvpm-render-2", route = "L2", hazards = "none", recs = "grove-walk",
       note = "the original jumps through the protection stub 0x401512 for `used[0] = 0`; the de-duplication writes the caller's count argument slot (not modelled); with fewer than 2 points the hull walk reads p[1] past the caller's array as the original; more than 5 points overrun the original frame (here: a larger frame)")]
pub fn world_scan(points: GPtr<f32>, count: i32, scan: u32) {
    let mut fr = Frame([0u8; 0x40 + 8 * 16]);
    let p = |k: i32| GPtr::<f32>::from_addr((points.addr() as u32).wrapping_add((k as u32).wrapping_mul(8)) as usize);
    let px = |k: i32| p(k).read();
    let py = |k: i32| p(k).byte_add(4).read();
    let mut count = count;

    // 1. De-duplicate (0x72CAF9-0x72CBA1).
    let mut i = 0;
    while i < count - 1 {
        let mut j = i + 1;
        while j < count {
            if px(i) == px(j) && py(i) == py(j) {
                for kk in j..count - 1 {
                    let a = p(kk + 1).cast::<u32>();
                    let b = p(kk).cast::<u32>();
                    b.write(a.read());
                    b.byte_add(4).write(a.byte_add(4).read());
                }
                count -= 1;
                continue;
            }
            j += 1;
        }
        i += 1;
    }

    // 2. Start point: the first strict minimum of y (0x72CBA3-0x72CC37).
    let mut s = 0;
    let mut m = xf(py(0));
    for k2 in 1..count {
        if greater(m, xf(py(k2))) {
            s = k2;
            m = xf(py(k2));
        }
    }
    fr.0[S38 + s as usize] = 1;
    fr.set_f(HULL, xf(px(s)).to_f32());
    fr.set_f(HULL + 4, xf(py(s)).to_f32());
    let mut bi = fr.i(S38);
    fr.set_i(HULL_N, 1);
    fr.set_i(S30, 0);
    let mut cur = s;
    let two_pi = k(K_TWO_PI);
    let zero = xf(k(K_ZERO));
    loop {
        fr.set_i(S34, 0x47C34FF3);
        for c in 0..count {
            if c == cur {
                continue;
            }
            let dy = xf(py(c)).fsub_m(py(cur)).to_f32();
            let dx = xf(px(c)).fsub_m(px(cur)).to_f32();
            let mut a = X87::from_f64_exact(general_get_atan_of_xy(dx, dy)).fsub_m(fr.f(S30));
            if less(a, zero) {
                loop {
                    a = a.fadd_m(two_pi);
                    if !less(a, zero) {
                        break;
                    }
                }
            }
            while !c0(a, xf(two_pi)) {
                a = a.fsub_m(two_pi);
            }
            if less(a, xf(fr.f(S34))) {
                fr.set_f(S34, a.to_f32());
                bi = c;
            }
        }
        if fr.0.get((S38 as i32 + bi) as usize).copied().unwrap_or(0) != 0 {
            break;
        }
        let prev = xf(fr.f(S34)).fadd_m(fr.f(S30)).to_f32();
        let n = fr.i(HULL_N);
        let src = p(bi).cast::<u32>();
        fr.set_i(HULL + 8 * n as usize, src.read() as i32);
        fr.set_f(S30, prev);
        fr.set_i(HULL + 8 * n as usize + 4, src.byte_add(4).read() as i32);
        fr.set_i(HULL_N, n + 1);
        if let Some(u) = fr.0.get_mut((S38 as i32 + bi) as usize) {
            *u = 1;
        }
        cur = bi;
    }

    // 3.
    let hn = fr.i(HULL_N);
    if hn < 3 {
        return;
    }

    // 4. Bounds (0x72CD5C-0x72CE85).
    let mut mi = 0;
    let mut mn = xf(fr.hy(0));
    fr.set_i(Y_MAX_SLOPE_R, fr.i(HULL + 4));
    fr.set_i(X_LO, 9999);
    fr.set_i(X_HI, -9999);
    for c in 1..hn {
        if greater(mn, xf(fr.hy(c))) {
            mi = c;
            mn = xf(fr.hy(c));
        } else if greater(xf(fr.hy(c)), xf(fr.f(Y_MAX_SLOPE_R))) {
            fr.set_i(Y_MAX_SLOPE_R, fr.i(HULL + 8 * c as usize + 4));
        }
    }
    let row0 = floor_i(mn);
    let row_end = floor_f(fr.f(Y_MAX_SLOPE_R));
    fr.set_i(ROW, row0);
    fr.set_i(S34, row_end);

    // Left walker (0x72CEC0-0x72CF3B): ebx = vertex, esi = next (index - 1).
    let mut esi = mi;
    let mut cnt = 0;
    loop {
        cnt += 1;
        let ebx = esi;
        esi -= 1;
        fr.set_i(S30, cnt);
        fr.set_i(L_CUR_XL, ebx);
        if esi < 0 {
            esi = hn - 1;
        }
        if greater(X87::from_i32(fr.i(X_LO)), xf(fr.hx(ebx))) {
            fr.set_i(X_LO, floor_f(fr.hx(ebx)));
        }
        let fy = floor_d(f64::from(xf(fr.hy(esi)).to_f32()));
        fr.set_f(S38, fy as f32);
        let a = floor_f(fr.hy(ebx));
        let b = ftol(f64::from(fr.f(S38)));
        if !(a == b && fr.i(S30) < hn) {
            break;
        }
    }
    fr.set_i(L_NEXT, esi);
    // Right walker (0x72CF47-0x72CFBF): ebx = vertex, edi = next (index + 1).
    let mut edi = mi;
    let mut ebx;
    cnt = 0;
    loop {
        cnt += 1;
        ebx = edi;
        edi += 1;
        fr.set_i(S30, cnt);
        if edi == hn {
            edi = 0;
        }
        if less(X87::from_i32(fr.i(X_HI)), xf(fr.hx(ebx))) {
            fr.set_i(X_HI, floor_f(fr.hx(ebx)));
        }
        let fy = floor_d(f64::from(xf(fr.hy(edi)).to_f32()));
        fr.set_f(S38, fy as f32);
        let a = floor_f(fr.hy(ebx));
        let b = ftol(f64::from(fr.f(S38)));
        if !(a == b && fr.i(S30) < hn) {
            break;
        }
    }
    // Edge set-up (0x72CFC1-0x72D03B).
    let dxr = xf(fr.hx(edi)).fsub_m(fr.hx(ebx));
    fr.set_i(S30, edi);
    let dyr = xf(fr.hy(edi)).fsub_m(fr.hy(ebx));
    fr.set_f(Y_MAX_SLOPE_R, dxr.fdiv(dyr).to_f32());
    let lc = fr.i(L_CUR_XL);
    let ln = fr.i(L_NEXT);
    let xr = X87::from_f64_exact(ceil_d(f64::from(fr.hy(ebx))))
        .fsub_m(fr.hy(ebx))
        .fmul_m(fr.f(Y_MAX_SLOPE_R))
        .fadd_m(fr.hx(ebx));
    fr.set_f(X_R, xr.to_f32());
    let dxl = xf(fr.hx(ln)).fsub_m(fr.hx(lc));
    let dyl = xf(fr.hy(ln)).fsub_m(fr.hy(lc));
    fr.set_f(SLOPE_L, dxl.fdiv(dyl).to_f32());
    let xl = X87::from_f64_exact(ceil_d(f64::from(fr.hy(lc))))
        .fsub_m(fr.hy(lc))
        .fmul_m(fr.f(SLOPE_L))
        .fadd_m(fr.hx(lc));
    fr.set_f(L_CUR_XL, xl.to_f32());
    if fr.i(ROW) != fr.i(S34) {
        if less(xf(fr.f(SLOPE_L)), zero) {
            let d = fr.store_double(fr.f(L_CUR_XL));
            let v = ftol(floor_d(d));
            if fr.i(X_LO) > v {
                fr.set_i(X_LO, v);
            }
        }
        if !c0(xf(fr.f(Y_MAX_SLOPE_R)), zero) {
            let d = fr.store_double(fr.f(X_R));
            let v = ftol(floor_d(d));
            if fr.i(X_HI) < v {
                fr.set_i(X_HI, v);
            }
        }
    }
    if fr.i(ROW) > fr.i(S34) {
        flush_extra(scan);
        return;
    }

    // Row loop (0x72D0F0-0x72D58D).
    loop {
        let mut xc = fr.i(X_LO);
        fr.set_i(S38, xc);
        if xc <= fr.i(X_HI) {
            loop {
                let row = fr.i(ROW);
                call_scan(scan, xc, row);
                remove_extra(&mut fr, xc, row);
                xc += 1;
                fr.set_i(S38, xc);
                if xc > fr.i(X_HI) {
                    break;
                }
            }
        }
        // Next row, left edge (0x72D1B8-0x72D3A8).
        let nxr = xf(fr.f(Y_MAX_SLOPE_R)).fadd_m(fr.f(X_R)).to_f32();
        let row = fr.i(ROW) + 1;
        let mut b = fr.i(L_NEXT);
        fr.set_f(X_R, nxr);
        fr.set_i(ROW, row);
        let nxl = xf(fr.f(SLOPE_L)).fadd_m(fr.f(L_CUR_XL)).to_f32();
        fr.set_f(L_CUR_XL, nxl);
        let vy = floor_f(fr.hy(b));
        let sl_less = less(xf(fr.f(SLOPE_L)), zero);
        if row != vy {
            let t = if sl_less { xf(fr.f(L_CUR_XL)) } else { xf(fr.f(L_CUR_XL)).fsub_m(fr.f(SLOPE_L)) };
            fr.set_i(X_LO, floor_i(t));
        } else if row == fr.i(S34) {
            if sl_less {
                loop {
                    let v = floor_f(fr.hx(b));
                    fr.set_i(X_LO, v);
                    b -= 1;
                    if b < 0 {
                        b = fr.i(HULL_N) - 1;
                    }
                    if v <= floor_f(fr.hx(b)) {
                        break;
                    }
                }
                fr.set_i(L_NEXT, b);
            } else {
                let t = xf(fr.f(L_CUR_XL)).fsub_m(fr.f(SLOPE_L));
                fr.set_i(X_LO, floor_i(t));
            }
        } else {
            let t = if sl_less { xf(fr.hx(b)) } else { xf(fr.f(L_CUR_XL)).fsub_m(fr.f(SLOPE_L)) };
            fr.set_i(X_LO, floor_i(t));
            let mut e;
            loop {
                e = b;
                b -= 1;
                fr.set_i(L_NEXT, b);
                if b < 0 {
                    fr.set_i(L_NEXT, fr.i(HULL_N) - 1);
                }
                let hxv = fr.hx(e);
                fr.set_f(L_CUR_XL, xf(hxv).to_f32());
                let d = fr.store_double(hxv);
                let v = ftol(floor_d(d));
                if fr.i(X_LO) > v {
                    fr.set_i(X_LO, v);
                }
                b = fr.i(L_NEXT);
                if fr.i(ROW) != floor_f(fr.hy(b)) {
                    break;
                }
            }
            let dx = xf(fr.hx(b)).fsub_m(fr.hx(e));
            let dy = xf(fr.hy(b)).fsub_m(fr.hy(e));
            fr.set_f(SLOPE_L, dx.fdiv(dy).to_f32());
            let nl = X87::from_f64_exact(ceil_d(f64::from(fr.hy(e))))
                .fsub_m(fr.hy(e))
                .fmul_m(fr.f(SLOPE_L))
                .fadd_m(fr.f(L_CUR_XL));
            fr.set_f(L_CUR_XL, nl.to_f32());
            if less(xf(fr.f(SLOPE_L)), zero) {
                let d = fr.store_double(fr.f(L_CUR_XL));
                let v = ftol(floor_d(d));
                if fr.i(X_LO) > v {
                    fr.set_i(X_LO, v);
                }
            }
        }
        // Right edge (0x72D3AC-0x72D581).
        let mut e = fr.i(S30);
        let vy = floor_f(fr.hy(e));
        let row = fr.i(ROW);
        let sr_c0 = c0(xf(fr.f(Y_MAX_SLOPE_R)), zero);
        if row != vy {
            let t = if sr_c0 { xf(fr.f(X_R)).fsub_m(fr.f(Y_MAX_SLOPE_R)) } else { xf(fr.f(X_R)) };
            fr.set_i(X_HI, floor_i(t));
        } else if row == fr.i(S34) {
            if sr_c0 {
                let t = xf(fr.f(X_R)).fsub_m(fr.f(Y_MAX_SLOPE_R));
                fr.set_i(X_HI, floor_i(t));
            } else {
                loop {
                    let v = floor_f(fr.hx(e));
                    e += 1;
                    fr.set_i(X_HI, v);
                    if e == fr.i(HULL_N) {
                        e = 0;
                    }
                    if v >= floor_f(fr.hx(e)) {
                        break;
                    }
                }
                fr.set_i(S30, e);
            }
        } else {
            let t = if sr_c0 { xf(fr.f(X_R)).fsub_m(fr.f(Y_MAX_SLOPE_R)) } else { xf(fr.hx(e)) };
            fr.set_i(X_HI, floor_i(t));
            let mut nx;
            loop {
                nx = fr.i(S30);
                e = nx;
                nx += 1;
                fr.set_i(S30, nx);
                if nx == fr.i(HULL_N) {
                    fr.set_i(S30, 0);
                }
                let hxv = fr.hx(e);
                fr.set_f(X_R, xf(hxv).to_f32());
                let d = fr.store_double(hxv);
                let v = ftol(floor_d(d));
                if fr.i(X_HI) < v {
                    fr.set_i(X_HI, v);
                }
                nx = fr.i(S30);
                if fr.i(ROW) != floor_f(fr.hy(nx)) {
                    break;
                }
            }
            let dx = xf(fr.hx(nx)).fsub_m(fr.hx(e));
            let dy = xf(fr.hy(nx)).fsub_m(fr.hy(e));
            fr.set_f(Y_MAX_SLOPE_R, dx.fdiv(dy).to_f32());
            let nr = X87::from_f64_exact(ceil_d(f64::from(fr.hy(e))))
                .fsub_m(fr.hy(e))
                .fmul_m(fr.f(Y_MAX_SLOPE_R))
                .fadd_m(fr.f(X_R));
            fr.set_f(X_R, nr.to_f32());
            if !c0(xf(fr.f(Y_MAX_SLOPE_R)), zero) {
                let d = fr.store_double(fr.f(X_R));
                let v = ftol(floor_d(d));
                if fr.i(X_HI) < v {
                    fr.set_i(X_HI, v);
                }
            }
        }
        if fr.i(ROW) > fr.i(S34) {
            break;
        }
    }
    flush_extra(scan);
}

/// Phase 5 (0x72D593-0x72D5C0): `scan(X[i], Y[i])` while `i < count` (re-read after each call), then count = 0.
fn flush_extra(scan: u32) {
    let cnt = GPtr::<i32>::from_addr(EXTRA_COUNT);
    let mut i: u32 = 0;
    while (i as i32) < cnt.read() {
        let y = GPtr::<i32>::from_addr((EXTRA_Y + i * 4) as usize).read();
        let x = GPtr::<i32>::from_addr((EXTRA_X + i * 4) as usize).read();
        call_scan(scan, x, y);
        i += 1;
    }
    cnt.write(0);
}

pub const HOOKS: &[HookReg] = &[HOOK_WORLD_SCAN];
