//! Script-system initialisers of the loading path (batch `mvpf-script-1`): `CRunningScript::Init`,
//! `CStreamedScripts::{Initialise, RegisterScript, RemoveStreamedScriptFromMemory}`,
//! `CScriptResourceManager::Initialise` and the `CTheScripts` table resets / header readers. Ported from
//! the 1.0 US `.HOODLUM` bodies behind the stubs; unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// CRT `free` (0x8213AE)
    fn crt_free(p: GPtr<u8>) = cdecl 0x8213ae => unported;
    /// `CModelInfo::GetModelInfo(const char* name, int32* index)` (0x4C5940)
    /// (ported in sa-stream, which sa-script does not depend on)
    fn get_model_info(name: GPtr<u8>, index: GPtr<i32>) -> GPtr<u8> = cdecl 0x4c5940 => unported;
}

fn at<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> GPtr<T> {
    p.byte_add(off).cast::<T>()
}

fn ga<T: sa_core::mem::GameData>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}

/// `CStreamedScripts::RemoveStreamedScriptFromMemory(int32 index)` (stub 0x4708E0 → 0x156F710, `ret 4`):
/// `free(m_aScripts[index].data)` (entry `index << 5`, data +0), then the pointer is cleared.
#[hook(addr = 0x4708e0, conv = thiscall, name = "CStreamedScripts::RemoveStreamedScriptFromMemory", expect = "E9 2B EE 0F 01",
       this = "CStreamedScripts", unverified = "mvpf-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn remove_streamed_script_from_memory(this: GPtr<u8>, index: i32) {
    let e = this.byte_add(index.wrapping_shl(5) as isize);
    crt_free(at::<GPtr<u8>>(e, 0).read());
    at::<u32>(e, 0).write(0);
}

/// `CRunningScript::Init()` (stub 0x4648E0 → 0x15626B0): name "noname" (7 bytes from 0x859D44) at +8,
/// +0x10, +4, +0, +0x14 = 0, the 8 dwords +0x18..+0x38 = 0, u16 +0x38 = 0, +0xCC = 0, bytes
/// +0xC4..=+0xC8 = 0, +0xC9 = 0xFF, the 0x22 dwords +0x3C..+0xC4 = 0, u16 +0xD0 = 0, +0xD2 = 0,
/// +0xD4 = 0, +0xD8 = 0, +0xDC = 0, +0xD3 = 1 (in that order).
#[hook(addr = 0x4648e0, conv = thiscall, name = "CRunningScript::Init", expect = "E9 CB DD 0F 01",
       this = "CRunningScript", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn running_script_init(this: GPtr<u8>) {
    at::<u32>(this, 8).write(ga::<u32>(0x859D44).read());
    at::<u16>(this, 0xC).write(ga::<u16>(0x859D48).read());
    at::<u8>(this, 0xE).write(ga::<u8>(0x859D4A).read());
    for o in [0x10isize, 4, 0, 0x14] {
        at::<u32>(this, o).write(0);
    }
    for k in 0..8 {
        at::<u32>(this, 0x18 + 4 * k).write(0);
    }
    at::<u16>(this, 0x38).write(0);
    at::<u32>(this, 0xCC).write(0);
    for o in 0xC4isize..=0xC8 {
        at::<u8>(this, o).write(0);
    }
    at::<u8>(this, 0xC9).write(0xFF);
    for k in 0..0x22 {
        at::<u32>(this, 0x3C + 4 * k).write(0);
    }
    at::<u16>(this, 0xD0).write(0);
    at::<u8>(this, 0xD2).write(0);
    at::<u8>(this, 0xD4).write(0);
    at::<u32>(this, 0xD8).write(0);
    at::<u8>(this, 0xDC).write(0);
    at::<u8>(this, 0xD3).write(1);
}

/// `CTheScripts::ClearAllSuppressedCarModels()` (stub 0x46A7C0 → 0x156F400): the 0x28 dwords at 0xA44940
/// = -1.
#[hook(addr = 0x46a7c0, conv = cdecl, name = "CTheScripts::ClearAllSuppressedCarModels", expect = "E9 3B 4C 10 01",
       unverified = "mvpf-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn clear_all_suppressed_car_models() {
    for k in 0..0x28usize {
        ga::<i32>(0xA44940 + 4 * k).write(-1);
    }
}

/// `CTheScripts::ClearAllVehicleModelsBlockedByScript()` (stub 0x46A840 → 0x1565EA0): the 0x14 dwords at
/// 0xA448F0 = -1.
#[hook(addr = 0x46a840, conv = cdecl, name = "CTheScripts::ClearAllVehicleModelsBlockedByScript", expect = "E9 5B B6 0F 01",
       unverified = "mvpf-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn clear_all_vehicle_models_blocked_by_script() {
    for k in 0..0x14usize {
        ga::<i32>(0xA448F0 + 4 * k).write(-1);
    }
}

/// `CScriptResourceManager::Initialise()` (stub 0x470480 → 0x1565710): 0x4B entries of 0xC bytes, each
/// id +0 = -1, u16 type +8 = 0, script +4 = 0.
#[hook(addr = 0x470480, conv = thiscall, name = "CScriptResourceManager::Initialise", expect = "E9 8B 52 0F 01",
       this = "CScriptResourceManager", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn script_resource_manager_initialise(this: GPtr<u8>) {
    for k in 0..0x4Bisize {
        let e = this.byte_add(0xC * k);
        at::<i32>(e, 0).write(-1);
        at::<u16>(e, 8).write(0);
        at::<u32>(e, 4).write(0);
    }
}

/// `CStreamedScripts::Initialise()` (stub 0x470660 → 0x1562430): 0x52 entries of 0x20 bytes, each data +0
/// = 0, byte +4 = 0, u16 +6 = 0xFFFF, byte +8 = 0, +0x1C = 0; then +0xA40 = 0 and the u16 count
/// +0xA44 = 0. The body leaves ECX untouched.
#[hook(addr = 0x470660, conv = thiscall, name = "CStreamedScripts::Initialise", expect = "E9 CB 1D 0F 01",
       this = "CStreamedScripts", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn streamed_scripts_initialise(this: GPtr<u8>) {
    for k in 0..0x52isize {
        let e = this.byte_add(0x20 * k);
        at::<u32>(e, 0).write(0);
        at::<u8>(e, 4).write(0);
        at::<u16>(e, 6).write(0xFFFF);
        at::<u8>(e, 8).write(0);
        at::<u32>(e, 0x1C).write(0);
    }
    at::<u32>(this, 0xA40).write(0);
    at::<u16>(this, 0xA44).write(0);
}

/// `CStreamedScripts::RegisterScript(const char* name)` (stub 0x4706C0 → 0x156B7A0, `ret 4`): copies
/// `name` with its terminator to `m_aScripts[count].name` (`this + count*0x20 + 8`, no length check),
/// increments the u16 count (+0xA44) and returns `count - 1` (the new index, from the zero-extended
/// count).
#[hook(addr = 0x4706c0, conv = thiscall, name = "CStreamedScripts::RegisterScript", expect = "E9 DB B0 0F 01",
       this = "CStreamedScripts", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn streamed_scripts_register_script(this: GPtr<u8>, name: GPtr<u8>) -> i32 {
    let cnt = at::<u16>(this, 0xA44);
    let mut d = this.byte_add((cnt.read() as isize) << 5).byte_add(8);
    let mut s = name;
    loop {
        let c = s.cast::<u8>().read();
        d.cast::<u8>().write(c);
        s = s.byte_add(1);
        d = d.byte_add(1);
        if c == 0 {
            break;
        }
    }
    cnt.write(cnt.read().wrapping_add(1));
    cnt.read() as i32 - 1
}

/// `CTheScripts::ReadMultiScriptFileOffsetsFromScript()` (stub 0x4867C0 → 0x1565E20): follows the
/// script header's jumps (`ScriptSpace + 3` holds the first jump's target, the dword there + 3 the
/// second one's) to the multi-script block at `ScriptSpace + 8 + that`, and reads: `MainScriptSize`
/// (0xA444C4) = dword +0, `LargestMissionScriptSize` (0xA444C0) = dword +4, `NumberOfMissionScripts`
/// (0xA444BC, i16) = word +8, `NumberOfExclusiveMissionScripts` (0xA444B8) = word +0xA,
/// `LargestNumberOfMissionScriptLocalVariables` (0xA444B4) = dword +0xC, then the mission offsets
/// (0xA444C8) from +0x10, as many as the sign-extended mission count.
#[hook(addr = 0x4867c0, conv = cdecl, name = "CTheScripts::ReadMultiScriptFileOffsetsFromScript", expect = "E9 5B F6 0D 01",
       unverified = "mvpf-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn read_multi_script_file_offsets_from_script() {
    let o1 = ga::<u32>(0xA49963).read();
    let o2 = ga::<u32>((o1.wrapping_add(0xA49963)) as usize).read();
    let b = GPtr::<u8>::from_addr(o2.wrapping_add(0xA49968) as usize);
    ga::<u32>(0xA444C4).write(at::<u32>(b, 0).read());
    ga::<u32>(0xA444C0).write(at::<u32>(b, 4).read());
    let n = at::<i16>(b, 8).read();
    ga::<u16>(0xA444B8).write(at::<u16>(b, 0xA).read());
    ga::<u32>(0xA444B4).write(at::<u32>(b, 0xC).read());
    ga::<i16>(0xA444BC).write(n);
    let mut i: u16 = 0;
    while (i as i32) < n as i32 {
        ga::<u32>(0xA444C8 + 4 * i as usize).write(at::<u32>(b, 0x10 + 4 * i as isize).read());
        i = i.wrapping_add(1);
    }
}

/// `CTheScripts::UpdateObjectIndices()` (stub 0x486780 → 0x1562660): for the used object names 1 ..
/// `NumberOfUsedObjects` (u16 0xA44B6C; 0x1C-byte entries at 0xA44B6C + 4: name +0, index +0x18): index
/// = -1, then `CModelInfo::GetModelInfo(name, &index)`.
#[hook(addr = 0x486780, conv = cdecl, name = "CTheScripts::UpdateObjectIndices", expect = "E9 DB BE 0D 01",
       unverified = "mvpf-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn update_object_indices() {
    let mut i: u16 = 1;
    while i < ga::<u16>(0xA44B6C).read() {
        let e = i as usize * 0x1C;
        let idx = ga::<i32>(0xA44B88 + e);
        idx.write(-1);
        get_model_info(GPtr::from_addr(0xA44B70 + e), idx);
        i = i.wrapping_add(1);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_REMOVE_STREAMED_SCRIPT_FROM_MEMORY,
    HOOK_RUNNING_SCRIPT_INIT,
    HOOK_CLEAR_ALL_SUPPRESSED_CAR_MODELS,
    HOOK_CLEAR_ALL_VEHICLE_MODELS_BLOCKED_BY_SCRIPT,
    HOOK_SCRIPT_RESOURCE_MANAGER_INITIALISE,
    HOOK_STREAMED_SCRIPTS_INITIALISE,
    HOOK_STREAMED_SCRIPTS_REGISTER_SCRIPT,
    HOOK_READ_MULTI_SCRIPT_FILE_OFFSETS_FROM_SCRIPT,
    HOOK_UPDATE_OBJECT_INDICES,
];
