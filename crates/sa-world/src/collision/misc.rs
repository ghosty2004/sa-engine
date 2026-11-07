//! Small `CCollision` / `CTempColModels` helpers around the camera and the temporary col models.
//! Ported from the 1.0 US bytes; gta-reversed `Collision.cpp`/`TempColModels.cpp` as reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::{GBool, GPtr};
use sa_core::{hook, orig};

sa_core::globals! {
    list GLOBALS;
    /// `CCollision::bCamCollideWithPeds`
    pub static CAM_COLLIDE_WITH_PEDS @ 0x8A5B17: GBool = GBool::TRUE;
    /// `gnTop` (camera collision sector rect, inclusive)
    pub static GN_TOP @ 0x965590: i32 = 0;
    /// `gnRight`
    pub static GN_RIGHT @ 0x965594: i32 = 0;
    /// `gnBottom`
    pub static GN_BOTTOM @ 0x965598: i32 = 0;
    /// `gnLeft`
    pub static GN_LEFT @ 0x96559C: i32 = 0;
    /// `gpMadeInvisibleEntities[10]`
    pub static MADE_INVISIBLE_ENTITIES @ 0x9655A0: [GPtr<u8>; 10] = [GPtr::NULL; 10];
    /// `gNumEntitiesSetInvisible`
    pub static NUM_ENTITIES_SET_INVISIBLE @ 0x9655DC: i32 = 0;
}

orig! {
    /// `CCollision::CheckCameraCollisionPeds(int32 x, int32 y, const CVector& pos, const CVector& dir,
    /// float& t)` (0x415320)
    pub fn check_camera_collision_peds(x: i32, y: i32, pos: GPtr<f32>, dir: GPtr<f32>, t: GPtr<f32>) -> u8
        = cdecl 0x415320 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    pub fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<GPtr<u8>>) = thiscall 0x571A00 => unported;
    /// `CColModel::RemoveCollisionVolumes` (0x40F9E0)
    pub fn col_model_remove_collision_volumes(cm: GPtr<u8>) = thiscall 0x40F9E0 => col_model_rcv_port;
}

pub fn col_model_rcv_port(cm: GPtr<u8>) {
    crate::collision::data::col_model_remove_collision_volumes(cm.cast())
}

/// `CCollision::CheckPeds(const CVector& pos, const CVector& dir, float& t)` (stub 0x4154A0 →
/// 0x1567650, AL): false when `bCamCollideWithPeds` is clear; otherwise `CheckCameraCollisionPeds` for
/// every sector `x` in `gnLeft..=gnRight`, `y` in `gnBottom..=gnTop` (rows outer, bounds re-read
/// once), true when any call returns true.
#[hook(addr = 0x4154A0, conv = cdecl, name = "CCollision::CheckPeds", expect = "E9 AB 21 15 01")]
pub fn check_peds(pos: GPtr<f32>, dir: GPtr<f32>, t: GPtr<f32>) -> u8 {
    if !CAM_COLLIDE_WITH_PEDS.get().get() {
        return 0;
    }
    let (right, left, bottom, top) = (GN_RIGHT.get(), GN_LEFT.get(), GN_BOTTOM.get(), GN_TOP.get());
    let mut found = 0u8;
    let mut y = bottom;
    while y <= top {
        let mut x = left;
        while x <= right {
            if check_camera_collision_peds(x, y, pos, dir, t) != 0 {
                found = 1;
            }
            x += 1;
        }
        y += 1;
    }
    found
}

/// `CCollision::ResetMadeInvisibleObjects` (stub 0x415540 → 0x156D280): every non-null entry of the
/// first `gNumEntitiesSetInvisible` (re-read each step) gets flag 0x80 (+0x1C), its reference cleaned
/// up and the slot cleared; then the count is 0.
#[hook(addr = 0x415540, conv = cdecl, name = "CCollision::ResetMadeInvisibleObjects", expect = "E9 3B 7D 15 01")]
pub fn reset_made_invisible_objects() {
    let arr = MADE_INVISIBLE_ENTITIES.ptr().cast::<GPtr<u8>>();
    let mut i = 0;
    while i < NUM_ENTITIES_SET_INVISIBLE.get() {
        let slot = arr.wrapping_index(i);
        let e = slot.read();
        if !e.is_null() {
            let f = e.byte_add(0x1C).cast::<u32>();
            f.write(f.read() | 0x80);
            let e = slot.read();
            if !e.is_null() {
                clean_up_old_reference(e, slot);
            }
            slot.write(GPtr::NULL);
        }
        i += 1;
    }
    NUM_ENTITIES_SET_INVISIBLE.set(0);
}

/// `CTempColModels::Shutdown` (stub 0x41B360 → 0x156CBC0): `RemoveCollisionVolumes` of the twelve
/// single temp models (0x968A00, 0x968DF0, ... 0x968FD0, as listed) and then of the 20 models of the
/// array at 0x968A30.
#[hook(addr = 0x41B360, conv = cdecl, name = "CTempColModels::Shutdown", expect = "E9 5B 18 15 01")]
pub fn temp_col_models_shutdown() {
    const SINGLE: [usize; 12] = [
        0x968A00, 0x968DF0, 0x968E20, 0x968E50, 0x968E80, 0x968EB0, 0x968EE0, 0x968F10, 0x968F40, 0x968F70,
        0x968FA0, 0x968FD0,
    ];
    for a in SINGLE {
        col_model_remove_collision_volumes(temp_model(a));
    }
    for i in 0..20 {
        col_model_remove_collision_volumes(temp_model(0x968A30 + 0x30 * i));
    }
}

/// A `CTempColModels` static col model (fixed address in phase A; phases B/C need the module's own
/// storage, which comes with `CTempColModels::Initialise`).
fn temp_model(addr: usize) -> GPtr<u8> {
    GPtr::from_addr(addr)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_CHECK_PEDS,
    HOOK_RESET_MADE_INVISIBLE_OBJECTS,
    HOOK_TEMP_COL_MODELS_SHUTDOWN,
];
