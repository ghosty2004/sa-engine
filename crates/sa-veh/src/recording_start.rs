//! `CVehicleRecording::StartPlaybackRecordedCar` (INTRO-2: the intro's police-car ride, opcode 05EB). Ported from the
//! 1.0 US bytes (stub 0x45A980 → `.HOODLUM` 0x1565F10); gta-reversed `VehicleRecording.cpp` as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::checks::{NUM_PLAYBACK_FILES, PLAYBACK_BUFFER, PLAYBACK_GOING_ON, PLAYBACK_PAUSED, STREAMING_ARRAY, VEHICLE_FOR_PLAYBACK};
use crate::checks2::{
    PLAYBACK_BUFFER_SIZE, PLAYBACK_INDEX, PLAYBACK_LOOPED, PLAYBACK_RUNNING_TIME, PLAYBACK_SPEED, PLAYBACK_STREAMING_INDEX,
    USE_CAR_AI,
};
use crate::common::{rd, wr};

orig! {
    /// `CEntity::RegisterReference(CEntity** slot)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CVehicleRecording::SetRecordingToPointClosestToCoors(int32, CVector)` (0x45A1E0)
    fn set_recording_to_point_closest_to_coors(i: i32, x: u32, y: u32, z: u32) = cdecl 0x45A1E0 => unported;
}

/// The `ebx += 7` step of the unrolled free-slot search: the original adds the value of the no-CD VM 0xCB73E0
/// (descriptor 0x155FF74, return 0x155FF74 pushed as its second argument). Evaluated on the exe image (unicorn, 464
/// instructions from the entry to the trap-flag probe, no write outside the stack): 7; the probe after it
/// (`push ss; pop ss; pushf`) adds 0x28 only under a single-step debugger.
const VM_SLOT_STEP_7: u32 = 7;

/// `CVehicleRecording::StartPlaybackRecordedCar(CVehicle*, int32 fileNumber, bool useCarAI, bool looped)` (stub
/// 0x45A980 → 0x1565F10, cdecl).
///
/// The streaming index is the first of `NumPlayBackFiles` entries of `StreamingArray` (stride 0x10, +0 file number)
/// with the file number, else 0. The slot is the first index with `bPlaybackGoingOn` clear (unrolled by 8; the 8th
/// step is the VM's 7), else 16 (written out of bounds as the original does). Then `pVehicleForPlayback[slot] =
/// vehicle`, `vehicle->RegisterReference(&pVehicleForPlayback[slot])`, looped flag, streaming index, buffer (+4) and
/// size (+8) of the streaming entry, its byte reference count (+0xC) incremented, `bUseCarAI`, index 0, running time 0,
/// speed 1.0, going-on 1, paused 0. With car AI the (re-read) vehicle's autopilot mission byte +0x3BA becomes 0x22
/// and `SetRecordingToPointClosestToCoors(slot, position)` follows (position = matrix +0x30 when the matrix +0x14 is
/// set, else the placement +4); without, physical flags +0x40 get bit 2 and (re-read) lose bit 3. Finally the
/// (re-read) vehicle's byte +0x424 = slot.
#[hook(addr = 0x45A980, conv = cdecl, name = "CVehicleRecording::StartPlaybackRecordedCar", expect = "E9 8B B5 10 01",
       unverified = "intro3-1", route = "L2", hazards = "heap-writes", recs = "intro",
       note = "the 8th step of the slot search adds the no-CD VM 0xCB73E0's value (evaluated: 7)")]
pub fn start_playback_recorded_car(veh: GPtr<u8>, file: i32, use_car_ai: u32, looped: u32) {
    let n = NUM_PLAYBACK_FILES.ptr().cast::<i32>().read();
    let streams = STREAMING_ARRAY.ptr().cast::<u8>();
    let mut si: i32 = 0;
    let mut i = 0;
    while i < n {
        if rd::<i32>(streams, (i as isize) << 4) == file {
            si = i;
            break;
        }
        i += 1;
    }
    let going = PLAYBACK_GOING_ON.ptr().cast::<u8>();
    let mut slot: i32 = 0;
    'search: loop {
        for j in 0..8 {
            if going.wrapping_index(slot + j).read() == 0 {
                slot += if j == 7 { VM_SLOT_STEP_7 as i32 } else { j };
                break 'search;
            }
        }
        slot += 8;
        if slot >= 0x10 {
            break;
        }
    }
    let vslot = VEHICLE_FOR_PLAYBACK.ptr().cast::<GPtr<u8>>().wrapping_index(slot);
    vslot.write(veh);
    entity_register_reference(veh, vslot.cast());
    PLAYBACK_LOOPED.ptr().cast::<u8>().wrapping_index(slot).write(looped as u8);
    PLAYBACK_STREAMING_INDEX.ptr().cast::<i32>().wrapping_index(slot).write(si);
    let e = streams.byte_add((si << 4) as isize);
    PLAYBACK_BUFFER.ptr().cast::<u32>().wrapping_index(slot).write(rd::<u32>(e, 4));
    PLAYBACK_BUFFER_SIZE.ptr().cast::<u32>().wrapping_index(slot).write(rd::<u32>(e, 8));
    wr::<u8>(e, 0xC, rd::<u8>(e, 0xC).wrapping_add(1));
    USE_CAR_AI.ptr().cast::<u8>().wrapping_index(slot).write(use_car_ai as u8);
    let v = vslot.read();
    PLAYBACK_INDEX.ptr().cast::<u32>().wrapping_index(slot).write(0);
    PLAYBACK_RUNNING_TIME.ptr().cast::<u32>().wrapping_index(slot).write(0);
    PLAYBACK_SPEED.ptr().cast::<u32>().wrapping_index(slot).write(0x3F80_0000);
    going.wrapping_index(slot).write(1);
    PLAYBACK_PAUSED.ptr().cast::<u8>().wrapping_index(slot).write(0);
    if use_car_ai as u8 != 0 {
        wr::<u8>(v, 0x3BA, 0x22);
        let m = rd::<GPtr<u8>>(veh, 0x14);
        let pos = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) };
        set_recording_to_point_closest_to_coors(slot, rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8));
    } else {
        wr::<u32>(v, 0x40, rd::<u32>(v, 0x40) | 4);
        let v = vslot.read();
        wr::<u32>(v, 0x40, rd::<u32>(v, 0x40) & 0xFFFF_FFF7);
    }
    wr::<u8>(vslot.read(), 0x424, slot as u8);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_START_PLAYBACK_RECORDED_CAR];
