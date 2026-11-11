//! Script start-up and script-brain bookkeeping of the loading path (batch `mvph-script-1`):
//! `CTheScripts::StartNewScript`, the waiting-for-brain array, `CScriptsForBrains::{Init,
//! AddNewStreamedScriptBrainForCodeUse}`, `CStreamedScripts::{GetProperIndexFromIndexUsedByScript,
//! StartNewStreamedScript}` and `CScriptResourceManager::AddToResourceManager`. Ported from the 1.0 US
//! `.HOODLUM` bodies behind the stubs (and the no-CD fragment 0x401F5F); unverified
//! (docs/verify/HOWTO.md).
//!
//! `tScriptBrain` (0x14 bytes, 0x46 entries): model / index int16 +0, attach type +2, type +3,
//! active +4, radius f32 +8, name char[8] +0xC. `tStreamedScript` 0x20 bytes (data +0, status +4,
//! script index int16 +6). Resource entries 0xC bytes (0x4B): id +0, owner +4, type u16 +8.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// `CTheScripts::pIdleScripts` (0xA8B428)
const IDLE_SCRIPTS: usize = 0xA8B428;
/// `CTheScripts::pActiveScripts` (0xA8B42C)
const ACTIVE_SCRIPTS: usize = 0xA8B42C;
/// `CTheScripts::EntitiesWaitingForScriptBrain` (0xA476B0, 0x96 entries of 8 bytes)
const WAITING_FOR_BRAIN: usize = 0xA476B0;
/// The default script-brain activation radius (0x859D9C, f32 read as a dword).
const BRAIN_RADIUS: usize = 0x859D9C;

orig! {
    /// `CRunningScript::Init()` (0x4648E0)
    fn running_script_init(s: GPtr<u8>) = thiscall 0x4648e0 => crate::script_init::running_script_init;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571a00 => unported;
    /// `CStreaming::SetMissionDoesntRequireModel(int32)` (0x409C90)
    fn set_mission_doesnt_require_model(model: i32) = cdecl 0x409c90 => unported;
}

fn at<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> GPtr<T> {
    p.byte_add(off).cast::<T>()
}

fn ga<T: sa_core::mem::GameData>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}

/// `CTheScripts::StartNewScript(uint8* ip)` (stub 0x464C20 → 0x156A2A0, cdecl): takes the head of
/// the idle list (no null check), unlinks it (`prev->next = next` when prev, `pIdleScripts = next`,
/// `next->prev = prev` when next; next +0, prev +4), `CRunningScript::Init`, base ip +0x14 = `ip`,
/// pushes it at the head of the active list (next = old head, prev = 0, old head's prev = it) and
/// sets +0xC4 (active) to 1. Returns the script.
#[hook(addr = 0x464c20, conv = cdecl, name = "CTheScripts::StartNewScript-last-idle", expect = "E9 7B 56 10 01",
       unverified = "mvph-script-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "moves a CRunningScript (ScriptsArray 0xA8B430) from the idle to the active list")]
pub fn start_new_script(ip: GPtr<u8>) -> GPtr<u8> {
    let s = ga::<GPtr<u8>>(IDLE_SCRIPTS).read();
    let prev = at::<GPtr<u8>>(s, 4).read();
    if !prev.is_null() {
        at::<GPtr<u8>>(prev, 0).write(at::<GPtr<u8>>(s, 0).read());
    }
    ga::<GPtr<u8>>(IDLE_SCRIPTS).write(at::<GPtr<u8>>(s, 0).read());
    let next = at::<GPtr<u8>>(s, 0).read();
    if !next.is_null() {
        at::<GPtr<u8>>(next, 4).write(at::<GPtr<u8>>(s, 4).read());
    }
    running_script_init(s);
    at::<GPtr<u8>>(s, 0x14).write(ip);
    at::<GPtr<u8>>(s, 0).write(ga::<GPtr<u8>>(ACTIVE_SCRIPTS).read());
    at::<u32>(s, 4).write(0);
    let head = ga::<GPtr<u8>>(ACTIVE_SCRIPTS).read();
    if !head.is_null() {
        at::<GPtr<u8>>(head, 4).write(s);
    }
    ga::<GPtr<u8>>(ACTIVE_SCRIPTS).write(s);
    at::<u8>(s, 0xC4).write(1);
    s
}

/// `CTheScripts::RemoveFromWaitingForScriptBrainArray(CEntity* entity, int16 modelIndex)` (stub
/// 0x46ABC0 → 0x1569CD0, cdecl): every one of the 0x96 entries of 0xA476B0 (entity +0, int16 +4)
/// holding `entity` and `modelIndex` gets `CleanUpOldReference(&entry.entity)` (entity non-null),
/// entity 0 and index -1.
#[hook(addr = 0x46abc0, conv = cdecl, name = "CTheScripts::RemoveFromWaitingForScriptBrainArray",
       expect = "E9 0B F1 0F 01", unverified = "mvph-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn remove_from_waiting_for_script_brain_array(entity: GPtr<u8>, model_index: u32) {
    let id = model_index as u16;
    for i in 0..0x96usize {
        let e = ga::<u8>(WAITING_FOR_BRAIN + 8 * i);
        let ent = at::<GPtr<u8>>(e, 0).read();
        if ent == entity && at::<u16>(e, 4).read() == id {
            if !ent.is_null() {
                clean_up_old_reference(ent, e);
            }
            at::<u32>(e, 0).write(0);
            at::<u16>(e, 4).write(0xFFFF);
        }
    }
}

/// `CScriptsForBrains::Init()` (stub 0x46A8C0 → 0x156BB70): each of the 0x46 brains gets index -1
/// (+0), +0xC and +0xE zero (u16), +0x10 zero, attach type and type 0xFF (+2, +3), active 1 (+4) and
/// the radius dword of 0x859D9C (+8, reread per entry), in the bytes' store order.
#[hook(addr = 0x46a8c0, conv = thiscall, name = "CScriptsForBrains::Init", expect = "E9 AB 12 10 01",
       unverified = "mvph-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn scripts_for_brains_init(this: GPtr<u8>) {
    for i in 0..0x46isize {
        let e = this.byte_add(0x14 * i);
        let radius = ga::<u32>(BRAIN_RADIUS).read();
        at::<u16>(e, 0).write(0xFFFF);
        at::<u16>(e, 0xC).write(0);
        at::<u16>(e, 0xE).write(0);
        at::<u32>(e, 0x10).write(0);
        at::<u8>(e, 2).write(0xFF);
        at::<u8>(e, 3).write(0xFF);
        at::<u8>(e, 4).write(1);
        at::<u32>(e, 8).write(radius);
    }
}

/// `CScriptsForBrains::AddNewStreamedScriptBrainForCodeUse(int16 index, const char* name, int8
/// attachType)` (stub 0x46A9C0 → 0x15628A0, `ret 0xC`): the first brain with index -1 (0x46 entries)
/// gets `index`, the name copied byte by byte up to and including its NUL at +0xC (no length limit),
/// attach type +2, type 0xFF (+3), active 1 (+4) and the radius dword of 0x859D9C (+8). No free
/// entry: nothing.
#[hook(addr = 0x46a9c0, conv = thiscall, name = "CScriptsForBrains::AddNewStreamedScriptBrainForCodeUse",
       expect = "E9 DB 7E 0F 01", unverified = "mvph-script-1", route = "L2",
       hazards = "none", recs = "grove-walk")]
pub fn add_new_streamed_script_brain_for_code_use(this: GPtr<u8>, index: u32, name: GPtr<u8>, attach: u32) {
    let Some(i) = (0..0x46isize).find(|&i| at::<u16>(this, 0x14 * i).read() == 0xFFFF) else {
        return;
    };
    let e = this.byte_add(0x14 * i);
    at::<u16>(e, 0).write(index as u16);
    let mut k = 0isize;
    loop {
        let c = at::<u8>(name, k).read();
        at::<u8>(e, 0xC + k).write(c);
        k += 1;
        if c == 0 {
            break;
        }
    }
    let radius = ga::<u32>(BRAIN_RADIUS).read();
    at::<u8>(e, 2).write(attach as u8);
    at::<u8>(e, 3).write(0xFF);
    at::<u8>(e, 4).write(1);
    at::<u32>(e, 8).write(radius);
}

/// `CStreamedScripts::GetProperIndexFromIndexUsedByScript(int16 index)` (0x470810, entry `jmp` into
/// the no-CD fragment 0x401F5F, `ret 4`, AX): the first of the 0x52 streamed scripts whose int16 +6
/// equals `index` (entries 0x20 bytes); else 0xFFFF. EAX's upper half is 0 on both paths.
#[hook(addr = 0x470810, conv = thiscall, name = "CStreamedScripts::GetProperIndexFromIndexUsedByScript",
       expect = "E9 4A 17 F9 FF 33 C0 56", this = "CStreamedScripts", unverified = "mvph-script-1", route = "L2",
       hazards = "none", recs = "grove-walk", note = "entry jmp goes to a no-CD fragment (not .HOODLUM); expect covers 8 bytes")]
pub fn get_proper_index_from_index_used_by_script(this: GPtr<u8>, index: u32) -> u32 {
    let want = index as u16;
    for i in 0..0x52u32 {
        if at::<u16>(this, (i << 5) as isize + 6).read() == want {
            return i;
        }
    }
    0xFFFF
}

/// `CStreamedScripts::StartNewStreamedScript(int32 index)` (stub 0x470890 → 0x156BFB0, `ret 4`):
/// entry `index << 5`; no data (+0): 0. Else `StartNewScript(data)`, the script's base ip (+0x10) =
/// data, +0xC7 (streamed) = 1, the entry's status byte (+4) incremented and
/// `SetMissionDoesntRequireModel(index + 0x6676)`; returns the script.
#[hook(addr = 0x470890, conv = thiscall, name = "CStreamedScripts::StartNewStreamedScript", expect = "E9 1B B7 0F 01",
       this = "CStreamedScripts", unverified = "mvph-script-1", route = "L2", hazards = "none",
       recs = "grove-walk", note = "StartNewScript moves a CRunningScript between the script lists")]
pub fn start_new_streamed_script(this: GPtr<u8>, index: i32) -> GPtr<u8> {
    let e = this.byte_add((index << 5) as isize);
    let data = at::<GPtr<u8>>(e, 0).read();
    if data.is_null() {
        return GPtr::NULL;
    }
    let s = start_new_script_call(data);
    at::<GPtr<u8>>(s, 0x10).write(data);
    at::<u8>(s, 0xC7).write(1);
    let st = at::<u8>(e, 4).read().wrapping_add(1);
    at::<u8>(e, 4).write(st);
    set_mission_doesnt_require_model(index.wrapping_add(0x6676));
    s
}

orig! {
    /// `CTheScripts::StartNewScript(uint8*)` (0x464C20)
    fn start_new_script_call(ip: GPtr<u8>) -> GPtr<u8> = cdecl 0x464c20 => start_new_script;
}

/// `CScriptResourceManager::AddToResourceManager(int32 id, uint32 type, CRunningScript* owner)` (stub
/// 0x4704B0 → 0x156ACE0, `ret 0xC`): over the 0x4B entries (id +0, owner +4, u16 type +8): an entry
/// with this id, `(uint32)type16 == type` (the full argument dword) and owner ends it; every free
/// entry (type 0) is remembered, the last one wins. Then the remembered entry, if any, gets id, the
/// low 16 bits of `type` and owner.
#[hook(addr = 0x4704b0, conv = thiscall, name = "CScriptResourceManager::AddToResourceManager", expect = "E9 2B A8 0F 01",
       this = "CScriptResourceManager", unverified = "mvph-script-1", route = "L2", hazards = "none",
       recs = "grove-walk")]
pub fn add_to_resource_manager(this: GPtr<u8>, id: i32, ty: u32, owner: GPtr<u8>) {
    let mut free: i32 = -1;
    for i in 0..0x4Bi32 {
        let e = this.byte_add(0xC * i as isize);
        let t = u32::from(at::<u16>(e, 8).read());
        if at::<i32>(e, 0).read() == id && t == ty && at::<GPtr<u8>>(e, 4).read() == owner {
            return;
        }
        if t == 0 {
            free = i;
        }
    }
    if free != -1 {
        let e = this.byte_add(0xC * free as isize);
        at::<i32>(e, 0).write(id);
        at::<u16>(e, 8).write(ty as u16);
        at::<GPtr<u8>>(e, 4).write(owner);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_START_NEW_SCRIPT,
    HOOK_REMOVE_FROM_WAITING_FOR_SCRIPT_BRAIN_ARRAY,
    HOOK_SCRIPTS_FOR_BRAINS_INIT,
    HOOK_ADD_NEW_STREAMED_SCRIPT_BRAIN_FOR_CODE_USE,
    HOOK_GET_PROPER_INDEX_FROM_INDEX_USED_BY_SCRIPT,
    HOOK_START_NEW_STREAMED_SCRIPT,
    HOOK_ADD_TO_RESOURCE_MANAGER,
];
