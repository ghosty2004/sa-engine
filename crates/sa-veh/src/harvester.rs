//! FRONTIER-2 turn 11, lane B (batch `frontier2-b25`): `CAutomobile::ProcessHarvester` (0x6A9680). Ported from the
//! 1.0 US bytes with docs/re/specs/VEH/CAutomobile.deviations.md and gta-reversed as reading aids (the bytes win).
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::bindings::{matrix_mul_vec, object_ctor_5a1d70, object_new, world_add};
use sa_core::field::{gf, gr16u, gr32, ptr_of as vp};
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::fpu::{ge_or_nan, le_or_nan, xi};
use sa_math::vector::CVector;
use sa_math::x87::X87;
use sa_world::world::common::entity_pos;
use sa_world::world::mvpb_world::entity_update_rw_matrix;

use crate::automobile_flying_component::v_set_is_static;
use crate::bindings::fx_prt_mult_set_up;
use crate::common::{f32_of, g, rand_at, rd, wr, x};
use crate::mvpd_v1::read_vec;
use crate::vehicle2::ped_vslot_54 as flag_to_destroy_when_next_processed;
use sa_core::bindings::entity_update_rw_frame;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{
    audio_add_event, fx_add_particle, fx_prt_mult_default_ctor, localisation_blood, localisation_shoot_limbs, object_damage,
    physical_45afb0 as set_turn_speed, placeable_set_matrix, placeable_set_posn,
};

/// `CStreaming::m_bStreamHarvesterModelsThisFrame`, `CTimer::m_FrameCounter`, the ped / object pools,
/// `CObject::nNoTempObjects`, `g_fx.m_SmokeII3expand`.
const STREAM_HARVESTER: usize = 0x8E_4B9D;
const FRAME_COUNTER: usize = 0xB7_CB4C;
const PED_POOL: usize = 0xB7_4490;
const OBJECT_POOL: usize = 0xB7_449C;
const NO_TEMP_OBJECTS: usize = 0xBB_4A70;
const FX_SMOKE_II3_EXPAND: usize = 0xA9_AE24;
/// `MI_HARVESTERBODYPART1..4` (0x8CD6E4 / 0x8CD6E8 / 0x8CD6EC / 0x8CD6F0), `MI_GRASSHOUSE` / `MI_GRASSPLANT`
/// (0x8CD6F4 / 0x8CD6F8), all u16.
const MI_BODY_PART: [u32; 4] = [0x8C_D6E4, 0x8C_D6E8, 0x8C_D6EC, 0x8C_D6F0];
const MI_GRASS_HOUSE: u32 = 0x8C_D6F4;
const MI_GRASS_PLANT: u32 = 0x8C_D6F8;
/// 0.01, 4.0, 5.0, 0.0, -0.1, 1/32767, 0.1, 0.05, 0.08, 0.04.
const K_MIN_SPEED: usize = 0x85_8C58;
const K_FOUR: usize = 0x85_8B90;
const K_FIVE: usize = 0x85_8C80;
const K_ZERO: usize = 0x85_8B50;
const K_BACK: usize = 0x85_8EF4;
const K_RAND: usize = 0x85_8C7C;
const K_TENTH: usize = 0x85_8B1C;
const K_HALF_TENTH: usize = 0x85_8C28;
const K_TURN_SCALE: usize = 0x85_9018;
const K_TURN_OFFSET: usize = 0x85_8CEC;

/// `|v|` as the bytes take it: `fcom 0.0; test ah, 5; jp` keeps `v` when `v >= 0` (or NaN), else `fchs`.
fn abs_x(v: X87) -> X87 {
    if ge_or_nan(v, x(gf(K_ZERO))) { v } else { v.fchs() }
}

/// The box test in front of the harvester: `d = e - this` (f32 per component), `4 < d . forward < 5`,
/// `|d . right| < 4` and `|d . up| < 4` (each dot `(d.y * a.y + d.z * a.z) + d.x * a.x`).
fn in_cutter(this: GPtr<u8>, m: GPtr<u8>, e: GPtr<u8>) -> bool {
    let tp = entity_pos(this);
    let ep = entity_pos(e);
    let dz = f32_of(x(rd::<f32>(ep, 8)).fsub_m(rd::<f32>(tp, 8)));
    let dy = f32_of(x(rd::<f32>(ep, 4)).fsub_m(rd::<f32>(tp, 4)));
    let dx = f32_of(x(rd::<f32>(ep, 0)).fsub_m(rd::<f32>(tp, 0)));
    let dot = |o: isize| x(dy).fmul_m(rd::<f32>(m, o + 4)).fadd(x(dz).fmul_m(rd::<f32>(m, o + 8))).fadd(x(dx).fmul_m(rd::<f32>(m, o)));
    let four = x(gf(K_FOUR));
    let f = dot(0x10);
    if le_or_nan(f, four) || ge_or_nan(f, x(gf(K_FIVE))) {
        return false;
    }
    !ge_or_nan(abs_x(dot(0)), four) && !ge_or_nan(abs_x(dot(0x20)), four)
}

/// `(rand() * 1/32767) * 0.1 + v - 0.05` (f32).
fn jitter(site: u32, v: f32) -> f32 {
    f32_of(xi(rand_at(site)).fmul_m(gf(K_RAND)).fmul_m(gf(K_TENTH)).fadd_m(v).fsub_m(gf(K_HALF_TENTH)))
}

/// `CAutomobile::ProcessHarvester()` (0x6A9680, thiscall, plain `ret`; SEH frame for the limb's `new`, not modelled).
///
/// Player-status only; sets `m_bStreamHarvesterModelsThisFrame`. Moving faster than 0.01 (2D), on odd frames every
/// non-player ped of the pool (from the last slot) in the cutter box is flagged for destruction (vtable +0x54), the
/// particle counter (+0x980) becomes 20 and the body-harvest audio event (0x73) is added; on even frames every
/// object of the pool whose own model is the grass house / plant, with +0x1C bit 7 set and bit 9 clear (gta-reversed
/// tests the harvester's model and not the flags), in the box takes `ObjectDamage(99999, null, null, this, 50)`
/// (the matrix +0x14 is read without a null check). With particles left: `pos = matrix * (-1.2, -3.8, 1.5)`, `vel =
/// forward * -0.1` plus x / y jitter; counters 1 / 7, 2 / 4, 3, 6 pick body parts 1, 2, 3, 4: with ShootLimbs a
/// temporary `CObject(part, true)` at `pos` with the harvester's matrix (set on the limb), `vel` and the turn speed
/// `(C, B, A)` of three rand() draws (`t * 0.08 - 0.04`, the first rounded to f32 before the scale) goes into the
/// world. The counter drops by one; with Blood and `counter % 3 == 0` a SmokeII3expand particle at `pos`.
#[hook(addr = 0x6A9680, conv = thiscall, name = "CAutomobile::ProcessHarvester", this = "CAutomobile",
       expect = "6A FF 68 6B 80 84 00", unverified = "frontier2-b25", route = "L2", hazards = "heap-writes",
       note = "creates limb objects, flags peds, damages objects; consumes up to five rand() values")]
pub fn process_harvester(this: GPtr<u8>) {
    if rd::<u8>(this, 0x36) & 0xF8 != 0 {
        return;
    }
    g::<u8>(STREAM_HARVESTER).write(1);
    let m = rd::<GPtr<u8>>(this, 0x14);
    let sx = x(rd::<f32>(this, 0x44));
    let sy = x(rd::<f32>(this, 0x48));
    if !le_or_nan(sx.fmul(sx).fadd(sy.fmul(sy)).fsqrt(), x(gf(K_MIN_SPEED))) {
        if gr32(FRAME_COUNTER) & 1 != 0 {
            let pool = g::<GPtr<u8>>(PED_POOL).read();
            let mut i = rd::<i32>(pool, 8);
            while i != 0 {
                i -= 1;
                if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
                    continue;
                }
                let ped: GPtr<u8> = GPtr::from_addr(rd::<u32>(pool, 0).wrapping_add((i as u32).wrapping_mul(0x7C4)) as usize);
                if ped.is_null() || ped_is_player(ped) != 0 || !in_cutter(this, m, ped) {
                    continue;
                }
                flag_to_destroy_when_next_processed(ped);
                wr::<u8>(this, 0x980, 0x14);
                audio_add_event(this.byte_add(0x138), 0x73, 0.0);
            }
        } else {
            let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
            let mut i = rd::<i32>(pool, 8);
            while i != 0 {
                i -= 1;
                if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
                    continue;
                }
                let o: GPtr<u8> = GPtr::from_addr(rd::<u32>(pool, 0).wrapping_add((i as u32).wrapping_mul(0x19C)) as usize);
                if o.is_null() {
                    continue;
                }
                let mi = i32::from(rd::<i16>(o, 0x22));
                if mi != i32::from(gr16u(MI_GRASS_HOUSE)) && mi != i32::from(gr16u(MI_GRASS_PLANT)) {
                    continue;
                }
                let flags = rd::<u32>(o, 0x1C);
                if flags & 0x80 == 0 || flags & 0x200 != 0 || !in_cutter(this, m, o) {
                    continue;
                }
                object_damage(o, f32::from_bits(0x47C3_4F80), GPtr::NULL, GPtr::NULL, this, 0x32);
            }
        }
    }
    if rd::<u8>(this, 0x980) == 0 {
        return;
    }
    let mut off = CVector::new(f32::from_bits(0xBF99_999A), f32::from_bits(0xC073_3333), 1.5);
    let mut out = CVector::default();
    let mut pos = read_vec(matrix_mul_vec(vp(&mut out), m, vp(&mut off)));
    let back = gf(K_BACK);
    let vz = f32_of(x(rd::<f32>(m, 0x18)).fmul_m(back));
    let vy = f32_of(x(rd::<f32>(m, 0x14)).fmul_m(back));
    let vx = f32_of(x(rd::<f32>(m, 0x10)).fmul_m(back));
    let vx = jitter(0x6A_9A31, vx);
    let vy = jitter(0x6A_9A58, vy);
    let mut vel = CVector::new(vx, vy, vz);
    let part = match rd::<u8>(this, 0x980) {
        1 | 7 => i32::from(gr16u(MI_BODY_PART[0])),
        2 | 4 => i32::from(gr16u(MI_BODY_PART[1])),
        3 => i32::from(gr16u(MI_BODY_PART[2])),
        6 => i32::from(gr16u(MI_BODY_PART[3])),
        _ => -1,
    };
    if localisation_shoot_limbs() != 0 && part >= 0 {
        let mem = object_new(0x17C);
        let limb = if mem.is_null() { mem } else { object_ctor_5a1d70(mem, part, 1) };
        placeable_set_matrix(limb, m);
        placeable_set_posn(limb, vp(&mut pos).cast());
        wr::<f32>(limb, 0x44, vel.x);
        wr::<f32>(limb, 0x48, vel.y);
        wr::<f32>(limb, 0x4C, vel.z);
        let k = gf(K_RAND);
        let (scale, offset) = (gf(K_TURN_SCALE), gf(K_TURN_OFFSET));
        let a = f32_of(xi(rand_at(0x6A_9B2A)).fmul_m(k));
        let t4 = xi(rand_at(0x6A_9B41)).fmul_m(k);
        let za = f32_of(x(a).fmul_m(scale).fsub_m(offset));
        let yb = f32_of(t4.fmul_m(scale).fsub_m(offset));
        let xc = f32_of(xi(rand_at(0x6A_9B7A)).fmul_m(k).fmul_m(scale).fsub_m(offset));
        set_turn_speed(limb, xc.to_bits(), yb.to_bits(), za.to_bits());
        wr::<u8>(limb, 0x13C, 3);
        entity_update_rw_matrix(limb);
        entity_update_rw_frame(limb);
        v_set_is_static(limb, 0);
        let n = g::<u16>(NO_TEMP_OBJECTS);
        n.write(n.read().wrapping_add(1));
        world_add(limb);
    }
    wr::<u8>(this, 0x980, rd::<u8>(this, 0x980).wrapping_sub(1));
    if localisation_blood() != 0 && rd::<u8>(this, 0x980) % 3 == 0 {
        let mut prt = [0u32; 7];
        let pp = GPtr::from_raw(prt.as_mut_ptr()).cast::<u8>();
        fx_prt_mult_default_ctor(pp.addr() as u32);
        fx_prt_mult_set_up(pp, f32::from_bits(0x3E19_999A), 0.0, 0.0, 1.0, 1.0, 0.0, 1.0);
        fx_add_particle(
            g::<GPtr<u8>>(FX_SMOKE_II3_EXPAND).read(),
            vp(&mut pos),
            vp(&mut vel),
            0,
            pp,
            0xBF80_0000,
            f32::from_bits(0x3F99_999A),
            f32::from_bits(0x3F19_999A),
            0,
        );
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_HARVESTER];
