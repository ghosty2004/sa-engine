//! PORT-PED-2 (`docs/arch/ped-port-2.md`): ped construction and destruction. `CPed` constructor and
//! destructor, the `CCivilianPed` / `CCopPed` / `CEmergencyPed` / `CPlayerPed` constructors, destructors and
//! scalar deleting destructors, `CPed::SetModelIndex`, the ped pool's handle-indexed `New`, and
//! `CPedIntelligence::operator new`. Ported from the 1.0 US bytes (gta-reversed `Ped.cpp`, `CopPed.cpp`,
//! `CivilianPed.cpp`, `EmergencyPed.cpp`, `PlayerPed.cpp` as reading aids).
//!
//! Not reproduced (stack-only, invisible to callers): the SEH frames (`fs:[0]`, the state byte) of the
//! constructors and destructors, the constructor's write of the `CTask*` it allocates into its own argument
//! slot, and the stack locals the constructor zeroes before using one of them for `+0x580`. The
//! `CEventAcquaintancePedHate` the constructor builds on its stack lives in a Rust stack buffer of the same
//! size and is built and destroyed by the original constructor/destructor.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

/// `CPed` vtable (0x86C358), `CCivilianPed` (0x86C0A8), `CCopPed` (0x86C120), `CEmergencyPed` (0x86C200),
/// `CPlayerPed` (0x86D168).
const VT_PED: u32 = 0x86C358;
const VT_CIVILIAN: u32 = 0x86C0A8;
const VT_COP: u32 = 0x86C120;
const VT_EMERGENCY: u32 = 0x86C200;
const VT_PLAYER: u32 = 0x86D168;
/// `CPools::ms_pPedIntelligencePool` (slot 0x294)
const PED_INTEL_POOL: usize = 0xB744C0;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CPedStats::ms_apPedStats` (array base pointer, 0x34 each)
const PED_STATS: usize = 0xC0BBEC;
/// `CPopulation::ms_nTotalMissionPeds`-style counter decremented for peds with flag bit 0x10000 of +0x470
const MISSION_PED_COUNT: usize = 0xC0FCB8;
/// the "peds hate the player" switch the constructor tests (byte)
const PEDS_HATE_PLAYER: usize = 0x96913F;
/// `RpAnimBlendClumpData` plugin offset (`ClumpOffset`)
const CLUMP_ANIM_OFFSET: usize = 0xB5F878;
/// `CStreaming::ms_aInfoForModel`-adjacent table the destructor reads (`CTheScripts` mission-ped model
/// table, 20 bytes per entry, i16 model at +0)
const SCRIPT_MODEL_TABLE: usize = 0xA90CF0;
/// `CAnimManager::ms_aAnimAssocGroups`-style pointer (20 bytes per entry, block pointer at +0)
const ANIM_ASSOC_GROUPS: usize = 0xB4EA34;
/// `CAnimManager::ms_aAnimBlocks` (0x20 each)
const ANIM_BLOCKS: u32 = 0xB5D4A0;
/// `CAEAudioEntity` base vtable written back by the destructor (0x859988)
const VT_AUDIO_ENTITY: u32 = 0x859988;

orig! {
    /// `CPhysical::CPhysical()` (0x542260)
    fn physical_ctor(e: GPtr<u8>) = thiscall 0x542260 => unported;
    /// `CPhysical::~CPhysical()` (0x542450)
    fn physical_dtor(e: GPtr<u8>) = thiscall 0x542450 => unported;
    /// `CAEPedSpeechAudioEntity::CAEPedSpeechAudioEntity()` (0x4E4F10)
    fn speech_ctor(e: GPtr<u8>) = thiscall 0x4E4F10 => unported;
    /// `CPedIK::CPedIK(CPed*)` (0x5FD8C0)
    fn ped_ik_ctor(ik: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5FD8C0 => unported;
    /// `` `eh vector constructor iterator' `` (0x821D30, stdcall)
    fn eh_vector_ctor(p: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// `` `eh vector destructor iterator' `` (0x821E02, stdcall)
    fn eh_vector_dtor(p: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// `CAEPedWeaponAudioEntity::Initialise(CPed*)` (0x4E6AA0)
    fn weapon_audio_init(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4E6AA0 => unported;
    /// `CAEPedWeaponAudioEntity::Terminate()` (0x4E6AC0)
    fn weapon_audio_terminate(e: GPtr<u8>) = thiscall 0x4E6AC0 => unported;
    /// `CAEPedAudioEntity::Initialise(CEntity*)` (0x4E0E80)
    fn ped_audio_init(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4E0E80 => unported;
    /// `CAEPedAudioEntity::Terminate()` (0x4E1360)
    fn ped_audio_terminate(e: GPtr<u8>) = thiscall 0x4E1360 => unported;
    /// `CAEPedAudioEntity::~CAEPedAudioEntity()` (0x5DE920)
    fn ped_audio_dtor(e: GPtr<u8>) = thiscall 0x5DE920 => unported;
    /// `CAESound` array / `CAEAudioEntity` member destructor (0x4EF660, on entity + 8)
    fn audio_entity_member_dtor(e: GPtr<u8>) = thiscall 0x4EF660 => unported;
    /// `CAEPedSpeechAudioEntity::Initialise(CEntity*)` (0x4E68D0)
    fn speech_init(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4E68D0 => unported;
    /// speech entity vtable slot 2 (`Terminate`)
    fn speech_terminate(e: GPtr<u8>) = thiscall vslot 0x08 => unported;
    /// `CPedType::GetPedTypeAcquaintances(ePedType)` (0x6089B0)
    fn get_ped_type_acquaintances(ty: u32) -> GPtr<u32> = cdecl 0x6089B0 => unported;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830)
    fn get_ped_flag(ty: u32) -> u32 = cdecl 0x608830 => unported;
    /// `CAcquaintance::SetAsAcquaintance(int32, uint32)` (0x608DA0)
    fn set_as_acquaintance(a: GPtr<u8>, k: u32, flag: u32) = thiscall 0x608DA0 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32, bool)` (0x5E6080)
    fn give_weapon(p: GPtr<u8>, ty: u32, ammo: u32, b: u32) -> i32 = thiscall 0x5E6080 => unported;
    /// `CPed::GiveDelayedWeapon(eWeaponType, uint32)` (0x5E89B0)
    fn give_delayed_weapon(p: GPtr<u8>, ty: u32, ammo: u32) = thiscall 0x5E89B0 => unported;
    /// `CPed::SetCurrentWeapon(eWeaponType)` (0x5E6280)
    fn set_current_weapon(p: GPtr<u8>, ty: i32) = thiscall 0x5E6280 => crate::ped_weapon::set_current_weapon_type;
    /// `CPedIntelligence::operator new(uint32)` (0x6074A0)
    fn intel_new(size: u32) -> GPtr<u8> = cdecl 0x6074A0 => ped_intelligence_new;
    /// `CPedIntelligence::operator delete(void*)` (0x6074E0)
    fn intel_delete(p: GPtr<u8>) = cdecl 0x6074E0 => unported;
    /// `CPedIntelligence::CPedIntelligence(CPed*)` (0x607140)
    fn intel_ctor(i: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x607140 => unported;
    /// `CPedIntelligence::~CPedIntelligence()` (0x607300)
    fn intel_dtor(i: GPtr<u8>) = thiscall 0x607300 => unported;
    /// `CPool<CPedIntelligence>::New()` (0x605EC0)
    fn intel_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x605EC0 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// default primary task constructor for non-player peds (0x690D20)
    fn task_primary_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x690D20 => unported;
    /// default secondary task constructor (0x62F310, `(0, 1, 0, 8.0f)`)
    fn task_secondary_ctor(t: GPtr<u8>, a: u32, b: u32, c: u32, d: f32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskManager::SetTask(CTask*, int32)` (0x681B60)
    fn task_manager_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: u32) = thiscall 0x681B60 => unported;
    /// `CTaskManager::SetTaskSecondary(CTask*, int32)` (0x681AF0)
    fn task_manager_set_task_secondary(tm: GPtr<u8>, t: GPtr<u8>, slot: u32, z: u32) = thiscall 0x681AF0 => unported;
    /// population ped count update `(CPed*, bool removing)` (0x611570)
    fn population_update_ped_count(ped: GPtr<u8>, removing: u32) = cdecl 0x611570 => unported;
    /// `CEventAcquaintancePedHate::CEventAcquaintancePedHate(CPed*)` (0x4AF820)
    fn event_hate_ctor(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4AF820 => unported;
    /// `CEventAcquaintancePed::~CEventAcquaintancePed()` (0x4AF890)
    fn event_hate_dtor(e: GPtr<u8>) = thiscall 0x4AF890 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CReplay::RecordPedDeleted`-style hook the destructor calls first (0x45EC20)
    fn dtor_notify_replay(ped: GPtr<u8>) = cdecl 0x45EC20 => unported;
    /// `CStreaming::SetMissionDoesntRequireModel(int32)` (0x409C90)
    fn set_mission_doesnt_require_model(m: i32) = cdecl 0x409C90 => unported;
    /// script ped-model release `(CPed*, uint16)` (0x46ABC0)
    fn script_release_ped_model(ped: GPtr<u8>, idx: u32) = cdecl 0x46ABC0 => unported;
    /// `CWorld::Remove`-side cleanup of references to a deleted entity (0x563280)
    fn world_remove_references(ped: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CPedGroups`-side cleanup for a deleted ped (0x43A960)
    fn dtor_cleanup_43a960(ped: GPtr<u8>) = cdecl 0x43A960 => unported;
    /// `CFire::Extinguish()` (0x5393F0)
    fn fire_extinguish(f: GPtr<u8>) = thiscall 0x5393F0 => unported;
    /// `CPed::ClearWeapons()`-style weapon cleanup (0x5E6320)
    fn ped_dtor_weapons(ped: GPtr<u8>) = thiscall 0x5E6320 => unported;
    /// `CPed::operator delete(void*)` (0x5E4760)
    fn ped_delete(p: GPtr<u8>) = cdecl 0x5E4760 => unported;
    /// `CPed::~CPed()` (0x5E8620)
    fn ped_dtor_call(p: GPtr<u8>) = thiscall 0x5E8620 => ped_destructor;
    /// `CPed::CPed(ePedType)` (0x5E8030)
    fn ped_ctor_call(p: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x5E8030 => ped_constructor;
    /// `CPed::SetModelIndex(uint32)` (0x5E4880)
    fn ped_set_model_index_call(p: GPtr<u8>, m: u32) = thiscall 0x5E4880 => ped_set_model_index;
    /// `CCivilianPed::~CCivilianPed()` (0x5DDBE0)
    fn civilian_dtor_call(p: GPtr<u8>) = thiscall 0x5DDBE0 => civilian_ped_dtor;
    /// `CCopPed::~CCopPed()` (0x5DE0D0)
    fn cop_dtor_call(p: GPtr<u8>) = thiscall 0x5DE0D0 => cop_ped_destructor;
    /// `CEmergencyPed::~CEmergencyPed()` (0x5DE3B0)
    fn emergency_dtor_call(p: GPtr<u8>) = thiscall 0x5DE3B0 => emergency_ped_dtor;
    /// `CPlayerPed::~CPlayerPed()` (0x6093B0)
    fn player_dtor_call(p: GPtr<u8>) = thiscall 0x6093B0 => player_ped_dtor;
    /// `CWanted::RemoveCop`-style call on the player's wanted (0x562C10)
    fn wanted_remove_cop(w: GPtr<u8>, cop: GPtr<u8>) = thiscall 0x562C10 => unported;
    /// `CCopPed` member cleanup (0x5DE070)
    fn cop_clear_criminals(cop: GPtr<u8>) = thiscall 0x5DE070 => unported;
    /// `CStreaming::GetDefaultCopModel()` (0x407C00)
    fn get_default_cop_model() -> u32 = cdecl 0x407C00 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn remove_anim_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CPedGroups::RemoveGroup(int32)` (0x5FB870)
    fn ped_groups_remove_group(g: u32) = cdecl 0x5FB870 => unported;
    /// `CEntity::SetModelIndex(uint32)` (0x532AE0)
    fn entity_set_model_index(e: GPtr<u8>, m: u32) = thiscall 0x532AE0 => unported;
    /// `RpAnimBlendClumpInit(RpClump*)` (0x4D6720)
    fn anim_blend_clump_init(c: GPtr<u8>) = cdecl 0x4D6720 => unported;
    /// `RpAnimBlendClumpFillFrameArray(RpClump*, AnimBlendFrameData**)` (0x4D64A0)
    fn anim_blend_clump_fill_frame_array(c: GPtr<u8>, out: GPtr<u8>) = cdecl 0x4D64A0 => unported;
    /// `CPopulation::IsMale`-style model test `(int32 model, int32 k)` (0x610210)
    fn model_test_610210(m: i32, k: i32) -> u8 = cdecl 0x610210 => unported;
    /// `CAnimManager::AddAnimation(RpClump*, AssocGroupId, AnimationId)` (0x4D3AA0)
    fn add_animation(c: GPtr<u8>, g: u32, a: u32) -> GPtr<u8> = cdecl 0x4D3AA0 => unported;
    /// `CPedModelInfo::CreateHitColModelSkinned(RpClump*)` (0x4C6D90)
    fn create_hit_col_model_skinned(mi: GPtr<u8>, c: GPtr<u8>) = thiscall 0x4C6D90 => unported;
    /// `CEntity::UpdateRpHAnim()`-style follow-up (0x532B20)
    fn entity_update_after_model(e: GPtr<u8>) = thiscall 0x532B20 => unported;
}

/// The pool index of `p` (`(p - objects) / size` as the exe's signed magic-number division).
fn pool_index(pool: GPtr<u8>, p: GPtr<u8>, size: usize) -> i32 {
    let base = rd::<GPtr<u8>>(pool, 0);
    (p.addr() as u32).wrapping_sub(base.addr() as u32) as i32 / size as i32
}

/// `CPool<CPed, CCopPed>::New(int32 handle)` (0x5E0540, unnamed in the catalogue, `ret 4`): the slot of
/// `handle >> 8` is marked used with the handle's low 7 bits as its id, then the first-free index (+0xC)
/// restarts at 0 and walks up to the first free slot (no bound check, as the original). EAX: the low byte
/// written to the flag byte with the index's upper bytes when slot 0 is free, else the first-free index.
#[hook(addr = 0x5E0540, conv = thiscall, name = "FUN_005e0540", expect = "8B 51 04 53 8B 5C 24 08")]
pub fn ped_pool_new_at(this: GPtr<u8>, handle: i32) -> u32 {
    let flags = rd::<GPtr<u8>>(this, 4);
    let idx = handle >> 8;
    let f = flags.byte_add(idx as isize);
    f.write(f.read() & 0x7F);
    let v = f.read();
    let al = (v ^ handle as u8) & 0x7F;
    f.write(v ^ al);
    wr::<i32>(this, 0xC, 0);
    let mut eax = (idx as u32 & 0xFFFF_FF00) | u32::from(al);
    let mut first = 0i32;
    while (rd::<GPtr<u8>>(this, 4).byte_add(first as isize).read() as i8) >= 0 {
        first = first.wrapping_add(1);
        wr::<i32>(this, 0xC, first);
        eax = first as u32;
    }
    eax
}

orig! {
    /// `CPool<CPed>::New(int32)` (0x5E0540)
    fn ped_pool_new_at_call(pool: GPtr<u8>, handle: i32) -> u32 = thiscall 0x5E0540 => ped_pool_new_at;
}

/// `CPed::operator new(uint32 size, int32 poolRef)` (0x5E4730, cdecl): `New(poolRef)` on the ped pool,
/// then the slot `objects + (poolRef >> 8) * 0x7C4` (computed before the call, from the objects pointer read
/// before it).
#[hook(addr = 0x5E4730, conv = cdecl, name = "CPed::operator new-poolIndexed", expect = "8B 44 24 08 8B 0D 90 44 B7 00")]
pub fn ped_new_at(_size: u32, handle: i32) -> GPtr<u8> {
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let slot = rd::<GPtr<u8>>(pool, 0).byte_add(((handle >> 8) as isize).wrapping_mul(PED_SLOT as isize));
    ped_pool_new_at_call(pool, handle);
    slot
}

/// `CPedIntelligence::operator new(uint32)` (0x6074A0, cdecl): `ms_pPedIntelligencePool->New()`.
#[hook(addr = 0x6074A0, conv = cdecl, name = "CPedIntelligence::operator new", expect = "8B 0D C0 44 B7 00")]
pub fn ped_intelligence_new(_size: u32) -> GPtr<u8> {
    intel_pool_new(g::<GPtr<u8>>(PED_INTEL_POOL).read())
}

/// `CPed::CPed(ePedType)` (0x5E8030, `ret 4`). The field initialisation in the original order (callees
/// read some of the fields written before them), one `rand()` (return address 0x5E82DE) deciding bit 17
/// of the flags dword +0x46C, the intelligence from `CPedIntelligence::operator new`, a primary task for
/// non-player ped types (0x690D20 into slot 3) and a secondary task (0x62F310 `(0, 1, 0, 8.0f)` into
/// slot 4), and with the switch byte 0x96913F set (non-player types) the hate acquaintance towards the
/// player plus a `CEventAcquaintancePedHate` (response timer word +0xE = 1000) added to the event group.
#[hook(addr = 0x5E8030, conv = thiscall, name = "CPed::Constructor", expect = "6A FF 68 03 DA 83 00", this = "CPed")]
pub fn ped_constructor(this: GPtr<u8>, ped_type: u32) -> GPtr<u8> {
    let p = this;
    physical_ctor(p);
    wr::<u32>(p, 0, VT_PED);
    // CAEPedAudioEntity (+0x138), inline
    let a = p.byte_add(0x138);
    wr::<u32>(a, 0x10, 0);
    wr::<u32>(a, 0, 0x86C2A8);
    wr::<u32>(a, 0xB8, 0);
    wr::<u32>(a, 0xA8, 0x85F438);
    wr::<u16>(a, 0x130, 0);
    wr::<u32>(a, 0x148, 0);
    wr::<u32>(a, 0x14C, 0);
    wr::<u32>(a, 0x94, 0);
    wr::<u8>(a, 0x7C, 0);
    wr::<u32>(a, 0x9C, 0);
    wr::<u32>(a, 0xA0, 0);
    wr::<u32>(a, 0xA4, 0);
    speech_ctor(p.byte_add(0x294));
    // CAEPedWeaponAudioEntity (+0x394), inline
    let w = p.byte_add(0x394);
    for off in [0x10, 0x84, 0x88, 0x8C, 0x9C] {
        wr::<u32>(w, off, 0);
    }
    wr::<u8>(w, 0x7E, 0);
    wr::<u32>(w, 0x90, 0);
    wr::<u8>(w, 0x7C, 0);
    wr::<u8>(w, 0x7D, 0);
    wr::<u32>(w, 0x94, 0);
    wr::<u32>(w, 0x98, 0);
    wr::<u32>(w, 0, 0x86C2AC);
    wr::<u32>(w, 0xA4, 0);
    wr::<u8>(w, 0xA0, 0);
    wr::<u32>(p, 0x4D8, 0);
    wr::<u32>(p, 0x4DC, 0);
    for k in 0..5 {
        wr::<u32>(p, 0x4E0 + 4 * k, 0);
    }
    ped_ik_ctor(p.byte_add(0x50C), p);
    wr::<u32>(p, 0x540, 0x42C8_0000);
    wr::<u32>(p, 0x544, 0x42C8_0000);
    wr::<u32>(p, 0x548, 0);
    wr::<u32>(p, 0x598, ped_type);
    eh_vector_ctor(p.byte_add(0x5A0), 0x1C, 0xD, 0x441E00, 0x4411A0);
    let b36 = rd::<u8>(p, 0x36);
    let f40 = rd::<u32>(p, 0x40);
    wr::<u8>(p, 0x36, (b36 & 0xFB) | 3);
    wr::<u32>(p, 0x40, f40 | 0x1000_0010);
    wr::<u8>(p, 0x484, 1);
    for off in [0x58C, 0x52C, 0x744, 0x74C, 0x748, 0x750, 0x550, 0x554, 0x56C, 0x570, 0x574, 0x578, 0x57C] {
        wr::<u32>(p, off, 0);
    }
    wr::<u32>(p, 0x530, 1);
    wr::<u32>(p, 0x534, 1);
    let flags = (rd::<u32>(p, 0x46C) & 0xFFFE_2000) | 0x2000;
    wr::<u32>(p, 0x558, 0);
    wr::<u32>(p, 0x560, 0x4170_0000);
    wr::<u32>(p, 0x564, 0x3DCC_CCCD);
    wr::<u32>(p, 0x55C, 0);
    wr::<u32>(p, 0x568, 0);
    wr::<u32>(p, 0x580, 0x3F80_0000);
    wr::<u8>(p, 0x719, 0x28);
    for off in [0x594, 0x128, 0x538, 0x730] {
        wr::<u32>(p, off, 0);
    }
    wr::<u32>(p, 0x734, 0x3F80_0000);
    for off in [0x71C, 0x738, 0x73C, 0x584] {
        wr::<u32>(p, off, 0);
    }
    wr::<u32>(p, 0x588, 0x47C3_4FFF);
    wr::<u32>(p, 0x8C, 0x428C_0000);
    wr::<u32>(p, 0x90, 0x42C8_0000);
    wr::<u32>(p, 0x98, 0x3BBB_3EE7);
    wr::<u32>(p, 0x9C, 0x3D4C_CCCD);
    wr::<u8>(p, 0x754, 0xFF);
    wr::<u32>(p, 0x46C, flags);
    let r = at_site(0x5E82DE, rand);
    let f46c = rd::<u32>(p, 0x46C);
    let f474 = rd::<u32>(p, 0x474);
    wr::<u32>(p, 0x470, 0x0610_0000);
    let bit = u32::from(r as u8 & 3 == 0);
    wr::<u32>(p, 0x46C, (f46c & 0x1FFFF) | 0x8000_0000 | (bit << 17));
    let f478 = rd::<u32>(p, 0x478);
    wr::<u32>(p, 0x474, (f474 & 0x2000_0000) | 0x0400_0000);
    wr::<u32>(p, 0x478, (f478 & 0xFFC2_1020) | 0x21000);
    weapon_audio_init(w, p);
    ped_audio_init(a, p);
    let acq = get_ped_type_acquaintances(rd::<u32>(p, 0x598));
    for k in 0..5 {
        wr::<u32>(p, 0x4E0 + 4 * k as isize, acq.wrapping_index(k).read());
    }
    wr::<u32>(p, 0x70C, 0x37);
    wr::<u32>(p, 0x710, 0x37);
    wr::<u8>(p, 0x718, 0);
    for s in 0..13isize {
        let slot = p.byte_add(0x5A0 + s * 0x1C);
        for k in 0..5 {
            wr::<u32>(slot, 4 * k, 0);
        }
    }
    wr::<u8>(p, 0x72C, 1);
    wr::<u8>(p, 0x72D, 4);
    wr::<u8>(p, 0x72E, 0);
    give_weapon(p, 0, 0, 1);
    wr::<u8>(p, 0x71A, 0x3C);
    wr::<u8>(p, 0x760, 0xFF);
    for off in [0x764, 0x768, 0xFC, 0x784] {
        wr::<u32>(p, off, 0);
    }
    wr::<u8>(p, 0x460, 0);
    wr::<u32>(p, 0x468, 0);
    wr::<u32>(p, 0x740, 0xFFFF_FFFF);
    wr::<u16>(p, 0x756, 0);
    wr::<u8>(p, 0x72F, 0);
    for off in [0x54C, 0x590, 0x4F4, 0x4F8, 0x4FC, 0x500] {
        wr::<u32>(p, off, 0);
    }
    for off in [0x504, 0x506, 0x508, 0x50A] {
        wr::<u16>(p, off, 0);
    }
    wr::<u32>(p, 0x788, 0);
    wr::<u32>(p, 0x78C, 0);
    wr::<u32>(p, 0x798, 0xFFFF_FFFF);
    let mem = intel_new(0x294);
    let intel = if mem.is_null() { GPtr::from_addr(0) } else { intel_ctor(mem, p) };
    wr::<GPtr<u8>>(p, 0x47C, intel);
    let ty = rd::<u32>(p, 0x598);
    wr::<u32>(p, 0x480, 0);
    if ty != 0 && ty != 1 {
        let t = task_new(0x20);
        let t = if t.is_null() { t } else { task_primary_ctor(t) };
        task_manager_set_task(rd::<GPtr<u8>>(p, 0x47C).byte_add(4), t, 3);
    }
    let t = task_new(0x20);
    let t = if t.is_null() { t } else { task_secondary_ctor(t, 0, 1, 0, 8.0) };
    task_manager_set_task_secondary(rd::<GPtr<u8>>(p, 0x47C).byte_add(4), t, 4, 0);
    wr::<u32>(p, 0x758, 0);
    wr::<u32>(p, 0x790, 0x3F80_0000);
    wr::<u16>(p, 0x794, 0xFFFF);
    population_update_ped_count(p, 0);
    let f474 = rd::<u32>(p, 0x474);
    let f478 = rd::<u32>(p, 0x478);
    wr::<u32>(p, 0x474, f474 & 0xDFFF_FFFF);
    wr::<u32>(p, 0x478, f478 & 0xFFFF_FFDF);
    if g::<u8>(PEDS_HATE_PLAYER).read() != 0 {
        let ty = rd::<u32>(p, 0x598);
        if ty != 0 && ty != 1 {
            let flag = get_ped_flag(0);
            set_as_acquaintance(p.byte_add(0x4E0), 4, flag);
            let player = find_player_ped(-1);
            let mut ev = [0u32; sa_core::layouts::size::CEventAcquaintancePedHate / 4];
            let e = lp(&mut ev).cast::<u8>();
            event_hate_ctor(e, player);
            wr::<u32>(e, 0, 0x858E68);
            wr::<u16>(e, 0xE, 1000);
            event_group_add(rd::<GPtr<u8>>(p, 0x47C).byte_add(0x68), e, 0);
            wr::<u32>(e, 0, 0x858E68);
            event_hate_dtor(e);
        }
    }
    p
}

/// `CPed::~CPed()` (0x5E8620; entry `push -1` + `jmp` into a protection chunk at 0x40136C that pushes the
/// SEH handler computed from `[0x1560624] - [0x6FF415]` and jumps back to 0x5E8627). The SEH frame is
/// not reproduced; the body in order.
#[hook(addr = 0x5E8620, conv = thiscall, name = "CPed::Destructor", expect = "6A FF E9 45 8D E1 FF", this = "CPed")]
pub fn ped_destructor(this: GPtr<u8>) {
    let p = this;
    wr::<u32>(p, 0, VT_PED);
    dtor_notify_replay(p);
    if rd::<u8>(p, 0x477) & 1 != 0 {
        let idx = i32::from(rd::<i16>(p, 0x794));
        let m = g::<i16>(SCRIPT_MODEL_TABLE.wrapping_add((idx as usize).wrapping_mul(20))).read();
        set_mission_doesnt_require_model(i32::from(m) + 0x6676);
        let f = rd::<u32>(p, 0x474);
        wr::<u32>(p, 0x474, f & 0xFEFF_FFFF);
        script_release_ped_model(p, u32::from(rd::<u16>(p, 0x794)));
        wr::<u16>(p, 0x794, 0xFFFF);
    }
    world_remove_references(p);
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let idx = pool_index(pool, p, PED_SLOT);
    let fl = rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize).read();
    crate::ped::clear_blip_for_entity(2, i32::from(fl).wrapping_add(idx.wrapping_shl(8)));
    dtor_cleanup_43a960(p);
    let v = rd::<GPtr<u8>>(p, 0x58C);
    if !v.is_null() {
        crate::ped::clean_up_old_reference(v, p.byte_add(0x58C).cast());
    }
    wr::<u32>(p, 0x58C, 0);
    let fire = rd::<GPtr<u8>>(p, 0x730);
    if !fire.is_null() {
        fire_extinguish(fire);
    }
    let cp = rd::<GPtr<u8>>(p, 0x788);
    if !cp.is_null() {
        crate::ped::release_cover_point_for_ped(cp, p);
        wr::<u32>(p, 0x788, 0);
    }
    ped_dtor_weapons(p);
    if rd::<u8>(p, 0x472) & 1 != 0 {
        let c = g::<i32>(MISSION_PED_COUNT);
        c.write(c.read().wrapping_sub(1));
    }
    population_update_ped_count(p, 1);
    let speech = p.byte_add(0x294);
    speech_terminate(speech);
    let wa = p.byte_add(0x394);
    weapon_audio_terminate(wa);
    let pa = p.byte_add(0x138);
    ped_audio_terminate(pa);
    let intel = rd::<GPtr<u8>>(p, 0x47C);
    if !intel.is_null() {
        intel_dtor(intel);
        intel_delete(intel);
    }
    let look = rd::<GPtr<u8>>(p, 0x738);
    if !look.is_null() {
        crate::ped::clean_up_old_reference(look, p.byte_add(0x738).cast());
    }
    eh_vector_dtor(p.byte_add(0x5A0), 0x1C, 0xD, 0x4411A0);
    wr::<u32>(wa, 0, VT_AUDIO_ENTITY);
    audio_entity_member_dtor(wa.byte_add(8));
    wr::<u32>(speech, 0, VT_AUDIO_ENTITY);
    audio_entity_member_dtor(speech.byte_add(8));
    ped_audio_dtor(pa);
    physical_dtor(p);
}

/// `CPed::`scalar deleting destructor'(uint8 flags)` (0x5E8C80, `ret 4`): the destructor, then with bit 0
/// the pool slot freed inline (flag byte |= 0x80, first-free index lowered).
#[hook(addr = 0x5E8C80, conv = thiscall, name = "CPed::`scalar deleting destructor'", expect = "56 57 8B F9 E8 97 F9 FF FF", this = "CPed")]
pub fn ped_scalar_deleting_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    ped_dtor_call(this);
    if flags & 1 != 0 {
        let pool = g::<GPtr<u8>>(PED_POOL).read();
        let idx = pool_index(pool, this, PED_SLOT);
        let f = rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize);
        f.write(f.read() | 0x80);
        if idx < rd::<i32>(pool, 0xC) {
            wr::<i32>(pool, 0xC, idx);
        }
    }
    this
}

fn deleting(this: GPtr<u8>, flags: u32, dtor: fn(GPtr<u8>)) -> GPtr<u8> {
    dtor(this);
    if flags & 1 != 0 {
        ped_delete(this);
    }
    this
}

/// `CCivilianPed::~CCivilianPed()` (0x5DDBE0): vtable, then `CPed::~CPed` (tail jump).
#[hook(addr = 0x5DDBE0, conv = thiscall, name = "CCivilianPed::~CCivilianPed", expect = "C7 01 A8 C0 86 00", this = "CCivilianPed")]
pub fn civilian_ped_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, VT_CIVILIAN);
    ped_dtor_call(this);
}

/// `CCivilianPed::Destructor` (0x5DDC30, the scalar deleting destructor, `ret 4`).
#[hook(addr = 0x5DDC30, conv = thiscall, name = "CCivilianPed::Destructor", expect = "56 8B F1 E8 A8 FF FF FF", this = "CCivilianPed")]
pub fn civilian_ped_deleting_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    deleting(this, flags, civilian_dtor_call)
}

/// `CEmergencyPed::~CEmergencyPed()` (0x5DE3B0): vtable, then `CPed::~CPed` (tail jump).
#[hook(addr = 0x5DE3B0, conv = thiscall, name = "CEmergencyPed::~CEmergencyPed", expect = "C7 01 00 C2 86 00", this = "CEmergencyPed")]
pub fn emergency_ped_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, VT_EMERGENCY);
    ped_dtor_call(this);
}

/// `CEmergencyPed::`scalar deleting destructor'` (0x5DE3E0, `ret 4`).
#[hook(addr = 0x5DE3E0, conv = thiscall, name = "CEmergencyPed::`scalar deleting destructor'", expect = "56 8B F1 E8 C8 FF FF FF", this = "CEmergencyPed")]
pub fn emergency_ped_deleting_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    deleting(this, flags, emergency_dtor_call)
}

/// `CCopPed::Destructor` (0x5DE0D0, the complete-object destructor despite the catalogue name): vtable;
/// with a player ped, `FindPlayerWanted(-1)->RemoveCop(this)` (0x562C10); the criminal reference +0x7A8
/// cleaned up; 0x5DE070; `CPed::~CPed`. SEH frame not reproduced.
#[hook(addr = 0x5DE0D0, conv = thiscall, name = "CCopPed::Destructor", expect = "6A FF 68 A8 D7 83 00", this = "CCopPed")]
pub fn cop_ped_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0, VT_COP);
    if !find_player_ped(-1).is_null() {
        wanted_remove_cop(find_player_wanted(-1), this);
    }
    let c = rd::<GPtr<u8>>(this, 0x7A8);
    if !c.is_null() {
        crate::ped::clean_up_old_reference(c, this.byte_add(0x7A8).cast());
    }
    cop_clear_criminals(this);
    ped_dtor_call(this);
}

/// `CCopPed::`scalar deleting destructor'` (0x5DE2C0, `ret 4`).
#[hook(addr = 0x5DE2C0, conv = thiscall, name = "CCopPed::`scalar deleting destructor'", expect = "56 8B F1 E8 08 FE FF FF", this = "CCopPed")]
pub fn cop_ped_deleting_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    deleting(this, flags, cop_dtor_call)
}

/// Releases the anim block whose group index is in the player data dword at `off` (0x48 / 0x4C).
fn release_player_anim_block(this: GPtr<u8>, off: isize) {
    let d = rd::<GPtr<u8>>(this, 0x480);
    let k = rd::<u32>(d, off);
    if k != 0 {
        let groups = g::<GPtr<u8>>(ANIM_ASSOC_GROUPS).read();
        let blk = rd::<u32>(groups, (k.wrapping_mul(20)) as isize);
        remove_anim_block_ref((blk.wrapping_sub(ANIM_BLOCKS) as i32) >> 5);
        wr::<u32>(rd::<GPtr<u8>>(this, 0x480), off, 0);
    }
}

/// `CPlayerPed::~CPlayerPed()` (0x6093B0): vtable; the anim blocks of the player data's +0x48 / +0x4C groups
/// released (`(block - ms_aAnimBlocks) >> 5`, signed) and cleared; without flag 0x200000 of +0x478 the
/// player group (`player data +0x38`) removed (0x5FB870); `CPed::~CPed`. SEH frame not reproduced.
#[hook(addr = 0x6093B0, conv = thiscall, name = "CPlayerPed::~CPlayerPed", expect = "6A FF 68 18 EC 83 00", this = "CPlayerPed")]
pub fn player_ped_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, VT_PLAYER);
    release_player_anim_block(this, 0x48);
    release_player_anim_block(this, 0x4C);
    if rd::<u32>(this, 0x478) & 0x20_0000 == 0 {
        ped_groups_remove_group(rd::<u32>(rd::<GPtr<u8>>(this, 0x480), 0x38));
    }
    ped_dtor_call(this);
}

/// `CPlayerPed::`scalar deleting destructor'` (0x60A9A0, `ret 4`).
#[hook(addr = 0x60A9A0, conv = thiscall, name = "CPlayerPed::`scalar deleting destructor'", expect = "56 8B F1 E8 08 EA FF FF", this = "CPlayerPed")]
pub fn player_ped_deleting_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    deleting(this, flags, player_dtor_call)
}

/// `CCivilianPed::CCivilianPed(ePedType, uint32 modelIndex)` (0x5DDB70, `ret 8`): `CPed(type)`, vtable,
/// `SetModelIndex(model)`, `m_pedSpeech.Initialise(this)`. SEH frame not reproduced.
#[hook(addr = 0x5DDB70, conv = thiscall, name = "CCivilianPed::Constructor", expect = "6A FF 68 68 D7 83 00", this = "CCivilianPed")]
pub fn civilian_ped_ctor(this: GPtr<u8>, ped_type: u32, model: u32) -> GPtr<u8> {
    ped_ctor_call(this, ped_type);
    wr::<u32>(this, 0, VT_CIVILIAN);
    ped_set_model_index_call(this, model);
    speech_init(this.byte_add(0x294), this);
    this
}

/// `CCopPed::CCopPed(eCopType)` (0x5DDC60, `ret 4`): `CPed(PED_TYPE_COP = 6)`, vtable, +0x7A0 = type, then
/// per type (jump table 0x5DDE5C): 0 = `CStreaming::GetDefaultCopModel()`, 1 = 284, 7 = 283, 6 and > 7 =
/// the type value itself as the model (original), each with a nightstick (3, 1000) and a delayed pistol
/// (0x16, 1000), +0x718 = 0, armour 0, accuracy 30, +0x71A = 60; 2/3 = 285 with an MP5 (0x1C), armour
/// 50, 70, 68; 4 = 286 with an M4 (0x1D? 0x1D), armour 100, 60, 76; 5 = 287 with 0x1F, armour 100, 80, 84
/// (delayed weapon + `SetCurrentWeapon`). Then the common tail. SEH frame not reproduced.
#[hook(addr = 0x5DDC60, conv = thiscall, name = "CCopPed::Constructor", expect = "6A FF 68 88 D7 83 00", this = "CCopPed")]
pub fn cop_ped_ctor(this: GPtr<u8>, cop_type: u32) -> GPtr<u8> {
    let p = this;
    ped_ctor_call(p, 6);
    wr::<u32>(p, 0, VT_COP);
    wr::<u32>(p, 0x7A0, cop_type);
    let special = |model: u32, weapon: u32, armour: u32, acc: u8, b: u8| {
        ped_set_model_index_call(p, model);
        give_delayed_weapon(p, weapon, 1000);
        set_current_weapon(p, weapon as i32);
        wr::<u32>(p, 0x548, armour);
        wr::<u8>(p, 0x719, acc);
        wr::<u8>(p, 0x71A, b);
    };
    match cop_type {
        2 | 3 => special(0x11D, 0x1C, 0x4248_0000, 0x46, 0x44),
        4 => special(0x11E, 0x1D, 0x42C8_0000, 0x3C, 0x4C),
        5 => special(0x11F, 0x1F, 0x42C8_0000, 0x50, 0x54),
        _ => {
            let model = match cop_type {
                0 => get_default_cop_model(),
                1 => 0x11C,
                7 => 0x11B,
                t => t,
            };
            ped_set_model_index_call(p, model);
            give_weapon(p, 3, 1000, 1);
            give_delayed_weapon(p, 0x16, 1000);
            wr::<u8>(p, 0x718, 0);
            wr::<u32>(p, 0x548, 0);
            wr::<u8>(p, 0x719, 0x1E);
            wr::<u8>(p, 0x71A, 0x3C);
        }
    }
    let crim = rd::<GPtr<u8>>(p, 0x71C);
    wr::<u8>(p, 0x79C, 0);
    wr::<u32>(p, 0x74C, 0);
    wr::<u8>(p, 0x79D, 0);
    wr::<u32>(p, 0x7A4, 0);
    if !crim.is_null() {
        crate::ped::clean_up_old_reference(crim, p.byte_add(0x71C).cast());
    }
    wr::<u32>(p, 0x71C, 0);
    speech_init(p.byte_add(0x294), p);
    let intel = rd::<GPtr<u8>>(p, 0x47C);
    wr::<u32>(intel, 0xC8, 0x4270_0000);
    wr::<u32>(intel, 0xC4, 8);
    for k in 0..6 {
        wr::<u32>(p, 0x7A8 + 4 * k, 0);
    }
    wr::<u8>(p, 0x7C0, 0);
    p
}

/// `CPed::SetModelIndex(uint32)` (0x5E4880, `ret 4`): entity flag 0x80 of +0x1C, `CEntity::SetModelIndex`,
/// anim blend clump init and frame array (+0x488), ped stats from the model info's stats type (+0x2C,
/// `ms_apPedStats + n*0x34`, turn speed +0x560 = stats +0x20), decision maker (-2 for player types, -1
/// for created-by 2, else the stats' byte +0x32), +0x756 = `rand() % 50 + 20` when 0x610210(model, 1) or
/// (model, 6) holds else `rand() % 25`, overridden with 400 when `(rand() & 0xFFFF) * (1/32768) * 100`
/// truncates below 3 (x87); anim group +0x4D4 from the model info (+0x24) with `AddAnimation(clump, group,
/// 3)`; flag 2 of +0x528 for states 0x32/0x3A or flag 0x4000000 of +0x46C; the clump's anim data +0xC =
/// this + 0x4D8; the hit col model created when the model info has none; 0x532B20.
#[hook(addr = 0x5E4880, conv = thiscall, name = "CPed::SetModelIndex", expect = "56 8B F1 81 4E 1C 80 00 00 00", this = "CPed")]
pub fn ped_set_model_index(this: GPtr<u8>, model: u32) {
    let p = this;
    let f = rd::<u32>(p, 0x1C);
    wr::<u32>(p, 0x1C, f | 0x80);
    entity_set_model_index(p, model);
    anim_blend_clump_init(rd::<GPtr<u8>>(p, 0x18));
    anim_blend_clump_fill_frame_array(rd::<GPtr<u8>>(p, 0x18), p.byte_add(0x488));
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * (i32::from(rd::<i16>(p, 0x22)) as usize)).read();
    let stats = g::<GPtr<u8>>(PED_STATS)
        .read()
        .byte_add((rd::<i32>(mi, 0x2C).wrapping_mul(0x34)) as isize);
    wr::<GPtr<u8>>(p, 0x59C, stats);
    wr::<u32>(p, 0x560, rd::<u32>(stats, 0x20));
    let ty = rd::<u32>(p, 0x598);
    let dm = if ty == 0 || ty == 1 {
        -2
    } else if rd::<u8>(p, 0x484) == 2 {
        -1
    } else {
        i32::from(rd::<i8>(rd::<GPtr<u8>>(p, 0x59C), 0x32))
    };
    crate::ped::set_ped_decision_maker_type(rd::<GPtr<u8>>(p, 0x47C), dm);
    let m = i32::from(rd::<i16>(p, 0x22));
    let v = if model_test_610210(m, 1) != 0 || model_test_610210(i32::from(rd::<i16>(p, 0x22)), 6) != 0 {
        at_site(0x5E4948, rand) % 50 + 20
    } else {
        at_site(0x5E4939, rand) % 25
    };
    wr::<u16>(p, 0x756, v as u16);
    let r = at_site(0x5E495F, rand) & 0xFFFF;
    let t = xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(100.0);
    if ftol32(t) < 3 {
        wr::<u16>(p, 0x756, 0x190);
    }
    let group = rd::<u32>(mi, 0x24);
    wr::<u32>(p, 0x4D4, group);
    add_animation(rd::<GPtr<u8>>(p, 0x18), group, 3);
    let st = rd::<i32>(p, 0x530);
    if st == 0x32 || st == 0x3A || rd::<u32>(p, 0x46C) & 0x0400_0000 != 0 {
        let f = rd::<u32>(p, 0x528);
        wr::<u32>(p, 0x528, f | 2);
    }
    let clump = rd::<GPtr<u8>>(p, 0x18);
    let data = rd::<GPtr<u8>>(clump, g::<i32>(CLUMP_ANIM_OFFSET).read() as isize);
    wr::<GPtr<u8>>(data, 0xC, p.byte_add(0x4D8));
    if rd::<u32>(mi, 0x34) == 0 {
        create_hit_col_model_skinned(mi, rd::<GPtr<u8>>(p, 0x18));
    }
    entity_update_after_model(p);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_PED_POOL_NEW_AT,
    HOOK_PED_NEW_AT,
    HOOK_PED_INTELLIGENCE_NEW,
    HOOK_PED_CONSTRUCTOR,
    HOOK_PED_DESTRUCTOR,
    HOOK_PED_SCALAR_DELETING_DTOR,
    HOOK_CIVILIAN_PED_DTOR,
    HOOK_CIVILIAN_PED_DELETING_DTOR,
    HOOK_EMERGENCY_PED_DTOR,
    HOOK_EMERGENCY_PED_DELETING_DTOR,
    HOOK_COP_PED_DESTRUCTOR,
    HOOK_COP_PED_DELETING_DTOR,
    HOOK_PLAYER_PED_DTOR,
    HOOK_PLAYER_PED_DELETING_DTOR,
    HOOK_CIVILIAN_PED_CTOR,
    HOOK_COP_PED_CTOR,
    HOOK_PED_SET_MODEL_INDEX,
];
