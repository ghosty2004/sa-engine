//! L1 specs for PORT-RENDER (`sa_render`): helper B modules (the FX system's pure setters and
//! leaves).

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_render::fx;

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

/// GetMem alignments: small ones (1..16) half the time, else odd raw words (remainders that are negative
/// as signed ints included); never 0 (`div` by zero faults in the original and panics in Rust).
fn g_align(r: &mut Rng) -> u32 {
    let v = g_rand(r);
    if v & 1 == 0 { (v >> 1) % 16 + 1 } else { v | 1 }
}

pub fn all() -> Vec<Spec> {
    vec![
        b(
            "FxSystemSetConstTime",
            "FxSystem_c::SetConstTime",
            0x4AA6C0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(0x18)])
        .args(vec![A::P(0), A::I(g_rand), A::F])
        .callees(&[0x821B40])
        .rust(|c| {
            let (on, t) = (c.i(1), c.f(2).to_bits());
            fx::fx_system_set_const_time(tp(c, 0), on, t);
            ExpRet::Void
        }),
        b(
            "FxSystemSetRateMult",
            "FxSystem_c::SetRateMult",
            0x4AA6F0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(0x18)])
        .args(vec![A::P(0), A::F])
        .callees(&[0x821B40])
        .rust(|c| {
            let m = c.f(1).to_bits();
            fx::fx_system_set_rate_mult(tp(c, 0), m);
            ExpRet::Void
        }),
        b(
            "FxSystemSetLocalParticles",
            "FxSystem_c::SetLocalParticles",
            0x4AA910,
            Thiscall,
            Void,
        )
        .regions(vec![raw(0x19)])
        .args(vec![A::P(0), A::I(g_rand)])
        .rust(|c| {
            let on = c.i(1);
            fx::fx_system_set_local_particles(tp(c, 0), on);
            ExpRet::Void
        }),
        b(
            "FxSystemSetVelAdd",
            "FxSystem_c::SetVelAdd",
            0x4AA730,
            Thiscall,
            Void,
        )
        .regions(vec![raw(0x1D), raw(3)])
        .args(vec![A::P(0), A::P(1)])
        .rust(|c| {
            let v = tp(c, 1);
            fx::fx_system_set_vel_add(tp(c, 0), v);
            ExpRet::Void
        }),
        b(
            "FxMemoryPoolGetMem",
            "FxMemoryPool_c::GetMem",
            0x4A9CA0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(3)])
        .args(vec![A::P(0), A::I(g_rand), A::I(g_align)])
        .rust(|c| {
            let (size, align) = (c.i(1), c.i(2));
            ExpRet::Eax(fx::fx_memory_pool_get_mem(tp(c, 0), size, align).addr() as u32)
        }),
        b(
            "FxManagerFxRwMatrixCreate",
            "FxManager_c::FxRwMatrixCreate",
            0x4A9440,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(0x2F)])
        .args(vec![A::P(0)])
        .fix(|v, l| {
            // the index +0x88 stays inside the 0x2F-word region (words 0x23.. hold the matrices)
            if let Some(s) = l.word_slot[0][0x22] {
                v[s] %= 0xC;
            }
        })
        .rust(|c| ExpRet::Eax(fx::fx_manager_fx_rw_matrix_create(tp(c, 0)))),
        b(
            "FxManagerFxRwMatrixDestroy",
            "FxManager_c::FxRwMatrixDestroy",
            0x4A9460,
            Thiscall,
            Void,
        )
        .regions(vec![raw(0x23)])
        .args(vec![A::P(0), A::I(g_rand)])
        .rust(|c| {
            let m = c.i(1);
            fx::fx_manager_fx_rw_matrix_destroy(tp(c, 0), m);
            ExpRet::Void
        }),
        b(
            "FxManagerSetWindData",
            "FxManager_c::SetWindData",
            0x4A93E0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(0xD)])
        .args(vec![A::P(0), A::I(g_rand), A::I(g_rand)])
        .rust(|c| {
            let (a, b2) = (c.i(1), c.i(2));
            fx::fx_manager_set_wind_data(tp(c, 0), a, b2);
            ExpRet::Void
        }),
        b(
            "FxEmitterInit",
            "FxEmitter_c::Init_Reversed",
            0x4A2550,
            Thiscall,
            Al,
        )
        .regions(vec![raw(5)])
        .args(vec![A::P(0), A::I(g_rand), A::I(g_rand)])
        .rust(|c| {
            let (a, b2) = (c.i(1), c.i(2));
            ExpRet::Al(fx::fx_emitter_init(tp(c, 0), a, b2))
        }),
        b(
            "FxInterpInfoFloatCtor",
            "FxInterpInfoFloat_c::FxInterpInfoFloat_c",
            0x4A8440,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(4)])
        .args(vec![A::P(0)])
        .rust(|c| {
            let t = tp::<u8>(c, 0);
            let r = fx::fx_interp_info_float_ctor(t);
            ExpRet::Eax(if r == t { c.addr(0) } else { r.addr() as u32 })
        }),
        b(
            "FxInterpInfoU255Ctor",
            "FxInterpInfoU255_c::FxInterpInfoU255_c",
            0x4A87D0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(4)])
        .args(vec![A::P(0)])
        .rust(|c| {
            let t = tp::<u8>(c, 0);
            let r = fx::fx_interp_info_u255_ctor(t);
            ExpRet::Eax(if r == t { c.addr(0) } else { r.addr() as u32 })
        }),
        b(
            "FxSystemBPCtor",
            "FxSystemBP_c::Constructor",
            0x4AA0D0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(9)])
        .args(vec![A::P(0)])
        .callees(&[0x4A8DB0])
        .rust(|c| {
            let t = tp::<u8>(c, 0);
            let r = fx::fx_system_bp_constructor(t);
            ExpRet::Eax(if r == t { c.addr(0) } else { r.addr() as u32 })
        }),
        b("FxPrimBPCtor", "FxPrimBP_c::FxPrimBP_c", 0x4A9CF0, Thiscall, Eax)
            .regions(vec![raw(0x10)])
            .args(vec![A::P(0)])
            .callees(&[0x4A8DD0])
            .rust(|c| {
                let t = tp::<u8>(c, 0);
                let r = fx::fx_prim_bp_constructor(t);
                ExpRet::Eax(if r == t { c.addr(0) } else { r.addr() as u32 })
            }),
    ]
}
