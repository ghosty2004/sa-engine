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

use sa_math::x87::X87;

use super::common::{ftol32, g, rd, wr, x, xi};

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
    /// `CVector::NormaliseAndMag()` (0x59C970): the magnitude in ST0
    fn normalise_and_mag(v: GPtr<u8>) -> f64 = thiscall 0x59C970 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0), ST0
    fn radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50), ST0
    fn limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(float, float)` (0x41BD90), ST0
    fn random_float_in_range(lo: u32, hi: u32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CGeneral::GetRandomNumberInRange_int32(int32, int32)` (0x407180)
    fn random_int_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CPlaceable::GetTopDirection(CVector* out)` (0x41CCB0): returns `out`
    fn placeable_get_top_direction(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x41CCB0 => unported;
    /// `CWorld::ProcessLineOfSight(start, end, CColPoint&, CEntity*&, bool x8)` (0x56BA00)
    fn process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, ent: GPtr<u8>, bl: u32, v: u32, pd: u32, o: u32, d: u32, s: u32, c: u32, sh: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWaterLevel::GetWaterLevel(float, float, float, float*, bool, CVector*)` (0x6EB690)
    fn water_level(x: u32, y: u32, z: u32, out: GPtr<u8>, waves: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `WaterCreatureManager_c::TryToExitGroup(WaterCreature_c*)` (0x6E45B0) on the manager (0xC1DF30)
    fn try_to_exit_group(m: GPtr<u8>, c: GPtr<u8>) = thiscall 0x6E45B0 => unported;
}

/// `1 / 32767` (0x858C7C).
const RAND_K: f32 = f32::from_bits(0x3800_0100);

/// `(float)rand() * (1/32767)` with the original return address `site`.
fn rand_t(site: u32) -> X87 {
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

/// The object position: its matrix's +0x30, or the placement +4.
fn obj_pos(obj: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(obj, 0x14);
    if m.is_null() { obj.byte_add(4) } else { m.byte_add(0x30) }
}

/// Writes a position as the original does: matrix +0x30/+0x34/+0x38, or placement +4/+8/+0xC.
fn set_obj_pos(obj: GPtr<u8>, v: [u32; 3]) {
    let m = rd::<GPtr<u8>>(obj, 0x14);
    let at = if m.is_null() { obj.byte_add(4) } else { m.byte_add(0x30) };
    for (k, w) in v.iter().enumerate() {
        wr::<u32>(at, 4 * k as isize, *w);
    }
}

/// The f32 an `fstp dword` makes of an orig ST0 result.
fn st0(v: f64) -> f32 {
    X87::from_f64_exact(v).to_f32()
}

/// The object's forward (up) vector times `scale` as the two branches load it: with a matrix its up
/// vector, without one `(-sin h, cos h, 0)` of the placement heading (FSIN/FCOS).
fn forward(obj: GPtr<u8>) -> (X87, X87, f32) {
    let m = rd::<GPtr<u8>>(obj, 0x14);
    if m.is_null() {
        let h = x(rd::<f32>(obj, 0x10));
        (h.fsin().fchs(), h.fcos(), 0.0)
    } else {
        (x(rd::<f32>(m, 0x10)), x(rd::<f32>(m, 0x14)), rd::<f32>(m, 0x18))
    }
}

/// `WaterCreature_c::Update(float ts)` (0x6E4670). A follower steers towards its followed creature's
/// position plus its offset: heading = `LimitRadianAngle(GetRadianAngleBetweenPoints(d.x, d.y, 0,
/// 0))` of the normalised direction when the distance is above 0; with `ftol(r * 100) < 5` a new
/// speed between the minimum and the followed one's; the followed creature's current speed while it
/// is changing speed; beyond 2 m `speed * (dist - 2) * 1.1`; the step `speed * ts` capped at `dist -
/// 0.01` (not below 0) along the forward vector; it stops following beyond `maxDist * 5`. A leader:
/// when idle (+0x30 and +0xE clear) and `t < chance`, a random heading (0x41BD90 in [-pi, pi]); one
/// `rand()` per update for the move test (drawn even with zero speed); with speed > 0 and
/// `ftol(r * 100) < 10`, a new heading or an active flag, a line-of-sight probe `speed` ahead
/// (buildings only) turns it away along the hit normal (active) or clears active. Its heading turns
/// towards the target by at most the turn rate (wrapped by 2pi; unchanged within the rate). When not
/// active and the player is closer than 5 m it swims away (speed boost 35..50 frames over
/// 2000..4500 ms, heading away; applied at once for types 0..2); when active and not boosting,
/// a 8-in-1000 chance of a 5..15 frame boost over 1000..2500 ms. While boosting the timer runs
/// (`ftol(ts * 1000)`), speed = `speed + frames * 0.1 * max(0, 1 - timer / duration) * speed`
/// (stored as the current speed, the boost ends at the duration); it moves `forward * ts * speed`.
/// Turtles / dolphins (types 3, 4) snap to the water level - 0.2. Then the RW matrix and frame are
/// refreshed, the object re-added to the world, and beyond 60 m from the camera the creature leaves
/// its group (`TryToExitGroup`).
#[hook(addr = 0x6E4670, conv = thiscall, name = "WaterCreature_c::Update", this = "WaterCreature_c",
       expect = "81 EC 94 00 00 00",
       unverified = "world5-2", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "rand at_site 0x6E475B/0x6E478D (follower), 0x6E4943, 0x6E499E (leader); FSIN/FCOS without a matrix, FPATAN of the heading; CWorld::Remove/Add of the object; TryToExitGroup")]
pub fn update(this: GPtr<u8>, ts: f32) {
    let info = GPtr::<u8>::from_addr(0x8D_3698).byte_add(isize::from(rd::<u8>(this, 0xC)) * 0x2C);
    let mut inf = [0u32; 11];
    for (k, v) in inf.iter_mut().enumerate() {
        *v = rd::<u32>(info, 4 * k as isize);
    }
    let fi = |o: usize| f32::from_bits(inf[o / 4]);
    let followed = rd::<GPtr<u8>>(this, 0x1C);
    let obj = || rd::<GPtr<u8>>(this, 8);
    if !followed.is_null() {
        let fpos = obj_pos(rd::<GPtr<u8>>(followed, 8));
        let tx = x(rd::<f32>(this, 0x20)).fadd_m(rd::<f32>(fpos, 0)).to_f32();
        let ty = x(rd::<f32>(this, 0x24)).fadd_m(rd::<f32>(fpos, 4)).to_f32();
        let tz = x(rd::<f32>(this, 0x28)).fadd_m(rd::<f32>(fpos, 8));
        let me = obj_pos(obj());
        let mut d = [
            x(tx).fsub_m(rd::<f32>(me, 0)).to_f32(),
            x(ty).fsub_m(rd::<f32>(me, 4)).to_f32(),
            tz.fsub_m(rd::<f32>(me, 8)).to_f32(),
        ];
        let mag_x = X87::from_f64_exact(normalise_and_mag(p(&mut d)));
        let mag = mag_x.to_f32();
        if !mag_x.fcom(x(0.0)).test_any(0x41) {
            let a = st0(radian_angle_between_points(d[0].to_bits(), d[1].to_bits(), 0, 0));
            let h = st0(limit_radian_angle(a.to_bits()));
            apply_heading(obj(), h.to_bits());
        }
        let r = at_site(0x6E_475B, rand) & 0xFFFF;
        if ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(100.0)) < 5 {
            let fs = rd::<f32>(followed, 0x14);
            let t = xi(at_site(0x6E_478D, rand)).fmul_m(RAND_K);
            wr::<f32>(this, 0x14, x(fs).fsub_m(fi(0x18)).fmul(t).fadd_m(fi(0x18)).to_f32());
        }
        let mut s = x(rd::<f32>(this, 0x14));
        if rd::<i8>(followed, 0xE) > 0 {
            s = x(rd::<f32>(followed, 0x18));
        }
        if !x(mag).fcom(x(2.0)).test_any(0x41) {
            s = x(mag).fsub_m(2.0).fmul(s).fmul_m(f32::from_bits(0x3F8C_CCCD));
        }
        let me = obj_pos(obj());
        let mut pp = [rd::<f32>(me, 0), rd::<f32>(me, 4), rd::<f32>(me, 8)];
        let m2 = x(mag).fsub_m(f32::from_bits(0x3C23_D70A));
        let m2f = m2.to_f32();
        let cap = if !m2.fcom(x(0.0)).test_any(0x41) { m2f } else { 0.0 };
        let mut v = s.fmul_m(ts);
        if v.fcom(x(cap)).test_parity(0x05) {
            v = if !x(m2f).fcom(x(0.0)).test_any(0x41) { x(m2f) } else { x(0.0) };
        }
        let (fx, fy, fz) = forward(obj());
        let mx = fx.fmul(v).to_f32();
        let my = fy.fmul(v).to_f32();
        let mz = x(fz).fmul(v).to_f32();
        let nx = x(mx).fadd_m(pp[0]).to_f32();
        pp[1] = x(pp[1]).fadd_m(my).to_f32();
        pp[2] = x(pp[2]).fadd_m(mz).to_f32();
        set_obj_pos(obj(), [nx.to_bits(), pp[1].to_bits(), pp[2].to_bits()]);
        if !x(fi(0x14)).fmul_m(5.0).fcom(x(mag)).test_parity(0x05) {
            wr::<u32>(this, 0x1C, 0);
        }
    } else {
        let mut turned = false;
        if rd::<u8>(this, 0x30) == 0 && rd::<u8>(this, 0xE) == 0 {
            let t = xi(at_site(0x6E_4943, rand)).fmul_m(RAND_K);
            if !t.fcom(x(fi(0x24))).test_parity(0x05) {
                let h = st0(random_float_in_range(0xC049_0FDB, 0x4049_0FDB));
                wr::<f32>(this, 0x2C, h);
                turned = true;
            }
        }
        let me = obj_pos(obj());
        let mut pp = [rd::<f32>(me, 0), rd::<f32>(me, 4), rd::<f32>(me, 8)];
        let r = xi(at_site(0x6E_499E, rand) & 0xFFFF).fmul_m(f32::from_bits(0x3800_0000));
        if !x(fi(0x28)).fcom(x(0.0)).test_any(0x41) {
            let n = ftol32(r.fmul_m(100.0));
            let probe = !xi(n).fcom(x(10.0)).test_parity(0x05) || turned || rd::<u8>(this, 0x30) != 0;
            if probe {
                let mut fwd = [0u32; 3];
                let f = placeable_get_top_direction(obj(), p(&mut fwd));
                let sp = fi(0x28);
                let ex = x(sp).fmul_m(rd::<f32>(f, 0)).to_f32();
                let ey = x(sp).fmul_m(rd::<f32>(f, 4)).to_f32();
                let ez = x(sp).fmul_m(rd::<f32>(f, 8));
                let me = obj_pos(obj());
                let mut end = [
                    x(ex).fadd_m(rd::<f32>(me, 0)).to_f32(),
                    x(ey).fadd_m(rd::<f32>(me, 4)).to_f32(),
                    ez.fadd_m(rd::<f32>(me, 8)).to_f32(),
                ];
                let mut cp = [0u32; 11];
                let mut ent = 0u32;
                if process_line_of_sight(p(&mut pp), p(&mut end), p(&mut cp), p(&mut ent), 1, 0, 0, 0, 0, 0, 0, 0) != 0 {
                    let a = st0(radian_angle_between_points(cp[4], cp[5], 0, 0));
                    wr::<f32>(this, 0x2C, st0(limit_radian_angle(a.to_bits())));
                    wr::<u8>(this, 0x30, 1);
                } else {
                    wr::<u8>(this, 0x30, 0);
                }
            }
        }
        let o = obj();
        let m = rd::<GPtr<u8>>(o, 0x14);
        let mut cur = if m.is_null() {
            rd::<f32>(o, 0x10)
        } else {
            x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14))).to_f32()
        };
        let pi = f32::from_bits(0x4049_0FDB);
        let two_pi = f32::from_bits(0x40C9_0FDB);
        let target = rd::<f32>(this, 0x2C);
        if !x(cur).fadd_m(pi).fcom(x(target)).test_parity(0x05) {
            wr::<f32>(this, 0x2C, x(target).fsub_m(two_pi).to_f32());
        } else if !x(cur).fsub_m(pi).fcom(x(target)).test_any(0x41) {
            wr::<f32>(this, 0x2C, x(target).fadd_m(two_pi).to_f32());
        }
        let diff = x(rd::<f32>(this, 0x2C)).fsub_m(cur);
        let ad = if !diff.fcom(x(0.0)).test_parity(0x05) { diff.fchs() } else { diff };
        let turn = fi(0x20);
        if !ad.fcom(x(turn)).test_any(0x41) {
            cur = if diff.fcom(x(0.0)).c0() { x(cur).fsub_m(turn).to_f32() } else { x(turn).fadd_m(cur).to_f32() };
        }
        apply_heading(o, cur.to_bits());
        if rd::<u8>(this, 0x30) == 0 {
            let pl = obj_pos(find_player_ped(0));
            let mut d = [
                x(pp[0]).fsub_m(rd::<f32>(pl, 0)).to_f32(),
                x(pp[1]).fsub_m(rd::<f32>(pl, 4)).to_f32(),
                x(pp[2]).fsub_m(rd::<f32>(pl, 8)).to_f32(),
            ];
            let mag = X87::from_f64_exact(normalise_and_mag(p(&mut d)));
            if !mag.fcom(x(5.0)).test_parity(0x05) {
                wr::<u8>(this, 0xE, random_int_in_range(0x23, 0x32) as u8);
                wr::<u16>(this, 0x10, 0);
                wr::<u16>(this, 0x12, random_int_in_range(2000, 4500) as u16);
                let a = st0(radian_angle_between_points(d[0].to_bits(), d[1].to_bits(), 0, 0));
                let h = st0(limit_radian_angle(a.to_bits()));
                wr::<f32>(this, 0x2C, h);
                if rd::<u8>(this, 0xC) <= 2 {
                    apply_heading(obj(), h.to_bits());
                }
            }
        } else if rd::<u8>(this, 0xE) == 0 && random_int_in_range(0, 1000) < 8 {
            wr::<u8>(this, 0xE, random_int_in_range(5, 15) as u8);
            wr::<u16>(this, 0x10, 0);
            wr::<u16>(this, 0x12, random_int_in_range(1000, 2500) as u16);
        }
        let frames = rd::<i8>(this, 0xE);
        let mut speed = x(rd::<f32>(this, 0x14));
        if frames > 0 {
            let step = ftol32(x(ts).fmul_m(1000.0));
            wr::<u16>(this, 0x10, rd::<u16>(this, 0x10).wrapping_add(step as u16));
            let (cx, dx) = (rd::<u16>(this, 0x10), rd::<u16>(this, 0x12));
            let mut f = xi(i32::from(cx)).fdiv(xi(i32::from(dx))).fsubr_m(1.0);
            if f.fcom(x(0.0)).test_any(0x41) {
                f = x(0.0);
            }
            let b = xi(i32::from(frames)).fmul_m(f32::from_bits(0x3DCC_CCCD)).fmul(f).fmul(speed);
            speed = b.fadd(speed);
            if cx >= dx {
                wr::<u8>(this, 0xE, 0);
            }
            wr::<f32>(this, 0x18, speed.to_f32());
        }
        let (fx, fy, fz) = forward(obj());
        // the original stores the forward y (FCOS without a matrix) to its frame before the multiply
        let fxt = fx.fmul_m(ts);
        let fyt = x(fy.to_f32()).fmul_m(ts);
        let fzt = x(fz).fmul_m(ts).to_f32();
        let mx = fxt.fmul(speed).to_f32();
        let my = fyt.fmul(speed);
        let mz = x(fzt).fmul(speed).to_f32();
        let nx = x(mx).fadd_m(pp[0]).to_f32();
        let ny = my.fadd_m(pp[1]).to_f32();
        let nz = x(mz).fadd_m(pp[2]).to_f32();
        set_obj_pos(obj(), [nx.to_bits(), ny.to_bits(), nz.to_bits()]);
    }
    let ty = rd::<u8>(this, 0xC);
    if ty == 3 || ty == 4 {
        let at = obj_pos(obj());
        let (px, py, pz) = (rd::<u32>(at, 0), rd::<u32>(at, 4), rd::<u32>(at, 8));
        let mut level = 0f32;
        if water_level(px, py, pz, p(&mut level), 1, 0) != 0 {
            let z = x(level).fsub_m(f32::from_bits(0x3E4C_CCCD)).to_f32();
            set_obj_pos(obj(), [px, py, z.to_bits()]);
        }
    }
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
    world_remove(obj());
    world_add(obj());
    let cam = g::<GPtr<u8>>(0xB6_F03C).read();
    let c = if cam.is_null() { GPtr::<u8>::from_addr(0xB6_F02C) } else { cam.byte_add(0x30) };
    let at = obj_pos(obj());
    let dx = x(rd::<f32>(at, 0)).fsub_m(rd::<f32>(c, 0));
    let dy = x(rd::<f32>(at, 4)).fsub_m(rd::<f32>(c, 4));
    let dz = x(rd::<f32>(at, 8)).fsub_m(rd::<f32>(c, 8));
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if !d2.fcom(x(3600.0)).test_any(0x41) {
        try_to_exit_group(GPtr::from_addr(0xC1_DF30), this);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_INIT, HOOK_UPDATE];
