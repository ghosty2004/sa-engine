//! `CRunningScript::ProcessCommands2100To2199` (0x470A90): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-4). Every opcode with a jump-table case is in [`PORTED`]; 0854 and 0865 reach the default (`or
//! al,0xFF` at 0x472158: -1), as does every command outside the range.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2100To2199.md`.
//!
//! Divergence notes (ledger classes of PORT-SCRIPT-2/3): text labels are read into zeroed buffers one byte
//! longer than the label (8-byte labels: 16 bytes); stack objects (0851's `CEventScriptCommand`, 085B's
//! `CTaskSimpleSetKindaStayInSamePlace`), entity lists and out-parameters start zeroed; bools the original
//! pushes as a dword whose low byte only was written are passed as 0/1 (exactly where the upper bytes are a
//! known operand). The SEH state stores are not ported.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0834, 0x0835, 0x0836, 0x0837, 0x0838, 0x0839, 0x083A, 0x083B, 0x083C, 0x083D, 0x083E, 0x083F, 0x0840,
    0x0841, 0x0842, 0x0843, 0x0844, 0x0845, 0x0846, 0x0847, 0x0848, 0x0849, 0x084A, 0x084B, 0x084C, 0x084D,
    0x084E, 0x084F, 0x0850, 0x0851, 0x0852, 0x0853, 0x0855, 0x0856, 0x0857, 0x0858, 0x0859, 0x085A, 0x085B,
    0x085C, 0x085D, 0x085E, 0x085F, 0x0860, 0x0861, 0x0862, 0x0863, 0x0864, 0x0866, 0x0867, 0x0868, 0x0869,
    0x086A, 0x086B, 0x086C, 0x086D, 0x086E, 0x086F, 0x0870, 0x0871, 0x0872, 0x0873, 0x0874, 0x0875, 0x0876,
    0x0877, 0x0878, 0x0879, 0x087A, 0x087B, 0x087C, 0x087D, 0x087E, 0x087F, 0x0880, 0x0881, 0x0882, 0x0883,
    0x0884, 0x0885, 0x0886, 0x0887, 0x0888, 0x0889, 0x088A, 0x088B, 0x088C, 0x088D, 0x088E, 0x088F, 0x0890,
    0x0891, 0x0892, 0x0893, 0x0894, 0x0895, 0x0896, 0x0897,
];

/// `CEntryExitManager::ms_entryExitStackPosn` 0x96A7C4 (owned by sa-world; written by address as the original does).
const ENTRY_EXIT_STACK_POSN: usize = 0x96A7C4;
/// `.rdata`: 0x858B38 0.02f, 0x858B40 50.0f, 0x8595EC 0.017453292f, 0x858B50 0.0f, 0x858B54 "" (a zero byte)
const K_0_02: u32 = 0x3CA3_D70A;
const K_50: u32 = 0x4248_0000;
const K_DEG2RAD: u32 = 0x3C8E_FA35;
const K_ZERO: u32 = 0;
const EMPTY_STR: usize = 0x858B54;
/// `g_interiorMan` 0xBAF670.
const INTERIOR_MAN: usize = 0xBAF670;

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0), AL
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CTimeCycle::StopExtraColour(bool)` (0x55FF20)
    fn stop_extra_colour(a: i32) = cdecl 0x55FF20 => unported;
    /// `CVehicle::SetTowLink(CVehicle*, bool)` through the vtable slot +0xF4, read at run time; AL discarded
    fn vcall_set_tow_link(f: u32, this: GPtr<u8>, c: GPtr<u8>, b: u32) = thiscall dyn => unported;
    /// `CRunningScript::ReadTextLabelFromScript(char*, uint8)` (0x463D50)
    fn read_label(s: Scr, buf: GPtr<u8>, len: u32) = thiscall 0x463D50 => crate::vm::read_text_label_from_script;
    /// `CRunningScript::GetIndexOfGlobalVariable()` (0x464700), AX
    fn global_index(s: Scr) -> u16 = thiscall 0x464700 => crate::vm::get_index_of_global_variable;
    /// `CCam::DoCamBump(float, float)` (0x50CB30); raw
    fn do_cam_bump(cam: GPtr<u8>, a: u32, b: u32) = thiscall 0x50CB30 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, const char*)` (0x4D6870)
    fn clump_get_association(clump: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D6870 => unported;
    /// `RpAnimBlendClumpIsInitialized(RpClump*)` (0x4D6760), AL
    fn clump_is_initialized(clump: GPtr<u8>) -> u8 = cdecl 0x4D6760 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80); raw
    fn set_current_time(a: GPtr<u8>, t: u32) = thiscall 0x4CEA80 => unported;
    /// 0x441130 (thiscall on a `CPed`, three floats: sets the move speed); raw
    fn set_ped_velocity(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x441130 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280) / `CWorld::Add(CEntity*)` (0x563220)
    fn world_remove_(e: GPtr<u8>) = cdecl 0x563280 => unported;
    fn world_add_(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80); raw
    fn set_orientation(e: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x439A80 => unported;
    /// `CTheZones::FindSmallestZoneForPosition(const CVector&, bool)` (0x572360)
    fn find_smallest_zone_for_position(pos: GPtr<u32>, b: u32) -> GPtr<u8> = cdecl 0x572360 => unported;
    /// CRT `strncpy` (0x821F40)
    fn strncpy_(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `InteriorManager_c::ActivatePeds(bool)` (0x598080) / `IsGroupActive(int32)` (0x598280, AL)
    fn interior_activate_peds(m: GPtr<u8>, b: u32) = thiscall 0x598080 => unported;
    fn interior_is_group_active(m: GPtr<u8>, g: u32) -> u8 = thiscall 0x598280 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 pedHandle, CTask*, int32 opcode)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// `CTaskComplexFollowPedFootsteps::CTaskComplexFollowPedFootsteps(CPed*)` (0x694E20)
    fn task_follow_footsteps_ctor(t: GPtr<u8>, p: GPtr<u8>) -> GPtr<u8> = thiscall 0x694E20 => unported;
    /// `CTaskComplexDie::CTaskComplexDie(...)` (0x630040), 9 dwords; floats raw
    #[allow(clippy::too_many_arguments)]
    fn task_die_ctor(t: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32, h: u32, i: u32) -> GPtr<u8> = thiscall 0x630040 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32 primaryTaskIndex, CTask*, bool)` (0x4B0A00) and its
    /// destructor 0x4B0A50
    fn event_script_command_ctor(e: GPtr<u8>, idx: u32, task: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGlobalGroup::GetSoundLevel(CEntity*, CVector&)` (0x4AB900), ST0 (read as f64)
    fn get_sound_level(g: GPtr<u8>, e: GPtr<u8>, pos: GPtr<u32>) -> f64 = thiscall 0x4AB900 => unported;
    /// `CTaskComplexWalkAlongsidePed::CTaskComplexWalkAlongsidePed(CPed*, float)` (0x683240); raw
    fn task_walk_alongside_ctor(t: GPtr<u8>, p: GPtr<u8>, d: u32) -> GPtr<u8> = thiscall 0x683240 => unported;
    /// `CCarCtrl::ScriptGenerateOneEmergencyServicesCar(uint32 model, CVector)` (0x42FBC0), AL; vector raw
    fn script_generate_emergency_car(m: u32, x: u32, y: u32, z: u32) -> u8 = cdecl 0x42FBC0 => unported;
    /// `CTaskSimpleSetKindaStayInSamePlace` constructor (0x62F610, bool), `ProcessPed(CPed*)` (0x62F660) and
    /// destructor (0x62F650)
    fn task_kinda_stay_ctor(t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x62F610 => unported;
    fn task_kinda_stay_process_ped(t: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x62F660 => unported;
    fn task_kinda_stay_dtor(t: GPtr<u8>) = thiscall 0x62F650 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10)
    fn task_sequence_add(seq: GPtr<u8>, t: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CVehicleRecording::StartPlaybackRecordedCar(CVehicle*, int32, bool, bool)` (0x45A980)
    fn start_playback_recorded_car(v: GPtr<u8>, i: u32, ai: u32, looped: u32) = cdecl 0x45A980 => unported;
    /// `CEntryExitManager::FindNearestEntryExit(const CVector2D&, float, int32)` (0x43F4B0)
    fn find_nearest_entry_exit(pos: GPtr<u32>, range: u32, ignore: i32) -> i32 = cdecl 0x43F4B0 => unported;
    /// The entry-exit slot accessor at 0x43EF00 (cdecl (index) -> CEntryExit*; a `.HOODLUM` thunk)
    fn entry_exit_at(i: i32) -> GPtr<u8> = cdecl 0x43EF00 => unported;
    /// `CWorld::FindObjectsInRange(const CVector&, float, bool, int16*, int16, CEntity**, bool x5)` (0x564A20)
    #[allow(clippy::too_many_arguments)]
    fn find_objects_in_range(pos: GPtr<u32>, r: u32, two_d: u32, count: GPtr<u16>, max: u32, out: GPtr<u32>, b: u32, v: u32, p: u32, o: u32, d: u32) = cdecl 0x564A20 => unported;
    /// `CWorld::FindObjectsOfTypeInRange(uint32, const CVector&, float, bool, int16*, int16, CEntity**, bool x5)`
    /// (0x564C70)
    #[allow(clippy::too_many_arguments)]
    fn find_objects_of_type_in_range(model: u32, pos: GPtr<u32>, r: u32, two_d: u32, count: GPtr<u16>, max: u32, out: GPtr<u32>, b: u32, v: u32, p: u32, o: u32, d: u32) = cdecl 0x564C70 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0 (an `fsqrt` result under PC=24: exact as f64)
    fn vector_magnitude(v: GPtr<u32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CPool<CObject>::GetRef(CObject*)` with the raw result (0x465070)
    fn object_get_ref_(pool: GPtr<u8>, o: GPtr<u8>) -> u32 = thiscall 0x465070 => unported;
    /// `CTheScripts::AddToSwitchJumpTable(int32 value, int32 address)` (0x470390)
    fn add_to_switch_jump_table(v: u32, a: u32) = cdecl 0x470390 => unported;
    /// `CTheScripts::UseSwitchJumpTable(int32* out)` (0x4703C0; writes 0 first, then the target when found)
    fn use_switch_jump_table(out: GPtr<u32>) = cdecl 0x4703C0 => unported;
    /// `CVehicleRecording::RemoveRecordingFile(int32)` (0x45A0A0)
    fn remove_recording_file(i: u32) = cdecl 0x45A0A0 => unported;
    /// `CTheZones::FindZoneByLabel(const char*, eZoneType)` (0x572C40), AX
    fn find_zone_by_label(label: GPtr<u8>, ty: u32) -> u16 = cdecl 0x572C40 => unported;
    /// `CBirds::CreateNumberOfBirds(CVector, CVector, int32, int32, bool)` (0x711EF0); vectors raw
    #[allow(clippy::too_many_arguments)]
    fn create_number_of_birds(ax: u32, ay: u32, az: u32, bx: u32, by: u32, bz: u32, n: u32, ty: u32, c: u32) = cdecl 0x711EF0 => unported;
    /// `CGangWars::SetGangWarsActive(bool)` (0x446570) / `GangWarGoingOn()` (0x443AA0, AL)
    fn set_gang_wars_active(b: u32) = cdecl 0x446570 => unported;
    fn gang_war_going_on() -> u8 = cdecl 0x443AA0 => unported;
    /// `CPedClothesDesc::SetTextureAndModel(const char* tex, const char* model, int32)` (0x5A8080)
    fn set_texture_and_model_names(d: GPtr<u8>, tex: GPtr<u8>, model: GPtr<u8>, ty: u32) = thiscall 0x5A8080 => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
    /// `CShopping::SetPlayerHasBought(uint32)` (0x49B610) / `RemoveLoadedShop()` (0x49AE30)
    fn set_player_has_bought(k: u32) = cdecl 0x49B610 => unported;
    fn remove_loaded_shop() = cdecl 0x49AE30 => unported;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: u32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// 0x53FB70 (cdecl `(int32)`: `CPad::GetPad`)
    fn get_pad(i: u32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `FxSystem_c::AttachToBone(CEntity*, int32)` (0x4AA400)
    fn fx_attach_to_bone(fx: GPtr<u8>, e: GPtr<u8>, bone: u32) = thiscall 0x4AA400 => unported;
    /// `CStreamedScripts::GetProperIndexFromIndexUsedByScript(int16)` (0x470810), AX
    fn get_proper_index(m: GPtr<u8>, i: u32) -> u16 = thiscall 0x470810 => unported;
    /// `CScriptsForBrains::AddNewStreamedScriptBrainForCodeUse(int16, const char*, int8)` (0x46A9C0)
    fn add_brain_for_code_use(m: GPtr<u8>, i: u32, name: GPtr<u8>, ty: u32) = thiscall 0x46A9C0 => unported;
    /// `CRadar::SetEntityBlip(eBlipType, int32, uint32, eBlipDisplay, const char*)` (0x5839A0)
    fn set_entity_blip(ty: u32, h: u32, colour: u32, display: u32, name: GPtr<u8>) -> u32 = cdecl 0x5839A0 => unported;
    /// `CRadar::ChangeBlipScale(int32, int32)` (0x583CC0)
    fn change_blip_scale(b: u32, s: u32) = cdecl 0x583CC0 => unported;
    /// `CPed::GetBonePosition(RwV3d&, uint32 bone, bool)` (0x5E4280)
    fn get_bone_position(p: GPtr<u8>, out: GPtr<u32>, bone: u32, b: u32) = thiscall 0x5E4280 => unported;
    /// `CRunningScript::PlayAnimScriptCommand(int32 command)` (0x470150; reads its own parameters)
    fn play_anim_script_command(s: Scr, command: u32) = thiscall 0x470150 => unported;
    /// `CCarCtrl::SetCoordsOfScriptCar(CVehicle*, float, float, float, bool, bool)` (0x4342A0); raw
    fn set_coords_of_script_car(v: GPtr<u8>, x: u32, y: u32, z: u32, a: u32, b: u32) = cdecl 0x4342A0 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), ST0 (read as f64: exact); floats raw
    fn find_ground_z(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CClock::OffsetClockByADay(uint32)` (0x52D0B0)
    fn offset_clock_by_a_day(n: u32) = cdecl 0x52D0B0 => unported;
    /// `COnscreenTimer::SetClockBeepCountdownSecs(uint32 varId, uint32 secs)` (0x44CEE0)
    fn set_clock_beep_countdown_secs(t: GPtr<u8>, id: u32, secs: u32) = thiscall 0x44CEE0 => unported;
    /// `CPhysical::GetHasCollidedWith(CEntity*)` (0x543540), AL
    fn get_has_collided_with(p: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x543540 => unported;
}

/// `VEH(h)` / `PED(h)` / `OBJ(h)` (GetAtRef; no null check by the callers unless stated).
#[inline]
fn veh(h: u32) -> GPtr<u8> {
    vehicle_at_ref(pool(VEHICLE_POOL), h as i32)
}
#[inline]
fn ped(h: u32) -> GPtr<u8> {
    ped_at_ref(pool(PED_POOL), h as i32)
}
#[inline]
fn obj(h: u32) -> GPtr<u8> {
    object_at_ref(pool(OBJECT_POOL), h as i32)
}

/// The entity position: `m_matrix (+0x14) ? m_matrix + 0x30 : this + 4`.
fn ent_pos(e: GPtr<u8>) -> GPtr<u32> {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
    .cast::<u32>()
}

/// `task` or null when `operator new` failed (the constructor runs only on a non-null block).
#[inline]
fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() {
        GPtr::NULL
    } else {
        ctor(t)
    }
}

/// `dword = v != 0 ? d | bit : d & !bit` on `p + off`.
#[inline]
fn set_bit(p: GPtr<u8>, off: isize, bit: u32, on: bool) {
    let f = p.byte_add(off).cast::<u32>();
    let v = f.read();
    f.write(if on { v | bit } else { v & !bit });
}

/// The object animation cases' lookup (0836/0837/0839/083A): `o = OBJ(P0)` (P0 already collected), 24-byte
/// anim label (zeroed 25-byte buffer); returns `(o, label)`.
fn object_anim_label(s: Scr, label: &mut [u8; 25]) -> (GPtr<u8>, GPtr<u8>) {
    let o = obj(sp(0));
    let lp = GPtr::from_raw(label.as_mut_ptr());
    read_label(s, lp, 0x18);
    (o, lp)
}

/// The distance of `e`'s position to `(x, y, z)` (components `fsub`, stored f32, then `CVector::Magnitude`),
/// as the ST0 value.
fn distance_to(e: GPtr<u8>, x: f32, y: f32, z: f32) -> X87 {
    let p = ent_pos(e).cast::<f32>();
    let mut d = [
        X87::from_f32(p.read()).fsub_m(x).to_f32().to_bits(),
        X87::from_f32(p.byte_add(4).read()).fsub_m(y).to_f32().to_bits(),
        X87::from_f32(p.byte_add(8).read()).fsub_m(z).to_f32().to_bits(),
    ];
    X87::from_f64_exact(vector_magnitude(GPtr::from_raw(d.as_mut_ptr())))
}

/// `CRunningScript::ProcessCommands2100To2199(int32 command)` (0x470A90, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x470A90, conv = thiscall, name = "CRunningScript::ProcessCommands2100To2199", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_2100_to_2199(s: Scr, command: u32) -> i8 {
    match command {
        // 0838 083B 0848..084C 084F 0857 085C 085D 085F 0862 0863 086A 086D..0870 0877 0880 0882 0885 0886 088F
        // 0891 0892 0894..0896: the jump table sends them to 0x472154 (`xor al,al`: no operand read, return 0).
        0x0838 | 0x083B | 0x0848..=0x084C | 0x084F | 0x0857 | 0x085C | 0x085D | 0x085F | 0x0862 | 0x0863 => {}
        0x086A | 0x086D..=0x0870 | 0x0877 | 0x0880 | 0x0882 | 0x0885 | 0x0886 | 0x088F | 0x0891 | 0x0892 => {}
        0x0894..=0x0896 => {}
        // 0868 0869 086B 086C (gr: nop) (0x47175E): `GetIndexOfGlobalVariable()` consumes one operand; return 0.
        0x0868 | 0x0869 | 0x086B | 0x086C => {
            global_index(s);
        }
        // 0834 DO_CAMERA_BUMP (0x470ACE): `TheCamera.m_aCams[(u8)[0xB6F081]]` (0xB6F19C + 0x238 * i)
        // `.DoCamBump(P0, P1)`.
        0x0834 => {
            collect(s, 2);
            let (a, b) = (sp(0), sp(1));
            let i = u32::from(GPtr::<u8>::from_addr(0xB6F081).read());
            do_cam_bump(GPtr::from_addr((0xB6F19C + i * 0x238) as usize), a, b);
        }
        // 0835 GET_CURRENT_DATE (0x470B03): P0 = (u8)[0xB70154], P1 = (u8)[0xB70155]; store 2.
        0x0835 => {
            set_sp(0, u32::from(GPtr::<u8>::from_addr(0xB70154).read()));
            set_sp(1, u32::from(GPtr::<u8>::from_addr(0xB70155).read()));
            store(s, 2);
        }
        // 0836 SET_OBJECT_ANIM_SPEED (0x470B2C): `a = RpAnimBlendClumpGetAssociation(o+0x18, label)`;
        // `CollectParameters(1)`; `a` -> a+0x24 = P0 (raw).
        0x0836 => {
            collect(s, 1);
            let mut label = [0u8; 25];
            let (o, lp) = object_anim_label(s, &mut label);
            let a = clump_get_association(o.byte_add(0x18).cast::<GPtr<u8>>().read(), lp);
            collect(s, 1);
            if !a.is_null() {
                a.byte_add(0x24).cast::<u32>().write(sp(0));
            }
        }
        // 0837 IS_OBJECT_PLAYING_ANIM (0x470B91): flag = clump (o+0x18) non-null, its type byte == 2,
        // `RpAnimBlendClumpIsInitialized` and `GetAssociation(clump, label)` non-null.
        0x0837 => {
            collect(s, 1);
            let mut label = [0u8; 25];
            let (o, lp) = object_anim_label(s, &mut label);
            let c = o.byte_add(0x18).cast::<GPtr<u8>>();
            let r = !c.read().is_null()
                && c.read().read() == 2
                && clump_is_initialized(c.read()) != 0
                && !clump_get_association(c.read(), lp).is_null();
            set_flag(s, r);
        }
        // 0839 GET_OBJECT_ANIM_CURRENT_TIME (0x470C0C): P0 = `a ? a+0x20 / a->hier (+0x14)+0x10 : 0` (x87, f32);
        // store 1.
        0x0839 => {
            collect(s, 1);
            let mut label = [0u8; 25];
            let (o, lp) = object_anim_label(s, &mut label);
            let a = clump_get_association(o.byte_add(0x18).cast::<GPtr<u8>>().read(), lp);
            let mut v = 0u32;
            if !a.is_null() {
                let h = a.byte_add(0x14).cast::<GPtr<f32>>().read();
                let t = a.byte_add(0x20).cast::<f32>().read();
                v = X87::from_f32(t)
                    .fdiv_m(h.byte_add(0x10).read())
                    .to_f32()
                    .to_bits();
            }
            set_sp(0, v);
            store(s, 1);
        }
        // 083A SET_OBJECT_ANIM_CURRENT_TIME (0x470C81): `a`; `CollectParameters(1)`; `a` ->
        // `a->SetCurrentTime(hier+0x10 * P0)`.
        0x083A => {
            collect(s, 1);
            let mut label = [0u8; 25];
            let (o, lp) = object_anim_label(s, &mut label);
            let a = clump_get_association(o.byte_add(0x18).cast::<GPtr<u8>>().read(), lp);
            collect(s, 1);
            if !a.is_null() {
                let h = a.byte_add(0x14).cast::<GPtr<f32>>().read();
                let t = X87::from_f32(h.byte_add(0x10).read()).fmul_m(spf(0)).to_f32();
                set_current_time(a, t.to_bits());
            }
        }
        // 083C SET_CHAR_VELOCITY (0x470CF4): `0x441130(PED(P0), P1 * 0.02f, P2 * 0.02f, P3 * 0.02f)`.
        0x083C => {
            collect(s, 4);
            let k = |i: i32| X87::from_f32(spf(i)).fmul_m(f(K_0_02)).to_f32().to_bits();
            let (z, y, x) = (k(3), k(2), k(1));
            set_ped_velocity(ped(sp(0)), x, y, z);
        }
        // 083D GET_CHAR_VELOCITY (0x470D4F): P0..P2 = `PED(P0)->m_vecMoveSpeed (+0x44) * 50.0f`; store 3.
        0x083D => {
            collect(s, 1);
            let p = ped(sp(0)).byte_add(0x44).cast::<f32>();
            for i in 0..3 {
                let v = X87::from_f32(p.byte_add(4 * i as isize).read())
                    .fmul_m(f(K_50))
                    .to_f32();
                set_sp(i, v.to_bits());
            }
            store(s, 3);
        }
        // 083E SET_CHAR_ROTATION (0x470DAA): `p = PED(P0)`; `CWorld::Remove(p)`; `p->SetOrientation(P1, P2, P3 in
        // radians)`; `CWorld::Add(p)`.
        0x083E => {
            collect(s, 4);
            let p = ped(sp(0));
            world_remove_(p);
            let k = |i: i32| X87::from_f32(spf(i)).fmul_m(f(K_DEG2RAD)).to_f32().to_bits();
            let (z, y, x) = (k(3), k(2), k(1));
            set_orientation(p, x, y, z);
            world_add_(p);
        }
        // 083F GET_CAR_UPRIGHT_VALUE (0x470E15): P0 = `VEH(P0)->m_matrix->up.z` (+0x28, raw); store 1.
        0x083F => {
            collect(s, 1);
            let m = veh(sp(0)).byte_add(0x14).cast::<GPtr<u32>>().read();
            set_sp(0, m.byte_add(0x28).read());
            store(s, 1);
        }
        // 0840 SET_VEHICLE_AREA_VISIBLE (0x470E5B): `VEH(SP0)->m_nAreaCode (+0x2F) = (u8)SP1` (no null check)
        0x0840 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            v.byte_add(0x2F).write(sp(1) as u8);
        }
        // 0841 SELECT_WEAPONS_FOR_VEHICLE (0x470E86): byte `VEH(P0)+0x513` = (u8)P1.
        0x0841 => {
            collect(s, 2);
            let v = veh(sp(0));
            v.byte_add(0x513).write(sp(1) as u8);
        }
        // 0842 GET_CITY_PLAYER_IS_IN (0x470EB4): operand read and ignored; P0 = `CTheZones::m_CurrLevel` (0xBA6718).
        0x0842 => {
            collect(s, 1);
            set_sp(0, GPtr::<u32>::from_addr(0xBA6718).read());
            store(s, 1);
        }
        // 0843 GET_NAME_OF_ZONE (0x470ED9): `z = FindSmallestZoneForPosition(&(P0, P1, P2), true)`;
        // `strncpy(GetPointerToScriptVariable(2), z+8, 8)`.
        0x0843 => {
            collect(s, 3);
            let mut p = [sp(0), sp(1), sp(2)];
            let z = find_smallest_zone_for_position(GPtr::from_raw(p.as_mut_ptr()), 1);
            let d = var_ptr(s, 2);
            strncpy_(d.cast(), z.byte_add(8), 8);
        }
        // 0844/0846 IS_VAR_TEXT_LABEL(16)_EMPTY (0x470F3B) / 0845/0847 IS_LVAR_... (0x470F73): `p =
        // GetPointerToScriptVariable(2 / 1)`; flag = `p[0] == [0x858B54]` (one-byte `repe cmpsb`).
        0x0844..=0x0847 => {
            let p = var_ptr(s, if command & 1 == 0 { 2 } else { 1 }).cast::<u8>();
            set_flag(s, p.read() == GPtr::<u8>::from_addr(EMPTY_STR).read());
        }
        // 084D ACTIVATE_INTERIOR_PEDS (0x470F77): `g_interiorMan.ActivatePeds(P0 != 0)`.
        0x084D => {
            collect(s, 1);
            interior_activate_peds(GPtr::from_addr(INTERIOR_MAN), u32::from(sp(0) != 0));
        }
        // 084E SET_VEHICLE_CAN_BE_TARGETTED (0x470FAA): byte `VEH(P0)+0x42D` bit 2 = P1 != 0.
        0x084E => {
            collect(s, 2);
            let b = veh(sp(0)).byte_add(0x42D);
            let c = b.read();
            b.write(if sp(1) != 0 { c | 4 } else { c & 0xFB });
        }
        // 0850 TASK_FOLLOW_FOOTSTEPS (0x470FF5): task (0x20) `CTaskComplexFollowPedFootsteps(PED(P1))`;
        // `GivePedScriptedTask(P0, ...)` (P0 read after the constructor).
        0x0850 => {
            collect(s, 2);
            let target = ped(sp(1));
            let t = new_task(0x20, |t| task_follow_footsteps_ctor(t, target));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 0851 DAMAGE_CHAR (0x47105C)
        0x0851 => damage_char(s),
        // 0852 SET_CAR_CAN_BE_VISIBLY_DAMAGED (0x471161): byte `VEH(P0)+0x868` bit 4 = P1 != 0.
        0x0852 => {
            collect(s, 2);
            let b = veh(sp(0)).byte_add(0x868);
            let c = b.read();
            b.write(if sp(1) != 0 { c | 0x10 } else { c & 0xEF });
        }
        // 0853 SET_HELI_REACHED_TARGET_DISTANCE (0x4711AB): byte `VEH(P0)+0x3DF` = (u8)P1.
        0x0853 => {
            collect(s, 2);
            let v = veh(sp(0));
            v.byte_add(0x3DF).write(sp(1) as u8);
        }
        // 0855 GET_SOUND_LEVEL_AT_COORDS (0x4711D9): `p = P0 == -1 ? 0 : PED(P0)`; P0 =
        // `(float)GetEventGlobalGroup()->GetSoundLevel(p, &(P1, P2, P3))`; store 1.
        0x0855 => {
            collect(s, 4);
            let p = if spi(0) == -1 { GPtr::NULL } else { ped(sp(0)) };
            let mut v = [sp(1), sp(2), sp(3)];
            let g = get_event_global_group();
            let l = get_sound_level(g, p, GPtr::from_raw(v.as_mut_ptr()));
            set_sp(0, X87::from_f64_exact(l).to_f32().to_bits());
            store(s, 1);
        }
        // 0856 SET_CHAR_ALLOWED_TO_DUCK (0x47125A): `PED(P0)` +0x46C bit 24 = P1 == 0.
        0x0856 => {
            collect(s, 2);
            let p = ped(sp(0));
            set_bit(p, 0x46C, 0x100_0000, sp(1) == 0);
        }
        // 0858 SET_HEADING_FOR_ATTACHED_PLAYER (0x4712AB): [0xA4449C] = P1 * 0.017453292, [0xA44498] = P2 *
        // 0.017453292 (P0 unused).
        0x0858 => {
            collect(s, 3);
            let a = X87::from_f32(spf(1)).fmul_m(f(K_DEG2RAD)).to_f32();
            GPtr::<f32>::from_addr(0xA4449C).write(a);
            let b = X87::from_f32(spf(2)).fmul_m(f(K_DEG2RAD)).to_f32();
            GPtr::<f32>::from_addr(0xA44498).write(b);
        }
        // 0859 TASK_WALK_ALONGSIDE_CHAR (0x4712DF): task (0x20) `CTaskComplexWalkAlongsidePed(PED(P1), 10.0f)`;
        // `GivePedScriptedTask(P0, ...)` (P0 read after the constructor).
        0x0859 => {
            collect(s, 2);
            let target = ped(sp(1));
            let t = new_task(0x20, |t| task_walk_alongside_ctor(t, target, 0x4120_0000));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 085A CREATE_EMERGENCY_SERVICES_CAR (0x47134B): flag = `ScriptGenerateOneEmergencyServicesCar(P0, (P1, P2,
        // P3)) != 0`.
        0x085A => {
            collect(s, 4);
            let r = script_generate_emergency_car(sp(0), sp(1), sp(2), sp(3));
            set_flag(s, r != 0);
        }
        // 085B TASK_KINDA_STAY_IN_SAME_PLACE (0x4713B5): `P0 != -1`: a stack
        // `CTaskSimpleSetKindaStayInSamePlace(P1 != 0)` (zeroed 0x10-byte buffer), `ProcessPed(PED(P0))` (not
        // null-checked), destructor. Else `task = new(0xC) (P1 != 0)` (the pushed dword is `setne dl` over EDX =
        // P1: `(P1 & 0xFFFFFF00) | b`), `ms_taskSequence[[0x8D2E98]] (0xC178F0 + 0x40 * i).AddTask(task)`.
        0x085B => {
            collect(s, 2);
            if spi(0) != -1 {
                let p = ped(sp(0));
                let b = u32::from(sp(1) != 0);
                let mut buf = [0u32; 4];
                let t = GPtr::from_raw(buf.as_mut_ptr()).cast::<u8>();
                task_kinda_stay_ctor(t, b);
                task_kinda_stay_process_ped(t, p);
                task_kinda_stay_dtor(t);
            } else {
                let t = new_task(0xC, |t| {
                    let p1 = sp(1);
                    task_kinda_stay_ctor(t, (p1 & 0xFFFF_FF00) | u32::from(p1 != 0))
                });
                let i = GPtr::<u32>::from_addr(0x8D2E98).read();
                task_sequence_add(GPtr::from_addr((i << 6).wrapping_add(0xC178F0) as usize), t);
            }
        }
        // 085E START_PLAYBACK_RECORDED_CAR_LOOPED (0x471470): `StartPlaybackRecordedCar(VEH(P0), P1, false, true)`.
        0x085E => {
            collect(s, 2);
            let p1 = sp(1);
            start_playback_recorded_car(veh(sp(0)), p1, 0, 1);
        }
        // 0860 SET_CHAR_AREA_VISIBLE (0x4714A5): `p = PED(SP0)` (no null check); `+0x2F = (u8)SP1`; if `SP1 == 0`:
        // `+0x78C = 0`, and if `p->IsPlayer()`: `CEntryExitManager::ms_entryExitStackPosn = 0`, `CTimeCycle::StopExtraColour(false)`.
        0x0860 => {
            collect(s, 2);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            p.byte_add(0x2F).write(sp(1) as u8);
            if sp(1) == 0 {
                p.byte_add(0x78C).cast::<u32>().write(0);
                if ped_is_player(p) != 0 {
                    GPtr::<u32>::from_addr(ENTRY_EXIT_STACK_POSN).write(0);
                    stop_extra_colour(0);
                }
            }
        }
        // 0861 IS_ATTACHED_PLAYER_HEADING_ACHIEVED (0x471504): operand ignored; flag = `[0xA44498] == 0.0f` (`test
        // ah,0x44; jp`: ordered equality).
        0x0861 => {
            collect(s, 1);
            let v = GPtr::<f32>::from_addr(0xA44498).read();
            set_flag(s, X87::from_f32(v).fcom(X87::from_f32(f(K_ZERO))) == Cmp::Equal);
        }
        // 0864 ENABLE_ENTRY_EXIT_PLAYER_GROUP_WARPING (0x47152E): `ee = 0x43EF00(FindNearestEntryExit(&(P0, P1), P2,
        // -1))` (not null-checked); P3 != 0: word +0x30 `|= 0x100` on ee and on `ee->m_pLink (+0x38)` when non-null;
        // else `&= 0xFEFF`.
        0x0864 => {
            collect(s, 4);
            let mut p = [sp(0), sp(1)];
            let i = find_nearest_entry_exit(GPtr::from_raw(p.as_mut_ptr()), sp(2), -1);
            let ee = entry_exit_at(i);
            let on = sp(3) != 0;
            let upd = |e: GPtr<u8>| {
                let w = e.byte_add(0x30).cast::<u16>();
                w.write(if on { w.read() | 0x100 } else { w.read() & 0xFEFF });
            };
            upd(ee);
            let link = ee.byte_add(0x38).cast::<GPtr<u8>>().read();
            if !link.is_null() {
                upd(link);
            }
        }
        // 0866 GET_CLOSEST_STEALABLE_OBJECT (0x4715B8)
        0x0866 => closest_stealable_object(s),
        // 0867 IS_PROCEDURAL_INTERIOR_ACTIVE (0x47173A): flag = `g_interiorMan.IsGroupActive(P0) != 0`.
        0x0867 => {
            collect(s, 1);
            set_flag(
                s,
                interior_is_group_active(GPtr::from_addr(INTERIOR_MAN), sp(0)) != 0,
            );
        }
        // 0871 SWITCH_START (0x47176C) / 0872 SWITCH_CONTINUED
        0x0871 | 0x0872 => switch_start(s, command == 0x0871),
        // 0873 REMOVE_CAR_RECORDING (0x47188A): `CVehicleRecording::RemoveRecordingFile(P0)`.
        0x0873 => {
            collect(s, 1);
            remove_recording_file(sp(0));
        }
        // 0874 SET_ZONE_POPULATION_RACE (0x4718A9): 8-byte label (zeroed 16-byte buffer), `CollectParameters(1)`, `z
        // = FindZoneByLabel(label, 2)`; `z >= 0`: byte `0xBA1E00 + 17 * (u16)[0xBA37B4 + 32 * z]` = `b ^ ((b ^ P0) &
        // 0xF)`.
        0x0874 => {
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_label(s, lp, 8);
            collect(s, 1);
            let z = find_zone_by_label(lp, 2) as i16;
            if z >= 0 {
                let navi =
                    GPtr::<u16>::from_addr(((i32::from(z) << 5) as u32).wrapping_add(0xBA37B4) as usize)
                        .read();
                let b = GPtr::<u8>::from_addr((u32::from(navi) * 0x11 + 0xBA1E00) as usize);
                let c = b.read();
                b.write(c ^ ((c ^ sp(0) as u8) & 0xF));
            }
        }
        // 0875 SET_OBJECT_ONLY_DAMAGED_BY_PLAYER (0x471916): `OBJ(P0)` +0x40 bit 22 = P1 != 0.
        0x0875 => {
            collect(s, 2);
            let o = obj(sp(0));
            set_bit(o, 0x40, 0x40_0000, sp(1) != 0);
        }
        // 0876 CREATE_BIRDS (0x47195E): `CBirds::CreateNumberOfBirds(((float)P0, (float)P1, (float)P2), ((float)P3,
        // (float)P4, (float)P5), P6, P7, false)` (the coordinates `fild`: read as integers, as compiled).
        0x0876 => {
            collect(s, 8);
            let c = |i: i32| X87::from_i32(spi(i)).to_f32().to_bits();
            create_number_of_birds(c(0), c(1), c(2), c(3), c(4), c(5), sp(6), sp(7), 0);
        }
        // 0878 SET_VEHICLE_DIRT_LEVEL (0x471A00): `VEH(P0)+0x4B0` = P1 (raw).
        0x0878 => {
            collect(s, 2);
            let v = veh(sp(0));
            v.byte_add(0x4B0).cast::<u32>().write(sp(1));
        }
        // 0879 SET_GANG_WARS_ACTIVE (0x471A2D): `CGangWars::SetGangWarsActive(P0 != 0)`.
        0x0879 => {
            collect(s, 1);
            set_gang_wars_active(u32::from(sp(0) != 0));
        }
        // 087A IS_GANG_WAR_GOING_ON (0x471A61): flag = `CGangWars::GangWarGoingOn() != 0`.
        0x087A => set_flag(s, gang_war_going_on() != 0),
        // 087B GIVE_PLAYER_CLOTHES_OUTSIDE_SHOP (0x471A70): `info = Players[P0]`; 16-byte tex and model labels
        // (zeroed 17-byte buffers); `CollectParameters(1)`; `info.m_pPedClothesDesc (+8)->SetTextureAndModel(tex,
        // model, P0)`; `CShopping::SetPlayerHasBought(CKeyGen::GetUppercaseKey(tex))`.
        0x087B => {
            collect(s, 1);
            let info = player_info(sp(0));
            let mut tex = [0u8; 17];
            let mut model = [0u8; 17];
            let tp = GPtr::from_raw(tex.as_mut_ptr());
            let mp = GPtr::from_raw(model.as_mut_ptr());
            read_label(s, tp, 0x10);
            read_label(s, mp, 0x10);
            collect(s, 1);
            let d = info.byte_add(8).cast::<GPtr<u8>>().read();
            set_texture_and_model_names(d, tp, mp, sp(0));
            set_player_has_bought(get_uppercase_key(tp));
        }
        // 087C CLEAR_LOADED_SHOP (0x471AF2): `CShopping::RemoveLoadedShop()`.
        0x087C => remove_loaded_shop(),
        // 087D SET_GROUP_SEQUENCE (0x471AFE): `g = GetActualScriptThingIndex(P0, 8)`; `0 <= g < 8`: dword `0xC09BEC
        // + g * 0x2D4` = `P1 == -1 ? -1 : GetActualScriptThingIndex(P1, 4)`.
        0x087D => {
            collect(s, 2);
            let g = get_actual_script_thing_index(sp(0), 8);
            if (0..8).contains(&g) {
                let v = if spi(1) == -1 {
                    -1
                } else {
                    get_actual_script_thing_index(sp(1), 4)
                };
                GPtr::<i32>::from_addr(0xC09BEC + g as usize * 0x2D4).write(v);
            }
        }
        // 087E SET_CHAR_DROPS_WEAPONS_WHEN_DEAD (0x471B4F): `PED(SP0)` +0x474 bit 9 (bDoesntDropWeaponsWhenDead) =
        // `SP1 == 0` (no null check)
        0x087E => {
            collect(s, 2);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            let fl = p.byte_add(0x474).cast::<u32>();
            let v = fl.read();
            fl.write(if sp(1) != 0 { v & 0xFFFF_FDFF } else { v | 0x200 });
        }
        // 087F SET_CHAR_NEVER_LEAVES_GROUP (0x471BA0): `PED(P0)` +0x474 bit 14 = P1 != 0.
        0x087F => {
            collect(s, 2);
            let p = ped(sp(0));
            set_bit(p, 0x474, 0x4000, sp(1) != 0);
        }
        // 0881 SET_PLAYER_FIRE_BUTTON (0x471BF1): byte `CPad::GetPad(P0)+0x11B` = P1 != 0 ? 0 : 1.
        0x0881 => {
            collect(s, 2);
            let on = sp(1) != 0;
            get_pad(sp(0)).byte_add(0x11B).write(u8::from(!on));
        }
        // 0883 ATTACH_FX_SYSTEM_TO_CHAR_BONE (0x471C3C): `p = PED(P1)`; `i = GetActualScriptThingIndex(P0, 1)`; `i >=
        // 0` and `fx = [0xA44114 + 8 * i]` non-null -> `fx->AttachToBone(p, P2)`.
        0x0883 => {
            collect(s, 3);
            let p0 = sp(0);
            let p = ped(sp(1));
            let i = get_actual_script_thing_index(p0, 1);
            if i >= 0 {
                let fx =
                    GPtr::<GPtr<u8>>::from_addr((i as u32).wrapping_mul(8).wrapping_add(0xA44114) as usize)
                        .read();
                if !fx.is_null() {
                    fx_attach_to_bone(fx, p, sp(2));
                }
            }
        }
        // 0884 REGISTER_ATTRACTOR_SCRIPT_BRAIN_FOR_CODE_USE (0x471C8F): as 07D3 with type 5.
        0x0884 => {
            collect(s, 1);
            let i = get_proper_index(GPtr::from_addr(0xA47B60), sp(0) & 0xFFFF);
            set_sp(0, i32::from(i as i16) as u32);
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_label(s, lp, 8);
            add_brain_for_code_use(GPtr::from_addr(0xA90CF0), sp(0) & 0xFFFF, lp, 5);
        }
        // 0887 SET_HEADING_LIMIT_FOR_ATTACHED_CHAR (0x471CE9): `p = PED(P0)`; word +0x778 = (u16)P1; +0x77C = P2 *
        // 0.017453292.
        0x0887 => {
            collect(s, 3);
            let p = ped(sp(0));
            let a = X87::from_f32(spf(2)).fmul_m(f(K_DEG2RAD)).to_f32();
            p.byte_add(0x778).cast::<u16>().write(sp(1) as u16);
            p.byte_add(0x77C).cast::<f32>().write(a);
        }
        // 0888 ADD_BLIP_FOR_DEAD_CHAR (0x471D2B): `GetActualBlipArrayIndex(CollectNextParameterWithoutIncreasingPC())`
        // (discarded); `b = CRadar::SetEntityBlip(2, P0, 1, 3, this->m_szName)`; `ChangeBlipScale(b, 3)`; P0 = b;
        // store 1.
        0x0888 => {
            collect(s, 1);
            get_actual_blip_array_index(collect_next_parameter_without_increasing_pc(s));
            let b = set_entity_blip(2, sp(0), 1, 3, s.byte_add(8));
            change_blip_scale(b, 3);
            set_sp(0, b);
            store(s, 1);
        }
        // 0889 GET_DEAD_CHAR_COORDINATES (0x471D7A): in a vehicle (+0x46C bit 8) with `v = +0x58C` non-null: v's
        // position; else `p->GetBonePosition(&out, 1, false)` (zeroed out vector); P0..P2 = it; store 3.
        0x0889 => {
            collect(s, 1);
            let p = ped(sp(0));
            let mut out = [0u32; 3];
            let v = p.byte_add(0x58C).cast::<GPtr<u8>>();
            if p.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 && !v.read().is_null() {
                let q = ent_pos(v.read());
                out = [q.read(), q.byte_add(4).read(), q.byte_add(8).read()];
            } else {
                get_bone_position(p, GPtr::from_raw(out.as_mut_ptr()), 1, 0);
            }
            set_sp(0, out[0]);
            set_sp(1, out[1]);
            set_sp(2, out[2]);
            store(s, 3);
        }
        // 088A TASK_PLAY_ANIM_WITH_FLAGS (0x471E0C): `this->PlayAnimScriptCommand(command)`.
        0x088A => play_anim_script_command(s, command),
        // 088B SET_VEHICLE_AIR_RESISTANCE_MULTIPLIER (0x471E1B): `h = VEH(P0)->m_pHandlingData (+0x384)`; non-null:
        // +0x98 = `h+0x10 > [0x858C58] (0.01)` (only Greater) ? `h+0x10 / [0x859E80] * [0x858B8C]` (f32) : h+0x10
        // (raw); then +0x98 = `P1 * +0x98`.
        0x088B => {
            collect(s, 2);
            let v = veh(sp(0));
            let p1 = spf(1);
            let h = v.byte_add(0x384).cast::<GPtr<f32>>().read();
            if !h.is_null() {
                let c = |a: usize| GPtr::<f32>::from_addr(a).read();
                let drag = h.byte_add(0x10);
                let r = v.byte_add(0x98).cast::<f32>();
                if X87::from_f32(drag.read()).fcom(X87::from_f32(c(0x858C58))) == Cmp::Greater {
                    r.write(
                        X87::from_f32(drag.read())
                            .fdiv_m(c(0x859E80))
                            .fmul_m(c(0x858B8C))
                            .to_f32(),
                    );
                } else {
                    r.cast::<u32>().write(drag.cast::<u32>().read());
                }
                r.write(X87::from_f32(p1).fmul_m(r.read()).to_f32());
            }
        }
        // 088C SET_CAR_COORDINATES_NO_OFFSET (0x471EA5): `CCarCtrl::SetCoordsOfScriptCar(VEH(P0), P1, P2, P3, false,
        // false)`.
        0x088C => {
            collect(s, 4);
            let (x, y, z) = (sp(1), sp(2), sp(3));
            set_coords_of_script_car(veh(sp(0)), x, y, z, 0, 0);
        }
        // 088D SET_USES_COLLISION_OF_CLOSEST_OBJECT_OF_TYPE (0x471EE8)
        0x088D => uses_collision_of_closest_object(s),
        // 088E SET_TIME_ONE_DAY_FORWARD (0x47207D): `CClock::OffsetClockByADay(1)`.
        0x088E => offset_clock_by_a_day(1),
        // 0890 SET_TIMER_BEEP_COUNTDOWN_TIME (0x47208E): `i = GetIndexOfGlobalVariable()`; `CollectParameters(1)`;
        // `CUserDisplay::OnscnTimer (0xBA1788).SetClockBeepCountdownSecs((u16)i, P0)`.
        0x0890 => {
            let i = global_index(s);
            collect(s, 1);
            set_clock_beep_countdown_secs(GPtr::from_addr(0xBA1788), u32::from(i), sp(0));
        }
        // 0893 ATTACH_TRAILER_TO_CAB (0x4720BB): `t = VEH(SP0)`, `c = VEH(SP1)` (pool loaded once); if both:
        // `t->SetTowLink(c, true)` (vtable +0xF4)
        0x0893 => {
            collect(s, 2);
            let pl = pool(VEHICLE_POOL);
            let t = vehicle_at_ref(pl, spi(0));
            let c = vehicle_at_ref(pl, spi(1));
            if !t.is_null() && !c.is_null() {
                let f = t.cast::<GPtr<u32>>().read().byte_add(0xF4).read();
                vcall_set_tow_link(f, t, c, 1);
            }
        }
        // 0897 IS_VEHICLE_TOUCHING_OBJECT (0x472100): `v = VEH(P0)`, `o = OBJ(P1)`; flag = `v &&
        // v->GetHasCollidedWith(o) != 0` (o may be null).
        0x0897 => {
            collect(s, 2);
            let v = veh(sp(0));
            let o = obj(sp(1));
            let r = !v.is_null() && get_has_collided_with(v, o) != 0;
            set_flag(s, r);
        }
        _ => return -1,
    }
    0
}

/// 0851 DAMAGE_CHAR (0x47105C): `CollectParameters(3)`; `p = PED(P0)`; `d = fild P1` (x87). P2 != 0 (armour
/// first): `d > armour (+0x548)` or unordered (`test ah,0x41; jp`) -> `d -= armour`, armour = 0; else armour =
/// `armour - d` (f32), `d = 0.0`. Then `health (+0x540) = health - d` (`fst`); ST0 `<= 0` (Less or Equal) ->
/// health = 0, a stack `CEventScriptCommand(3, new(0x28) CTaskComplexDie(0, 0, 0xF, 4.0f, 0, 0, 0, 0, 0), false)`
/// (zeroed 0x20-byte buffer) added to `p->m_pIntelligence->m_eventGroup (+0x68)` (`Add(&event, false)`), the
/// event destructor.
fn damage_char(s: Scr) {
    collect(s, 3);
    let p = ped(sp(0));
    let mut d = X87::from_i32(spi(1));
    if sp(2) != 0 {
        let armour = p.byte_add(0x548).cast::<f32>();
        if d.fcom(X87::from_f32(armour.read())).test_parity(0x41) {
            d = d.fsub_m(armour.read());
            armour.cast::<u32>().write(0);
        } else {
            armour.write(X87::from_f32(armour.read()).fsub(d).to_f32());
            d = X87::from_f32(f(K_ZERO));
        }
    }
    let health = p.byte_add(0x540).cast::<f32>();
    let h = d.fsubr_m(health.read());
    health.write(h.to_f32());
    if h.fcom(X87::from_f32(f(K_ZERO))).test_parity(0x41) {
        return;
    }
    health.cast::<u32>().write(0);
    let t = new_task(0x28, |t| task_die_ctor(t, 0, 0, 0xF, 0x4080_0000, 0, 0, 0, 0, 0));
    let mut ev = [0u32; 8];
    let e = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
    event_script_command_ctor(e, 3, t, 0);
    let intel = p.byte_add(0x47C).cast::<GPtr<u8>>().read();
    event_group_add(intel.byte_add(0x68), e, 0);
    event_script_command_dtor(e);
}

/// 0866 GET_CLOSEST_STEALABLE_OBJECT (0x4715B8): `CollectParameters(4)`: x P0, y P1 (`fld/fst`), z P2 (raw),
/// radius P3. `best = (float)(P3 + P3)`; `CWorld::FindObjectsInRange(&pos, P3, true, &count, 16, list, 0, 0, 0, 1,
/// 0)`; for `i < (i16)count` (unsigned): `e` an object (`+0x36 & 7 == 4`) with `+0x140 & 0x2000`: `d` = the
/// distance (components f32); `d < best` (`test ah,5`) -> best = d (`fstp`), chosen = e. P0 = chosen ?
/// GetRef(chosen) : -1; store 1.
fn closest_stealable_object(s: Scr) {
    collect(s, 4);
    let x = fld_fst(spf(0));
    let y = fld_fst(spf(1));
    let z = sp(2);
    let r = sp(3);
    let mut best = X87::from_f32(f(r)).fadd(X87::from_f32(f(r))).to_f32();
    let mut pos = [x.to_bits(), y.to_bits(), z];
    let mut count = [0u16; 2];
    let mut list = [0u32; 16];
    find_objects_in_range(
        GPtr::from_raw(pos.as_mut_ptr()),
        r,
        1,
        GPtr::from_raw(count.as_mut_ptr()),
        0x10,
        GPtr::from_raw(list.as_mut_ptr()),
        0,
        0,
        0,
        1,
        0,
    );
    let n = i32::from(count[0] as i16) as u32;
    let mut chosen = GPtr::<u8>::NULL;
    let lp = GPtr::from_raw(list.as_mut_ptr());
    let mut i = 0u32;
    while i < n {
        let e = lp.byte_add(4 * i as isize).cast::<GPtr<u8>>().read();
        i += 1;
        if e.byte_add(0x36).read() & 7 != 4 || e.byte_add(0x140).cast::<u32>().read() & 0x2000 == 0 {
            continue;
        }
        let d = distance_to(e, x, y, f(z));
        if d.fcom(X87::from_f32(best)) == Cmp::Less {
            best = d.to_f32();
            chosen = e;
        }
    }
    let v = if chosen.is_null() {
        u32::MAX
    } else {
        object_get_ref_(pool(OBJECT_POOL), chosen)
    };
    set_sp(0, v);
    store(s, 1);
}

/// 0871 SWITCH_START (0x47176C) / 0872 SWITCH_CONTINUED: 0871: `CollectParameters(4)`;
/// `ValueToCheckInSwitchStatement` (0xA43F5C) = P0, `NumberOfEntriesStillToReadForSwitch` (u16 0xA43F60) =
/// `(u16)(P1 * 2)`, `SwitchDefaultExists` (0xA43F58) = P2 != 0, `SwitchDefaultAddress` (0xA43F54) = P3; `n = 14`.
/// 0872: `n = 18`. `CollectParameters(n)`. `N > n` (u16): `AddToSwitchJumpTable(P[i], P[i+1])` for `i < n` step 2,
/// `N -= n`, `N != 0` -> return. Else: the same for `i < N`, `N = 0`. Then `UseSwitchJumpTable(&a)` (writes `a`
/// first) and `UpdatePC(a)`.
fn switch_start(s: Scr, start: bool) {
    let n: u32 = if start {
        collect(s, 4);
        GPtr::<u32>::from_addr(0xA43F5C).write(sp(0));
        let cnt = sp(1).wrapping_mul(2) as u16;
        let def = u8::from(sp(2) != 0);
        GPtr::<u16>::from_addr(0xA43F60).write(cnt);
        GPtr::<u8>::from_addr(0xA43F58).write(def);
        GPtr::<u32>::from_addr(0xA43F54).write(sp(3));
        14
    } else {
        18
    };
    collect(s, n as _);
    let left = GPtr::<u16>::from_addr(0xA43F60);
    if u32::from(left.read()) > n {
        let mut i = 0;
        while i < n {
            add_to_switch_jump_table(sp(i as i32), sp(i as i32 + 1));
            i += 2;
        }
        let rest = left.read().wrapping_sub(n as u16);
        left.write(rest);
        if rest != 0 {
            return;
        }
    } else {
        let k = u32::from(left.read());
        let mut i = 0;
        while i < k {
            add_to_switch_jump_table(sp(i as i32), sp(i as i32 + 1));
            i += 2;
        }
        left.write(0);
    }
    let mut a = [0u32; 1];
    use_switch_jump_table(GPtr::from_raw(a.as_mut_ptr()));
    crate::vm::update_pc_call(s, a[0] as i32);
}

/// 088D SET_USES_COLLISION_OF_CLOSEST_OBJECT_OF_TYPE (0x471EE8): `CollectParameters(6)`: x P0, y P1, z P2 (raw, or
/// `(float)FindGroundZForCoord(x, y)` when `z <= -100`), radius P3, model P4 (UsedObjectArray rule), flag P5.
/// `best = (float)(P3 + P3)`; `FindObjectsOfTypeInRange(model, &pos, P3, true, &count, 16, list, 1, 0, 0, 1, 1)`;
/// for `(u16)i < (i16)count` (signed): `d` = the distance of `list[i]` (components f32); `d < best` -> best = d,
/// chosen = it. chosen -> entity flags +0x1C bit 0 = P5 != 0.
fn uses_collision_of_closest_object(s: Scr) {
    collect(s, 6);
    let (x, y) = (sp(0), sp(1));
    let mut z = sp(2);
    if !X87::from_f32(f(z))
        .fcom(X87::from_f32(f(K_MINUS_100)))
        .test_parity(0x41)
    {
        z = X87::from_f64_exact(find_ground_z(x, y)).to_f32().to_bits();
    }
    let r = sp(3);
    let mut best = X87::from_f32(f(r)).fadd(X87::from_f32(f(r))).to_f32();
    let model = {
        let m = sp(4);
        if (m as i32) < 0 {
            GPtr::<u32>::from_addr(0xA44B88u32.wrapping_sub(m.wrapping_mul(0x1C)) as usize).read()
        } else {
            m
        }
    };
    let mut pos = [x, y, z];
    let mut count = [0u16; 2];
    let mut list = [0u32; 16];
    find_objects_of_type_in_range(
        model,
        GPtr::from_raw(pos.as_mut_ptr()),
        r,
        1,
        GPtr::from_raw(count.as_mut_ptr()),
        0x10,
        GPtr::from_raw(list.as_mut_ptr()),
        1,
        0,
        0,
        1,
        1,
    );
    let n = i32::from(count[0] as i16);
    let mut chosen = GPtr::<u8>::NULL;
    let lp = GPtr::from_raw(list.as_mut_ptr());
    if n > 0 {
        let mut i: u16 = 0;
        loop {
            let e = lp.byte_add(4 * i as isize).cast::<GPtr<u8>>().read();
            let d = distance_to(e, f(x), f(y), f(z));
            if d.fcom(X87::from_f32(best)) == Cmp::Less {
                best = d.to_f32();
                chosen = e;
            }
            i = i.wrapping_add(1);
            if i32::from(i) >= n {
                break;
            }
        }
    }
    if !chosen.is_null() {
        set_bit(chosen, 0x1C, 1, sp(5) != 0);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2100_TO_2199];
