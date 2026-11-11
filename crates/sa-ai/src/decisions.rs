//! `CDecision`, `CDecisionMaker`, `CDecisionMakerTypesFileLoader` (the non-file parts) and `CMentalState::Process`
//! (PORT-AI-5). Ported from the 1.0 US bytes (the exe wins over gta-reversed). Every call to another catalogued
//! function goes through `orig!` at the original address.
//!
//! `CDecision` (0x3C bytes): +0 event-response task types int32[6], +0x18 probabilities uint8[6][4], +0x30
//! bool[6][2]. `CDecisionMaker` = 0x29 decisions (0x99C bytes). `CMentalState`: +0 / +1 anger (current, last),
//! +0x10 / +0x11 ped health byte (current, last), +0x12 / +0x13 vehicle health byte (current, last).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CDecision::SetDefault` (0x600530)
    fn decision_set_default(d: GPtr<u8>) = thiscall 0x600530 => unported;
    /// `CDecision::CDecision` (0x6040C0)
    fn decision_ctor(d: GPtr<u8>) -> GPtr<u8> = thiscall 0x6040C0 => unported;
    /// `CDecisionMaker::CDecisionMaker` (0x4650A0)
    fn decision_maker_ctor(d: GPtr<u8>) -> GPtr<u8> = thiscall 0x4650A0 => unported;
    /// `CDecisionMakerTypes::GetInstance` (0x4684F0, cdecl)
    fn decision_maker_types_get_instance() -> GPtr<u8> = cdecl 0x4684F0 => unported;
    /// `CDecisionMakerTypes::RemoveDecisionMaker(int32)` (0x6043A0, `ret 4`)
    fn decision_maker_types_remove(t: GPtr<u8>, i: i32) = thiscall 0x6043A0 => unported;
    /// `CDecisionMakerTypes::AddDecisionMaker(CDecisionMaker*, int32, bool)` (0x607050, `ret 0xC`)
    fn decision_maker_types_add(t: GPtr<u8>, dm: GPtr<u8>, ty: u32, b: u32) -> u32 = thiscall 0x607050 => unported;
    /// `CDecisionMakerTypesFileLoader::LoadDecisionMaker(const char*, CDecisionMaker*)` (0x6076B0, cdecl)
    fn file_loader_load_decision_maker_ptr(name: u32, dm: GPtr<u8>) = cdecl 0x6076B0 => unported;
    /// `CEventEditableResponse::CEventEditableResponse` (0x4AC450)
    fn editable_response_ctor(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AC450 => unported;
    /// `CEventEditableResponse::~CEventEditableResponse` (0x4AC480)
    fn editable_response_dtor(e: GPtr<u8>) = thiscall 0x4AC480 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn eg_add(eg: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

/// `CDecision::SetDefault` (0x600530): types -1, probabilities 0, flags 0. The caller `CDecision::CDecision`
/// (0x6040C0) returns ECX after the call, so the hook keeps ECX.
#[hook(addr = 0x600530, conv = thiscall, name = "CDecision::SetDefault", expect = "33 C0 8D 51 30", preserve = "ecx")]
pub fn decision_set_default_port(this: GPtr<u8>) {
    for i in 0..6 {
        wr::<u32>(this, 4 * i, 0xFFFF_FFFF);
        wr::<u32>(this, 0x18 + 4 * i, 0);
        wr::<u16>(this, 0x30 + 2 * i, 0);
    }
}

/// `CDecision::CDecision` (0x6040C0, catalogued as `CDecision`): `SetDefault`, returns this.
#[hook(addr = 0x6040C0, conv = thiscall, name = "CDecision", expect = "E8 6B C4 FF FF")]
pub fn decision_ctor_port(this: GPtr<u8>) -> GPtr<u8> {
    decision_set_default(this);
    this
}

/// `CDecisionMaker::CDecisionMaker` (0x4650A0, `.HOODLUM` stub; catalogued as `CDecisionMaker`): constructs the 0x29
/// decisions, then sets every one to its default again. Returns this.
#[hook(addr = 0x4650A0, conv = thiscall, name = "CDecisionMaker", expect = "E9 DB F1 0F 01")]
pub fn decision_maker_ctor_port(this: GPtr<u8>) -> GPtr<u8> {
    for i in 0..0x29 {
        decision_ctor(this.byte_add(0x3C * i));
    }
    for i in 0..0x29 {
        decision_set_default(this.byte_add(0x3C * i));
    }
    this
}

/// `CDecision::Set(int32* types, float* probs, int32* bools, float*)` (0x600570, `ret 0x10`; the fourth argument is
/// unused): for each of the 6 responses the type, the four probabilities `_ftol`'d to bytes and the two flags (!= 0).
#[hook(addr = 0x600570, conv = thiscall, name = "CDecision::Set", expect = "51 53 55 8B 6C 24 18")]
pub fn decision_set(this: GPtr<u8>, types: GPtr<u8>, probs: GPtr<u8>, bools: GPtr<u8>, _facial: u32) {
    for i in 0..6isize {
        wr::<u32>(this, 4 * i, rd::<u32>(types, 4 * i));
        for k in 0..4isize {
            wr::<u8>(this, 0x18 + 4 * i + k, ftol32(x(rd::<f32>(probs, 0x10 * i + 4 * k))) as u8);
        }
        wr::<u8>(this, 0x30 + 2 * i, (rd::<u32>(bools, 8 * i) != 0) as u8);
        wr::<u8>(this, 0x31 + 2 * i, (rd::<u32>(bools, 8 * i + 4) != 0) as u8);
    }
}

/// `CDecisionMakerTypesFileLoader::UnloadDecisionMaker(int32)` (0x607A70, cdecl): `RemoveDecisionMaker` on the
/// instance (the argument pushed for `GetInstance` is consumed by the thiscall).
#[hook(addr = 0x607A70, conv = cdecl, name = "CDecisionMakerTypesFileLoader::UnloadDecisionMaker", expect = "8B 44 24 04 50")]
pub fn file_loader_unload_decision_maker(i: i32) {
    let t = decision_maker_types_get_instance();
    decision_maker_types_remove(t, i);
}

/// `CDecisionMakerTypesFileLoader::LoadDecisionMaker(const char*, int32 type, bool)` (0x607D30, cdecl; entry `nop;
/// jmp 0x40168C`, the island allocates the 0x99C-byte frame and jumps back): a `CDecisionMaker` on the stack (a Rust
/// local here), loaded from the file and added to the instance; returns `AddDecisionMaker`'s EAX.
#[hook(addr = 0x607D30, conv = cdecl, name = "CDecisionMakerTypesFileLoader::LoadDecisionMaker-enum", expect = "90 E9 56 99 DF FF")]
pub fn file_loader_load_decision_maker_enum(name: u32, ty: u32, b: u32) -> u32 {
    let mut dm = [0u32; 0x99C / 4];
    let p: GPtr<u8> = GPtr::from_raw(&mut dm as *mut [u32; 0x99C / 4]).cast();
    decision_maker_ctor(p);
    file_loader_load_decision_maker_ptr(name, p);
    let t = decision_maker_types_get_instance();
    decision_maker_types_add(t, p, ty, b)
}

/// Adds an editable-response event with vtable `vtbl`, built on the stack (a Rust local here), to the ped's event
/// group, then destroys it (vtable stored again before the destructor, as the original does).
fn add_event(ped: GPtr<u8>, vtbl: u32) {
    let mut ev = [0u32; 8];
    let e: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 8]).cast();
    editable_response_ctor(e);
    wr::<u32>(e, 0, vtbl);
    eg_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
    wr::<u32>(e, 0, vtbl);
    editable_response_dtor(e);
}

/// `CMentalState::Process(const CPed&)` (0x6008A0, `ret 4`): records the ped's health (`_ftol` of +0x540, low byte)
/// and, in a vehicle, the vehicle's (+0x4C0). On foot, health falling below 50 / 10 raises the events with vtables
/// 0x86CBA0 / 0x86CBE8; in a vehicle the original compares the zero-extended health bytes with 600 and 300, which no
/// byte reaches, so nothing is raised (kept). Anger (+0) rising above 3 / 6 raises 0x86CC30 / 0x86CC78. Then the
/// current values become the last ones.
#[hook(addr = 0x6008A0, conv = thiscall, name = "CMentalState::Process", expect = "6A FF 64 A1 00 00 00 00")]
pub fn mental_state_process(this: GPtr<u8>, ped: GPtr<u8>) {
    wr::<u8>(this, 0x10, ftol32(x(rd::<f32>(ped, 0x540))) as u8);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() {
            wr::<u8>(this, 0x12, ftol32(x(rd::<f32>(v, 0x4C0))) as u8);
        }
    }
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
        let last = rd::<u8>(this, 0x11);
        if last >= 0x32 {
            let cur = rd::<u8>(this, 0x10);
            if cur < 0xA {
                add_event(ped, 0x86CBE8);
            } else if cur < 0x32 {
                add_event(ped, 0x86CBA0);
            }
        } else if last >= 0xA && rd::<u8>(this, 0x10) < 0xA {
            add_event(ped, 0x86CBE8);
        }
    } else {
        let last = rd::<u8>(this, 0x13) as i32;
        if last >= 0x258 {
            let cur = rd::<u8>(this, 0x12) as i32;
            if cur < 0x12C {
                add_event(ped, 0x86CBE8);
            } else if cur < 0x258 {
                add_event(ped, 0x86CBA0);
            }
        } else if last >= 0x12C && (rd::<u8>(this, 0x12) as i32) < 0x12C {
            add_event(ped, 0x86CBE8);
        }
    }
    let last = rd::<u8>(this, 1);
    if last <= 3 {
        let cur = rd::<u8>(this, 0);
        if cur > 6 {
            add_event(ped, 0x86CC78);
        } else if cur > 3 {
            add_event(ped, 0x86CC30);
        }
    } else if last <= 6 && rd::<u8>(this, 0) > 6 {
        add_event(ped, 0x86CC78);
    }
    let a = rd::<u8>(this, 0);
    let h = rd::<u8>(this, 0x10);
    let vh = rd::<u8>(this, 0x12);
    wr::<u8>(this, 1, a);
    wr::<u8>(this, 0x11, h);
    wr::<u8>(this, 0x13, vh);
}


orig! {
    /// `CFileMgr::SetDir(dir)` (0x5387D0)
    fn dm_set_dir(dir: GPtr<u8>) = cdecl 0x5387D0 => unported;
    /// `CFileMgr::OpenFile(path, mode)` (0x538900)
    fn dm_open_file(path: u32, mode: GPtr<u8>) -> i32 = cdecl 0x538900 => unported;
    /// `CFileMgr::ReadLine(file, buf, size)` (0x5389B0, bool in AL)
    fn dm_read_line(file: i32, buf: GPtr<u8>, size: u32) -> u8 = cdecl 0x5389B0 => unported;
    /// `CFileMgr::CloseFile(file)` (0x5389D0)
    fn dm_close_file(file: i32) -> i32 = cdecl 0x5389D0 => unported;
    /// CRT `sscanf(line, [0x86CDA0], 80 pointers)` (0x8220AD)
    fn dm_sscanf(line: GPtr<u8>, fmt: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32, a17: u32, a18: u32, a19: u32, a20: u32, a21: u32, a22: u32, a23: u32, a24: u32, a25: u32, a26: u32, a27: u32, a28: u32, a29: u32, a30: u32, a31: u32, a32: u32, a33: u32, a34: u32, a35: u32, a36: u32, a37: u32, a38: u32, a39: u32, a40: u32, a41: u32, a42: u32, a43: u32, a44: u32, a45: u32, a46: u32, a47: u32, a48: u32, a49: u32, a50: u32, a51: u32, a52: u32, a53: u32, a54: u32, a55: u32, a56: u32, a57: u32, a58: u32, a59: u32, a60: u32, a61: u32, a62: u32, a63: u32, a64: u32, a65: u32, a66: u32, a67: u32, a68: u32, a69: u32, a70: u32, a71: u32, a72: u32, a73: u32, a74: u32, a75: u32, a76: u32, a77: u32, a78: u32, a79: u32) -> i32 = cdecl 0x8220AD => unported;
    /// `CDecision::Set(types, probs, bools, facial)` (0x600570)
    fn dm_decision_set(d: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, e: GPtr<u8>) = thiscall 0x600570 => unported;
    /// `CDecision::From(const CDecision&)` (0x6006B0)
    fn dm_decision_from(d: GPtr<u8>, src: GPtr<u8>) = thiscall 0x6006B0 => unported;
    /// `CDecisionMakerTypes::LoadEventIndices()` (0x600840)
    fn dm_load_event_indices(t: GPtr<u8>) = thiscall 0x600840 => unported;
    /// `CDecisionMakerTypesFileLoader::LoadDecisionMaker(const char*, int32, bool)` (0x607D30)
    fn dm_load_enum(name: GPtr<u8>, ty: u32, b: u32) -> u32 = cdecl 0x607D30 => unported;
}

/// `CDecision::From(const CDecision&)` (0x6006B0, thiscall, `ret 4`; the loop count 6 is set in the no-CD island
/// 0x40394C): copies the 6 response types (dwords), the 6x4 probability bytes and the 6x2 flag bytes, i.e. the
/// whole 0x3C bytes, response by response.
///
/// Not registered: 0x6006B0 is hooked by sa-ai mvpa_ai1.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn decision_from(this: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..6isize {
        wr::<u32>(this, 4 * i, rd::<u32>(src, 4 * i));
        for k in 0..4isize {
            wr::<u8>(this, 0x18 + 4 * i + k, rd::<u8>(src, 0x18 + 4 * i + k));
        }
        wr::<u8>(this, 0x30 + 2 * i, rd::<u8>(src, 0x30 + 2 * i));
        wr::<u8>(this, 0x31 + 2 * i, rd::<u8>(src, 0x31 + 2 * i));
    }
}

/// `CDecisionMakerTypesFileLoader::LoadDecisionMaker(const char* name, CDecisionMaker* dm)` (0x6076B0, cdecl, bool in
/// AL). Opens `name` under `data\decision\allowed\` (`SetDir` around `OpenFile(name, "r")`); a handle <= 0 → 0.
/// The first line is a header; every further line (`ReadLine`, 0x200 bytes) is read with the 80-conversion format at
/// 0x86CDA0 into the original's stack locals, then a decision is built in a local `CDecision` (types -1,
/// probabilities and flags 0), `CDecision::Set(types, probabilities, flags, ...)` from those locals, and copied with
/// `From` into `dm->decisions[CDecisionMakerTypes::m_EventIndices[event]]` (instance +0xC034). Closes the file → 1.
/// The port keeps the original's frame layout (a byte array indexed by the frame offsets of the bytes), so
/// conversions `sscanf` does not perform leave the previous line's values in place, as in the original (the
/// frame starts zeroed here, uninitialised there).
#[hook(addr = 0x6076B0, conv = cdecl, name = "CDecisionMakerTypesFileLoader::LoadDecisionMaker-ptr", expect = "81 EC 7C 03 00 00 53",
       unverified = "std4-data-1", route = "L2", hazards = "uninit-locals", recs = "boot")]
pub fn file_loader_load_decision_maker_ptr_port(name: u32, dm: GPtr<u8>) -> bool {
    // ESP-relative frame of the loop (ESP at 0x607730): locals F+0x10 .. F+0x38C
    let mut frame = [0u8; 0x390];
    let base = frame.as_mut_ptr();
    let fp = |o: usize| base.wrapping_add(o) as usize as u32;
    let fg = |o: usize| GPtr::<u8>::from_raw(base.wrapping_add(o));
    dm_set_dir(g::<u8>(0x86CEE4));
    let file = dm_open_file(name, g::<u8>(0x85A53C));
    dm_set_dir(g::<u8>(0x858B54));
    if file <= 0 {
        return false;
    }
    dm_read_line(file, fg(0x18C), 0x200);
    if dm_read_line(file, fg(0x18C), 0x200) != 0 {
        loop {
            dm_sscanf(fg(0x18C), g::<u8>(0x86CDA0), fp(0x10), fp(0x14c), fp(0x14), fp(0x64), fp(0x68), fp(0x60), fp(0x5c), fp(0x30), fp(0x2c), fp(0xbc), fp(0xc0), fp(0xc4), fp(0xc8), fp(0xcc), fp(0xd0), fp(0x18), fp(0x74), fp(0x78), fp(0x70), fp(0x6c), fp(0x38), fp(0x34), fp(0xd4), fp(0xd8), fp(0xdc), fp(0xe0), fp(0xe4), fp(0xe8), fp(0x1c), fp(0x84), fp(0x88), fp(0x80), fp(0x7c), fp(0x40), fp(0x3c), fp(0xec), fp(0xf0), fp(0xf4), fp(0xf8), fp(0xfc), fp(0x100), fp(0x20), fp(0x94), fp(0x98), fp(0x90), fp(0x8c), fp(0x48), fp(0x44), fp(0x104), fp(0x108), fp(0x10c), fp(0x110), fp(0x114), fp(0x118), fp(0x24), fp(0xa4), fp(0xa8), fp(0xa0), fp(0x9c), fp(0x50), fp(0x4c), fp(0x11c), fp(0x120), fp(0x124), fp(0x128), fp(0x12c), fp(0x130), fp(0x28), fp(0xb4), fp(0xb8), fp(0xb0), fp(0xac), fp(0x58), fp(0x54), fp(0x134), fp(0x138), fp(0x13c), fp(0x140), fp(0x144), fp(0x148));
            for i in 0..6usize {
                fg(0x150 + 4 * i).cast::<u32>().write(u32::MAX);
                fg(0x168 + 4 * i).cast::<u32>().write(0);
                fg(0x180 + 2 * i).cast::<u16>().write(0);
            }
            dm_decision_set(fg(0x150), fg(0x14), fg(0x5C), fg(0x2C), fg(0xBC));
            let event = fg(0x10).cast::<u32>().read() as usize;
            let t = decision_maker_types_get_instance();
            let idx = rd::<u32>(t, (event * 4 + 0xC034) as isize);
            dm_decision_from(dm.byte_add(idx.wrapping_mul(0x3C) as i32 as isize), fg(0x150));
            if dm_read_line(file, fg(0x18C), 0x200) == 0 {
                break;
            }
        }
    }
    dm_close_file(file);
    true
}

/// `CDecisionMakerTypesFileLoader::LoadDefaultDecisionMaker()` (0x5BF400, cdecl): `LoadEventIndices`, unloads the 20
/// decision makers, constructs a local `CDecisionMaker` (41 `CDecision`s) and, for each default file (`RANDOM.ped`,
/// `m_norm.ped`, `m_plyr.ped`, `RANDOM.grp`, `MISSION.grp`), resets it with `SetDefault` (twice before the first),
/// loads the file into it and copies its 41 decisions with `From` into the instance's default makers (+0xC1B4,
/// +0xCB50, +0xD4EC, +0xDE88, +0xE824). Then loads the named makers `GangMbr.ped` .. (8 ped types, type 0) and
/// `RANDOM.grp`, `RANDOM2.grp` (type 1) through `LoadDecisionMaker(name, type, false)`.
#[hook(addr = 0x5BF400, conv = cdecl, name = "CDecisionMakerTypesFileLoader::LoadDefaultDecisionMaker", expect = "81 EC 9C 09 00 00",
       unverified = "std4-data-1", route = "L2", hazards = "none", recs = "boot")]
pub fn file_loader_load_default_decision_maker() {
    let t = decision_maker_types_get_instance();
    dm_load_event_indices(t);
    for i in 0..0x14 {
        file_loader_unload_decision_maker(i);
    }
    let mut local = [0u32; 0x99C / 4];
    let p: GPtr<u8> = GPtr::from_raw(&mut local as *mut [u32; 0x99C / 4]).cast();
    let each = |f: &dyn Fn(GPtr<u8>)| {
        for i in 0..0x29isize {
            f(p.byte_add(0x3C * i));
        }
    };
    each(&|d| {
        decision_ctor(d);
    });
    each(&|d| decision_set_default(d));
    for (k, (file, off)) in [(0x86AB5Cu32, 0xC1B4isize), (0x86AB50, 0xCB50), (0x86AB44, 0xD4EC), (0x86AB38, 0xDE88), (0x86AB2C, 0xE824)]
        .into_iter()
        .enumerate()
    {
        let _ = k;
        each(&|d| decision_set_default(d));
        file_loader_load_decision_maker_ptr(file, p);
        let dst = decision_maker_types_get_instance().byte_add(off);
        for i in 0..0x29isize {
            dm_decision_from(dst.byte_add(0x3C * i), p.byte_add(0x3C * i));
        }
    }
    for name in [0x86AB20usize, 0x86AB18, 0x86AB0C, 0x86AB00, 0x86AAF4, 0x86AAE8, 0x86AADC, 0x86AAD0] {
        dm_load_enum(g::<u8>(name), 0, 0);
    }
    dm_load_enum(g::<u8>(0x86AB38), 1, 0);
    dm_load_enum(g::<u8>(0x86AAC4), 1, 0);
}

/// `CDecisionMakerTypes::CDecisionMakerTypes()` (0x4650F0: a `jmp` stub into `.HOODLUM` 0x1567930, thiscall):
/// +0 := 0; the 20 `CDecisionMaker`s at +4 (0x99C each) constructed (`eh vector constructor iterator` with
/// `CDecisionMaker::CDecisionMaker` 0x4650A0; its destructor argument 0x4634C0 only matters on an exception); the
/// five default makers (+0xC1B4, +0xCB50, +0xD4EC, +0xDE88, +0xE824) each get their 41 `CDecision` constructors
/// and then `SetDefault`s; the static block 0xC0AFE0..0xC0B030 is reset (0xC0AFE0..0xC0AFF0 and 0xC0B01C..0xC0B02C
/// zero, the ten dwords at 0xC0AFF4 = 0x10001). Returns `this`.
#[hook(addr = 0x4650F0, conv = thiscall, name = "thunk_FUN_01567930", expect = "E9 3B 28 10 01",
       unverified = "std4-data-1", route = "L2", hazards = "none", recs = "boot")]
pub fn decision_maker_types_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    for i in 0..0x14isize {
        decision_maker_ctor(this.byte_add(4 + 0x99C * i));
    }
    for off in [0xC1B4isize, 0xCB50, 0xD4EC, 0xDE88, 0xE824] {
        for i in 0..0x29isize {
            decision_ctor(this.byte_add(off + 0x3C * i));
        }
        for i in 0..0x29isize {
            decision_set_default(this.byte_add(off + 0x3C * i));
        }
    }
    for a in (0xC0AFE0usize..0xC0AFF4).step_by(4) {
        g::<u32>(a).write(0);
    }
    g::<u32>(0xC0B01C).write(0);
    for i in 0..10 {
        g::<u32>(0xC0AFF4 + 4 * i).write(0x10001);
    }
    for a in [0xC0B020usize, 0xC0B024, 0xC0B028, 0xC0B02C] {
        g::<u32>(a).write(0);
    }
    this
}

orig! {
    /// CRT `sscanf(line, "%s %d", name, &index)` (0x8220AD)
    fn ei_sscanf(line: GPtr<u8>, fmt: GPtr<u8>, name: GPtr<u8>, index: GPtr<u32>) -> i32 = cdecl 0x8220AD => unported;
}

/// `FUN_005bb9f0(int32* indices, const char* file)` (0x5BB9F0, stdcall; the event index table loader
/// `CDecisionMakerTypes::LoadEventIndices` calls): zeroes the 0x60 indices, opens `file` under `data\decision\`
/// (`SetDir("")`, `SetDir(dir)`, `OpenFile(file, "r")`, `SetDir("")`), then for every line (`ReadLine`, 0x100 bytes)
/// that is neither empty nor starts with a newline: `sscanf(line, "%s %d", name, &event)` and `indices[event] = n`
/// with n counting those lines. The line buffer and the `%d` local persist across lines (as the original's stack
/// slots; zero before the first line here).
#[hook(addr = 0x5BB9F0, conv = stdcall, name = "FUN_005bb9f0", expect = "81 EC 04 02 00 00 53",
       unverified = "std4-data-1", route = "L2", hazards = "uninit-locals", recs = "boot")]
pub fn load_event_index_file(indices: GPtr<u8>, file: u32) {
    for i in 0..0x60isize {
        wr::<u32>(indices, 4 * i, 0);
    }
    dm_set_dir(g::<u8>(0x858B54));
    dm_set_dir(g::<u8>(0x86A6B4));
    let f = dm_open_file(file, g::<u8>(0x85A53C));
    dm_set_dir(g::<u8>(0x858B54));
    let mut buf = [0u8; 0x100];
    let mut name = [0u8; 0x100];
    let mut event = 0u32;
    let mut n = 0u32;
    let bp = GPtr::<u8>::from_raw(buf.as_mut_ptr());
    while dm_read_line(f, bp, 0x100) != 0 {
        let c = bp.read();
        if c == 0 || c == b'\n' {
            continue;
        }
        ei_sscanf(bp, g::<u8>(0x86A6AC), GPtr::from_raw(name.as_mut_ptr()), GPtr::from_raw(&raw mut event));
        wr::<u32>(indices, 4 * event as i32 as isize, n);
        n += 1;
    }
    dm_close_file(f);
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_DECISION_SET_DEFAULT_PORT,
    HOOK_DECISION_CTOR_PORT,
    HOOK_DECISION_MAKER_CTOR_PORT,
    HOOK_DECISION_SET,
    HOOK_FILE_LOADER_UNLOAD_DECISION_MAKER,
    HOOK_FILE_LOADER_LOAD_DECISION_MAKER_ENUM,
    HOOK_MENTAL_STATE_PROCESS,
    HOOK_FILE_LOADER_LOAD_DECISION_MAKER_PTR_PORT,
    HOOK_FILE_LOADER_LOAD_DEFAULT_DECISION_MAKER,
    HOOK_DECISION_MAKER_TYPES_CTOR,
    HOOK_LOAD_EVENT_INDEX_FILE,
];
