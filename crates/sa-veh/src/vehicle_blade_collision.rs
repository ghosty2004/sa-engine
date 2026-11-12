//! PORT-VEH-4 (batch `veh4-3`): `CVehicle::DoBladeCollision`. Ported from the 1.0 US bytes; gta-reversed
//! as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, x};

orig! {
    /// `CBox::Set(const CVector& min, const CVector& max)` (0x40EDE0)
    fn box_set(b: GPtr<u8>, min: GPtr<u8>, max: GPtr<u8>) = thiscall 0x40EDE0 => unported;
    /// `CSphere::Set(float radius, const CVector& center)` (0x40FCF0, a `.HOODLUM` stub)
    fn sphere_set(s: GPtr<u8>, radius: u32, center: GPtr<u8>) = thiscall 0x40FCF0 => unported;
    /// `CColSphere::Set(float radius, const CVector& center, uint8 material, uint8 flags, uint8 lighting)` (0x40FD10)
    fn col_sphere_set(s: GPtr<u8>, radius: u32, center: GPtr<u8>, material: u32, flags: u32, light: u32) = thiscall 0x40FD10 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, the result through `out`)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CVehicle::BladeColSectorList(CPtrListSingleLink&, CColModel&, CMatrix&, int16 rotorType, float
    /// damageMult)` (0x6DAF00, AL)
    fn blade_col_sector_list(v: GPtr<u8>, list: GPtr<u8>, col: GPtr<u8>, m: GPtr<u8>, rotor: u32, mult: u32) -> u8 = thiscall 0x6DAF00 => unported;
}

/// CRT `floor` (0x8219F0) of a double: exact.
fn floor_f64(d: f64) -> f64 {
    if d.is_nan() || d.abs() >= 4_503_599_627_370_496.0 {
        return d;
    }
    let t = d as i64 as f64;
    if t > d { t - 1.0 } else { t }
}

/// `_ftol(floor((c +- r) * 0.02 + 60))` (the sector index the original computes).
fn sector(c: f32, r: f32, plus: bool) -> i32 {
    let v = if plus { x(c).fadd_m(r) } else { x(c).fsub_m(r) };
    let d = v.fmul_m(f32::from_bits(0x3CA3_D70A)).fadd_m(60.0).to_f64();
    ftol32(X87::from_f64_exact(floor_f64(d)))
}

/// The blade test model (0xC1CD38: its bounding box, sphere 0xC1CD50, col data 0xC1CD68 with the one
/// col sphere 0xC1CD98).
const MODEL: usize = 0xC1_CD38;

/// `CVehicle::DoBladeCollision(CVector pos, CMatrix& matrix, int16 rotorType, float radius, float
/// damageMult)` (0x6E2E50): a blade disc of `radius` at `pos` (its box flattened to the 0.05 thickness
/// at 0x8D34EC along x for rotor types +-1, y for +-2, z for +-3) becomes the static test model's box,
/// bounding sphere and single col sphere; every world sector within the disc's world-space bounds
/// (clamped to 0..119; the start index forced to 0 unless positive, computed twice as the original
/// does) is tested with `BladeColSectorList` (buildings, vehicles, peds without damage, objects) under
/// a new scan code. The model's col data is cleared again. True when any list reported a hit.
///
/// Kept as in the original: the hit flag is kept in the low byte of the rotor-type argument and the
/// sector bounds in the radius/matrix argument slots (locals here).
#[hook(addr = 0x6E2E50, conv = thiscall, name = "CVehicle::DoBladeCollision", this = "CVehicle",
       expect = "83 EC 38 8B 44 24 50",
       unverified = "veh4-3", route = "L2", hazards = "heap-writes",
       note = "writes the static blade col model 0xC1CD38..0xC1CDA8 and the scan code; BladeColSectorList damages and pushes what the blades hit")]
#[allow(clippy::too_many_arguments)]
pub fn vehicle_do_blade_collision(this: GPtr<u8>, px: f32, py: f32, pz: f32, matrix: GPtr<u8>, rotor: u32, radius: f32, mult: u32) -> u8 {
    let r = radius;
    let nr = f32_of(x(r).fchs());
    let thick = g::<f32>(0x8D_34EC).read();
    let nthick = f32_of(x(thick).fchs());
    let mut mx = [r, r, r];
    let mut mn = [nr, nr, nr];
    match rotor as i16 {
        -1 | 1 => {
            mx[0] = thick;
            mn[0] = nthick;
        }
        -2 | 2 => {
            mx[1] = thick;
            mn[1] = nthick;
        }
        -3 | 3 => {
            mx[2] = thick;
            mn[2] = nthick;
        }
        _ => {}
    }
    let p = [px, py, pz];
    let mut lo = [0u32; 3];
    let mut hi = [0u32; 3];
    for k in 0..3 {
        lo[k] = f32_of(x(p[k]).fadd_m(mn[k])).to_bits();
        hi[k] = f32_of(x(p[k]).fadd_m(mx[k])).to_bits();
    }
    let buf = |b: &mut [u32; 3]| -> GPtr<u8> { GPtr::from_raw(b as *mut [u32; 3]).cast() };
    box_set(GPtr::from_addr(MODEL), buf(&mut lo), buf(&mut hi));
    let mut pos = [px.to_bits(), py.to_bits(), pz.to_bits()];
    sphere_set(GPtr::from_addr(MODEL + 0x18), r.to_bits(), buf(&mut pos));
    g::<u32>(MODEL + 0x2C).write(0xC1_CD68);
    col_sphere_set(GPtr::from_addr(MODEL + 0x60), r.to_bits(), buf(&mut pos), 0, 0, 0xFF);
    g::<u32>(MODEL + 0x38).write(0xC1_CD98);
    g::<u16>(MODEL + 0x30).write(1);
    let mut c = [0u32; 3];
    matrix_mul_vec(buf(&mut c), matrix, buf(&mut pos));
    let (cx, cy) = (f32::from_bits(c[0]), f32::from_bits(c[1]));
    let mut hit = false;
    let x0 = if sector(cx, r, false) > 0 { sector(cx, r, false) } else { 0 };
    let y0 = if sector(cy, r, false) > 0 { sector(cy, r, false) } else { 0 };
    let x1 = if sector(cx, r, true) < 0x78 { sector(cx, r, true) } else { 0x78 };
    let y1 = if sector(cy, r, true) < 0x78 { sector(cy, r, true) } else { 0x78 };
    let scan = g::<u16>(0xB7_CD78);
    if scan.read() < 0xFFFF {
        scan.write(scan.read() + 1);
    } else {
        clear_scan_codes();
        scan.write(1);
    }
    let model = GPtr::<u8>::from_addr(MODEL);
    let mut y = y0;
    while y <= y1 {
        let mut xx = x0;
        while xx <= x1 {
            let sy = if y > 0 { y } else { 0 }.min(0x77);
            let sx = if xx > 0 { xx } else { 0 }.min(0x77);
            let sector = GPtr::<u8>::from_addr(0xB7_D0B8 + 8 * (sy * 0x78 + sx) as usize);
            let rep = ((xx & 0xF) + ((y & 0xF) << 4)) as usize;
            let rep = GPtr::<u8>::from_addr(0xB9_92B8 + 12 * rep);
            if blade_col_sector_list(this, sector, model, matrix, rotor, mult) != 0 {
                hit = true;
            }
            if blade_col_sector_list(this, rep, model, matrix, rotor, mult) != 0 {
                hit = true;
            }
            if blade_col_sector_list(this, rep.byte_add(4), model, matrix, rotor, 0) != 0 {
                hit = true;
            }
            if blade_col_sector_list(this, rep.byte_add(8), model, matrix, rotor, mult) != 0 {
                hit = true;
            }
            xx += 1;
        }
        y += 1;
    }
    g::<u32>(MODEL + 0x38).write(0);
    g::<u16>(MODEL + 0x30).write(0);
    g::<u32>(MODEL + 0x2C).write(0);
    u8::from(hit)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_DO_BLADE_COLLISION];
