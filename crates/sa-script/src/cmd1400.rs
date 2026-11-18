//! `CRunningScript::ProcessCommands1400To1499` (0x48EAA0): complete port (PORT-SCRIPT-2..4, ADR-002): every jump-table case
//! is in [`PORTED`]; other opcodes reach the original default (-1).
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1400To1499.md`.
//!
//! The task cases build the task with `CTask::operator new` + the constructor (EAX = the task, null when `new`
//! fails) and give it with `GivePedScriptedTask(ped, task, command)`. The SEH state writes around `new` + ctor
//! (`[esp+0x450]`, the stub 0x4011FA) only matter for exception unwinding and are not ported.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_math::x87::ci_acos;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x057B, 0x057D, 0x057E, 0x057F, 0x0581, 0x0582, 0x0583, 0x0585, 0x0587, 0x0588, 0x058A, 0x058C, 0x058D,
    0x058E, 0x0591, 0x0592, 0x0594, 0x0595, 0x0596, 0x0597, 0x059A, 0x059B, 0x059C, 0x059D, 0x059E, 0x059F,
    0x05A0, 0x05A1, 0x05A2, 0x05A3, 0x05A4, 0x05A5, 0x05A6, 0x05A7, 0x05A8, 0x05A9, 0x05AA, 0x05AD, 0x05AE,
    0x05B0, 0x05B1, 0x05B2, 0x05B3, 0x05B5, 0x05B6, 0x05B7, 0x05B8, 0x05B9, 0x05BA, 0x05BB, 0x05BC, 0x05BD,
    0x05BE, 0x05BF, 0x05C0, 0x05C1, 0x05C2, 0x05C3, 0x05C4, 0x05C5, 0x05C6, 0x05C7, 0x05C8, 0x05C9, 0x05CA,
    0x05CB, 0x05CC, 0x05CD, 0x05CE, 0x05CF, 0x05D0, 0x05D1, 0x05D2, 0x05D3, 0x05D4, 0x05D5, 0x05D6, 0x05D7,
    0x05D8, 0x05D9, 0x05DA, 0x05DB,
];

/// `.rdata` 0x86E69C / 0x86E698: the default enter-car times (int 20000), read at run time as the original does.
const ENTER_AS_PASSENGER_DEFAULT_TIME: usize = 0x86E69C;
const ENTER_AS_DRIVER_DEFAULT_TIME: usize = 0x86E698;
/// The dword 0x8D237C (initial value 6) the enter-car cases copy into the new task and then reset to 6
/// (spec open question: its name).
const NEXT_ENTER_CAR_DWORD: usize = 0x8D237C;

use sa_ai::bindings::task_new;
use sa_world::bindings::leave_car_ctor as task_complex_leave_car_ctor;
use crate::bindings::give_ped_scripted_task;
use sa_world::bindings::compute_target_door_to_enter_as_passenger;
use sa_world::bindings::hands_up_ctor as task_simple_hands_up_ctor;
use sa_ai::bindings::enter_car_as_passenger_timed_ctor as task_enter_car_as_passenger_timed_ctor;
orig! {
    /// `CTaskComplexEnterCarAsDriverTimed::CTaskComplexEnterCarAsDriverTimed(CVehicle*, int32 time)` (0x63AD70), returns this
    fn task_enter_car_as_driver_timed_ctor(t: GPtr<u8>, v: GPtr<u8>, time: u32) -> GPtr<u8> = thiscall 0x63AD70 => unported;
}

/// The enter-car time mapping: -2 -> -1, -1 -> the dword at `default_at`, else the value.
#[inline]
fn enter_car_time(t: u32, default_at: usize) -> u32 {
    match t as i32 {
        -2 => u32::MAX,
        -1 => GPtr::<u32>::from_addr(default_at).read(),
        _ => t,
    }
}

/// `t->field = [0x8D237C]` (t not null-checked, as in the original), then `[0x8D237C] = 6`.
#[inline]
fn take_next_enter_car_dword(t: GPtr<u8>, field: isize) {
    let g = GPtr::<u32>::from_addr(NEXT_ENTER_CAR_DWORD);
    t.byte_add(field).cast::<u32>().write(g.read());
    g.write(6);
}

use crate::bindings::affect_secondary_behaviour_ctor as task_affect_secondary_ctor;
use sa_world::bindings::physical_45afb0 as set_turn_speed;
use sa_ai::bindings::task_trigger_look_at_ctor;
use sa_core::bindings::strncmp;
use sa_core::bindings::vector_magnitude;
use sa_world::bindings::achieve_heading_ctor as task_achieve_heading_ctor;
use sa_world::bindings::add_light;
use sa_world::bindings::car_drive_wander_ctor as task_car_drive_wander_ctor;
use sa_world::bindings::complex_jump_ctor as task_jump_ctor;
use sa_world::bindings::cower_ctor as task_cower_ctor;
use sa_world::bindings::ctor_48df30 as task_scratch_head_ctor;
use sa_world::bindings::die_ctor as task_die_ctor;
use sa_world::bindings::entity_get_is_static;
use sa_world::bindings::find_zone;
use sa_world::bindings::flee_entity_ctor as task_flee_entity_ctor;
use sa_world::bindings::get_percentage_progress;
use sa_world::bindings::get_stat_value;
use sa_world::bindings::look_about_ctor as task_look_about_ctor;
use sa_world::bindings::physical_add_to_moving_list as add_to_moving_list;
use sa_world::bindings::seek_entity_standard_ctor as task_seek_entity_ctor;
use sa_world::bindings::sequence_add_task;
use sa_world::bindings::set_stat_value;
use sa_world::bindings::shake_fist_ctor as task_shake_fist_ctor;
use sa_world::bindings::stand_still_ctor as task_stand_still_ctor;
use sa_world::bindings::task_use_atm_ctor;
use sa_world::bindings::tired_ctor as task_tired_ctor;
use crate::bindings::global_index;
use sa_world::bindings::stats_increment_stat as increment_stat;
use sa_ai::bindings::tm_set_task_secondary as set_task_secondary;
use sa_stream::bindings::rand as crt_rand;
use sa_world::bindings::duck_ctor as task_duck_ctor;
use sa_world::bindings::fall_and_get_up_ctor2 as task_fall_and_get_up_ctor;
use sa_world::bindings::flee_point_ctor as task_flee_point_ctor;
use sa_world::bindings::follow_point_route_ctor as task_follow_point_route_ctor;
use sa_world::bindings::fx_trigger_gunshot as trigger_gunshot;
use sa_world::bindings::go_to_point_and_stand_still_ctor as task_go_to_point_and_stand_still_ctor;
use sa_world::bindings::ik_look_at;
use sa_world::bindings::leave_car_and_flee_ctor as task_leave_car_and_flee_ctor;
use sa_world::bindings::multiply3x3 as multiply_3x3;
use sa_world::bindings::multiply3x3_vm as multiply_3x3_vm;
use sa_world::bindings::pause_ctor as task_pause_ctor;
use sa_ped::bindings::player_info_get_pos as player_get_pos;
use sa_world::bindings::point_route_add;
use sa_world::bindings::say_ctor as task_say_ctor;
use sa_world::bindings::set_clump_alpha;
use sa_world::bindings::vec_mul_assign as vector_mul_assign;
use sa_world::bindings::vector_add_assign;
use sa_world::bindings::vector_mul_scalar as vector_mul;
use sa_core::bindings::cross_product;
use sa_core::bindings::vector_normalise;
use sa_ai::bindings::drive_to_point_ctor as task_drive_to_point_ctor;
orig! {
    /// `CStats::RegisterBestPosition(eStats, int32)` (0x55A160)
    fn register_best_position(stat: u32, pos: u32) = cdecl 0x55A160 => unported;
    /// `CUserDisplay::OnscnTimer.SetCounterFlashWhenFirstDisplayed(uint16 varId, bool)` (0x44CEB0)
    fn set_counter_flash_when_first_displayed(t: GPtr<u8>, id: u32, b: u32) = thiscall 0x44CEB0 => unported;
    /// virtual `CEntity::SetIsStatic(bool)` (vtable +0x10)
    fn vcall_set_is_static(f: u32, e: GPtr<u8>, b: u32) = thiscall dyn => unported;
    /// `CVector::operator/=(float)` (0x411A30), the float as its raw dword
    fn vector_div_assign(v: GPtr<u32>, k: u32) = thiscall 0x411A30 => unported;
    /// `CTaskComplexGoToPointAndStandStillTimed::...(int32 moveState, const CVector&, float, float, int32 time)` (0x6685E0)
    fn task_go_to_point_and_stand_still_timed_ctor(t: GPtr<u8>, ms: u32, p: GPtr<u32>, r1: u32, r2: u32, time: u32) -> GPtr<u8> = thiscall 0x6685E0 => unported;
}

use sa_core::field::gr32 as rd;

use sa_ai::common::new_task;

/// The sequence-add tail (0x48FEFD): `CTaskSequences::ms_taskSequence[ms_iActiveSequence].AddTask(t)`; the index is
/// read through the protection stub 0x409AD7 (`[0x15608E0] + [0x4EF1C5]` = 0x8D2E98, decoded from the image).
#[inline]
fn sequence_add(t: GPtr<u8>) {
    let i = rd(0x8D2E98);
    sequence_add_task(GPtr::from_addr(0xC178F0u32.wrapping_add(i << 6) as usize), t);
}

/// `fld SP[i]` (a memory load: a signalling NaN comes back quiet).
#[inline]
fn ld(i: i32) -> X87 {
    X87::from_f32(spf(i))
}

/// 0587 / 0588 SET_LOAD_COLLISION_FOR_CAR_FLAG / ..._FOR_CHAR_FLAG common body (0x48EC0E), `e` not null-checked.
fn set_load_collision_flag(s: Scr, e: GPtr<u8>) {
    let pf = e.byte_add(0x40).cast::<u32>();
    let ef = e.byte_add(0x1C).cast::<u32>();
    if sp(1) != 0 {
        pf.write(pf.read() & 0xFFFF_BFFF);
        if uses_mission_cleanup(s) {
            world_remove(e);
            ef.write(ef.read() | 0x40000);
            world_add(e);
        }
    } else {
        pf.write(pf.read() | 0x4000);
        let v = ef.read();
        if v & 0x40000 != 0 {
            ef.write(v & 0xFFFB_FFFF);
            if entity_get_is_static(e) as u8 == 0 {
                add_to_moving_list(e);
            }
        }
    }
}

/// The static check of the object velocity cases: `if (o->m_nFlags & 4 /*bIsStatic*/) { o->SetIsStatic(false);
/// o->AddToMovingList(); }` (`test byte [o+0x1C], 4`, vtable +0x10).
fn unstatic(o: GPtr<u8>) {
    if o.byte_add(0x1C).read() & 4 != 0 {
        let f = o.cast::<GPtr<u32>>().read().byte_add(0x10).read();
        vcall_set_is_static(f, o, 0);
        add_to_moving_list(o);
    }
}

/// The "-100 ground rule" (`fld z; fcom [-100.0]; test ah,0x41; jp keep`, else `fstp st(0)` and ST0 =
/// `FindGroundZForCoord(x, y)`; ST0 stored as f32).
fn ground_rule(x: f32, y: f32, z: f32) -> u32 {
    let st = X87::from_f32(z);
    let st = if st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        st
    } else {
        X87::from_f64_exact(find_ground_z_for_coord(x, y))
    };
    st.to_f32().to_bits()
}

/// `.rdata` 0x858B38: 0.02f; 0x858B40: 50.0f; 0x859878: 57.295776f; 0x8595EC: 0.017453292f; 0x858C18: 1e-6f;
/// 0x858B14: 3.0517578e-05f.
const K_002: u32 = 0x3CA3_D70A;
const K_50: u32 = 0x4248_0000;
const K_RAD2DEG: u32 = 0x4265_2EE0;
const K_DEG2RAD: u32 = 0x3C8E_FA35;
const K_1EM6: u32 = 0x3586_37BD;
const K_INV_32768: u32 = 0x3800_0000;
/// `.rdata` f64 0x85A308: 0.01; 0x859060: 0.5.
const K_D001: f64 = 0.01;
const K_D05: f64 = 0.5;

/// `CRunningScript::ProcessCommands1400To1499(int32 command)` (0x48EAA0, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x48EAA0, conv = thiscall, name = "CRunningScript::ProcessCommands1400To1499", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_1400_to_1499(s: Scr, command: u32) -> i8 {
    match command {
        // 05C4 TASK_HANDS_UP (0x49017F): `time = SP1 == -1 ? 20000 : SP1 == -2 ? 0x7FFFFFFF : SP1`;
        // `new(0x34) CTaskSimpleHandsUp(time)`; tail 0x48FB64/0x48FB66
        0x05C4 => {
            collect(s, 2);
            let time = match spi(1) {
                -1 => 0x4E20,
                -2 => 0x7FFF_FFFF,
                v => v as u32,
            };
            let ped = sp(0);
            let t = task_new(0x34);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                task_simple_hands_up_ctor(t, time)
            };
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05CA TASK_ENTER_CAR_AS_PASSENGER (0x49036C): `v = VEH(SP1)`; `door = SP3 == -1 ? 0 :
        // ComputeTargetDoorToEnterAsPassenger(v, SP3)`; `new(0x2C) CTaskComplexEnterCarAsPassengerTimed(v, door, time,
        // true)`; tail 0x490431: `t+0x1C = [0x8D237C]` (unchecked), `[0x8D237C] = 6`, give.
        0x05CA => {
            collect(s, 4);
            let (ped, time, seat) = (sp(0), sp(2), sp(3));
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let door = if seat as i32 == -1 {
                0
            } else {
                compute_target_door_to_enter_as_passenger(v, seat)
            };
            let t = task_new(0x2C);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                let time = enter_car_time(time, ENTER_AS_PASSENGER_DEFAULT_TIME);
                task_enter_car_as_passenger_timed_ctor(t, v, door, time, 1)
            };
            take_next_enter_car_dword(t, 0x1C);
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05CB TASK_ENTER_CAR_AS_DRIVER (0x49046B): `new(0x24) CTaskComplexEnterCarAsDriverTimed(VEH(SP1), time)`;
        // `t+0x14 = [0x8D237C]` (unchecked), `[0x8D237C] = 6`; give (tail 0x4908F6).
        0x05CB => {
            collect(s, 3);
            let (ped, time) = (sp(0), sp(2));
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let t = task_new(0x24);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                let time = enter_car_time(time, ENTER_AS_DRIVER_DEFAULT_TIME);
                task_enter_car_as_driver_timed_ctor(t, v, time)
            };
            take_next_enter_car_dword(t, 0x14);
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05CD TASK_LEAVE_CAR (0x490554): `new(0x34) CTaskComplexLeaveCar(VEH(SP1), 0, 0, true, false)`; tail
        // 0x490B60/0x490B62
        0x05CD => {
            collect(s, 2);
            let ped = sp(0);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let t = task_new(0x34);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                task_complex_leave_car_ctor(t, v, 0, 0, 1, 0)
            };
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 057B, 057D, 057F, 0585, 058D, 058E, 0591, 0592, 059B, 05B1, 05B2, 05B3, 05B5, 05B7, 05B8, 05C6, 05CC, 05CE,
        // 05D0, 05D5 (0x490C0B): the jump table goes straight to the epilogue: 0 without reading the operands.
        0x057B | 0x057D | 0x057F | 0x0585 | 0x058D | 0x058E | 0x0591 | 0x0592 | 0x059B | 0x05B1 => {}
        0x05B2 | 0x05B3 | 0x05B5 | 0x05B7 | 0x05B8 | 0x05C6 | 0x05CC | 0x05CE | 0x05D0 | 0x05D5 => {}
        // 0596 IS_PLAYER_IN_SHORTCUT_TAXI (0x48EE39): `CollectParameters(1)`; UpdateCompareFlag(false).
        0x0596 => {
            collect(s, 1);
            set_flag(s, false);
        }
        // 057E SET_PLAYER_IS_IN_STADIUM (0x48EADE): `CTheScripts::bPlayerIsInTheStatium [0xA444A4] = SP0 != 0`.
        0x057E => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0xA444A4).write(u8::from(sp(0) != 0));
        }
        // 0581 DISPLAY_RADAR (0x48EAFD): `CHud::bScriptDontDisplayRadar [0xBAA3FB] = SP0 == 0`.
        0x0581 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0xBAA3FB).write(u8::from(sp(0) == 0));
        }
        // 0582 REGISTER_BEST_POSITION (0x48EB1D): `CStats::RegisterBestPosition(SP0, SP1)`.
        0x0582 => {
            collect(s, 2);
            register_best_position(sp(0), sp(1));
        }
        // 0583 IS_PLAYER_IN_INFO_ZONE (0x48EB42): `p = &CWorld::Players[SP0]`; 8-byte label; the name copied
        // up to its NUL into a zeroed 8-byte local; `pos = p->GetPos()`; flag = `CTheZones::FindZone(&pos,
        // name.lo, name.hi, 2 /*INFO*/) != 0`. The label buffer starts zeroed (PORT-SCRIPT-3 decision): an
        // 8-character label stops at byte 8 instead of the original's stale stack bytes.
        0x0583 => {
            collect(s, 1);
            let p = player_info(sp(0));
            let mut buf = [0u8; 16];
            read_text_label(s, GPtr::from_raw(buf.as_mut_ptr()), 8);
            let mut name = [0u8; 8];
            for (i, b) in buf.iter().take(8).enumerate() {
                if *b == 0 {
                    break;
                }
                name[i] = *b;
            }
            let mut out = [0u32; 3];
            let r = player_get_pos(p, GPtr::from_raw(out.as_mut_ptr()));
            let mut pos = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
            let lo = u32::from_le_bytes([name[0], name[1], name[2], name[3]]);
            let hi = u32::from_le_bytes([name[4], name[5], name[6], name[7]]);
            let b = find_zone(GPtr::from_raw(pos.as_mut_ptr()), lo, hi, 2) != 0;
            set_flag(s, b);
        }
        // 0587 SET_LOAD_COLLISION_FOR_CAR_FLAG (0x48EBF4, gr: unregistered) / 0588 ..._FOR_CHAR_FLAG (0x48EC8A,
        // jumps into 0587's body with the ped): see `set_load_collision_flag`.
        0x0587 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_load_collision_flag(s, v);
        }
        0x0588 => {
            collect(s, 2);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            set_load_collision_flag(s, p);
        }
        // 058A ADD_BIG_GUN_FLASH (0x48ECAA): `dir = (SP3-SP0, SP4-SP1, SP5-SP2)` (x87, f32), `dir.Normalise()`;
        // `CPointLights::AddLight(0, (SP0,SP1,SP2), (0,0,0), 5.0, 1.0, 0.8, 0.0, 0, 0, 0)`;
        // `g_fx.TriggerGunshot(null, &pos, &dir, true)`.
        0x058A => {
            collect(s, 6);
            let mut pos = [sp(0), sp(1), sp(2)];
            let mut dir = [
                ld(3).fsub_m(spf(0)).to_f32().to_bits(),
                ld(4).fsub_m(spf(1)).to_f32().to_bits(),
                ld(5).fsub_m(spf(2)).to_f32().to_bits(),
            ];
            let dp = GPtr::from_raw(dir.as_mut_ptr());
            vector_normalise(dp.cast::<[f32;3]>());
            add_light(0, f32::from_bits(pos[0]), f32::from_bits(pos[1]), f32::from_bits(pos[2]), f32::from_bits(0), f32::from_bits(0), f32::from_bits(0), f32::from_bits(0x40A0_0000), f32::from_bits(0x3F80_0000), f32::from_bits(0x3F4C_CCCD), f32::from_bits(0), 0, 0, GPtr::NULL);
            trigger_gunshot(GPtr::from_addr(0xA9AE00), GPtr::NULL, GPtr::<u32>::cast::<u8>(GPtr::from_raw(pos.as_mut_ptr())), dp.cast::<u8>(), 1);
        }
        // 058C GET_PROGRESS_PERCENTAGE (0x48EDA0, gr: unregistered): `SP0 = CStats::GetPercentageProgress()`
        // (fstp f32); StoreParameters(1).
        0x058C => {
            set_sp(0, X87::from_f64_exact(get_percentage_progress().to_f64()).to_f32().to_bits());
            store(s, 1);
        }
        // 0594 SET_VEHICLE_TO_FADE_IN (0x48EDBB, gr: unregistered): `SetClumpAlpha(veh(SP0)->m_pRwClump, SP1)`.
        0x0594 => {
            collect(s, 2);
            let a = sp(1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_clump_alpha(GPtr::from_addr((v.byte_add(0x18).cast::<u32>().read()) as usize), a as i32);
        }
        // 0595 REGISTER_ODDJOB_MISSION_PASSED (0x48EDEF, gr: unregistered): IncrementStat(0x93, 1.0);
        // `v = GetStatValue(0xB0)` (fstp f32 over the dead 1.0 slot); IncrementStat(0xB1, v); SetStatValue(0xB0,
        // 0.0); `CTheScripts::LastMissionPassedTime [0xA476A8] = CTimer::m_snTimeInMilliseconds [0xB7CB84]`.
        0x0595 => {
            increment_stat(0x93, f32::from_bits(0x3F80_0000));
            let v = X87::from_f64_exact(get_stat_value(0xB0)).to_f32().to_bits();
            increment_stat(0xB1, f32::from_bits(v));
            set_stat_value(0xB0, f32::from_bits(0));
            GPtr::<u32>::from_addr(0xA476A8).write(rd(0xB7CB84));
        }
        // 059A IS_AUSTRALIAN_GAME (0x48EE42, gr: unregistered): UpdateCompareFlag(false).
        0x059A => set_flag(s, false),
        // 0597 IS_CHAR_DUCKING (0x48EE52): flag = bit 26 of `ped+0x46C` (bIsDucking); no null check.
        0x0597 => {
            collect(s, 1);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            set_flag(s, (p.byte_add(0x46C).cast::<u32>().read() >> 26) & 1 != 0);
        }
        // 059C SET_ONSCREEN_COUNTER_FLASH_WHEN_FIRST_DISPLAYED (0x48EE90, gr: unregistered):
        // `i = GetIndexOfGlobalVariable()`; CollectParameters(1); `OnscnTimer.SetCounterFlashWhenFirstDisplayed((u16)i,
        // SP0 != 0)`.
        0x059C => {
            let i = global_index(s);
            collect(s, 1);
            set_counter_flash_when_first_displayed(GPtr::from_addr(0xBA1788), u32::from(i), u32::from(sp(0) != 0));
        }
        // 059D SHUFFLE_CARD_DECKS (0x48EED9, gr: unregistered): CollectParameters(1); zero `CTheScripts::CardStack`
        // (0xA44218, 0x9C dwords); local `L[i] = i` (312 int16); `rem = (int16)(SP0 * 52)` decremented per card;
        // for card 1..52, for each deck (`SP0` re-read per pass): `k = (int16)_ftol((float)(rand() & 0xFFFF) *
        // 3.0517578e-05f * rem)` (fild, fmul m32, fimul m32); `CardStack[L[k]] = card`; shift `L[j] = j > 310 ?
        // 0 : L[j+1]` for j = k..rem-1; finally `CardStackPosition [0xA44210] = 0`.
        0x059D => {
            collect(s, 1);
            for i in 0..0x9C {
                GPtr::<u32>::from_addr(0xA44218 + 4 * i).write(0);
            }
            let mut l = [0i16; 312];
            for (i, v) in l.iter_mut().enumerate() {
                *v = i as i16;
            }
            let mut rem = sp(0).wrapping_mul(0x34);
            for card in 1u16..0x35 {
                let mut d = 0i32;
                while d < spi(0) {
                    let r = (sa_core::orig::at_site(0x48EF2C, crt_rand) & 0xFFFF) as i32;
                    let n = i32::from(rem as i16);
                    let k = X87::from_i32(r).fmul_m(f(K_INV_32768)).fmul(X87::from_i32(n)).ftol() as u32;
                    let k = i32::from(k as u16 as i16);
                    let idx = i32::from(l[k as usize]);
                    GPtr::<u16>::from_addr((0xA44218 + idx * 2) as u32 as usize).write(card);
                    let mut j = k;
                    while j < n {
                        l[j as usize] = if j > 0x136 { 0 } else { l[j as usize + 1] };
                        j += 1;
                    }
                    rem = rem.wrapping_sub(1);
                    d += 1;
                }
            }
            GPtr::<u16>::from_addr(0xA44210).write(0);
        }
        // 059E FETCH_NEXT_CARD (0x48EFAD, gr: unregistered): `p = CardStackPosition`; `CardStack[p] == 0 -> p = 0`;
        // `SP0 = CardStack[p]` (sign-extended); `p++`, `p == 312 -> 0`; StoreParameters(1).
        0x059E => {
            let mut p = GPtr::<u16>::from_addr(0xA44210).read();
            let card = |p: u16| GPtr::<i16>::from_addr((0xA44218 + i32::from(p as i16) * 2) as u32 as usize).read();
            if card(p) == 0 {
                p = 0;
            }
            set_sp(0, i32::from(card(p)) as u32);
            p = p.wrapping_add(1);
            GPtr::<u16>::from_addr(0xA44210).write(if p == 0x138 { 0 } else { p });
            store(s, 1);
        }
        // 059F GET_OBJECT_VELOCITY (0x48EFFC): SP0..2 = `obj(SP0)->m_vecMoveSpeed * 50.0` (x87, f32); no null
        // check; StoreParameters(3).
        0x059F => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0)).byte_add(0x44).cast::<f32>();
            for k in 0..3 {
                let v = X87::from_f32(o.wrapping_index(k).read()).fmul_m(f(K_50)).to_f32().to_bits();
                set_sp(k, v);
            }
            store(s, 3);
        }
        // 05A0 IS_DEBUG_CAMERA_ON (0x48F057): flag = `TheCamera.m_bWorldViewerBeingUsed [0xB6F07E] != 0`.
        0x05A0 => set_flag(s, GPtr::<u8>::from_addr(0xB6F07E).read() != 0),
        // 05A1 ADD_TO_OBJECT_ROTATION_VELOCITY (0x48F07D, gr: unregistered): `v = (SP1, SP2, SP3) * 0.02` (f32);
        // `w = Multiply3x3(*o->m_matrix, v)`; `ts = o->m_vecTurnSpeed + w` (f32 per component, read before the
        // static check); static check; `o->m_vecTurnSpeed = ts`. No null checks.
        0x05A1 => {
            collect(s, 4);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let turn = o.byte_add(0x50).cast::<f32>();
            let ts0 = [turn.read(), turn.wrapping_index(1).read(), turn.wrapping_index(2).read()];
            let mut v = [0u32; 3];
            for k in 0..3 {
                v[k as usize] = ld(k + 1).fmul_m(f(K_002)).to_f32().to_bits();
            }
            let mut out = [0u32; 3];
            let m = o.byte_add(0x14).cast::<GPtr<u8>>().read();
            let w = GPtr::<u8>::cast::<u32>(multiply_3x3(GPtr::<u32>::cast::<u8>(GPtr::from_raw(out.as_mut_ptr())), m, GPtr::<u32>::cast::<u8>(GPtr::from_raw(v.as_mut_ptr())))).cast::<f32>();
            let mut ts = [0f32; 3];
            for k in 0..3 {
                ts[k] = X87::from_f32(ts0[k]).fadd_m(w.wrapping_index(k as i32).read()).to_f32();
            }
            unstatic(o);
            for (k, t) in ts.iter().enumerate() {
                turn.wrapping_index(k as i32).write(*t);
            }
        }
        // 05A2 SET_OBJECT_ROTATION_VELOCITY (0x48F17E, gr: unregistered): `v = (ts*SP1, ts*SP2, ts*SP3)` with
        // `ts = CTimer::ms_fTimeStep [0xB7CB5C]` (fld ts; fmul SP); `w = Multiply3x3(*o->m_matrix, v)`; static
        // check; `0x45AFB0(o, w.x*0.02, w.y*0.02, w.z*0.02)`.
        0x05A2 => {
            collect(s, 4);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let step = f32::from_bits(rd(0xB7CB5C));
            let mut v = [0u32; 3];
            for k in 0..3 {
                v[k as usize] = X87::from_f32(step).fmul_m(spf(k + 1)).to_f32().to_bits();
            }
            let mut out = [0u32; 3];
            let m = o.byte_add(0x14).cast::<GPtr<u8>>().read();
            let w = GPtr::<u8>::cast::<u32>(multiply_3x3(GPtr::<u32>::cast::<u8>(GPtr::from_raw(out.as_mut_ptr())), m, GPtr::<u32>::cast::<u8>(GPtr::from_raw(v.as_mut_ptr())))).cast::<f32>();
            let w = [w.read(), w.wrapping_index(1).read(), w.wrapping_index(2).read()];
            unstatic(o);
            let z = X87::from_f32(w[2]).fmul_m(f(K_002)).to_f32().to_bits();
            let y = X87::from_f32(w[1]).fmul_m(f(K_002)).to_f32().to_bits();
            let x = X87::from_f32(w[0]).fmul_m(f(K_002)).to_f32().to_bits();
            set_turn_speed(o, x, y, z);
        }
        // 05A3 IS_OBJECT_STATIC (0x48F260, gr: unregistered): flag = `CEntity::GetIsStatic(obj(SP0))`.
        0x05A3 => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            set_flag(s, entity_get_is_static(o) as u8 != 0);
        }
        // 05A4 GET_ANGLE_BETWEEN_2D_VECTORS (0x48F2A2, gr: unregistered): a = (SP0, SP1), b = (SP2, SP3), all in
        // x87 registers: `c = (b.y*a.y + b.x*a.x) / (sqrt(b.y² + b.x²) * sqrt(a.y² + a.x²))`; `SP0 = _CIacos(c) *
        // 57.295776` (fstp f32); StoreParameters(1).
        0x05A4 => {
            collect(s, 4);
            let (ax, ay, bx, by) = (spf(0), spf(1), spf(2), spf(3));
            let dot = X87::from_f32(by).fmul_m(ay).fadd(X87::from_f32(bx).fmul_m(ax));
            let lb = X87::from_f32(by).fmul_m(by).fadd(X87::from_f32(bx).fmul_m(bx)).fsqrt();
            let la = X87::from_f32(ay).fmul_m(ay).fadd(X87::from_f32(ax).fmul_m(ax)).fsqrt();
            let c = dot.fdiv(lb.fmul(la));
            set_sp(0, ci_acos(c).fmul_m(f(K_RAD2DEG)).to_f32().to_bits());
            store(s, 1);
        }
        // 05A5 DO_2D_RECTANGLES_COLLIDE (0x48F339, gr: unregistered): halves with the f64 0.5; b1, t2, l1, l2, r1,
        // r2 stored f32, t1 = SP1 + SP3*0.5 and b2 = SP5 - SP7*0.5 kept in registers; flag cleared when b2 > t1,
        // b1 > t2, r1 < l2, l1 > r2 (strict; equality and NaN keep it).
        0x05A5 => {
            collect(s, 8);
            let h = X87::from_f64_exact(K_D05);
            let a = ld(3).fmul(h);
            let b1 = ld(1).fsub(a).to_f32();
            let bb = ld(7).fmul(h);
            let t2 = ld(5).fadd(bb).to_f32();
            let c = ld(2).fmul(h);
            let l1 = ld(0).fsub(c).to_f32();
            let d = ld(6).fmul(h);
            let l2 = ld(4).fsub(d).to_f32();
            let r1 = ld(0).fadd(c).to_f32();
            let r2 = ld(4).fadd(d).to_f32();
            let t1 = ld(1).fadd(a);
            let b2 = ld(5).fsub(bb);
            use sa_math::x87::Cmp;
            let mut ok = b2.fcom(t1) != Cmp::Greater;
            if X87::from_f32(b1).fcom(X87::from_f32(t2)) == Cmp::Greater {
                ok = false;
            }
            if X87::from_f32(r1).fcom(X87::from_f32(l2)) == Cmp::Less {
                ok = false;
            }
            if X87::from_f32(l1).fcom(X87::from_f32(r2)) == Cmp::Greater {
                ok = false;
            }
            set_flag(s, ok);
        }
        // 05A6 GET_OBJECT_ROTATION_VELOCITY (0x48F461, gr: unregistered): `r = Multiply3x3(copy of o->m_vecTurnSpeed,
        // *o->m_matrix)` (0x59C810); SP0..2 = r * 50.0 (f32); StoreParameters(3) (tail 0x48F03B).
        0x05A6 => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let t = o.byte_add(0x50).cast::<u32>();
            let mut ts = [t.read(), t.wrapping_index(1).read(), t.wrapping_index(2).read()];
            let m = o.byte_add(0x14).cast::<GPtr<u8>>().read();
            let mut out = [0u32; 3];
            let r = GPtr::<u8>::cast::<u32>(multiply_3x3_vm(GPtr::<u32>::cast::<u8>(GPtr::from_raw(out.as_mut_ptr())), GPtr::<u32>::cast::<u8>(GPtr::from_raw(ts.as_mut_ptr())), m)).cast::<f32>();
            let r = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
            for (k, x) in r.iter().enumerate() {
                set_sp(k as i32, X87::from_f32(*x).fmul_m(f(K_50)).to_f32().to_bits());
            }
            store(s, 3);
        }
        // 05A7 ADD_VELOCITY_RELATIVE_TO_OBJECT_VELOCITY (0x48F4E9, gr: unregistered): `v = (SP1,SP2,SP3)`;
        // `v *= ms_fTimeStep` (0x40FEF0); `v /= 50.0` (0x411A30); static -> return 0 (0x48F550); `f = o->m_vecMoveSpeed`
        // normalised; `f.z == 1.0` (f64 compare) -> return 0; `r = Cross(f, (0,0,1))` normalised; `u = Cross(r, f)`
        // normalised; `m = o->m_vecMoveSpeed; m += r*v.x; m += f*v.y; m += u*v.z` (0x40FEC0 / 0x411A00); the dead
        // static re-check (the flag cannot have changed: kept as the original runs it); `o->m_vecMoveSpeed = m`.
        0x05A7 => {
            collect(s, 4);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let mut v = [sp(1), sp(2), sp(3)];
            let vp = GPtr::from_raw(v.as_mut_ptr());
            vector_mul_assign(vp.cast::<u8>(), f32::from_bits(rd(0xB7CB5C)));
            vector_div_assign(vp, K_50);
            if o.byte_add(0x1C).read() & 4 == 0 {
                let ms = o.byte_add(0x44).cast::<u32>();
                let mut fv = [ms.read(), ms.wrapping_index(1).read(), ms.wrapping_index(2).read()];
                let fp = GPtr::from_raw(fv.as_mut_ptr());
                vector_normalise(fp.cast::<[f32;3]>());
                let fz = X87::from_f32(f32::from_bits(fv[2]));
                if fz.fcom(X87::one()) != sa_math::x87::Cmp::Equal {
                    let mut fc = fv;
                    let mut up = [0u32, 0, 0x3F80_0000];
                    let mut tmp = [0u32; 3];
                    let c = GPtr::<u8>::cast::<u32>(cross_product(GPtr::<u32>::cast::<u8>(GPtr::from_raw(tmp.as_mut_ptr())), fp.cast::<u8>(), GPtr::<u32>::cast::<u8>(GPtr::from_raw(up.as_mut_ptr()))));
                    let mut r = [c.read(), c.wrapping_index(1).read(), c.wrapping_index(2).read()];
                    let rp = GPtr::from_raw(r.as_mut_ptr());
                    vector_normalise(rp.cast::<[f32;3]>());
                    let mut tmp2 = [0u32; 3];
                    let fcp = GPtr::from_raw(fc.as_mut_ptr());
                    let c = GPtr::<u8>::cast::<u32>(cross_product(GPtr::<u32>::cast::<u8>(GPtr::from_raw(tmp2.as_mut_ptr())), rp.cast::<u8>(), fcp.cast::<u8>()));
                    let mut u = [c.read(), c.wrapping_index(1).read(), c.wrapping_index(2).read()];
                    let up_ = GPtr::from_raw(u.as_mut_ptr());
                    vector_normalise(up_.cast::<[f32;3]>());
                    let mut m = [ms.read(), ms.wrapping_index(1).read(), ms.wrapping_index(2).read()];
                    let mp = GPtr::from_raw(m.as_mut_ptr());
                    let mut t1 = [0u32; 3];
                    let a = GPtr::<u8>::cast::<u32>(vector_mul(GPtr::<u32>::cast::<u8>(GPtr::from_raw(t1.as_mut_ptr())), rp.cast::<u8>(), f32::from_bits(v[0])));
                    vector_add_assign(mp.cast::<u8>(), a.cast::<u8>());
                    let mut t2 = [0u32; 3];
                    let a = GPtr::<u8>::cast::<u32>(vector_mul(GPtr::<u32>::cast::<u8>(GPtr::from_raw(t2.as_mut_ptr())), fcp.cast::<u8>(), f32::from_bits(v[1])));
                    vector_add_assign(mp.cast::<u8>(), a.cast::<u8>());
                    let mut t3 = [0u32; 3];
                    let a = GPtr::<u8>::cast::<u32>(vector_mul(GPtr::<u32>::cast::<u8>(GPtr::from_raw(t3.as_mut_ptr())), up_.cast::<u8>(), f32::from_bits(v[2])));
                    vector_add_assign(mp.cast::<u8>(), a.cast::<u8>());
                    unstatic(o);
                    for (k, x) in m.iter().enumerate() {
                        ms.wrapping_index(k as i32).write(*x);
                    }
                }
            }
        }
        // 05A8 GET_OBJECT_SPEED (0x48F73F, gr: unregistered): `SP0 = obj(SP0)->m_vecMoveSpeed.Magnitude() * 50.0`
        // (ST0 times m32, fstp f32); StoreParameters(1).
        0x05A8 => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let m = X87::from_f64_exact(vector_magnitude(GPtr::<u8>::cast::<u32>(o.byte_add(0x44))));
            set_sp(0, m.fmul_m(f(K_50)).to_f32().to_bits());
            store(s, 1);
        }
        // 05A9 SET_VAR_TEXT_LABEL (0x48F77F) / 05AA SET_LVAR_TEXT_LABEL (0x48F7B5): `p = GetPointerToScriptVariable(2 / 1)`;
        // 8-byte label (zeroed 16-byte buffer); `strncpy(p, label, 8)`.
        0x05A9 | 0x05AA => {
            let p = var_ptr(s, if command == 0x05A9 { 2 } else { 1 });
            let mut buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 8);
            crate::vm::strncpy(p.cast(), b, 8);
        }
        // 05AD IS_VAR_TEXT_LABEL_EQUAL_TO_TEXT_LABEL (0x48F7EB) / 05AE IS_LVAR_... (0x48F7EF): `p = var(2 / 1)`; 8-byte
        // label; flag = `strncmp(p, label, 8) == 0`.
        0x05AD | 0x05AE => {
            let p = var_ptr(s, if command == 0x05AD { 2 } else { 1 });
            let mut buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 8);
            let r = strncmp(p.cast(), b, 8);
            set_flag(s, r == 0);
        }
        // 05B0 GET_2D_LINES_INTERSECT_POINT (0x48F830, gr: unregistered): the register/memory sequence of the bytes
        // (see the spec); X = Y = -1000000.0 and flag 0 on failure; UpdateCompareFlag, then SP0 = X, SP1 = Y,
        // StoreParameters(2).
        0x05B0 => {
            collect(s, 8);
            two_d_lines_intersect(s);
        }
        // 05B6 SAVE_STRING_TO_DEBUG_FILE (0x48FAEC, gr: nop): copies 128 bytes at `m_IP` into a dead local buffer,
        // advancing `m_IP` by 128.
        0x05B6 => {
            let p = crate::vm::ip(s);
            crate::vm::set_ip(s, p.byte_add(0x80));
        }
        // 05B9 TASK_PAUSE (0x48FB0E): `new(0x18) CTaskSimplePause(SP1)`; give.
        0x05B9 => {
            collect(s, 2);
            let (ped, time) = (sp(0), sp(1));
            let t = new_task(0x18, |t| task_pause_ctor(t, time));
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05BA TASK_STAND_STILL (0x48FB82): `new(0x20) CTaskSimpleStandStill(time, looped, false, 8.0)`: SP1 == -1 ->
        // ([0x86DB24] = 20000, false), -2 -> (999999, true), else (SP1, false); give.
        0x05BA => {
            collect(s, 2);
            let (ped, time) = (sp(0), sp(1));
            let t = new_task(0x20, |t| match time as i32 {
                -1 => task_stand_still_ctor(t, rd(0x86DB24) as i32, 0, 0, f32::from_bits(0x4100_0000)),
                -2 => task_stand_still_ctor(t, 0xF423F, 1, 0, f32::from_bits(0x4100_0000)),
                v => task_stand_still_ctor(t, (v as u32) as i32, 0, 0, f32::from_bits(0x4100_0000)),
            });
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05BB TASK_FALL_AND_GET_UP (0x48FC9D): `new(0x18) CTaskComplexFallAndGetUp(SP1, SP2)`; give (tail 0x490BEF).
        0x05BB => {
            collect(s, 3);
            let (ped, a, b) = (sp(0), sp(1), sp(2));
            let t = new_task(0x18, |t| task_fall_and_get_up_ctor(t, a, b));
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05BC TASK_JUMP (0x48FCE7): `t = new(0x14) CTaskComplexJump(0)`; `byte [t+0x10] = SP1 != 0` (also when t is
        // null, as the original writes it); give.
        0x05BC => {
            collect(s, 2);
            let (ped, b) = (sp(0), sp(1));
            let t = new_task(0x14, |t| task_jump_ctor(t, 0));
            t.byte_add(0x10).write(u8::from(b != 0));
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05BD TASK_TIRED (0x48FD4A): `new(0x1C) CTaskSimpleTired(SP1)`; give.
        0x05BD => {
            collect(s, 2);
            let (ped, time) = (sp(0), sp(1));
            let t = new_task(0x1C, |t| task_tired_ctor(t, time));
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05BE TASK_DIE (0x48FD8D): `new(0x28) CTaskComplexDie(0, 0, 15, 4.0, 0, 0, 0, 0, 0)`; give.
        0x05BE => {
            collect(s, 1);
            let ped = sp(0);
            let t = new_task(0x28, |t| task_die_ctor(t, 0, 0, 0xF, f32::from_bits(0x4080_0000), f32::from_bits(0), 0, 0, 0, 0));
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05BF TASK_LOOK_AT_CHAR (0x48FE0F) / 05C0 TASK_LOOK_AT_VEHICLE (0x48FF19): `time = SP2 == -1 ? 20000 : SP2 == -2 ?
        // (05BF: -1, 05C0: 0x7FFFFFFF) : SP2`; `target = ped(SP1)` / `veh(SP1)`; SP0 != -1: `g_ikChainMan.LookAt(name,
        // ped(SP0), target, time, 5 / -1, null, false, 0.25, 500, 6, true)`; SP0 == -1: `new(0x30) 0x634440(target,
        // time, 5 / -1, (0,0,0), true, 0.25, 1000, 3)`, sequence add.
        0x05BF | 0x05C0 => {
            collect(s, 3);
            let char_ = command == 0x05BF;
            let time = match spi(2) {
                -1 => 0x4E20,
                -2 => {
                    if char_ {
                        u32::MAX
                    } else {
                        0x7FFF_FFFF
                    }
                }
                v => v as u32,
            };
            let ped = spi(0);
            let target = if char_ {
                ped_at_ref(pool(PED_POOL), spi(1))
            } else {
                vehicle_at_ref(pool(VEHICLE_POOL), spi(1))
            };
            let bone = if char_ { 5 } else { u32::MAX };
            if ped != -1 {
                let p = ped_at_ref(pool(PED_POOL), ped);
                let name = if char_ { 0x85A2EC } else { 0x85A2CC };
                ik_look_at(GPtr::from_addr(0xC15448), GPtr::from_addr(name as usize), p, target, time as i32, bone as i32, GPtr::NULL, 0, f32::from_bits(0x3E80_0000), 0x1F4, 6, 1);
            } else {
                let t = new_task(0x30, |t| {
                    task_trigger_look_at_ctor(t, target, time, bone, 0, 0, 0, 1, 0x3E80_0000, 0x3E8, 3)
                });
                sequence_add(t);
            }
        }
        // 05C1 TASK_SAY (0x490006) / 05C2 TASK_SHAKE_FIST (0x4900B2): `s = new(0x1C) CTaskSimpleSay(SP1, -1)` /
        // `new(0x10) CTaskSimpleShakeFist()`, slot 2 / 4; SP0 != -1: `ped(SP0)->m_pIntelligence->m_TaskMgr.
        // SetTaskSecondary(s, slot)` (no null check); else `new(0x14) CTaskSimpleAffectSecondaryBehaviour(true, slot,
        // s)`, sequence add.
        0x05C1 | 0x05C2 => {
            let (sec, slot) = if command == 0x05C1 {
                collect(s, 2);
                let id = sp(1);
                (new_task(0x1C, |t| task_say_ctor(t, id, u32::MAX)), 2)
            } else {
                collect(s, 1);
                (new_task(0x10, task_shake_fist_ctor), 4)
            };
            let ped = spi(0);
            if ped != -1 {
                let p = ped_at_ref(pool(PED_POOL), ped);
                let m = p.byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
                set_task_secondary(m, sec, slot as i32);
            } else {
                let t = new_task(0x14, |t| task_affect_secondary_ctor(t, 1, slot, sec));
                sequence_add(t);
            }
        }
        // 05C3 TASK_COWER (0x490143): `new(0x20) CTaskSimpleCower()`; give.
        0x05C3 => {
            collect(s, 1);
            let ped = sp(0);
            let t = new_task(0x20, task_cower_ctor);
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05C5 TASK_DUCK (0x4901D8): `new(0x28) CTaskSimpleDuck(0, len, -1)`: SP1 == -2 -> 0, -1 -> the word
        // [0x870A8C] (20000, zero-extended), else the SP1 dword; give.
        0x05C5 => {
            collect(s, 2);
            let (ped, len) = (sp(0), sp(1));
            let t = new_task(0x28, |t| match len as i32 {
                -2 => task_duck_ctor(t, 0, 0, u32::MAX),
                -1 => task_duck_ctor(t, 0, u32::from(GPtr::<u16>::from_addr(0x870A8C).read()), u32::MAX),
                v => task_duck_ctor(t, 0, v as u32, u32::MAX),
            });
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05C7 TASK_USE_ATM (0x49029B) / 05C8 TASK_SCRATCH_HEAD (0x4902D7): `new(0x20)` ctor 0x48DFE0 / 0x48DF30; give.
        0x05C7 | 0x05C8 => {
            collect(s, 1);
            let ped = sp(0);
            let t = if command == 0x05C7 {
                new_task(0x20, task_use_atm_ctor)
            } else {
                new_task(0x20, task_scratch_head_ctor)
            };
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05C9 TASK_LOOK_ABOUT (0x490313): time as 05C4; `new(0x34) CTaskSimpleLookAbout(time)`; give (0x490B62).
        0x05C9 => {
            collect(s, 2);
            let time = match spi(1) {
                -1 => 0x4E20,
                -2 => 0x7FFF_FFFF,
                v => v as u32,
            };
            let ped = sp(0);
            let t = new_task(0x34, |t| task_look_about_ctor(t, time));
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05CF TASK_LEAVE_CAR_AND_FLEE (0x4905AC): `new(0x28) CTaskComplexLeaveCarAndFlee(veh(SP1), &(SP2,SP3,SP4),
        // 0, 0, false)`; give.
        0x05CF => {
            collect(s, 5);
            let ped = sp(0);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let mut p = [sp(2), sp(3), sp(4)];
            let t = new_task(0x28, |t| task_leave_car_and_flee_ctor(t, v, GPtr::<u32>::cast::<u8>(GPtr::from_raw(p.as_mut_ptr())), 0, 0, 0));
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05D1 TASK_CAR_DRIVE_TO_COORD (0x490649): `v = SP1 < 0 ? null : veh(SP1)`; -100 ground rule on SP4;
        // `model = SP7 == 0 ? -1 : SP7 == 1 ? 0x19F : SP7`; `new(0x3C) CTaskComplexDriveToPoint(v, &(SP2,SP3,z), SP5,
        // SP6, model, -1.0, SP8)`; give.
        0x05D1 => {
            collect(s, 9);
            let ped = sp(0);
            let v = if spi(1) < 0 { GPtr::NULL } else { vehicle_at_ref(pool(VEHICLE_POOL), spi(1)) };
            let z = ground_rule(spf(2), spf(3), spf(4));
            let mut p = [sp(2), sp(3), z];
            let model = match sp(7) {
                0 => u32::MAX,
                1 => 0x19F,
                m => m,
            };
            let (speed, mode, style) = (sp(5), sp(6), sp(8));
            let t = new_task(0x3C, |t| {
                task_drive_to_point_ctor(t, v, GPtr::<u32>::cast::<u8>(GPtr::from_raw(p.as_mut_ptr())), speed, mode, model, 0xBF80_0000, style)
            });
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05D2 TASK_CAR_DRIVE_WANDER (0x490762): `v = SP1 < 0 ? null : veh(SP1)`; `new(0x24)
        // CTaskComplexCarDriveWander(v, SP3, SP2)`; give.
        0x05D2 => {
            collect(s, 4);
            let ped = sp(0);
            let (speed, style) = (sp(2), sp(3));
            let v = if spi(1) < 0 { GPtr::NULL } else { vehicle_at_ref(pool(VEHICLE_POOL), spi(1)) };
            let t = new_task(0x24, |t| task_car_drive_wander_ctor(t, v, style, f32::from_bits(speed)));
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05D3 TASK_GO_STRAIGHT_TO_COORD (0x4907CE): radii [0x86FC84] / [0x86FC88]; SP5 == -2: `new(0x28)
        // CTaskComplexGoToPointAndStandStill(SP4, pt, r1, r2, false, true)`; -1: `new(0x38) ...Timed(SP4, pt, r1, r2,
        // [0x86FC8C])`; else `...Timed(SP4, pt, r1, r2, SP5)`; give.
        0x05D3 => {
            collect(s, 6);
            let ped = sp(0);
            let mut p = [sp(1), sp(2), sp(3)];
            let pp = GPtr::from_raw(p.as_mut_ptr());
            let (r1, r2) = (rd(0x86FC84), rd(0x86FC88));
            let (ms, time) = (sp(4), sp(5));
            let t = match time as i32 {
                -2 => new_task(0x28, |t| task_go_to_point_and_stand_still_ctor(t, ms, pp, f32::from_bits(r1), f32::from_bits(r2), 0, 1)),
                -1 => new_task(0x38, |t| task_go_to_point_and_stand_still_timed_ctor(t, ms, pp, r1, r2, rd(0x86FC8C))),
                v => new_task(0x38, |t| task_go_to_point_and_stand_still_timed_ctor(t, ms, pp, r1, r2, v as u32)),
            };
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05D4 TASK_ACHIEVE_HEADING (0x49090B): `a = SP1 * 0.017453292` (f32); `new(0x18) CTaskSimpleAchieveHeading(a,
        // [0x86FC7C], [0x86FC80])`; give.
        0x05D4 => {
            collect(s, 2);
            let a = ld(1).fmul_m(f(K_DEG2RAD)).to_f32().to_bits();
            let ped = sp(0);
            let t = new_task(0x18, |t| task_achieve_heading_ctor(t, f32::from_bits(a), rd(0x86FC7C), rd(0x86FC80)));
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05D6 FLUSH_ROUTE (0x490981): the script `CPointRoute`'s count [0xC18D50] = 0.
        0x05D6 => GPtr::<u32>::from_addr(0xC18D50).write(0),
        // 05D7 EXTEND_ROUTE (0x490992): `0x48E180(&route 0xC18D50, &(SP0, SP1, SP2))`.
        0x05D7 => {
            collect(s, 3);
            let mut p = [sp(0), sp(1), sp(2)];
            point_route_add(GPtr::from_addr(0xC18D50), GPtr::<u32>::cast::<u8>(GPtr::from_raw(p.as_mut_ptr())));
        }
        // 05D8 TASK_FOLLOW_POINT_ROUTE (0x4909D4): `new(0x3C) CTaskComplexFollowPointRoute(SP1, route 0xC18D50, SP2,
        // [0x86FC98], [0x86FC9C], false, true, true)`; give.
        0x05D8 => {
            collect(s, 3);
            let (ped, ms, mode) = (sp(0), sp(1), sp(2));
            let t = new_task(0x3C, |t| {
                task_follow_point_route_ctor(t, ms, GPtr::from_addr(0xC18D50), mode, rd(0x86FC98), rd(0x86FC9C), 0, 1, 1)
            });
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05D9 TASK_GOTO_CHAR (0x490A55): `target = ped(SP1)`; `time = SP2 == -2 ? -1 : SP2 == -1 ? 50000 : SP2`;
        // `new(0x4C) CTaskComplexSeekEntity<Standard>(target, time, 1000, SP3, [0x859E30], [0x859E34], true, true)`;
        // give.
        0x05D9 => {
            collect(s, 4);
            let (ped, dist) = (sp(0), sp(3));
            let target = ped_at_ref(pool(PED_POOL), spi(1));
            let time = match spi(2) {
                -2 => u32::MAX,
                -1 => 0xC350,
                v => v as u32,
            };
            let t = new_task(0x4C, |t| {
                task_seek_entity_ctor(t, target, time, 0x3E8, f32::from_bits(dist), rd(0x859E30), rd(0x859E34), 1, 1)
            });
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05DA TASK_FLEE_POINT (0x490AE6): `new(0x3C) CTaskComplexFleePoint(&(SP1,SP2,SP3), true, SP4, SP5)`; give.
        0x05DA => {
            collect(s, 6);
            let ped = sp(0);
            let mut p = [sp(1), sp(2), sp(3)];
            let (dist, time) = (sp(4), sp(5));
            let t = new_task(0x3C, |t| task_flee_point_ctor(t, GPtr::<u32>::cast::<u8>(GPtr::from_raw(p.as_mut_ptr())), 1, dist, time));
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        // 05DB TASK_FLEE_CHAR (0x490B7E): `d = (float)SP2` (fild, fstp f32); `new(0x3C) CTaskComplexFleeEntity(ped(SP1),
        // true, d, SP3, [0x86F664], [0x86F668])`; give.
        0x05DB => {
            collect(s, 4);
            let d = X87::from_i32(spi(2)).to_f32().to_bits();
            let (ped, time) = (sp(0), sp(3));
            let e = ped_at_ref(pool(PED_POOL), spi(1));
            let t = new_task(0x3C, |t| task_flee_entity_ctor(t, e, 1, d, time, rd(0x86F664), rd(0x86F668)));
            give_ped_scripted_task(s, ped as i32, t, command);
        }
        _ => return -1,
    }
    0
}

/// 05B0 GET_2D_LINES_INTERSECT_POINT (0x48F830) after `CollectParameters(8)`: x1..y4 = SP0..SP7, x1 kept in a
/// register, the others reloaded from their f32 slots; compares against the f64 0.0 / 0.01 constants
/// (`[0x859EF8]`, `[0x85A308]`). Steps as numbered in the spec.
fn two_d_lines_intersect(s: Scr) {
    use sa_math::x87::Cmp;
    let (x1, y1, x2, y2, x3, y3, x4, y4) = (spf(0), spf(1), spf(2), spf(3), spf(4), spf(5), spf(6), spf(7));
    let zero = X87::zero();
    let eq0 = |v: X87| v.fcom(zero) == Cmp::Equal;
    let x1r = X87::from_f32(x1);
    // 1. dx1 (register); den1 (memory) = dx1 == 0 ? 1e-6 : dx1
    let dx1 = x1r.fsub_m(x2);
    let den1 = if eq0(dx1) { f(K_1EM6) } else { dx1.to_f32() };
    // 2. dx2 = x3 - x4, == 0 -> fld 1e-6
    let dx2 = X87::from_f32(x3).fsub_m(x4);
    let dx2 = if eq0(dx2) { X87::constant(K_1EM6) } else { dx2 };
    // 3. dy1 (stored), m1 = dy1 / den1 (register), m2 = (y3 - y4) / dx2 (stored)
    let dy1r = X87::from_f32(y1).fsub_m(y2);
    let dy1 = dy1r.to_f32();
    let m1 = dy1r.fdiv_m(den1);
    let m2 = X87::from_f32(y3).fsub_m(y4).fdiv(dx2).to_f32();
    // c1 = y1 - m1*x1 (register x1), c2 = y3 - m2*x3 (stored)
    let c1 = m1.fmul(x1r).fsubr_m(y1).to_f32();
    let c2 = X87::from_f32(m2).fmul_m(x3).fsubr_m(y3).to_f32();
    // 4. det = (-m1) - (-m2) (stored); num = (-m1)*c2 - (-m2)*c1 (stored)
    let nm1 = m1.fchs();
    let nm2 = X87::from_f32(m2).fchs();
    let det = nm1.fsub(nm2).to_f32();
    let num = nm1.fmul_m(c2).fsub(nm2.fmul_m(c1)).to_f32();
    let (ok, mut xr, mut yr) = if eq0(X87::from_f32(det)) {
        (false, 0f32, 0f32)
    } else {
        // 5. X, Y
        let xr = if eq0(dx1) { x1r.to_f32() } else { X87::from_f32(c1).fsub_m(c2).fdiv_m(det).to_f32() };
        let yr = if eq0(X87::from_f32(dy1)) { y1 } else { X87::from_f32(num).fdiv_m(det).to_f32() };
        // 6. sort each pair (swap when first > second); x1 stays in the register
        let gt = |a: X87, b: f32| a.fcom(X87::from_f32(b)) == Cmp::Greater;
        let (xlo1, xhi1) = if gt(x1r, x2) { (X87::from_f32(x2), x1r.to_f32()) } else { (x1r, x2) };
        let (ylo1, yhi1) = if gt(X87::from_f32(y1), y2) { (y2, y1) } else { (y1, y2) };
        let (xlo2, xhi2) = if gt(X87::from_f32(x3), x4) { (x4, x3) } else { (x3, x4) };
        let (ylo2, yhi2) = if gt(X87::from_f32(y3), y4) { (y4, y3) } else { (y3, y4) };
        // 7. margins (f64 0.01): `lo - 0.01 <= v` (fail on Greater/Unordered), `hi + 0.01 >= v` (fail on Less/Unordered)
        let k = X87::from_f64_exact(K_D001);
        let le = |lo: X87, v: f32| matches!(lo.fsub(k).fcom(X87::from_f32(v)), Cmp::Less | Cmp::Equal);
        let ge = |hi: f32, v: f32| matches!(X87::from_f32(hi).fadd(k).fcom(X87::from_f32(v)), Cmp::Greater | Cmp::Equal);
        let ok = le(xlo1, xr)
            && ge(xhi1, xr)
            && le(X87::from_f32(xlo2), xr)
            && ge(xhi2, xr)
            && le(X87::from_f32(ylo1), yr)
            && ge(yhi1, yr)
            && le(X87::from_f32(ylo2), yr)
            && ge(yhi2, yr);
        (ok, xr, yr)
    };
    if !ok {
        xr = f32::from_bits(0xC974_2400);
        yr = xr;
    }
    set_flag(s, ok);
    set_sp(0, xr.to_bits());
    set_sp(1, yr.to_bits());
    store(s, 2);
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1400_TO_1499];
