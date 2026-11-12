//! Script helpers on objects of the loading path (batch `mvpi-script-1`): `CTheScripts::
//! CleanUpThisObject` and `CTheScripts::ScriptConnectLodsFunction`. Ported from the 1.0 US
//! `.HOODLUM` bodies behind the stubs (the time read at 0x156B8AF is a pushfd/popfd-obfuscated
//! `mov ecx, [0xB7CB84]`); unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// `CPools::ms_pObjectPool` (0xB7449C): objects +0, flag bytes +4; objects are 0x19C bytes.
const OBJECT_POOL: usize = 0xB7449C;
/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIME_MS: usize = 0xB7CB84;
/// `CObject::nNoTempObjects` (0xBB4A70, int16)
const NO_TEMP_OBJECTS: usize = 0xBB4A70;

orig! {
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CEntity::SetupBigBuilding()` (0x533150)
    fn setup_big_building(e: GPtr<u8>) = thiscall 0x533150 => unported;
}

fn at<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> GPtr<T> {
    p.byte_add(off).cast::<T>()
}

fn ga<T: sa_core::mem::GameData>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}

/// `CTheScripts::CleanUpThisObject(CObject*)` (stub 0x4866C0 → 0x156B890, cdecl): a non-null object
/// created by a mission (+0x13C 2) or as a mission brain object (6) becomes temporary (3), its
/// removal time +0x150 = now + 20 000 000 ms, object flag 0x80 (+0x140) cleared, +0x14A = -1, and
/// `CObject::nNoTempObjects` (0xBB4A70) incremented (stores in the bytes' order).
#[hook(addr = 0x4866c0, conv = cdecl, name = "CTheScripts::CleanUpThisObject", expect = "E9 CB 51 0E 01",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn clean_up_this_object(obj: GPtr<u8>) {
    if obj.is_null() {
        return;
    }
    let created = at::<u8>(obj, 0x13C).read();
    if created != 2 && created != 6 {
        return;
    }
    at::<u8>(obj, 0x13C).write(3);
    at::<u32>(obj, 0x150).write(ga::<u32>(TIME_MS).read().wrapping_add(0x1312D00));
    let fl = at::<u32>(obj, 0x140).read() & 0xFFFF_FF7F;
    at::<u16>(obj, 0x14A).write(0xFFFF);
    at::<u32>(obj, 0x140).write(fl);
    let n = ga::<u16>(NO_TEMP_OBJECTS);
    n.write(n.read().wrapping_add(1));
}

/// The object of pool handle `h` (`h >> 8` arithmetic, its flag byte equal to `h`'s low byte), else
/// null.
fn object_at(pool: GPtr<u8>, flags: GPtr<u8>, h: i32) -> GPtr<u8> {
    let i = h >> 8;
    if at::<u8>(flags, i as isize).read() == h as u8 {
        at::<GPtr<u8>>(pool, 0).read().byte_add(0x19C * i as isize)
    } else {
        GPtr::NULL
    }
}

/// `CTheScripts::ScriptConnectLodsFunction(int32 hObject, int32 hLod)` (stub 0x470A20 → 0x156D9F0,
/// cdecl): both handles through the object pool (pool and flag array read once; an invalid handle
/// gives null, which the original then dereferences); `object->m_pLod` (+0x30) = lod, the lod's
/// child count (+0x34) incremented, then `CWorld::Remove(lod)`, `lod->SetupBigBuilding()`,
/// `CWorld::Add(lod)`.
#[hook(addr = 0x470a20, conv = cdecl, name = "CTheScripts::ScriptConnectLodsFunction", expect = "E9 CB CF 0F 01",
       unverified = "mvpi-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn script_connect_lods_function(h_object: i32, h_lod: i32) {
    let pool = ga::<GPtr<u8>>(OBJECT_POOL).read();
    let flags = at::<GPtr<u8>>(pool, 4).read();
    let obj = object_at(pool, flags, h_object);
    let lod = object_at(pool, flags, h_lod);
    at::<GPtr<u8>>(obj, 0x30).write(lod);
    let c = at::<u8>(lod, 0x34).read().wrapping_add(1);
    at::<u8>(lod, 0x34).write(c);
    world_remove(lod);
    setup_big_building(lod);
    world_add(lod);
}

pub const HOOKS: &[HookReg] = &[HOOK_CLEAN_UP_THIS_OBJECT, HOOK_SCRIPT_CONNECT_LODS_FUNCTION];
