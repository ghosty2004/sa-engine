//! Two unnamed helpers of the melee and hassle tasks that SA:MP executes (UNK-game rows of
//! `docs/samp/native-unported.csv`, batch `samp-unk-1`): the static one-sphere collision model used for melee hit
//! tests and the melee animation's finish callback. Names proposed in `docs/samp/native-unported-names.csv`. Ported
//! from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CColSphere::Set(float radius, const CVector& centre, uint8 material, uint8 pieceType, uint8 lighting)`
    /// (0x40FD10, thiscall, `ret 0x14`)
    fn col_sphere_set(s: GPtr<u8>, radius: u32, centre: GPtr<u32>, material: u32, piece: u32, light: u32) = thiscall 0x40FD10 => unported;
}

/// The static `CColModel` (0xC17824): bounding box min (+0) and max (+0xC), bounding sphere centre (+0x18) and
/// radius (+0x24), collision data pointer (+0x2C, 0xC17850); its `CCollisionData` at 0xC17854 (sphere count word
/// +0, spheres pointer +8 = 0xC1785C) and the one `CColSphere` at 0xC17884.
const COL_MODEL: usize = 0xC1_7824;
const COL_DATA_PTR: usize = 0xC1_7850;
const COL_DATA: usize = 0xC1_7854;
const COL_SPHERES_PTR: usize = 0xC1_785C;
const COL_SPHERE: usize = 0xC1_7884;

/// `FUN_0061d5f0` (0x61D5F0), proposed `SetUpSphereColModel(float radius)` (thiscall with the caller's task in
/// ECX, which it ignores; `ret 4`; callers 0x6240B0 (melee) and 0x6637C0 (`CTaskGangHassleVehicle`)): the first
/// time (data pointer null) the static model's data pointer, sphere count 1 and spheres pointer are set up;
/// then `CColSphere::Set(radius, (0,0,0), 0, 0, 0xFF)` on its sphere, the bounding sphere centre 0 and radius
/// `radius`, the box from `-radius` (`fchs`, stored by `fst`) to `radius` on all three axes. The stores follow the
/// original's order. EAX ends as the radius bits; no caller reads it.
#[hook(addr = 0x61D5F0, conv = thiscall, name = "FUN_0061d5f0", expect = "A1 50 78 C1 00",
       unverified = "samp-unk-1", route = "L2", hazards = "none", recs = "weapon-combat")]
pub fn set_up_sphere_col_model(_this: GPtr<u8>, radius: u32) {
    if g::<u32>(COL_DATA_PTR).read() == 0 {
        g::<u32>(COL_DATA_PTR).write(COL_DATA as u32);
        g::<u32>(COL_SPHERES_PTR).write(COL_SPHERE as u32);
        g::<u16>(COL_DATA).write(1);
    }
    let mut centre = [0u32; 3];
    col_sphere_set(
        GPtr::from_addr(g::<u32>(COL_SPHERES_PTR).read() as usize),
        radius,
        GPtr::from_raw(centre.as_mut_ptr()),
        0,
        0,
        0xFF,
    );
    let neg = X87::from_f32(f32::from_bits(radius)).fchs().to_f32().to_bits();
    let m = GPtr::<u32>::from_addr(COL_MODEL);
    // 0xC17848, 0xC1783C..44 (centre 0), 0xC17824..2C (min), 0xC17830..38 (max), in the original's order
    m.wrapping_index(9).write(radius);
    m.wrapping_index(6).write(0);
    m.wrapping_index(7).write(0);
    m.wrapping_index(8).write(0);
    for i in 0..3 {
        m.wrapping_index(i).write(neg);
    }
    for i in 3..6 {
        m.wrapping_index(i).write(radius);
    }
}

/// `CTaskSimpleFight`'s melee animation callback 0x61DAE0 (catalogue: missed by Ghidra, data-fnptr-table),
/// proposed `CTaskSimpleFight::FinishMeleeAnimCB(CAnimBlendAssociation*, void* task)` (cdecl): the finished
/// association is forgotten (task +0x1C when it is that one, else task +0x20 when it is that one); an animation id
/// (+0x2C word) of 0x2F sets the finished flag (task +8); then, when task +0x20 is null and the combo state byte
/// +0x27 is 0x10, 0x11, 0x0F or 1, the finished flag is set too.
#[hook(addr = 0x61DAE0, conv = cdecl, name = "(missed by ghidra: data-fnptr-table-unreferenced)",
       expect = "8B 44 24 08 8B 4C 24 04", unverified = "samp-unk-1", route = "L2", hazards = "none",
       recs = "weapon-combat")]
pub fn fight_finish_melee_anim_cb(anim: GPtr<u8>, task: GPtr<u8>) {
    let a = anim.addr() as u32;
    if rd::<u32>(task, 0x1C) == a {
        wr::<u32>(task, 0x1C, 0);
    } else if rd::<u32>(task, 0x20) == a {
        wr::<u32>(task, 0x20, 0);
    }
    if rd::<u16>(anim, 0x2C) == 0x2F {
        wr::<u8>(task, 8, 1);
    }
    if rd::<u32>(task, 0x20) == 0 {
        let s = rd::<u8>(task, 0x27);
        if matches!(s, 0x10 | 0x11 | 0x0F | 1) {
            wr::<u8>(task, 8, 1);
        }
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SET_UP_SPHERE_COL_MODEL, HOOK_FIGHT_FINISH_MELEE_ANIM_CB];
