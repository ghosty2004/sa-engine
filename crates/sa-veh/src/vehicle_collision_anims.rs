//! Vehicle collision entry points of the physics pass
//! (`CAutomobile::ProcessEntityCollision`, `CBike::ProcessEntityCollision`), the
//! `PlaceOnRoadProperly` ground placement of cars and bikes and the driving-anim functions of
//! `CVehicle` and `CBmx`. Ported from the 1.0 US bytes (SecuROM fragments inlined), x87 sequences
//! through the [`Fpu`] model; gta-reversed was a reading aid only. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal, clippy::needless_range_loop)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::collision::fpu::Fpu;
use sa_world::physical::attach::{
    allocate_matrix, matrix_copy_ctor, matrix_dtor, process_col_models, simple_transform_update_matrix, LocalMatrix,
};
use sa_world::physical::shared::*;

orig! {
    /// `CPhysical::AddCollisionRecord(CEntity*)` (0x543490)
    pub fn add_collision_record(e: Ent, other: Ent) = thiscall 0x543490 => sa_world::physical::base::add_collision_record;
    /// `CColPoint::operator=(const CColPoint&)` (stub 0x40FC80 → 0x15637E0)
    pub fn col_point_assign(dst: Ent, src: Ent) -> Ent = thiscall 0x40fc80 => unported;
    /// `eh vector constructor iterator(void* p, uint32 size, int32 count, void (*ctor)(void*))` (0x403D60)
    pub fn vector_ctor_iterator(p: GPtr<u8>, size: u32, count: i32, ctor: u32) = stdcall 0x403d60 => unported;
    /// `CDamageManager::GetWheelStatus(int32)` (0x6C21B0)
    pub fn get_wheel_status(dm: GPtr<u8>, wheel: i32) -> u32 = thiscall 0x6c21b0 => crate::damage::get_wheel_status;
    /// `SurfaceInfos_c::GetAdhesionGroup(uint32)` (0x55E5C0)
    pub fn get_adhesion_group(si: Ent, id: u32) -> u32 = thiscall 0x55e5c0 => sa_world::physical::mvpe_phys1::get_adhesion_group_port;
    /// `CPed::IsPlayer()` (0x5DF8F0, AL)
    pub fn ped_is_player(p: Ent) -> u8 = thiscall 0x5df8f0 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, uint32 animId)` (0x4D68B0)
    pub fn clump_get_association(clump: Ent, id: u32) -> Ent = cdecl 0x4d68b0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    pub fn blend_animation(clump: Ent, group: u32, id: u32, delta: f32) -> Ent = cdecl 0x4d4610 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40, ST0)
    pub fn get_stat_value(stat: u16) -> f64 = cdecl 0x558e40 => sa_world::world::restart::stat_value_port;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    pub fn get_vehicle_appearance(v: Ent) -> u32 = thiscall 0x6d1080 => crate::vehicle2::get_vehicle_appearance;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x6, CStoredCollPoly*)`
    /// (0x5674E0, AL)
    pub fn process_vertical_line(origin: GPtr<u8>, distance: f32, cp: GPtr<u8>, out: GPtr<u8>, b: u8, v: u8, p: u8,
        o: u8, d: u8, see: u8, poly: GPtr<u8>) -> u8 = cdecl 0x5674e0 => sa_world::world::mvpc_world1::process_vertical_line;
    /// virtual `CVehicle::GetHeightAboveRoad()` (vtable +0xD4, ST0)
    pub fn get_height_above_road_v(v: Ent) -> f64 = thiscall vslot 0xD4 => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn normalise(v: GPtr<sa_math::vector::CVector>) = thiscall 0x59c910 => sa_world::collision::data::normalise_port;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00)
    pub fn ped_set_move_state(p: Ent, s: i32) = thiscall 0x5dec00 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80)
    pub fn assoc_set_current_time(a: Ent, t: f32) = thiscall 0x4cea80 => unported;
    /// `CBike::ProcessRiderAnims(CPed*, CVehicle*, CRideAnimData*, tBikeHandlingData*, int16)` (0x6B7280)
    pub fn process_rider_anims(ped: Ent, v: Ent, data: Ent, handling: u32, a5: u32) = cdecl 0x6b7280 => unported;
    /// `CGeneral::GetATanOfXY(float, float)` (0x53CC70, ST0)
    pub fn get_atan_of_xy(x: f32, y: f32) -> f64 = cdecl 0x53cc70 => unported;
}
/// `TheCamera.m_nActiveCam` (0xB6F081, byte)
const CAM_ACTIVE: usize = 0xB6F081;
/// `TheCamera.m_aCams[0].m_nMode` (0xB6F1A8, int16; stride 0x238)
const CAM0_MODE: usize = 0xB6F1A8;
/// `TheCamera.m_aCams[0].m_nDirectionWasLooking` (0xB6F1C4; stride 0x238)
const CAM0_LOOKING: usize = 0xB6F1C4;

/// The active camera is in mode 0x10 and looking in direction 1 (the first-person look-back test of
/// the driving-anim functions).
fn cam_looking_back() -> bool {
    let i = usize::from(GPtr::<u8>::from_addr(CAM_ACTIVE).read()) * 0x238;
    GPtr::<u16>::from_addr(CAM0_MODE + i).read() == 0x10 && GPtr::<u32>::from_addr(CAM0_LOOKING + i).read() == 1
}

/// `CAnimBlendAssociation::m_fBlendAmount` (+0x18)
const BLEND_AMOUNT: isize = 0x18;
/// `CAnimBlendAssociation::m_fBlendDelta` (+0x1C)
const BLEND_DELTA: isize = 0x1C;

/// `CVehicle::ProcessDrivingAnims(CPed* driver, bool blend)` (0x6DF4A0, `ret 8`): the player driver's
/// sit / lean anims.
///
/// Nothing with entity flag 0x20000 or a non-player driver. The association 0xBD of the driver's clump
/// (+0x18; with `blend` `BlendAnimation(clump, 0, 0xBD, 4.0)` instead) ends it when present. The move
/// speed length `sqrt(x*x + y*y + z*z)` (x87, stored as f32 into the `blend` argument slot) picks the
/// four-anim table: vehicle flag +0x429 bit 3 → 0x8D3514; vehicle type +0x590 == 5 and handling
/// (+0x384) model flags (+0xCC) without 0x400 → 0x8D3524; the handling anim group (+0xDE) flags of
/// 0xC1CDC8[g*0x94] bit 2 → 0x8D35A4, bit 3 → 0x8D3594; else stat 0xA0 (`GetStatValue` called twice)
/// `>= 100` → 0x8D3574 / 0x8D3584 (speed > 0.4 / not), `< 50` → 0x8D3534 / 0x8D3544, else 0x8D3554 /
/// 0x8D3564. Associations a0..a3 of the table's ids (a2 lives in the `driver` argument slot, a3 in
/// the `blend` slot). No a0: with association 0x3C blend in t[0] (delta 4.0). a0's blend amount below
/// 1.0 (not NaN) ends it. a3 with the look-back camera gets delta -1000. The first present of 0x4A..
/// 0x4D is `lean`. Steer: vehicle flag bit 3 clear, +0x49C (steer angle) < 0, no lean, appearance not
/// 3/5: look-back camera ends it; a3 with blend amount < 1 or delta > 0 (or NaN) ends it; else blend
/// in t[3]. Otherwise +0x494 nonzero (or NaN) and no lean: `+0x494 * ±1.6393442` (0x871FD4 /
/// 0x871FD0 by sign) clamped to [0, 1] on the x87 stack, the other side's anim zeroed, the matching
/// side gets the value as blend amount (delta 0; missing → blend in t[1] / t[2]), a3 delta -4.
/// Else a1..a3 delta -4.
#[hook(addr = 0x6df4a0, conv = thiscall, name = "CVehicle::ProcessDrivingAnims", expect = "51 57 8B F9 F7 47 1C 00 00 02 00",
       this = "CVehicle", unverified = "mvpf-veh-1", route = "L2", hazards = "stack-residue", recs = "driving",
       note = "argument slots reused: S+4 (driver) holds the third association, S+8 (blend) the speed length, then the fourth association; S-4 (pushed ecx) holds this")]
pub fn vehicle_process_driving_anims(this: Ent, ped: Ent, blend: u8) {
    if ru(this, EFLAGS) & 0x20000 != 0 {
        return;
    }
    if ped_is_player(ped) == 0 {
        return;
    }
    let clump = rp(ped, RW_OBJECT);
    let mut a = clump_get_association(clump, 0xBD);
    if blend != 0 {
        a = blend_animation(clump, 0, 0xBD, 4.0);
    }
    if !a.is_null() {
        return;
    }
    let mut f = Fpu::new();
    f.fld(rf(this, MOVE_SPEED + 8));
    f.fld(rf(this, MOVE_SPEED + 4));
    f.fld(rf(this, MOVE_SPEED));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fsqrt();
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    let speed = f.fstp();
    let fast = |f: &mut Fpu| {
        f.fld(speed);
        f.fcomp_m(cf(0x3ECC_CCCD));
        !f.jne(0x41)
    };
    let table: usize = if rb(this, 0x429) & 8 != 0 {
        0x8D3514
    } else if ru(this, 0x590) == 5 && ru(rp(this, 0x384), 0xCC) & 0x400 == 0 {
        0x8D3524
    } else {
        let g = usize::from(rb(rp(this, 0x384), 0xDE));
        let fl = GPtr::<u32>::from_addr(0xC1CDC8 + g * 0x94).read();
        if (fl >> 2) & 1 != 0 {
            0x8D35A4
        } else if (fl >> 3) & 1 != 0 {
            0x8D3594
        } else {
            f.push(X87::from_f64_exact(get_stat_value(0xA0)));
            f.fcomp_m(cf(0x42C8_0000));
            if !f.jne(1) {
                if fast(&mut f) { 0x8D3574 } else { 0x8D3584 }
            } else {
                f.push(X87::from_f64_exact(get_stat_value(0xA0)));
                f.fcomp_m(cf(0x4248_0000));
                let low = f.jne(1);
                let fa = fast(&mut f);
                if low {
                    if fa { 0x8D3534 } else { 0x8D3544 }
                } else if fa {
                    0x8D3554
                } else {
                    0x8D3564
                }
            }
        }
    };
    let t = |i: usize| GPtr::<u32>::from_addr(table + 4 * i).read();
    let a0 = clump_get_association(clump, t(0));
    let a1 = clump_get_association(clump, t(1));
    let a2 = clump_get_association(clump, t(2));
    let a3 = clump_get_association(clump, t(3));
    if a0.is_null() {
        if clump_get_association(clump, 0x3C).is_null() {
            return;
        }
        blend_animation(clump, 0, t(0), 4.0);
        return;
    }
    f.fld(rf(a0, BLEND_AMOUNT));
    f.fcomp_m(cf(0x3F80_0000));
    if !f.jp(5) {
        return;
    }
    if !a3.is_null() && cam_looking_back() {
        wu(a3, BLEND_DELTA, 0xC47A_0000);
    }
    let mut lean = Ent::NULL;
    for id in [0x4A, 0x4B, 0x4C, 0x4D] {
        lean = clump_get_association(clump, id);
        if !lean.is_null() {
            break;
        }
    }
    let mut steer_branch = false;
    if rb(this, 0x429) & 8 == 0 {
        f.fld(rf(this, 0x49C));
        f.fcomp_m(0.0);
        if !f.jp(5)
            && lean.is_null()
            && get_vehicle_appearance(this) != 3
            && get_vehicle_appearance(this) != 5
        {
            steer_branch = true;
        }
    }
    if steer_branch {
        if cam_looking_back() {
            return;
        }
        if !a3.is_null() {
            f.fld(rf(a3, BLEND_AMOUNT));
            f.fcomp_m(cf(0x3F80_0000));
            if f.jp(5) {
                return;
            }
            f.fld(rf(a3, BLEND_DELTA));
            f.fcomp_m(0.0);
            if f.jp(0x41) {
                return;
            }
        }
        blend_animation(clump, 0, t(3), 4.0);
        return;
    }
    f.fld(rf(this, 0x494));
    f.fcomp_m(0.0);
    if !f.jp(0x44) || !lean.is_null() {
        for x in [a1, a2, a3] {
            if !x.is_null() {
                wu(x, BLEND_DELTA, 0xC080_0000);
            }
        }
        return;
    }
    f.fld(rf(this, 0x494));
    f.fcomp_m(0.0);
    f.fld(rf(this, 0x494));
    let (on, off, id) = if !f.jne(0x41) {
        f.fmul_m(cf(0x3FD1_D608));
        (a1, a2, t(1))
    } else {
        f.fmul_m(cf(0xBFD1_D608));
        (a2, a1, t(2))
    };
    f.fcom_m(0.0);
    if !f.jp(5) {
        f.fstp_st(0);
        f.fld(0.0);
    } else {
        f.fcom_m(cf(0x3F80_0000));
        if !f.jne(0x41) {
            f.fstp_st(0);
            f.fld(cf(0x3F80_0000));
        }
    }
    if !off.is_null() {
        wu(off, BLEND_AMOUNT, 0);
        wu(off, BLEND_DELTA, 0);
    }
    if on.is_null() {
        f.fstp_st(0);
        blend_animation(clump, 0, id, 4.0);
    } else {
        wf(on, BLEND_AMOUNT, f.fstp());
        wu(on, BLEND_DELTA, 0);
    }
    if !a3.is_null() {
        wu(a3, BLEND_DELTA, 0xC080_0000);
    }
}

/// One `CWorld::ProcessVerticalLine` ground probe of the `PlaceOnRoadProperly` functions at `(x, y)`
/// from `pos.z + 5` down to `pos.z - 5` (buildings only). A hit stores the col point's lighting byte
/// (+0x25) as a dword at `light_off`, makes the entity the last ground entity (+0x48C) with its entity
/// flag bits 30/31 passed on, and returns the point's z; no hit returns `pos.z`. The f32 `pos.z - 5`
/// is computed first (x87 `fsub`, stored) and also used as the line's distance argument.
fn ground_probe(this: Ent, x: u32, y: u32, light_off: isize) -> u32 {
    let mut f = Fpu::new();
    f.fld(rf(pos_ptr(this), 8));
    f.fsub_m(cf(0x40A0_0000));
    let lo = f.fstp();
    let mut origin = [x, y, lo.to_bits()];
    f.fld(rf(pos_ptr(this), 8));
    f.fadd_m(cf(0x40A0_0000));
    origin[2] = f.fstp().to_bits();
    let mut cp = [0u32; 0x2C / 4];
    let mut ent = Ent::NULL;
    let hit = process_vertical_line(lp(&mut origin).cast(), lo, lp(&mut cp).cast(), lp(&mut ent).cast(), 1, 0, 0, 0, 0, 0, Ent::NULL);
    if hit == 0 {
        return ru(pos_ptr(this), 8);
    }
    let cpp = lp(&mut cp).cast::<u8>();
    wu(this, light_off, u32::from(rb(cpp, 0x25)));
    wp(this, 0x48C, ent);
    let a = ru(this, EFLAGS) ^ ((ru(this, EFLAGS) ^ ru(ent, EFLAGS)) & 0x4000_0000);
    wu(this, EFLAGS, a);
    let ef = ru(ent, EFLAGS);
    wu(this, EFLAGS, ((ef ^ a) & 0x7FFF_FFFF) ^ ef);
    ru(cpp, 8)
}

/// `CBike::PlaceOnRoadProperly()` (0x6BEEB0, `ret`): places the bike on the ground between its front
/// and rear contact points.
///
/// From the col model's bounding box: `rear = -min.y` (+4, f32), `front = max.y` (+0x10). With the
/// matrix forward (+0x10) and the position: rear point `pos.xy - fwd.xy * rear`, front point `pos.xy +
/// fwd.xy * front` (each f32). Ground probes (front first, lighting dword +0x544; rear +0x570). Then
/// on the x87 stack `len = rear + front`, `a = atan(dz / len)` (`fpatan` with 1.0), right = `((fy -
/// ry) / len, -(fx - rx) / len, 0)`, forward = `(-cos a * right.y, cos a * right.x, sin a)`, up =
/// `CrossProduct(right, forward)`, position = the mean of the two points (x87 sums times 0.5) with z
/// plus the virtual `GetHeightAboveRoad` (vtable +0xD4); without a matrix the placement position.
/// A null matrix is dereferenced for the forward vector like the original (+0x10).
#[hook(addr = 0x6beeb0, conv = thiscall, name = "CBike::PlaceOnRoadProperly", expect = "83 EC 74 56 8B F1",
       this = "CBike", unverified = "mvpf-veh-1", route = "L2", hazards = "x87-transcendental", recs = "driving")]
pub fn bike_place_on_road_properly(this: Ent) {
    let front = rf(get_col_model(this), 0x10);
    let mut f = Fpu::new();
    f.fld(rf(get_col_model(this), 4));
    f.fchs();
    let rear = f.fstp();
    let m = matrix(this);
    let fwd = rv(m, 0x10);
    let calc = |f: &mut Fpu, d: f32, mul: f32, sub: bool, p: f32| {
        f.fld(d);
        f.fmul_m(mul);
        if sub {
            f.fsubr_m(p);
        } else {
            f.fadd_m(p);
        }
        f.fstp()
    };
    let rx = calc(&mut f, fwd[0], rear, true, rf(pos_ptr(this), 0));
    let ry = calc(&mut f, fwd[1], rear, true, rf(pos_ptr(this), 4));
    let fx = calc(&mut f, fwd[0], front, false, rf(pos_ptr(this), 0));
    let fy = calc(&mut f, fwd[1], front, false, rf(pos_ptr(this), 4));
    let fz = f32::from_bits(ground_probe(this, fx.to_bits(), fy.to_bits(), 0x544));
    let rz = f32::from_bits(ground_probe(this, rx.to_bits(), ry.to_bits(), 0x570));
    f.fld(rear);
    f.fadd_m(front);
    f.fld(fz);
    f.fsub_m(rz);
    f.fdiv_st(1);
    f.fld1();
    let x = f.pop();
    let y = f.st(0);
    f.pop();
    f.push(y.fpatan(x));
    f.fld_st(0);
    let c = f.st(0).fcos();
    f.pop();
    f.push(c);
    let m = matrix(this);
    f.fld(fy);
    f.fsub_m(ry);
    f.fdiv_st(3);
    wf(m, 0, f.fstp());
    f.fld(fx);
    f.fsub_m(rx);
    f.fdiv_st(3);
    f.fchs();
    wf(matrix(this), 4, f.fstp());
    wu(matrix(this), 8, 0);
    let m = matrix(this);
    f.fld_st(0);
    f.fmul_m(rf(m, 4));
    f.fchs();
    wf(m, 0x10, f.fstp());
    let m = matrix(this);
    f.fmul_m(rf(m, 0));
    wf(m, 0x14, f.fstp());
    let s = f.st(0).fsin();
    f.pop();
    f.push(s);
    f.fstp_st(1);
    wf(matrix(this), 0x18, f.fstp());
    let m = matrix(this);
    let mut up = [0u32; 3];
    cross_product(lp(&mut up).cast(), m.cast(), m.byte_add(0x10).cast());
    f.fld(fx);
    f.fadd_m(rx);
    f.fld(fy);
    f.fadd_m(ry);
    f.fld(rz);
    f.fadd_m(fz);
    wu(matrix(this), 0x20, up[0]);
    wu(matrix(this), 0x24, up[1]);
    wu(matrix(this), 0x28, up[2]);
    let zs = f.fstp();
    f.fxch(1);
    f.fmul_m(0.5);
    let px = f.fstp();
    f.fmul_m(0.5);
    let py = f.fstp();
    f.fld(zs);
    f.fmul_m(0.5);
    let pz = f.fstp();
    f.push(X87::from_f64_exact(get_height_above_road_v(this)));
    f.fadd_m(pz);
    let m = matrix(this);
    if !m.is_null() {
        wf(m, 0x30, px);
        wf(matrix(this), 0x34, py);
        wf(matrix(this), 0x38, f.fstp());
    } else {
        wf(this, 0xC, f.fstp());
        wf(this, 4, px);
        wf(this, 8, py);
    }
}

/// The two vertical-line probes of one axle of `CAutomobile::PlaceOnRoadProperly` at `(x, y)`: up
/// from `pos.z` to `pos.z + 5`, then down from `pos.z` to `pos.z - 5` (buildings only), sharing one
/// col point and one entity slot like the original's locals. The hit nearer to `pos.z` (the down hit
/// only when strictly nearer, x87 `fabs` compare) gives the z and becomes the last ground entity
/// (+0x48C, entity flag bits 30/31 passed on). With any hit the col point buffer's lighting byte
/// (+0x25: the down probe's when it ran into something, even if the up hit won) is stored as a dword
/// at `light_off` and `Some(z)` returned (as an x87 value: the rear axle keeps it on the stack).
fn axle_probe(this: Ent, x: u32, y: u32, pz: u32, light_off: isize, rear: bool) -> Option<X87> {
    let mut origin = [x, y, pz];
    let mut cp = [0u32; 0x2C / 4];
    let mut ent = Ent::NULL;
    let cpp = lp(&mut cp).cast::<u8>();
    let set_ground = |this: Ent, ent: Ent| {
        wp(this, 0x48C, ent);
        let a = ru(this, EFLAGS) ^ ((ru(this, EFLAGS) ^ ru(ent, EFLAGS)) & 0x4000_0000);
        wu(this, EFLAGS, a);
        let ef = ru(ent, EFLAGS);
        wu(this, EFLAGS, ((ef ^ a) & 0x7FFF_FFFF) ^ ef);
    };
    let mut f = Fpu::new();
    f.fld(f32::from_bits(pz));
    f.fadd_m(cf(0x40A0_0000));
    let hi = f.fstp();
    let up = process_vertical_line(lp(&mut origin).cast(), hi, cpp, lp(&mut ent).cast(), 1, 0, 0, 0, 0, 0, Ent::NULL);
    let mut z1 = 0f32;
    if up != 0 {
        z1 = rf(cpp, 8);
        set_ground(this, ent);
    }
    f.fld(f32::from_bits(pz));
    f.fsub_m(cf(0x40A0_0000));
    let lo = f.fstp();
    let down = process_vertical_line(lp(&mut origin).cast(), lo, cpp, lp(&mut ent).cast(), 1, 0, 0, 0, 0, 0, Ent::NULL);
    let z = if down != 0 {
        let take_down = up == 0 || {
            let p = f32::from_bits(pz);
            if rear {
                f.fld(z1);
                f.fld(p);
                f.fsub_st(1);
            } else {
                f.fld(p);
                f.fsub_m(z1);
            }
            f.fabs();
            f.fld(p);
            f.fsub_m(rf(cpp, 8));
            f.fabs();
            f.fcompp();
            !f.jp(5)
        };
        if take_down {
            if rear && up != 0 {
                f.fstp_st(0);
            }
            set_ground(this, ent);
            X87::from_f32(rf(cpp, 8))
        } else if rear {
            f.pop()
        } else {
            X87::from_f32(z1)
        }
    } else if up != 0 {
        X87::from_f32(z1)
    } else {
        return None;
    };
    wu(this, light_off, u32::from(rb(cpp, 0x25)));
    Some(z)
}

/// `CAutomobile::PlaceOnRoadProperly()` (0x6AF420, `ret`; entry `nop; jmp` into the 0x401C57
/// fragment holding `sub esp, 0x84`): places the car on the ground between its front and rear axle.
///
/// From the col data's lines (col model +0x2C → +0x10): `front = line[0].start.y` (+4), `rear =
/// -line[3].start.y` (+0x64). Axle points `front = pos.xy + fwd.xy * front`, `rear = pos.xy - fwd.xy
/// * rear` (f32, matrix forward +0x10). Each axle: [`axle_probe`] (front: lighting dword +0x544, rear
/// +0x570); without a hit the z stays `pos.z`. Then `frontZ = GetHeightAboveRoad() (vtable +0xD4) +
/// frontZ`, `rearZ = +0x89C + rearZ`, `len = rear + front` (f32). Right = `((fy - ry) / len, -(fx -
/// rx) / len, 0)`, forward = `Normalise(front - rear)`, up = `CrossProduct(right, forward)`. The new
/// position is `(front * rear + rearPoint * front) / len` (products and sums on the x87 stack as the
/// original orders them, `1/len` stored as f32 and multiplied). Type +0x594 == 4: +0x9A4 =
/// `GetATanOfXY(fwd.x, fwd.y)`.
#[hook(addr = 0x6af420, conv = thiscall, name = "CAutomobile::PlaceOnRoadProperly", expect = "90 E9 31 28 D5 FF",
       this = "CAutomobile", unverified = "mvpf-veh-1", route = "L2", hazards = "none", recs = "driving",
       note = "entry is `nop; jmp 0x401C57` (SecuROM fragment with the frame setup)")]
pub fn automobile_place_on_road_properly(this: Ent) {
    let lines = rp(rp(get_col_model(this), 0x2C), 0x10);
    let front = rf(lines, 4);
    let mut f = Fpu::new();
    f.fld(rf(lines, 0x64));
    f.fchs();
    let rear = f.fstp();
    let fwd = rv(matrix(this), 0x10);
    let calc = |f: &mut Fpu, d: f32, mul: f32, sub: bool, p: f32| {
        f.fld(d);
        f.fmul_m(mul);
        if sub {
            f.fsubr_m(p);
        } else {
            f.fadd_m(p);
        }
        f.fstp()
    };
    let rx = calc(&mut f, fwd[0], rear, true, rf(pos_ptr(this), 0));
    let ry = calc(&mut f, fwd[1], rear, true, rf(pos_ptr(this), 4));
    let fx = calc(&mut f, fwd[0], front, false, rf(pos_ptr(this), 0));
    let fy = calc(&mut f, fwd[1], front, false, rf(pos_ptr(this), 4));
    let pz = ru(pos_ptr(this), 8);
    let mut fz = f32::from_bits(pz);
    if let Some(z) = axle_probe(this, fx.to_bits(), fy.to_bits(), pz, 0x544, false) {
        fz = z.to_f32();
    }
    let pz = ru(pos_ptr(this), 8);
    let mut rz = f32::from_bits(pz);
    if let Some(z) = axle_probe(this, rx.to_bits(), ry.to_bits(), pz, 0x570, true) {
        rz = z.to_f32();
    }
    f.push(X87::from_f64_exact(get_height_above_road_v(this)));
    f.fadd_m(fz);
    let fz = f.fstp();
    f.fld(rf(this, 0x89C));
    f.fadd_m(rz);
    let rz = f.fstp();
    f.fld(rear);
    f.fadd_m(front);
    let len = f.fstp();
    f.fld(fy);
    f.fsub_m(ry);
    f.fdiv_m(len);
    wf(matrix(this), 0, f.fstp());
    f.fld(fx);
    f.fsub_m(rx);
    f.fdiv_m(len);
    f.fchs();
    wf(matrix(this), 4, f.fstp());
    wu(matrix(this), 8, 0);
    let mut fw = [0f32; 3];
    for (i, (a, b)) in [(fx, rx), (fy, ry), (fz, rz)].into_iter().enumerate() {
        f.fld(a);
        f.fsub_m(b);
        fw[i] = f.fstp();
    }
    normalise(lp(&mut fw).cast());
    wf(matrix(this), 0x10, fw[0]);
    wf(matrix(this), 0x14, fw[1]);
    wf(matrix(this), 0x18, fw[2]);
    let m = matrix(this);
    let mut up = [0u32; 3];
    cross_product(lp(&mut up).cast(), m.cast(), m.byte_add(0x10).cast());
    wu(matrix(this), 0x20, up[0]);
    wu(matrix(this), 0x24, up[1]);
    wu(matrix(this), 0x28, up[2]);
    f.fld(rx);
    f.fmul_m(front);
    f.fld(ry);
    f.fmul_m(front);
    f.fld(rz);
    f.fmul_m(front);
    let p3 = f.fstp();
    f.fld(fx);
    f.fmul_m(rear);
    f.fld(fy);
    f.fmul_m(rear);
    let q2 = f.fstp();
    f.fld(fz);
    f.fmul_m(rear);
    let q3 = f.fstp();
    f.fadd_st(2);
    let sx = f.fstp();
    f.fld(q2);
    f.fadd_st(1);
    let sy = f.fstp();
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(q3);
    f.fadd_m(p3);
    let sz = f.fstp();
    f.fld(cf(0x3F80_0000));
    f.fdiv_m(len);
    let inv = f.fstp();
    f.fld(sx);
    f.fmul_m(inv);
    f.fld(sy);
    f.fmul_m(inv);
    let ny = f.fstp();
    f.fld(sz);
    f.fmul_m(inv);
    let nz = f.fstp();
    let m = matrix(this);
    if !m.is_null() {
        wf(m, 0x30, f.fstp());
        wf(matrix(this), 0x34, ny);
        wf(matrix(this), 0x38, nz);
    } else {
        wf(this, 4, f.fstp());
        wf(this, 8, ny);
        wf(this, 0xC, nz);
    }
    if ru(this, 0x594) == 4 {
        let m = matrix(this);
        let a = get_atan_of_xy(rf(m, 0x10), rf(m, 0x14));
        wf(this, 0x9A4, X87::from_f64_exact(a).to_f32());
    }
}

/// `if (!e->m_matrix) { AllocateMatrix(); m_placement.UpdateMatrix(m_matrix); }`
fn ensure_matrix(e: Ent) {
    if matrix(e).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(PLACEMENT_POS), matrix(e));
    }
}

/// `CModelInfo::ms_modelInfoPtrs[model]`.
fn model_info(e: Ent) -> Ent {
    let model = rh(e, MODEL_INDEX) as i16 as i32;
    sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(model).read()
}

/// The ground contact bookkeeping of one wheel of the bike / car entity collision loops: lighting
/// byte from the wheel col point (+0x25) to `+0x574 + i`; a vehicle or object becomes the wheel's
/// ground entity (`ent_off + 4i`, reference registered) with the contact offset `cp.point - its
/// position` (x87 `fsub`, f32) at `ofs_off + 0xC*i`, a vehicle also passes its own `+0x574 + i`
/// byte; the contact surface (+0xBA) from surface B (+0x23); a building becomes the last ground
/// entity (+0x48C) and passes its entity flag bits 30 and 31 on.
fn wheel_contact(this: Ent, e: Ent, i: isize, cp: Ent, ent_off: isize, ofs_off: isize) {
    wb(this, 0x574 + i, rb(cp, 0x25));
    let et = etype(e);
    if et == ty::VEHICLE || et == ty::OBJECT {
        let slot = this.byte_add(ent_off + 4 * i);
        wp(slot, 0, e);
        register_reference(e, slot.cast());
        let pos = pos_ptr(e);
        let mut f = Fpu::new();
        let mut d = [0f32; 3];
        for (j, d) in d.iter_mut().enumerate() {
            f.fld(rf(cp, 4 * j as isize));
            f.fsub_m(rf(pos, 4 * j as isize));
            *d = f.fstp();
        }
        wv(this, ofs_off + 0xC * i, d);
        if etype(e) == ty::VEHICLE {
            wb(this, 0x574 + i, rb(e, 0x574 + i));
        }
    }
    wb(this, CONTACT_SURFACE, rb(cp, 0x23));
    if etype(e) == ty::BUILDING {
        wp(this, 0x48C, e);
        let a = ru(this, EFLAGS) ^ ((ru(this, EFLAGS) ^ ru(e, EFLAGS)) & 0x4000_0000);
        wu(this, EFLAGS, a);
        let ef = ru(e, EFLAGS);
        wu(this, EFLAGS, ((ef ^ a) & 0x7FFF_FFFF) ^ ef);
    }
}

/// The common tail of the entity collision functions: with points or wheel contacts both record the
/// collision (a building does not); with points a building or an object with physical flag 4 sets
/// entity flag 0x1000. Returns `n`.
fn collision_tail(this: Ent, e: Ent, n: i32, wheels: i32) -> i32 {
    if n > 0 || wheels > 0 {
        add_collision_record(this, e);
        if etype(e) != ty::BUILDING {
            add_collision_record(e, this);
        }
        if n > 0 {
            let et = etype(e);
            if et == ty::BUILDING || (et == ty::OBJECT && rb(e, PFLAGS) & 4 != 0) {
                wu(this, EFLAGS, ru(this, EFLAGS) | 0x1000);
            }
        }
    }
    n
}

/// The function-local `static CColPoint aCollisionPoints[32]` of `CBike::ProcessEntityCollision`
/// (0xC1C280, guard bit 0 of 0xC1C800).
const BIKE_STATIC_CPS: usize = 0xC1C280;
const BIKE_STATIC_CPS_GUARD: usize = 0xC1C800;

/// `CBike::ProcessEntityCollision(CEntity* entity, CColPoint* colPoints)` (0x6BDEA0, `ret 8`, EAX =
/// number of points).
///
/// Not simple (status != 2): `bVehicleColProcessed` (+0x42B bit 0). The four wheel ratios (+0x710..)
/// are copied to a local array; physical flags 0x9000 or a ped zero the col data's line count (+6).
/// `ProcessColModels(this, entity, colPoints, wheel col points +0x660, this+0x710, false)` (the
/// ratios are written in place; the entity gets a matrix first). With a driver (+0x460) and byte
/// +0x7B6 set: the static col point array is constructed once (`eh vector constructor iterator`
/// with the trivial ctor 0x4217B0); unless the driver's col data has lines, a copy of the bike's
/// matrix with its position moved by `Multiply3x3(matrix, vehicle struct +0x5C (+0x30 unless model
/// info +0x3C == 5))` (x87 `fadd`, f32) collides the driver's col model against the entity into the
/// static array; with hits: +0x7B6 == 1 is cleared to 0, otherwise the points with byte +0x21 != 0
/// are appended to `colPoints` (`CColPoint::operator=`) while `n < 31`. Then (line count 0: restored
/// to 4) each wheel whose new ratio is `< 1.0` and `<` its old value counts and does the ground
/// bookkeeping (ground entities +0x7C0, offsets +0x7D0). Then the collision records.
///
/// The C++ EH frame (state 0/1/-1 at S-0xC) is not reproduced; nothing in it can throw.
#[hook(addr = 0x6bdea0, conv = thiscall, name = "CBike::ProcessEntityCollision", expect = "64 A1 00 00 00 00 6A FF",
       this = "CBike", unverified = "mvpf-veh-1", route = "L2", hazards = "heap-writes", recs = "driving",
       note = "constructs and writes the function-local static CColPoint[32] at 0xC1C280 (guard 0xC1C800) when the rider collides")]
pub fn bike_process_entity_collision(this: Ent, e: Ent, cps: Ent) -> i32 {
    if rb(this, TYPE) & 0xF8 != 0x10 {
        wb(this, 0x42B, rb(this, 0x42B) | 1);
    }
    let cm = get_col_model(this);
    let old: [u32; 4] = [ru(this, 0x710), ru(this, 0x714), ru(this, 0x718), ru(this, 0x71C)];
    let cd = rp(cm, 0x2C);
    let mut wheels: i32 = 0;
    if ru(this, PFLAGS) & 0x9000 != 0 || etype(e) == ty::PED {
        wb(cd, 6, 0);
    }
    ensure_matrix(e);
    let em = matrix(e);
    let tm = matrix(this);
    let ecm = get_col_model(e);
    let mut n = process_col_models(tm, cm, em, ecm, cps, this.byte_add(0x660), this.byte_add(0x710), 0);
    let driver = rp(this, 0x460);
    if !driver.is_null() && rb(this, 0x7B6) != 0 {
        let dcm = get_col_model(driver);
        let guard = GPtr::<u32>::from_addr(BIKE_STATIC_CPS_GUARD);
        if guard.read() & 1 == 0 {
            guard.write(guard.read() | 1);
            vector_ctor_iterator(GPtr::from_addr(BIKE_STATIC_CPS), 0x2C, 0x20, 0x4217B0);
        }
        if rb(rp(dcm, 0x2C), 6) == 0 {
            let mut lm: LocalMatrix = [0; 18];
            let lmp = lp(&mut lm).cast::<u8>();
            matrix_copy_ctor(lmp, matrix(this));
            let mi = model_info(this);
            let mut vs = rp(mi, 0x5C);
            if ru(mi, 0x3C) != 5 {
                vs = vs.byte_add(0x30);
            }
            let mut ofs = rv_bits(vs, 0);
            let mut out = [0u32; 3];
            let r = multiply3x3(lp(&mut out).cast(), matrix(this), lp(&mut ofs).cast());
            let o = rv(r.cast(), 0);
            let mut f = Fpu::new();
            for j in 0..3isize {
                f.fld(rf(lmp, 0x30 + 4 * j));
                f.fadd_m(o[j as usize]);
                wf(lmp, 0x30 + 4 * j, f.fstp());
            }
            ensure_matrix(e);
            let em = matrix(e);
            let ecm = get_col_model(e);
            let dcm = get_col_model(driver);
            let scps = GPtr::<u8>::from_addr(BIKE_STATIC_CPS);
            let n2 = process_col_models(lmp, dcm, em, ecm, scps, Ent::NULL, Ent::NULL, 0);
            if n2 > 0 {
                if rb(this, 0x7B6) == 1 {
                    wb(this, 0x7B6, 0);
                } else {
                    for i in 0..n2 {
                        if n >= 0x1F {
                            break;
                        }
                        let src = scps.byte_add(0x2C * i as isize);
                        if rb(src, 0x21) != 0 {
                            col_point_assign(cps.byte_add(0x2C * n as isize), src);
                            n += 1;
                        }
                    }
                }
            }
            matrix_dtor(lmp);
        }
    }
    if rb(cd, 6) == 0 {
        wb(cd, 6, 4);
        return collision_tail(this, e, n, wheels);
    }
    for i in 0..4isize {
        let mut f = Fpu::new();
        f.fld(rf(this, 0x710 + 4 * i));
        f.fcomp_m(cf(0x3F80_0000));
        if f.jp(5) {
            continue;
        }
        f.fld(rf(this, 0x710 + 4 * i));
        f.fcomp_m(f32::from_bits(old[i as usize]));
        if f.jp(5) {
            continue;
        }
        wheels += 1;
        wheel_contact(this, e, i, this.byte_add(0x660 + 0x2C * i), 0x7C0, 0x7D0);
    }
    collision_tail(this, e, n, wheels)
}

/// The global line col point array `CAutomobile::ProcessEntityCollision` collides its suspension
/// lines into (0xC1BFF8, 0x2C each).
const AUTO_LINE_CPS: usize = 0xC1BFF8;
/// `g_surfaceInfos` (0xB79538)
const SURFACE_INFOS: usize = 0xB79538;
/// The rhino's (model 0x1B0) eight extra line ratios (+0x618.., two of every three dwords).
const RHINO_RATIOS: [isize; 8] = [0x618, 0x61C, 0x624, 0x628, 0x630, 0x634, 0x63C, 0x640];

/// `CAutomobile::ProcessEntityCollision(CEntity* entity, CColPoint* colPoints)` (0x6ACE70, `ret 8`,
/// EAX = number of points).
///
/// Not simple (status != 2): `bVehicleColProcessed`. The col data's line count (+6, i8) is saved;
/// the line ratios are copied into a local array of 12 (wheel compressions +0x7D4.. and, for the
/// rhino (model 0x1B0), the eight extra ratios), plus an untouched copy. Physical flags 0x9000, a
/// ped, or model -2 against a vehicle zero the line count. Against the towing pair (+0x4C4 / +0x4C8
/// == entity) both col datas' box counts (+4, i16) are zeroed over the call and restored. Status 12:
/// no points; else `ProcessColModels(this, entity, colPoints, line col points 0xC1BFF8, ratios,
/// false)`. A zero line count is restored to the saved one (no wheel pass). Otherwise each wheel
/// with `ratio < 1.0` and `< +0x7D4[i]` takes the ratio, its col point is copied to +0x724
/// (`CColPoint::operator=`) and the ground bookkeeping runs (ground entities +0x8E8, offsets +0x8F8);
/// the rhino keeps the smaller of new and old for its extra ratios. The forklift (model 0x212) with
/// points against an object, both fork words +0x86C/+0x86E zero: points with surface A 0x41 are
/// removed (later points moved down). Then the collision records; with wheel contacts on a building
/// and handling +0xB4 > 0, each wheel whose `(min(old, 1 - (1 - +0x878/+0x888) * (1 - +0x7E4)) -
/// +0x7D4) * +0x888 > 0.1` (0x8D3200), not burst (`GetWheelStatus`), not on adhesion group 4 nor
/// surface 0xB2 appends its col point (pieces 0xD/0xF/0xE/0x10, surfaces 0x3C) with depth `|normal .
/// up| * min(|speed . fwd|, 0.3) * min(that - G_C1C214, 0.2) * 3.3333333 * +0xB4`; at 31 points the
/// last slot is overwritten.
#[hook(addr = 0x6ace70, conv = thiscall, name = "CAutomobile::ProcessEntityCollision", expect = "81 EC 88 00 00 00 53 55",
       this = "CAutomobile", unverified = "mvpf-veh-1", route = "L2", hazards = "heap-writes", recs = "driving",
       note = "writes the global line col points 0xC1BFF8 (through ProcessColModels)")]
pub fn automobile_process_entity_collision(this: Ent, e: Ent, cps: Ent) -> i32 {
    if rb(this, TYPE) & 0xF8 != 0x10 {
        wb(this, 0x42B, rb(this, 0x42B) | 1);
    }
    let cm = get_col_model(this);
    let cd = rp(cm, 0x2C);
    let saved_lines = rb(cd, 6);
    let mut ratios = [0u32; 12];
    let mut old = [0u32; 12];
    for i in 0..4 {
        ratios[i] = ru(this, 0x7D4 + 4 * i as isize);
        old[i] = ratios[i];
    }
    let model = rh(this, MODEL_INDEX);
    let mut wheels: i32 = 0;
    if model == 0x1B0 {
        for (j, off) in RHINO_RATIOS.iter().enumerate() {
            ratios[4 + j] = ru(this, *off);
            old[4 + j] = ratios[4 + j];
        }
    }
    let et = etype(e);
    if ru(this, PFLAGS) & 0x9000 != 0 || et == ty::PED || (model == 0xFFFE && et == ty::VEHICLE) {
        wb(cd, 6, 0);
    }
    let mut own_boxes: i32 = -1;
    let mut other_boxes: i32 = -1;
    if rp(this, 0x4C4) == e || rp(this, 0x4C8) == e {
        own_boxes = i32::from(rh(cd, 4) as i16);
        wh(cd, 4, 0);
        other_boxes = i32::from(rh(rp(get_col_model(e), 0x2C), 4) as i16);
        wh(rp(get_col_model(e), 0x2C), 4, 0);
    }
    let mut n = if rb(this, TYPE) & 0xF8 == 0x60 {
        0
    } else {
        ensure_matrix(e);
        let em = matrix(e);
        let tm = matrix(this);
        let ecm = get_col_model(e);
        process_col_models(tm, cm, em, ecm, cps, GPtr::from_addr(AUTO_LINE_CPS), lp(&mut ratios).cast(), 0)
    };
    if other_boxes >= 0 {
        wh(rp(get_col_model(e), 0x2C), 4, other_boxes as u16);
    }
    if own_boxes >= 0 {
        wh(cd, 4, own_boxes as u16);
    }
    if rb(cd, 6) == 0 {
        wb(cd, 6, saved_lines);
    } else {
        for i in 0..4usize {
            let mut f = Fpu::new();
            let r = f32::from_bits(ratios[i]);
            f.fld(r);
            f.fcomp_m(cf(0x3F80_0000));
            if f.jp(5) {
                continue;
            }
            f.fld(r);
            f.fcomp_m(rf(this, 0x7D4 + 4 * i as isize));
            if f.jp(5) {
                continue;
            }
            wu(this, 0x7D4 + 4 * i as isize, ratios[i]);
            let wcp = this.byte_add(0x724 + 0x2C * i as isize);
            col_point_assign(wcp, GPtr::from_addr(AUTO_LINE_CPS + 0x2C * i));
            wheels += 1;
            wheel_contact(this, e, i as isize, wcp, 0x8E8, 0x8F8);
        }
        if rh(this, MODEL_INDEX) == 0x1B0 {
            for (j, off) in RHINO_RATIOS.iter().enumerate() {
                let mut f = Fpu::new();
                f.fld(f32::from_bits(ratios[4 + j]));
                f.fcomp_m(f32::from_bits(old[4 + j]));
                let v = if f.jp(5) { old[4 + j] } else { ratios[4 + j] };
                wu(this, *off, v);
            }
        }
    }
    if rh(this, MODEL_INDEX) == 0x212
        && n > 0
        && etype(e) == ty::OBJECT
        && rh(this, 0x86C) == 0
        && rh(this, 0x86E) == 0
    {
        let mut i: i32 = 0;
        let mut last = n - 1;
        while i < n {
            let cp = cps.byte_add(0x2C * i as isize);
            if rb(cp, 0x20) == 0x41 {
                if i < last {
                    for k in i..last {
                        let d = cps.byte_add(0x2C * k as isize);
                        col_point_assign(d, d.byte_add(0x2C));
                    }
                    i -= 1;
                }
                n -= 1;
                last -= 1;
            }
            i += 1;
        }
    }
    if n <= 0 && wheels <= 0 {
        return n;
    }
    collision_tail(this, e, n, wheels);
    if wheels <= 0 || etype(e) != ty::BUILDING {
        return n;
    }
    let h = rp(this, 0x384);
    let mut f = Fpu::new();
    f.fld(rf(h, 0xB4));
    f.fcomp_m(0.0);
    if f.jne(0x41) {
        return n;
    }
    let mut slot = cps.byte_add(0x2C * n as isize);
    for i in 0..4isize {
        let mut f = Fpu::new();
        f.fld(rf(this, 0x878 + 4 * i));
        f.fdiv_m(rf(this, 0x888 + 4 * i));
        f.fsubr_m(cf(0x3F80_0000));
        f.fld(cf(0x3F80_0000));
        f.fsub_st(1);
        f.fmul_m(rf(this, 0x7E4 + 4 * i));
        f.fadd_st(1);
        let mut t = f.fstp();
        f.fstp_st(0);
        f.fld(f32::from_bits(old[i as usize]));
        f.fcomp_m(t);
        if !f.jp(5) {
            t = f32::from_bits(old[i as usize]);
        }
        f.fld(t);
        f.fsub_m(rf(this, 0x7D4 + 4 * i));
        f.fmul_m(rf(this, 0x888 + 4 * i));
        f.fcomp_m(GPtr::<f32>::from_addr(0x8D3200).read());
        if f.jne(0x41) {
            continue;
        }
        if get_wheel_status(this.byte_add(0x5A0), i as i32) != 0 {
            continue;
        }
        let wcp = this.byte_add(0x724 + 0x2C * i);
        let surf = u32::from(rb(wcp, 0x23));
        if get_adhesion_group(GPtr::from_addr(SURFACE_INFOS), surf) == 4 {
            continue;
        }
        if rb(wcp, 0x23) == 0xB2 {
            continue;
        }
        col_point_assign(slot, wcp);
        f.fld(t);
        f.fsub_m(rf(this, 0x7D4 + 4 * i));
        f.fmul_m(rf(this, 0x888 + 4 * i));
        f.fsub_m(GPtr::<f32>::from_addr(0xC1C214).read());
        let lim = GPtr::<f32>::from_addr(0x8D31FC).read();
        f.fld(lim);
        f.fcomp_st(1);
        if !f.jp(5) {
            f.fstp_st(0);
            f.fld(lim);
        }
        let m = matrix(this);
        let speed = |f: &mut Fpu| {
            f.fld(rf(this, MOVE_SPEED + 8));
            f.fmul_m(rf(m, 0x18));
            f.fld(rf(this, MOVE_SPEED + 4));
            f.fmul_m(rf(m, 0x14));
            f.faddp(1);
            f.fld(rf(this, MOVE_SPEED));
            f.fmul_m(rf(m, 0x10));
            f.faddp(1);
            f.fabs();
        };
        speed(&mut f);
        f.fcomp_m(cf(0x3E99_999A));
        if !f.jne(0x41) {
            f.fld(cf(0x3E99_999A));
        } else {
            speed(&mut f);
        }
        f.fld(rf(slot, 0x18));
        f.fmul_m(rf(m, 0x28));
        f.fld(rf(slot, 0x10));
        f.fmul_m(rf(m, 0x20));
        f.faddp(1);
        f.fld(rf(m, 0x24));
        f.fmul_m(rf(slot, 0x14));
        f.faddp(1);
        f.fabs();
        f.fmul_st(1);
        f.fmul_st(2);
        f.fmul_m(cf(0x4055_5555));
        f.fmul_m(rf(rp(this, 0x384), 0xB4));
        wf(slot, 0x28, f.fstp());
        f.fstp_st(0);
        f.fstp_st(0);
        let piece: u8 = match i {
            0 => 0xD,
            1 => 0xF,
            2 => 0xE,
            _ => 0x10,
        };
        wb(slot, 0x21, piece);
        wb(slot, 0x20, 0x3C);
        wb(slot, 0x23, 0x3C);
        if n < 0x1F {
            n += 1;
            slot = slot.byte_add(0x2C);
        }
    }
    n
}

/// The tail shared by every path of `CBmx::ProcessDrivingAnims` (0x6C0371): `+0x824 = -st0`,
/// `+0x828 = -(+0x820)`.
fn bmx_finish(this: Ent, f: &mut Fpu) {
    f.fchs();
    wf(this, 0x824, f.fstp());
    f.fld(rf(this, 0x820));
    f.fchs();
    wf(this, 0x828, f.fstp());
}

/// `a` missing, or (blend amount < 1 and delta <= 0): the anim has to be blended in.
fn bmx_needs_blend(a: Ent) -> bool {
    if a.is_null() {
        return true;
    }
    let mut f = Fpu::new();
    f.fld(rf(a, BLEND_AMOUNT));
    f.fcomp_m(cf(0x3F80_0000));
    if f.jp(5) {
        return false;
    }
    f.fld(rf(a, BLEND_DELTA));
    f.fcomp_m(0.0);
    !f.jp(0x41)
}

/// `BlendAnimation(clump, +0x640, id, 4.0)` for the bmx rider.
fn bmx_blend(this: Ent, ped: Ent, id: u32) -> Ent {
    blend_animation(rp(ped, RW_OBJECT), ru(this, 0x640), id, 4.0)
}

/// The first of `assocs` with blend amount above `min` picks its constant's address.
fn bmx_pick(assocs: [(Ent, usize); 3], min: u32) -> Option<usize> {
    for (a, k) in assocs {
        if a.is_null() {
            continue;
        }
        let mut f = Fpu::new();
        f.fld(rf(a, BLEND_AMOUNT));
        f.fcomp_m(cf(min));
        if !f.jne(0x41) {
            return Some(k);
        }
    }
    None
}

/// `+0x820 = [0x871500] (0.0) - a.currentTime / a.hier.totalLength * 2pi` (0x858CBC).
fn bmx_phase_from(this: Ent, a: Ent) {
    let mut f = Fpu::new();
    f.fld(rf(a, 0x20));
    f.fdiv_m(rf(rp(a, 0x14), 0x10));
    f.fmul_m(cf(0x40C9_0FDB));
    f.fsubr_m(GPtr::<f32>::from_addr(0x871500).read());
    wf(this, 0x820, f.fstp());
}

/// `+0x654 *= 0.95; +0x658 *= 0.95` (0x8714E8).
fn bmx_damp(this: Ent) {
    let k = cf(0x3F73_3333);
    let mut f = Fpu::new();
    f.fld(k);
    f.fmul_m(rf(this, 0x654));
    wf(this, 0x654, f.fstp());
    f.fld(k);
    f.fmul_m(rf(this, 0x658));
    wf(this, 0x658, f.fstp());
}

/// `fld [this+off]; fcomp 0; test ah, 0x41; jp`: above zero or NaN.
fn above_zero(this: Ent, off: isize) -> bool {
    let mut f = Fpu::new();
    f.fld(rf(this, off));
    f.fcomp_m(0.0);
    f.jp(0x41)
}

/// `fld v; fcomp 0; test ah, 0x41; je`: strictly above zero.
fn strictly_above_zero(v: f32) -> bool {
    let mut f = Fpu::new();
    f.fld(v);
    f.fcomp_m(0.0);
    !f.jne(0x41)
}

/// `CBmx::ProcessDrivingAnims(CPed* rider, bool)` (0x6BFB50, `ret 8`): the bmx rider's pedalling,
/// sprint and coasting anims and the pedal phase (+0x820, mirrored negated into +0x824/+0x828).
///
/// Entity flag 0x20000 with no or a non-player rider: nothing. Flags +0x7B4/+0x7B5 = 1. The rider's
/// association 0xCE: `+0x820 = (1 - b) * +0x820 + pi/2 * b`, done. A rider with +0x480 gets
/// `SetMoveState(0)`. `V = speed . forward`; associations 0xCF (pedal), 0xD0 (sprint), 0xC4, 0xC5,
/// 0xC7 and the first of 0xCA/0xCB/0xCC (lean, kept in the `rider` argument slot).
///
/// Pedalling (`|+0x648| < (+0x818 > 5 ? 0.7 : 0.4)`, that threshold `> +0x650`, `V > 0.01`, no lean):
/// the pedal speed (model 0x1FE: `5V / (gear(+0x4B4) * handling +0x88 - 0.25)`, gear 0: 0 and 2.0;
/// else `3V` and 2.5) in the argument slot. `+0x818 > 5` and speed below that limit: sprint (0xD0
/// blended in when needed, flag bit 0 of +0x2E set on it and on the pedal anim, the speed into the
/// pedal anim (else the sprint anim, which becomes the anim)). Else with steer +0x49C zero, +0x818
/// <= 0 and status != 2: coast (0xCF blended in when needed, flag bit 0 cleared; unless +0x428 bit
/// 5 a wheel ratio +0x720..+0x72C below 1 sets +0x834); else pedal (0xCF, flag bit 0 set, speed).
/// No anim: `+0x820 *= 0.97^timeStep` (`_CIpow`). A freshly blended anim with one of 0xC4/0xC5/0xC7
/// above 0.5 starts at that phase (pi, 0, pi/2; unless <= -1000): time = len * frac(-phase / 2pi
/// (0.15915494)), `SetCurrentTime`, +0x820 = the fraction; otherwise +0x820 from the anim's current
/// time. Then `|+0x654| > 0.05 || |+0x658| > 0.05` damps both by 0.95.
///
/// Not pedalling: pedal or sprint anim still fading in (delta >= 0, amount > 0) fades both out
/// (delta -8, flag bit 0 cleared) and damps; else `CBike::ProcessRiderAnims(rider, this, +0x640,
/// +0x63C, 0)`. +0x820 from the pedal (else sprint) anim's time or 0, then blended toward pi / 0 /
/// pi/2 by the 0xC4/0xC5/0xC7 amount above 0.1 (the first two set +0x834), else `*= 0.97^timeStep`;
/// a lean clears +0x7B5 and sets +0x834.
///
/// Player rider: +0x614 bit 7 recomputed from the wheel ratios +0x730..+0x73C, forward z and the
/// bike handling (+0x63C) wheelie / stoppie heights (+0x2C * 0.5, +0x30 * 0.6).
#[hook(addr = 0x6bfb50, conv = thiscall, name = "CBmx::ProcessDrivingAnims", expect = "83 EC 10 55 8B 6C 24 18",
       this = "CBmx", unverified = "mvpf-veh-1", route = "L2", hazards = "stack-residue", recs = "driving",
       note = "the rider argument slot (S+4) is reused: lean association, then the pedal speed / phase f32; `_CIpow` 0x822130 via x87_binary")]
pub fn bmx_process_driving_anims(this: Ent, ped: Ent, _b: u8) {
    if ru(this, EFLAGS) & 0x20000 != 0 && (ped.is_null() || ped_is_player(ped) == 0) {
        return;
    }
    wb(this, 0x7B4, 1);
    wb(this, 0x7B5, 1);
    let mut f = Fpu::new();
    let a = clump_get_association(rp(ped, RW_OBJECT), 0xCE);
    if !a.is_null() {
        f.fld(cf(0x3F80_0000));
        f.fsub_m(rf(a, BLEND_AMOUNT));
        f.fmul_m(rf(this, 0x820));
        f.fld(cf(0x3FC9_0FDB));
        f.fmul_m(rf(a, BLEND_AMOUNT));
        f.faddp(1);
        wf(this, 0x820, f.fst());
        bmx_finish(this, &mut f);
        return;
    }
    if ru(ped, 0x480) != 0 {
        ped_set_move_state(ped, 0);
    }
    let m = matrix(this);
    f.fld(rf(this, MOVE_SPEED + 8));
    f.fmul_m(rf(m, 0x18));
    f.fld(rf(this, MOVE_SPEED + 4));
    f.fmul_m(rf(m, 0x14));
    f.faddp(1);
    f.fld(rf(m, 0x10));
    f.fmul_m(rf(this, MOVE_SPEED));
    f.faddp(1);
    let v = f.fstp();
    let clump = || rp(ped, RW_OBJECT);
    let mut edi = clump_get_association(clump(), 0xCF);
    let mut ebx = clump_get_association(clump(), 0xD0);
    let a_c4 = clump_get_association(clump(), 0xC4);
    let a_c5 = clump_get_association(clump(), 0xC5);
    let a_c7 = clump_get_association(clump(), 0xC7);
    let mut lean = clump_get_association(clump(), 0xCA);
    if lean.is_null() {
        lean = clump_get_association(clump(), 0xCB);
        if lean.is_null() {
            lean = clump_get_association(clump(), 0xCC);
        }
    }
    f.fld(rf(this, 0x818));
    f.fcomp_m(cf(0x40A0_0000));
    if !f.jne(0x41) {
        f.fld(cf(0x3F33_3333));
    } else {
        f.fld(cf(0x3ECC_CCCD));
    }
    f.fld(rf(this, 0x648));
    f.fabs();
    f.fcomp_st(1);
    let pedal = if f.jp(5) {
        f.fstp_st(0);
        false
    } else {
        f.fcomp_m(rf(this, 0x650));
        if f.jne(0x41) {
            false
        } else {
            f.fld(v);
            f.fcomp_m(cf(0x3C23_D70A));
            !f.jne(0x41) && lean.is_null()
        }
    };
    if pedal {
        let l24: u32;
        if rh(this, MODEL_INDEX) == 0x1FE {
            let gear = rb(this, 0x4B4);
            if gear < 1 {
                f.fld(cf(0x4000_0000));
                l24 = 0;
            } else {
                f.fld(cf(0x40A0_0000));
                f.fmul_m(v);
                f.fild(i32::from(gear));
                f.fmul_m(rf(rp(this, 0x384), 0x88));
                f.fsub_m(cf(0x3E80_0000));
                f.fdivp(1);
                l24 = f.fstp().to_bits();
                f.fld(cf(0x4000_0000));
            }
        } else {
            f.fld(cf(0x4040_0000));
            f.fmul_m(v);
            l24 = f.fstp().to_bits();
            f.fld(cf(0x4020_0000));
        }
        let mut fresh = false;
        f.fld(rf(this, 0x818));
        f.fcomp_m(cf(0x40A0_0000));
        let sprint = if f.jne(0x41) {
            f.fstp_st(0);
            false
        } else {
            f.fld(f32::from_bits(l24));
            f.fcomp_st(1);
            f.fstp_st(0);
            !f.jp(5)
        };
        if sprint {
            if bmx_needs_blend(ebx) {
                ebx = bmx_blend(this, ped, 0xD0);
                fresh = true;
            }
            wh(ebx, 0x2E, rh(ebx, 0x2E) | 1);
            if !edi.is_null() {
                wh(edi, 0x2E, rh(edi, 0x2E) | 1);
                wu(edi, 0x24, l24);
            } else {
                wu(ebx, 0x24, l24);
                edi = ebx;
            }
        } else {
            f.fld(rf(this, 0x49C));
            f.fcomp_m(0.0);
            let coast = !f.jp(0x44) && !strictly_above_zero(rf(this, 0x818)) && rb(this, TYPE) & 0xF8 != 0x10;
            if coast {
                if bmx_needs_blend(edi) {
                    edi = bmx_blend(this, ped, 0xCF);
                    fresh = true;
                }
                wb(edi, 0x2E, rb(edi, 0x2E) & 0xFE);
                if rb(this, 0x428) & 0x20 == 0 {
                    for off in [0x720, 0x724, 0x728, 0x72C] {
                        f.fld(rf(this, off));
                        f.fcomp_m(cf(0x3F80_0000));
                        if !f.jp(5) {
                            wb(this, 0x834, 1);
                            break;
                        }
                    }
                }
            } else {
                if bmx_needs_blend(edi) {
                    edi = bmx_blend(this, ped, 0xCF);
                    fresh = true;
                }
                wb(edi, 0x2E, rb(edi, 0x2E) | 1);
                wu(edi, 0x24, l24);
            }
        }
        if edi.is_null() {
            let p = ci_pow(X87::from_f32(cf(0x3F78_51EC)), X87::from_f32(sa_core::timer::TIME_STEP.get()));
            f.push(p);
            f.fmul_m(rf(this, 0x820));
            wf(this, 0x820, f.fstp());
        } else {
            let start = if fresh {
                bmx_pick([(a_c4, 0x871504), (a_c5, 0x871508), (a_c7, 0x87150C)], 0x3F00_0000)
            } else {
                None
            };
            let mut timed = false;
            if let Some(k) = start {
                f.fld(GPtr::<f32>::from_addr(k).read());
                f.fcom_m(cf(0xC47A_0000));
                if f.jne(0x41) {
                    f.fstp_st(0);
                } else {
                    f.fld(GPtr::<f32>::from_addr(0x871500).read());
                    f.fsub_st(1);
                    f.fmul_m(cf(0x3E22_F983));
                    let mut t = f.fstp();
                    f.fstp_st(0);
                    f.fld(t);
                    f.fcomp_m(0.0);
                    if !f.jp(5) {
                        f.fld(t);
                        f.fadd_m(cf(0x3F80_0000));
                        t = f.fstp();
                    }
                    f.fld(rf(rp(edi, 0x14), 0x10));
                    f.fmul_m(t);
                    assoc_set_current_time(edi, f.fstp());
                    wf(this, 0x820, t);
                    timed = true;
                }
            }
            if !timed {
                bmx_phase_from(this, edi);
            }
        }
        f.fld(rf(this, 0x654));
        f.fabs();
        f.fcomp_m(cf(0x3D4C_CCCD));
        let damp = !f.jne(0x41) || {
            f.fld(rf(this, 0x658));
            f.fabs();
            f.fcomp_m(cf(0x3D4C_CCCD));
            !f.jne(0x41)
        };
        if damp {
            bmx_damp(this);
        }
    } else {
        let fading = |a: Ent| -> bool {
            if a.is_null() {
                return false;
            }
            let mut f = Fpu::new();
            f.fld(rf(a, BLEND_DELTA));
            f.fcomp_m(0.0);
            if f.jne(1) {
                return false;
            }
            f.fld(rf(a, BLEND_AMOUNT));
            f.fcomp_m(0.0);
            !f.jne(0x41)
        };
        if fading(edi) || fading(ebx) {
            for a in [edi, ebx] {
                if !a.is_null() {
                    wh(a, 0x2E, rh(a, 0x2E) & 0xFFFE);
                    wu(a, BLEND_DELTA, 0xC100_0000);
                }
            }
            bmx_damp(this);
        } else {
            process_rider_anims(ped, this, this.byte_add(0x640), ru(this, 0x63C), 0);
        }
        let src = if !edi.is_null() { edi } else { ebx };
        if src.is_null() {
            wu(this, 0x820, 0);
        } else {
            bmx_phase_from(this, src);
        }
        let mut blended = false;
        for (a, k, flag) in [(a_c4, 0x871504usize, true), (a_c5, 0x871508, true), (a_c7, 0x87150C, false)] {
            if a.is_null() {
                continue;
            }
            f.fld(rf(a, BLEND_AMOUNT));
            f.fcomp_m(cf(0x3DCC_CCCD));
            if f.jne(0x41) {
                continue;
            }
            f.fld(cf(0x3F80_0000));
            f.fsub_m(rf(a, BLEND_AMOUNT));
            f.fmul_m(rf(this, 0x820));
            f.fld(GPtr::<f32>::from_addr(k).read());
            f.fmul_m(rf(a, BLEND_AMOUNT));
            if flag {
                wb(this, 0x834, 1);
            }
            f.faddp(1);
            blended = true;
            break;
        }
        if !blended {
            let p = ci_pow(X87::from_f32(cf(0x3F78_51EC)), X87::from_f32(sa_core::timer::TIME_STEP.get()));
            f.push(p);
            f.fmul_m(rf(this, 0x820));
        }
        wf(this, 0x820, f.fstp());
        if !lean.is_null() {
            wb(this, 0x7B5, 0);
            wb(this, 0x834, 1);
        }
    }
    if ped_is_player(ped) != 0 {
        let dl = rb(this, 0x614) & 0x7F;
        wb(this, 0x614, dl);
        let fz = rf(matrix(this), 0x18);
        // 0x6C0258: wheelie test, falling through to the stoppie test at 0x6C02E5
        let mut wheelie = false;
        if !above_zero(this, 0x730) && !above_zero(this, 0x734) && strictly_above_zero(fz) {
            wheelie = strictly_above_zero(rf(this, 0x738)) || strictly_above_zero(rf(this, 0x73C));
        }
        let set = if wheelie {
            let h = rp(this, 0x63C);
            f.fld(rf(h, 0x2C));
            f.fsub_m(fz);
            f.fld(rf(h, 0x2C));
            f.fmul_m(cf(0x3F00_0000));
            f.fcompp();
            !f.jne(0x41)
        } else if !above_zero(this, 0x738) && !above_zero(this, 0x73C) && {
            f.fld(fz);
            f.fcomp_m(0.0);
            !f.jp(5)
        } && (strictly_above_zero(rf(this, 0x730)) || strictly_above_zero(rf(this, 0x734)))
        {
            let h = rp(this, 0x63C);
            f.fld(rf(h, 0x30));
            f.fsub_m(fz);
            f.fld(rf(h, 0x30));
            f.fmul_m(cf(0x3F19_999A));
            f.fcompp();
            !f.jp(5)
        } else {
            false
        };
        if set {
            wb(this, 0x614, dl | 0x80);
        }
    }
    f.fld(rf(this, 0x820));
    bmx_finish(this, &mut f);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_VEHICLE_PROCESS_DRIVING_ANIMS,
    HOOK_BIKE_PLACE_ON_ROAD_PROPERLY,
    HOOK_AUTOMOBILE_PLACE_ON_ROAD_PROPERLY,
    HOOK_BIKE_PROCESS_ENTITY_COLLISION,
    HOOK_AUTOMOBILE_PROCESS_ENTITY_COLLISION,
    HOOK_BMX_PROCESS_DRIVING_ANIMS,
];
