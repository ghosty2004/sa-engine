//! FRONTIER-2 turn 10, lane B (batch `frontier2-b22`): the monster truck's suspension virtuals,
//! `CMonsterTruck::SetupSuspensionLines` (0x6C7FB0, vtable +0xC0) and `CMonsterTruck::ProcessSuspension`
//! (0x6C83A0, vtable +0x118). Ported from the 1.0 US bytes with docs/re/specs/VEH/CMonsterTruck.md as
//! reading aid (gta-reversed has no body for either).

use sa_core::abi::HookReg;
use sa_core::bindings::{memory_free, memory_malloc};
use sa_core::field::gf;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{gt, lt};
use sa_math::vector::CVector;
use sa_stream::bindings::get_wheel_posn;
use sa_world::world::common::entity_pos;

use crate::common::{f32_of, model_info, rd, wr, x};
use crate::mvpd_v1::{read_vec, vptr};
use crate::mvpd_v2::magnitude;

use sa_world::bindings::apply_force;
use sa_world::bindings::apply_spring_collision_alt;
use sa_world::bindings::apply_spring_dampening;
use sa_world::bindings::physical_get_speed as get_speed;

orig! {
    /// `CColDisk::Set(float radius, const CVector& centre, const CVector& axis, float thickness, uint8
    /// material, uint8 piece, uint8 lighting)` (0x40FD50, `ret 0x1C`)
    fn col_disk_set(d: u32, radius: u32, centre: u32, axis: u32, thickness: u32, material: u32, piece: u32, lighting: u32) -> u32
        = thiscall 0x40FD50 => crate::generated::generated_setters::g_ccol_disk_set_40fd50;
    /// `CVehicle::VehicleDamage(float, uint16 component, CEntity*, CVector*, CVector*, eWeaponType)` (vtable +0xE0)
    fn v_vehicle_damage(v: GPtr<u8>, dmg: u32, comp: u32, e: GPtr<u8>, p: GPtr<u8>, d: GPtr<u8>, weapon: u32) = thiscall vslot 0xE0 => unported;
}

/// 1.0f ([0x858624]).
const ONE: f32 = 1.0;

/// `CMonsterTruck::SetupSuspensionLines()` (0x6C7FB0, thiscall, plain `ret`).
///
/// The suspension radius +0x998 = front wheel size * 0.5. The col data gets four disks (`Malloc(0x90)`,
/// `bUsesDisks` and `m_nNumLines = 4` set first; an existing line array is freed first, an existing disk
/// array kept with its count). Per wheel `i`: `GetWheelPosn(i, pos, false)`, the disk (radius, pos, axis
/// x = -1 for the front wheels / 1 for the rear, thickness radius * 0.6, material 0x3C, piece 0x0D,
/// lighting 0xFF) with the piece then 0x0F / 0x0E / 0x10 for wheels 1 / 2 / 3; +0x878 = pos.z + the
/// upper limit, +0x888 = pos.z + the lower limit. The ride height `(1 - 1 / (force * 4)) * (spring0 -
/// line0) + (radius - spring0)` goes to +0x898 / +0x89C; every wheel gets ratio 1.0 and position `front
/// size * 0.5 - front height`. The box min z takes `front height - radius` when lower; the sphere radius
/// the larger corner magnitude when larger.
#[hook(addr = 0x6C7FB0, conv = thiscall, name = "CMonsterTruck::SetupSuspensionLines", this = "CMonsterTruck",
       expect = "83 EC 28 53 55 56 8B F1", unverified = "frontier2-b22", route = "L2", hazards = "heap-writes",
       note = "allocates the col model's disk array once")]
pub fn monster_truck_setup_suspension_lines(this: GPtr<u8>) {
    let mi = model_info(i32::from(rd::<i16>(this, 0x22)));
    let cm = rd::<GPtr<u8>>(mi, 0x14);
    let cd = rd::<GPtr<u8>>(cm, 0x2C);
    let half = gf(0x85_8B8C);
    wr::<f32>(this, 0x998, f32_of(x(rd::<f32>(mi, 0x40)).fmul_m(half)));
    let disks = rd::<GPtr<u8>>(cd, 0x10);
    let fresh = if disks.is_null() {
        true
    } else if rd::<u8>(cd, 7) & 1 == 0 {
        memory_free(disks);
        true
    } else {
        false
    };
    if fresh {
        wr::<u8>(cd, 7, rd::<u8>(cd, 7) | 1);
        wr::<u8>(cd, 6, 4);
        wr(cd, 0x10, memory_malloc(0x90, 0));
    }
    let mut axis = CVector::default();
    for i in 0..4isize {
        let mut pos = CVector::default();
        get_wheel_posn(mi, i as i32, vptr(&mut pos), 0);
        axis.x = if i < 2 { -1.0 } else { 1.0 };
        let thickness = f32_of(x(rd::<f32>(this, 0x998)).fmul_m(gf(0x85_8CC8)));
        let disk = rd::<GPtr<u8>>(cd, 0x10).byte_add(0x24 * i);
        col_disk_set(
            disk.addr() as u32,
            rd::<u32>(this, 0x998),
            vptr(&mut pos).addr() as u32,
            vptr(&mut axis).addr() as u32,
            thickness.to_bits(),
            0x3C,
            0x0D,
            0xFF,
        );
        let piece = match i {
            1 => Some(0x0F),
            2 => Some(0x0E),
            3 => Some(0x10),
            _ => None,
        };
        if let Some(pc) = piece {
            wr::<u8>(rd::<GPtr<u8>>(cd, 0x10), 0x24 * i + 0x11, pc);
        }
        let h = rd::<GPtr<u8>>(this, 0x384);
        wr::<f32>(this, 0x878 + 4 * i, f32_of(x(pos.z).fadd_m(rd::<f32>(h, 0xB8))));
        let h = rd::<GPtr<u8>>(this, 0x384);
        wr::<f32>(this, 0x888 + 4 * i, f32_of(x(pos.z).fadd_m(rd::<f32>(h, 0xBC))));
    }
    let h = rd::<GPtr<u8>>(this, 0x384);
    let t = x(rd::<f32>(h, 0xAC)).fmul_m(gf(0x85_8B90)).fdivr_m(ONE).fsubr_m(ONE);
    let u = x(rd::<f32>(this, 0x878)).fsub_m(rd::<f32>(this, 0x888));
    let w = x(rd::<f32>(this, 0x998)).fsub_m(rd::<f32>(this, 0x878));
    let height = f32_of(u.fmul(t).fadd(w));
    wr::<f32>(this, 0x898, height);
    wr::<f32>(this, 0x89C, height);
    for i in 0..4isize {
        wr::<u32>(this, 0x7D4 + 4 * i, ONE.to_bits());
        let v = x(rd::<f32>(mi, 0x40)).fmul_m(half).fsub_m(rd::<f32>(this, 0x898));
        wr::<f32>(this, 0x838 + 4 * i, f32_of(v));
    }
    let low = x(rd::<f32>(this, 0x898)).fsub_m(rd::<f32>(this, 0x998));
    if lt(low, x(rd::<f32>(cm, 8))) {
        wr::<f32>(cm, 8, f32_of(low));
    }
    let mut m = magnitude(cm);
    if !gt(m, magnitude(cm.byte_add(0xC))) {
        m = magnitude(cm.byte_add(0xC));
    }
    if gt(m, x(rd::<f32>(cm, 0x24))) {
        wr::<f32>(cm, 0x24, f32_of(m));
    }
}

/// `comp[i] < 1.0f` (`fcomp; test ah, 5; jp` skips on >= and NaN).
fn compressed(this: GPtr<u8>, i: isize) -> bool {
    lt(x(rd::<f32>(this, 0x7D4 + 4 * i)), x(ONE))
}

/// `a - b` per component, each an f32 store.
fn sub(a: GPtr<u8>, b: GPtr<u8>) -> CVector {
    let c = |k: isize| f32_of(x(rd::<f32>(a, 4 * k)).fsub_m(rd::<f32>(b, 4 * k)));
    CVector::new(c(0), c(1), c(2))
}

/// `CMonsterTruck::ProcessSuspension()` (0x6C83A0, thiscall, plain `ret`).
///
/// With the matrix read once: per wheel `dir = up * -1`, and for a compressed wheel (ratio +0x7D4 < 1)
/// the contact point relative to the position, else 0. Each compressed wheel gets
/// `ApplySpringCollisionAlt(force level, dir, point, ratio, bias (1 - bias for wheels 1 / 3), normal,
/// force)`. The contact speed is `GetSpeed(point)` minus the collision entity's `GetSpeed` at +0x8F8;
/// a compressed wheel on a normal with z > 0.35 uses `dir = -normal`; then `ApplySpringDampening`
/// (damping level, force, dir, point, speed). Finally a compressed wheel on a vehicle: below ratio 0.5
/// the vehicle takes `VehicleDamage((1 - ratio) * mass * 0.05, pieceB, this, point, normal, 0x31)`; on
/// a normal with z > 0.5 it is pushed with `ApplyForce(n' * (1 - ratio) * its mass, point - its
/// position, true)` where `n' = (n.x * 0.25 * -0.05, n.y * 0.25 * -0.05, n.z * -0.05)` (no 0.25 on z,
/// as compiled). Every wheel's collision entity +0x8E8 is cleared (no reference clean-up).
#[hook(addr = 0x6C83A0, conv = thiscall, name = "CMonsterTruck::ProcessSuspension", this = "CMonsterTruck",
       expect = "81 EC E8 00 00 00 53 55", unverified = "frontier2-b22", route = "L2", hazards = "none",
       note = "the damage component's upper 16 bits are the point address's, as compiled")]
#[allow(clippy::too_many_lines)]
pub fn monster_truck_process_suspension(this: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(this, 0x14);
    let minus_one = gf(0x85_8C1C);
    let mut dir = [CVector::default(); 4];
    let mut pt = [CVector::default(); 4];
    let mut force = [0f32; 4];
    let mut speed = [CVector::default(); 4];
    for i in 0..4isize {
        let up = |k: isize| f32_of(x(rd::<f32>(m, 0x20 + 4 * k)).fmul_m(minus_one));
        dir[i as usize] = CVector::new(up(0), up(1), up(2));
        pt[i as usize] = if compressed(this, i) {
            sub(this.byte_add(0x724 + 0x2C * i), entity_pos(this))
        } else {
            CVector::default()
        };
    }
    for i in 0..4isize {
        if !compressed(this, i) {
            continue;
        }
        let h = rd::<GPtr<u8>>(this, 0x384);
        let mut bias = rd::<f32>(h, 0xC0);
        if i == 1 || i == 3 {
            bias = f32_of(x(ONE).fsub_m(bias));
        }
        let u = i as usize;
        apply_spring_collision_alt(
            this,
            rd::<f32>(h, 0xAC),
            vptr(&mut dir[u]),
            vptr(&mut pt[u]),
            rd::<f32>(this, 0x7D4 + 4 * i),
            bias,
            this.byte_add(0x734 + 0x2C * i),
            GPtr::from_raw(&mut force[u]),
        );
    }
    for i in 0..4isize {
        let u = i as usize;
        let mut out = CVector::default();
        speed[u] = read_vec(get_speed(this, GPtr::from_raw(&mut out), pt[u]).cast());
        let e = rd::<GPtr<u8>>(this, 0x8E8 + 4 * i);
        if !e.is_null() {
            let mut out2 = CVector::default();
            let r = get_speed(e, GPtr::from_raw(&mut out2), read_vec(this.byte_add(0x8F8 + 12 * i))).cast::<u8>();
            let mut s = speed[u];
            s.x = f32_of(x(s.x).fsub_m(rd::<f32>(r, 0)));
            s.y = f32_of(x(s.y).fsub_m(rd::<f32>(r, 4)));
            s.z = f32_of(x(s.z).fsub_m(rd::<f32>(r, 8)));
            speed[u] = s;
        }
        let n = this.byte_add(0x734 + 0x2C * i);
        if lt(x(rd::<f32>(this, 0x7D4 + 4 * i)), x(gf(0x87_17D4))) && gt(x(rd::<f32>(n, 8)), x(gf(0x85_8F9C))) {
            let c = |k: isize| f32_of(x(rd::<f32>(n, 4 * k)).fchs());
            dir[u] = CVector::new(c(0), c(1), c(2));
        }
    }
    for i in 0..4isize {
        if !compressed(this, i) {
            continue;
        }
        let u = i as usize;
        let damping = rd::<f32>(rd::<GPtr<u8>>(this, 0x384), 0xB0);
        apply_spring_dampening(this, damping, force[u], vptr(&mut dir[u]), vptr(&mut pt[u]), vptr(&mut speed[u]));
    }
    let k5 = gf(0x8D_33B8);
    let km5 = gf(0x8D_33B4);
    let quarter = gf(0x85_8C84);
    for i in 0..4isize {
        let slot = this.byte_add(0x8E8 + 4 * i);
        let e = rd::<GPtr<u8>>(slot, 0);
        let cp = this.byte_add(0x724 + 0x2C * i);
        let normal = cp.byte_add(0x10);
        if compressed(this, i) && !e.is_null() && rd::<u8>(e, 0x36) & 7 == 2 {
            let ratio = rd::<f32>(this, 0x7D4 + 4 * i);
            if lt(x(ratio), x(gf(0x85_8B8C))) {
                let dmg = f32_of(x(ONE).fsub_m(ratio).fmul_m(rd::<f32>(this, 0x8C)).fmul_m(k5));
                // `movzx dx, byte [pieceB]` over EDX = &point: the upper 16 bits stay the address's
                let comp = (cp.addr() as u32 & 0xFFFF_0000) | u32::from(rd::<u8>(cp, 0x24));
                v_vehicle_damage(e, dmg.to_bits(), comp, this, cp, normal, 0x31);
            }
            if gt(x(rd::<f32>(normal, 8)), x(gf(0x85_8B8C))) {
                let ny = f32_of(x(rd::<f32>(normal, 4)).fmul_m(quarter));
                let ax = x(rd::<f32>(normal, 0)).fmul_m(quarter).fmul_m(km5);
                let e = rd::<GPtr<u8>>(slot, 0);
                let point = sub(cp, entity_pos(e));
                let cy = x(km5).fmul_m(ny);
                let dz = f32_of(x(rd::<f32>(normal, 8)).fmul_m(km5));
                let k = f32_of(x(ONE).fsub_m(rd::<f32>(this, 0x7D4 + 4 * i)));
                let fx = f32_of(ax.fmul_m(k));
                let fy = cy.fmul_m(k);
                let fz = x(dz).fmul_m(k);
                let mass = x(rd::<f32>(e, 0x8C));
                let f = CVector::new(f32_of(x(fx).fmul(mass)), f32_of(fy.fmul(mass)), f32_of(fz.fmul(mass)));
                apply_force(e, f, point, 1);
            }
        }
        wr::<u32>(slot, 0, 0);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_MONSTER_TRUCK_SETUP_SUSPENSION_LINES, HOOK_MONSTER_TRUCK_PROCESS_SUSPENSION];
