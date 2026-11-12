//! WRITER-WORLD (batch `world5-1`, SA:MP-executed rows): `WaterCreature_c::Init` and `Update`, the
//! fish / jellyfish / turtles / dolphins swimming around the player. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win: every random range is inlined differently, see
//! `docs/re/specs/WORLD/WaterCreature_c.deviations.md`).
//!
//! `WaterCreature_c` (0x34): +8 object, +0xC type, +0xD changed-direction flag, +0xE speed-change
//! frames, +0x10 / +0x12 speed-change timers (u16), +0x14 default speed, +0x18 current speed, +0x1C
//! followed creature, +0x20 offset to the followed creature, +0x2C heading, +0x30 active.
//! `tWaterCreatureInfo` (0x2C, table 0x8D3698): +0 model index pointer, +8 min depth, +0x10 / +0x14
//! min / max distance from the followed creature, +0x18 / +0x1C min / max speed.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use super::common::{g, rd, wr, x, xi};

orig! {
    /// `CObject::operator new(uint32)` (0x5A1EE0)
    fn object_new(size: u32) -> GPtr<u8> = cdecl 0x5A1EE0 => unported;
    /// `CObject::CObject(int32 modelId, bool create)` (0x5A1D70)
    fn object_ctor(o: GPtr<u8>, model: u32, create: u32) -> GPtr<u8> = thiscall 0x5A1D70 => unported;
    /// virtual slot +0x10 (`CEntity::SetIsStatic(bool)`)
    fn v_set_is_static(e: GPtr<u8>, on: u32) = thiscall vslot 0x10 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn placeable_set_posn(e: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4241C0 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, a: u32) = thiscall 0x59B020 => unported;
    /// `CMatrix::UpdateRW(RwMatrix*)` (0x59AD70)
    fn matrix_update_rw(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
}

/// `1 / 32767` (0x858C7C).
const RAND_K: f32 = f32::from_bits(0x3800_0100);

/// `(float)rand() * (1/32767)` with the original return address `site`.
fn rand_t(site: u32) -> sa_math::x87::X87 {
    xi(at_site(site, rand)).fmul_m(RAND_K)
}

/// A Rust local passed by address to an original callee.
fn p<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// The object's heading: `CMatrix::SetRotateZOnly(heading)` on its matrix, or the placement heading
/// (+0x10) without one.
fn apply_heading(obj: GPtr<u8>, heading: u32) {
    let m = rd::<GPtr<u8>>(obj, 0x14);
    if !m.is_null() {
        matrix_set_rotate_z_only(m, heading);
    } else {
        wr::<u32>(obj, 0x10, heading);
    }
}

/// `WaterCreature_c::Init(int32 type, CVector* pos, WaterCreature_c* followed, float a, float b)`
/// (0x6E4160): false at the 150-creature cap (0xBB4A70). Without a creature to follow the spawn
/// depth is drawn first: `hi = a - info.minDepth`, `lo = max((a - b) + 2, a - 50)` (the larger
/// unless unordered), false when `hi < lo`, else `pos.z = (hi - lo) * t + lo`. Then a `CObject` of
/// the type's model (pool locked for the allocation), false when the pool is full; it gets the
/// current area, static, flags (+0x1C bit 20 on, bit 0 off, +0x40 bit 1 off, bit 14 on for types 3
/// and 4), created-by 5. A follower is placed at a random offset (`dir = (2t-1, 2t-1, t-0.5) *
/// lerp(minDist, maxDist)`) from `pos`, keeps the offset back to it, takes the followed object's
/// heading (`atan2(-up.x, up.y)` with a matrix, else the placement heading) and a speed between the
/// minimum and the followed one's; a leader sits at `pos` with a random heading (`t * 2pi - pi`)
/// and speed. The heading is applied, the object added to the world, the creature counted. True.
#[hook(addr = 0x6E4160, conv = thiscall, name = "WaterCreature_c::Init", this = "WaterCreature_c",
       expect = "64 A1 00 00 00 00",
       unverified = "world5-1", route = "split", hazards = "heap-writes,x87-transcendental",
       note = "creates a CObject (object pool) and adds it to the world; rand at_site 0x6E421A, 0x6E42FE/0x6E431D/0x6E433C/0x6E4359/0x6E4417 (follower), 0x6E4442/0x6E4464 (leader); FPATAN of the followed heading")]
pub fn init(this: GPtr<u8>, ty: i32, pos: GPtr<u8>, followed: GPtr<u8>, a: f32, b: f32) -> u8 {
    if g::<u16>(0xBB_4A70).read() >= 0x96 {
        return 0;
    }
    let info = GPtr::<u8>::from_addr(0x8D_3698).byte_add(ty as isize * 0x2C);
    let mut inf = [0u32; 11];
    for (k, v) in inf.iter_mut().enumerate() {
        *v = rd::<u32>(info, 4 * k as isize);
    }
    let fi = |o: usize| f32::from_bits(inf[o / 4]);
    if followed.is_null() {
        let hi = x(a).fsub_m(fi(8));
        let hi32 = hi.to_f32();
        let b2 = x(a).fsub_m(b).fadd_m(2.0);
        let c = x(a).fsub_m(50.0).to_f32();
        let lo = if b2.fcom(x(c)).test_any(0x41) { c } else { b2.to_f32() };
        if !hi.fcom(x(lo)).test_parity(0x05) {
            return 0;
        }
        let t = rand_t(0x6E_421A);
        wr::<f32>(pos, 8, x(hi32).fsub_m(lo).fmul(t).fadd_m(lo).to_f32());
    }
    let pool = g::<GPtr<u8>>(0xB7_449C).read();
    wr::<u8>(pool, 0x11, 1);
    let mem = object_new(0x17C);
    let obj = if mem.is_null() {
        mem
    } else {
        object_ctor(mem, u32::from(rd::<u16>(GPtr::from_addr(inf[0] as usize), 0)), 0)
    };
    wr::<GPtr<u8>>(this, 8, obj);
    wr::<u8>(g::<GPtr<u8>>(0xB7_449C).read(), 0x11, 0);
    if rd::<GPtr<u8>>(this, 8).is_null() {
        return 0;
    }
    let obj = || rd::<GPtr<u8>>(this, 8);
    wr::<u8>(obj(), 0x2F, g::<u8>(0xB7_2914).read());
    v_set_is_static(obj(), 1);
    wr::<u32>(obj(), 0x1C, rd::<u32>(obj(), 0x1C) | 0x10_0000);
    wr::<u32>(obj(), 0x40, rd::<u32>(obj(), 0x40) & 0xFFFF_FFFD);
    wr::<u32>(obj(), 0x1C, rd::<u32>(obj(), 0x1C) & 0xFFFF_FFFE);
    wr::<u8>(obj(), 0x13C, 5);
    if ty == 3 || ty == 4 {
        wr::<u32>(obj(), 0x1C, rd::<u32>(obj(), 0x1C) | 0x4000);
    }
    wr::<u8>(this, 0xC, ty as u8);
    wr::<GPtr<u8>>(this, 0x1C, followed);
    wr::<u8>(this, 0xD, 0);
    let speed_t;
    let speed_hi;
    if !followed.is_null() {
        let dx = {
            let t = rand_t(0x6E_42FE);
            t.fadd(t).fsub_m(1.0).to_f32()
        };
        let dy = {
            let t = rand_t(0x6E_431D);
            t.fadd(t).fsub_m(1.0).to_f32()
        };
        let dz = rand_t(0x6E_433C).fsub_m(f32::from_bits(0x3F00_0000)).to_f32();
        let t = rand_t(0x6E_4359);
        let dist = x(fi(0x14)).fsub_m(fi(0x10)).fmul(t).fadd_m(fi(0x10));
        let ox = x(dx).fmul(dist);
        let oy = dist.fmul_m(dy).to_f32();
        let oz = dist.fmul_m(dz).to_f32();
        let mut at = [
            ox.fadd_m(rd::<f32>(pos, 0)).to_f32(),
            x(oy).fadd_m(rd::<f32>(pos, 4)).to_f32(),
            x(oz).fadd_m(rd::<f32>(pos, 8)).to_f32(),
        ];
        for k in 0..3isize {
            wr::<f32>(this, 0x20 + 4 * k, x(rd::<f32>(pos, 4 * k)).fsub_m(at[k as usize]).to_f32());
        }
        placeable_set_posn(obj(), p(&mut at));
        let fobj = rd::<GPtr<u8>>(followed, 8);
        let m = rd::<GPtr<u8>>(fobj, 0x14);
        let h = if m.is_null() {
            rd::<f32>(fobj, 0x10)
        } else {
            x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14))).to_f32()
        };
        wr::<f32>(this, 0x2C, h);
        speed_hi = rd::<f32>(followed, 0x14);
        speed_t = rand_t(0x6E_4417);
    } else {
        wr::<u32>(this, 0x28, 0);
        wr::<u32>(this, 0x24, 0);
        wr::<u32>(this, 0x20, 0);
        placeable_set_posn(obj(), pos);
        let h = rand_t(0x6E_4442)
            .fmul_m(f32::from_bits(0x40C9_0FDB))
            .fsub_m(f32::from_bits(0x4049_0FDB));
        wr::<f32>(this, 0x2C, h.to_f32());
        speed_t = rand_t(0x6E_4464);
        speed_hi = fi(0x1C);
    }
    wr::<u8>(this, 0xE, 0);
    wr::<u8>(this, 0x30, 1);
    let heading = rd::<u32>(this, 0x2C);
    wr::<f32>(this, 0x14, x(speed_hi).fsub_m(fi(0x18)).fmul(speed_t).fadd_m(fi(0x18)).to_f32());
    apply_heading(obj(), heading);
    let o = obj();
    let rw = rd::<GPtr<u8>>(o, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(o, 0x14);
        if !m.is_null() {
            matrix_update_rw(m, rwm);
        } else {
            simple_transform_update_rw_matrix(o.byte_add(4), rwm);
        }
    }
    entity_update_rw_frame(obj());
    world_add(obj());
    g::<u16>(0xBB_4A70).write(g::<u16>(0xBB_4A70).read().wrapping_add(1));
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_INIT];
