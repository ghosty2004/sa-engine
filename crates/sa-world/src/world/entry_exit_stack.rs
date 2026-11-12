//! PORT-VEH-3 (batch `world3-1`, SA:MP-executed rows): the entry-exit stack, the display-name
//! lookup, the nearest door object of an entry-exit and the interior ambient ped requests. Ported
//! from the 1.0 US bytes (two of them are `.HOODLUM` bodies whose stack-count address is computed from
//! obfuscated dwords); gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, x};

orig! {
    /// `CWorld::FindObjectsInRange(const CVector&, float, bool, int16*, int16, CEntity**, bool x5)` (0x564A20)
    fn find_objects_in_range(c: GPtr<u8>, r: f32, b2d: u32, count: GPtr<u8>, max: u32, out: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32) = cdecl 0x564A20 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CStreaming::StreamPedsIntoRandomSlots(int32*)` (0x40BDA0)
    fn stream_peds_into_random_slots(m: GPtr<u8>) = cdecl 0x40BDA0 => unported;
    /// `CStreaming::RemoveDodgyPedsFromRandomSlots()` (0x40BE60)
    fn remove_dodgy_peds_from_random_slots() = cdecl 0x40BE60 => unported;
}

/// `CEntryExitManager::ms_entryExitStackPosn` (0x96A7C4).
const STACK_POSN: usize = 0x96_A7C4;
/// `CEntryExitManager::ms_entryExitStack` (0x96A720, `CEntryExit*[]`).
const STACK: usize = 0x96_A720;
/// `CEntryExitManager::ms_spawnPoint` (0x96A7C0): the entry-exit the player is in.
const SPAWN_POINT: usize = 0x96_A7C0;

fn stack_at(k: i32) -> GPtr<GPtr<u8>> {
    g::<GPtr<u8>>(STACK.wrapping_add((k as usize).wrapping_mul(4)))
}

/// `CEntryExitManager::AddEntryExitToStack(CEntryExit*)` (0x43E410, `.HOODLUM` body 0x1564090): with
/// `e` its link (+0x38) or itself; when the stack top is `e` the stack is popped; else an entry-exit
/// in area 0 (+0x32) empties the stack, any other pushes the argument (not the link).
#[hook(addr = 0x43E410, conv = cdecl, name = "CEntryExitManager::AddEntryExitToStack",
       expect = "E9 7B 5C 12 01",
       unverified = "world3-1", route = "L2", hazards = "none",
       note = "the pop stores through rol([0x15604E0], [0x4F748A]) = 0x96A7C4 (protection-obfuscated address)")]
pub fn add_entry_exit_to_stack(ee: GPtr<u8>) {
    let link = rd::<GPtr<u8>>(ee, 0x38);
    let e = if link.is_null() { ee } else { link };
    let n = g::<i32>(STACK_POSN).read();
    if n > 0 && stack_at(n - 1).read() == e {
        g::<i32>(STACK_POSN).write(n - 1);
        return;
    }
    if rd::<u8>(e, 0x32) == 0 {
        g::<i32>(STACK_POSN).write(0);
        return;
    }
    stack_at(n).write(ee);
    g::<i32>(STACK_POSN).write(n + 1);
}

/// `CEntryExit::GetEntryExitToDisplayNameOf()` (0x43E650, `.HOODLUM` body 0x1569780): null when the
/// spawn point's area (+0x32) is 0 or this has no name; this when its own area is 0, the stack holds
/// at most one entry or the top is not the spawn point; else the entry below the top.
#[hook(addr = 0x43E650, conv = thiscall, name = "CEntryExit::GetEntryExitToDisplayNameOf", this = "CEntryExit",
       expect = "E9 2B B1 12 01",
       unverified = "world3-1", route = "L2", hazards = "none",
       note = "the stack count is read through [0x547B21] ^ [0x156054C] = 0x96A7C4 (protection-obfuscated address)")]
pub fn get_entry_exit_to_display_name_of(this: GPtr<u8>) -> GPtr<u8> {
    let sp = g::<GPtr<u8>>(SPAWN_POINT).read();
    if rd::<u8>(sp, 0x32) == 0 || rd::<u8>(this, 0) == 0 {
        return GPtr::from_addr(0);
    }
    if rd::<u8>(this, 0x32) == 0 {
        return this;
    }
    let n = g::<i32>(STACK_POSN).read();
    if n <= 1 || stack_at(n - 1).read() != sp {
        return this;
    }
    stack_at(n - 2).read()
}

/// `CEntryExitManager::FindNearestDoor(const CEntryExit&, float)` (0x43F630): among up to 100
/// objects within `radius` of the entrance centre (rect centre, entrance z) the nearest (3D) one with
/// physical flag byte +0x40 bit 5 (a door), else null.
#[hook(addr = 0x43F630, conv = cdecl, name = "CEntryExitManager::FindNearestDoor",
       expect = "81 EC A4 01 00 00",
       unverified = "world3-1", route = "L2", hazards = "x87-transcendental",
       note = "the object count word's upper half is uninitialised stack; only DX is used")]
pub fn find_nearest_door(ee: GPtr<u8>, radius: f32) -> GPtr<u8> {
    let mut best = radius;
    let c = [
        x(rd::<f32>(ee, 0x10)).fadd_m(rd::<f32>(ee, 0x8)).fmul_m(0.5).to_f32(),
        x(rd::<f32>(ee, 0x14)).fadd_m(rd::<f32>(ee, 0xC)).fmul_m(0.5).to_f32(),
        rd::<f32>(ee, 0x18),
    ];
    let mut count = 0u32;
    let mut ents = [0u32; 100];
    let cp: GPtr<u8> = GPtr::from_raw(&c as *const [f32; 3] as *mut [f32; 3]).cast();
    find_objects_in_range(
        cp,
        radius,
        0,
        GPtr::from_raw(&mut count as *mut u32).cast(),
        100,
        GPtr::from_raw(&mut ents as *mut [u32; 100]).cast(),
        0,
        0,
        0,
        1,
        0,
    );
    let mut res = GPtr::from_addr(0);
    let mut n = count as u16;
    while n != 0 {
        n = n.wrapping_sub(1);
        let e: GPtr<u8> = GPtr::from_addr(ents[n as i16 as usize] as usize);
        if rd::<u8>(e, 0x40) & 0x20 == 0 {
            continue;
        }
        let m = rd::<GPtr<u8>>(e, 0x14);
        let p = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
        let a = x(c[0]).fsub_m(rd::<f32>(p, 0));
        let b = x(c[1]).fsub_m(rd::<f32>(p, 4));
        let z = x(c[2]).fsub_m(rd::<f32>(p, 8));
        let d = z.fmul(z).fadd(a.fmul(a)).fadd(b.fmul(b)).fsqrt();
        if !d.fcom(x(best)).test_parity(0x05) {
            best = d.to_f32();
            res = e;
        }
    }
    res
}

/// `CEntryExit::RequestAmbientPeds()` (0x43E6D0): outside interiors (area 0xB72914 == 0) it only calls
/// `RemoveDodgyPedsFromRandomSlots`; else for the interiors named "bar1", "strip2", "LAstrip", "MAFCAS"
/// and "TRICAS" (case-insensitive) it streams their 8 ped models into the random slots.
#[hook(addr = 0x43E6D0, conv = thiscall, name = "CEntryExit::RequestAmbientPeds", this = "CEntryExit",
       expect = "A1 14 29 B7 00",
       unverified = "world3-1", route = "L2", hazards = "heap-writes",
       note = "StreamPedsIntoRandomSlots requests models (streaming lists)")]
pub fn request_ambient_peds(this: GPtr<u8>) {
    if g::<u32>(0xB7_2914).read() == 0 {
        remove_dodgy_peds_from_random_slots();
        return;
    }
    const N: i32 = -1;
    let sets: [(usize, [i32; 8]); 5] = [
        (0x85_95C4, [0xC, 0x38, 0x5D, 0x65, 0x14, 0x16, 0x3A, 0x3C]),
        (0x85_95BC, [0x100, 0x101, N, N, N, N, N, N]),
        (0x85_95B4, [0x57, 0xF4, 0xF6, N, N, N, N, N]),
        (0x85_95AC, [0x9, 0x28, 0x3B, 0x62, 0x38, 0x52, 0x5F, 0xE]),
        (0x85_95A4, [0xC, 0x14, 0x26, 0x2B, 0x5E, 0x5B, 0x3A, 0x53]),
    ];
    for (name, models) in sets {
        if stricmp(GPtr::from_addr(name), this) == 0 {
            let mut m = models;
            stream_peds_into_random_slots(GPtr::from_raw(&mut m as *mut [i32; 8]).cast());
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_ADD_ENTRY_EXIT_TO_STACK,
    HOOK_GET_ENTRY_EXIT_TO_DISPLAY_NAME_OF,
    HOOK_FIND_NEAREST_DOOR,
    HOOK_REQUEST_AMBIENT_PEDS,
];
