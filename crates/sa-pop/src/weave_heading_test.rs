//! WRITER-NATIVE (batch `native-15`): 0x421A50, the heading test of `CCarCtrl::WeaveForOtherCar`'s turn loops
//! (unnamed, unreversed in gta-reversed). Ported from the 1.0 US bytes; x87 arithmetic follows the original's
//! instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;


orig! {
    /// `CCollision::Test2DLineAgainst2DLine(x1, y1, dx1, dy1, x2, y2, dx2, dy2)` (0x4138D0, AL)
    #[allow(clippy::too_many_arguments)]
    fn test_2d_line_against_2d_line(x1: f32, y1: f32, dx1: f32, dy1: f32, x2: f32, y2: f32, dx2: f32, dy2: f32) -> u8 = cdecl 0x4138D0 => unported;
    /// `VectorAdd(CVector* out, const CVector* a, const CVector* b)` (0x40FE30, returns `out`)
    fn vector_add(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x40FE30 => unported;
}

fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `(a - b)` rounded to f32 (`fld a; fsub b; fstp`).
fn sub(a: f32, b: f32) -> f32 {
    x(a).fsub_m(b).to_f32()
}

/// `(a + b)` rounded to f32 (`fld a; fadd b; fstp`).
fn add(a: f32, b: f32) -> f32 {
    x(a).fadd_m(b).to_f32()
}

/// The four edge tests of one quad `b -> c -> d -> e -> b` (each edge as start + delta) against the line
/// `(p, dir)`; true at the first hit.
fn quad_hit(p: GPtr<f32>, dir: GPtr<f32>, corners: &[[f32; 2]; 4]) -> bool {
    let (px, py) = (p.wrapping_index(0).read(), p.wrapping_index(1).read());
    let (dx, dy) = (dir.wrapping_index(0).read(), dir.wrapping_index(1).read());
    for k in 0..4 {
        let a = corners[k];
        let b = corners[(k + 1) % 4];
        let (ex, ey) = (sub(b[0], a[0]), sub(b[1], a[1]));
        if test_2d_line_against_2d_line(px, py, dx, dy, a[0], a[1], ex, ey) != 0 {
            return true;
        }
    }
    false
}

/// The weave heading test (0x421A50, cdecl, bool in AL): `(float angle, CVector* p1, CVector* d1, CVector* p2,
/// CVector* d2, CVector* p3, CVector* d3, CVector* p4, CVector* d4, float vx, float vy, float speed, bool swap)`.
///
/// `A = ((vx - cos(angle) * speed) * 100, (vy - sin(angle) * speed) * 100)` (x87, rounded once). Without `swap`
/// the lines `(p1, d1)` and `(p2, d2)` are tested against the quads built from `(p3, d3)` and `(p4, d4)`; with
/// it the roles swap and `A` is negated. Each quad from `(p, d)`: `b = p`, `c = p + d`, then `b + A` and
/// `c + A`, walked `b -> c -> b + A -> c + A -> b` (the original's order: two of the edges cross the quad).
/// The first quad's `c` is `d + p` (x87), the second's comes from `VectorAdd(p, d)`. Every edge is tested with
/// `Test2DLineAgainst2DLine(line, edge start, edge end - edge start)`: first quad against both lines, then the
/// second quad; true at the first hit. The original keeps the edge deltas in its argument slots.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x421A50, conv = cdecl, name = "FUN_00421a50",
       expect = "83 EC 3C D9 44 24 40",
       unverified = "native-15", route = "L2", hazards = "x87-transcendental,stack-residue,ax-al-return",
       recs = "grove-walk,car-ai",
       note = "fsin/fcos; rewrites its argument slots (swap case and edge deltas)")]
pub fn weave_heading_blocked(
    angle: f32,
    p1: GPtr<f32>,
    d1: GPtr<f32>,
    p2: GPtr<f32>,
    d2: GPtr<f32>,
    p3: GPtr<f32>,
    d3: GPtr<f32>,
    p4: GPtr<f32>,
    d4: GPtr<f32>,
    vx: f32,
    vy: f32,
    speed: f32,
    swap: u32,
) -> u8 {
    let k = f32::from_bits(0x42C8_0000);
    let mut ax = x(angle).fcos().fmul_m(speed).fsubr_m(vx).fmul_m(k).to_f32();
    let mut ay = x(angle).fsin().fmul_m(speed).fsubr_m(vy).fmul_m(k).to_f32();
    // (line 1, line 2, quad 1, quad 2)
    let (l1, l2, q1, q2) = if swap as u8 != 0 {
        ax = x(ax).fchs().to_f32();
        ay = x(ay).fchs().to_f32();
        ((p3, d3), (p4, d4), (p1, d1), (p2, d2))
    } else {
        ((p1, d1), (p2, d2), (p3, d3), (p4, d4))
    };
    let rd2 = |v: GPtr<f32>| [v.wrapping_index(0).read(), v.wrapping_index(1).read()];
    // first quad: c = d + p (x87 adds, y as p.y + d.y)
    let (qp, qd) = (rd2(q1.0), rd2(q1.1));
    let b = qp;
    let c = [add(qd[0], qp[0]), add(qp[1], qd[1])];
    let corners1 = [b, c, [add(ax, b[0]), add(ay, b[1])], [add(c[0], ax), add(c[1], ay)]];
    if quad_hit(l1.0, l1.1, &corners1) || quad_hit(l2.0, l2.1, &corners1) {
        return 1;
    }
    // second quad: c = VectorAdd(p, d)
    let b = rd2(q2.0);
    let mut sum = [0f32; 3];
    vector_add(GPtr::from_raw(sum.as_mut_ptr()), q2.0, q2.1);
    let c = [sum[0], sum[1]];
    let corners2 = [b, c, [add(ax, b[0]), add(ay, b[1])], [add(ax, c[0]), add(ay, c[1])]];
    u8::from(quad_hit(l1.0, l1.1, &corners2) || quad_hit(l2.0, l2.1, &corners2))
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_WEAVE_HEADING_BLOCKED];
