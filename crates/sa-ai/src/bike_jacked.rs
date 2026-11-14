//! `CTaskSimpleBikeJacked` (batch `frontier-1-1`): the rider of a bike (or a passenger) pulled off by a jacker. The
//! constructor and `ProcessPed`, ported from the 1.0 US bytes (the other members come from the generators). Fields:
//! +0x8 anim played, +0xC the first anim, +0x10 the second anim id (0xBF = none), +0x14 the vehicle, +0x18 the door,
//! +0x1C the time, +0x20 the jacker, +0x24 victim-is-driver, +0x28 the line-up utility.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::orig::at_site;

use crate::common::*;

/// `CTaskSimpleBikeJacked::CTaskSimpleBikeJacked(CVehicle*, int32 door, int32 time, CPed* jacker, bool isVictimDriver)`
/// (0x648B90, `ret 0x14`; entry `push -1; jmp 0x40681E`, a protection fragment that only pushes the SEH handler).
/// `CTaskSimple` base, vtable 0x86EFAC, the fields in the original's store order, then the vehicle's and the
/// jacker's references (each when non-null).
#[hook(addr = 0x648B90, conv = thiscall, name = "CTaskSimpleBikeJacked::Constructor",
       expect = "6A FF E9 87 DC DB FF",
       unverified = "frontier-1-1", route = "L2", hazards = "heap-writes",
       note = "the SEH frame (protection fragment 0x40681E) is not reproduced")]
pub fn bike_jacked_ctor(this: GPtr<u8>, veh: GPtr<u8>, door: u32, time: u32, jacker: GPtr<u8>, victim_driver: u8) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0x18, door);
    wr::<u32>(this, 0x1C, time);
    wr::<GPtr<u8>>(this, 0x20, jacker);
    wr::<u32>(this, 0, 0x86_EFAC);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 0x10, 0xBF);
    wr::<GPtr<u8>>(this, 0x14, veh);
    wr::<u8>(this, 0x24, victim_driver);
    wr::<u32>(this, 0x28, 0);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0x14));
    }
    let j = rd::<GPtr<u8>>(this, 0x20);
    if !j.is_null() {
        entity_register_reference(j, this.byte_add(0x20));
    }
    this
}

/// `CTaskSimpleBikeJacked::ProcessPed(CPed*)` (0x64C970, `ret 4`). Done (true) without a vehicle, or when the ped is
/// out of the vehicle (+0x46C bit 8 clear) and no first anim plays. Once the first anim has finished (+0x8): done
/// when there is no second anim; else blend it (100.0), then a `CEventKnockOffBike(vehicle, its move speed, +0xE0,
/// 0, 0, 0x37, 0, time, jacker, victim-is-driver, true)` with the "forced" bit (+0x35 bit 1) goes into the ped's
/// event group, +0x10 = 0xBF. Otherwise: a non-player driver gets its radio station set, a player in a vehicle tells
/// its audio it is about to exit; with no first anim yet: the jacker's pull-out anim (0x16D, 0x16C or 0x16B) must
/// be past `[0x858C24]` (or absent), then the jacked / jacking lines in an order drawn by `rand()` (call site
/// 0x64CB27), and the bike's fall anim 0xC9 (ride anim group, delta 8.0) with `FinishAnimBikeHitCB` (0x648D30) and
/// ped audio event 0x48; finally the line-up utility (0x1C bytes, zero offset, the door) is created once.
#[hook(addr = 0x64C970, conv = thiscall, name = "CTaskSimpleBikeJacked::ProcessPed",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier-1-1", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x64CB2C; the event is a stack local (0x3C bytes, zero-initialised here); the SEH frame is not reproduced")]
pub fn bike_jacked_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<GPtr<u8>>(this, 0x14).is_null() {
        return 1;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 && rd::<GPtr<u8>>(this, 0xC).is_null() {
        return 1;
    }
    if rd::<u8>(this, 8) != 0 {
        let id = rd::<u32>(this, 0x10);
        if id == 0xBF {
            return 1;
        }
        anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), 0, id, 100.0);
        let veh = rd::<GPtr<u8>>(this, 0x14);
        let mut ev = [0u32; 16];
        let e: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
        event_knock_off_bike_ctor_o(
            e,
            veh,
            veh.byte_add(0x44),
            veh.byte_add(0xE0),
            0,
            0,
            0x37,
            0,
            rd::<u32>(this, 0x1C),
            rd::<GPtr<u8>>(this, 0x20),
            u32::from(rd::<u8>(this, 0x24)),
            1,
        );
        wr::<u8>(e, 0x35, rd::<u8>(e, 0x35) | 2);
        event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 1);
        wr::<u32>(this, 0x10, 0xBF);
        event_knock_off_bike_dtor_o(e);
        return 0;
    }
    if ped_is_player(ped) == 0 && vehicle_is_driver(rd::<GPtr<u8>>(this, 0x14), ped) != 0 {
        ped_set_radio_station(ped);
    }
    if ped_is_player(ped) != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() {
            vehicle_audio_player_about_to_exit_as_driver(v.byte_add(0x138));
        }
    }
    if rd::<GPtr<u8>>(this, 0xC).is_null() && first_anim_may_start(this, ped) {
        let veh = rd::<GPtr<u8>>(this, 0x14);
        let group = rd::<u32>(vehicle_get_ride_anim_data(veh), 0);
        let a = anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), group, 0xC9, 8.0);
        wr::<GPtr<u8>>(this, 0xC, a);
        assoc_set_finish_callback(a, 0x64_8D30, this);
        ped_audio_add_audio_event(ped.byte_add(0x138), 0x48, 0.0, 1.0, GPtr::NULL, 0, 0, 0);
    }
    if rd::<u32>(this, 0x28) == 0 {
        let u = crt_new(0x1C);
        let u = if u.is_null() {
            u
        } else {
            let mut off = [0u32; 3];
            line_up_ped_with_car_ctor(u, GPtr::from_raw(off.as_mut_ptr()).cast(), 0, 0, rd::<u32>(this, 0x18))
        };
        wr::<GPtr<u8>>(this, 0x28, u);
    }
    0
}

/// The jacker's pull-out anim check and the jack lines before the fall anim (0x64CABA..0x64CB7F); false when the
/// pull-out anim still runs (its time at or below `[0x858C24]`, or NaN).
fn first_anim_may_start(this: GPtr<u8>, ped: GPtr<u8>) -> bool {
    let jacker = rd::<GPtr<u8>>(this, 0x20);
    if jacker.is_null() {
        return true;
    }
    let mut a = clump_get_association(rd::<GPtr<u8>>(jacker, 0x18), 0x16D);
    if a.is_null() {
        a = clump_get_association(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x20), 0x18), 0x16C);
    }
    if a.is_null() {
        a = clump_get_association(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x20), 0x18), 0x16B);
    }
    if rd::<GPtr<u8>>(this, 0x20).is_null() {
        return true;
    }
    if !a.is_null() && x(rd::<f32>(a, 0x20)).fcom(x(g::<f32>(0x85_8C24).read())).test_any(0x41) {
        return false;
    }
    let veh = rd::<GPtr<u8>>(this, 0x14);
    if (at_site(0x64_CB2C, rand) & 0x3FF) > 0x200 {
        if say_jacked(ped, veh, 0) == 0 {
            say_jacking_o(rd::<GPtr<u8>>(this, 0x20), ped, rd::<GPtr<u8>>(this, 0x14), 0);
        }
    } else if say_jacking_o(rd::<GPtr<u8>>(this, 0x20), ped, veh, 0) == 0 {
        say_jacked(ped, rd::<GPtr<u8>>(this, 0x14), 0);
    }
    true
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_BIKE_JACKED_CTOR, HOOK_BIKE_JACKED_PROCESS_PED];
