//! `CAtomicModelInfo` (vtable 0x85BBF0), `CDamageAtomicModelInfo` (0x85BC30) and
//! `CLodAtomicModelInfo` (0x85BC70).
//!
//! Ported from the 1.0 US bytes; gta-reversed `Models/AtomicModelInfo.cpp`,
//! `DamageAtomicModelInfo.cpp`, `LodAtomicModelInfo.cpp` are reading aids. Kept from the original:
//! - `GetAtomicFromDistance` compares `TheCamera.m_fLODDistMultiplier * m_fDrawDistance` (x87,
//!   PC=24, product kept in ST0) with the distance: the atomic is returned only when the product is
//!   strictly greater (NaN → null);
//! - `SetupVehicleUpgradeFlags` first truncates the flags word to its low byte, then matches the name
//!   prefix with `strncmp(prefix, name, strlen(prefix))` against two tables; the first table sets
//!   `0x8000 | id << 10`, the second `0x8100 | id << 10` (16-bit shift);
//! - the destructors of these classes are reduced to the base vptr store (`docs/re/specs/STREAM`).

use sa_core::abi::HookReg;
use sa_core::light::RwMatrix;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use super::base::{
    add_2dfx_count, atomic_2dfx_count, f, operator_delete, vcall_get_anim_file_index, MiPtr,
    VTBL_BASE_MODEL_INFO,
};
use crate::shared::mi;

sa_core::globals! {
    list GLOBALS;
    /// `CDamageAtomicModelInfo::ms_bCreateDamagedVersion`
    pub static CREATE_DAMAGED_VERSION @ 0xA9B0B0: u8 = 0;
    // foreign (owner: CCamera, `TheCamera.m_fLODDistMultiplier`)
    /// `TheCamera.m_fLODDistMultiplier`
    pub static LOD_DIST_MULTIPLIER @ 0xB6F118: f32 = 0.0;
}

orig! {
    /// `CBaseModelInfo::Init` (0x4C4B10)
    pub fn base_init(this: MiPtr) = thiscall 0x4C4B10 => super::base::init;
    /// `CBaseModelInfo::AddTexDictionaryRef` (0x4C4B80)
    pub fn add_tex_dictionary_ref(this: MiPtr) = thiscall 0x4C4B80 => super::base::add_tex_dictionary_ref;
    /// `CBaseModelInfo::RemoveTexDictionaryRef` (0x4C4B90)
    pub fn remove_tex_dictionary_ref(this: MiPtr) = thiscall 0x4C4B90 => super::base::remove_tex_dictionary_ref;
    /// `CBaseModelInfo::AddRef` (0x4C4BA0)
    pub fn add_ref(this: MiPtr) = thiscall 0x4C4BA0 => super::base::add_ref;
    /// `CBaseModelInfo::RemoveRef` (0x4C4BB0)
    pub fn remove_ref(this: MiPtr) = thiscall 0x4C4BB0 => super::base::remove_ref;
    /// `CBaseModelInfo::SetBaseModelInfoFlags` (0x5B3AD0)
    pub fn set_base_model_info_flags(m: MiPtr, flags: u32) = cdecl 0x5B3AD0 => super::base::set_base_model_info_flags;
    /// `CAtomicModelInfo::Init` (0x4C4430)
    pub fn atomic_init(this: MiPtr) = thiscall 0x4C4430 => init;
    /// `CAtomicModelInfo::DeleteRwObject` (0x4C4440)
    pub fn atomic_delete_rw_object(this: MiPtr) = thiscall 0x4C4440 => delete_rw_object;
    /// `CAtomicModelInfo::CreateInstance(RwMatrix*)` (0x4C44D0)
    pub fn atomic_create_instance_rwmat(this: MiPtr, m: GPtr<RwMatrix>) -> GPtr<u8> = thiscall 0x4C44D0 => create_instance_rwmat;
    /// `CAtomicModelInfo::~CAtomicModelInfo` (0x4C5590)
    pub fn atomic_destructor(this: MiPtr) = thiscall 0x4C5590 => destructor;
    /// `CDamageAtomicModelInfo::~CDamageAtomicModelInfo` (0x4C55D0)
    pub fn damage_atomic_destructor(this: MiPtr) = thiscall 0x4C55D0 => damage_destructor;
    /// `CLodAtomicModelInfo::~CLodAtomicModelInfo` (0x4C5620)
    pub fn lod_atomic_destructor(this: MiPtr) = thiscall 0x4C5620 => lod_destructor;
    /// `CAnimManager::AddAnimBlockRef` (0x4D3FB0)
    pub fn add_anim_block_ref(index: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::RemoveAnimBlockRef` (0x4D3FD0)
    pub fn remove_anim_block_ref(index: i32) = cdecl 0x4D3FD0 => unported;
    /// `CCustomBuildingRenderer::IsCBPCPipelineAttached` (0x5D7F40)
    pub fn is_cbpc_pipeline_attached(atomic: GPtr<u8>) -> i32 = cdecl 0x5D7F40 => unported;
    /// `CCustomBuildingRenderer::AtomicSetup` (0x5D7F00)
    pub fn custom_building_atomic_setup(atomic: GPtr<u8>) -> i32 = cdecl 0x5D7F00 => unported;
    /// `CCarFXRenderer::IsCCPCPipelineAttached` (0x5D5B80)
    pub fn is_ccpc_pipeline_attached(atomic: GPtr<u8>) -> i32 = cdecl 0x5D5B80 => unported;
    /// `CCarFXRenderer::SetCustomFXAtomicRenderPipelinesVMICB` (0x5D5B60)
    pub fn set_custom_fx_atomic_render_pipelines_vmicb(atomic: GPtr<u8>, data: u32) -> GPtr<u8> = cdecl 0x5D5B60 => unported;
    /// `CTagManager::SetupAtomic` (0x49CE10)
    pub fn tag_manager_setup_atomic(atomic: GPtr<u8>) = cdecl 0x49CE10 => unported;
    /// `RpAtomicClone` (0x749E60)
    pub fn rp_atomic_clone(atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x749E60 => unported;
    /// `RpAtomicDestroy` (0x749DC0)
    pub fn rp_atomic_destroy(atomic: GPtr<u8>) -> i32 = cdecl 0x749DC0 => unported;
    /// `RpAtomicSetFrame` (0x74BF20)
    pub fn rp_atomic_set_frame(atomic: GPtr<u8>, frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
    /// `RwFrameCreate` (0x7F0410)
    pub fn rw_frame_create() -> GPtr<u8> = cdecl 0x7F0410 => unported;
    /// `RwFrameDestroy` (0x7F05A0)
    pub fn rw_frame_destroy(frame: GPtr<u8>) -> i32 = cdecl 0x7F05A0 => unported;
    /// CRT `strncmp` (0x8214D0)
    pub fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
}

fn rw(this: MiPtr) -> GPtr<u8> {
    f::<GPtr<u8>>(this, mi::RW_OBJECT).read()
}

/// `this + 0x20`: `CDamageAtomicModelInfo::m_pDamagedAtomic`.
const DAMAGED_ATOMIC: isize = 0x20;

/// `CAtomicModelInfo::GetAtomicFromDistance(float)` (0x4C44B0, `ret 4`).
#[hook(addr = 0x4C44B0, conv = thiscall, name = "CAtomicModelInfo::GetAtomicFromDistance", expect = "D9 05 18 F1 B6 00", this = "CAtomicModelInfo")]
pub fn get_atomic_from_distance(this: MiPtr, distance: f32) -> GPtr<u8> {
    get_atomic_from_distance_with(this, LOD_DIST_MULTIPLIER.get(), distance)
}

/// [`get_atomic_from_distance`] with the multiplier as a value (L1).
pub fn get_atomic_from_distance_with(this: MiPtr, multiplier: f32, distance: f32) -> GPtr<u8> {
    let d = f::<f32>(this, mi::DRAW_DISTANCE).read();
    if in_draw_range(multiplier, d, distance) { rw(this) } else { GPtr::NULL }
}

/// `multiplier * draw_distance > distance` as the original's `fld; fmul; fcomp; test ah, 0x41`
/// (product kept in ST0 under PC=24; less, equal and unordered are all "no").
pub fn in_draw_range(multiplier: f32, draw_distance: f32, distance: f32) -> bool {
    X87::from_f32(multiplier).fmul_m(draw_distance).fcom(X87::from_f32(distance)) == Cmp::Greater
}

/// First prefix table of `SetupVehicleUpgradeFlags` (address, text, id): `0x8000 | id << 10`.
const UPGRADE_TABLE_A: [(u32, &[u8], u32); 13] = [
    (0x85BB88, b"bnt_", 0),
    (0x85BB78, b"bntl_", 1),
    (0x85BB70, b"bntr_", 2),
    (0x85BB68, b"spl_", 6),
    (0x85BB60, b"wg_l_", 8),
    (0x85BB58, b"wg_r_", 9),
    (0x85BB50, b"fbb_", 0xA),
    (0x85BB48, b"bbb_", 0xB),
    (0x85BB40, b"lgt_", 0xC),
    (0x85BB3C, b"rf_", 0xE),
    (0x85BB34, b"nto_", 0xF),
    (0x85BB28, b"hydralics", 0x10),
    (0x85BB20, b"stereo", 0x11),
];

/// Second prefix table: `0x8100 | (id << 10)` (16-bit shift).
const UPGRADE_TABLE_B: [(u32, &[u8], u32); 8] = [
    (0x85BB80, b"chss_", 1),
    (0x85BB18, b"wheel_", 2),
    (0x85BB10, b"exh_", 0x13),
    (0x85BB08, b"fbmp_", 0xC),
    (0x85BB00, b"rbmp_", 0xD),
    (0x85BAF8, b"misc_a_", 0x14),
    (0x85BAF0, b"misc_b_", 0x15),
    (0x85BAE8, b"misc_c_", 0x16),
];

/// `CAtomicModelInfo::SetupVehicleUpgradeFlags(const char*)` (0x4C4570, `ret 4`; entry is
/// `nop; jmp 0x40222E` into a protection chunk that allocates the 0xBC-byte frame).
#[hook(addr = 0x4C4570, conv = thiscall, name = "CAtomicModelInfo::SetupVehicleUpgradeFlags", expect = "90 E9 B8 DC F3 FF", this = "CAtomicModelInfo")]
pub fn setup_vehicle_upgrade_flags(this: MiPtr, name: GPtr<u8>) {
    setup_vehicle_upgrade_flags_with(this, |addr, _text, len| {
        strncmp(GPtr::from_addr(addr as usize), name, len) == 0
    })
}

/// [`setup_vehicle_upgrade_flags`] with the prefix test as a closure `(address, text, length) ->
/// equal` (the original's `strncmp(prefix, name, strlen(prefix)) == 0`; L1 passes a host model).
pub fn setup_vehicle_upgrade_flags_with(this: MiPtr, mut matches: impl FnMut(u32, &[u8], u32) -> bool) {
    let fl = f::<u16>(this, mi::FLAGS);
    let w = fl.read();
    if w & 0x8000 != 0 {
        return;
    }
    fl.write(w & 0xFF);
    for (addr, s, id) in UPGRADE_TABLE_A {
        if matches(addr, s, s.len() as u32) {
            let v = (((id | 0xFFE0) << 10) & 0xFFFF) as u16;
            fl.write(fl.read() | v);
            return;
        }
    }
    for (addr, s, id) in UPGRADE_TABLE_B {
        if matches(addr, s, s.len() as u32) {
            let v = (((id << 10) & 0xFFFF) as u16) | 0x8100;
            fl.write(fl.read() | v);
            return;
        }
    }
}

/// The renderer setup `SetAtomic`/`SetDamagedAtomic` share: building pipeline → `AtomicSetup`,
/// else car pipeline → `SetCustomFXAtomicRenderPipelinesVMICB(atomic, 0)`.
fn setup_pipeline(atomic: GPtr<u8>) {
    if is_cbpc_pipeline_attached(atomic) != 0 {
        custom_building_atomic_setup(atomic);
    } else if is_ccpc_pipeline_attached(atomic) != 0 {
        set_custom_fx_atomic_render_pipelines_vmicb(atomic, 0);
    }
}

/// `CAtomicModelInfo::Init` (0x4C4430): `jmp CBaseModelInfo::Init`.
#[hook(addr = 0x4C4430, conv = thiscall, name = "CAtomicModelInfo::Init", expect = "E9 DB 06 00 00 90", this = "CAtomicModelInfo")]
pub fn init(this: MiPtr) {
    base_init(this)
}

/// `CAtomicModelInfo::SetAtomic(RpAtomic*)` (0x4C4360, `ret 4`).
#[hook(addr = 0x4C4360, conv = thiscall, name = "CAtomicModelInfo::SetAtomic", expect = "56 8B F1 8B 46 1C", this = "CAtomicModelInfo")]
pub fn set_atomic(this: MiPtr, atomic: GPtr<u8>) {
    let old = rw(this);
    if !old.is_null() {
        add_2dfx_count(this, atomic_2dfx_count(old), true);
    }
    f::<GPtr<u8>>(this, mi::RW_OBJECT).write(atomic);
    add_2dfx_count(this, atomic_2dfx_count(atomic), false);
    add_tex_dictionary_ref(this);
    if vcall_get_anim_file_index(this) != -1 {
        add_anim_block_ref(vcall_get_anim_file_index(this));
    }
    setup_pipeline(atomic);
    let w = f::<u16>(this, mi::FLAGS).read();
    if w & 0x8000 == 0 && w & 0x7800 == 0x3000 {
        tag_manager_setup_atomic(atomic);
    }
    let p = f::<u8>(this, mi::FLAGS);
    p.write(p.read() | 1);
}

/// `CAtomicModelInfo::DeleteRwObject` (0x4C4440).
#[hook(addr = 0x4C4440, conv = thiscall, name = "CAtomicModelInfo::DeleteRwObject", expect = "56 8B F1 8B 46 1C", this = "CAtomicModelInfo")]
pub fn delete_rw_object(this: MiPtr) {
    let atomic = rw(this);
    if atomic.is_null() {
        return;
    }
    add_2dfx_count(this, atomic_2dfx_count(atomic), true);
    let frame = atomic.byte_add(4).cast::<GPtr<u8>>().read();
    rp_atomic_destroy(atomic);
    rw_frame_destroy(frame);
    f::<u32>(this, mi::RW_OBJECT).write(0);
    remove_tex_dictionary_ref(this);
    if vcall_get_anim_file_index(this) != -1 {
        remove_anim_block_ref(vcall_get_anim_file_index(this));
    }
}

/// Clone of `atomic` on a new frame, the frame's modelling matrix (+0x10) copied from `m` first.
fn clone_on_new_frame(atomic: GPtr<u8>, m: Option<GPtr<RwMatrix>>) -> GPtr<u8> {
    let clone = rp_atomic_clone(atomic);
    let frame = rw_frame_create();
    if let Some(m) = m {
        frame.byte_add(0x10).cast::<RwMatrix>().write(m.read());
    }
    rp_atomic_set_frame(clone, frame);
    clone
}

/// `CAtomicModelInfo::CreateInstance(RwMatrix*)` (0x4C44D0, `ret 4`).
#[hook(addr = 0x4C44D0, conv = thiscall, name = "CAtomicModelInfo::CreateInstance-rwmat", expect = "53 8B D9 8B 43 1C", this = "CAtomicModelInfo")]
pub fn create_instance_rwmat(this: MiPtr, m: GPtr<RwMatrix>) -> GPtr<u8> {
    if rw(this).is_null() {
        return GPtr::NULL;
    }
    add_ref(this);
    let clone = clone_on_new_frame(rw(this), Some(m));
    remove_ref(this);
    clone
}

/// `CAtomicModelInfo::CreateInstance()` (0x4C4530).
#[hook(addr = 0x4C4530, conv = thiscall, name = "CAtomicModelInfo::CreateInstance-void", expect = "56 8B F1 8B 46 1C", this = "CAtomicModelInfo")]
pub fn create_instance_void(this: MiPtr) -> GPtr<u8> {
    if rw(this).is_null() {
        return GPtr::NULL;
    }
    add_ref(this);
    let clone = clone_on_new_frame(rw(this), None);
    remove_ref(this);
    clone
}

/// `CAtomicModelInfo::AsAtomicModelInfoPtr` (0x4C5560): `this` (unhookable: 3 bytes).
#[sa_core::native(addr = 0x4C5560, conv = thiscall, name = "CAtomicModelInfo::AsAtomicModelInfoPtr")]
pub fn as_atomic_model_info_ptr(this: MiPtr) -> MiPtr {
    this
}

/// `CAtomicModelInfo::GetModelType` (0x4C5570): AL = 1 (unhookable).
#[sa_core::native(addr = 0x4C5570, conv = thiscall, name = "CAtomicModelInfo::GetModelType")]
pub fn get_model_type(_this: MiPtr) -> u8 {
    1
}

/// `CAtomicModelInfo::GetRwModelType` (0x4C5580): 1 (`rpATOMIC`).
#[hook(addr = 0x4C5580, conv = thiscall, name = "CAtomicModelInfo::GetRwModelType", expect = "B8 01 00 00 00", this = "CAtomicModelInfo")]
pub fn get_rw_model_type(_this: MiPtr) -> u32 {
    1
}

/// `CAtomicModelInfo::~CAtomicModelInfo` (0x4C5590, catalogue `FUN_004c5590`): vptr = 0x85BB9C.
#[hook(addr = 0x4C5590, conv = thiscall, name = "CAtomicModelInfo::~CAtomicModelInfo", expect = "C7 01 9C BB 85 00", this = "CAtomicModelInfo")]
pub fn destructor(this: MiPtr) {
    f::<u32>(this, mi::VTABLE).write(VTBL_BASE_MODEL_INFO);
}

/// The scalar deleting destructor shape shared by the model-info classes (slot 0): destructor,
/// `operator delete(this)` when `flags & 1`, return `this`.
pub fn scalar_delete(this: MiPtr, flags: u32, dtor: fn(MiPtr)) -> MiPtr {
    dtor(this);
    if flags & 1 != 0 {
        operator_delete(this.cast());
    }
    this
}

/// `CAtomicModelInfo`'s scalar deleting destructor (0x4C6210, `ret 4`).
#[hook(addr = 0x4C6210, conv = thiscall, name = "CAtomicModelInfo::`scalar deleting destructor'", expect = "56 8B F1 E8 78 F3 FF FF", this = "CAtomicModelInfo")]
pub fn scalar_deleting_destructor(this: MiPtr, flags: u32) -> MiPtr {
    scalar_delete(this, flags, atomic_destructor)
}

/// The flags `CAtomicModelInfo::SetAtomicModelInfoFlags` writes after `SetBaseModelInfoFlags`:
/// (byte +0x13 bit 0, special-type word updates, byte +0x13 bit 2), as a pure function of the
/// flags word.
pub fn atomic_flags_word(w: u16, flags: u32) -> u16 {
    let mut w = w;
    w = if flags & 1 != 0 { w | 0x0100 } else { w & !0x0100 };
    let sets: [(u32, u16, u16); 5] = [
        (0x200, 0xA7FF, 0x2000),
        (0x400, 0xAFFF, 0x2800),
        (0x800, 0xBFFF, 0x3800),
        (0x2000, 0x8FFF, 0x0800),
        (0x4000, 0x97FF, 0x1000),
    ];
    for (bit, and, or) in sets {
        if flags & bit != 0 {
            w = (w & and) | or;
        }
    }
    w = if flags & 0x8000 != 0 { w | 0x0400 } else { w & !0x0400 };
    for (bit, and, or) in [(0x10_0000u32, 0xB7FFu16, 0x3000u16), (0x40_0000, 0xDFFF, 0x5800), (0x8_0000, 0xD7FF, 0x5000)] {
        if flags & bit != 0 {
            w = (w & and) | or;
        }
    }
    w
}

/// `CAtomicModelInfo::SetAtomicModelInfoFlags(CAtomicModelInfo*, uint32)` (0x5B3B20, cdecl):
/// `SetBaseModelInfoFlags`, then [`atomic_flags_word`] (as byte and word read-modify-writes).
#[hook(addr = 0x5B3B20, conv = cdecl, name = "CAtomicModelInfo::SetAtomicModelInfoFlags", expect = "8B 54 24 04 53")]
pub fn set_atomic_model_info_flags(m: MiPtr, flags: u32) {
    set_base_model_info_flags(m, flags);
    let p = f::<u16>(m, mi::FLAGS);
    p.write(atomic_flags_word(p.read(), flags));
}

// ------------------------------------------------------------------- CDamageAtomicModelInfo ---

/// `CDamageAtomicModelInfo::Init` (0x4C48B0): base init, `m_pDamagedAtomic = 0`.
#[hook(addr = 0x4C48B0, conv = thiscall, name = "CDamageAtomicModelInfo::Init", expect = "56 8B F1 E8 58 02 00 00", this = "CDamageAtomicModelInfo")]
pub fn damage_init(this: MiPtr) {
    base_init(this);
    f::<u32>(this, DAMAGED_ATOMIC).write(0);
}

/// `CDamageAtomicModelInfo::SetDamagedAtomic(RpAtomic*)` (0x4C48D0, `ret 4`).
#[hook(addr = 0x4C48D0, conv = thiscall, name = "CDamageAtomicModelInfo::SetDamagedAtomic", expect = "56 8B 74 24 08", this = "CDamageAtomicModelInfo")]
pub fn set_damaged_atomic(this: MiPtr, atomic: GPtr<u8>) {
    f::<GPtr<u8>>(this, DAMAGED_ATOMIC).write(atomic);
    setup_pipeline(atomic);
}

/// `CDamageAtomicModelInfo::CreateInstance(RwMatrix*)` (0x4C4910, `ret 4`): with
/// `ms_bCreateDamagedVersion`, a clone of the damaged atomic (null when there is none; no ref
/// counting); otherwise a tail jump to `CAtomicModelInfo::CreateInstance(RwMatrix*)`.
#[hook(addr = 0x4C4910, conv = thiscall, name = "CDamageAtomicModelInfo::CreateInstance-rwmat", expect = "A0 B0 B0 A9 00", this = "CDamageAtomicModelInfo")]
pub fn damage_create_instance_rwmat(this: MiPtr, m: GPtr<RwMatrix>) -> GPtr<u8> {
    if CREATE_DAMAGED_VERSION.get() == 0 {
        return atomic_create_instance_rwmat(this, m);
    }
    let d = f::<GPtr<u8>>(this, DAMAGED_ATOMIC).read();
    if d.is_null() {
        return GPtr::NULL;
    }
    clone_on_new_frame(d, Some(m))
}

/// `CDamageAtomicModelInfo::CreateInstance()` (0x4C4960): as above; the undamaged path is an
/// inlined `CAtomicModelInfo::CreateInstance()`.
#[hook(addr = 0x4C4960, conv = thiscall, name = "CDamageAtomicModelInfo::CreateInstance-void", expect = "A0 B0 B0 A9 00", this = "CDamageAtomicModelInfo")]
pub fn damage_create_instance_void(this: MiPtr) -> GPtr<u8> {
    if CREATE_DAMAGED_VERSION.get() != 0 {
        let d = f::<GPtr<u8>>(this, DAMAGED_ATOMIC).read();
        if d.is_null() {
            return GPtr::NULL;
        }
        return clone_on_new_frame(d, None);
    }
    create_instance_void(this)
}

/// `CDamageAtomicModelInfo::DeleteRwObject` (0x4C49D0): destroys the damaged atomic and its frame,
/// then tail-jumps to `CAtomicModelInfo::DeleteRwObject`.
#[hook(addr = 0x4C49D0, conv = thiscall, name = "CDamageAtomicModelInfo::DeleteRwObject", expect = "56 8B F1 8B 46 20", this = "CDamageAtomicModelInfo")]
pub fn damage_delete_rw_object(this: MiPtr) {
    let d = f::<GPtr<u8>>(this, DAMAGED_ATOMIC).read();
    if !d.is_null() {
        let frame = d.byte_add(4).cast::<GPtr<u8>>().read();
        rp_atomic_destroy(d);
        rw_frame_destroy(frame);
        f::<u32>(this, DAMAGED_ATOMIC).write(0);
    }
    atomic_delete_rw_object(this)
}

/// `CDamageAtomicModelInfo::AsDamageAtomicModelInfoPtr` (0x4C55C0): `this` (unhookable).
#[sa_core::native(addr = 0x4C55C0, conv = thiscall, name = "CDamageAtomicModelInfo::AsDamageAtomicModelInfoPtr")]
pub fn as_damage_atomic_model_info_ptr(this: MiPtr) -> MiPtr {
    this
}

/// `CDamageAtomicModelInfo::~CDamageAtomicModelInfo` (0x4C55D0, `FUN_004c55d0`).
#[hook(addr = 0x4C55D0, conv = thiscall, name = "CDamageAtomicModelInfo::~CDamageAtomicModelInfo", expect = "C7 01 9C BB 85 00", this = "CDamageAtomicModelInfo")]
pub fn damage_destructor(this: MiPtr) {
    f::<u32>(this, mi::VTABLE).write(VTBL_BASE_MODEL_INFO);
}

/// `CDamageAtomicModelInfo`'s scalar deleting destructor (0x4C6230).
#[hook(addr = 0x4C6230, conv = thiscall, name = "CDamageAtomicModelInfo::`scalar deleting destructor'", expect = "56 8B F1 E8 98 F3 FF FF", this = "CDamageAtomicModelInfo")]
pub fn damage_scalar_deleting_destructor(this: MiPtr, flags: u32) -> MiPtr {
    scalar_delete(this, flags, damage_atomic_destructor)
}

// ---------------------------------------------------------------------- CLodAtomicModelInfo ---

/// `CLodAtomicModelInfo::Init` (0x4C54D0): `jmp CAtomicModelInfo::Init`.
#[hook(addr = 0x4C54D0, conv = thiscall, name = "CLodAtomicModelInfo::Init", expect = "E9 5B EF FF FF 90", this = "CLodAtomicModelInfo")]
pub fn lod_init(this: MiPtr) {
    atomic_init(this)
}

/// `CLodAtomicModelInfo::GetModelType` (0x4C5600): AL = 8 (unhookable).
#[sa_core::native(addr = 0x4C5600, conv = thiscall, name = "CLodAtomicModelInfo::GetModelType")]
pub fn lod_get_model_type(_this: MiPtr) -> u8 {
    8
}

/// `CLodAtomicModelInfo::AsLodAtomicModelInfoPtr` (0x4C5610): `this` (unhookable).
#[sa_core::native(addr = 0x4C5610, conv = thiscall, name = "CLodAtomicModelInfo::AsLodAtomicModelInfoPtr")]
pub fn as_lod_atomic_model_info_ptr(this: MiPtr) -> MiPtr {
    this
}

/// `CLodAtomicModelInfo::~CLodAtomicModelInfo` (0x4C5620, `FUN_004c5620`).
#[hook(addr = 0x4C5620, conv = thiscall, name = "CLodAtomicModelInfo::~CLodAtomicModelInfo", expect = "C7 01 9C BB 85 00", this = "CLodAtomicModelInfo")]
pub fn lod_destructor(this: MiPtr) {
    f::<u32>(this, mi::VTABLE).write(VTBL_BASE_MODEL_INFO);
}

/// `CLodAtomicModelInfo`'s scalar deleting destructor (0x4C6250).
#[hook(addr = 0x4C6250, conv = thiscall, name = "CLodAtomicModelInfo::`scalar deleting destructor'", expect = "56 8B F1 E8 C8 F3 FF FF", this = "CLodAtomicModelInfo")]
pub fn lod_scalar_deleting_destructor(this: MiPtr, flags: u32) -> MiPtr {
    scalar_delete(this, flags, lod_atomic_destructor)
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_ATOMIC_FROM_DISTANCE,
    HOOK_SETUP_VEHICLE_UPGRADE_FLAGS,
    HOOK_INIT,
    HOOK_SET_ATOMIC,
    HOOK_DELETE_RW_OBJECT,
    HOOK_CREATE_INSTANCE_RWMAT,
    HOOK_CREATE_INSTANCE_VOID,
    HOOK_GET_RW_MODEL_TYPE,
    HOOK_DESTRUCTOR,
    HOOK_SCALAR_DELETING_DESTRUCTOR,
    HOOK_SET_ATOMIC_MODEL_INFO_FLAGS,
    HOOK_DAMAGE_INIT,
    HOOK_SET_DAMAGED_ATOMIC,
    HOOK_DAMAGE_CREATE_INSTANCE_RWMAT,
    HOOK_DAMAGE_CREATE_INSTANCE_VOID,
    HOOK_DAMAGE_DELETE_RW_OBJECT,
    HOOK_DAMAGE_DESTRUCTOR,
    HOOK_DAMAGE_SCALAR_DELETING_DESTRUCTOR,
    HOOK_LOD_INIT,
    HOOK_LOD_DESTRUCTOR,
    HOOK_LOD_SCALAR_DELETING_DESTRUCTOR,
];
