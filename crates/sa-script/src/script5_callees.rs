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
];
