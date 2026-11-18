//! The replay playback set-up's leaves: the `CReplay` buffer scans and the pool conversion tables that
//! `CReplay::TriggerPlayback` calls. Several bodies live in `.HOODLUM`, reached through their 5-byte stubs.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use super::common::{g, rd, wr};
use super::frontier_b3::replay_find_size_of_packet;

/// `CReplay::BufferStatus[8]` (bytes) and `CReplay::Buffers[8]` (100000 bytes each).
const BUFFER_STATUS: usize = 0x97_FB7C;
const BUFFERS: usize = 0x97_FB88;
const BUFFER_SIZE: usize = 100_000;
/// `CReplay::m_VehiclePoolConversion[110]`, `CReplay::m_PedPoolConversion[140]`.
const VEHICLE_POOL_CONVERSION: usize = 0x97_F680;
const PED_POOL_CONVERSION: usize = 0x97_F838;
/// `CPools::ms_pPedPool`, `CPools::ms_pVehiclePool`.
const PED_POOL: usize = 0xB7_4490;
const VEHICLE_POOL: usize = 0xB7_4494;

/// The bytes' scan of the replay buffers: every buffer whose status byte is non-zero, every packet from its start up
/// to the end marker (type 0; the next packet at `FindSizeOfPacket(type)`). `f(packet, type)` returning true stops
/// the scan (true returned).
pub(crate) fn scan_replay_packets(mut f: impl FnMut(GPtr<u8>, u8) -> bool) -> bool {
    for i in 0..8usize {
        if g::<u8>(BUFFER_STATUS + i).read() == 0 {
            continue;
        }
        let buf = GPtr::<u8>::from_addr(BUFFERS + BUFFER_SIZE * i);
        let mut off = 0isize;
        let mut ty = rd::<u8>(buf, 0);
        while ty != 0 {
            if f(buf.byte_add(off), ty) {
                return true;
            }
            off += replay_find_size_of_packet(u32::from(ty)) as isize;
            ty = rd::<u8>(buf, off);
        }
    }
    false
}

/// `CReplay::IsThisPedUsedInRecording(int32 index)` (stub 0x45DDE0 → 0x1565580, cdecl, AL): a ped header packet
/// (type 3) whose pool byte (+1, unsigned) equals `index`.
#[hook(addr = 0x45DDE0, conv = cdecl, name = "CReplay::IsThisPedUsedInRecording", expect = "E9 9B 77 10 01",
       unverified = "frontier2-b30", route = "L2", hazards = "none")]
pub fn is_this_ped_used_in_recording(index: i32) -> u8 {
    u8::from(scan_replay_packets(|p, ty| ty == 3 && i32::from(rd::<u8>(p, 1)) == index))
}

/// `CReplay::IsThisVehicleUsedInRecording(int32 index)` (0x45DE40, cdecl, AL): the jump table at 0x45DEC4 / 0x45DED0
/// takes the pool byte (+1, unsigned) of types 1, 2 and 15..18 (the vehicle packets), and for a ped update (type 4)
/// its vehicle byte (+3) minus 1 when non-zero.
#[hook(addr = 0x45DE40, conv = cdecl, name = "CReplay::IsThisVehicleUsedInRecording", expect = "53 55 56 57 8B 7C 24 14",
       unverified = "frontier2-b30", route = "L2", hazards = "none")]
pub fn is_this_vehicle_used_in_recording(index: i32) -> u8 {
    u8::from(scan_replay_packets(|p, ty| match ty {
        1 | 2 | 15..=18 => i32::from(rd::<u8>(p, 1)) == index,
        4 => {
            let v = rd::<u8>(p, 3);
            v != 0 && i32::from(v) - 1 == index
        }
        _ => false,
    }))
}

/// `CReplay::FindFirstFocusCoordinate(CVector& out)` (stub 0x45D6C0 → 0x156F180, cdecl): `out = (0, 0, 0)`, then the
/// first general packet's (type 5) focus position (+0x4C).
#[hook(addr = 0x45D6C0, conv = cdecl, name = "CReplay::FindFirstFocusCoordinate", expect = "E9 BB 1A 11 01",
       unverified = "frontier2-b30", route = "L2", hazards = "none")]
pub fn find_first_focus_coordinate(out: GPtr<u8>) {
    for o in [0, 4, 8] {
        wr::<u32>(out, o, 0);
    }
    scan_replay_packets(|p, ty| {
        if ty != 5 {
            return false;
        }
        for o in [0, 4, 8] {
            wr::<u32>(out, o, rd::<u32>(p, 0x4C + o));
        }
        true
    });
}

/// The shared body of the two conversion tables: `table[0..n] = -1`; for each of the pool's first `n` slots (stride
/// `stride`) that is free (flag bit 7) or whose object address `objects + offset` is 0, the next conversion index
/// `c` the recording uses (searched from `c` with `used`) takes the slot index. `vehicle_tail` keeps the vehicle
/// table's bounds (the search stops at `c == n`, no bound after a store); the ped table stops at `c >= n` in both.
fn init_conversion_table(pool: usize, table: usize, n: u32, stride: u32, used: fn(i32) -> u8, vehicle_tail: bool) {
    let pool = g::<GPtr<u8>>(pool).read();
    for i in 0..n as usize {
        g::<i32>(table + 4 * i).write(-1);
    }
    let mut c: u32 = 0;
    for slot in 0..n {
        let free = rd::<i8>(rd::<GPtr<u8>>(pool, 4), slot as isize) < 0;
        if !free && rd::<u32>(pool, 0).wrapping_add(slot * stride) != 0 {
            continue;
        }
        if used(c as i32) == 0 {
            loop {
                c = c.wrapping_add(1);
                if if vehicle_tail { c == n } else { c as i32 >= n as i32 } {
                    return;
                }
                if used(c as i32) != 0 {
                    break;
                }
            }
        }
        g::<u32>((table as u32).wrapping_add(c.wrapping_mul(4)) as usize).write(slot);
        c = c.wrapping_add(1);
        if !vehicle_tail && c as i32 >= n as i32 {
            return;
        }
    }
}

/// `CReplay::InitialisePedPoolConversionTable()` (stub 0x45EF20 → 0x156B3A0, cdecl): [`init_conversion_table`] over
/// the ped pool's 140 slots (stride 0x7C4) with [`is_this_ped_used_in_recording`].
#[hook(addr = 0x45EF20, conv = cdecl, name = "CReplay::InitialisePedPoolConversionTable", expect = "E9 7B C4 10 01",
       unverified = "frontier2-b30", route = "L2", hazards = "none")]
pub fn initialise_ped_pool_conversion_table() {
    init_conversion_table(PED_POOL, PED_POOL_CONVERSION, 0x8C, 0x7C4, is_this_ped_used_in_recording, false);
}

/// `CReplay::InitialiseVehiclePoolConversionTable()` (0x45EFA0, cdecl): [`init_conversion_table`] over the vehicle
/// pool's 110 slots (stride 0xA18) with [`is_this_vehicle_used_in_recording`].
#[hook(addr = 0x45EFA0, conv = cdecl, name = "CReplay::InitialiseVehiclePoolConversionTable", expect = "51 53 55 8B 2D 94 44 B7 00",
       unverified = "frontier2-b30", route = "L2", hazards = "none")]
pub fn initialise_vehicle_pool_conversion_table() {
    init_conversion_table(VEHICLE_POOL, VEHICLE_POOL_CONVERSION, 0x6E, 0xA18, is_this_vehicle_used_in_recording, true);
}

/// `CReplay::InitialisePoolConversionTables()` (0x45F370, cdecl): the ped table, then the vehicle table (gta-reversed
/// has them the other way round).
#[hook(addr = 0x45F370, conv = cdecl, name = "CReplay::InitialisePoolConversionTables", expect = "E8 AB FB FF FF",
       unverified = "frontier2-b30", route = "L2", hazards = "none")]
pub fn initialise_pool_conversion_tables() {
    initialise_ped_pool_conversion_table();
    initialise_vehicle_pool_conversion_table();
}

/// The live slots of a pool walked from the last down to 0 (flag bit 7 clear and `objects + i * stride` non-zero), as
/// `EmptyPedsAndVehiclePools_NoDestructors` does.
fn for_each_live_desc(pool: usize, stride: u32, mut f: impl FnMut(GPtr<u8>)) {
    let pool = g::<GPtr<u8>>(pool).read();
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        let flag = rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize - 1);
        i -= 1;
        let e = rd::<u32>(pool, 0).wrapping_add(i.wrapping_mul(stride));
        if flag >= 0 && e != 0 {
            f(GPtr::from_addr(e as usize));
        }
    }
}

/// `CReplay::EmptyPedsAndVehiclePools_NoDestructors()` (0x45D390, cdecl): every live vehicle (last slot first)
/// `CWorld::Remove`d, then every live ped removed and its real-time shadow (+0x134) returned to
/// `g_realTimeShadowMan` (0xC40350).
#[hook(addr = 0x45D390, conv = cdecl, name = "CReplay::EmptyPedsAndVehiclePools_NoDestructors", expect = "53 55 8B 2D 90 44 B7 00",
       unverified = "frontier2-b30", route = "L2", hazards = "none")]
pub fn empty_peds_and_vehicle_pools_no_destructors() {
    use crate::bindings::world_remove;
    use crate::physical::base::return_real_time_shadow;
    for_each_live_desc(VEHICLE_POOL, 0xA18, world_remove);
    for_each_live_desc(PED_POOL, 0x7C4, |ped| {
        world_remove(ped);
        let shadow = rd::<GPtr<u8>>(ped, 0x134);
        if !shadow.is_null() {
            return_real_time_shadow(GPtr::from_addr(0xC4_0350), shadow);
        }
    });
}

/// `CSpecialFX::ReplayStarted()` (0x721D30, cdecl): when `bSnapShotActive` (0xC7C714) is set it is cleared and
/// `CTimer::ms_fTimeScale` (0xB7CB64) goes back to 1.0.
#[hook(addr = 0x721D30, conv = cdecl, name = "CSpecialFX::ReplayStarted", expect = "A0 14 C7 C7 00",
       unverified = "frontier2-b30", route = "L2", hazards = "none")]
pub fn special_fx_replay_started() {
    if g::<u8>(0xC7_C714).read() != 0 {
        g::<u8>(0xC7_C714).write(0);
        g::<u32>(0xB7_CB64).write(0x3F80_0000);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_IS_THIS_PED_USED_IN_RECORDING,
    HOOK_IS_THIS_VEHICLE_USED_IN_RECORDING,
    HOOK_FIND_FIRST_FOCUS_COORDINATE,
    HOOK_INITIALISE_PED_POOL_CONVERSION_TABLE,
    HOOK_INITIALISE_VEHICLE_POOL_CONVERSION_TABLE,
    HOOK_INITIALISE_POOL_CONVERSION_TABLES,
    HOOK_EMPTY_PEDS_AND_VEHICLE_POOLS_NO_DESTRUCTORS,
    HOOK_SPECIAL_FX_REPLAY_STARTED,
];
