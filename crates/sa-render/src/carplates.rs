//! PORT-RENDER-2: `CCustomCarPlateMgr`. Ported from the 1.0 US bytes; gta-reversed
//! `CustomCarPlateMgr.cpp` as a reading aid.
//!
//! The plate texture is a 64x16x32 raster with eight 8x16 characters copied from the locked
//! "platecharset" raster (4 characters per row, 32 bytes each, 16 lines per character row).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, k, rd, wr, xi};

sa_core::globals! {
    list GLOBALS;
    /// `CCustomCarPlateMgr::pCharSetTex`
    pub static CHARSET_TEX @ 0xC3EF5C: u32 = 0;
    /// `CCustomCarPlateMgr::pPlatebackTexTab[3]`
    pub static PLATEBACK_TEX @ 0xC3EF60: [u32; 3] = [0; 3];
    /// The per-design table `CreatePlateTexture` passes on as `RenderLicenseplateTextToRaster`'s third
    /// argument (`pPalette1555Tab`-like; unused there)
    pub static DESIGN_TAB @ 0xC3EF6C: [u32; 3] = [0; 3];
    /// `CCustomCarPlateMgr::pCharSetLockedData`
    pub static CHARSET_LOCKED @ 0xC3EF78: u32 = 0;
    /// `CCustomCarPlateMgr::pPlateMaterial` (the material `SetupClump` finds)
    pub static PLATE_MATERIAL @ 0xC3EF7C: u32 = 0;
    /// `CCustomCarPlateMgr::CurrentPlateType` (the design `SetupClump` passes to the callbacks)
    pub static CURRENT_PLATE_TYPE @ 0xC3EF80: u8 = 0;
}

/// Strings of the exe: "vehicle" (0x86982C), "platecharset" (0x872560), "plateback1".."3" (0x872554,
/// 0x872548, 0x87253C), "carplate" (0x872584), "carpback" (0x872578).
const S_VEHICLE: u32 = 0x86982C;
const S_PLATECHARSET: u32 = 0x872560;
const S_PLATEBACK: [u32; 3] = [0x872554, 0x872548, 0x87253C];
const S_CARPLATE: u32 = 0x872584;
const S_CARPBACK: u32 = 0x872578;
/// "DATA" (0x86A784), "r" (0x85A53C), "" (0x858B54), ";the end" (0x86A958, 9 bytes with the NUL).
const S_DATA: u32 = 0x86A784;
const S_MODE_R: u32 = 0x85A53C;
const S_EMPTY: u32 = 0x858B54;
const S_THE_END: u32 = 0x86A958;
/// The callbacks the original passes by address.
const MATERIAL_SET_CB: u32 = 0x6FE060;
const ATOMIC_SET_CB: u32 = 0x6FE0D0;
const MATERIAL_UPGRADE_CB: u32 = 0x6FDF50;
const ATOMIC_UPGRADE_CB: u32 = 0x6FDFC0;

/// 1/32768 (0x858B14), -23.0 (0x872574), -9.0 (0x872570)
const K_INV_32768: u32 = 0x3800_0000;
const K_M23: u32 = 0xC1B8_0000;
const K_M9: u32 = 0xC110_0000;

orig! {
    /// `CCustomCarPlateMgr::GetCharacterPositionInCharSet(char c (AL), uint32& col, uint32& row)` (0x6FD7C0)
    fn char_position(c: u8, col: GPtr<u32>, row: GPtr<u32>) = reg("args = al stack stack") 0x6FD7C0 => get_character_position_in_char_set;
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    pub fn file_mgr_set_dir(d: u32) = cdecl 0x5387D0 => unported;
    /// `CFileMgr::OpenFile(const char* name, const char* mode)` (0x538900)
    pub fn file_mgr_open_file(name: u32, mode: u32) -> u32 = cdecl 0x538900 => unported;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    pub fn file_mgr_close_file(f: u32) -> i32 = cdecl 0x5389D0 => unported;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    pub fn load_line(f: u32) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// CRT `strtok` (0x82244B)
    pub fn strtok(s: GPtr<u8>, delim: GPtr<u8>) -> GPtr<u8> = cdecl 0x82244B => unported;
    /// CRT `atol` (0x82258E)
    pub fn atol(s: GPtr<u8>) -> i32 = cdecl 0x82258E => unported;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn find_txd_slot(name: u32) -> i32 = cdecl 0x731850 => unported;
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    pub fn push_current_txd() = cdecl 0x7316A0 => unported;
    /// `CTxdStore::SetCurrentTxd(int32)` (0x7319C0)
    pub fn set_current_txd(slot: i32) = cdecl 0x7319C0 => unported;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    pub fn pop_current_txd() = cdecl 0x7316B0 => unported;
    /// `CTxdStore::RemoveTxd(int32)` (0x731E90)
    pub fn remove_txd(slot: i32) = cdecl 0x731E90 => unported;
    /// `RwTextureRead(const char* name, const char* mask)` (0x7F3AC0)
    pub fn rw_texture_read(name: u32, mask: u32) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    pub fn rw_texture_destroy(t: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// `RwTextureCreate(RwRaster*)` (0x7F37C0)
    pub fn rw_texture_create(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F37C0 => unported;
    /// `RwTextureSetName(RwTexture*, const char*)` (0x7F38A0)
    pub fn rw_texture_set_name(t: GPtr<u8>, name: u32) -> GPtr<u8> = cdecl 0x7F38A0 => unported;
    /// `RwRasterCreate(int32 w, int32 h, int32 depth, int32 flags)` (0x7FB230)
    pub fn rw_raster_create(w: i32, h: i32, depth: i32, flags: i32) -> GPtr<u8> = cdecl 0x7FB230 => unported;
    /// `RwRasterDestroy(RwRaster*)` (0x7FB020)
    pub fn rw_raster_destroy(r: GPtr<u8>) -> i32 = cdecl 0x7FB020 => unported;
    /// `RwRasterLock(RwRaster*, uint8 level, int32 mode)` (0x7FB2D0)
    pub fn rw_raster_lock(r: GPtr<u8>, level: u32, mode: i32) -> GPtr<u8> = cdecl 0x7FB2D0 => unported;
    /// `RwRasterUnlock(RwRaster*)` (0x7FAEC0)
    pub fn rw_raster_unlock(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7FAEC0 => unported;
    /// `RpMaterialSetTexture(RpMaterial*, RwTexture*)` (0x74DBC0)
    pub fn rp_material_set_texture(m: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> = cdecl 0x74DBC0 => unported;
    /// CRT `_stricmp` (0x8229B6)
    pub fn stricmp(a: u32, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `RpGeometryReplaceOldMaterialWithNewMaterial(RpGeometry*, RpMaterial* old, RpMaterial* new)` (0x734DE0)
    pub fn rp_geometry_replace_material(g: GPtr<u8>, old: GPtr<u8>, new: u32) = cdecl 0x734DE0 => unported;
    /// `RpGeometryForAllMaterials(RpGeometry*, callback, void* data)` (0x74C790)
    pub fn rp_geometry_for_all_materials(g: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x74C790 => unported;
    /// `RpClumpForAllAtomics(RpClump*, callback, void* data)` (0x749B70)
    pub fn rp_clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `CCustomCarPlateMgr::RenderLicenseplateTextToRaster(text, charset, palette, raster)` (0x6FDD70)
    pub fn render_text_to_raster_call(text: GPtr<u8>, charset: GPtr<u8>, pal: u32, raster: GPtr<u8>) -> u8 = cdecl 0x6FDD70 => unported;
    /// `CCustomCarPlateMgr::CreatePlateTexture(const char* text, uint8 design)` (0x6FDEA0)
    pub fn create_plate_texture_call(text: GPtr<u8>, design: u32) -> GPtr<u8> = cdecl 0x6FDEA0 => unported;
    /// `CCustomCarPlateMgr::SetupMaterialPlatebackTexture(RpMaterial*, uint8 design)` (0x6FDE50)
    pub fn setup_material_plateback_texture_call(m: GPtr<u8>, design: u32) -> GPtr<u8> = cdecl 0x6FDE50 => unported;
    /// `CCustomCarPlateMgr::SetupMaterialPlateTexture(RpMaterial*, const char*, uint8 design)` (0x6FE020)
    pub fn setup_material_plate_texture_call(m: GPtr<u8>, text: GPtr<u8>, design: u32) -> GPtr<u8> = cdecl 0x6FE020 => unported;
}

/// The design of the current weather region, as inlined by the original: region 1 → 2, regions 3..4 → 1,
/// otherwise 0.
fn region_design() -> u8 {
    match i32::from(crate::weather::WEATHER_REGION.get()) {
        1 => 2,
        3 | 4 => 1,
        _ => 0,
    }
}

/// `CCustomCarPlateMgr::Initialise()` (0x6FD500, AL): from the "vehicle" TXD, the "platecharset"
/// texture (filter 1, addressing 0x33) and "plateback1".."3" (filter 2, addressing 0x33); the charset
/// raster locked (level 0, read mode 2); returns `locked != null`.
#[hook(addr = 0x6FD500, conv = cdecl, name = "CCustomCarPlateMgr::Initialise", expect = "53 56 68 2C 98 86 00")]
pub fn initialise() -> u8 {
    let slot = find_txd_slot(S_VEHICLE);
    push_current_txd();
    set_current_txd(slot);
    let t = rw_texture_read(S_PLATECHARSET, 0);
    wr::<u8>(t, 0x50, 1);
    wr::<u8>(t, 0x51, 0x33);
    CHARSET_TEX.set(t.addr() as u32);
    for (i, s) in S_PLATEBACK.into_iter().enumerate() {
        let t = rw_texture_read(s, 0);
        wr::<u8>(t, 0x50, 2);
        wr::<u8>(t, 0x51, 0x33);
        g::<u32>(0xC3EF60 + i * 4).write(t.addr() as u32);
    }
    pop_current_txd();
    let raster = rd::<GPtr<u8>>(GPtr::from_addr(CHARSET_TEX.get() as usize), 0);
    let p = rw_raster_lock(raster, 0, 2);
    CHARSET_LOCKED.set(p.addr() as u32);
    u8::from(!p.is_null())
}

/// `CCustomCarPlateMgr::Shutdown()` (0x6FD720): the charset raster unlocked and its texture destroyed
/// (when set), the plateback textures destroyed, the "vehicle" TXD removed when found.
#[hook(addr = 0x6FD720, conv = cdecl, name = "CCustomCarPlateMgr::Shutdown", expect = "A1 5C EF C3 00")]
pub fn shutdown() {
    if CHARSET_TEX.get() != 0 {
        rw_raster_unlock(rd::<GPtr<u8>>(GPtr::from_addr(CHARSET_TEX.get() as usize), 0));
        CHARSET_LOCKED.set(0);
        rw_texture_destroy(GPtr::from_addr(CHARSET_TEX.get() as usize));
        CHARSET_TEX.set(0);
    }
    for i in 0..3 {
        let e = g::<u32>(0xC3EF60 + i * 4);
        let t = e.read();
        if t != 0 {
            rw_texture_destroy(GPtr::from_addr(t as usize));
            e.write(0);
        }
    }
    let slot = find_txd_slot(S_VEHICLE);
    if slot != -1 {
        remove_txd(slot);
    }
}

/// `CCustomCarPlateMgr::GetMapRegionPlateDesign()` (0x6FD7A0, AL).
#[hook(addr = 0x6FD7A0, conv = cdecl, name = "CCustomCarPlateMgr::GetMapRegionPlateDesign", expect = "0F BF 05 14 13 C8 00")]
pub fn get_map_region_plate_design() -> u8 {
    region_design()
}

/// One `rand()` at its original call site (caller attribution `ret`) scaled as the original does:
/// `_ftol((rand() & 0xFFFF) * 1/32768 * m)`.
fn rnd(ret: u32, m: u32) -> u8 {
    let r = sa_core::orig::at_site(ret, sa_core::crt::rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(k(K_INV_32768)).fmul_m(k(m))) as u8
}

/// `CCustomCarPlateMgr::GeneratePlateText(char* buf, int32 len)` (0x6FD5B0, AL): false for `len < 4`;
/// else two letters `'A' + 0..22`, two digits `'0' + 0..8`, then while the index is below `len`: a
/// space, a digit and two letters (four bytes per pass, so it can write past `len`). Seven `rand()` call
/// sites. The original uses its `len` argument slot as scratch (stack only).
#[hook(addr = 0x6FD5B0, conv = cdecl, name = "CCustomCarPlateMgr::GeneratePlateText", expect = "53 8B 5C 24 0C")]
pub fn generate_plate_text(buf: GPtr<u8>, len: i32) -> u8 {
    if len < 4 {
        return 0;
    }
    let put = |i: i32, v: u8| buf.wrapping_index(i).write(v);
    put(0, b'A'.wrapping_sub(rnd(0x6FD5C5, K_M23)));
    put(1, b'A'.wrapping_sub(rnd(0x6FD5F2, K_M23)));
    put(2, b'0'.wrapping_sub(rnd(0x6FD61C, K_M9)));
    put(3, b'0'.wrapping_sub(rnd(0x6FD646, K_M9)));
    let mut i = 4;
    if len > i {
        loop {
            put(i, b' ');
            i += 1;
            put(i, b'0'.wrapping_sub(rnd(0x6FD68A, K_M9)));
            i += 1;
            put(i, b'A'.wrapping_sub(rnd(0x6FD6B5, K_M23)));
            i += 1;
            put(i, b'A'.wrapping_sub(rnd(0x6FD6E0, K_M23)));
            i += 1;
            if i >= len {
                break;
            }
        }
    }
    1
}

/// `CCustomCarPlateMgr::GetCharacterPositionInCharSet(char c (AL), uint32& col, uint32& row)`
/// (0x6FD7C0): letters (either case) and digits are numbered A..Z = 0..25, 0..9 = 26..35, at column
/// `n % 4`, row `n / 4`; everything else (0, 9). The character is passed in AL (HARNESS-5 register
/// convention); `RenderLicenseplateTextToRaster` calls it through `orig!`.
#[hook(addr = 0x6FD7C0, conv = reg("args = al stack stack"), name = "CCustomCarPlateMgr::GetCharacterPositionInCharSet",
       expect = "0F B6 C0 83 C0 E0",
       route = "L1-pure", hazards = "none", recs = "driving",
       note = "register convention AL + 2 stack words (HARNESS-5); EAX/ECX after return are not read by the caller")]
pub fn get_character_position_in_char_set(c: u8, col: GPtr<u32>, row: GPtr<u32>) {
    let n = match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a'),
        b'0'..=b'9' => Some(c - b'0' + 26),
        _ => None,
    };
    let (cl, rw) = match n {
        Some(n) => (u32::from(n % 4), u32::from(n / 4)),
        None => (0, 9),
    };
    col.write(cl);
    row.write(rw);
}

/// `CCustomCarPlateMgr::RenderLicenseplateTextToRaster(const char* text, RwRaster* charset, void*
/// palette, RwRaster* out)` (0x6FDD70, AL): `out` locked (level 0, mode 5); false (and left locked)
/// when that, the locked charset data or either raster's stride (+0x18) is 0; else the eight characters'
/// 32x16-byte cells copied from the charset data, `out` unlocked, true. `palette` is unused.
#[hook(addr = 0x6FDD70, conv = cdecl, name = "CCustomCarPlateMgr::RenderLicenseplateTextToRaster", expect = "83 EC 10 56 8B 74 24 24")]
pub fn render_licenseplate_text_to_raster(text: GPtr<u8>, charset: GPtr<u8>, _palette: u32, out: GPtr<u8>) -> u8 {
    let dst0 = rw_raster_lock(out, 0, 5);
    if dst0.is_null() || CHARSET_LOCKED.get() == 0 {
        return 0;
    }
    let cs = rd::<i32>(charset, 0x18);
    if cs == 0 {
        return 0;
    }
    let os = rd::<i32>(out, 0x18);
    if os == 0 {
        return 0;
    }
    for i in 0..8i32 {
        let (mut col, mut row) = (1u32, 1u32);
        char_position(
            text.wrapping_index(i).read(),
            GPtr::from_raw(&mut col),
            GPtr::from_raw(&mut row),
        );
        let off = (row.wrapping_shl(6).wrapping_add(col)).wrapping_shl(5);
        let mut src = CHARSET_LOCKED.get().wrapping_add(off);
        let mut dst = (dst0.addr() as u32).wrapping_add((i as u32) * 0x20);
        for _ in 0..16 {
            for b in 0..0x20u32 {
                g::<u8>(dst.wrapping_add(b) as usize).write(g::<u8>(src.wrapping_add(b) as usize).read());
            }
            src = src.wrapping_add(cs as u32);
            dst = dst.wrapping_add(os as u32);
        }
    }
    rw_raster_unlock(out);
    1
}

/// `CCustomCarPlateMgr::SetupMaterialPlatebackTexture(RpMaterial*, uint8 design)` (0x6FDE50): design
/// 0xFF = the region's; the material gets plateback texture `design`; returns the material.
#[hook(addr = 0x6FDE50, conv = cdecl, name = "CCustomCarPlateMgr::SetupMaterialPlatebackTexture", expect = "8A 44 24 08 3C FF")]
pub fn setup_material_plateback_texture(m: GPtr<u8>, design: u32) -> GPtr<u8> {
    let mut d = design as u8;
    if d == 0xFF {
        d = region_design();
    }
    rp_material_set_texture(m, GPtr::from_addr(g::<u32>(0xC3EF60 + usize::from(d) * 4).read() as usize));
    m
}

/// `CCustomCarPlateMgr::CreatePlateTexture(const char* text, uint8 design)` (0x6FDEA0): design 0xFF =
/// the region's; a 64x16x32 raster (flags 0x604) rendered from `text`, wrapped in a texture named
/// `text` with filter 1. Null (raster destroyed) when the raster or the charset raster is missing or
/// the render fails; null without destroying the raster when the texture cannot be created.
#[hook(addr = 0x6FDEA0, conv = cdecl, name = "CCustomCarPlateMgr::CreatePlateTexture", expect = "53 8A 5C 24 0C")]
pub fn create_plate_texture(text: GPtr<u8>, design: u32) -> GPtr<u8> {
    let mut d = design as u8;
    if d == 0xFF {
        d = region_design();
    }
    let r = rw_raster_create(0x40, 0x10, 0x20, 0x604);
    if r.is_null() {
        return GPtr::NULL;
    }
    let cs = rd::<GPtr<u8>>(GPtr::from_addr(CHARSET_TEX.get() as usize), 0);
    if cs.is_null() {
        rw_raster_destroy(r);
        return GPtr::NULL;
    }
    let pal = g::<u32>(0xC3EF6C + usize::from(d) * 4).read();
    if render_text_to_raster_call(text, cs, pal, r) == 0 {
        rw_raster_destroy(r);
        return GPtr::NULL;
    }
    let t = rw_texture_create(r);
    if t.is_null() {
        return GPtr::NULL;
    }
    rw_texture_set_name(t, text.addr() as u32);
    wr::<u8>(t, 0x50, 1);
    t
}

/// `CCustomCarPlateMgr::SetupMaterialPlateTexture(RpMaterial*, const char* text, uint8 design)`
/// (0x6FE020): null material → null; a new plate texture (none → null) set on the material and released
/// (`RwTextureDestroy` drops this reference); returns the material.
#[hook(addr = 0x6FE020, conv = cdecl, name = "CCustomCarPlateMgr::SetupMaterialPlateTexture", expect = "57 8B 7C 24 08")]
pub fn setup_material_plate_texture(m: GPtr<u8>, text: GPtr<u8>, design: u32) -> GPtr<u8> {
    if m.is_null() {
        return GPtr::NULL;
    }
    let t = create_plate_texture_call(text, design);
    if t.is_null() {
        return GPtr::NULL;
    }
    rp_material_set_texture(m, t);
    rw_texture_destroy(t);
    m
}

/// The material's texture name (`texture + 0x10`) when the material has a texture and that address is
/// not null (the original tests both).
fn tex_name(m: GPtr<u8>) -> Option<GPtr<u8>> {
    let t = rd::<GPtr<u8>>(m, 0);
    if t.is_null() {
        return None;
    }
    let n = t.byte_add(0x10);
    if n.is_null() { None } else { Some(n) }
}

/// `CCustomCarPlateMgr::MaterialSetCarplateTextureCB(RpMaterial*, void* text)` (0x6FE060): a "carplate"
/// material (`_stricmp`) becomes `pPlateMaterial` and gets the plate texture; a "carpback" material the
/// plateback texture (current design). Returns the material.
#[hook(addr = 0x6FE060, conv = cdecl, name = "CCustomCarPlateMgr::MaterialSetCarplateTextureCB", expect = "56 8B 74 24 08")]
pub fn material_set_carplate_texture_cb(m: GPtr<u8>, text: GPtr<u8>) -> GPtr<u8> {
    if let Some(n) = tex_name(m) {
        if stricmp(S_CARPLATE, n) == 0 {
            PLATE_MATERIAL.set(m.addr() as u32);
            setup_material_plate_texture_call(m, text, u32::from(CURRENT_PLATE_TYPE.get()));
        } else if stricmp(S_CARPBACK, n) == 0 {
            setup_material_plateback_texture_call(m, u32::from(CURRENT_PLATE_TYPE.get()));
        }
    }
    m
}

/// `CCustomCarPlateMgr::AtomicSetCarplateTextureCB(RpAtomic*, void* text)` (0x6FE0D0): every material
/// of the atomic's geometry through `MaterialSetCarplateTextureCB`; returns the atomic.
#[hook(addr = 0x6FE0D0, conv = cdecl, name = "CCustomCarPlateMgr::AtomicSetCarplateTextureCB", expect = "8B 44 24 08 56")]
pub fn atomic_set_carplate_texture_cb(a: GPtr<u8>, text: u32) -> GPtr<u8> {
    rp_geometry_for_all_materials(rd::<GPtr<u8>>(a, 0x18), MATERIAL_SET_CB, text);
    a
}

/// `CCustomCarPlateMgr::SetupClump(RpClump*, const char* text, uint8 design)` (0x6FE0F0): design
/// stored, `pPlateMaterial = 0`, every atomic through `AtomicSetCarplateTextureCB`; returns
/// `pPlateMaterial`.
#[hook(addr = 0x6FE0F0, conv = cdecl, name = "CCustomCarPlateMgr::SetupClump", expect = "8B 4C 24 08 8B 54 24 04")]
pub fn setup_clump(c: GPtr<u8>, text: u32, design: u32) -> u32 {
    CURRENT_PLATE_TYPE.set(design as u8);
    PLATE_MATERIAL.set(0);
    rp_clump_for_all_atomics(c, ATOMIC_SET_CB, text);
    PLATE_MATERIAL.get()
}

/// `CCustomCarPlateMgr::MaterialUpgradeSetCarplateTextureCB(RpMaterial*, void* geometry)` (0x6FDF50): a
/// "carplate" material is replaced in the geometry by `pPlateMaterial`; a "carpback" material gets the
/// plateback texture. Returns the material.
#[hook(addr = 0x6FDF50, conv = cdecl, name = "CCustomCarPlateMgr::MaterialUpgradeSetCarplateTextureCB", expect = "56 57 8B 7C 24 0C")]
pub fn material_upgrade_set_carplate_texture_cb(m: GPtr<u8>, geom: GPtr<u8>) -> GPtr<u8> {
    if let Some(n) = tex_name(m) {
        if stricmp(S_CARPLATE, n) == 0 {
            rp_geometry_replace_material(geom, m, PLATE_MATERIAL.get());
        } else if stricmp(S_CARPBACK, n) == 0 {
            setup_material_plateback_texture_call(m, u32::from(CURRENT_PLATE_TYPE.get()));
        }
    }
    m
}

/// `CCustomCarPlateMgr::AtomicUpgradeSetCarplateTextureCB(RpAtomic*, void*)` (0x6FDFC0): every material
/// of the geometry through `MaterialUpgradeSetCarplateTextureCB` with the geometry as data; returns the
/// atomic.
#[hook(addr = 0x6FDFC0, conv = cdecl, name = "CCustomCarPlateMgr::AtomicUpgradeSetCarplateTextureCB", expect = "56 8B 74 24 08")]
pub fn atomic_upgrade_set_carplate_texture_cb(a: GPtr<u8>, _data: u32) -> GPtr<u8> {
    let geom = rd::<GPtr<u8>>(a, 0x18);
    rp_geometry_for_all_materials(geom, MATERIAL_UPGRADE_CB, geom.addr() as u32);
    a
}

/// `CCustomCarPlateMgr::SetupClumpAfterVehicleUpgrade(RpClump*, RpMaterial* plate, uint8 design)`
/// (0x6FDFE0, AL): false without a plate material; else design and plate material stored, every atomic
/// through `AtomicUpgradeSetCarplateTextureCB`, true.
#[hook(addr = 0x6FDFE0, conv = cdecl, name = "CCustomCarPlateMgr::SetupClumpAfterVehicleUpgrade", expect = "8B 44 24 08 85 C0")]
pub fn setup_clump_after_vehicle_upgrade(c: GPtr<u8>, plate: u32, design: u32) -> u8 {
    if plate == 0 {
        return 0;
    }
    CURRENT_PLATE_TYPE.set(design as u8);
    PLATE_MATERIAL.set(plate);
    rp_clump_for_all_atomics(c, ATOMIC_UPGRADE_CB, 0);
    1
}

/// `CCustomCarPlateMgr::LoadPlatecharsetDat(const char* file, uint32* out)` (0x6FDC00, AL): opens `file`
/// in "DATA" (mode "r"; directory reset to ""), reads lines until EOF or a line starting with the 9
/// bytes ";the end\0" (`repz cmpsb`); lines starting with ';' are skipped; others are split with
/// `strtok(" \t")` (the first token is not checked for null) and the first three tokens' `atol` values
/// (16-bit compare, > 0xFF → 0xFF) become r, g, b; fewer than three tokens return false at once (the
/// file left open); otherwise `*out++ = 0x80000000 | b << 16 | g << 8 | r`. Returns true after closing
/// the file. The tokens persist across lines in locals (the r byte starts uninitialised in the original;
/// it is always set before it is stored). The original builds the delimiters in its `out` argument
/// slot (stack only).
#[hook(addr = 0x6FDC00, conv = cdecl, name = "CCustomCarPlateMgr::LoadPlatecharsetDat", expect = "83 EC 08 53 55")]
pub fn load_platecharset_dat(file: u32, out: GPtr<u32>) -> u8 {
    file_mgr_set_dir(S_DATA);
    let f = file_mgr_open_file(file, S_MODE_R);
    file_mgr_set_dir(S_EMPTY);
    let mut out = out;
    let mut rgb = [0u8; 3];
    let mut delims = [b' ', b'\t', 0u8, 0u8];
    let dp = GPtr::<u8>::from_raw(delims.as_mut_ptr());
    let mut line = load_line(f);
    while !line.is_null() {
        let end = (0..9).all(|i| line.wrapping_index(i).read() == g::<u8>(S_THE_END as usize + i as usize).read());
        if end {
            break;
        }
        if line.read() != b';' {
            let mut tok = strtok(line, dp);
            let mut n: u32 = 0;
            loop {
                if n < 3 {
                    let v = atol(tok) as u16;
                    rgb[n as usize] = if v > 0xFF { 0xFF } else { v as u8 };
                }
                tok = strtok(GPtr::NULL, dp);
                n += 1;
                if tok.is_null() {
                    break;
                }
            }
            if n < 3 {
                return 0;
            }
            out.write(0x8000_0000 | u32::from(rgb[2]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[0]));
            out = out.wrapping_index(1);
        }
        line = load_line(f);
    }
    file_mgr_close_file(f);
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INITIALISE,
    HOOK_SHUTDOWN,
    HOOK_GET_MAP_REGION_PLATE_DESIGN,
    HOOK_GENERATE_PLATE_TEXT,
    HOOK_RENDER_LICENSEPLATE_TEXT_TO_RASTER,
    HOOK_GET_CHARACTER_POSITION_IN_CHAR_SET,
    HOOK_SETUP_MATERIAL_PLATEBACK_TEXTURE,
    HOOK_CREATE_PLATE_TEXTURE,
    HOOK_SETUP_MATERIAL_PLATE_TEXTURE,
    HOOK_MATERIAL_SET_CARPLATE_TEXTURE_CB,
    HOOK_ATOMIC_SET_CARPLATE_TEXTURE_CB,
    HOOK_SETUP_CLUMP,
    HOOK_MATERIAL_UPGRADE_SET_CARPLATE_TEXTURE_CB,
    HOOK_ATOMIC_UPGRADE_SET_CARPLATE_TEXTURE_CB,
    HOOK_SETUP_CLUMP_AFTER_VEHICLE_UPGRADE,
    HOOK_LOAD_PLATECHARSET_DAT,
];
