//! STD-7: phase B natives of functions whose 1.0 US body is a lone `ret` (sa-hook cannot patch a 1-byte
//! entry), reached on the first in-world frames.

/// `CCollision::Update()` (0x411E20): `ret`.
#[crate::native(addr = 0x411E20, conv = cdecl, name = "CCollision::Update")]
pub fn collision_update() {}

/// `FUN_004a8dc0` (0x4A8DC0): `ret`.
#[crate::native(addr = 0x4A8DC0, conv = cdecl, name = "FUN_004a8dc0")]
pub fn nullsub_4a8dc0() {}

/// `CRubbish::SetVisibility` (0x7204C0): `ret`.
#[crate::native(addr = 0x7204C0, conv = cdecl, name = "CRubbish::SetVisibility")]
pub fn rubbish_set_visibility() {}

/// `CTrain::UpdateTrains` (0x6F5900): `ret`.
#[crate::native(addr = 0x6F5900, conv = cdecl, name = "CTrain::UpdateTrains")]
pub fn train_update_trains() {}

/// `CPed::UpdateStatEnteringVehicle` (0x5E01A0): `ret`.
#[crate::native(addr = 0x5E01A0, conv = cdecl, name = "CPed::UpdateStatEnteringVehicle")]
pub fn ped_update_stat_entering_vehicle() {}

/// `ProcObjectMan_c::Update` (0x5A3110): `ret`.
#[crate::native(addr = 0x5A3110, conv = cdecl, name = "ProcObjectMan_c::Update")]
pub fn proc_object_man_update() {}

/// `CHeli::SpecialHeliPreRender` (0x6C4750): `ret`.
#[crate::native(addr = 0x6C4750, conv = cdecl, name = "CHeli::SpecialHeliPreRender")]
pub fn heli_special_heli_pre_render() {}

/// `CMovingThings::Render` (0x717150): `ret`.
#[crate::native(addr = 0x717150, conv = cdecl, name = "CMovingThings::Render")]
pub fn moving_things_render() {}

/// `CVehicleRecording::Render` (0x459F70): `ret`.
#[crate::native(addr = 0x459F70, conv = cdecl, name = "CVehicleRecording::Render")]
pub fn vehicle_recording_render() {}

/// `nullsub_4d1390` (0x4D1390): `ret`.
#[crate::native(addr = 0x4D1390, conv = cdecl, name = "nullsub_4d1390")]
pub fn nullsub_4d1390() {}

/// `nullsub_4411a0` (0x4411A0): `ret`.
#[crate::native(addr = 0x4411A0, conv = cdecl, name = "nullsub_4411a0")]
pub fn nullsub_4411a0() {}

/// `nullsub_4c7400` (0x4C7400): `ret`.
#[crate::native(addr = 0x4C7400, conv = cdecl, name = "nullsub_4c7400")]
pub fn nullsub_4c7400() {}

/// `_rxD3D9VertexShaderDefaultEndCallBack` (0x761000): `ret`.
#[crate::native(addr = 0x761000, conv = cdecl, name = "_rxD3D9VertexShaderDefaultEndCallBack")]
pub fn rx_d3d9_vertex_shader_default_end_callback() {}

/// The shared entry of [`LONE_RET_NATIVES`]: returns at once (cdecl; callee-cleaned conventions never
/// reach these, every listed body is `ret` without an immediate).
#[cfg(all(target_arch = "x86", not(sa_phase_a)))]
extern "C" fn lone_ret_entry() {}

/// wasm32: the shared type-erased entry ([`crate::wabi`]): returns nothing.
#[cfg(all(target_arch = "wasm32", not(sa_phase_a)))]
extern "C" fn lone_ret_wasm(_callee: u32, _args: *const u32, _ret: *mut crate::wabi::Ret) {}

fn lone_ret_addr() -> usize {
    #[cfg(all(target_arch = "x86", not(sa_phase_a)))]
    {
        lone_ret_entry as *const () as usize
    }
    #[cfg(all(target_arch = "wasm32", not(sa_phase_a)))]
    {
        lone_ret_wasm as crate::wabi::Erased as usize
    }
    #[cfg(not(all(any(target_arch = "x86", target_arch = "wasm32"), not(sa_phase_a))))]
    {
        0
    }
}

macro_rules! lone_rets {
    ($(($a:expr, $n:expr)),* $(,)?) => {
        /// The lone-`ret` functions as phase B natives (one shared entry).
        pub const LONE_RET_NATIVES: &[crate::abi::NativeReg] =
            &[$(crate::abi::NativeReg { addr: $a, name: $n, entry: lone_ret_addr }),*];
    };
}

// Every other catalogued function in game `.text` whose 1.0 US body is a lone `ret` (0xC3), registered by
// address (the name is the catalogue's; `nullsub_ADDR` where it has none): `(address, name)`.
lone_rets! {
    (0x403E40, "CEntity::ProcessControl"),
    (0x403E50, "CEntity::ProcessCollision"),
    (0x403E60, "CEntity::ProcessShift"),
    (0x403EB0, "CEntity::FlagToDestroyWhenNextProcessed"),
    (0x41BC80, "CBridge::Update"),
    (0x41BCA0, "CBridge::FindBridgeEntities"),
    (0x4395A0, "CCheat::PredatorCheat"),
    (0x43C600, "CCurves::TestCurves"),
    (0x43CE70, "nullsub_43ce70"),
    (0x453FB0, "nullsub_453fb0"),
    (0x4634C0, "nullsub_4634c0"),
    (0x463760, "nullsub_463760"),
    (0x463780, "nullsub_463780"),
    (0x46A380, "nullsub_46a380"),
    (0x485DE0, "CTheScripts::ScriptDebugLine3D"),
    (0x486AE0, "FUN_00486ae0"),
    (0x49EA60, "Fx_c::InitEntitySystems"),
    (0x4A4950, "FxInfoManager_c::~FxInfoManager_c"),
    (0x4A8DE0, "FUN_004a8de0"),
    (0x4A9C20, "FxMemoryPool_c::~FxMemoryPool_c"),
    (0x4AA120, "FxSystemBP_c::Unload"),
    (0x4AD420, "CPedDamageResponseCalculator::~CPedDamageResponseCalculator"),
    (0x4C88F4, "CVehicleModelInfo::SetRenderPipelinesCB"),
    (0x4E3340, "nullsub_4e3340"),
    (0x4EF070, "nullsub_4ef070"),
    (0x508E90, "nullsub_508e90"),
    (0x508EA0, "nullsub_508ea0"),
    (0x5097C0, "nullsub_5097c0"),
    (0x5097D0, "nullsub_5097d0"),
    (0x509800, "nullsub_509800"),
    (0x50AF90, "CCamera::InitialiseCameraForDebugMode"),
    (0x50B880, "CCamera::ProcessObbeCinemaCameraPed"),
    (0x50E080, "nullsub_50e080"),
    (0x538BA0, "CFire"),
    (0x53BED0, "CGame::ReloadIPLs"),
    (0x53E160, "Game::RenderDebugShit"),
    (0x541336, "FUN_00541336"),
    (0x5531D0, "CRenderer::Shutdown"),
    (0x5601E0, "CTimeCycle::Shutdown"),
    (0x561FD0, "CWanted::ResetPolicePursuit"),
    (0x56CDF0, "nullsub_56cdf0"),
    (0x59A130, "Interior_c::Shop_FurnishCeiling"),
    (0x59C9D0, "nullsub_59c9d0"),
    (0x59E660, "BreakManager_c::Exit"),
    (0x5B0440, "CCutsceneMgr::SetHeadAnim"),
    (0x5B9E30, "nullsub_5b9e30"),
    (0x5C36F0, "nullsub_5c36f0"),
    (0x5C79E0, "nullsub_5c79e0"),
    (0x5C9340, "nullsub_5c9340"),
    (0x5CF880, "FUN_005cf880"),
    (0x5DABA0, "CGrassRenderer::Shutdown"),
    (0x5DB2A0, "nullsub_5db2a0"),
    (0x5DB2C0, "nullsub_5db2c0"),
    (0x5DE540, "CGangInfo"),
    (0x5DEBE0, "CPed::Update"),
    (0x5E01B0, "CPed::UpdateStatLeavingVehicle"),
    (0x6082E0, "nullsub_6082e0"),
    (0x609490, "CPlayerPed::SetMoveAnim"),
    (0x60F8C0, "FUN_0060f8c0"),
    (0x610EC0, "CPopulation::Shutdown"),
    (0x6116A0, "CPopulation::MoveCarsAndPedsOutOfAbandonedZones"),
    (0x6116B0, "CPopulation::DealWithZoneChange"),
    (0x618E70, "CGenericGameStorage::InitRadioStationPositionList"),
    (0x62EC50, "nullsub_62ec50"),
    (0x6A6290, "CVehicle::FlagToDestroyWhenNextProcessed"),
    (0x6B67A0, "CBike::DebugCode"),
    (0x6B7070, "CBike::SetupDamageAfterLoad"),
    (0x6D0B30, "FUN_006d0b30"),
    (0x6D6380, "CVehicle::SetupSuspensionLines"),
    (0x6D6390, "CVehicle::Fix"),
    (0x6D63A0, "CVehicle::SetupDamageAfterLoad"),
    (0x6D63B0, "CVehicle::DoBurstAndSoftGroundRatios"),
    (0x6D63C0, "CVehicle::PlayCarHorn"),
    (0x6F3FE0, "CCranes::UpdateCranes"),
    (0x6F5910, "CTrain::ProcessTrainAnnouncements"),
    (0x6F5920, "CTrain::PlayAnnouncement"),
    (0x6F9E00, "nullsub_6f9e00"),
    (0x6F9E30, "nullsub_6f9e30"),
    (0x7076B0, "CShadows::PrintDebugPoly"),
    (0x70CE10, "CSprite::Initialise"),
    (0x70F8E0, "CStencilShadowObject::Destructor"),
    (0x717820, "CPlaneBanners::Init"),
    (0x71D700, "CMBlur::MotionBlurRender"),
    (0x7228F0, "CCheckpoints::Shutdown"),
    (0x732307, "CVisibilityPlugins::SetClumpForAllAtomicsFlag"),
    (0x734620, "DestroyDebugFont"),
    (0x735720, "Light::LightsEnable"),
    (0x736000, "CBulletInfo::Shutdown"),
    (0x7368F0, "CExplosion::Shutdown"),
    (0x739C20, "CShotInfo::Shutdown"),
    (0x742AA0, "CWeaponEffects"),
    (0x743C40, "CWeaponInfo"),
    (0x743C50, "CWeaponInfo::Shutdown"),
    (0x744DB5, "CLoadingScreen::IsActive"),
    (0x7458A0, "Ps::psTerminate"),
    (0x76E686, "FUN_0076e686"),
    (0x782810, "nullsub_782810"),
    (0x7836C0, "nullsub_7836c0"),
    (0x7856D0, "FUN_007856d0"),
    (0x7B4D5F, "FUN_007b4d5f"),
    (0x7CC740, "RpUVAnimParamKeyFrameMulRecip"),
    (0x7CC750, "RpUVAnimParamKeyFrameAdd"),
    (0x7CF9A0, "nullsub_7cf9a0"),
    (0x8055C0, "RxClusterUnlock"),
    (0x807C80, "FUN_00807c80"),
    (0x8563D0, "__dtor_gIdleCam"),
    (0x856440, "__dtor_84e0f0"),
    (0x856450, "__dtor_84e100"),
    (0x8564A0, "__dtor_84e6e0"),
    (0x8564B0, "__dtor_CTimeCycle__m_BelowHorizonGrey"),
    (0x856700, "__dtor_g_c03970"),
    (0x856710, "__dtor_CPlantMgr__m_AmbientColor"),
    (0x856940, "__dtor_852850"),
    (0x856B00, "__dtor_854c70"),
    (0x856B10, "__dtor_854c80"),
    (0x856B80, "__dtor_854da0"),
    (0x856B90, "__dtor_854db0"),
    (0x856C80, "__dtor_855fc0"),
}

/// Every `#[native]` of this module.
pub const NATIVES: &[crate::abi::NativeReg] = &[
    NATIVE_COLLISION_UPDATE,
    NATIVE_NULLSUB_4A8DC0,
    NATIVE_RUBBISH_SET_VISIBILITY,
    NATIVE_TRAIN_UPDATE_TRAINS,
    NATIVE_PED_UPDATE_STAT_ENTERING_VEHICLE,
    NATIVE_PROC_OBJECT_MAN_UPDATE,
    NATIVE_HELI_SPECIAL_HELI_PRE_RENDER,
    NATIVE_MOVING_THINGS_RENDER,
    NATIVE_VEHICLE_RECORDING_RENDER,
    NATIVE_NULLSUB_4D1390,
    NATIVE_NULLSUB_4411A0,
    NATIVE_NULLSUB_4C7400,
    NATIVE_RX_D3D9_VERTEX_SHADER_DEFAULT_END_CALLBACK,
];
