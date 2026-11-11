//! `CAESound` / `CAESoundManager` (PORT-AUDIO): sound objects and the sound manager (`AESoundManager` 0xB62CB0).
//! Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids.
//!
//! `CAESound` (0x74) fields used here: +0 bank slot (u16), +2 sound id (u16), +4 base audio entity,
//! +8 physical entity (registered reference), +0xC event (i32), +0x1C volume, +0x20 volume variance,
//! +0x56 flags (u16), +0x58 is-used (u16), +0x5A, +0x5E is-playing (u16), +0x64 final volume,
//! +0x68 stop request (u16), +0x6C headroom. `CAESoundManager`: +4 `CAESound[300]`, +0x8CB5 manual pause.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{AE_AUDIO_HARDWARE, g, rd, wr, x};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

/// `sizeof(CAESound)`
const SOUND_SIZE: isize = 0x74;
/// `CAESoundManager::m_aSounds` length
const NUM_SOUNDS: isize = 300;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity** ref)` (0x571A00)
    pub fn cleanup_old_reference(entity: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity** ref)` (0x571B70)
    pub fn register_reference(entity: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// `CAEAudioHardware::GetSoundHeadroom(uint16 soundId, int16 bankSlot)` (0x4D8E30, ST0)
    pub fn hw_get_sound_headroom(hw: GPtr<u8>, sound: u32, slot: u32) -> f32 = thiscall 0x4D8E30 => unported;
    /// `CAEAudioUtility::GetRandomNumberInRange<float>(float, float)` (0x4D9C50, ST0)
    pub fn ae_random_float(min: f32, max: f32) -> f32 = cdecl 0x4D9C50 => crate::utility::random_float_f32;
    /// `CAESound::SetPosition(CVector)` (0x4EF880; the vector by value = three dwords)
    pub fn sound_set_position(this: GPtr<u8>, px: u32, py: u32, pz: u32) = thiscall 0x4EF880 => set_position;
    /// `CAEAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (virtual, vtable slot 0)
    pub fn entity_update_parameters(entity: GPtr<u8>, sound: GPtr<u8>, cur: u32) = thiscall vslot 0x00 => unported;
    /// `CAEAudioEnvironment::GetPositionRelativeToCamera(CVector* out, const CVector* pos)` (0x4D80B0)
    pub fn position_relative_to_camera_vec(out: GPtr<u8>, pos: GPtr<u8>) -> u32 = cdecl 0x4D80B0 => unported;
    /// `CAEAudioEnvironment::GetDistanceAttenuation(float)` (0x4D7F20, ST0)
    pub fn distance_attenuation(dist: f32) -> f32 = cdecl 0x4D7F20 => crate::utility::distance_attenuation_f32;
    /// `CAEAudioEnvironment::GetDirectionalMikeAttenuation(const CVector&)` (0x4D7F60, ST0)
    pub fn directional_mike_attenuation(dir: GPtr<u8>) -> f32 = cdecl 0x4D7F60 => crate::utility::directional_mike_attenuation_f32;
}

/// `CAESound::CAESound()` (0x446FE0) / `CAESound::~CAESound()` (0x4EF660), for the array constructor
const SOUND_CTOR: u32 = 0x446FE0;
const SOUND_DTOR: u32 = 0x4EF660;

/// The physical-entity reference (+8): `CleanUpOldReference(&m_pPhysicalEntity)` and null, when set.
fn release_physical(s: GPtr<u8>) {
    let r = s.byte_add(8);
    let e = rd::<GPtr<u8>>(s, 8);
    if !e.is_null() {
        cleanup_old_reference(e, r);
        wr::<u32>(s, 8, 0);
    }
}

/// `StopSoundAndForget` body: flag 4 cleared, stop requested, entity forgotten, reference released.
fn stop_and_forget(s: GPtr<u8>) {
    wr::<u8>(s, 0x56, rd::<u8>(s, 0x56) & 0xFB);
    wr::<u16>(s, 0x68, 1);
    wr::<u32>(s, 4, 0);
    release_physical(s);
}

// ---------------------------------------------------------------------------------------------------
// CAESound
// ---------------------------------------------------------------------------------------------------

/// `CAESound::SetFlags(uint16 mask, uint16 on)` (0x4EF2B0).
#[hook(addr = 0x4EF2B0, conv = thiscall, name = "CAESound::SetFlags", expect = "66 83 7C 24 08 00")]
pub fn set_flags(this: GPtr<u8>, mask: u32, on: u32) {
    let f = rd::<u16>(this, 0x56);
    let m = mask as u16;
    wr::<u16>(this, 0x56, if on as u16 != 0 { f | m } else { f & !m });
}

/// `CAESound::StopSound()` (0x4EF1C0; its body continues at 0x40972C and jumps back): stop requested
/// (+0x68 = 1), physical-entity reference released.
#[hook(addr = 0x4EF1C0, conv = thiscall, name = "CAESound::StopSound", expect = "56 8D 71 08 90")]
pub fn stop_sound(this: GPtr<u8>) {
    wr::<u16>(this, 0x68, 1);
    release_physical(this);
}

/// `CAESound::StopSoundAndForget()` (0x4EF850).
#[hook(addr = 0x4EF850, conv = thiscall, name = "CAESound::StopSoundAndForget", expect = "80 61 56 FB 56")]
pub fn stop_sound_and_forget(this: GPtr<u8>) {
    stop_and_forget(this);
}

/// `CAESound::RegisterWithPhysicalEntity(CEntity*)` (0x4EF820): the old reference released, the new
/// entity (when not null) stored and registered.
#[hook(addr = 0x4EF820, conv = thiscall, name = "CAESound::RegisterWithPhysicalEntity", expect = "56 8D 71 08 8B 0E")]
pub fn register_with_physical_entity(this: GPtr<u8>, entity: GPtr<u8>) {
    release_physical(this);
    if !entity.is_null() {
        wr::<GPtr<u8>>(this, 8, entity);
        register_reference(entity, this.byte_add(8));
    }
}

/// `CAESound::NewVPSLEntry()` (0x4EF7A0): playing / stop / +0x5A cleared, in use; headroom from the
/// hardware; final volume = volume + `GetRandomNumberInRange(-variance, variance)` when
/// `0 < variance < volume`, else the volume.
#[hook(addr = 0x4EF7A0, conv = thiscall, name = "CAESound::NewVPSLEntry", expect = "33 C0 56 8B F1")]
pub fn new_vpsl_entry(this: GPtr<u8>) {
    wr::<u16>(this, 0x5E, 0);
    wr::<u16>(this, 0x68, 0);
    wr::<u16>(this, 0x5A, 0);
    let slot = u32::from(rd::<u16>(this, 0));
    let id = u32::from(rd::<u16>(this, 2));
    wr::<u16>(this, 0x58, 1);
    let h = hw_get_sound_headroom(g(AE_AUDIO_HARDWARE), id, slot);
    wr::<f32>(this, 0x6C, h);
    let var: f32 = rd(this, 0x20);
    let vol: f32 = rd(this, 0x1C);
    // fcomp 0.0; test ah, 41h; jne -> plain volume (variance <= 0 or unordered)
    // fcomp volume; test ah, 5; jp -> plain volume (variance >= volume or unordered)
    if x(var).fcom(x(0.0)).test_any(0x41) || x(var).fcom(x(vol)).test_parity(0x05) {
        wr::<u32>(this, 0x64, rd::<u32>(this, 0x1C));
        return;
    }
    let lo = x(var).fmul_m(-1.0).to_f32();
    let r = ae_random_float(lo, var);
    wr::<f32>(this, 0x64, x(r).fadd_m(vol).to_f32());
}

/// `CAESound::Initialise(int16 bankSlot, int16 soundId, CAEAudioEntity*, CVector pos, float volume,
/// float maxDistance, float speed, float timeScale, uint8 ignoredServiceCycles, uint16 flags,
/// float speedVariance, int16 playPos)` (0x4EFE50, `ret 0x38`): fields set in the original's order;
/// the old physical reference is released, the previous position (+0x30) zeroed, event (+0xC) -1,
/// +0x10 -1.0, +0x3C 0 before `SetPosition(pos)`; the rest (flags included) after it.
#[hook(addr = 0x4EFE50, conv = thiscall, name = "CAESound::Initialise", expect = "66 8B 44 24 04")]
#[allow(clippy::too_many_arguments)]
pub fn initialise(
    this: GPtr<u8>,
    slot: u32,
    id: u32,
    entity: u32,
    px: u32,
    py: u32,
    pz: u32,
    volume: u32,
    max_distance: u32,
    speed: u32,
    time_scale: u32,
    ignored_cycles: u32,
    flags: u32,
    speed_variance: u32,
    play_pos: u32,
) {
    wr::<u16>(this, 2, id as u16);
    wr::<u16>(this, 0, slot as u16);
    wr::<u32>(this, 4, entity);
    release_physical(this);
    wr::<u32>(this, 0x14, volume);
    wr::<u32>(this, 0x18, max_distance);
    wr::<u32>(this, 0x1C, speed);
    wr::<u32>(this, 0x20, speed_variance);
    wr::<u32>(this, 0x30, 0);
    wr::<u32>(this, 0x34, 0);
    wr::<u32>(this, 0x38, 0);
    wr::<i32>(this, 0xC, -1);
    wr::<u32>(this, 0x10, 0xBF80_0000); // -1.0
    wr::<u32>(this, 0x3C, 0);
    sound_set_position(this, px, py, pz);
    wr::<u32>(this, 0x50, time_scale);
    wr::<u16>(this, 0x70, 0xFFFF);
    wr::<u16>(this, 0x5E, 0);
    wr::<u16>(this, 0x68, 0);
    wr::<u32>(this, 0x6C, 0);
    wr::<u8>(this, 0x54, ignored_cycles as u8);
    wr::<u16>(this, 0x56, flags as u16);
    wr::<u16>(this, 0x58, 1);
    wr::<u16>(this, 0x5C, play_pos as u16);
    wr::<u32>(this, 0x60, 0xC2C8_0000); // -100.0
    wr::<u32>(this, 0x64, 0x3F80_0000); // 1.0
}

/// `CAESound::operator=(const CAESound&)` (0x4EF680): nothing on self-assignment; otherwise the own
/// physical reference released, every field copied in the original's order (not +8, +0x55, +0x5A,
/// +0x6A, +0x72), then the source's physical entity (if any) stored and registered. Returns `this`.
#[hook(addr = 0x4EF680, conv = thiscall, name = "CAESound::operator=", expect = "56 57 8B 7C 24 0C")]
pub fn assign(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    if this.addr() == src.addr() {
        return this;
    }
    release_physical(this);
    for off in [0isize, 2] {
        wr::<u16>(this, off, rd::<u16>(src, off));
    }
    for off in [
        4isize, 0xC, 0x10, 0x14, 0x18, 0x1C, 0x20, 0x24, 0x28, 0x2C, 0x30, 0x34, 0x38, 0x3C, 0x40, 0x44,
        0x48, 0x4C, 0x50,
    ] {
        wr::<u32>(this, off, rd::<u32>(src, off));
    }
    wr::<u8>(this, 0x54, rd::<u8>(src, 0x54));
    for off in [0x56isize, 0x58, 0x5C, 0x5E] {
        wr::<u16>(this, off, rd::<u16>(src, off));
    }
    for off in [0x60isize, 0x64] {
        wr::<u32>(this, off, rd::<u32>(src, off));
    }
    wr::<u16>(this, 0x68, rd::<u16>(src, 0x68));
    wr::<u32>(this, 0x6C, rd::<u32>(src, 0x6C));
    wr::<u16>(this, 0x70, rd::<u16>(src, 0x70));
    wr::<u32>(this, 8, 0);
    let e = rd::<GPtr<u8>>(src, 8);
    if !e.is_null() {
        wr::<GPtr<u8>>(this, 8, e);
        register_reference(rd::<GPtr<u8>>(src, 8), this.byte_add(8));
    }
    this
}

/// `|pos - camera position|` as `SetPosition` computes it: `sqrt((dz*dz + dy*dy) + dx*dx)`.
fn camera_distance(px: u32, py: u32, pz: u32) -> f32 {
    let c = sa_world::world::common::cam_pos();
    let dx = x(f32::from_bits(px)).fsub_m(c.read());
    let dy = x(f32::from_bits(py)).fsub_m(c.byte_add(4).read());
    let dz = x(f32::from_bits(pz)).fsub_m(c.byte_add(8).read());
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt().to_f32()
}

/// `CAESound::SetPosition(CVector pos)` (0x4EF880): +0x24 current position, +0x30 previous position,
/// +0x3C frame of the last update (`CTimer` 0xB7CB4C), +0x40 / +0x44 current / previous time (0xB7CB84),
/// +0x48 / +0x4C current / previous camera distance. First call (frame 0): both positions and distances
/// set; a new frame: current values shift to previous; the same frame: only the current values.
/// Not hooked: the original keeps ECX and a caller (0x4EFDE4) reads it after the call (needs
/// `preserve = "ecx"` and an L1 spec); reached through `orig!` only.
pub fn set_position(this: GPtr<u8>, px: u32, py: u32, pz: u32) {
    let frame = sa_core::timer::FRAME_COUNTER.get();
    let last = rd::<u32>(this, 0x3C);
    let set_cur = |base: isize| {
        wr::<u32>(this, base, px);
        wr::<u32>(this, base + 4, py);
        wr::<u32>(this, base + 8, pz);
    };
    if last == 0 {
        set_cur(0x30);
        set_cur(0x24);
        let d = camera_distance(px, py, pz);
        wr::<f32>(this, 0x48, d);
        wr::<f32>(this, 0x4C, d);
        wr::<u32>(this, 0x3C, frame);
        let t = sa_core::timer::TIME_MS.get();
        wr::<u32>(this, 0x40, t);
        wr::<u32>(this, 0x44, t);
        return;
    }
    if frame != last {
        for off in [0x24isize, 0x28, 0x2C] {
            wr::<u32>(this, off + 0xC, rd::<u32>(this, off));
        }
        wr::<u32>(this, 0x4C, rd::<u32>(this, 0x48));
        wr::<u32>(this, 0x44, rd::<u32>(this, 0x40));
        set_cur(0x24);
        wr::<f32>(this, 0x48, camera_distance(px, py, pz));
        wr::<u32>(this, 0x3C, sa_core::timer::FRAME_COUNTER.get());
        wr::<u32>(this, 0x40, sa_core::timer::TIME_MS.get());
        return;
    }
    set_cur(0x24);
    wr::<f32>(this, 0x48, camera_distance(px, py, pz));
    wr::<u32>(this, 0x40, sa_core::timer::TIME_MS.get());
}

/// `CAESound::UpdateParameters(int16 curPlayPos)` (0x4EFF50): with flag 0x80, follows the physical
/// entity's position (`SetPosition` with its matrix or placement position) or, without an entity,
/// requests a stop; with flag 4 and an audio entity, the entity's virtual `UpdateParameters(this, pos)`
/// and then, for a zero volume variance, final volume = volume.
#[hook(addr = 0x4EFF50, conv = thiscall, name = "CAESound::UpdateParameters", expect = "56 8B F1 8A 46 56")]
pub fn update_parameters(this: GPtr<u8>, cur: u32) {
    if (rd::<u8>(this, 0x56) as i8) < 0 {
        let e = rd::<GPtr<u8>>(this, 8);
        if e.is_null() {
            wr::<u16>(this, 0x68, 1);
        } else {
            let m = rd::<GPtr<u8>>(e, 0x14);
            let p = if m.is_null() {
                e.byte_add(4)
            } else {
                m.byte_add(0x30)
            };
            sound_set_position(this, rd(p, 0), rd(p, 4), rd(p, 8));
        }
    }
    if rd::<u8>(this, 0x56) & 4 == 0 {
        return;
    }
    let ent = rd::<GPtr<u8>>(this, 4);
    if ent.is_null() {
        return;
    }
    entity_update_parameters(ent, this, cur);
    // fcomp 0.0; test ah, 44h; jp -> done (not equal / unordered)
    if !x(rd::<f32>(this, 0x20)).fcom(x(0.0)).test_parity(0x44) {
        wr::<u32>(this, 0x64, rd::<u32>(this, 0x1C));
    }
}

/// `CAESound::CalculateVolume()` (0x4EFA10): +0x60 = (flag 1: the volume (+0x14); otherwise the
/// directional mike attenuation + the distance attenuation of `|camera-relative position| /
/// sound distance (+0x18)` + the volume) - the headroom (+0x6C).
#[hook(addr = 0x4EFA10, conv = thiscall, name = "CAESound::CalculateVolume", expect = "83 EC 10 56 8B F1")]
pub fn calculate_volume(this: GPtr<u8>) {
    if rd::<u8>(this, 0x56) & 1 != 0 {
        wr::<f32>(this, 0x60, x(rd(this, 0x14)).to_f32());
    } else {
        let mut v = [0u32; 3];
        let vp = GPtr::<u32>::from_raw(v.as_mut_ptr()).cast::<u8>();
        position_relative_to_camera_vec(vp, this.byte_add(0x24));
        let [px, py, pz] = v.map(f32::from_bits);
        let len = x(pz)
            .fmul_m(pz)
            .fadd(x(py).fmul_m(py))
            .fadd(x(px).fmul_m(px))
            .fsqrt();
        let d = len.fdiv_m(rd(this, 0x18)).to_f32();
        let att = x(distance_attenuation(d)).to_f32();
        let mike = directional_mike_attenuation(vp);
        let r = x(mike).fadd_m(att).fadd_m(rd(this, 0x14));
        wr::<f32>(this, 0x60, r.to_f32());
    }
    let r = x(rd(this, 0x6C)).fsubr_m(rd(this, 0x60));
    wr::<f32>(this, 0x60, r.to_f32());
}

// ---------------------------------------------------------------------------------------------------
// CAESoundManager
// ---------------------------------------------------------------------------------------------------

fn sound(mgr: GPtr<u8>, i: isize) -> GPtr<u8> {
    mgr.byte_add(4 + i * SOUND_SIZE)
}

/// `CAESoundManager::CAESoundManager()` (0x4EFD30): the 300 sounds constructed; returns `this`.
#[hook(addr = 0x4EFD30, conv = thiscall, name = "CAESoundManager::CAESoundManager", expect = "56 68 60 F6 4E 00")]
pub fn sound_manager_ctor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(
        this.byte_add(4),
        SOUND_SIZE as u32,
        NUM_SOUNDS as u32,
        SOUND_CTOR,
        SOUND_DTOR,
    );
    this
}

/// `CAESoundManager::PauseManually(bool)` (0x4EF510): `this[+0x8CB5]`.
#[hook(addr = 0x4EF510, conv = thiscall, name = "CAESoundManager::PauseManually", expect = "8A 44 24 04 88 81 B5 8C 00 00")]
pub fn pause_manually(this: GPtr<u8>, pause: u8) {
    wr::<u8>(this, 0x8CB5, pause);
}

/// The `AreSounds*` scan: 2 at the first used match that is playing, else 1 when a used match exists,
/// else 0. Returned in AX (the original leaves the slot address's high half in EAX[31:16] for 2).
fn scan(this: GPtr<u8>, hit: impl Fn(GPtr<u8>) -> bool) -> u16 {
    let mut found = false;
    for i in 0..NUM_SOUNDS {
        let s = sound(this, i);
        if rd::<u16>(s, 0x58) != 0 && hit(s) {
            if rd::<u16>(s, 0x5E) != 0 {
                return 2;
            }
            found = true;
        }
    }
    u16::from(found)
}

/// `CAESoundManager::AreSoundsPlayingInBankSlot(int16 slot)` (0x4EF520).
#[hook(addr = 0x4EF520, conv = thiscall, name = "CAESoundManager::AreSoundsPlayingInBankSlot", expect = "53 56 66 8B 74 24 0C")]
pub fn are_sounds_playing_in_bank_slot(this: GPtr<u8>, slot: u32) -> u16 {
    scan(this, |s| rd::<u16>(s, 0) == slot as u16)
}

/// `CAESoundManager::AreSoundsOfThisEventPlayingForThisEntity(int16 event, CAEAudioEntity*)` (0x4EF570).
#[hook(addr = 0x4EF570, conv = thiscall, name = "CAESoundManager::AreSoundsOfThisEventPlayingForThisEntity", expect = "53 55 56 8B 74 24 14")]
pub fn are_sounds_of_this_event_playing_for_this_entity(this: GPtr<u8>, event: u32, entity: u32) -> u16 {
    let ev = i32::from(event as u16 as i16);
    scan(this, |s| rd::<i32>(s, 0xC) == ev && rd::<u32>(s, 4) == entity)
}

/// `CAESoundManager::AreSoundsOfThisEventPlayingForThisEntityAndPhysical(int16 event,
/// CAEAudioEntity*, CPhysical*)` (0x4EF5D0).
#[hook(addr = 0x4EF5D0, conv = thiscall, name = "CAESoundManager::AreSoundsOfThisEventPlayingForThisEntityAndPhysical", expect = "53 55 56 8B 74 24 18")]
pub fn are_sounds_of_this_event_playing_for_this_entity_and_physical(
    this: GPtr<u8>,
    event: u32,
    entity: u32,
    physical: u32,
) -> u16 {
    let ev = i32::from(event as u16 as i16);
    scan(this, |s| {
        rd::<i32>(s, 0xC) == ev && rd::<u32>(s, 4) == entity && rd::<u32>(s, 8) == physical
    })
}

/// `CAESoundManager::CancelSoundsOfThisEventPlayingForThisEntity(int16 event, CAEAudioEntity*)`
/// (0x4EFB90): every used match is stopped and forgotten.
#[hook(addr = 0x4EFB90, conv = thiscall, name = "CAESoundManager::CancelSoundsOfThisEventPlayingForThisEntity", expect = "51 56 8D 71 0C")]
pub fn cancel_sounds_of_this_event_playing_for_this_entity(this: GPtr<u8>, event: u32, entity: u32) {
    let ev = i32::from(event as u16 as i16);
    for i in 0..NUM_SOUNDS {
        let s = sound(this, i);
        if rd::<u16>(s, 0x58) != 0 && rd::<i32>(s, 0xC) == ev && rd::<u32>(s, 4) == entity {
            stop_and_forget(s);
        }
    }
}

/// `StopSound()` or, with `forget`, `StopSoundAndForget()` (as the cancel loops inline them).
fn stop_maybe_forget(s: GPtr<u8>, forget: u8) {
    if forget != 0 {
        stop_and_forget(s);
    } else {
        wr::<u16>(s, 0x68, 1);
        release_physical(s);
    }
}

/// `CAESoundManager::CancelSoundsInBankSlot(int16 slot, bool forget)` (0x4EFC60).
#[hook(addr = 0x4EFC60, conv = thiscall, name = "CAESoundManager::CancelSoundsInBankSlot", expect = "51 56 57 8D 71 0C")]
pub fn cancel_sounds_in_bank_slot(this: GPtr<u8>, slot: u32, forget: u8) {
    for i in 0..NUM_SOUNDS {
        let s = sound(this, i);
        if rd::<u16>(s, 0x58) != 0 && rd::<u16>(s, 0) == slot as u16 {
            stop_maybe_forget(s, forget);
        }
    }
}

/// `CAESoundManager::CancelSoundsOwnedByAudioEntity(CAEAudioEntity*, bool forget)` (0x4EFCD0).
#[hook(addr = 0x4EFCD0, conv = thiscall, name = "CAESoundManager::CancelSoundsOwnedByAudioEntity", expect = "51 56 57 8D 71 0C")]
pub fn cancel_sounds_owned_by_audio_entity(this: GPtr<u8>, entity: u32, forget: u8) {
    for i in 0..NUM_SOUNDS {
        let s = sound(this, i);
        if rd::<u16>(s, 0x58) != 0 && rd::<u32>(s, 4) == entity {
            stop_maybe_forget(s, forget);
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SET_FLAGS,
    HOOK_STOP_SOUND,
    HOOK_STOP_SOUND_AND_FORGET,
    HOOK_REGISTER_WITH_PHYSICAL_ENTITY,
    HOOK_NEW_VPSL_ENTRY,
    HOOK_INITIALISE,
    HOOK_ASSIGN,
    HOOK_UPDATE_PARAMETERS,
    HOOK_CALCULATE_VOLUME,
    HOOK_SOUND_MANAGER_CTOR,
    HOOK_PAUSE_MANUALLY,
    HOOK_ARE_SOUNDS_PLAYING_IN_BANK_SLOT,
    HOOK_ARE_SOUNDS_OF_THIS_EVENT_PLAYING_FOR_THIS_ENTITY,
    HOOK_ARE_SOUNDS_OF_THIS_EVENT_PLAYING_FOR_THIS_ENTITY_AND_PHYSICAL,
    HOOK_CANCEL_SOUNDS_OF_THIS_EVENT_PLAYING_FOR_THIS_ENTITY,
    HOOK_CANCEL_SOUNDS_IN_BANK_SLOT,
    HOOK_CANCEL_SOUNDS_OWNED_BY_AUDIO_ENTITY,
];
