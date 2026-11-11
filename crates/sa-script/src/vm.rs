//! The `CRunningScript` VM core: operand decoding (`CollectParameters`, `StoreParameters`,
//! `CollectNextParameterWithoutIncreasingPC`, `ReadParametersForNewlyStartedScript`,
//! `ReadTextLabelFromScript`), variable addressing (`GetPointerToLocalVariable`,
//! `GetPointerToLocalArrayElement`, `ReadArrayInformation`, `GetPointerToScriptVariable`,
//! `GetIndexOfGlobalVariable`), `UpdateCompareFlag`, `UpdatePC`, and the command loop
//! (`ProcessOneCommand`, `Process`).
//!
//! Ported from the 1.0 US bytes (gta-reversed `RunningScript.cpp` as reading aid). Where the original
//! inlines a helper (the local-variable and array-index lookups inside the collectors, `UpdatePC`
//! inside `Process`) the port inlines it too; where it calls one (`CollectNextParameter...` calls
//! `GetPointerToLocalVariable`, `ReadArrayInformation`, `GetPointerToLocalArrayElement`) the port
//! calls it through `orig!`, so each function's hook toggle governs the calls made from Rust and the
//! L2 callee sequence is the original's.
//!
//! Operand type bytes (`eScriptParameterType`): 1 int32, 2 global var, 3 local var, 4 int8, 5 int16,
//! 6 float, 7 global array, 8 local array, 9 short string (8 bytes), 0xA/0xB global/local short-string
//! var, 0xC/0xD global/local short-string array, 0xE pascal string, 0xF long string (16 bytes),
//! 0x10/0x11 global/local long-string var, 0x12/0x13 global/local long-string array.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GameData};
use sa_core::{hook, orig};

/// A `CRunningScript*`.
pub type Scr = GPtr<u8>;

/// `CTheScripts::ScriptSpace` size (`MAIN_SCRIPT_SIZE` 200000 + `MISSION_SCRIPT_SIZE` 69000).
pub const SCRIPT_SPACE_SIZE: usize = 269_000;

sa_core::globals! {
    list GLOBALS;
    /// `ScriptParams` (`tScriptParam[32]`)
    pub static SCRIPT_PARAMS @ 0xA43C78: [u32; 32] = [0; 32];
    /// `CTheScripts::FailCurrentMission`
    pub static FAIL_CURRENT_MISSION @ 0xA444B0: u8 = 0;
    /// `CTheScripts::CommandsExecuted`
    pub static COMMANDS_EXECUTED @ 0xA447F4: u16 = 0;
    /// `CTheScripts::LocalVariablesForCurrentMission` (`tScriptParam[1024]`)
    pub static MISSION_LOCALS @ 0xA48960: [u32; 1024] = [0; 1024];
    /// `CTheScripts::ScriptSpace` (MAIN.SCM, then the mission block)
    pub static SCRIPT_SPACE @ 0xA49960: [u8; SCRIPT_SPACE_SIZE] = [0; SCRIPT_SPACE_SIZE];
    /// `CHud::m_BigMessage[1][0]` (cleared when a cutscene is skipped)
    pub static BIG_MESSAGE_1 @ 0xBAAD40: u8 = 0;
    /// `CRunningScript::s_OriginalCommandHandlerTable`: the 27 `ProcessCommandsXToY` switches
    pub static COMMAND_HANDLERS @ 0x8A6168: [u32; 27] = [
        0x465E60, 0x466DE0, 0x469390, 0x47C100, 0x47D210, 0x47E090, 0x47F370, 0x47FA30, 0x481300,
        0x483BD0, 0x489500, 0x48A320, 0x48B590, 0x48CDD0, 0x48EAA0, 0x490DB0, 0x493FE0, 0x496E00,
        0x46D050, 0x46B460, 0x472310, 0x470A90, 0x474900, 0x4762D0, 0x478000, 0x47A760, 0x479DA0,
    ];
}

// CRunningScript fields (gta-reversed RunningScript.h, offsets checked against the bytes above).
pub const BASE_IP: isize = 0x10;
pub const IP: isize = 0x14;
pub const IP_STACK: isize = 0x18;
pub const STACK_DEPTH: isize = 0x38;
pub const LOCALS: isize = 0x3C;
pub const COND_RESULT: isize = 0xC5;
pub const USES_MISSION_CLEANUP: isize = 0xC6;
pub const WAKE_TIME: isize = 0xCC;
pub const AND_OR_STATE: isize = 0xD0;
pub const NOT_FLAG: isize = 0xD2;
pub const SCENE_SKIP_IP: isize = 0xD8;
pub const IS_MISSION: isize = 0xDC;

orig! {
    /// `CRunningScript::GetPointerToLocalVariable(int32)` (0x463CA0)
    pub fn get_pointer_to_local_variable_call(s: Scr, loc: i32) -> GPtr<u32> = thiscall 0x463CA0 => get_pointer_to_local_variable;
    /// `CRunningScript::GetPointerToLocalArrayElement(int32, uint16, uint8)` (0x463CC0)
    pub fn get_pointer_to_local_array_element_call(s: Scr, base: i32, idx: u32, size: u32) -> GPtr<u32> = thiscall 0x463CC0 => local_array_element_port;
    /// `CRunningScript::ReadArrayInformation(int32, uint16*, int32*)` (0x463CF0)
    pub fn read_array_information_call(s: Scr, update_ip: i32, out_base: GPtr<u16>, out_idx: GPtr<i32>) = thiscall 0x463CF0 => read_array_information;
    /// `CRunningScript::CollectParameters(int16)` (0x464080)
    pub fn collect_parameters_call(s: Scr, count: u32) = thiscall 0x464080 => collect_parameters;
    /// `CRunningScript::StoreParameters(int16)` (0x464370)
    pub fn store_parameters_call(s: Scr, count: u32) = thiscall 0x464370 => store_parameters;
    /// `CRunningScript::ReadParametersForNewlyStartedScript(CRunningScript*)` (0x464500)
    pub fn read_parameters_for_newly_started_script_call(s: Scr, n: Scr) = thiscall 0x464500 => read_parameters_for_newly_started_script;
    /// `CRunningScript::GetPointerToScriptVariable(eScriptVariableType)` (0x464790)
    pub fn get_pointer_to_script_variable_call(s: Scr, ty: i32) -> GPtr<u32> = thiscall 0x464790 => get_pointer_to_script_variable;
    /// `CRunningScript::UpdatePC(int32)` (0x464DA0)
    pub fn update_pc_call(s: Scr, new_ip: i32) = thiscall 0x464DA0 => update_pc;
    /// `CRunningScript::UpdateCompareFlag(bool)` (0x4859D0)
    pub fn update_compare_flag_call(s: Scr, state: u32) = thiscall 0x4859D0 => update_compare_flag;
    /// CRT `strncpy` (0x821F40)
    pub fn strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => strncpy_port;
    /// `CCutsceneMgr::IsCutsceneSkipButtonBeingPressed` (0x4D5D10)
    pub fn is_cutscene_skip_button_being_pressed() -> bool = cdecl 0x4D5D10 => unported;
    /// `CRunningScript::DoDeathArrestCheck` (0x485A50)
    pub fn do_death_arrest_check(s: Scr) = thiscall 0x485A50 => unported;
    /// `CTheScripts::ReinitialiseSwitchStatementData` (0x470370)
    pub fn reinitialise_switch_statement_data() = cdecl 0x470370 => unported;
    /// A `ProcessCommandsXToY` switch from `s_OriginalCommandHandlerTable` (thiscall, command id on the
    /// stack, AL = OpcodeResult)
    pub fn call_command_handler(f: u32, s: Scr, command: i32) -> i8 = thiscall dyn => unported;
}

/// CRT `strncpy` semantics (phases B/C path of [`strncpy`]): copies up to `n` bytes, stops after the
/// terminating NUL and pads the rest of the `n` bytes with zeros; returns `dst`.
pub fn strncpy_port(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> {
    let mut i = 0u32;
    while i < n {
        let c = src.byte_add(i as isize).read();
        dst.byte_add(i as isize).write(c);
        i += 1;
        if c == 0 {
            break;
        }
    }
    while i < n {
        dst.byte_add(i as isize).write(0);
        i += 1;
    }
    dst
}

#[inline]
fn rd<T: GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}

#[inline]
fn wr<T: GameData>(p: GPtr<u8>, off: isize, v: T) {
    p.byte_add(off).cast::<T>().write(v)
}

/// `m_IP`
#[inline]
pub fn ip(s: Scr) -> GPtr<u8> {
    rd::<GPtr<u8>>(s, IP)
}

#[inline]
pub fn set_ip(s: Scr, p: GPtr<u8>) {
    wr(s, IP, p)
}

/// Reads a `T` at `m_IP` and advances `m_IP` by its size (`CTheScripts::ReadNBytesFromScript`).
#[inline]
pub fn read_ip<T: GameData>(s: Scr) -> T {
    let p = ip(s);
    let v = p.cast::<T>().read();
    set_ip(s, p.byte_add(core::mem::size_of::<T>() as isize));
    v
}

/// `&ScriptSpace[off]` (32-bit wrapping, as `[reg + 0xA49960]`).
#[inline]
pub fn ss<T>(off: i32) -> GPtr<T> {
    SCRIPT_SPACE.ptr().cast::<u8>().byte_add(off as isize).cast()
}

/// `&ScriptParams[i]`
#[inline]
pub fn param(i: i32) -> GPtr<u32> {
    SCRIPT_PARAMS.ptr().cast::<u32>().wrapping_index(i)
}

/// The inlined `GetPointerToLocalVariable`: `m_ThisMustBeTheOnlyMissionRunning ?
/// &LocalVariablesForCurrentMission[loc] : &m_LocalVars[loc]` (`loc * 4`, 32-bit wrapping).
#[inline]
pub fn local(s: Scr, loc: i32) -> GPtr<u32> {
    if rd::<u8>(s, IS_MISSION) != 0 {
        MISSION_LOCALS.ptr().cast::<u32>().wrapping_index(loc)
    } else {
        s.byte_add(LOCALS).cast::<u32>().wrapping_index(loc)
    }
}

/// The array operand (`scm::ArrayAccess`, 6 bytes at `m_IP`): base offset, index variable, flags
/// (bit 15 = the index variable is global). Returns `(base, index value, ip after the operand)`; the
/// index is read as the original does (global: `ScriptSpace[loc]`, local: the inlined local lookup).
#[inline]
fn array_operand(s: Scr) -> (u16, i32, GPtr<u8>) {
    let p = ip(s);
    let base = p.cast::<u16>().read();
    let loc = p.byte_add(2).cast::<u16>().read();
    let flags = p.byte_add(4).cast::<u16>().read();
    let idx = if flags & 0x8000 != 0 {
        ss::<i32>(i32::from(loc)).read()
    } else {
        local(s, i32::from(loc)).cast::<i32>().read()
    };
    (base, idx, p.byte_add(6))
}

/// `CRunningScript::GetPointerToLocalVariable(int32 loc)` (stub 0x463CA0 → 0x156E2B0).
#[hook(addr = 0x463CA0, conv = thiscall, name = "CRunningScript::GetPointerToLocalVariable", expect = "E9 0B A6 10 01", this = "CRunningScript", preserve = "ecx")]
pub fn get_pointer_to_local_variable(s: Scr, loc: i32) -> GPtr<u32> {
    local(s, loc)
}

/// `CRunningScript::GetPointerToLocalArrayElement(int32 base, uint16 idx, uint8 size)` (0x463CC0):
/// `local(base + idx * size)`; `idx`/`size` are read as a word/byte of their stack slots.
#[hook(addr = 0x463CC0, conv = thiscall, name = "CRunningScript::GetPointerToLocalArrayElement", expect = "0F B6 54 24 0C", this = "CRunningScript", preserve = "ecx")]
pub fn get_pointer_to_local_array_element(s: Scr, base: i32, idx: u32, size: u32) -> GPtr<u32> {
    local_array_element_port(s, base, idx, size)
}

pub fn local_array_element_port(s: Scr, base: i32, idx: u32, size: u32) -> GPtr<u32> {
    let i = ((idx & 0xFFFF) * (size & 0xFF)) as i32;
    local(s, i.wrapping_add(base))
}

/// `CRunningScript::ReadArrayInformation(int32 updateIP, uint16* outBase, int32* outIndex)` (stub
/// 0x463CF0 → 0x156E350): `*outBase` is written before the index variable is read; `m_IP` advances by
/// 6 only when `updateIP != 0`.
#[hook(addr = 0x463CF0, conv = thiscall, name = "CRunningScript::ReadArrayInformation", expect = "E9 5B A6 10 01", this = "CRunningScript", preserve = "ecx")]
pub fn read_array_information(s: Scr, update_ip: i32, out_base: GPtr<u16>, out_idx: GPtr<i32>) {
    let p = ip(s);
    out_base.write(p.cast::<u16>().read());
    let loc = p.byte_add(2).cast::<u16>().read();
    let flags = p.byte_add(4).cast::<u16>().read();
    let idx = if flags & 0x8000 != 0 {
        ss::<i32>(i32::from(loc)).read()
    } else {
        local(s, i32::from(loc)).cast::<i32>().read()
    };
    out_idx.write(idx);
    if update_ip != 0 {
        set_ip(s, p.byte_add(6));
    }
}

/// `CRunningScript::CollectParameters(int16 count)` (0x464080): `count` is tested for zero and then
/// used as an unsigned word (a negative count runs up to 65535 slots). Unknown operand types fill
/// nothing but still use up a `ScriptParams` slot.
#[hook(addr = 0x464080, conv = thiscall, name = "CRunningScript::CollectParameters", expect = "66 8B 44 24 04", this = "CRunningScript", preserve = "ecx")]
pub fn collect_parameters(s: Scr, count: u32) {
    let n = count & 0xFFFF;
    for i in 0..n as i32 {
        let t = read_ip::<u8>(s) as i8;
        let out = param(i);
        match t {
            1 | 6 => out.write(read_ip::<u32>(s)),
            2 => {
                let loc = read_ip::<u16>(s);
                out.write(ss::<u32>(i32::from(loc)).read());
            }
            3 => {
                let loc = read_ip::<u16>(s);
                out.write(local(s, i32::from(loc)).read());
            }
            4 => out.write(read_ip::<u8>(s) as i8 as i32 as u32),
            5 => out.write(read_ip::<u16>(s) as i16 as i32 as u32),
            7 => {
                let (base, idx, next) = array_operand(s);
                set_ip(s, next);
                out.write(ss::<u32>(i32::from(base).wrapping_add(idx.wrapping_mul(4))).read());
            }
            8 => {
                let (base, idx, next) = array_operand(s);
                set_ip(s, next);
                out.write(local(s, i32::from(base) + (idx & 0xFFFF)).read());
            }
            _ => {}
        }
    }
}

/// `CRunningScript::CollectNextParameterWithoutIncreasingPC()` (0x464250): the value of the next
/// operand (-1 for an unknown type); `m_IP` is restored. Calls `GetPointerToLocalVariable`,
/// `ReadArrayInformation(0, ...)` and `GetPointerToLocalArrayElement` as the original does.
#[hook(addr = 0x464250, conv = thiscall, name = "CRunningScript::CollectNextParameterWithoutIncreasingPC", expect = "83 EC 08 56 57", this = "CRunningScript", preserve = "ecx")]
pub fn collect_next_parameter_without_increasing_pc(s: Scr) -> i32 {
    let start = ip(s);
    let t = read_ip::<u8>(s) as i8;
    let r = match t {
        1 | 6 => read_ip::<i32>(s),
        2 => {
            let loc = read_ip::<u16>(s);
            ss::<i32>(i32::from(loc)).read()
        }
        3 => {
            let loc = read_ip::<u16>(s);
            get_pointer_to_local_variable_call(s, i32::from(loc))
                .cast::<i32>()
                .read()
        }
        4 => read_ip::<u8>(s) as i8 as i32,
        5 => read_ip::<u16>(s) as i16 as i32,
        7 | 8 => {
            let mut v = [0u32; 2];
            let base_p = GPtr::from_raw(v.as_mut_ptr()).cast::<u16>();
            let idx_p = GPtr::from_raw(v.as_mut_ptr()).byte_add(4).cast::<i32>();
            read_array_information_call(s, 0, base_p, idx_p);
            let base = base_p.read();
            let idx = idx_p.read();
            if t == 7 {
                ss::<i32>(i32::from(base).wrapping_add(idx.wrapping_mul(4))).read()
            } else {
                get_pointer_to_local_array_element_call(s, i32::from(base), idx as u32, 1)
                    .cast::<i32>()
                    .read()
            }
        }
        _ => -1,
    };
    set_ip(s, start);
    r
}

/// `CRunningScript::StoreParameters(int16 count)` (0x464370): `count <= 0` (signed) stores nothing.
/// Types 2/3/7/8 store `ScriptParams[i]`; 4/5/6 consume only the type byte; others store nothing.
#[hook(addr = 0x464370, conv = thiscall, name = "CRunningScript::StoreParameters", expect = "66 8B 44 24 04", this = "CRunningScript", preserve = "ecx")]
pub fn store_parameters(s: Scr, count: u32) {
    let n = count as u16 as i16;
    for i in 0..i32::from(n) {
        let t = read_ip::<u8>(s) as i8;
        let v = param(i);
        match t {
            2 => {
                let loc = read_ip::<u16>(s);
                ss::<u32>(i32::from(loc)).write(v.read());
            }
            3 => {
                let loc = read_ip::<u16>(s);
                local(s, i32::from(loc)).write(v.read());
            }
            7 => {
                let (base, idx, next) = array_operand(s);
                set_ip(s, next);
                ss::<u32>(i32::from(base).wrapping_add(idx.wrapping_mul(4))).write(v.read());
            }
            8 => {
                let (base, idx, next) = array_operand(s);
                set_ip(s, next);
                local(s, i32::from(base) + (idx & 0xFFFF)).write(v.read());
            }
            _ => {}
        }
    }
}

/// `CRunningScript::ReadParametersForNewlyStartedScript(CRunningScript* newScript)` (0x464500):
/// operands up to the 0 type byte go to `newScript->m_LocalVars[0..]` (no bound check).
#[hook(addr = 0x464500, conv = thiscall, name = "CRunningScript::ReadParametersForNewlyStartedScript", expect = "8B 51 14 8A 02", this = "CRunningScript", preserve = "ecx")]
pub fn read_parameters_for_newly_started_script(s: Scr, new_script: Scr) {
    let mut out = new_script.byte_add(LOCALS).cast::<u32>();
    let mut t = read_ip::<u8>(s);
    while t != 0 {
        match t as i8 {
            1 | 6 => out.write(read_ip::<u32>(s)),
            2 => {
                let loc = read_ip::<u16>(s);
                out.write(ss::<u32>(i32::from(loc)).read());
            }
            3 => {
                let loc = read_ip::<u16>(s);
                out.write(local(s, i32::from(loc)).read());
            }
            4 => out.write(read_ip::<u8>(s) as i8 as i32 as u32),
            5 => out.write(read_ip::<u16>(s) as i16 as i32 as u32),
            7 => {
                let (base, idx, next) = array_operand(s);
                set_ip(s, next);
                out.write(ss::<u32>(i32::from(base).wrapping_add(idx.wrapping_mul(4))).read());
            }
            8 => {
                let (base, idx, next) = array_operand(s);
                set_ip(s, next);
                out.write(local(s, i32::from(base) + (idx & 0xFFFF)).read());
            }
            _ => {}
        }
        out = out.wrapping_index(1);
        t = read_ip::<u8>(s);
    }
}

/// Copies `n` bytes from `m_IP` to `buf`, advancing `m_IP` byte by byte.
fn copy_from_ip(s: Scr, buf: GPtr<u8>, n: u32) {
    for k in 0..n {
        let b = read_ip::<u8>(s);
        buf.byte_add(k as isize).write(b);
    }
}

/// `CRunningScript::ReadTextLabelFromScript(char* buffer, uint8 length)` (0x463D50). Differs from
/// gta-reversed (the exe wins): the static long string (0xF) is copied byte by byte, not with
/// `strncpy`: `min(length, 16)` raw bytes, then `m_IP` skips the rest of the 16 (`length == 0` copies
/// nothing and skips 16); the long-string variable/array forms `strncpy` `length` bytes when
/// `length < 16`, else 16; the pascal string's length byte is signed for the copy and unsigned for
/// the zero fill.
#[hook(addr = 0x463D50, conv = thiscall, name = "CRunningScript::ReadTextLabelFromScript", expect = "8B 41 14 83 EC 08", this = "CRunningScript")]
pub fn read_text_label_from_script(s: Scr, buf: GPtr<u8>, length: u32) {
    let len = length as u8;
    let long_n = if len < 0x10 { u32::from(len) } else { 0x10 };
    let t = read_ip::<u8>(s) as i8;
    match t {
        0x9 => copy_from_ip(s, buf, 8),
        0xA => {
            let loc = read_ip::<u16>(s);
            strncpy(buf, ss(i32::from(loc)), 8);
        }
        0xB => {
            let loc = read_ip::<u16>(s);
            let p = get_pointer_to_local_variable_call(s, i32::from(loc));
            strncpy(buf, p.cast(), 8);
        }
        0xC | 0x12 => {
            let mut v = [0u32; 2];
            let base_p = GPtr::from_raw(v.as_mut_ptr()).cast::<u16>();
            let idx_p = GPtr::from_raw(v.as_mut_ptr()).byte_add(4).cast::<i32>();
            read_array_information_call(s, 1, base_p, idx_p);
            let base = i32::from(base_p.read());
            let idx = idx_p.read();
            if t == 0xC {
                strncpy(buf, ss(base.wrapping_add(idx.wrapping_mul(8))), 8);
            } else {
                strncpy(buf, ss(idx.wrapping_mul(16).wrapping_add(base)), long_n);
            }
        }
        0xD | 0x13 => {
            let mut v = [0u32; 2];
            let base_p = GPtr::from_raw(v.as_mut_ptr()).cast::<u16>();
            let idx_p = GPtr::from_raw(v.as_mut_ptr()).byte_add(4).cast::<i32>();
            read_array_information_call(s, 1, base_p, idx_p);
            let base = i32::from(base_p.read());
            let idx = idx_p.read() as u32;
            if t == 0xD {
                let p = get_pointer_to_local_array_element_call(s, base, idx, 2);
                strncpy(buf, p.cast(), 8);
            } else {
                let p = get_pointer_to_local_array_element_call(s, base, idx, 4);
                strncpy(buf, p.cast(), long_n);
            }
        }
        0xE => {
            let n = read_ip::<u8>(s);
            let signed = i32::from(n as i8);
            let mut k: u8 = 0;
            if signed > 0 {
                loop {
                    let b = read_ip::<u8>(s);
                    buf.byte_add(isize::from(k)).write(b);
                    k = k.wrapping_add(1);
                    if i32::from(k) >= signed {
                        break;
                    }
                }
            }
            if n < len {
                for j in n..len {
                    buf.byte_add(isize::from(j)).write(0);
                }
            }
        }
        0xF => {
            if len < 0x10 {
                copy_from_ip(s, buf, u32::from(len));
                let p = ip(s);
                set_ip(s, p.byte_add(isize::from(0x10 - len)));
            } else {
                copy_from_ip(s, buf, 0x10);
            }
        }
        0x10 => {
            let loc = read_ip::<u16>(s);
            strncpy(buf, ss(i32::from(loc)), long_n);
        }
        0x11 => {
            let loc = read_ip::<u16>(s);
            let p = get_pointer_to_local_variable_call(s, i32::from(loc));
            strncpy(buf, p.cast(), long_n);
        }
        _ => {}
    }
}

/// `CRunningScript::GetIndexOfGlobalVariable()` (stub 0x464700 → 0x156F340): type 2 → the word
/// operand; type 7 → `base + 4 * index` (array operand inlined, `m_IP` += 6). Any other type returns
/// the low word of `this` (the original returns the word it pushed to save ECX) and consumes only the
/// type byte. The original's upper EAX bits are not part of the `uint16` result (type 7: EDI's upper
/// half of the caller); the port returns the word zero-extended.
#[hook(addr = 0x464700, conv = thiscall, name = "CRunningScript::GetIndexOfGlobalVariable", expect = "E9 3B AC 10 01", this = "CRunningScript", preserve = "ecx")]
pub fn get_index_of_global_variable(s: Scr) -> u16 {
    let t = read_ip::<u8>(s) as i8;
    match t {
        2 => read_ip::<u16>(s),
        7 => {
            let (base, idx, next) = array_operand(s);
            set_ip(s, next);
            (u32::from(base).wrapping_add((idx as u32).wrapping_mul(4))) as u16
        }
        _ => s.addr() as u16,
    }
}

/// `CRunningScript::GetPointerToScriptVariable(eScriptVariableType)` (0x464790): types 2/0xA/0x10
/// global, 3/0xB/0x11 local (inlined), 7/0xC/0x12 global arrays (element 4/8/16 bytes, via
/// `ReadArrayInformation(1)`), 8/0xD/0x13 local arrays (via `GetPointerToLocalArrayElement`, size
/// 1/2/4); any other type returns NULL. The argument is not read.
#[hook(addr = 0x464790, conv = thiscall, name = "CRunningScript::GetPointerToScriptVariable", expect = "8B 41 14 83 EC 08", this = "CRunningScript")]
pub fn get_pointer_to_script_variable(s: Scr, _ty: i32) -> GPtr<u32> {
    let t = read_ip::<u8>(s) as i8;
    match t {
        2 | 0xA | 0x10 => {
            let loc = read_ip::<u16>(s);
            ss(i32::from(loc))
        }
        3 | 0xB | 0x11 => {
            let loc = read_ip::<u16>(s);
            local(s, i32::from(loc))
        }
        7 | 0xC | 0x12 | 8 | 0xD | 0x13 => {
            let mut v = [0u32; 2];
            let base_p = GPtr::from_raw(v.as_mut_ptr()).cast::<u16>();
            let idx_p = GPtr::from_raw(v.as_mut_ptr()).byte_add(4).cast::<i32>();
            read_array_information_call(s, 1, base_p, idx_p);
            let base = i32::from(base_p.read());
            let idx = idx_p.read();
            match t {
                0x12 => ss(idx.wrapping_mul(16).wrapping_add(base)),
                0xC => ss(base.wrapping_add(idx.wrapping_mul(8))),
                7 => ss(base.wrapping_add(idx.wrapping_mul(4))),
                0x13 => get_pointer_to_local_array_element_call(s, base, idx as u32, 4),
                0xD => get_pointer_to_local_array_element_call(s, base, idx as u32, 2),
                _ => get_pointer_to_local_array_element_call(s, base, idx as u32, 1),
            }
        }
        _ => GPtr::NULL,
    }
}

/// `CRunningScript::UpdatePC(int32 newIP)` (stub 0x464DA0 → 0x15620B0): `newIP >= 0` →
/// `&ScriptSpace[newIP]`, else `m_BaseIP - newIP`.
#[hook(addr = 0x464DA0, conv = thiscall, name = "CRunningScript::UpdatePC", expect = "E9 0B D3 0F 01", this = "CRunningScript", preserve = "ecx")]
pub fn update_pc(s: Scr, new_ip: i32) {
    set_ip(s, update_pc_target(s, new_ip));
}

#[inline]
fn update_pc_target(s: Scr, new_ip: i32) -> GPtr<u8> {
    if new_ip >= 0 {
        ss(new_ip)
    } else {
        rd::<GPtr<u8>>(s, BASE_IP).byte_add(-(new_ip as isize))
    }
}

/// `CRunningScript::UpdateCompareFlag(bool)` (0x4859D0). The argument byte is used as is (not
/// normalised) unless `m_NotFlag` is set, which makes it `arg == 0`; AND/OR states 1..8 / 21..28 count
/// down to 0; any other state leaves the flag unchanged.
#[hook(addr = 0x4859D0, conv = thiscall, name = "CRunningScript::UpdateCompareFlag", expect = "8A 81 D2 00 00 00", this = "CRunningScript", preserve = "ecx")]
pub fn update_compare_flag(s: Scr, state: u32) {
    let arg = state as u8;
    let v = if rd::<u8>(s, NOT_FLAG) != 0 {
        u8::from(arg == 0)
    } else {
        arg
    };
    let st = rd::<u16>(s, AND_OR_STATE);
    if st == 0 {
        wr(s, COND_RESULT, v);
    } else if (1..=8).contains(&st) {
        wr(s, COND_RESULT, rd::<u8>(s, COND_RESULT) & v);
        wr(s, AND_OR_STATE, if st == 1 { 0 } else { st - 1 });
    } else if (21..=28).contains(&st) {
        wr(s, COND_RESULT, rd::<u8>(s, COND_RESULT) | v);
        wr(s, AND_OR_STATE, if st == 21 { 0 } else { st - 1 });
    }
}

/// One command: `CommandsExecuted++`, read the command word, `m_IP += 2`, `m_NotFlag` = bit 15, call
/// `s_OriginalCommandHandlerTable[id / 100]` with the masked id.
#[inline]
fn dispatch(s: Scr) -> i8 {
    COMMANDS_EXECUTED.set(COMMANDS_EXECUTED.get().wrapping_add(1));
    let w = read_ip::<u16>(s);
    wr(s, NOT_FLAG, (w >> 15) as u8);
    let id = i32::from(w & 0x7FFF);
    let f = COMMAND_HANDLERS
        .ptr()
        .cast::<u32>()
        .wrapping_index(id / 100)
        .read();
    call_command_handler(f, s, id)
}

/// `CRunningScript::ProcessOneCommand()` (0x469EB0). Not called by the exe (inlined into `Process`);
/// returns the switch's AL.
#[hook(addr = 0x469EB0, conv = thiscall, name = "CRunningScript::ProcessOneCommand", expect = "66 FF 05 F4 47 A4 00", this = "CRunningScript")]
pub fn process_one_command(s: Scr) -> i8 {
    dispatch(s)
}

/// `CRunningScript::Process()` (0x469F00): cutscene skip (inlined `UpdatePC`, `CHud::m_BigMessage[1][0]
/// = 0`, wake time 0), death/arrest check, mission failure unwinds the gosub stack to its bottom
/// entry, `ReinitialiseSwitchStatementData`, then commands until a switch returns non-zero
/// (`TimeInMilliseconds >= m_WakeTime`, unsigned). Returns 0.
#[hook(addr = 0x469F00, conv = thiscall, name = "CRunningScript::Process", expect = "53 56 8B F1 8B 86 D8 00 00 00", this = "CRunningScript")]
pub fn process(s: Scr) -> i8 {
    if rd::<i32>(s, SCENE_SKIP_IP) != 0 && is_cutscene_skip_button_being_pressed() {
        BIG_MESSAGE_1.set(0);
        let target = update_pc_target(s, rd::<i32>(s, SCENE_SKIP_IP));
        set_ip(s, target);
        wr::<i32>(s, SCENE_SKIP_IP, 0);
        wr::<i32>(s, WAKE_TIME, 0);
    }
    if rd::<u8>(s, USES_MISSION_CLEANUP) != 0 {
        do_death_arrest_check(s);
    }
    if rd::<u8>(s, IS_MISSION) != 0 && FAIL_CURRENT_MISSION.get() == 1 && rd::<u16>(s, STACK_DEPTH) >= 1 {
        wr::<u16>(s, STACK_DEPTH, 0);
        let bottom = rd::<u32>(s, IP_STACK);
        wr::<u32>(s, IP, bottom);
    }
    reinitialise_switch_statement_data();
    if sa_core::timer::TIME_MS.get() >= rd::<u32>(s, WAKE_TIME) {
        while dispatch(s) == 0 {}
    }
    0
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_POINTER_TO_LOCAL_VARIABLE,
    HOOK_GET_POINTER_TO_LOCAL_ARRAY_ELEMENT,
    HOOK_READ_ARRAY_INFORMATION,
    HOOK_COLLECT_PARAMETERS,
    HOOK_COLLECT_NEXT_PARAMETER_WITHOUT_INCREASING_PC,
    HOOK_STORE_PARAMETERS,
    HOOK_READ_PARAMETERS_FOR_NEWLY_STARTED_SCRIPT,
    HOOK_READ_TEXT_LABEL_FROM_SCRIPT,
    HOOK_GET_INDEX_OF_GLOBAL_VARIABLE,
    HOOK_GET_POINTER_TO_SCRIPT_VARIABLE,
    HOOK_UPDATE_PC,
    HOOK_UPDATE_COMPARE_FLAG,
    HOOK_PROCESS_ONE_COMMAND,
    HOOK_PROCESS,
];
