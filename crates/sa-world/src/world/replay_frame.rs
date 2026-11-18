//! Replay playback, one frame at a time: `CReplay::PlayBackThisFrameInterpolation` and the replay camera it leaves
//! the frame with.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::field::{copy3, gf as k};
use sa_core::mem::GPtr;
use sa_core::pools::call::vehicle_new_at;
use sa_core::{hook, orig};

use super::common::{entity_pos, g, rd, wr, x};
use super::pickups::entity_vdelete;
use super::replay_peds::{deal_with_new_ped_packet, matrix_add_assign, model_loaded, ped_conversion, process_ped_update};
use super::replay_playback::finish_playback;
use super::replay_setup::{
    pool_slot, BUFFERS, BUFFER_SIZE, BUFFER_STATUS, PED_POOL, PED_POOL_CONVERSION, VEHICLE_POOL,
    VEHICLE_POOL_CONVERSION,
};
use crate::bindings::{
    calculate_derived_values, find_player_coors, find_player_ped, find_player_vehicle, matrix_copy_ctor, matrix_dtor,
    process_line_of_sight, rw_frame_ortho_normalize, set_game_clock, world_add, world_remove,
};
use sa_core::bindings::{get_pad, rw_camera_set_near_clip_plane};
use sa_math::fpu::{ge_or_nan, gt, le_or_nan, lt};
use sa_core::bindings::{
    automobile_ctor, bike_ctor, bmx_ctor, boat_ctor, cross_product, decompress_into_full_matrix, heli_ctor,
    monster_truck_ctor, plane_ctor, quad_bike_ctor, rw_frame_update_objects, rw_matrix_update, streaming_request_model,
    trailer_ctor, train_ctor, vector_normalise,
};

orig! {
    /// `CBulletTraces::Init()` (0x721D50)
    fn bullet_traces_init() = cdecl 0x721D50 => unported;
    /// `tReplayVehicleBlock::ExtractVehicleUpdateData(CVehicle*, float)` (0x45BA90)
    fn extract_vehicle_update_data(block: GPtr<u8>, veh: GPtr<u8>, t: f32) = thiscall 0x45BA90 => unported;
    /// `tReplayBikeBlock::ExtractBikeUpdateData(CBike*, float)` (0x45C0D0)
    fn extract_bike_update_data(block: GPtr<u8>, veh: GPtr<u8>, t: f32) = thiscall 0x45C0D0 => unported;
    /// `tReplayBmxBlock::ExtractBmxUpdateData(CBmx*, float)` (0x45BFF0)
    fn extract_bmx_update_data(block: GPtr<u8>, veh: GPtr<u8>, t: f32) = thiscall 0x45BFF0 => unported;
    /// `tReplayTrainBlock::ExtractTrainUpdateData(CTrain*, float)` (0x45CDB0)
    fn extract_train_update_data(block: GPtr<u8>, veh: GPtr<u8>, t: f32) = thiscall 0x45CDB0 => unported;
}

/// `TheCamera` and its `m_pRwCamera` (+0x954).
const THE_CAMERA: usize = 0xB6_F028;
const RW_CAMERA: usize = 0xB6_F97C;
/// `TheCamera.m_mCameraMatrix` rows: right, forward, up, position.
const CAM_RIGHT: usize = 0xB6_F99C;
const CAM_FORWARD: usize = 0xB6_F9AC;
const CAM_UP: usize = 0xB6_F9BC;
const CAM_POS: usize = 0xB6_F9CC;
/// `TheCamera.m_vecGameCamPos`.
const GAME_CAM_POS: usize = 0xB6_F930;
/// `CReplay::CameraFocus` x / y / z (stored in that descending order).
pub const FOCUS_X: usize = 0x97_F678;
pub const FOCUS_Y: usize = 0x97_F674;
pub const FOCUS_Z: usize = 0x97_F670;
/// `CReplay::CameraFixed` x / y / z.
const FIXED_X: usize = 0x97_FB2C;
const FIXED_Y: usize = 0x97_FB28;
const FIXED_Z: usize = 0x97_FB24;
/// `CReplay::CameraMode` (u8; 1 top-down, 2 fixed).
const CAMERA_MODE: usize = 0x97_FB30;
/// `CReplay::bIsUsingRemoteCar`.
pub const IS_USING_REMOTE_CAR: usize = 0x97_F66D;
const ONE: usize = 0x85_8624;
const FIFTEEN: usize = 0x85_8B48;

/// The RW frame of `TheCamera.m_pRwCamera`.
pub fn camera_frame() -> GPtr<u8> {
    rd::<GPtr<u8>>(g::<GPtr<u8>>(RW_CAMERA).read(), 4)
}

/// The camera matrix rows copied into the RW modelling matrix (`CMatrix` forward is RW's at, up is RW's up).
pub fn camera_matrix_to_rw(modelling: GPtr<u8>) {
    copy3(modelling.byte_add(0x30), GPtr::from_addr(CAM_POS));
    copy3(modelling.byte_add(0x20), GPtr::from_addr(CAM_FORWARD));
    copy3(modelling.byte_add(0x10), GPtr::from_addr(CAM_UP));
    copy3(modelling, GPtr::from_addr(CAM_RIGHT));
}

/// The camera's derived values recomputed and its RW frame brought up to date.
pub fn refresh_camera_frame(frame: GPtr<u8>) {
    calculate_derived_values(GPtr::from_addr(THE_CAMERA), 0, 1);
    rw_matrix_update(frame.byte_add(0x10));
    rw_frame_update_objects(frame);
    rw_frame_ortho_normalize(frame);
}

fn write3(at: usize, v: [f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        g::<f32>(at + 4 * i).write(*c);
    }
}

fn normalise(v: &mut [f32; 3]) {
    vector_normalise(GPtr::from_raw(v as *mut [f32; 3]));
}

fn cross(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    let mut out = [0f32; 3];
    let p = |v: &[f32; 3]| GPtr::<u8>::from_raw(v.as_ptr().cast_mut().cast::<u8>());
    cross_product(GPtr::from_raw(out.as_mut_ptr().cast::<u8>()), p(a), p(b));
    out
}

/// The camera looking from `from` at `to`: forward = |to - from|, right = |(0, 0, 1) x forward|, up = |forward x
/// right| (each component difference stored as an f32).
pub fn look_at_rows(to: [f32; 3], from: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let mut forward = [0f32; 3];
    for i in 0..3 {
        forward[i] = x(to[i]).fsub_m(from[i]).to_f32();
    }
    normalise(&mut forward);
    let mut right = cross(&[0.0, 0.0, 1.0], &forward);
    normalise(&mut right);
    let mut up = cross(&forward, &right);
    normalise(&mut up);
    (forward, up, right)
}

fn focus() -> [f32; 3] {
    [g::<f32>(FOCUS_X).read(), g::<f32>(FOCUS_Y).read(), g::<f32>(FOCUS_Z).read()]
}

/// `CReplay::ProcessReplayCamera()` (0x45D060, cdecl). Top-down (mode 1): the camera 15 above the focus, forward
/// (0, 0, -1), up (0, 1, 0), right (1, 0, 0). Fixed (mode 2): at `CameraFixed`, looking at the focus
/// ([`look_at_rows`]). Both copy the rows to the RW matrix. Every mode: `m_vecGameCamPos` = the camera position,
/// `CalculateDerivedValues(false, true)`, `RwMatrixUpdate`, `RwFrameUpdateObjects`, `RwFrameOrthoNormalize`.
#[hook(addr = 0x45D060, conv = cdecl, name = "CReplay::ProcessReplayCamera", expect = "A1 7C F9 B6 00 83 EC 48",
       unverified = "frontier2-b32", route = "L2", hazards = "heap-writes")]
pub fn process_replay_camera() {
    let frame = camera_frame();
    match g::<u8>(CAMERA_MODE).read() {
        1 => {
            let f = focus();
            write3(CAM_POS, [f[0], f[1], x(f[2]).fadd_m(g::<f32>(FIFTEEN).read()).to_f32()]);
            write3(CAM_FORWARD, [0.0, 0.0, -1.0]);
            write3(CAM_UP, [0.0, 1.0, 0.0]);
            write3(CAM_RIGHT, [1.0, 0.0, 0.0]);
            camera_matrix_to_rw(frame.byte_add(0x10));
        }
        2 => {
            for (dst, src) in [(CAM_POS, FIXED_X), (CAM_POS + 4, FIXED_Y), (CAM_POS + 8, FIXED_Z)] {
                g::<u32>(dst).write(g::<u32>(src).read());
            }
            let fixed = [g::<f32>(FIXED_X).read(), g::<f32>(FIXED_Y).read(), g::<f32>(FIXED_Z).read()];
            let (forward, up, right) = look_at_rows(focus(), fixed);
            write3(CAM_FORWARD, forward);
            write3(CAM_UP, up);
            write3(CAM_RIGHT, right);
            camera_matrix_to_rw(frame.byte_add(0x10));
        }
        _ => {}
    }
    copy3(GPtr::from_addr(GAME_CAM_POS), entity_pos(GPtr::from_addr(THE_CAMERA)));
    refresh_camera_frame(frame);
}

/// The vehicle a playback packet creates: marked as played back from the buffer (status 1), used for the replay,
/// without collision, its matrix decompressed from the packet (+4), its colours (+0x2D / +0x2E), a boat's +0x5AC
/// bit 2 cleared when `boat_flag` (only the plain vehicle packet does that), and added to the world.
fn set_up_replay_vehicle(veh: GPtr<u8>, packet: GPtr<u8>, boat_flag: bool) {
    wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 8);
    let flags = rd::<u32>(veh, 0x1C);
    let matrix = rd::<GPtr<u8>>(veh, 0x14);
    wr::<u8>(veh, 0x42F, rd::<u8>(veh, 0x42F) | 8);
    wr::<u32>(veh, 0x1C, flags & !1);
    decompress_into_full_matrix(packet.byte_add(4), matrix);
    wr::<u8>(veh, 0x434, rd::<u8>(packet, 0x2D));
    wr::<u8>(veh, 0x435, rd::<u8>(packet, 0x2E));
    if boat_flag && rd::<u32>(veh, 0x594) == 5 {
        wr::<u8>(veh, 0x5AC, rd::<u8>(veh, 0x5AC) & 0xFB);
    }
    world_add(veh);
}

fn vehicle_slot(index: i32) -> GPtr<u8> {
    pool_slot(VEHICLE_POOL, 0xA18, index)
}

/// `new (index << 8)` of `size` bytes, constructed by `ctor` when the pool gave memory (null otherwise).
fn new_vehicle(size: u32, index: i32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let mem = vehicle_new_at(size, index << 8);
    if mem.is_null() {
        GPtr::NULL
    } else {
        ctor(mem)
    }
}

/// A vehicle packet: when the recording's vehicle (byte +1 through the conversion table) has no pool vehicle yet, it
/// is created by `create` if its model (int16 +0x1A) is loaded, else the model is requested. Then, in the same
/// frame, the pool vehicle (the one just created included) gets `update`.
fn vehicle_packet(
    packet: GPtr<u8>,
    create: impl FnOnce(i32, i32),
    update: impl FnOnce(GPtr<u8>),
) {
    let conversion = |p: GPtr<u8>| g::<i32>(VEHICLE_POOL_CONVERSION + 4 * usize::from(rd::<u8>(p, 1))).read();
    let index = conversion(packet);
    if vehicle_slot(index).is_null() {
        let model = i32::from(rd::<i16>(packet, 0x1A));
        if model_loaded(model) {
            create(index, model);
        } else {
            streaming_request_model(model, 0);
        }
    }
    let veh = vehicle_slot(conversion(packet));
    if !veh.is_null() {
        update(veh);
    }
}

/// Each `CReplay::m_pBuffer` advance of a packet type, for the packets that only add a fixed size.
fn advance(buffer: GPtr<u8>, by: u32) {
    wr::<u32>(buffer, 0, rd::<u32>(buffer, 0).wrapping_add(by));
}

/// `CReplay::PlayBackThisFrameInterpolation(CAddressInReplayBuffer& buffer, float interpolation, uint32* outTimer)`
/// (0x45F380, cdecl, AL). `CBulletTraces::Init`, then the packets at `buffer` (+0 offset, +4 base, +8 slot) up to
/// the end-of-frame packet (8; skipped, then [`process_replay_camera`] and false):
/// - 0, end of a buffer: when its status is 2 (the last one recorded) `FinishPlayback` and true; else the next
///   buffer (slot + 1 mod 8) from its start;
/// - 1 vehicle (0x34 bytes): created by subtype (+0x30): 0 `CAutomobile`, 1 `CMonsterTruck`, 2 `CQuadBike`, 5 `CBoat`,
///   6 `CTrain`, 11 `CTrailer`; another subtype creates nothing but still runs the set-up on the vehicle register
///   left from earlier (the buffer itself at first, the last vehicle created or deleted later); then
///   `ExtractVehicleUpdateData`;
/// - 2 bike (0x38), 15 BMX (0x38), 16 heli (0x38; rotor speed +0x84C from +0x34), 17 plane (0x3C; +0x9C8 / +0x9C4
///   from +0x38 / +0x34), 18 train (0x4C): created as that class, then their update;
/// - 3 new ped (`DealWithNewPedPacket(packet, false, null)`, 8), 4 ped update (`ProcessPedUpdate`, which advances);
/// - 5 camera (0x58): the camera matrix blended with the packet's (+4) by the interpolation and copied to RW, the
///   focus blended with +0x4C..+0x54, `bIsUsingRemoteCar` = byte +1 != 0;
/// - 6 clock (`SetGameClock(+1, +2, 0)`, 4), 7 weather (old / new type and interpolation, 8), 9 timer (`*outTimer`
///   when given and `CTimer::m_snTimeInMilliseconds` = +4, 8), 10 bullet trace (index +3, start / end from +4,
///   0x1C), 12 misc (camera shake start / force, video / lift cam bits, `CGame::currArea`, 0x10), 13 / 14 deleted
///   vehicle / ped (int16 +2 through the conversion table: removed from the world and deleted, 4), 19 (0x78);
/// - 11 and unknown types: nothing, the same packet again (the original loops on it forever).
#[hook(addr = 0x45F380, conv = cdecl, name = "CReplay::PlayBackThisFrameInterpolation", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-b32", route = "L2", hazards = "heap-writes",
       note = "replay playback, every frame (PlayBackThisFrame / FastForwardToTime)")]
pub fn play_back_this_frame_interpolation(buffer: GPtr<u8>, interpolation: f32, out_timer: GPtr<u32>) -> u8 {
    bullet_traces_init();
    // EBP of the original: set by every vehicle creation and deleted-vehicle packet, read by the vehicle packet's
    // set-up even when its subtype created nothing.
    let mut last_vehicle = buffer;
    loop {
        let packet = rd::<GPtr<u8>>(buffer, 4).byte_add(rd::<u32>(buffer, 0) as isize);
        let ty = rd::<u8>(packet, 0);
        if ty == 8 {
            advance(buffer, 4);
            process_replay_camera();
            return 0;
        }
        match ty {
            0 => {
                let slot = rd::<u8>(buffer, 8);
                if g::<u8>(BUFFER_STATUS + usize::from(slot)).read() == 2 {
                    finish_playback();
                    return 1;
                }
                let next = (u32::from(slot) + 1) & 7;
                wr::<u8>(buffer, 8, next as u8);
                wr::<u32>(buffer, 0, 0);
                wr::<u32>(buffer, 4, (BUFFERS as u32).wrapping_add(next * BUFFER_SIZE as u32));
            }
            1 => {
                vehicle_packet(
                    packet,
                    |index, model| {
                        let created = match rd::<u8>(packet, 0x30) {
                            0 => Some(new_vehicle(0x988, index, |m| automobile_ctor(m, model as u32, 2, 1))),
                            1 => Some(new_vehicle(0x99C, index, |m| monster_truck_ctor(m, model, 2))),
                            2 => Some(new_vehicle(0x9BC, index, |m| quad_bike_ctor(m, model, 2))),
                            5 => Some(new_vehicle(0x7E8, index, |m| boat_ctor(m, model, 2))),
                            6 => Some(new_vehicle(0x6AC, index, |m| train_ctor(m, model as u32, 2))),
                            11 => Some(new_vehicle(0x9F4, index, |m| trailer_ctor(m, model, 2))),
                            _ => None,
                        };
                        if let Some(veh) = created {
                            last_vehicle = veh;
                        }
                        set_up_replay_vehicle(last_vehicle, packet, true);
                    },
                    |veh| extract_vehicle_update_data(packet, veh, interpolation),
                );
                advance(buffer, 0x34);
            }
            2 | 15 | 16 | 17 | 18 => {
                vehicle_packet(
                    packet,
                    |index, model| {
                        let veh = match ty {
                            2 => new_vehicle(0x814, index, |m| bike_ctor(m, model, 2)),
                            15 => new_vehicle(0x838, index, |m| bmx_ctor(m, model, 2)),
                            16 => new_vehicle(0xA18, index, |m| heli_ctor(m, model, 2)),
                            17 => new_vehicle(0xA04, index, |m| plane_ctor(m, model, 2)),
                            _ => new_vehicle(0x6AC, index, |m| train_ctor(m, model as u32, 2)),
                        };
                        last_vehicle = veh;
                        set_up_replay_vehicle(veh, packet, false);
                    },
                    |veh| match ty {
                        2 => extract_bike_update_data(packet, veh, interpolation),
                        15 => extract_bmx_update_data(packet, veh, interpolation),
                        16 => {
                            extract_vehicle_update_data(packet, veh, interpolation);
                            wr::<u32>(veh, 0x84C, rd::<u32>(packet, 0x34));
                        }
                        17 => {
                            extract_vehicle_update_data(packet, veh, interpolation);
                            wr::<u32>(veh, 0x9C8, rd::<u32>(packet, 0x38));
                            wr::<u32>(veh, 0x9C4, rd::<u32>(packet, 0x34));
                        }
                        _ => extract_train_update_data(packet, veh, interpolation),
                    },
                );
                advance(buffer, match ty { 17 => 0x3C, 18 => 0x4C, _ => 0x38 });
            }
            3 => {
                deal_with_new_ped_packet(packet, 0, GPtr::NULL);
                advance(buffer, 8);
            }
            4 => {
                let ped = pool_slot(PED_POOL, 0x7C4, ped_conversion(rd::<u8>(packet, 1)));
                process_ped_update(ped, interpolation, buffer);
            }
            5 => play_back_camera_packet(buffer, packet, interpolation),
            6 => {
                set_game_clock(u32::from(rd::<u8>(packet, 1)), u32::from(rd::<u8>(packet, 2)), 0);
                advance(buffer, 4);
            }
            7 => {
                g::<u16>(0xC8_1320).write(u16::from(rd::<u8>(packet, 1)));
                g::<u16>(0xC8_131C).write(u16::from(rd::<u8>(packet, 2)));
                g::<u32>(0xC8_130C).write(rd::<u32>(packet, 4));
                advance(buffer, 8);
            }
            9 => {
                if !out_timer.is_null() {
                    out_timer.write(rd::<u32>(packet, 4));
                }
                g::<u32>(0xB7_CB84).write(rd::<u32>(packet, 4));
                advance(buffer, 8);
            }
            10 => {
                let trace = |o: usize| 0xC7_C748 + usize::from(rd::<u8>(packet, 3)) * 0x2C + o;
                g::<u8>(trace(0x18)).write(1);
                for o in (0..0x18).step_by(4) {
                    g::<u32>(trace(o)).write(rd::<u32>(packet, 4 + o as isize));
                }
                advance(buffer, 0x1C);
            }
            12 => {
                g::<u32>(0xB6_F084).write(rd::<u32>(packet, 4));
                g::<u32>(0xB6_F154).write(rd::<u32>(packet, 8));
                let bits = rd::<u8>(packet, 0xD);
                g::<u8>(0xC7_C70C).write(bits & 1);
                g::<u8>(0xC7_C70D).write((bits >> 1) & 1);
                g::<u32>(0xB7_2914).write(u32::from(rd::<u8>(packet, 0xC)));
                advance(buffer, 0x10);
            }
            13 => {
                let index = g::<i32>(table_entry(VEHICLE_POOL_CONVERSION, packet)).read();
                if index >= 0 {
                    let veh = vehicle_slot(index);
                    last_vehicle = veh;
                    if !veh.is_null() {
                        world_remove(veh);
                        entity_vdelete(veh, 1);
                    }
                }
                advance(buffer, 4);
            }
            14 => {
                let index = g::<i32>(table_entry(PED_POOL_CONVERSION, packet)).read();
                if index >= 0 {
                    let ped = pool_slot(PED_POOL, 0x7C4, index);
                    if !ped.is_null() {
                        world_remove(ped);
                        entity_vdelete(ped, 1);
                    }
                }
                advance(buffer, 4);
            }
            19 => advance(buffer, 0x78),
            _ => {}
        }
    }
}

/// The conversion table entry of a deleted entity packet's int16 index (+2).
fn table_entry(table: usize, packet: GPtr<u8>) -> usize {
    (table as isize + 4 * isize::from(rd::<i16>(packet, 2))) as usize
}

/// The camera packet (type 5): `m_mCameraMatrix` rows scaled by `1 - t` (x87, each stored as an f32), a stack copy of
/// the packet's matrix (+4) scaled by `t` and added, the rows copied to RW; the focus = `t * packet + focus * (1 - t)`
/// with `1 - t` rounded to an f32; `bIsUsingRemoteCar`; the offset advanced before the copy is destroyed.
fn play_back_camera_packet(buffer: GPtr<u8>, packet: GPtr<u8>, t: f32) {
    let rest = x(g::<f32>(ONE).read()).fsub_m(t);
    let rest_f32 = rest.to_f32();
    for row in [CAM_RIGHT, CAM_FORWARD, CAM_UP, CAM_POS] {
        for c in 0..3 {
            let cell = g::<f32>(row + 4 * c);
            cell.write(rest.fmul_m(cell.read()).to_f32());
        }
    }
    let mut m = [0u32; 0x48 / 4];
    let tmp = GPtr::from_raw(m.as_mut_ptr().cast::<u8>());
    matrix_copy_ctor(tmp, packet.byte_add(4));
    for row in [0isize, 0x10, 0x20, 0x30] {
        for c in 0..3 {
            let o = row + 4 * c;
            wr::<f32>(tmp, o, x(t).fmul_m(rd::<f32>(tmp, o)).to_f32());
        }
    }
    matrix_add_assign(GPtr::from_addr(CAM_RIGHT), tmp);
    camera_matrix_to_rw(camera_frame().byte_add(0x10));
    for (cell, o) in [(FOCUS_X, 0x4Cisize), (FOCUS_Y, 0x50), (FOCUS_Z, 0x54)] {
        let blended = x(t).fmul_m(rd::<f32>(packet, o)).fadd(x(g::<f32>(cell).read()).fmul_m(rest_f32));
        g::<f32>(cell).write(blended.to_f32());
    }
    g::<u8>(IS_USING_REMOTE_CAR).write(u8::from(rd::<u8>(packet, 1) != 0));
    advance(buffer, 0x58);
    matrix_dtor(tmp);
}

/// `CReplay::FramesActiveLookAroundCam`, the look-around distance / view angle / direction angle and
/// `CReplay::bAllowLookAroundCam`.
const FRAMES_ACTIVE: usize = 0x97_FAD0;
const LOOK_DISTANCE: usize = 0x97_FAD4;
const LOOK_VIEW_ANGLE: usize = 0x97_FAD8;
const LOOK_DIR_ANGLE: usize = 0x97_FADC;
const ALLOW_LOOK_AROUND: usize = 0x97_FAE0;
/// Pad 0's `NewMouseControllerState`: left / right button bytes, the x / y movement.
const MOUSE_LEFT: usize = 0xB7_3418;
const MOUSE_RIGHT: usize = 0xB7_3419;
const MOUSE_X: usize = 0xB7_3424;
const MOUSE_Y: usize = 0xB7_3428;
/// `Scene.m_pRwCamera`.
const SCENE_CAMERA: usize = 0xC1_703C;

/// `|v| > 0.01` as the original tests it (the sign flipped unless `v >= 0`).
fn moved(v: f32) -> bool {
    let v = x(v);
    let a = if ge_or_nan(v, x(k(0x85_8B50))) { v } else { v.fchs() };
    gt(a, x(k(0x85_8C58)))
}

/// The distance and direction from the player to the camera when the look-around starts: the distance (x87,
/// stored as an f32) clamped to 3..15 (the upper bound tested before rounding), the angle `atan2(cam.x - player.x,
/// cam.y - player.y)` from two further `FindPlayerCoors` calls.
fn start_look_around() {
    let cam = entity_pos(GPtr::from_addr(THE_CAMERA));
    let mut tmp = [0f32; 3];
    let out = GPtr::<f32>::from_raw(tmp.as_mut_ptr());
    let p = find_player_coors(out, -1);
    let d = [0, 1, 2].map(|i| x(p.wrapping_index(i).read()).fsub_m(rd::<f32>(cam, 4 * i as isize)));
    let dist = d[2].fmul(d[2]).fadd(d[1].fmul(d[1])).fadd(d[0].fmul(d[0])).fsqrt();
    g::<f32>(LOOK_DISTANCE).write(dist.to_f32());
    let clamped = if ge_or_nan(dist, x(k(FIFTEEN))) {
        k(FIFTEEN)
    } else {
        let stored = g::<f32>(LOOK_DISTANCE).read();
        if gt(x(stored), x(k(0x85_8B3C))) { stored } else { k(0x85_8B3C) }
    };
    g::<f32>(LOOK_DISTANCE).write(clamped);
    let cam = entity_pos(GPtr::from_addr(THE_CAMERA));
    let p = find_player_coors(out, -1);
    let dy = x(rd::<f32>(cam, 4)).fsub_m(p.wrapping_index(1).read()).to_f32();
    let cam_x = rd::<f32>(cam, 0);
    let mut tmp2 = [0f32; 3];
    let p = find_player_coors(GPtr::from_raw(tmp2.as_mut_ptr()), -1);
    let dx = x(cam_x).fsub_m(p.read());
    g::<f32>(LOOK_DIR_ANGLE).write(dx.fpatan(x(dy)).to_f32());
}

/// The look-around step: both mouse buttons move the distance by `2 * dy` (clamped to 3..15, the upper bound
/// tested before rounding), otherwise the view angle by `dy` (clamped to 0.1..1.5).
fn adjust_look_around(dy: f32) {
    let left = g::<u8>(MOUSE_LEFT).read();
    get_pad(0);
    let both = left != 0 && {
        let right = g::<u8>(MOUSE_RIGHT).read();
        get_pad(0);
        right != 0
    };
    if both {
        let d = x(dy).fdouble().fadd_m(g::<f32>(LOOK_DISTANCE).read());
        g::<f32>(LOOK_DISTANCE).write(d.to_f32());
        if !lt(d, x(k(FIFTEEN))) {
            g::<u32>(LOOK_DISTANCE).write(0x4170_0000);
        } else if le_or_nan(x(g::<f32>(LOOK_DISTANCE).read()), x(k(0x85_8B3C))) {
            g::<u32>(LOOK_DISTANCE).write(0x4040_0000);
        }
    } else {
        let view = x(g::<f32>(LOOK_VIEW_ANGLE).read()).fadd_m(dy).to_f32();
        g::<f32>(LOOK_VIEW_ANGLE).write(view);
        if !le_or_nan(x(k(0x85_8B1C)), x(view)) {
            g::<u32>(LOOK_VIEW_ANGLE).write(0x3DCC_CCCD);
        } else if !ge_or_nan(x(k(0x85_8CE8)), x(view)) {
            g::<u32>(LOOK_VIEW_ANGLE).write(0x3FC0_0000);
        }
    }
}

/// The look-around camera position: the focus plus `(sin a cos v, cos a cos v, sin v) * distance` (each offset an
/// f32); when the line from the focus hits buildings (seeing through see-through and camera-ignoring objects), the
/// hit point moved a quarter of the way back along the unit direction to the focus.
fn look_around_position() -> [f32; 3] {
    let view = x(g::<f32>(LOOK_VIEW_ANGLE).read());
    let dir = x(g::<f32>(LOOK_DIR_ANGLE).read());
    let dist = g::<f32>(LOOK_DISTANCE).read();
    let cos_view = view.fcos();
    let off = [
        dir.fsin().fmul(cos_view).fmul_m(dist).to_f32(),
        dir.fcos().fmul(cos_view).fmul_m(dist).to_f32(),
        view.fsin().fmul_m(dist).to_f32(),
    ];
    let start = focus();
    let mut target = [
        x(off[0]).fadd_m(start[0]).to_f32(),
        x(start[1]).fadd_m(off[1]).to_f32(),
        x(start[2]).fadd_m(off[2]).to_f32(),
    ];
    let mut col_point = [0u32; 0x2C / 4];
    let mut entity = 0u32;
    let p = |v: &[f32; 3]| GPtr::<u8>::from_raw(v.as_ptr().cast_mut().cast::<u8>());
    let hit = process_line_of_sight(
        p(&start),
        p(&target),
        GPtr::from_raw(col_point.as_mut_ptr().cast::<u8>()),
        GPtr::from_raw((&mut entity as *mut u32).cast::<u8>()),
        1, 0, 0, 0, 0, 1, 1, 0,
    );
    if hit != 0 {
        let point = [0, 1, 2].map(|i| f32::from_bits(col_point[i]));
        let mut back = [0f32; 3];
        for i in 0..3 {
            back[i] = x(start[i]).fsub_m(point[i]).to_f32();
        }
        normalise(&mut back);
        let quarter = k(0x85_8C84);
        target = [
            x(back[0]).fmul_m(quarter).fadd_m(point[0]).to_f32(),
            x(back[1]).fmul_m(quarter).fadd_m(point[1]).to_f32(),
            x(x(back[2]).fmul_m(quarter).to_f32()).fadd_m(point[2]).to_f32(),
        ];
    }
    target
}

/// `CReplay::ProcessLookAroundCam()` (0x45D760, cdecl). With `bAllowLookAroundCam`: pad 0's mouse movement times
/// 0.005 (x, y); a movement above 0.01 on either axis (re)starts the look-around for 60 frames, measuring the player's
/// distance and direction to the camera when none was running. While the remote car is used the frames are reset;
/// otherwise each active frame turns the direction by x, adjusts the distance or view angle by y, places the camera
/// ([`look_around_position`]) looking at the focus ([`look_at_rows`]) and updates the RW camera. Always: the scene
/// camera's near clip plane 0.5; while active the player ped and vehicle are made visible.
#[hook(addr = 0x45D760, conv = cdecl, name = "CReplay::ProcessLookAroundCam", expect = "A0 E0 FA 97 00",
       unverified = "frontier2-b32", route = "L2", hazards = "heap-writes")]
pub fn process_look_around_cam() {
    if g::<u8>(ALLOW_LOOK_AROUND).read() != 0 {
        get_pad(0);
        let dx = x(k(MOUSE_X)).fmul_m(k(0x85_8B4C)).to_f32();
        get_pad(0);
        let dy = x(k(MOUSE_Y)).fmul_m(k(0x85_8B4C)).to_f32();
        if moved(dx) || moved(dy) {
            if g::<u32>(FRAMES_ACTIVE).read() == 0 {
                start_look_around();
            }
            g::<u32>(FRAMES_ACTIVE).write(60);
        }
        if g::<u8>(IS_USING_REMOTE_CAR).read() != 0 {
            g::<u32>(FRAMES_ACTIVE).write(0);
        } else if g::<u32>(FRAMES_ACTIVE).read() != 0 {
            let frames = g::<u32>(FRAMES_ACTIVE).read();
            g::<f32>(LOOK_DIR_ANGLE).write(x(g::<f32>(LOOK_DIR_ANGLE).read()).fadd_m(dx).to_f32());
            g::<u32>(FRAMES_ACTIVE).write(frames.wrapping_sub(1));
            adjust_look_around(dy);
            let frame = camera_frame();
            let target = look_around_position();
            let (forward, up, right) = look_at_rows(focus(), target);
            write3(CAM_FORWARD, forward);
            write3(CAM_UP, up);
            write3(CAM_RIGHT, right);
            write3(CAM_POS, target);
            camera_matrix_to_rw(frame.byte_add(0x10));
            refresh_camera_frame(frame);
        }
    }
    rw_camera_set_near_clip_plane(g::<GPtr<u8>>(SCENE_CAMERA).read(), 0.5);
    if g::<u32>(FRAMES_ACTIVE).read() != 0 {
        if !find_player_ped(-1).is_null() {
            let ped = find_player_ped(-1);
            wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) | 0x80);
        }
        if !find_player_vehicle(-1, 0).is_null() {
            let veh = find_player_vehicle(-1, 0);
            wr::<u32>(veh, 0x1C, rd::<u32>(veh, 0x1C) | 0x80);
        }
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_REPLAY_CAMERA,
    HOOK_PLAY_BACK_THIS_FRAME_INTERPOLATION,
    HOOK_PROCESS_LOOK_AROUND_CAM,
];
