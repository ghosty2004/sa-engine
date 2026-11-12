//! SCRIPT-5: callees of the opcodes MAIN and the Grove Street missions (INTRO1 .. SWEET7) run that were still on the
//! original: `CRunningScript::GetPadState`, `CTaskSequences::GetAvailableSlot`,
//! `CDecisionMakerTypesFileLoader::GetPedDMName`, `CTheScripts::RemoveScriptTextureDictionary`.
//! Ported from the 1.0 US bytes (gta-reversed as a reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::vm::Scr;

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// CRT `sprintf(char*, const char*, ...)` (0x821BB5), called with the format only
    fn crt_sprintf(out: GPtr<u8>, fmt: u32) -> i32 = cdecl 0x821BB5 => unported;
    /// `CSprite2d::Delete()` (0x727240)
    fn sprite2d_delete(s: GPtr<u8>) = thiscall 0x727240 => unported;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    fn txd_find_slot(name: u32) -> i32 = cdecl 0x731850 => unported;
    /// `CTxdStore::RemoveTxd(int32)` (0x731E90)
    fn txd_remove(slot: i32) = cdecl 0x731E90 => unported;
    /// `CTaskComplexEnterCar::~CTaskComplexEnterCar()` (0x63DFA0)
    fn enter_car_dtor(t: GPtr<u8>) = thiscall 0x63DFA0 => unported;
    /// `CTask::CTask()` (0x61A390)
    fn task_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CEntity::RegisterReference(CEntity** slot)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `CTaskSimpleTriggerLookAt` vtable (0x86E3CC)
const VT_TRIGGER_LOOK_AT: u32 = 0x86E3CC;

/// `CTaskSimpleTriggerLookAt::CTaskSimpleTriggerLookAt(CEntity* e, int32 time, int32 bone, CVector offset, bool
/// useTorso, float speed, int32 blendTime, int8 priority)` (0x634440, thiscall, `ret 0x28`; unnamed in the
/// catalogue, the type from its vtable 0x86E3CC).
///
/// `CTask::CTask()`, then +0xC time, +0x10 bone, +0x14..+0x1C the offset, byte +0x20 useTorso, +0x24 speed, the
/// vtable, +8 the entity, +0x28 blendTime, byte +0x2D priority; a non-null entity gets `RegisterReference(&this->+8)`
/// and byte +0x2C = 1, else +0x2C = 0. Returns `this`. The original's SEH frame (state 0 after the base constructor)
/// has nothing to unwind; its handler push sits behind a `jmp` into the 0x40384F thunk area, whose rel32 the
/// protection reads at 0x401D0A (sa-hook keycells).
#[hook(addr = 0x634440, conv = thiscall, name = "FUN_00634440", expect = "6A FF E9 08 F4 DC FF",
       this = "CTaskSimpleTriggerLookAt", unverified = "script5-4", route = "L2", hazards = "none", recs = "grove-walk",
       note = "SEH frame not reproduced (no unwind action)")]
#[allow(clippy::too_many_arguments)]
pub fn task_simple_trigger_look_at_ctor(
    this: GPtr<u8>,
    e: GPtr<u8>,
    time: u32,
    bone: u32,
    x: u32,
    y: u32,
    z: u32,
    use_torso: u32,
    speed: u32,
    blend: u32,
    prio: u32,
) -> GPtr<u8> {
    task_ctor(this);
    let w = |off: isize, v: u32| this.byte_add(off).cast::<u32>().write(v);
    w(0xC, time);
    w(0x10, bone);
    w(0x14, x);
    w(0x18, y);
    w(0x1C, z);
    this.byte_add(0x20).write(use_torso as u8);
    w(0x24, speed);
    w(0x0, VT_TRIGGER_LOOK_AT);
    w(0x8, e.addr() as u32);
    w(0x28, blend);
    this.byte_add(0x2D).write(prio as u8);
    if e.is_null() {
        this.byte_add(0x2C).write(0);
    } else {
        entity_register_reference(e, this.byte_add(8));
        this.byte_add(0x2C).write(1);
    }
    this
}

/// `CPointRoute::AddRoutePoint(const CVector&)` (stub 0x48E180 → 0x1569990, thiscall, `ret 4`).
///
/// With fewer than 8 points (dword +0) the vector is copied to `+4 + 12 * count`, the count incremented and AL = 1;
/// otherwise AL = 0.
#[hook(addr = 0x48e180, conv = thiscall, name = "thunk_FUN_01569990", expect = "E9 0B B8 0D 01",
       this = "CPointRoute", unverified = "script5-3", route = "L2", hazards = "none", recs = "grove-walk",
       note = "AL only; the original's upper EAX is the copied z or the count")]
pub fn point_route_add_route_point(this: GPtr<u8>, p: GPtr<u32>) -> u8 {
    let n = this.cast::<i32>().read();
    if n >= 8 {
        return 0;
    }
    let d = this.byte_add(4 + 12 * n as isize).cast::<u32>();
    d.write(p.read());
    d.byte_add(4).write(p.byte_add(4).read());
    d.byte_add(8).write(p.byte_add(8).read());
    this.cast::<i32>().write(n + 1);
    1
}

/// `CTaskComplexEnterCarAsPassenger` vtable (0x86EADC)
const VT_ENTER_CAR_AS_PASSENGER: u32 = 0x86EADC;

/// `CTaskComplexEnterCarAsPassenger::~CTaskComplexEnterCarAsPassenger()` (0x640380, thiscall): the vtable, then a
/// tail jump to `CTaskComplexEnterCar::~CTaskComplexEnterCar()` (0x63DFA0).
#[hook(addr = 0x640380, conv = thiscall, name = "FUN_00640380",
       expect = "C7 01 DC EA 86 00", this = "CTaskComplexEnterCarAsPassenger", unverified = "script5-3", route = "L2",
       hazards = "none", recs = "grove-walk")]
pub fn task_enter_car_as_passenger_dtor(this: GPtr<u8>) {
    this.cast::<u32>().write(VT_ENTER_CAR_AS_PASSENGER);
    enter_car_dtor(this);
}

/// `CRunningScript::GetPadState(uint16 pad, uint16 button)` (0x485B10, thiscall, `ret 8`; `this` unused).
///
/// `CPad::GetPad(pad)` (zero-extended), then by `button` (zero-extended, jump table 0x485BC4): 0..19 → the word at
/// `+2 * button` of the pad (the `CControllerState` words), anything else 0 (`xor ax, ax`). AX only: the original's
/// upper EAX half is the pad pointer's (or GetPad's for the default).
#[hook(addr = 0x485b10, conv = thiscall, name = "CRunningScript::GetPadState", expect = "0F B7 44 24 04 50",
       this = "CRunningScript", unverified = "script5-2", route = "L2", hazards = "none", recs = "grove-walk",
       note = "AX only; the original's upper EAX half is the pad pointer's")]
pub fn running_script_get_pad_state(this: Scr, pad: u32, button: u32) -> u16 {
    let _ = this;
    let p = get_pad(i32::from(pad as u16));
    let b = button as u16;
    if b > 0x13 {
        return 0;
    }
    p.byte_add(2 * b as isize).cast::<u16>().read()
}

/// `CTaskSequences::ms_bIsOpened` (0xC17898, 64 bytes)
const SEQ_OPENED: usize = 0xC17898;
/// `CTaskSequences::ms_taskSequence` (0xC17900, 64 x 0x40)
const SEQ_TASKS: usize = 0xC17900;

/// `CTaskSequences::GetAvailableSlot(bool forScript2)` (0x632E00, cdecl).
///
/// The byte argument selects slots 32..63 (set) or 0..31; the first slot that is not opened (byte 0xC17898 + i) and
/// whose sequence's first dword (0xC17900 + 0x40 * i, the first task) is 0 is returned, else -1.
#[hook(addr = 0x632e00, conv = cdecl, name = "CTaskSequences::GetAvailableSlot", expect = "8A 4C 24 04 53",
       unverified = "script5-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn task_sequences_get_available_slot(second: u32) -> i32 {
    let (lo, hi) = if second as u8 != 0 { (0x20, 0x40) } else { (0, 0x20) };
    let mut i = lo;
    while i < hi {
        if GPtr::<u8>::from_addr(SEQ_OPENED + i).read() == 0 && GPtr::<u32>::from_addr(SEQ_TASKS + 0x40 * i).read() == 0 {
            return i as i32;
        }
        i += 1;
    }
    -1
}

/// `CDecisionMakerTypesFileLoader::ms_PedDMNames` (0x8D2364, `const char*[]`)
const PED_DM_NAMES: usize = 0x8D2364;

/// `CDecisionMakerTypesFileLoader::GetPedDMName(int32 index, char* out)` (0x600860, cdecl): `sprintf(out,
/// ms_PedDMNames[index])` (the name is the format; no bounds check).
#[hook(addr = 0x600860, conv = cdecl, name = "CDecisionMakerTypesFileLoader::GetPedDMName", expect = "8B 44 24 04 8B 0C 85 64 23 8D 00",
       unverified = "script5-2", route = "L2", hazards = "none", recs = "grove-walk",
       note = "EAX = sprintf's result (the original returns it unchanged)")]
pub fn decision_maker_get_ped_dm_name(index: i32, out: GPtr<u8>) -> i32 {
    let name = GPtr::<u32>::from_addr(PED_DM_NAMES).byte_add(4 * index as isize).read();
    crt_sprintf(out, name)
}

/// `CTheScripts::ScriptSprites` (0xA94B68, 128 x `CSprite2d`)
const SCRIPT_SPRITES: usize = 0xA94B68;
/// The string "script" (0x859D4C)
const S_SCRIPT: u32 = 0x859D4C;
/// `CTxdStore::ms_pTxdPool` (0xC8800C; 12-byte slots)
const TXD_POOL: usize = 0xC8800C;

/// `CTheScripts::RemoveScriptTextureDictionary()` (0x465A40, cdecl).
///
/// `CSprite2d::Delete` on the 128 script sprites, then `slot = CTxdStore::FindTxdSlot("script")`; when it is not -1
/// and the pool slot is in use (flags byte not negative) with a non-null texture dictionary (first dword),
/// `CTxdStore::RemoveTxd(slot)`. The sprite loop's start (`mov esi, 0xA94B68`) sits behind a `jmp` into the
/// 0x401554 thunk area and back.
#[hook(addr = 0x465a40, conv = cdecl, name = "CTheScripts::RemoveScriptTextureDictionary", expect = "56 57 E9 0D BB F9 FF",
       unverified = "script5-2", route = "L2", hazards = "none", recs = "grove-walk",
       note = "the original jumps through 0x401554 to load the sprite array address")]
pub fn the_scripts_remove_script_texture_dictionary() {
    for i in 0..0x80 {
        sprite2d_delete(GPtr::from_addr(SCRIPT_SPRITES + 4 * i));
    }
    let slot = txd_find_slot(S_SCRIPT);
    if slot == -1 {
        return;
    }
    let pool = GPtr::<GPtr<u8>>::from_addr(TXD_POOL).read();
    let flags = pool.byte_add(4).cast::<GPtr<u8>>().read();
    let entry = if (flags.byte_add(slot as isize).read() as i8) < 0 {
        GPtr::<u8>::NULL
    } else {
        pool.cast::<GPtr<u8>>().read().byte_add(slot as isize * 12)
    };
    if entry.cast::<u32>().read() != 0 {
        txd_remove(slot);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RUNNING_SCRIPT_GET_PAD_STATE,
    HOOK_TASK_SEQUENCES_GET_AVAILABLE_SLOT,
    HOOK_DECISION_MAKER_GET_PED_DM_NAME,
    HOOK_THE_SCRIPTS_REMOVE_SCRIPT_TEXTURE_DICTIONARY,
    HOOK_POINT_ROUTE_ADD_ROUTE_POINT,
    HOOK_TASK_ENTER_CAR_AS_PASSENGER_DTOR,
    HOOK_TASK_SIMPLE_TRIGGER_LOOK_AT_CTOR,
    HOOK_TASK_USE_MOBILE_PHONE_STOP,
    HOOK_SUPPRESS_VEHICLE_MODEL,
    HOOK_DONT_SUPPRESS_VEHICLE_MODEL,
];

orig! {
    /// Virtual slot +0x18 of a task (`MakeAbortable(CPed*, int32 priority, const CEvent*)`, AL)
    fn vcall_make_abortable(f: u32, t: GPtr<u8>, ped: GPtr<u8>, prio: u32, ev: u32) -> u8 = thiscall dyn => unported;
}

/// The mobile phone task's stop (0x634A40, thiscall, `ret 4`; unnamed in the catalogue, reached by 0729
/// TASK_USE_MOBILE_PHONE with "off"). Once only (byte +0x1D): the byte is set, then `MakeAbortable(ped, 0, NULL)`
/// (vtable +0x18) and its AL is returned; a second call returns the byte (AL, nonzero) without calling.
#[hook(addr = 0x634a40, conv = thiscall, name = "FUN_00634a40", expect = "8A 41 1D 84 C0 75 12",
       this = "CTaskComplexUseMobilePhone", unverified = "script5-3", route = "L2", hazards = "none", recs = "grove-walk",
       note = "name from its use by 0729; AL only (the original's upper EAX is stale)")]
pub fn task_use_mobile_phone_stop(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let b = this.byte_add(0x1D).read();
    if b != 0 {
        return b;
    }
    this.byte_add(0x1D).write(1);
    let f = this.cast::<GPtr<u8>>().read().byte_add(0x18).cast::<u32>().read();
    vcall_make_abortable(f, this, ped, 0, 0)
}

/// `CTheScripts::SuppressedVehicleModels` (0xA44940, 40 x int32, -1 = free)
const SUPPRESSED_MODELS: usize = 0xA44940;

/// Add a model to `CTheScripts::SuppressedVehicleModels` (0x46B1A0, cdecl; unnamed in the catalogue, 0732
/// SUPPRESS_CAR_MODEL). Nothing when the model is already listed (byte index loop over 40). Else the first -1 slot
/// takes it; with none the index ends at 40 and the original writes one past the array (0xA449E0), as here.
#[hook(addr = 0x46b1a0, conv = cdecl, name = "FUN_0046b1a0", expect = "56 8B 74 24 08 32 C0",
       unverified = "script5-3", route = "L2", hazards = "none", recs = "grove-walk",
       note = "name from its use by 0732; a full array writes slot 40 (one past the end), as the original")]
pub fn suppress_vehicle_model(model: i32) {
    let slot = |i: usize| GPtr::<i32>::from_addr(SUPPRESSED_MODELS + 4 * i);
    for i in 0..0x28 {
        if slot(i).read() == model {
            return;
        }
    }
    let mut i = 0;
    while i < 0x28 && slot(i).read() != -1 {
        i += 1;
    }
    slot(i).write(model);
}

/// Remove a model from `CTheScripts::SuppressedVehicleModels` (stub 0x46A7E0 → 0x15624A0, cdecl; unnamed in the
/// catalogue, 0733 DONT_SUPPRESS_CAR_MODEL): every one of the 40 slots holding it becomes -1.
#[hook(addr = 0x46a7e0, conv = cdecl, name = "thunk_FUN_015624a0", expect = "E9 BB 7C 0F 01",
       unverified = "script5-3", route = "L2", hazards = "none", recs = "grove-walk", note = "name from its use by 0733")]
pub fn dont_suppress_vehicle_model(model: i32) {
    for i in 0..0x28 {
        let s = GPtr::<i32>::from_addr(SUPPRESSED_MODELS + 4 * i);
        if s.read() == model {
            s.write(-1);
        }
    }
}
