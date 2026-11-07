//! `SurfaceInfos_c` (global `g_surfaceInfos` 0xB79538, size 0x8F4): the surface property table.
//!
//! Ported from the 1.0 US bytes (capstone over the exe; `docs/re/specs/COLL/SurfaceInfos_c.md` and
//! `SurfaceInfos_c.deviations.md`); gta-reversed `SurfaceInfos_c.cpp` is a reading aid only.
//! Layout: `float m_adhesiveLimits[6][6]` at +0x00 (`[i][j]` at `i*0x18 + j*4`), then the
//! `SurfaceInfo_c` entries at +0x90 (stride 0xC): `int8 tyreGrip` +0, `int8 wetGrip` +1, `uint32
//! flags1` +4, `uint32 flags2` +8. The getters index with the surface id as the original does
//! (`this + id*12 + off`, 32-bit wrapping, no range check) and return the masked field in EAX.
//!
//! Kept from the original (differences to gta-reversed):
//! - the three loaders have no null check on the opened file (gr `FIX_BUGS`, rows
//!   SurfaceInfos_c.cpp:258/304/381);
//! - `LoadSurfaceInfos` stores `int8(_ftol(wetGrip * 100.0f))` (low byte of the truncated signed value,
//!   row :329), the skid-mark type SANDY = 2 and MUDDY = 1 (gr: 1 and 2), and bit 0 of each flag column
//!   (gr `(bool)v`);
//! - `GetSurfaceIdFromName` compares exactly (`repe cmpsb` over the literal including its NUL) and
//!   returns 0 (DEFAULT) for unknown names.
//!
//! Original uninitialised values: the loaders' sscanf output locals start as stack garbage and keep
//! the previous line's values when a line is short; the ports start them at 0 and keep them across
//! lines the same way.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// `SurfaceInfos_c` (opaque: accessed through byte offsets; size from layouts.csv).
#[repr(C)]
pub struct SurfaceInfos {
    pub bytes: [u8; sa_core::layouts::size::SurfaceInfos_c],
}

/// A `SurfaceInfos_c*`.
pub type SiPtr = GPtr<SurfaceInfos>;

/// `g_surfaceInfos`
pub const G_SURFACE_INFOS: u32 = 0xB79538;

sa_core::globals! {
    list GLOBALS;
    // foreign (owner: CWeather)
    /// `CWeather::WetRoads`
    pub static WET_ROADS @ 0xC81308: f32 = 0.0;
}

/// f32 constants of `.rdata`.
mod k {
    /// 0x858B1C: 0.1
    pub const F0_1: u32 = 0x3DCC_CCCD;
    /// 0x858C58: 0.01
    pub const F0_01: u32 = 0x3C23_D70A;
    /// 0x858624: 1.0
    pub const F1: u32 = 0x3F80_0000;
    /// 0x85862C: 10.0
    pub const F10: u32 = 0x4120_0000;
    /// 0x858628: 100.0
    pub const F100: u32 = 0x42C8_0000;
}

orig! {
    /// `CFileMgr::SetDir` (0x5387D0)
    pub fn file_set_dir(path: GPtr<u8>) -> i32 = cdecl 0x5387D0 => sa_core::filemgr::set_dir;
    /// `CFileMgr::OpenFile` (0x538900)
    pub fn file_open(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
    /// `CFileMgr::CloseFile` (0x5389D0)
    pub fn file_close(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    pub fn load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => sa_stream::fileloader::load_line_file;
    /// `SurfaceInfos_c::GetSurfaceIdFromName` (0x55D220)
    pub fn get_surface_id_from_name_call(this: SiPtr, name: GPtr<u8>) -> u32
        = thiscall 0x55D220 => get_surface_id_from_name;
    /// `SurfaceInfos_c::LoadAdhesiveLimits` (0x55D0E0)
    pub fn load_adhesive_limits_call(this: SiPtr) = thiscall 0x55D0E0 => load_adhesive_limits;
    /// `SurfaceInfos_c::LoadSurfaceInfos` (0x55EB90)
    pub fn load_surface_infos_call(this: SiPtr) = thiscall 0x55EB90 => load_surface_infos;
    /// `SurfaceInfos_c::LoadSurfaceAudioInfos` (0x55F2B0)
    pub fn load_surface_audio_infos_call(this: SiPtr) = thiscall 0x55F2B0 => load_surface_audio_infos;
}

use sa_stream::modelinfo::vehicle::sscanf_n;

/// A `.rdata` string of the exe (format strings, file names, literals).
fn s(addr: u32) -> GPtr<u8> {
    GPtr::from_addr(addr as usize)
}

/// A pointer to a local (sscanf target).
fn lp<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// `this + id*12 + off` (the original's `lea eax,[id+id*2]` then `[ecx+eax*4+off]`, 32-bit wrapping).
fn entry<T>(this: SiPtr, id: u32, off: u32) -> GPtr<T> {
    let o = id.wrapping_mul(12).wrapping_add(off) as i32;
    this.cast::<u8>().byte_add(o as isize).cast()
}

fn read_u32(this: SiPtr, id: u32, off: u32) -> u32 {
    entry::<u32>(this, id, off).read()
}

fn read_u8(this: SiPtr, id: u32, off: u32) -> u8 {
    entry::<u8>(this, id, off).read()
}

/// `SurfaceInfos_c::GetTyreGrip` (0x55E5E0, thiscall, `ret 4`): `fild(int8 tyreGrip) * 0.1f` in ST0.
#[hook(addr = 0x55E5E0, conv = thiscall, name = "SurfaceInfos_c::GetTyreGrip", expect = "8B 44 24 04 8D 44 40 24", this = "SurfaceInfos_c")]
pub fn get_tyre_grip(this: SiPtr, id: u32) -> X87 {
    let g = read_u8(this, id, 0x90) as i8;
    X87::from_i32(g as i32).fmul_m(f32::from_bits(k::F0_1))
}

/// `SurfaceInfos_c::GetWetMultiplier` (0x55E600, thiscall, `ret 4`):
/// `(fild(int8 wetGrip) * 0.01f) * CWeather::WetRoads + 1.0f` in ST0.
#[hook(addr = 0x55E600, conv = thiscall, name = "SurfaceInfos_c::GetWetMultiplier", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn get_wet_multiplier(this: SiPtr, id: u32) -> X87 {
    let g = read_u8(this, id, 0x91) as i8;
    X87::from_i32(g as i32)
        .fmul_m(f32::from_bits(k::F0_01))
        .fmul_m(WET_ROADS.get())
        .fadd_m(f32::from_bits(k::F1))
}

/// `SurfaceInfos_c::GetAdhesiveLimit(CColPoint&)` (0x55EB50, thiscall, `ret 4`):
/// `m_adhesiveLimits[group(cp.surfaceB)][group(cp.surfaceA)]` in ST0 (`fld dword`), the groups read
/// from the entries of the colpoint's surface bytes +0x23 and +0x20.
#[hook(addr = 0x55EB50, conv = thiscall, name = "SurfaceInfos_c::GetAdhesiveLimit", expect = "8B 54 24 04 0F B6 42 23", this = "SurfaceInfos_c")]
pub fn get_adhesive_limit(this: SiPtr, cp: GPtr<u8>) -> X87 {
    let b = cp.byte_add(0x23).read() as u32;
    let gb = read_u32(this, b, 0x94) & 7;
    let a = cp.byte_add(0x20).read() as u32;
    let ga = read_u32(this, a, 0x94) & 7;
    let idx = ga.wrapping_add(gb.wrapping_mul(6));
    X87::from_f32(this.cast::<f32>().wrapping_index(idx as i32).read())
}

/// The names `GetSurfaceIdFromName` compares, in id order (the `.rdata` literals of its
/// `repe cmpsb` chain; their index is the returned id).
pub const SURFACE_NAMES: [&[u8]; 179] = [
    b"DEFAULT",
    b"TARMAC",
    b"TARMAC_FUCKED",
    b"TARMAC_REALLYFUCKED",
    b"PAVEMENT",
    b"PAVEMENT_FUCKED",
    b"GRAVEL",
    b"FUCKED_CONCRETE",
    b"PAINTED_GROUND",
    b"GRASS_SHORT_LUSH",
    b"GRASS_MEDIUM_LUSH",
    b"GRASS_LONG_LUSH",
    b"GRASS_SHORT_DRY",
    b"GRASS_MEDIUM_DRY",
    b"GRASS_LONG_DRY",
    b"GOLFGRASS_ROUGH",
    b"GOLFGRASS_SMOOTH",
    b"STEEP_SLIDYGRASS",
    b"STEEP_CLIFF",
    b"FLOWERBED",
    b"MEADOW",
    b"WASTEGROUND",
    b"WOODLANDGROUND",
    b"VEGETATION",
    b"MUD_WET",
    b"MUD_DRY",
    b"DIRT",
    b"DIRTTRACK",
    b"SAND_DEEP",
    b"SAND_MEDIUM",
    b"SAND_COMPACT",
    b"SAND_ARID",
    b"SAND_MORE",
    b"SAND_BEACH",
    b"CONCRETE_BEACH",
    b"ROCK_DRY",
    b"ROCK_WET",
    b"ROCK_CLIFF",
    b"WATER_RIVERBED",
    b"WATER_SHALLOW",
    b"CORNFIELD",
    b"HEDGE",
    b"WOOD_CRATES",
    b"WOOD_SOLID",
    b"WOOD_THIN",
    b"GLASS",
    b"GLASS_WINDOWS_LARGE",
    b"GLASS_WINDOWS_SMALL",
    b"EMPTY1",
    b"EMPTY2",
    b"GARAGE_DOOR",
    b"THICK_METAL_PLATE",
    b"SCAFFOLD_POLE",
    b"LAMP_POST",
    b"METAL_GATE",
    b"METAL_CHAIN_FENCE",
    b"GIRDER",
    b"FIRE_HYDRANT",
    b"CONTAINER",
    b"NEWS_VENDOR",
    b"WHEELBASE",
    b"CARDBOARDBOX",
    b"PED",
    b"CAR",
    b"CAR_PANEL",
    b"CAR_MOVINGCOMPONENT",
    b"TRANSPARENT_CLOTH",
    b"RUBBER",
    b"PLASTIC",
    b"TRANSPARENT_STONE",
    b"WOOD_BENCH",
    b"CARPET",
    b"FLOORBOARD",
    b"STAIRSWOOD",
    b"P_SAND",
    b"P_SAND_DENSE",
    b"P_SAND_ARID",
    b"P_SAND_COMPACT",
    b"P_SAND_ROCKY",
    b"P_SANDBEACH",
    b"P_GRASS_SHORT",
    b"P_GRASS_MEADOW",
    b"P_GRASS_DRY",
    b"P_WOODLAND",
    b"P_WOODDENSE",
    b"P_ROADSIDE",
    b"P_ROADSIDEDES",
    b"P_FLOWERBED",
    b"P_WASTEGROUND",
    b"P_CONCRETE",
    b"P_OFFICEDESK",
    b"P_711SHELF1",
    b"P_711SHELF2",
    b"P_711SHELF3",
    b"P_RESTUARANTTABLE",
    b"P_BARTABLE",
    b"P_UNDERWATERLUSH",
    b"P_UNDERWATERBARREN",
    b"P_UNDERWATERCORAL",
    b"P_UNDERWATERDEEP",
    b"P_RIVERBED",
    b"P_RUBBLE",
    b"P_BEDROOMFLOOR",
    b"P_KIRCHENFLOOR",
    b"P_LIVINGRMFLOOR",
    b"P_CORRIDORFLOOR",
    b"P_711FLOOR",
    b"P_FASTFOODFLOOR",
    b"P_SKANKYFLOOR",
    b"P_MOUNTAIN",
    b"P_MARSH",
    b"P_BUSHY",
    b"P_BUSHYMIX",
    b"P_BUSHYDRY",
    b"P_BUSHYMID",
    b"P_GRASSWEEFLOWERS",
    b"P_GRASSDRYTALL",
    b"P_GRASSLUSHTALL",
    b"P_GRASSGRNMIX",
    b"P_GRASSBRNMIX",
    b"P_GRASSLOW",
    b"P_GRASSROCKY",
    b"P_GRASSSMALLTREES",
    b"P_DIRTROCKY",
    b"P_DIRTWEEDS",
    b"P_GRASSWEEDS",
    b"P_RIVEREDGE",
    b"P_POOLSIDE",
    b"P_FORESTSTUMPS",
    b"P_FORESTSTICKS",
    b"P_FORRESTLEAVES",
    b"P_DESERTROCKS",
    b"P_FORRESTDRY",
    b"P_SPARSEFLOWERS",
    b"P_BUILDINGSITE",
    b"P_DOCKLANDS",
    b"P_INDUSTRIAL",
    b"P_INDUSTJETTY",
    b"P_CONCRETELITTER",
    b"P_ALLEYRUBISH",
    b"P_JUNKYARDPILES",
    b"P_JUNKYARDGRND",
    b"P_DUMP",
    b"P_CACTUSDENSE",
    b"P_AIRPORTGRND",
    b"P_CORNFIELD",
    b"P_GRASSLIGHT",
    b"P_GRASSLIGHTER",
    b"P_GRASSLIGHTER2",
    b"P_GRASSMID1",
    b"P_GRASSMID2",
    b"P_GRASSDARK",
    b"P_GRASSDARK2",
    b"P_GRASSDIRTMIX",
    b"P_RIVERBEDSTONE",
    b"P_RIVERBEDSHALLOW",
    b"P_RIVERBEDWEEDS",
    b"P_SEAWEED",
    b"DOOR",
    b"PLASTICBARRIER",
    b"PARKGRASS",
    b"STAIRSSTONE",
    b"STAIRSMETAL",
    b"STAIRSCARPET",
    b"FLOORMETAL",
    b"FLOORCONCRETE",
    b"BIN_BAG",
    b"THIN_METAL_SHEET",
    b"METAL_BARREL",
    b"PLASTIC_CONE",
    b"PLASTIC_DUMPSTER",
    b"METAL_DUMPSTER",
    b"WOOD_PICKET_FENCE",
    b"WOOD_SLATTED_FENCE",
    b"WOOD_RANCH_FENCE",
    b"UNBREAKABLE_GLASS",
    b"HAY_BALE",
    b"GORE",
    b"RAILTRACK",
];

/// `SurfaceInfos_c::GetSurfaceIdFromName` (0x55D220, thiscall, `ret 4`; `this` unused): the id of the
/// first name that equals `name` exactly, 0 when none does. Each compare reads `name` up to the first
/// differing byte (at most the literal's length + 1), as `repe cmpsb` does.
#[hook(addr = 0x55D220, conv = thiscall, name = "SurfaceInfos_c::GetSurfaceIdFromName", expect = "8B 44 24 04 56", this = "SurfaceInfos_c")]
pub fn get_surface_id_from_name(_this: SiPtr, name: GPtr<u8>) -> u32 {
    for (id, lit) in SURFACE_NAMES.iter().enumerate() {
        let mut equal = true;
        for i in 0..=lit.len() {
            let want = if i < lit.len() { lit[i] } else { 0 };
            if name.byte_add(i as isize).read() != want {
                equal = false;
                break;
            }
        }
        if equal {
            return id as u32;
        }
    }
    0
}

/// `SurfaceInfos_c::Init` (0x55F420, thiscall): LoadAdhesiveLimits, LoadSurfaceInfos,
/// LoadSurfaceAudioInfos (tail jump).
#[hook(addr = 0x55F420, conv = thiscall, name = "SurfaceInfos_c::Init", expect = "56 8B F1 E8 B8 DC FF FF", this = "SurfaceInfos_c")]
pub fn init(this: SiPtr) {
    load_adhesive_limits_call(this);
    load_surface_infos_call(this);
    load_surface_audio_infos_call(this);
}

/// Skips to the next ' ' or '\t' (not stopping at NUL), as 0x55D157..0x55D16A.
fn skip_to_blank(mut p: GPtr<u8>) -> GPtr<u8> {
    let mut c = p.read();
    while c != b' ' && c != b'\t' {
        c = p.byte_add(1).read();
        p = p.byte_add(1);
    }
    p
}

/// `SurfaceInfos_c::LoadAdhesiveLimits` (0x55D0E0, thiscall). `SetDir("")`, then
/// `data\surface.dat` ("rb", no null check). Every line that does not start with ';' or NUL is row
/// `r` (counted from 0, unbounded): its first token is read with `%s` and skipped (to the next blank,
/// not stopping at NUL), then `r + 1` values follow; each is 0 when it starts with '-', else `%f`;
/// value `j` goes to `[r][j]` and `[j][r]`.
#[hook(addr = 0x55D0E0, conv = thiscall, name = "SurfaceInfos_c::LoadAdhesiveLimits", expect = "81 EC 14 01 00 00", this = "SurfaceInfos_c")]
pub fn load_adhesive_limits(this: SiPtr) {
    file_set_dir(s(0x858B54));
    let file = file_open(s(0x864318), s(0x859C6C));
    let mut row: i32 = 0;
    let mut row_ptr = this.cast::<u32>();
    let mut col_ptr = this.cast::<u32>();
    let mut name = [0u8; 0xF0];
    let mut line = load_line(file);
    while !line.is_null() {
        let c = line.read();
        if c != b';' && c != 0 {
            sscanf_n(line, s(0x85C714), &[GPtr::from_raw(name.as_mut_ptr())]);
            let mut p = skip_to_blank(line);
            if row >= 0 {
                let (mut a, mut b) = (row_ptr, col_ptr);
                let mut n = row + 1;
                loop {
                    loop {
                        let c = p.read();
                        if c != b' ' && c != b'\t' {
                            break;
                        }
                        p = p.byte_add(1);
                    }
                    let mut v: u32 = 0;
                    if p.read() != b'-' {
                        sscanf_n(p, s(0x862EF0), &[lp(&mut v)]);
                    }
                    let mut c = p.read();
                    while c != b' ' && c != b'\t' && c != 0 {
                        c = p.byte_add(1).read();
                        p = p.byte_add(1);
                    }
                    a.write(v);
                    b.write(v);
                    a = a.wrapping_index(1);
                    b = b.wrapping_index(6);
                    n -= 1;
                    if n == 0 {
                        break;
                    }
                }
            }
            row += 1;
            row_ptr = row_ptr.wrapping_index(6);
            col_ptr = col_ptr.wrapping_index(1);
        }
        line = load_line(file);
    }
    file_close(file);
}

/// `flags = flags ^ ((v << pos ^ flags) & mask)`: the low bit(s) of `v` replace the field.
fn merge(flags: u32, v: u32, pos: u32, mask: u32) -> u32 {
    flags ^ ((v.wrapping_shl(pos) ^ flags) & mask)
}

/// True when the NUL-terminated local `buf` equals `.rdata` literal `lit` (`repe cmpsb` over the
/// literal's length + 1).
fn eq_lit(buf: &[u8], lit: &[u8]) -> bool {
    (0..=lit.len()).all(|i| buf.get(i).copied().unwrap_or(0) == if i < lit.len() { lit[i] } else { 0 })
}

/// `SurfaceInfos_c::LoadSurfaceInfos` (0x55EB90, thiscall): `data\surfinfo.dat` ("r", no null check);
/// lines starting with '#' or NUL are skipped; the rest is read with the 36-conversion format
/// 0x864D48 (result ignored) and stored into the entry of `GetSurfaceIdFromName(name)` (see the
/// module docs and `docs/re/specs/COLL/SurfaceInfos_c.md` for the columns).
#[hook(addr = 0x55EB90, conv = thiscall, name = "SurfaceInfos_c::LoadSurfaceInfos", expect = "81 EC 40 01 00 00", this = "SurfaceInfos_c")]
pub fn load_surface_infos(this: SiPtr) {
    let file = file_open(s(0x864DB4), s(0x85A53C));
    let mut name = [0u8; 64];
    let mut adhesion = [0u8; 32];
    let mut skid = [0u8; 32];
    let mut friction = [0u8; 32];
    let mut bullet = [0u8; 32];
    let mut tyre_grip: f32 = 0.0;
    let mut wet_grip: f32 = 0.0;
    let mut col = [0u32; 29];
    let mut line = load_line(file);
    while !line.is_null() {
        let c = line.read();
        if c != b'#' && c != 0 {
            let mut t: [GPtr<u8>; 36] = [GPtr::NULL; 36];
            t[0] = GPtr::from_raw(name.as_mut_ptr());
            t[1] = GPtr::from_raw(adhesion.as_mut_ptr());
            t[2] = lp(&mut tyre_grip);
            t[3] = lp(&mut wet_grip);
            t[4] = GPtr::from_raw(skid.as_mut_ptr());
            t[5] = GPtr::from_raw(friction.as_mut_ptr());
            for (k, v) in col.iter_mut().enumerate() {
                t[6 + k] = lp(v);
            }
            t[35] = GPtr::from_raw(bullet.as_mut_ptr());
            sscanf_n(line, s(0x864D48), &t);
            let id = get_surface_id_from_name_call(this, GPtr::from_raw(name.as_mut_ptr()));
            let f1 = entry::<u32>(this, id, 0x94);
            // adhesion group (bits 0-2)
            for (lit, v) in [
                (&b"RUBBER"[..], 0u32),
                (b"HARD", 1),
                (b"ROAD", 2),
                (b"LOOSE", 3),
                (b"SAND", 4),
                (b"WET", 5),
            ] {
                if eq_lit(&adhesion, lit) {
                    f1.write((f1.read() & !7) | v);
                    break;
                }
            }
            let tg = X87::from_f32(tyre_grip).fmul_m(f32::from_bits(k::F10)).ftol() as u8;
            entry::<u8>(this, id, 0x90).write(tg);
            let wg = X87::from_f32(wet_grip).fmul_m(f32::from_bits(k::F100)).ftol() as u8;
            entry::<u8>(this, id, 0x91).write(wg);
            // skid mark (bits 3-4): DEFAULT 0, SANDY 2, MUDDY 1
            if eq_lit(&skid, b"DEFAULT") {
                f1.write(f1.read() & !0x18);
            } else if eq_lit(&skid, b"SANDY") {
                f1.write((f1.read() & !0x08) | 0x10);
            } else if eq_lit(&skid, b"MUDDY") {
                f1.write((f1.read() & !0x10) | 0x08);
            }
            // friction effect (bits 5-7): NONE 0, SPARKS 1
            if eq_lit(&friction, b"NONE") {
                f1.write(f1.read() & !0xE0);
            } else if eq_lit(&friction, b"SPARKS") {
                f1.write((f1.read() & !0xC0) | 0x20);
            }
            // bullet fx (bits 8-10)
            for (lit, clear, set) in [
                (&b"NONE"[..], 0x700u32, 0u32),
                (b"SPARKS", 0x600, 0x100),
                (b"SAND", 0x500, 0x200),
                (b"WOOD", 0x400, 0x300),
                (b"DUST", 0x300, 0x400),
            ] {
                if eq_lit(&bullet, lit) {
                    f1.write((f1.read() & !clear) | set);
                    break;
                }
            }
            // columns 7..25 -> flags1 bits 11..31
            const F1_FIELDS: [(u32, u32); 19] = [
                (11, 0x800),
                (12, 0x1000),
                (13, 0x2000),
                (14, 0x4000),
                (15, 0x8000),
                (16, 0x1_0000),
                (17, 0x2_0000),
                (18, 0x4_0000),
                (19, 0x8_0000),
                (20, 0x10_0000),
                (21, 0x20_0000),
                (22, 0x40_0000),
                (23, 0x180_0000),
                (25, 0x600_0000),
                (27, 0x800_0000),
                (28, 0x1000_0000),
                (29, 0x2000_0000),
                (30, 0x4000_0000),
                (31, 0x8000_0000),
            ];
            let mut f = f1.read();
            for (k, (pos, mask)) in F1_FIELDS.iter().enumerate() {
                f = if *pos == 31 {
                    (f & 0x7FFF_FFFF) | (col[k] << 31)
                } else {
                    merge(f, col[k], *pos, *mask)
                };
                f1.write(f);
            }
            // columns 26..35 -> flags2 bits 0..9
            let f2 = entry::<u32>(this, id, 0x98);
            let mut g = f2.read();
            for b in 0..10u32 {
                g = merge(g, col[19 + b as usize], b, 1 << b);
                f2.write(g);
            }
        }
        line = load_line(file);
    }
    file_close(file);
}

/// `SurfaceInfos_c::LoadSurfaceAudioInfos` (0x55F2B0, thiscall): `data\surfaud.dat` ("r", no null
/// check); lines starting with '#' or NUL are skipped; `sscanf(line, "%s %d x9")` (result ignored),
/// then bit 0 of the nine values into flags2 bits 10..18 of `GetSurfaceIdFromName(name)`'s entry.
#[hook(addr = 0x55F2B0, conv = thiscall, name = "SurfaceInfos_c::LoadSurfaceAudioInfos", expect = "83 EC 64 56 57", this = "SurfaceInfos_c")]
pub fn load_surface_audio_infos(this: SiPtr) {
    let file = file_open(s(0x864DE8), s(0x85A53C));
    let mut name = [0u8; 64];
    let mut d = [0u32; 9];
    let mut line = load_line(file);
    while !line.is_null() {
        let c = line.read();
        if c != b'#' && c != 0 {
            let mut t: [GPtr<u8>; 10] = [GPtr::NULL; 10];
            t[0] = GPtr::from_raw(name.as_mut_ptr());
            for (k, v) in d.iter_mut().enumerate() {
                t[1 + k] = lp(v);
            }
            sscanf_n(line, s(0x864DC8), &t);
            let id = get_surface_id_from_name_call(this, GPtr::from_raw(name.as_mut_ptr()));
            let f2 = entry::<u32>(this, id, 0x98);
            let mut g = f2.read();
            for (k, v) in d.iter().enumerate() {
                let pos = 10 + k as u32;
                g = merge(g, *v, pos, 1 << pos);
                f2.write(g);
            }
        }
        line = load_line(file);
    }
    file_close(file);
}

/// `SurfaceInfos_c::GetAdhesionGroup` (0x55E5C0, thiscall, `ret 4`): dword `+0x94` & 0x7 of entry `id`, in EAX.
#[hook(addr = 0x55E5C0, conv = thiscall, name = "SurfaceInfos_c::GetAdhesionGroup", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn get_adhesion_group(this: SiPtr, id: u32) -> u32 {
    read_u32(this, id, 0x94) & 0x7
}

/// `SurfaceInfos_c::GetSkidmarkType` (0x55E630, thiscall, `ret 4`): dword `+0x94` >> 3 & 0x3 of entry `id`, in EAX.
#[hook(addr = 0x55E630, conv = thiscall, name = "SurfaceInfos_c::GetSkidmarkType", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn get_skidmark_type(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 3) & 0x3
}

/// `SurfaceInfos_c::GetFrictionEffect` (0x55E650, thiscall, `ret 4`): dword `+0x94` >> 5 & 0x7 of entry `id`, in EAX.
#[hook(addr = 0x55E650, conv = thiscall, name = "SurfaceInfos_c::GetFrictionEffect", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn get_friction_effect(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 5) & 0x7
}

/// `SurfaceInfos_c::GetBulletFx` (0x55E670, thiscall, `ret 4`): dword `+0x94` >> 8 & 0x7 of entry `id`, in EAX.
#[hook(addr = 0x55E670, conv = thiscall, name = "SurfaceInfos_c::GetBulletFx", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn get_bullet_fx(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 8) & 0x7
}

/// `SurfaceInfos_c::IsSoftLanding` (0x55E690, thiscall, `ret 4`): dword `+0x94` >> 11 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E690, conv = thiscall, name = "SurfaceInfos_c::IsSoftLanding", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_soft_landing(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 11) & 0x1
}

/// `SurfaceInfos_c::IsSeeThrough` (0x55E6B0, thiscall, `ret 4`): dword `+0x94` >> 12 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E6B0, conv = thiscall, name = "SurfaceInfos_c::IsSeeThrough", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_see_through(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 12) & 0x1
}

/// `SurfaceInfos_c::IsShootThrough` (0x55E6D0, thiscall, `ret 4`): dword `+0x94` >> 13 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E6D0, conv = thiscall, name = "SurfaceInfos_c::IsShootThrough", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_shoot_through(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 13) & 0x1
}

/// `SurfaceInfos_c::IsSand` (0x55E6F0, thiscall, `ret 4`): dword `+0x94` >> 14 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E6F0, conv = thiscall, name = "SurfaceInfos_c::IsSand", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_sand(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 14) & 0x1
}

/// `SurfaceInfos_c::IsWater` (0x55E710, thiscall, `ret 4`): dword `+0x94` >> 15 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E710, conv = thiscall, name = "SurfaceInfos_c::IsWater", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_water(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 15) & 0x1
}

/// `SurfaceInfos_c::IsShallowWater` (0x55E730, thiscall, `ret 4`): byte `+0x96` & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E730, conv = thiscall, name = "SurfaceInfos_c::IsShallowWater", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_shallow_water(this: SiPtr, id: u32) -> u32 {
    read_u8(this, id, 0x96) as u32 & 0x1
}

/// `SurfaceInfos_c::IsBeach` (0x55E750, thiscall, `ret 4`): dword `+0x94` >> 17 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E750, conv = thiscall, name = "SurfaceInfos_c::IsBeach", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_beach(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 17) & 0x1
}

/// `SurfaceInfos_c::IsSteepSlope` (0x55E770, thiscall, `ret 4`): dword `+0x94` >> 18 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E770, conv = thiscall, name = "SurfaceInfos_c::IsSteepSlope", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_steep_slope(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 18) & 0x1
}

/// `SurfaceInfos_c::IsGlass` (0x55E790, thiscall, `ret 4`): dword `+0x94` >> 19 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E790, conv = thiscall, name = "SurfaceInfos_c::IsGlass", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_glass(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 19) & 0x1
}

/// `SurfaceInfos_c::IsStairs` (0x55E7B0, thiscall, `ret 4`): dword `+0x94` >> 20 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E7B0, conv = thiscall, name = "SurfaceInfos_c::IsStairs", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_stairs(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 20) & 0x1
}

/// `SurfaceInfos_c::IsSkateable` (0x55E7D0, thiscall, `ret 4`): dword `+0x94` >> 21 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E7D0, conv = thiscall, name = "SurfaceInfos_c::IsSkateable", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_skateable(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 21) & 0x1
}

/// `SurfaceInfos_c::IsPavement` (0x55E7F0, thiscall, `ret 4`): dword `+0x94` >> 22 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E7F0, conv = thiscall, name = "SurfaceInfos_c::IsPavement", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_pavement(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 22) & 0x1
}

/// `SurfaceInfos_c::GetRoughness` (0x55E810, thiscall, `ret 4`): dword `+0x94` >> 23 & 0x3 of entry `id`, in EAX.
#[hook(addr = 0x55E810, conv = thiscall, name = "SurfaceInfos_c::GetRoughness", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn get_roughness(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 23) & 0x3
}

/// `SurfaceInfos_c::GetFlammability` (0x55E830, thiscall, `ret 4`): dword `+0x94` >> 25 & 0x3 of entry `id`, in EAX.
#[hook(addr = 0x55E830, conv = thiscall, name = "SurfaceInfos_c::GetFlammability", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn get_flammability(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 25) & 0x3
}

/// `SurfaceInfos_c::CreatesSparks` (0x55E850, thiscall, `ret 4`): dword `+0x94` >> 27 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E850, conv = thiscall, name = "SurfaceInfos_c::CreatesSparks", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn creates_sparks(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 27) & 0x1
}

/// `SurfaceInfos_c::CantSprintOn` (0x55E870, thiscall, `ret 4`): dword `+0x94` >> 28 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E870, conv = thiscall, name = "SurfaceInfos_c::CantSprintOn", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn cant_sprint_on(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 28) & 0x1
}

/// `SurfaceInfos_c::LeavesFootsteps` (0x55E890, thiscall, `ret 4`): dword `+0x94` >> 29 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E890, conv = thiscall, name = "SurfaceInfos_c::LeavesFootsteps", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn leaves_footsteps(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 29) & 0x1
}

/// `SurfaceInfos_c::ProducesFootDust` (0x55E8B0, thiscall, `ret 4`): dword `+0x94` >> 30 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E8B0, conv = thiscall, name = "SurfaceInfos_c::ProducesFootDust", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn produces_foot_dust(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x94) >> 30) & 0x1
}

/// `SurfaceInfos_c::MakesCarDirty` (0x55E8D0, thiscall, `ret 4`): dword `+0x94` >> 31 of entry `id`, in EAX.
#[hook(addr = 0x55E8D0, conv = thiscall, name = "SurfaceInfos_c::MakesCarDirty", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn makes_car_dirty(this: SiPtr, id: u32) -> u32 {
    read_u32(this, id, 0x94) >> 31
}

/// `SurfaceInfos_c::MakesCarClean` (0x55E8F0, thiscall, `ret 4`): dword `+0x98` & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E8F0, conv = thiscall, name = "SurfaceInfos_c::MakesCarClean", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn makes_car_clean(this: SiPtr, id: u32) -> u32 {
    read_u32(this, id, 0x98) & 0x1
}

/// `SurfaceInfos_c::CreatesWheelGrass` (0x55E910, thiscall, `ret 4`): dword `+0x98` >> 1 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E910, conv = thiscall, name = "SurfaceInfos_c::CreatesWheelGrass", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn creates_wheel_grass(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 1) & 0x1
}

/// `SurfaceInfos_c::CreatesWheelGravel` (0x55E930, thiscall, `ret 4`): dword `+0x98` >> 2 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E930, conv = thiscall, name = "SurfaceInfos_c::CreatesWheelGravel", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn creates_wheel_gravel(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 2) & 0x1
}

/// `SurfaceInfos_c::CreatesWheelMud` (0x55E950, thiscall, `ret 4`): dword `+0x98` >> 3 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E950, conv = thiscall, name = "SurfaceInfos_c::CreatesWheelMud", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn creates_wheel_mud(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 3) & 0x1
}

/// `SurfaceInfos_c::CreatesWheelDust` (0x55E970, thiscall, `ret 4`): dword `+0x98` >> 4 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E970, conv = thiscall, name = "SurfaceInfos_c::CreatesWheelDust", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn creates_wheel_dust(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 4) & 0x1
}

/// `SurfaceInfos_c::CreatesWheelSand` (0x55E990, thiscall, `ret 4`): dword `+0x98` >> 5 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E990, conv = thiscall, name = "SurfaceInfos_c::CreatesWheelSand", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn creates_wheel_sand(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 5) & 0x1
}

/// `SurfaceInfos_c::CreatesWheelSpray` (0x55E9B0, thiscall, `ret 4`): dword `+0x98` >> 6 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E9B0, conv = thiscall, name = "SurfaceInfos_c::CreatesWheelSpray", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn creates_wheel_spray(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 6) & 0x1
}

/// `SurfaceInfos_c::CreatesPlants` (0x55E9D0, thiscall, `ret 4`): dword `+0x98` >> 7 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E9D0, conv = thiscall, name = "SurfaceInfos_c::CreatesPlants", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn creates_plants(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 7) & 0x1
}

/// `SurfaceInfos_c::CreatesObjects` (0x55E9F0, thiscall, `ret 4`): dword `+0x98` >> 8 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55E9F0, conv = thiscall, name = "SurfaceInfos_c::CreatesObjects", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn creates_objects(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 8) & 0x1
}

/// `SurfaceInfos_c::CanClimb` (0x55EA10, thiscall, `ret 4`): dword `+0x98` >> 9 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55EA10, conv = thiscall, name = "SurfaceInfos_c::CanClimb", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn can_climb(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 9) & 0x1
}

/// `SurfaceInfos_c::IsAudioConcrete` (0x55EA30, thiscall, `ret 4`): dword `+0x98` >> 10 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55EA30, conv = thiscall, name = "SurfaceInfos_c::IsAudioConcrete", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_audio_concrete(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 10) & 0x1
}

/// `SurfaceInfos_c::IsAudioGrass` (0x55EA50, thiscall, `ret 4`): dword `+0x98` >> 11 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55EA50, conv = thiscall, name = "SurfaceInfos_c::IsAudioGrass", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_audio_grass(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 11) & 0x1
}

/// `SurfaceInfos_c::IsAudioSand` (0x55EA70, thiscall, `ret 4`): dword `+0x98` >> 12 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55EA70, conv = thiscall, name = "SurfaceInfos_c::IsAudioSand", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_audio_sand(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 12) & 0x1
}

/// `SurfaceInfos_c::IsAudioGravel` (0x55EA90, thiscall, `ret 4`): dword `+0x98` >> 13 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55EA90, conv = thiscall, name = "SurfaceInfos_c::IsAudioGravel", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_audio_gravel(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 13) & 0x1
}

/// `SurfaceInfos_c::IsAudioWood` (0x55EAB0, thiscall, `ret 4`): dword `+0x98` >> 14 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55EAB0, conv = thiscall, name = "SurfaceInfos_c::IsAudioWood", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_audio_wood(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 14) & 0x1
}

/// `SurfaceInfos_c::IsAudioWater` (0x55EAD0, thiscall, `ret 4`): dword `+0x98` >> 15 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55EAD0, conv = thiscall, name = "SurfaceInfos_c::IsAudioWater", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_audio_water(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 15) & 0x1
}

/// `SurfaceInfos_c::IsAudioMetal` (0x55EAF0, thiscall, `ret 4`): byte `+0x9a` & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55EAF0, conv = thiscall, name = "SurfaceInfos_c::IsAudioMetal", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_audio_metal(this: SiPtr, id: u32) -> u32 {
    read_u8(this, id, 0x9a) as u32 & 0x1
}

/// `SurfaceInfos_c::IsAudioLongGrass` (0x55EB10, thiscall, `ret 4`): dword `+0x98` >> 17 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55EB10, conv = thiscall, name = "SurfaceInfos_c::IsAudioLongGrass", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_audio_long_grass(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 17) & 0x1
}

/// `SurfaceInfos_c::IsAudioTile` (0x55EB30, thiscall, `ret 4`): dword `+0x98` >> 18 & 0x1 of entry `id`, in EAX.
#[hook(addr = 0x55EB30, conv = thiscall, name = "SurfaceInfos_c::IsAudioTile", expect = "8B 44 24 04 8D 04 40", this = "SurfaceInfos_c")]
pub fn is_audio_tile(this: SiPtr, id: u32) -> u32 {
    (read_u32(this, id, 0x98) >> 18) & 0x1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_LOAD_ADHESIVE_LIMITS,
    HOOK_GET_SURFACE_ID_FROM_NAME,
    HOOK_GET_ADHESION_GROUP,
    HOOK_GET_TYRE_GRIP,
    HOOK_GET_WET_MULTIPLIER,
    HOOK_GET_SKIDMARK_TYPE,
    HOOK_GET_FRICTION_EFFECT,
    HOOK_GET_BULLET_FX,
    HOOK_IS_SOFT_LANDING,
    HOOK_IS_SEE_THROUGH,
    HOOK_IS_SHOOT_THROUGH,
    HOOK_IS_SAND,
    HOOK_IS_WATER,
    HOOK_IS_SHALLOW_WATER,
    HOOK_IS_BEACH,
    HOOK_IS_STEEP_SLOPE,
    HOOK_IS_GLASS,
    HOOK_IS_STAIRS,
    HOOK_IS_SKATEABLE,
    HOOK_IS_PAVEMENT,
    HOOK_GET_ROUGHNESS,
    HOOK_GET_FLAMMABILITY,
    HOOK_CREATES_SPARKS,
    HOOK_CANT_SPRINT_ON,
    HOOK_LEAVES_FOOTSTEPS,
    HOOK_PRODUCES_FOOT_DUST,
    HOOK_MAKES_CAR_DIRTY,
    HOOK_MAKES_CAR_CLEAN,
    HOOK_CREATES_WHEEL_GRASS,
    HOOK_CREATES_WHEEL_GRAVEL,
    HOOK_CREATES_WHEEL_MUD,
    HOOK_CREATES_WHEEL_DUST,
    HOOK_CREATES_WHEEL_SAND,
    HOOK_CREATES_WHEEL_SPRAY,
    HOOK_CREATES_PLANTS,
    HOOK_CREATES_OBJECTS,
    HOOK_CAN_CLIMB,
    HOOK_IS_AUDIO_CONCRETE,
    HOOK_IS_AUDIO_GRASS,
    HOOK_IS_AUDIO_SAND,
    HOOK_IS_AUDIO_GRAVEL,
    HOOK_IS_AUDIO_WOOD,
    HOOK_IS_AUDIO_WATER,
    HOOK_IS_AUDIO_METAL,
    HOOK_IS_AUDIO_LONG_GRASS,
    HOOK_IS_AUDIO_TILE,
    HOOK_GET_ADHESIVE_LIMIT,
    HOOK_LOAD_SURFACE_INFOS,
    HOOK_LOAD_SURFACE_AUDIO_INFOS,
    HOOK_INIT,
];
