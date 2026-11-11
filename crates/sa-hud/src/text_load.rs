//! GXT loading (PORT-HUD-2): `CText::Load` / `LoadMissionText` / `LoadMissionPackText`, the chunk readers
//! (`CKeyArray::Load`, `CData::Load`, `CMissionTextOffsets::Load`, `CText::ReadChunkHeader`),
//! `GxtCharToAscii` and the two unreferenced unload copies. Ported from the 1.0 US bytes;
//! `docs/re/specs/HUD/{CText.deviations,CKeyArray,CData,_unk-B}.md` and gta-reversed `Text/*.cpp` as
//! reading aids. File IO, allocation and the CRT stay original (`orig!`).
//!
//! `CText` layout: see `crate::text`; +0x12C `CMissionTextOffsets` (200 entries of { char name[8];
//! uint32 offset }, count u16 at +0x960 of it = `CText` +0xA8C).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::platform::PREFS_LANGUAGE;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

/// This module's globals (none of its own; the `GxtCharToAscii` buffer is reached by address).
pub const GLOBALS: &[GlobalDesc] = &[];

/// `GxtCharToAscii`'s static result buffer (0xC1BDD0, 256 bytes)
const ASCII_BUF: usize = 0xC1BDD0;
/// `TheText` (0xC1B340)
const THE_TEXT: usize = 0xC1B340;
/// `FrontEndMenuManager` (0xBA6748)
const FRONT_END_MENU_MANAGER: usize = 0xBA6748;
/// The selected mission pack (byte 0xB72910)
const MISSION_PACK: usize = 0xB72910;
/// "" (0x858B54)
const S_EMPTY: usize = 0x858B54;
/// "TEXT" (0x870FB0)
const S_TEXT_DIR: usize = 0x870FB0;
/// "rb" (0x859C6C)
const S_RB: usize = 0x859C6C;
/// "TABL" (0x870FB8), "TKEY" (0x870F68), "TDAT" (0x870F60)
const S_TABL: usize = 0x870FB8;
const S_TKEY: usize = 0x870F68;
const S_TDAT: usize = 0x870F60;
/// "CDERROR" (0x865264)
const S_CDERROR: usize = 0x865264;
/// "MPACK//MPACK%d//TEXT.GXT" (0x8656E4)
const S_MPACK_FMT: usize = 0x8656E4;
/// The mission-pack mission name (0x870F58, 7 bytes copied)
const S_MPACK_NAME: usize = 0x870F58;
/// The GXT file names by language 0..4 (jump tables 0x6A0448 / 0x69FF0C): AMERICAN, FRENCH, GERMAN,
/// ITALIAN, SPANISH.
const GXT_NAMES: [usize; 5] = [0x870FA0, 0x870F94, 0x870F88, 0x870F7C, 0x870F70];

orig! {
    /// CRT `operator new` (0x821195)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// CRT `free` (0x8213AE)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// CRT `strncmp` (0x8214D0)
    pub fn crt_strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    /// CRT `sprintf(buf, fmt)` (0x821BB5)
    pub fn sprintf0(buf: GPtr<u8>, fmt: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `sprintf(buf, fmt, a)` (0x821BB5)
    pub fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: u32) -> i32 = cdecl 0x821BB5 => unported;
    /// `CFileMgr::Read(file, buf, size)` (0x538950)
    pub fn file_read(f: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 = cdecl 0x538950 => sa_core::filemgr::read;
    /// `CFileMgr::Seek(file, offset, origin)` (0x538990)
    pub fn file_seek(f: GPtr<u8>, offset: i32, origin: i32) -> bool = cdecl 0x538990 => sa_core::filemgr::seek;
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    pub fn file_set_dir(dir: GPtr<u8>) -> i32 = cdecl 0x5387D0 => sa_core::filemgr::set_dir;
    /// `CFileMgr::SetDirMyDocuments()` (0x538860)
    pub fn file_set_dir_my_documents() -> i32 = cdecl 0x538860 => sa_core::filemgr::set_dir_my_documents;
    /// `CFileMgr::OpenFile(const char* path, const char* mode)` (0x538900)
    pub fn file_open(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    pub fn file_close(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
    /// `CTimer::Suspend()` (0x5619D0)
    pub fn timer_suspend() = cdecl 0x5619D0 => unported;
    /// `CTimer::Resume()` (0x561A00)
    pub fn timer_resume() = cdecl 0x561A00 => unported;
    /// `CMenuManager::CheckMissionPackValidMenu()` (0x57D720, thiscall)
    pub fn check_mission_pack_valid_menu(this: GPtr<u8>) -> u8 = thiscall 0x57D720 => unported;
    /// `CMessages::ClearAllMessagesDisplayedByGame(bool)` (0x69EDC0)
    pub fn clear_all_messages_displayed_by_game_call(flag: u32) = cdecl 0x69EDC0 => crate::messages::clear_all_messages_displayed_by_game;
    /// `CText::Unload(bool)` (0x69FF20, thiscall)
    pub fn text_unload_call(this: GPtr<u8>, keep_mission: u32) = thiscall 0x69FF20 => crate::text::text_unload;
    /// `CText::Get(const char*)` (0x6A0050, thiscall)
    pub fn text_get_call(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => crate::text::text_get;
    /// `CKeyArray::Load(uint32 length, FILE*, uint32* offset, uint8 nSkip)` (0x69F490, thiscall)
    pub fn key_array_load_call(this: GPtr<u8>, length: u32, f: GPtr<u8>, offset: GPtr<u8>, skip: u32) -> u8 = thiscall 0x69F490 => key_array_load;
    /// `CData::Load(uint32 length, FILE*, uint32* offset, uint8 nSkip)` (0x69F5D0, thiscall)
    pub fn data_load_call(this: GPtr<u8>, length: u32, f: GPtr<u8>, offset: GPtr<u8>, skip: u32) -> u8 = thiscall 0x69F5D0 => data_load;
    /// `CMissionTextOffsets::Load(uint32 length, FILE*, uint32* offset, uint32)` (0x69F670, thiscall)
    pub fn mission_text_offsets_load_call(this: GPtr<u8>, length: u32, f: GPtr<u8>, offset: GPtr<u8>, unused: u32) = thiscall 0x69F670 => mission_text_offsets_load;
    /// `CText::ReadChunkHeader(ChunkHeader*, FILE*, uint32* offset, uint8 nSkip)` (0x69F940, thiscall)
    pub fn read_chunk_header_call(this: GPtr<u8>, header: GPtr<u8>, f: GPtr<u8>, offset: GPtr<u8>, skip: u32) -> u8 = thiscall 0x69F940 => read_chunk_header;
    /// `CText::LoadMissionPackText()` (0x69F9A0, thiscall)
    pub fn load_mission_pack_text_call(this: GPtr<u8>) = thiscall 0x69F9A0 => load_mission_pack_text;
    /// `GxtCharToAscii(const GxtChar*, uint8 start)` (0x69F7E0)
    pub fn gxt_char_to_ascii_call(src: GPtr<u8>, start: u32) -> GPtr<u8> = cdecl 0x69F7E0 => gxt_char_to_ascii;
}

fn local<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(core::ptr::from_mut(v)).cast()
}

fn inc_u32(p: GPtr<u8>) {
    wr(p, 0, rd::<u32>(p, 0).wrapping_add(1));
}

/// Frees `[p + off]` when set and nulls it, then zeroes the size at `p + off + 4`.
fn unload_pair(p: GPtr<u8>, off: isize) {
    let d = rd::<GPtr<u8>>(p, off);
    if !d.is_null() {
        crt_free(d);
        wr(p, off, GPtr::<u8>::NULL);
    }
    wr::<u32>(p, off + 4, 0);
}

/// `CKeyArray::Update(data)` inlined in the loaders: every entry's string offset += the data base.
fn update_keys(keys: GPtr<u8>, data: u32) {
    let mut e = rd::<GPtr<u8>>(keys, 0);
    let mut i = 0u32;
    while i < rd::<u32>(keys, 4) {
        wr(e, 0, rd::<u32>(e, 0).wrapping_add(data));
        e = e.byte_add(8);
        i += 1;
    }
}

/// The byte-wise reader shared by `CKeyArray::Load` and `CData::Load`: `n` bytes into `dst`, one
/// `CFileMgr::Read(.., 1)` each into the first argument's stack slot (`slot`, initially the length);
/// with `skip` set no read happens and the slot's low byte is stored while the `skip` word is 1 (any
/// other non-zero word fails at once). `*offset` counts the bytes. AL = success.
fn load_bytes(dst: GPtr<u8>, n: u32, f: GPtr<u8>, offset: GPtr<u8>, skip: u32, mut slot: u32) -> u8 {
    let mut res = skip;
    let mut i = 0u32;
    while i < n {
        if skip as u8 == 0 {
            res = file_read(f, local(&mut slot), 1);
        }
        if res != 1 {
            return 0;
        }
        wr(dst, i as isize, slot as u8);
        inc_u32(offset);
        i += 1;
    }
    1
}

/// `CKeyArray::Load(uint32 length, FILE*, uint32* offset, uint8 nSkip)` (0x69F490, thiscall): count =
/// `length >> 3`, entries = `operator new(count * 8)`, then `length` bytes read one by one. AL.
#[hook(addr = 0x69F490, conv = thiscall, name = "CKeyArray::Load", expect = "53 55 8B 6C 24 0C 56 57")]
pub fn key_array_load(this: GPtr<u8>, length: u32, f: GPtr<u8>, offset: GPtr<u8>, skip: u32) -> u8 {
    let count = length >> 3;
    wr(this, 4, count);
    let p = operator_new(count << 3);
    wr(this, 0, p);
    load_bytes(p, length, f, offset, skip, length)
}

/// `CData::Load(uint32 length, FILE*, uint32* offset, uint8 nSkip)` (0x69F5D0, thiscall): size =
/// `length`, data = `operator new(length)`, then `length` bytes read one by one. AL.
#[hook(addr = 0x69F5D0, conv = thiscall, name = "CData::Load", expect = "53 55 8B 6C 24 0C 56 57")]
pub fn data_load(this: GPtr<u8>, length: u32, f: GPtr<u8>, offset: GPtr<u8>, skip: u32) -> u8 {
    wr(this, 4, length);
    let p = operator_new(length);
    wr(this, 0, p);
    load_bytes(p, length, f, offset, skip, length)
}

/// `CMissionTextOffsets::Load(uint32 length, FILE*, uint32* offset, uint32)` (0x69F670, thiscall): count
/// = `length / 12`; per entry 8 name bytes and 4 offset bytes, one `Read(.., 1)` each (results not
/// checked; the offset goes through the `offset` argument's stack slot), `*offset` counting; the entry
/// index wraps at 16 bits. Stores the count (u16) at +0x960. The fourth argument is unused.
#[hook(addr = 0x69F670, conv = thiscall, name = "CMissionTextOffsets::Load", expect = "83 EC 0C B8 AB AA AA AA")]
pub fn mission_text_offsets_load(this: GPtr<u8>, length: u32, f: GPtr<u8>, offset: GPtr<u8>, _unused: u32) {
    let count = length / 12;
    if count != 0 {
        let mut slot = length;
        let mut off_slot = offset.addr() as u32;
        let mut i: u32 = 0;
        loop {
            let entry = this.byte_add((i as u16 as isize) * 12);
            for k in 0..8 {
                file_read(f, local(&mut slot), 1);
                wr(entry, k, slot as u8);
                inc_u32(offset);
            }
            let op = local(&mut off_slot);
            for k in 0..4 {
                file_read(f, op.byte_add(k), 1);
                inc_u32(offset);
            }
            wr(entry, 8, off_slot);
            i = i.wrapping_add(1);
            if u32::from(i as u16) >= count {
                break;
            }
        }
    }
    wr(this, 0x960, count as u16);
}

/// `CText::ReadChunkHeader(ChunkHeader*, FILE*, uint32* offset, uint8 nSkip)` (0x69F940, thiscall,
/// `this` unused): 8 bytes into the header, one `Read(.., 1)` each into the `offset` argument's stack
/// slot (with `nSkip` its low byte, while the `nSkip` word is 1); `*offset` counting. AL = 0 on a short
/// read, else the last `Read` result / `nSkip` word's low byte (1).
#[hook(addr = 0x69F940, conv = thiscall, name = "CText::ReadChunkHeader", expect = "8B 44 24 10 53 8B 5C 24 0C")]
pub fn read_chunk_header(_this: GPtr<u8>, header: GPtr<u8>, f: GPtr<u8>, offset: GPtr<u8>, skip: u32) -> u8 {
    let mut slot = offset.addr() as u32;
    let mut res = skip;
    let mut i: u16 = 0;
    loop {
        if skip as u8 == 0 {
            res = file_read(f, local(&mut slot), 1);
        }
        if res != 1 {
            return 0;
        }
        wr(header, i as isize, slot as u8);
        inc_u32(offset);
        i = i.wrapping_add(1);
        if i >= 8 {
            break;
        }
    }
    res as u8
}

/// `GxtCharToAscii(const GxtChar* src, uint8 start)` (0x69F7E0, catalogue `FUN_0069f7e0`; the loop
/// test `i < 255` sits in the `.text` chunk 0x4012BB): converts up to 255 characters from `src + start`
/// into the static buffer 0xC1BDD0 (0x80..0xCC shifted by range, 0xCD -> 0xD1, 0xCE -> 0xF1, 0xCF ->
/// 0xBF, 0xD0..0xFF -> '#'), NUL-terminated; returns the buffer. A null `src + start` gives "".
#[hook(addr = 0x69F7E0, conv = cdecl, name = "FUN_0069f7e0", expect = "8A 54 24 08 8B 44 24 04")]
pub fn gxt_char_to_ascii(src: GPtr<u8>, start: u32) -> GPtr<u8> {
    let buf: GPtr<u8> = g(ASCII_BUF);
    let mut p = src;
    if start as u8 != 0 {
        p = src.byte_add(isize::from(start as u8));
    }
    let mut i: i32 = 0;
    if !p.is_null() {
        loop {
            if i >= 0xFF {
                break;
            }
            let c = rd::<u8>(p, 0);
            if c == 0 {
                break;
            }
            let m = match c {
                0x00..=0x7F => c,
                0x80..=0x83 => c.wrapping_add(0x40),
                0x84..=0x8D => c.wrapping_add(0x42),
                0x8E..=0x91 => c.wrapping_add(0x44),
                0x92..=0x95 => c.wrapping_add(0x47),
                0x96..=0x9A => c.wrapping_add(0x49),
                0x9B..=0xA4 => c.wrapping_add(0x4B),
                0xA5..=0xA8 => c.wrapping_add(0x4D),
                0xA9..=0xCC => c.wrapping_add(0x50),
                0xCD => 0xD1,
                0xCE => 0xF1,
                0xCF => 0xBF,
                _ => 0x23,
            };
            wr(buf, i as isize, m);
            p = p.byte_add(1);
            i += 1;
            if p.is_null() {
                break;
            }
        }
    }
    wr::<u8>(buf, i as isize, 0);
    buf
}

/// `sprintf(buf, "<LANGUAGE>.GXT")` for languages 0..4; other values print nothing (the original then
/// opens its uninitialised stack buffer; the port's is zeroed).
fn gxt_file_name(buf: GPtr<u8>) {
    let lang = i32::from(PREFS_LANGUAGE.get() as i8);
    if (0..=4).contains(&lang) {
        sprintf0(buf, g(GXT_NAMES[lang as usize]));
    }
}

/// Reads a chunk header inline (8 single-byte reads into `hdr`, stopping at a short read, `offset`
/// counting).
fn read_header_inline(f: GPtr<u8>, hdr: GPtr<u8>, offset: &mut u32) {
    let mut b = 0u8;
    let mut i: u16 = 0;
    loop {
        if file_read(f, local(&mut b), 1) != 1 {
            break;
        }
        *offset = offset.wrapping_add(1);
        wr(hdr, i as isize, b);
        i = i.wrapping_add(1);
        if i >= 8 {
            break;
        }
    }
}

/// Skips `size` bytes (single-byte reads, results not checked; the counter wraps at 16 bits).
fn skip_bytes(f: GPtr<u8>, size: u32, offset: &mut u32) {
    if size == 0 {
        return;
    }
    let mut b = 0u8;
    let mut i: u16 = 0;
    loop {
        file_read(f, local(&mut b), 1);
        *offset = offset.wrapping_add(1);
        i = i.wrapping_add(1);
        if u32::from(i) >= size {
            break;
        }
    }
}

/// `CText::Load(bool keepMissionPack)` (0x6A01A0, thiscall): `Unload(keepMissionPack)`, the language's
/// GXT from "TEXT" (version and encoding words both read into one slot), then chunks until TKEY and TDAT
/// are loaded: TABL (`CMissionTextOffsets::Load`, offsets loaded flag), TKEY, TDAT, others skipped; key
/// strings rebased, file closed, `GxtCharToAscii(TheText.Get("CDERROR"))` copied to +0x2C, loaded flag
/// (+0x22). Without `keepMissionPack`, a selected mission pack and no mission text: the mission pack
/// text is loaded (timer suspended) while `CheckMissionPackValidMenu` passes, until mission text is
/// loaded or the pack is deselected.
#[hook(addr = 0x6A01A0, conv = thiscall, name = "CText::Load", expect = "83 EC 28 8B 44 24 2C")]
pub fn text_load(this: GPtr<u8>, keep_mission_pack: u32) {
    let mut tkey = false;
    let mut tdat = false;
    wr::<u8>(this, 0x21, 0);
    text_unload_call(this, keep_mission_pack);
    file_set_dir(g(S_TEXT_DIR));
    let mut name = [0u8; 0x14];
    let np: GPtr<u8> = GPtr::from_raw(name.as_mut_ptr());
    gxt_file_name(np);
    let f = file_open(np, g(S_RB));
    let mut word = 0u16;
    file_read(f, local(&mut word), 2);
    file_read(f, local(&mut word), 2);
    let mut offset: u32 = 4;
    let mut hdr = [0u32; 2];
    let hp: GPtr<u8> = local(&mut hdr);
    while !(tkey && tdat) {
        read_header_inline(f, hp, &mut offset);
        let size: u32 = rd(hp, 4);
        if size == 0 {
            continue;
        }
        if crt_strncmp(hp, g(S_TABL), 4) == 0 {
            mission_text_offsets_load_call(this.byte_add(0x12C), size, f, local(&mut offset), 0x58000);
            wr::<u8>(this, 0x21, 1);
        } else if crt_strncmp(hp, g(S_TKEY), 4) == 0 {
            key_array_load_call(this, size, f, local(&mut offset), 0);
            tkey = true;
        } else if crt_strncmp(hp, g(S_TDAT), 4) == 0 {
            data_load_call(this.byte_add(8), size, f, local(&mut offset), 0);
            tdat = true;
        } else {
            skip_bytes(f, size, &mut offset);
        }
    }
    update_keys(this, rd(this, 8));
    file_close(f);
    let s = gxt_char_to_ascii_call(text_get_call(g(THE_TEXT), g(S_CDERROR)), 0);
    let mut i = 0isize;
    loop {
        let c = rd::<u8>(s, i);
        wr(this, 0x2C + i, c);
        if c == 0 {
            break;
        }
        i += 1;
    }
    wr::<u8>(this, 0x22, 1);
    file_set_dir(g(S_EMPTY));
    if keep_mission_pack as u8 != 0
        || g::<u8>(MISSION_PACK).read() == 0
        || rd::<u8>(this, 0x23) != 0
    {
        return;
    }
    loop {
        if g::<u8>(MISSION_PACK).read() == 0 {
            break;
        }
        if check_mission_pack_valid_menu(g(FRONT_END_MENU_MANAGER)) != 0 {
            timer_suspend();
            load_mission_pack_text_call(this);
            file_set_dir(g(S_EMPTY));
            timer_resume();
        }
        if rd::<u8>(this, 0x23) != 0 {
            break;
        }
    }
}

/// `CText::LoadMissionPackText()` (0x69F9A0, thiscall): clears the game's messages, frees the mission
/// tables, clears the mission name, opens `MPACK//MPACK%d//TEXT.GXT` (the selected pack) from the user
/// directory; a handle `<= 0` (signed) or a short version / encoding read fails (mission text flag
/// cleared). Then chunk headers (`ReadChunkHeader`) until the mission TKEY and TDAT are loaded, other
/// chunks skipped (a short read fails); keys rebased, the mission name from 0x870F58 (7 bytes) and the
/// mission text flag set. The file is never closed (as the original).
#[hook(addr = 0x69F9A0, conv = thiscall, name = "CText::LoadMissionPackText", expect = "55 8B EC 83 E4 F8")]
pub fn load_mission_pack_text(this: GPtr<u8>) {
    clear_all_messages_displayed_by_game_call(1);
    file_set_dir(g(S_EMPTY));
    unload_pair(this, 0x10);
    unload_pair(this, 0x18);
    wr::<u8>(this, 0x23, 0);
    wr::<u32>(this, 0x24, 0);
    let mut tkey = false;
    let mut tdat = false;
    wr::<u32>(this, 0x28, 0);
    file_set_dir_my_documents();
    let mut name = [0u8; 0x40];
    let np: GPtr<u8> = GPtr::from_raw(name.as_mut_ptr());
    sprintf1(np, g(S_MPACK_FMT), u32::from(g::<u8>(MISSION_PACK).read()));
    let f = file_open(np, g(S_RB));
    let fail = || wr::<u8>(this, 0x23, 0);
    if (f.addr() as u32 as i32) <= 0 {
        return fail();
    }
    let mut word = 0u16;
    if file_read(f, local(&mut word), 2) != 2 || file_read(f, local(&mut word), 2) != 2 {
        return fail();
    }
    let mut offset: u32 = 4;
    let mut hdr = [0u32; 2];
    let hp: GPtr<u8> = local(&mut hdr);
    while !(tkey && tdat) {
        if read_chunk_header_call(this, hp, f, local(&mut offset), 0) == 0 {
            return fail();
        }
        let size: u32 = rd(hp, 4);
        if size == 0 {
            continue;
        }
        if crt_strncmp(hp, g(S_TKEY), 4) == 0 {
            key_array_load_call(this.byte_add(0x10), size, f, local(&mut offset), 0);
            tkey = true;
        } else if crt_strncmp(hp, g(S_TDAT), 4) == 0 {
            data_load_call(this.byte_add(0x18), size, f, local(&mut offset), 0);
            tdat = true;
        } else {
            let mut b = 0u8;
            let mut i: u16 = 0;
            loop {
                if file_read(f, local(&mut b), 1) != 1 {
                    return fail();
                }
                offset = offset.wrapping_add(1);
                i = i.wrapping_add(1);
                if u32::from(i) >= size {
                    break;
                }
            }
        }
    }
    update_keys(this.byte_add(0x10), rd(this, 0x18));
    wr(this, 0x24, g::<u32>(S_MPACK_NAME).read());
    wr(this, 0x28, g::<u16>(S_MPACK_NAME + 4).read());
    wr(this, 0x2A, g::<u8>(S_MPACK_NAME + 6).read());
    wr::<u8>(this, 0x23, 1);
}

/// `CText::LoadMissionText(const char* mission)` (0x69FBF0, thiscall): nothing with a mission pack
/// selected. Clears the game's messages, frees the mission tables and name, finds the mission in the
/// offsets table (equal u16 length and `strncmp`); if none matches it returns (gta-reversed traps).
/// Otherwise (timer suspended) opens the language's GXT from "TEXT" (uninitialised name for other
/// languages in the original), seeks to the entry's offset (index sign-extended), reads the 8-byte
/// mission name (compare result unused), then chunks until the mission TKEY and TDAT are loaded
/// (headers read inline into the mission argument's slot, others skipped); keys rebased, file closed,
/// timer resumed, directory reset, name copied (unbounded) and the mission text flag set.
#[hook(addr = 0x69FBF0, conv = thiscall, name = "CText::LoadMissionText", expect = "83 EC 34 A0 10 29 B7 00")]
pub fn load_mission_text(this: GPtr<u8>, mission: GPtr<u8>) {
    if g::<u8>(MISSION_PACK).read() != 0 {
        return;
    }
    clear_all_messages_displayed_by_game_call(1);
    unload_pair(this, 0x10);
    unload_pair(this, 0x18);
    wr::<u8>(this, 0x23, 0);
    wr::<u32>(this, 0x24, 0);
    wr::<u32>(this, 0x28, 0);
    let mut len: u32 = 0;
    while rd::<u8>(mission, len as isize) != 0 {
        len = len.wrapping_add(1);
    }
    let count = rd::<u16>(this, 0xA8C);
    let mut found = false;
    let mut idx: u16 = 0;
    if count == 0 {
        return;
    }
    while idx < count {
        let entry = this.byte_add(0x12C + idx as isize * 12);
        let mut n: u32 = 0;
        while rd::<u8>(entry, n as isize) != 0 {
            n = n.wrapping_add(1);
        }
        if len as u16 == n as u16 && crt_strncmp(entry, mission, u32::from(len as u16)) == 0 {
            found = true;
            break;
        }
        idx = idx.wrapping_add(1);
    }
    if !found {
        return;
    }
    file_set_dir(g(S_TEXT_DIR));
    let mut name = [0u8; 0x14];
    let np: GPtr<u8> = GPtr::from_raw(name.as_mut_ptr());
    gxt_file_name(np);
    timer_suspend();
    let file_off: i32 = rd(this, 0x134 + isize::from(idx as i16) * 12);
    let f = file_open(np, g(S_RB));
    file_seek(f, file_off, 0);
    let mut offset: u32 = 8;
    let mut mname = [0u8; 8];
    let mp: GPtr<u8> = GPtr::from_raw(mname.as_mut_ptr());
    for k in 0..8 {
        file_read(f, mp.byte_add(k), 1);
    }
    crt_strncmp(mp, mission, 8);
    let mut tkey = false;
    let mut tdat = false;
    let mut hdr = [0u32; 2];
    let hp: GPtr<u8> = local(&mut hdr);
    while !(tkey && tdat) {
        read_header_inline(f, hp, &mut offset);
        let size: u32 = rd(hp, 4);
        if size == 0 {
            continue;
        }
        if crt_strncmp(hp, g(S_TKEY), 4) == 0 {
            key_array_load_call(this.byte_add(0x10), size, f, local(&mut offset), 0);
            tkey = true;
        } else if crt_strncmp(hp, g(S_TDAT), 4) == 0 {
            data_load_call(this.byte_add(0x18), size, f, local(&mut offset), 0);
            tdat = true;
        } else {
            skip_bytes(f, size, &mut offset);
        }
    }
    update_keys(this.byte_add(0x10), rd(this, 0x18));
    file_close(f);
    timer_resume();
    file_set_dir(g(S_EMPTY));
    let mut i = 0isize;
    loop {
        let c = rd::<u8>(mission, i);
        wr(this, 0x24 + i, c);
        if c == 0 {
            break;
        }
        i += 1;
    }
    wr::<u8>(this, 0x23, 1);
}

/// `FUN_0069ffa0` (0x69FFA0): an unreferenced copy of `CKeyArray::Unload` (thiscall).
#[hook(addr = 0x69FFA0, conv = thiscall, name = "FUN_0069ffa0", expect = "56 8B F1 8B 06 85 C0")]
pub fn key_array_unload_copy(this: GPtr<u8>) {
    unload_pair(this, 0);
}

/// `FUN_0069ffd0` (0x69FFD0): an unreferenced copy of `CData::Unload` (thiscall).
#[hook(addr = 0x69FFD0, conv = thiscall, name = "FUN_0069ffd0", expect = "56 8B F1 8B 06 85 C0")]
pub fn data_unload_copy(this: GPtr<u8>) {
    unload_pair(this, 0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_KEY_ARRAY_LOAD,
    HOOK_DATA_LOAD,
    HOOK_MISSION_TEXT_OFFSETS_LOAD,
    HOOK_READ_CHUNK_HEADER,
    HOOK_GXT_CHAR_TO_ASCII,
    HOOK_TEXT_LOAD,
    HOOK_LOAD_MISSION_PACK_TEXT,
    HOOK_LOAD_MISSION_TEXT,
    HOOK_KEY_ARRAY_UNLOAD_COPY,
    HOOK_DATA_UNLOAD_COPY,
];
