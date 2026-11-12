//! SAVE-1: the script block of the save game (gta_sa.exe 1.0 US): `CTheScripts::Save` / `Load` and
//! `DoScriptSetupAfterPoolsHaveLoaded`. Ported from the bytes (objdump; gta-reversed `TheScripts.cpp` as a
//! reading aid); docs/arch/save-port.md.
//!
//! Layout (both directions): the main script's size (the GOTO operand at ScriptSpace +3), the script space in
//! 0xC800-byte chunks plus the rest, 70 x 0x14 at 0xA90CF0, two dwords, 25 "invisibility settings"
//! (type, pool index + 1, two dwords; buildings only), 20 "building swaps" (type 0 / 2 building / 3 object /
//! 4 dummy, pool index + 1), 20 dwords, 10 x 8 (LOD links), 8 x 0x14 (anim group attachments), two bytes, two
//! dwords, two words, a dword, then the active scripts that are neither external nor brain scripts (+0xC7 == 0,
//! +0xC9 == 0xFF): their count, then from the list's tail backwards each one's index (2), its 0xE0 bytes, its
//! base IP and 8 stack entries as script-space offsets.
//!
//! Original behaviour kept: `Load` reads the five trailing values (0xA444C4, 0xA444C0, 0xA444BC, 0xA444B8,
//! 0xA444B4 in `Save`) into locals and drops them; a building swap of an entity type other than building /
//! object / dummy writes nothing and saves the previous iteration's locals (the first time: the stack
//! residue at entry ESP -0x18, from the footprint snapshot); load types 1 and above 4 only decrement the
//! index local.

use sa_core::abi::{HookReg, footprint_residue_u8};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

const SCRIPT_SPACE: u32 = 0xA49960;
/// `CTheScripts::pActiveScripts`
const ACTIVE_SCRIPTS: usize = 0xA8B42C;
/// `CPools::ms_pBuildingPool` / `ms_pObjectPool` / `ms_pDummyPool`
const BUILDING_POOL: usize = 0xB74498;
const OBJECT_POOL: usize = 0xB7449C;
const DUMMY_POOL: usize = 0xB744A0;
const WINDOW: u32 = 0x1000;

fn g<T>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

orig! {
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
    fn save_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1270 => sa_core::save::save_data_to_work_buffer;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
    fn load_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1300 => sa_core::save::load_data_from_work_buffer;
    /// `CTheScripts::Init()` (0x468D50)
    fn the_scripts_init() = cdecl 0x468D50 => unported;
    /// `CTheScripts::GetScriptIndexFromPointer(CRunningScript*)` (0x464D20)
    fn get_script_index_from_pointer(s: GPtr<u8>) -> u32 = cdecl 0x464D20 => unported;
    /// `CTheScripts::StartNewScript(uint8* ip, uint16 index)` (0x464C90; the callee reads the low word)
    fn start_new_script(ip: u32, index: u32) -> GPtr<u8> = cdecl 0x464C90 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CBuilding::ReplaceWithNewModel(int32)` (0x403EC0)
    fn building_replace_with_new_model(e: GPtr<u8>, model: i32) = thiscall 0x403EC0 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CTheScripts::ScriptAttachAnimGroupToCharModel(int32 model, char* group)` (0x474800)
    fn script_attach_anim_group_to_char_model(model: i32, group: GPtr<u8>) -> u8 = cdecl 0x474800 => unported;
}

/// `(e - objects) / size + 1` as the bytes compute the pool index: a magic multiply (`imul m`, plus the
/// dividend when `add` is set), `sar 5`, rounded toward zero.
fn pool_index_plus_one(e: u32, objects: u32, m: u32, add: bool) -> u32 {
    let d = e.wrapping_sub(objects) as i32;
    let mut hi = ((i64::from(d) * i64::from(m as i32)) >> 32) as i32;
    if add {
        hi = hi.wrapping_add(d);
    }
    let q = hi >> 5;
    (q.wrapping_add((q as u32 >> 31) as i32) as u32).wrapping_add(1)
}

fn pool_objects(pool: usize) -> u32 {
    g::<GPtr<u32>>(pool).read().read()
}

/// The script space chunks of `Save` / `Load`: whole 0xC800 chunks while more than 0xC800 bytes are left
/// (`(size - 0xC801) / 0xC800 + 1` of them, unsigned), then the rest.
fn script_space(size: i32, io: fn(GPtr<u8>, i32) -> u8) {
    let mut off = 0u32;
    let mut rest = size;
    if size > 0xC800 {
        let n = (size as u32).wrapping_sub(0xC801) / 0xC800 + 1;
        rest = size.wrapping_sub((n.wrapping_mul(0xC800)) as i32);
        for _ in 0..n {
            io(g((SCRIPT_SPACE + off) as usize), 0xC800);
            off += 0xC800;
        }
    }
    io(g((SCRIPT_SPACE + off) as usize), rest);
}

/// `CTheScripts::Save()` (0x5D4C40): see the module doc. The locals (dwords at entry ESP -0x1C .. -4) start
/// as the footprint snapshot's residue. AL = 1.
#[hook(addr = 0x5D4C40, conv = cdecl, name = "CTheScripts::Save", expect = "83 EC 1C A1 63 99 A4 00",
       route = "split", hazards = "uninit-locals", recs = "save-load", footprint = 0x1000)]
pub fn the_scripts_save() -> u8 {
    // locals: [0] type (-0x1C), [1] swap index (-0x18), [2] IP offset (-0x14), [3] count (-0x10),
    // [4] size (-0xC), [5] invisibility index (-8), [6] script index (-4)
    let mut l = [0u32; 7];
    for (k, v) in l.iter_mut().enumerate() {
        let off = -0x1C + 4 * k as i32;
        *v = u32::from_le_bytes([0, 1, 2, 3].map(|b| footprint_residue_u8(off + b, WINDOW)));
    }
    let lp = GPtr::<u32>::from_raw(l.as_mut_ptr());
    let loc = |k: isize| lp.byte_add(4 * k);
    let at = |k: isize| loc(k).cast::<u8>();
    loc(4).write(g::<u32>(SCRIPT_SPACE as usize + 3).read());
    save_data(at(4), 4);
    script_space(loc(4).read() as i32, save_data);
    for k in 0..70usize {
        save_data(g(0xA90CF0 + 0x14 * k), 0x14);
    }
    save_data(g(0xA476AC), 4);
    save_data(g(0xA476A8), 4);
    for k in 0..25usize {
        let e = g::<u32>(0xA44A30 + 0xC * k).read();
        if e != 0 {
            loc(0).write(2);
            loc(5).write(pool_index_plus_one(e, pool_objects(BUILDING_POOL), 0x9249_2493, true));
        } else {
            loc(0).write(0);
            loc(5).write(0);
        }
        save_data(at(0), 4);
        save_data(at(5), 4);
        save_data(g(0xA44A34 + 0xC * k), 4);
        save_data(g(0xA44A38 + 0xC * k), 4);
    }
    for k in 0..20usize {
        let e = g::<u32>(0xA449E0 + 4 * k).read();
        if e != 0 {
            match g::<u8>(e as usize + 0x36).read() & 7 {
                1 => {
                    loc(0).write(2);
                    loc(1).write(pool_index_plus_one(e, pool_objects(BUILDING_POOL), 0x9249_2493, true));
                }
                4 => {
                    loc(0).write(3);
                    loc(1).write(pool_index_plus_one(e, pool_objects(OBJECT_POOL), 0x13E2_2CBD, false));
                }
                5 => {
                    loc(0).write(4);
                    loc(1).write(pool_index_plus_one(e, pool_objects(DUMMY_POOL), 0x9249_2493, true));
                }
                _ => {}
            }
        } else {
            loc(0).write(0);
            loc(1).write(0);
        }
        save_data(at(0), 4);
        save_data(at(1), 4);
    }
    for k in 0..20usize {
        save_data(g(0xA448F0 + 4 * k), 4);
    }
    for k in 0..10usize {
        save_data(g(0xA44800 + 8 * k), 8);
    }
    for k in 0..8usize {
        save_data(g(0xA44850 + 0x14 * k), 0x14);
    }
    for (a, n) in [(0xA447E8usize, 1), (0xA444A6, 1), (0xA444C4, 4), (0xA444C0, 4), (0xA444BC, 2), (0xA444B8, 2), (0xA444B4, 4)] {
        save_data(g(a), n);
    }
    let saved = |s: GPtr<u8>| s.byte_add(0xC7).read() == 0 && s.byte_add(0xC9).read() == 0xFF;
    loc(3).write(0);
    let mut s = g::<GPtr<u8>>(ACTIVE_SCRIPTS).read();
    while !s.is_null() {
        if saved(s) {
            loc(3).write(loc(3).read().wrapping_add(1));
        }
        s = s.cast::<GPtr<u8>>().read();
    }
    save_data(at(3), 4);
    let mut s = g::<GPtr<u8>>(ACTIVE_SCRIPTS).read();
    loop {
        let next = s.cast::<GPtr<u8>>().read();
        if next.is_null() {
            break;
        }
        s = next;
    }
    loop {
        if saved(s) {
            loc(6).write(get_script_index_from_pointer(s));
            save_data(at(6), 2);
            save_data(s, 0xE0);
            loc(2).write(s.byte_add(0x14).cast::<u32>().read().wrapping_sub(SCRIPT_SPACE));
            save_data(at(2), 4);
            for k in 0..8isize {
                let v = s.byte_add(0x18 + 4 * k).cast::<u32>().read();
                loc(2).write(if v == 0 { 0 } else { v.wrapping_sub(SCRIPT_SPACE) });
                save_data(at(2), 4);
            }
        }
        s = s.byte_add(4).cast::<GPtr<u8>>().read();
        if s.is_null() {
            break;
        }
    }
    1
}

/// The entity of a saved pool index: the index local is decremented first; a free slot (flag bit 7) gives
/// null.
fn pool_entity(pool: usize, idx: &mut u32, stride: u32) -> u32 {
    *idx = idx.wrapping_sub(1);
    let p = g::<GPtr<u8>>(pool).read();
    let flags = p.byte_add(4).cast::<GPtr<u8>>().read();
    if flags.byte_add(*idx as i32 as isize).read() & 0x80 != 0 {
        return 0;
    }
    idx.wrapping_mul(stride).wrapping_add(p.cast::<u32>().read())
}

/// `CTheScripts::Load()` (0x5D4FD0): `CTheScripts::Init()`, then the layout of `Save` read back (see the module
/// doc): the invisibility settings re-resolve buildings and `CWorld::Remove` / `ReplaceWithNewModel(model)` /
/// `CWorld::Add` them; the building swaps clear bits 7 and 0 of +0x1C of their entity; each anim group
/// attachment with a model calls `ScriptAttachAnimGroupToCharModel`; each saved script is started with
/// `StartNewScript(nullptr, index)`, its 0xE0 bytes read over it (the list links kept) and its IPs rebased.
/// EAX = the saved script count (the last value the bytes load into EAX; split run save1: AL 0x1C, not 1).
#[hook(addr = 0x5D4FD0, conv = cdecl, name = "CTheScripts::Load", expect = "83 EC 24 53 55 56 57",
       route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn the_scripts_load() -> u32 {
    let mut size = 0i32;
    let mut ty = 0u32;
    let mut idx = 0u32;
    let mut word = 0u32;
    let mut dropped = [0u32; 2];
    let p8 = |v: &mut u32| GPtr::<u8>::from_raw((v as *mut u32).cast());
    the_scripts_init();
    load_data(GPtr::from_raw((&mut size as *mut i32).cast()), 4);
    script_space(size, load_data);
    for k in 0..70usize {
        load_data(g(0xA90CF0 + 0x14 * k), 0x14);
    }
    load_data(g(0xA476AC), 4);
    load_data(g(0xA476A8), 4);
    for k in 0..25usize {
        load_data(p8(&mut ty), 4);
        load_data(p8(&mut idx), 4);
        let model = g::<u8>(0xA44A34 + 0xC * k);
        load_data(model, 4);
        load_data(g(0xA44A38 + 0xC * k), 4);
        let slot = g::<u32>(0xA44A30 + 0xC * k);
        match ty {
            0 => slot.write(0),
            1 => idx = idx.wrapping_sub(1),
            2 => slot.write(pool_entity(BUILDING_POOL, &mut idx, 0x38)),
            _ => {}
        }
        let e = slot.read();
        if e != 0 {
            world_remove(g(e as usize));
            building_replace_with_new_model(g(e as usize), model.cast::<i32>().read());
            world_add(g(e as usize));
        }
    }
    for k in 0..20usize {
        load_data(p8(&mut ty), 4);
        load_data(p8(&mut idx), 4);
        let slot = g::<u32>(0xA449E0 + 4 * k);
        match ty {
            0 => slot.write(0),
            1 => idx = idx.wrapping_sub(1),
            2 => slot.write(pool_entity(BUILDING_POOL, &mut idx, 0x38)),
            3 => slot.write(pool_entity(OBJECT_POOL, &mut idx, 0x19C)),
            4 => slot.write(pool_entity(DUMMY_POOL, &mut idx, 0x38)),
            _ => {}
        }
        if slot.read() != 0 {
            let f = g::<u32>(slot.read() as usize + 0x1C);
            f.write(f.read() & 0xFFFF_FF7F);
            let f = g::<u32>(slot.read() as usize + 0x1C);
            f.write(f.read() & 0xFFFF_FFFE);
        }
    }
    for k in 0..20usize {
        load_data(g(0xA448F0 + 4 * k), 4);
    }
    for k in 0..10usize {
        load_data(g(0xA44800 + 8 * k), 8);
    }
    for k in 0..8usize {
        let a = g::<u8>(0xA44850 + 0x14 * k);
        load_data(a, 0x14);
        let model = a.cast::<i32>().read();
        if model != -1 {
            script_attach_anim_group_to_char_model(model, a.byte_add(4));
        }
    }
    load_data(g(0xA447E8), 1);
    load_data(g(0xA444A6), 1);
    let dp = GPtr::<u8>::from_raw(dropped.as_mut_ptr().cast());
    load_data(dp, 4);
    load_data(dp, 4);
    load_data(dp.byte_add(4), 2);
    load_data(dp.byte_add(4), 2);
    load_data(dp, 4);
    let mut count = 0u32;
    let mut s = g::<GPtr<u8>>(ACTIVE_SCRIPTS).read();
    while !s.is_null() {
        count = count.wrapping_add(1);
        s = s.cast::<GPtr<u8>>().read();
    }
    load_data(GPtr::from_raw((&mut count as *mut u32).cast()), 4);
    let mut i = 0u32;
    while i < count {
        load_data(p8(&mut word), 2);
        let s = start_new_script(0, word);
        let (next, prev) = (s.cast::<u32>().read(), s.byte_add(4).cast::<u32>().read());
        load_data(s, 0xE0);
        s.cast::<u32>().write(next);
        s.byte_add(4).cast::<u32>().write(prev);
        load_data(p8(&mut idx), 4);
        s.byte_add(0x14).cast::<u32>().write(idx.wrapping_add(SCRIPT_SPACE));
        for k in 0..8isize {
            load_data(p8(&mut idx), 4);
            s.byte_add(0x18 + 4 * k).cast::<u32>().write(if idx == 0 { 0 } else { idx.wrapping_add(SCRIPT_SPACE) });
        }
        i += 1;
    }
    count
}

pub const HOOKS: &[HookReg] = &[HOOK_THE_SCRIPTS_SAVE, HOOK_THE_SCRIPTS_LOAD];

#[cfg(test)]
mod tests {
    use super::pool_index_plus_one;

    #[test]
    fn pool_indices() {
        for k in 0..2000u32 {
            assert_eq!(pool_index_plus_one(0x1000_0000 + k * 0x38, 0x1000_0000, 0x9249_2493, true), k + 1);
            assert_eq!(pool_index_plus_one(0x1000_0000 + k * 0x19C, 0x1000_0000, 0x13E2_2CBD, false), k + 1);
        }
    }
}
