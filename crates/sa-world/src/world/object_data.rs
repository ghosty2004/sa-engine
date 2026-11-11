//! STD-5: `CObjectData::Initialise(const char* fileName)` (0x5B5360), the `object.dat` loader, ported from
//! the 1.0 US bytes (gta-reversed `ObjectData.cpp` as the reading aid; the bytes win: the buoyancy is
//! `100 / submerged * mass * (1/125)` in that order, the default-entry shortcut maps camera-avoid 0 to
//! entries 1 / 3, and the duplicate search compares the fields one by one with the sparks flag as a byte).
//!
//! `CObjectData` (0x50, `ms_aObjectInfo` 0xBB4A90): +0 mass, +4 turn mass, +8 air resistance, +0xC
//! elasticity, +0x10 buoyancy, +0x14 uproot limit, +0x18 collision damage multiplier, bytes +0x1C damage
//! effect, +0x1D special response, +0x1E camera avoid, +0x1F causes explosion, +0x20 fx type, +0x24 fx
//! offset, +0x30 fx system, +0x34 smash multiplier, +0x38 break velocity, +0x44 break velocity random,
//! +0x48 gun break mode, +0x4C sparks on impact.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    fn file_set_dir(dir: u32) = cdecl 0x5387D0 => unported;
    /// `CFileMgr::OpenFile(const char*, const char*)` (0x538900)
    fn file_open(path: GPtr<u8>, mode: u32) -> GPtr<u8> = cdecl 0x538900 => unported;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    fn file_close(f: GPtr<u8>) -> u32 = cdecl 0x5389D0 => unported;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    fn load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// CRT `sscanf` (0x8220AD) with the 24 conversions of the object.dat format
    #[allow(clippy::too_many_arguments)]
    fn sscanf24(s: GPtr<u8>, fmt: u32, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32, a17: u32, a18: u32, a19: u32, a20: u32, a21: u32, a22: u32, a23: u32) -> i32 = cdecl 0x8220AD => unported;
    /// `FxManager_c::FindFxSystemBP(const char*)` (0x4A9360)
    fn find_fx_system_bp(this: GPtr<u8>, name: GPtr<u8>) -> u32 = thiscall 0x4A9360 => unported;
    /// `CModelInfo::GetModelInfo(const char*, int32*)` (0x4C5940)
    fn get_model_info(name: GPtr<u8>, index: u32) -> GPtr<u8> = cdecl 0x4C5940 => unported;
}

/// `CObjectData::ms_aObjectInfo`.
const OBJECT_INFO: usize = 0xBB4A90;

fn at(i: u32) -> GPtr<u8> {
    GPtr::from_addr(OBJECT_INFO + 0x50 * i as usize)
}
fn f(p: GPtr<u8>, o: isize) -> f32 {
    p.byte_add(o).cast::<f32>().read()
}
fn b(p: GPtr<u8>, o: isize) -> u8 {
    p.byte_add(o).read()
}
fn w32(p: GPtr<u8>, o: isize, v: u32) {
    p.byte_add(o).cast::<u32>().write(v)
}
fn ptr(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// The field-by-field comparison of the duplicate search (f32 `==` for the floats, NaN unequal).
fn same(a: GPtr<u8>, c: GPtr<u8>) -> bool {
    let fe = |o: isize| f(a, o) == f(c, o);
    let be = |o: isize| b(a, o) == b(c, o);
    let de = |o: isize| a.byte_add(o).cast::<u32>().read() == c.byte_add(o).cast::<u32>().read();
    [0, 4, 8, 0xC, 0x10, 0x14, 0x18].into_iter().all(fe)
        && [0x1C, 0x1D, 0x1E, 0x1F, 0x20].into_iter().all(be)
        && [0x24, 0x28, 0x2C].into_iter().all(fe)
        && de(0x30)
        && [0x34, 0x38, 0x3C, 0x40, 0x44].into_iter().all(fe)
        && de(0x48)
        && be(0x4C)
}

/// `CObjectData::Initialise(const char*)` (0x5B5360): the five default entries (0: mass / turn mass
/// 99999, air resistance 0.99, elasticity 0.1, buoyancy 1599.9841, uproot 0, damage multiplier 1, camera
/// avoid 2; 1 = 0 with camera avoid 0; 2 = 0 with special response 4; 3 = 2 with camera avoid 0; 4 = 0
/// with special response 5), then per line of the file (`SetDir("")`, mode "rb"; ';' and empty lines
/// skipped, '*' ends) the next free entry is zeroed and `sscanf`'d (format 0x868DC8), the fx system looked
/// up when the fx type is > 0, the buoyancy and the byte fields stored; a known model gets entry 1 / 0
/// (default physics, response 0, camera avoid 0 / other), 3 / 2 (response 4), an equal earlier entry, or
/// the new entry (which then stays used).
#[hook(addr = 0x5B5360, conv = cdecl, name = "CObjectData::Initialise", expect = "81 EC 2C 02 00 00 53 56 57 C7",
       unverified = "std5-objdata-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn initialise(file_name: GPtr<u8>) {
    let d0 = at(0);
    for (o, v) in [(0, 0x47C3_4F80u32), (4, 0x47C3_4F80), (8, 0x3F7D_70A4), (0xC, 0x3DCC_CCCD), (0x10, 0x44C7_FF7E), (0x14, 0), (0x18, 0x3F80_0000)] {
        w32(d0, o, v);
    }
    d0.byte_add(0x1C).write(0);
    d0.byte_add(0x1D).write(0);
    d0.byte_add(0x1E).write(2);
    let copy0 = |i: u32| {
        for k in 0..0x14isize {
            at(i).byte_add(4 * k).cast::<u32>().write(d0.byte_add(4 * k).cast::<u32>().read());
        }
    };
    copy0(1);
    at(1).byte_add(0x1E).write(0);
    copy0(2);
    at(2).byte_add(0x1D).write(4);
    copy0(3);
    at(3).byte_add(0x1D).write(4);
    at(3).byte_add(0x1E).write(0);
    copy0(4);
    at(4).byte_add(0x1D).write(5);
    file_set_dir(0x858B54);
    let file = file_open(file_name, 0x859C6C);
    let mut count = 5u32;
    let mut e = at(5);
    let mut model_name = [0u8; 0x100];
    let mut effect_name = [0u8; 0x100];
    let mut ints = [0u32; 6]; // submerged %, damage effect, special response, camera avoid, explosion, fx type
    let mut line = load_line(file);
    while !line.is_null() {
        let c = line.read();
        if c == b'*' {
            break;
        }
        if c != b';' && c != 0 {
            for k in 0..0x14isize {
                e.byte_add(4 * k).cast::<u32>().write(0);
            }
            let p = |o: isize| ptr(e) + o as u32;
            let base = ints.as_mut_ptr();
            let lp = |i: usize| ptr(GPtr::<u32>::from_raw(base.wrapping_add(i)).cast());
            sscanf24(
                line,
                0x868DC8,
                ptr(GPtr::from_raw(model_name.as_mut_ptr())),
                p(0),
                p(4),
                p(8),
                p(0xC),
                lp(0),
                p(0x14),
                p(0x18),
                lp(1),
                lp(2),
                lp(3),
                lp(4),
                lp(5),
                p(0x24),
                p(0x28),
                p(0x2C),
                ptr(GPtr::from_raw(effect_name.as_mut_ptr())),
                p(0x34),
                p(0x38),
                p(0x3C),
                p(0x40),
                p(0x44),
                p(0x48),
                p(0x4C),
            );
            w32(e, 0x30, 0);
            e.byte_add(0x20).write(ints[5] as u8);
            if (ints[5] as i32) > 0 {
                w32(e, 0x30, find_fx_system_bp(GPtr::from_addr(0xA9AE80), GPtr::from_raw(effect_name.as_mut_ptr())));
            }
            let hundred = GPtr::<f32>::from_addr(0x858628).read();
            let k = GPtr::<f32>::from_addr(0x863984).read();
            let buoy = X87::from_f32(hundred).fdiv_m(f32::from_bits(ints[0])).fmul_m(f(e, 0)).fmul_m(k);
            e.byte_add(0x1D).write(ints[2] as u8);
            e.byte_add(0x1C).write(ints[1] as u8);
            e.byte_add(0x1E).write(ints[3] as u8);
            e.byte_add(0x1F).write(ints[4] as u8);
            e.byte_add(0x10).cast::<f32>().write(buoy.to_f32());
            let mi = get_model_info(GPtr::from_raw(model_name.as_mut_ptr()), 0);
            if !mi.is_null() {
                let idx = mi.byte_add(0x10).cast::<u16>();
                let defaults = f(e, 0) == f(d0, 0) && f(e, 0x18) == f(d0, 0x18) && b(e, 0x1C) == b(d0, 0x1C);
                let special = b(e, 0x1D);
                if defaults && special == b(d0, 0x1D) {
                    idx.write(u16::from(b(e, 0x1E) == 0));
                } else if defaults && special == b(at(2), 0x1D) {
                    idx.write(if b(e, 0x1E) != 0 { 2 } else { 3 });
                } else {
                    let found = (0..count).find(|&i| same(e, at(i)));
                    match found {
                        Some(i) => idx.write(i as u16),
                        None => {
                            idx.write(count as u16);
                            count += 1;
                            e = e.byte_add(0x50);
                        }
                    }
                }
            }
        }
        line = load_line(file);
    }
    file_close(file);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_INITIALISE];
