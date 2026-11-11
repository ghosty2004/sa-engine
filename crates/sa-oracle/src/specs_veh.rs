//! L1 specs for PORT-VEH (`sa_veh`): the entries whose hooks preserve a register (VER-2 preserve
//! specs).
//!
//! Only the window of the vehicle the function touches is in scratch (`this` / `veh` points before
//! scratch). The ports read `m_pHandlingData` (+0x384) and `m_pFlyingHandlingData` (+0x388) as
//! `GPtr`s, which are 8 bytes on the host:
//!
//! - `ComputePassengerIndexFromCarDoor` reads only +0x384: the host copy widens it
//!   ([`crate::hosted::Widened`]).
//! - `CPlane::SetGearUp` reads both, and the two 8-byte host fields overlap (+0x388..+0x38C is the
//!   high half of the first and the low half of the second). The host image is therefore placed at
//!   the fixed host address `X << 32` ([`crate::hosted::fixed_map`]) with `X` = the image offset of
//!   the flying-handling pointer: then the high half of the handling pointer (X) equals the low half
//!   of the flying-handling pointer, and both host pointers are exact. The plane's +0x98 and +0x9CC
//!   lie too far from +0x384..+0x5A8 for one scratch window: their two displacements are patched to
//!   +0x398 / +0x39C (inside the window), and the Rust side's writes there are moved alike.

use crate::call::Conv::*;
use crate::call::{SCRATCH, SCRATCH_LEN};
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::abi::{PRESERVE_ECX, PRESERVE_EDX};
use sa_core::mem::GPtr;
use sa_veh::{enterexit, heli};

/// Guest address of region `r` of a spec with regions of `words` dwords (the `B::layout` rule).
fn g_addr(words: &[usize], r: usize) -> u32 {
    let mut o = 0x10usize;
    for w in &words[..r] {
        o += (4 * w + 0x10 + 0xF) & !0xF;
    }
    SCRATCH + o as u32
}

fn hash(v: &[u32]) -> u32 {
    v.iter()
        .fold(0x811C_9DC5u32, |h, &w| (h ^ w).wrapping_mul(0x0100_0193))
}

/// A door id: mostly the ones the function maps, sometimes any.
fn g_door(r: &mut Rng) -> u32 {
    match r.below(10) {
        0..=6 => r.pick(&[8, 9, 10, 11, 18, 0, 7, 12]),
        7..=8 => r.below(32),
        _ => r.next_u32(),
    }
}

/// ComputePassengerIndexFromCarDoor: vehicle window +0x384..+0x594, handling flags (+0xCC).
const PI_BASE: u32 = 0x384;
const PI_WORDS: [usize; 2] = [(0x594 - 0x384) / 4, 1];

/// SetGearUp: plane window +0x380..+0x5AC (139 words).
const GEAR_BASE: u32 = 0x380;
const GEAR_WORDS: usize = (0x5AC - 0x380) / 4;
/// Window words holding `fMass` (handling +0x10) and the flying handling's +0x30.
const GEAR_MASS_W: usize = (0x400 - 0x380) / 4;
const GEAR_FLY_W: usize = (0x410 - 0x380) / 4;
/// Guest `this`, handling and flying-handling pointers.
const GEAR_THIS: u32 = SCRATCH + 0x10 - GEAR_BASE;
const GEAR_HANDLING: u32 = SCRATCH + 0x10 + 4 * GEAR_MASS_W as u32 - 0x10;
const GEAR_FLYING: u32 = SCRATCH + 0x10 + 4 * GEAR_FLY_W as u32 - 0x30;
/// `X`: the image offset of the flying-handling pointer, and the high half of the host image base.
const GEAR_X: u32 = GEAR_FLYING - SCRATCH;
/// The relocated +0x98 / +0x9CC.
const GEAR_98_TO: u32 = 0x398;
const GEAR_9CC_TO: u32 = 0x39C;

fn gear_region() -> Region {
    let mut w = vec![W::R; GEAR_WORDS];
    w[1] = W::C(GEAR_HANDLING);
    w[2] = W::C(GEAR_FLYING);
    w[GEAR_MASS_W] = W::F;
    w[GEAR_FLY_W] = W::F;
    Region {
        words: w,
        kind: Kind::Any,
    }
}

/// The Rust side of SetGearUp on the host image at `X << 32` (see the module docs).
fn gear_rust(c: &mut Ctx) -> ExpRet {
    let _g = crate::host_globals();
    let base_addr = (GEAR_X as usize) << 32;
    let map = crate::hosted::fixed_map(base_addr - 0x1000, 0x3000);
    // SAFETY: fixed_map mapped 0x3000 bytes at `map`; held under host_globals. Every access below
    // lies inside it (this + 0x98 = image - 0x2D8, this + 0x9CC = image + 0x65C).
    unsafe {
        core::ptr::write_bytes(map, 0, 0x3000);
        let img = map.add(0x1000);
        core::ptr::copy_nonoverlapping(c.mem.as_ptr(), img, SCRATCH_LEN);
        let host = |g: u32| img.addr() as u64 + u64::from(g - SCRATCH);
        let (h, f) = (host(GEAR_HANDLING), host(GEAR_FLYING));
        assert_eq!(
            ((h >> 32) as u32, f as u32),
            (GEAR_X, GEAR_X),
            "SetGearUp host pointer shape"
        );
        let field = 0x10 + (0x384 - GEAR_BASE) as usize;
        img.add(field).cast::<u64>().write_unaligned(h);
        img.add(field + 4).cast::<u64>().write_unaligned(f);
        let this = img.wrapping_sub(GEAR_BASE as usize).add(0x10);
        heli::set_gear_up(GPtr::from_raw(this));
        let mut out = [0u8; SCRATCH_LEN];
        core::ptr::copy_nonoverlapping(img, out.as_mut_ptr(), SCRATCH_LEN);
        out[field..field + 12].copy_from_slice(&c.mem[field..field + 12]);
        let rd = |o: usize| this.add(o).cast::<u32>().read_unaligned();
        let to = |o: u32| 0x10 + (o - GEAR_BASE) as usize;
        out[to(GEAR_98_TO)..to(GEAR_98_TO) + 4].copy_from_slice(&rd(0x98).to_le_bytes());
        out[to(GEAR_9CC_TO)..to(GEAR_9CC_TO) + 4].copy_from_slice(&rd(0x9CC).to_le_bytes());
        c.mem = out;
    }
    ExpRet::Void
}

pub fn all() -> Vec<Spec> {
    let handling = g_addr(&PI_WORDS, 1) - 0xCC;
    vec![
        // CCarEnterExit::SetPedInCarDirect pushes EDX after the call (0x650360)
        b(
            "VehComputePassengerIndexFromCarDoor",
            "CCarEnterExit::ComputePassengerIndexFromCarDoor",
            0x64F1E0,
            Cdecl,
            Eax,
        )
        .regions(vec![
            {
                let mut w = vec![W::R; PI_WORDS[0]];
                w[0] = W::C(handling);
                Region {
                    words: w,
                    kind: Kind::Any,
                }
            },
            raw(PI_WORDS[1]),
        ])
        .args(vec![A::K(SCRATCH + 0x10 - PI_BASE), A::I(g_door)])
        .fix(|v, l| {
            // the vehicle sub-type (+0x590): bike (9) often
            let s = l.word_slot[0][PI_WORDS[0] - 1].unwrap();
            let h = hash(v);
            if h.is_multiple_of(3) {
                v[s] = 9;
            } else if h % 3 == 1 {
                v[s] = (h >> 2) % 11;
            }
        })
        .preserve(PRESERVE_EDX)
        .rust(|c| {
            let field = c.lay.off[0];
            let veh = SCRATCH + 0x10 - PI_BASE;
            let mut w = crate::hosted::Widened::new(&c.mem, &[field]);
            let r = enterexit::compute_passenger_index_from_car_door(w.at(veh), c.i(1) as i32);
            w.finish(&mut c.mem);
            ExpRet::Eax(r as u32)
        }),
        // its callers keep `this` in ECX across the call (VER-2)
        b("VehPlaneSetGearUp", "CPlane::SetGearUp", 0x6CAC20, Thiscall, Void)
            .regions(vec![gear_region()])
            .args(vec![A::K(GEAR_THIS)])
            .patch(vec![
                // `fstp [ecx+0x98]` -> +0x398, `mov [ecx+0x9CC], 1.0` -> +0x39C
                (0x6CAC58, GEAR_98_TO.to_le_bytes().to_vec()),
                (0x6CAC28, GEAR_9CC_TO.to_le_bytes().to_vec()),
            ])
            .preserve(PRESERVE_ECX)
            .rust(gear_rust),
    ]
}
