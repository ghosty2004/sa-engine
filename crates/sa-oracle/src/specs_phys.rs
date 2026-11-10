//! L1 specs for PORT-PHYS `sa_world::physical` (see `docs/arch/phys-port.md`): the PHYS functions
//! that read no pointer out of a struct (so they run hosted on the scratch image). `this` is a
//! region holding the `CPhysical` fields the function touches (flags +0x40 raw, speeds and masses
//! generated floats).

use crate::call::Conv::*;
use crate::call::SCRATCH;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_math::vector::CVector;
use sa_world::physical::{base, buoyancy, forces};

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

/// A `CPhysical` prefix of `n` words: raw words except the float words listed.
fn phys(n: usize, floats: &[usize]) -> Region {
    let mut words = vec![W::R; n];
    for &f in floats {
        words[f] = W::F;
    }
    Region {
        words,
        kind: Kind::Any,
    }
}

/// Physical flags with every combination of the low byte's bits likely (0x10/0x20/0x40/0x80 decide
/// the paths).
fn fix_flags(v: &mut [u32], lay: &Layout, region: usize) {
    let s = lay.word_slot[region][16].unwrap();
    if !v[s].is_multiple_of(3) {
        v[s] &= 0xFF;
    }
}

fn g_weapon(r: &mut Rng) -> u32 {
    let x = g_rand(r);
    if x.is_multiple_of(8) { x } else { x % 0x40 }
}

pub fn all() -> Vec<Spec> {
    // CPhysical words: flags 16 (+0x40), move speed 17..19, friction move 23..25, mass 35 (+0x8C)
    let mf: Vec<usize> = vec![17, 18, 19, 23, 24, 25, 35];
    vec![
        b("PhysUpdateTimeStep", "FUN_00541f40", 0x541F40, Cdecl, Void)
            .regions(vec![out1()])
            .args(vec![A::F])
            .patch(vec![
                (0x541F53, (SCRATCH + 0x10).to_le_bytes().to_vec()),
                (0x541F61, (SCRATCH + 0x10).to_le_bytes().to_vec()),
            ])
            .rust(|c| {
                let _g = crate::host_globals();
                base::update_time_step(c.f(0).to_bits());
                c.set_u(0, 0, sa_core::timer::TIME_STEP.get().to_bits());
                ExpRet::Void
            }),
        b(
            "PhysSetVelocity",
            "CPhysical::SetVelocity",
            0x441130,
            Thiscall,
            Void,
        )
        .regions(vec![phys(20, &[])])
        .args(vec![A::P(0), A::F, A::F, A::F])
        .rust(|c| {
            let (x, y, z) = (c.f(1).to_bits(), c.f(2).to_bits(), c.f(3).to_bits());
            base::set_velocity(tp(c, 0), x, y, z);
            ExpRet::Void
        }),
        b(
            "PhysApplyMoveForceVec",
            "CPhysical::ApplyMoveForce-vec",
            0x5429F0,
            Thiscall,
            Void,
        )
        .regions(vec![phys(36, &mf)])
        .args(vec![A::P(0), A::F, A::F, A::F])
        .preserve(sa_core::abi::PRESERVE_EDX)
        .fix(|v, l| fix_flags(v, l, 0))
        .rust(|c| {
            let v = CVector::new(c.f(1), c.f(2), c.f(3));
            forces::apply_move_force(tp(c, 0), v);
            ExpRet::Void
        }),
        b(
            "PhysApplyFrictionMoveForce",
            "CPhysical::ApplyFrictionMoveForce",
            0x5430A0,
            Thiscall,
            Void,
        )
        .regions(vec![phys(36, &mf)])
        .args(vec![A::P(0), A::F, A::F, A::F])
        .fix(|v, l| fix_flags(v, l, 0))
        .rust(|c| {
            let v = CVector::new(c.f(1), c.f(2), c.f(3));
            forces::apply_friction_move_force(tp(c, 0), v);
            ExpRet::Void
        }),
        b(
            "PhysCanPhysicalBeDamaged",
            "CPhysical::CanPhysicalBeDamaged",
            0x5448B0,
            Thiscall,
            Al,
        )
        .regions(vec![phys(17, &[]), raw(1)])
        .args(vec![A::P(0), A::I(g_weapon), A::P(1)])
        .rust(|c| {
            let w = c.i(1);
            let out = tp(c, 1);
            ExpRet::Al(base::can_physical_be_damaged(tp(c, 0), w, out))
        }),
        // ApplyMoveSpeed reads the matrix pointer (no hosted run); this spec covers its zeroing path
        // (`bDontApplySpeed` / `bDisableMoveForce`) and compares the ECX it preserves for ApplySpeed.
        b(
            "PhysApplyMoveSpeedZero",
            "CPhysical::ApplyMoveSpeed",
            0x542DD0,
            Thiscall,
            Void,
        )
        .regions(vec![phys(20, &[17, 18, 19])])
        .args(vec![A::P(0)])
        .preserve(sa_core::abi::PRESERVE_ECX)
        .fix(|v, l| {
            let s = l.word_slot[0][16].unwrap();
            v[s] |= if v[s] & 1 != 0 { 0x2000 } else { 0x20 };
        })
        .rust(|c| {
            forces::apply_move_speed(tp(c, 0));
            ExpRet::Void
        }),
        b("BuoyancyCtor", "cBuoyancy::cBuoyancy", 0x6C2740, Thiscall, Eax)
            .regions(vec![raw(0x34)])
            .args(vec![A::P(0)])
            .rust(|c| {
                buoyancy::constructor(tp(c, 0));
                ExpRet::Eax(c.addr(0))
            }),
    ]
}
