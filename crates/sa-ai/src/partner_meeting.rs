//! `CTaskComplexPartner::ComputeMeetingPositions` (batch `std8-ai-1`), ported from the 1.0 US bytes (not
//! reversed in gta-reversed). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{X87, rd, wr, x};

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
}

/// `0.7f` (0x858CB0): the minimum 2D distance of the meeting point from either ped
const MIN_DIST: u32 = 0x858CB0;
/// `0.75f` (0x858F34)
const PUSH_BASE: u32 = 0x858F34;
/// `0.5f` (0x858B8C)
const HALF: u32 = 0x858B8C;

fn kf(a: u32) -> f32 {
    GPtr::<f32>::from_addr(a as usize).read()
}

/// The entity's position: its matrix's (+0x14, +0x30) or the placement's (+4).
fn pos_of(e: GPtr<u8>) -> [f32; 3] {
    let m = rd::<GPtr<u8>>(e, 0x14);
    let p = if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

/// `sqrt(dx*dx + dy*dy)` of the x87 values `m - p` (`fsub m32`), as the loop computes it.
fn dist2d(mx: X87, my: X87, px: f32, py: f32) -> X87 {
    let (dx, dy) = (mx.fsub_m(px), my.fsub_m(py));
    dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt()
}

/// `CTaskComplexPartner::ComputeMeetingPositions(CPed* ped, CVector* outPed, CVector* outPartner)` (0x681FE0,
/// thiscall `ret 0xC`; +0x38 the partner, +0x3C the meeting fraction, +0x5C the "no room" flag).
///
/// `P` = the ped's position, `Q` = the partner's, `D = f32(Q - P)`; the meeting point
/// `M = (D.x*k + P.x, f32(D.y*k) + P.y, f32(D.z*k) + P.z)` (`k` = +0x3C, stored as f32); `D` normalised. Unless the
/// task type is 0x4B4, up to 11 passes move `M` (kept in x87 registers) away from either ped closer than 0.7 in 2D
/// (`M += D * (0.75 - d)` from `P`, `M -= D * (0.75 - d)` from `Q`; the x and y offsets as f32, z not) until a pass
/// moves nothing; after 11 passes +0x5C = 1 and nothing is written. With `h = D * 0.5`: `*outPed = (M.x - h.x,
/// M.y - f32(h.y), f32(M.z) - f32(h.z))`, `*outPartner = (f32(h.x) + M.x, f32(h.y) + f32(M.y), f32(h.z) +
/// f32(M.z))` (each stored as f32).
#[hook(addr = 0x681FE0, conv = thiscall, name = "(missed by ghidra: cat7-call)",
       expect = "83 EC 50 53 56 8B F1", route = "L2", hazards = "x87-transcendental",
       recs = "grove-walk", note = "FSQRT per pass; the meeting point lives in x87 registers across the passes")]
pub fn partner_compute_meeting_positions(
    this: GPtr<u8>,
    ped: GPtr<u8>,
    out_ped: GPtr<u8>,
    out_partner: GPtr<u8>,
) {
    let p = pos_of(ped);
    let q = pos_of(rd::<GPtr<u8>>(this, 0x38));
    let mut d = [
        x(q[0]).fsub_m(p[0]).to_f32(),
        x(q[1]).fsub_m(p[1]).to_f32(),
        x(q[2]).fsub_m(p[2]).to_f32(),
    ];
    let k = x(rd::<f32>(this, 0x3C));
    let s1c = x(d[0]).fmul(k).fadd_m(p[0]).to_f32();
    let s44 = x(d[1]).fmul(k).to_f32();
    let s48 = x(d[2]).fmul(k).to_f32();
    let mut s20 = x(s44).fadd_m(p[1]).to_f32();
    let mut s24 = x(s48).fadd_m(p[2]).to_f32();
    vector_normalise(GPtr::from_raw(d.as_mut_ptr()).cast());
    let mut mx = x(s1c);
    let mut my = x(s20);
    if v_task_type(this) != 0x4B4 {
        let mut mz = x(s24);
        let (min, base) = (kf(MIN_DIST), kf(PUSH_BASE));
        let mut pass = 0;
        loop {
            if pass > 10 {
                wr::<u8>(this, 0x5C, 1);
                return;
            }
            let mut moved_from_ped = false;
            let d1 = dist2d(mx, my, p[0], p[1]);
            if !d1.fcom(x(min)).test_parity(5) {
                let e = d1.fsubr_m(base);
                moved_from_ped = true;
                let ox = x(d[0]).fmul(e).to_f32();
                let oy = x(d[1]).fmul(e).to_f32();
                let oz = e.fmul_m(d[2]);
                mx = mx.fadd(x(ox));
                my = my.fadd(x(oy));
                mz = mz.fadd(oz);
            }
            let d2 = dist2d(mx, my, q[0], q[1]);
            let mut done = false;
            if !d2.fcom(x(min)).test_parity(5) {
                let e = d2.fsubr_m(base);
                let ox = x(d[0]).fmul(e).to_f32();
                let oy = x(d[1]).fmul(e).to_f32();
                let oz = e.fmul_m(d[2]);
                mx = mx.fsub_m(ox);
                my = my.fsub_m(oy);
                mz = mz.fsub(oz);
            } else if !moved_from_ped {
                done = true;
            }
            pass += 1;
            if done {
                break;
            }
        }
        s24 = mz.to_f32();
        s20 = my.to_f32();
    }
    let half = kf(HALF);
    let hx = x(d[0]).fmul_m(half);
    let hxf = hx.to_f32();
    let hyf = x(d[1]).fmul_m(half).to_f32();
    let hzf = x(d[2]).fmul_m(half).to_f32();
    wr::<f32>(out_ped, 0, mx.fsub(hx).to_f32());
    wr::<f32>(out_ped, 4, my.fsub_m(hyf).to_f32());
    wr::<f32>(out_ped, 8, x(s24).fsub_m(hzf).to_f32());
    wr::<f32>(out_partner, 0, x(hxf).fadd(mx).to_f32());
    wr::<f32>(out_partner, 4, x(hyf).fadd_m(s20).to_f32());
    wr::<f32>(out_partner, 8, x(hzf).fadd_m(s24).to_f32());
}

pub const HOOKS: &[HookReg] = &[HOOK_PARTNER_COMPUTE_MEETING_POSITIONS];
