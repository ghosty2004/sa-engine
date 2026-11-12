//! `CClothes` / `CPedClothesDesc` data helpers (PORT-WEAPON-PED). Ported from the 1.0 US bytes
//! (gta-reversed `Clothes.cpp`, `PedClothesDesc.cpp`, `docs/re/specs/PED/CClothes*.md`).
//!
//! `CPedClothesDesc` (0x78): +0 model keys[10], +0x28 texture keys[18], +0x70 fat stat, +0x74 muscle stat.
//! `PlayerClothes` (0xBC1C78) is the CPedClothesDesc the player was last built with.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

const _: () = assert!(sa_core::layouts::size::CPedClothesDesc == 0x78);

sa_core::globals! {
    list GLOBALS;
    /// `PlayerClothes` (the player's current `CPedClothesDesc`)
    pub static PLAYER_CLOTHES @ 0xBC1C78: [u32; 30] = [0; 30];
    /// `CClothes::ms_clothesImageId`
    pub static CLOTHES_IMAGE_ID @ 0xBC12F8: i32 = 0;
    /// `CClothes::ms_numRuleTags`
    pub static NUM_RULE_TAGS @ 0xBC12FC: i32 = 0;
}

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CTxdStore::ms_pTxdPool`
const TXD_POOL: usize = 0xC8800C;

orig! {
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    pub fn get_uppercase_key(s: u32) -> u32 = cdecl 0x53CF30 => unported;
    /// `CClothes::GetTextureDependency(int32)` (0x5A7EA0)
    pub fn get_texture_dependency_call(t: i32) -> i32 = cdecl 0x5A7EA0 => get_texture_dependency;
    /// `CStreaming::AddImageToList(const char*, bool)` (0x407610)
    pub fn add_image_to_list(name: u32, cd: u32) -> i32 = cdecl 0x407610 => unported;
    /// `CClothes::LoadClothesFile` (0x5A7B30)
    pub fn load_clothes_file() = cdecl 0x5A7B30 => load_clothes_file_port;
    /// `CFileMgr::OpenFile(path, mode)` (0x538900)
    pub fn file_open(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => unported;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    pub fn file_close(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => unported;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    pub fn load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// CRT `strncmp` (0x8214D0)
    pub fn crt_strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    /// CRT `strtok` (0x82244B)
    pub fn crt_strtok(s: GPtr<u8>, delim: GPtr<u8>) -> GPtr<u8> = cdecl 0x82244B => unported;
    /// CRT `_stricmp` (0x8229B6)
    pub fn crt_stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `GetClothesModelFromName(const char*)` (0x5A7A20)
    pub fn clothes_model_from_name(name: GPtr<u8>) -> i32 = cdecl 0x5A7A20 => get_clothes_model_from_name;
    /// `CClothes::GetPlayerMotionGroupToLoad` (0x5A7FB0)
    pub fn get_player_motion_group_to_load_call() -> i32 = cdecl 0x5A7FB0 => get_player_motion_group_to_load;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    pub fn get_animation_block_index(name: u32) -> i32 = cdecl 0x4D3990 => unported;
    /// `CTimer::Suspend` (0x5619D0)
    pub fn timer_suspend() = cdecl 0x5619D0 => unported;
    /// `CTimer::Resume` (0x561A00)
    pub fn timer_resume() = cdecl 0x561A00 => unported;
    /// `CClothesBuilder::CreateSkinnedClump(RpClump*, RwTexDictionary*, CPedClothesDesc&, const CPedClothesDesc*, bool)` (0x5A69D0)
    pub fn create_skinned_clump(c: GPtr<u8>, txd: GPtr<u8>, d: GPtr<u8>, old: GPtr<u8>, cut: u32) -> GPtr<u8> = cdecl 0x5A69D0 => unported;
    /// `CClothes::RequestMotionGroupAnims` (0x5A8120)
    pub fn request_motion_group_anims_call() = cdecl 0x5A8120 => request_motion_group_anims;
    /// `CBaseModelInfo::AddTexDictionaryRef` (0x4C4B80)
    pub fn add_tex_dictionary_ref(mi: GPtr<u8>) = thiscall 0x4C4B80 => unported;
    /// `CBaseModelInfo::RemoveTexDictionaryRef` (0x4C4B90)
    pub fn remove_tex_dictionary_ref(mi: GPtr<u8>) = thiscall 0x4C4B90 => unported;
    /// `CBaseModelInfo::DeleteRwObject` (vtable slot 0x20)
    pub fn mi_delete_rw_object(mi: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// `CClumpModelInfo::SetClump(RpClump*)` (vtable slot 0x40)
    pub fn mi_set_clump(mi: GPtr<u8>, c: GPtr<u8>) = thiscall vslot 0x40 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    pub fn load_all_requested_models(b: u32) = cdecl 0x40EA10 => unported;
    /// `CClothes::ConstructPedModel(uint32, CPedClothesDesc&, const CPedClothesDesc*, bool)` (0x5A81E0)
    pub fn construct_ped_model_call(model: u32, d: GPtr<u8>, old: GPtr<u8>, cut: u32) = cdecl 0x5A81E0 => construct_ped_model;
    /// `CClothes::RebuildPlayer(CPlayerPed*, bool)` (0x5A82C0)
    pub fn rebuild_player_call(p: GPtr<u8>, ignore: u32) = cdecl 0x5A82C0 => rebuild_player;
    /// `RpAnimBlendClumpExtractAssociations(RpClump*)` (0x4D6BE0)
    pub fn clump_extract_associations(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D6BE0 => unported;
    /// `RpAnimBlendClumpGiveAssociations(RpClump*, CAnimBlendAssociation*)` (0x4D6C30)
    pub fn clump_give_associations(c: GPtr<u8>, a: GPtr<u8>) = cdecl 0x4D6C30 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810)
    pub fn get_task_secondary(tm: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTask::MakeAbortable(CPed*, eAbortPriority, const CEvent*)` (vtable slot 0x18)
    pub fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CEntity::DeleteRwObject` (vtable slot 0x20)
    pub fn entity_delete_rw_object(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CPed::Dress` (0x5E0130)
    pub fn ped_dress(p: GPtr<u8>) = thiscall 0x5E0130 => unported;
}

/// `CPedClothesDesc::Initialise` (0x5A78F0): zeroes every key and both stats.
#[hook(addr = 0x5A78F0, conv = thiscall, name = "CPedClothesDesc::Initialise", expect = "8B D1 57 33 C0", this = "CPedClothesDesc")]
pub fn desc_initialise(this: GPtr<u8>) {
    for i in 0..30 {
        wr::<u32>(this, 4 * i, 0);
    }
}

/// `CPedClothesDesc::CPedClothesDesc` (0x5A8020): as `Initialise`; returns `this`.
#[hook(addr = 0x5A8020, conv = thiscall, name = "CPedClothesDesc::Constructor", expect = "8B D1 57 33 C0", this = "CPedClothesDesc")]
pub fn desc_constructor(this: GPtr<u8>) -> GPtr<u8> {
    desc_initialise(this);
    this
}

/// `CPedClothesDesc::SetModel(uint32 key, eClothesModelPart)` (0x5A7910, `ret 8`)
#[hook(addr = 0x5A7910, conv = thiscall, name = "CPedClothesDesc::SetModel-modelHash", expect = "8B 44 24 08 8B 54 24 04", this = "CPedClothesDesc")]
pub fn set_model_hash(this: GPtr<u8>, key: u32, part: i32) {
    wr::<u32>(this, (part as isize).wrapping_mul(4), key);
}

/// `CPedClothesDesc::SetModel(const char*, eClothesModelPart)` (0x5A7920, `ret 8`): key of the upper-case
/// name, 0 for null.
#[hook(addr = 0x5A7920, conv = thiscall, name = "CPedClothesDesc::SetModel-modelName", expect = "8B 44 24 04 56", this = "CPedClothesDesc")]
pub fn set_model_name(this: GPtr<u8>, name: u32, part: i32) {
    let k = if name == 0 { 0 } else { get_uppercase_key(name) };
    wr::<u32>(this, (part as isize).wrapping_mul(4), k);
}

/// `CPedClothesDesc::HasVisibleTattoo` (0x5A79D0, AL): any of texture keys 4..12 (+0x38..+0x58) set.
#[hook(addr = 0x5A79D0, conv = thiscall, name = "CPedClothesDesc::HasVisibleTattoo", expect = "8B 41 38 85 C0", this = "CPedClothesDesc")]
pub fn has_visible_tattoo(this: GPtr<u8>) -> u8 {
    (0..9).any(|i| rd::<u32>(this, 0x38 + 4 * i) != 0) as u8
}

/// `GetClothesModelFromName(const char*)` (0x5A7A20, cdecl): the model part of "torso", "head", "hands",
/// "legs", "feet", "necklace", "watch", "glasses", "hat", "extra1" (9); an unknown name gives 0 (torso), as
/// the original's `setne`/`dec`/`and 9` computes.
#[hook(addr = 0x5A7A20, conv = cdecl, name = "GetClothesModelFromName", expect = "8B 44 24 04 56")]
pub fn get_clothes_model_from_name(name: GPtr<u8>) -> i32 {
    // the .rdata strings 0x8590CC, 0x8590E8, 0x867130, 0x859124, 0x867128, 0x8670A0, 0x867098, 0x867090,
    // 0x86708C, 0x85C3B0, each compared with its terminating NUL (`repe cmpsb`)
    const T: [&[u8]; 10] = [
        b"torso\0", b"head\0", b"hands\0", b"legs\0", b"feet\0", b"necklace\0", b"watch\0", b"glasses\0", b"hat\0",
        b"extra1\0",
    ];
    for (i, s) in T.iter().enumerate() {
        if s.iter().enumerate().all(|(k, c)| name.byte_add(k as isize).read() == *c) {
            return i as i32;
        }
    }
    0
}

/// `CClothes::GetTextureDependency(eClothesTexturePart)` (0x5A7EA0, cdecl): the model part a texture part
/// depends on (10 = none). Callers keep ECX/EDX across the call (`CClothes::Init`, `SetTextureAndModel`).
#[hook(addr = 0x5A7EA0, conv = cdecl, name = "CClothes::GetTextureDependency", preserve = "ecx,edx", expect = "8B 44 24 04 83 F8 11")]
pub fn get_texture_dependency(t: i32) -> i32 {
    match t {
        0 => 0,
        1 => 1,
        2 => 3,
        3 => 4,
        13 => 5,
        14 => 6,
        15 => 7,
        16 => 8,
        17 => 9,
        _ => 10,
    }
}

/// `CClothes::GetDependentTexture(eClothesModelPart)` (0x5A7F30, cdecl): the texture part a model part
/// depends on (0x12 = none).
#[hook(addr = 0x5A7F30, conv = cdecl, name = "CClothes::GetDependentTexture", expect = "8B 44 24 04 83 F8 09")]
pub fn get_dependent_texture(m: i32) -> i32 {
    match m {
        0 => 0,
        1 => 1,
        3 => 2,
        4 => 3,
        5 => 0xD,
        6 => 0xE,
        7 => 0xF,
        8 => 0x10,
        9 => 0x11,
        _ => 0x12,
    }
}

/// `CPedClothesDesc::HasVisibleNewHairCut(int32 arg)` (0x5A7970, `ret 4`, AL): false when wearing a
/// balaclava (key +0x24), the "head" model (+4) or any +0x20 key; then true for `arg != 1`, and for
/// `arg == 1` only with the "afro" head.
#[hook(addr = 0x5A7970, conv = thiscall, name = "CPedClothesDesc::HasVisibleNewHairCut", expect = "56 57 8B F1 8B 7E 24", this = "CPedClothesDesc")]
pub fn has_visible_new_hair_cut(this: GPtr<u8>, arg: i32) -> u8 {
    if rd::<u32>(this, 0x24) == get_uppercase_key(0x86718C) {
        return 0;
    }
    if rd::<u32>(this, 4) == get_uppercase_key(0x8590E8) {
        return 0;
    }
    if rd::<u32>(this, 0x20) != 0 {
        return 0;
    }
    if arg != 1 {
        return 1;
    }
    (rd::<u32>(this, 4) == get_uppercase_key(0x867198)) as u8
}

fn stat(i: u32) -> X87 {
    X87::from_f64_exact(get_stat_value(i))
}

/// `CClothes::GetPlayerMotionGroupToLoad` (0x5A7FB0): 0x37 (fat) when fat > 500 and fat > muscle
/// (muscle stored as f32, fat re-read); else 0x38 (muscular) when muscle > 500; else 0x36.
#[hook(addr = 0x5A7FB0, conv = cdecl, name = "CClothes::GetPlayerMotionGroupToLoad", expect = "51 6A 15 E8 88 0E FB FF")]
pub fn get_player_motion_group_to_load() -> i32 {
    if !stat(0x15).fcom(x(500.0)).test_any(0x41) {
        let muscle = stat(0x17).to_f32();
        if !stat(0x15).fcom(x(muscle)).test_any(0x41) {
            return 0x37;
        }
    }
    if !stat(0x17).fcom(x(500.0)).test_any(0x41) { 0x38 } else { 0x36 }
}

/// `CPedClothesDesc::SetTextureAndModel(uint32 texKey, uint32 modelKey, eClothesTexturePart)` (0x5A8050,
/// `ret 0xC`): texture key set; the dependent model part (if any) gets the model key.
#[hook(addr = 0x5A8050, conv = thiscall, name = "CPedClothesDesc::SetTextureAndModel-hash", expect = "8B 54 24 0C 56", this = "CPedClothesDesc")]
pub fn set_texture_and_model_hash(this: GPtr<u8>, tex: u32, model: u32, part: i32) {
    let dep = get_texture_dependency_call(part);
    wr::<u32>(this, 0x28 + (part as isize).wrapping_mul(4), tex);
    if dep != 10 {
        wr::<u32>(this, (dep as isize).wrapping_mul(4), model);
    }
}

/// `CPedClothesDesc::SetTextureAndModel(const char* tex, const char* model, eClothesTexturePart)`
/// (0x5A8080, `ret 0xC`): as the key version with upper-case keys (0 for null names).
#[hook(addr = 0x5A8080, conv = thiscall, name = "CPedClothesDesc::SetTextureAndModel-name", expect = "8B 44 24 04 53", this = "CPedClothesDesc")]
pub fn set_texture_and_model_name(this: GPtr<u8>, tex: u32, model: u32, part: i32) {
    let tk = if tex == 0 { 0 } else { get_uppercase_key(tex) };
    let mk = if model == 0 { 0 } else { get_uppercase_key(model) };
    let dep = get_texture_dependency_call(part);
    wr::<u32>(this, 0x28 + (part as isize).wrapping_mul(4), tk);
    if dep != 10 {
        wr::<u32>(this, (dep as isize).wrapping_mul(4), mk);
    }
}

/// `CClothes::Init` (0x5A80D0): marks every texture part and its dependent model part of `PlayerClothes`
/// as -1, registers "MODELS\PLAYER.IMG" (`CStreaming::AddImageToList(name, false)`), then tail-calls
/// `LoadClothesFile`.
#[hook(addr = 0x5A80D0, conv = cdecl, name = "CClothes::Init", expect = "33 C9 83 CA FF")]
pub fn init() {
    let pc = PLAYER_CLOTHES.ptr().cast::<u8>();
    for i in 0..0x12 {
        let dep = get_texture_dependency_call(i);
        wr::<i32>(pc, 0x28 + 4 * i as isize, -1);
        if dep != 10 {
            wr::<i32>(pc, 4 * dep as isize, -1);
        }
    }
    NUM_RULE_TAGS.set(0);
    CLOTHES_IMAGE_ID.set(add_image_to_list(0x86713C, 0));
    load_clothes_file();
}

/// `CClothes::ms_clothesRules` (0xBC1300, u32 tags and arguments, counted by `ms_numRuleTags`).
const CLOTHES_RULES: usize = 0xBC1300;

fn add_rule(v: u32) {
    let n = NUM_RULE_TAGS.get();
    g::<u32>(CLOTHES_RULES + 4 * n as usize).write(v);
    NUM_RULE_TAGS.set(n + 1);
}

/// `"-"` compared with its NUL (`repe cmpsb`, 2 bytes): `0` for a dash, else the token's key.
fn key_or_dash(t: GPtr<u8>) -> u32 {
    if t.read() == b'-' && t.byte_add(1).read() == 0 { 0 } else { get_uppercase_key(t.addr() as u32) }
}

/// `CClothes::LoadClothesFile` (0x5A7B30): `ms_numRuleTags` = 0, then "DATA\\CLOTHES.DAT" (0x8671F8,
/// mode 0x85A53C) line by line (`CFileLoader::LoadLine`, the file not checked): empty and `#` lines
/// skipped; outside a block only a line starting "rule" (`strncmp` 4) opens one; inside, "end"
/// (`strncmp` 3) closes it, else `strtok(line, " \\t,")` gives the tag, matched with `_stricmp` against
/// cuts 0, setc 1, tex 2, hide 3, ignore 5, endignore 4, exclusive 7, endexclusive 6 (no match keeps
/// the previous line's tag: the original's stack local is never reset). The tag is appended to
/// `ms_clothesRules`, two more tokens are always taken, and per tag: cuts / tex: two keys
/// (`CKeyGen::GetUppercaseKey`); setc: two more tokens, then key(1st), `GetClothesModelFromName(2nd)`,
/// key or 0 for "-" of the 3rd and 4th; hide: key(1st), model(2nd); 4..7: key(1st).
#[hook(addr = 0x5A7B30, conv = cdecl, name = "CClothes::LoadClothesFile", expect = "83 EC 14 55 56 57 33 ED",
       unverified = "std6-ped-1", route = "L2", hazards = "uninit-locals", recs = "new-game",
       note = "the tag local is uninitialised before the first matching line (CLOTHES.DAT always starts with a valid tag)")]
pub fn load_clothes_file_port() {
    let mut delim: [u8; 4] = [0x20, 0x09, 0x2C, 0];
    let dl = GPtr::from_raw(delim.as_mut_ptr());
    let mut in_rule = false;
    // the stack slot the original never initialises; its first-line value is the caller's residue
    let mut tag: u32 = 0;
    NUM_RULE_TAGS.set(0);
    let f = file_open(GPtr::from_addr(0x8671F8), GPtr::from_addr(0x85A53C));
    let mut line = load_line(f);
    while !line.is_null() {
        let c = line.read();
        if c == b'#' || c == 0 {
        } else if !in_rule {
            if crt_strncmp(GPtr::from_addr(0x8671F0), line, 4) == 0 {
                in_rule = true;
            }
        } else if crt_strncmp(GPtr::from_addr(0x8671EC), line, 3) == 0 {
            in_rule = false;
        } else {
            let t = crt_strtok(line, dl);
            if !t.is_null() {
                // 0x8671E4 "cuts", 0x8671DC "setc", 0x8671D8 "tex", 0x8671D0 "hide", 0x8671C8 "ignore",
                // 0x8671BC "endignore", 0x8671B0 "exclusive", 0x8671A0 "endexclusive"
                const NAMES: [(usize, u32); 8] = [
                    (0x8671E4, 0),
                    (0x8671DC, 1),
                    (0x8671D8, 2),
                    (0x8671D0, 3),
                    (0x8671C8, 5),
                    (0x8671BC, 4),
                    (0x8671B0, 7),
                    (0x8671A0, 6),
                ];
                for (a, v) in NAMES {
                    if crt_stricmp(t, GPtr::from_addr(a)) == 0 {
                        tag = v;
                        break;
                    }
                }
                add_rule(tag);
                let a1 = crt_strtok(GPtr::NULL, dl);
                let a2 = crt_strtok(GPtr::NULL, dl);
                match tag {
                    0 | 2 => {
                        add_rule(get_uppercase_key(a1.addr() as u32));
                        add_rule(get_uppercase_key(a2.addr() as u32));
                    }
                    1 => {
                        let a3 = crt_strtok(GPtr::NULL, dl);
                        let a4 = crt_strtok(GPtr::NULL, dl);
                        add_rule(get_uppercase_key(a1.addr() as u32));
                        add_rule(clothes_model_from_name(a2) as u32);
                        add_rule(key_or_dash(a3));
                        add_rule(key_or_dash(a4));
                    }
                    3 => {
                        add_rule(get_uppercase_key(a1.addr() as u32));
                        add_rule(clothes_model_from_name(a2) as u32);
                    }
                    4..=7 => add_rule(get_uppercase_key(a1.addr() as u32)),
                    _ => {}
                }
            }
        }
        line = load_line(f);
    }
    file_close(f);
}

/// `CClothes::RequestMotionGroupAnims` (0x5A8120): requests the "fat" or "muscular" anim block (model
/// index block + 0x63E7, flags 0x12) for the motion group to load and marks the other deletable (both
/// deletable for the normal group).
#[hook(addr = 0x5A8120, conv = cdecl, name = "CClothes::RequestMotionGroupAnims", expect = "53 56 57 E8 88 FE FF FF")]
pub fn request_motion_group_anims() {
    let g = get_player_motion_group_to_load_call();
    let fat = get_animation_block_index(0x85A508);
    let mus = get_animation_block_index(0x86720C);
    if g == 0x37 {
        request_model(fat.wrapping_add(0x63E7), 0x12);
        set_model_is_deletable(mus.wrapping_add(0x63E7));
    } else if g == 0x38 {
        request_model(mus.wrapping_add(0x63E7), 0x12);
        set_model_is_deletable(fat.wrapping_add(0x63E7));
    } else {
        set_model_is_deletable(mus.wrapping_add(0x63E7));
        set_model_is_deletable(fat.wrapping_add(0x63E7));
    }
}

/// `CClothes::ConstructPedModel(uint32 model, CPedClothesDesc&, const CPedClothesDesc* old, bool
/// cutscene)` (0x5A81E0, cdecl): with the timer suspended, builds the skinned clump from the model's clump
/// and its TXD (a free TXD slot reads address 0, as the original) and gives it to the model info; tail
/// call `CTimer::Resume`.
#[hook(addr = 0x5A81E0, conv = cdecl, name = "CClothes::ConstructPedModel", expect = "56 57 E8 E9 97 FB FF")]
pub fn construct_ped_model(model: u32, desc: GPtr<u8>, old: GPtr<u8>, cut: u32) {
    timer_suspend();
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * model as usize).read();
    let idx = i32::from(rd::<i16>(mi, 0xA));
    let pool = g::<GPtr<u8>>(TXD_POOL).read();
    let free = (rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize).read() as i8) < 0;
    let slot: GPtr<u8> = if free { GPtr::NULL } else { rd::<GPtr<u8>>(pool, 0).byte_add((idx as isize).wrapping_mul(12)) };
    let txd = rd::<GPtr<u8>>(slot, 0);
    let c = create_skinned_clump(rd::<GPtr<u8>>(mi, 0x1C), txd, desc, old, cut);
    if !c.is_null() {
        request_motion_group_anims_call();
        add_tex_dictionary_ref(mi);
        mi_delete_rw_object(mi);
        mi_set_clump(mi, c);
        remove_tex_dictionary_ref(mi);
        load_all_requested_models(1);
    }
    timer_resume();
}

/// `CClothes::RebuildCutscenePlayer(CPlayerPed*, int32 model)` (0x5A8270, cdecl): fat/muscle stats into the
/// player's clothes, then `ConstructPedModel(model, clothes, null, true)`.
#[hook(addr = 0x5A8270, conv = cdecl, name = "CClothes::RebuildCutscenePlayer", expect = "56 8B 74 24 08")]
pub fn rebuild_cutscene_player(p: GPtr<u8>, model: u32) {
    let d = rd::<GPtr<u8>>(p, 0x480);
    let f = stat(0x15).to_f32();
    wr::<f32>(rd::<GPtr<u8>>(d, 4), 0x70, f);
    let d = rd::<GPtr<u8>>(p, 0x480);
    let m = stat(0x17).to_f32();
    wr::<f32>(rd::<GPtr<u8>>(d, 4), 0x74, m);
    let c = rd::<GPtr<u8>>(rd::<GPtr<u8>>(p, 0x480), 4);
    construct_ped_model_call(model, c, GPtr::NULL, 1);
}

/// `CClothes::RebuildPlayer(CPlayerPed*, bool ignoreFatAndMuscle)` (0x5A82C0, cdecl): extracts the anim
/// associations, aborts the secondary task 5, deletes the RW object, removes the ped from the world,
/// updates fat/muscle unless ignored, rebuilds the model from the player's clothes against
/// `PlayerClothes`, dresses the ped, gives the associations back and copies the clothes into
/// `PlayerClothes`.
#[hook(addr = 0x5A82C0, conv = cdecl, name = "CClothes::RebuildPlayer", expect = "56 8B 74 24 08")]
pub fn rebuild_player(p: GPtr<u8>, ignore: u32) {
    let assoc = clump_extract_associations(rd::<GPtr<u8>>(p, 0x18));
    let t = get_task_secondary(rd::<GPtr<u8>>(p, 0x47C).byte_add(4), 5);
    if !t.is_null() {
        task_make_abortable(t, p, 2, 0);
    }
    entity_delete_rw_object(p);
    world_remove(p);
    if ignore as u8 == 0 {
        let d = rd::<GPtr<u8>>(p, 0x480);
        let f = stat(0x15).to_f32();
        wr::<f32>(rd::<GPtr<u8>>(d, 4), 0x70, f);
        let d = rd::<GPtr<u8>>(p, 0x480);
        let m = stat(0x17).to_f32();
        wr::<f32>(rd::<GPtr<u8>>(d, 4), 0x74, m);
    }
    let c = rd::<GPtr<u8>>(rd::<GPtr<u8>>(p, 0x480), 4);
    let model = i32::from(rd::<i16>(p, 0x22)) as u32;
    construct_ped_model_call(model, c, PLAYER_CLOTHES.ptr().cast(), 0);
    ped_dress(p);
    clump_give_associations(rd::<GPtr<u8>>(p, 0x18), assoc);
    let src = rd::<GPtr<u8>>(rd::<GPtr<u8>>(p, 0x480), 4);
    let pc = PLAYER_CLOTHES.ptr().cast::<u8>();
    for i in 0..30 {
        wr::<u32>(pc, 4 * i, rd::<u32>(src, 4 * i));
    }
}

/// `CClothes::RebuildPlayerIfNeeded(CPlayerPed*)` (0x5A8390, cdecl): `RebuildPlayer(p, false)` when the fat
/// or muscle stat differs from the clothes' stored value (NaN counts as different).
#[hook(addr = 0x5A8390, conv = cdecl, name = "CClothes::RebuildPlayerIfNeeded", expect = "56 8B 74 24 08")]
pub fn rebuild_player_if_needed(p: GPtr<u8>) {
    let fat = rd::<f32>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(p, 0x480), 4), 0x70);
    let differs = if stat(0x15).fcom(x(fat)).test_parity(0x44) {
        true
    } else {
        let m = rd::<f32>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(p, 0x480), 4), 0x74);
        stat(0x17).fcom(x(m)).test_parity(0x44)
    };
    // jp (fat differs) → rebuild; jnp after the muscle compare (equal) → skip
    if differs {
        rebuild_player_call(p, 0);
    }
}

// --- hook list (generated) ---

orig! {
    /// `NodeNamePlugin::GetFrameNodeName(RwFrame*)` (0x72FB30)
    fn get_frame_node_name(f: GPtr<u8>) -> u32 = cdecl 0x72FB30 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn stricmp(a: u32, b: u32) -> i32 = cdecl 0x8229B6 => unported;
}

/// The "balaclava" model name (`.rdata` 0x86718C).
const BALACLAVA_NAME: u32 = 0x86718C;

/// `CPedClothesDesc::GetIsWearingBalaclava()` (0x5A7950, thiscall; the name push sits in the fragment
/// 0x403887): model key 9 (+0x24) == `CKeyGen::GetUppercaseKey("balaclava")`. `sete al`: the upper
/// three bytes of EAX are the key's.
#[hook(addr = 0x5a7950, conv = thiscall, name = "CPedClothesDesc::GetIsWearingBalaclava", expect = "56 8B 71 24 E9 2E BF E5 FF",
       route = "L2", hazards = "entry-protection-read", recs = "grove-walk",
       note = "0x407E2C reads entry bytes +5 (keycells)")]
pub fn get_is_wearing_balaclava(this: GPtr<u8>) -> u32 {
    let mine = rd::<u32>(this, 0x24);
    let key = get_uppercase_key(BALACLAVA_NAME);
    (key & 0xFFFF_FF00) | u32::from(mine == key)
}

/// `CClothes::GetDefaultPlayerMotionGroup()` (0x5A81B0, cdecl; the `ms_aAnimAssocGroups` load sits in
/// the fragment 0x406B49): `GetPlayerMotionGroupToLoad()`, unless it is 0x36 or the group's anim block
/// (`ms_aAnimAssocGroups` 0xB4EA34, 0x14-byte groups, block +0) is null or not loaded (block +0x10),
/// else 0x36.
#[hook(addr = 0x5a81b0, conv = cdecl, name = "CClothes::GetDefaultPlayerMotionGroup", expect = "E8 FB FD FF FF",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_default_player_motion_group() -> i32 {
    let m = get_player_motion_group_to_load_call();
    if m == 0x36 {
        return 0x36;
    }
    let groups = g::<GPtr<u8>>(0xB4EA34).read();
    let block = rd::<GPtr<u8>>(groups, (m as isize).wrapping_mul(0x14));
    if block.is_null() || rd::<u8>(block, 0x10) == 0 {
        return 0x36;
    }
    m
}

/// `FUN_005a47e0` (0x5A47E0, cdecl; unnamed in the catalogue): the `RpClumpForAllAtomics` callback of
/// `CClothesBuilder` that finds an atomic by its frame's node name: `data` = { +0 name, +4 found atomic };
/// when `_stricmp(name, GetFrameNodeName(atomic->frame +4))` is 0 the atomic is stored at +4 and null is
/// returned (stop), else the atomic (continue).
#[hook(addr = 0x5a47e0, conv = cdecl, name = "FUN_005a47e0", expect = "56 8B 74 24 08",
       unverified = "mvpi-ped-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_atomic_with_name_cb(atomic: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    let name = get_frame_node_name(rd::<GPtr<u8>>(atomic, 4));
    if stricmp(rd::<u32>(data, 0), name) == 0 {
        wr::<GPtr<u8>>(data, 4, atomic);
        return GPtr::NULL;
    }
    atomic
}

pub const HOOKS: &[HookReg] = &[
    HOOK_LOAD_CLOTHES_FILE_PORT,
    HOOK_GET_IS_WEARING_BALACLAVA,
    HOOK_GET_DEFAULT_PLAYER_MOTION_GROUP,
    HOOK_GET_ATOMIC_WITH_NAME_CB,
    HOOK_DESC_INITIALISE,
    HOOK_DESC_CONSTRUCTOR,
    HOOK_SET_MODEL_HASH,
    HOOK_SET_MODEL_NAME,
    HOOK_HAS_VISIBLE_TATTOO,
    HOOK_GET_CLOTHES_MODEL_FROM_NAME,
    HOOK_GET_TEXTURE_DEPENDENCY,
    HOOK_GET_DEPENDENT_TEXTURE,
    HOOK_HAS_VISIBLE_NEW_HAIR_CUT,
    HOOK_GET_PLAYER_MOTION_GROUP_TO_LOAD,
    HOOK_SET_TEXTURE_AND_MODEL_HASH,
    HOOK_SET_TEXTURE_AND_MODEL_NAME,
    HOOK_INIT,
    HOOK_REQUEST_MOTION_GROUP_ANIMS,
    HOOK_CONSTRUCT_PED_MODEL,
    HOOK_REBUILD_CUTSCENE_PLAYER,
    HOOK_REBUILD_PLAYER,
    HOOK_REBUILD_PLAYER_IF_NEEDED,
];
