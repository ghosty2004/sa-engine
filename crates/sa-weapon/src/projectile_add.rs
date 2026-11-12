//! `CProjectileInfo::AddProjectile` (0x737C80, batch `samp2-weapon-1`): thrown weapons, rockets, bombs and
//! flares. Unreversed in gta-reversed; ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;
use crate::projectile::cross_product;
use crate::weapon::ped_is_player;
use crate::weapon_fire::{find_player_ped, matrix_dtor, vector_magnitude};

orig! {
    /// `CMatrix::SetTranslate(float, float, float)` (0x59AF40, thiscall)
    fn matrix_set_translate(m: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x59AF40 => unported;
    /// `CMatrix::RotateZ(float)` (0x59B390, thiscall)
    fn matrix_rotate_z(m: GPtr<u8>, a: f32) = thiscall 0x59B390 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, thiscall)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790)
    fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990, thiscall)
    fn placeable_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `operator new(uint32)` (0x5A1EE0, the pool-aware one of CObject / CProjectile)
    fn object_new(size: u32) -> GPtr<u8> = cdecl 0x5A1EE0 => unported;
    /// `CProjectile::CProjectile(int32 model)` (0x5A4030, thiscall)
    fn projectile_ctor(p: GPtr<u8>, model: i32) -> GPtr<u8> = thiscall 0x5A4030 => unported;
    /// `CColModel::AllocateData(int32 spheres, int32 boxes, int32 lines, int32 vertices, int32 triangles,
    /// bool disk)` (0x40F870, thiscall)
    #[allow(clippy::too_many_arguments)]
    fn col_model_allocate_data(cm: GPtr<u8>, s: i32, b: i32, l: i32, v: i32, t: i32, d: u32) = thiscall 0x40F870 => unported;
    /// `CColSphere::Set(float radius, const CVector& center, uint8 material, uint8 flags, uint8 lighting)`
    /// (0x40FD10, thiscall)
    fn col_sphere_set(s: GPtr<u8>, radius: f32, center: GPtr<u8>, material: u32, flags: u32, light: u32) = thiscall 0x40FD10 => unported;
    /// `CMemoryMgr::Malloc(uint32, uint32)` (0x72F420)
    fn memory_mgr_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, thiscall)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPlaceable::SetMatrix(const CMatrix&)` (0x54F610, thiscall)
    fn placeable_set_matrix(e: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F610 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CEntity::GetModellingMatrix()` (0x46A2D0, thiscall)
    fn entity_get_modelling_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x46A2D0 => unported;
    /// `FxManager_c::CreateFxSystem(const char*, CVector*, RwMatrix*, bool)` (0x4A9BE0, thiscall)
    fn fx_create_system(fx: GPtr<u8>, name: u32, point: GPtr<u8>, parent: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `CRadar::SetEntityBlip(eBlipType, int32 handle, uint32 colour, eBlipDisplay, const char*)` (0x5839A0)
    fn radar_set_entity_blip(ty: u32, handle: i32, colour: u32, display: u32, name: u32) -> i32 = cdecl 0x5839A0 => unported;
    /// `CRadar::ChangeBlipScale(int32, int32)` (0x583CC0)
    fn radar_change_blip_scale(blip: i32, scale: i32) = cdecl 0x583CC0 => unported;
    /// `CRadar::ChangeBlipColour(int32, uint32)` (0x583AB0)
    fn radar_change_blip_colour(blip: i32, colour: u32) = cdecl 0x583AB0 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, b: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CAudioEngine::ReportWeaponEvent(int32 event, eWeaponType, CPhysical*)` (0x506F40, thiscall)
    fn audio_report_weapon_event(ae: GPtr<u8>, ev: i32, ty: u32, e: GPtr<u8>) = thiscall 0x506F40 => unported;
}

/// `CProjectileInfo::gaProjectileInfo` (32 x 0x24 at 0xC891A8: type +0, creator +4, target +8, destroy
/// time +0xC, active +0x10, last position +0x14, fx +0x20) and `ms_apProjectile` (0xC89110).
const INFOS: usize = 0xC8_91A8;
const OBJECTS: usize = 0xC8_9110;
/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8), `ModelIndices::MI_FLARE` (0x8CD75C, u16).
const MODEL_INFOS: usize = 0xA9_B0C8;
const MI_FLARE: usize = 0x8C_D75C;

fn now() -> u32 {
    sa_core::timer::TIME_MS.get()
}

/// `force == 0 ? 0 : force * 0.22 + 0.15`.
fn throw_speed(force: f32) -> f32 {
    if force_is_zero(force) { 0.0 } else { x(force).fmul_m(cf(0x3E61_47AE)).fadd_m(cf(0x3E19_999A)).to_f32() }
}

fn force_is_zero(f: f32) -> bool {
    !x(f).fcom(x(0.0)).test_parity(0x44)
}

/// The creator's heading: `atan2(-fwd.x, fwd.y)` (x87 `fpatan`) with a matrix, else the placement heading.
fn heading(c: GPtr<u8>) -> f32 {
    let m = rp(c, 0x14);
    if m.is_null() { rf(c, 0x10) } else { x(rf(m, 0x10)).fchs().fpatan(x(rf(m, 0x14))).to_f32() }
}

/// `SetTranslate(0, 0, 0)`, `RotateZ(h)`, position += (x, y, z).
fn place(mat: GPtr<u8>, h: f32, p: [f32; 3]) {
    matrix_set_translate(mat, 0.0, 0.0, 0.0);
    matrix_rotate_z(mat, h);
    for (i, v) in p.iter().enumerate() {
        let off = 0x30 + 4 * i as isize;
        wf(mat, off, x(rf(mat, off)).fadd_m(*v).to_f32());
    }
}

/// The thrown velocity `(-sin(h) * sp, cos(h) * sp, z)` (the two products of the 80-bit sine / cosine).
fn throw_vel(h: f32, sp: f32) -> (f32, f32) {
    let s = x(h).fsin();
    let c = x(h).fcos().fmul_m(sp);
    (s.fmul_m(sp).fmul_m(-1.0).to_f32(), c.to_f32())
}

/// `CProjectileInfo::AddProjectile(CEntity* creator, eWeaponType type, CVector origin, float force, CVector*
/// dir, CEntity* target)` (0x737C80, cdecl; entry `push -1; jmp` into the protection chunk 0x401C3D, which
/// pushes the SEH handler).
///
/// By type (a local `CMatrix` M, velocity V, speed, destroy time, elasticity 0.75, the collision bit 1):
/// * 17 tear gas: time + 20000; speed = `throw_speed(force)`; heading h; M = RotateZ(h) at origin; V =
///   `(-sin h * sp, cos h * sp, (force + 1) * 0.4 * sp)`; surface marker 5, elasticity 0.5.
/// * 16 grenade / 39 satchel: time + 2000; speed halved for 39; a vehicle creator turns h by pi
///   (`LimitRadianAngle`); as 17; creator model 405 adds its move speed; marker 5, elasticity 0.5 (0.03 for
///   39).
/// * 18 molotov: time + 2000; `speed = max(force * 0.22 + 0.15, 0.2)`; V.z = `(force * 0.2 + 0.4) * sp`.
/// * 19 / 20 rockets: time + 3000 / 10000, speed 0.4 / 0.2. Vehicle creator: M = its matrix at origin, speed
///   += |move speed|. Player: M.right = `cam.up x cam.front`, M.fwd = front, M.up = up (active camera), at
///   origin. Others: with `dir` M.fwd = dir, M.right = the creator's right, M.up = right x dir, at origin;
///   without, M = the creator's matrix (its own position). V = `Multiply3x3(M, (0, speed, 0))`, no collision
///   bit.
/// * 21 freefall bomb / 58 flare (model requested): time + 2000000 / 10000; V = the creator's move speed
///   for physicals else 0; M = the creator's matrix at origin.
/// * other types: nothing is set up (the original then reads stale locals and the slot pointer; the port
///   uses zeroed locals).
///
/// A free slot among 32 (none: false). Object: `new CProjectile(GetWeaponInfo(type, 1)->modelId)` (flare:
/// MI_FLARE, buoyancy 0.9); for 16/17/18/39 the model's col model gets one sphere of 0.75 x the bounding
/// radius (allocating the data, or the sphere array when there is none). No object: false. Then the info
/// (type, creator + reference, destroy time, target + reference, active), `SetMatrix(M)`, V, the collision
/// bit, elasticity (+0x9C), marker 5 -> +0x160 = 0xBB4BD0, `CWorld::Add`, the slot reference, the last
/// position; tear gas plays an "teargas" FX at the modelling matrix; `+0x128 = creator`, flags
/// `|= 0x10000000`, a physical creator without one gets itself at +0x128; type 20 gets a red object blip
/// (scale 1, colour -1 when fired by the player or the player's vehicle); `ReportWeaponEvent(0x94, type,
/// object)`; true.
#[hook(addr = 0x737C80, conv = cdecl, name = "CProjectileInfo::AddProjectile", expect = "6A FF E9 B6 9F CC FF",
       unverified = "samp2-weapon-1", route = "L2", hazards = "x87-transcendental,heap-writes,uninit-locals,entry-protection-read",
       recs = "weapon-combat",
       note = "entry jmps to the protection chunk 0x401C3D (SEH handler push); fpatan/fsin/fcos; creates pool objects")]
#[allow(clippy::too_many_arguments)]
pub fn add_projectile(creator: GPtr<u8>, ty: u32, ox: f32, oy: f32, oz: f32, force: f32, dir: GPtr<u8>,
    target: GPtr<u8>) -> u8 {
    let mut mat = [0u32; 0x48 / 4];
    let mp: GPtr<u8> = lp(&mut mat).cast();
    let origin = [ox, oy, oz];
    let mut vel = [0f32; 3];
    let mut time = 0u32;
    let mut elasticity = cf(0x3F40_0000);
    let mut collide = 1u8;
    let mut marker = 0u8;
    match ty {
        0x11 => {
            time = now().wrapping_add(20_000);
            let sp = throw_speed(force);
            let h = heading(creator);
            place(mp, h, origin);
            marker = 5;
            elasticity = 0.5;
            let (vx, vy) = throw_vel(h, sp);
            vel = [vx, vy, x(force).fadd_m(1.0).fmul_m(cf(0x3ECC_CCCD)).fmul_m(sp).to_f32()];
        }
        0x10 | 0x27 => {
            time = now().wrapping_add(2000);
            let mut sp = throw_speed(force);
            if ty == 0x27 {
                sp = x(sp).fmul_m(0.5).to_f32();
            }
            let mut h = heading(creator);
            if etype(creator) == 2 {
                h = st0(limit_radian_angle(x(h).fadd_m(cf(0x4049_0FDB)).to_f32())).to_f32();
            }
            place(mp, h, origin);
            let (vx, vy) = throw_vel(h, sp);
            vel = [vx, vy, x(force).fadd_m(1.0).fmul_m(cf(0x3ECC_CCCD)).fmul_m(sp).to_f32()];
            if rh(creator, 0x22) == 0x195 {
                vel[0] = x(vx).fadd_m(rf(creator, 0x44)).to_f32();
                vel[1] = x(vel[1]).fadd_m(rf(creator, 0x48)).to_f32();
                vel[2] = x(vel[2]).fadd_m(rf(creator, 0x4C)).to_f32();
            }
            marker = 5;
            elasticity = if ty == 0x27 { cf(0x3CF5_C28F) } else { 0.5 };
        }
        0x12 => {
            time = now().wrapping_add(2000);
            let v = x(force).fmul_m(cf(0x3E61_47AE)).fadd_m(cf(0x3E19_999A));
            let sp = if !v.fcom(x(cf(0x3E4C_CCCD))).test_parity(5) { cf(0x3E4C_CCCD) } else { v.to_f32() };
            let h = heading(creator);
            place(mp, h, origin);
            let (vx, vy) = throw_vel(h, sp);
            vel = [vx, vy, x(force).fmul_m(cf(0x3E4C_CCCD)).fadd_m(cf(0x3ECC_CCCD)).fmul_m(sp).to_f32()];
        }
        0x13 | 0x14 => {
            let mut sp;
            if ty == 0x13 {
                time = now().wrapping_add(3000);
                sp = cf(0x3ECC_CCCD);
            } else {
                time = now().wrapping_add(10_000);
                sp = cf(0x3E4C_CCCD);
            }
            let t = etype(creator);
            if t == 2 {
                matrix_assign(mp, placeable_get_matrix(creator));
                wf(mp, 0x30, ox);
                wf(mp, 0x34, oy);
                wf(mp, 0x38, oz);
                sp = st0(vector_magnitude(creator.byte_add(0x44))).fadd_m(sp).to_f32();
            } else if t == 3 && ped_is_player(creator) != 0 {
                let off = usize::from(g::<u8>(0xB6_F081).read()) * 0x238;
                let front: [f32; 3] = g::<[f32; 3]>(0xB6_F32C + off).read();
                let up: [f32; 3] = g::<[f32; 3]>(0xB6_F350 + off).read();
                let mut out = [0f32; 3];
                let r = cross_product(lp(&mut out).cast(), g(0xB6_F350 + off), g(0xB6_F32C + off));
                let right: [f32; 3] = r.cast::<[f32; 3]>().read();
                for i in 0..3 {
                    let o = 4 * i as isize;
                    wf(mp, 0x10 + o, front[i]);
                    wf(mp, 0x20 + o, up[i]);
                    wf(mp, o, right[i]);
                }
                wf(mp, 0x30, ox);
                wf(mp, 0x34, oy);
                wf(mp, 0x38, oz);
            } else if !dir.is_null() {
                let d: [f32; 3] = dir.cast::<[f32; 3]>().read();
                for (i, v) in d.iter().enumerate() {
                    wf(mp, 0x10 + 4 * i as isize, *v);
                }
                let m = placeable_get_matrix(creator);
                for i in 0..3 {
                    wf(mp, 4 * i, rf(m, 4 * i));
                }
                let mut out = [0f32; 3];
                let u = cross_product(lp(&mut out).cast(), mp, mp.byte_add(0x10));
                let up: [f32; 3] = u.cast::<[f32; 3]>().read();
                for (i, v) in up.iter().enumerate() {
                    wf(mp, 0x20 + 4 * i as isize, *v);
                }
                wf(mp, 0x30, ox);
                wf(mp, 0x34, oy);
                wf(mp, 0x38, oz);
            } else {
                matrix_assign(mp, placeable_get_matrix(creator));
            }
            let mut v = [0.0, sp, 0.0];
            let mut out = [0f32; 3];
            let r = multiply3x3(lp(&mut out).cast(), mp, lp(&mut v).cast());
            vel = r.cast::<[f32; 3]>().read();
            collide = 0;
        }
        0x15 | 0x3A => {
            if ty == 0x3A {
                streaming_request_model(i32::from(g::<u16>(MI_FLARE).read()), 0);
                time = now().wrapping_add(10_000);
            } else {
                time = now().wrapping_add(2_000_000);
            }
            let t = etype(creator);
            if t > 1 && t < 5 {
                vel = [rf(creator, 0x44), rf(creator, 0x48), rf(creator, 0x4C)];
            }
            matrix_assign(mp, placeable_get_matrix(creator));
            wf(mp, 0x30, ox);
            wf(mp, 0x34, oy);
            wf(mp, 0x38, oz);
        }
        _ => {}
    }

    let active = |i: usize| g::<u8>(INFOS + i * 0x24 + 0x10).read() != 0;
    let Some(slot_i) = (0..32).find(|&i| !active(i)) else {
        matrix_dtor(mp);
        return 0;
    };
    let slot: GPtr<GPtr<u8>> = g(OBJECTS + slot_i * 4);
    match ty {
        0x10..=0x12 | 0x27 => {
            let p = object_new(0x17C);
            let obj = if p.is_null() { GPtr::NULL } else { projectile_ctor(p, ru(get_weapon_info(ty as i32, 1), 0xC) as i32) };
            slot.write(obj);
            if !obj.is_null() {
                let mi = g::<GPtr<u8>>((MODEL_INFOS as u32).wrapping_add((i32::from(rh(obj, 0x22) as i16) * 4) as u32) as usize).read();
                let cm = rp(mi, 0x14);
                let data = rp(cm, 0x2C);
                let radius = || x(rf(cm, 0x24)).fmul_m(cf(0x3F40_0000)).to_f32();
                if data.is_null() {
                    col_model_allocate_data(cm, 1, 0, 0, 0, 0, 0);
                    col_sphere_set(rp(rp(cm, 0x2C), 8), radius(), cm.byte_add(0x18), 0x38, 0, 0xFF);
                } else if rh(data, 0) == 0 && rp(data, 8).is_null() {
                    wh(data, 0, 1);
                    let s = memory_mgr_malloc(0x14, 0);
                    wp(data, 8, s);
                    col_sphere_set(s, radius(), cm.byte_add(0x18), 0x38, 0, 0xFF);
                }
            }
        }
        0x13..=0x15 => {
            let p = object_new(0x17C);
            let obj = if p.is_null() { GPtr::NULL } else { projectile_ctor(p, ru(get_weapon_info(ty as i32, 1), 0xC) as i32) };
            slot.write(obj);
        }
        0x3A => {
            let p = object_new(0x17C);
            let obj = if p.is_null() { GPtr::NULL } else { projectile_ctor(p, i32::from(g::<u16>(MI_FLARE).read())) };
            slot.write(obj);
            if !obj.is_null() {
                wf(obj, 0x98, cf(0x3F66_6666));
            }
        }
        _ => {}
    }
    let obj = slot.read();
    if obj.is_null() {
        matrix_dtor(mp);
        return 0;
    }
    let info: GPtr<u8> = g(INFOS + slot_i * 0x24);
    wu(info, 0, ty);
    wp(info, 4, creator);
    entity_register_reference(creator, info.byte_add(4));
    placeable_set_matrix(slot.read(), mp);
    let o = slot.read();
    wf(o, 0x44, vel[0]);
    wf(o, 0x48, vel[1]);
    wf(o, 0x4C, vel[2]);
    let fl = ru(o, 0x40);
    wu(o, 0x40, fl ^ ((u32::from(collide) << 1 ^ fl) & 2));
    wu(info, 0xC, time);
    wf(slot.read(), 0x9C, elasticity);
    if marker == 5 {
        wu(slot.read(), 0x160, 0xBB_4BD0);
    }
    wp(info, 8, target);
    if !target.is_null() {
        entity_register_reference(target, info.byte_add(8));
    }
    wb(info, 0x10, 1);
    world_add(slot.read());
    entity_register_reference(slot.read(), slot.cast());
    let o = slot.read();
    let m = rp(o, 0x14);
    let p = if m.is_null() { o.byte_add(4) } else { m.byte_add(0x30) };
    wf(info, 0x14, rf(p, 0));
    wf(info, 0x18, rf(p, 4));
    wf(info, 0x1C, rf(p, 8));
    if ty != 0x13 && ty != 0x14 && ty == 0x11 {
        let mut pt = [0f32; 3];
        let mm = entity_get_modelling_matrix(o);
        if !mm.is_null() {
            let fx = fx_create_system(g(0xA9_AE80), 0x87_2BF0, lp(&mut pt).cast(), mm, 0);
            wp(info, 0x20, fx);
            if !fx.is_null() {
                fx_play(fx);
            }
        }
    }
    let o = slot.read();
    wp(o, 0x128, creator);
    wu(o, 0x40, ru(o, 0x40) | 0x1000_0000);
    if !creator.is_null() && matches!(etype(creator), 2..=4) && rp(creator, 0x128).is_null() {
        wp(creator, 0x128, creator);
    }
    if ty == 0x14 {
        let pool = g::<GPtr<u8>>(0xB7_449C).read();
        let idx = ((slot.read().addr() as i64 - rp(pool, 0).addr() as i64) / 0x19C) as i32;
        let handle = (idx << 8) + i32::from(rb(rp(pool, 4), idx as isize));
        let blip = radar_set_entity_blip(3, handle, 0xFF00_00FF, 2, 0x87_2BE8);
        radar_change_blip_scale(blip, 1);
        let own = creator == find_player_ped(-1) || creator == find_player_vehicle(-1, 0);
        radar_change_blip_colour(blip, if own { 0xFFFF_FFFF } else { 0xFF00_00FF });
    }
    audio_report_weapon_event(g(0xB6_BC90), 0x94, ty, slot.read());
    matrix_dtor(mp);
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ADD_PROJECTILE];
