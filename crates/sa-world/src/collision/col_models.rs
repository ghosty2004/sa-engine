//! `CCollision::ProcessColModels` (batch `mvpf-coll-1`, part 2): the model-vs-model
//! collision every ped and vehicle runs against what it touches. Ported from the 1.0 US bytes (the
//! entry is a `jmp` into a fragment loading the static-init guard), x87 sequences through the [`Fpu`]
//! model (PC = 24) in the original's operand order; gta-reversed `Collision.cpp` was a reading aid
//! only (it differs from the bytes in several places, see the doc comment). Unverified
//! (docs/verify/HOWTO.md).
//!
//! Function statics (all in `.bss`, raw addresses as the original indexes them): `matAtoB` 0x968988
//! (`Invert(B) * A`), `matBtoA` 0x968940 (`Invert(A) * B`), A's spheres in B's space 0x967F40
//! (0x14 bytes each), their indices 0x967D40, B's spheres (and disks) in A's space 0x967340, their
//! indices 0x967140, B's box indices 0x967040 (64), B's triangle indices 0x9666E0 (0x257, the cap
//! check lets one more through per face group), A's lines in B's space 0x9664D0 (0x20 each), the line
//! index list 0x966490, the per-line hit bytes 0x9666D0, A's triangle indices 0x965B30, A's box
//! indices 0x965A30 (no cap). Guard dword 0x9689D0.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::fpu::Fpu;
use super::mvpc_coll1::local;
use crate::world::common::{g, rd, wr};

const GUARD: usize = 0x9689D0;
const MAT_A_TO_B: usize = 0x968988;
const MAT_B_TO_A: usize = 0x968940;
const SPH_A: usize = 0x967F40;
const COLL_SPH_A: usize = 0x967D40;
const SPH_B: usize = 0x967340;
const COLL_SPH_B: usize = 0x967140;
const COLL_BOX_B: usize = 0x967040;
const COLL_TRI_B: usize = 0x9666E0;
const LINES_IN_B: usize = 0x9664D0;
const LINE_IDX: usize = 0x966490;
const LINE_HIT: usize = 0x9666D0;
const COLL_TRI_A: usize = 0x965B30;
const COLL_BOX_A: usize = 0x965A30;
/// `1e24f` (0x6753C21C), the "no touch yet" distance.
const NO_TOUCH: u32 = 0x6753C21C;
/// `-1.0f`, the "unused" colpoint depth.
const DEPTH_UNUSED: u32 = 0xBF800000;
/// f32 at 0x858C1C (`-1.0f`): the normal flip of B-vs-A points.
const K_FLIP: usize = 0x858C1C;
const CP: isize = 0x2C;

use sa_core::bindings::atexit;
use sa_core::bindings::matrix_assign;
use sa_core::bindings::matrix_invert;
use sa_core::bindings::matrix_mul_vec;
use crate::bindings::multiply3x3;
use crate::bindings::process_line_box;
use crate::bindings::process_line_sphere;
use crate::bindings::process_line_triangle;
use crate::bindings::process_sphere_box;
use crate::bindings::test_sphere_box;
use crate::bindings::test_sphere_triangle;
use sa_core::bindings::matrix_mul_assign;
use crate::bindings::calculate_triangle_planes_416330 as calculate_triangle_planes;
orig! {
    /// `CCollision::ProcessSphereSphere(const CColSphere&, const CColSphere&, CColPoint&, float&)` (0x416450, AL)
    fn process_sphere_sphere(a: GPtr<f32>, b: GPtr<f32>, cp: GPtr<f32>, max: GPtr<f32>) -> u8
        = cdecl 0x416450 => super::prim::process_sphere_sphere;
    /// `CCollision::ProcessSphereTriangle(sphere, verts, tri, plane, CColPoint&, float&)` (0x416BA0, AL)
    fn process_sphere_triangle(sp: GPtr<u8>, verts: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>, cp: GPtr<u8>, min: GPtr<u8>) -> u8
        = cdecl 0x416ba0 => super::mvpe_coll1::process_sphere_triangle;
    /// `CCollision::ProcessDiscCollision(tri, matBA, disk, diskCP, lineCollision, lineRatio, lineCP)` (0x413960, AL)
    fn process_disc_collision(tri: GPtr<f32>, mat: GPtr<f32>, disk: GPtr<f32>, disk_cp: GPtr<f32>, line_collision: GPtr<u8>,
        line_ratio: GPtr<f32>, line_cp: GPtr<f32>) -> u8
        = cdecl 0x413960 => super::prim::process_disc_collision;
}
use crate::bindings::col_point_assign;

use sa_core::field::pa as p;

fn idx(base: usize, i: i32) -> i32 {
    g::<i32>(base + (i as isize as usize).wrapping_mul(4)).read()
}

fn set_idx(base: usize, i: i32, v: i32) {
    g::<i32>(base + (i as isize as usize).wrapping_mul(4)).write(v);
}

/// `[base + i*0x14]`
fn sph(base: usize, i: i32) -> GPtr<u8> {
    p(base).byte_add(i as isize * 0x14)
}

use sa_core::field::copy3;

/// Copies the 3 surface bytes (word + byte).
fn copy_surf(dst: GPtr<u8>, src: GPtr<u8>) {
    wr::<u16>(dst, 0, rd::<u16>(src, 0));
    wr::<u8>(dst, 2, rd::<u8>(src, 2));
}

/// `out = m * v` through a stack temporary, then copied to `out` (the original's `operator*` into a
/// local followed by three dword moves).
fn xform_point(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) {
    let mut t = [0u32; 3];
    let r = matrix_mul_vec(local(&mut t), m, v);
    copy3(out, r);
}

/// `out = Multiply3x3(m, v)` through a stack temporary.
fn xform_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) {
    let mut t = [0u32; 3];
    let r = multiply3x3(local(&mut t), m, v);
    copy3(out, r);
}

/// `fld a; fadd b` (`add`) or `fld a; fsub b`, then `fcomp m`.
fn cmp_op(a: f32, b: f32, add: bool, m: f32) -> Fpu {
    let mut f = Fpu::new();
    f.fld(a);
    if add {
        f.fadd_m(b);
    } else {
        f.fsub_m(b);
    }
    f.fcomp_m(m);
    f
}

/// The original's inline sphere-vs-box overlap (`c + r` against `min` with `test ah, 5; jnp`, `c - r`
/// against `max` with `test ah, 0x41; je`, x then y then z): false as soon as `c + r < min` or
/// `c - r > max` on an axis (NaN passes). `rad_first` selects `fld r; fadd c` (B's spheres) over
/// `fld c; fadd r` (the same value).
fn sphere_box_inline(c: [f32; 3], r: f32, bx: GPtr<u8>) -> bool {
    for k in 0..3isize {
        if !cmp_op(c[k as usize], r, true, rd::<f32>(bx, 4 * k)).jp(5) {
            return false;
        }
        if !cmp_op(c[k as usize], r, false, rd::<f32>(bx, 0xC + 4 * k)).jne(0x41) {
            return false;
        }
    }
    true
}

use sa_core::field::rd3f as v3;

/// `normal = normal * [0x858C1C]` per component through f32 stores (x, y, z), written back after all
/// three are computed (the original stores x and y before computing z, which reads only z).
fn flip_normal(cp: GPtr<u8>) {
    let k = g::<f32>(K_FLIP).read();
    let n = cp.byte_add(0x10);
    for o in [0isize, 4, 8] {
        let mut f = Fpu::new();
        f.fld(rd::<f32>(n, o));
        f.fmul_m(k);
        wr::<f32>(n, o, f.fstp());
    }
}

/// First-call initialisation of the function statics (guard 0x9689D0 bits 0..4): bits 0..2 only set;
/// bit 3 clears `matAtoB`'s attach fields (0x9689C8, 0x9689CC) and registers its destructor 0x856020;
/// bit 4 the same for `matBtoA` (0x968980, 0x968984, 0x856010).
fn statics_init() {
    let gd = g::<u32>(GUARD);
    for bit in [1u32, 2, 4] {
        if gd.read() & bit == 0 {
            gd.write(gd.read() | bit);
        }
    }
    if gd.read() & 8 == 0 {
        gd.write(gd.read() | 8);
        g::<u32>(0x9689C8).write(0);
        g::<u32>(0x9689CC).write(0);
        atexit(0x856020);
    }
    if gd.read() & 0x10 == 0 {
        gd.write(gd.read() | 0x10);
        g::<u32>(0x968980).write(0);
        g::<u32>(0x968984).write(0);
        atexit(0x856010);
    }
}

/// `CCollision::ProcessColModels(const CMatrix& transformA, CColModel& cmA, const CMatrix& transformB,
/// CColModel& cmB, CColPoint* sphereCPs, CColPoint* lineCPs, float* maxTouch, bool returnAll)`
/// (0x4185C0, EAX = number of sphere colpoints).
///
/// Statics initialised; no data on either model → 0.
/// `matAtoB = Invert(B) * A`, A's bounding sphere moved into B's space must touch B's box
/// (`TestSphereBox`) or 0. `matBtoA = Invert(A) * B`. A's spheres → B's space (radius, centre,
/// surface), those touching B's box (`TestSphereBox`) listed; B's spheres → A's space, listed by the
/// inline overlap test against A's box. Nothing listed for A, no A lines and nothing for B → 0. B's
/// disks (B +7 bit 0 with lines) are moved into A's space after B's spheres, and the ones overlapping
/// A's box are written to the B list FROM INDEX 0 as raw disk indices, without raising the B count
/// (the original's bug; their number is kept and subtracted from the B count in the B-vs-A pass). B's
/// boxes overlapping A's bounding sphere (inline test, `c.x + r` stored as f32) are listed (cap 64);
/// `CalculateTrianglePlanes(B)`; B's triangles touching A's bounding sphere (`TestSphereTriangle`) are
/// listed through the face groups (bbox inline test; the 0x257 cap only ends the current group) or all
/// in order (cap 0x257). Nothing for B → 0.
///
/// A vs B: `sphereCPs[0].depth = -1`; per listed A sphere, minimum `1e24`: listed B spheres (an index
/// below B's sphere count is a sphere, otherwise a disk when B has disks with lines, else skipped)
/// through `ProcessSphereSphere`, boxes through `ProcessSphereBox` (the point gets the A sphere's
/// surface as surface A), triangles through `ProcessSphereTriangle`; with `returnAll`, an A piece <= 2
/// and fewer than 31 points each box/triangle hit takes its own point (next depth -1, minimum reset);
/// otherwise the point is kept and finished after the sphere (at 31 points the pass ends).
///
/// A's disks (A +7 bit 0, the monster truck's wheels): see [`disk_pass`]. A's lines (no disks): moved into B's space; per line (hit byte cleared) `ProcessLineSphere` against
/// the listed B sphere indices (as spheres, no disk check), `ProcessLineBox`, `ProcessLineTriangle`
/// (no poly) into `lineCPs[i]` with `maxTouch[i]`; a hit line's point and normal go to world space
/// (`transformB`). The sphere points so far go to world space (`transformB`).
///
/// B vs A (only with B spheres listed and A having triangles or boxes): B's bounding sphere moved into
/// A's space; `CalculateTrianglePlanes(A)` and A's triangles touching it listed (cap 0x257); A's boxes
/// overlapping it listed (inline test with `c.x + r` kept on the x87 stack; no cap). Per listed B
/// sphere (count minus the disks written above): all A triangles into one point (`ProcessSphereTriangle`,
/// minimum `1e24`), a hit flips the normal (`* -1`) and takes a new slot unless 31 points (then the
/// triangle part ends); then per B sphere all A boxes (`ProcessSphereBox`), each hit a new point with
/// surface A = the box's, surface B = the sphere's, normal flipped (at 31 points the next sphere
/// follows). These new points go to world space (`transformA`) with surfaces A and B swapped.
#[hook(addr = 0x4185c0, conv = cdecl, name = "CCollision::ProcessColModels", expect = "E9 D8 96 FE FF 81 EC F0 00 00 00",
       unverified = "frontier2-a23", route = "L2", hazards = "heap-writes", recs = "grove-walk,driving",
       note = "writes the function statics 0x965A30..0x9689D0 and the callers' colpoint arrays; the A-disk path also the disks' centre z")]
#[allow(clippy::too_many_arguments)]
pub fn process_col_models(ta: GPtr<u8>, cma: GPtr<u8>, tb: GPtr<u8>, cmb: GPtr<u8>, sphere_cps: GPtr<u8>,
    line_cps: GPtr<u8>, max_touch: GPtr<u8>, return_all: u8) -> i32 {
    statics_init();
    let cdb = rd::<GPtr<u8>>(cmb, 0x2C);
    let cda = rd::<GPtr<u8>>(cma, 0x2C);
    if cda.is_null() || cdb.is_null() {
        return 0;
    }
    let a2b = p(MAT_A_TO_B);
    let b2a = p(MAT_B_TO_A);
    let r = matrix_invert(tb, a2b);
    matrix_assign(a2b, r);
    matrix_mul_assign(a2b, ta);
    // A's bounding sphere in B's space: centre S+0x58, radius S+0x64 (surface bytes not written)
    let mut bsa = [0u32; 5];
    let bound_a = local(&mut bsa);
    wr::<u32>(bound_a, 0xC, rd::<u32>(cma, 0x24));
    xform_point(bound_a, a2b, cma.byte_add(0x18));
    if test_sphere_box(bound_a.cast(), cmb.cast()) == 0 {
        return 0;
    }
    let r = matrix_invert(ta, b2a);
    matrix_assign(b2a, r);
    matrix_mul_assign(b2a, tb);
    // A's spheres into B's space
    let mut i = 0i32;
    while i < rd::<i16>(cda, 0) as i32 {
        let src = rd::<GPtr<u8>>(cda, 8).byte_add(i as isize * 0x14);
        let d = sph(SPH_A, i);
        wr::<u32>(d, 0xC, rd::<u32>(src, 0xC));
        xform_point(d, a2b, src);
        copy_surf(d.byte_add(0x10), rd::<GPtr<u8>>(cda, 8).byte_add(i as isize * 0x14 + 0x10));
        i += 1;
    }
    let mut n_sph_a = 0i32;
    let mut i = 0i32;
    while i < rd::<i16>(cda, 0) as i32 {
        if test_sphere_box(sph(SPH_A, i).cast(), cmb.cast()) != 0 {
            set_idx(COLL_SPH_A, n_sph_a, i);
            n_sph_a += 1;
        }
        i += 1;
    }
    // B's spheres into A's space
    let mut i = 0i32;
    while i < rd::<i16>(cdb, 0) as i32 {
        let src = rd::<GPtr<u8>>(cdb, 8).byte_add(i as isize * 0x14);
        let d = sph(SPH_B, i);
        wr::<u32>(d, 0xC, rd::<u32>(src, 0xC));
        xform_point(d, b2a, src);
        copy_surf(d.byte_add(0x10), rd::<GPtr<u8>>(cdb, 8).byte_add(i as isize * 0x14 + 0x10));
        i += 1;
    }
    let mut n_sph_b = 0i32;
    let mut i = 0i32;
    while i < rd::<i16>(cdb, 0) as i32 {
        let s = sph(SPH_B, i);
        if sphere_box_inline(v3(s), rd::<f32>(s, 0xC), cma) {
            set_idx(COLL_SPH_B, n_sph_b, i);
            n_sph_b += 1;
        }
        i += 1;
    }
    if n_sph_a == 0 && rd::<u8>(cda, 6) == 0 && n_sph_b == 0 {
        return 0;
    }
    // B's disks
    let mut n_disk_b = 0i32;
    if rd::<u8>(cdb, 7) & 1 != 0 && rd::<u8>(cdb, 6) != 0 {
        let mut i = 0i32;
        while i < rd::<i8>(cdb, 6) as i32 {
            let disk = rd::<GPtr<u8>>(cdb, 0x10).byte_add(i as isize * 0x24);
            let k = rd::<i16>(cdb, 0) as i32 + i;
            wr::<u32>(sph(SPH_B, k), 0xC, rd::<u32>(disk, 0xC));
            xform_point(sph(SPH_B, k), b2a, disk);
            let k = rd::<i16>(cdb, 0) as i32 + i;
            copy_surf(sph(SPH_B, k).byte_add(0x10), rd::<GPtr<u8>>(cdb, 0x10).byte_add(i as isize * 0x24 + 0x10));
            i += 1;
        }
        let mut i = 0i32;
        while i < rd::<i8>(cdb, 6) as i32 {
            let s = sph(SPH_B, rd::<i16>(cdb, 0) as i32 + i);
            if sphere_box_inline(v3(s), rd::<f32>(s, 0xC), cma) {
                set_idx(COLL_SPH_B, n_disk_b, i);
                n_disk_b += 1;
                if n_disk_b >= 0x80 {
                    break;
                }
            }
            i += 1;
        }
    }
    // B's boxes against A's bounding sphere
    let c = v3(bound_a);
    let rad = rd::<f32>(bound_a, 0xC);
    let mut n_box_b = 0i32;
    if rd::<i16>(cdb, 2) > 0 {
        let mut f = Fpu::new();
        f.fld(c[0]);
        f.fadd_m(rad);
        let sx = f.fstp();
        let mut i = 0i32;
        while i < rd::<i16>(cdb, 2) as i32 {
            let bx = rd::<GPtr<u8>>(cdb, 0xC).byte_add(i as isize * 0x1C);
            let mut f = Fpu::new();
            f.fld(sx);
            f.fcomp_m(rd::<f32>(bx, 0));
            let pass = f.jp(5)
                && cmp_op(c[0], rad, false, rd::<f32>(bx, 0xC)).jne(0x41)
                && cmp_op(c[1], rad, true, rd::<f32>(bx, 4)).jp(5)
                && cmp_op(c[1], rad, false, rd::<f32>(bx, 0x10)).jne(0x41)
                && cmp_op(c[2], rad, true, rd::<f32>(bx, 8)).jp(5)
                && cmp_op(c[2], rad, false, rd::<f32>(bx, 0x14)).jne(0x41);
            if pass {
                set_idx(COLL_BOX_B, n_box_b, i);
                n_box_b += 1;
                if n_box_b >= 0x40 {
                    break;
                }
            }
            i += 1;
        }
    }
    // B's triangles against A's bounding sphere
    calculate_triangle_planes(cdb.cast());
    // [esp+0x54]: B's plane array as it stands after CalculateTrianglePlanes (read by the disk pass)
    let planes_b = rd::<GPtr<u8>>(cdb, 0x1C);
    let mut n_tri_b = 0i32;
    if rd::<i16>(cdb, 4) != 0 {
        if rd::<u8>(cdb, 7) & 2 != 0 {
            let mut gi = 0i32;
            let mut off = 0isize;
            loop {
                let n = if rd::<u8>(cdb, 7) & 2 != 0 { rd::<i32>(rd::<GPtr<u8>>(cdb, 0x18), -4) } else { 0 };
                if gi >= n {
                    break;
                }
                let grp = rd::<GPtr<u8>>(cdb, 0x18).byte_add(-off - 0x20);
                if sphere_box_inline(c, rad, grp) {
                    let mut t = rd::<i16>(grp, 0x18) as i32;
                    while t <= rd::<i16>(grp, 0x1A) as i32 {
                        let plane = rd::<GPtr<u8>>(cdb, 0x1C).byte_add(t as isize * 10);
                        let tri = rd::<GPtr<u8>>(cdb, 0x18).byte_add(t as isize * 8);
                        if test_sphere_triangle(bound_a, rd(cdb, 0x14), tri, plane) != 0 {
                            set_idx(COLL_TRI_B, n_tri_b, t);
                            n_tri_b += 1;
                            if n_tri_b >= 0x257 {
                                break;
                            }
                        }
                        t += 1;
                    }
                }
                gi += 1;
                off += 0x1C;
            }
        } else if rd::<i16>(cdb, 4) > 0 {
            let mut t = 0i32;
            loop {
                let plane = rd::<GPtr<u8>>(cdb, 0x1C).byte_add(t as isize * 10);
                let tri = rd::<GPtr<u8>>(cdb, 0x18).byte_add(t as isize * 8);
                if test_sphere_triangle(bound_a, rd(cdb, 0x14), tri, plane) != 0 {
                    set_idx(COLL_TRI_B, n_tri_b, t);
                    n_tri_b += 1;
                    if n_tri_b >= 0x257 {
                        break;
                    }
                }
                t += 1;
                if t >= rd::<i16>(cdb, 4) as i32 {
                    break;
                }
            }
        }
    }
    if n_sph_b == 0 && n_box_b == 0 && n_tri_b == 0 {
        return 0;
    }
    // A's spheres against B
    let mut n_cps = 0i32;
    wr::<u32>(sphere_cps, 0x28, DEPTH_UNUSED);
    let mut mt = [NO_TOUCH];
    let min_touch = local(&mut mt);
    let mut k = 0i32;
    'spheres: while k < n_sph_a {
        let ia = idx(COLL_SPH_A, k);
        let sa = sph(SPH_A, ia);
        let sa_orig = rd::<GPtr<u8>>(cda, 8).byte_add(ia as isize * 0x14);
        let mut advance = false;
        wr::<u32>(min_touch, 0, NO_TOUCH);
        let mut j = 0i32;
        while j < n_sph_b {
            let ib = idx(COLL_SPH_B, j);
            let sb = if ib < rd::<i16>(cdb, 0) as i32 {
                Some(rd::<GPtr<u8>>(cdb, 8).byte_add(ib as isize * 0x14))
            } else if rd::<u8>(cdb, 7) & 1 != 0 && rd::<u8>(cdb, 6) != 0 {
                Some(rd::<GPtr<u8>>(cdb, 0x10).byte_add((ib - rd::<i16>(cdb, 0) as i32) as isize * 0x24))
            } else {
                None
            };
            if let Some(sb) = sb {
                let cp = sphere_cps.byte_add(n_cps as isize * CP);
                if process_sphere_sphere(sa.cast(), sb.cast(), cp.cast(), min_touch.cast()) != 0 {
                    advance = true;
                }
            }
            j += 1;
        }
        let mut cp = sphere_cps.byte_add(n_cps as isize * CP);
        let mut j = 0i32;
        while j < n_box_b {
            let bx = rd::<GPtr<u8>>(cdb, 0xC).byte_add(idx(COLL_BOX_B, j) as isize * 0x1C);
            if process_sphere_box(sa.cast(), bx.cast(), cp.cast(), min_touch.cast()) != 0 {
                copy_surf(cp.byte_add(0x20), sa_orig.byte_add(0x10));
                if return_all != 0 && rd::<u8>(sa_orig, 0x11) <= 2 && n_cps < 0x1F {
                    n_cps += 1;
                    cp = cp.byte_add(CP);
                    advance = false;
                    wr::<u32>(min_touch, 0, NO_TOUCH);
                    wr::<u32>(cp, 0x28, DEPTH_UNUSED);
                } else {
                    advance = true;
                }
            }
            j += 1;
        }
        let mut cp = sphere_cps.byte_add(n_cps as isize * CP);
        let mut j = 0i32;
        while j < n_tri_b {
            let t = idx(COLL_TRI_B, j);
            let tri = rd::<GPtr<u8>>(cdb, 0x18).byte_add(t as isize * 8);
            let plane = rd::<GPtr<u8>>(cdb, 0x1C).byte_add(t as isize * 10);
            if process_sphere_triangle(sa, rd(cdb, 0x14), tri, plane, cp, min_touch) != 0 {
                if return_all != 0 && rd::<u8>(sa_orig, 0x11) <= 2 && n_cps < 0x1F {
                    n_cps += 1;
                    cp = cp.byte_add(CP);
                    advance = false;
                    wr::<u32>(min_touch, 0, NO_TOUCH);
                    wr::<u32>(cp, 0x28, DEPTH_UNUSED);
                } else {
                    advance = true;
                }
            }
            j += 1;
        }
        if advance {
            if n_cps >= 0x1F {
                break 'spheres;
            }
            n_cps += 1;
            wr::<u32>(sphere_cps.byte_add(n_cps as isize * CP), 0x28, DEPTH_UNUSED);
        }
        k += 1;
    }
    let b = BLists { n_sph: n_sph_b, n_box: n_box_b, n_tri: n_tri_b, planes: planes_b };
    if rd::<u8>(cda, 7) & 1 != 0 {
        n_cps = disk_pass(cda, cdb, cmb, tb, sphere_cps, line_cps, max_touch, n_cps, &b);
    } else {
        line_pass(cda, cdb, a2b, tb, line_cps, max_touch, &b);
    }
    world_pass(cma, cda, cdb, ta, tb, cmb, b2a, sphere_cps, n_cps, n_sph_b, n_disk_b)
}

/// B's pieces listed against A (the function's locals `[esp+0x48]`, `[esp+0x4C]`, `[esp+0x44]`,
/// `[esp+0x54]`).
struct BLists {
    n_sph: i32,
    n_box: i32,
    n_tri: i32,
    planes: GPtr<u8>,
}

/// A's lines (0x4196B2, A without disks).
#[allow(clippy::too_many_arguments)]
fn line_pass(cda: GPtr<u8>, cdb: GPtr<u8>, a2b: GPtr<u8>, tb: GPtr<u8>, line_cps: GPtr<u8>, max_touch: GPtr<u8>,
    b: &BLists) {
    let (n_sph_b, n_box_b, n_tri_b) = (b.n_sph, b.n_box, b.n_tri);
    if rd::<i8>(cda, 6) > 0 {
        let mut i = 0i32;
        while i < rd::<i8>(cda, 6) as i32 {
            let line = rd::<GPtr<u8>>(cda, 0x10).byte_add(i as isize * 0x20);
            let d = p(LINES_IN_B).byte_add(i as isize * 0x20);
            xform_point(d, a2b, line);
            let line = rd::<GPtr<u8>>(cda, 0x10).byte_add(i as isize * 0x20);
            xform_point(d.byte_add(0x10), a2b, line.byte_add(0x10));
            i += 1;
        }
    }
    let mut n_lines = 0i32;
    while n_lines < rd::<i8>(cda, 6) as i32 {
        set_idx(LINE_IDX, n_lines, n_lines);
        n_lines += 1;
    }
    let mut k = 0i32;
    while k < n_lines {
        let hit = g::<u8>(LINE_HIT + k as usize);
        hit.write(0);
        let l = idx(LINE_IDX, k);
        let line = p(LINES_IN_B).byte_add(l as isize * 0x20);
        let lcp = line_cps.byte_add(l as isize * CP);
        let mt = max_touch.byte_add(l as isize * 4);
        let mut j = 0i32;
        while j < n_sph_b {
            let sb = rd::<GPtr<u8>>(cdb, 8).byte_add(idx(COLL_SPH_B, j) as isize * 0x14);
            if process_line_sphere(line.cast(), sb.cast(), lcp.cast(), mt.cast()) != 0 {
                hit.write(1);
            }
            j += 1;
        }
        let mut j = 0i32;
        while j < n_box_b {
            let bx = rd::<GPtr<u8>>(cdb, 0xC).byte_add(idx(COLL_BOX_B, j) as isize * 0x1C);
            if process_line_box(line, bx, lcp, mt) != 0 {
                hit.write(1);
            }
            j += 1;
        }
        let mut j = 0i32;
        while j < n_tri_b {
            let t = idx(COLL_TRI_B, j);
            let tri = rd::<GPtr<u8>>(cdb, 0x18).byte_add(t as isize * 8);
            let plane = rd::<GPtr<u8>>(cdb, 0x1C).byte_add(t as isize * 10);
            if process_line_triangle(line, rd(cdb, 0x14), tri, plane, lcp, mt, GPtr::NULL) != 0 {
                hit.write(1);
            }
            j += 1;
        }
        if hit.read() != 0 {
            let lcp = line_cps.byte_add(idx(LINE_IDX, k) as isize * CP);
            xform_point(lcp, tb, lcp);
            let lcp = line_cps.byte_add(idx(LINE_IDX, k) as isize * CP);
            xform_vec(lcp.byte_add(0x10), tb, lcp.byte_add(0x10));
        }
        k += 1;
    }
}

/// From 0x419968: the sphere points so far into world space (`transformB`), then B's spheres against A.
#[allow(clippy::too_many_arguments)]
fn world_pass(cma: GPtr<u8>, cda: GPtr<u8>, cdb: GPtr<u8>, ta: GPtr<u8>, tb: GPtr<u8>, cmb: GPtr<u8>, b2a: GPtr<u8>,
    sphere_cps: GPtr<u8>, mut n_cps: i32, n_sph_b: i32, n_disk_b: i32) -> i32 {
    // the sphere points so far into world space
    let mut q = 0i32;
    while q < n_cps {
        let cp = sphere_cps.byte_add(q as isize * CP);
        xform_point(cp, tb, cp);
        xform_vec(cp.byte_add(0x10), tb, cp.byte_add(0x10));
        q += 1;
    }
    // B's spheres against A
    let mut n_tri_a = 0i32;
    let mut n_box_a = 0i32;
    let mut n_new = 0i32;
    if n_sph_b > 0 && (rd::<i16>(cda, 4) != 0 || rd::<i16>(cda, 2) != 0) {
        let mut bsb = [0u32; 5];
        let bound_b = local(&mut bsb);
        wr::<u32>(bound_b, 0xC, rd::<u32>(cmb, 0x24));
        xform_point(bound_b, b2a, cmb.byte_add(0x18));
        if rd::<i16>(cda, 4) != 0 {
            let cd = rd::<GPtr<u8>>(cma, 0x2C);
            if !cd.is_null() {
                calculate_triangle_planes(cd.cast());
            }
            let mut t = 0i32;
            while t < rd::<i16>(cda, 4) as i32 {
                let plane = rd::<GPtr<u8>>(cda, 0x1C).byte_add(t as isize * 10);
                let tri = rd::<GPtr<u8>>(cda, 0x18).byte_add(t as isize * 8);
                if test_sphere_triangle(bound_b, rd(cda, 0x14), tri, plane) != 0 {
                    set_idx(COLL_TRI_A, n_tri_a, t);
                    n_tri_a += 1;
                    if n_tri_a >= 0x257 {
                        break;
                    }
                }
                t += 1;
            }
        }
        if rd::<i16>(cda, 2) > 0 {
            let cb = v3(bound_b);
            let rb = rd::<f32>(bound_b, 0xC);
            let mut f = Fpu::new();
            f.fld(cb[0]);
            f.fadd_m(rb);
            let mut i = 0i32;
            while i < rd::<i16>(cda, 2) as i32 {
                let bx = rd::<GPtr<u8>>(cda, 0xC).byte_add(i as isize * 0x1C);
                f.fcom_m(rd::<f32>(bx, 0));
                let pass = f.jp(5)
                    && cmp_op(cb[0], rb, false, rd::<f32>(bx, 0xC)).jne(0x41)
                    && cmp_op(cb[1], rb, true, rd::<f32>(bx, 4)).jp(5)
                    && cmp_op(cb[1], rb, false, rd::<f32>(bx, 0x10)).jne(0x41)
                    && cmp_op(cb[2], rb, true, rd::<f32>(bx, 8)).jp(5)
                    && cmp_op(cb[2], rb, false, rd::<f32>(bx, 0x14)).jne(0x41);
                if pass {
                    set_idx(COLL_BOX_A, n_box_a, i);
                    n_box_a += 1;
                }
                i += 1;
            }
        }
        let mut mt2 = [NO_TOUCH];
        let min2 = local(&mut mt2);
        if n_tri_a != 0 {
            let mut cp = sphere_cps.byte_add(n_cps as isize * CP);
            wr::<u32>(cp, 0x28, DEPTH_UNUSED);
            let count = n_sph_b - n_disk_b;
            let mut k = 0i32;
            'tris: while k < count {
                wr::<u32>(min2, 0, NO_TOUCH);
                let sb = sph(SPH_B, idx(COLL_SPH_B, k));
                let mut adv = false;
                let mut j = 0i32;
                while j < n_tri_a {
                    let t = idx(COLL_TRI_A, j);
                    let plane = rd::<GPtr<u8>>(cda, 0x1C).byte_add(t as isize * 10);
                    let tri = rd::<GPtr<u8>>(cda, 0x18).byte_add(t as isize * 8);
                    if process_sphere_triangle(sb, rd(cda, 0x14), tri, plane, cp, min2) != 0 {
                        adv = true;
                    }
                    j += 1;
                }
                if adv {
                    flip_normal(cp);
                    if n_cps >= 0x1F {
                        break 'tris;
                    }
                    cp = cp.byte_add(CP);
                    n_cps += 1;
                    n_new += 1;
                    wr::<u32>(cp, 0x28, DEPTH_UNUSED);
                }
                k += 1;
            }
        }
        if n_box_a != 0 {
            let count = n_sph_b - n_disk_b;
            let mut k = 0i32;
            while k < count {
                let ib = idx(COLL_SPH_B, k);
                let sb_orig = rd::<GPtr<u8>>(cdb, 8).byte_add(ib as isize * 0x14);
                wr::<u32>(min2, 0, NO_TOUCH);
                let mut cp = sphere_cps.byte_add(n_cps as isize * CP);
                let mut j = 0i32;
                while j < n_box_a {
                    let bx = rd::<GPtr<u8>>(cda, 0xC).byte_add(idx(COLL_BOX_A, j) as isize * 0x1C);
                    let sb = sph(SPH_B, idx(COLL_SPH_B, k));
                    if process_sphere_box(sb.cast(), bx.cast(), cp.cast(), min2.cast()) != 0 {
                        let bx = rd::<GPtr<u8>>(cda, 0xC).byte_add(idx(COLL_BOX_A, j) as isize * 0x1C);
                        copy_surf(cp.byte_add(0x20), bx.byte_add(0x18));
                        copy_surf(cp.byte_add(0x23), sb_orig.byte_add(0x10));
                        flip_normal(cp);
                        if n_cps >= 0x1F {
                            break;
                        }
                        cp = cp.byte_add(CP);
                        n_cps += 1;
                        n_new += 1;
                        wr::<u32>(cp, 0x28, DEPTH_UNUSED);
                    }
                    j += 1;
                }
                k += 1;
            }
        }
    }
    // the B-vs-A points into world space (transformA), surfaces A and B swapped
    let mut q = n_cps - n_new;
    while q < n_cps {
        let cp = sphere_cps.byte_add(q as isize * CP);
        xform_point(cp, ta, cp);
        xform_vec(cp.byte_add(0x10), ta, cp.byte_add(0x10));
        let a = (rd::<u16>(cp, 0x20), rd::<u8>(cp, 0x22));
        wr::<u16>(cp, 0x20, rd::<u16>(cp, 0x23));
        wr::<u8>(cp, 0x22, rd::<u8>(cp, 0x25));
        wr::<u16>(cp, 0x23, a.0);
        wr::<u8>(cp, 0x25, a.1);
        q += 1;
    }
    n_cps
}

/// The wheel-touch height of a disk at a point in A's space (0x4191C9 / 0x419364):
/// `sqrt((r*r - dy*dy) - dx*dx) + p.z` on the x87 stack (no clamp: NaN when the point is outside the
/// disk's circle), `dy = p.y - c.y` and `dx = p.x - c.x` kept extended.
fn disk_height(f: &mut Fpu, pa: [f32; 3], disk: GPtr<u8>) {
    f.fld(pa[1]);
    f.fsub_m(rd::<f32>(disk, 4));
    f.fld(pa[0]);
    f.fsub_m(rd::<f32>(disk, 0));
    f.fld(rd::<f32>(disk, 0xC));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.fsubp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.fsubp(1);
    f.fsqrt();
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fadd_m(pa[2]);
}

/// A line-touch from a sphere or box hit (0x41921F / 0x4193E1): the hit byte set, `maxTouch = h` (pops
/// the height), `lineCP = cp`, its depth re-read through the current depth slot, its surface A the
/// disk's.
fn disk_line_touch(f: &mut Fpu, hit: GPtr<u8>, mt: GPtr<u8>, lcp: GPtr<u8>, cp: GPtr<u8>, depth: GPtr<u8>,
    disk: GPtr<u8>) {
    hit.write(1);
    wr::<f32>(mt, 0, f.fstp());
    col_point_assign(lcp, cp);
    wr::<u32>(lcp, 0x28, rd::<u32>(depth, 0));
    copy_surf(lcp.byte_add(0x20), disk.byte_add(0x10));
}

/// A's disks (0x418F52, A +7 bit 0): the wheels of a monster truck. Each disk centre's z is first
/// overwritten in the live model with its touch distance `maxTouch[i]` (never restored); the disks
/// (radius, centre through `matAtoB`, surface) then go to A's sphere list FROM INDEX 0. Per disk (hit
/// byte cleared, the current point's depth -1), when its sphere overlaps B's box (inline test):
/// B's listed spheres (as spheres, no disk check) through `ProcessSphereSphere` into the current point;
/// a hit's point and normal go to A's space and the disk height `h` (see [`disk_height`]) is a line
/// touch unless `h < maxTouch[i]` (NaN: no touch); the minimum is reset after a hit only. B's listed
/// boxes through `ProcessSphereBox`: a hit below the disk centre with an A-space normal z above 0.5
/// gives `h`; unless `h < maxTouch[i]`, a rise above `0.35 * r` also keeps the point (its minimum
/// recorded, surface A the disk's) and the touch is taken; any other hit keeps the point when its
/// distance is below the kept one; the minimum is reset after every box. B's listed triangles through
/// `ProcessSphereTriangle` into a copy of the current point (taken once, depth -1 per triangle, the
/// minimum never reset): a hit's normal is replaced by the plane's (`n / 4096`) when its z > 0.3 and
/// `transformB`'s at.z > 0.9, then `ProcessDiscCollision` decides (a kept point when it returns 1).
/// A kept point takes a new slot unless 31 points. A touched line's point and normal go to world space
/// (`transformB`), an untouched one gets `maxTouch = -1e8`.
#[allow(clippy::too_many_arguments)]
#[inline(never)] // a gdb count point (tools/e2e/scenarios/frontier2-a13-counts.gdb)
fn disk_pass(cda: GPtr<u8>, cdb: GPtr<u8>, cmb: GPtr<u8>, tb: GPtr<u8>, sphere_cps: GPtr<u8>, line_cps: GPtr<u8>,
    max_touch: GPtr<u8>, mut n_cps: i32, b: &BLists) -> i32 {
    let a2b = p(MAT_A_TO_B);
    let b2a = p(MAT_B_TO_A);
    let mut i = 0i32;
    while i < rd::<i8>(cda, 6) as i32 {
        let disk = rd::<GPtr<u8>>(cda, 0x10).byte_add(i as isize * 0x24);
        wr::<u32>(disk, 8, rd::<u32>(max_touch, i as isize * 4));
        i += 1;
    }
    let mut i = 0i32;
    while i < rd::<i8>(cda, 6) as i32 {
        let d = sph(SPH_A, i);
        let disk = rd::<GPtr<u8>>(cda, 0x10).byte_add(i as isize * 0x24);
        wr::<u32>(d, 0xC, rd::<u32>(disk, 0xC));
        xform_point(d, a2b, disk);
        copy_surf(d.byte_add(0x10), rd::<GPtr<u8>>(cda, 0x10).byte_add(i as isize * 0x24 + 0x10));
        i += 1;
    }
    let mut cp = sphere_cps.byte_add(n_cps as isize * CP);
    let mut depth = cp.byte_add(0x28);
    let k_half = g::<f32>(0x858B8C).read();
    let k_rise = g::<f32>(0x858C0C).read();
    let k_plane = g::<f32>(0x858BFC).read();
    let k_nz = g::<f32>(0x858C24).read();
    let k_atz = g::<f32>(0x858C20).read();
    let mut i = 0i32;
    while i < rd::<i8>(cda, 6) as i32 {
        let hit = g::<u8>(LINE_HIT + i as usize);
        hit.write(0);
        wr::<u32>(depth, 0, DEPTH_UNUSED);
        let sa = sph(SPH_A, i);
        if !sphere_box_inline(v3(sa), rd::<f32>(sa, 0xC), cmb) {
            i += 1;
            continue;
        }
        let lcp = line_cps.byte_add(i as isize * CP);
        let mt = max_touch.byte_add(i as isize * 4);
        let mut kept = false;
        let mut m = [NO_TOUCH, NO_TOUCH];
        let min = local(&mut m);
        let kept_min = min.byte_add(4);
        // B's spheres
        let mut j = 0i32;
        while j < b.n_sph {
            let sb = rd::<GPtr<u8>>(cdb, 8).byte_add(idx(COLL_SPH_B, j) as isize * 0x14);
            if process_sphere_sphere(sa.cast(), sb.cast(), cp.cast(), min.cast()) != 0 {
                let mut pa = [0u32; 3];
                xform_point(local(&mut pa), b2a, cp);
                let mut na = [0u32; 3];
                multiply3x3(local(&mut na), b2a, cp.byte_add(0x10));
                let pa = pa.map(f32::from_bits);
                let disk = rd::<GPtr<u8>>(cda, 0x10).byte_add(i as isize * 0x24);
                let mut f = Fpu::new();
                disk_height(&mut f, pa, disk);
                f.fld_st(0);
                f.fcomp_m(rd::<f32>(mt, 0));
                if f.jne(1) {
                    f.fstp_st(0);
                } else {
                    disk_line_touch(&mut f, hit, mt, lcp, cp, depth, disk);
                }
                wr::<u32>(min, 0, NO_TOUCH);
            }
            j += 1;
        }
        // B's boxes
        let mut j = 0i32;
        while j < b.n_box {
            let bx = rd::<GPtr<u8>>(cdb, 0xC).byte_add(idx(COLL_BOX_B, j) as isize * 0x1C);
            if process_sphere_box(sa.cast(), bx.cast(), cp.cast(), min.cast()) != 0 {
                let mut pa = [0u32; 3];
                xform_point(local(&mut pa), b2a, cp);
                let mut na = [0u32; 3];
                xform_vec(local(&mut na), b2a, cp.byte_add(0x10));
                let pa = pa.map(f32::from_bits);
                let disk = rd::<GPtr<u8>>(cda, 0x10).byte_add(i as isize * 0x24);
                let mut f = Fpu::new();
                f.fld(pa[2]);
                f.fcomp_m(rd::<f32>(disk, 8));
                let mut wheel = !f.jp(5);
                if wheel {
                    f.fld(f32::from_bits(na[2]));
                    f.fcomp_m(k_half);
                    wheel = !f.jne(0x41);
                }
                if wheel {
                    disk_height(&mut f, pa, disk);
                    f.fld_st(0);
                    f.fcomp_m(rd::<f32>(mt, 0));
                    if f.jne(1) {
                        f.fstp_st(0);
                    } else {
                        f.fld_st(0);
                        f.fsub_m(rd::<f32>(mt, 0));
                        f.fld(k_rise);
                        f.fmul_m(rd::<f32>(disk, 0xC));
                        f.fcompp();
                        if !f.jp(5) {
                            wr::<u32>(kept_min, 0, rd::<u32>(min, 0));
                            copy_surf(cp.byte_add(0x20), disk.byte_add(0x10));
                            kept = true;
                        }
                        disk_line_touch(&mut f, hit, mt, lcp, cp, depth, disk);
                    }
                } else {
                    let mut f = Fpu::new();
                    f.fld(rd::<f32>(min, 0));
                    f.fcomp_m(rd::<f32>(kept_min, 0));
                    if !f.jp(5) {
                        wr::<u32>(kept_min, 0, rd::<u32>(min, 0));
                        copy_surf(cp.byte_add(0x20), disk.byte_add(0x10));
                        kept = true;
                    }
                }
            }
            wr::<u32>(min, 0, NO_TOUCH);
            j += 1;
        }
        // B's triangles
        let mut tcp = [0u32; 11];
        for (o, v) in tcp.iter_mut().enumerate() {
            *v = rd::<u32>(cp, o as isize * 4);
        }
        let tri_cp = local(&mut tcp);
        let mut t = 0i32;
        while t < b.n_tri {
            let ti = idx(COLL_TRI_B, t);
            let plane = rd::<GPtr<u8>>(cdb, 0x1C).byte_add(ti as isize * 10);
            let tri = rd::<GPtr<u8>>(cdb, 0x18).byte_add(ti as isize * 8);
            wr::<u32>(tri_cp, 0x28, DEPTH_UNUSED);
            if process_sphere_triangle(sa, rd(cdb, 0x14), tri, plane, tri_cp, min) != 0 {
                let ti = idx(COLL_TRI_B, t);
                let pl = b.planes.byte_add(ti as isize * 10);
                let mut n = [0f32; 3];
                let mut f = Fpu::new();
                for (k, v) in n.iter_mut().enumerate() {
                    f.fild(rd::<i16>(pl, k as isize * 2) as i32);
                    f.fmul_m(k_plane);
                    if k < 2 {
                        *v = f.fstp();
                    } else {
                        *v = f.fst();
                        f.fcomp_m(k_nz);
                    }
                }
                if !f.jne(0x41) {
                    let mut g2 = Fpu::new();
                    g2.fld(rd::<f32>(tb, 0x28));
                    g2.fcomp_m(k_atz);
                    if !g2.jne(0x41) {
                        for (k, v) in n.iter().enumerate() {
                            wr::<f32>(tri_cp, 0x10 + k as isize * 4, *v);
                        }
                    }
                }
                let disk = rd::<GPtr<u8>>(cda, 0x10).byte_add(i as isize * 0x24);
                if process_disc_collision(tri_cp.cast(), b2a.cast(), disk.cast(), cp.cast(), hit, mt.cast(), lcp.cast())
                    != 0
                {
                    kept = true;
                }
            }
            t += 1;
        }
        if kept && n_cps < 0x1F {
            n_cps += 1;
            depth = depth.byte_add(CP);
            cp = depth.byte_add(-0x28);
            wr::<u32>(depth, 0, DEPTH_UNUSED);
        }
        if hit.read() != 0 {
            xform_point(lcp, tb, lcp);
            xform_vec(lcp.byte_add(0x10), tb, lcp.byte_add(0x10));
        } else {
            wr::<u32>(mt, 0, 0xCCBEBC20);
        }
        i += 1;
    }
    n_cps
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COL_MODELS];
