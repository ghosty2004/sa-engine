//! FRONTIER-2 turn 2, lane B (batch `frontier2-b8`): render functions on the phase B frontier (the moving fog).
//! Ported from the 1.0 US bytes; gta-reversed as a reading aid only (the bytes win). Unverified
//! (docs/verify/HOWTO.md). `ms_mf` (tMovingFog, 0xC6C158): slots[350] +0, positions +0x160 (0xC6C2B8), sizes
//! +0x11C8 (0xC6D320), intensities +0x1740 (0xC6D898), max intensities +0x1CB8 (0xC6DE10), wind +0x2230 (0xC6E388),
//! speeds +0x223C (0xC6E394).

use sa_core::abi::HookReg;
use sa_core::crt::rand_at;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x, xi};
use sa_world::world::common::cam_pos;

/// `rand() * (1 / RAND_MAX)` (0x858C7C) at `site`, as the fog code computes it (`fild; fmul`).
fn rand_unit(site: u32) -> X87 {
    xi(rand_at(site)).fmul_m(g::<f32>(0x85_8C7C).read())
}

/// `CClouds::MovingFog_Create(CVector* centre)` (0x713760, cdecl): the first free of the 350 slots (none: nothing)
/// gets a position `centre + (r * [0x87283C] - [0x866C50], r * [0x87283C] - [0x866C50], r * 10 - [0x858C80])`, a size
/// `r * [0x858B44] + 4`, intensity 1.0, a max intensity `r * [0x858CCC] + [0x859000]`, a speed `r * [0x872838] + 0.5`
/// (each `r` a fresh `rand() / RAND_MAX`; rand sites 0x7137AB, 0x7137D0, 0x7137F5, 0x71384A, 0x71387B, 0x7138A1) and
/// is marked used.
#[hook(addr = 0x713760, conv = cdecl, name = "CClouds::MovingFog_Create", expect = "83 EC 10 56 33 F6",
       unverified = "frontier2-b8", route = "L2", hazards = "none",
       note = "rand sites 0x7137AB 0x7137D0 0x7137F5 0x71384A 0x71387B 0x7138A1")]
pub fn moving_fog_create(centre: GPtr<u8>) {
    let Some(i) = (0..0x15Eusize).find(|&i| g::<u8>(0xC6_C158 + i).read() == 0) else {
        return;
    };
    let c = [rd::<f32>(centre, 0), rd::<f32>(centre, 4), rd::<f32>(centre, 8)];
    let p = GPtr::<u8>::from_addr(0xC6_C2B8 + 12 * i);
    let k = |a: usize| g::<f32>(a).read();
    wr::<f32>(p, 0, rand_unit(0x71_37AB).fmul_m(k(0x87_283C)).fsub_m(k(0x86_6C50)).to_f32());
    wr::<f32>(p, 4, rand_unit(0x71_37D0).fmul_m(k(0x87_283C)).fsub_m(k(0x86_6C50)).to_f32());
    wr::<f32>(p, 8, rand_unit(0x71_37F5).fmul_m(k(0x85_862C)).fsub_m(k(0x85_8C80)).to_f32());
    for (o, cv) in [(0isize, c[0]), (4, c[1]), (8, c[2])] {
        wr::<f32>(p, o, x(cv).fadd_m(rd(p, o)).to_f32());
    }
    g::<f32>(0xC6_D320 + 4 * i).write(rand_unit(0x71_384A).fmul_m(k(0x85_8B44)).fadd_m(k(0x85_8B90)).to_f32());
    g::<u32>(0xC6_D898 + 4 * i).write(0x3F80_0000);
    g::<f32>(0xC6_DE10 + 4 * i).write(rand_unit(0x71_387B).fmul_m(k(0x85_8CCC)).fadd_m(k(0x85_9000)).to_f32());
    g::<f32>(0xC6_E394 + 4 * i).write(rand_unit(0x71_38A1).fmul_m(k(0x87_2838)).fadd_m(k(0x85_8B8C)).to_f32());
    g::<u8>(0xC6_C158 + i).write(1);
}

/// `CClouds::MovingFog_Update()` (0x716B10, cdecl): nothing while `Foggyness_SF` (0xC812FC) is 0. Otherwise, with the
/// camera position (TheCamera's entity matrix or 0xB6F02C) and the wind (0xC6E388): every unused slot gets a new fog
/// (`MovingFog_Create(&camera)`); a used one is moved by `wind.xy * speed` and, by its distance to the camera before the
/// move (`sqrt((dz*dz + dy*dy) + dx*dx)`, x87), beyond 60 fades out (`intensity -= ms_fTimeStep`, deleted at 0 or
/// below: the inlined `MovingFog_Delete` with its index clamp) or else fades in (`+= ms_fTimeStep`, capped at the max).
#[hook(addr = 0x716B10, conv = cdecl, name = "CClouds::MovingFog_Update", expect = "D9 05 FC 12 C8 00 83 EC 18",
       unverified = "frontier2-b8", route = "L2", hazards = "x87-transcendental", note = "FSQRT distance")]
pub fn moving_fog_update() {
    let zero = g::<f32>(0x85_8B50).read();
    if !x(g::<f32>(0xC8_12FC).read()).fcom(x(zero)).test_parity(0x44) {
        return;
    }
    let cp = cam_pos();
    let mut cam = [cp.read(), cp.byte_add(4).read(), cp.byte_add(8).read()];
    let wind = [g::<f32>(0xC6_E388).read(), g::<f32>(0xC6_E38C).read(), g::<f32>(0xC6_E390).read()];
    let ts = g::<f32>(0xB7_CB5C).read();
    for i in 0..350usize {
        let p = GPtr::<u8>::from_addr(0xC6_C2B8 + 12 * i);
        let dx = x(rd(p, 0)).fsub_m(cam[0]);
        let dy = x(rd(p, 4)).fsub_m(cam[1]);
        let dz = x(rd(p, 8)).fsub_m(cam[2]);
        if g::<u8>(0xC6_C158 + i).read() == 0 {
            moving_fog_create_call(GPtr::from_raw(cam.as_mut_ptr()).cast());
            continue;
        }
        let speed = g::<f32>(0xC6_E394 + 4 * i).read();
        wr::<f32>(p, 0, x(wind[0]).fmul_m(speed).fadd_m(rd(p, 0)).to_f32());
        wr::<f32>(p, 4, x(wind[1]).fmul_m(speed).fadd_m(rd(p, 4)).to_f32());
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
        let a = g::<f32>(0xC6_D898 + 4 * i);
        if !d.fcom(x(g::<f32>(0x85_8B34).read())).test_any(0x41) {
            let v = x(a.read()).fsub_m(ts);
            a.write(v.to_f32());
            if !v.fcom(x(zero)).test_parity(0x41) {
                let idx = if 0xC6_C2B8 + 12 * i > 0xC6_D314 { 0x15D } else { i };
                g::<u8>(0xC6_C158 + idx).write(0);
            }
        } else {
            let v = x(ts).fadd_m(a.read());
            a.write(v.to_f32());
            let max = g::<f32>(0xC6_DE10 + 4 * i);
            if !v.fcom(x(max.read())).test_any(0x41) {
                g::<u32>(0xC6_D898 + 4 * i).write(g::<u32>(0xC6_DE10 + 4 * i).read());
            }
        }
    }
}

sa_core::orig! {
    /// `CClouds::MovingFog_Create(CVector*)` (0x713760), as `MovingFog_Update` calls it
    fn moving_fog_create_call(centre: GPtr<u8>) = cdecl 0x713760 => crate::frontier_b2::moving_fog_create;
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_MOVING_FOG_CREATE, HOOK_MOVING_FOG_UPDATE];
