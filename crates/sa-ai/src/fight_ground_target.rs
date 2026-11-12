//! WRITER-NATIVE (batch `native-22`): 0x61D6F0, the fight task's "kick the target on the ground" test (unnamed,
//! called by the attack choosers on every punch), and the two `CLocalisation` leaves it and the punch path call
//! (`Blood` 0x56D230, `KickingWhenDown` 0x56D270; 3-byte bodies, registered for phases B/C only). Ported from the
//! 1.0 US bytes; x87 arithmetic follows the original's instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CLocalisation::KickingWhenDown()` (0x56D270)
    fn localisation_kicking_when_down() -> u8 = cdecl 0x56D270 => localisation_kicking_when_down_native;
    /// `CPed::GetBonePosition(RwV3d&, uint32 bone, bool)` (0x5E4280)
    fn ped_get_bone_position(p: GPtr<u8>, out: GPtr<u8>, bone: u32, b: u32) = thiscall 0x5E4280 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn placeable_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60, returns `out`)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `DotProduct(const CVector*, const CVector*)` (0x40FDB0, ST0)
    fn dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
}

/// `CLocalisation::Blood()` (0x56D230): true.
#[sa_core::native(addr = 0x56D230, conv = cdecl, name = "CLocalisation::Blood")]
pub fn localisation_blood_native() -> u8 {
    1
}

/// `CLocalisation::KickingWhenDown()` (0x56D270): true.
#[sa_core::native(addr = 0x56D270, conv = cdecl, name = "CLocalisation::KickingWhenDown")]
pub fn localisation_kicking_when_down_native() -> u8 {
    1
}

/// This module's phase-B registrations.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_LOCALISATION_BLOOD_NATIVE, NATIVE_LOCALISATION_KICKING_WHEN_DOWN_NATIVE];

/// The entity's position (`m_matrix` +0x30, else the placement position +4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

const PI: u32 = 0x4049_0FDB;
const TWO_PI: u32 = 0x40C9_0FDB;
const NEG_PI: u32 = 0xC049_0FDB;
/// 0.2 (0x858CC4), 0.4 (0x858EE8)
const K1: u32 = 0x3E4C_CCCD;
const K2: u32 = 0x3ECC_CCCD;

/// `(r.z dz + r.y dy) + r.x dx` for the matrix row at `row` (dz on the x87 stack, dy / dx as f32).
fn row_dot(m: GPtr<u8>, row: isize, dx: f32, dy: f32, dz: X87) -> X87 {
    dz.fmul_m(rd::<f32>(m, row + 8))
        .fadd(x(dy).fmul_m(rd::<f32>(m, row + 4)))
        .fadd(x(dx).fmul_m(rd::<f32>(m, row)))
}

/// `CTaskSimpleFight` 0x61D6F0 (`bool (CPed* ped)`, `ret 4`): may the ped kick a target lying on the ground?
///
/// False unless `KickingWhenDown()`. Ranges: `R1 = 0.2 + X`, `R2 = X + 0.4` with `X = [0xC177EC] + [0xC1710C +
/// (+0x24 - 4) * 0x88]`. A ped target (+0x18): its head bone (3) must be below the ped's z - 0.2 (alive targets;
/// otherwise only peds flagged +0x46C bit 26, with `R2 - 0.4`), not in a task 0xCF whose anim time is past its
/// limit, and within `|forward . d| < R2`, `|right . d| < R1` (`d = target - ped`, through `GetMatrix`,
/// `VectorSub`, `DotProduct`). Any other target: true only when it is the ped's +0x568. Without a target, only
/// for the player: the scanned peds (intelligence +0x130, 16) whose head is below z - 0.2 (or in state 0x37),
/// inside both ranges of their own matrix, with `|wrap(atan2(-dx, dy) - heading +0x558)| < 1.0472`, choose the
/// smallest angle difference (a dead best is replaced by a live one, health +0x540); its angle goes to +0x55C.
/// Without one: true when +0x568 is an automobile. The original keeps the best ped in the ped argument slot.
#[hook(addr = 0x61D6F0, conv = thiscall, name = "FUN_0061d6f0",
       expect = "83 EC 2C 57 8B F9",
       unverified = "native-22", route = "L2", hazards = "x87-transcendental,ax-al-return,stack-residue",
       recs = "weapon-combat,grove-walk",
       note = "fpatan; the ped argument slot holds the best target in the player loop")]
pub fn fight_ground_target(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if localisation_kicking_when_down() == 0 {
        return 0;
    }
    let xr = x(g::<f32>(0xC1_77EC).read()).fadd_m(
        g::<f32>((0xC1_710Cu32).wrapping_add((i32::from(rd::<i8>(this, 0x24)) - 4).wrapping_mul(0x88) as u32) as usize)
            .read(),
    );
    let r1 = x(f32::from_bits(K1)).fadd(xr).to_f32();
    let mut r2 = xr.fadd_m(f32::from_bits(K2)).to_f32();
    let mut bone = [0f32; 3];
    let bp: GPtr<u8> = GPtr::from_raw(bone.as_mut_ptr()).cast();
    let target = rd::<GPtr<u8>>(this, 0x18);
    if !target.is_null() {
        if rd::<u8>(target, 0x36) & 7 != 3 {
            return u8::from(target == rd::<GPtr<u8>>(ped, 0x568));
        }
        ped_get_bone_position(target, bp, 3, 0);
        if ped_is_alive(rd::<GPtr<u8>>(this, 0x18)) != 0 {
            let low = x(rd::<f32>(pos(ped), 8)).fsub_m(f32::from_bits(K1));
            if low.fcom(x(bone[2])).test_any(0x41) {
                if rd::<u32>(rd::<GPtr<u8>>(this, 0x18), 0x46C) & 0x0400_0000 == 0 {
                    return 0;
                }
                r2 = x(r2).fsub_m(f32::from_bits(K2)).to_f32();
            }
        }
        let t = rd::<GPtr<u8>>(this, 0x18);
        let s = tm_get_simplest_active_task(rd::<GPtr<u8>>(t, 0x47C).byte_add(4));
        if !s.is_null() && task_get_type(s) == 0xCF {
            let a = rd::<GPtr<u8>>(s, 0x14);
            if !a.is_null() {
                let cur = x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10));
                if !cur.fcom(x(rd::<f32>(a, 0x20))).test_any(0x41) {
                    return 0;
                }
            }
        }
        let t = rd::<GPtr<u8>>(this, 0x18);
        let m = placeable_get_matrix(t);
        let (tp, pp) = (pos(t), pos(ped));
        let dx = x(rd::<f32>(tp, 0)).fsub_m(rd::<f32>(pp, 0));
        let dy = x(rd::<f32>(tp, 4)).fsub_m(rd::<f32>(pp, 4));
        let dz = x(rd::<f32>(tp, 8)).fsub_m(rd::<f32>(pp, 8));
        let f = dz
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(dy.fmul_m(rd::<f32>(m, 0x14)))
            .fadd(dx.fmul_m(rd::<f32>(m, 0x10)));
        if f.fabs().fcom(x(r2)).test_parity(0x05) {
            return 0;
        }
        let t = rd::<GPtr<u8>>(this, 0x18);
        let m = placeable_get_matrix(t);
        let mut d = [0f32; 3];
        let out = vector_sub(GPtr::from_raw(d.as_mut_ptr()).cast(), pos(t), pos(ped));
        let r = X87::from_f64_exact(dot_product(out, m));
        return u8::from(!r.fabs().fcom(x(r1)).test_parity(0x05));
    }
    if ped_is_player(ped) == 0 {
        return 0;
    }
    let list = rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x130);
    let mut best: GPtr<u8> = GPtr::NULL;
    let mut best_angle: f32 = 0.0;
    let mut best_diff = x(f32::from_bits(PI)).to_f32();
    for i in 0..16 {
        let e = rd::<GPtr<u8>>(list, 4 * i);
        if e.is_null() {
            continue;
        }
        ped_get_bone_position(e, bp, 3, 0);
        if rd::<u32>(e, 0x530) != 0x37 {
            let low = x(rd::<f32>(pos(ped), 8)).fsub_m(f32::from_bits(K1));
            if low.fcom(x(bone[2])).test_any(0x41) {
                continue;
            }
        }
        let (ep, pp) = (pos(e), pos(ped));
        let dx = x(rd::<f32>(ep, 0)).fsub_m(rd::<f32>(pp, 0)).to_f32();
        let dy = x(rd::<f32>(ep, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32();
        let dz = x(rd::<f32>(ep, 8)).fsub_m(rd::<f32>(pp, 8));
        let m = rd::<GPtr<u8>>(e, 0x14);
        if row_dot(m, 0x10, dx, dy, dz).fabs().fcom(x(r2)).test_parity(0x05) {
            continue;
        }
        if row_dot(m, 0, dx, dy, dz).fabs().fcom(x(r1)).test_parity(0x05) {
            continue;
        }
        let ang = x(dx).fchs().fpatan(x(dy)).to_f32();
        let mut diff = x(ang).fsub_m(rd::<f32>(ped, 0x558));
        if !diff.fcom(x(f32::from_bits(NEG_PI))).test_parity(0x05) {
            diff = diff.fadd_m(f32::from_bits(TWO_PI));
        } else if !diff.fcom(x(f32::from_bits(PI))).test_any(0x41) {
            diff = diff.fsub_m(f32::from_bits(TWO_PI));
        }
        if !diff.fcom(x(0.0)).test_parity(0x05) {
            diff = diff.fmul_m(-1.0);
        }
        if diff.fcom(x(f32::from_bits(0x3F86_0A92))).test_parity(0x05) {
            continue;
        }
        let take = if best.is_null() {
            true
        } else {
            let best_dead = !x(rd::<f32>(best, 0x540)).fcom(x(0.0)).test_parity(0x41);
            if best_dead && !x(rd::<f32>(e, 0x540)).fcom(x(0.0)).test_any(0x41) {
                true
            } else {
                !diff.fcom(x(best_diff)).test_parity(0x05)
            }
        };
        if take {
            best_diff = diff.to_f32();
            best = e;
            best_angle = ang;
        }
    }
    if !best.is_null() {
        wr::<f32>(ped, 0x55C, best_angle);
        return 1;
    }
    let v = rd::<GPtr<u8>>(ped, 0x568);
    u8::from(!v.is_null() && rd::<u8>(v, 0x36) & 7 == 2 && rd::<u32>(v, 0x590) == 0)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIGHT_GROUND_TARGET];
