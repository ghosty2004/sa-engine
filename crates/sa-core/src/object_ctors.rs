//! STD-5: the element / object constructors the CRT's C++ static initialisers (`_cinit`, the
//! `__xc_a..__xc_z` table) call that had no port, so phase B's static initialisation leaves the global
//! objects with their vtables and default fields (model-info stores, collision models, pads, the
//! interior / furniture / procedural-object managers, ...). Ported from the 1.0 US bytes; the
//! managers' SEH frames are not reproduced (no constructor throws). Constructors of one instruction
//! (`mov eax, ecx; ret`) and the protected `ListItem_c` constructor cannot be hooked: `#[native]`.

use sa_math::x87::X87;

use crate::abi::{HookReg, NativeReg};
use crate::mem::GPtr;
use crate::{hook, orig};

fn wr<T: crate::mem::GameData>(p: GPtr<u8>, off: isize, v: T) {
    p.byte_add(off).cast::<T>().write(v)
}
fn rd<T: crate::mem::GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}

orig! {
    /// `CBaseModelInfo::CBaseModelInfo()` (0x4C4A60)
    fn base_model_info_ctor_call(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C4A60 => base_model_info_ctor;
    /// `ListItem_c::ListItem_c()` (0x4A8DB0)
    fn list_item_ctor_call(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8DB0 => list_item_ctor;
    /// `List_c::List_c()` (0x4A8DD0)
    fn list_ctor_call(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8DD0 => list_ctor;
    /// `CBoundingBox::CBoundingBox()` (0x40FAF0)
    fn bounding_box_ctor_call(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FAF0 => bounding_box_ctor;
    /// `CCam::Init()` (0x50E180)
    fn cam_init(this: GPtr<u8>) = thiscall 0x50E180 => unported;
    /// `CPad::Clear(bool, bool)` (0x541A70)
    fn pad_clear(this: GPtr<u8>, enable: u32, phase: u32) = thiscall 0x541A70 => unported;
    /// `eh vector constructor iterator(ptr, size, count, ctor, dtor)` (0x821D30)
    fn eh_vector_ctor(p: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
}

/// `CBaseModelInfo::CBaseModelInfo()` (0x4C4A60): vtable 0x85BB9C, u16 +8 = 0, u16 +0xA = 0xFFFF.
#[hook(addr = 0x4C4A60, conv = thiscall, name = "CBaseModelInfo", expect = "8B C1 C7 00 9C BB 85 00",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn base_model_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x85BB9C);
    wr::<u16>(this, 8, 0);
    wr::<u16>(this, 0xA, 0xFFFF);
    this
}

/// A model-info constructor: the base constructor, then the class's vtable.
fn model_info(this: GPtr<u8>, vtable: u32) -> GPtr<u8> {
    base_model_info_ctor_call(this);
    wr::<u32>(this, 0, vtable);
    this
}

/// `CAtomicModelInfo::CAtomicModelInfo()` (0x4C5540): vtable 0x85BBF0.
#[hook(addr = 0x4C5540, conv = thiscall, name = "FUN_004c5540", expect = "56 8B F1 E8 18 F5 FF FF",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn atomic_model_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    model_info(this, 0x85BBF0)
}

/// `CDamageAtomicModelInfo::CDamageAtomicModelInfo()` (0x4C55A0): vtable 0x85BC30.
#[hook(addr = 0x4C55A0, conv = thiscall, name = "FUN_004c55a0", expect = "56 8B F1 E8 B8 F4 FF FF",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn damage_atomic_model_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    model_info(this, 0x85BC30)
}

/// `CLodAtomicModelInfo::CLodAtomicModelInfo()` (0x4C55E0): u16 +0x20 = u16 +0x22 = 0, vtable 0x85BC70.
#[hook(addr = 0x4C55E0, conv = thiscall, name = "FUN_004c55e0", expect = "56 8B F1 E8 78 F4 FF FF",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn lod_atomic_model_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    base_model_info_ctor_call(this);
    wr::<u16>(this, 0x20, 0);
    wr::<u16>(this, 0x22, 0);
    wr::<u32>(this, 0, 0x85BC70);
    this
}

/// `CTimeModelInfo::CTimeModelInfo()` (0x4C5640): vtable 0x85BCB0, u16 +0x22 = 0xFFFF.
#[hook(addr = 0x4C5640, conv = thiscall, name = "FUN_004c5640", expect = "56 8B F1 E8 18 F4 FF FF",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn time_model_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    model_info(this, 0x85BCB0);
    wr::<u16>(this, 0x22, 0xFFFF);
    this
}

/// `CLodTimeModelInfo::CLodTimeModelInfo()` (0x4C5690): u16 +0x20 = u16 +0x22 = 0, vtable 0x85BCF0,
/// u16 +0x26 = 0xFFFF.
#[hook(addr = 0x4C5690, conv = thiscall, name = "FUN_004c5690", expect = "56 8B F1 E8 C8 F3 FF FF",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn lod_time_model_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    base_model_info_ctor_call(this);
    wr::<u16>(this, 0x20, 0);
    wr::<u16>(this, 0x22, 0);
    wr::<u32>(this, 0, 0x85BCF0);
    wr::<u16>(this, 0x26, 0xFFFF);
    this
}

/// `CClumpModelInfo::CClumpModelInfo()` (0x4C56F0): vtable 0x85BD30.
#[hook(addr = 0x4C56F0, conv = thiscall, name = "FUN_004c56f0", expect = "56 8B F1 E8 68 F3 FF FF",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn clump_model_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    model_info(this, 0x85BD30)
}

/// `CWeaponModelInfo::CWeaponModelInfo()` (0x4C5760): vtable 0x85BD78.
#[hook(addr = 0x4C5760, conv = thiscall, name = "FUN_004c5760", expect = "56 8B F1 E8 F8 F2 FF FF",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn weapon_model_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    model_info(this, 0x85BD78)
}

/// The saved-weapon element constructor of `CGameLogic::SavedWeapons` (0x441E00): bytes +0x14 and
/// +0x15 = 0, +0x18 = 0.
#[hook(addr = 0x441E00, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "8B C1 33 C9 88 48 14 88",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn saved_weapon_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(this, 0x14, 0);
    wr::<u8>(this, 0x15, 0);
    wr::<u32>(this, 0x18, 0);
    this
}

/// `CCam::CCam()` (0x5173F0): `CCam::Init()` (0x50E180, which keeps ECX), returns `this`.
#[crate::native(addr = 0x5173F0, conv = thiscall, name = "CCam::CCam")]
pub fn cam_ctor(this: GPtr<u8>) -> GPtr<u8> {
    cam_init(this);
    this
}

/// `CPad::CPad()` (0x541D80): `Clear(true, true)`.
#[hook(addr = 0x541D80, conv = thiscall, name = "CPad", expect = "56 6A 01 6A 01 8B F1 E8",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn pad_ctor(this: GPtr<u8>) -> GPtr<u8> {
    pad_clear(this, 1, 1);
    this
}

/// `CPtrListSingleLink::CPtrListSingleLink()` (0x43E3F0, the LOD lists' element): head = NULL.
#[hook(addr = 0x43E3F0, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "8B C1 C7 00 00 00 00 00",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn ptr_list_single_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    this
}

/// `IKChain_c::IKChain_c()` (0x617850): its `ListItem_c` base.
#[hook(addr = 0x617850, conv = thiscall, name = "FUN_00617850", expect = "56 8B F1 E8 58 15 E9 FF",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn ik_chain_ctor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor_call(this);
    this
}

/// `CColModel::CColModel()` (stub 0x40FB60 → .HOODLUM 0x156C690): the bounding box, byte +0x28 = 0,
/// +0x2C = 0, then byte +0x29 = (+0x29 & 0xFC) | 4.
#[hook(addr = 0x40FB60, conv = thiscall, name = "CColModel", expect = "E9 2B CB 15 01",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn col_model_ctor(this: GPtr<u8>) -> GPtr<u8> {
    bounding_box_ctor_call(this);
    wr::<u8>(this, 0x28, 0);
    wr::<u32>(this, 0x2C, 0);
    wr::<u8>(this, 0x29, (rd::<u8>(this, 0x29) & 0xFC) | 4);
    this
}

/// `CBoundingBox::CBoundingBox()` (stub 0x40FAF0 → .HOODLUM 0x1566620): min = (1, 1, 1), max =
/// (-1, -1, -1) (each through a stack temporary, not reproduced).
#[hook(addr = 0x40FAF0, conv = thiscall, name = "CBoundingBox", expect = "E9 2B 6B 15 01",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn bounding_box_ctor(this: GPtr<u8>) -> GPtr<u8> {
    for i in 0..3 {
        wr::<u32>(this, 4 * i, 0x3F80_0000);
    }
    for i in 0..3 {
        wr::<u32>(this, 0xC + 4 * i, 0xBF80_0000);
    }
    this
}

/// The constructor of the 0x977BE8 global (stub 0x4541E0 → .HOODLUM 0x1568BF0): the eh vector
/// constructor over 0x32 elements of 0x34 bytes at +8 (element constructor 0x453FA0, destructor
/// 0x453FB0).
#[hook(addr = 0x4541E0, conv = thiscall, name = "thunk_FUN_01568bf0", expect = "E9 0B 4A 11 01",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn path_node_array_owner_ctor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this.byte_add(8), 0x34, 0x32, 0x453FA0, 0x453FB0);
    this
}

/// `CTaskComplexFollowLeaderInFormation` offsets constructor (0x694850): the formation's 16 (x, y)
/// offsets from the distance +0x80 (= 1.0) and the constants 0x858C84 (added) and 0x858B18 (factor),
/// in the original's x87 order (one value goes through an f32 stack temporary).
#[hook(addr = 0x694850, conv = thiscall, name = "FUN_00694850", expect = "83 EC 08 8B C1",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn formation_offsets_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let st = |o: isize, v: X87| wr::<f32>(this, o, v.to_f32());
    wr::<u32>(this, 0, 0xBF80_0000);
    let one = 0x3F80_0000u32;
    wr::<u32>(this, 0x80, one);
    wr::<u32>(this, 4, one);
    wr::<u32>(this, 8, one);
    wr::<u32>(this, 0xC, one);
    let a = || X87::from_f32(rd::<f32>(this, 0x80));
    let c1 = GPtr::<f32>::from_addr(0x858C84).read();
    let c2 = GPtr::<f32>::from_addr(0x858B18).read();
    st(0x14, a());
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x1C, 0);
    st(0x18, a().fchs());
    wr::<u32>(this, 0x20, rd::<u32>(this, 0x80));
    wr::<u32>(this, 0x24, 0);
    let v = a().fchs();
    st(0x2C, v);
    st(0x28, v);
    wr::<u32>(this, 0x30, 0);
    st(0x34, a().fchs());
    let (p, n) = (a(), a().fchs());
    st(0x38, p);
    st(0x3C, n);
    wr::<u32>(this, 0x44, 0);
    st(0x40, a().fchs());
    wr::<u32>(this, 0x48, rd::<u32>(this, 0x80));
    wr::<u32>(this, 0x4C, 0);
    let s = a().fadd_m(c1);
    wr::<u32>(this, 0x50, 0);
    st(0x54, s.fchs());
    let v = a().fadd_m(c1).fchs();
    st(0x5C, v);
    st(0x58, v);
    let s = a().fadd_m(c1);
    let tmp = s.to_f32();
    let neg = X87::from_f32(tmp).fchs();
    st(0x60, s);
    st(0x64, neg);
    wr::<u32>(this, 0x68, 0);
    st(0x6C, a().fadd_m(c1).fmul_m(c2));
    let n = a().fchs();
    let m = a().fadd_m(c1).fmul_m(c2);
    st(0x70, n);
    st(0x74, m);
    let p = a();
    let m = a().fadd_m(c1).fmul_m(c2);
    st(0x78, p);
    st(0x7C, m);
    this
}

/// `CGlass` pane constructor (0x71A8B0): +0x40 = +0x44 = 0.
#[hook(addr = 0x71A8B0, conv = thiscall, name = "CFallingGlassPane::CFallingGlassPane", expect = "8B C1 33 C9 89 48 40",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn glass_pane_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x40, 0);
    wr::<u32>(this, 0x44, 0);
    this
}

/// `List_c::List_c()` (0x4A8DD0): head, tail, count = 0.
#[hook(addr = 0x4A8DD0, conv = thiscall, name = "FUN_004a8dd0", expect = "8B C1 33 C9 89 08",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn list_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    this
}

/// `ListItem_c::ListItem_c()` (0x4A8DB0, protected: `mov eax, ecx; nop; jmp 0x401C62`, whose chunk
/// writes +0 and jumps back): +0 = +4 = 0.
#[crate::native(addr = 0x4A8DB0, conv = thiscall, name = "ListItem_c::ListItem_c")]
pub fn list_item_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    this
}

/// The trivial constructors (`mov eax, ecx; ret`): `CRGBA::CRGBA()` 0x463360, `cTransmission`
/// 0x6D0450, `CStencilShadowObject` 0x70F8D0, the cross-hair element 0x742A90, `CWeaponInfo`
/// 0x743C30, the path element 0x453FA0.
#[crate::native(addr = 0x463360, conv = thiscall, name = "CRGBA::CRGBA")]
pub fn rgba_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this
}

/// See [`rgba_ctor`].
#[crate::native(addr = 0x6D0450, conv = thiscall, name = "cTransmission::cTransmission")]
pub fn transmission_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this
}

/// See [`rgba_ctor`].
#[crate::native(addr = 0x70F8D0, conv = thiscall, name = "CStencilShadowObject::CStencilShadowObject")]
pub fn stencil_shadow_object_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this
}

/// See [`rgba_ctor`].
#[crate::native(addr = 0x742A90, conv = thiscall, name = "FUN_00742a90")]
pub fn cross_hair_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this
}

/// See [`rgba_ctor`].
#[crate::native(addr = 0x743C30, conv = thiscall, name = "CWeaponInfo::CWeaponInfo")]
pub fn weapon_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this
}

/// See [`rgba_ctor`].
#[crate::native(addr = 0x453FA0, conv = thiscall, name = "FUN_00453fa0")]
pub fn path_element_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this
}




/// The element constructor of a `ListItem_c` base.
#[hook(addr = 0x5A3070, conv = thiscall, name = "FUN_005a3070", expect = "56 8B F1 E8 38 5D F0 FF",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn proc_object_ctor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor_call(this);
    this
}

/// The element constructor of a `ListItem_c` base (0x4A94E0: the FX system's 1000 particle slots of 0x3C).
#[hook(addr = 0x4A94E0, conv = thiscall, name = "FxEmitterPrt_c::FxEmitterPrt_c", expect = "56 8B F1 E8 C8 F8 FF FF",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn fx_particle_slot_ctor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor_call(this);
    this
}


/// The script element constructor 0x46AE30: bytes +0 and +0x18 = 0.
#[hook(addr = 0x46AE30, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "8B C1 C6 00 00 C6 40 18",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn script_element_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 0x18, 0);
    this
}

/// The 2D-effect element constructor 0x6FA770: +0, +4, +0xC, +0x10, +8, +0x14, +0x1C = -1 (in that
/// order), byte +0x20 = 0.
#[hook(addr = 0x6FA770, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "8B C1 83 C9 FF 89 08 89",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn effect_slot_ctor(this: GPtr<u8>) -> GPtr<u8> {
    for o in [0, 4, 0xC, 0x10, 8, 0x14, 0x1C] {
        wr::<u32>(this, o, u32::MAX);
    }
    wr::<u8>(this, 0x20, 0);
    this
}

/// `ProcSurfaceInfo_c` constructor (0x5A2E90): the `List_c` at +0x3C.
#[hook(addr = 0x5A2E90, conv = thiscall, name = "ProcSurfaceInfo_c::ProcSurfaceInfo_c", expect = "56 8B F1 8D 4E 3C",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn proc_surface_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    list_ctor_call(this.byte_add(0x3C));
    this
}


/// `FurnitureManager_c::FurnitureManager_c()` (0x591260): 9 groups of 0xC (0x590DE0 / 0x590DF0), 0x200
/// items of 0x10 at +0x6C (0x590E00 / 0x590E10), the `List_c` at +0x206C.
#[hook(addr = 0x591260, conv = thiscall, name = "FurnitureManager_c::FurnitureManager_c", expect = "6A FF 68 EC CE 83 00",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn furniture_manager_ctor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this, 0xC, 9, 0x590DE0, 0x590DF0);
    eh_vector_ctor(this.byte_add(0x6C), 0x10, 0x200, 0x590E00, 0x590E10);
    list_ctor_call(this.byte_add(0x206C));
    this
}

/// `InteriorManager_c::InteriorManager_c()` (0x5984C0): 8 interiors of 0x794 (0x5921D0 / 0x591360),
/// the `List_c` at +0x3CA0, 8 groups of 0xBC at +0x3CAC (0x597FE0 / 0x597FF0), the `List_c`s at
/// +0x428C and +0x4298.
#[hook(addr = 0x5984C0, conv = thiscall, name = "InteriorManager_c::InteriorManager_c", expect = "6A FF 68 2D D1 83 00",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn interior_manager_ctor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this, 0x794, 8, 0x5921D0, 0x591360);
    list_ctor_call(this.byte_add(0x3CA0));
    eh_vector_ctor(this.byte_add(0x3CAC), 0xBC, 8, 0x597FE0, 0x597FF0);
    list_ctor_call(this.byte_add(0x428C));
    list_ctor_call(this.byte_add(0x4298));
    this
}

/// `ProcObjectMan_c::ProcObjectMan_c()` (0x5A3E20): 0x80 surface infos of 0x48 at +8 (0x5A2E90 /
/// 0x5A2EA0), 0x200 objects of 0x14 at +0x2408 (0x5A3070 / 0x5A3080), the `List_c` at +0x4C08.
#[hook(addr = 0x5A3E20, conv = thiscall, name = "ProcObjectMan_c::ProcObjectMan_c", expect = "6A FF 68 04 D4 83 00",
       unverified = "std5-ctor-1", route = "L2", hazards = "none", recs = "boot")]
pub fn proc_object_man_ctor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this.byte_add(8), 0x48, 0x80, 0x5A2E90, 0x5A2EA0);
    eh_vector_ctor(this.byte_add(0x2408), 0x14, 0x200, 0x5A3070, 0x5A3080);
    list_ctor_call(this.byte_add(0x4C08));
    this
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_BASE_MODEL_INFO_CTOR,
    HOOK_ATOMIC_MODEL_INFO_CTOR,
    HOOK_DAMAGE_ATOMIC_MODEL_INFO_CTOR,
    HOOK_LOD_ATOMIC_MODEL_INFO_CTOR,
    HOOK_TIME_MODEL_INFO_CTOR,
    HOOK_LOD_TIME_MODEL_INFO_CTOR,
    HOOK_CLUMP_MODEL_INFO_CTOR,
    HOOK_WEAPON_MODEL_INFO_CTOR,
    HOOK_SAVED_WEAPON_CTOR,
    HOOK_PAD_CTOR,
    HOOK_PTR_LIST_SINGLE_CTOR,
    HOOK_IK_CHAIN_CTOR,
    HOOK_COL_MODEL_CTOR,
    HOOK_BOUNDING_BOX_CTOR,
    HOOK_PATH_NODE_ARRAY_OWNER_CTOR,
    HOOK_FORMATION_OFFSETS_CTOR,
    HOOK_GLASS_PANE_CTOR,
    HOOK_LIST_CTOR,
    HOOK_PROC_OBJECT_CTOR,
    HOOK_PROC_SURFACE_INFO_CTOR,
    HOOK_FURNITURE_MANAGER_CTOR,
    HOOK_INTERIOR_MANAGER_CTOR,
    HOOK_PROC_OBJECT_MAN_CTOR,
    HOOK_FX_PARTICLE_SLOT_CTOR,
    HOOK_SCRIPT_ELEMENT_CTOR,
    HOOK_EFFECT_SLOT_CTOR,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[
    NATIVE_CAM_CTOR,
    NATIVE_LIST_ITEM_CTOR,
    NATIVE_RGBA_CTOR,
    NATIVE_TRANSMISSION_CTOR,
    NATIVE_STENCIL_SHADOW_OBJECT_CTOR,
    NATIVE_CROSS_HAIR_CTOR,
    NATIVE_WEAPON_INFO_CTOR,
    NATIVE_PATH_ELEMENT_CTOR,
];
