//! `CPed::ProcessEntityCollision`: the ped's collision against one entity of the physics pass (ground
//! lines, landing damage, the col point filtering). Ported from the 1.0 US bytes (SecuROM fragments
//! inlined), x87 sequences through the [`Fpu`] model; gta-reversed was a reading aid only. Unverified
//! (docs/verify/HOWTO.md).
//!
//! `CPed` fields: +0x12C surface lighting, +0x46C/+0x470/+0x474/+0x478 ped flag words, +0x47C
//! intelligence, +0x530 ped state, +0x568 ground physical, +0x56C its contact offset, +0x578 ground
//! normal, +0x584 last ground entity, +0x588 the ground z, +0x58C, +0x598 ped type (0/1 players).
//! `CColModel` +0x8 bounding box min.z, +0x14 max.z, +0x24 sphere radius, +0x2C col data (`+6` line
//! count, `+8` spheres (0x14 each), `+0x10` lines (0x20 each)). `CColPoint` 0x2C (normal +0x10,
//! surface A +0x20, piece A +0x21, surface B +0x23, lighting B +0x25, depth +0x28).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::collision::fpu::Fpu;
use sa_world::physical::attach::{allocate_matrix, process_col_models, simple_transform_update_matrix};
use sa_world::physical::shared::*;

orig! {
    /// `CPhysical::AddCollisionRecord(CEntity*)` (0x543490)
    fn add_collision_record(e: Ent, other: Ent) = thiscall 0x543490 => sa_world::physical::base::add_collision_record;
    /// `CColPoint::operator=(const CColPoint&)` (stub 0x40FC80 → 0x15637E0)
    fn col_point_assign(dst: Ent, src: Ent) -> Ent = thiscall 0x40fc80 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn normalise(v: GPtr<sa_math::vector::CVector>) = thiscall 0x59c910 => sa_world::collision::data::normalise_port;
    /// `CPedIntelligence::GetTaskUseGun()` (0x600F70)
    fn get_task_use_gun(intel: Ent) -> Ent = thiscall 0x600f70 => unported;
    /// `CPlaceable::GetHeading()` (0x441DB0, ST0)
    fn get_heading(e: Ent) -> f64 = thiscall 0x441db0 => unported;
    /// `CPlaceable::SetHeading(float)` (0x43E0C0)
    fn set_heading(e: GPtr<u8>, h: f32) = thiscall 0x43e0c0 => sa_world::world::entity::set_heading;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020)
    fn matrix_set_rotate_z_only(m: Ent, h: f32) = thiscall 0x59b020 => unported;
    /// `SurfaceInfos_c::IsSteepSlope(uint32)` (0x55E770)
    fn is_steep_slope(si: Ent, id: u32) -> u32 = thiscall 0x55e770 => is_steep_slope_port;
    /// `SurfaceInfos_c::IsSoftLanding(uint32)` (0x55E690)
    fn is_soft_landing(si: Ent, id: u32) -> u32 = thiscall 0x55e690 => is_soft_landing_port;
    /// `CEntity::GetIsTypePhysical()` (0x4DA030)
    fn get_is_type_physical(e: GPtr<u8>) -> u32 = thiscall 0x4da030 => sa_world::world::entity::get_is_type_physical;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, uint32 animId)` (0x4D68B0)
    fn clump_get_association(clump: Ent, id: u32) -> Ent = cdecl 0x4d68b0 => unported;
    /// `CPed::GetLocalDirection(const CVector2D&)` (0x5DEF60)
    fn get_local_direction(p: GPtr<u8>, v: GPtr<f32>) -> i32 = thiscall 0x5def60 => crate::ped::get_local_direction;
    /// `CEventDamage::CEventDamage(CEntity*, uint32 time, eWeaponType, ePedPieceTypes, uint8 dir, bool, bool)`
    /// (0x4AD830)
    fn event_damage_ctor(ev: Ent, src: Ent, time: u32, weapon: u32, piece: u32, dir: u32, a: u32, b: u32) -> Ent
        = thiscall 0x4ad830 => unported;
    /// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0, AL)
    fn event_damage_affects_ped(ev: Ent, ped: Ent) -> u8 = thiscall 0x4b35a0 => unported;
    /// `CEventDamage::~CEventDamage()` (0x4AD960)
    fn event_damage_dtor(ev: Ent) = thiscall 0x4ad960 => unported;
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, eWeaponType,
    /// ePedPieceTypes, bool)` (0x4AD3F0)
    fn calc_ctor(c: GPtr<u8>, damager: u32, damage: u32, weapon: u32, piece: u32, speak: u32) -> GPtr<u8>
        = thiscall 0x4ad3f0 => crate::damage::constructor;
    /// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse&, bool)` (0x4B5AC0)
    fn calc_compute(c: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>, speak: u32) = thiscall 0x4b5ac0 => crate::damage::compute_damage_response;
    /// `CPedDamageResponseCalculator::~CPedDamageResponseCalculator()` (0x4AD420)
    fn calc_dtor(c: Ent) = thiscall 0x4ad420 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: Ent, e: Ent, b: u32) -> Ent = thiscall 0x4ab420 => unported;
}

/// Phases B/C side of `IsSteepSlope` 0x55E770.
pub fn is_steep_slope_port(si: Ent, id: u32) -> u32 {
    sa_world::collision::surface::is_steep_slope(si.cast(), id)
}

/// Phases B/C side of `IsSoftLanding` 0x55E690.
pub fn is_soft_landing_port(si: Ent, id: u32) -> u32 {
    sa_world::collision::surface::is_soft_landing(si.cast(), id)
}

/// `g_surfaceInfos` (0xB79538)
const SURFACE_INFOS: usize = 0xB79538;
/// The ped's static ground-hit col model used while aiming in the first person look (0x968E20).
const STATIC_PED_COL_MODEL: usize = 0x968E20;
/// `CPed` landing / fall damage tweakables (.data): horizontal and vertical speed limits.
const FALL_SPEED_XY: usize = 0x86C290;
const FALL_SPEED_Z: usize = 0x86C294;

/// An f32 of the exe image (constants and tweakables, read where the original reads them).
fn kf(a: usize) -> f32 {
    GPtr::<f32>::from_addr(a).read()
}

fn ensure_matrix(e: Ent) {
    if matrix(e).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(PLACEMENT_POS), matrix(e));
    }
}

/// `fld v; fcomp m; test ah, 5; jp` not taken: `v < m`.
fn lt(v: f32, m: f32) -> bool {
    let mut f = Fpu::new();
    f.fld(v);
    f.fcomp_m(m);
    !f.jp(5)
}

/// `fld v; fcomp m; test ah, 0x41; je` taken: `v > m`.
fn gt(v: f32, m: f32) -> bool {
    let mut f = Fpu::new();
    f.fld(v);
    f.fcomp_m(m);
    !f.jne(0x41)
}

/// `CEventDamage` (0x44 bytes) on the original's stack.
type EventBuf = [u32; 0x11];

/// One fall-damage event (0x5E2F23 / 0x5E2FF0): `CEventDamage(e, time, 0x36 fall, 3 torso, dir, 0,
/// 0)`; if it affects the ped a `CPedDamageResponseCalculator(e, damage, 0x36, 3, false)` computes
/// the response into the event (+0x38) and the event goes to the intelligence's event group (+0x68);
/// destructors as the original runs them.
fn fall_damage_event(this: Ent, e: Ent, dir: u32, damage: u32) {
    let mut ev: EventBuf = [0; 0x11];
    let evp = lp(&mut ev).cast::<u8>();
    event_damage_ctor(evp, e, sa_core::timer::TIME_MS.get(), 0x36, 3, dir, 0, 0);
    if event_damage_affects_ped(evp, this) != 0 {
        let mut calc = [0u32; 5];
        let cp = lp(&mut calc).cast::<u8>();
        calc_ctor(cp, e.addr() as u32, damage, 0x36, 3, 0);
        calc_compute(cp, this, evp.byte_add(0x38), 1);
        event_group_add(rp(this, 0x47C).byte_add(0x68), evp, 0);
        calc_dtor(cp);
    }
    event_damage_dtor(evp);
}

/// `CPed::ProcessEntityCollision(CEntity* entity, CColPoint* colPoints)` (0x5E2530, `ret 8`, EAX =
/// number of points).
///
/// Col model: the model info's (+0x14); with ped flag +0x474 bit 15 and `bUsesCollision` the static
/// 0x968E20 one (a player aiming a gun-task with flag bit 1 in camera mode 0x35 first turns to the
/// camera: heading saved, `SetHeading(atan2(-front.x, front.y))`). Neither `bUsesCollision` nor
/// physical flag 0x10000: 0. Physical flags 0x19000, attached or a ped entity: no lines, bounding
/// radius 1, min z -1, max z 0.95. Else entity flag 2 set, line 0 from z 0 down to -1 (+ timeStep *
/// -0.15 with +0x46C bit 1), top = sphere 2 z + radius; with +0x478 bit 8 a second line around the
/// top and two lines. `ProcessColModels(this, entity, colPoints, lines[2], ratios {1, 1}, flag 0x10
/// and (building or physical flag 4))`, then the col model restored. Ground line hit (ratio 0 < 1):
/// steep test over the points (normal z < -0.867), the lighting (+0x12C, players blended by
/// timeStep * 0.1), the ground physical / last ground entity bookkeeping, the ground z (+0x588), the
/// ped's z put on the ground (capped by the second line), normal, surface (+0xBA) and steep slope flag
/// (+0x470 bit 5), then the landing: relative speed against the speed limits (doubled in state 1)
/// gives fall damage events (`CEventDamage`, `CPedDamageResponseCalculator`), +0x46C bit 0, speed z
/// 0, and a player lifted more than 0.1 collides again without lines. Otherwise only the second
/// line's ground z. Points: a player with bit 15 drops points of piece > 2 (setting the hit flags
/// +0x474 bits 16..19 by piece 6/5/8/4), a train with physical flag 4 flattens downward normals. The
/// saved heading is restored (`SetRotateZOnly`). Static entities with +0x46C bit 1: steep normals of
/// piece 2 (players) turn into a push against the horizontal speed, others are flattened; a sloped
/// ground normal adds one point under the ped. Then the collision records.
///
/// The C++ EH frame (states 0..3 at S-0xC) is not reproduced; the destructors run as on the normal
/// path.
#[hook(addr = 0x5e2530, conv = thiscall, name = "CPed::ProcessEntityCollision", expect = "6A FF 68 46 D9 83 00",
       this = "CPed", unverified = "mvpf-ped-1", route = "L2", hazards = "heap-writes", recs = "grove-walk,driving",
       note = "writes the static col model 0x968E20 and the shared col model of the ped model; ProcessColModels' bool argument is a dword with uninitialised upper bytes in the original")]
pub fn ped_process_entity_collision(this: Ent, e: Ent, cps: Ent) -> i32 {
    let mut f = Fpu::new();
    f.fld(sa_core::timer::TIME_STEP.get());
    f.fmul_m(kf(0x86C330));
    let ts_k = f.fstp();
    let mut ratios: [f32; 2] = [1.0, 1.0];
    let model = rh(this, MODEL_INDEX) as i16 as i32;
    let mi = sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(model).read();
    let mut cm = rp(mi, 0x14);
    let mut cd = rp(cm, 0x2C);
    let mut nvec: [f32; 3] = [0.0, 0.0, 1.0];
    let mut on_boat = false;
    let mut lines_set = false;
    let mut top = cf(0x3F70_A3D7);
    let mut heading = cf(0xC47A_4000);
    let uses_col = ru(this, EFLAGS) & 1 != 0;
    if ru(this, 0x474) & 0x8000 != 0 && uses_col {
        let pt = ru(this, 0x598);
        if (pt == 0 || pt == 1) && cam_mode() == 0x35 {
            let intel = rp(this, 0x47C);
            if !get_task_use_gun(intel).is_null()
                && !rp(get_task_use_gun(intel), 0x30).is_null()
                && (ru(rp(get_task_use_gun(intel), 0x30), 0x18) >> 1) & 1 != 0
            {
                heading = X87::from_f64_exact(get_heading(this)).to_f32();
                let i = usize::from(GPtr::<u8>::from_addr(0xB6F081).read()) * 0x238;
                let y = X87::from_f32(kf(0xB6F330 + i));
                let x = X87::from_f32(kf(0xB6F32C + i)).fchs();
                set_heading(this, x.fpatan(y).to_f32());
            }
        }
        cd = GPtr::<GPtr<u8>>::from_addr(STATIC_PED_COL_MODEL + 0x2C).read();
        cm = GPtr::from_addr(STATIC_PED_COL_MODEL);
    }
    if !uses_col && rb(this, 0x42) & 1 == 0 {
        return 0;
    }
    if etype(e) == ty::VEHICLE && ru(e, 0x590) == 5 {
        on_boat = true;
    }
    if ru(this, PFLAGS) & 0x19000 != 0 || !rp(this, ATTACHED_TO).is_null() || etype(e) == ty::PED {
        wb(cd, 6, 0);
        wu(cm, 0x24, 0x3F80_0000);
        wu(cm, 0x14, 0x3F73_3333);
        wu(cm, 8, 0xBF80_0000);
    } else {
        if (ru(this, EFLAGS) >> 1) & 1 == 0 {
            wu(this, EFLAGS, ru(this, EFLAGS) | 2);
        }
        wu(rp(cd, 0x10), 8, 0);
        wu(rp(cd, 0x10), 0x18, 0xBF80_0000);
        lines_set = true;
        if (ru(this, 0x46C) >> 1) & 1 != 0 {
            let l = rp(cd, 0x10);
            f.fld(ts_k);
            f.fadd_m(rf(l, 0x18));
            wf(l, 0x18, f.fstp());
        }
        let s = rp(cd, 8);
        f.fld(rf(s, 0x34));
        f.fadd_m(rf(s, 0x30));
        top = f.fstp();
        if ru(this, 0x478) & 0x100 != 0 {
            f.fld(rf(s, 8));
            f.fsub_m(rf(s, 0xC));
            wf(rp(cd, 0x10), 0x28, top);
            f.fadd_m(cf(0x3F80_0000));
            let l = rp(cd, 0x10);
            f.fld(rf(l, 0x28));
            f.fsub_st(1);
            wf(l, 0x28, f.fstp());
            wf(rp(cd, 0x10), 0x38, top);
            let l = rp(cd, 0x10);
            f.fadd_m(rf(l, 0x38));
            wf(l, 0x38, f.fstp());
        }
        let l = rp(cd, 0x10);
        if ru(this, 0x478) & 0x100 != 0 {
            wb(cd, 6, 2);
            wu(cm, 0x24, ru(l, 0x38));
            wu(cm, 0x14, ru(rp(cd, 0x10), 0x38));
            wu(cm, 8, ru(rp(cd, 0x10), 0x18));
        } else {
            wb(cd, 6, 1);
            wu(cm, 0x14, 0x3F73_3333);
            f.fld(rf(l, 0x18));
            f.fabs();
            wf(cm, 0x24, f.fstp());
            wu(cm, 8, ru(rp(cd, 0x10), 0x18));
        }
    }
    let b20 = (ru(this, EFLAGS) >> 4) & 1 != 0 && (etype(e) == ty::BUILDING || rb(e, PFLAGS) & 4 != 0);
    ensure_matrix(e);
    // the two line col points (the original's locals at S-0xAC and S-0x80)
    let mut lcps = [0u32; 22];
    let lcp = lp(&mut lcps).cast::<u8>();
    let em = matrix(e);
    let tm = matrix(this);
    let ecm = get_col_model(e);
    let mut n = process_col_models(tm, cm, em, ecm, cps, lcp, lp(&mut ratios).cast(), u32::from(b20));
    let old_z = rf(matrix(this), 0x38);
    let lz = |o: isize| rf(lcp, o);
    if lines_set {
        wb(cd, 6, 0);
        wu(cm, 0x24, 0x3F80_0000);
        wu(cm, 8, 0xBF80_0000);
        wu(cm, 0x14, 0x3F73_3333);
        if lt(ratios[0], 1.0) {
            let b43 = rb(this, 0x46C) & 1;
            let mut skip_ground = false;
            if b43 != 0 {
                f.fld(lz(8));
                f.fadd_m(cf(0x3F80_0000));
                f.fcomp_m(rf(matrix(this), 0x38));
                if f.jne(0x41) {
                    if !on_boat {
                        skip_ground = true;
                    } else {
                        f.fld(lz(8));
                        f.fadd_m(cf(0x4040_0000));
                        f.fcomp_m(rf(matrix(this), 0x38));
                        skip_ground = f.jne(0x41);
                    }
                }
            }
            if !skip_ground {
                ground_contact(this, e, cps, n, lcp, b43, top, ratios[1], &mut nvec);
            }
            // 0x5E2D06: the landing
            let mut lim_xy = kf(FALL_SPEED_XY);
            let mut lim_z = kf(FALL_SPEED_Z);
            if ru(this, 0x530) == 1 {
                let mut g = Fpu::new();
                g.fld(kf(FALL_SPEED_XY));
                g.fadd_st(0);
                lim_xy = g.fstp();
                g.fld(kf(FALL_SPEED_Z));
                g.fmul_m(cf(0x3FC0_0000));
                lim_z = g.fstp();
            }
            let fall_anim = clump_get_association(rp(this, RW_OBJECT), 0x78);
            let mut sp = rv(this, MOVE_SPEED);
            let et = etype(e);
            if et > 1 && et < 5 {
                for (j, s) in sp.iter_mut().enumerate() {
                    f.fld(*s);
                    f.fsub_m(rf(e, MOVE_SPEED + 4 * j as isize));
                    *s = f.fstp();
                }
            }
            let falling = (ru(this, 0x46C) >> 1) & 1 != 0;
            let mut hit = false;
            if !falling {
                f.fld(sp[1]);
                f.fmul_m(sp[1]);
                f.fld(sp[0]);
                f.fmul_m(sp[0]);
                f.faddp(1);
                f.fsqrt();
                let h = f.fst();
                f.fcomp_m(lim_xy);
                let fast_xy = !f.jne(0x41) && ru(this, 0x470) & 0x0800_0000 == 0;
                if (fast_xy || lt(sp[2], lim_z)) && rp(this, IGNORED_COLLISION) != e && rp(this, 0x58C) != e {
                    hit = true;
                    let mut l20 = kf(FALL_SPEED_Z);
                    if is_soft_landing(GPtr::from_addr(SURFACE_INFOS), u32::from(rb(lcp, 0x23))) as u8 != 0 {
                        f.fld(kf(0x86C324));
                        let mut g = Fpu::new();
                        g.fld(l20);
                        g.fmul_m(cf(0x3FC0_0000));
                        l20 = g.fstp();
                    } else {
                        f.fld(cf(0x3E80_0000));
                    }
                    f.fsubr_m(h);
                    let clamp0 = |f: &mut Fpu| {
                        f.fld(0.0);
                        f.fcomp_st(1);
                        if !f.jne(0x41) {
                            f.fstp_st(0);
                            f.fld(0.0);
                        }
                    };
                    clamp0(&mut f);
                    f.fmul_m(kf(0x86C298));
                    f.fld(l20);
                    f.fsub_m(sp[2]);
                    clamp0(&mut f);
                    f.fmul_m(kf(0x86C29C));
                    f.fadd_st(1);
                    let mut damage = f.fstp();
                    f.fstp_st(0);
                    if lt(sp[2], kf(0x86C2A0)) {
                        damage = cf(0x43FA_0000);
                    }
                    let mut dir: u32 = 0x3E80_0002;
                    let k310 = kf(0x86C310);
                    if gt(sp[0], cf(0x3C23_D70A)) || lt(sp[0], k310) || gt(sp[1], cf(0x3C23_D70A)) || lt(sp[1], k310) {
                        let mut g = Fpu::new();
                        g.fld(sp[1]);
                        g.fmul_m(cf(0xBF80_0000));
                        g.fld(sp[0]);
                        g.fmul_m(cf(0xBF80_0000));
                        let mut v2 = [0f32; 2];
                        v2[0] = g.fstp();
                        v2[1] = g.fstp();
                        let d = get_local_direction(this, lp(&mut v2).cast());
                        dir = 0x3E80_0000 | (d as u32 & 0xFF);
                    }
                    fall_damage_event(this, e, dir, damage.to_bits());
                }
            }
            if !hit && !falling && !fall_anim.is_null() {
                let mut g = Fpu::new();
                g.fld(sa_core::timer::TIME_STEP.get());
                g.fmul_m(kf(0x86C320));
                g.fcomp_m(sp[2]);
                if !g.jne(0x41) && rp(this, 0x58C) != e {
                    fall_damage_event(this, e, 2, 0x4170_0000);
                }
            }
            wu(this, 0x46C, ru(this, 0x46C) | 1);
            wu(this, MOVE_SPEED + 8, 0);
            f.fld(old_z);
            f.fadd_m(cf(0x3DCC_CCCD));
            f.fcomp_m(rf(matrix(this), 0x38));
            let pt = ru(this, 0x598);
            if !f.jp(5) && rb(cd, 6) == 0 && (pt == 0 || pt == 1) {
                ensure_matrix(e);
                let em = matrix(e);
                let tm = matrix(this);
                let ecm = get_col_model(e);
                n = process_col_models(tm, cm, em, ecm, cps, Ent::NULL, Ent::NULL, 0);
            }
        } else {
            // 0x5E3138: only the second line
            let z1 = lz(0x2C + 8);
            if ru(this, 0x478) & 0x100 != 0
                && lt(ratios[1], 1.0)
                && lt(z1, rf(this, 0x588))
                && (get_is_type_physical(e) as u8 == 0 || rb(e, PFLAGS) & 8 != 0)
            {
                wf(this, 0x588, z1);
                if rb(this, 0x46C) & 1 != 0 {
                    let m = matrix(this);
                    f.fld(top);
                    f.fadd_m(rf(pos_ptr(this), 8));
                    f.fcomp_m(z1);
                    if !f.jne(0x41) {
                        f.fld(z1);
                        f.fsub_m(top);
                        wf(m, 0x38, f.fstp());
                    }
                }
            }
            wu(this, 0x470, ru(this, 0x470) & 0xFFFF_FFFD);
        }
    }
    // 0x5E31C6: the point filter
    let mut i: i32 = 0;
    let mut last = n - 1;
    while i < n {
        let cp = cps.byte_add(0x2C * i as isize);
        if ru(this, 0x474) & 0x8000 != 0 && ru(this, EFLAGS) & 1 != 0 && rb(cp, 0x21) > 2 {
            let (mut cl, mut dl, mut b13) = (true, true, true);
            if etype(e) == ty::PED {
                b13 = false;
                if ru(e, 0x474) & 0x10_0000 != 0 {
                    cl = false;
                } else if ru(e, 0x46C) & 0x400_0000 != 0 {
                    dl = false;
                }
            }
            let flags = ru(this, 0x474);
            let bit = match rb(cp, 0x21) {
                6 if cl && dl => 0x1_0000,
                5 if cl && dl => 0x2_0000,
                8 if cl => 0x4_0000,
                4 if b13 => 0x8_0000,
                _ => 0,
            };
            if bit != 0 {
                wu(this, 0x474, flags | bit);
            }
            if i < last {
                for k in i..last {
                    let d = cps.byte_add(0x2C * k as isize);
                    col_point_assign(d, d.byte_add(0x2C));
                }
                i -= 1;
            }
            n -= 1;
            last -= 1;
        } else if etype(e) == ty::VEHICLE && ru(e, 0x594) == 6 && rb(e, PFLAGS) & 4 != 0 && lt(rf(cp, 0x18), 0.0) {
            wu(cp, 0x18, 0);
            normalise(cp.byte_add(0x10).cast());
        }
        i += 1;
    }
    {
        let mut g = Fpu::new();
        g.fld(heading);
        g.fcomp_m(-1000.0);
        if !g.jne(0x41) {
            let m = matrix(this);
            if !m.is_null() {
                matrix_set_rotate_z_only(m, heading);
            } else {
                wf(this, 0x10, heading);
            }
        }
    }
    let et = etype(e);
    let ef = ru(e, EFLAGS);
    if (et == ty::BUILDING || ef & 4 != 0 || ef & 0x40000 != 0) && (ru(this, 0x46C) >> 1) & 1 != 0 && n > 0 {
        for k in 0..n {
            let nrm = cps.byte_add(0x2C * k as isize + 0x10);
            let mut v = rv(nrm, 0);
            let pt = ru(this, 0x598);
            if lt(v[2], kf(0x86C314)) && rb(nrm, 0x11) == 2 && (pt == 0 || pt == 1) {
                let mut g = Fpu::new();
                g.fld(rf(this, MOVE_SPEED + 4));
                g.fld(rf(this, MOVE_SPEED));
                g.fld_st(0);
                g.fmul_st(1);
                g.fld_st(2);
                g.fmul_st(3);
                g.faddp(1);
                g.fsqrt();
                g.fstp_st(2);
                g.fstp_st(0);
                g.fcom_m(cf(0x3A83_126F));
                if !g.jp(5) {
                    g.fstp_st(0);
                    g.fld(cf(0x3A83_126F));
                }
                g.fdivr_m(cf(0xBF80_0000));
                g.fld_st(0);
                g.fmul_m(rf(this, MOVE_SPEED));
                v[0] = g.fstp();
                g.fmul_m(rf(this, MOVE_SPEED + 4));
                v[1] = g.fstp();
                let m = matrix(this);
                g.fld(rf(m, 0x38));
                g.fsub_m(cf(0x3D4C_CCCD));
                wf(m, 0x38, g.fstp());
                wu(this, 0x470, ru(this, 0x470) | 0x20_0020);
            } else {
                let mut g = Fpu::new();
                g.fld(v[1]);
                g.fmul_m(v[1]);
                g.fld(v[0]);
                g.fmul_m(v[0]);
                g.faddp(1);
                g.fsqrt();
                g.fcom_m(0.0);
                if g.jp(0x44) {
                    g.fdivr_m(cf(0x3F80_0000));
                    g.fld(v[0]);
                    g.fmul_st(1);
                    v[0] = g.fstp();
                    g.fld(v[1]);
                    g.fmul_st(1);
                    v[1] = g.fstp();
                }
                g.fstp_st(0);
            }
            normalise(lp(&mut v).cast());
            wv(nrm, 0, v);
            if is_steep_slope(GPtr::from_addr(SURFACE_INFOS), u32::from(rb(nrm, 0x13))) as u8 != 0 {
                wu(this, 0x470, ru(this, 0x470) | 0x20);
            }
        }
    }
    if lt(nvec[2], 1.0) {
        let cp = cps.byte_add(0x2C * n as isize);
        let nrm = cp.byte_add(0x10);
        wf(nrm, 0, nvec[0]);
        wf(nrm, 4, nvec[1]);
        wu(nrm, 8, 0);
        normalise(nrm.cast());
        let k = cf(0x3EB3_3333);
        let mut g = Fpu::new();
        g.fld(rf(nrm, 0));
        g.fmul_m(k);
        g.fld(rf(nrm, 4));
        g.fmul_m(k);
        g.fld(rf(nrm, 8));
        g.fmul_m(k);
        let kz = g.fstp();
        let pos = pos_ptr(this);
        g.fld(rf(pos, 0));
        g.fsub_st(2);
        let px = g.fstp();
        g.fld(rf(pos, 4));
        g.fsub_st(1);
        let py = g.fstp();
        g.fstp_st(0);
        g.fstp_st(0);
        g.fld(rf(pos, 8));
        g.fsub_m(kz);
        let pz = g.fstp();
        wv(cp, 0, [px, py, pz]);
        n += 1;
        wu(this, 0x470, ru(this, 0x470) | 0x20);
    }
    if n > 0 || lt(ratios[0], 1.0) {
        add_collision_record(this, e);
        if etype(e) != ty::BUILDING {
            add_collision_record(e, this);
        }
        if n > 0 {
            let ef = ru(e, EFLAGS);
            if etype(e) == ty::BUILDING || ef & 4 != 0 || ef & 0x40000 != 0 {
                wu(this, EFLAGS, ru(this, EFLAGS) | 0x1000);
            }
        }
    }
    n
}

/// `TheCamera.m_aCams[m_nActiveCam].m_nMode` (0xB6F1A8 + 0x238 * byte 0xB6F081).
fn cam_mode() -> u16 {
    let i = usize::from(GPtr::<u8>::from_addr(0xB6F081).read()) * 0x238;
    GPtr::<u16>::from_addr(0xB6F1A8 + i).read()
}

/// 0x5E28DB..0x5E2D06 of `CPed::ProcessEntityCollision`: the ground line hit. Steep flag over the
/// points (normal z < -0.867); lighting +0x12C from the line point's lighting byte (+0x25: low and
/// high nibble * 1/30 mixed by 0x8D12C0; players blend it in by `timeStep * 0.1`); unless +0x46C bit
/// 0: a vehicle / object becomes the ground physical (+0x568, contact offset +0x56C, a boat sets
/// +0x470 bit 1, a vehicle passes its lighting) and the last ground entity (+0x584); else only the
/// latter, +0x470 bits 1 and 7 cleared. The second line's z lowers +0x588 (against a static or
/// physical-flag-8 entity). Unless steep (and with +0x470 bit 21 only while above the hit or on a
/// building moving away from it), the ped's matrix z goes to `hit.z + 1` (capped by `+0x588 - top`
/// when that is set, the hit normal then saved), else with bit 21 a non-vehicle hit saves the
/// normal. Then the ground normal +0x578, the contact surface (+0xBA) and the steep slope flag.
#[allow(clippy::too_many_arguments)]
fn ground_contact(this: Ent, e: Ent, cps: Ent, n: i32, lcp: Ent, b43: u8, top: f32, r1: f32, nvec: &mut [f32; 3]) {
    let mut steep = false;
    for k in 0..n.max(0) {
        if lt(rf(cps, 0x2C * k as isize + 0x18), kf(0x86C32C)) {
            steep = true;
        }
    }
    let light = u32::from(rb(lcp, 0x25));
    let k30 = cf(0x3D08_8889);
    let mut f = Fpu::new();
    let pt = ru(this, 0x598);
    let player = pt == 0 || pt == 1;
    if player {
        f.fld(sa_core::timer::TIME_STEP.get());
        f.fmul_m(kf(0x8D21E4));
    }
    f.fild((light & 0xF) as i32);
    f.fmul_m(k30);
    f.fild((light >> 4) as i32);
    f.fmul_m(k30);
    f.fld(kf(0x8D12C0));
    f.fld(cf(0x3F80_0000));
    f.fsub_st(1);
    f.fmul_st(3);
    f.fxch(1);
    f.fmul_st(2);
    f.faddp(1);
    if player {
        f.fmul_st(3);
        f.fld(cf(0x3F80_0000));
        f.fsub_st(4);
        f.fmul_m(rf(this, 0x12C));
        f.faddp(1);
        wf(this, 0x12C, f.fstp());
        f.fstp_st(0);
        f.fstp_st(0);
        f.fstp_st(0);
    } else {
        wf(this, 0x12C, f.fstp());
        f.fstp_st(0);
        f.fstp_st(0);
    }
    let et = etype(e);
    if b43 == 0 && (et == ty::VEHICLE || et == ty::OBJECT) {
        let slot = this.byte_add(0x568);
        wp(slot, 0, e);
        register_reference(e, slot.cast());
        let pos = pos_ptr(e);
        f.fld(rf(lcp, 8));
        f.fsub_m(rf(pos, 8));
        f.fld(rf(lcp, 4));
        f.fsub_m(rf(pos, 4));
        f.fld(rf(lcp, 0));
        f.fsub_m(rf(pos, 0));
        wp(this, 0x584, e);
        wf(this, 0x56C, f.fstp());
        wf(this, 0x570, f.fstp());
        wf(this, 0x574, f.fstp());
        register_reference(e, this.byte_add(0x584).cast());
        if etype(e) == ty::VEHICLE && ru(e, 0x590) == 5 {
            wu(this, 0x470, ru(this, 0x470) | 2);
        } else {
            wu(this, 0x470, ru(this, 0x470) & 0xFFFF_FFFD);
        }
        if etype(e) == ty::VEHICLE {
            wu(this, 0x12C, ru(rp(this, 0x568), 0x12C));
        }
    } else {
        wp(this, 0x584, e);
        register_reference(e, this.byte_add(0x584).cast());
        wu(this, 0x470, ru(this, 0x470) & 0xFFFF_FF7D);
    }
    let z1 = rf(lcp, 0x2C + 8);
    if ru(this, 0x478) & 0x100 != 0 && lt(r1, 1.0) && lt(z1, rf(this, 0x588)) {
        let et = etype(e);
        if et <= 1 || et >= 5 || rb(e, PFLAGS) & 8 != 0 {
            wf(this, 0x588, z1);
        }
    }
    let bit21 = (ru(this, 0x470) >> 21) & 1 != 0;
    let mut take = true;
    if bit21 {
        f.fld(rf(lcp, 8));
        f.fadd_m(cf(0x3F80_0000));
        f.fcomp_m(rf(matrix(this), 0x38));
        if f.jp(5) {
            // not below the matrix z
            take = false;
            if etype(e) == ty::BUILDING {
                f.fld(rf(lcp, 0x14));
                f.fmul_m(rf(this, MOVE_SPEED + 4));
                f.fld(rf(lcp, 0x18));
                f.fmul_m(rf(this, MOVE_SPEED + 8));
                f.faddp(1);
                f.fld(rf(lcp, 0x10));
                f.fmul_m(rf(this, MOVE_SPEED));
                f.faddp(1);
                f.fcomp_m(0.0);
                take = !f.jne(0x41);
            }
        }
    }
    if take && !steep {
        f.fld(rf(this, 0x588));
        f.fcomp_m(kf(0x86C328));
        f.fld(rf(lcp, 8));
        if f.jp(5) {
            f.fadd_m(cf(0x3F80_0000));
            wf(matrix(this), 0x38, f.fstp());
            let v = ru(this, 0x470);
            if v & 0x20_0000 != 0 {
                wu(this, 0x470, v & 0xFFDF_FFFF);
            }
        } else {
            f.fadd_m(top);
            f.fadd_m(cf(0x3F80_0000));
            f.fcomp_m(rf(this, 0x588));
            f.fld(rf(lcp, 8));
            f.fadd_m(cf(0x3F80_0000));
            if !f.jne(0x41) {
                f.fld(rf(this, 0x588));
                *nvec = rv(lcp, 0x10);
                f.fsub_m(top);
                let c = f.fst();
                f.fcomp_st(1);
                if !f.jp(5) {
                    f.fstp_st(0);
                    f.fld(c);
                }
            }
            wf(matrix(this), 0x38, f.fstp());
        }
    } else if bit21 && etype(e) != ty::VEHICLE {
        *nvec = rv(lcp, 0x10);
    }
    wv_bits(this, 0x578, rv_bits(lcp, 0x10));
    let surf = rb(lcp, 0x23);
    wb(this, CONTACT_SURFACE, surf);
    if is_steep_slope(GPtr::from_addr(SURFACE_INFOS), u32::from(surf)) as u8 != 0 {
        wu(this, 0x470, ru(this, 0x470) | 0x20);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PED_PROCESS_ENTITY_COLLISION];
