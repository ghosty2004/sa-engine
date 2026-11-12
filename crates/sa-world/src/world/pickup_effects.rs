//! `CPickups::DoPickUpEffects(CEntity*)` (0x455720, batch `std12-world-1`): the pickup object's corona / camera
//! glint, its on-screen price / help message entry and its spin. Unreversed in gta-reversed; ported from the 1.0 US
//! bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{ftol32, g, rd, wr, x, xi};

orig! {
    /// `CPickups::FindPickUpForThisObject(CObject*)` (0x4551C0)
    fn find_pick_up_for_this_object(o: GPtr<u8>) -> GPtr<u8> = cdecl 0x4551C0 => unported;
    /// `CPickup::PickUpShouldBeInvisible()` (0x454D20), AL
    fn pick_up_should_be_invisible(p: GPtr<u8>) -> u8 = thiscall 0x454D20 => unported;
    /// `CPostEffects::IsVisionFXActive()` (0x7034F0), AL
    fn is_vision_fx_active() -> u8 = cdecl 0x7034F0 => unported;
    /// `CCoronas::RegisterCorona(id, attachTo, r, g, b, a, pos, size, farClip, texture, flare, reflection,
    /// checkObstacles, unused, angle, longDistance, nearClip, fadeState, fadeSpeed, onlyFromBelow, reflectionDelay)`
    /// (0x6FC580, the texture-index overload; floats as bits)
    fn register_corona(id: u32, attach: u32, r: u32, gr: u32, b: u32, a: u32, pos: GPtr<u8>, size: u32, far: u32,
        tex: u32, flare: u32, refl: u32, obst: u32, unused: u32, angle: u32, long: u32, near: u32, fade: u32,
        fade_speed: u32, below: u32, delay: u32) = cdecl 0x6FC580 => unported;
    /// `CPickups::WeaponForModel(int32)` (0x454AE0)
    fn weapon_for_model(mi: i32) -> u32 = cdecl 0x454AE0 => unported;
    /// `CSprite::CalcScreenCoors(const RwV3d&, RwV3d*, float* w, float* h, bool, bool)` (0x70CE30), AL
    fn calc_screen_coors(p: GPtr<u8>, out: GPtr<u8>, w: GPtr<u8>, h: GPtr<u8>, max: u32, min: u32) -> u8 = cdecl 0x70CE30 => unported;
    /// `CPickup::FindStringForTextIndex(int32)` (0x455540)
    fn find_string_for_text_index(i: u32) -> GPtr<u8> = cdecl 0x455540 => unported;
    /// `CText::Get(const char*)` (0x6A0050) on `TheText` (0xC1B340)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
}

/// `MI_PICKUP_CAMERA` .. the pickup model index words (`.data`).
fn mi(addr: usize) -> i32 {
    i32::from(g::<u16>(addr).read())
}

fn k(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// The entity's position: the matrix position, or the simple transform's.
fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `if (!m_matrix) { AllocateMatrix(); m_placement.UpdateMatrix(m_matrix); }` before each matrix store.
fn ensure_matrix(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(e, 0x14) == 0 {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// `CPickups::DoPickUpEffects(CEntity* e)` (0x455720, cdecl).
///
/// Invisibility (entity flags +0x140 bit 25): for the camera pickup model (0x8CD5D8) cleared while the active camera
/// mode is 0x2E (the camera weapon), else set and, unless `[0xB70153] >= 5` without a vision effect, a corona:
/// `r = 100 - _ftol(rand * [0x859B60])`, `g = b = _ftol(r * [0x858CB0])`, size `(sin((time & 0x1FFF) * [0x859B5C]) +
/// [0x859B58]) * [0x859B54]`, far clip 100, near clip 1.5, fade speed 15. Other models: bit 25 =
/// `PickUpShouldBeInvisible()`. An invisible pickup ends here.
///
/// The weapon type for the message colour: models 0x8CD5BC, 0x8CD5C8, 0x8CD5CC, 0x8CD5EC, 0x8CD5F0 → 0x2F, 0x8CD5C0 →
/// 0x30, 0x8CD5E0 → 0x35, `WeaponForModel` for unlisted models; for 0x8CD5D0, 0x8CD5C4, 0x8CD5D4, 0x8CD5E8 and 0x8CD5E4
/// the original leaves its argument slot (the entity pointer) as the "type" and indexes the colour table with its low
/// word (kept). With flags +0x140 bits 2/3, the byte +0x13D or the word +0x13E set, the camera closer than [0x859030]
/// (2D) and fewer than 16 messages: a message entry (0x24 bytes at 0x978680) for the on-screen position
/// (`CalcScreenCoors` of the position + [0x858CB0] in z), `WeaponForModel`, the colour, alpha `_ftol((1 - d *
/// [0x859B50]) * [0x859AAC])`, flags, +0x13D, `+0x13E * 5`, and for the two property models the pickup's text.
///
/// Spin: scale from the collision box (`max(sx, sy, sz)` with sx at register precision, `[0x858F08] / m`, at least 1,
/// `(s - 1) * [0x858CC8] + 1`; 1.2 for model 0x16A), angle `(time & 0x7FF) * [0x859B0C]`: the matrix's right / forward
/// / up rows become `(c, s, 0)`, `(-s, c, 0)`, `(0, 0, scale)`.
///
/// The corona id is the original's stack address of its argument slot minus 3 (`lea eax, [esp+0x85]`); the port uses
/// the same expression on its own copy of the argument (`stack-residue`).
#[hook(addr = 0x455720, conv = cdecl, name = "CPickups::DoPickUpEffects", expect = "83 EC 28 53 55 56 8B 74 24 38",
       unverified = "std12-world-1", route = "split", hazards = "x87-transcendental,stack-residue,heap-writes",
       recs = "grove-walk", note = "corona id is a stack address; the weapon-type slot keeps the entity pointer for five models")]
pub fn do_pick_up_effects(e: GPtr<u8>) {
    let slot: [u32; 1] = [e.addr() as u32];
    let mut ty: u32 = slot[0];
    let pickup = find_pick_up_for_this_object(e);
    let model = i32::from(rd::<i16>(e, 0x22));
    const INVIS: u32 = 0x0200_0000;
    if model == mi(0x8CD5D8) {
        let cam = g::<u8>(0xB6F081).read() as usize;
        if g::<u16>(0xB6F1A8 + cam * 0x238).read() == 0x2E {
            wr::<u32>(e, 0x140, rd::<u32>(e, 0x140) & !INVIS);
        } else {
            wr::<u32>(e, 0x140, rd::<u32>(e, 0x140) | INVIS);
            if g::<u8>(0xB70153).read() < 5 || is_vision_fx_active() != 0 {
                let r = sa_core::orig::at_site(0x455798, sa_core::crt::rand);
                let a = ftol32(xi(r & 0xFFFF).fmul_m(k(0x858B14)).fmul_m(k(0x859B60)));
                let red = 100 - a;
                let t = g::<u32>(0xB7CB84).read() & 0x1FFF;
                let size = xi(t as i32).fmul_m(k(0x859B5C)).fsin().fadd_m(k(0x859B58)).fmul_m(k(0x859B54)).to_f32();
                let pos = pos_of(e);
                let gb = ftol32(xi(red).fmul_m(k(0x858CB0))) as u32;
                let id = (slot.as_ptr() as u32).wrapping_sub(3);
                register_corona(id, 0, red as u32, gb, gb, 0xFF, pos, size.to_bits(), 0x42C8_0000, 1, 0, 0, 0, 0, 0, 0,
                    0x3FC0_0000, 0, 0x4170_0000, 0, 0);
            }
        }
    } else {
        let inv = u32::from(pick_up_should_be_invisible(pickup)) << 25;
        let f = rd::<u32>(e, 0x140);
        wr::<u32>(e, 0x140, f ^ ((inv ^ f) & INVIS));
    }
    if rd::<u32>(e, 0x140) & INVIS != 0 {
        return;
    }

    if model == mi(0x8CD5BC) {
        ty = 0x2F;
    } else if model == mi(0x8CD5C0) {
        ty = 0x30;
    } else if [0x8CD5D0, 0x8CD5C4, 0x8CD5D4].iter().any(|&a| model == mi(a)) {
    } else if [0x8CD5C8, 0x8CD5CC].iter().any(|&a| model == mi(a)) {
        ty = 0x2F;
    } else if model == mi(0x8CD5E8) {
    } else if model == mi(0x8CD5EC) {
        ty = 0x2F;
    } else if model == mi(0x8CD5E0) {
        ty = 0x35;
    } else if model == mi(0x8CD5E4) {
    } else if model == mi(0x8CD5F0) {
        ty = 0x2F;
    } else {
        ty = weapon_for_model(model);
    }

    let flags = rd::<u32>(e, 0x140);
    if flags & 0xC != 0 || rd::<u8>(e, 0x13D) != 0 || rd::<u16>(e, 0x13E) != 0 {
        message(e, model, ty);
    }
    spin(e);
}

/// The message entry of [`do_pick_up_effects`] (0x455964..0x455C07).
fn message(e: GPtr<u8>, model: i32, ty: u32) {
    let cm = g::<GPtr<u8>>(0xB6F03C).read();
    let c = if cm.is_null() { GPtr::<u8>::from_addr(0xB6F02C) } else { cm.byte_add(0x30) };
    let p = pos_of(e);
    let dy = x(rd::<f32>(c, 4)).fsub_m(rd::<f32>(p, 4));
    let dx = x(rd::<f32>(c, 0)).fsub_m(rd::<f32>(p, 0));
    let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
    let df = d.to_f32();
    if d.fcom(x(k(0x859030))).test_parity(0x05) {
        return;
    }
    let n = g::<u16>(0x978678);
    if n.read() >= 0x10 {
        return;
    }
    let p = pos_of(e);
    let mut inp = [rd::<f32>(p, 0), rd::<f32>(p, 4), x(rd::<f32>(p, 8)).fadd_m(k(0x858CB0)).to_f32()];
    let mut out = [0f32; 3];
    let mut w = [0f32; 1];
    let mut h = [0f32; 1];
    let ok = calc_screen_coors(
        GPtr::from_raw(inp.as_mut_ptr()).cast(),
        GPtr::from_raw(out.as_mut_ptr()).cast(),
        GPtr::from_raw(w.as_mut_ptr()).cast(),
        GPtr::from_raw(h.as_mut_ptr()).cast(),
        1,
        1,
    );
    if ok == 0 {
        return;
    }
    let m = GPtr::<u8>::from_addr(0x978680 + usize::from(n.read()) * 0x24);
    wr::<f32>(m, 0, out[0]);
    wr::<f32>(m, 4, out[1]);
    wr::<f32>(m, 0xC, w[0]);
    wr::<f32>(m, 0x10, h[0]);
    let m = GPtr::<u8>::from_addr(0x978680 + usize::from(n.read()) * 0x24);
    wr::<u32>(m, 8, weapon_for_model(model));
    let col = GPtr::<u8>::from_addr((0x8A5FB0u32.wrapping_add((i32::from(ty as i16) << 3) as u32)) as usize);
    wr::<u8>(m, 0x14, rd::<u8>(col, 0));
    wr::<u8>(m, 0x15, rd::<u8>(col, 1));
    wr::<u8>(m, 0x16, rd::<u8>(col, 2));
    let a = ftol32(x(df).fmul_m(k(0x859B50)).fsubr_m(k(0x858624)).fmul_m(k(0x859AAC)));
    wr::<u8>(m, 0x17, a as u8);
    let fl = rd::<u8>(m, 0x18);
    wr::<u8>(m, 0x18, if rd::<u32>(e, 0x140) & 8 != 0 { fl | 1 } else { fl & 0xFE });
    wr::<u8>(m, 0x19, rd::<u8>(e, 0x13D));
    wr::<u32>(m, 0x1C, u32::from(rd::<u16>(e, 0x13E)) * 5);
    let property = |m: GPtr<u8>| {
        let pk = find_pick_up_for_this_object(e);
        let key = find_string_for_text_index(u32::from((rd::<u8>(pk, 0x1D) >> 4) & 7));
        wr::<u32>(m, 0x20, text_get(GPtr::from_addr(0xC1B340), key).addr() as u32);
    };
    let bit1 = if model == mi(0x8CD5E8) {
        property(GPtr::<u8>::from_addr(0x978680 + usize::from(n.read()) * 0x24));
        false
    } else if model == mi(0x8CD5EC) {
        property(GPtr::<u8>::from_addr(0x978680 + usize::from(n.read()) * 0x24));
        true
    } else {
        wr::<u32>(m, 0x20, 0);
        false
    };
    let m = GPtr::<u8>::from_addr(0x978680 + usize::from(n.read()) * 0x24);
    let fl = rd::<u8>(m, 0x18);
    n.write(n.read().wrapping_add(1));
    wr::<u8>(m, 0x18, if bit1 { fl | 2 } else { fl & 0xFD });
}

/// The spin of [`do_pick_up_effects`] (0x455C07..).
fn spin(e: GPtr<u8>) {
    let model = rd::<i16>(e, 0x22);
    let mi = sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(i32::from(model)).read();
    let bb = rd::<GPtr<u8>>(mi, 0x14);
    let sx = x(rd::<f32>(bb, 0xC)).fsub_m(rd::<f32>(bb, 0));
    let sy = x(rd::<f32>(bb, 0x10)).fsub_m(rd::<f32>(bb, 4)).to_f32();
    let sz = x(rd::<f32>(bb, 0x14)).fsub_m(rd::<f32>(bb, 8)).to_f32();
    let max_yz = || if !x(sy).fcom(x(sz)).test_any(0x41) { x(sy) } else { x(sz) };
    let m = if sx.fcom(max_yz()).test_any(0x41) { max_yz() } else { sx };
    let mut q = m.fdivr_m(k(0x858F08));
    if !x(k(0x858624)).fcom(q).test_any(0x41) {
        q = x(k(0x858624));
    }
    let mut scale = q.fsub_m(k(0x858624)).fmul_m(k(0x858CC8)).fadd_m(k(0x858624)).to_f32();
    if model as u16 == 0x16A {
        scale = f32::from_bits(0x3F99_999A);
    }
    let t = g::<u32>(0xB7CB84).read() & 0x7FF;
    let a: X87 = xi(t as i32).fmul_m(k(0x859B0C));
    let c = a.fcos().fmul_m(scale).to_f32();
    let s = a.fsin().fmul_m(scale).to_f32();
    wr::<f32>(ensure_matrix(e), 0, c);
    wr::<f32>(ensure_matrix(e), 4, s);
    wr::<u32>(ensure_matrix(e), 8, 0);
    let m = ensure_matrix(e);
    wr::<f32>(m, 0x10, x(s).fchs().to_f32());
    wr::<f32>(ensure_matrix(e), 0x14, c);
    wr::<u32>(ensure_matrix(e), 0x18, 0);
    wr::<u32>(ensure_matrix(e), 0x20, 0);
    wr::<u32>(ensure_matrix(e), 0x24, 0);
    wr::<f32>(ensure_matrix(e), 0x28, scale);
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_DO_PICK_UP_EFFECTS];
