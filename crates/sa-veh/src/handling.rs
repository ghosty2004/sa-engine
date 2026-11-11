//! cHandlingDataMgr (PORT-VEH). Ported from the 1.0 US bytes; gta-reversed as a reading aid.
//!
//! The manager (0xC624, global instance 0xC2B9C8): +0x14 car handling entries (0xE0 each, drive type at
//! entry +0x74), +0x7B24 + 0x58 * id flying data, +0x9A50 + 0x3C * id boat data. All members are
//! thiscall (the callers pass the manager in ECX even where it is unused).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x};
use sa_math::x87::X87;

orig! {
    /// CRT `strncmp` (0x8214D0)
    pub fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    /// CRT `strncpy` (0x821F40)
    pub fn strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `cHandlingDataMgr::LoadHandlingData()` (0x5BD830)
    pub fn load_handling_data(mgr: GPtr<u8>) = thiscall 0x5BD830 => unported;
}

/// `VehicleNames[210][14]` (0x8D3978), the handling.cfg identifiers.
const VEHICLE_NAMES: usize = 0x8D3978;

/// The drive-type test of HasFront/RearWheelDrive: EAX keeps the upper bytes of `id * 0xE0` from the
/// `imul` (only AL is replaced by the result).
fn drive_type_is_not(mgr: GPtr<u8>, id: u32, c: u8) -> u32 {
    let e = (id & 0xFF).wrapping_mul(0xE0);
    let t = rd::<u8>(mgr, e as isize + 0x88);
    (e & 0xFFFF_FF00) | u32::from(t != c)
}

/// `cHandlingDataMgr::HasFrontWheelDrive(uint8)` (0x6A0480): drive type != 'R'.
#[hook(addr = 0x6A0480, conv = thiscall, name = "cHandlingDataMgr::HasFrontWheelDrive", expect = "0F B6 44 24 04", this = "cHandlingDataMgr")]
pub fn has_front_wheel_drive(mgr: GPtr<u8>, id: u32) -> u32 {
    drive_type_is_not(mgr, id, b'R')
}

/// `cHandlingDataMgr::HasRearWheelDrive(uint8)` (0x6A04B0): drive type != 'F'.
#[hook(addr = 0x6A04B0, conv = thiscall, name = "cHandlingDataMgr::HasRearWheelDrive", expect = "0F B6 44 24 04", this = "cHandlingDataMgr")]
pub fn has_rear_wheel_drive(mgr: GPtr<u8>, id: u32) -> u32 {
    drive_type_is_not(mgr, id, b'F')
}

/// `cHandlingDataMgr::GetBoatPointer(uint8)` (0x6F5300): ids 0xAF..=0xBA -> `this + 0x9A50 + 0x3C * id`,
/// else `this + 0xC354`.
#[hook(addr = 0x6F5300, conv = thiscall, name = "cHandlingDataMgr::GetBoatPointer", expect = "8A 44 24 04 3C AF", this = "cHandlingDataMgr")]
pub fn get_boat_pointer(mgr: GPtr<u8>, id: u32) -> GPtr<u8> {
    let id = id as u8;
    if (0xAF..=0xBA).contains(&id) {
        mgr.byte_add(isize::from(id) * 0x3C + 0x9A50)
    } else {
        mgr.byte_add(0xC354)
    }
}

/// `cHandlingDataMgr::GetFlyingPointer(uint8)` (0x6F52D0, tail at 0x403751): ids 0xBA..=0xD1 ->
/// `this + 0x7B24 + 0x58 * id`, else `this + 0xBB14`.
#[hook(addr = 0x6F52D0, conv = thiscall, name = "cHandlingDataMgr::GetFlyingPointer", expect = "8A 44 24 04 3C BA", this = "cHandlingDataMgr")]
pub fn get_flying_pointer(mgr: GPtr<u8>, id: u32) -> GPtr<u8> {
    let id = id as u8;
    if (0xBA..=0xD1).contains(&id) {
        mgr.byte_add(isize::from(id) * 0x58 + 0x7B24)
    } else {
        mgr.byte_add(0xBB14)
    }
}

/// `cHandlingDataMgr::ConvertBikeDataToGameUnits(tBikeHandlingData*)` (0x6F5290): degrees to radians
/// (`* 0x8595EC`); +0x14, +0x2C and +0x30 also go through FSIN, +0x18 does not.
#[hook(addr = 0x6F5290, conv = thiscall, name = "cHandlingDataMgr::ConvertBikeDataToGameUnits", expect = "8B 44 24 04 D9 40 14", this = "cHandlingDataMgr")]
pub fn convert_bike_data_to_game_units(_mgr: GPtr<u8>, data: GPtr<u8>) {
    let k = g::<f32>(0x8595EC).read();
    wr::<f32>(data, 0x14, x(rd::<f32>(data, 0x14)).fmul_m(k).fsin().to_f32());
    wr::<f32>(data, 0x18, x(rd::<f32>(data, 0x18)).fmul_m(k).to_f32());
    wr::<f32>(data, 0x2C, x(rd::<f32>(data, 0x2C)).fmul_m(k).fsin().to_f32());
    wr::<f32>(data, 0x30, x(rd::<f32>(data, 0x30)).fmul_m(k).fsin().to_f32());
}

/// `cHandlingDataMgr::GetHandlingId(const char*)` (0x6F4FD0): index of the first of the 210
/// `VehicleNames` entries with `strncmp(name, entry, 14) == 0`; 210 when none matches.
#[hook(addr = 0x6F4FD0, conv = thiscall, name = "cHandlingDataMgr::GetHandlingId", expect = "53 8B 5C 24 08", this = "cHandlingDataMgr")]
pub fn get_handling_id(_mgr: GPtr<u8>, name: GPtr<u8>) -> i32 {
    let mut i = 0;
    while i < 210 {
        if strncmp(name, GPtr::from_addr(VEHICLE_NAMES + 14 * i as usize), 14) == 0 {
            break;
        }
        i += 1;
    }
    i
}

/// `cHandlingDataMgr::FindExactWord(const char* line, const char* table, int32 entrySize, int32 count)`
/// (0x6F4F30): do-while over the table (the first entry is tested even for `count <= 0`): copies the
/// entry's `strlen` characters to a 20-byte stack buffer with `strncpy` and compares `strncmp(line, buf,
/// len)`; returns the index of the first match, else `count`. Deviation: an entry longer than 20
/// characters overflows the original's stack frame; here it is compared in place instead.
#[hook(addr = 0x6F4F30, conv = thiscall, name = "cHandlingDataMgr::FindExactWord", expect = "83 EC 14 53 55", this = "cHandlingDataMgr")]
pub fn find_exact_word(_mgr: GPtr<u8>, line: GPtr<u8>, table: GPtr<u8>, entry_size: i32, count: i32) -> i32 {
    let mut i = 0i32;
    let mut p = table;
    loop {
        let mut len = 0u32;
        while rd::<u8>(p, len as isize) != 0 {
            len += 1;
        }
        let mut buf = [0u8; 0x14];
        let bp = GPtr::from_raw(&raw mut buf).cast::<u8>();
        let cmp = if len as usize <= buf.len() {
            strncpy(bp, p, len);
            strncmp(line, bp, len)
        } else {
            strncmp(line, p, len)
        };
        if cmp == 0 {
            return i;
        }
        i += 1;
        p = p.byte_add(entry_size as isize);
        if i >= count {
            return count;
        }
    }
}

/// `cHandlingDataMgr::ConvertDataToWorldUnits(tHandlingData*)` (0x6F5010): `t = 1.0 / [0xC2B9B4]`;
/// +0x7C = t * +0x7C, then (re-read from the f32 store) times 4.0 for drive type '4' or doubled; +0x84 /=
/// [0xC2B9BC]; +0x94 *= t; +0xC8 = +0x4 * +0xC8 * 0.0005.
#[hook(addr = 0x6F5010, conv = thiscall, name = "cHandlingDataMgr::ConvertDataToWorldUnits", expect = "D9 05 24 86 85 00", this = "cHandlingDataMgr")]
pub fn convert_data_to_world_units(_mgr: GPtr<u8>, data: GPtr<u8>) {
    let t = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0xC2B9B4).read());
    let four_wd = rd::<u8>(data, 0x74) == 0x34;
    let s = t.fmul_m(rd::<f32>(data, 0x7C)).to_f32();
    wr::<f32>(data, 0x7C, s);
    wr::<f32>(
        data,
        0x84,
        x(rd::<f32>(data, 0x84))
            .fdiv_m(g::<f32>(0xC2B9BC).read())
            .to_f32(),
    );
    wr::<f32>(data, 0x94, t.fmul_m(rd::<f32>(data, 0x94)).to_f32());
    let r = if four_wd {
        x(s).fmul_m(g::<f32>(0x858B90).read())
    } else {
        x(s).fdouble()
    };
    wr::<f32>(data, 0x7C, r.to_f32());
    let v = x(rd::<f32>(data, 4))
        .fmul_m(rd::<f32>(data, 0xC8))
        .fmul_m(g::<f32>(0x859978).read());
    wr::<f32>(data, 0xC8, v.to_f32());
}

/// `cHandlingDataMgr::Initialise()` (0x5BF3D0): LoadHandlingData, then the five manager floats
/// +0..+0x10 (0.1, 0.9, 1.0, 0.8, 0.98).
#[hook(addr = 0x5BF3D0, conv = thiscall, name = "cHandlingDataMgr::Initialise", expect = "56 8B F1 E8 58 E4 FF FF", this = "cHandlingDataMgr")]
pub fn initialise(mgr: GPtr<u8>) {
    load_handling_data(mgr);
    wr::<u32>(mgr, 0, 0x3DCC_CCCD);
    wr::<u32>(mgr, 4, 0x3F66_6666);
    wr::<u32>(mgr, 8, 0x3F80_0000);
    wr::<u32>(mgr, 0xC, 0x3F4C_CCCD);
    wr::<u32>(mgr, 0x10, 0x3F7A_E148);
}

orig! {
    fn hd_set_dir(dir: GPtr<u8>) = cdecl 0x5387D0 => unported;
    fn hd_open_file(path: GPtr<u8>, mode: GPtr<u8>) -> u32 = cdecl 0x538900 => unported;
    fn hd_close_file(file: u32) -> i32 = cdecl 0x5389D0 => unported;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    fn hd_load_line(file: u32) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// CRT `strtok(s, delims)` (0x82244B)
    fn hd_strtok(s: GPtr<u8>, delims: GPtr<u8>) -> GPtr<u8> = cdecl 0x82244B => unported;
    /// CRT `atof` (0x823CEE, result in ST0)
    fn hd_atof(s: GPtr<u8>) -> f64 = cdecl 0x823CEE => unported;
    /// CRT `atol` (0x82258E)
    fn hd_atol(s: GPtr<u8>) -> i32 = cdecl 0x82258E => unported;
    /// CRT `sscanf(s, "%x", out)` (0x8220AD)
    fn hd_sscanf_x(s: GPtr<u8>, fmt: GPtr<u8>, out: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// `cHandlingDataMgr::FindExactWord(word, names, nameLen, count)` (0x6F4F30)
    fn hd_find_exact_word(mgr: GPtr<u8>, word: GPtr<u8>, names: GPtr<u8>, len: u32, count: u32) -> u32 = thiscall 0x6F4F30 => unported;
    fn hd_get_boat_pointer(mgr: GPtr<u8>, id: u32) -> GPtr<u8> = thiscall 0x6F5300 => unported;
    fn hd_convert_bike_data(mgr: GPtr<u8>, bike: GPtr<u8>) = thiscall 0x6F5290 => unported;
    fn hd_convert_data(mgr: GPtr<u8>, h: GPtr<u8>) = thiscall 0x6F5080 => unported;
    /// `CVehicleAnimGroup::CVehicleAnimGroup()` (0x5BD420)
    fn hd_anim_group_ctor(g: GPtr<u8>) -> GPtr<u8> = thiscall 0x5BD420 => unported;
    /// `CVehicleAnimGroup::InitAnimGroup(firstGroup, secondGroup, flags, animType, genTimes*, startTimes*, endTimes*)` (0x5B9EB0)
    fn hd_init_anim_group(g: GPtr<u8>, a: u32, b: u32, flags: u32, ty: u32, t1: GPtr<u8>, t2: GPtr<u8>, t3: GPtr<u8>) = thiscall 0x5B9EB0 => unported;
    /// `CVehicleAnimGroup::CopyAnimGroup(const CVehicleAnimGroup*)` (0x5B9E40)
    fn hd_copy_anim_group(g: GPtr<u8>, src: GPtr<u8>) = thiscall 0x5B9E40 => unported;
}

/// `fstp dword` of an `atof` result (ST0 holds the CRT's double): one rounding to f32.
fn atof32(s: GPtr<u8>) -> f32 {
    hd_atof(s) as f32
}

/// `cHandlingDataMgr::LoadHandlingData()` (0x5BD830, thiscall). `data\handling.cfg` line by line
/// (`CFileLoader::LoadLine`) until a line equal to ";the end" (9-byte `repe cmpsb` with the terminator)
/// or the end; lines starting with ';' are skipped. Each other line is split with `strtok(" \t")` (the
/// delimiters copied into a local per section) and dispatched on its first character:
/// - `!` bikes: token 1 = name → `FindExactWord(name, VehicleNames, 14, 210)`, record `this + 0x8F54 +
///   0x40 * id` (+0 := id), tokens 2..16 floats at +4..+0x3C; then `ConvertBikeDataToGameUnits(record)`.
/// - `$` flying: record `this + 0x7B24 + 0x58 * id`, tokens 2..22 floats at +4..+0x54 (no conversion).
/// - `%` boats: record `GetBoatPointer(id)`, tokens 2..15 floats at +4..+0x1C, +0x24..+0x38 and the last
///   at +0x20.
/// - `^` animation groups: tokens 1..3 `atol` (group id, first and second group), 4..21 flags `|= 2^k`
///   when non-zero (`FUN_00470000(2.0, k)` = CRT pow of floats then `_ftol`; exact powers of two), 22..34
///   floats into three time arrays, 35 `atol` (anim type); then a local `CVehicleAnimGroup` is constructed,
///   `InitAnimGroup(first + 0x58, second + 0x58, flags, type, gen[5], start[4], end[4])` (the `+ 0x58` byte
///   adds) and copied to `0xC1CDC0 + 0x94 * id`.
/// - anything else, cars: token 0 = name, record `this + 0x14 + 0xE0 * id`, tokens 1..35 into the
///   handling entry (floats, `atol` bytes/dword, the transmission chars, a `!= 0` bool, `%x` model / handling
///   flags, +0x7C = `atof * 0.4` at PC=24); then `ConvertDataToGameUnits(record)`.
/// The current record of each section (bike, flying, boat in stack slots, car in EBP) and the
/// animation-group locals persist across lines as in the original (a section line without a name token
/// writes into the previous record; zero/NULL before the first, as the original initialises them).
#[hook(addr = 0x5BD830, conv = thiscall, name = "cHandlingDataMgr::LoadHandlingData", expect = "81 EC FC 00 00 00 55 56",
       unverified = "std4-data-1", route = "L2", hazards = "uninit-locals", recs = "boot")]
pub fn load_handling_data_port(this: GPtr<u8>) {
    hd_set_dir(g::<u8>(0x86A784));
    let file = hd_open_file(g::<u8>(0x86A964), g::<u8>(0x859C6C));
    hd_set_dir(g::<u8>(0x858B54));
    let mut bike = GPtr::<u8>::NULL;
    let mut flying = GPtr::<u8>::NULL;
    let mut boat = GPtr::<u8>::NULL;
    let mut car = GPtr::<u8>::NULL;
    // the animation section's locals (frame offsets 0x38..0x10C of the original)
    let mut frame = [0u8; 0x110];
    let fb = frame.as_mut_ptr();
    let fp = |o: usize| GPtr::<u8>::from_raw(fb.wrapping_add(o));
    let names = g::<u8>(VEHICLE_NAMES);
    loop {
        let line = hd_load_line(file);
        if line.is_null() {
            break;
        }
        if (0..9).all(|i| line.byte_add(i).read() == g::<u8>(0x86A958).byte_add(i).read()) {
            break;
        }
        let c = line.read();
        if c == b';' {
            continue;
        }
        let mut delims = [0u8; 4];
        delims[0] = g::<u8>(0x85A510).read();
        delims[1] = g::<u8>(0x85A511).read();
        delims[2] = g::<u8>(0x85A512).read();
        let dp = GPtr::<u8>::from_raw(delims.as_mut_ptr());
        let mut tok = hd_strtok(line, dp);
        let mut k: u8 = 0;
        match c {
            b'!' => {
                while !tok.is_null() {
                    match k.wrapping_sub(1) {
                        0 => {
                            let id = hd_find_exact_word(this, tok, names, 0xE, 0xD2);
                            bike = this.byte_add((id << 6) as isize + 0x8F54);
                            wr::<u32>(bike, 0, id);
                        }
                        n @ 1..=0xF => wr::<f32>(bike, 4 * n as isize, atof32(tok)),
                        _ => {}
                    }
                    tok = hd_strtok(GPtr::NULL, dp);
                    k = k.wrapping_add(1);
                }
                hd_convert_bike_data(this, bike);
            }
            b'$' => {
                while !tok.is_null() {
                    match k.wrapping_sub(1) {
                        0 => {
                            let id = hd_find_exact_word(this, tok, names, 0xE, 0xD2);
                            flying = this.byte_add(id.wrapping_mul(0x58) as isize + 0x7B24);
                            wr::<u32>(flying, 0, id);
                        }
                        n @ 1..=0x15 => wr::<f32>(flying, 4 * n as isize, atof32(tok)),
                        _ => {}
                    }
                    tok = hd_strtok(GPtr::NULL, dp);
                    k = k.wrapping_add(1);
                }
            }
            b'%' => {
                while !tok.is_null() {
                    match k.wrapping_sub(1) {
                        0 => {
                            let id = hd_find_exact_word(this, tok, names, 0xE, 0xD2);
                            boat = hd_get_boat_pointer(this, id);
                            wr::<u32>(boat, 0, id);
                        }
                        n @ 1..=7 => wr::<f32>(boat, 4 * n as isize, atof32(tok)),
                        n @ 8..=0xD => wr::<f32>(boat, 4 * (n as isize + 1), atof32(tok)),
                        0xE => wr::<f32>(boat, 0x20, atof32(tok)),
                        _ => {}
                    }
                    tok = hd_strtok(GPtr::NULL, dp);
                    k = k.wrapping_add(1);
                }
            }
            b'^' => {
                let mut flags: u32 = 0;
                let mut anim_type: u32 = 0;
                // float k -> frame offset (the original's [esp + N] minus the pushed argument)
                const FLOATS: [usize; 13] = [0x64, 0x68, 0x6C, 0x70, 0x74, 0x54, 0x44, 0x58, 0x48, 0x5C, 0x4C, 0x60, 0x50];
                while !tok.is_null() {
                    match k.wrapping_sub(1) {
                        0 => fp(0x40).cast::<i32>().write(hd_atol(tok)),
                        1 => fp(0x38).cast::<i32>().write(hd_atol(tok)),
                        2 => fp(0x3C).cast::<i32>().write(hd_atol(tok)),
                        n @ 3..=0x14 => {
                            if hd_atol(tok) != 0 {
                                flags |= 1u32 << (n - 3);
                            }
                        }
                        n @ 0x15..=0x21 => fp(FLOATS[(n - 0x15) as usize]).cast::<f32>().write(atof32(tok)),
                        0x22 => anim_type = hd_atol(tok) as u32,
                        _ => {}
                    }
                    tok = hd_strtok(GPtr::NULL, dp);
                    k = k.wrapping_add(1);
                }
                let group = fp(0x78);
                hd_anim_group_ctor(group);
                let first = u32::from(fp(0x38).read().wrapping_add(0x58));
                let second = u32::from(fp(0x3C).read().wrapping_add(0x58));
                hd_init_anim_group(group, first, second, flags, anim_type, fp(0x64), fp(0x54), fp(0x44));
                let id = fp(0x40).cast::<u32>().read();
                hd_copy_anim_group(GPtr::from_addr((0xC1CDC0u32.wrapping_add(id.wrapping_mul(0x94))) as usize), group);
            }
            _ => {
                while !tok.is_null() {
                    let f = |o: isize| wr::<f32>(car, o, atof32(tok));
                    match k {
                        0 => {
                            let id = hd_find_exact_word(this, tok, names, 0xE, 0xD2);
                            car = this.byte_add(id.wrapping_mul(0xE0) as isize + 0x14);
                            wr::<u32>(car, 0, id);
                        }
                        1 => f(4),
                        n @ 2..=6 => f(4 * n as isize + 4),
                        7 => wr::<u8>(car, 0x20, hd_atol(tok) as u8),
                        8 => f(0x28),
                        9 => f(0xA4),
                        10 => f(0xA8),
                        11 => wr::<u8>(car, 0x76, hd_atol(tok) as u8),
                        12 => f(0x84),
                        13 => {
                            let v = X87::from_f64_exact(hd_atof(tok)).fmul(X87::from_f64_exact(g::<f64>(0x86A950).read()));
                            wr::<f32>(car, 0x7C, v.to_f32());
                        }
                        14 => f(0x80),
                        15 => wr::<u8>(car, 0x74, tok.read()),
                        16 => wr::<u8>(car, 0x75, tok.read()),
                        17 => f(0x94),
                        18 => f(0x98),
                        19 => wr::<u8>(car, 0x9C, u8::from(hd_atol(tok) != 0)),
                        20 => f(0xA0),
                        n @ 21..=27 => f(0xAC + 4 * (n as isize - 21)),
                        28 => f(0xD4),
                        29 => f(0xC8),
                        30 => wr::<i32>(car, 0xD8, hd_atol(tok)),
                        31 => {
                            hd_sscanf_x(tok, g::<u8>(0x86A94C), car.byte_add(0xCC));
                        }
                        32 => {
                            hd_sscanf_x(tok, g::<u8>(0x86A94C), car.byte_add(0xD0));
                            wr::<u32>(car, 0x78, rd::<u32>(car, 0xD0));
                        }
                        33 => wr::<u8>(car, 0xDC, hd_atol(tok) as u8),
                        34 => wr::<u8>(car, 0xDD, hd_atol(tok) as u8),
                        35 => wr::<u8>(car, 0xDE, hd_atol(tok) as u8),
                        _ => {}
                    }
                    tok = hd_strtok(GPtr::NULL, dp);
                    k = k.wrapping_add(1);
                }
                hd_convert_data(this, car);
            }
        }
    }
    hd_close_file(file);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_LOAD_HANDLING_DATA_PORT,
    HOOK_HAS_FRONT_WHEEL_DRIVE,
    HOOK_HAS_REAR_WHEEL_DRIVE,
    HOOK_GET_BOAT_POINTER,
    HOOK_GET_FLYING_POINTER,
    HOOK_CONVERT_BIKE_DATA_TO_GAME_UNITS,
    HOOK_GET_HANDLING_ID,
    HOOK_FIND_EXACT_WORD,
    HOOK_CONVERT_DATA_TO_WORLD_UNITS,
    HOOK_INITIALISE,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
