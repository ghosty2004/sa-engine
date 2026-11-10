//! `CEntryExit` / `CEntryExitManager` leaves (PORT-WORLD). Ported from the 1.0 US bytes (gta-reversed
//! `EntryExit.cpp`, `EntryExitManager.cpp`, `docs/re/specs/WORLD/CEntryExit*.md` as reading aids).
//!
//! `CEntryExit` (0x3C): +0 name (char[8]), +8 entrance rect (left, bottom, right, top), +0x18 entrance z,
//! +0x1C entrance angle, +0x20 exit position, +0x2C exit angle, +0x30 flags (u16), +0x32 area code, +0x34
//! time on, +0x35 time off, +0x38 linked entry-exit. They live in the pool `mp_poolEntryExits`
//! (0x96A7D8; `CPool`: +0 objects, +4 byte map, +8 size).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `CEntryExitManager::ms_entryExitStack` (`CEntryExit*[4]`; the code addresses entry `n - 1` as
    /// `0x96A71C + 4n`)
    pub static ENTRY_EXIT_STACK @ 0x96A720: [GPtr<u8>; 4] = [GPtr::NULL; 4];
    /// `CEntryExitManager::ms_entryExitStackPosn`
    pub static ENTRY_EXIT_STACK_POSN @ 0x96A7C4: i32 = 0;
    /// `CEntryExitManager::ms_exitEnterState`
    pub static EXIT_ENTER_STATE @ 0x96A7CC: i32 = 0;
    /// `CEntryExitManager::mp_poolEntryExits`
    pub static POOL_ENTRY_EXITS @ 0x96A7D8: GPtr<u8> = GPtr::NULL;
}

orig! {
    /// `CWorld::TestSphereAgainstWorld(CVector, float, CEntity*, bool x5, bool)` (0x569E20)
    pub fn test_sphere_against_world(px: u32, py: u32, pz: u32, r: u32, ignore: GPtr<u8>, b0: i32, b1: i32, b2: i32, b3: i32, b4: i32, b5: i32) -> GPtr<u8>
        = cdecl 0x569E20 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490)
    pub fn get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, b0: i32, b1: i32, b2: i32, b3: i32, b4: i32, b5: i32, b6: i32) -> u8
        = cdecl 0x56A490 => unported;
    /// CRT `_strnicmp` (0x835AC7)
    pub fn strnicmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x835AC7 => unported;
    /// `CEntryExitManager::GetEntryExitIndex(const char*, uint16 enabledFlags, uint16 disabledFlags)` (0x43EFD0)
    pub fn get_entry_exit_index(name: GPtr<u8>, en: u32, dis: u32) -> i32 = cdecl 0x43EFD0 => unported;
    /// `CEntryExit::GetPositionRelativeToOutsideWorld(CVector&)` (0x43EA00, thiscall)
    pub fn ee_get_position_relative_to_outside_world_call(this: GPtr<u8>, pos: GPtr<f32>) = thiscall 0x43EA00 => get_position_relative_to_outside_world;
    /// `CEntryExitManager::SetEntryExitFlag(const char*, uint16, bool)` (0x43EF20)
    pub fn set_entry_exit_flag_call(name: GPtr<u8>, flag: u32, set: u32) = cdecl 0x43EF20 => set_entry_exit_flag;
}

/// 0x858B8C: 0.5
const F0_5: f32 = 0.5;

/// Pool slot `i`, or null when the byte map marks it free (bit 7).
fn pool_at(i: i32) -> GPtr<u8> {
    let pool = POOL_ENTRY_EXITS.get();
    let map = rd::<GPtr<u8>>(pool, 4);
    if (map.byte_add(i as isize).read() as i8) < 0 {
        return GPtr::NULL;
    }
    rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x3C)
}

/// `CEntryExitManager::WeAreInInteriorTransition` (stub 0x43E400 → 0x1560F30): `ms_exitEnterState != 0`.
#[hook(addr = 0x43E400, conv = cdecl, name = "CEntryExitManager::WeAreInInteriorTransition", expect = "E9 2B 2B 12 01")]
pub fn we_are_in_interior_transition() -> u8 {
    (EXIT_ENTER_STATE.get() != 0) as u8
}

/// `CEntryExit::GetPositionRelativeToOutsideWorld(CVector&)` (0x43EA00, thiscall, `ret 4`): when the
/// entry-exit (or its link) has an area code, `pos += entrance centre (z: entrance z) - exit position`,
/// with the f32 roundings of the bytes. The original also stores the z sum into its own stack argument
/// slot (`fst [esp+0x10]`), which no caller reads; not reproduced.
#[hook(addr = 0x43EA00, conv = thiscall, name = "CEntryExit::GetPositionRelativeToOutsideWorld", preserve = "edx", expect = "83 EC 0C D9 41 10", this = "CEntryExit")]
pub fn get_position_relative_to_outside_world(this: GPtr<u8>, pos: GPtr<f32>) {
    let f = |p: GPtr<u8>, o: isize| rd::<f32>(p, o);
    let cx = x(f(this, 0x10)).fadd_m(f(this, 8)).fmul_m(F0_5);
    let cz = rd::<u32>(this, 0x18);
    let cy = x(f(this, 0x14)).fadd_m(f(this, 0xC)).fmul_m(F0_5).to_f32();
    let link = rd::<GPtr<u8>>(this, 0x38);
    let e = if link.is_null() { this } else { link };
    if rd::<u8>(e, 0x32) == 0 {
        return;
    }
    let sx = cx.fadd_m(pos.read());
    pos.write(sx.to_f32());
    let sy = x(cy).fadd_m(pos.wrapping_index(1).read());
    pos.wrapping_index(1).write(sy.to_f32());
    let sz = x(f32::from_bits(cz)).fadd_m(pos.wrapping_index(2).read()).to_f32();
    pos.wrapping_index(2).write(sz);
    let (ex, ey, ez) = (f(e, 0x20), x(f(e, 0x24)).to_f32(), x(f(e, 0x28)).to_f32());
    pos.write(sx.fsub(x(ex)).to_f32());
    pos.wrapping_index(1).write(sy.fsub_m(ey).to_f32());
    pos.wrapping_index(2).write(x(sz).fsub_m(ez).to_f32());
}

/// `IsTeleportPointValid(const CVector& from, const CVector& to)` (0x43EA90): no world collision in a
/// 0.35 sphere at `to` (`TestSphereAgainstWorld(to, 0.35, null, 1,1,1,1,1, 0)` returns null) and a clear
/// line of sight `from` → `to` (`GetIsLineOfSightClear(.., 1,1,0,1,1,0,0)`).
#[hook(addr = 0x43EA90, conv = cdecl, name = "IsTeleportPointValid", expect = "56 8B 74 24 0C")]
pub fn is_teleport_point_valid(from: GPtr<u8>, to: GPtr<u8>) -> u8 {
    let (a, b, c) = (rd::<u32>(to, 0), rd::<u32>(to, 4), rd::<u32>(to, 8));
    if !test_sphere_against_world(a, b, c, 0x3EB3_3333, GPtr::NULL, 0, 1, 1, 1, 1, 1).is_null() {
        return 0;
    }
    (get_is_line_of_sight_clear(from, to, 1, 1, 0, 1, 1, 0, 0) != 0) as u8
}

/// `CEntryExitManager::GetEntryExit(int32)` (stub 0x43EF00 → 0x156DA90): pool slot or null.
#[hook(addr = 0x43EF00, conv = cdecl, name = "CEntryExitManager::GetEntryExit", expect = "E9 8B EB 12 01")]
pub fn get_entry_exit(i: i32) -> GPtr<u8> {
    pool_at(i)
}

/// `CEntryExitManager::SetEntryExitFlag(const char* name, uint16 flag, bool set)` (stub 0x43EF20 →
/// 0x15615C0): for every used slot (pool size read once, the pool pointer re-read) whose name matches
/// (`_strnicmp(entry, name, 8)`), sets or clears the flag bits.
#[hook(addr = 0x43EF20, conv = cdecl, name = "CEntryExitManager::SetEntryExitFlag", expect = "E9 9B 26 12 01")]
pub fn set_entry_exit_flag(name: GPtr<u8>, flag: u32, set: u32) {
    let n = rd::<i32>(POOL_ENTRY_EXITS.get(), 8);
    let bits = flag as u16;
    let mut i = 0;
    while i < n {
        let pool = POOL_ENTRY_EXITS.get();
        let map = rd::<GPtr<u8>>(pool, 4);
        if (map.byte_add(i as isize).read() as i8) >= 0 {
            let e = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x3C);
            if strnicmp(e, name, 8) == 0 {
                let f = rd::<u16>(e, 0x30);
                wr::<u16>(e, 0x30, if set as u8 != 0 { f | bits } else { f & !bits });
            }
        }
        i += 1;
    }
}

/// `CEntryExitManager::LinkEntryExit(CEntryExit*)` (0x43F050): the entry-exit with the same name and
/// flag 4 (`GetEntryExitIndex(e, 4, 0)`) becomes `e`'s link; `e` becomes its link when it has none; its
/// times are set to 0..24.
#[hook(addr = 0x43F050, conv = cdecl, name = "CEntryExitManager::LinkEntryExit", expect = "56 8B 74 24 08")]
pub fn link_entry_exit(e: GPtr<u8>) {
    let i = get_entry_exit_index(e, 4, 0);
    if i == -1 {
        return;
    }
    let t = pool_at(i);
    wr::<GPtr<u8>>(e, 0x38, t);
    if rd::<GPtr<u8>>(t, 0x38).is_null() {
        wr::<GPtr<u8>>(t, 0x38, e);
    }
    wr::<u8>(t, 0x34, 0);
    wr::<u8>(t, 0x35, 0x18);
}

/// `CEntryExitManager::GetPositionRelativeToOutsideWorld(CVector&)` (stub 0x43F150 → 0x1566880): the
/// stack entries `posn - 1` down to 0.
#[hook(addr = 0x43F150, conv = cdecl, name = "CEntryExitManager::GetPositionRelativeToOutsideWorld", expect = "E9 2B 77 12 01")]
pub fn manager_get_position_relative_to_outside_world(pos: GPtr<f32>) {
    let mut n = ENTRY_EXIT_STACK_POSN.get();
    let st = ENTRY_EXIT_STACK.ptr().cast::<GPtr<u8>>();
    while n != 0 {
        let e = st.wrapping_index(n - 1).read();
        n -= 1;
        ee_get_position_relative_to_outside_world_call(e, pos);
    }
}

/// `CEntryExitManager::SetEnabledByName(const char*, bool)` (stub 0x43F9B0 → 0x156AE10):
/// `SetEntryExitFlag(name, 0x4000, enable)`.
#[hook(addr = 0x43F9B0, conv = cdecl, name = "CEntryExitManager::SetEnabledByName", expect = "E9 5B B4 12 01")]
pub fn set_enabled_by_name(name: GPtr<u8>, enable: u32) {
    set_entry_exit_flag_call(name, 0x4000, enable);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_WE_ARE_IN_INTERIOR_TRANSITION,
    HOOK_GET_POSITION_RELATIVE_TO_OUTSIDE_WORLD,
    HOOK_IS_TELEPORT_POINT_VALID,
    HOOK_GET_ENTRY_EXIT,
    HOOK_SET_ENTRY_EXIT_FLAG,
    HOOK_LINK_ENTRY_EXIT,
    HOOK_MANAGER_GET_POSITION_RELATIVE_TO_OUTSIDE_WORLD,
    HOOK_SET_ENABLED_BY_NAME,
];
