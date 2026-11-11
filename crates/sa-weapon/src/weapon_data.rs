//! `CWeaponInfo::LoadWeaponData` (WRITER-MVP-F batch `mvpf-weapon-1`): the `data\weapon.dat` parser
//! that fills `aWeaponInfo[80]` (0xC8AAB8) and `ms_aWeaponAimOffsets` (0xC8A8A8). Ported from the 1.0
//! US bytes (gta-reversed `WeaponInfo.cpp` a reading aid only). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CAnimManager::ms_numAnimAssocDefinitions` (0xB4EA28), the anim group count the loops scan.
const NUM_ANIM_GROUPS: usize = 0xB4EA28;
/// `aWeaponInfo` / `ms_aWeaponAimOffsets`
const WEAPON_INFOS: usize = 0xC8AAB8;
const AIM_OFFSETS: usize = 0xC8A8A8;

/// `.rdata` strings and constants
const S_WEAPON_DAT: u32 = 0x86AA28; // "DATA\\WEAPON.DAT"
const S_RB: u32 = 0x859C6C; // "rb"
const F_MELEE: u32 = 0x86AA04; // "%s %s %s %f %f %d %d %d %s %d %x %s"
const F_GUN: u32 = 0x86A9A8; // "%s %s %s %f %f %d %d %d %s %d %d %f %f %f %d %d %f %f %d %d %d %d %d %d %d %x %f %f %f %f"
const F_AIM: u32 = 0x86A984; // "%s %s %f %f %f %f %d %d %d %d"
const F_WORD: u32 = 0x85C714; // "%s"
const S_END: u32 = 0x86A974; // "ENDWEAPONDATA"
const S_NULL: u32 = 0x85BBDC; // "null"
const K_1_30: u32 = 0x3D088889; // 0x858F10 (1/30)
const K_50: u32 = 0x42480000; // 0x858B40
const K_0_1: u32 = 0x3DCCCCCD; // 0x858B1C
const K_0_02: u32 = 0x3CA3D70A; // 0x858B38
const K_0_006: u32 = 0x3BC49BA6; // 0x859CDC

orig! {
    /// `CFileMgr::OpenFile(const char*, const char*)` (0x538900)
    fn file_open(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389d0)
    fn file_close(f: GPtr<u8>) -> i32 = cdecl 0x5389d0 => sa_core::filemgr::close_file;
    /// `CFileLoader::LoadLine(FILE*)` (0x536f80; ported in sa-stream, not a dependency of this crate)
    fn load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536f80 => unported;
    /// `CAnimManager::GetAnimGroupName(AssocGroupId)` (0x4d3a20; ported in sa-anim, not a dependency)
    fn get_anim_group_name(i: i32) -> GPtr<u8> = cdecl 0x4d3a20 => unported;
    /// `CWeaponInfo::FindWeaponType(const char*)` (0x743d10)
    fn find_weapon_type(name: GPtr<u8>) -> i32 = cdecl 0x743d10 => crate::weapon_info::find_weapon_type;
    /// `CWeaponInfo::FindWeaponFireType(const char*)` (0x5bcf30)
    fn find_weapon_fire_type(s: GPtr<u8>) -> u32 = cdecl 0x5bcf30 => crate::weapon_info::find_weapon_fire_type;
    /// `FUN_0061db30` (0x61db30): the melee anim group of a name
    fn melee_anim_group(s: GPtr<u8>) -> u32 = cdecl 0x61db30 => crate::weapon_info::melee_anim_group;
    /// CRT `_strncmp` (0x8214d0)
    fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214d0 => unported;
    /// CRT `floor(double)` (0x8219f0, ST0)
    fn floor(v: f64) -> f64 = cdecl 0x8219f0 => sa_world::physical::base::floor_f64;
    /// CRT `_sscanf` (0x8220ad), one target
    fn sscanf1(s: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>) -> i32 = cdecl 0x8220ad => unported;
    /// CRT `_sscanf` (0x8220ad), ten targets
    #[allow(clippy::too_many_arguments)]
    fn sscanf10(s: GPtr<u8>, fmt: GPtr<u8>, a1: GPtr<u8>, a2: GPtr<u8>, a3: GPtr<u8>, a4: GPtr<u8>, a5: GPtr<u8>,
        a6: GPtr<u8>, a7: GPtr<u8>, a8: GPtr<u8>, a9: GPtr<u8>, a10: GPtr<u8>) -> i32 = cdecl 0x8220ad => unported;
    /// CRT `_sscanf` (0x8220ad), twelve targets
    #[allow(clippy::too_many_arguments)]
    fn sscanf12(s: GPtr<u8>, fmt: GPtr<u8>, a1: GPtr<u8>, a2: GPtr<u8>, a3: GPtr<u8>, a4: GPtr<u8>, a5: GPtr<u8>,
        a6: GPtr<u8>, a7: GPtr<u8>, a8: GPtr<u8>, a9: GPtr<u8>, a10: GPtr<u8>, a11: GPtr<u8>, a12: GPtr<u8>) -> i32
        = cdecl 0x8220ad => unported;
    /// CRT `_sscanf` (0x8220ad), thirty targets
    #[allow(clippy::too_many_arguments)]
    fn sscanf30(s: GPtr<u8>, fmt: GPtr<u8>, a1: GPtr<u8>, a2: GPtr<u8>, a3: GPtr<u8>, a4: GPtr<u8>, a5: GPtr<u8>,
        a6: GPtr<u8>, a7: GPtr<u8>, a8: GPtr<u8>, a9: GPtr<u8>, a10: GPtr<u8>, a11: GPtr<u8>, a12: GPtr<u8>,
        a13: GPtr<u8>, a14: GPtr<u8>, a15: GPtr<u8>, a16: GPtr<u8>, a17: GPtr<u8>, a18: GPtr<u8>, a19: GPtr<u8>,
        a20: GPtr<u8>, a21: GPtr<u8>, a22: GPtr<u8>, a23: GPtr<u8>, a24: GPtr<u8>, a25: GPtr<u8>, a26: GPtr<u8>,
        a27: GPtr<u8>, a28: GPtr<u8>, a29: GPtr<u8>, a30: GPtr<u8>) -> i32 = cdecl 0x8220ad => unported;
}

fn ga(addr: u32) -> GPtr<u8> {
    GPtr::from_addr(addr as usize)
}

/// The inlined `strcmp(a, b) == 0` (two bytes per step, stops at the first difference or NUL).
fn str_eq(a: GPtr<u8>, b: GPtr<u8>) -> bool {
    let mut i = 0;
    loop {
        let c = rb(a, i);
        if c != rb(b, i) {
            return false;
        }
        if c == 0 {
            return true;
        }
        i += 1;
    }
}

/// The anim group loop: the first index `0..[0xB4EA28]` whose `GetAnimGroupName` equals `name`, or
/// the count (0 when the count is <= 0) when none does. `GetAnimGroupName` is called once per index.
fn find_anim_group(name: GPtr<u8>) -> (i32, bool) {
    let mut i = 0i32;
    if g::<i32>(NUM_ANIM_GROUPS).read() <= 0 {
        return (0, false);
    }
    loop {
        let n = get_anim_group_name(i);
        if str_eq(name, n) {
            return (i, true);
        }
        i += 1;
        if i >= g::<i32>(NUM_ANIM_GROUPS).read() {
            return (i, false);
        }
    }
}

/// `x - lo` rounded to whole 0.02 steps: `floor((x - lo) * 50 + 0.1) * 0.02 - 0.006 + lo` on the x87
/// stack (the difference stored as a double for `floor`; the result stored as f32).
fn snap(lo: f32, hi: f32) -> f32 {
    let d = x(hi).fsub_m(lo).fmul_m(cf(K_50)).fadd_m(cf(K_0_1));
    X87::from_f64_exact(floor(d.to_f64())).fmul_m(cf(K_0_02)).fsub_m(cf(K_0_006)).fadd_m(lo).to_f32()
}

/// `CWeaponInfo::LoadWeaponData()` (0x5BE670, cdecl): opens `DATA\WEAPON.DAT` (`"rb"`) and reads it
/// with `CFileLoader::LoadLine` until EOF or an `ENDWEAPONDATA` word. Lines that are empty or start
/// with `#` are skipped; otherwise the four string slots S+0xC0/0xE0/0x100/0xA0 get a NUL first byte
/// and the first character selects:
/// - `£` (0xA3, melee): `"%s %s %s %f %f %d %d %d %s %d %x %s"`; `info = aWeaponInfo[FindWeaponType]`
///   (the std-skill slot): +0 fire type, +4/+8 ranges, +0xC/+0x10 model ids, +0x14 slot, +0x6F the
///   `%d` byte, +0x6E `FUN_0061db30(melee group name)`, +0x18 flags (`%x`); unless the anim group word
///   starts with `null` (`strncmp(.., 4)`), +0x1C = the index of the matching anim group (untouched when
///   none matches). Then `ms_modelInfoPtrs[model1]->+0x24 = type` when model1 > 0.
/// - `$` (gun): the 30-field format; S+0x3C..0x48 (the four last `%f`) are zeroed before the call.
///   Skill index: types 22..32 use the skill field (0 → type+25, 1 → type, 2 → type+36, 3 → type+47,
///   other values keep the previous S+0x18, an uninitialised/stale local); other types force skill 1.
///   Fills +0..+0x68 of `aWeaponInfo[(int16)index]` (the four animation times `fild * 1/30`), the anim
///   group (as above), `+0x6C = group - 11` for groups 11..31, then snaps +0x44 and +0x50 to 0.02 steps
///   above +0x40 / +0x4C (`floor`). With skill 1 and type != 40 → the model info write as above.
/// - `%` (aim offsets): `"%s %s %f %f %f %f %d %d %d %d"`, the anim group index (count when not found)
///   minus 11 selects `ms_aWeaponAimOffsets` (0x18 each): +0/+4/+8/+0xC floats, +0x10..+0x16 int16.
/// - anything else: `"%s"`; `ENDWEAPONDATA` (13 chars) ends the loop.
///
/// Closes the file. All locals live in one 0x120-byte frame image (offset = the original's `esp+k`
/// after the prologue), persistent across lines like the original's: fields `sscanf` does not match
/// keep the previous line's values.
#[hook(addr = 0x5BE670, conv = cdecl, name = "CWeaponInfo::LoadWeaponData", expect = "55 8B EC 83 E4 F8 81 EC 10 01 00 00",
       unverified = "mvpf-weapon-1", route = "split", hazards = "uninit-locals",
       recs = "grove-walk", note = "file read (DATA/WEAPON.DAT) on the boot path: a second run reads the file again; one-time init")]
pub fn load_weapon_data() {
    let mut frame = [0u32; 0x120 / 4];
    let base: GPtr<u8> = GPtr::from_raw(&mut frame as *mut [u32; 0x120 / 4]).cast();
    let s = |k: isize| base.byte_add(k);
    let si = |k: isize| ru(base, k) as i32;
    let f = file_open(ga(S_WEAPON_DAT), ga(S_RB));
    wp(base, 0x78, f);
    let mut line = load_line(f);
    while !line.is_null() {
        let c = rb(line, 0);
        if c != 0 && c != b'#' {
            wb(base, 0xC0, 0);
            wb(base, 0xE0, 0);
            wb(base, 0x100, 0);
            wb(base, 0xA0, 0);
            // the model-info write at 0x5BEC3E: `Some(type)` when reached
            let mut set_model: Option<i32> = None;
            match c {
                0xA3 => {
                    sscanf12(line, ga(F_MELEE), s(0x98), s(0xC0), s(0xE0), s(0x10), s(0x14), s(0x1C), s(0x24), s(0x28),
                        s(0x100), s(0x64), s(0x20), s(0xA0));
                    let ty = find_weapon_type(s(0xC0));
                    let info = g::<u8>(WEAPON_INFOS).byte_add((ty as isize).wrapping_mul(0x70));
                    let ft = find_weapon_fire_type(s(0xE0));
                    wu(info, 4, ru(base, 0x10));
                    wu(info, 0, ft);
                    wu(info, 0x10, ru(base, 0x24));
                    wu(info, 8, ru(base, 0x14));
                    wu(info, 0xC, ru(base, 0x1C));
                    wu(info, 0x14, ru(base, 0x28));
                    wb(info, 0x6F, rb(base, 0x64));
                    let mg = melee_anim_group(s(0x100));
                    wb(info, 0x6E, mg as u8);
                    wu(info, 0x18, ru(base, 0x20));
                    if strncmp(s(0xA0), ga(S_NULL), 4) != 0 {
                        let (gi, found) = find_anim_group(s(0xA0));
                        if found {
                            wu(info, 0x1C, gi as u32);
                        }
                    }
                    set_model = Some(ty);
                }
                b'$' => {
                    wu(base, 0x40, 0);
                    wu(base, 0x44, 0);
                    wu(base, 0x48, 0);
                    wu(base, 0x3C, 0);
                    sscanf30(line, ga(F_GUN), s(0x98), s(0xC0), s(0xE0), s(0x10), s(0x14), s(0x1C), s(0x24), s(0x28),
                        s(0xA0), s(0x90), s(0x54), s(0x30), s(0x34), s(0x38), s(0x2C), s(0x8C), s(0x5C), s(0x94), s(0x4C),
                        s(0x88), s(0x6C), s(0x80), s(0x74), s(0x84), s(0x7C), s(0x20), s(0x3C), s(0x48), s(0x44), s(0x40));
                    let ty = find_weapon_type(s(0xC0));
                    wu(base, 0x70, ty as u32);
                    let mut skill = si(0x2C);
                    if (0x16..=0x20).contains(&ty) {
                        match skill {
                            0 => wu(base, 0x18, ty.wrapping_add(0x19) as u32),
                            1 => wu(base, 0x18, ty as u32),
                            2 => wu(base, 0x18, ty.wrapping_add(0x24) as u32),
                            3 => wu(base, 0x18, ty.wrapping_add(0x2F) as u32),
                            _ => {} // S+0x18 keeps its previous value
                        }
                    } else {
                        skill = 1;
                        wu(base, 0x2C, 1);
                        wu(base, 0x18, ty as u32);
                    }
                    let idx = i32::from(rh(base, 0x18) as i16);
                    let info = g::<u8>(WEAPON_INFOS).byte_add((idx as isize).wrapping_mul(0x70));
                    let ft = find_weapon_fire_type(s(0xE0));
                    let t30 = |k: isize| xi(si(k)).fmul_m(cf(K_1_30)).to_f32();
                    wu(info, 0, ft);
                    wu(info, 4, ru(base, 0x10));
                    wu(info, 8, ru(base, 0x14));
                    wf(info, 0x40, t30(0x4C));
                    wu(info, 0xC, ru(base, 0x1C));
                    wu(info, 0x10, ru(base, 0x24));
                    wu(info, 0x14, ru(base, 0x28));
                    wh(info, 0x20, rh(base, 0x90));
                    wf(info, 0x44, t30(0x88));
                    wh(info, 0x22, rh(base, 0x54));
                    wu(info, 0x24, ru(base, 0x30));
                    wu(info, 0x28, ru(base, 0x34));
                    wu(info, 0x2C, ru(base, 0x38));
                    wf(info, 0x48, t30(0x6C));
                    wu(info, 0x34, ru(base, 0x8C));
                    wf(info, 0x4C, t30(0x80));
                    wu(info, 0x3C, ru(base, 0x94));
                    wu(info, 0x38, ru(base, 0x5C));
                    wu(info, 0x60, ru(base, 0x48));
                    wu(info, 0x18, ru(base, 0x20));
                    wf(info, 0x50, t30(0x74));
                    wu(info, 0x5C, ru(base, 0x3C));
                    wu(info, 0x30, skill as u32);
                    wu(info, 0x64, ru(base, 0x44));
                    wf(info, 0x54, t30(0x84));
                    wu(info, 0x68, ru(base, 0x40));
                    wf(info, 0x58, t30(0x7C));
                    if strncmp(s(0xA0), ga(S_NULL), 4) != 0 {
                        let (gi, found) = find_anim_group(s(0xA0));
                        if found {
                            wu(info, 0x1C, gi as u32);
                        }
                    }
                    let grp = ru(info, 0x1C) as i32;
                    if (0xB..=0x1F).contains(&grp) {
                        wh(info, 0x6C, (grp as u16).wrapping_sub(0xB));
                    }
                    let model = si(0x2C) == 1 && ty != 0x28;
                    wf(info, 0x44, snap(rf(info, 0x40), rf(info, 0x44)));
                    wf(info, 0x50, snap(rf(info, 0x4C), rf(info, 0x50)));
                    if model {
                        set_model = Some(si(0x70));
                    }
                }
                b'%' => {
                    sscanf10(line, ga(F_AIM), s(0x98), s(0xA0), s(0x30), s(0x38), s(0x10), s(0x14), s(0x50), s(0x58),
                        s(0x60), s(0x68));
                    let (gi, _) = find_anim_group(s(0xA0));
                    let o = g::<u8>(AIM_OFFSETS).byte_add((gi.wrapping_sub(0xB) as isize).wrapping_mul(0x18));
                    wu(o, 0, ru(base, 0x30));
                    wu(o, 4, ru(base, 0x38));
                    wu(o, 8, ru(base, 0x10));
                    wu(o, 0xC, ru(base, 0x14));
                    wh(o, 0x10, rh(base, 0x50));
                    wh(o, 0x12, rh(base, 0x58));
                    wh(o, 0x14, rh(base, 0x60));
                    wh(o, 0x16, rh(base, 0x68));
                }
                _ => {
                    sscanf1(line, ga(F_WORD), s(0xC0));
                    if strncmp(s(0xC0), ga(S_END), 13) == 0 {
                        break;
                    }
                }
            }
            if let Some(ty) = set_model {
                let m = si(0x1C);
                if m > 0 {
                    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * m as usize).read();
                    wu(mi, 0x24, ty as u32);
                }
            }
        }
        line = load_line(rp(base, 0x78));
    }
    file_close(rp(base, 0x78));
}

pub const HOOKS: &[HookReg] = &[HOOK_LOAD_WEAPON_DATA];
