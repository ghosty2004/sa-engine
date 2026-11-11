//! WRITER-MVP-E batch `mvpe-phys-1`: the `CPhysical` collision driver (`ProcessCollision`) and the
//! sector-list passes it and `ProcessShift` run (`ProcessShiftSectorList`, ...). Ported from the 1.0 US
//! bytes (SecuROM fragments inlined), x87 sequences through the [`Fpu`] model (PC = 24) in the
//! original's operand order, values the original keeps on the x87 stack across loop iterations kept
//! on the model's stack; gta-reversed `Entity/Physical.cpp` was a reading aid only. Unverified
//! (docs/verify/HOWTO.md).
//!
//! Sectors: `CWorld::ms_aSectors` 0xB7D0B8 (120 x 120, 8 bytes: buildings list +0, dummies +4),
//! `CWorld::ms_aRepeatSectors` 0xB992B8 (16 x 16, 12 bytes: vehicles +0, peds +4, objects +8);
//! `CPtrNode` item +0, next +4. `CColPoint` 0x2C bytes (normal +0x10, piece B +0x24, depth +0x28).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::attach::{
    apply_speed, check_collision_call, matrix_assign, matrix_copy_ctor, matrix_dtor, matrix_mul_point,
    matrix_reorthogonalise, ped_is_player, remove_and_add_call, LocalMatrix,
};
use super::shared::*;
use crate::collision::fpu::Fpu;

/// `CWorld::ms_aSectors` (0xB7D0B8)
pub const SECTORS: usize = 0xB7D0B8;
/// `CWorld::ms_aRepeatSectors` (0xB992B8)
pub const REPEAT_SECTORS: usize = 0xB992B8;
/// `CWorld::ms_nCurrentScanCode` (0xB7CD78, uint16)
pub const SCAN_CODE: usize = 0xB7CD78;
/// Size of a `CColPoint`.
pub const COL_POINT_SIZE: usize = 0x2C;
/// The original's local `CColPoint` array (32 entries, the `CCollision` maximum).
pub type LocalColPoints = [u32; 32 * COL_POINT_SIZE / 4];

orig! {
    /// `CEntity::GetBoundCentre(CVector&)` (0x534290)
    pub fn get_bound_centre_ref(e: Ent, out: Ent) -> Ent = thiscall 0x534290 => crate::world::entity::get_bound_centre_ref;
    /// `CEntity::GetIsTouching(const CVector&, float)` (0x5344B0, EAX 0/1)
    pub fn get_is_touching_vec(e: Ent, pos: Ent, radius: f32) -> u32 = thiscall 0x5344B0 => crate::world::entity::get_is_touching_vec;
    /// `CEntity::DoesNotCollideWithFlyers()` (0x532D40)
    pub fn does_not_collide_with_flyers(e: Ent) -> u8 = thiscall 0x532D40 => crate::world::entity::does_not_collide_with_flyers;
    /// virtual `CPhysical::SpecialEntityPreCollisionStuff(CPhysical*, bool, bool&, bool&, bool&, bool&)`
    /// (vtable +0x3C)
    pub fn special_entity_pre_collision_stuff_v(e: Ent, other: Ent, b: u32, p1: Ent, p2: Ent, p3: Ent, p4: Ent)
        = thiscall vslot 0x3C => unported;
    /// virtual `CPhysical::ProcessEntityCollision(CEntity*, CColPoint*)` (vtable +0x58, EAX count)
    pub fn process_entity_collision_v(e: Ent, other: Ent, cps: Ent) -> i32 = thiscall vslot 0x58 => unported;
}

/// `x <= 0 ? 0 : x`, then capped at 0x77 (`setle; dec; and` + `cmp 0x77; jl`).
fn clamp_sector(v: i32) -> i32 {
    let c = if v <= 0 { 0 } else { v };
    if c < 0x77 { c } else { 0x77 }
}

/// `CPhysical::ProcessShiftSectorList(int32 x, int32 y)` (0x546670, `ret 8`, AL).
///
/// Bound centre (`GetBoundCentre`) and the model's col-model radius (+0x24). The four lists are
/// walked in the switch order of the bytes (jump table 0x546CF0, counter 4 down to 1): repeat sector
/// `[(y&15)*16 + (x&15)]` objects (+8), peds (+4), vehicles (+0), then sector `[clamp(y)*120 +
/// clamp(x)]` buildings (x and y clamped to 0..=0x77). Per entity: `stat` = building, object with
/// physical flag 4, or (this a ped, it an object with entity flag 4 or 0x40000 and no object flag
/// +0x140 bit 0x40). Skipped: itself, current scan code, `!bUsesCollision`, and (this has entity
/// flag 0x1000 and `!stat`). `GetIsTouching(centre, radius)`; a building: skip-test flag from this'
/// physical flag 4 (unless a non-train vehicle), the attached-to entity (vehicle/ped/object with
/// physical flag 4), the ignored-collision entity (+0x128), flag 0x80 without 2, or flag 0x1000000
/// with status != 0, 8 and `DoesNotCollideWithFlyers`; others: virtual
/// `SpecialEntityPreCollisionStuff(it, true, ...)`. A ped sets physical flag 0x1000. Unless skipped:
/// scan code, virtual `ProcessEntityCollision`; each point with depth > 0 and piece B outside 13..=16
/// counts and adds its normal to the shift (a vehicle hit by a ped: only normals with z < 0, the z
/// added as `n.z*0`; a ped against an object with physical flag 0x20: only `|n.z| <= 0.1`) and raises
/// the maximum depth. The shift sum stays on the x87 stack (st0 y, st1 x, st2 z) across entities; it
/// is stored to the locals after each processed entity and reloaded from them after a call.
///
/// No point: false. Otherwise the sum is normalised when longer than 1; `z < -0.5`: shift `n * depth
/// * 0.75` (G_8CD7D8); a ped: `n * clamp(1.5*depth, 0.005, 0.3)` (G_8CD7D4); else `n * depth * 1.5`.
/// The position moves by the shift (matrix or placement). A vehicle with G_8CD7D0 > 0 adds `(x, y,
/// max(z, 0)) * 0.008 * timeStep` to its move speed (x and y re-read from the f32 locals when z < 0).
/// True.
#[hook(addr = 0x546670, conv = thiscall, name = "CPhysical::ProcessShiftSectorList", expect = "81 EC D4 05 00 00 53 55",
       this = "CPhysical", unverified = "mvpe-phys-1", route = "L2", hazards = "none", recs = "grove-walk,driving")]
pub fn process_shift_sector_list(this: Ent, x: i32, y: i32) -> u8 {
    let model = rh(this, MODEL_INDEX) as i16 as i32;
    let mi = sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(model).read();
    let radius = rf(rp(mi, 0x14), 0x24);
    let mut centre = [0f32; 3];
    get_bound_centre_ref(this, lp(&mut centre).cast());
    let xs = clamp_sector(x);
    let ys = clamp_sector(y);
    let sector = GPtr::<u8>::from_addr(SECTORS + 8 * (ys * 120 + xs) as usize);
    let rep_idx = (((y & 0xF) << 4) + (x & 0xF)) as usize;
    let repeat = GPtr::<u8>::from_addr(REPEAT_SECTORS + 12 * rep_idx);
    // the f32 locals of the shift sum (esp+0x30 x, +0x34 y, +0x38 z), the count and the depth
    let (mut mx, mut my, mut mz) = (0f32, 0f32, 0f32);
    let mut count: u32 = 0;
    let mut max_depth = 0f32;
    let mut f = Fpu::new();
    f.fld(mz);
    f.fld(mx);
    f.fld(my);
    let mut cps: LocalColPoints = [0; 32 * COL_POINT_SIZE / 4];
    let cpp = lp(&mut cps).cast::<u8>();
    for list in [repeat.byte_add(8), repeat.byte_add(4), repeat, sector] {
        let mut node = rp(list, 0);
        while !node.is_null() {
            let e = rp(node, 0);
            node = rp(node, 4);
            let mut b12: u8 = 0;
            let mut b13: u8 = 0;
            let mut b2e: u8 = 0;
            let mut b2f: u8 = 0;
            let et = etype(e);
            let stat = if et == ty::BUILDING || (et == ty::OBJECT && rb(e, PFLAGS) & 4 != 0) {
                true
            } else if etype(this) == ty::PED && et == ty::OBJECT {
                let ef = ru(e, EFLAGS);
                if ef & 4 != 0 || ef & 0x40000 != 0 { rb(e, 0x140) & 0x40 == 0 } else { false }
            } else {
                false
            };
            if e == this
                || rh(e, 0x2C) == GPtr::<u16>::from_addr(SCAN_CODE).read()
                || rb(e, EFLAGS) & 1 == 0
                || ((ru(this, EFLAGS) >> 12) & 1 != 0 && !stat)
            {
                continue;
            }
            f.fstp_st(0);
            f.fstp_st(0);
            f.fstp_st(0);
            let mut processed = false;
            if get_is_touching_vec(e, lp(&mut centre).cast(), radius) as u8 != 0 {
                if etype(e) == ty::BUILDING {
                    let pf = ru(this, PFLAGS);
                    b13 = 0;
                    if pf & 4 != 0 && (etype(this) != ty::VEHICLE || ru(this, 0x594) == 6) {
                        b12 = 1;
                    } else {
                        let at = rp(this, ATTACHED_TO);
                        let at_t = if at.is_null() { 0 } else { etype(at) };
                        if (!at.is_null() && at_t > 1 && at_t < 5 && rb(at, PFLAGS) & 4 != 0)
                            || rp(this, IGNORED_COLLISION) == e
                            || (pf & 0x80 != 0 && pf & 2 == 0)
                        {
                            b12 = 1;
                        } else if pf & 0x0100_0000 != 0 {
                            let st = rb(this, TYPE) >> 3;
                            if st != 0 && st != 8 && does_not_collide_with_flyers(e) != 0 {
                                b12 = 1;
                            }
                        }
                    }
                } else {
                    special_entity_pre_collision_stuff_v(
                        this,
                        e,
                        1,
                        lp(&mut b12),
                        lp(&mut b13),
                        lp(&mut b2e),
                        lp(&mut b2f),
                    );
                }
                if etype(this) == ty::PED {
                    wu(this, PFLAGS, ru(this, PFLAGS) | 0x1000);
                }
                if b13 == 0 && b12 == 0 {
                    wh(e, 0x2C, GPtr::<u16>::from_addr(SCAN_CODE).read());
                    let n = process_entity_collision_v(this, e, cpp);
                    if n > 0 {
                        processed = true;
                        f.fld(mz);
                        f.fld(mx);
                        f.fld(my);
                        for i in 0..n as usize {
                            let cp = cpp.byte_add((i * COL_POINT_SIZE) as isize);
                            let depth = rf(cp, 0x28);
                            f.fld(depth);
                            f.fcomp_m(cf(k::K_858B50));
                            if f.jne(0x41) {
                                continue;
                            }
                            let piece = rb(cp, 0x24);
                            if (0xD..=0x10).contains(&piece) {
                                continue;
                            }
                            count = count.wrapping_add(1);
                            let tt = etype(this);
                            let mut normal = true;
                            if tt == ty::VEHICLE && etype(e) == ty::PED {
                                f.fld(rf(cp, 0x18));
                                f.fcomp_m(cf(k::K_858B50));
                                if !f.jp(5) {
                                    // n.z < 0: x += n.x, y += n.y, z += n.z*0
                                    f.fld(rf(cp, 0x10));
                                    f.fld(rf(cp, 0x18));
                                    f.fmul_m(cf(k::K_858B50));
                                    let t = f.fstp();
                                    f.faddp(2);
                                    f.fadd_m(rf(cp, 0x14));
                                    f.fld(t);
                                    f.faddp(3);
                                    normal = false;
                                }
                            }
                            if normal {
                                if tt == ty::PED && etype(e) == ty::OBJECT && rb(e, PFLAGS) & 0x20 != 0 {
                                    f.fld(rf(cp, 0x18));
                                    f.fabs();
                                    f.fcomp_m(cf(k::K_858B1C));
                                    if !f.jne(0x41) {
                                        continue;
                                    }
                                }
                                f.fxch(1);
                                f.fadd_m(rf(cp, 0x10));
                                f.fxch(1);
                                f.fadd_m(rf(cp, 0x14));
                                f.fxch(2);
                                f.fadd_m(rf(cp, 0x18));
                                f.fxch(2);
                            }
                            f.fld(depth);
                            f.fcomp_m(max_depth);
                            if !f.jne(0x41) {
                                max_depth = depth;
                            }
                        }
                        // fst [esp+0x34] twice (y), the first store is dead
                        f.fxch(2);
                        mz = f.fst();
                        f.fxch(2);
                        my = f.fst();
                        f.fxch(1);
                        mx = f.fst();
                        f.fxch(1);
                    }
                }
            }
            if !processed {
                f.fld(mz);
                f.fld(mx);
                f.fld(my);
            }
        }
    }
    if count == 0 {
        return 0;
    }
    // st0 y, st1 x, st2 z
    f.fld_st(1);
    f.fmul_st(2);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(1);
    f.fsqrt();
    f.fcom_m(cf(k::K_858624));
    if !f.jne(0x41) {
        f.fdivr_m(cf(k::K_858624));
        f.fld_st(0);
        f.fmulp(3);
        f.fxch(2);
        mx = f.fst();
        f.fxch(2);
        f.fxch(1);
        f.fmul_st(1);
        my = f.fst();
        f.fxch(1);
        f.fmulp(3);
    } else {
        f.fstp_st(0);
    }
    f.fld_st(2);
    f.fcomp_m(cf(k::K_858F40));
    let shift: [f32; 3] = if !f.jp(5) {
        f.fld_st(1);
        f.fmul_m(max_depth);
        let t18 = f.fstp();
        f.fmul_m(max_depth);
        f.fld_st(2);
        f.fmul_m(max_depth);
        f.fld(t18);
        f.fmul_m(G_8CD7D8.get());
        let sx = f.fstp();
        f.fxch(1);
        f.fmul_m(G_8CD7D8.get());
        let sy = f.fstp();
        f.fmul_m(G_8CD7D8.get());
        let sz = f.fstp();
        [sx, sy, sz]
    } else if etype(this) == ty::PED {
        f.fld(G_8CD7D4.get());
        f.fmul_m(max_depth);
        let fd = f.fstp();
        let m;
        f.fld(cf(k::K_858B4C));
        f.fcomp_m(fd);
        if !f.jne(0x41) {
            f.fld(cf(k::K_858B4C));
            m = f.fstp();
        } else {
            f.fld(fd);
            f.fld(cf(k::K_858C24));
            f.fcomp_m(fd);
            if f.jp(5) {
                m = f.fstp();
            } else {
                f.fstp_st(0);
                m = cf(0x3E99_999A);
            }
        }
        f.fld(m);
        f.fmul_st(2);
        let sx = f.fstp();
        f.fmul_m(m);
        let sy = f.fstp();
        f.fld(m);
        f.fmul_st(2);
        let sz = f.fstp();
        [sx, sy, sz]
    } else {
        f.fld_st(1);
        f.fmul_m(max_depth);
        let t18 = f.fstp();
        f.fmul_m(max_depth);
        f.fld_st(2);
        f.fmul_m(max_depth);
        f.fld(t18);
        f.fmul_m(G_8CD7D4.get());
        let sx = f.fstp();
        f.fxch(1);
        f.fmul_m(G_8CD7D4.get());
        let sy = f.fstp();
        f.fmul_m(G_8CD7D4.get());
        let sz = f.fstp();
        [sx, sy, sz]
    };
    // stack: st0 x, st1 z
    let m = matrix(this);
    let pos = if m.is_null() { this.byte_add(PLACEMENT_POS) } else { m.byte_add(0x30) };
    f.fld(shift[0]);
    f.fadd_m(rf(pos, 0));
    f.fld(shift[1]);
    f.fadd_m(rf(pos, 4));
    let py = f.fstp();
    f.fld(shift[2]);
    f.fadd_m(rf(pos, 8));
    let pz = f.fstp();
    if !m.is_null() {
        wf(m, 0x30, f.fstp());
        wf(matrix(this), 0x34, py);
        wf(matrix(this), 0x38, pz);
    } else {
        wf(this, 4, f.fstp());
        wf(this, 8, py);
        wf(this, 0xC, pz);
    }
    if etype(this) != ty::VEHICLE {
        return 1;
    }
    f.fld(G_8CD7D0.get());
    f.fcomp_m(cf(k::K_858B50));
    if f.jne(0x41) {
        return 1;
    }
    f.fld_st(1);
    f.fcomp_m(cf(k::K_858B50));
    if !f.jp(5) {
        f.fstp_st(0);
        f.fstp_st(0);
        f.fld(cf(k::K_858B50));
        f.fld(mx);
    }
    f.fmul_m(cf(k::K_863984));
    let t18 = f.fstp();
    f.fld(my);
    f.fmul_m(cf(k::K_863984));
    let t1c = f.fstp();
    f.fmul_m(cf(k::K_863984));
    let ts = sa_core::timer::TIME_STEP.get();
    f.fld(t18);
    f.fmul_m(ts);
    f.fld(t1c);
    f.fmul_m(ts);
    let vy = f.fstp();
    f.fxch(1);
    f.fmul_m(ts);
    let vz = f.fstp();
    f.fadd_m(rf(this, MOVE_SPEED));
    wf(this, MOVE_SPEED, f.fstp());
    f.fld(vy);
    f.fadd_m(rf(this, MOVE_SPEED + 4));
    wf(this, MOVE_SPEED + 4, f.fstp());
    f.fld(vz);
    f.fadd_m(rf(this, MOVE_SPEED + 8));
    wf(this, MOVE_SPEED + 8, f.fstp());
    1
}

orig! {
    /// `CCarCtrl::SwitchVehicleToRealPhysics(CVehicle*)` (0x423FC0)
    pub fn switch_vehicle_to_real_physics(v: Ent) = cdecl 0x423FC0 => unported;
    /// `CCarCtrl::FindGhostRoadHeight(CVehicle*)` (0x422370, ST0)
    pub fn find_ghost_road_height(v: Ent) -> f64 = cdecl 0x422370 => unported;
    /// `CMatrix::ForceUpVector(CVector)` (0x59B7E0, vector by value)
    pub fn matrix_force_up_vector(m: Ent, up: CVector) = thiscall 0x59B7E0 => unported;
    /// virtual `CVehicle::GetHeightAboveRoad()` (vtable +0xD4, ST0)
    pub fn get_height_above_road_v(v: Ent) -> f64 = thiscall vslot 0xD4 => unported;
    /// virtual `CPhysical::SpecialEntityCalcCollisionSteps(bool&, bool&)` (vtable +0x40, AL)
    pub fn special_entity_calc_collision_steps_v(e: Ent, b1: GPtr<u8>, b2: GPtr<u8>) -> u8 = thiscall vslot 0x40 => unported;
    /// `CPhysical::CheckCollision_SimpleCar()` (0x54DAB0)
    pub fn check_collision_simple_car_call(e: Ent) -> u8 = thiscall 0x54DAB0 => super::attach::check_collision_simple_car;
}

/// `TIME_STEP = v > 1e-5f (or NaN) ? v : 1e-5f` with `v` on the x87 stack (`fcom; test ah, 0x41;
/// jp; fstp st0; fld 1e-5`), popped into `CTimer::ms_fTimeStep`.
fn store_time_step(f: &mut Fpu) {
    f.fcom_m(cf(k::K_858C14));
    if !f.jp(0x41) {
        f.fstp_st(0);
        f.fld(cf(k::K_858C14));
    }
    sa_core::timer::TIME_STEP.set(f.fstp());
}

/// The ped standing fix-up of the step loop (0x54E504 / 0x54E7B5): a ped with `moveSpeed.z == 0`,
/// `+0x46C` bit 1 clear and bit 0 set copies the matrix' pos.z (raw) into the saved matrix.
fn ped_keep_z(this: Ent, saved: &mut LocalMatrix) {
    if etype(this) != ty::PED {
        return;
    }
    let mut f = Fpu::new();
    f.fld(rf(this, MOVE_SPEED + 8));
    f.fcomp_m(cf(k::K_858B50));
    if f.jp(0x44) {
        return;
    }
    if (ru(this, 0x46C) >> 1) & 1 != 0 || rb(this, 0x46C) & 1 == 0 {
        return;
    }
    saved[14] = ru(matrix(this), 0x38);
}

/// `CPhysical::ProcessCollision()` (0x54DFB0, virtual).
///
/// Moving speed 0, physical flags `&= ~0x9000`. Without `bUsesCollision` or with physical flag
/// 0x20000: entity flags `(& ~0x10) | 0x20`, `RemoveAndAdd`. Status 2 (simple):
/// `CheckCollision_SimpleCar`; on a hit a still-simple entity becomes status 3 and a vehicle
/// `SwitchVehicleToRealPhysics`; then the flags as above and `RemoveAndAdd`.
///
/// Status 12 (ghost; a vehicle): the wheel arrays by `+0x594` (0: ground entities +0x8E8..,
/// compressions +0x7D4, col points +0x724, positions +0x8F8; else +0x7C0, +0x710, +0x660, +0x7D0).
/// Per col line of the col model (none with the disk flag, col data +7 bit 0): both ends transformed,
/// the col point gets normal (0, 0, 1), surfaces 60/1, depth 0, and against `FindGhostRoadHeight` the
/// compression 0 (road above the start: the start), 1 (road at or below the end: the end) or `(s.z -
/// h) / (s.z - e.z)` (the point `(e - s) * c + s`); the position is the point minus the entity
/// position. Then move z and turn x/y zeroed, entity flags as above, `ForceUpVector(0, 0, 1)`,
/// `pos.z = FindGhostRoadHeight + GetHeightAboveRoad()`, `ApplySpeed`, `Reorthogonalise`,
/// `RemoveAndAdd`, vehicle flag `+0x42B |= 1`.
///
/// Otherwise the move speed, the time step and the elasticity are saved, a local copy of the matrix
/// is made and virtual `SpecialEntityCalcCollisionSteps(&pre, &b)` gives the steps (`step = timeStep
/// / steps`, f32). With `pre`: `ApplySpeed`, `Reorthogonalise`, physical flags 0x9000 cleared and
/// 0x10000 set, `bUsesCollision` cleared over `CheckCollision`, both restored; no hit: a vehicle sets
/// `+0x42B |= 1` and the end path below; a hit: matrix and move speed restored, a vehicle with
/// `+0x428` bit 0 multiplies its elasticity by `HIGHSPEED_ELASTICITY_MULT_COPCAR`. Steps 1..n-1: time
/// step `max(i*step, 1e-5)`, `ApplySpeed`, `CheckCollision` (a hit: the ped z fix-up, matrix and
/// time step restored, elasticity restored, done); else the ped z fix-up, matrix and time step
/// restored and a vehicle's wheel compressions set to 1.0 (+0x590 type 0: 4 at +0x7D4; 9: 4 at
/// +0x710; 11: 3 at +0x7D4). Then `ApplySpeed`, `Reorthogonalise`, flags 0x9000 cleared; when
/// anything moves (each of the six speed words `!= 0`), flag 0x800, status 0, a vehicle with `+0x42C`
/// bit 1, or a ped that is the player, has `+0x474` bit 15 or is not standing (`+0x46C` bit 0
/// clear): `+0x42B |= 1` for a vehicle and `CheckCollision` (a hit restores the matrix and the
/// elasticity, done). Otherwise a ped is set standing (`+0x46C |= 1`). End path: flags `(& ~0x10) |
/// 0x20`, physical flags `&= ~0x1800`, elasticity restored, moving speed `|pos - saved pos|`
/// (`(dx² + dy²) + dz²`), `RemoveAndAdd`. The local matrix is destroyed on every path that built it.
#[hook(addr = 0x54DFB0, conv = thiscall, name = "CPhysical::ProcessCollision", expect = "6A FF 64 A1 00 00 00 00",
       this = "CPhysical", unverified = "mvpe-phys-1", route = "L2", hazards = "none", recs = "grove-walk,driving",
       note = "SEH frame with a local CMatrix (dtor on every path that built it)")]
pub fn process_collision(this: Ent) {
    let pf0 = ru(this, PFLAGS) & 0xFFFF_6FFF;
    wu(this, MOVING_SPEED, 0);
    wu(this, PFLAGS, pf0);
    let safe = |this: Ent| {
        wu(this, EFLAGS, (ru(this, EFLAGS) & 0xFFFF_FFEF) | 0x20);
        remove_and_add_call(this);
    };
    if rb(this, EFLAGS) & 1 == 0 || pf0 & 0x20000 != 0 {
        safe(this);
        return;
    }
    let status = rb(this, TYPE) >> 3;
    if status == 2 {
        if check_collision_simple_car_call(this) != 0 {
            let t = rb(this, TYPE);
            if t & 0xF8 == 0x10 {
                let t = (t & 7) | 0x18;
                wb(this, TYPE, t);
                if t & 7 == ty::VEHICLE {
                    switch_vehicle_to_real_physics(this);
                }
            }
        }
        safe(this);
        return;
    }
    if status == 0xC {
        process_collision_ghost(this);
        return;
    }
    let saved_ms = rv_bits(this, MOVE_SPEED);
    let saved_ts = sa_core::timer::TIME_STEP.get();
    let saved_el = ru(this, ELASTICITY);
    let mut saved: LocalMatrix = [0; 18];
    let sp = lp(&mut saved).cast::<u8>();
    matrix_copy_ctor(sp, matrix(this));
    let mut pre: u8 = 0;
    let mut b27: u8 = 0;
    let steps = special_entity_calc_collision_steps_v(this, lp(&mut pre), lp(&mut b27));
    let mut f = Fpu::new();
    f.fild(i32::from(steps));
    f.fdivr_m(saved_ts);
    let step = f.fstp();
    let finish = |this: Ent, saved: &mut LocalMatrix| {
        wu(this, EFLAGS, (ru(this, EFLAGS) & 0xFFFF_FFEF) | 0x20);
        let pf = ru(this, PFLAGS) & 0xFFFF_F7FF;
        wu(this, PFLAGS, pf);
        let m = matrix(this);
        let s = |i: usize| f32::from_bits(saved[12 + i]);
        let mut f = Fpu::new();
        f.fld(rf(m, 0x38));
        f.fsub_m(s(2));
        f.fld(rf(m, 0x34));
        f.fsub_m(s(1));
        f.fld(rf(m, 0x30));
        wu(this, PFLAGS, pf & 0xFFFF_EFFF);
        f.fsub_m(s(0));
        wu(this, ELASTICITY, saved_el);
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fld_st(3);
        f.fmul_st(4);
        f.faddp(1);
        f.fstp_st(3);
        f.fstp_st(0);
        f.fstp_st(0);
        f.fsqrt();
        wf(this, MOVING_SPEED, f.fstp());
        remove_and_add_call(this);
        matrix_dtor(lp(saved).cast());
    };
    if pre != 0 {
        apply_speed(this);
        matrix_reorthogonalise(matrix(this));
        let uses = rb(this, EFLAGS) & 1;
        let pf = (ru(this, PFLAGS) & 0xFFFF_6FFF) | 0x10000;
        wu(this, EFLAGS, ru(this, EFLAGS) & 0xFFFF_FFFE);
        wu(this, PFLAGS, pf);
        let hit = check_collision_call(this);
        let e = ru(this, EFLAGS);
        wu(this, EFLAGS, e ^ ((u32::from(uses) ^ e) & 1));
        wu(this, PFLAGS, ru(this, PFLAGS) & 0xFFFE_FFFF);
        if hit == 0 {
            if etype(this) == ty::VEHICLE {
                wb(this, 0x42B, rb(this, 0x42B) | 1);
            }
            finish(this, &mut saved);
            return;
        }
        matrix_assign(matrix(this), sp);
        wv_bits(this, MOVE_SPEED, saved_ms);
        if etype(this) == ty::VEHICLE && rb(this, 0x428) & 1 != 0 {
            f.fld(HIGHSPEED_ELASTICITY_MULT_COPCAR.get());
            f.fmul_m(rf(this, ELASTICITY));
            wf(this, ELASTICITY, f.fstp());
        }
    }
    let restore_ts = |f: &mut Fpu| {
        f.fld(saved_ts);
        f.fld(saved_ts);
        f.fcomp_m(cf(k::K_858C14));
        if !f.jp(0x41) {
            f.fstp_st(0);
            f.fld(cf(k::K_858C14));
        }
        sa_core::timer::TIME_STEP.set(f.fstp());
    };
    let n = u16::from(steps);
    let mut i: u16 = 1;
    while n > i {
        f.fild(i32::from(i));
        f.fmul_m(step);
        store_time_step(&mut f);
        apply_speed(this);
        if check_collision_call(this) != 0 {
            ped_keep_z(this, &mut saved);
            matrix_assign(matrix(this), sp);
            restore_ts(&mut f);
            wu(this, ELASTICITY, saved_el);
            matrix_dtor(sp);
            return;
        }
        ped_keep_z(this, &mut saved);
        matrix_assign(matrix(this), sp);
        restore_ts(&mut f);
        if etype(this) == ty::VEHICLE {
            let one = 0x3F80_0000;
            match ru(this, 0x590) {
                0 => (0..4).for_each(|j| wu(this, 0x7D4 + 4 * j, one)),
                9 => (0..4).for_each(|j| wu(this, 0x710 + 4 * j, one)),
                0xB => (0..3).for_each(|j| wu(this, 0x7D4 + 4 * j, one)),
                _ => {}
            }
        }
        i += 1;
    }
    apply_speed(this);
    matrix_reorthogonalise(matrix(this));
    let pf = ru(this, PFLAGS) & 0xFFFF_6FFF;
    wu(this, PFLAGS, pf);
    let moving = (0..6).any(|j| {
        f.fld(rf(this, MOVE_SPEED + 4 * j));
        f.fcomp_m(cf(k::K_858B50));
        f.jp(0x44)
    });
    let check = moving
        || pf & 0x800 != 0
        || rb(this, TYPE) & 0xF8 == 0
        || (etype(this) == ty::VEHICLE && rb(this, 0x42C) & 2 != 0)
        || (etype(this) == ty::PED
            && (ped_is_player(this) != 0 || ru(this, 0x474) & 0x8000 != 0 || rb(this, 0x46C) & 1 == 0));
    if check {
        if etype(this) == ty::VEHICLE {
            wb(this, 0x42B, rb(this, 0x42B) | 1);
        }
        if check_collision_call(this) != 0 {
            matrix_assign(matrix(this), sp);
            wu(this, ELASTICITY, saved_el);
            matrix_dtor(sp);
            return;
        }
    } else if etype(this) == ty::PED {
        wu(this, 0x46C, ru(this, 0x46C) | 1);
    }
    finish(this, &mut saved);
}

/// The status-12 (ghost vehicle) path of `ProcessCollision` (0x54E04F..0x54E392).
fn process_collision_ghost(this: Ent) {
    let (comp, cps, posv) = if ru(this, 0x594) == 0 {
        wv_bits(this, 0x8E8, [0; 3]);
        wu(this, 0x8F4, 0);
        (0x7D4isize, 0x724isize, 0x8F8isize)
    } else {
        wv_bits(this, 0x7C0, [0; 3]);
        wu(this, 0x7CC, 0);
        (0x710, 0x660, 0x7D0)
    };
    let col_data = rp(get_col_model(this), 0x2C);
    let mut i: i32 = 0;
    loop {
        let count = if rb(col_data, 7) & 1 != 0 { 0 } else { i32::from(rb(col_data, 6) as i8) };
        if i >= count {
            break;
        }
        let line = rp(col_data, 0x10).byte_add(0x20 * i as isize);
        let mut o1 = CVector::new(0.0, 0.0, 0.0);
        let s = rv(matrix_mul_point(lp(&mut o1), matrix(this), line.cast()).cast(), 0);
        let mut o2 = CVector::new(0.0, 0.0, 0.0);
        let e = rv(matrix_mul_point(lp(&mut o2), matrix(this), line.byte_add(0x10).cast()).cast(), 0);
        let cp = this.byte_add(cps + 0x2C * i as isize);
        wv_bits(cp, 0x10, [0, 0, 0x3F80_0000]);
        wb(cp, 0x20, 0x3C);
        wb(cp, 0x23, 1);
        wu(cp, 0x28, 0);
        let ci = comp + 4 * i as isize;
        let mut f = Fpu::new();
        f.push(X87::from_f64_exact(find_ghost_road_height(this)));
        let h = f.fst();
        f.fcomp_m(s[2]);
        if !f.jne(0x41) {
            wu(this, ci, 0);
            wv(cp, 0, s);
        } else {
            f.fld(h);
            f.fcomp_m(e[2]);
            if f.jne(0x41) {
                wf(this, ci, cf(k::K_863B58));
                wv(cp, 0, e);
            } else {
                f.fld(s[2]);
                f.fsub_m(h);
                f.fld(s[2]);
                f.fsub_m(e[2]);
                f.fdivp(1);
                f.fld_st(0);
                wf(this, ci, f.fstp());
                f.fld(e[0]);
                f.fsub_m(s[0]);
                f.fld(e[1]);
                f.fsub_m(s[1]);
                let dy = f.fstp();
                f.fld(e[2]);
                f.fsub_m(s[2]);
                let dz = f.fstp();
                f.fxch(1);
                let r = f.fstp();
                f.fmul_m(r);
                f.fld(dy);
                f.fmul_m(r);
                f.fld(dz);
                f.fmul_m(r);
                let tz = f.fstp();
                f.fxch(1);
                f.fadd_m(s[0]);
                wf(cp, 0, f.fstp());
                f.fadd_m(s[1]);
                wf(cp, 4, f.fstp());
                f.fld(tz);
                f.fadd_m(s[2]);
                wf(cp, 8, f.fstp());
            }
        }
        let pos = pos_ptr(this);
        let pv = this.byte_add(posv + 0xC * i as isize);
        for j in 0..3isize {
            f.fld(rf(cp, 4 * j));
            f.fsub_m(rf(pos, 4 * j));
            wf(pv, 4 * j, f.fstp());
        }
        i += 1;
    }
    wu(this, MOVE_SPEED + 8, 0);
    wu(this, TURN_SPEED + 4, 0);
    wu(this, TURN_SPEED, 0);
    wu(this, EFLAGS, (ru(this, EFLAGS) & 0xFFFF_FFEF) | 0x20);
    let m = matrix(this);
    matrix_force_up_vector(m, CVector::new(0.0, 0.0, 1.0));
    let mut f = Fpu::new();
    f.push(X87::from_f64_exact(get_height_above_road_v(this)));
    let hr = f.fstp();
    f.push(X87::from_f64_exact(find_ghost_road_height(this)));
    f.fadd_m(hr);
    wf(m, 0x38, f.fstp());
    apply_speed(this);
    matrix_reorthogonalise(matrix(this));
    remove_and_add_call(this);
    wb(this, 0x42B, rb(this, 0x42B) | 1);
}

orig! {
    /// `CPhysical::ApplyCollision(CEntity*, CColPoint&, float&, float&)` (0x548680, AL)
    pub fn apply_collision_2(e: Ent, other: Ent, cp: Ent, d1: GPtr<f32>, d2: GPtr<f32>) -> u8 = thiscall 0x548680 => unported;
    /// `CPhysical::SetDamagedPieceRecord(float, CEntity*, CColPoint&, float)` (0x5428C0; the
    /// intensity as the dword the original pushes)
    pub fn set_damaged_piece_record_call(e: Ent, intensity: u32, other: Ent, cp: Ent, dist: f32)
        = thiscall 0x5428C0 => super::forces::set_damaged_piece_record;
    /// `SurfaceInfos_c::GetAdhesiveLimit(CColPoint&)` (0x55EB50, thiscall on `g_surfaceInfos`, ST0)
    pub fn get_adhesive_limit_call(si: Ent, cp: Ent) -> f64 = thiscall 0x55EB50 => get_adhesive_limit_port;
    /// `CPhysical::ApplyFriction(CPhysical*, float, CColPoint&)` (0x545980, AL)
    pub fn apply_friction_other(e: Ent, other: Ent, adhesive: f32, cp: Ent) -> u8 = thiscall 0x545980 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    pub fn allocate_matrix_call(e: Ent) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    pub fn simple_transform_update_matrix_call(t: Ent, m: Ent) = thiscall 0x54F1B0 => unported;
}

/// Phases B/C side of `GetAdhesiveLimit` (the port returns the f32 table entry as an `X87`, exact).
pub fn get_adhesive_limit_port(si: Ent, cp: Ent) -> f64 {
    crate::collision::surface::get_adhesive_limit(si.cast(), cp).to_f64()
}

/// `g_surfaceInfos` (0xB79538)
const SURFACE_INFOS: usize = 0xB79538;
/// `CWorld::m_aTempColPts` as `ProcessCollisionSectorList_SimpleCar` passes it (0xB73C98)
const SIMPLE_CAR_COL_PTS: usize = 0xB73C98;
/// The function-static guard byte of `ProcessCollisionSectorList_SimpleCar` (0xB74218)
const SIMPLE_CAR_GUARD: usize = 0xB74218;

/// The contact loop of `ProcessCollisionSectorList_SimpleCar`: per point `ApplyCollision(other, cp,
/// &d1, &d2)`; on true `SetDamagedPieceRecord(d1, other, cp, 1.0)` on this and `(d2, this, cp, -1.0)`
/// on the other, and with `friction`: `ApplyFriction(other, adhesiveLimit(cp) / n, cp)` (`fidiv`),
/// true sets entity flag 8 on both. Returns the last adhesive dword computed (stack residue only).
fn simple_car_contacts(this: Ent, other: Ent, n: i32, friction: bool, d: &mut [f32; 2]) {
    for i in 0..n {
        let cp = GPtr::<u8>::from_addr(SIMPLE_CAR_COL_PTS + COL_POINT_SIZE * i as usize);
        let (d1, d2) = d.split_at_mut(1);
        if apply_collision_2(this, other, cp, lp(&mut d1[0]), lp(&mut d2[0])) == 0 {
            continue;
        }
        set_damaged_piece_record_call(this, d[0].to_bits(), other, cp, 1.0);
        set_damaged_piece_record_call(other, d[1].to_bits(), this, cp, -1.0);
        if !friction {
            continue;
        }
        let mut f = Fpu::new();
        f.push(X87::from_f64_exact(get_adhesive_limit_call(GPtr::from_addr(SURFACE_INFOS), cp)));
        f.fild(n);
        f.fdivp(1);
        let adh = f.fstp();
        if apply_friction_other(this, other, adh, cp) != 0 {
            wu(this, EFLAGS, ru(this, EFLAGS) | 8);
            wu(other, EFLAGS, ru(other, EFLAGS) | 8);
        }
    }
}

/// `CPhysical::ProcessCollisionSectorList_SimpleCar(CRepeatSector*)` (0x54CFF0, `ret 4`, AL).
///
/// Sets the function-static guard bit (0xB74218 bit 0). Without `bUsesCollision` false. Bound centre
/// and the model col-model radius; the repeat sector's objects (+8), then vehicles (+0). Skipped:
/// itself, an object with object flag +0x140 bit 0x100 whose up.z < 0.66 (the matrix allocated
/// first if missing), current scan code, `!bUsesCollision`, not `GetIsTouching`. Else scan code set
/// and virtual `ProcessEntityCollision(it, 0xB73C98)`; the first entity with points ends the search:
/// both with entity flag 8: the collisions only; this with flag 8: its friction move/turn speeds are
/// saved and zeroed and flag 8 cleared, collisions with friction, and unless a friction contact set
/// flag 8 again the speeds are restored and flag 8 set; the other with flag 8: the same on the
/// other; neither: collisions with friction. Then the other entity at status 2 becomes status 3
/// (a vehicle `SwitchVehicleToRealPhysics`); true. The damage locals start at -1.0.
///
/// Stack: the original reuses its argument slot (S+4) for the entity flags, the contact counter or
/// the last adhesive limit, and the list counter local for the adhesive limit (not reproduced).
#[hook(addr = 0x54CFF0, conv = thiscall, name = "CPhysical::ProcessCollisionSectorList_SimpleCar", expect = "A0 18 42 B7 00 83 EC 48",
       this = "CPhysical", unverified = "mvpe-phys-1", route = "L2", hazards = "stack-residue", recs = "driving",
       note = "writes flags / counters / adhesive limits into its argument slot S+4; nested hooks, so no footprint")]
pub fn process_collision_sector_list_simple_car(this: Ent, sector: Ent) -> u8 {
    let guard = GPtr::<u32>::from_addr(SIMPLE_CAR_GUARD);
    if guard.cast::<u8>().read() & 1 == 0 {
        guard.write(guard.read() | 1);
    }
    let mut d = [-1.0f32, -1.0f32];
    if rb(this, EFLAGS) & 1 == 0 {
        return 0;
    }
    let model = rh(this, MODEL_INDEX) as i16 as i32;
    let mi = sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(model).read();
    let radius = rf(rp(mi, 0x14), 0x24);
    let mut centre = [0f32; 3];
    get_bound_centre_ref(this, lp(&mut centre).cast());
    let scan = GPtr::<u16>::from_addr(SCAN_CODE);
    for list in [sector.byte_add(8), sector] {
        let mut node = rp(list, 0);
        while !node.is_null() {
            let e = rp(node, 0);
            node = rp(node, 4);
            let mut tilted = false;
            if etype(e) == ty::OBJECT && ru(e, 0x140) & 0x100 != 0 {
                if matrix(e).is_null() {
                    allocate_matrix_call(e);
                    simple_transform_update_matrix_call(e.byte_add(PLACEMENT_POS), matrix(e));
                }
                let mut f = Fpu::new();
                f.fld(rf(matrix(e), 0x28));
                f.fcomp_m(cf(k::K_85BA50));
                tilted = !f.jp(5);
            }
            if e == this
                || tilted
                || rh(e, 0x2C) == scan.read()
                || rb(e, EFLAGS) & 1 == 0
                || get_is_touching_vec(e, lp(&mut centre).cast(), radius) as u8 == 0
            {
                continue;
            }
            wh(e, 0x2C, scan.read());
            let n = process_entity_collision_v(this, e, GPtr::from_addr(SIMPLE_CAR_COL_PTS));
            if n <= 0 {
                continue;
            }
            let tf = ru(this, EFLAGS);
            if (tf >> 3) & 1 != 0 && (ru(e, EFLAGS) >> 3) & 1 != 0 {
                simple_car_contacts(this, e, n, false, &mut d);
            } else if (tf >> 3) & 1 != 0 || (ru(e, EFLAGS) >> 3) & 1 != 0 {
                let who = if (tf >> 3) & 1 != 0 { this } else { e };
                let fm = rv_bits(who, FRICTION_MOVE);
                let ft = rv_bits(who, FRICTION_TURN);
                wu(who, EFLAGS, ru(who, EFLAGS) & 0xFFFF_FFF7);
                wv_bits(who, FRICTION_MOVE, [0; 3]);
                wv_bits(who, FRICTION_TURN, [0; 3]);
                simple_car_contacts(this, e, n, true, &mut d);
                let w = ru(who, EFLAGS);
                if (w >> 3) & 1 == 0 {
                    wv_bits(who, FRICTION_MOVE, fm);
                    wv_bits(who, FRICTION_TURN, ft);
                    wu(who, EFLAGS, w | 8);
                }
            } else {
                simple_car_contacts(this, e, n, true, &mut d);
            }
            let t = rb(e, TYPE);
            if t & 0xF8 == 0x10 {
                let t = (t & 7) | 0x18;
                wb(e, TYPE, t);
                if t & 7 == ty::VEHICLE {
                    switch_vehicle_to_real_physics(e);
                }
            }
            return 1;
        }
    }
    0
}

orig! {
    /// `CPhysical::ApplySoftCollision(CPhysical*, CColPoint&, float&, float&)` (0x54A2C0, AL)
    pub fn apply_soft_collision_2(e: Ent, other: Ent, cp: Ent, d1: GPtr<f32>, d2: GPtr<f32>) -> u8 = thiscall 0x54A2C0 => unported;
    /// `CPhysical::ApplyCollisionAlt(CEntity*, CColPoint&, float&, CVector&, CVector&)` (0x544D50, AL)
    pub fn apply_collision_alt(e: Ent, other: Ent, cp: Ent, d: GPtr<f32>, move_: Ent, turn: Ent) -> u8 = thiscall 0x544D50 => unported;
    /// `CPhysical::ApplySoftCollision(CEntity*, CColPoint&, float&)` (0x543890, AL)
    pub fn apply_soft_collision_1(e: Ent, other: Ent, cp: Ent, d: GPtr<f32>) -> u8 = thiscall 0x543890 => super::collide::apply_soft_collision;
    /// `CPhysical::ApplyFriction(float, CColPoint&)` (0x5454C0, AL)
    pub fn apply_friction_self_call(e: Ent, adhesive: f32, cp: Ent) -> u8 = thiscall 0x5454C0 => super::friction::apply_friction;
    /// `CEntity::GetIsStatic()` (0x4633E0)
    pub fn get_is_static_call(e: Ent) -> u32 = thiscall 0x4633E0 => crate::world::entity::get_is_static;
    /// `CVector::SquaredMagnitude()` (0x406DA0, ST0)
    pub fn squared_magnitude(v: Ent) -> f64 = thiscall 0x406DA0 => squared_magnitude_port;
    /// `DotProduct(const CVector&, const CVector&)` (0x40FDB0, cdecl, ST0)
    pub fn dot_product(a: Ent, b: Ent) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CPed::KillPedWithCar(CVehicle*, float, bool)` (0x5F0360)
    pub fn kill_ped_with_car(ped: Ent, veh: Ent, impulse: f32, b: u32) = thiscall 0x5F0360 => unported;
    /// `CObject::ObjectDamage(float, const CVector*, const CVector*, CEntity*, eWeaponType)` (0x5A0D90)
    pub fn object_damage(obj: Ent, damage: f32, pt: Ent, normal: Ent, by: Ent, weapon: u32) = thiscall 0x5A0D90 => unported;
    /// `VectorSub(CVector* out, const CVector*, const CVector*)` (0x40FE60)
    pub fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => crate::world::mvpa_math::vector_sub;
    /// `CVector operator/(const CVector&, float)` (0x4119D0, hidden return pointer)
    pub fn vector_div_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x4119D0 => crate::world::mvpa_math::vector_div_scalar;
    /// `CVector::operator+=(const CVector&)` (0x411A00)
    pub fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x411A00 => crate::world::mvpa_math::vector_add_assign;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => crate::world::cworld::find_player_ped;
    /// `SurfaceInfos_c::GetAdhesionGroup(uint32)` (0x55E5C0, thiscall on `g_surfaceInfos`)
    pub fn get_adhesion_group_call(si: Ent, id: u32) -> u32 = thiscall 0x55E5C0 => get_adhesion_group_port;
    /// `Invert(const CMatrix& src, CMatrix& dst)` (0x59B920, returns dst)
    pub fn matrix_invert(src: Ent, dst: Ent) -> Ent = cdecl 0x59B920 => unported;
}

/// Phases B/C side of `CVector::SquaredMagnitude` (sums of f32 products at PC 24: exact as f64).
pub fn squared_magnitude_port(v: Ent) -> f64 {
    crate::world::mvpa_math::vector_squared_magnitude(v).to_f64()
}

/// Phases B/C side of `GetAdhesionGroup`.
pub fn get_adhesion_group_port(si: Ent, id: u32) -> u32 {
    crate::collision::surface::get_adhesion_group(si.cast(), id)
}

/// `CWorld::m_aTempColPts` (0xB73710), the col points `ProcessCollisionSectorList` collects into.
const TEMP_COL_PTS: usize = 0xB73710;
/// The function-static guard byte of `ProcessCollisionSectorList` (0xB73C90)
const SECTOR_LIST_GUARD: usize = 0xB73C90;
/// `CWorld::bSecondShift`
const SECOND_SHIFT: usize = 0xB7CD6C;
/// `CWorld::bNoMoreCollisionTorque`
const NO_MORE_COLLISION_TORQUE: usize = 0xB7CD72;

/// The locals of `ProcessCollisionSectorList` the contact loops share (S+0x14 / S+0x1C damages, S+0x34
/// / S+0x2C their maxima, S+0x18 the soft-contact count, S+0x13 the soft flag).
struct PcslState {
    d: [f32; 2],
    max1: f32,
    max2: f32,
    soft: i32,
    b13: u8,
}

fn in_wheels(p: u8) -> bool {
    (0xD..=0x10).contains(&p)
}

fn temp_cp(i: i32) -> Ent {
    GPtr::from_addr(TEMP_COL_PTS + COL_POINT_SIZE * i as usize)
}

/// `if (v > *max) *max = v` (`fld; fcomp; test ah, 0x41; jne`).
fn raise_max(v: f32, max: &mut f32) {
    let mut f = Fpu::new();
    f.fld(v);
    f.fcomp_m(*max);
    if !f.jne(0x41) {
        *max = v;
    }
}

/// The contact loops of `ProcessCollisionSectorList`'s ordinary path (0x54BD83, 0x54BF40, 0x54C202,
/// 0x54C400). Hard contact (no soft flag and no wheel piece: both pieces, or with `piece_a_only` piece
/// A twice, the original's bug in the neither-flag loop): `ApplyCollision(it, cp, &d1, &d2)`; on true
/// the maxima, `SetDamagedPieceRecord(d1, it, cp, 1.0)` on this and `(d2, this, cp, -1.0)` on it,
/// and with `friction` the adhesive limit / n (`fidiv`), for two vehicles where this moves (`|v|² >
/// 0.02` or `|w|² > 0.01`) `* G_8CD7AC * d1`, then `ApplyFriction(adh, cp)` on this against a static
/// entity (true: this flag 8) or `ApplyFriction(it, adh, cp)` (true: flag 8 on both). Soft contact:
/// count, `ApplySoftCollision(it, cp, &d1, &d2)`, a wheel piece B gives it `SetDamagedPieceRecord(d2,
/// this, cp, -1.0)`.
fn pcsl_contacts(this: Ent, e: Ent, n: i32, friction: bool, piece_a_only: bool, s: &mut PcslState) {
    for i in 0..n {
        let cp = temp_cp(i);
        let pa = rb(cp, 0x21);
        let hard = s.b13 == 0 && !in_wheels(pa) && (piece_a_only || !in_wheels(rb(cp, 0x24)));
        let (d1p, d2p) = (lp(&mut s.d[0]), lp(&mut s.d[1]));
        if !hard {
            s.soft += 1;
            apply_soft_collision_2(this, e, cp, d1p, d2p);
            if in_wheels(rb(cp, 0x24)) {
                set_damaged_piece_record_call(e, s.d[1].to_bits(), this, cp, -1.0);
            }
            continue;
        }
        if apply_collision_2(this, e, cp, d1p, d2p) == 0 {
            continue;
        }
        raise_max(s.d[0], &mut s.max1);
        raise_max(s.d[1], &mut s.max2);
        set_damaged_piece_record_call(this, s.d[0].to_bits(), e, cp, 1.0);
        set_damaged_piece_record_call(e, s.d[1].to_bits(), this, cp, -1.0);
        if !friction {
            continue;
        }
        let mut f = Fpu::new();
        f.push(X87::from_f64_exact(get_adhesive_limit_call(GPtr::from_addr(SURFACE_INFOS), cp)));
        f.fild(n);
        f.fdivp(1);
        let mut adh = f.fstp();
        if etype(this) == ty::VEHICLE && etype(e) == ty::VEHICLE {
            f.push(X87::from_f64_exact(squared_magnitude(this.byte_add(MOVE_SPEED))));
            f.fcomp_m(G_8CD7B0.get());
            let mut scale = !f.jne(0x41);
            if !scale {
                f.push(X87::from_f64_exact(squared_magnitude(this.byte_add(TURN_SPEED))));
                f.fcomp_m(G_8CD7B4.get());
                scale = !f.jne(0x41);
            }
            if scale {
                f.fld(adh);
                f.fmul_m(G_8CD7AC.get());
                f.fmul_m(s.d[0]);
                adh = f.fstp();
            }
        }
        if get_is_static_call(e) as u8 != 0 {
            if apply_friction_self_call(this, adh, cp) != 0 {
                wu(this, EFLAGS, ru(this, EFLAGS) | 8);
            }
        } else if apply_friction_other(this, e, adh, cp) != 0 {
            wu(this, EFLAGS, ru(this, EFLAGS) | 8);
            wu(e, EFLAGS, ru(e, EFLAGS) | 8);
        }
    }
}

/// The car-object damage test of `ProcessCollisionSectorList` (0x54C6AE / 0x54C803): `obj`'s bounding
/// box diagonal (`VectorSub(max, min)`) transformed by `obj`'s matrix (`p`); `this_is_obj` decides whether
/// the vehicle's position is on the damaging side at once (`pos.z > p.z` for an object hit by this
/// car, `p.z < pos.z` for this object hit by a car, both through the bytes' compare forms), else the
/// point in the vehicle's inverted matrix with `z < 0`; then `ObjectDamage(50.0, cps[0].point,
/// cps[0].normal, veh, 50)` on the object. A local CMatrix (attach fields 0) is destroyed after.
fn pcsl_object_crush(obj: Ent, veh: Ent, this_is_obj: bool) {
    let mut m: LocalMatrix = [0; 18];
    let mp = lp(&mut m).cast::<u8>();
    let model = rh(obj, MODEL_INDEX) as i16 as i32;
    let mi = sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(model).read();
    let cm = rp(mi, 0x14);
    let mut diag = [0f32; 3];
    vector_sub(lp(&mut diag).cast(), cm.byte_add(0xC), cm);
    let mut o = CVector::new(0.0, 0.0, 0.0);
    let p = rv(matrix_mul_point(lp(&mut o), matrix(obj), lp(&mut diag).cast()).cast(), 0);
    // the transformed point overwrites the diagonal's local (the second product uses it)
    diag = p;
    let vm = matrix(veh);
    let pos = if vm.is_null() { veh.byte_add(PLACEMENT_POS) } else { vm.byte_add(0x30) };
    let mut f = Fpu::new();
    let at_once = if this_is_obj {
        f.fld(p[2]);
        f.fcomp_m(rf(pos, 8));
        !f.jp(5)
    } else {
        f.fld(rf(pos, 8));
        f.fcomp_m(p[2]);
        !f.jne(0x41)
    };
    let damage = if at_once {
        true
    } else {
        let r = matrix_invert(vm, mp);
        matrix_assign(mp, r);
        let mut o2 = CVector::new(0.0, 0.0, 0.0);
        let q = rv(matrix_mul_point(lp(&mut o2), mp, lp(&mut diag).cast()).cast(), 0);
        f.fld(q[2]);
        f.fcomp_m(cf(k::K_858B50));
        !f.jp(5)
    };
    if damage {
        object_damage(obj, 50.0, GPtr::from_addr(TEMP_COL_PTS), GPtr::from_addr(TEMP_COL_PTS + 0x10), veh, 0x32);
    }
    matrix_dtor(mp);
}

/// The alternative path of `ProcessCollisionSectorList` (0x54C94B: a building, an entity with
/// physical flag 8, or the third pre-collision flag). Returns `Some(result)` to return at once.
fn pcsl_alt(this: Ent, e: Ent, s: &mut PcslState, result: &mut u8) -> Option<u8> {
    let scan = GPtr::<u16>::from_addr(SCAN_CODE);
    wh(e, 0x2C, scan.read());
    let n = process_entity_collision_v(this, e, GPtr::from_addr(TEMP_COL_PTS));
    if n <= 0 {
        return None;
    }
    let mut cnt: u8 = 0;
    let mut max_alt = 0f32;
    let mut va = [0f32; 3];
    let mut vb = [0f32; 3];
    let mut f = Fpu::new();
    let flag8 = (ru(this, EFLAGS) >> 3) & 1 != 0;
    for i in 0..n {
        let cp = temp_cp(i);
        let d1p = lp(&mut s.d[0]);
        if s.b13 != 0 || in_wheels(rb(cp, 0x21)) {
            let r = apply_soft_collision_1(this, e, cp, d1p);
            if flag8 || r == 0 || (rb(cp, 0x20) == 0x3C && rb(cp, 0x23) == 0x3C) {
                continue;
            }
            f.push(X87::from_f64_exact(get_adhesive_limit_call(GPtr::from_addr(SURFACE_INFOS), cp)));
            let adh = f.fstp();
            if apply_friction_self_call(this, adh, cp) != 0 {
                wu(this, EFLAGS, ru(this, EFLAGS) | 8);
            }
            continue;
        }
        if apply_collision_alt(this, e, cp, d1p, lp(&mut va).cast(), lp(&mut vb).cast()) == 0 {
            continue;
        }
        cnt = cnt.wrapping_add(1);
        raise_max(s.d[0], &mut max_alt);
        if flag8 {
            if etype(this) == ty::VEHICLE && ru(this, 0x590) == 5 && rb(cp, 0x23) == 0x2B {
                continue;
            }
            set_damaged_piece_record_call(this, s.d[0].to_bits(), e, cp, 1.0);
            continue;
        }
        f.push(X87::from_f64_exact(get_adhesive_limit_call(GPtr::from_addr(SURFACE_INFOS), cp)));
        f.fild(n);
        f.fdivp(1);
        let mut adh = f.fstp();
        if etype(this) != ty::VEHICLE {
            f.fld(G_8CD7A8.get());
            f.fmul_m(adh);
            f.fmul_m(s.d[0]);
            set_damaged_piece_record_call(this, s.d[0].to_bits(), e, cp, 1.0);
            let a2 = f.fstp();
            if apply_friction_self_call(this, a2, cp) != 0 {
                wu(this, EFLAGS, ru(this, EFLAGS) | 8);
            }
            continue;
        }
        // vehicle
        let mut model_check = true;
        if ru(this, 0x590) == 5 && rb(cp, 0x23) == 0x2B {
            adh = 0.0;
        } else {
            set_damaged_piece_record_call(this, s.d[0].to_bits(), e, cp, 1.0);
        }
        // 0x54CB44: the model 0x1B9 scale, else the type / status rules (0x54CB5B)
        if rh(this, MODEL_INDEX) == 0x1B9 {
            f.fld(adh);
            f.fmul_m(cf(k::K_858CC4));
            adh = f.fstp();
            model_check = false;
        }
        if model_check {
            if ru(this, 0x590) == 5 {
                f.fld(rf(cp, 0x18));
                f.fcomp_m(cf(k::K_858CC8));
                if f.jne(0x41) {
                    adh = 0.0;
                } else {
                    let si: Ent = GPtr::from_addr(SURFACE_INFOS);
                    let g = get_adhesion_group_call(si, u32::from(rb(cp, 0x23)));
                    if g == 3 || get_adhesion_group_call(si, u32::from(rb(cp, 0x23))) == 4 {
                        f.fld(adh);
                        f.fmul_m(cf(k::K_858B3C));
                        adh = f.fstp();
                    }
                }
            } else if ru(this, 0x594) != 6 {
                let st = rb(this, TYPE) >> 3;
                if st == 5 {
                    f.fld(adh);
                    f.fmul_m(cf(k::K_858B3C));
                    adh = f.fstp();
                } else {
                    let mut zero = false;
                    f.fld(rf(matrix(this), 0x28));
                    f.fcomp_m(cf(k::K_858C24));
                    if !f.jne(0x41) {
                        f.push(X87::from_f64_exact(squared_magnitude(this.byte_add(MOVE_SPEED))));
                        f.fcomp_m(G_8CD7B0.get());
                        if !f.jp(5) {
                            f.push(X87::from_f64_exact(squared_magnitude(this.byte_add(TURN_SPEED))));
                            f.fcomp_m(G_8CD7B4.get());
                            zero = !f.jp(5);
                        }
                    }
                    if zero {
                        adh = 0.0;
                    } else {
                        let scale = if st == 4 {
                            true
                        } else {
                            f.push(X87::from_f64_exact(dot_product(cp.byte_add(0x10), matrix(this).byte_add(0x20))));
                            f.fcomp_m(cf(k::K_863C1C));
                            !f.jp(5)
                        };
                        if scale {
                            f.fld(G_8CD7A8.get());
                            f.fdiv_m(rf(this, MASS));
                            f.fmul_m(adh);
                            f.fmul_m(s.d[0]);
                            adh = f.fstp();
                        }
                    }
                }
            }
        }
        // 0x54CC5D
        if etype(this) == ty::VEHICLE && ru(this, 0x594) == 6 {
            f.fld(adh);
            f.fadd_st(0);
            adh = f.fstp();
        }
        if apply_friction_self_call(this, adh, cp) != 0 {
            wu(this, EFLAGS, ru(this, EFLAGS) | 8);
        }
    }
    if cnt == 0 {
        return None;
    }
    f.fild(i32::from(cnt));
    f.fdivr_m(cf(k::K_858624));
    for (base, v) in [(MOVE_SPEED, va), (TURN_SPEED, vb)] {
        if base == MOVE_SPEED {
            f.fld_st(0);
        }
        f.fld(v[0]);
        f.fmul_st(1);
        f.fld_st(1);
        f.fmul_m(v[1]);
        let t1 = f.fstp();
        f.fxch(1);
        f.fmul_m(v[2]);
        let t2 = f.fstp();
        f.fadd_m(rf(this, base));
        wf(this, base, f.fstp());
        f.fld(t1);
        f.fadd_m(rf(this, base + 4));
        wf(this, base + 4, f.fstp());
        f.fld(t2);
        f.fadd_m(rf(this, base + 8));
        wf(this, base + 8, f.fstp());
    }
    if GPtr::<u8>::from_addr(NO_MORE_COLLISION_TORQUE).read() == 0
        && rb(this, TYPE) & 0xF8 == 0
        && etype(this) == ty::VEHICLE
    {
        let big = |off: isize, f: &mut Fpu| {
            f.fld(rf(this, off));
            f.fcomp_m(cf(k::K_858B50));
            f.fld(rf(this, off));
            if !f.jp(5) {
                f.fchs();
            }
            f.fcomp_m(cf(k::K_858CC4));
            !f.jne(0x41)
        };
        if big(MOVE_SPEED, &mut f) && big(MOVE_SPEED + 4, &mut f) && ru(this, PFLAGS) & 0x100 == 0 {
            f.fild(n);
            let nf = f.fstp();
            for j in 0..2isize {
                f.fld(va[j as usize]);
                f.fmul_m(cf(k::K_858C24));
                f.fdiv_m(nf);
                f.fsubr_m(rf(this, FRICTION_MOVE + 4 * j));
                wf(this, FRICTION_MOVE + 4 * j, f.fstp());
            }
            let mut t = [0f32; 3];
            for (j, t) in t.iter_mut().enumerate() {
                f.fld(vb[j]);
                f.fmul_m(cf(k::K_858EE4));
                *t = f.fstp();
            }
            let mut out = [0f32; 3];
            let r = vector_div_scalar(lp(&mut out).cast(), lp(&mut t).cast(), nf);
            vector_add_assign(this.byte_add(FRICTION_TURN), r);
        }
    }
    if etype(e) == ty::OBJECT && rb(e, 0x144) != 0 {
        f.fld(max_alt);
        f.fcomp_m(cf(k::K_858BA4));
        if !f.jne(0x41) {
            object_damage(e, max_alt, GPtr::from_addr(TEMP_COL_PTS), GPtr::from_addr(TEMP_COL_PTS + 0x10), this, 0x37);
        }
    }
    if GPtr::<u8>::from_addr(SECOND_SHIFT).read() != 0 {
        *result = 1;
        return None;
    }
    Some(1)
}

/// `CPhysical::ProcessCollisionSectorList(int32 x, int32 y)` (0x54BA60, `ret 8`, AL).
///
/// Sets the function-static guard bit (0xB73C90 bit 0). Bound centre and col-model radius; the lists
/// in the jump-table order of the bytes (0x54CFE0): sector `[clamp(y)*120 + clamp(x)]` buildings,
/// then repeat sector `[(y&15)*16 + (x&15)]` vehicles (+0), objects (+8), peds (+4). Per entity
/// (`bUsesCollision`, not itself, not the current scan code): not touching (`GetIsTouching`) clears
/// this' ignored-collision entity when it is that entity and not attached to it, and the same the
/// other way for a vehicle/ped/object. Touching: physical flag 0x1000 cleared; a building: the soft
/// flag from physical flag 0x40 with entity flag 0x10, the skip flag as in `ProcessShiftSectorList`;
/// others: virtual `SpecialEntityPreCollisionStuff(it, false, &skip, &noCol, &alt, &soft)`.
///
/// Without `bUsesCollision`, `noCol` or `skip`: scan code set, and unless `skip`
/// `ProcessEntityCollision`; with physical flag 0x10000, no `noCol` and points: true; no points with
/// it the ignored entity and this the player: the ignored entity cleared. A building, physical flag 8
/// or `alt`: the alternative path (`ApplyCollisionAlt` accumulating move/turn deltas averaged over the
/// hits, adhesion rules per vehicle type, status and model, collision torque friction, object damage
/// 55). Otherwise `ProcessEntityCollision` into 0xB73710 and the contact loops ([`pcsl_contacts`]) by
/// entity flag 8 of both (both: no friction; one: its friction speeds saved, zeroed, restored unless
/// a friction contact set flag 8 again; none: piece A checked twice). Then: a ped hit by this car
/// (not the player, or the player with entity flag 0x1000 and car `|v|² > 0.0025`):
/// `KillPedWithCar(this, max2, false)`; this ped hit by a train (`v . lastImpactVelocity > 0.2`, or
/// ped flag 0x100000 and train `|v|² > 0.0005`): `KillPedWithCar(train, 2*max1, false)`; car vs
/// object with collision damage effect: `ObjectDamage(max2, ...)` above 20, else for an effect >= 20
/// the crush test (the reverse case tests the car's byte +0x144, the original's bug). The other entity
/// at status 2 becomes 3 (`SwitchVehicleToRealPhysics`). With `bSecondShift` the result becomes true
/// and the walk goes on; else more points than soft contacts returns true.
#[hook(addr = 0x54BA60, conv = thiscall, name = "CPhysical::ProcessCollisionSectorList", expect = "6A FF 64 A1 00 00 00 00",
       this = "CPhysical", unverified = "mvpe-phys-1", route = "L2", hazards = "none", recs = "grove-walk,driving",
       note = "SEH frame with two local CMatrix (crush tests)")]
pub fn process_collision_sector_list(this: Ent, x: i32, y: i32) -> u8 {
    let guard = GPtr::<u32>::from_addr(SECTOR_LIST_GUARD);
    if guard.cast::<u8>().read() & 1 == 0 {
        guard.write(guard.read() | 1);
    }
    let mut s = PcslState { d: [-1.0, -1.0], max1: 0.0, max2: 0.0, soft: 0, b13: 0 };
    let mut result: u8 = 0;
    let model = rh(this, MODEL_INDEX) as i16 as i32;
    let mi = sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(model).read();
    let radius = rf(rp(mi, 0x14), 0x24);
    let mut centre = [0f32; 3];
    get_bound_centre_ref(this, lp(&mut centre).cast());
    let xs = clamp_sector(x);
    let ys = clamp_sector(y);
    let sector = GPtr::<u8>::from_addr(SECTORS + 8 * (ys * 120 + xs) as usize);
    let repeat = GPtr::<u8>::from_addr(REPEAT_SECTORS + 12 * ((((y & 0xF) << 4) + (x & 0xF)) as usize));
    let scan = GPtr::<u16>::from_addr(SCAN_CODE);
    for list in [sector, repeat, repeat.byte_add(8), repeat.byte_add(4)] {
        let mut node = rp(list, 0);
        while !node.is_null() {
            let e = rp(node, 0);
            node = rp(node, 4);
            if rb(e, EFLAGS) & 1 == 0 || e == this || rh(e, 0x2C) == scan.read() {
                continue;
            }
            if get_is_touching_vec(e, lp(&mut centre).cast(), radius) as u8 == 0 {
                if rp(this, IGNORED_COLLISION) == e && rp(this, ATTACHED_TO) != e {
                    wu(this, IGNORED_COLLISION, 0);
                }
                let t = etype(e);
                if t > 1 && t < 5 && rp(e, IGNORED_COLLISION) == this && rp(e, ATTACHED_TO) != this {
                    wu(e, IGNORED_COLLISION, 0);
                }
                continue;
            }
            let pf = ru(this, PFLAGS) & 0xFFFF_EFFF;
            let mut b12: u8 = 0;
            let mut b29: u8 = 0;
            let mut b2a: u8 = 0;
            s.b13 = 0;
            wu(this, PFLAGS, pf);
            if etype(e) == ty::BUILDING {
                if pf & 0x40 != 0 && (ru(this, EFLAGS) >> 4) & 1 != 0 {
                    s.b13 = 1;
                }
                if pf & 4 != 0 && (etype(this) != ty::VEHICLE || ru(this, 0x594) == 6) {
                    b12 = 1;
                } else {
                    let at = rp(this, ATTACHED_TO);
                    let at_t = if at.is_null() { 0 } else { etype(at) };
                    if (!at.is_null() && at_t > 1 && at_t < 5 && rb(at, PFLAGS) & 4 != 0)
                        || rp(this, IGNORED_COLLISION) == e
                        || (pf & 0x80 != 0 && pf & 2 == 0)
                    {
                        b12 = 1;
                    } else if pf & 0x0100_0000 != 0 {
                        let st = rb(this, TYPE) >> 3;
                        if st != 0 && st != 8 && does_not_collide_with_flyers(e) != 0 {
                            b12 = 1;
                        }
                    }
                }
            } else {
                special_entity_pre_collision_stuff_v(
                    this,
                    e,
                    0,
                    lp(&mut b12),
                    lp(&mut b29),
                    lp(&mut b2a),
                    lp(&mut s.b13),
                );
            }
            if rb(this, EFLAGS) & 1 == 0 || b29 != 0 || b12 != 0 {
                wh(e, 0x2C, scan.read());
                if b12 != 0 {
                    continue;
                }
                let n = process_entity_collision_v(this, e, GPtr::from_addr(TEMP_COL_PTS));
                if rb(this, PFLAGS + 2) & 1 != 0 && b29 == 0 && n > 0 {
                    return 1;
                }
                if n == 0 && rp(this, IGNORED_COLLISION) == e && this == find_player_ped(-1) {
                    wu(this, IGNORED_COLLISION, 0);
                }
                continue;
            }
            if etype(e) == ty::BUILDING || rb(e, PFLAGS) & 8 != 0 || b2a != 0 {
                if let Some(r) = pcsl_alt(this, e, &mut s, &mut result) {
                    return r;
                }
                continue;
            }
            wh(e, 0x2C, scan.read());
            let n = process_entity_collision_v(this, e, GPtr::from_addr(TEMP_COL_PTS));
            s.soft = 0;
            if n <= 0 {
                continue;
            }
            s.max1 = 0.0;
            s.max2 = 0.0;
            let t8 = (ru(this, EFLAGS) >> 3) & 1 != 0;
            let e8 = (ru(e, EFLAGS) >> 3) & 1 != 0;
            if t8 && e8 {
                pcsl_contacts(this, e, n, false, false, &mut s);
            } else if t8 || e8 {
                let who = if t8 { this } else { e };
                let fm = rv_bits(who, FRICTION_MOVE);
                let ft = rv_bits(who, FRICTION_TURN);
                wu(who, EFLAGS, ru(who, EFLAGS) & 0xFFFF_FFF7);
                wv_bits(who, FRICTION_MOVE, [0; 3]);
                wv_bits(who, FRICTION_TURN, [0; 3]);
                pcsl_contacts(this, e, n, true, false, &mut s);
                let w = ru(who, EFLAGS);
                if (w >> 3) & 1 == 0 {
                    wu(who, EFLAGS, w | 8);
                    wv_bits(who, FRICTION_MOVE, fm);
                    wv_bits(who, FRICTION_TURN, ft);
                }
            } else {
                pcsl_contacts(this, e, n, true, true, &mut s);
            }
            pcsl_after(this, e, &s);
            let t = rb(e, TYPE);
            if t & 0xF8 == 0x10 {
                let t = (t & 7) | 0x18;
                wb(e, TYPE, t);
                if t & 7 == ty::VEHICLE {
                    switch_vehicle_to_real_physics(e);
                }
            }
            if GPtr::<u8>::from_addr(SECOND_SHIFT).read() != 0 {
                result = 1;
                continue;
            }
            if n > s.soft {
                return 1;
            }
        }
    }
    result
}

/// The kill / object damage tests after the ordinary contact loops (0x54C586..0x54C8FB).
fn pcsl_after(this: Ent, e: Ent, s: &PcslState) {
    let mut f = Fpu::new();
    if etype(e) == ty::PED && etype(this) == ty::VEHICLE {
        let kill = if ped_is_player(e) != 0 {
            if ru(e, EFLAGS) & 0x1000 != 0 {
                f.push(X87::from_f64_exact(squared_magnitude(this.byte_add(MOVE_SPEED))));
                f.fcomp_m(cf(k::K_858CD8));
                !f.jne(0x41)
            } else {
                false
            }
        } else {
            true
        };
        if kill {
            kill_ped_with_car(e, this, s.max2, 0);
            return;
        }
    }
    let tt = etype(this);
    if tt == ty::PED && etype(e) == ty::VEHICLE && ru(e, 0x594) == 6 {
        f.push(X87::from_f64_exact(dot_product(e.byte_add(MOVE_SPEED), this.byte_add(LAST_IMPACT_VELOCITY))));
        f.fcomp_m(cf(k::K_858CC4));
        let mut kill = !f.jne(0x41);
        if !kill && ru(this, 0x474) & 0x10_0000 != 0 {
            f.push(X87::from_f64_exact(squared_magnitude(e.byte_add(MOVE_SPEED))));
            f.fcomp_m(cf(k::K_859978));
            kill = !f.jne(0x41);
        }
        if kill {
            f.fld(s.max1);
            f.fadd_st(0);
            kill_ped_with_car(this, e, f.fstp(), 0);
            return;
        }
    }
    let et = etype(e);
    if et == ty::OBJECT && rb(e, EFLAGS) & 1 != 0 && tt == ty::VEHICLE {
        let fx = rb(e, 0x144);
        if fx != 0 {
            f.fld(s.max2);
            f.fcomp_m(cf(k::K_858BA4));
            if !f.jne(0x41) {
                object_damage(e, s.max2, GPtr::from_addr(TEMP_COL_PTS), GPtr::from_addr(TEMP_COL_PTS + 0x10), this, 0x32);
                return;
            }
        }
        if fx >= 0x14 {
            pcsl_object_crush(e, this, false);
        }
    } else if tt == ty::OBJECT && rb(this, EFLAGS) & 1 != 0 && et == ty::VEHICLE {
        if rb(this, 0x144) != 0 {
            f.fld(s.max2);
            f.fcomp_m(cf(k::K_858BA4));
            if !f.jne(0x41) {
                object_damage(this, s.max2, GPtr::from_addr(TEMP_COL_PTS), GPtr::from_addr(TEMP_COL_PTS + 0x10), e, 0x32);
                return;
            }
        }
        if rb(e, 0x144) >= 0x14 {
            pcsl_object_crush(this, e, true);
        }
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_SHIFT_SECTOR_LIST,
    HOOK_PROCESS_COLLISION,
    HOOK_PROCESS_COLLISION_SECTOR_LIST_SIMPLE_CAR,
    HOOK_PROCESS_COLLISION_SECTOR_LIST,
];
