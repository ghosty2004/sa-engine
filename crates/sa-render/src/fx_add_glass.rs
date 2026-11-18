//! FRONTIER-2 lane C (batch `frontier2-c20`): `Fx_c::AddGlass` (0x49F970), the glass shards a falling glass pane
//! leaves where it lands. Unreversed in gta-reversed; ported from the 1.0 US bytes. x87 arithmetic follows the
//! original's instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::bindings::{fx_add_particle, fx_prt_mult_default_ctor};
use crate::common::{g, k, rd, wr, x, xi};
use sa_core::crt::rand_at;
use sa_core::field::ptr_of as lp;

/// `Fx_c::AddGlass(CVector& pos, RwRGBA& colour, float size, int32 count)` (0x49F970, thiscall, `ret 0x10`; the
/// entry is a SecuROM fragment jump that loads `TheCamera.m_matrix` 0xB6F03C, decoded statically: `~[0x156065C] ^
/// [0x546FF1]`). Nothing when the camera (its matrix position, else the placement position) is more than 25 away
/// (`(dz² + dx²) + dy² > 625`). Otherwise a default `FxPrtMult_c` gets the colour bytes `/ 255`, the size, life 0.2
/// and spin `((rand() % 10000) * 0.0001 + 1) * 0.5`; `count` particles of velocity `rand() / 32767 * 0.5 * 20 - 5` in
/// x and y and `rand() / 32767 * 0.15 * 20 + 2` in z go to `g_fx`'s glass system (0xA9AE40) at `pos` (AddParticle
/// time 0, rotation -1, light 1.2 / 0.6). The original uses its `pos` argument slot as the integer scratch of the
/// colour and spin conversions.
#[hook(addr = 0x49F970, conv = thiscall, name = "Fx_c::AddGlass", this = "Fx_c", expect = "E9 70 1C F6 FF 83 EC 28",
       unverified = "frontier2-c20", route = "L2", hazards = "heap-writes,stack-residue,entry-protection-read",
       note = "entry is a SecuROM fragment jump (loads the camera matrix); rand sites 0x49FA2F / 0x49FA70 / 0x49FA99 / 0x49FAC2; its pos argument slot is scratch")]
pub fn fx_add_glass(_this: GPtr<u8>, pos: GPtr<u8>, colour: GPtr<u8>, size: f32, count: i32) {
    let cm = g::<GPtr<u8>>(0xB6_F03C).read();
    let cam = if cm.is_null() { g::<u8>(0xB6_F02C) } else { cm.byte_add(0x30) };
    let d = |o: isize| x(rd::<f32>(cam, o)).fsub_m(rd::<f32>(pos, o));
    let (dx, dy, dz) = (d(0), d(4), d(8));
    if !dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).fcom(x(k(0x441C_4000))).test_any(0x41) {
        return;
    }
    let mut prt = [0u32; 7];
    let p = lp(&mut prt);
    fx_prt_mult_default_ctor(p.addr() as u32);
    for i in 0..4 {
        let c = i32::from(rd::<u8>(colour, i));
        wr::<f32>(p, i * 4, xi(c).fmul_m(k(0x3B80_8081)).to_f32());
    }
    wr::<f32>(p, 0x10, size);
    let r = rand_at(0x49_FA34) % 10000;
    wr::<u32>(p, 0x18, 0x3E4C_CCCD);
    wr::<f32>(p, 0x14, xi(r).fmul_m(k(0x38D1_B717)).fadd_m(k(0x3F80_0000)).fmul_m(k(0x3F00_0000)).to_f32());
    let unit = |site: u32| xi(rand_at(site)).fmul_m(k(0x3800_0100));
    for _ in 0..count.max(0) {
        let mut vel = [0f32; 3];
        vel[0] = unit(0x49_FA75).fmul_m(k(0x3F00_0000)).fmul_m(k(0x41A0_0000)).fsub_m(k(0x40A0_0000)).to_f32();
        vel[1] = unit(0x49_FA9E).fmul_m(k(0x3F00_0000)).fmul_m(k(0x41A0_0000)).fsub_m(k(0x40A0_0000)).to_f32();
        let sys = g::<GPtr<u8>>(0xA9_AE40).read();
        vel[2] = unit(0x49_FAC7).fmul_m(k(0x3E19_999A)).fmul_m(k(0x41A0_0000)).fadd_m(k(0x4000_0000)).to_f32();
        fx_add_particle(sys, pos, lp(&mut vel), 0, p, 0xBF80_0000, f32::from_bits(0x3F99_999A), f32::from_bits(0x3F19_999A), 0);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_FX_ADD_GLASS];
