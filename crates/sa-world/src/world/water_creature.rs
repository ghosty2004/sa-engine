//! PORT-VEH-4 (batch `world4-2`, SA:MP-executed rows): `WaterCreature_c::Init` and `Update`. Ported from the 1.0 US
//! bytes; gta-reversed as a reading aid (the bytes win).
//!
//! `WaterCreature_c` (0x34): +8 object, +0xC type, +0xD delete flag, +0xE target swim speed, +0x14
//! default speed, +0x1C followed creature, +0x20 offset from the followed one, +0x2C heading, +0x30
//! changed-direction flag. The creature infos (0x2C each) are at 0x8D3698: +0 model id pointer (int16),
//! +8 minimum spawn depth, +0x10/+0x14 distance from the followed creature (min/max), +0x18/+0x1C speed
//! (min/max).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use super::common::{f32_of, g, rd, wr, x, xi};

orig! {
    /// `CObject::operator new(uint32)` (0x5A1EE0)
    fn object_new(size: u32) -> GPtr<u8> = cdecl 0x5A1EE0 => unported;
    /// `CObject::CObject(int32 modelId, bool create)` (0x5A1D70)
    fn object_ctor(o: GPtr<u8>, model: u32, create: u32) -> GPtr<u8> = thiscall 0x5A1D70 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn placeable_set_posn(e: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4241C0 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, angle: f32) = thiscall 0x59B020 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// virtual `CEntity::SetIsStatic(bool)` (vtable +0x10)
    fn v_set_is_static(e: GPtr<u8>, s: u32) = thiscall vslot 0x10 => unported;
}

/// `rand()` as the original call returning to `site` calls it, times 1/RAND_MAX (0x858C7C).
fn rand_unit(site: u32) -> X87 {
    xi(sa_core::orig::at_site(site, sa_core::crt::rand)).fmul_m(f32::from_bits(0x3800_0100))
}

/// `CObject::nNoTempObjects` (0xBB4A70, uint16).
fn temp_objects() -> GPtr<u16> {
    g::<u16>(0xBB_4A70)
}

/// `WaterCreature_c::Init(int32 type, CVector* pos, WaterCreature_c* parent, float waterLevel, float
/// waterDepth)` (0x6E4160).
///
/// False at 150 temporary objects. Without a parent the spawn height is random in `[minZ, maxZ]`, `maxZ =
/// level - info.minSpawnDepth` (stored as f32), `minZ = max(level - depth + 2, level - 50)` (the latter
/// as f32), and false when `maxZ < minZ`. A `CObject` of the info's model is created with the pool's
/// deal-with-no-memory flag set (false when that fails): area = current area, static, underwater flag,
/// no gravity, no collision, type 5 (decoration), draw-last for the jellyfish (types 3/4). With a parent:
/// a random direction `(u*2-1, u*2-1, u-0.5)` times a random distance in the info's range offsets the
/// position; the stored offset is `pos - (pos + d * dist)` (each sum stored as f32: the negated offset,
/// as the original computes it); heading from the parent object's matrix (`atan2(-fwd.x, fwd.y)`, or its
/// placement heading without a matrix); speed random between the info's minimum and the parent's
/// speed. Without: zero offset, heading `u * 2pi - pi`, speed in the info's range. Then target swim speed
/// 0, changed-direction set, heading applied (`SetRotateZOnly`, or the placement heading), RW matrix and
/// frame updated, added to the world, temporary objects +1. True.
#[hook(addr = 0x6E4160, conv = thiscall, name = "WaterCreature_c::Init", this = "WaterCreature_c",
       expect = "64 A1 00 00 00 00",
       unverified = "world4-2", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "creates a CObject (object pool) and adds it to the world; FPATAN of the parent's forward vector; rand at_site 0x6E421A, 0x6E42FE/0x6E431D/0x6E433C/0x6E4359/0x6E4417 (parent) or 0x6E4442/0x6E4464; stores maxZ/minZ into its own argument slots; the SEH frame is not reproduced")]
pub fn water_creature_init(this: GPtr<u8>, kind: i32, pos: GPtr<u8>, parent: GPtr<u8>, level: f32, depth: f32) -> u8 {
    if temp_objects().read() >= 0x96 {
        return 0;
    }
    let info = GPtr::<u8>::from_addr(0x8D_3698).byte_add(kind as isize * 0x2C);
    let info_f = |o: isize| rd::<f32>(info, o);
    if parent.is_null() {
        let max_z = x(level).fsub_m(info_f(8));
        let max_zf = f32_of(max_z);
        let t = x(level).fsub_m(depth).fadd_m(2.0);
        let lm50 = f32_of(x(level).fsub_m(50.0));
        let min_z = if t.fcom(x(lm50)).test_any(0x41) { lm50 } else { f32_of(t) };
        if max_z.fcom(x(min_z)) == Cmp::Less {
            return 0;
        }
        let u = rand_unit(0x6E_421A);
        wr::<f32>(pos, 8, f32_of(x(max_zf).fsub_m(min_z).fmul(u).fadd_m(min_z)));
    }
    let model_ptr = rd::<GPtr<u16>>(info, 0);
    wr::<u8>(g::<GPtr<u8>>(0xB7_449C).read(), 0x11, 1);
    let mem = object_new(0x17C);
    let obj = if mem.is_null() { mem } else { object_ctor(mem, u32::from(model_ptr.read()), 0) };
    wr::<GPtr<u8>>(this, 8, obj);
    wr::<u8>(g::<GPtr<u8>>(0xB7_449C).read(), 0x11, 0);
    let obj = rd::<GPtr<u8>>(this, 8);
    if obj.is_null() {
        return 0;
    }
    wr::<u8>(obj, 0x2F, g::<u8>(0xB7_2914).read());
    v_set_is_static(rd::<GPtr<u8>>(this, 8), 1);
    let o = || rd::<GPtr<u8>>(this, 8);
    wr::<u32>(o(), 0x1C, rd::<u32>(o(), 0x1C) | 0x10_0000);
    wr::<u32>(o(), 0x40, rd::<u32>(o(), 0x40) & 0xFFFF_FFFD);
    wr::<u32>(o(), 0x1C, rd::<u32>(o(), 0x1C) & 0xFFFF_FFFE);
    wr::<u8>(o(), 0x13C, 5);
    if kind == 3 || kind == 4 {
        wr::<u32>(o(), 0x1C, rd::<u32>(o(), 0x1C) | 0x4000);
    }
    wr::<u8>(this, 0xC, kind as u8);
    wr::<GPtr<u8>>(this, 0x1C, parent);
    wr::<u8>(this, 0xD, 0);
    let (speed_base, u_speed) = if !parent.is_null() {
        let dx = f32_of(rand_unit(0x6E_42FE).fdouble().fsub_m(1.0));
        let dy = f32_of(rand_unit(0x6E_431D).fdouble().fsub_m(1.0));
        let dz = f32_of(rand_unit(0x6E_433C).fsub_m(0.5));
        let u = rand_unit(0x6E_4359);
        let dist = x(info_f(0x14)).fsub_m(info_f(0x10)).fmul(u).fadd_m(info_f(0x10));
        let ox = x(dx).fmul(dist);
        let oy = f32_of(dist.fmul_m(dy));
        let oz = f32_of(dist.fmul_m(dz));
        let mut np = [
            f32_of(ox.fadd_m(rd::<f32>(pos, 0))).to_bits(),
            f32_of(x(oy).fadd_m(rd::<f32>(pos, 4))).to_bits(),
            f32_of(x(oz).fadd_m(rd::<f32>(pos, 8))).to_bits(),
        ];
        for (k, o) in [(0usize, 0isize), (1, 4), (2, 8)] {
            let d = f32_of(x(rd::<f32>(pos, o)).fsub_m(f32::from_bits(np[k])));
            wr::<f32>(this, 0x20 + o, d);
        }
        placeable_set_posn(rd::<GPtr<u8>>(this, 8), GPtr::from_raw(&mut np as *mut [u32; 3]).cast());
        let pobj = rd::<GPtr<u8>>(parent, 8);
        let m = rd::<GPtr<u8>>(pobj, 0x14);
        let heading = if m.is_null() {
            x(rd::<f32>(pobj, 0x10))
        } else {
            x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
        };
        wr::<f32>(this, 0x2C, f32_of(heading));
        let ps = rd::<f32>(parent, 0x14);
        let u = rand_unit(0x6E_4417);
        (ps, u)
    } else {
        wr::<u32>(this, 0x28, 0);
        wr::<u32>(this, 0x24, 0);
        wr::<u32>(this, 0x20, 0);
        placeable_set_posn(rd::<GPtr<u8>>(this, 8), pos);
        let h = rand_unit(0x6E_4442).fmul_m(f32::from_bits(0x40C9_0FDB)).fsub_m(f32::from_bits(0x4049_0FDB));
        wr::<f32>(this, 0x2C, f32_of(h));
        let u = rand_unit(0x6E_4464);
        (info_f(0x1C), u)
    };
    let min_speed = info_f(0x18);
    wr::<u8>(this, 0xE, 0);
    wr::<u8>(this, 0x30, 1);
    let heading = rd::<f32>(this, 0x2C);
    wr::<f32>(this, 0x14, f32_of(x(speed_base).fsub_m(min_speed).fmul(u_speed).fadd_m(min_speed)));
    let obj = rd::<GPtr<u8>>(this, 8);
    let m = rd::<GPtr<u8>>(obj, 0x14);
    if m.is_null() {
        wr::<f32>(obj, 0x10, heading);
    } else {
        matrix_set_rotate_z_only(m, heading);
    }
    let obj = rd::<GPtr<u8>>(this, 8);
    let rw = rd::<GPtr<u8>>(obj, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(obj, 0x14);
        if m.is_null() {
            simple_transform_update_rw_matrix(obj.byte_add(4), rwm);
        } else {
            matrix_update_rw_matrix(m, rwm);
        }
    }
    entity_update_rw_frame(rd::<GPtr<u8>>(this, 8));
    world_add(rd::<GPtr<u8>>(this, 8));
    temp_objects().write(temp_objects().read().wrapping_add(1));
    1
}

orig! {
    /// `CVector::NormaliseAndMag()` (0x59C970): ST0
    fn vector_normalise_and_mag(v: GPtr<u8>) -> f64 = thiscall 0x59C970 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float, float, float, float)` (0x53CBE0): ST0
    fn radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50): ST0
    fn limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CGeneral::GetRandomNumberInRange(float, float)` (0x41BD90): ST0
    fn random_in_range_f(lo: f32, hi: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_in_range_i(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CPlaceable::GetForward()` (0x41CCB0, the result through `out`)
    fn placeable_get_forward(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x41CCB0 => unported;
    /// `CWorld::ProcessLineOfSight(const CVector&, const CVector&, CColPoint&, CEntity*&, bool buildings,
    /// bool vehicles, bool peds, bool objects, bool dummies, bool seeThrough, bool camera, bool shootThrough)` (0x56BA00, AL)
    fn process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, ent: GPtr<u8>, b0: u32, b1: u32, b2: u32, b3: u32, b4: u32, b5: u32, b6: u32, b7: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(n: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* level, bool touching, CVector* normal)` (0x6EB690, AL)
    fn water_get_level(x: f32, y: f32, z: f32, level: GPtr<u8>, touching: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `WaterCreatureManager_c::TryToExitGroup(WaterCreature_c*)` (0x6E45B0) on `g_waterCreatureMan` (0xC1DF30)
    fn try_to_exit_group(m: GPtr<u8>, c: GPtr<u8>) = thiscall 0x6E45B0 => unported;
}

/// The position of an entity: its matrix's (+0x30) or its placement's (+4).
fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// The inlined `CPlaceable::SetHeading`: `SetRotateZOnly` on the matrix, or the placement heading.
fn set_heading(e: GPtr<u8>, h: f32) {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        wr::<f32>(e, 0x10, h);
    } else {
        matrix_set_rotate_z_only(m, h);
    }
}

/// `LimitRadianAngle(f32(GetRadianAngleBetweenPoints(x, y, 0, 0)))` as ST0.
fn heading_of(x0: f32, y0: f32) -> X87 {
    let a = f32_of(X87::from_f64_exact(radian_angle_between_points(x0, y0, 0.0, 0.0)));
    X87::from_f64_exact(limit_radian_angle(a))
}

/// A stack buffer of `N` dwords as a game pointer.
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// `WaterCreature_c::Update(float timeStep)` (0x6E4670).
///
/// A follower swims toward the followed creature's position plus its offset: turned toward it (when
/// the distance is positive), a 5% chance (`ftol(u * 100) < 5`, u from `rand() & 0xFFFF`) of a new speed
/// between the info's minimum and the leader's, the leader's current speed while it sprints, sped up by
/// `(d - 2) * 1.1` beyond 2 units, a step of `speed * timeStep` capped at `max(d - 0.01, 0)` along its
/// forward axis; beyond five times the info's maximum follow distance it leaves the group.
/// A leader: when it neither turned nor sprints, a random turn (`u < info+0x24`, heading in
/// [-pi, pi]); with a positive look-ahead (info+0x28) a line of sight to `pos + forward * lookAhead`
/// (when `ftol(u * 100) < 10`, it turned or it changed direction) sets the heading from the hit normal
/// and the changed-direction flag (else clears it); the heading moves toward the target (wrapped by
/// 2pi into [h - pi, h + pi]) by at most info+0x20; without a direction change it flees the player
/// within 5 units (a sprint of 35..50 for 2000..4500 ms, heading away, applied at once for the fish
/// types 0..2), else it may start a random sprint (8 in 1000: 5..15 for 1000..2500 ms); a sprint adds
/// `sprint * 0.1 * max(1 - cur/total, 0)` of the speed (stored as the current speed) and ends at its
/// total time; it moves by `forward * timeStep * speed`. Jellyfish (types 3/4) sit 0.2 below the water
/// surface. Then the RW matrix and frame are updated, the object re-added to the world, and beyond 60
/// units from the camera the creature tries to leave its group.
///
/// Kept as in the original: the forward vector of a matrix-less object is computed with FSIN/FCOS of the
/// placement heading, and the follower's offset is the one Init stored (see there).
#[hook(addr = 0x6E4670, conv = thiscall, name = "WaterCreature_c::Update", this = "WaterCreature_c",
       expect = "81 EC 94 00 00 00 55 56",
       unverified = "world4-2", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "FPATAN/FSIN/FCOS of headings; rand at_site 0x6E475B/0x6E478D (follower), 0x6E4943/0x6E499E (leader); re-adds the object to the world (sector lists)")]
pub fn water_creature_update(this: GPtr<u8>, ts: f32) {
    let info = GPtr::<u8>::from_addr(0x8D_3698).byte_add(rd::<u8>(this, 0xC) as isize * 0x2C);
    let info_f = |o: isize| rd::<f32>(info, o);
    let parent = rd::<GPtr<u8>>(this, 0x1C);
    if !parent.is_null() {
        let pp = pos_ptr(rd::<GPtr<u8>>(parent, 8));
        let tx = f32_of(x(rd::<f32>(this, 0x20)).fadd_m(rd::<f32>(pp, 0)));
        let ty = f32_of(x(rd::<f32>(this, 0x24)).fadd_m(rd::<f32>(pp, 4)));
        let tz = x(rd::<f32>(this, 0x28)).fadd_m(rd::<f32>(pp, 8));
        let mp = pos_ptr(rd::<GPtr<u8>>(this, 8));
        let mut dir = [
            f32_of(x(tx).fsub_m(rd::<f32>(mp, 0))).to_bits(),
            f32_of(x(ty).fsub_m(rd::<f32>(mp, 4))).to_bits(),
            f32_of(tz.fsub_m(rd::<f32>(mp, 8))).to_bits(),
        ];
        let dist_x = X87::from_f64_exact(vector_normalise_and_mag(buf(&mut dir)));
        let dist = f32_of(dist_x);
        if !dist_x.fcom(x(0.0)).test_any(0x41) {
            let h = f32_of(heading_of(f32::from_bits(dir[0]), f32::from_bits(dir[1])));
            set_heading(rd::<GPtr<u8>>(this, 8), h);
        }
        let r = sa_core::orig::at_site(0x6E_475B, sa_core::crt::rand) & 0xFFFF;
        let pct = super::common::ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(100.0));
        if pct < 5 {
            let ps = rd::<f32>(rd::<GPtr<u8>>(this, 0x1C), 0x14);
            let u = rand_unit(0x6E_478D);
            let lo = info_f(0x18);
            wr::<f32>(this, 0x14, f32_of(x(ps).fsub_m(lo).fmul(u).fadd_m(lo)));
        }
        let mut speed = x(rd::<f32>(this, 0x14));
        let par = rd::<GPtr<u8>>(this, 0x1C);
        if rd::<i8>(par, 0xE) > 0 {
            speed = x(rd::<f32>(par, 0x18));
        }
        if !x(dist).fcom(x(2.0)).test_any(0x41) {
            speed = speed.fmul(x(dist).fsub_m(2.0)).fmul_m(f32::from_bits(0x3F8C_CCCD));
        }
        let obj = rd::<GPtr<u8>>(this, 8);
        let mp = pos_ptr(obj);
        let my = [rd::<f32>(mp, 0), rd::<f32>(mp, 4), rd::<f32>(mp, 8)];
        let dm = f32_of(x(dist).fsub_m(f32::from_bits(0x3C23_D70A)));
        let cap = if x(dm).fcom(x(0.0)).test_any(0x41) { 0.0 } else { dm };
        let mut step = speed.fmul_m(ts);
        if step.fcom(x(cap)) != Cmp::Less {
            step = if x(dm).fcom(x(0.0)).test_any(0x41) { x(0.0) } else { x(dm) };
        }
        let m = rd::<GPtr<u8>>(obj, 0x14);
        let (fx, fy, fz) = if m.is_null() {
            let h = rd::<f32>(obj, 0x10);
            (x(h).fsin().fchs(), x(h).fcos(), 0.0f32)
        } else {
            (x(rd::<f32>(m, 0x10)), x(rd::<f32>(m, 0x14)), rd::<f32>(m, 0x18))
        };
        let sx = f32_of(fx.fmul(step));
        let sy = f32_of(fy.fmul(step));
        let sz = f32_of(x(fz).fmul(step));
        let nx = f32_of(x(sx).fadd_m(my[0]));
        let ny = f32_of(x(my[1]).fadd_m(sy));
        let nz = f32_of(x(my[2]).fadd_m(sz));
        let m = rd::<GPtr<u8>>(obj, 0x14);
        if m.is_null() {
            wr::<f32>(obj, 4, nx);
            wr::<f32>(obj, 8, ny);
            wr::<f32>(obj, 0xC, nz);
        } else {
            wr::<f32>(m, 0x30, nx);
            wr::<f32>(rd::<GPtr<u8>>(obj, 0x14), 0x34, ny);
            wr::<f32>(rd::<GPtr<u8>>(obj, 0x14), 0x38, nz);
        }
        if x(info_f(0x14)).fmul_m(5.0).fcom(x(dist)) == Cmp::Less {
            wr::<u32>(this, 0x1C, 0);
        }
    } else {
        leader_update(this, info, ts);
    }
    let kind = rd::<u8>(this, 0xC);
    if kind == 3 || kind == 4 {
        let obj = rd::<GPtr<u8>>(this, 8);
        let p = pos_ptr(obj);
        let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
        let mut level = [0u32; 1];
        if water_get_level(px, py, pz, buf(&mut level), 1, 0) != 0 {
            let obj = rd::<GPtr<u8>>(this, 8);
            let z = f32_of(x(f32::from_bits(level[0])).fsub_m(f32::from_bits(0x3E4C_CCCD)));
            let m = rd::<GPtr<u8>>(obj, 0x14);
            if m.is_null() {
                wr::<f32>(obj, 0xC, z);
                wr::<f32>(obj, 4, px);
                wr::<f32>(obj, 8, py);
            } else {
                wr::<f32>(m, 0x30, px);
                wr::<f32>(rd::<GPtr<u8>>(obj, 0x14), 0x34, py);
                wr::<f32>(rd::<GPtr<u8>>(obj, 0x14), 0x38, z);
            }
        }
    }
    let obj = rd::<GPtr<u8>>(this, 8);
    let rw = rd::<GPtr<u8>>(obj, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(obj, 0x14);
        if m.is_null() {
            simple_transform_update_rw_matrix(obj.byte_add(4), rwm);
        } else {
            matrix_update_rw_matrix(m, rwm);
        }
    }
    entity_update_rw_frame(rd::<GPtr<u8>>(this, 8));
    world_remove(rd::<GPtr<u8>>(this, 8));
    world_add(rd::<GPtr<u8>>(this, 8));
    let cm = g::<GPtr<u8>>(0xB6_F03C).read();
    let cp = if cm.is_null() { GPtr::<u8>::from_addr(0xB6_F02C) } else { cm.byte_add(0x30) };
    let op = pos_ptr(rd::<GPtr<u8>>(this, 8));
    let dx = x(rd::<f32>(op, 0)).fsub_m(rd::<f32>(cp, 0));
    let dy = x(rd::<f32>(op, 4)).fsub_m(rd::<f32>(cp, 4));
    let dz = x(rd::<f32>(op, 8)).fsub_m(rd::<f32>(cp, 8));
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if !d2.fcom(x(3600.0)).test_any(0x41) {
        try_to_exit_group(GPtr::from_addr(0xC1_DF30), this);
    }
}

/// The leader branch of `Update` (0x6E492D..0x6E4DD6).
fn leader_update(this: GPtr<u8>, info: GPtr<u8>, ts: f32) {
    let info_f = |o: isize| rd::<f32>(info, o);
    let pi = f32::from_bits(0x4049_0FDB);
    let two_pi = f32::from_bits(0x40C9_0FDB);
    let mut turned = false;
    if rd::<u8>(this, 0x30) == 0 && rd::<u8>(this, 0xE) == 0 {
        let u = rand_unit(0x6E_4943);
        if u.fcom(x(info_f(0x24))) == Cmp::Less {
            let h = X87::from_f64_exact(random_in_range_f(-pi, pi));
            wr::<f32>(this, 0x2C, f32_of(h));
            turned = true;
        }
    }
    let mp = pos_ptr(rd::<GPtr<u8>>(this, 8));
    let mut my = [rd::<u32>(mp, 0), rd::<u32>(mp, 4), rd::<u32>(mp, 8)];
    let r = sa_core::orig::at_site(0x6E_499E, sa_core::crt::rand) & 0xFFFF;
    let frac = xi(r).fmul_m(f32::from_bits(0x3800_0000));
    let look = info_f(0x28);
    if !x(look).fcom(x(0.0)).test_any(0x41) {
        let pct = super::common::ftol32(frac.fmul_m(100.0));
        if xi(pct).fcom(x(10.0)) == Cmp::Less || turned || rd::<u8>(this, 0x30) != 0 {
            let mut fwd = [0u32; 3];
            let f = placeable_get_forward(rd::<GPtr<u8>>(this, 8), buf(&mut fwd));
            let sx = f32_of(x(look).fmul_m(rd::<f32>(f, 0)));
            let sy = f32_of(x(look).fmul_m(rd::<f32>(f, 4)));
            let szx = x(look).fmul_m(rd::<f32>(f, 8));
            let p = pos_ptr(rd::<GPtr<u8>>(this, 8));
            let mut target = [
                f32_of(x(sx).fadd_m(rd::<f32>(p, 0))).to_bits(),
                f32_of(x(sy).fadd_m(rd::<f32>(p, 4))).to_bits(),
                f32_of(szx.fadd_m(rd::<f32>(p, 8))).to_bits(),
            ];
            let mut cp = [0u32; 11];
            let mut ent = [0u32; 1];
            let hit = process_line_of_sight(buf(&mut my), buf(&mut target), buf(&mut cp), buf(&mut ent), 1, 0, 0, 0, 0, 0, 0, 0);
            if hit != 0 {
                let h = heading_of(f32::from_bits(cp[4]), f32::from_bits(cp[5]));
                wr::<f32>(this, 0x2C, f32_of(h));
                wr::<u8>(this, 0x30, 1);
            } else {
                wr::<u8>(this, 0x30, 0);
            }
        }
    }
    let obj = rd::<GPtr<u8>>(this, 8);
    let m = rd::<GPtr<u8>>(obj, 0x14);
    let cur = if m.is_null() {
        rd::<f32>(obj, 0x10)
    } else {
        f32_of(x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14))))
    };
    let tgt = rd::<f32>(this, 0x2C);
    if x(cur).fadd_m(pi).fcom(x(tgt)) == Cmp::Less {
        wr::<f32>(this, 0x2C, f32_of(x(tgt).fsub_m(two_pi)));
    } else if !x(cur).fsub_m(pi).fcom(x(tgt)).test_any(0x41) {
        wr::<f32>(this, 0x2C, f32_of(x(tgt).fadd_m(two_pi)));
    }
    let diff = x(rd::<f32>(this, 0x2C)).fsub_m(cur);
    let ad = if diff.fcom(x(0.0)) == Cmp::Less { diff.fchs() } else { diff };
    let maxc = info_f(0x20);
    let mut h = cur;
    if !ad.fcom(x(maxc)).test_any(0x41) {
        h = if diff.fcom(x(0.0)).c0() { f32_of(x(cur).fsub_m(maxc)) } else { f32_of(x(maxc).fadd_m(cur)) };
    }
    set_heading(obj, h);
    if rd::<u8>(this, 0x30) == 0 {
        let pp = pos_ptr(find_player_ped(0));
        let my_f = |k: usize| f32::from_bits(my[k]);
        let mut away = [
            f32_of(x(my_f(0)).fsub_m(rd::<f32>(pp, 0))).to_bits(),
            f32_of(x(my_f(1)).fsub_m(rd::<f32>(pp, 4))).to_bits(),
            f32_of(x(my_f(2)).fsub_m(rd::<f32>(pp, 8))).to_bits(),
        ];
        let mag = X87::from_f64_exact(vector_normalise_and_mag(buf(&mut away)));
        if mag.fcom(x(5.0)) == Cmp::Less {
            let s = random_in_range_i(0x23, 0x32);
            wr::<u8>(this, 0xE, s as u8);
            wr::<u16>(this, 0x10, 0);
            let t = random_in_range_i(2000, 4500);
            wr::<u16>(this, 0x12, t as u16);
            let h = heading_of(f32::from_bits(away[0]), f32::from_bits(away[1]));
            wr::<f32>(this, 0x2C, f32_of(h));
            if rd::<u8>(this, 0xC) <= 2 {
                set_heading(rd::<GPtr<u8>>(this, 8), f32_of(h));
            }
        }
    } else if rd::<u8>(this, 0xE) == 0 && random_in_range_i(0, 1000) < 8 {
        let s = random_in_range_i(5, 15);
        wr::<u8>(this, 0xE, s as u8);
        wr::<u16>(this, 0x10, 0);
        let t = random_in_range_i(1000, 2500);
        wr::<u16>(this, 0x12, t as u16);
    }
    let sprint = rd::<i8>(this, 0xE);
    let mut speed = x(rd::<f32>(this, 0x14));
    if sprint > 0 {
        let inc = super::common::ftol32(x(ts).fmul_m(1000.0)) as u16;
        wr::<u16>(this, 0x10, rd::<u16>(this, 0x10).wrapping_add(inc));
        let (c, t) = (rd::<u16>(this, 0x10), rd::<u16>(this, 0x12));
        let mut f = xi(i32::from(c)).fdiv(xi(i32::from(t))).fsubr_m(1.0);
        if f.fcom(x(0.0)).test_any(0x41) {
            f = x(0.0);
        }
        let add = xi(i32::from(sprint)).fmul_m(f32::from_bits(0x3DCC_CCCD)).fmul(f).fmul(speed);
        speed = speed.fadd(add);
        if c >= t {
            wr::<u8>(this, 0xE, 0);
        }
        wr::<f32>(this, 0x18, f32_of(speed));
    }
    let obj = rd::<GPtr<u8>>(this, 8);
    let m = rd::<GPtr<u8>>(obj, 0x14);
    let (fx, fy, fz) = if m.is_null() {
        let h = rd::<f32>(obj, 0x10);
        (x(h).fsin().fchs(), f32_of(x(h).fcos()), 0.0f32)
    } else {
        (x(rd::<f32>(m, 0x10)), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18))
    };
    let fxt = fx.fmul_m(ts);
    let fyt = x(fy).fmul_m(ts);
    let fzt = f32_of(x(fz).fmul_m(ts));
    let dx = f32_of(fxt.fmul(speed));
    let dz = f32_of(x(fzt).fmul(speed));
    let my_f = |k: usize| f32::from_bits(my[k]);
    let nx = f32_of(x(dx).fadd_m(my_f(0)));
    let ny = f32_of(fyt.fmul(speed).fadd_m(my_f(1)));
    let nz = f32_of(x(dz).fadd_m(my_f(2)));
    let m = rd::<GPtr<u8>>(obj, 0x14);
    if m.is_null() {
        wr::<f32>(obj, 8, ny);
        wr::<f32>(obj, 4, nx);
        wr::<f32>(obj, 0xC, nz);
    } else {
        wr::<f32>(m, 0x30, nx);
        wr::<f32>(rd::<GPtr<u8>>(obj, 0x14), 0x34, ny);
        wr::<f32>(rd::<GPtr<u8>>(obj, 0x14), 0x38, nz);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_WATER_CREATURE_INIT, HOOK_WATER_CREATURE_UPDATE];
