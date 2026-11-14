//! `CIdleCam::SetTarget` (PEDHIT: the idle camera starts when the player stands still after running into peds).
//! Ported from the 1.0 US bytes.

use sa_core::mem::GPtr;
use sa_core::{native, orig};

use crate::common::{g, rd, wr, x};

orig! {
    /// `CEntity::CleanUpOldReference(CEntity** ref)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity** ref)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `CIdleCam::SetTarget(CEntity* target)` (0x50A280, thiscall, `ret 4`). The entry jumps into the protection chunk
/// 0x403961, which loads `EAX = [rol([0x156077C], [0x5E4224])]` = `[0xB7CB84]` (`CTimer::m_snTimeInMilliseconds`,
/// evaluated on the image: rol(0xB8400B7C, 12)) for the body's sign test at 0x50A285.
///
/// `now = (float)time` (unsigned: 2^32 added when negative); the last position +4..+0xC becomes the old target's
/// position +0x18..+0x20 (dword copies) when there is a target (+0), else the camera's (+0x98) source + front
/// (`+0x19C + +0x190`, ... as f32); the reference moves to the new target (`CleanUpOldReference` of the old one,
/// `RegisterReference` of the new one; none: the field cleared); +0x10 and +0x34 = now, +0x54 = 0, byte +0x79 = 0.
/// Registered for phase B only (`#[native]`): the entry is a jump into protection code.
#[native(addr = 0x50A280, conv = thiscall, name = "CIdleCam::SetTarget")]
pub fn idle_cam_set_target(this: GPtr<u8>, target: GPtr<u8>) {
    let t = g::<u32>(0xB7_CB84).read();
    let mut now = sa_math::x87::X87::from_i32(t as i32);
    if (t as i32) < 0 {
        now = now.fadd_m(4_294_967_296.0);
    }
    let now = now.to_f32();
    let old = rd::<GPtr<u8>>(this, 0);
    if !old.is_null() {
        for o in [0, 4, 8] {
            wr::<u32>(this, 4 + o, rd::<u32>(this, 0x18 + o));
        }
    } else {
        let cam = rd::<GPtr<u8>>(this, 0x98);
        for o in [0, 4, 8] {
            wr::<f32>(this, 4 + o, x(rd::<f32>(cam, 0x19C + o)).fadd_m(rd::<f32>(cam, 0x190 + o)).to_f32());
        }
    }
    if !target.is_null() {
        if !old.is_null() {
            entity_clean_up_old_reference(old, this);
        }
        wr::<GPtr<u8>>(this, 0, target);
        entity_register_reference(target, this);
    } else if !old.is_null() {
        entity_clean_up_old_reference(old, this);
        wr::<u32>(this, 0, 0);
    }
    wr::<f32>(this, 0x10, now);
    wr::<f32>(this, 0x34, now);
    wr::<u32>(this, 0x54, 0);
    wr::<u8>(this, 0x79, 0);
}

/// This module's `#[native]` registrations.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_IDLE_CAM_SET_TARGET];
