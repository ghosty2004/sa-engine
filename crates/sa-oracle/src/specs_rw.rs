//! L1 specs for PORT-RW-1 (`sa_rwcore`, see `docs/arch/rw-port.md`): the table sqrt / inverse sqrt,
//! vectors, matrices, bounding boxes and the SSE2 transform kernels.
//!
//! The VECTORMODULE tables live in RW's heap in the game. The specs patch `RwEngineInstance`
//! (0xC97B24) to 0 and the module offset globals so that `G + [offset]` points at a module placed in
//! `.data` space the functions never touch otherwise, with both tables (built by the port's
//! `fill_sqrt_table` / `fill_inv_sqrt_table`) behind it; the ports' `_in` variants read the same
//! tables from host memory. MATRIXMODULE gets `+4 = 0x20000` the same way.

use std::sync::LazyLock;

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, Inputs, RetKind::*, Route, Spec};
use sa_core::light::RwV3d;
use sa_core::mem::GPtr;
use sa_rwcore::{matrix, vector};

const G_CELL: u32 = 0xC97B24;
const VEC_OFF: u32 = 0xC97934;
const MAT_OFF: u32 = 0xC979BC;
/// Fake VECTORMODULE (sqrt table ptr, inverse table ptr) and MATRIXMODULE (+4 mask).
const VMOD: u32 = 0xB00000;
const MMOD: u32 = 0xB00020;
const TSQRT: u32 = 0xB00100;
const TINV: u32 = 0xB04100;

static TABLES: LazyLock<(Vec<u32>, Vec<u32>)> = LazyLock::new(|| {
    let mut t = vec![0u32; 4096];
    let mut u = vec![0u32; 4096];
    vector::fill_sqrt_table(GPtr::from_raw(t.as_mut_ptr()));
    vector::fill_inv_sqrt_table(GPtr::from_raw(u.as_mut_ptr()));
    (t, u)
});

fn tsq() -> GPtr<u32> {
    GPtr::from_raw(TABLES.0.as_ptr().cast_mut())
}
fn tinv() -> GPtr<u32> {
    GPtr::from_raw(TABLES.1.as_ptr().cast_mut())
}

fn words(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// Patches for the vector tables (and the matrix module mask).
fn rw_patches() -> Vec<(u32, Vec<u8>)> {
    vec![
        (G_CELL, words(&[0])),
        (VEC_OFF, words(&[VMOD])),
        (MAT_OFF, words(&[MMOD])),
        (VMOD, words(&[TSQRT, TINV])),
        (MMOD, words(&[0, 0x20000])),
        (TSQRT, words(&TABLES.0)),
        (TINV, words(&TABLES.1)),
    ]
}

fn rw_reads() -> Vec<(u32, u32)> {
    vec![
        (G_CELL, G_CELL + 4),
        (VEC_OFF, VEC_OFF + 4),
        (MAT_OFF, MAT_OFF + 4),
        (VMOD, VMOD + 8),
        (MMOD, MMOD + 8),
        (TSQRT, TSQRT + 0x4000),
        (TINV, TINV + 0x4000),
    ]
}

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn eax(c: &Ctx, r: usize) -> ExpRet {
    ExpRet::Eax(c.addr(r))
}

/// A combine op 0..=2.
fn g_op3(r: &mut Rng) -> u32 {
    r.below(3)
}

/// A vertex / point count 1..=8.
fn g_n8(r: &mut Rng) -> u32 {
    1 + r.below(8)
}

/// A float argument word: mostly a float, sometimes any bit pattern (the lookups take raw bits).
fn g_word(r: &mut Rng) -> u32 {
    r.next_u32()
}

/// `RwV3dNormalize`'s error path calls `_rwerror`/`RwErrorSet` (RW error state, beyond L1): inputs
/// whose length sum is +0.0 get x = 1.
fn no_zero_len(i: &mut Inputs, l: &Layout) {
    let s = |i: &Vec<u32>, k: usize| f32::from_bits(i[l.word_slot[1][k].unwrap()]);
    let v = RwV3d {
        x: s(i, 0),
        y: s(i, 1),
        z: s(i, 2),
    };
    if vector::len_sq(v).to_f32().to_bits() == 0 {
        i[l.word_slot[1][0].unwrap()] = 1.0f32.to_bits();
    }
}

/// Unicorn's FSIN/FCOS are QEMU's (host `sin`/`cos` of the double, an infinite operand left
/// unchanged); the CPU reduces against its 66-bit pi and returns the indefinite for infinity (the
/// `X87` model follows the CPU, checked by sa-oracle's x87 specs). Angles are kept finite and within
/// +-3600 degrees, where both agree after the f32 store.
fn finite_angle(i: &mut Inputs, l: &Layout) {
    let s = l.arg_slot[2].unwrap();
    let a = f32::from_bits(i[s]);
    if a.is_infinite() {
        i[s] = 0;
    } else if a.abs() > 3600.0 {
        i[s] = (a % 3600.0).to_bits();
    }
}

// The SSE2 transform kernels (0x80D440..0x80D600) have no L1 spec: Unicorn does not flush SSE
// results under MXCSR.FZ (the game runs with FZ on) and picks between two NaN operands by the x87 rule,
// and the hazard scanner has no SSE/FZ hazard to route them to L2. Their parity comes from L2
// (docs/arch/rw-port.md).

pub fn all() -> Vec<Spec> {
    let mut rot_omcs = b(
        "RwMatrixRotateOneMinusCosineSineReplace",
        "RwMatrixRotateOneMinusCosineSine",
        0x7F1D00,
        Cdecl,
        Eax,
    )
    .regions(vec![rwmat(), vec3()])
    .args(vec![A::P(0), A::P(1), A::F, A::F, A::K(0)])
    .patch(rw_patches())
    .reads(rw_reads());
    // The local's pads are uninitialised stack in the original (written as 0 by the port).
    rot_omcs.ignore.extend([(0, 28), (0, 44), (0, 60)]);
    let mut rot = b("RwMatrixRotateReplace", "RwMatrixRotate", 0x7F1FD0, Cdecl, Eax)
        .regions(vec![rwmat(), vec3()])
        .args(vec![A::P(0), A::P(1), A::F, A::K(0)])
        .callees(&[0x7EDB90, 0x7F1D00])
        .fix(finite_angle)
        .route(Route::L2 {
            reason: "FSIN/FCOS: Unicorn uses QEMU's libm, not the CPU's reduction (X87 model follows the CPU)",
        })
        .patch(rw_patches())
        .reads(rw_reads());
    rot.ignore.extend([(0, 28), (0, 44), (0, 60)]);
    vec![
        b("RwSqrt", "_rwSqrt", 0x7EDB30, Cdecl, St0)
            .args(vec![A::I(g_word)])
            .patch(rw_patches())
            .reads(rw_reads())
            .rust(|c| st0(vector::rw_sqrt_in(tsq(), f32::from_bits(c.i(0))))),
        b("RwSqrtF", "_rwSqrt", 0x7EDB30, Cdecl, St0)
            .args(vec![A::F])
            .patch(rw_patches())
            .reads(rw_reads())
            .rust(|c| st0(vector::rw_sqrt_in(tsq(), c.f(0)))),
        b("RwInvSqrt", "_rwInvSqrt", 0x7EDB90, Cdecl, St0)
            .args(vec![A::I(g_word)])
            .patch(rw_patches())
            .reads(rw_reads())
            .rust(|c| st0(vector::rw_inv_sqrt_in(tinv(), f32::from_bits(c.i(0))))),
        b("RwInvSqrtF", "_rwInvSqrt", 0x7EDB90, Cdecl, St0)
            .args(vec![A::F])
            .patch(rw_patches())
            .reads(rw_reads())
            .rust(|c| st0(vector::rw_inv_sqrt_in(tinv(), c.f(0)))),
        b("RwV3dLength", "RwV3dLength", 0x7EDAC0, Cdecl, St0)
            .regions(vec![vec3()])
            .args(vec![A::P(0)])
            .patch(rw_patches())
            .reads(rw_reads())
            .rust(|c| st0(vector::rw_v3d_length_in(tsq(), tp(c, 0)))),
        b("RwV3dNormalize", "RwV3dNormalize", 0x7ED9B0, Cdecl, St0)
            .regions(vec![vec3(), vec3()])
            .args(vec![A::P(0), A::P(1)])
            .alias(0, 1, 20)
            .fix(no_zero_len)
            .patch(rw_patches())
            .reads(rw_reads())
            .rust(|c| {
                let (o, i) = (c.ar(0), c.ar(1));
                let (len, err) = vector::rw_v3d_normalize_in(tsq(), tinv(), tp(c, o), tp(c, i));
                assert!(!err, "inputs are fixed away from the error path");
                st0(len)
            }),
        b(
            "RwMatrixDeterminant",
            "_rwMatrixDeterminant",
            0x7F1450,
            Cdecl,
            St0,
        )
        .regions(vec![rwmat()])
        .args(vec![A::P(0)])
        .rust(|c| st0(matrix::rw_matrix_determinant(tp(c, 0)))),
        b(
            "RwMatrixOrthogonalError",
            "_rwMatrixOrthogonalError",
            0x7F14A0,
            Cdecl,
            St0,
        )
        .regions(vec![rwmat()])
        .args(vec![A::P(0)])
        .rust(|c| st0(matrix::rw_matrix_orthogonal_error(tp(c, 0)))),
        b(
            "RwMatrixNormalError",
            "_rwMatrixNormalError",
            0x7F1500,
            Cdecl,
            St0,
        )
        .regions(vec![rwmat()])
        .args(vec![A::P(0)])
        .rust(|c| st0(matrix::rw_matrix_normal_error(tp(c, 0)))),
        b("RwMatrixUpdate", "RwMatrixUpdate", 0x7F18A0, Cdecl, Eax)
            .regions(vec![rwmat()])
            .args(vec![A::P(0)])
            .rust(|c| {
                matrix::rw_matrix_update(tp(c, 0));
                eax(c, 0)
            }),
        b("RwMatrixInvert", "RwMatrixInvert", 0x7F2070, Cdecl, Eax)
            .regions(vec![rwmat(), rwmat()])
            .args(vec![A::P(0), A::P(1)])
            .callees(&[0x7F2160])
            .patch(rw_patches())
            .reads(rw_reads())
            .rust(|c| {
                matrix::rw_matrix_invert_with(0x20000, tp(c, 0), tp(c, 1));
                eax(c, 0)
            }),
        b("RwMatrixInvertGeneral", "FUN_007f2160", 0x7F2160, Cdecl, Eax)
            .regions(vec![rwmat(), rwmat()])
            .args(vec![A::P(0), A::P(1)])
            .rust(|c| {
                matrix::rw_matrix_invert_general(tp(c, 0), tp(c, 1));
                eax(c, 0)
            }),
        rot_omcs.rust(|c| {
            matrix::rw_matrix_rotate_one_minus_cosine_sine(tp(c, 0), tp(c, 1), c.f(2), c.f(3), 0);
            eax(c, 0)
        }),
        rot.rust(|c| {
            matrix::rw_matrix_rotate_with_tables(tinv(), tp(c, 0), tp(c, 1), c.f(2), 0);
            eax(c, 0)
        }),
        b("RwMatrixScale", "RwMatrixScale", 0x7F22C0, Cdecl, Eax)
            .regions(vec![rwmat(), vec3()])
            .args(vec![A::P(0), A::P(1), A::I(g_op3)])
            .alias(1, 0, 10)
            .rust(|c| {
                let s = c.ar(1);
                matrix::rw_matrix_scale(tp(c, 0), tp(c, s), c.i(2) as i32);
                eax(c, 0)
            }),
        b("RwMatrixTranslate", "RwMatrixTranslate", 0x7F2450, Cdecl, Eax)
            .regions(vec![rwmat(), vec3()])
            .args(vec![A::P(0), A::P(1), A::I(g_op3)])
            .alias(1, 0, 10)
            .rust(|c| {
                let s = c.ar(1);
                matrix::rw_matrix_translate(tp(c, 0), tp(c, s), c.i(2) as i32);
                eax(c, 0)
            }),
        b(
            "RwMatrixTransformReplace",
            "RwMatrixTransform",
            0x7F25A0,
            Cdecl,
            Eax,
        )
        .regions(vec![rwmat(), rwmat()])
        .args(vec![A::P(0), A::P(1), A::K(0)])
        .rust(|c| {
            matrix::rw_matrix_transform(tp(c, 0), tp(c, 1), 0);
            eax(c, 0)
        }),
        b("RwBBoxCalculate", "RwBBoxCalculate", 0x808F60, Cdecl, Eax)
            .regions(vec![raw(6), floats(24)])
            .args(vec![A::P(0), A::P(1), A::I(g_n8)])
            .rust(|c| {
                matrix::rw_bbox_calculate(tp(c, 0), tp(c, 1), c.i(2) as i32);
                eax(c, 0)
            }),
    ]
}
