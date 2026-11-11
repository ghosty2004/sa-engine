//! WRITER-MVP-E batch `mvpe-veh-1`: vehicle collision entry points of the physics pass
//! (`CMonsterTruck::ProcessEntityCollision`, `CTrailer::ProcessEntityCollision`) and
//! `CQuadBike::ProcessDrivingAnims`. Ported from the 1.0 US bytes (SecuROM fragments inlined), x87
//! compares through the [`Fpu`] model; gta-reversed was a reading aid only. Unverified
//! (docs/verify/HOWTO.md).
//!
//! `CVehicle`/`CAutomobile` fields: +0x42B vehicle flags byte (bit 0 `bVehicleColProcessed`), +0x48C
//! last ground entity, +0x4C4/+0x4C8 the towing pair, +0x574 wheel lighting bytes, +0x724 wheel col
//! points (4 x 0x2C), +0x7D4 wheel compressions, +0x8E8 wheel ground entities, +0x8F8 wheel contact
//! offsets (4 x CVector); monster truck: +0x838 its own compression copies; trailer: +0x988 leg col
//! points (2 x 0x2C), +0x9E0 leg compressions, +0x9EC (-1000: no legs). `CCollisionData` +4 i16 boxes,
//! +6 i8 lines.
#![allow(clippy::unreadable_literal, clippy::needless_range_loop)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_world::collision::fpu::Fpu;
use sa_world::physical::attach::{allocate_matrix, process_col_models, simple_transform_update_matrix};
use sa_world::physical::shared::*;

orig! {
    /// `CBike::ProcessRiderAnims(CPed*, CVehicle*, CRideAnimData*, tBikeHandlingData*, int16)` (0x6B7280)
    pub fn process_rider_anims(ped: Ent, v: Ent, data: Ent, handling: u32, a5: u32) = cdecl 0x6B7280 => unported;
    /// `CPhysical::AddCollisionRecord(CEntity*)` (0x543490)
    pub fn add_collision_record(e: Ent, other: Ent) = thiscall 0x543490 => sa_world::physical::base::add_collision_record;
    /// `CAutomobile::ProcessEntityCollision(CEntity*, CColPoint*)` (0x6ACE70, EAX count)
    pub fn automobile_process_entity_collision(v: Ent, e: Ent, cps: Ent) -> i32 = thiscall 0x6ACE70 => unported;
    /// `CColPoint::operator=(const CColPoint&)` (stub 0x40FC80 → 0x15637E0)
    pub fn col_point_assign(dst: Ent, src: Ent) -> Ent = thiscall 0x40FC80 => unported;
}

/// `if (!e->m_matrix) { AllocateMatrix(); m_placement.UpdateMatrix(m_matrix); }`
fn ensure_matrix(e: Ent) {
    if matrix(e).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(PLACEMENT_POS), matrix(e));
    }
}

/// `CQuadBike::ProcessDrivingAnims(CPed* ped, bool)` (0x6CE280, `ret 8`): unless entity flag
/// 0x20000, `CBike::ProcessRiderAnims(ped, this, &this->m_rideAnimData (+0x98C), this->+0x988,
/// 0)`.
#[hook(addr = 0x6CE280, conv = thiscall, name = "CQuadBike::ProcessDrivingAnims", expect = "F7 41 1C 00 00 02 00",
       this = "CQuadBike", unverified = "mvpe-veh-1", route = "L2", hazards = "none", recs = "driving")]
pub fn quad_bike_process_driving_anims(this: Ent, ped: Ent, _b: u32) {
    if ru(this, EFLAGS) & 0x20000 != 0 {
        return;
    }
    process_rider_anims(ped, this, this.byte_add(0x98C), ru(this, 0x988), 0);
}

/// The ground contact bookkeeping of one wheel (shared by the monster truck and trailer loops):
/// lighting byte from the wheel col point (+0x25), a vehicle/object becomes the wheel's ground
/// entity (reference registered) with the contact offset `cp.point - its position`; the contact
/// surface (+0xBA) from surface B; a building becomes the last ground entity (+0x48C) and passes its
/// entity flag bits 30 and 31 on. `copy_light` (monster truck): a vehicle also passes its wheel
/// lighting byte.
fn wheel_contact(this: Ent, e: Ent, i: isize, copy_light: bool) {
    let cp = this.byte_add(0x724 + 0x2C * i);
    wb(this, 0x574 + i, rb(cp, 0x25));
    let et = etype(e);
    if et == ty::VEHICLE || et == ty::OBJECT {
        let slot = this.byte_add(0x8E8 + 4 * i);
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
        wv(this, 0x8F8 + 0xC * i, d);
        if copy_light && etype(e) == ty::VEHICLE {
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

/// The common tail: with points or wheel contacts both record the collision (a building does not);
/// with points a building or an object with physical flag 4 sets entity flag 0x1000. Returns `n`.
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

/// `CMonsterTruck::ProcessEntityCollision(CEntity* entity, CColPoint* colPoints)` (0x6C8AE0, `ret 8`,
/// EAX = number of points).
///
/// Not simple (status != 2): `bVehicleColProcessed`. The compressions at +0x838..+0x844 are copied
/// into the local ratio array; with physical flags 0x9000 or a ped the col data's line count is
/// zeroed first. `ProcessColModels(this, entity, colPoints, wheel col points +0x724, ratios, false)`
/// (the entity gets a matrix first). A zero line count is restored to 4 (no wheel pass). Otherwise
/// each wheel with `ratio < +0x838[i]` (and `bUsesCollision` or no points) counts, clears +0x7D4[i],
/// takes the ratio and does the ground contact bookkeeping. Then the collision records.
///
/// Stack: the original stores the entity matrix into its `entity` argument slot (S+4) and the wheel
/// offset pointer into the `colPoints` slot (S+8) (not reproduced).
#[hook(addr = 0x6C8AE0, conv = thiscall, name = "CMonsterTruck::ProcessEntityCollision", expect = "83 EC 24 56 8B F1",
       this = "CMonsterTruck", unverified = "mvpe-veh-1", route = "L2", hazards = "stack-residue", recs = "driving",
       note = "argument slots S+4/S+8 reused as locals (entity matrix, wheel offset pointer); nested hooks, no footprint")]
pub fn monster_truck_process_entity_collision(this: Ent, e: Ent, cps: Ent) -> i32 {
    if rb(this, TYPE) & 0xF8 != 0x10 {
        wb(this, 0x42B, rb(this, 0x42B) | 1);
    }
    let cm = get_col_model(this);
    let mut ratios: [u32; 4] = [ru(this, 0x838), ru(this, 0x83C), ru(this, 0x840), ru(this, 0x844)];
    let cd = rp(cm, 0x2C);
    let mut wheels: i32 = 0;
    if ru(this, PFLAGS) & 0x9000 != 0 || etype(e) == ty::PED {
        wb(cd, 6, 0);
    }
    ensure_matrix(e);
    let em = matrix(e);
    let tm = matrix(this);
    let ecm = get_col_model(e);
    let n = process_col_models(tm, cm, em, ecm, cps, this.byte_add(0x724), lp(&mut ratios).cast(), 0);
    if rb(cd, 6) == 0 {
        wb(cd, 6, 4);
    } else {
        for i in 0..4isize {
            let r = f32::from_bits(ratios[i as usize]);
            let mut f = Fpu::new();
            f.fld(r);
            f.fcomp_m(rf(this, 0x838 + 4 * i));
            if f.jne(0x41) {
                continue;
            }
            if rb(this, EFLAGS) & 1 == 0 && n != 0 {
                continue;
            }
            wheels += 1;
            wu(this, 0x7D4 + 4 * i, 0);
            wu(this, 0x838 + 4 * i, ratios[i as usize]);
            wheel_contact(this, e, i, true);
        }
    }
    collision_tail(this, e, n, wheels)
}

/// `CTrailer::ProcessEntityCollision(CEntity* entity, CColPoint* colPoints)` (0x6CFFD0, `ret 8`, EAX
/// = number of points).
///
/// Without legs (+0x9EC == -1000.0 as a dword compare) `CAutomobile::ProcessEntityCollision`.
/// Otherwise: not simple → `bVehicleColProcessed`; the four wheel compressions (+0x7D4) and the two
/// leg compressions (+0x9E0) copied into a local ratio array of 6; physical flags 0x9000, a ped or a
/// vehicle zero the line count. Against the towing pair (+0x4C4 / +0x4C8 == entity) both col datas'
/// box counts are zeroed over the call and restored. `ProcessColModels(this, entity, colPoints, local
/// line col points[6], ratios, false)`. A zero line count is restored to 6. Otherwise each wheel with
/// `ratio < 1.0` and `< +0x7D4[i]` counts, takes the ratio, its col point is copied (`CColPoint::
/// operator=`) and the ground bookkeeping runs; each leg the same against +0x9E0 with the col point
/// copied to +0x988. Then the collision records.
#[hook(addr = 0x6CFFD0, conv = thiscall, name = "CTrailer::ProcessEntityCollision", expect = "81 EC 44 01 00 00 56",
       this = "CTrailer", unverified = "mvpe-veh-1", route = "L2", hazards = "none", recs = "driving")]
pub fn trailer_process_entity_collision(this: Ent, e: Ent, cps: Ent) -> i32 {
    if ru(this, 0x9EC) == 0xC47A_0000 {
        return automobile_process_entity_collision(this, e, cps);
    }
    if rb(this, TYPE) & 0xF8 != 0x10 {
        wb(this, 0x42B, rb(this, 0x42B) | 1);
    }
    let cm = get_col_model(this);
    let cd = rp(cm, 0x2C);
    let mut ratios: [u32; 6] = [
        ru(this, 0x7D4),
        ru(this, 0x7D8),
        ru(this, 0x7DC),
        ru(this, 0x7E0),
        ru(this, 0x9E0),
        ru(this, 0x9E4),
    ];
    let mut wheels: i32 = 0;
    let et = etype(e);
    if ru(this, PFLAGS) & 0x9000 != 0 || et == ty::PED || et == ty::VEHICLE {
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
    ensure_matrix(e);
    let em = matrix(e);
    let tm = matrix(this);
    let mut lcps = [0u32; 6 * 0x2C / 4];
    let ecm = get_col_model(e);
    let n = process_col_models(tm, cm, em, ecm, cps, lp(&mut lcps).cast(), lp(&mut ratios).cast(), 0);
    if own_boxes >= 0 {
        wh(cd, 4, own_boxes as u16);
    }
    if other_boxes >= 0 {
        wh(rp(get_col_model(e), 0x2C), 4, other_boxes as u16);
    }
    if rb(cd, 6) == 0 {
        wb(cd, 6, 6);
        return collision_tail(this, e, n, wheels);
    }
    let lbase = lp(&mut lcps).cast::<u8>();
    let lcp = |i: usize| lbase.byte_add((0x2C * i) as isize);
    let below = |r: f32, field: f32| {
        let mut f = Fpu::new();
        f.fld(r);
        f.fcomp_m(1.0);
        if f.jp(5) {
            return false;
        }
        f.fld(r);
        f.fcomp_m(field);
        !f.jp(5)
    };
    for i in 0..4usize {
        let r = f32::from_bits(ratios[i]);
        if !below(r, rf(this, 0x7D4 + 4 * i as isize)) {
            continue;
        }
        wheels += 1;
        wu(this, 0x7D4 + 4 * i as isize, ratios[i]);
        col_point_assign(this.byte_add(0x724 + 0x2C * i as isize), lcp(i));
        wheel_contact(this, e, i as isize, false);
    }
    for i in 0..2usize {
        let r = f32::from_bits(ratios[4 + i]);
        if !below(r, rf(this, 0x9E0 + 4 * i as isize)) {
            continue;
        }
        wheels += 1;
        wu(this, 0x9E0 + 4 * i as isize, ratios[4 + i]);
        col_point_assign(this.byte_add(0x988 + 0x2C * i as isize), lcp(4 + i));
    }
    collision_tail(this, e, n, wheels)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_QUAD_BIKE_PROCESS_DRIVING_ANIMS,
    HOOK_MONSTER_TRUCK_PROCESS_ENTITY_COLLISION,
    HOOK_TRAILER_PROCESS_ENTITY_COLLISION,
];
