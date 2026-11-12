//! Script brains requested for new entities and their scripts' start and shutdown (batch
//! `mvpi-script-1`): `CScriptsForBrains::{CheckIfNewEntityNeedsScript,
//! StartOrRequestNewStreamedScriptBrain, StartNewStreamedScriptBrain}`, `CRunningScript::ShutdownThisScript`. Ported
//! from the 1.0 US bytes; unverified (docs/verify/HOWTO.md).
//!
//! `tScriptBrain` (0x14 bytes, 0x46 entries at `this`): streamed script index int16 +0, attach type +2,
//! active +4, model index int16 +0xC, percentage int16 +0xE.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// `CStreaming::ms_aInfoForModel[0x6676].m_nLoadState` (the streamed-script model slots start at 0x6676).
const SCRIPT_LOAD_STATE: usize = 0x964E08;
/// `CTheScripts::StreamedScripts` (0xA47B60): 0x20-byte entries from +4 (status byte +4 of each).
const STREAMED_SCRIPTS: usize = 0xA47B60;
/// `CPools::ms_pPedPool` (0xB74490, 0x7C4-byte peds) and `ms_pObjectPool` (0xB7449C, 0x19C-byte objects)
const PED_POOL: usize = 0xB74490;
const OBJECT_POOL: usize = 0xB7449C;
/// `CTheScripts::LocalVariablesForCurrentMission` (0xA48960)
const MISSION_LOCALS: usize = 0xA48960;

orig! {
    /// `CTheScripts::AddToWaitingForScriptBrainArray(CEntity*, int16 brainIndex)` (0x46AB60, cdecl)
    fn add_to_waiting_for_script_brain_array(e: GPtr<u8>, index: u32) = cdecl 0x46AB60 => unported;
    /// `CScriptsForBrains::StartNewStreamedScriptBrain(uint8 index, CEntity*, bool)` (0x46B270)
    fn start_new_streamed_script_brain(this: GPtr<u8>, index: u32, e: GPtr<u8>, b: u32) = thiscall 0x46B270 => start_new_streamed_script_brain_port;
    /// `CStreamedScripts::StartNewStreamedScript(int32 index)` (0x470890)
    fn start_new_streamed_script(this: GPtr<u8>, index: i32) -> GPtr<u8> = thiscall 0x470890 => crate::script_brains::start_new_streamed_script;
    /// `CStreamedScripts::GetStreamedScriptWithThisStartAddress(uint8* ip)` (0x470910)
    fn get_streamed_script_with_this_start_address(this: GPtr<u8>, ip: u32) -> i32 = thiscall 0x470910 => crate::script_tables::get_streamed_script_with_this_start_address_host;
    /// `CTheScripts::RemoveFromWaitingForScriptBrainArray(CEntity*, int16)` (0x46ABC0)
    fn remove_from_waiting_for_script_brain_array(e: GPtr<u8>, index: u32) = cdecl 0x46ABC0 => crate::script_brains::remove_from_waiting_for_script_brain_array;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `thunk_FUN_0156f250` (0x463640): the 8-byte task constructor `ShutdownThisScript` hands to the
    /// brain task store; returns `this`
    fn brain_finish_task_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x463640 => unported;
    /// `CScriptedBrainTaskStore::SetTask(CPed*, CTask*)` (0x635720)
    fn brain_task_store_set_task(ped: GPtr<u8>, task: GPtr<u8>) -> GPtr<u8> = cdecl 0x635720 => unported;
    /// `CStreaming::RequestModel(int32 model, int32 flags)` (0x4087E0): EAX is kept (its upper half is
    /// pushed again below)
    fn request_model(model: i32, flags: u32) -> u32 = cdecl 0x4087E0 => unported;
    /// `CScriptsForBrains::StartOrRequestNewStreamedScriptBrain(...)` (0x46CD80)
    fn start_or_request_call(this: GPtr<u8>, index: u32, e: GPtr<u8>, attach: u32, add: u32) = thiscall 0x46CD80 => start_or_request_new_streamed_script_brain;
}

fn at<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> GPtr<T> {
    p.byte_add(off).cast::<T>()
}

fn brain(this: GPtr<u8>, i: u8) -> GPtr<u8> {
    this.byte_add(isize::from(i) * 0x14)
}

/// `CScriptsForBrains::StartOrRequestNewStreamedScriptBrain(uint8 index, CEntity* entity, int8
/// attachType, bool addToWaitingArray)` (0x46CD80, thiscall, `ret 0x10`). With `addToWaitingArray`:
/// nothing when the brain is inactive (+4); by the exe's jump table, attach types 0 / 3 (peds) stop on
/// ped flags 0x800000 or 0x1000000 (+0x474), types 1 / 4 (objects) stop on object flags 0x300000
/// (+0x140). Attach type 1: with `addToWaitingArray` object +0x168 = index and flags (+0x140) bit
/// 0x100000 set, 0x200000 cleared, then `AddToWaitingForScriptBrainArray(entity, index)`; without it
/// bit 0x200000 set and 0x100000 cleared. A loaded script model (load state 1 of model 0x6676 + the
/// brain's script index) starts the brain (`StartNewStreamedScriptBrain(index, entity, false)`);
/// otherwise `RequestModel(0x6676 + index, 4)` and, for attach types 0, 3, 4 with
/// `addToWaitingArray`, ped +0x794 = index, ped flags |= 0x1000000 and
/// `AddToWaitingForScriptBrainArray(entity, index)` (the pushed dword's upper half is
/// `RequestModel`'s EAX upper half).
#[hook(addr = 0x46cd80, conv = thiscall, name = "CScriptsForBrains::StartOrRequestNewStreamedScriptBrain",
       expect = "53 8B 5C 24 08", unverified = "mvpi-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn start_or_request_new_streamed_script_brain(this: GPtr<u8>, index: u32, e: GPtr<u8>, attach: u32, add: u32) {
    let idx = index as u8;
    let add = add as u8 != 0;
    let at8 = attach as u8;
    if add {
        if at::<u8>(brain(this, idx), 4).read() == 0 {
            return;
        }
        match at8 as i8 {
            0 | 3 => {
                let f = at::<u32>(e, 0x474).read();
                if f & 0x80_0000 != 0 || f & 0x100_0000 != 0 {
                    return;
                }
            }
            1 | 4 if at::<u32>(e, 0x140).read() & 0x30_0000 != 0 => return,
            _ => {}
        }
    }
    if at8 == 1 {
        if add {
            let f = at::<u32>(e, 0x140).read();
            at::<u16>(e, 0x168).write(u16::from(idx));
            at::<u32>(e, 0x140).write((f & 0xFFDF_FFFF) | 0x10_0000);
            add_to_waiting_for_script_brain_array(e, u32::from(idx));
        } else {
            let f = at::<u32>(e, 0x140).read();
            at::<u32>(e, 0x140).write((f & 0xFFEF_FFFF) | 0x20_0000);
        }
    }
    let slot = i32::from(at::<i16>(brain(this, idx), 0).read());
    let state = GPtr::<u8>::from_addr((SCRIPT_LOAD_STATE as u32).wrapping_add((slot as u32).wrapping_mul(0x14)) as usize);
    if state.read() == 1 {
        start_new_streamed_script_brain(this, index, e, 0);
        return;
    }
    let eax = request_model(slot.wrapping_add(0x6676), 4);
    let t = at8 as i8;
    if (t == 0 || (3..=4).contains(&t)) && add {
        let f = at::<u32>(e, 0x474).read();
        at::<u16>(e, 0x794).write(u16::from(idx));
        at::<u32>(e, 0x474).write(f | 0x100_0000);
        add_to_waiting_for_script_brain_array(e, (eax & 0xFFFF_0000) | u32::from(idx));
    }
}

/// `CScriptsForBrains::CheckIfNewEntityNeedsScript(CEntity* entity, int8 attachType, void* unused)`
/// (0x46FF20, thiscall, `ret 0xC`). Peds (type 0) with flags 0x800000 or 0x1000000 (+0x474), or other
/// entities with flags 0x300000 (+0x140), are skipped. Then, until one brain started, for each of the
/// 0x46 brains of this attach type whose model (+0xC) is the entity's (+0x22): `rand()` (return address
/// 0x46FF95), `_ftol((rand & 0xFFFF) * 1/32768 * 100.0)` (x87) below the brain's percentage (+0xE) →
/// `StartOrRequestNewStreamedScriptBrain(i, entity, attachType, true)`.
///
/// Stack: the original stores the masked rand value in its own `entity` argument slot (S+4, `fild`
/// temporary; not reproduced), and pushes the brain index as the dword of a local whose upper three
/// bytes it never wrote (zero here).
#[hook(addr = 0x46ff20, conv = thiscall, name = "CScriptsForBrains::CheckIfNewEntityNeedsScript",
       expect = "83 EC 08 53 8B 5C 24 14", route = "L2", hazards = "stack-residue,uninit-locals",
       recs = "grove-walk")]
pub fn check_if_new_entity_needs_script(this: GPtr<u8>, e: GPtr<u8>, attach: u32, _unused: u32) {
    let at8 = attach as u8;
    if at8 == 0 {
        let f = at::<u32>(e, 0x474).read();
        if f & 0x80_0000 != 0 || f & 0x100_0000 != 0 {
            return;
        }
    } else if at::<u32>(e, 0x140).read() & 0x30_0000 != 0 {
        return;
    }
    for i in 0u8..0x46 {
        let b = brain(this, i);
        if at::<u8>(b, 2).read() != at8 || at::<u16>(b, 0xC).read() != at::<u16>(e, 0x22).read() {
            continue;
        }
        let r = sa_core::orig::at_site(0x46FF95, sa_core::crt::rand);
        let pct = i32::from(at::<u16>(b, 0xE).read());
        let v = X87::from_i32(r & 0xFFFF)
            .fmul_m(f32::from_bits(0x3800_0000))
            .fmul_m(100.0)
            .ftol() as u32 as i32;
        if v < pct {
            start_or_request_call(this, u32::from(i), e, attach, 1);
            return;
        }
    }
}

/// `CPool::GetRef`: `((e - objects) / size) << 8 | flags[index]`, the division through the
/// compiler's magic multiply (`imul magic; sar edx, shift; add sign`).
fn pool_ref(pool_cell: usize, e: GPtr<u8>, magic: i32, shift: u32) -> i32 {
    let pool = GPtr::<GPtr<u8>>::from_addr(pool_cell).read();
    let diff = (e.addr() as u32).wrapping_sub(at::<u32>(pool, 0).read()) as i32;
    let hi = ((i64::from(diff) * i64::from(magic)) >> 32) as i32;
    let q = (hi >> shift).wrapping_add(((hi >> shift) as u32 >> 31) as i32);
    let flags = at::<u32>(pool, 4).read();
    let f = GPtr::<u8>::from_addr(flags.wrapping_add(q as u32) as usize).read();
    i32::from(f).wrapping_add(q << 8)
}

/// `CScriptsForBrains::StartNewStreamedScriptBrain(uint8 index, CEntity* entity, bool hasBrain)`
/// (0x46B270, thiscall, `ret 0xC`): `StartNewStreamedScript(brain.scriptIndex)` (int16 +0), the
/// script's type +0xC9 = the brain's attach type (+2); by the exe's jump table, types 0 / 3 / 5 (peds):
/// script +0x3C = the ped's pool handle, ped flags |= 0x800000 (+0x474), for type 5 script +0x40 =
/// `hasBrain` (byte), then ped flags &= !0x1000000, `RemoveFromWaitingForScriptBrainArray(ped,
/// +0x794)` and +0x794 = -1; types 1 / 4 (objects): script +0x3C = the object's pool handle, object
/// flags |= 0x300000 (+0x140).
#[hook(addr = 0x46b270, conv = thiscall, name = "CScriptsForBrains::StartNewStreamedScriptBrain",
       expect = "0F B6 44 24 04", unverified = "mvpi-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn start_new_streamed_script_brain_port(this: GPtr<u8>, index: u32, e: GPtr<u8>, has_brain: u32) {
    let b = brain(this, index as u8);
    let s = start_new_streamed_script(GPtr::from_addr(STREAMED_SCRIPTS), i32::from(at::<i16>(b, 0).read()));
    at::<u8>(s, 0xC9).write(at::<u8>(b, 2).read());
    match at::<i8>(b, 2).read() {
        0 | 3 | 5 => {
            at::<i32>(s, 0x3C).write(pool_ref(PED_POOL, e, 0x41EE_7CA7, 9));
            let f = at::<u32>(e, 0x474).read();
            at::<u32>(e, 0x474).write(f | 0x80_0000);
            if at::<u8>(b, 2).read() == 5 {
                at::<u32>(s, 0x40).write(has_brain & 0xFF);
            }
            let f = at::<u32>(e, 0x474).read() & 0xFEFF_FFFF;
            let idx = u32::from(at::<u16>(e, 0x794).read());
            at::<u32>(e, 0x474).write(f);
            remove_from_waiting_for_script_brain_array(e, idx);
            at::<u16>(e, 0x794).write(0xFFFF);
        }
        1 | 4 => {
            at::<i32>(s, 0x3C).write(pool_ref(OBJECT_POOL, e, 0x13E2_2CBD, 5));
            let f = at::<u32>(e, 0x140).read();
            at::<u32>(e, 0x140).write(f | 0x30_0000);
        }
        _ => {}
    }
}

/// The entity of pool handle `h` in the pool at `cell` (entries of `size` bytes), else null.
fn pool_at(cell: usize, size: isize, h: i32) -> GPtr<u8> {
    let pool = GPtr::<GPtr<u8>>::from_addr(cell).read();
    let flags = at::<GPtr<u8>>(pool, 4).read();
    let i = h >> 8;
    if at::<u8>(flags, i as isize).read() == h as u8 {
        at::<GPtr<u8>>(pool, 0).read().byte_add(size * i as isize)
    } else {
        GPtr::NULL
    }
}

/// `CRunningScript::ShutdownThisScript()` (0x465AA0, thiscall): active +0xC4 = 0; an external script
/// (+0xC7) decrements the status byte (+4) of its streamed script
/// (`GetStreamedScriptWithThisStartAddress(base ip +0x10)`). By the exe's jump table on the type +0xC9:
/// 0 / 2 / 3 / 5 → the ped of local variable 0 (mission locals 0xA48960 when +0xDC is set, else the
/// script's +0x3C): flags &= !0x800000 (+0x474), and for type 5 a new 8-byte task (`operator new(8)`,
/// constructor 0x463640 when non-null) goes to `CScriptedBrainTaskStore::SetTask(ped, task)`; 1 / 4 →
/// the object of local variable 0: flags (+0x140) bit 0x100000 set, 0x200000 cleared.
#[hook(addr = 0x465aa0, conv = thiscall, name = "CRunningScript::ShutdownThisScript", expect = "6A FF 68 FB 7C 83 00",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "allocates a task (task pool) for type-5 brains")]
pub fn shutdown_this_script(this: GPtr<u8>) {
    at::<u8>(this, 0xC4).write(0);
    if at::<u8>(this, 0xC7).read() != 0 {
        let i = get_streamed_script_with_this_start_address(GPtr::from_addr(STREAMED_SCRIPTS), at::<u32>(this, 0x10).read());
        let c = GPtr::<u8>::from_addr(((i as u32) << 5).wrapping_add(STREAMED_SCRIPTS as u32 + 4) as usize);
        c.write(c.read().wrapping_sub(1));
    }
    let ty = at::<i8>(this, 0xC9).read();
    let local0 = || {
        let locals: GPtr<u8> =
            if at::<u8>(this, 0xDC).read() != 0 { GPtr::from_addr(MISSION_LOCALS) } else { this.byte_add(0x3C) };
        at::<i32>(locals, 0).read()
    };
    match ty {
        0 | 2 | 3 | 5 => {
            let ped = pool_at(PED_POOL, 0x7C4, local0());
            if ped.is_null() {
                return;
            }
            let f = at::<u32>(ped, 0x474).read();
            at::<u32>(ped, 0x474).write(f & 0xFF7F_FFFF);
            if at::<u8>(this, 0xC9).read() == 5 {
                let mem = task_new(8);
                let t = if mem.is_null() { GPtr::NULL } else { brain_finish_task_ctor(mem) };
                brain_task_store_set_task(ped, t);
            }
        }
        1 | 4 => {
            let obj = pool_at(OBJECT_POOL, 0x19C, local0());
            if obj.is_null() {
                return;
            }
            let f = at::<u32>(obj, 0x140).read();
            at::<u32>(obj, 0x140).write((f & 0xFFDF_FFFF) | 0x10_0000);
        }
        _ => {}
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_START_NEW_STREAMED_SCRIPT_BRAIN_PORT,
    HOOK_SHUTDOWN_THIS_SCRIPT,
    HOOK_START_OR_REQUEST_NEW_STREAMED_SCRIPT_BRAIN,
    HOOK_CHECK_IF_NEW_ENTITY_NEEDS_SCRIPT,
];
