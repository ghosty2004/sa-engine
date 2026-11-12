//! WRITER-NATIVE (batch `native-1`): `Fx_c::AddSparks` (0x49F040), reached by car collisions, bike
//! scrapes and friction on the native walk. Unreversed in gta-reversed; ported from the 1.0 US bytes. x87
//! arithmetic follows the original's instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GameData};
use sa_core::{hook, orig};
use sa_math::x87::X87;

fn g<T: GameData>(a: u32) -> GPtr<T> {
    GPtr::from_addr(a as usize)
}

fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

fn xi(v: i32) -> X87 {
    X87::from_i32(v)
}

orig! {
    /// `FxPrtMult_c::FxPrtMult_c(r, g, b, a, size, spin, life)` (0x4AB290)
    #[allow(clippy::too_many_arguments)]
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: f32, g: f32, b: f32, a: f32, size: f32, spin: f32, life: f32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    #[allow(clippy::too_many_arguments)]
    fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: f32, mult: GPtr<u8>, rot: f32, bright: f32, cam: f32, local: u32) = thiscall 0x4AA440 => unported;
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C)
const TIME_STEP: u32 = 0xB7_CB5C;
/// `CTimer::m_FrameCounter` (0xB7CB4C)
const FRAME_COUNTER: u32 = 0xB7_CB4C;
/// `TheCamera.m_matrix` (0xB6F03C, `CPlaceable` +0x14); null: the placement position `TheCamera` +4.
const CAMERA_MATRIX: u32 = 0xB6_F03C;
/// `rand() * (1 / 32767)` factor (0x858C7C)
const RAND_UNIT: u32 = 0x3800_0100;

/// `Fx_c::AddSparks(const CVector& origin, const CVector& direction, float force, int32 amount, CVector
/// across, eSparkType type, float spread, float life)` (0x49F040, thiscall, `ret 0x28`; `this` unused).
///
/// The entry `jmp`s to the protection chunk 0x40742C, which loads `eax = [0xB6F03C]` (decoded from
/// `[0x1560860] + [0x4D7A73]`) and returns to 0x49F045: the camera position. Nothing when the squared
/// camera distance is above 22500, or above 225 on odd frames. Otherwise a `FxPrtMult_c(1, 1, 1, 1, 1, 0,
/// life * 0.8)`, `across * timestep`, and for each of `amount` sparks: `t = 1 - i / amount`, a velocity
/// `(direction + 3 x rand in [-spread, spread]) * force` (the z sum kept on the x87 stack), the position
/// `origin - across * timestep * t`, and `AddParticle(pos, vel, t * 0.05, mult, -1, 1.2, 0.6, 0)` on
/// the system at 0xA9AE28 (type's low byte non-zero) or 0xA9AE2C. The original keeps `-spread` in the
/// origin / direction / across.x / life argument slots and `t` in the type slot (stack only).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x49F040, conv = thiscall, name = "Fx_c::AddSparks", this = "Fx_c",
       expect = "E9 E7 83 F6 FF 83 EC 64 85 C0",
       unverified = "native-1", route = "L2", hazards = "heap-writes,stack-residue",
       recs = "driving,grove-walk",
       note = "entry is a jmp to the protection chunk 0x40742C (camera matrix load); rand at 0x49F1A1 / 0x49F1DB / 0x49F207; particles go to the FX pools; the caller's argument slots are overwritten")]
pub fn fx_add_sparks(
    _this: GPtr<u8>,
    origin: GPtr<f32>,
    dir: GPtr<f32>,
    force: f32,
    amount: i32,
    across_x: f32,
    across_y: f32,
    across_z: f32,
    kind: u32,
    spread: f32,
    life: f32,
) {
    let m = g::<u32>(CAMERA_MATRIX).read();
    let cam: GPtr<f32> = if m != 0 { GPtr::from_addr(m as usize + 0x30) } else { g::<f32>(0xB6_F02C) };
    let dx = x(cam.wrapping_index(0).read()).fsub_m(origin.wrapping_index(0).read());
    let dy = x(cam.wrapping_index(1).read()).fsub_m(origin.wrapping_index(1).read());
    let dz = x(cam.wrapping_index(2).read()).fsub_m(origin.wrapping_index(2).read());
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if !d2.fcom(x(22500.0)).test_any(0x41) {
        return;
    }
    if !d2.fcom(x(225.0)).test_any(0x41) && g::<u8>(FRAME_COUNTER).read() & 1 != 0 {
        return;
    }
    let mut mult = [0u32; 8];
    let mp: GPtr<u8> = GPtr::from_raw(&mut mult as *mut [u32; 8]).cast();
    let plife = x(life).fmul_m(f32::from_bits(0x3F4C_CCCD)).to_f32();
    fx_prt_mult_ctor(mp, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, plife);
    let ts = g::<f32>(TIME_STEP).read();
    let at = [
        x(across_x).fmul_m(ts).to_f32(),
        x(across_y).fmul_m(ts).to_f32(),
        x(across_z).fmul_m(ts).to_f32(),
    ];
    if amount <= 0 {
        return;
    }
    let famount = xi(amount).to_f32();
    let neg = x(spread).fchs().to_f32();
    let range = x(spread).fsub_m(neg).to_f32();
    let mut i = 0i32;
    while i < amount {
        let v = [dir.wrapping_index(0).read(), dir.wrapping_index(1).read(), dir.wrapping_index(2).read()];
        let t = xi(i).fdiv_m(famount).fsubr_m(1.0).to_f32();
        let r1 = sa_core::orig::at_site(0x49_F1A1, sa_core::crt::rand);
        let vx = xi(r1)
            .fmul_m(f32::from_bits(RAND_UNIT))
            .fmul(x(spread).fsub_m(neg))
            .fadd_m(neg)
            .fadd_m(v[0])
            .to_f32();
        let r2 = sa_core::orig::at_site(0x49_F1DB, sa_core::crt::rand);
        let vy = xi(r2)
            .fmul_m(f32::from_bits(RAND_UNIT))
            .fmul(x(spread).fsub_m(neg))
            .fadd_m(neg)
            .fadd_m(v[1])
            .to_f32();
        let r3 = sa_core::orig::at_site(0x49_F207, sa_core::crt::rand);
        let vz = xi(r3).fmul_m(f32::from_bits(RAND_UNIT)).fmul_m(range).fadd_m(neg).fadd_m(v[2]);
        let mut vel = [x(vx).fmul_m(force).to_f32(), x(vy).fmul_m(force).to_f32(), vz.fmul_m(force).to_f32()];
        let ax = x(at[0]).fmul_m(t);
        let ay = x(at[1]).fmul_m(t);
        let az = x(at[2]).fmul_m(t).to_f32();
        let mut pos = [
            x(origin.wrapping_index(0).read()).fsub(ax).to_f32(),
            x(origin.wrapping_index(1).read()).fsub(ay).to_f32(),
            x(origin.wrapping_index(2).read()).fsub_m(az).to_f32(),
        ];
        let time = x(t).fmul_m(f32::from_bits(0x3D4C_CCCD)).to_f32();
        // the system pointer is read at every call
        let sys = if kind as u8 != 0 { g::<GPtr<u8>>(0xA9_AE28).read() } else { g::<GPtr<u8>>(0xA9_AE2C).read() };
        fx_add_particle(
            sys,
            GPtr::from_raw(&mut pos as *mut [f32; 3]).cast(),
            GPtr::from_raw(&mut vel as *mut [f32; 3]).cast(),
            time,
            mp,
            -1.0,
            f32::from_bits(0x3F99_999A),
            f32::from_bits(0x3F19_999A),
            0,
        );
        i += 1;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FX_ADD_SPARKS];
