//! SAVE-1: the ped blocks of the save game (gta_sa.exe 1.0 US): `CPlayerInfo::Save` / `Load` and their
//! save structure (0x5D24B0 / 0x5D2560), `CPedType::Save` / `Load`, `CPed::Save` / `Load` with
//! `CPedSaveStructure::Construct` / `Extract`, `CPlayerPed::Save` / `Load`. Ported from the bytes (objdump;
//! `docs/re/specs/SAVE/CPedSaveStructure.md`); docs/arch/save-port.md.
//!
//! The save structures are uninitialised stack locals the originals write whole after filling them only
//! partly: `CPlayerInfo::Save` (0x28 bytes at entry ESP -0x28: +7, +0x11..+0x13, +0x25 unwritten),
//! `CPed::Save` (0x18C bytes at entry ESP -0x198: +0x183, +0x18A, +0x18B), `CPlayerPed::Save` (0x84 bytes in an
//! `and esp, -8` frame: +5..+7, +0x81..+0x83). The ports take those bytes from the footprint snapshot (ADR-001
//! §5b; the aligned frame through `footprint_entry_esp`), which equals the original's residue whenever the
//! caller is original code. `CPed::Save` / `Load` run the `eh vector constructor / destructor iterator`s of the
//! structure's 13 `CWeapon`s in the original; the constructor's bytes are all overwritten by the copy / the
//! read, and `CWeapon::~CWeapon` (0x4411A0) is a lone `ret`, so the ports skip them.

use sa_core::abi::{HookReg, footprint_entry_esp, footprint_residue_u8};
use sa_core::{hook, orig};

use crate::common::*;

const WINDOW: u32 = 0x1000;
/// `CEntryExitManager::mp_poolEntryExits`
const ENTRY_EXIT_POOL: usize = 0x96A7D8;
/// `CPedType::ms_apPedTypes` (pointer to 32 `CAcquaintance` of 5 dwords)
const PED_TYPES: usize = 0xC0BBE8;

orig! {
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
    fn save_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1270 => sa_core::save::save_data_to_work_buffer;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
    fn load_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1300 => sa_core::save::load_data_from_work_buffer;
    /// `CPlayerInfoSaveStructure::Construct(CPlayerInfo*)` (0x5D24B0)
    fn player_info_save_construct(s: GPtr<u8>, info: GPtr<u8>) = thiscall 0x5D24B0 => player_info_save_structure_construct;
    /// `CPlayerInfoSaveStructure::Extract(CPlayerInfo*)` (0x5D2560)
    fn player_info_save_extract(s: GPtr<u8>, info: GPtr<u8>) = thiscall 0x5D2560 => player_info_save_structure_extract;
    /// `CAcquaintance::GetAcquaintances(int32)` (0x608970)
    fn get_acquaintances(a: GPtr<u8>, i: u32) -> u32 = thiscall 0x608970 => unported;
    /// `CAcquaintance::SetAcquaintances(int32, uint32)` (0x608960)
    fn set_acquaintances(a: GPtr<u8>, i: u32, v: u32) -> u32 = thiscall 0x608960 => unported;
    /// `CPedSaveStructure::Construct(CPed*)` (0x5D43D0)
    fn ped_save_construct(s: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5D43D0 => ped_save_structure_construct;
    /// `CPedSaveStructure::Extract(CPed*)` (0x5D44B0)
    fn ped_save_extract(s: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5D44B0 => ped_save_structure_extract;
    /// `CPed::Save()` (0x5D5730)
    fn ped_save_call(ped: GPtr<u8>) -> u8 = thiscall 0x5D5730 => ped_save;
    /// `CPed::Load()` (0x5D4640)
    fn ped_load_call(ped: GPtr<u8>) -> u8 = thiscall 0x5D4640 => ped_load;
    /// `CPedClothesDesc::CPedClothesDesc()` (0x5A8020)
    fn clothes_desc_ctor(d: GPtr<u8>) -> GPtr<u8> = thiscall 0x5A8020 => crate::clothes::desc_constructor;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0)
    fn set_char_created_by(ped: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn get_weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    fn request_model(model: i32, flags: u32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    fn load_all_requested_models(priority: u32) = cdecl 0x40EA10 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32, bool)` (0x5E6080)
    fn give_weapon(ped: GPtr<u8>, ty: u32, ammo: u32, likely_to_be_held: u32) -> u32 = thiscall 0x5E6080 => unported;
    /// `CPed::SetCurrentWeapon(int32 slot)` (0x5E61F0)
    fn set_current_weapon(ped: GPtr<u8>, slot: i32) = thiscall 0x5E61F0 => unported;
}

/// A residue-filled local: byte k of the returned array is the original's stack byte at entry ESP + `base`
/// + k (0 outside a footprint wrapper).
fn residue<const N: usize>(base: i32) -> [u8; N] {
    let mut b = [0u8; N];
    for (k, v) in b.iter_mut().enumerate() {
        *v = footprint_residue_u8(base + k as i32, WINDOW);
    }
    b
}

/// An `f32` as `fld dword` / `fstp dword` copy it: a signalling NaN comes out quiet.
fn x87_copy(bits: u32) -> u32 {
    if bits & 0x7F80_0000 == 0x7F80_0000 && bits & 0x7F_FFFF != 0 { bits | 0x40_0000 } else { bits }
}

// ----------------------------------------------------------------------------- CPlayerInfo -----

/// `CPlayerInfo::Save()` (0x5D3AC0, thiscall): the structure size 0x28 (4) and the 0x28-byte structure
/// `Construct`ed from this player info (the residue local, see the module doc). AL = 1.
#[hook(addr = 0x5D3AC0, conv = thiscall, name = "CPlayerInfo::Save", expect = "83 EC 2C 51 8D 4C 24 08", this = "CPlayerInfo",
       route = "split", hazards = "uninit-locals", recs = "save-load", footprint = 0x1000)]
pub fn player_info_save(this: GPtr<u8>) -> u8 {
    let mut size = 0x28u32;
    let mut s = residue::<0x28>(-0x28);
    let sp = GPtr::<u8>::from_raw(s.as_mut_ptr());
    player_info_save_construct(sp, this);
    save_data(GPtr::from_raw(&mut size).cast(), 4);
    save_data(sp, 0x28);
    1
}

/// `CPlayerInfo::Load()` (0x5D3B00, thiscall): the size (4, ignored) and the 0x28-byte structure read, then
/// `Extract`ed into this player info. AL = 1.
#[hook(addr = 0x5D3B00, conv = thiscall, name = "CPlayerInfo::Load", expect = "83 EC 2C 56 8D 44 24 04", this = "CPlayerInfo",
       route = "split", hazards = "none", recs = "save-load")]
pub fn player_info_load(this: GPtr<u8>) -> u8 {
    let mut size = 0u32;
    let mut s = [0u8; 0x28];
    let sp = GPtr::<u8>::from_raw(s.as_mut_ptr());
    load_data(GPtr::from_raw(&mut size).cast(), 4);
    load_data(sp, 0x28);
    player_info_save_extract(sp, this);
    1
}

/// The (structure offset, player info offset, size) pairs of the player info save structure, in the bytes'
/// order (`Construct`).
const PLAYER_INFO_FIELDS: [(isize, isize, u8); 17] = [
    (0x0, 0xB8, 4),
    (0x4, 0x130, 2),
    (0x6, 0xDC, 1),
    (0x8, 0x134, 4),
    (0xC, 0xBC, 4),
    (0x10, 0x144, 1),
    (0x14, 0xC0, 4),
    (0x18, 0xC4, 4),
    (0x1C, 0x14C, 1),
    (0x1D, 0x14D, 1),
    (0x1E, 0x14E, 1),
    (0x1F, 0x14F, 1),
    (0x20, 0x150, 1),
    (0x21, 0x151, 1),
    (0x22, 0x152, 1),
    (0x23, 0x153, 1),
    (0x24, 0x154, 1),
];

fn copy_field(dst: GPtr<u8>, src: GPtr<u8>, n: u8) {
    match n {
        1 => dst.write(src.read()),
        2 => dst.cast::<u16>().write(src.cast::<u16>().read()),
        _ => dst.cast::<u32>().write(src.cast::<u32>().read()),
    }
}

/// `CPlayerInfoSaveStructure::Construct(CPlayerInfo*)` (0x5D24B0, thiscall, `ret 4`): money (+0xB8 -> +0),
/// the +0x130 word, the player state byte, the +0x134 / +0xBC dwords, the +0x144 byte, the +0xC0 / +0xC4 dwords,
/// the bytes +0x14C..+0x154 (-> +0x1C..+0x24), the +0x156 word (-> +0x26). +7, +0x11..+0x13, +0x25 are not
/// written.
#[hook(addr = 0x5D24B0, conv = thiscall, name = "FUN_005d24b0", expect = "8B 44 24 04 8B 90 B8 00 00 00",
       route = "split", hazards = "none", recs = "save-load")]
pub fn player_info_save_structure_construct(this: GPtr<u8>, info: GPtr<u8>) {
    for (s, i, n) in PLAYER_INFO_FIELDS {
        copy_field(this.byte_add(s), info.byte_add(i), n);
    }
    copy_field(this.byte_add(0x26), info.byte_add(0x156), 2);
}

/// `CPlayerInfoSaveStructure::Extract(CPlayerInfo*)` (0x5D2560, thiscall, `ret 4`): the inverse of
/// `Construct` in the same order, the +0x130 word written twice, the +0x144 field written as the
/// zero-extended word of the structure's byte +0x10.
#[hook(addr = 0x5D2560, conv = thiscall, name = "FUN_005d2560", expect = "8B 11 8B 44 24 04 89 90 B8 00 00 00",
       route = "split", hazards = "none", recs = "save-load")]
pub fn player_info_save_structure_extract(this: GPtr<u8>, info: GPtr<u8>) {
    for (s, i, n) in PLAYER_INFO_FIELDS {
        match s {
            0x6 => {
                copy_field(info.byte_add(i), this.byte_add(s), n);
                copy_field(info.byte_add(0x130), this.byte_add(4), 2);
            }
            0x10 => info.byte_add(0x144).cast::<u16>().write(u16::from(this.byte_add(0x10).read())),
            _ => copy_field(info.byte_add(i), this.byte_add(s), n),
        }
    }
    copy_field(info.byte_add(0x156), this.byte_add(0x26), 2);
}

// -------------------------------------------------------------------------------- CPedType -----

/// `CPedType::Save()` (0x5D3CD0): for the 32 ped types and their 5 acquaintance sets, `GetAcquaintances(k)`
/// into a dword local, written (4). AL = 1.
#[hook(addr = 0x5D3CD0, conv = cdecl, name = "CPedType::Save", expect = "51 56 57 33 FF 33 F6",
       route = "split", hazards = "none", recs = "save-load")]
pub fn ped_type_save() -> u8 {
    let mut v = [0u32; 1];
    let vp = GPtr::<u8>::from_raw(v.as_mut_ptr().cast());
    for t in 0..32u32 {
        for k in 0..5u32 {
            let a = g::<GPtr<u8>>(PED_TYPES).read().byte_add((t * 0x14) as isize);
            vp.cast::<u32>().write(get_acquaintances(a, k));
            save_data(vp, 4);
        }
    }
    1
}

/// `CPedType::Load()` (0x5D3D10): each value read (4) and stored with `SetAcquaintances(k, value)`. AL = 1.
#[hook(addr = 0x5D3D10, conv = cdecl, name = "CPedType::Load", expect = "51 56 57 33 FF 33 F6",
       route = "split", hazards = "none", recs = "save-load")]
pub fn ped_type_load() -> u8 {
    let mut v = [0u32; 1];
    let vp = GPtr::<u8>::from_raw(v.as_mut_ptr().cast());
    for t in 0..32u32 {
        for k in 0..5u32 {
            load_data(vp, 4);
            let a = g::<GPtr<u8>>(PED_TYPES).read().byte_add((t * 0x14) as isize);
            set_acquaintances(a, k, vp.cast::<u32>().read());
        }
    }
    1
}

// -------------------------------------------------------------------------------- CPed ---------

/// `CPed::Save()` (0x5D5730, thiscall): the size 0x18C (4) and the `CPedSaveStructure` (0x18C bytes, the
/// residue local) `Construct`ed from this ped. AL = 1.
#[hook(addr = 0x5D5730, conv = thiscall, name = "CPed::Save", expect = "6A FF 68 1B D7 83 00", this = "CPed",
       route = "split", hazards = "uninit-locals", recs = "save-load", footprint = 0x1000)]
pub fn ped_save(this: GPtr<u8>) -> u8 {
    let mut size = 0x18Cu32;
    let mut s = residue::<0x18C>(-0x198);
    let sp = GPtr::<u8>::from_raw(s.as_mut_ptr());
    ped_save_construct(sp, this);
    save_data(GPtr::from_raw(&mut size).cast(), 4);
    save_data(sp, 0x18C);
    1
}

/// `CPed::Load()` (0x5D4640, thiscall): the size (4) and the 0x18C-byte structure read, `Extract`ed into this
/// ped. AL = 1.
#[hook(addr = 0x5D4640, conv = thiscall, name = "CPed::Load", expect = "6A FF 68 DB D6 83 00", this = "CPed",
       route = "split", hazards = "none", recs = "save-load")]
pub fn ped_load(this: GPtr<u8>) -> u8 {
    let mut size = 0u32;
    let mut s = [0u8; 0x18C];
    let sp = GPtr::<u8>::from_raw(s.as_mut_ptr());
    load_data(GPtr::from_raw(&mut size).cast(), 4);
    load_data(sp, 0x18C);
    ped_save_extract(sp, this);
    1
}

/// `(p - objects) / 0x3C` as `imul 0x88888889; sar 5` rounds it (toward zero).
fn entry_exit_index(p: u32, objects: u32) -> i32 {
    let d = p.wrapping_sub(objects) as i32;
    let hi = ((i64::from(d) * i64::from(0x8888_8889u32 as i32)) >> 32) as i32;
    let q = hi.wrapping_add(d) >> 5;
    q.wrapping_add((q as u32 >> 31) as i32)
}

/// `CPedSaveStructure::Construct(CPed*)` (0x5D43D0, thiscall, `ret 4`): position (matrix +0x30 or
/// placement), health, armour, created-by, active weapon slot, area, +0x72D / +0x72E; +0x184 = -1, or the
/// pool index of the ped's entry-exit (+0x78C) when it (or its link +0x38) has the +0x32 byte set; then the
/// 13 weapons (0x16C bytes from +0x5A0).
#[hook(addr = 0x5D43D0, conv = thiscall, name = "CPedSaveStructure::Construct", expect = "56 8B 74 24 08 8B 46 14",
       route = "split", hazards = "none", recs = "save-load")]
pub fn ped_save_structure_construct(this: GPtr<u8>, ped: GPtr<u8>) {
    let m = rd::<u32>(ped, 0x14);
    let pos = if m != 0 { g::<u8>(m as usize + 0x30) } else { ped.byte_add(4) };
    for k in 0..3isize {
        wr::<u32>(this, 4 * k, rd::<u32>(pos, 4 * k));
    }
    wr::<u32>(this, 0xC, rd::<u32>(ped, 0x540));
    wr::<u32>(this, 0x10, rd::<u32>(ped, 0x548));
    wr::<u8>(this, 0x180, rd::<u8>(ped, 0x484));
    wr::<u8>(this, 0x181, rd::<u8>(ped, 0x718));
    wr::<u8>(this, 0x182, rd::<u8>(ped, 0x2F));
    wr::<u8>(this, 0x188, rd::<u8>(ped, 0x72D));
    wr::<u8>(this, 0x189, rd::<u8>(ped, 0x72E));
    wr::<i32>(this, 0x184, -1);
    let ee = rd::<u32>(ped, 0x78C);
    if ee != 0 {
        let link = g::<u32>(ee as usize + 0x38).read();
        let t = if link != 0 { link } else { ee };
        if g::<u8>(t as usize + 0x32).read() != 0 {
            let objects = g::<GPtr<u32>>(ENTRY_EXIT_POOL).read().read();
            wr::<i32>(this, 0x184, entry_exit_index(ee, objects));
        }
    }
    for k in 0..0x5Bisize {
        wr::<u32>(this, 0x14 + 4 * k, rd::<u32>(ped, 0x5A0 + 4 * k));
    }
}

/// `CPedSaveStructure::Extract(CPed*)` (0x5D44B0, thiscall, `ret 4`): position (x87 copies) into the matrix
/// or the placement, health, armour, the weapon slot byte, `SetCharCreatedBy`, +0x72D / +0x72E; each saved
/// weapon with a type: its models requested and loaded (`GetWeaponInfo(type, 1)` +0xC / +0x10, fetched for
/// each), `GiveWeapon(type, total ammo, false)` and the ammo in the clip; `SetCurrentWeapon((int8)slot)`;
/// area; the entry-exit from its pool index (null for -1 or a free slot).
#[hook(addr = 0x5D44B0, conv = thiscall, name = "CPedSaveStructure::Extract", expect = "53 57 8B 7C 24 0C 8B 47 14",
       route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn ped_save_structure_extract(this: GPtr<u8>, ped: GPtr<u8>) {
    let (z, y, x) = (x87_copy(rd::<u32>(this, 8)), x87_copy(rd::<u32>(this, 4)), x87_copy(rd::<u32>(this, 0)));
    let m = rd::<u32>(ped, 0x14);
    if m != 0 {
        g::<u32>(m as usize + 0x30).write(x);
        g::<u32>(rd::<u32>(ped, 0x14) as usize + 0x34).write(y);
        g::<u32>(rd::<u32>(ped, 0x14) as usize + 0x38).write(z);
    } else {
        wr::<u32>(ped, 4, x);
        wr::<u32>(ped, 8, y);
        wr::<u32>(ped, 0xC, z);
    }
    wr::<u32>(ped, 0x540, rd::<u32>(this, 0xC));
    wr::<u32>(ped, 0x548, rd::<u32>(this, 0x10));
    wr::<u8>(ped, 0x718, rd::<u8>(this, 0x181));
    set_char_created_by(ped, u32::from(rd::<u8>(this, 0x180)));
    wr::<u8>(ped, 0x72D, rd::<u8>(this, 0x188));
    wr::<u8>(ped, 0x72E, rd::<u8>(this, 0x189));
    for k in 0..13isize {
        let w = this.byte_add(0x14 + 0x1C * k);
        let ty = rd::<u32>(w, 0);
        if ty == 0 {
            continue;
        }
        for field in [0xCisize, 0x10] {
            let model = rd::<i32>(get_weapon_info(rd::<u32>(w, 0), 1), field);
            if model != -1 {
                request_model(model, 8);
                load_all_requested_models(0);
            }
        }
        give_weapon(ped, rd::<u32>(w, 0), rd::<u32>(w, 0xC), 0);
        wr::<u32>(ped, 0x5A8 + 0x1C * k, rd::<u32>(w, 8));
    }
    set_current_weapon(ped, i32::from(rd::<u8>(this, 0x181) as i8));
    wr::<u8>(ped, 0x2F, rd::<u8>(this, 0x182));
    let idx = rd::<i32>(this, 0x184);
    let ee = if idx == -1 {
        0
    } else {
        let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
        let flags = pool.byte_add(4).cast::<GPtr<u8>>().read();
        if (flags.byte_add(idx as isize).read() as i8) < 0 {
            0
        } else {
            pool.cast::<u32>().read().wrapping_add((idx as u32).wrapping_mul(0x3C))
        }
    };
    wr::<u32>(ped, 0x78C, ee);
}

// --------------------------------------------------------------------------- CPlayerPed --------

/// `CPlayerPed::Save()` (0x5D57E0, thiscall; `and esp, -8` frame): a `CPlayerPedSaveStructure` (0x84 bytes at
/// `((entry ESP - 4) & ~7) - 0x88`): `CPedClothesDesc`'s constructor run at +8, then +0 the wanted record's
/// dword +0 and +4 its byte +0x2C, +8 the 0x78 clothes bytes, +0x80 the player data's byte +0x20 (the residue
/// bytes +5..+7 and +0x81..+0x83 come from the snapshot); then `CPed::Save()`, the size 0x84 (4) and the
/// structure. AL = 1.
#[hook(addr = 0x5D57E0, conv = thiscall, name = "CPlayerPed::Save", expect = "55 8B EC 83 E4 F8", this = "CPlayerPed",
       route = "split", hazards = "uninit-locals", recs = "save-load", footprint = 0x1000)]
pub fn player_ped_save(this: GPtr<u8>) -> u8 {
    // The structure's place relative to the entry ESP (aligned frame): -0x8C or -0x90.
    let base = footprint_entry_esp().map_or(-0x8C, |e| ((e.wrapping_sub(4) & !7).wrapping_sub(0x88)).wrapping_sub(e) as i32);
    let mut size = 0x84u32;
    let mut s = residue::<0x84>(base);
    let sp = GPtr::<u8>::from_raw(s.as_mut_ptr());
    clothes_desc_ctor(sp.byte_add(8));
    let data = rd::<GPtr<u8>>(this, 0x480);
    let wanted = rd::<GPtr<u8>>(data, 0);
    wr::<u32>(sp, 0, rd::<u32>(wanted, 0));
    wr::<u8>(sp, 4, rd::<u8>(rd::<GPtr<u8>>(data, 0), 0x2C));
    let clothes = rd::<GPtr<u8>>(data, 4);
    for k in 0..0x1Eisize {
        wr::<u32>(sp, 8 + 4 * k, rd::<u32>(clothes, 4 * k));
    }
    wr::<u8>(sp, 0x80, rd::<u8>(data, 0x20));
    ped_save_call(this);
    save_data(GPtr::from_raw(&mut size).cast(), 4);
    save_data(sp, 0x84);
    1
}

/// `CPlayerPed::Load()` (0x5D46E0, thiscall): `CPedClothesDesc`'s constructor on the local's +8, `CPed::Load()`,
/// the size (4) and the 0x84-byte structure read; the wanted record's +0 dword and +0x2C (a dword: the
/// zero-extended byte +4), the 0x78 clothes bytes, the player data's byte +0x20 restored. AL = 1.
#[hook(addr = 0x5D46E0, conv = thiscall, name = "CPlayerPed::Load", expect = "81 EC 88 00 00 00", this = "CPlayerPed",
       route = "split", hazards = "none", recs = "save-load")]
pub fn player_ped_load(this: GPtr<u8>) -> u8 {
    let mut size = 0u32;
    let mut s = [0u8; 0x84];
    let sp = GPtr::<u8>::from_raw(s.as_mut_ptr());
    clothes_desc_ctor(sp.byte_add(8));
    ped_load_call(this);
    load_data(GPtr::from_raw(&mut size).cast(), 4);
    load_data(sp, 0x84);
    let data = rd::<GPtr<u8>>(this, 0x480);
    wr::<u32>(rd::<GPtr<u8>>(data, 0), 0, rd::<u32>(sp, 0));
    wr::<u32>(rd::<GPtr<u8>>(data, 0), 0x2C, u32::from(rd::<u8>(sp, 4)));
    let clothes = rd::<GPtr<u8>>(data, 4);
    for k in 0..0x1Eisize {
        wr::<u32>(clothes, 4 * k, rd::<u32>(sp, 8 + 4 * k));
    }
    wr::<u8>(data, 0x20, rd::<u8>(sp, 0x80));
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PLAYER_INFO_SAVE,
    HOOK_PLAYER_INFO_LOAD,
    HOOK_PLAYER_INFO_SAVE_STRUCTURE_CONSTRUCT,
    HOOK_PLAYER_INFO_SAVE_STRUCTURE_EXTRACT,
    HOOK_PED_TYPE_SAVE,
    HOOK_PED_TYPE_LOAD,
    HOOK_PED_SAVE,
    HOOK_PED_LOAD,
    HOOK_PED_SAVE_STRUCTURE_CONSTRUCT,
    HOOK_PED_SAVE_STRUCTURE_EXTRACT,
    HOOK_PLAYER_PED_SAVE,
    HOOK_PLAYER_PED_LOAD,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(entry_exit_index(0x2000_0000 + 5 * 0x3C, 0x2000_0000), 5);
        assert_eq!(x87_copy(0x7F80_0001), 0x7FC0_0001);
        assert_eq!(x87_copy(0x3F80_0000), 0x3F80_0000);
        assert_eq!(x87_copy(0x7F80_0000), 0x7F80_0000);
    }
}
