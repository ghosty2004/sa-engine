//! `CTrafficLights::DisplayActualLight` (WRITER-POP-4, lead). Ported from the 1.0 US bytes; gta-reversed
//! and `docs/re/specs/POP/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne_mask, jp5, model_info, rd, x, xi};
use crate::misc::{allocate_matrix, update_matrix};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// `CTrafficLights::FindOrientationForTrafficLightType(CEntity*)` (0x49D520, ST0)
    fn find_orientation_call(e: GPtr<u8>) -> f64 = cdecl 0x49D520 => find_orientation_abi;
    /// `CTrafficLights::FindTrafficLightTypeFromOrientation(float)` (0x49D4D0)
    fn find_type_from_orientation_call(a: f32) -> i32 = cdecl 0x49D4D0 => crate::misc::find_traffic_light_type_from_orientation;
    /// `CTrafficLights::LightForCars1_Visual()` (0x49D350)
    fn light_for_cars1_visual_call() -> u8 = cdecl 0x49D350 => crate::misc::light_for_cars1_visual;
    /// `CTrafficLights::LightForCars2_Visual()` (0x49D3A0)
    fn light_for_cars2_visual_call() -> u8 = cdecl 0x49D3A0 => crate::misc::light_for_cars2_visual;
    /// `CBaseModelInfo::Get2dEffect(int32)` (0x4C4C70, thiscall)
    fn get_2d_effect(mi: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4C4C70 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, hidden result pointer first)
    fn matrix_mul_vec(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C890 => unported;
    /// `CCoronas::RegisterCorona(uint32 id, CEntity*, uint8 r, g, b, a, const CVector&, float radius, float
    /// farClip, RwTexture*, flare, refl, los, trail, float normalAngle, bool neonFade, float pull, bool
    /// fullBrightAtStart, float fadeSpeed, bool onlyFromBelow, bool reflectionDelay)` (0x6FC580); floats as bits
    fn register_corona(id: u32, ent: u32, r: u32, gr: u32, b: u32, a: u32, pos: GPtr<f32>, radius: u32, far: u32, tex: u32, flare: u32, refl: u32, los: u32, trail: u32, normal_angle: u32, neon: u32, pull: u32, full_bright: u32, fade: u32, below: u32, refl_delay: u32) = cdecl 0x6FC580 => unported;
    /// `CBrightLights::RegisterOne(CVector pos, CVector up, CVector right, CVector fwd, uint8 color, uint8,
    /// uint8, uint8)` (0x724770, vectors by value as bits)
    fn bright_lights_register_one(p0: u32, p1: u32, p2: u32, u0: u32, u1: u32, u2: u32, r0: u32, r1: u32, r2: u32, f0: u32, f1: u32, f2: u32, color: u32, a: u32, b: u32, c: u32) = cdecl 0x724770 => unported;
    /// `CPointLights::AddLight(uint8 type, CVector pos, CVector dir, float radius, float r, float g, float b,
    /// uint8 fogType, bool extraShadows, CEntity* attached)` (0x7000E0, by-value vectors and floats as bits)
    fn point_lights_add_light(ty: u32, px: u32, py: u32, pz: u32, dx: u32, dy: u32, dz: u32, radius: u32, r: u32, gr: u32, b: u32, fog: u32, shadows: u32, attached: u32) = cdecl 0x7000E0 => unported;
    /// `CShadows::StoreStaticShadow(uint32 id, uint8 type, RwTexture*, const CVector* pos, float frontX, float
    /// frontY, float sideX, float sideY, int16 intensity, uint8 r, uint8 g, uint8 b, float zDistance, float
    /// scale, float drawDistance, bool temporary, float upDistance)` (0x70BA00, floats as bits)
    fn store_static_shadow(id: u32, ty: u32, tex: u32, pos: GPtr<f32>, fx: u32, fy: u32, sx: u32, sy: u32, intensity: u32, r: u32, gr: u32, b: u32, zd: u32, scale: u32, draw: u32, temp: u32, up: u32) = cdecl 0x70BA00 => unported;
    /// `CShinyTexts::RegisterOne(CVector p0, p1, p2, p3, float u0, v0, u1, v1, u2, v2, u3, v3, uint8 type,
    /// uint8 r, g, b, float maxDistance)` (0x724B60, by value, floats as bits)
    fn shiny_texts_register_one(p0: u32, p1: u32, p2: u32, q0: u32, q1: u32, q2: u32, s0: u32, s1: u32, s2: u32, t0: u32, t1: u32, t2: u32, u0: u32, v0: u32, u1: u32, v1: u32, u2: u32, v2: u32, u3: u32, v3: u32, ty: u32, r: u32, gr: u32, b: u32, max_dist: u32) = cdecl 0x724B60 => unported;
}

/// The `orig!` routing target of 0x49D520 (an ST0 result crosses `orig!` as an f64).
#[cfg_attr(sa_phase_a, allow(dead_code))]
fn find_orientation_abi(e: GPtr<u8>) -> f64 {
    crate::misc::find_orientation_for_traffic_light_type(e).to_f64()
}

/// The inlined `CPlaceable::GetMatrix`: `AllocateMatrix` + `CSimpleTransform::UpdateMatrix` when +0x14 is null.
fn matrix(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        allocate_matrix(e);
        update_matrix(e.byte_add(4), rd(e, 0x14));
    }
    rd(e, 0x14)
}

/// `operator*(m, v)` into a fresh local, returned as bits.
fn transform(m: GPtr<u8>, v: [f32; 3]) -> [u32; 3] {
    let mut vin = v;
    let mut out = [0f32; 3];
    matrix_mul_vec(
        GPtr::from_raw(out.as_mut_ptr()),
        m,
        GPtr::from_raw(vin.as_mut_ptr()),
    );
    [out[0].to_bits(), out[1].to_bits(), out[2].to_bits()]
}

/// A row of three f32 at `m + off`, as bits.
fn row(m: GPtr<u8>, off: isize) -> [u32; 3] {
    [rd::<u32>(m, off), rd::<u32>(m, off + 4), rd::<u32>(m, off + 8)]
}

/// `vec` guarded by bit `bit` of 0xA9ADE0 (a function-local static `CVector` initialised once with `init`).
fn static_vec(addr: usize, bit: u32, init: [u32; 3]) -> [f32; 3] {
    let flags = g::<u32>(0xA9ADE0);
    let f = flags.read();
    if f & bit == 0 {
        flags.write(f | bit);
        for (k, v) in init.iter().enumerate() {
            g::<u32>(addr + 4 * k).write(*v);
        }
    }
    [
        g::<f32>(addr).read(),
        g::<f32>(addr + 4).read(),
        g::<f32>(addr + 8).read(),
    ]
}

/// `(a + b) * 0.5`, stored f32.
fn mid(a: f32, b: f32) -> f32 {
    x(a).fadd_m(b).fmul_m(0.5).to_f32()
}

/// `CTrafficLights::DisplayActualLight(CEntity*)` (0x49DAB0), from the bytes:
/// - returns when the matrix's up.z (`+0x28`) is below 0.96 (`test ah, 5; jnp`, so NaN continues) or bit 9 of
///   `+0x1C` is set. Bright-light colour base 1, or 0xD for the models `ModelIndices` 0x8CD508 / 0x8CD50C /
///   0x8CD510 / 0x8CD514 / 0x8CD4FC (`movsx` model vs `movzx` index).
/// - light state: `LightForCars1_Visual` when `FindTrafficLightTypeFromOrientation((float)FindOrientation…)`
///   is 1, else `LightForCars2_Visual` (the inlined `FindTrafficLightType`).
/// - `sameDir = cam.fwd · m.fwd > 0` (0xB6F9AC.., summed y*y', z*z', then x*x' on the FPU).
/// - for each 2D effect of type 0 (light): `lp = m * effect.pos`, `centre += lp` (f32); colour state 0, or 2 /
///   1 when its red byte > 200 (green byte > 100 gives 1); skipped when `(pos.y > 0) == sameDir` or the colour
///   state differs from the light state; else a corona (id `entity + i`, radius `[0xB7C4E0] * 0.175`, colours
///   `ftol(table[state] * ([0xB7C4E4] * 0.07))` from the tables 0x8A6214 / 0x8A6218 / 0xA9AD94 as r / g / b,
///   far clip 50, refl 1, pull 1.5, fade 15) and `CBrightLights::RegisterOne(lp, m.up, m.right, 0, base +
///   state)`.
/// - with effects: `centre *= 1 / count` (80-bit reciprocal); for state < 3, a point light when
///   `TrafficLightsBrightness` (0xC812A8) > 0.5 (colours `max(table, 50) * 0.0015686` per channel, radius 14)
///   and a static shadow when it is > 0.05 (colours `ftol(table * [0xB7C4F8] * brightness * 0.0125)`).
/// - pedestrian light for four models: corners from constants or function-local statics (guard bits of
///   0xA9ADE0); the state inlined from `LightForPeds` (`(ms >> 1) & 0x3FFF`: < 0x2EE0 red, < 0x3C18 green,
///   else flashing on bit 8 of the time); the half of the sprite as `CShinyTexts::RegisterOne` quads.
#[hook(addr = 0x49DAB0, conv = cdecl, name = "CTrafficLights::DisplayActualLight", expect = "81 EC 9C 00 00 00 53 55",
       unverified = "popw4-frame-1", route = "L2", hazards = "heap-writes",
       recs = "driving,car-ai,grove-walk,police-gang",
       note = "every frame per visible traffic light; writes coronas / bright lights / point lights / static shadows / shiny texts tables and may allocate the entity matrix")]
pub fn display_actual_light(e: GPtr<u8>) {
    let m = matrix(e);
    if !jp5(x(rd::<f32>(m, 0x28)), x(f32::from_bits(0x3F75_C28F))) || rd::<u32>(e, 0x1C) & 0x200 != 0 {
        return;
    }
    let model = i32::from(rd::<i16>(e, 0x22));
    let idx = |a: usize| i32::from(g::<u16>(a).read());
    let base: u32 = if [0x8CD508, 0x8CD50C, 0x8CD510, 0x8CD514, 0x8CD4FC]
        .iter()
        .any(|&a| model == idx(a))
    {
        0xD
    } else {
        1
    };
    let orient = X87::from_f64_exact(find_orientation_call(e)).to_f32();
    let state = u32::from(if find_type_from_orientation_call(orient) == 1 {
        light_for_cars1_visual_call()
    } else {
        light_for_cars2_visual_call()
    });
    let mi = model_info(i32::from(rd::<i16>(e, 0x22)));
    let m = matrix(e);
    let dot = x(g::<f32>(0xB6F9B0).read())
        .fmul_m(rd::<f32>(m, 0x14))
        .fadd(x(g::<f32>(0xB6F9B4).read()).fmul_m(rd::<f32>(m, 0x18)))
        .fadd(x(g::<f32>(0xB6F9AC).read()).fmul_m(rd::<f32>(m, 0x10)));
    let same_dir = !jne_mask(dot, X87::zero(), 0x41);
    let count = i32::from(rd::<u8>(mi, 0xD));
    let tr = |s: u32| i32::from(g::<u8>(0x8A6214 + s as usize).read());
    let tg = |s: u32| i32::from(g::<u8>(0x8A6218 + s as usize).read());
    let tb = |s: u32| i32::from(g::<u8>(0xA9AD94 + s as usize).read());
    let mut centre = [0f32; 3];
    for i in 0..count {
        let eff = get_2d_effect(mi, i);
        if rd::<u8>(eff, 0xC) != 0 {
            continue;
        }
        let m = matrix(e);
        let mut lp = [0f32; 3];
        matrix_mul_vec(GPtr::from_raw(lp.as_mut_ptr()), m, eff.cast());
        for k in 0..3 {
            centre[k] = x(centre[k]).fadd_m(lp[k]).to_f32();
        }
        let red = rd::<u8>(eff, 0x10);
        let cs: u32 = if red > 200 {
            if rd::<u8>(eff, 0x11) > 100 { 1 } else { 2 }
        } else {
            0
        };
        let y_pos = !jne_mask(x(rd::<f32>(eff, 4)), X87::zero(), 0x41);
        if y_pos == same_dir || cs != state {
            continue;
        }
        let bright = x(g::<f32>(0xB7C4E4).read()).fmul_m(f32::from_bits(0x3D8F_5C29));
        let radius = x(g::<f32>(0xB7C4E0).read())
            .fmul_m(f32::from_bits(0x3E33_3333))
            .to_f32();
        let cb = ftol32(xi(tb(cs)).fmul(bright)) as u32;
        let cg = ftol32(xi(tg(cs)).fmul(bright)) as u32;
        let cr = ftol32(xi(tr(cs)).fmul(bright)) as u32;
        register_corona(
            e.addr() as u32 + i as u32,
            0,
            cr,
            cg,
            cb,
            0xFF,
            GPtr::from_raw(lp.as_mut_ptr()),
            radius.to_bits(),
            50f32.to_bits(),
            0,
            0,
            1,
            0,
            0,
            0,
            0,
            1.5f32.to_bits(),
            0,
            15f32.to_bits(),
            0,
            0,
        );
        let m = matrix(e);
        let up = row(m, 0x20);
        let right = row(m, 0);
        let color = u32::from((base as u8).wrapping_add(state as u8));
        bright_lights_register_one(
            lp[0].to_bits(),
            lp[1].to_bits(),
            lp[2].to_bits(),
            up[0],
            up[1],
            up[2],
            right[0],
            right[1],
            right[2],
            0,
            0,
            0,
            color,
            0,
            0,
            0,
        );
    }
    if count != 0 {
        let inv = xi(count).fdivr_m(1.0);
        for c in centre.iter_mut() {
            *c = x(*c).fmul(inv).to_f32();
        }
        if state < 3 {
            let tlb = || x(g::<f32>(0xC812A8).read());
            if !jne_mask(tlb(), x(0.5), 0x41) {
                let mult = f32::from_bits(0x3ACD_9A67);
                let b = tb(state).max(50);
                let gg = tg(state).max(50);
                let r = tr(state).max(50);
                point_lights_add_light(
                    0,
                    centre[0].to_bits(),
                    centre[1].to_bits(),
                    centre[2].to_bits(),
                    0,
                    0,
                    0,
                    14f32.to_bits(),
                    xi(r).fmul_m(mult).to_f32().to_bits(),
                    xi(gg).fmul_m(mult).to_f32().to_bits(),
                    xi(b).fmul_m(mult).to_f32().to_bits(),
                    1,
                    1,
                    0,
                );
            }
            if !jne_mask(tlb(), x(f32::from_bits(0x3D4C_CCCD)), 0x41) {
                let col = |v: i32| {
                    ftol32(
                        xi(v)
                            .fmul_m(g::<f32>(0xB7C4F8).read())
                            .fmul_m(g::<f32>(0xC812A8).read())
                            .fmul_m(f32::from_bits(0x3C4C_CCCD)),
                    ) as u32
                };
                let b = col(tb(state));
                let gg = col(tg(state));
                let r = col(tr(state));
                store_static_shadow(
                    e.addr() as u32,
                    2,
                    g::<u32>(0xC403F4).read(),
                    GPtr::from_raw(centre.as_mut_ptr()),
                    8f32.to_bits(),
                    0,
                    0,
                    (-8f32).to_bits(),
                    0x80,
                    r,
                    gg,
                    b,
                    12f32.to_bits(),
                    1f32.to_bits(),
                    40f32.to_bits(),
                    0,
                    0,
                );
            }
        }
    }
    // the pedestrian light: corners `a` ([esp+0x14]) and `b` ([esp+0x2C]) per model
    let (a, b): ([f32; 3], [f32; 3]) = if model == idx(0x8CD50C) || model == idx(0x8CD514) {
        (
            [
                f32::from_bits(0x3E41_8937),
                f32::from_bits(0xBE20_C49C),
                f32::from_bits(0x3FEC_8B44),
            ],
            [
                f32::from_bits(0xBE10_624E),
                f32::from_bits(0xBE20_C49C),
                f32::from_bits(0x3FCD_B22D),
            ],
        )
    } else if model == idx(0x8CD510) {
        let b = static_vec(0xA9ADD4, 1, [0xBD34_3958, 0xBE85_A1CB, 0x3FC7_CED9]);
        let a = static_vec(0xA9ADC8, 2, [0x3E63_53F8, 0xBE85_A1CB, 0x3FEE_F9DB]);
        (a, b)
    } else if model == idx(0x8CD4F4) {
        let b = static_vec(0xA9ADBC, 4, [0x402B_126F, 0x3FA1_8937, 0xBF09_FBE7]);
        let a = static_vec(0xA9ADB0, 8, [0x402B_126F, 0x3F36_872B, 0xBE02_0C4A]);
        (a, b)
    } else {
        return;
    };
    let ms = g::<u32>(0xB7CB84).read();
    let t = (ms >> 1) & 0x3FFF;
    let peds = if t < 0x2EE0 {
        2
    } else if t < 0x3C18 {
        0
    } else {
        1
    };
    if peds == 2 {
        let mx = mid(a[0], b[0]);
        let my = mid(a[1], b[1]);
        let m1 = matrix(e);
        let m2 = matrix(e);
        let m3 = matrix(e);
        let m4 = matrix(e);
        let r1 = transform(m1, [mx, my, b[2]]);
        let r2 = transform(m2, b);
        let r3 = transform(m3, [mx, my, a[2]]);
        let r4 = transform(m4, [b[0], b[1], a[2]]);
        shiny_texts_register_one(
            r4[0],
            r4[1],
            r4[2],
            r3[0],
            r3[1],
            r3[2],
            r2[0],
            r2[1],
            r2[2],
            r1[0],
            r1[1],
            r1[2],
            0.5f32.to_bits(),
            0,
            0,
            0,
            0.5f32.to_bits(),
            1f32.to_bits(),
            0,
            1f32.to_bits(),
            1,
            0xFF,
            0,
            0,
            60f32.to_bits(),
        );
        return;
    }
    if peds != 0 && ms & 0x100 == 0 {
        return;
    }
    let m1 = matrix(e);
    let mx = mid(a[0], b[0]);
    let my = mid(a[1], b[1]);
    let m2 = matrix(e);
    let m3 = matrix(e);
    let m4 = matrix(e);
    let r1 = transform(m1, [a[0], a[1], b[2]]);
    let r2 = transform(m2, [mx, my, b[2]]);
    let r3 = transform(m3, a);
    let r4 = transform(m4, [mx, my, a[2]]);
    shiny_texts_register_one(
        r4[0],
        r4[1],
        r4[2],
        r3[0],
        r3[1],
        r3[2],
        r2[0],
        r2[1],
        r2[2],
        r1[0],
        r1[1],
        r1[2],
        1f32.to_bits(),
        0,
        0.5f32.to_bits(),
        0,
        1f32.to_bits(),
        1f32.to_bits(),
        0.5f32.to_bits(),
        1f32.to_bits(),
        1,
        0xFF,
        0xFF,
        0xFF,
        60f32.to_bits(),
    );
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_DISPLAY_ACTUAL_LIGHT];
