//! PORT-VEH-2 (docs/arch/veh-port.md, second pass): `CVehicleRecording` playback, `CAutoPilot`,
//! `CStuckCarCheck` and other small checks. Ported from the 1.0 US bytes; gta-reversed and
//! docs/re/specs/VEH as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::checks::{PLAYBACK_BUFFER, PLAYBACK_GOING_ON, PLAYBACK_PAUSED, VEHICLE_FOR_PLAYBACK};
use crate::common::{g, jne_mask, jp5, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `CVehicleRecording::bUseCarAI[16]`
    pub static USE_CAR_AI @ 0x97D6C0: [u8; 16] = [0; 16];
    /// `CVehicleRecording::bPlaybackLooped[16]`
    pub static PLAYBACK_LOOPED @ 0x97D6E0: [u8; 16] = [0; 16];
    /// `CVehicleRecording::PlaybackSpeed[16]`
    pub static PLAYBACK_SPEED @ 0x97D700: [f32; 16] = [0.0; 16];
    /// `CVehicleRecording::PlaybackRunningTime[16]`
    pub static PLAYBACK_RUNNING_TIME @ 0x97D740: [f32; 16] = [0.0; 16];
    /// `CVehicleRecording::PlaybackBufferSize[16]` (bytes)
    pub static PLAYBACK_BUFFER_SIZE @ 0x97D780: [u32; 16] = [0; 16];
    /// `CVehicleRecording::PlayBackStreamingIndex[16]` (StreamingArray index of each playback)
    pub static PLAYBACK_STREAMING_INDEX @ 0x97D670: [u32; 16] = [0; 16];
    /// `CVehicleRecording::PlaybackIndex[16]` (byte offset of the current frame)
    pub static PLAYBACK_INDEX @ 0x97D7C0: [u32; 16] = [0; 16];
}

orig! {
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    pub fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// CRT `sscanf` with one output (0x8220AD)
    pub fn sscanf(s: GPtr<u8>, fmt: u32, out: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// `CMemoryMgr::Malloc(uint32, uint32)` (0x72F420)
    pub fn memory_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `RwStreamRead(RwStream*, void*, uint32)` (0x7EC9D0)
    pub fn rw_stream_read(st: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    /// `RwStreamClose(RwStream*, void*)` (0x7ECE20)
    pub fn rw_stream_close(st: GPtr<u8>, data: GPtr<u8>) -> i32 = cdecl 0x7ECE20 => unported;
    /// `RwIm3DTransform(RwIm3DVertex*, uint32, RwMatrix*, uint32)` (0x7EF450)
    pub fn rw_im3d_transform(verts: u32, n: i32, m: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(RwPrimitiveType, uint16*, int32)` (0x7EF550)
    pub fn rw_im3d_render_indexed_primitive(ty: u32, idx: u32, n: i32) -> i32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    pub fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
    /// `CVehicleRecording::SmoothRecording(int32)` (0x45A0F0)
    pub fn smooth_recording_o(idx: i32) = cdecl 0x45A0F0 => crate::checks2::smooth_recording;
    /// `CMemoryMgr::Free(void*)` (0x72F430)
    pub fn memory_free(p: GPtr<u8>) = cdecl 0x72F430 => unported;
    /// `CStreaming::RemoveModel(int32)` (0x4089A0)
    pub fn streaming_remove_model(id: i32) = cdecl 0x4089A0 => unported;
    /// `CVehicleRecording::RestoreInfoForMatrix(CMatrix&, CVehicleStateEachFrame*)` (0x459960)
    pub fn restore_info_for_matrix_o(m: GPtr<u8>, f: GPtr<u8>) = cdecl 0x459960 => crate::checks2::restore_info_for_matrix;
    /// `CVehicleRecording::SetRecordingToPointClosestToCoors(int32, CVector)` (0x45A1E0)
    pub fn set_recording_to_point_closest_to_coors_o(i: i32, x: f32, y: f32, z: f32) = cdecl 0x45A1E0 => crate::checks2::set_recording_to_point_closest_to_coors;
    /// `CVector::CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730)
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0)
    pub fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BCF0 => unported;
    /// `CMatrix::Scale(float)` (0x459350)
    pub fn matrix_scale(m: GPtr<u8>, s: u32) = thiscall 0x459350 => unported;
    /// `operator+(const CMatrix&, const CMatrix&)` (0x59BFA0), hidden return pointer first
    pub fn matrix_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59BFA0 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    pub fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CStuckCarCheck::ResetArrayElement(uint16)` (0x463970)
    pub fn stuck_reset_array_element(c: GPtr<u8>, i: u32) = thiscall 0x463970 => unported;
    /// `CStuckCarCheck::AttemptToWarpVehicle(CVehicle*, CVector*, float)` (0x463A60)
    pub fn stuck_attempt_to_warp_vehicle(c: GPtr<u8>, v: GPtr<u8>, pos: GPtr<u8>, angle: f32) -> u8 = thiscall 0x463A60 => unported;
    /// `CEntity::GetBoundCentre(CVector&)` (0x534250)
    pub fn get_bound_centre(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x534250 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40)
    pub fn camera_is_sphere_visible(cam: GPtr<u8>, c: GPtr<u8>, r: u32) -> u8 = thiscall 0x420D40 => unported;
    /// `CPathFind::FindNodeCoorsForScript(CNodeAddress, CNodeAddress, float*, bool*)` (0x450780)
    pub fn find_node_coors_two(pf: GPtr<u8>, out: GPtr<u8>, a: u32, b: u32, angle: GPtr<u8>, found: GPtr<u8>) -> GPtr<u8> = thiscall 0x450780 => unported;
    /// `CPathFind::FindNthNodeClosestToCoors(CVector, uint8, float, bool, bool, int32, bool, bool, CNodeAddress*)` (0x44F8C0)
    pub fn find_nth_node_closest_to_coors(pf: GPtr<u8>, out: GPtr<u8>, x: u32, y: u32, z: u32, ty: u32, dist: u32, a: u32, b: u32, n: u32, c: u32, d: u32, e: u32) -> GPtr<u8> = thiscall 0x44F8C0 => unported;
    /// `CPathFind::FindNodeCoorsForScript(CNodeAddress, bool*)` (0x4505E0)
    pub fn find_node_coors_linked(pf: GPtr<u8>, out: GPtr<u8>, node: u32, found: GPtr<u8>) -> GPtr<u8> = thiscall 0x4505E0 => unported;
    /// `CPathFind::FindNodeOrientationForCarPlacement(CNodeAddress)` (0x450320)
    pub fn find_node_orientation_for_car_placement(pf: GPtr<u8>, node: u32) -> f32 = thiscall 0x450320 => unported;
    /// CRT `printf` (0x821982, the debug message of a big playback step)
    pub fn printf(fmt: u32) -> i32 = cdecl 0x821982 => unported;
    /// `CVehicleRecording::StopPlaybackWithIndex(int32)` (0x459440)
    pub fn stop_playback_with_index_o(i: i32) = cdecl 0x459440 => crate::checks2::stop_playback_with_index;
    /// `CVehicleRecording::StopPlaybackRecordedCar(CVehicle*)` (0x45A280)
    pub fn stop_playback_recorded_car_o(v: GPtr<u8>) = cdecl 0x45A280 => crate::checks2::stop_playback_recorded_car;
    /// `CVehicleRecording::RestoreInfoForCar(CVehicle*, CVehicleStateEachFrame*, bool)` (0x459A30)
    pub fn restore_info_for_car_o(v: GPtr<u8>, f: GPtr<u8>, force: u32) = cdecl 0x459A30 => crate::checks2::restore_info_for_car;
    /// `CVehicleRecording::InterpolateInfoForCar(CVehicle*, CVehicleStateEachFrame*, float)` (0x459B30)
    pub fn interpolate_info_for_car_o(v: GPtr<u8>, f: GPtr<u8>, t: f32) = cdecl 0x459B30 => crate::checks2::interpolate_info_for_car;
    /// `CTrain::FindPositionOnTrackFromCoors()` (0x6F6CC0)
    pub fn train_find_position_on_track_from_coors(t: GPtr<u8>) = thiscall 0x6F6CC0 => unported;
    /// `CPhysical` virtual slot 0x5C (`ProcessControlCollisionCheck(bool)`)
    pub fn process_control_collision_check(v: GPtr<u8>, b: u32) = thiscall vslot 0x5C => unported;
    /// `CPhysical::RemoveAndAdd()` (0x542560)
    pub fn remove_and_add(v: GPtr<u8>) = thiscall 0x542560 => unported;
    /// `CEntity::UpdateRwMatrix()` (0x446F90)
    pub fn update_rw_matrix(v: GPtr<u8>) = thiscall 0x446F90 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    pub fn update_rw_frame(v: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CTrain::MarkSurroundingEntitiesForCollisionWithTrain(CVector, float, CEntity*, bool)` (0x6F6640)
    pub fn mark_surrounding_entities_for_collision_with_train(x: u32, y: u32, z: u32, radius: u32, e: GPtr<u8>, only_trains: u32) = cdecl 0x6F6640 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`.
fn now() -> u32 {
    g::<u32>(0xB7CB84).read()
}

/// `fild dword` of an unsigned value as MSVC converts it: negative as i32 -> `+ 2^32` (0x858C54).
fn fild_u(v: u32) -> X87 {
    let r = xi(v as i32);
    if (v as i32) < 0 {
        r.fadd_m(g::<f32>(0x858C54).read())
    } else {
        r
    }
}

// ---------------------------------------------------------------------------------------------
// CAutoPilot

/// `CAutoPilot::CAutoPilot()` (0x6D5E20): node addresses -1, timers = now, cruise speed 10, ... The
/// two flag bytes keep `+0x4B & 4` and `+0x4C & 0xFC`; `+0x50 = (rand() & 7) + 2`. Returns `this`.
#[hook(addr = 0x6D5E20, conv = thiscall, name = "CAutoPilot::CAutoPilot", expect = "BA FF FF 00 00", this = "CAutoPilot")]
pub fn auto_pilot_ctor(a: GPtr<u8>) -> GPtr<u8> {
    for off in [0, 4, 8, 0x14, 0x16, 0x18] {
        wr::<u16>(a, off, 0xFFFF);
    }
    for k in 0..8 {
        wr::<u16>(a, 0x68 + 4 * k, 0xFFFF);
    }
    wr::<u8>(a, 0x26, 1);
    wr::<u8>(a, 0x25, 1);
    let b4b = rd::<u8>(a, 0x4B) & 0xFC;
    wr::<u8>(a, 0x4B, b4b);
    wr::<u32>(a, 0xC, 0);
    wr::<u32>(a, 0x10, 1000);
    for off in [0x28, 0x27, 0x29, 0x2A, 0x2B] {
        wr::<u8>(a, off, 0);
    }
    wr::<u8>(a, 0x40, 0xA);
    wr::<u32>(a, 0x38, 0x4120_0000);
    wr::<u16>(a, 0x88, 0);
    wr::<u32>(a, 0x8C, 0);
    let t = now();
    wr::<u8>(a, 0x4B, b4b & 7);
    let b4c = rd::<u8>(a, 0x4C) & 0xFC;
    wr::<u32>(a, 0x1C, t);
    wr::<u32>(a, 0x20, t);
    wr::<u8>(a, 0x41, 0);
    wr::<u32>(a, 0x44, 0x3F80_0000);
    wr::<u8>(a, 0x94, 0xFF);
    wr::<u8>(a, 0x35, 0);
    wr::<u8>(a, 0x4D, 0x14);
    wr::<u32>(a, 0x30, 0);
    wr::<u8>(a, 0x34, 0);
    wr::<u8>(a, 0x49, 0);
    wr::<u8>(a, 0x4A, 0);
    wr::<u8>(a, 0x4E, 0xA);
    wr::<u8>(a, 0x4F, 0xA);
    wr::<u8>(a, 0x95, 0);
    wr::<u8>(a, 0x4C, b4c);
    let r = crate::common::rand_at(0x6D5EED);
    wr::<u8>(a, 0x50, ((r as u8) & 7).wrapping_add(2));
    wr::<u32>(a, 0x90, 0);
    wr::<u32>(a, 0x3C, 0);
    a
}

// ---------------------------------------------------------------------------------------------
// CVehicleRecording

/// `CVehicleRecording::SaveOrRetrieveDataForThisFrame()` (0x45A610): nothing in replay playback
/// (0xA43088 == 1). For every playback going on: a missing or scorched (`+0x40` bit 29) vehicle stops
/// it (`StopPlaybackWithIndex`); `bUseCarAI` skips it; unless paused the running time advances by
/// `(now - previous ms) * speed * 0.25` (a step above [0x858B58] also `printf`s 0x859BE8). The frame
/// index moves forward while the next frame's time `<` the running time (the bound is checked before
/// each step, the time read after it, so the frame at the buffer end is read), then back while the
/// current frame's time `>` it and the frame is above the buffer start (gta-reversed bounds it by the
/// old index instead). At the end: looped -> time and index 0, else `StopPlaybackRecordedCar`. Otherwise
/// `RestoreInfoForCar(cur, false)`, `InterpolateInfoForCar(next, (t - cur) / (next - cur))`, trains
/// `FindPositionOnTrackFromCoors`, vtable +0x5C(false), RemoveAndAdd, UpdateRwMatrix, UpdateRwFrame and
/// `MarkSurroundingEntitiesForCollisionWithTrain(pos, 5.0, vehicle, true)`.
#[hook(addr = 0x45A610, conv = cdecl, name = "CVehicleRecording::SaveOrRetrieveDataForThisFrame", expect = "51 80 3D 88 30 A4 00 01")]
pub fn save_or_retrieve_data_for_this_frame() {
    if g::<u8>(0xA43088).read() == 1 {
        return;
    }
    let going = PLAYBACK_GOING_ON.ptr().cast::<u8>();
    let veh = VEHICLE_FOR_PLAYBACK.ptr().cast::<GPtr<u8>>();
    let ai = USE_CAR_AI.ptr().cast::<u8>();
    let paused = PLAYBACK_PAUSED.ptr().cast::<u8>();
    let speed = PLAYBACK_SPEED.ptr().cast::<f32>();
    let rt = PLAYBACK_RUNNING_TIME.ptr().cast::<f32>();
    let buf = PLAYBACK_BUFFER.ptr().cast::<u32>();
    let idx = PLAYBACK_INDEX.ptr().cast::<u32>();
    let size = PLAYBACK_BUFFER_SIZE.ptr().cast::<u32>();
    let looped = PLAYBACK_LOOPED.ptr().cast::<u8>();
    let q = g::<f32>(0x858C84);
    let p = |a: u32| GPtr::<u8>::from_addr(a as usize);
    for i in 0..16i32 {
        if going.wrapping_index(i).read() == 0 {
            continue;
        }
        let v = veh.wrapping_index(i).read();
        if v.is_null() || rd::<u32>(v, 0x40) & 0x2000_0000 != 0 {
            stop_playback_with_index_o(i);
            continue;
        }
        if ai.wrapping_index(i).read() != 0 {
            continue;
        }
        if paused.wrapping_index(i).read() == 0 {
            let dt = now().wrapping_sub(g::<u32>(0xB7CB6C).read());
            let step = fild_u(dt).fmul_m(speed.wrapping_index(i).read()).fmul_m(q.read());
            if !jne_mask(step, x(g::<f32>(0x858B58).read()), 0x41) {
                printf(0x0085_9BE8);
            }
            let dt = now().wrapping_sub(g::<u32>(0xB7CB6C).read());
            let t = fild_u(dt)
                .fmul_m(speed.wrapping_index(i).read())
                .fmul_m(q.read())
                .fadd_m(rt.wrapping_index(i).read());
            rt.wrapping_index(i).write(t.to_f32());
        }
        let base = buf.wrapping_index(i).read();
        let mut cur = base.wrapping_add(idx.wrapping_index(i).read());
        let mut next = cur.wrapping_add(0x20);
        if !jp5(fild_u(rd::<u32>(p(cur), 0x20)), x(rt.wrapping_index(i).read())) {
            let end = size.wrapping_index(i).read().wrapping_add(base);
            loop {
                if next >= end {
                    break;
                }
                idx.wrapping_index(i)
                    .write(idx.wrapping_index(i).read().wrapping_add(0x20));
                next = next.wrapping_add(0x20);
                cur = cur.wrapping_add(0x20);
                if jp5(fild_u(rd::<u32>(p(next), 0)), x(rt.wrapping_index(i).read())) {
                    break;
                }
            }
        }
        if !jne_mask(fild_u(rd::<u32>(p(cur), 0)), x(rt.wrapping_index(i).read()), 0x41) {
            loop {
                if cur <= base {
                    break;
                }
                idx.wrapping_index(i)
                    .write(idx.wrapping_index(i).read().wrapping_sub(0x20));
                cur = cur.wrapping_sub(0x20);
                next = next.wrapping_sub(0x20);
                if jne_mask(fild_u(rd::<u32>(p(cur), 0)), x(rt.wrapping_index(i).read()), 0x41) {
                    break;
                }
            }
        }
        if next >= size.wrapping_index(i).read().wrapping_add(base) {
            if looped.wrapping_index(i).read() != 0 {
                rt.wrapping_index(i).write(0.0);
                idx.wrapping_index(i).write(0);
            } else {
                stop_playback_recorded_car_o(veh.wrapping_index(i).read());
            }
            continue;
        }
        restore_info_for_car_o(veh.wrapping_index(i).read(), p(cur), 0);
        let t0 = rd::<u32>(p(cur), 0);
        let a = fild_u(t0).fsubr_m(rt.wrapping_index(i).read());
        let d = fild_u(rd::<u32>(p(next), 0).wrapping_sub(t0));
        let f = a.fdiv(d).to_f32();
        interpolate_info_for_car_o(veh.wrapping_index(i).read(), p(next), f);
        let v = veh.wrapping_index(i).read();
        if rd::<u32>(v, 0x594) == 6 {
            train_find_position_on_track_from_coors(v);
        }
        process_control_collision_check(veh.wrapping_index(i).read(), 0);
        remove_and_add(veh.wrapping_index(i).read());
        update_rw_matrix(veh.wrapping_index(i).read());
        update_rw_frame(veh.wrapping_index(i).read());
        let v = veh.wrapping_index(i).read();
        let m = rd::<GPtr<u8>>(v, 0x14);
        let pos = if m.is_null() {
            v.byte_add(4)
        } else {
            m.byte_add(0x30)
        };
        mark_surrounding_entities_for_collision_with_train(
            rd::<u32>(pos, 0),
            rd::<u32>(pos, 4),
            rd::<u32>(pos, 8),
            0x40A0_0000,
            v,
            1,
        );
    }
}

/// The unrolled search of the playback functions: the first index whose playback is going on with
/// vehicle `v`, else 16 (the value the originals go on with).
fn find_playback(v: GPtr<u8>) -> i32 {
    let going = PLAYBACK_GOING_ON.ptr().cast::<u8>();
    let veh = VEHICLE_FOR_PLAYBACK.ptr().cast::<GPtr<u8>>();
    for i in 0..16 {
        if going.wrapping_index(i).read() != 0 && veh.wrapping_index(i).read() == v {
            return i;
        }
    }
    16
}

/// The streaming release shared by StopPlaybackWithIndex and SkipToEndAndStopPlaybackRecordedCar.
fn release_streamed_recording(i: i32) {
    let si = PLAYBACK_STREAMING_INDEX
        .ptr()
        .cast::<u32>()
        .wrapping_index(i)
        .read();
    let e = crate::checks::STREAMING_ARRAY
        .ptr()
        .cast::<u8>()
        .byte_add((si << 4) as i32 as isize);
    let rc = rd::<u8>(e, 0xC).wrapping_sub(1);
    wr::<u8>(e, 0xC, rc);
    if rc == 0 {
        memory_free(rd::<GPtr<u8>>(e, 4));
        wr::<u32>(e, 4, 0);
        streaming_remove_model(si.wrapping_add(0x649B) as i32);
    }
}

/// `CVehicleRecording::StopPlaybackWithIndex(int32)` (0x459440): the vehicle (if any) gets `+0x424 =
/// 0xFF` and loses `+0x40` bit 2; vehicle, buffer, size and the going-on flag are cleared; the
/// streamed file's reference count drops and at 0 its data is freed (`CMemoryMgr::Free`) and the
/// model `0x649B + index` removed.
#[hook(addr = 0x459440, conv = cdecl, name = "CVehicleRecording::StopPlaybackWithIndex", expect = "8B 44 24 04 8B 0C 85 40 D8 97 00")]
pub fn stop_playback_with_index(i: i32) {
    let veh = VEHICLE_FOR_PLAYBACK.ptr().cast::<GPtr<u8>>().wrapping_index(i);
    let v = veh.read();
    if !v.is_null() {
        wr::<u8>(v, 0x424, 0xFF);
        let v = veh.read();
        wr::<u32>(v, 0x40, rd::<u32>(v, 0x40) & 0xFFFF_FFFB);
    }
    veh.write(GPtr::NULL);
    PLAYBACK_BUFFER
        .ptr()
        .cast::<GPtr<u8>>()
        .wrapping_index(i)
        .write(GPtr::NULL);
    PLAYBACK_BUFFER_SIZE
        .ptr()
        .cast::<u32>()
        .wrapping_index(i)
        .write(0);
    PLAYBACK_GOING_ON.ptr().cast::<u8>().wrapping_index(i).write(0);
    release_streamed_recording(i);
}

/// `CVehicleRecording::StopPlaybackRecordedCar(CVehicle*)` (0x45A280): StopPlaybackWithIndex of the
/// vehicle's playback (tail jump), nothing when it has none.
#[hook(addr = 0x45A280, conv = cdecl, name = "CVehicleRecording::StopPlaybackRecordedCar", expect = "8B 4C 24 04 33 C0")]
pub fn stop_playback_recorded_car(v: GPtr<u8>) {
    let i = find_playback(v);
    if i < 16 {
        stop_playback_with_index_o(i);
    }
}

/// `CVehicleRecording::SetPlaybackSpeed(CVehicle*, float)` (0x459660).
#[hook(addr = 0x459660, conv = cdecl, name = "CVehicleRecording::SetPlaybackSpeed", expect = "8B 4C 24 04 33 C0")]
pub fn set_playback_speed(v: GPtr<u8>, speed: f32) {
    let i = find_playback(v);
    if i < 16 {
        PLAYBACK_SPEED
            .ptr()
            .cast::<f32>()
            .wrapping_index(i)
            .write(x(speed).to_f32());
    }
}

/// `CVehicleRecording::IsPlaybackGoingOnForCar(CVehicle*)` (0x4594C0, body in .HOODLUM 0x1561F20).
#[hook(addr = 0x4594C0, conv = cdecl, name = "CVehicleRecording::IsPlaybackGoingOnForCar", expect = "E9 5B 8A 10 01")]
pub fn is_playback_going_on_for_car(v: GPtr<u8>) -> u8 {
    u8::from(find_playback(v) < 16)
}

/// `CVehicleRecording::PausePlaybackRecordedCar(CVehicle*)` (0x459740, body 0x1565B90): `bPlaybackPaused
/// [i] = 1`; without a playback index 16 is written, i.e. `bPlaybackLooped[0]` (kept).
#[hook(addr = 0x459740, conv = cdecl, name = "CVehicleRecording::PausePlaybackRecordedCar", expect = "E9 4B C4 10 01")]
pub fn pause_playback_recorded_car(v: GPtr<u8>) {
    let i = find_playback(v);
    PLAYBACK_PAUSED.ptr().cast::<u8>().wrapping_index(i).write(1);
}

/// `CVehicleRecording::UnpausePlaybackRecordedCar(CVehicle*)` (0x459850, body 0x156BA60): as Pause with 0
/// (index 16 clears `bPlaybackLooped[0]`, kept).
#[hook(addr = 0x459850, conv = cdecl, name = "CVehicleRecording::UnpausePlaybackRecordedCar", expect = "E9 0B 22 11 01")]
pub fn unpause_playback_recorded_car(v: GPtr<u8>) {
    let i = find_playback(v);
    PLAYBACK_PAUSED.ptr().cast::<u8>().wrapping_index(i).write(0);
}

/// `CVehicleRecording::RestoreInfoForMatrix(CMatrix&, CVehicleStateEachFrame*)` (0x459960): right and
/// forward from the six i8 at frame `+0xA..+0xF` times [0x859BCC], up = `CrossProduct(right, forward)`,
/// position = frame `+0x14..+0x1C`.
#[hook(addr = 0x459960, conv = cdecl, name = "CVehicleRecording::RestoreInfoForMatrix", expect = "83 EC 18 56 8B 74 24 20")]
pub fn restore_info_for_matrix(m: GPtr<u8>, f: GPtr<u8>) {
    let k = g::<f32>(0x859BCC).read();
    for (j, off) in [0, 4, 8, 0x10, 0x14, 0x18].into_iter().enumerate() {
        let b = rd::<i8>(f, 0xA + j as isize);
        wr::<f32>(m, off, xi(i32::from(b)).fmul_m(k).to_f32());
    }
    let mut o = [0u32; 3];
    let r = cross_product(GPtr::from_raw(&raw mut o).cast(), m, m.byte_add(0x10));
    for j in 0..3 {
        wr::<u32>(m, 0x20 + 4 * j, rd::<u32>(r, 4 * j));
    }
    for j in 0..3 {
        wr::<u32>(m, 0x30 + 4 * j, rd::<u32>(f, 0x14 + 4 * j));
    }
}

/// `CVehicleRecording::RestoreInfoForCar(CVehicle*, CVehicleStateEachFrame*, bool force)` (0x459A30):
/// matrix from the frame; move speed = the i16 at `+4/+6/+8` times [0x858EAC], turn speed 0; steer
/// `+0x494` = i8 `+0x10` * [0x858C28], gas `+0x49C` / brake `+0x4A0` = i8 `+0x11` / `+0x12` *
/// [0x858C58]; horn bit `+0x428 & 0x20` from `+0x13`. `force` zeroes gas, brake, move speed and the
/// horn bit. Type 9 clears bits 3/4 of `+0x614`.
#[hook(addr = 0x459A30, conv = cdecl, name = "CVehicleRecording::RestoreInfoForCar", expect = "56 8B 74 24 08")]
pub fn restore_info_for_car(v: GPtr<u8>, f: GPtr<u8>, force: u32) {
    restore_info_for_matrix_o(rd::<GPtr<u8>>(v, 0x14), f);
    let k = g::<f32>(0x858EAC).read();
    let sp = |o: isize| xi(i32::from(rd::<i16>(f, o))).fmul_m(k).to_f32();
    wr::<f32>(v, 0x44, sp(4));
    wr::<f32>(v, 0x48, sp(6));
    let z = sp(8);
    wr::<u32>(v, 0x50, 0);
    wr::<u32>(v, 0x54, 0);
    wr::<u32>(v, 0x58, 0);
    wr::<f32>(v, 0x4C, z);
    let b = |o: isize, a: usize| xi(i32::from(rd::<i8>(f, o))).fmul_m(g::<f32>(a).read()).to_f32();
    wr::<f32>(v, 0x494, b(0x10, 0x858C28));
    wr::<f32>(v, 0x49C, b(0x11, 0x858C58));
    wr::<f32>(v, 0x4A0, b(0x12, 0x858C58));
    let h = rd::<u8>(v, 0x428);
    wr::<u8>(
        v,
        0x428,
        if rd::<u8>(f, 0x13) != 0 {
            h | 0x20
        } else {
            h & 0xDF
        },
    );
    if force as u8 != 0 {
        for off in [0x49C, 0x4A0, 0x44, 0x48, 0x4C] {
            wr::<u32>(v, off, 0);
        }
        wr::<u8>(v, 0x428, rd::<u8>(v, 0x428) & 0xDF);
    }
    if rd::<u32>(v, 0x590) == 9 {
        wr::<u8>(v, 0x614, rd::<u8>(v, 0x614) & 0xE7);
    }
}

/// `CVehicleRecording::InterpolateInfoForCar(CVehicle*, CVehicleStateEachFrame*, float t)` (0x459B30):
/// `matrix = Scale(frame matrix, t) + Scale(copy of the vehicle matrix, 1 - t)` (positions scaled
/// separately, `1 - t` stored as f32), assigned through a temporary (`operator+`, `operator=`); move
/// speed = `frame * [0x858EAC] * t + (1 - t) * old`.
#[hook(addr = 0x459B30, conv = cdecl, name = "CVehicleRecording::InterpolateInfoForCar", expect = "6A FF 68 DE 79 83 00")]
pub fn interpolate_info_for_car(v: GPtr<u8>, f: GPtr<u8>, t: f32) {
    let mut m1 = [0u32; 0x12];
    let mut m2 = [0u32; 0x12];
    let mut m3 = [0u32; 0x12];
    let p1 = GPtr::from_raw(&raw mut m1).cast::<u8>();
    let p2 = GPtr::from_raw(&raw mut m2).cast::<u8>();
    let p3 = GPtr::from_raw(&raw mut m3).cast::<u8>();
    matrix_copy_ctor(p2, rd::<GPtr<u8>>(v, 0x14));
    restore_info_for_matrix_o(p1, f);
    matrix_scale(p1, t.to_bits());
    for off in [0x30, 0x34, 0x38] {
        wr::<f32>(p1, off, x(rd::<f32>(p1, off)).fmul_m(t).to_f32());
    }
    let u = x(g::<f32>(0x858624).read()).fsub_m(t).to_f32();
    matrix_scale(p2, u.to_bits());
    for off in [0x30, 0x34, 0x38] {
        wr::<f32>(p2, off, x(u).fmul_m(rd::<f32>(p2, off)).to_f32());
    }
    let r = matrix_add(p3, p1, p2);
    matrix_assign(rd::<GPtr<u8>>(v, 0x14), r);
    matrix_dtor(p3);
    let k = g::<f32>(0x858EAC).read();
    for (fo, vo) in [(4, 0x44), (6, 0x48), (8, 0x4C)] {
        let a = xi(i32::from(rd::<i16>(f, fo))).fmul_m(k).fmul_m(t);
        let b = x(u).fmul_m(rd::<f32>(v, vo));
        wr::<f32>(v, vo, a.fadd(b).to_f32());
    }
    matrix_dtor(p2);
    matrix_dtor(p1);
}

/// `CVehicleRecording::ChangeCarPlaybackToUseAI(CVehicle*)` (0x45A360): `bUseCarAI[i] = 1` (index 16
/// when the car has no playback, i.e. `bPlaybackPaused[0]`; kept); autopilot mission 0x22; the
/// recording index set to the frame closest to the car (`SetRecordingToPointClosestToCoors`); `+0x40`
/// bit 2 cleared; vtable +0x5C(false); status 3 (`+0x36` high bits 0x18).
#[hook(addr = 0x45A360, conv = cdecl, name = "CVehicleRecording::ChangeCarPlaybackToUseAI", expect = "56 8B 74 24 08")]
pub fn change_car_playback_to_use_ai(v: GPtr<u8>) {
    let i = find_playback(v);
    USE_CAR_AI.ptr().cast::<u8>().wrapping_index(i).write(1);
    wr::<u8>(v, 0x3BA, 0x22);
    let m = rd::<GPtr<u8>>(v, 0x14);
    let pos = if m.is_null() {
        v.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    set_recording_to_point_closest_to_coors_o(i, rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8));
    wr::<u32>(v, 0x40, rd::<u32>(v, 0x40) & 0xFFFF_FFFB);
    process_control_collision_check(v, 0);
    wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x18);
}

/// `CVehicleRecording::SkipToEndAndStopPlaybackRecordedCar(CVehicle*)` (0x45A4A0): with a playback: bit
/// 2 of `+0x40` cleared, `RestoreInfoForCar(last frame, false)`, vtable +0x5C(false), the playback
/// slots cleared, `+0x424 = 0xFF` on the argument, the streamed file released (as StopPlaybackWithIndex).
#[hook(addr = 0x45A4A0, conv = cdecl, name = "CVehicleRecording::SkipToEndAndStopPlaybackRecordedCar", expect = "53 56 57 8B 7C 24 10")]
pub fn skip_to_end_and_stop_playback_recorded_car(v: GPtr<u8>) {
    let i = find_playback(v);
    if i >= 16 {
        return;
    }
    let veh = VEHICLE_FOR_PLAYBACK.ptr().cast::<GPtr<u8>>().wrapping_index(i);
    let pv = veh.read();
    wr::<u32>(pv, 0x40, rd::<u32>(pv, 0x40) & 0xFFFF_FFFB);
    let size = PLAYBACK_BUFFER_SIZE.ptr().cast::<u32>().wrapping_index(i).read();
    let buf = PLAYBACK_BUFFER.ptr().cast::<u32>().wrapping_index(i).read();
    let last = GPtr::<u8>::from_addr(size.wrapping_add(buf).wrapping_sub(0x20) as usize);
    restore_info_for_car_o(veh.read(), last, 0);
    process_control_collision_check(veh.read(), 0);
    veh.write(GPtr::NULL);
    PLAYBACK_BUFFER
        .ptr()
        .cast::<GPtr<u8>>()
        .wrapping_index(i)
        .write(GPtr::NULL);
    PLAYBACK_BUFFER_SIZE
        .ptr()
        .cast::<u32>()
        .wrapping_index(i)
        .write(0);
    PLAYBACK_GOING_ON.ptr().cast::<u8>().wrapping_index(i).write(0);
    wr::<u8>(v, 0x424, 0xFF);
    release_streamed_recording(i);
}

/// The recording file's `StreamingArray` entry (`CPath`, 16 bytes).
fn path_entry(k: i32) -> GPtr<u8> {
    crate::checks::STREAMING_ARRAY
        .ptr()
        .cast::<u8>()
        .byte_add(16 * k as isize)
}

/// `CVehicleRecording::RequestRecordingFile(int32 file)` (0x45A020, body 0x156D110): `RequestModel(0x649B
/// + index, 0xC)` of the file's StreamingArray index (0 when it is not registered).
#[hook(addr = 0x45A020, conv = cdecl, name = "CVehicleRecording::RequestRecordingFile", expect = "E9 EB 30 11 01")]
pub fn request_recording_file(file: i32) {
    let n = crate::checks::NUM_PLAYBACK_FILES.get();
    let mut i = 0;
    while i < n {
        if rd::<i32>(path_entry(i), 0) == file {
            break;
        }
        i += 1;
    }
    if i >= n {
        i = 0;
    }
    streaming_request_model(i + 0x649B, 0xC);
}

/// Frees the data of every StreamingArray entry with data that `keep` does not keep, then
/// `RemoveModel(0x649B + k)` (the data pointer is cleared between the two calls).
fn free_recordings(keep: impl Fn(GPtr<u8>) -> bool) {
    for k in 0..475 {
        let e = path_entry(k);
        let d = rd::<GPtr<u8>>(e, 4);
        if d.is_null() || keep(e) {
            continue;
        }
        memory_free(d);
        wr::<u32>(e, 4, 0);
        streaming_remove_model(k + 0x649B);
    }
}

/// `CVehicleRecording::ShutDown()` (0x459400, body 0x156E770): every loaded recording freed and its
/// streaming model removed.
#[hook(addr = 0x459400, conv = cdecl, name = "CVehicleRecording::ShutDown", expect = "E9 6B 53 11 01")]
pub fn recording_shut_down() {
    free_recordings(|_| false);
}

/// `CVehicleRecording::RemoveAllRecordingsThatArentUsed()` (0x45A160, body 0x15688F0): as ShutDown for
/// the entries whose reference count (`+0xC`) is 0.
#[hook(addr = 0x45A160, conv = cdecl, name = "CVehicleRecording::RemoveAllRecordingsThatArentUsed", expect = "E9 8B E7 10 01")]
pub fn remove_all_recordings_that_arent_used() {
    free_recordings(|e| rd::<u8>(e, 0xC) != 0);
}

/// `CVehicleRecording::RemoveRecordingFile(int32 file)` (0x45A0A0, body 0x15640F0): as
/// RemoveAllRecordingsThatArentUsed for the entries of that file number.
#[hook(addr = 0x45A0A0, conv = cdecl, name = "CVehicleRecording::RemoveRecordingFile", expect = "E9 4B A0 10 01")]
pub fn remove_recording_file(file: i32) {
    free_recordings(|e| rd::<i32>(e, 0) != file || rd::<u8>(e, 0xC) != 0);
}

/// `CVehicleRecording::RegisterRecordingFile(const char* name)` (0x459F80, body 0x156F110): the file
/// number from `sscanf(name, [0x859BDC], &n)`, else `sscanf(name, [0x859BD0], &n)` (default 850), stored
/// in a new StreamingArray entry (data cleared); returns the entry's index.
#[hook(addr = 0x459F80, conv = cdecl, name = "CVehicleRecording::RegisterRecordingFile", expect = "E9 8B 51 11 01")]
pub fn register_recording_file(name: GPtr<u8>) -> i32 {
    let mut num = 0x352i32;
    let np = GPtr::from_raw(&raw mut num).cast::<u8>();
    if sscanf(name, 0x0085_9BDC, np) == 0 {
        sscanf(name, 0x0085_9BD0, np);
    }
    let n = crate::checks::NUM_PLAYBACK_FILES.get();
    let e = path_entry(n);
    wr::<i32>(e, 0, num);
    wr::<u32>(e, 4, 0);
    crate::checks::NUM_PLAYBACK_FILES.set(n.wrapping_add(1));
    n
}

/// `CVehicleRecording::SmoothRecording(int32 index)` (0x45A0F0): every inner frame's time (u32 at
/// `+0`) becomes `_ftol((prev + next) * [0x858B8C])`, the sum wrapping and converted as unsigned; the
/// size is re-read each step.
#[hook(addr = 0x45A0F0, conv = cdecl, name = "CVehicleRecording::SmoothRecording", expect = "57 8B 7C 24 08")]
pub fn smooth_recording(idx: i32) {
    let e = path_entry(idx);
    if rd::<u32>(e, 8).wrapping_sub(0x20) <= 0x20 {
        return;
    }
    let data = rd::<u32>(e, 4);
    let mut m = 0x20u32;
    loop {
        let f = GPtr::<u8>::from_addr(data.wrapping_add(m) as usize);
        let s = rd::<u32>(f, -0x20).wrapping_add(rd::<u32>(f, 0x20));
        let t = crate::common::ftol32(fild_u(s).fmul_m(g::<f32>(0x858B8C).read()));
        wr::<i32>(f, 0, t);
        m = m.wrapping_add(0x20);
        if m >= rd::<u32>(e, 8).wrapping_sub(0x20) {
            break;
        }
    }
}

/// `CVehicleRecording::Load(RwStream*, int32 index, int32 size)` (0x45A8F0, body 0x156F510): the data
/// is `CMemoryMgr::Malloc(size)`, filled by `RwStreamRead(stream, data, 9999999)` (the read size is the
/// recording size), the stream closed; the size is cut at the first frame after the first whose time
/// is 0; `SmoothRecording`.
#[hook(addr = 0x45A8F0, conv = cdecl, name = "CVehicleRecording::Load", expect = "E9 1B 4C 11 01")]
pub fn recording_load(stream: GPtr<u8>, idx: i32, size: i32) {
    let data = memory_malloc(size as u32, 0);
    let e = path_entry(idx);
    wr::<GPtr<u8>>(e, 4, data);
    let n = rw_stream_read(stream, data, 0x0098_967F);
    wr::<u32>(e, 8, n);
    rw_stream_close(stream, GPtr::NULL);
    let mut off = 0i32;
    if rd::<i32>(e, 8) > 0 {
        loop {
            let d = rd::<GPtr<u8>>(e, 4);
            if rd::<u32>(d, ((off as u32 >> 5) << 5) as isize) == 0 && off != 0 {
                wr::<i32>(e, 8, off);
            }
            off = off.wrapping_add(0x20);
            if off >= rd::<i32>(e, 8) {
                break;
            }
        }
    }
    smooth_recording_o(idx);
}

/// `CVehicleRecording::SetRecordingToPointClosestToCoors(int32 index, CVector pos)` (0x45A1E0): the
/// playback index of the frame whose position is closest (`sqrt((dz² + dy²) + dx²)`, dx stored as f32,
/// minimum stored as f32, strictly below 999999.9).
#[hook(addr = 0x45A1E0, conv = cdecl, name = "CVehicleRecording::SetRecordingToPointClosestToCoors", expect = "83 EC 1C 56 57")]
pub fn set_recording_to_point_closest_to_coors(idx: i32, px: f32, py: f32, pz: f32) {
    let size = PLAYBACK_BUFFER_SIZE
        .ptr()
        .cast::<i32>()
        .wrapping_index(idx)
        .read();
    let buf = PLAYBACK_BUFFER
        .ptr()
        .cast::<GPtr<u8>>()
        .wrapping_index(idx)
        .read();
    let mut best = f32::from_bits(0x4974_23FE);
    let mut off = 0i32;
    while off < size {
        let f = buf.byte_add(off as isize);
        let dx = x(px).fsub_m(rd::<f32>(f, 0x14)).to_f32();
        let dy = x(py).fsub_m(rd::<f32>(f, 0x18));
        let dz = x(pz).fsub_m(rd::<f32>(f, 0x1C));
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(x(dx).fmul_m(dx)).fsqrt();
        if !jp5(d, x(best)) {
            best = d.to_f32();
            PLAYBACK_INDEX.ptr().cast::<i32>().wrapping_index(idx).write(off);
        }
        off += 0x20;
    }
}

/// `CVehicleRecording::SkipForwardInRecording(CVehicle*, float distance)` (0x459D10): while the
/// remaining distance is `> 0` (or NaN) the index steps forward (bounded by size - 0x20) and the 2D
/// frame-to-frame distance (`sqrt(dx² + dy²)`) is subtracted; when it is not, the index steps back
/// (above frame 1) and the distance is added while it stays negative. The running time becomes the
/// reached frame's time (unsigned); with `bUseCarAI` the car is restored to that frame and vtable
/// +0x5C(false). A car without playback works on index 16 (kept).
#[hook(addr = 0x459D10, conv = cdecl, name = "CVehicleRecording::SkipForwardInRecording", expect = "51 8B 44 24 08")]
pub fn skip_forward_in_recording(v: GPtr<u8>, distance: f32) {
    let i = find_playback(v);
    let pidx = PLAYBACK_INDEX.ptr().cast::<u32>().wrapping_index(i);
    let buf = PLAYBACK_BUFFER.ptr().cast::<u32>().wrapping_index(i).read();
    let at = |o: u32| GPtr::<u8>::from_addr(buf.wrapping_add(o) as usize);
    let mut cur = at(pidx.read());
    let zero = x(g::<f32>(0x858B50).read());
    let mut d = x(distance);
    loop {
        let off = pidx.read();
        if !jne_mask(d, zero, 0x41) {
            let size = PLAYBACK_BUFFER_SIZE.ptr().cast::<u32>().wrapping_index(i).read();
            if off >= size.wrapping_sub(0x20) {
                break;
            }
            let n = off.wrapping_add(0x20);
            pidx.write(n);
            let dy = x(rd::<f32>(at(off), 0x18)).fsub_m(rd::<f32>(at(n), 0x18));
            cur = at(n);
            let dx = x(rd::<f32>(at(off), 0x14)).fsub_m(rd::<f32>(cur, 0x14));
            let s = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
            d = d.fsub(s);
            if !d.fcom(zero).test_parity(0x41) {
                break;
            }
        } else {
            if off <= 0x20 {
                break;
            }
            let n = off.wrapping_sub(0x20);
            pidx.write(n);
            let dy = x(rd::<f32>(at(off), 0x18)).fsub_m(rd::<f32>(at(n), 0x18));
            cur = at(n);
            let dx = x(rd::<f32>(at(off), 0x14)).fsub_m(rd::<f32>(cur, 0x14));
            let s = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
            d = d.fadd(s);
            if !d.fcom(zero).test_any(1) {
                break;
            }
        }
    }
    let t = fild_u(rd::<u32>(cur, 0)).to_f32();
    PLAYBACK_RUNNING_TIME
        .ptr()
        .cast::<f32>()
        .wrapping_index(i)
        .write(t);
    if USE_CAR_AI.ptr().cast::<u8>().wrapping_index(i).read() != 0 {
        let veh = VEHICLE_FOR_PLAYBACK.ptr().cast::<GPtr<u8>>().wrapping_index(i);
        restore_info_for_car_o(veh.read(), at(pidx.read()), 0);
        process_control_collision_check(veh.read(), 0);
    }
}

/// `CVehicleRecording::RenderLineSegment(int32& numVertices)` (0x459F00): with more than one vertex,
/// the line-list indices `(k, k + 1)` at 0xC4B958, `RwIm3DTransform(0xC4D958, n, 0, 0)` and, when it
/// succeeds, `RwIm3DRenderIndexedPrimitive(line list, indices, 2n - 2)` and `RwIm3DEnd`; the count is
/// reset to 0.
#[hook(addr = 0x459F00, conv = cdecl, name = "CVehicleRecording::RenderLineSegment", expect = "56 8B 74 24 08")]
pub fn render_line_segment(n: GPtr<i32>) {
    if n.read() > 1 {
        let mut k = 0i32;
        if n.read() - 1 > 0 {
            loop {
                g::<u16>(0xC4B958 + 4 * k as usize).write(k as u16);
                g::<u16>(0xC4B95A + 4 * k as usize).write((k + 1) as u16);
                k += 1;
                if k >= n.read() - 1 {
                    break;
                }
            }
        }
        if !rw_im3d_transform(0xC4D958, n.read(), 0, 0).is_null() {
            rw_im3d_render_indexed_primitive(1, 0xC4B958, n.read().wrapping_mul(2).wrapping_sub(2));
            rw_im3d_end();
        }
    }
    n.write(0);
}

// ---------------------------------------------------------------------------------------------
// CStuckCarCheck

/// `ThePaths`.
const THE_PATHS: usize = 0x96F050;

/// `CStuckCarCheck::Process()` (0x465680): for the 16 entries (0x24 bytes: handle, last position, last
/// time, radius, interval, stuck byte, warp / stuck / upside-down / in-water flags, i8 node count):
/// an invalid handle or a freed slot resets the entry (`ResetArrayElement`); a vehicle without driver
/// just refreshes position and time. Otherwise, when `now > lastTime + interval` (unsigned), stuck =
/// moved distance (`sqrt((dz² + dy²) + dx²)`) `<` radius, position and time refreshed. With the warp
/// flag, a vehicle that is (stuck and flagged) or (upside down and flagged) or (in water, `+0x40 &
/// 0x100`, and flagged), and whose bounding sphere the camera cannot see, is warped: node count `< 0`
/// -> to the car's own two path nodes (`FindNodeCoorsForScript(+0x390, +0x394)`); else the n-th
/// closest car nodes (n from 0 below the count) are tried until `AttemptToWarpVehicle` succeeds.
#[hook(addr = 0x465680, conv = thiscall, name = "CStuckCarCheck::Process", expect = "83 EC 4C 53 55", this = "CStuckCarCheck")]
pub fn stuck_car_check_process(c: GPtr<u8>) {
    let now = now();
    let paths = GPtr::<u8>::from_addr(THE_PATHS);
    for i in 0..16u32 {
        let e = c.byte_add(0x24 * i as isize);
        let h = rd::<i32>(e, 0);
        if h < 0 {
            continue;
        }
        let pool = g::<GPtr<u8>>(0xB74494).read();
        let idx = h >> 8;
        if rd::<u8>(rd::<GPtr<u8>>(pool, 4), idx as isize) != h as u8 {
            stuck_reset_array_element(c, i);
            continue;
        }
        let v = rd::<GPtr<u8>>(pool, 0).byte_add(idx as isize * 0xA18);
        if v.is_null() {
            stuck_reset_array_element(c, i);
            continue;
        }
        let m = rd::<GPtr<u8>>(v, 0x14);
        let pos = if m.is_null() {
            v.byte_add(4)
        } else {
            m.byte_add(0x30)
        };
        if rd::<u32>(v, 0x460) == 0 {
            for k in 0..3 {
                wr::<u32>(e, 4 + 4 * k, rd::<u32>(pos, 4 * k));
            }
            wr::<u32>(e, 0x10, now);
            continue;
        }
        if now > rd::<u32>(e, 0x18).wrapping_add(rd::<u32>(e, 0x10)) {
            let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(e, 4));
            let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(e, 8));
            let dz = x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(e, 0xC));
            let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
            wr::<u8>(e, 0x1C, u8::from(!jp5(d, x(rd::<f32>(e, 0x14)))));
            for k in 0..3 {
                wr::<u32>(e, 4 + 4 * k, rd::<u32>(pos, 4 * k));
            }
            wr::<u32>(e, 0x10, now);
        }
        if rd::<u8>(e, 0x1D) == 0 {
            continue;
        }
        let mut warp = rd::<u8>(e, 0x1E) != 0 && rd::<u8>(e, 0x1C) != 0;
        if rd::<u8>(e, 0x1F) != 0 && crate::checks::is_car_upside_down_call(GPtr::from_addr(0xA4892C), v) != 0
        {
            warp = true;
        }
        if !(rd::<u8>(e, 0x20) != 0 && rd::<u32>(v, 0x40) & 0x100 != 0) && !warp {
            continue;
        }
        let mi = crate::common::model_info(i32::from(rd::<i16>(v, 0x22)));
        let radius = rd::<u32>(rd::<GPtr<u8>>(mi, 0x14), 0x24);
        let mut centre = [0u32; 3];
        let cp = get_bound_centre(v, GPtr::from_raw(&raw mut centre).cast());
        if camera_is_sphere_visible(GPtr::from_addr(0xB6F028), cp, radius) != 0 {
            continue;
        }
        let nodes = rd::<i8>(e, 0x21);
        let mut out = [0u32; 3];
        let op = GPtr::from_raw(&raw mut out).cast::<u8>();
        if nodes < 0 {
            let mut angle = 0f32;
            let mut found = 0u8;
            let r = find_node_coors_two(
                paths,
                op,
                rd::<u32>(v, 0x390),
                rd::<u32>(v, 0x394),
                GPtr::from_raw(&raw mut angle).cast(),
                GPtr::from_raw(&raw mut found).cast(),
            );
            let mut wp = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
            if found != 0 {
                stuck_attempt_to_warp_vehicle(c, v, GPtr::from_raw(&raw mut wp).cast(), angle);
            }
            continue;
        }
        let mut n: u8 = 0;
        let mut done = false;
        while i32::from(n) < i32::from(rd::<i8>(e, 0x21)) {
            if done {
                break;
            }
            let m = rd::<GPtr<u8>>(v, 0x14);
            let p = if m.is_null() {
                v.byte_add(4)
            } else {
                m.byte_add(0x30)
            };
            let mut na = 0u32;
            let r = find_nth_node_closest_to_coors(
                paths,
                GPtr::from_raw(&raw mut na).cast(),
                rd::<u32>(p, 0),
                rd::<u32>(p, 4),
                rd::<u32>(p, 8),
                0,
                0x4974_23FE,
                0,
                1,
                u32::from(n),
                0,
                0,
                0,
            );
            let node = rd::<u32>(r, 0);
            let mut found = 0u8;
            let r = find_node_coors_linked(paths, op, node, GPtr::from_raw(&raw mut found).cast());
            let mut wp = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
            if found != 0 {
                let angle = find_node_orientation_for_car_placement(paths, node);
                if stuck_attempt_to_warp_vehicle(c, v, GPtr::from_raw(&raw mut wp).cast(), angle) != 0 {
                    done = true;
                    continue;
                }
            }
            n = n.wrapping_add(1);
        }
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_REQUEST_RECORDING_FILE,
    HOOK_RECORDING_SHUT_DOWN,
    HOOK_REMOVE_ALL_RECORDINGS_THAT_ARENT_USED,
    HOOK_REMOVE_RECORDING_FILE,
    HOOK_REGISTER_RECORDING_FILE,
    HOOK_SMOOTH_RECORDING,
    HOOK_RECORDING_LOAD,
    HOOK_SET_RECORDING_TO_POINT_CLOSEST_TO_COORS,
    HOOK_SKIP_FORWARD_IN_RECORDING,
    HOOK_RENDER_LINE_SEGMENT,
    HOOK_STOP_PLAYBACK_WITH_INDEX,
    HOOK_STOP_PLAYBACK_RECORDED_CAR,
    HOOK_SET_PLAYBACK_SPEED,
    HOOK_IS_PLAYBACK_GOING_ON_FOR_CAR,
    HOOK_PAUSE_PLAYBACK_RECORDED_CAR,
    HOOK_UNPAUSE_PLAYBACK_RECORDED_CAR,
    HOOK_RESTORE_INFO_FOR_MATRIX,
    HOOK_RESTORE_INFO_FOR_CAR,
    HOOK_INTERPOLATE_INFO_FOR_CAR,
    HOOK_CHANGE_CAR_PLAYBACK_TO_USE_AI,
    HOOK_SKIP_TO_END_AND_STOP_PLAYBACK_RECORDED_CAR,
    HOOK_STUCK_CAR_CHECK_PROCESS,
    HOOK_AUTO_PILOT_CTOR,
    HOOK_SAVE_OR_RETRIEVE_DATA_FOR_THIS_FRAME,
];
