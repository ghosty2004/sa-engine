//! `CVehicle::SpecialEntityPreCollisionStuff` (batch `mvpk-veh-1`): which vehicle-vs-entity collisions are skipped or
//! softened. Ported from the 1.0 US bytes (gta-reversed `Vehicle.cpp` as a reading aid; the bytes win). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x};

orig! {
    /// `CEntity::DoesNotCollideWithFlyers()` (0x532D40)
    fn entity_does_not_collide_with_flyers(e: GPtr<u8>) -> u8 = thiscall 0x532D40 => unported;
    /// `CEntity::GetIsStatic()` (0x4633E0)
    fn entity_get_is_static(e: GPtr<u8>) -> u8 = thiscall 0x4633E0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CMatrix::operator*(CVector* out, const CMatrix&, const CVector&)` (0x59C890; rotation plus translation)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `Invert(const CMatrix& src, CMatrix& dst)` (0x59B920)
    fn matrix_invert(src: GPtr<u8>, dst: GPtr<u8>) -> GPtr<u8> = cdecl 0x59B920 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, `ret 4`)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}
fn ty(e: GPtr<u8>) -> u8 {
    rd::<u8>(e, 0x36) & 7
}
fn recording(e: GPtr<u8>) -> bool {
    (rd::<u32>(e, 0x1C) >> 4) & 1 != 0
}
fn set(p: GPtr<u8>) {
    p.write(1);
}
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}
fn special_model(m: u16) -> bool {
    matches!(m, 0x1B9 | 0x234 | 0x252)
}

/// `CVehicle::SpecialEntityPreCollisionStuff(CPhysical* other, bool ignoreStuck, bool& disabled, bool& ignored, bool&
/// unableToMove, bool& stuck)` (0x6D6640, thiscall, `ret 0x18`). x87 compares. "flag" below is +0x40 |= 0x1000.
///
/// A ped (+0x470 bit 31) sitting in this vehicle (+0x58C) → `disabled`. Flag +0x43 bit 0 with a status (+0x36 >> 3)
/// other than 0 and 8 and `other.DoesNotCollideWithFlyers()` → `disabled`. Touching (+0x128 either way) → `ignored` and
/// flag. Attached to the other → `ignored`; the other attached to this → `disabled` and flag. Both with +0x40 bit 2 →
/// `disabled`. This using collision recording (+0x1C bit 4) against a vehicle with +0x40 bit 2 without bit 3 → `ignored`
/// and flag. The other's +0x40 & 0xE0 → `ignoreStuck` ? `ignored` : (either recording → `stuck`).
///
/// - An object: flag +0x140 bit 8 with "at" z below 0.66 → `disabled` and its +0x128 = this; models 0x176..0x17D →
///   `disabled`; not type 3 (+0x13C), not +0x140 bit 6 and static → nothing; this model 0x196, 0x1E6 or 0x212 → either
///   recording → `stuck`; damage (+0x144) below 20, not a bike (+0x590 != 9), col box max all below 1.0 → `p = M_obj *
///   (max - min)`; this z above `p.z` → `ignored`, else (this inverse matrix * p).z below 0 → `ignored` (stack
///   `CMatrix`). Then unless `ignored`, `disabled` or `stuck`: the object recording → `unableToMove`.
/// - Others: the other or this (against a vehicle or ped) of models 0x1B9, 0x234, 0x252 → `ignored` and flag; the other
///   being +0x4C4 or +0x4C8 → +0x40 = (its value read before) | 0x1000 and `stuck`; else the other recording →
///   `unableToMove`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x6d6640, conv = thiscall, name = "CVehicle::SpecialEntityPreCollisionStuff", expect = "6A FF 64 A1 00 00 00 00",
       this = "CVehicle", unverified = "mvpk-veh-1", route = "L2", hazards = "none", recs = "driving",
       note = "the CMatrix temporary lives on the Rust stack; the exit chunk 0x4017C2 is a no-CD fragment (SEH restore)")]
pub fn vehicle_special_entity_pre_collision_stuff(
    this: GPtr<u8>,
    other: GPtr<u8>,
    ignore_stuck: u8,
    disabled: GPtr<u8>,
    ignored: GPtr<u8>,
    unable: GPtr<u8>,
    stuck: GPtr<u8>,
) {
    let flag = || wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x1000);
    if ty(other) == 3 && (rd::<u32>(other, 0x470) as i32) < 0 && rd::<GPtr<u8>>(other, 0x58C) == this {
        set(disabled);
        return;
    }
    if rd::<u8>(this, 0x43) & 1 != 0 {
        let st = rd::<u8>(this, 0x36) >> 3;
        if st != 0 && st != 8 && entity_does_not_collide_with_flyers(other) != 0 {
            set(disabled);
            return;
        }
    }
    if rd::<GPtr<u8>>(this, 0x128) == other || rd::<GPtr<u8>>(other, 0x128) == this {
        set(ignored);
        flag();
        return;
    }
    if rd::<GPtr<u8>>(this, 0xFC) == other {
        set(ignored);
        return;
    }
    if rd::<GPtr<u8>>(other, 0xFC) == this {
        set(disabled);
        flag();
        return;
    }
    let fl = rd::<u32>(this, 0x40);
    if fl & 4 != 0 && rd::<u8>(other, 0x40) & 4 != 0 {
        set(disabled);
        return;
    }
    let rec = recording(this);
    if rec && ty(other) == 2 {
        let of = rd::<u32>(other, 0x40);
        if of & 4 != 0 && of & 8 == 0 {
            set(ignored);
            flag();
            return;
        }
    }
    if rd::<u8>(other, 0x40) & 0xE0 != 0 {
        if ignore_stuck != 0 {
            set(ignored);
        } else if rec || recording(other) {
            set(stuck);
        }
        return;
    }
    let t = ty(other);
    if t != 4 {
        if special_model(rd::<u16>(other, 0x22)) || (special_model(rd::<u16>(this, 0x22)) && (t == 2 || t == 3)) {
            set(ignored);
            flag();
            return;
        }
        if other == rd::<GPtr<u8>>(this, 0x4C4) || other == rd::<GPtr<u8>>(this, 0x4C8) {
            wr::<u32>(this, 0x40, fl | 0x1000);
            set(stuck);
            return;
        }
        if recording(other) {
            set(unable);
        }
        return;
    }
    object_case(this, other, rec, disabled, ignored, stuck);
    if ignored.read() != 0 || disabled.read() != 0 || stuck.read() != 0 {
        return;
    }
    if recording(other) {
        set(unable);
    }
}

/// The object part of `SpecialEntityPreCollisionStuff` (up to the common unable-to-move tail).
fn object_case(this: GPtr<u8>, other: GPtr<u8>, rec: bool, disabled: GPtr<u8>, ignored: GPtr<u8>, stuck: GPtr<u8>) {
    let of = rd::<u32>(other, 0x140);
    if of & 0x100 != 0 && !x(rd::<f32>(rd::<GPtr<u8>>(other, 0x14), 0x28)).fcom(x(k(0x85BA50))).test_parity(5) {
        set(disabled);
        wr::<GPtr<u8>>(other, 0x128, this);
        return;
    }
    if (0x176..=0x17D).contains(&rd::<u16>(other, 0x22)) {
        set(disabled);
        return;
    }
    if rd::<u8>(other, 0x13C) != 3 && of & 0x40 == 0 && entity_get_is_static(other) != 0 {
        return;
    }
    if matches!(rd::<u16>(this, 0x22), 0x196 | 0x1E6 | 0x212) {
        if rec || recording(other) {
            set(stuck);
        }
        return;
    }
    if rd::<u8>(other, 0x144) >= 0x14 || rd::<u32>(this, 0x590) == 9 {
        return;
    }
    let col = entity_get_col_model(other);
    for o in [0xCisize, 0x10, 0x14] {
        if x(rd::<f32>(col, o)).fcom(x(k(0x858624))).test_parity(5) {
            return;
        }
    }
    let mut cm = [0u32; 0x12];
    let cm_p: GPtr<u8> = GPtr::from_raw(cm.as_mut_ptr()).cast();
    let mut d = [0u32; 3];
    let d_p: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
    vector_sub(d_p, col.byte_add(0xC), col);
    let mut out = [0u32; 3];
    let r = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()).cast(), rd::<GPtr<u8>>(other, 0x14), d_p);
    let mut p = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
    let p_p: GPtr<u8> = GPtr::from_raw(p.as_mut_ptr()).cast();
    let tm = rd::<GPtr<u8>>(this, 0x14);
    let above = !x(rd::<f32>(entity_pos(this), 8)).fcom(x(f32::from_bits(p[2]))).test_any(0x41);
    let hit = above || {
        let inv = matrix_invert(tm, cm_p);
        matrix_assign(cm_p, inv);
        let mut o2 = [0u32; 3];
        let r2 = matrix_mul_vec(GPtr::from_raw(o2.as_mut_ptr()).cast(), cm_p, p_p);
        !x(rd::<f32>(r2, 8)).fcom(x(k(0x858B50))).test_parity(5)
    };
    if hit {
        set(ignored);
    }
    matrix_dtor(cm_p);
}

pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_SPECIAL_ENTITY_PRE_COLLISION_STUFF];
