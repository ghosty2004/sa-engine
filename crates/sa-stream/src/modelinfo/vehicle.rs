//! `CVehicleModelInfo` (vtable 0x85C5C8, size 0x308), its `CVehicleStructure` and `CLinkedUpgradeList`,
//! and the static helpers of `VehicleModelInfo.cpp` (PORT-STREAM, owner C).
//!
//! Source: the 1.0 US bytes (capstone over the exe; `.HOODLUM` bodies and protection chunks followed)
//! with the Ghidra decompilation and gta-reversed `Models/VehicleModelInfo.cpp` (pin 20d8d2a) as
//! reading aids; `docs/re/specs/STREAM/CVehicleModelInfo*.md` where they say so. Where they differ the
//! original is kept (see `docs/arch/stream-port.md`):
//! - `ChooseComponent`/`ChooseSecondComponent`/`ChooseComponent-Global`: the original RNG formula
//!   (`GetRandomNumberInRange<int>` 0x407180 and the inline `_ftol((rand() & 0xFFFF) / 32768 * 3)`),
//!   gr-deviations `VehicleModelInfo.cpp:344/348/363/367/1627/1632/1639` (keep-original);
//! - `SetClump` has no dirt-material pass (gr-deviations `VehicleModelInfo.cpp:242`);
//! - `LoadVehicleColours` keeps the unreset r/g/b locals and does not retry `"%d.%d %d"`
//!   (gr-deviations `VehicleModelInfo.cpp:1328`).
//!
//! RenderWare (`Rp*`/`Rw*`), CVisibilityPlugins, CTxdStore and the CRT are called through `orig!`.
//! Callbacks handed to RenderWare are the ORIGINAL addresses (their own hooks decide Rust vs original).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GameData};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::shared::{field, model_info};

/// Field offsets of `CVehicleModelInfo` (gta-reversed `VehicleModelInfo.h`, checked against the bytes).
pub mod vmi {
    /// `CClumpModelInfo::m_nAnimFileIndex` / hierarchy union (+0x20)
    pub const CLUMP_ANIM: isize = 0x20;
    pub const PLATE_MATERIAL: isize = 0x24;
    pub const PLATE_TEXT: isize = 0x28;
    pub const PLATE_TYPE: isize = 0x31;
    pub const GAME_NAME: isize = 0x32;
    pub const VEHICLE_TYPE: isize = 0x3C;
    pub const WHEEL_SIZE_FRONT: isize = 0x40;
    pub const WHEEL_SIZE_REAR: isize = 0x44;
    pub const WHEEL_MODEL_INDEX: isize = 0x48;
    pub const HANDLING_ID: isize = 0x4A;
    pub const NUM_DOORS: isize = 0x4C;
    pub const VEHICLE_CLASS: isize = 0x4D;
    pub const FLAGS: isize = 0x4E;
    pub const WHEEL_UPGRADE_CLASS: isize = 0x4F;
    pub const TIMES_USED: isize = 0x50;
    pub const FRQ: isize = 0x52;
    pub const EXTRA_COMPS: isize = 0x54;
    pub const BIKE_STEER_ANGLE: isize = 0x58;
    pub const VEHICLE_STRUCT: isize = 0x5C;
    pub const DIRT_MATERIALS: isize = 0x230;
    pub const PRIMARY_COLORS: isize = 0x2B0;
    pub const SECONDARY_COLORS: isize = 0x2B8;
    pub const TERTIARY_COLORS: isize = 0x2C0;
    pub const QUATERNARY_COLORS: isize = 0x2C8;
    pub const NUM_COLOR_VARIATIONS: isize = 0x2D0;
    pub const LAST_COLOR_VARIATION: isize = 0x2D1;
    pub const CURRENT_COLORS: isize = 0x2D2;
    pub const UPGRADES: isize = 0x2D6;
    pub const REMAP_TXDS: isize = 0x2FA;
    pub const ANIM_BLOCK: isize = 0x304;
}

/// Field offsets of `CVehicleModelInfo::CVehicleStructure` (0x314).
pub mod vs {
    pub const DUMMY_POS: isize = 0x00;
    pub const UPGRADES: isize = 0xB4;
    /// `m_aUpgrades[i].m_nParentComponentId` = UPGRADES + i * 0x20 + 0x1C
    pub const UPGRADE_PARENT: isize = 0xD0;
    pub const EXTRAS: isize = 0x2F4;
    pub const NUM_EXTRAS: isize = 0x30C;
    pub const MASK_DAMAGEABLE: isize = 0x310;
}

/// `CVehicleModelInfo::CLinkedUpgradeList` (0x7C).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkedUpgradeList {
    pub upgrade1: [i16; 30],
    pub upgrade2: [i16; 30],
    pub count: u32,
}
// SAFETY: integers only.
#[allow(unsafe_code)]
unsafe impl GameData for LinkedUpgradeList {}
const _: () = assert!(core::mem::size_of::<LinkedUpgradeList>() == 0x7C);

impl LinkedUpgradeList {
    pub const ZERO: LinkedUpgradeList = LinkedUpgradeList {
        upgrade1: [0; 30],
        upgrade2: [0; 30],
        count: 0,
    };
}

/// `RpMaterialList` (RW, 0xC on 32-bit targets).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MaterialList {
    pub materials: GPtr<u8>,
    pub count: i32,
    pub space: i32,
}

sa_core::globals! {
    list GLOBALS;
    /// `CVehicleModelInfo::ms_linkedUpgrades`
    pub static LINKED_UPGRADES @ 0xB4E6D8: LinkedUpgradeList = LinkedUpgradeList::ZERO;
    /// `CVehicleModelInfo::ms_vehicleDescs` (RwObjectNameIdAssocation* per vehicle type)
    pub static VEHICLE_DESCS @ 0x8A7740: [u32; 12] = [
        0x8A6468, 0x8A6B80, 0x8A6D90, 0x8A6978, 0x8A6750, 0x8A6F80, 0x8A7068, 0x8A71B8, 0x8A7218,
        0x8A7270, 0x8A73D0, 0x8A7530,
    ];
    /// `CVehicleModelInfo::ms_wheelFrameIDs`
    pub static WHEEL_FRAME_IDS @ 0x8A7770: [i32; 4] = [5, 7, 2, 4];
    /// `CVehicleModelInfo::ms_compsToUse`
    pub static COMPS_TO_USE @ 0x8A6458: [i8; 2] = [-2, -2];
    /// `gLightSurfProps` (ambient, specular, diffuse)
    pub static LIGHT_SURF_PROPS @ 0x8A645C: [f32; 3] = [16.0, 0.0, 0.0];
    /// `CVehicleModelInfo` env-map coefficient (0.25f) used by SetEnvironmentMapCB
    pub static ENV_MAP_COEFF @ 0x8A7780: f32 = 0.25;
    /// 1.15f, used by PreprocessHierarchy (wheel-frame scale)
    pub static WHEEL_SCALE @ 0x8A7784: f32 = 1.15;
    /// `gRestoreEntries` (address, value) pairs, terminated by a null address
    pub static RESTORE_ENTRIES @ 0xB4DBE8: [u32; 512] = [0; 512];
    /// `ms_lightsOn`
    pub static LIGHTS_ON @ 0xB4E3E8: [u8; 4] = [0; 4];
    /// `gpWhiteTexture`
    pub static WHITE_TEXTURE @ 0xB4E3EC: GPtr<u8> = GPtr::NULL;
    /// `ms_currentCol`
    pub static CURRENT_COL @ 0xB4E3F0: [u8; 4] = [0; 4];
    /// `ms_upgradeWheels[4][15]`
    pub static UPGRADE_WHEELS @ 0xB4E3F8: [i16; 60] = [0; 60];
    /// `ms_numWheelUpgrades`
    pub static NUM_WHEEL_UPGRADES @ 0xB4E470: [i16; 4] = [0; 4];
    /// `ms_compsUsed`
    pub static COMPS_USED @ 0xB4E478: [i8; 2] = [0; 2];
    /// `ms_pRemapTexture`
    pub static REMAP_TEXTURE @ 0xB4E47C: GPtr<u8> = GPtr::NULL;
    /// `ms_vehicleColourTable` (CRGBA x 128)
    pub static VEHICLE_COLOUR_TABLE @ 0xB4E480: [u32; 128] = [0; 128];
    /// `vehicleTxd`
    pub static VEHICLE_TXD @ 0xB4E688: GPtr<u8> = GPtr::NULL;
    /// `ms_pLightsTexture`
    pub static LIGHTS_TEXTURE @ 0xB4E68C: GPtr<u8> = GPtr::NULL;
    /// `ms_pLightsOnTexture`
    pub static LIGHTS_ON_TEXTURE @ 0xB4E690: GPtr<u8> = GPtr::NULL;
    /// `SavedTextureFindCallback`
    pub static SAVED_TEXTURE_FIND_CALLBACK @ 0xB4E6A0: u32 = 0;
    /// `carFrame` (env-map frame)
    pub static CAR_FRAME @ 0xB4E6B8: GPtr<u8> = GPtr::NULL;
    /// cleared by ShutdownEnvironmentMaps (gr: unnamed)
    pub static ENV_MAP_UNK_B4E6BC @ 0xB4E6BC: u8 = 0;
    /// Protection chunk 0x406413 of UseCommonVehicleTexDicationary: `rol [0x1560810], cl=[0x71A0E2]`
    /// = 0x4C7510 (FindTextureCB); the second operand is [`PROT_FINDTEX_B`].
    pub static PROT_FINDTEX_A @ 0x1560810: u32 = 0x8EA2_0009;
    // foreign (owner: CWeather, not ported)
    /// `CWeather::NewWeatherType`
    pub static NEW_WEATHER_TYPE @ 0xC8131C: i16 = 0;
    /// `CWeather::OldWeatherType`
    pub static OLD_WEATHER_TYPE @ 0xC81320: i16 = 0;
}

/// Handling flags of handling id `h` (`mod_HandlingManager.m_aVehicleHandling[h].m_nHandlingFlags`,
/// 0xC2B9DC + h * 0xE0 + 0xCC).
fn handling_flags(h: u8) -> u32 {
    GPtr::<u32>::from_addr(0xC2B9DC + h as usize * 0xE0 + 0xCC).read()
}

/// `CCheat::m_aCheatsActive[]` bytes used by ChooseVehicleColour (0x969150 pink cars, 0x969151 black cars).
fn cheat(addr: usize) -> bool {
    GPtr::<u8>::from_addr(addr).read() != 0
}

/// Original string literals and callback addresses.
pub mod lit {
    pub const NULL_STR: u32 = 0x85BBDC; // "null"
    pub const REMAP: u32 = 0x85C5C0; // "remap"
    pub const DAM: u32 = 0x85C610; // "_dam"
    pub const OK: u32 = 0x85C60C; // "_ok"
    pub const VLO: u32 = 0x85C624; // "_vlo"
    pub const WINDSCREEN: u32 = 0x85C618; // "windscreen"
    pub const TOPROTOR: u32 = 0x85BF74; // "toprotor"
    pub const REARROTOR: u32 = 0x85C634; // "rearrotor"
    pub const MOVING_ROTOR: u32 = 0x85C1C4; // "moving_rotor"
    pub const MOVING_ROTOR2: u32 = 0x85C1A4; // "moving_rotor2"
    pub const BOAT_HI: u32 = 0x85C62C; // "boat_hi"
    pub const PARTICLE: u32 = 0x85C648; // "particle"
    pub const WHITE: u32 = 0x85C640; // "white"
    pub const DEFAULT: u32 = 0x85C658; // "DEFAULT"
    // CVisibilityPlugins render callbacks
    pub const RENDER_VEHICLE_HI_DETAIL_CB: u32 = 0x733240;
    pub const RENDER_VEHICLE_HI_DETAIL_ALPHA_CB: u32 = 0x733F80;
    pub const RENDER_VEHICLE_REALLY_LOW_DETAIL_CB: u32 = 0x7331E0;
    pub const RENDER_HELI_ROTOR_ALPHA_CB: u32 = 0x7340B0;
    pub const RENDER_HELI_TAIL_ROTOR_ALPHA_CB: u32 = 0x734170;
    pub const RENDER_VEHICLE_REALLY_LOW_DETAIL_CB_BIG_VEHICLE: u32 = 0x732820;
    pub const RENDER_VEHICLE_HI_DETAIL_CB_BIG_VEHICLE: u32 = 0x733420;
    pub const RENDER_VEHICLE_HI_DETAIL_ALPHA_CB_BIG_VEHICLE: u32 = 0x734370;
    pub const RENDER_VEHICLE_LO_DETAIL_CB_BOAT: u32 = 0x7334F0;
    pub const RENDER_VEHICLE_HI_DETAIL_ALPHA_CB_BOAT: u32 = 0x7344A0;
    pub const RENDER_VEHICLE_HI_DETAIL_CB_BOAT: u32 = 0x733550;
    pub const RENDER_TRAIN_HI_DETAIL_CB: u32 = 0x733330;
    pub const RENDER_TRAIN_HI_DETAIL_ALPHA_CB: u32 = 0x734240;
    pub const RENDER_WHEEL_ATOMIC_CB: u32 = 0x7323C0;
    pub const ATOMIC_DEFAULT_RENDER_CB: u32 = 0x7491C0;
    // our own callbacks (original addresses)
    pub const HAS_ALPHA_MATERIAL_CB: u32 = 0x4C77C0;
    pub const SET_ATOMIC_RENDERER_CB: u32 = 0x4C77E0;
    pub const SET_ATOMIC_RENDERER_CB_REAL_HELI: u32 = 0x4C7870;
    pub const SET_ATOMIC_RENDERER_CB_PLANE: u32 = 0x4C7930;
    pub const SET_ATOMIC_RENDERER_CB_BOAT: u32 = 0x4C79A0;
    pub const SET_ATOMIC_RENDERER_CB_TRAIN: u32 = 0x4C7AA0;
    pub const SET_ATOMIC_FLAG_CB: u32 = 0x4C7B90;
    pub const GET_OK_AND_DAMAGED_ATOMIC_CB: u32 = 0x4C7BD0;
    pub const MOVE_OBJECTS_CB: u32 = 0x4C7700;
    pub const REMOVE_WINDOW_ALPHA_CB: u32 = 0x4C83B0;
    pub const SET_EDITABLE_MATERIALS_CB_MAT: u32 = 0x4C8220;
    pub const SET_EDITABLE_MATERIALS_CB_ATOMIC: u32 = 0x4C83E0;
    pub const SET_ENVIRONMENT_MAP_CB: u32 = 0x4C8840;
    pub const SET_ENV_MAP_COEFF_CB: u32 = 0x4C88B0;
    pub const STORE_ATOMIC_USED_MATERIALS_CB: u32 = 0x4C8B60;
    pub const COLLAPSE_FRAMES_CB: u32 = 0x4C8E30;
    pub const SET_ENVIRONMENT_MAP_ATOMIC_CB: u32 = 0x4C9410;
    pub const SET_ENV_MAP_COEFF_ATOMIC_CB: u32 = 0x4C9430;
    pub const FIND_TEXTURE_CB: u32 = 0x4C7510;
    pub const FIND_FRAME_FROM_NAME_WITHOUT_ID_CB: u32 = 0x4C52F0;
    pub const FIND_FRAME_FROM_ID_CB: u32 = 0x4C5350;
}

/// Calls into other (possibly original) functions.
pub mod call {
    use super::*;
    orig! {
        /// RwTexDictionaryFindNamedTexture (0x7F39F0)
        pub fn rw_tex_dictionary_find_named_texture(dict: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F39F0 => unported;
        /// RwTexDictionaryGetCurrent (0x7F3A90)
        pub fn rw_tex_dictionary_get_current() -> GPtr<u8> = cdecl 0x7F3A90 => unported;
        /// RwTextureGetFindCallBack (0x7F34F0)
        pub fn rw_texture_get_find_call_back() -> u32 = cdecl 0x7F34F0 => unported;
        /// RwTextureSetFindCallBack (0x7F34D0)
        pub fn rw_texture_set_find_call_back(cb: u32) -> u32 = cdecl 0x7F34D0 => unported;
        /// RwTextureRead (0x7F3AC0)
        pub fn rw_texture_read(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
        /// RwTextureDestroy (0x7F3820)
        pub fn rw_texture_destroy(tex: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
        /// RpClumpForAllAtomics (0x749B70)
        pub fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B70 => unported;
        /// RwFrameForAllObjects (0x7F1200)
        pub fn rw_frame_for_all_objects(frame: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F1200 => unported;
        /// RwFrameForAllChildren (0x7F0DC0)
        pub fn rw_frame_for_all_children(frame: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0DC0 => unported;
        /// RpGeometryForAllMaterials (0x74C790)
        pub fn rp_geometry_for_all_materials(geo: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C790 => unported;
        /// RwFrameDestroy (0x7F05A0)
        pub fn rw_frame_destroy(frame: GPtr<u8>) -> i32 = cdecl 0x7F05A0 => unported;
        /// RwFrameCreate (0x7F0410)
        pub fn rw_frame_create() -> GPtr<u8> = cdecl 0x7F0410 => unported;
        /// RwMatrixRotate (0x7F1FD0)
        pub fn rw_matrix_rotate(m: GPtr<u8>, axis: GPtr<u8>, angle: f32, op: u32) -> GPtr<u8> = cdecl 0x7F1FD0 => unported;
        /// RwFrameUpdateObjects (0x7F0910)
        pub fn rw_frame_update_objects(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
        /// RwFrameGetLTM (0x7F0990)
        pub fn rw_frame_get_ltm(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
        /// RwFrameRemoveChild (0x7F0CD0)
        pub fn rw_frame_remove_child(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0CD0 => unported;
        /// RwFrameAddChild (0x7F0B00)
        pub fn rw_frame_add_child(parent: GPtr<u8>, child: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0B00 => unported;
        /// RwMatrixCreate (0x7F2A50)
        pub fn rw_matrix_create() -> GPtr<u8> = cdecl 0x7F2A50 => unported;
        /// RwMatrixTransform (0x7F25A0)
        pub fn rw_matrix_transform(m: GPtr<u8>, t: GPtr<u8>, op: u32) -> GPtr<u8> = cdecl 0x7F25A0 => unported;
        /// RwMatrixDestroy (0x7F2A20)
        pub fn rw_matrix_destroy(m: GPtr<u8>) -> i32 = cdecl 0x7F2A20 => unported;
        /// RwV3dTransformPoints (0x7EDD90)
        pub fn rw_v3d_transform_points(out: GPtr<u8>, pts: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
        /// RpAtomicSetFrame (0x74BF20)
        pub fn rp_atomic_set_frame(atomic: GPtr<u8>, frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
        /// RpAtomicClone (0x749E60)
        pub fn rp_atomic_clone(atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x749E60 => unported;
        /// RpClumpAddAtomic (0x74A490)
        pub fn rp_clump_add_atomic(clump: GPtr<u8>, atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A490 => unported;
        /// RpClumpRemoveAtomic (0x74A4C0)
        pub fn rp_clump_remove_atomic(clump: GPtr<u8>, atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A4C0 => unported;
        /// RpMaterialSetTexture (0x74DBC0)
        pub fn rp_material_set_texture(mat: GPtr<u8>, tex: GPtr<u8>) -> GPtr<u8> = cdecl 0x74DBC0 => unported;
        /// RpMatFXMaterialGetEffects (0x812140)
        pub fn rp_mat_fx_material_get_effects(mat: GPtr<u8>) -> i32 = cdecl 0x812140 => unported;
        /// RpMatFXMaterialSetEffects (0x811C80)
        pub fn rp_mat_fx_material_set_effects(mat: GPtr<u8>, fx: i32) -> GPtr<u8> = cdecl 0x811C80 => unported;
        /// RpMatFXMaterialSetEnvMapFrame (0x8125B0)
        pub fn rp_mat_fx_material_set_env_map_frame(mat: GPtr<u8>, frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x8125B0 => unported;
        /// RpMatFXMaterialSetEnvMapCoefficient (0x812680)
        pub fn rp_mat_fx_material_set_env_map_coefficient(mat: GPtr<u8>, coef: f32) -> GPtr<u8> = cdecl 0x812680 => unported;
        /// _rpMaterialListAppendMaterial (0x74E350)
        pub fn rp_material_list_append_material(list: GPtr<u8>, mat: GPtr<u8>) -> i32 = cdecl 0x74E350 => unported;
        /// _rpMaterialListDeinitialize (0x74E150)
        pub fn rp_material_list_deinitialize(list: GPtr<u8>) -> GPtr<u8> = cdecl 0x74E150 => unported;
        /// GetFrameNodeName (0x72FB30)
        pub fn get_frame_node_name(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x72FB30 => unported;
        /// RwHelper GetFirstObject (0x7348C0)
        pub fn get_first_object(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x7348C0 => unported;
        /// CVisibilityPlugins::SetAtomicFlag (0x7322B0)
        pub fn set_atomic_flag(atomic: GPtr<u8>, flag: u32) = cdecl 0x7322B0 => unported;
        /// CVisibilityPlugins::ClearAtomicFlag (0x732310)
        pub fn clear_atomic_flag(atomic: GPtr<u8>, flag: u32) = cdecl 0x732310 => unported;
        /// CVisibilityPlugins::GetAtomicId (0x732370)
        pub fn get_atomic_id(atomic: GPtr<u8>) -> u32 = cdecl 0x732370 => unported;
        /// CVisibilityPlugins::SetAtomicRenderCallback (0x7328A0)
        pub fn set_atomic_render_callback(atomic: GPtr<u8>, cb: u32) = cdecl 0x7328A0 => unported;
        /// CVisibilityPlugins::ClearClumpForAllAtomicsFlag (0x732350)
        pub fn clear_clump_for_all_atomics_flag(clump: GPtr<u8>, flag: u32) = cdecl 0x732350 => unported;
        /// CVisibilityPlugins::GetFrameHierarchyId (0x732A20)
        pub fn get_frame_hierarchy_id(frame: GPtr<u8>) -> i32 = cdecl 0x732A20 => unported;
        /// CTxdStore::FindTxdSlot(name) (0x731850)
        pub fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => crate::txdstore::find_txd_slot_name;
        /// CTxdStore::PushCurrentTxd (0x7316A0)
        pub fn push_current_txd() = cdecl 0x7316A0 => crate::txdstore::push_current_txd;
        /// CTxdStore::PopCurrentTxd (0x7316B0)
        pub fn pop_current_txd() = cdecl 0x7316B0 => crate::txdstore::pop_current_txd;
        /// CTxdStore::SetCurrentTxd (0x7319C0)
        pub fn set_current_txd(slot: i32) = cdecl 0x7319C0 => crate::txdstore::set_current_txd;
        /// CClumpModelInfo::Init (0x4C4E40)
        pub fn clump_init(mi: GPtr<u8>) = thiscall 0x4C4E40 => unported;
        /// CClumpModelInfo::SetClump (0x4C4F70)
        pub fn clump_set_clump(mi: GPtr<u8>, clump: GPtr<u8>) = thiscall 0x4C4F70 => unported;
        /// CClumpModelInfo::SetFrameIds (0x4C5460)
        pub fn clump_set_frame_ids(mi: GPtr<u8>, ids: u32) = thiscall 0x4C5460 => unported;
        /// CClumpModelInfo::GetFrameFromId (0x4C53C0)
        pub fn get_frame_from_id(clump: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x4C53C0 => crate::modelinfo::clump::get_frame_from_id;
        /// CClumpModelInfo::CreateInstance (0x4C5140)
        pub fn clump_create_instance(mi: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C5140 => unported;
        /// CClumpModelInfo::DeleteRwObject (0x4C4E70)
        pub fn clump_delete_rw_object(mi: GPtr<u8>) = thiscall 0x4C4E70 => unported;
        /// CAnimManager::GetAnimationBlockIndex(name) (0x4D3990)
        pub fn get_animation_block_index(name: GPtr<u8>) -> i32 = cdecl 0x4D3990 => unported;
        /// CModelInfo::GetModelInfo(name, minId, maxId) (0x4C5A20)
        pub fn get_model_info_min_max(name: GPtr<u8>, min: i32, max: i32) -> GPtr<u8> = cdecl 0x4C5A20 => unported;
        /// CModelInfo::GetModelInfo(name, &id) (0x4C5940)
        pub fn get_model_info_full(name: GPtr<u8>, id: GPtr<i32>) -> GPtr<u8> = cdecl 0x4C5940 => unported;
        /// CWorld::FindPlayerVehicle(playerId, bIncludeRemote) (0x56E0D0)
        pub fn find_player_vehicle(player: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
        /// CGeneral::GetRandomNumberInRange<int> (0x407180)
        pub fn get_random_number_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => sa_core::general::get_random_number_in_range_i32;
        /// CCustomCarPlateMgr::GeneratePlateText (0x6FD5B0)
        pub fn generate_plate_text(buf: GPtr<u8>, len: i32) -> u8 = cdecl 0x6FD5B0 => unported;
        /// CCustomCarPlateMgr::SetupClump (0x6FE0F0)
        pub fn plate_setup_clump(clump: GPtr<u8>, text: GPtr<u8>, ty: u8) -> GPtr<u8> = cdecl 0x6FE0F0 => unported;
        /// CCarFXRenderer::CustomCarPipeClumpSetup (0x5D5B40)
        pub fn custom_car_pipe_clump_setup(clump: GPtr<u8>) -> u8 = cdecl 0x5D5B40 => unported;
        /// CMemoryMgr::Malloc (0x72F420)
        pub fn malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
        /// CMemoryMgr::LockScratchPad (0x72F4A0)
        pub fn lock_scratch_pad() = cdecl 0x72F4A0 => unported;
        /// CMemoryMgr::InitScratchPad (0x72F480)
        pub fn init_scratch_pad() = cdecl 0x72F480 => unported;
        /// CMemoryMgr::ReleaseScratchPad (0x72F4B0)
        pub fn release_scratch_pad() = cdecl 0x72F4B0 => unported;
        /// CTimer::GetCurrentTimeInCycles (0x561A80)
        pub fn get_current_time_in_cycles() -> u32 = cdecl 0x561A80 => sa_core::timer::get_current_time_in_cycles;
        /// CTimer::GetCyclesPerMillisecond (0x561A40)
        pub fn get_cycles_per_millisecond() -> u32 = cdecl 0x561A40 => sa_core::timer::get_cycles_per_millisecond;
        /// CQuaternion::Set(const RwMatrix&) (0x59C3E0)
        pub fn quaternion_set(q: GPtr<u8>, m: GPtr<u8>) = thiscall 0x59C3E0 => unported;
        /// CPool<CVehicleStructure>::New via CVehicleStructure::operator new (0x4C9570)
        pub fn vehicle_structure_new(size: u32) -> GPtr<u8> = cdecl 0x4C9570 => unported;
        /// CVehicleStructure::CVehicleStructure (0x4C8D60)
        pub fn vehicle_structure_ctor(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C8D60 => super::vehicle_structure_ctor;
        /// CVehicleModelInfo::~CVehicleModelInfo (0x4C5920)
        pub fn dtor(mi: GPtr<u8>) = thiscall 0x4C5920 => super::destructor;
        /// CRT `operator delete` = `free` (0x8214BD)
        pub fn free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
        /// CRT `operator new` (0x82119A)
        pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
        /// CRT `_stricmp` (0x8229B6)
        pub fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
        /// CRT `strncmp` (0x8214D0)
        pub fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
        /// CRT `strstr` (0x822650)
        pub fn strstr(a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x822650 => unported;
        /// CRT `strncpy` (0x821F40)
        pub fn strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
        /// CRT `isdigit` (0x822789)
        pub fn isdigit(c: i32) -> i32 = cdecl 0x822789 => unported;
        /// CRT `fopen` via 0x8232D8
        pub fn fopen(name: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x8232D8 => unported;
        /// CRT `fclose` (0x82318B)
        pub fn fclose(f: GPtr<u8>) -> i32 = cdecl 0x82318B => unported;
        /// CRT `strtok` (0x82244B)
        pub fn strtok(s: GPtr<u8>, delim: GPtr<u8>) -> GPtr<u8> = cdecl 0x82244B => unported;
        /// CFileMgr::ReadLine (0x5389B0)
        pub fn read_line(f: GPtr<u8>, buf: GPtr<u8>, len: i32) -> bool = cdecl 0x5389B0 => sa_core::filemgr::read_line;
        /// CFileLoader::LoadLine(FILE*) (0x536F80)
        pub fn load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => crate::fileloader::load_line_file;
    }
}
use call::*;

fn s(addr: u32) -> GPtr<u8> {
    GPtr::from_addr(addr as usize)
}
fn rd8(p: GPtr<u8>, off: isize) -> u8 {
    field::<u8, u8>(p, off).read()
}
fn wr8(p: GPtr<u8>, off: isize, v: u8) {
    field::<u8, u8>(p, off).write(v)
}
fn rd16(p: GPtr<u8>, off: isize) -> i16 {
    field::<i16, u8>(p, off).read()
}
fn wr16(p: GPtr<u8>, off: isize, v: i16) {
    field::<i16, u8>(p, off).write(v)
}
fn rd32(p: GPtr<u8>, off: isize) -> u32 {
    field::<u32, u8>(p, off).read()
}
fn wr32(p: GPtr<u8>, off: isize, v: u32) {
    field::<u32, u8>(p, off).write(v)
}
fn rdp(p: GPtr<u8>, off: isize) -> GPtr<u8> {
    field::<GPtr<u8>, u8>(p, off).read()
}
fn wrp(p: GPtr<u8>, off: isize, v: GPtr<u8>) {
    field::<GPtr<u8>, u8>(p, off).write(v)
}
fn rdf(p: GPtr<u8>, off: isize) -> f32 {
    field::<f32, u8>(p, off).read()
}

/// `repe cmpsb` with `ecx = lit.len()` (the literal includes its NUL): equal iff the first `len`
/// bytes match, comparing byte by byte and stopping at the first difference.
fn cmpsb_eq(p: GPtr<u8>, lit: &[u8]) -> bool {
    for (k, b) in lit.iter().enumerate() {
        if rd8(p, k as isize) != *b {
            return false;
        }
    }
    true
}

/// `_ftol` of an x87 value: truncation toward zero, low dword.
fn ftol(v: X87) -> i32 {
    v.ftol() as u32 as i32
}

// ------------------------------------------------------------------------------- the class ---

/// `CVehicleModelInfo::~CVehicleModelInfo` (0x4C5920): only the final vptr store (0x85BB9C).
#[hook(addr = 0x4C5920, conv = thiscall, name = "CVehicleModelInfo::~CVehicleModelInfo", expect = "C7 01 9C BB 85 00", this = "CVehicleModelInfo")]
pub fn destructor(mi: GPtr<u8>) {
    wr32(mi, 0, 0x85BB9C);
}

/// `CVehicleModelInfo::GetNumDoors` (0x4C73C0): `movsx` of the byte at +0x4C.
#[hook(addr = 0x4C73C0, conv = thiscall, name = "CVehicleModelInfo::GetNumDoors", expect = "0F BE 41 4C C3", this = "CVehicleModelInfo")]
pub fn get_num_doors(mi: GPtr<u8>) -> i32 {
    rd8(mi, vmi::NUM_DOORS) as i8 as i32
}

/// `CLinkedUpgradeList::AddUpgradeLink` (0x4C74B0, `ret 8`): no bounds check.
#[hook(addr = 0x4C74B0, conv = thiscall, name = "CLinkedUpgradeList::AddUpgradeLink", expect = "8B 41 78 66 8B 54 24 04")]
pub fn add_upgrade_link(list: GPtr<u8>, u1: i16, u2: i16) {
    let n = rd32(list, 0x78) as i32 as isize;
    wr16(list, n * 2, u1);
    let n = rd32(list, 0x78) as i32 as isize;
    wr16(list, n * 2 + 0x3C, u2);
    wr32(list, 0x78, rd32(list, 0x78).wrapping_add(1));
}

/// `CLinkedUpgradeList::FindOtherUpgrade` (0x4C74D0, `ret 4`): searches from the last link down; the
/// result is in AX, the upper half of EAX holds the upper half of the loop index (0 for every real
/// list); not found: `or ax, 0xFFFF` over the index 0.
#[hook(addr = 0x4C74D0, conv = thiscall, name = "CLinkedUpgradeList::FindOtherUpgrade", expect = "8B 41 78 85 C0")]
pub fn find_other_upgrade(list: GPtr<u8>, upgrade: i16) -> u32 {
    let mut i = rd32(list, 0x78);
    while i != 0 {
        i = i.wrapping_sub(1);
        let k = i as i32 as isize;
        if rd16(list, k * 2) == upgrade {
            return (i & 0xFFFF_0000) | rd16(list, k * 2 + 0x3C) as u16 as u32;
        }
        if rd16(list, k * 2 + 0x3C) == upgrade {
            return (i & 0xFFFF_0000) | rd16(list, k * 2) as u16 as u32;
        }
    }
    0xFFFF
}

/// `CVehicleModelInfo::GetModelType` (0x4C7650): `mov al, 6; ret` (too short to hook).
#[sa_core::native(addr = 0x4C7650, conv = thiscall, name = "CVehicleModelInfo::GetModelType")]
pub fn get_model_type(_mi: GPtr<u8>) -> u8 {
    6
}

/// `CVehicleModelInfo::GetAnimFileIndex` (0x4C7660).
#[hook(addr = 0x4C7660, conv = thiscall, name = "CVehicleModelInfo::GetAnimFileIndex", expect = "8B 81 04 03 00 00", this = "CVehicleModelInfo")]
pub fn get_anim_file_index(mi: GPtr<u8>) -> u32 {
    rd32(mi, vmi::ANIM_BLOCK)
}

/// `CVehicleModelInfo::HasAlphaMaterialCB` (0x4C77C0): alpha != 0xFF → `*data = 1`, return null.
#[hook(addr = 0x4C77C0, conv = cdecl, name = "CVehicleModelInfo::HasAlphaMaterialCB", expect = "8B 44 24 04 80 78 07 FF")]
pub fn has_alpha_material_cb(mat: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    if rd8(mat, 7) != 0xFF {
        data.write(1);
        return GPtr::NULL;
    }
    mat
}

/// The inline weather test of rule 2 (ONLY_WHEN_RAINING): Old/New weather type 16 or 8.
fn rule_ok(rule: u32, old: i16, new: i16) -> bool {
    rule != 2 || old == 16 || new == 16 || old == 8 || new == 8
}

/// `CVehicleModelInfo::IsValidCompRule` (0x4C7E10).
#[hook(addr = 0x4C7E10, conv = cdecl, name = "CVehicleModelInfo::IsValidCompRule", expect = "8B 44 24 04 83 E8 02")]
pub fn is_valid_comp_rule(rule: i32) -> bool {
    if rule != 2 {
        return true;
    }
    let old = OLD_WEATHER_TYPE.get();
    if old == 16 {
        return true;
    }
    let new = NEW_WEATHER_TYPE.get();
    new == 16 || old == 8 || new == 8
}

/// Pure part of `GetListOfComponentsNotUsedByRules` (weather values read once at entry).
pub fn list_of_components_not_used_by_rules(comps: u32, num_extras: i32, old: i16, new: i16, out: &mut [i32; 6]) -> i32 {
    let mut list = [0i32, 1, 2, 3, 4, 5];
    let rule_a = comps >> 12 & 0xF;
    if rule_a != 0 && rule_ok(rule_a, old, new) {
        if rule_a == 4 {
            return 0;
        }
        for sh in [0, 4, 8] {
            let c = comps >> sh & 0xF;
            if c != 0xF {
                list[c as usize] = 0xF;
            }
        }
    }
    let rule_b = comps >> 28;
    if rule_b != 0 && rule_ok(rule_b, old, new) {
        for sh in [16, 20, 24] {
            let c = comps >> sh & 0xF;
            if c != 0xF {
                list[c as usize] = 0xF;
            }
        }
    }
    let mut n = 0;
    let mut i = 0;
    while i < num_extras {
        if list[i as usize] != 0xF {
            out[n as usize] = i;
            n += 1;
        }
        i += 1;
    }
    n
}

/// `CVehicleModelInfo::GetListOfComponentsNotUsedByRules` (0x4C7E50, cdecl). Components 0..14 index a
/// 6-entry stack array without a bounds check in the original; values 6..14 never occur in the data.
#[hook(addr = 0x4C7E50, conv = cdecl, name = "CVehicleModelInfo::GetListOfComponentsNotUsedByRules", expect = "83 EC 18 8B 54 24 1C")]
pub fn get_list_of_components_not_used_by_rules(comps: u32, num_extras: i32, out: GPtr<i32>) -> i32 {
    let new = NEW_WEATHER_TYPE.get();
    let old = OLD_WEATHER_TYPE.get();
    let mut tmp = [0i32; 6];
    let n = list_of_components_not_used_by_rules(comps, num_extras, old, new, &mut tmp);
    for (k, v) in tmp.iter().enumerate().take(n.max(0) as usize) {
        out.wrapping_index(k as i32).write(*v);
    }
    n
}

/// `CVehicleModelInfo::CountCompsInRule` (0x4C7F80): nibbles != 0xF, arithmetic shift.
#[hook(addr = 0x4C7F80, conv = cdecl, name = "CVehicleModelInfo::CountCompsInRule", expect = "8B 4C 24 04 33 C0")]
pub fn count_comps_in_rule(comps: i32) -> i32 {
    let (mut c, mut n) = (comps, 0);
    while c != 0 {
        if c & 0xF != 0xF {
            n += 1;
        }
        c >>= 4;
    }
    n
}

/// `CVehicleModelInfo::IsUpgradeAvailable` (0x4C8200, `ret 4`): parent component id >= 0.
#[hook(addr = 0x4C8200, conv = thiscall, name = "CVehicleModelInfo::IsUpgradeAvailable", expect = "8B 44 24 04 8B 49 5C", this = "CVehicleModelInfo")]
pub fn is_upgrade_available(mi: GPtr<u8>, upgrade: i32) -> bool {
    let st = rdp(mi, vmi::VEHICLE_STRUCT);
    (rd32(st, vs::UPGRADE_PARENT + (upgrade.wrapping_shl(5)) as isize) as i32) >= 0
}

/// Appends `(addr, value)` to the restore list (`*cursor` advances by 8).
fn push_restore(cursor: GPtr<GPtr<u8>>, addr: GPtr<u8>, value: u32) {
    let c = cursor.read();
    c.cast::<GPtr<u8>>().write(addr);
    let c = cursor.read().byte_add(4);
    cursor.write(c);
    c.cast::<u32>().write(value);
    cursor.write(cursor.read().byte_add(4));
}

/// `CVehicleModelInfo::SetEditableMaterialsCB(RpMaterial*, void*)` (0x4C8220): remap texture,
/// light textures/surface props and the four editable body colours; every change is recorded in the
/// restore list `*data` first.
#[hook(addr = 0x4C8220, conv = cdecl, name = "CVehicleModelInfo::SetEditableMaterialsCB-RpMaterial", expect = "8B 44 24 04 8B 0D 7C E4 B4 00")]
pub fn set_editable_materials_cb_material(mat: GPtr<u8>, data: GPtr<GPtr<u8>>) -> GPtr<u8> {
    let rgb = rd32(mat, 4) & 0xFF_FFFF;
    let color = field::<u8, u8>(mat, 4);
    if !REMAP_TEXTURE.get().is_null() {
        let tex = rdp(mat, 0);
        if !tex.is_null() && rd8(tex, 0x10) == b'#' {
            push_restore(data, mat, rd32(mat, 0));
            wrp(mat, 0, REMAP_TEXTURE.get());
        }
    }
    if rdp(mat, 0) == LIGHTS_TEXTURE.get() {
        let light: i32 = match rgb {
            0x00AFFF => 0,
            0xC8FF00 => 1,
            0x00FFB9 => 2,
            0x003CFF => 3,
            _ => -1,
        };
        push_restore(data, color, rd32(mat, 4));
        wr32(mat, 4, rd32(mat, 4) | 0xFF_FFFF);
        if light != -1 && LIGHTS_ON.get()[light as usize] != 0 {
            push_restore(data, mat, rd32(mat, 0));
            let surf = field::<u8, u8>(mat, 0xC);
            push_restore(data, surf, rd32(mat, 0xC));
            wrp(mat, 0, LIGHTS_ON_TEXTURE.get());
            let p = LIGHT_SURF_PROPS.get();
            field::<f32, u8>(mat, 0xC).write(p[0]);
            field::<f32, u8>(mat, 0x10).write(p[1]);
            field::<f32, u8>(mat, 0x14).write(p[2]);
        }
    } else {
        let idx = match rgb {
            0x00FF3C => 0,
            0xAF00FF => 1,
            0xFFFF00 => 2,
            0xFF00FF => 3,
            _ => return mat,
        };
        let col = CURRENT_COL.get()[idx] as usize;
        push_restore(data, color, rd32(mat, 4));
        let t = VEHICLE_COLOUR_TABLE.ptr().cast::<u8>();
        wr8(mat, 4, rd8(t, col as isize * 4));
        wr8(mat, 5, rd8(t, col as isize * 4 + 1));
        wr8(mat, 6, rd8(t, col as isize * 4 + 2));
    }
    mat
}

/// `CVehicleModelInfo::RemoveWindowAlphaCB` (0x4C83B0): alpha < 0xFF (unsigned) → record and clear
/// the whole colour dword. Returns the material in EAX (unchanged register).
#[hook(addr = 0x4C83B0, conv = cdecl, name = "CVehicleModelInfo::RemoveWindowAlphaCB", expect = "8B 44 24 04 80 78 07 FF")]
pub fn remove_window_alpha_cb(mat: GPtr<u8>, data: GPtr<GPtr<u8>>) -> GPtr<u8> {
    if rd8(mat, 7) < 0xFF {
        let color = field::<u8, u8>(mat, 4);
        push_restore(data, color, rd32(mat, 4));
        wr32(mat, 4, 0);
    }
    mat
}

/// `CVehicleModelInfo::ResetEditableMaterials` (0x4C8460, cdecl; the clump argument is unused): writes
/// every recorded value back, walking `gRestoreEntries` until a null address.
#[hook(addr = 0x4C8460, conv = cdecl, name = "CVehicleModelInfo::ResetEditableMaterials", expect = "8B 0D E8 DB B4 00")]
pub fn reset_editable_materials(_clump: GPtr<u8>) {
    let base = RESTORE_ENTRIES.ptr().cast::<u32>();
    let mut k = 0;
    let mut addr = base.read();
    while addr != 0 {
        let v = base.wrapping_index(k + 1).read();
        GPtr::<u32>::from_addr(addr as usize).write(v);
        k += 2;
        addr = base.wrapping_index(k).read();
    }
}

/// `CVehicleModelInfo::SetVehicleColour` (0x4C84B0, `ret 0x10`): `ms_currentCol` and the four
/// current colours of the model (store order of the original: [2], [0], [3], [1], +0x2D2, +0x2D5,
/// +0x2D3, +0x2D4).
#[hook(addr = 0x4C84B0, conv = thiscall, name = "CVehicleModelInfo::SetVehicleColour", expect = "8A 44 24 04 8A 54 24 08", this = "CVehicleModelInfo")]
pub fn set_vehicle_colour(mi: GPtr<u8>, prim: u8, sec: u8, tert: u8, quat: u8) {
    let c = CURRENT_COL.ptr().cast::<u8>();
    c.wrapping_index(2).write(tert);
    c.wrapping_index(0).write(prim);
    c.wrapping_index(3).write(quat);
    c.wrapping_index(1).write(sec);
    wr8(mi, 0x2D2, prim);
    wr8(mi, 0x2D5, quat);
    wr8(mi, 0x2D3, sec);
    wr8(mi, 0x2D4, tert);
}

/// `CVehicleModelInfo::GetNumRemaps` (0x4C86B0).
#[hook(addr = 0x4C86B0, conv = thiscall, name = "CVehicleModelInfo::GetNumRemaps", expect = "81 C1 FA 02 00 00", this = "CVehicleModelInfo")]
pub fn get_num_remaps(mi: GPtr<u8>) -> i32 {
    let mut n = 0;
    while rd16(mi, vmi::REMAP_TXDS + 2 * n as isize) != -1 && n < 4 {
        n += 1;
    }
    n
}

/// `CVehicleModelInfo::AddRemap` (0x4C86D0, `ret 4`): no capacity check (the 5th goes to +0x302).
#[hook(addr = 0x4C86D0, conv = thiscall, name = "CVehicleModelInfo::AddRemap", expect = "8D 91 FA 02 00 00", this = "CVehicleModelInfo")]
pub fn add_remap(mi: GPtr<u8>, txd: i16) {
    let n = get_num_remaps(mi);
    wr16(mi, vmi::REMAP_TXDS + 2 * n as isize, txd);
}

/// `CVehicleModelInfo::AddWheelUpgrade` (0x4C8700).
#[hook(addr = 0x4C8700, conv = cdecl, name = "CVehicleModelInfo::AddWheelUpgrade", expect = "8B 44 24 04 8B D0")]
pub fn add_wheel_upgrade(set: i32, model: i16) {
    let num = NUM_WHEEL_UPGRADES.ptr().cast::<i16>().wrapping_index(set);
    let n = num.read();
    UPGRADE_WHEELS
        .ptr()
        .cast::<i16>()
        .wrapping_index(set.wrapping_mul(15).wrapping_add(n as i32))
        .write(model);
    num.write(n.wrapping_add(1));
}

/// `CVehicleModelInfo::GetNumWheelUpgrades` (0x4C8740).
#[hook(addr = 0x4C8740, conv = cdecl, name = "CVehicleModelInfo::GetNumWheelUpgrades", expect = "8B 44 24 04 0F BF 04 45 70 E4 B4 00")]
pub fn get_num_wheel_upgrades(set: i32) -> i32 {
    NUM_WHEEL_UPGRADES.ptr().cast::<i16>().wrapping_index(set).read() as i32
}

/// `CVehicleModelInfo::GetWheelUpgrade` (0x4C8750).
#[hook(addr = 0x4C8750, conv = cdecl, name = "CVehicleModelInfo::GetWheelUpgrade", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn get_wheel_upgrade(set: i32, k: i32) -> i32 {
    UPGRADE_WHEELS
        .ptr()
        .cast::<i16>()
        .wrapping_index(set.wrapping_mul(15).wrapping_add(k))
        .read() as i32
}

/// `CVehicleModelInfo::DeleteVehicleColourTextures` (0x4C8770): empty (`ret`, too short to hook).
#[sa_core::native(addr = 0x4C8770, conv = cdecl, name = "CVehicleModelInfo::DeleteVehicleColourTextures")]
pub fn delete_vehicle_colour_textures() {}

/// `CVehicleModelInfo::GetCustomCarPlateText` (0x4C8970): `this + 0x28`, or null when empty.
#[hook(addr = 0x4C8970, conv = thiscall, name = "CVehicleModelInfo::GetCustomCarPlateText", expect = "8D 41 28 8A 08", this = "CVehicleModelInfo")]
pub fn get_custom_car_plate_text(mi: GPtr<u8>) -> GPtr<u8> {
    let t = field::<u8, u8>(mi, vmi::PLATE_TEXT);
    if t.read() != 0 { t } else { GPtr::NULL }
}

/// `CVehicleModelInfo::SetCustomCarPlateText` (0x4C8980, `ret 4`): `strncpy(+0x28, text, 8)`.
#[hook(addr = 0x4C8980, conv = thiscall, name = "CVehicleModelInfo::SetCustomCarPlateText", expect = "8B 44 24 04 85 C0", this = "CVehicleModelInfo")]
pub fn set_custom_car_plate_text(mi: GPtr<u8>, text: GPtr<u8>) {
    if !text.is_null() {
        strncpy(field(mi, vmi::PLATE_TEXT), text, 8);
    } else {
        wr8(mi, vmi::PLATE_TEXT, 0);
    }
}

/// Model ids hard-coded in GetMaximumNumberOfPassengersFromNumberOfDoors.
const MODEL_COACH: i32 = 0x1B5;
const MODEL_BUS: i32 = 0x1AF;

/// `CVehicleModelInfo::GetMaximumNumberOfPassengersFromNumberOfDoors` (0x4C89B0). The handling id
/// is read as a byte (`movzx [+0x4A]`); the door count of -1 / 0x197 / 0x1FC / 0x1A9 is forced to
/// 3 / 2 / 2 / 1 (model -1 still reads `ms_modelInfoPtrs[-1]` first, as the original).
#[hook(addr = 0x4C89B0, conv = cdecl, name = "CVehicleModelInfo::GetMaximumNumberOfPassengersFromNumberOfDoors", expect = "83 EC 0C 56 8B 74 24 14")]
pub fn get_maximum_number_of_passengers_from_number_of_doors(model: i32) -> i32 {
    let mi = model_info(model).cast::<u8>();
    let flags_of = || handling_flags(rd8(mi, vmi::HANDLING_ID));
    let ty = rd32(mi, vmi::VEHICLE_TYPE) as i32;
    if ty == 5 || ty == 11 {
        return 0;
    }
    let doors: u8 = if model == -1 {
        3
    } else if model == 0x197 || model == 0x1FC {
        2
    } else if model == 0x1A9 {
        1
    } else {
        let d = rd8(mi, vmi::NUM_DOORS);
        if d == 0 {
            if ty == 9 || ty == 10 || ty == 2 || flags_of() & 0x200 != 0 {
                let st = rdp(mi, vmi::VEHICLE_STRUCT);
                // seat-front dummy (+0x3C) all zero (x87 `fcomp`, NaN unequal) → 0
                if rdf(st, 0x3C) == 0.0 && rdf(st, 0x40) == 0.0 && rdf(st, 0x44) == 0.0 {
                    return 0;
                }
            } else if model == 0x1B9 || model == 0x234 {
                return 0;
            }
            return 1;
        }
        d
    };
    if flags_of() & 0x200 != 0 {
        let v = (doors as i32 - 2) / 2;
        return if v < 0 { 0 } else { v };
    }
    if model != MODEL_COACH && model != MODEL_BUS {
        return doors as i32 - 1;
    }
    8
}

/// `CVehicleModelInfo::SetupLightFlags(CVehicle*)` (0x4C8C90): `ms_lightsOn` from the vehicle's
/// light-state byte (+0x584): [1] = bit0, [0] = bit1, [3] = bit2, [2] = bit3.
#[hook(addr = 0x4C8C90, conv = cdecl, name = "CVehicleModelInfo::SetupLightFlags", expect = "8B 44 24 04 8A 88 84 05 00 00")]
pub fn setup_light_flags(vehicle: GPtr<u8>) {
    let l = LIGHTS_ON.ptr().cast::<u8>();
    l.wrapping_index(1).write(rd8(vehicle, 0x584) & 1);
    l.wrapping_index(0).write(rd8(vehicle, 0x584) >> 1 & 1);
    l.wrapping_index(3).write(rd8(vehicle, 0x584) >> 2 & 1);
    l.wrapping_index(2).write(rd8(vehicle, 0x584) >> 3 & 1);
}

/// `CVehicleModelInfo::CVehicleStructure::CVehicleStructure` (0x4C8D60): the vector constructor
/// iterator over the 18 upgrade descriptors runs no-op element ctors (0x4C73F0) and is not called;
/// then dummies = 0, parents = -1, extras = null, counters = 0 (spec CVehicleModelInfo.md).
#[hook(addr = 0x4C8D60, conv = thiscall, name = "CVehicleModelInfo::CVehicleStructure::CVehicleStructure", expect = "83 EC 0C 53 55")]
pub fn vehicle_structure_ctor(st: GPtr<u8>) -> GPtr<u8> {
    for i in 0..15isize {
        wr32(st, i * 12, 0);
        wr32(st, i * 12 + 4, 0);
        wr32(st, i * 12 + 8, 0);
    }
    for i in 0..18isize {
        wr32(st, vs::UPGRADE_PARENT + i * 0x20, 0xFFFF_FFFF);
    }
    for i in 0..6isize {
        wr32(st, vs::EXTRAS + i * 4, 0);
    }
    wr8(st, vs::NUM_EXTRAS, 0);
    wr32(st, vs::MASK_DAMAGEABLE, 0);
    st
}

/// `CVehicleModelInfo::GetWheelSize(bool front)` (0x6A06F0, `ret 4`, ST0).
#[hook(addr = 0x6A06F0, conv = thiscall, name = "CVehicleModelInfo::GetWheelSize", expect = "8A 44 24 04 84 C0", this = "CVehicleModelInfo")]
pub fn get_wheel_size(mi: GPtr<u8>, front: u8) -> f32 {
    if front != 0 {
        rdf(mi, vmi::WHEEL_SIZE_FRONT)
    } else {
        rdf(mi, vmi::WHEEL_SIZE_REAR)
    }
}

/// `CVehicleModelInfo::FindTextureCB` (0x4C7510): vehicle TXD, then the current TXD; for "remap*"
/// names a found texture gets `name[0] = '#'`, a missing one is looked up again as "#emap...".
#[hook(addr = 0x4C7510, conv = cdecl, name = "CVehicleModelInfo::FindTextureCB", expect = "A1 88 E6 B4 00")]
pub fn find_texture_cb(name: GPtr<u8>) -> GPtr<u8> {
    let t = rw_tex_dictionary_find_named_texture(VEHICLE_TXD.get(), name);
    if !t.is_null() {
        return t;
    }
    let cur = rw_tex_dictionary_get_current();
    let t = rw_tex_dictionary_find_named_texture(cur, name);
    if strncmp(name, s(lit::REMAP), 5) != 0 {
        return t;
    }
    if !t.is_null() {
        wr8(t, 0x10, b'#');
        return t;
    }
    let mut buf = [0u8; 32];
    let mut k = 0;
    loop {
        let c = rd8(name, k as isize);
        buf[k] = c;
        k += 1;
        if c == 0 {
            break;
        }
    }
    buf[0] = b'#';
    let p = GPtr::from_raw(buf.as_mut_ptr());
    let cur = rw_tex_dictionary_get_current();
    rw_tex_dictionary_find_named_texture(cur, p)
}

/// The rotate count of the protection chunk 0x406413: the original dword at 0x71A0E2, i.e. code bytes of
/// `CFont::GetStringWidth`'s entry (0x71A0E0). Since PORT-HUD-3 hooks that entry, the live bytes are the
/// hook's `jmp` (sa-hook's keycells redirect the original's own read at 0x40641C to a copy), so the port
/// uses the exe's value instead of reading the patched memory.
const PROT_FINDTEX_B: u32 = 0xFFCE_75D3;

/// `CVehicleModelInfo::UseCommonVehicleTexDicationary` (0x4C75A0): saves the find callback and
/// installs FindTextureCB, whose address the original computes in the protection chunk 0x406413.
#[hook(addr = 0x4C75A0, conv = cdecl, name = "CVehicleModelInfo::UseCommonVehicleTexDicationary", expect = "E8 4B BF 32 00")]
pub fn use_common_vehicle_tex_dicationary() {
    let saved = rw_texture_get_find_call_back();
    let a = PROT_FINDTEX_A.get();
    let cb = a.rotate_left(PROT_FINDTEX_B & 31);
    SAVED_TEXTURE_FIND_CALLBACK.set(saved);
    rw_texture_set_find_call_back(cb);
}

/// `CVehicleModelInfo::StopUsingCommonVehicleTexDicationary` (0x4C75C0).
#[hook(addr = 0x4C75C0, conv = cdecl, name = "CVehicleModelInfo::StopUsingCommonVehicleTexDicationary", expect = "A1 A0 E6 B4 00")]
pub fn stop_using_common_vehicle_tex_dicationary() {
    rw_texture_set_find_call_back(SAVED_TEXTURE_FIND_CALLBACK.get());
    SAVED_TEXTURE_FIND_CALLBACK.set(0);
}

/// `CVehicleModelInfo::SetAnimFile` (0x4C7670, `ret 4`): "null" (`_stricmp`) → -1, else a copy of
/// the name in `operator new(strlen + 1)`.
#[hook(addr = 0x4C7670, conv = thiscall, name = "CVehicleModelInfo::SetAnimFile", expect = "56 8B 74 24 08", this = "CVehicleModelInfo")]
pub fn set_anim_file(mi: GPtr<u8>, name: GPtr<u8>) {
    if stricmp(name, s(lit::NULL_STR)) == 0 {
        wr32(mi, vmi::ANIM_BLOCK, 0xFFFF_FFFF);
        return;
    }
    let mut len = 0;
    while rd8(name, len) != 0 {
        len += 1;
    }
    let p = operator_new(len as u32 + 1);
    wrp(mi, vmi::ANIM_BLOCK, p);
    for k in 0..=len {
        wr8(p, k, rd8(name, k));
    }
}

/// `CVehicleModelInfo::HideDamagedAtomicCB` (0x4C7720): "_dam" → hidden (flags byte = 0) +
/// SetAtomicFlag(2); "_ok" → SetAtomicFlag(1).
#[hook(addr = 0x4C7720, conv = cdecl, name = "CVehicleModelInfo::HideDamagedAtomicCB", expect = "56 8B 74 24 08")]
pub fn hide_damaged_atomic_cb(atomic: GPtr<u8>, _data: GPtr<u8>) -> GPtr<u8> {
    let frame = rdp(atomic, 4);
    if !strstr(get_frame_node_name(frame), s(lit::DAM)).is_null() {
        wr8(atomic, 2, 0);
        set_atomic_flag(atomic, 2);
        return atomic;
    }
    if !strstr(get_frame_node_name(frame), s(lit::OK)).is_null() {
        set_atomic_flag(atomic, 1);
    }
    atomic
}

/// `CVehicleModelInfo::HideAllComponentsAtomicCB` (0x4C7790): flags byte = `(id & data) ? 0 : 4`.
#[hook(addr = 0x4C7790, conv = cdecl, name = "CVehicleModelInfo::HideAllComponentsAtomicCB", expect = "56 8B 74 24 08")]
pub fn hide_all_components_atomic_cb(atomic: GPtr<u8>, data: u32) -> GPtr<u8> {
    let id = get_atomic_id(atomic);
    wr8(atomic, 2, if id & data != 0 { 0 } else { 4 });
    atomic
}

/// `CVehicleModelInfo::SetAtomicRendererCB_Heli` (0x4C7A30).
#[hook(addr = 0x4C7A30, conv = cdecl, name = "CVehicleModelInfo::SetAtomicRendererCB_Heli", expect = "56 8B 74 24 08")]
pub fn set_atomic_renderer_cb_heli(atomic: GPtr<u8>, _data: GPtr<u8>) -> GPtr<u8> {
    let name = get_frame_node_name(rdp(atomic, 4));
    if strncmp(name, s(lit::TOPROTOR), 8) == 0 {
        set_atomic_render_callback(atomic, lit::RENDER_HELI_ROTOR_ALPHA_CB);
    } else if strncmp(name, s(lit::REARROTOR), 9) == 0 {
        set_atomic_render_callback(atomic, lit::RENDER_HELI_TAIL_ROTOR_ALPHA_CB);
    } else {
        set_atomic_render_callback(atomic, 0);
    }
    atomic
}

/// `CVehicleModelInfo::SetAtomicRenderCallbacks` (0x4C7B10): per vehicle type (+0x3C).
#[hook(addr = 0x4C7B10, conv = thiscall, name = "CVehicleModelInfo::SetAtomicRenderCallbacks", expect = "8B 41 3C 83 F8 06", this = "CVehicleModelInfo")]
pub fn set_atomic_render_callbacks(mi: GPtr<u8>) {
    let clump = rdp(mi, 0x1C);
    match rd32(mi, vmi::VEHICLE_TYPE) {
        6 => {
            rp_clump_for_all_atomics(clump, lit::SET_ATOMIC_RENDERER_CB_TRAIN, GPtr::NULL);
        }
        8 | 4 => {
            rp_clump_for_all_atomics(clump, lit::SET_ATOMIC_RENDERER_CB_PLANE, GPtr::NULL);
        }
        5 => {
            rp_clump_for_all_atomics(clump, lit::SET_ATOMIC_RENDERER_CB_BOAT, clump);
        }
        3 => {
            rp_clump_for_all_atomics(clump, lit::SET_ATOMIC_RENDERER_CB_REAL_HELI, clump);
        }
        _ => {
            rp_clump_for_all_atomics(clump, lit::SET_ATOMIC_RENDERER_CB, clump);
        }
    }
}

/// `CVehicleModelInfo::SetAtomicFlagCB` (0x4C7B90).
#[hook(addr = 0x4C7B90, conv = cdecl, name = "CVehicleModelInfo::SetAtomicFlagCB", expect = "8B 44 24 08 56")]
pub fn set_atomic_flag_cb(object: GPtr<u8>, data: u32) -> GPtr<u8> {
    set_atomic_flag(object, data);
    object
}

/// `CVehicleModelInfo::ClearAtomicFlagCB` (0x4C7BB0).
#[hook(addr = 0x4C7BB0, conv = cdecl, name = "CVehicleModelInfo::ClearAtomicFlagCB", expect = "8B 44 24 08 56")]
pub fn clear_atomic_flag_cb(object: GPtr<u8>, data: u32) -> GPtr<u8> {
    clear_atomic_flag(object, data);
    object
}

/// `CVehicleModelInfo::GetOkAndDamagedAtomicCB` (0x4C7BD0): id bit0 → data[0], else bit1 → data[1].
#[hook(addr = 0x4C7BD0, conv = cdecl, name = "CVehicleModelInfo::GetOkAndDamagedAtomicCB", expect = "56 8B 74 24 08")]
pub fn get_ok_and_damaged_atomic_cb(atomic: GPtr<u8>, data: GPtr<GPtr<u8>>) -> GPtr<u8> {
    if get_atomic_id(atomic) & 1 != 0 {
        data.write(atomic);
        return atomic;
    }
    if get_atomic_id(atomic) & 2 != 0 {
        data.wrapping_index(1).write(atomic);
    }
    atomic
}

/// `CVehicleModelInfo::SetVehicleComponentFlags(RwFrame*, uint32 flags)` (0x4C7C10, `ret 8`).
#[hook(addr = 0x4C7C10, conv = thiscall, name = "CVehicleModelInfo::SetVehicleComponentFlags", expect = "53 8B 5C 24 0C", this = "CVehicleModelInfo")]
pub fn set_vehicle_component_flags(mi: GPtr<u8>, frame: GPtr<u8>, flags: u32) {
    let hflags_addr = 0xC2B9DC + rd8(mi, vmi::HANDLING_ID) as usize * 0xE0 + 0xCC;
    let hflags = || GPtr::<u32>::from_addr(hflags_addr).read();
    let set = |f: u32| {
        rw_frame_for_all_objects(frame, lit::SET_ATOMIC_FLAG_CB, GPtr::from_addr(f as usize));
    };
    if flags & 0x1000 != 0 {
        set(0x80);
    }
    if flags & 0x40_0000 != 0 {
        set(0x400);
    }
    if flags & 0x4_0000 != 0 {
        set(0x2000);
    }
    if flags & 0x80 != 0 {
        set(0x10);
    } else if flags & 0x100 != 0 && (hflags() & 1 != 0 || flags & 0x60 == 0) {
        set(0x20);
    } else if flags & 0x20 != 0 {
        set(4);
    } else if flags & 0x40 != 0 {
        set(8);
    }
    if flags & 0x8000 != 0 && ((hflags() as i32) < 0 || flags & 0x6000 != 0) {
        set(0x8000);
    }
    if flags & 0x2000 != 0 {
        set(0x100);
    } else if flags & 0x4000 != 0 {
        set(0x200);
    }
    if flags & 0x400 != 0 {
        set(0x40);
    }
}

/// `ChooseComponent(rule, comps)` (0x4C7FB0, the free function). Rule 4 tail-jumps into
/// `GetRandomNumberInRange(0, 5)` after overwriting its own argument slots (not reproduced: the
/// caller's argument area is dead after the call).
#[hook(addr = 0x4C7FB0, conv = cdecl, name = "CVehicleModelInfo::ChooseComponent-Global", expect = "8B 44 24 04 48")]
pub fn choose_component_global(rule: i32, comps: i32) -> i32 {
    match rule {
        1 | 2 => {
            let n = count_comps_in_rule(comps);
            let r = get_random_number_in_range(0, n);
            comps >> ((r << 2) & 31) & 0xF
        }
        3 => {
            let n = count_comps_in_rule(comps);
            let r = get_random_number_in_range(-1, n);
            if r == -1 {
                return -1;
            }
            comps >> ((r.wrapping_mul(4)) & 31) & 0xF
        }
        4 => get_random_number_in_range(0, 5),
        _ => -1,
    }
}

/// `CVehicleModelInfo::SetEditableMaterialsCB(RpAtomic*, void*)` (0x4C83E0): visible atomics only
/// (flags byte bit 2); id bit 0x1000 → RemoveWindowAlphaCB first.
#[hook(addr = 0x4C83E0, conv = cdecl, name = "CVehicleModelInfo::SetEditableMaterialsCB-RpAtomic", expect = "56 8B 74 24 08")]
pub fn set_editable_materials_cb_atomic(atomic: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    if rd8(atomic, 2) & 4 != 0 {
        if get_atomic_id(atomic) & 0x1000 != 0 {
            rp_geometry_for_all_materials(rdp(atomic, 0x18), lit::REMOVE_WINDOW_ALPHA_CB, data);
        }
        rp_geometry_for_all_materials(rdp(atomic, 0x18), lit::SET_EDITABLE_MATERIALS_CB_MAT, data);
    }
    atomic
}

/// `CVehicleModelInfo::SetEditableMaterials(RpClump*)` (0x4C8430): fills gRestoreEntries and
/// terminates it with a null address.
#[hook(addr = 0x4C8430, conv = cdecl, name = "CVehicleModelInfo::SetEditableMaterials", expect = "51 8B 4C 24 08")]
pub fn set_editable_materials(clump: GPtr<u8>) {
    let mut cursor: GPtr<u8> = RESTORE_ENTRIES.ptr().cast();
    let pc = GPtr::from_raw(&raw mut cursor);
    rp_clump_for_all_atomics(clump, lit::SET_EDITABLE_MATERIALS_CB_ATOMIC, pc.cast());
    cursor.cast::<u32>().write(0);
}

/// The colour pick of ChooseVehicleColour: advance the variation and copy the four colours.
fn pick_colours(mi: GPtr<u8>, out: [GPtr<u8>; 4], shift: i32) {
    let n = rd8(mi, vmi::NUM_COLOR_VARIATIONS) as i32;
    let v = (rd8(mi, vmi::LAST_COLOR_VARIATION) as i32).wrapping_add(shift);
    // `cdq; idiv` (signed); n > 0 here
    let r = v.wrapping_rem(n);
    wr8(mi, vmi::LAST_COLOR_VARIATION, r as u8);
    out[0].write(rd8(mi, vmi::PRIMARY_COLORS + (r as u8) as isize));
    out[1].write(rd8(mi, vmi::SECONDARY_COLORS + rd8(mi, vmi::LAST_COLOR_VARIATION) as isize));
    out[2].write(rd8(mi, vmi::TERTIARY_COLORS + rd8(mi, vmi::LAST_COLOR_VARIATION) as isize));
    out[3].write(rd8(mi, vmi::QUATERNARY_COLORS + rd8(mi, vmi::LAST_COLOR_VARIATION) as isize));
}

/// `CVehicleModelInfo::ChooseVehicleColour` (0x4C8500, `ret 0x14`): black-cars cheat → 0, pink-cars
/// cheat → 0x7E; else the next variation, and once more when the player's vehicle is this model
/// with exactly these colours.
#[hook(addr = 0x4C8500, conv = thiscall, name = "CVehicleModelInfo::ChooseVehicleColour", expect = "53 55 56 8B F1", this = "CVehicleModelInfo")]
pub fn choose_vehicle_colour(mi: GPtr<u8>, prim: GPtr<u8>, sec: GPtr<u8>, tert: GPtr<u8>, quat: GPtr<u8>, shift: i32) {
    if rd8(mi, vmi::NUM_COLOR_VARIATIONS) == 0 || cheat(0x969151) {
        prim.write(0);
        sec.write(0);
        tert.write(0);
        quat.write(0);
        return;
    }
    if cheat(0x969150) {
        prim.write(0x7E);
        sec.write(0x7E);
        tert.write(0x7E);
        quat.write(0x7E);
        return;
    }
    let out = [prim, sec, tert, quat];
    pick_colours(mi, out, shift);
    if rd8(mi, vmi::NUM_COLOR_VARIATIONS) <= 1 {
        return;
    }
    let veh = find_player_vehicle(-1, 0);
    if veh.is_null() {
        return;
    }
    let id = rd16(veh, 0x22) as i32;
    if model_info(id).cast::<u8>() != mi {
        return;
    }
    if rd8(veh, 0x434) == prim.read()
        && rd8(veh, 0x435) == sec.read()
        && rd8(veh, 0x436) == tert.read()
        && rd8(veh, 0x437) == quat.read()
    {
        pick_colours(mi, out, shift);
    }
}

/// `CVehicleModelInfo::GetMatFXEffectMaterialCB` (0x4C8810).
#[hook(addr = 0x4C8810, conv = cdecl, name = "CVehicleModelInfo::GetMatFXEffectMaterialCB", expect = "56 8B 74 24 08")]
pub fn get_mat_fx_effect_material_cb(mat: GPtr<u8>, data: GPtr<i32>) -> GPtr<u8> {
    if rp_mat_fx_material_get_effects(mat) != 0 {
        data.write(rp_mat_fx_material_get_effects(mat));
        return GPtr::NULL;
    }
    mat
}

/// The coefficient of SetEnvMapCoeffCB: `(float)(uint32)data * 0.001f` (x87 PC=24: `fild`, +2^32
/// when negative, `fmul`, stored as f32 argument).
pub fn env_map_coeff_from_data(data: i32) -> f32 {
    let mut v = X87::from_i32(data);
    if data < 0 {
        v = v.fadd_m(4294967296.0);
    }
    v.fmul_m(f32::from_bits(0x3A83_126F)).to_f32()
}

/// `CVehicleModelInfo::SetEnvMapCoeffCB` (0x4C88B0).
#[hook(addr = 0x4C88B0, conv = cdecl, name = "CVehicleModelInfo::SetEnvMapCoeffCB", expect = "56 8B 74 24 08")]
pub fn set_env_map_coeff_cb(mat: GPtr<u8>, data: i32) -> GPtr<u8> {
    if rp_mat_fx_material_get_effects(mat) == 2 {
        rp_mat_fx_material_set_env_map_coefficient(mat, env_map_coeff_from_data(data));
    }
    mat
}

/// `CVehicleModelInfo::scalar deleting destructor` (0x4C8E10, `ret 4`).
#[hook(addr = 0x4C8E10, conv = thiscall, name = "CVehicleModelInfo::`scalar deleting destructor'", expect = "56 8B F1 E8 08 CB FF FF", this = "CVehicleModelInfo")]
pub fn scalar_deleting_destructor(mi: GPtr<u8>, flags: u8) -> GPtr<u8> {
    dtor(mi);
    if flags & 1 != 0 {
        free(mi);
    }
    mi
}

/// `CVehicleModelInfo::SetEnvironmentMapAtomicCB` (0x4C9410).
#[hook(addr = 0x4C9410, conv = cdecl, name = "CVehicleModelInfo::SetEnvironmentMapAtomicCB", expect = "8B 44 24 08 56")]
pub fn set_environment_map_atomic_cb(atomic: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    rp_geometry_for_all_materials(rdp(atomic, 0x18), lit::SET_ENVIRONMENT_MAP_CB, data);
    atomic
}

/// `CVehicleModelInfo::SetEnvMapCoeffAtomicCB` (0x4C9430).
#[hook(addr = 0x4C9430, conv = cdecl, name = "CVehicleModelInfo::SetEnvMapCoeffAtomicCB", expect = "8B 44 24 08 56")]
pub fn set_env_map_coeff_atomic_cb(atomic: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    rp_geometry_for_all_materials(rdp(atomic, 0x18), lit::SET_ENV_MAP_COEFF_CB, data);
    atomic
}

/// `CVehicleModelInfo::DisableEnvMap` (0x4C97E0).
#[hook(addr = 0x4C97E0, conv = thiscall, name = "CVehicleModelInfo::DisableEnvMap", expect = "8B 41 1C 85 C0", this = "CVehicleModelInfo")]
pub fn disable_env_map(mi: GPtr<u8>) {
    let clump = rdp(mi, 0x1C);
    if !clump.is_null() {
        rp_clump_for_all_atomics(clump, lit::SET_ENVIRONMENT_MAP_ATOMIC_CB, GPtr::from_addr(0xFFFF));
    }
}

/// CRT `floor` (0x8219F0) of a double: exact.
pub fn floor_f64(d: f64) -> f64 {
    if d.is_nan() || d.abs() >= 4503599627370496.0 {
        return d;
    }
    let t = d as i64 as f64;
    let t = if t > d { t - 1.0 } else { t };
    // keep the sign of a zero result as `floor` does (-0.0 stays -0.0; (-0.5) -> -1.0)
    if t == 0.0 && d.is_sign_negative() { -0.0 } else { t }
}

/// The integer of SetEnvMapCoeff: `_ftol(floor((double)(coeff * 1000.0f)))` (the product under
/// x87 PC=24, stored as a double for the CRT `floor`).
pub fn env_map_coeff_to_int(coeff: f32) -> i32 {
    let d = X87::from_f32(coeff).fmul_m(1000.0).to_f64();
    ftol(X87::from_f64_exact(floor_f64(d)))
}

/// `CVehicleModelInfo::SetEnvMapCoeff(float)` (0x4C9800, `ret 4`): `floor` + `_ftol` run before the
/// clump test.
#[hook(addr = 0x4C9800, conv = thiscall, name = "CVehicleModelInfo::SetEnvMapCoeff", expect = "D9 44 24 04 56", this = "CVehicleModelInfo")]
pub fn set_env_map_coeff(mi: GPtr<u8>, coeff: f32) {
    let v = env_map_coeff_to_int(coeff);
    let clump = rdp(mi, 0x1C);
    if !clump.is_null() {
        rp_clump_for_all_atomics(clump, lit::SET_ENV_MAP_COEFF_ATOMIC_CB, GPtr::from_addr(v as u32 as usize));
    }
}

/// `CVehicleModelInfo::Init` (0x4C7630): CClumpModelInfo::Init, then type = -1, wheel model = -1,
/// bike steer angle = 999.99 (0x4479FF5C).
#[hook(addr = 0x4C7630, conv = thiscall, name = "CVehicleModelInfo::Init", expect = "56 8B F1 E8 08 D8 FF FF", this = "CVehicleModelInfo")]
pub fn init(mi: GPtr<u8>) {
    clump_init(mi);
    wr32(mi, vmi::VEHICLE_TYPE, 0xFFFF_FFFF);
    wr16(mi, vmi::WHEEL_MODEL_INDEX, -1);
    wr32(mi, vmi::BIKE_STEER_ANGLE, 0x4479_FF5C);
}

/// CRT `operator delete` (0x8213AE).
mod del {
    use super::*;
    orig! {
        pub fn operator_delete(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    }
}

/// `CVehicleModelInfo::ConvertAnimFileIndex` (0x4C76D0): name → anim block index, the name is freed
/// with `operator delete` after the lookup.
#[hook(addr = 0x4C76D0, conv = thiscall, name = "CVehicleModelInfo::ConvertAnimFileIndex", expect = "56 8B F1 8B 86 04 03 00 00", this = "CVehicleModelInfo")]
pub fn convert_anim_file_index(mi: GPtr<u8>) {
    if rd32(mi, vmi::ANIM_BLOCK) as i32 == -1 {
        return;
    }
    let idx = get_animation_block_index(rdp(mi, vmi::ANIM_BLOCK));
    del::operator_delete(rdp(mi, vmi::ANIM_BLOCK));
    wr32(mi, vmi::ANIM_BLOCK, idx as u32);
}

/// "_vlo" / alpha-material / "windscreen" choice shared by the car and real-heli renderer CBs.
fn car_render_cb(atomic: GPtr<u8>, name: GPtr<u8>) -> u32 {
    if !strstr(name, s(lit::VLO)).is_null() {
        return lit::RENDER_VEHICLE_REALLY_LOW_DETAIL_CB;
    }
    let mut alpha = 0u8;
    rp_geometry_for_all_materials(rdp(atomic, 0x18), lit::HAS_ALPHA_MATERIAL_CB, GPtr::from_raw(&raw mut alpha));
    if alpha != 0 || strncmp(name, s(lit::WINDSCREEN), 10) == 0 {
        lit::RENDER_VEHICLE_HI_DETAIL_ALPHA_CB
    } else {
        lit::RENDER_VEHICLE_HI_DETAIL_CB
    }
}

/// `CVehicleModelInfo::SetAtomicRendererCB` (0x4C77E0).
#[hook(addr = 0x4C77E0, conv = cdecl, name = "CVehicleModelInfo::SetAtomicRendererCB", expect = "56 8B 74 24 08")]
pub fn set_atomic_renderer_cb(atomic: GPtr<u8>, _data: GPtr<u8>) -> GPtr<u8> {
    let name = get_frame_node_name(rdp(atomic, 4));
    let cb = car_render_cb(atomic, name);
    set_atomic_render_callback(atomic, cb);
    hide_damaged_atomic_call(atomic);
    atomic
}

orig! {
    /// CVehicleModelInfo::HideDamagedAtomicCB (0x4C7720), called with data 0
    fn hide_damaged_atomic_cb_call(atomic: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x4C7720 => hide_damaged_atomic_cb;
}
fn hide_damaged_atomic_call(atomic: GPtr<u8>) {
    hide_damaged_atomic_cb_call(atomic, GPtr::NULL);
}

/// `CVehicleModelInfo::SetAtomicRendererCB_RealHeli` (0x4C7870): "moving_rotor" / "moving_rotor2"
/// (exact, `repe cmpsb` over 13/14 bytes), else as SetAtomicRendererCB.
#[hook(addr = 0x4C7870, conv = cdecl, name = "CVehicleModelInfo::SetAtomicRendererCB_RealHeli", expect = "53 8B 5C 24 08")]
pub fn set_atomic_renderer_cb_real_heli(atomic: GPtr<u8>, _data: GPtr<u8>) -> GPtr<u8> {
    let name = get_frame_node_name(rdp(atomic, 4));
    let cb = if cmpsb_eq(name, b"moving_rotor\0") {
        lit::RENDER_HELI_ROTOR_ALPHA_CB
    } else if cmpsb_eq(name, b"moving_rotor2\0") {
        lit::RENDER_HELI_TAIL_ROTOR_ALPHA_CB
    } else {
        car_render_cb(atomic, name)
    };
    set_atomic_render_callback(atomic, cb);
    hide_damaged_atomic_call(atomic);
    atomic
}

/// The plane/train choice: "_vlo" → `vlo`, else alpha material → `alpha`, else `hi`.
fn big_render_cb(atomic: GPtr<u8>, vlo: u32, hi: u32, alpha_cb: u32) -> u32 {
    let name = get_frame_node_name(rdp(atomic, 4));
    if !strstr(name, s(lit::VLO)).is_null() {
        return vlo;
    }
    let mut alpha = 0u8;
    rp_geometry_for_all_materials(rdp(atomic, 0x18), lit::HAS_ALPHA_MATERIAL_CB, GPtr::from_raw(&raw mut alpha));
    if alpha != 0 { alpha_cb } else { hi }
}

/// `CVehicleModelInfo::SetAtomicRendererCB_Plane` (0x4C7930).
#[hook(addr = 0x4C7930, conv = cdecl, name = "CVehicleModelInfo::SetAtomicRendererCB_Plane", expect = "56 8B 74 24 08")]
pub fn set_atomic_renderer_cb_plane(atomic: GPtr<u8>, _data: GPtr<u8>) -> GPtr<u8> {
    let cb = big_render_cb(
        atomic,
        lit::RENDER_VEHICLE_REALLY_LOW_DETAIL_CB_BIG_VEHICLE,
        lit::RENDER_VEHICLE_HI_DETAIL_CB_BIG_VEHICLE,
        lit::RENDER_VEHICLE_HI_DETAIL_ALPHA_CB_BIG_VEHICLE,
    );
    set_atomic_render_callback(atomic, cb);
    hide_damaged_atomic_call(atomic);
    atomic
}

/// `CVehicleModelInfo::SetAtomicRendererCB_Boat` (0x4C79A0): the alpha test runs first; "boat_hi"
/// (exact, 8 bytes) → hi; else "_vlo" → lo; else alpha → alpha; else hi.
#[hook(addr = 0x4C79A0, conv = cdecl, name = "CVehicleModelInfo::SetAtomicRendererCB_Boat", expect = "53 8B 5C 24 08")]
pub fn set_atomic_renderer_cb_boat(atomic: GPtr<u8>, _data: GPtr<u8>) -> GPtr<u8> {
    let name = get_frame_node_name(rdp(atomic, 4));
    let mut alpha = 0u8;
    rp_geometry_for_all_materials(rdp(atomic, 0x18), lit::HAS_ALPHA_MATERIAL_CB, GPtr::from_raw(&raw mut alpha));
    let cb = if cmpsb_eq(name, b"boat_hi\0") {
        lit::RENDER_VEHICLE_HI_DETAIL_CB_BOAT
    } else if !strstr(name, s(lit::VLO)).is_null() {
        lit::RENDER_VEHICLE_LO_DETAIL_CB_BOAT
    } else if alpha != 0 {
        lit::RENDER_VEHICLE_HI_DETAIL_ALPHA_CB_BOAT
    } else {
        lit::RENDER_VEHICLE_HI_DETAIL_CB_BOAT
    };
    set_atomic_render_callback(atomic, cb);
    hide_damaged_atomic_call(atomic);
    atomic
}

/// `CVehicleModelInfo::SetAtomicRendererCB_Train` (0x4C7AA0).
#[hook(addr = 0x4C7AA0, conv = cdecl, name = "CVehicleModelInfo::SetAtomicRendererCB_Train", expect = "56 8B 74 24 08")]
pub fn set_atomic_renderer_cb_train(atomic: GPtr<u8>, _data: GPtr<u8>) -> GPtr<u8> {
    let cb = big_render_cb(
        atomic,
        lit::RENDER_VEHICLE_REALLY_LOW_DETAIL_CB_BIG_VEHICLE,
        lit::RENDER_TRAIN_HI_DETAIL_CB,
        lit::RENDER_TRAIN_HI_DETAIL_ALPHA_CB,
    );
    set_atomic_render_callback(atomic, cb);
    hide_damaged_atomic_call(atomic);
    atomic
}

/// `CVehicleModelInfo::GetWheelPosn(int32 wheel, CVector& out, bool local)` (0x4C7D20, `ret 0xC`).
/// Planes (type 4) with `local == false`: the frame matrix transformed by every parent
/// (RwMatrixTransform, combine 2 = postconcat); else the frame's modelling position.
#[hook(addr = 0x4C7D20, conv = thiscall, name = "CVehicleModelInfo::GetWheelPosn", expect = "8B 44 24 04 55", this = "CVehicleModelInfo")]
pub fn get_wheel_posn(mi: GPtr<u8>, wheel: i32, out: GPtr<u8>, local: u8) {
    let id = WHEEL_FRAME_IDS.ptr().cast::<i32>().wrapping_index(wheel).read();
    let frame = get_frame_from_id(rdp(mi, 0x1C), id);
    if rd32(mi, vmi::VEHICLE_TYPE) == 4 && local == 0 {
        let m = rw_matrix_create();
        for k in 0..16isize {
            wr32(m, 4 * k, rd32(frame, 0x10 + 4 * k));
        }
        let mut p = rdp(frame, 4);
        while !p.is_null() {
            rw_matrix_transform(m, field(p, 0x10), 2);
            p = rdp(p, 4);
        }
        wr32(out, 0, rd32(m, 0x30));
        wr32(out, 4, rd32(m, 0x34));
        wr32(out, 8, rd32(m, 0x38));
        rw_matrix_destroy(m);
        return;
    }
    wr32(out, 0, rd32(frame, 0x40));
    wr32(out, 4, rd32(frame, 0x44));
    wr32(out, 8, rd32(frame, 0x48));
}

/// `CVehicleModelInfo::GetOriginalCompPosition(CVector& out, int32 component)` (0x4C7DD0, `ret 8`).
#[hook(addr = 0x4C7DD0, conv = thiscall, name = "CVehicleModelInfo::GetOriginalCompPosition", expect = "8B 44 24 08 8B 49 1C", this = "CVehicleModelInfo")]
pub fn get_original_comp_position(mi: GPtr<u8>, out: GPtr<u8>, component: i32) -> bool {
    let frame = get_frame_from_id(rdp(mi, 0x1C), component);
    if frame.is_null() {
        return false;
    }
    wr32(out, 0, rd32(frame, 0x40));
    wr32(out, 4, rd32(frame, 0x44));
    wr32(out, 8, rd32(frame, 0x48));
    true
}

/// `_ftol((rand() & 0xFFFF) * (1/32768.f) * 3.0f)` as ChooseComponent computes it inline (x87 PC=24).
pub fn rand_0_to_3(r: i32) -> i32 {
    let v = X87::from_i32(r & 0xFFFF).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(3.0);
    ftol(v)
}

orig! {
    fn get_list_of_components_call(comps: u32, num: i32, out: GPtr<i32>) -> i32 = cdecl 0x4C7E50 => get_list_of_components_not_used_by_rules;
    fn choose_component_global_call(rule: i32, comps: i32) -> i32 = cdecl 0x4C7FB0 => choose_component_global;
    fn is_valid_comp_rule_call(rule: i32) -> bool = cdecl 0x4C7E10 => is_valid_comp_rule;
}

/// `CVehicleModelInfo::ChooseComponent` (0x4C8040). gr-deviations `VehicleModelInfo.cpp:344/348`
/// (keep-original): the inline `_ftol(rand16 / 32768 * 3) < 2`, then `GetRandomNumberInRange`.
#[hook(addr = 0x4C8040, conv = thiscall, name = "CVehicleModelInfo::ChooseComponent", expect = "A0 58 64 8A 00", this = "CVehicleModelInfo")]
pub fn choose_component(mi: GPtr<u8>) -> i32 {
    let c = COMPS_TO_USE.ptr().cast::<i8>();
    if c.read() != -2 {
        let v = c.read() as i32;
        c.write(-2);
        return v;
    }
    let comps = rd32(mi, vmi::EXTRA_COMPS);
    let rule = comps >> 12 & 0xF;
    if rule != 0 && rule_ok(rule, OLD_WEATHER_TYPE.get(), NEW_WEATHER_TYPE.get()) {
        return choose_component_global_call(rule as i32, (comps & 0xFFF) as i32);
    }
    let r = sa_core::orig::at_site(0x4C80BC, sa_core::crt::rand);
    if rand_0_to_3(r) < 2 {
        let st = rdp(mi, vmi::VEHICLE_STRUCT);
        let mut list = [0i32; 6];
        let n = get_list_of_components_call(comps_reload(mi), rd8(st, vs::NUM_EXTRAS) as i8 as i32, GPtr::from_raw(list.as_mut_ptr()));
        if n != 0 {
            let k = get_random_number_in_range(0, n);
            return list[k as usize];
        }
    }
    -1
}

fn comps_reload(mi: GPtr<u8>) -> u32 {
    rd32(mi, vmi::EXTRA_COMPS)
}

/// `CVehicleModelInfo::ChooseSecondComponent` (0x4C8120). gr-deviations `VehicleModelInfo.cpp:363/367`.
#[hook(addr = 0x4C8120, conv = thiscall, name = "CVehicleModelInfo::ChooseSecondComponent", expect = "A0 59 64 8A 00", this = "CVehicleModelInfo")]
pub fn choose_second_component(mi: GPtr<u8>) -> i32 {
    let c = COMPS_TO_USE.ptr().cast::<i8>().wrapping_index(1);
    if c.read() != -2 {
        let v = c.read() as i32;
        c.write(-2);
        return v;
    }
    let comps = rd32(mi, vmi::EXTRA_COMPS);
    let rule_b = comps >> 28;
    if rule_b != 0 && rule_ok(rule_b, OLD_WEATHER_TYPE.get(), NEW_WEATHER_TYPE.get()) {
        return choose_component_global_call(rule_b as i32, (comps >> 16 & 0xFFF) as i32);
    }
    let rule_a = comps >> 12 & 0xF;
    if rule_a != 0 && is_valid_comp_rule_call(rule_a as i32) && get_random_number_in_range(0, 3) < 2 {
        let st = rdp(mi, vmi::VEHICLE_STRUCT);
        let mut list = [0i32; 6];
        let n = get_list_of_components_call(rd32(mi, vmi::EXTRA_COMPS), rd8(st, vs::NUM_EXTRAS) as i8 as i32, GPtr::from_raw(list.as_mut_ptr()));
        if n != 0 {
            let k = get_random_number_in_range(0, n);
            return list[k as usize];
        }
    }
    -1
}

/// `CVehicleModelInfo::LoadEnvironmentMaps` (0x4C8780): "white" texture from the "particle" TXD
/// (filter byte +0x50 = 2, +0x10 name byte = '@'); tail-jumps into PopCurrentTxd.
#[hook(addr = 0x4C8780, conv = cdecl, name = "CVehicleModelInfo::LoadEnvironmentMaps", expect = "56 68 48 C6 85 00")]
pub fn load_environment_maps() {
    let slot = find_txd_slot(s(lit::PARTICLE));
    push_current_txd();
    set_current_txd(slot);
    if WHITE_TEXTURE.get().is_null() {
        WHITE_TEXTURE.set(rw_texture_read(s(lit::WHITE), GPtr::NULL));
        wr8(WHITE_TEXTURE.get(), 0x10, 0x40);
        wr8(WHITE_TEXTURE.get(), 0x50, 2);
    }
    pop_current_txd();
}

/// `CVehicleModelInfo::StoreAtomicUsedMaterialsCB` (0x4C8B60): appends every mesh material of
/// atomics without id bit 0x2000 (the mesh header is re-read per mesh).
#[hook(addr = 0x4C8B60, conv = cdecl, name = "CVehicleModelInfo::StoreAtomicUsedMaterialsCB", expect = "55 56 8B 74 24 0C")]
pub fn store_atomic_used_materials_cb(atomic: GPtr<u8>, list: GPtr<u8>) -> GPtr<u8> {
    let geo = rdp(atomic, 0x18);
    let mesh0 = rdp(geo, 0x54).byte_add(0x10);
    if get_atomic_id(atomic) & 0x2000 != 0 {
        return atomic;
    }
    let mut k = 0isize;
    while (k as i32) < rd16(rdp(geo, 0x54), 4) as u16 as i32 {
        let mat = rdp(mesh0, 8 + 0xC * k);
        rp_material_list_append_material(list, mat);
        k += 1;
    }
    atomic
}

/// `CVehicleModelInfo::CollapseFramesCB` (0x4C8E30): recurse, move the objects to `data`, destroy.
#[hook(addr = 0x4C8E30, conv = cdecl, name = "CVehicleModelInfo::CollapseFramesCB", expect = "56 8B 74 24 08")]
pub fn collapse_frames_cb(frame: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    rw_frame_for_all_children(frame, lit::COLLAPSE_FRAMES_CB, data);
    rw_frame_for_all_objects(frame, lit::MOVE_OBJECTS_CB, data);
    rw_frame_destroy(frame);
    frame
}

/// `CVehicleModelInfo::AssignRemapTxd(name, txdSlot)` (0x4C9360): "<model><digits>" → the vehicle
/// model (ids 400..630) gets the remap. The name prefix is copied with `strncpy` into a 24-byte
/// stack buffer (no bound); a name of only digits copies an empty prefix.
#[hook(addr = 0x4C9360, conv = cdecl, name = "CVehicleModelInfo::AssignRemapTxd", expect = "83 EC 18 56 57")]
pub fn assign_remap_txd(name: GPtr<u8>, slot: i16) {
    let mut len = 0isize;
    while rd8(name, len) != 0 {
        len += 1;
    }
    if isdigit(rd8(name, len - 1) as i32) == 0 {
        return;
    }
    let mut i = len - 2;
    if isdigit(rd8(name, i) as i32) != 0 {
        loop {
            i -= 1;
            if isdigit(rd8(name, i) as i32) == 0 {
                break;
            }
        }
    }
    i += 1;
    let mut buf = [0u8; 64];
    let b = GPtr::from_raw(buf.as_mut_ptr());
    strncpy(b, name, i as u32);
    wr8(b, i, 0);
    let mi = get_model_info_min_max(b, 400, 0x276);
    if !mi.is_null() && model_type_of(mi) == 6 {
        add_remap_call(mi, slot);
    }
}

orig! {
    /// `CBaseModelInfo::GetModelType` (vtable slot +0x10)
    fn model_type_vcall(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
    fn add_remap_call(mi: GPtr<u8>, txd: i16) = thiscall 0x4C86D0 => add_remap;
}
fn model_type_of(mi: GPtr<u8>) -> u8 {
    model_type_vcall(mi)
}

/// `CVehicleModelInfo::SetCarCustomPlate` (0x4C9450): plate material/text cleared, type 0xFF, then a
/// generated plate ("DEFAULT" seed) set up on the clump.
#[hook(addr = 0x4C9450, conv = thiscall, name = "CVehicleModelInfo::SetCarCustomPlate", expect = "83 EC 10 56 8B F1", this = "CVehicleModelInfo")]
pub fn set_car_custom_plate(mi: GPtr<u8>) {
    wr32(mi, vmi::PLATE_MATERIAL, 0);
    wr8(mi, vmi::PLATE_TEXT, 0);
    wr8(mi, vmi::PLATE_TYPE, 0xFF);
    let mut buf = [0u8; 9];
    let d = s(lit::DEFAULT);
    for (k, b) in buf.iter_mut().enumerate().take(8) {
        *b = rd8(d, k as isize);
    }
    buf[8] = 0;
    let p = GPtr::from_raw(buf.as_mut_ptr());
    generate_plate_text(p, 8);
    let m = plate_setup_clump(rdp(mi, 0x1C), p, rd8(mi, vmi::PLATE_TYPE));
    if !m.is_null() {
        wrp(mi, vmi::PLATE_MATERIAL, m);
    }
}

/// `CVehicleModelInfo::ShutdownLightTexture` (0x4C7470).
#[hook(addr = 0x4C7470, conv = cdecl, name = "CVehicleModelInfo::ShutdownLightTexture", expect = "A1 8C E6 B4 00")]
pub fn shutdown_light_texture() {
    if !LIGHTS_TEXTURE.get().is_null() {
        rw_texture_destroy(LIGHTS_TEXTURE.get());
        LIGHTS_TEXTURE.set(GPtr::NULL);
    }
    if !LIGHTS_ON_TEXTURE.get().is_null() {
        rw_texture_destroy(LIGHTS_ON_TEXTURE.get());
        LIGHTS_ON_TEXTURE.set(GPtr::NULL);
    }
}

/// `CVehicleModelInfo::MoveObjectsCB` (0x4C7700).
#[hook(addr = 0x4C7700, conv = cdecl, name = "CVehicleModelInfo::MoveObjectsCB", expect = "8B 44 24 08 56")]
pub fn move_objects_cb(object: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    rp_atomic_set_frame(object, data);
    object
}

/// `CVehicleModelInfo::ShutdownEnvironmentMaps` (0x4C87D0): byte 0xB4E6BC = 0 first, then the white
/// texture is destroyed without a null test, then the env-map frame.
#[hook(addr = 0x4C87D0, conv = cdecl, name = "CVehicleModelInfo::ShutdownEnvironmentMaps", expect = "A1 EC E3 B4 00")]
pub fn shutdown_environment_maps() {
    ENV_MAP_UNK_B4E6BC.set(0);
    rw_texture_destroy(WHITE_TEXTURE.get());
    WHITE_TEXTURE.set(GPtr::NULL);
    let f = CAR_FRAME.get();
    if !f.is_null() {
        rw_frame_destroy(f);
    }
    CAR_FRAME.set(GPtr::NULL);
}

/// `CVehicleModelInfo::SetRenderPipelines` (0x4C8900): car pipe setup, and once the env-map frame
/// rotated 60 degrees around X.
#[hook(addr = 0x4C8900, conv = thiscall, name = "CVehicleModelInfo::SetRenderPipelines", expect = "8B 41 1C 83 EC 0C", this = "CVehicleModelInfo")]
pub fn set_render_pipelines(mi: GPtr<u8>) {
    custom_car_pipe_clump_setup(rdp(mi, 0x1C));
    if CAR_FRAME.get().is_null() {
        let mut axis = [1.0f32, 0.0, 0.0];
        let f = rw_frame_create();
        CAR_FRAME.set(f);
        rw_matrix_rotate(f.byte_add(0x10), GPtr::from_raw(axis.as_mut_ptr()).cast(), 60.0, 0);
        rw_frame_update_objects(CAR_FRAME.get());
        rw_frame_get_ltm(CAR_FRAME.get());
    }
}

/// `CVehicleModelInfo::SetDirtTextures(CVehicleModelInfo*, int32 level)` (0x5D5DB0): the first 16
/// dirt materials get `CCarFXRenderer::ms_aDirtTextures[level]` (0xC02BD0).
#[hook(addr = 0x5D5DB0, conv = cdecl, name = "CVehicleModelInfo::SetDirtTextures", expect = "53 8B 5C 24 0C")]
pub fn set_dirt_textures(mi: GPtr<u8>, level: i32) {
    for k in 0..16isize {
        let m = rdp(mi, vmi::DIRT_MATERIALS + 4 * k);
        if !m.is_null() {
            let t = GPtr::<GPtr<u8>>::from_addr(0xC02BD0).wrapping_index(level).read();
            rp_material_set_texture(m, t);
        }
    }
}

/// `CVehicleModelInfo::SetEnvironmentMapCB` (0x4C8840): data 0xFFFF → effects off; env-map
/// materials get the car frame, the white texture when untextured, coefficient 0.25.
#[hook(addr = 0x4C8840, conv = cdecl, name = "CVehicleModelInfo::SetEnvironmentMapCB", expect = "81 7C 24 08 FF FF 00 00")]
pub fn set_environment_map_cb(mat: GPtr<u8>, data: u32) -> GPtr<u8> {
    if data == 0xFFFF {
        rp_mat_fx_material_set_effects(mat, 0);
        return mat;
    }
    if rp_mat_fx_material_get_effects(mat) == 2 {
        rp_mat_fx_material_set_env_map_frame(mat, CAR_FRAME.get());
        if rdp(mat, 0).is_null() {
            rp_material_set_texture(mat, WHITE_TEXTURE.get());
        }
        rp_mat_fx_material_set_env_map_coefficient(mat, ENV_MAP_COEFF.get());
    }
    mat
}

/// `CVehicleModelInfo::ReduceMaterialsInVehicle` (0x4C8BD0): collects the used materials of the
/// clump and the extras into a scratch-pad material list {Malloc(0x50), 0, 0x14}, timed with
/// GetCurrentTimeInCycles (results unused), then clears atomic flag 0x2000 on the clump.
#[hook(addr = 0x4C8BD0, conv = thiscall, name = "CVehicleModelInfo::ReduceMaterialsInVehicle", expect = "83 EC 0C 55 56", this = "CVehicleModelInfo")]
pub fn reduce_materials_in_vehicle(mi: GPtr<u8>) {
    let clump = rdp(mi, 0x1C);
    lock_scratch_pad();
    init_scratch_pad();
    // RpMaterialList { materials, numMaterials, space }
    let mut list = MaterialList { materials: GPtr::NULL, count: 0, space: 0x14 };
    list.materials = malloc(0x50, 0);
    let lp = GPtr::from_raw(&raw mut list).cast::<u8>();
    get_current_time_in_cycles();
    rp_clump_for_all_atomics(clump, lit::STORE_ATOMIC_USED_MATERIALS_CB, lp);
    let mut k = 0;
    loop {
        let st = rdp(mi, vmi::VEHICLE_STRUCT);
        if k >= rd8(st, vs::NUM_EXTRAS) as i8 as i32 {
            break;
        }
        store_atomic_used_materials_call(rdp(st, vs::EXTRAS + 4 * k as isize), lp);
        k += 1;
    }
    get_current_time_in_cycles();
    get_cycles_per_millisecond();
    rp_material_list_deinitialize(lp);
    release_scratch_pad();
    clear_clump_for_all_atomics_flag(clump, 0x2000);
}

orig! {
    fn store_atomic_used_materials_call(atomic: GPtr<u8>, list: GPtr<u8>) -> GPtr<u8> = cdecl 0x4C8B60 => store_atomic_used_materials_cb;
    fn set_vehicle_component_flags_call(mi: GPtr<u8>, frame: GPtr<u8>, flags: u32) = thiscall 0x4C7C10 => set_vehicle_component_flags;
    fn set_atomic_render_callbacks_call(mi: GPtr<u8>) = thiscall 0x4C7B10 => set_atomic_render_callbacks;
    fn set_render_pipelines_call(mi: GPtr<u8>) = thiscall 0x4C8900 => set_render_pipelines;
    fn preprocess_hierarchy_call(mi: GPtr<u8>) = thiscall 0x4C8E60 => preprocess_hierarchy;
    fn reduce_materials_call(mi: GPtr<u8>) = thiscall 0x4C8BD0 => reduce_materials_in_vehicle;
    fn set_car_custom_plate_call(mi: GPtr<u8>) = thiscall 0x4C9450 => set_car_custom_plate;
    fn choose_component_call(mi: GPtr<u8>) -> i32 = thiscall 0x4C8040 => choose_component;
    fn choose_second_component_call(mi: GPtr<u8>) -> i32 = thiscall 0x4C8120 => choose_second_component;
    /// CPool<CVehicleStructure>::New (0x4C94C0)
    fn vehicle_structure_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C94C0 => unported;
    /// CVehicleStructure::~CVehicleStructure (0x4C7410)
    fn vehicle_structure_dtor(st: GPtr<u8>) = thiscall 0x4C7410 => unported;
    /// RwFrameTransform (0x7F0F70)
    fn rw_frame_transform(frame: GPtr<u8>, m: GPtr<u8>, op: u32) -> GPtr<u8> = cdecl 0x7F0F70 => unported;
    /// RwHelper GetFirstChild (0x734900)
    fn get_first_child(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x734900 => unported;
}

/// `CVehicleStructure::m_pInfoPool` (0xB4E680), foreign (owner: misc, CPool<CVehicleStructure>).
fn vehicle_structure_pool() -> GPtr<u8> {
    GPtr::<GPtr<u8>>::from_addr(0xB4E680).read()
}

/// `RwObjectNameIdAssocation` entry of a vehicle description (12 bytes: name, hier id, flags).
fn desc(table: GPtr<u8>, k: isize) -> (GPtr<u8>, i32, u32) {
    (rdp(table, 12 * k), rd32(table, 12 * k + 4) as i32, rd32(table, 12 * k + 8))
}

/// `CVehicleModelInfo::PreprocessHierarchy` (0x4C8E60). Two passes over
/// `ms_vehicleDescs[type]` (ported from the bytes; see the module notes):
/// 1. entries with flags 0x20208: dummies (8: position through the ancestors, frame destroyed),
///    upgrades (0x20000: position, rotation and parent hierarchy id), extras (atomic detached and
///    stored); entries with 0x110000: the first atomic below the frame (0x10000 → wheel, else the
///    "other" wheel atomic);
/// 2. other entries: door count (0x10), ok/damaged pairs (2), component flags, wheel clones.
#[hook(addr = 0x4C8E60, conv = thiscall, name = "CVehicleModelInfo::PreprocessHierarchy", expect = "55 8B EC 83 E4 F8", this = "CVehicleModelInfo")]
pub fn preprocess_hierarchy(mi: GPtr<u8>) {
    let hbase = 0xC2B9DC + rd8(mi, vmi::HANDLING_ID) as usize * 0xE0;
    let table_slot = VEHICLE_DESCS.ptr().cast::<u32>().wrapping_index(rd32(mi, vmi::VEHICLE_TYPE) as i32);
    let table = || GPtr::<u8>::from_addr(table_slot.read() as usize);
    wr8(mi, vmi::NUM_DOORS, 0);
    let mut wheel: GPtr<u8> = GPtr::NULL;
    let mut other: GPtr<u8> = GPtr::NULL;
    let clump_frame = || rdp(rdp(mi, 0x1C), 4);
    if !rdp(table(), 0).is_null() {
        let mut k = 0isize;
        loop {
            let (name, _, flags) = desc(table(), k);
            if flags & 0x20208 != 0 {
                let mut search: [GPtr<u8>; 2] = [name, GPtr::NULL];
                rw_frame_for_all_children(clump_frame(), lit::FIND_FRAME_FROM_NAME_WITHOUT_ID_CB, GPtr::from_raw(search.as_mut_ptr()).cast());
                let frame = search[1];
                if !frame.is_null() {
                    let (_, id, flags) = desc(table(), k);
                    if flags & 8 != 0 {
                        let (x, y, z) = (rd32(frame, 0x40), rd32(frame, 0x44), rd32(frame, 0x48));
                        let st = rdp(mi, vmi::VEHICLE_STRUCT);
                        let d = st.byte_add(12 * id as isize);
                        wr32(d, 0, x);
                        wr32(d, 4, y);
                        wr32(d, 8, z);
                        let mut p = rdp(frame, 4);
                        while !p.is_null() && !rdp(p, 4).is_null() {
                            let id = desc(table(), k).1;
                            let d = rdp(mi, vmi::VEHICLE_STRUCT).byte_add(12 * id as isize);
                            rw_v3d_transform_points(d, d, 1, p.byte_add(0x10));
                            p = rdp(p, 4);
                        }
                        rw_frame_destroy(frame);
                    } else if flags & 0x20000 != 0 {
                        let hier = get_frame_hierarchy_id(rdp(frame, 4));
                        let (x, y, z) = (rd32(frame, 0x40), rd32(frame, 0x44), rd32(frame, 0x48));
                        let up = rdp(mi, vmi::VEHICLE_STRUCT).byte_add(vs::UPGRADES + 0x20 * id as isize);
                        wr32(up, 0, x);
                        wr32(up, 4, y);
                        wr32(up, 8, z);
                        quaternion_set(rdp(mi, vmi::VEHICLE_STRUCT).byte_add(0xC0 + 0x20 * id as isize), frame.byte_add(0x10));
                        wr32(rdp(mi, vmi::VEHICLE_STRUCT), vs::UPGRADE_PARENT + 0x20 * id as isize, hier as u32);
                    } else {
                        let atomic = get_first_object(frame);
                        rp_clump_remove_atomic(rdp(mi, 0x1C), atomic);
                        rw_frame_remove_child(frame);
                        set_vehicle_component_flags_call(mi, frame, desc(table(), k).2);
                        let st = rdp(mi, vmi::VEHICLE_STRUCT);
                        let n = rd8(st, vs::NUM_EXTRAS) as i8 as isize;
                        wrp(st, vs::EXTRAS + 4 * n, atomic);
                        let st = rdp(mi, vmi::VEHICLE_STRUCT);
                        wr8(st, vs::NUM_EXTRAS, rd8(st, vs::NUM_EXTRAS).wrapping_add(1));
                    }
                }
            }
            let (_, id, flags) = desc(table(), k);
            if flags & 0x11_0000 != 0 {
                let mut search: [GPtr<u8>; 2] = [GPtr::from_addr(id as u32 as usize), GPtr::NULL];
                rw_frame_for_all_children(clump_frame(), lit::FIND_FRAME_FROM_ID_CB, GPtr::from_raw(search.as_mut_ptr()).cast());
                let mut f = search[1];
                while !f.is_null() {
                    if !get_first_object(f).is_null() {
                        if desc(table(), k).2 & 0x1_0000 != 0 {
                            wheel = get_first_object(f);
                        } else {
                            other = get_first_object(f);
                        }
                        break;
                    }
                    f = get_first_child(f);
                }
            }
            k += 1;
            if rdp(table(), 12 * k).is_null() {
                break;
            }
        }
    }
    if rdp(table(), 0).is_null() {
        return;
    }
    let mut k = 0isize;
    loop {
        let (_, id, flags) = desc(table(), k);
        if flags & 0x20208 == 0 {
            let mut search: [GPtr<u8>; 2] = [GPtr::from_addr(id as u32 as usize), GPtr::NULL];
            rw_frame_for_all_children(clump_frame(), lit::FIND_FRAME_FROM_ID_CB, GPtr::from_raw(search.as_mut_ptr()).cast());
            let frame = search[1];
            if !frame.is_null() {
                if desc(table(), k).2 & 0x10 != 0 {
                    wr8(mi, vmi::NUM_DOORS, rd8(mi, vmi::NUM_DOORS).wrapping_add(1));
                }
                if desc(table(), k).2 & 2 != 0 {
                    let mut pair: [GPtr<u8>; 2] = [GPtr::NULL, GPtr::NULL];
                    rw_frame_for_all_children(frame, lit::COLLAPSE_FRAMES_CB, frame);
                    rw_frame_update_objects(frame);
                    rw_frame_for_all_objects(frame, lit::GET_OK_AND_DAMAGED_ATOMIC_CB, GPtr::from_raw(pair.as_mut_ptr()).cast());
                    let (ok, dam) = (pair[0], pair[1]);
                    if !ok.is_null() && !dam.is_null() {
                        wr32(dam, 0x48, rd32(ok, 0x48));
                        if rd32(dam, 0x48) == 0 {
                            wr32(dam, 0x48, lit::ATOMIC_DEFAULT_RENDER_CB);
                        }
                        let hid = desc(table(), k).1 as u32;
                        let st = rdp(mi, vmi::VEHICLE_STRUCT);
                        wr32(st, vs::MASK_DAMAGEABLE, rd32(st, vs::MASK_DAMAGEABLE) | 1u32.wrapping_shl(hid & 31));
                    }
                }
                set_vehicle_component_flags_call(mi, frame, desc(table(), k).2);
                let flags = desc(table(), k).2;
                if flags & 0x1_0004 == 0 {
                    if flags & 0x20_0000 != 0 && !other.is_null() {
                        let c = rp_atomic_clone(other);
                        rp_atomic_set_frame(c, frame);
                        rp_clump_add_atomic(rdp(mi, 0x1C), c);
                        set_atomic_render_callback(c, lit::RENDER_WHEEL_ATOMIC_CB);
                    }
                } else if !wheel.is_null() {
                    if flags & 0x1_0000 == 0 {
                        let c = rp_atomic_clone(wheel);
                        rp_atomic_set_frame(c, frame);
                        rp_clump_add_atomic(rdp(mi, 0x1C), c);
                        set_atomic_render_callback(c, lit::RENDER_WHEEL_ATOMIC_CB);
                        let id = desc(table(), k).1;
                        let hflags = GPtr::<u32>::from_addr(hbase + 0xCC).read();
                        if id != 2 && id != 5 && hflags & 0x2000_0000 != 0 {
                            let c = rp_atomic_clone(wheel);
                            let f = rw_frame_create();
                            rp_atomic_set_frame(c, f);
                            rw_frame_add_child(frame, f);
                            let x = (WHEEL_SCALE.get() * f32::from_bits(0xBE80_0000)).to_bits();
                            let fl = rd32(f, 0x1C);
                            wr32(f, 0x38, 0x3F80_0000);
                            wr32(f, 0x24, 0x3F80_0000);
                            wr32(f, 0x40, x);
                            wr32(f, 0x10, 0x3F80_0000);
                            wr32(f, 0x20, 0);
                            wr32(f, 0x18, 0);
                            wr32(f, 0x14, 0);
                            wr32(f, 0x34, 0);
                            wr32(f, 0x30, 0);
                            wr32(f, 0x28, 0);
                            wr32(f, 0x48, 0);
                            wr32(f, 0x44, 0);
                            wr32(f, 0x1C, fl | 0x2_0003);
                            rp_clump_add_atomic(rdp(mi, 0x1C), c);
                            set_atomic_render_callback(c, lit::RENDER_WHEEL_ATOMIC_CB);
                        }
                    } else {
                        rw_frame_for_all_children(frame, lit::COLLAPSE_FRAMES_CB, frame);
                        rw_frame_update_objects(frame);
                        set_atomic_render_callback(wheel, lit::RENDER_WHEEL_ATOMIC_CB);
                    }
                }
            }
        }
        k += 1;
        if rdp(table(), 12 * k).is_null() {
            break;
        }
    }
}

/// `CVehicleModelInfo::SetClump(RpClump*)` (0x4C95C0, `ret 4`). gr-deviations
/// `VehicleModelInfo.cpp:242` (keep-original): no dirt-material pass.
#[hook(addr = 0x4C95C0, conv = thiscall, name = "CVehicleModelInfo::SetClump", expect = "6A FF 68 6B BC 83 00", this = "CVehicleModelInfo")]
pub fn set_clump(mi: GPtr<u8>, clump: GPtr<u8>) {
    let p = vehicle_structure_pool_new(vehicle_structure_pool());
    let st = if p.is_null() { GPtr::NULL } else { vehicle_structure_ctor(p) };
    wrp(mi, vmi::VEHICLE_STRUCT, st);
    clump_set_clump(mi, clump);
    set_atomic_render_callbacks_call(mi);
    let ids = VEHICLE_DESCS.ptr().cast::<u32>().wrapping_index(rd32(mi, vmi::VEHICLE_TYPE) as i32).read();
    clump_set_frame_ids(mi, ids);
    set_render_pipelines_call(mi);
    preprocess_hierarchy_call(mi);
    reduce_materials_call(mi);
    for k in 0..4 {
        wr8(mi, vmi::CURRENT_COLORS + k, 0xFF);
    }
    set_car_custom_plate_call(mi);
}

/// Clones extra `comp` onto the clump under `parent` (CreateInstance).
fn attach_extra(mi: GPtr<u8>, clump: GPtr<u8>, parent: GPtr<u8>, comp: i32) {
    let st = rdp(mi, vmi::VEHICLE_STRUCT);
    let extra = rdp(st, vs::EXTRAS + 4 * comp as isize);
    if extra.is_null() {
        return;
    }
    let c = rp_atomic_clone(extra);
    let f = rw_frame_create();
    let ex = rdp(rdp(mi, vmi::VEHICLE_STRUCT), vs::EXTRAS + 4 * comp as isize);
    rw_frame_transform(f, rdp(ex, 4).byte_add(0x10), 0);
    rp_atomic_set_frame(c, f);
    rp_clump_add_atomic(clump, c);
    rw_frame_add_child(parent, f);
}

/// `CVehicleModelInfo::CreateInstance` (0x4C9680): the clump plus up to two chosen extras
/// (`ms_compsUsed`), then env maps off-flagged with data 0.
#[hook(addr = 0x4C9680, conv = thiscall, name = "CVehicleModelInfo::CreateInstance", expect = "83 EC 08 56 57", this = "CVehicleModelInfo")]
pub fn create_instance(mi: GPtr<u8>) -> GPtr<u8> {
    let clump = clump_create_instance(mi);
    let used = COMPS_USED.ptr().cast::<u8>();
    if rd8(rdp(mi, vmi::VEHICLE_STRUCT), vs::NUM_EXTRAS) == 0 {
        used.write(0xFF);
        used.wrapping_index(1).write(0xFF);
    } else {
        let ty = rd32(mi, vmi::VEHICLE_TYPE) as i32;
        let mut parent = GPtr::NULL;
        if ty == 9 || ty == 10 || ty < 5 {
            parent = get_frame_from_id(clump, 1);
        }
        if parent.is_null() {
            parent = rdp(clump, 4);
        }
        let c1 = choose_component_call(mi);
        if c1 != -1 {
            attach_extra(mi, clump, parent, c1);
        }
        used.write(c1 as u8);
        let c2 = choose_second_component_call(mi);
        if c2 != -1 {
            attach_extra(mi, clump, parent, c2);
        }
        used.wrapping_index(1).write(c2 as u8);
    }
    rp_clump_for_all_atomics(clump, lit::SET_ENVIRONMENT_MAP_ATOMIC_CB, GPtr::NULL);
    clump
}

/// `CVehicleModelInfo::DeleteRwObject` (0x4C9890): the vehicle structure is destroyed and returned
/// to its pool (inline CPool delete: map |= 0x80, first-free lowered), then
/// CClumpModelInfo::DeleteRwObject (tail jump).
#[hook(addr = 0x4C9890, conv = thiscall, name = "CVehicleModelInfo::DeleteRwObject", expect = "56 57 8B F9 8B 77 5C", this = "CVehicleModelInfo")]
pub fn delete_rw_object(mi: GPtr<u8>) {
    let st = rdp(mi, vmi::VEHICLE_STRUCT);
    if !st.is_null() {
        vehicle_structure_dtor(st);
        let pool = vehicle_structure_pool().cast::<sa_core::pool::CPool<sa_core::pools::Slot<0x314>>>();
        sa_core::pool::pool_delete(pool, st.cast());
        wrp(mi, vmi::VEHICLE_STRUCT, GPtr::NULL);
    }
    clump_delete_rw_object(mi);
}

orig! {
    fn load_vehicle_colours_call() = cdecl 0x5B6890 => load_vehicle_colours;
    fn load_vehicle_upgrades_call() = cdecl 0x5B65A0 => load_vehicle_upgrades;
    fn load_environment_maps_call() = cdecl 0x4C8780 => load_environment_maps;
    /// CLoadingScreen::NewChunkLoaded (0x590D00)
    fn new_chunk_loaded() = cdecl 0x590D00 => unported;
    /// CTxdStore::AddTxdSlot (0x731C80)
    fn add_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731C80 => unported;
    /// CTxdStore::LoadTxd(slot, filename) (0x7320B0)
    fn load_txd(slot: i32, name: GPtr<u8>) -> u8 = cdecl 0x7320B0 => unported;
    /// CTxdStore::AddRef (0x731A00)
    fn txd_add_ref(slot: i32) = cdecl 0x731A00 => unported;
    /// CPool<CVehicleStructure>::CPool(size, name) (0x5B8340)
    fn vehicle_structure_pool_ctor(pool: GPtr<u8>, size: i32, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x5B8340 => unported;
    /// CCarFXRenderer::InitialiseDirtTexture (0x5D5BC0)
    fn initialise_dirt_texture() = cdecl 0x5D5BC0 => unported;
    /// CAtomicModelInfo::SetupVehicleUpgradeFlags (0x4C4570)
    fn setup_vehicle_upgrade_flags(mi: GPtr<u8>, name: GPtr<u8>) = thiscall 0x4C4570 => unported;
    fn add_wheel_upgrade_call(set: i32, model: i32) = cdecl 0x4C8700 => add_wheel_upgrade_i32;
    fn add_upgrade_link_call(list: GPtr<u8>, u1: i32, u2: i32) = thiscall 0x4C74B0 => add_upgrade_link_i32;
    /// CRT `sscanf` (0x8220AD), up to 34 targets
    fn sscanf(s: GPtr<u8>, fmt: GPtr<u8>, a0: GPtr<u8>, a1: GPtr<u8>, a2: GPtr<u8>, a3: GPtr<u8>, a4: GPtr<u8>, a5: GPtr<u8>, a6: GPtr<u8>, a7: GPtr<u8>, a8: GPtr<u8>, a9: GPtr<u8>, a10: GPtr<u8>, a11: GPtr<u8>, a12: GPtr<u8>, a13: GPtr<u8>, a14: GPtr<u8>, a15: GPtr<u8>, a16: GPtr<u8>, a17: GPtr<u8>, a18: GPtr<u8>, a19: GPtr<u8>, a20: GPtr<u8>, a21: GPtr<u8>, a22: GPtr<u8>, a23: GPtr<u8>, a24: GPtr<u8>, a25: GPtr<u8>, a26: GPtr<u8>, a27: GPtr<u8>, a28: GPtr<u8>, a29: GPtr<u8>, a30: GPtr<u8>, a31: GPtr<u8>, a32: GPtr<u8>, a33: GPtr<u8>, a34: GPtr<u8>, a35: GPtr<u8>, a36: GPtr<u8>, a37: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CFileMgr::OpenFile (0x538900)
    fn open_file(name: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
    /// CFileMgr::CloseFile (0x5389D0)
    fn close_file(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
}

// phase B/C target of an `orig!` declaration (unused in phase A)
#[cfg_attr(sa_phase_a, allow(dead_code))]
fn add_wheel_upgrade_i32(set: i32, model: i32) {
    add_wheel_upgrade(set, model as i16)
}
// phase B/C target of an `orig!` declaration (unused in phase A)
#[cfg_attr(sa_phase_a, allow(dead_code))]
fn add_upgrade_link_i32(list: GPtr<u8>, u1: i32, u2: i32) {
    add_upgrade_link(list, u1 as i16, u2 as i16)
}

/// `sscanf` with a list of targets (the rest passed as null; `sscanf` ignores extra arguments).
/// 38 slots: the largest caller is the 2dfx type-5 line in `CFileLoader::Load2dEffect` (38 targets).
pub fn sscanf_n(line: GPtr<u8>, fmt: GPtr<u8>, t: &[GPtr<u8>]) -> i32 {
    let mut a = [GPtr::<u8>::NULL; 38];
    a[..t.len()].copy_from_slice(t);
    sscanf(
        line, fmt, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9], a[10], a[11], a[12], a[13], a[14],
        a[15], a[16], a[17], a[18], a[19], a[20], a[21], a[22], a[23], a[24], a[25], a[26], a[27], a[28], a[29],
        a[30], a[31], a[32], a[33], a[34], a[35], a[36], a[37],
    )
}

/// `CVehicleModelInfo::SetupCommonData` (0x5B8F00): colours, upgrades, env maps (a loading-screen
/// chunk after each), the shared "vehicle" TXD and its light textures, the CVehicleStructure pool
/// (50, "VehicleStruct"), the dirt textures.
#[hook(addr = 0x5B8F00, conv = cdecl, name = "CVehicleModelInfo::SetupCommonData", expect = "6A FF 68 5B D5 83 00")]
pub fn setup_common_data() {
    load_vehicle_colours_call();
    new_chunk_loaded();
    load_vehicle_upgrades_call();
    new_chunk_loaded();
    load_environment_maps_call();
    new_chunk_loaded();
    let mut slot = find_txd_slot(s(0x86982C));
    if slot == -1 {
        slot = add_txd_slot(s(0x86982C));
    }
    load_txd(slot, s(0x869AD4));
    txd_add_ref(slot);
    if slot != -1 {
        let pool = crate::shared::TXD_POOL.get();
        let map = rdp(pool.cast(), 4);
        let def = if (rd8(map, slot as isize) as i8) < 0 {
            GPtr::NULL
        } else {
            rdp(pool.cast(), 0).byte_add(12 * slot as isize)
        };
        VEHICLE_TXD.set(rdp(def, 0));
    }
    LIGHTS_TEXTURE.set(rw_tex_dictionary_find_named_texture(VEHICLE_TXD.get(), s(0x869AC0)));
    LIGHTS_ON_TEXTURE.set(rw_tex_dictionary_find_named_texture(VEHICLE_TXD.get(), s(0x869AAC)));
    new_chunk_loaded();
    let p = operator_new(0x14);
    let pool = if p.is_null() { GPtr::NULL } else { vehicle_structure_pool_ctor(p, 0x32, s(0x869A9C)) };
    GPtr::<GPtr<u8>>::from_addr(0xB4E680).write(pool);
    initialise_dirt_texture();
}

/// `CVehicleModelInfo::LoadVehicleUpgrades` (0x5B65A0): DATA\CARMODS.DAT, sections link / mods /
/// wheel ... end (`strncmp` prefixes), tokens split by `strtok(" \t,")`. Reproduced quirks: a
/// section header is only recognised outside a section; "end" is tested before the section's own
/// handling; model lookups are not null-checked before SetupVehicleUpgradeFlags; the link line's
/// second token is not null-checked; `ms_numWheelUpgrades` is cleared as two dwords.
#[hook(addr = 0x5B65A0, conv = cdecl, name = "CVehicleModelInfo::LoadVehicleUpgrades", expect = "83 EC 18 53 56")]
pub fn load_vehicle_upgrades() {
    let mut delim: u32 = 0x002C_0920; // " \t,"
    let dp = GPtr::from_raw(&raw mut delim).cast::<u8>();
    let mut state: i32 = 0;
    let f = open_file(s(0x869668), s(0x85A53C));
    NUM_WHEEL_UPGRADES.ptr().cast::<u32>().write(0);
    NUM_WHEEL_UPGRADES.ptr().cast::<u32>().wrapping_index(1).write(0);
    let mut id: i32 = 0;
    let mut id1: i32 = 0;
    let mut wheel_set: i32 = 0;
    let idp = GPtr::from_raw(&raw mut id);
    let id1p = GPtr::from_raw(&raw mut id1);
    let mut line = load_line(f);
    while !line.is_null() {
        let c = rd8(line, 0);
        if c != b'#' && c != 0 {
            if state == 0 {
                if strncmp(s(0x869660), line, 4) == 0 {
                    state = 1;
                } else if strncmp(s(0x869658), line, 4) == 0 {
                    state = 2;
                } else if strncmp(s(0x869650), line, 5) == 0 {
                    state = 3;
                }
            } else {
                if strncmp(s(0x8671EC), line, 3) == 0 {
                    state = 0;
                }
                match state {
                    3 => {
                        id = -1;
                        sscanf_n(line, s(0x859508), &[GPtr::from_raw(&raw mut wheel_set).cast()]);
                        strtok(line, dp);
                        let mut tok = strtok(GPtr::NULL, dp);
                        while !tok.is_null() {
                            let m = get_model_info_full(tok, idp);
                            setup_vehicle_upgrade_flags(m, tok);
                            add_wheel_upgrade_call(wheel_set, id);
                            tok = strtok(GPtr::NULL, dp);
                        }
                    }
                    2 => {
                        let tok = strtok(line, dp);
                        if !tok.is_null() {
                            id = -1;
                            let veh = get_model_info_full(tok, idp);
                            let mut n = 0isize;
                            let mut tok = strtok(GPtr::NULL, dp);
                            while !tok.is_null() {
                                let m = get_model_info_full(tok, idp);
                                setup_vehicle_upgrade_flags(m, tok);
                                wr16(veh, vmi::UPGRADES + 2 * n, id as i16);
                                n += 1;
                                tok = strtok(GPtr::NULL, dp);
                            }
                            let m = get_model_info_full(s(0x85BB28), idp);
                            setup_vehicle_upgrade_flags(m, s(0x85BB28));
                            wr16(veh, vmi::UPGRADES + 2 * n, id as i16);
                            let m = get_model_info_full(s(0x85BB20), idp);
                            setup_vehicle_upgrade_flags(m, s(0x85BB20));
                            wr16(veh, vmi::UPGRADES + 2 + 2 * n, id as i16);
                        }
                    }
                    1 => {
                        id1 = -1;
                        id = -1;
                        let tok = strtok(line, dp);
                        if !tok.is_null() {
                            let m = get_model_info_full(tok, id1p);
                            setup_vehicle_upgrade_flags(m, tok);
                            let tok2 = strtok(GPtr::NULL, dp);
                            let m = get_model_info_full(tok2, idp);
                            setup_vehicle_upgrade_flags(m, tok2);
                            add_upgrade_link_call(LINKED_UPGRADES.ptr().cast(), id1, id);
                        }
                    }
                    _ => {}
                }
            }
        }
        line = load_line(f);
    }
    close_file(f);
}

/// `CVehicleModelInfo::LoadVehicleColours` (0x5B6890): DATA\CARCOLS.DAT read with CFileMgr::ReadLine
/// (0x400, the length comes from the protection chunk 0x405FB1). gr-deviations
/// `VehicleModelInfo.cpp:1328` (keep-original): ',' and '\r' → ' ', no "%d.%d %d" retry, r/g/b
/// kept from the previous "col" line. Section headers: "col", "car4" (tested before "car"), "car",
/// "end"; a "col" line also scans for a '#' (result unused). The r/g/b locals start at 0 here (the
/// original's first values are stack contents; every shipped "col" line sets all three before use
/// of a partial line).
#[hook(addr = 0x5B6890, conv = cdecl, name = "CVehicleModelInfo::LoadVehicleColours", expect = "81 EC D8 04 00 00")]
pub fn load_vehicle_colours() {
    let f = open_file(s(0x869724), s(0x85A53C));
    let mut buf = [0u8; 0x400 + 8];
    let b = GPtr::from_raw(buf.as_mut_ptr());
    let mut state = 0;
    let mut colour: GPtr<u8> = GPtr::from_addr(0xB4E481);
    let (mut r, mut g, mut bl) = (0i32, 0i32, 0i32);
    let mut vals = [0i32; 32];
    let mut name = [0u8; 64];
    while read_line(f, b, 0x400) {
        // skip leading bytes <= 0x20 up to NUL / '\n'
        let mut i = 0isize;
        if rd8(b, 0) <= 0x20 {
            loop {
                let c = rd8(b, i);
                if c == 0 || c == b'\n' {
                    break;
                }
                i += 1;
                if rd8(b, i) > 0x20 {
                    break;
                }
            }
        }
        let start = i;
        let mut j = i;
        while rd8(b, j) != b'\n' && rd8(b, j) != 0 {
            if rd8(b, j) == b',' || rd8(b, j) == b'\r' {
                wr8(b, j, b' ');
            }
            j += 1;
        }
        wr8(b, j, 0);
        let line = b.byte_add(start);
        let c0 = rd8(line, 0);
        if c0 == b'#' || c0 == 0 {
            continue;
        }
        if state == 0 {
            if c0 == b'c' && rd8(line, 1) == b'o' && rd8(line, 2) == b'l' {
                state = 1;
            } else if c0 == b'c' && rd8(line, 1) == b'a' && rd8(line, 2) == b'r' && rd8(line, 3) == b'4' {
                state = 3;
            } else if c0 == b'c' && rd8(line, 1) == b'a' && rd8(line, 2) == b'r' {
                state = 2;
            }
            continue;
        }
        if c0 == b'e' && rd8(line, 1) == b'n' && rd8(line, 2) == b'd' {
            state = 0;
            continue;
        }
        match state {
            1 => {
                sscanf_n(
                    line,
                    s(0x869718),
                    &[
                        GPtr::from_raw(&raw mut r).cast(),
                        GPtr::from_raw(&raw mut g).cast(),
                        GPtr::from_raw(&raw mut bl).cast(),
                    ],
                );
                wr8(colour, 0, g as u8);
                wr8(colour, -1, r as u8);
                wr8(colour, 1, bl as u8);
                wr8(colour, 2, 0xFF);
                if rd8(line, 0) != b'#' {
                    let mut p = line;
                    loop {
                        p = p.byte_add(1);
                        if rd8(p, 0) == b'#' {
                            break;
                        }
                    }
                }
                colour = colour.byte_add(4);
            }
            2 | 3 => {
                let per = if state == 2 { 2 } else { 4 };
                let mut t: [GPtr<u8>; 33] = [GPtr::NULL; 33];
                t[0] = GPtr::from_raw(name.as_mut_ptr());
                for k in 0..(if per == 2 { 16 } else { 32 }) {
                    t[1 + k] = GPtr::from_raw(&raw mut vals[k]).cast();
                }
                let fmt = if per == 2 { 0x8696E4 } else { 0x869680 };
                let n = sscanf_n(line, s(fmt), &t);
                let mi = get_model_info_full(GPtr::from_raw(name.as_mut_ptr()), GPtr::NULL);
                // (n - 1) / per, signed toward zero, stored as a byte
                let nv = ((n - 1) / per) as u8;
                wr8(mi, vmi::NUM_COLOR_VARIATIONS, nv);
                let mut v = 0usize;
                while (v as i32) < rd8(mi, vmi::NUM_COLOR_VARIATIONS) as i32 {
                    let at = |k: usize| vals.get(k).copied().unwrap_or(0) as u8;
                    if per == 2 {
                        wr8(mi, vmi::PRIMARY_COLORS + v as isize, at(2 * v));
                        wr8(mi, vmi::SECONDARY_COLORS + v as isize, at(2 * v + 1));
                        wr8(mi, vmi::TERTIARY_COLORS + v as isize, 0);
                        wr8(mi, vmi::QUATERNARY_COLORS + v as isize, 0);
                    } else {
                        wr8(mi, vmi::PRIMARY_COLORS + v as isize, at(4 * v));
                        wr8(mi, vmi::SECONDARY_COLORS + v as isize, at(4 * v + 1));
                        wr8(mi, vmi::TERTIARY_COLORS + v as isize, at(4 * v + 2));
                        wr8(mi, vmi::QUATERNARY_COLORS + v as isize, at(4 * v + 3));
                    }
                    v += 1;
                }
            }
            _ => {}
        }
    }
    close_file(f);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_DESTRUCTOR,
    HOOK_GET_NUM_DOORS,
    HOOK_ADD_UPGRADE_LINK,
    HOOK_FIND_OTHER_UPGRADE,
    HOOK_GET_ANIM_FILE_INDEX,
    HOOK_HAS_ALPHA_MATERIAL_CB,
    HOOK_IS_VALID_COMP_RULE,
    HOOK_GET_LIST_OF_COMPONENTS_NOT_USED_BY_RULES,
    HOOK_COUNT_COMPS_IN_RULE,
    HOOK_IS_UPGRADE_AVAILABLE,
    HOOK_SET_EDITABLE_MATERIALS_CB_MATERIAL,
    HOOK_REMOVE_WINDOW_ALPHA_CB,
    HOOK_RESET_EDITABLE_MATERIALS,
    HOOK_SET_VEHICLE_COLOUR,
    HOOK_GET_NUM_REMAPS,
    HOOK_ADD_REMAP,
    HOOK_ADD_WHEEL_UPGRADE,
    HOOK_GET_NUM_WHEEL_UPGRADES,
    HOOK_GET_WHEEL_UPGRADE,
    HOOK_GET_CUSTOM_CAR_PLATE_TEXT,
    HOOK_SET_CUSTOM_CAR_PLATE_TEXT,
    HOOK_GET_MAXIMUM_NUMBER_OF_PASSENGERS_FROM_NUMBER_OF_DOORS,
    HOOK_SETUP_LIGHT_FLAGS,
    HOOK_VEHICLE_STRUCTURE_CTOR,
    HOOK_GET_WHEEL_SIZE,
    HOOK_FIND_TEXTURE_CB,
    HOOK_USE_COMMON_VEHICLE_TEX_DICATIONARY,
    HOOK_STOP_USING_COMMON_VEHICLE_TEX_DICATIONARY,
    HOOK_SET_ANIM_FILE,
    HOOK_HIDE_DAMAGED_ATOMIC_CB,
    HOOK_HIDE_ALL_COMPONENTS_ATOMIC_CB,
    HOOK_SET_ATOMIC_RENDERER_CB_HELI,
    HOOK_SET_ATOMIC_RENDER_CALLBACKS,
    HOOK_SET_ATOMIC_FLAG_CB,
    HOOK_CLEAR_ATOMIC_FLAG_CB,
    HOOK_GET_OK_AND_DAMAGED_ATOMIC_CB,
    HOOK_SET_VEHICLE_COMPONENT_FLAGS,
    HOOK_CHOOSE_COMPONENT_GLOBAL,
    HOOK_SET_EDITABLE_MATERIALS_CB_ATOMIC,
    HOOK_SET_EDITABLE_MATERIALS,
    HOOK_CHOOSE_VEHICLE_COLOUR,
    HOOK_GET_MAT_FX_EFFECT_MATERIAL_CB,
    HOOK_SET_ENV_MAP_COEFF_CB,
    HOOK_SCALAR_DELETING_DESTRUCTOR,
    HOOK_SET_ENVIRONMENT_MAP_ATOMIC_CB,
    HOOK_SET_ENV_MAP_COEFF_ATOMIC_CB,
    HOOK_DISABLE_ENV_MAP,
    HOOK_SET_ENV_MAP_COEFF,
    HOOK_INIT,
    HOOK_CONVERT_ANIM_FILE_INDEX,
    HOOK_SET_ATOMIC_RENDERER_CB,
    HOOK_SET_ATOMIC_RENDERER_CB_REAL_HELI,
    HOOK_SET_ATOMIC_RENDERER_CB_PLANE,
    HOOK_SET_ATOMIC_RENDERER_CB_BOAT,
    HOOK_SET_ATOMIC_RENDERER_CB_TRAIN,
    HOOK_GET_WHEEL_POSN,
    HOOK_GET_ORIGINAL_COMP_POSITION,
    HOOK_CHOOSE_COMPONENT,
    HOOK_CHOOSE_SECOND_COMPONENT,
    HOOK_LOAD_ENVIRONMENT_MAPS,
    HOOK_STORE_ATOMIC_USED_MATERIALS_CB,
    HOOK_COLLAPSE_FRAMES_CB,
    HOOK_ASSIGN_REMAP_TXD,
    HOOK_SET_CAR_CUSTOM_PLATE,
    HOOK_SHUTDOWN_LIGHT_TEXTURE,
    HOOK_MOVE_OBJECTS_CB,
    HOOK_SHUTDOWN_ENVIRONMENT_MAPS,
    HOOK_SET_RENDER_PIPELINES,
    HOOK_SET_DIRT_TEXTURES,
    HOOK_SET_ENVIRONMENT_MAP_CB,
    HOOK_REDUCE_MATERIALS_IN_VEHICLE,
    HOOK_PREPROCESS_HIERARCHY,
    HOOK_SET_CLUMP,
    HOOK_CREATE_INSTANCE,
    HOOK_DELETE_RW_OBJECT,
    HOOK_SETUP_COMMON_DATA,
    HOOK_LOAD_VEHICLE_UPGRADES,
    HOOK_LOAD_VEHICLE_COLOURS,
];
