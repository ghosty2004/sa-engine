//! WRITER-NATIVE (batch `native-3`): `CAEWeaponAudioEntity::PlayGunSounds` (0x503CE0), the gunfire sounds
//! (ambient gangs, police) that blocked the native walk every frame once a ped fired. Ported from the 1.0 US
//! bytes; gta-reversed (`AEWeaponAudioEntity.cpp`) as a reading aid. x87 arithmetic follows the original's
//! instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::{AE_AUDIO_HARDWARE, AE_SOUND_MANAGER, AUDIO_ENGINE, GPtr, X87, g, rd, wr, x, xi};

orig! {
    /// `CAEAudioHardware::IsSoundBankLoaded(bank, slot)` (0x4D88C0)
    fn hw_is_sound_bank_loaded(hw: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4D88C0 => unported;
    /// `CAEAudioHardware::LoadSoundBank(bank, slot)` (0x4D88A0)
    fn hw_load_sound_bank(hw: GPtr<u8>, bank: u32, slot: u32) = thiscall 0x4D88A0 => unported;
    /// `CAudioEngine::IsLoadingTuneActive()` (0x506D90)
    fn engine_is_loading_tune_active(engine: GPtr<u8>) -> u8 = thiscall 0x506D90 => unported;
    /// `CAESound::Initialise(slot, sfx, CAEAudioEntity*, CVector pos, volume, maxDistance, speed, timeScale,
    /// ignoredServiceCycles, flags, speedVariance, playPos)` (0x4EFE50)
    #[allow(clippy::too_many_arguments)]
    fn sound_initialise(sound: GPtr<u8>, slot: u32, sfx: u32, entity: GPtr<u8>, px: u32, py: u32, pz: u32, vol: u32, max_dist: u32, speed: u32, time_scale: u32, ignored: u32, flags: u32, var: u32, play_pos: u32) = thiscall 0x4EFE50 => unported;
    /// `CAESound::RegisterWithPhysicalEntity(CEntity*)` (0x4EF820)
    fn sound_register_with_physical_entity(sound: GPtr<u8>, entity: GPtr<u8>) = thiscall 0x4EF820 => unported;
    /// `CAESoundManager::RequestNewSound(CAESound*)` (0x4EFB10)
    fn sm_request_new_sound(sm: GPtr<u8>, sound: GPtr<u8>) -> u32 = thiscall 0x4EFB10 => unported;
    /// `CAEAudioEnvironment::GetPositionRelativeToCamera(CVector* out, CPlaceable*)` (0x4D8340)
    fn env_position_relative_to_camera(out: GPtr<u8>, placeable: GPtr<u8>) -> u32 = cdecl 0x4D8340 => unported;
    /// `CAEAudioEnvironment::GetDistanceAttenuation(float)` (0x4D7F20, ST0)
    fn env_distance_attenuation_st0(dist: f32) -> f64 = cdecl 0x4D7F20 => unported;
    /// `CAEAudioUtility::GetRandomNumberInRange<float>(float, float)` (0x4D9C50, ST0)
    fn ae_random_float_st0(min: f32, max: f32) -> f64 = cdecl 0x4D9C50 => unported;
    /// `CAEAudioUtility::ResolveProbability(float)` (0x4D9C80)
    fn ae_resolve_probability(p: f32) -> u8 = cdecl 0x4D9C80 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude_st0(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
}

/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIME_MS: usize = 0xB7_CB84;
/// `CAEAudioEntity::m_pAudioEventVolumes` (0xBD00F8, an `int8` per event)
const EVENT_VOLUMES: usize = 0xBD_00F8;
/// `gfWeaponPlaneFrequencyVariations` (0x8CC29C)
const PLANE_FREQ_VARIATIONS: usize = 0x8C_C29C;

const AE_WEAPON_FIRE_PLANE: u32 = 0x95;
const AE_WEAPON_FIRE_MINIGUN_AMMO: u32 = 0x96;
const AE_WEAPON_FIRE_MINIGUN_PLANE: u32 = 0x98;

fn bits(v: f32) -> u32 {
    v.to_bits()
}

/// The entity's position (`m_matrix` +0x30, else the placement position +4).
fn position(e: GPtr<u8>) -> [u32; 3] {
    let m = rd::<GPtr<u8>>(e, 0x14);
    let p = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
    [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)]
}

/// One `m_tempSound` play: `Initialise` (bank slot 5), optionally `RegisterWithPhysicalEntity`, the event
/// override, `RequestNewSound`.
#[allow(clippy::too_many_arguments)]
fn play(this: GPtr<u8>, sfx: u32, pos: [u32; 3], vol: u32, max_dist: u32, speed: u32, flags: u32, register: Option<GPtr<u8>>, event: Option<u32>) {
    let sound = this.byte_add(8);
    sound_initialise(sound, 5, sfx, this, pos[0], pos[1], pos[2], vol, max_dist, speed, 0x3F80_0000, 0, flags, 0, 0);
    if let Some(e) = register {
        sound_register_with_physical_entity(sound, e);
    }
    if let Some(ev) = event {
        wr::<u32>(this, 0x14, ev);
    }
    sm_request_new_sound(g::<u8>(AE_SOUND_MANAGER), sound);
}

/// `CAEWeaponAudioEntity::PlayGunSounds(CPhysical* entity, int16 dryFire, int16 sub, int16 mainLeft, int16
/// mainRight, int16 tail, eAudioEvents event, float volumeOffset, float speed, float tailFrequencyScale)`
/// (0x503CE0, thiscall, `ret 0x28`).
///
/// Bank 0x8F in slot 5 not loaded: load it unless the loading tune plays, and return. Less than 25 ms since
/// the last shot (+0x98): return. Volume = the event's default volume + offset; plane fire (0x95) toggles the
/// frequency index +0x7E and uses roll-off 1.6 and `speed * variation * 0.7937`, minigun plane fire (0x98)
/// 1.8 and `speed * 0.7937`. The dry-fire and sub sounds play at the entity (roll-off x 0.66 / x 0.9, flags
/// 0x84, registered). Except for the plane events, the camera distance `|rel| / (rollOff * 1.25)` splits a
/// front-end volume (-100 beyond `12 / rollOff`; `atten + vol - 3` within `5 / rollOff`, which also lowers
/// the base volume by 3; between them a 20 log10 cross-fade with 0.7071 / 0.2929) from the base volume.
/// Speed x `(1 + rand(-0.02, 0.02))`. Main left / right: a front-end pair at x = -1 / +1 (flags 0x1005) and
/// again at the entity with the base volume. The tail (not for plane events): a front-end pair at x = -1 / +1
/// with one side's speed x 1.18921 (probability 0.5), roll-off x 3.5, volume `atten + vol - 20` capped at 0,
/// event 7 for minigun ammo, else 1. Every other sound gets event 6 for the minigun events. The original
/// keeps intermediate values in its argument slots (stack only).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x503CE0, conv = thiscall, name = "CAEWeaponAudioEntity::PlayGunSounds", this = "CAEWeaponAudioEntity",
       expect = "83 EC 28 53 55 56 57 6A 05 8B F1 E9",
       unverified = "native-3", route = "L2", hazards = "x87-transcendental,heap-writes,stack-residue",
       recs = "grove-walk",
       note = "push 0x8F comes from the protection chunk 0x405B7E; fyl2x; sounds go to the sound manager")]
pub fn play_gun_sounds(
    this: GPtr<u8>,
    entity: GPtr<u8>,
    dry_fire: u32,
    sub: u32,
    main_left: u32,
    main_right: u32,
    tail: u32,
    event: u32,
    volume_offset: f32,
    speed_arg: f32,
    tail_freq: f32,
) {
    if hw_is_sound_bank_loaded(g::<u8>(AE_AUDIO_HARDWARE), 0x8F, 5) == 0 {
        if engine_is_loading_tune_active(g::<u8>(AUDIO_ENGINE)) == 0 {
            hw_load_sound_bank(g::<u8>(AE_AUDIO_HARDWARE), 0x8F, 5);
        }
        return;
    }
    let now = g::<u32>(TIME_MS).read();
    if now < rd::<u32>(this, 0x98).wrapping_add(0x19) {
        return;
    }
    wr::<u32>(this, 0x98, now);
    let def = g::<GPtr<i8>>(EVENT_VOLUMES).read().byte_add(event as isize).read();
    let volume = xi(i32::from(def)).fadd_m(volume_offset).to_f32();
    let mut speed = speed_arg;
    let roll_off: f32;
    if event == AE_WEAPON_FIRE_PLANE {
        let idx = ((i32::from(rd::<i8>(this, 0x7E)) + 1) % 2) as i8;
        wr::<i8>(this, 0x7E, idx);
        roll_off = f32::from_bits(0x3FCC_CCCD);
        let var = g::<f32>(PLANE_FREQ_VARIATIONS).wrapping_index(i32::from(idx)).read();
        speed = x(speed).fmul_m(var).fmul_m(f32::from_bits(0x3F4B_2FEC)).to_f32();
    } else if event == AE_WEAPON_FIRE_MINIGUN_PLANE {
        roll_off = f32::from_bits(0x3FE6_6666);
        speed = x(speed).fmul_m(f32::from_bits(0x3F4B_2FEC)).to_f32();
    } else {
        roll_off = 1.0;
    }
    let minigun_event = (event == AE_WEAPON_FIRE_MINIGUN_AMMO || event == AE_WEAPON_FIRE_MINIGUN_PLANE).then_some(6);
    if dry_fire as u16 != 0xFFFF {
        let rd_off = x(roll_off).fmul_m(f32::from_bits(0x3F28_F5C3)).to_f32();
        play(this, dry_fire, position(entity), bits(volume), bits(rd_off), bits(speed), 0x84, Some(entity), minigun_event);
    }
    if sub as u16 != 0xFFFF {
        let rd_off = x(roll_off).fmul_m(f32::from_bits(0x3F66_6666)).to_f32();
        play(this, sub, position(entity), bits(volume), bits(rd_off), bits(speed), 0x84, Some(entity), minigun_event);
    }
    // front-end volume (arg slot +0x40) and base volume (arg slot +0x44)
    let (front_end, base): (f32, f32);
    if event == AE_WEAPON_FIRE_PLANE || event == AE_WEAPON_FIRE_MINIGUN_PLANE {
        front_end = -100.0;
        base = volume;
    } else {
        let mut rel = [0u32; 3];
        let relp: GPtr<u8> = GPtr::from_raw(&mut rel as *mut [u32; 3]).cast();
        env_position_relative_to_camera(relp, entity);
        let (rx, ry, rz) = (f32::from_bits(rel[0]), f32::from_bits(rel[1]), f32::from_bits(rel[2]));
        let len = x(rz).fmul_m(rz).fadd(x(ry).fmul_m(ry)).fadd(x(rx).fmul_m(rx)).fsqrt();
        let d = len.fdiv(x(roll_off).fmul_m(1.25)).to_f32();
        let inv = x(1.0).fdiv_m(roll_off);
        let near = x(5.0).fmul(inv).to_f32();
        if !x(d).fcom(x(near)).test_parity(0x41) {
            let b = x(volume).fsub_m(3.0).to_f32();
            let a = X87::from_f64_exact(env_distance_attenuation_st0(d));
            front_end = a.fadd_m(b).to_f32();
            base = b;
        } else {
            let far = inv.fmul_m(12.0);
            if x(d).fcom(far).test_any(0x01) {
                let t = far.fsub(x(d)).fdiv(far.fsub_m(near)).to_f32();
                let a = X87::from_f64_exact(env_distance_attenuation_st0(d));
                let l = X87::lg2().fyl2x(x(t).fmul_m(f32::from_bits(0x3F35_0481)));
                front_end = a.fadd(l.fmul_m(20.0)).fadd_m(volume).to_f32();
                let r = x(1.0)
                    .fsub_m(t)
                    .fmul_m(f32::from_bits(0x3E95_F6FE))
                    .fadd_m(f32::from_bits(0x3F35_0481));
                base = X87::lg2().fyl2x(r).fmul_m(20.0).fadd_m(volume).to_f32();
            } else {
                front_end = -100.0;
                base = volume;
            }
        }
    }
    let rnd = X87::from_f64_exact(ae_random_float_st0(f32::from_bits(0xBCA3_D70A), f32::from_bits(0x3CA3_D70A)));
    speed = rnd.fadd_m(1.0).fmul_m(speed).to_f32();
    let main_roll = x(roll_off).fmul_m(1.25).to_f32();
    let left = [bits(-1.0), 0, 0];
    let right = [bits(1.0), 0, 0];
    if main_left as u16 != 0xFFFF {
        play(this, main_left, left, bits(front_end), bits(main_roll), bits(speed), 0x1005, None, minigun_event);
    }
    if main_right as u16 != 0xFFFF {
        play(this, main_right, right, bits(front_end), bits(main_roll), bits(speed), 0x1005, None, minigun_event);
    }
    if main_left as u16 != 0xFFFF {
        play(this, main_left, position(entity), bits(base), bits(main_roll), bits(speed), 0x84, Some(entity), minigun_event);
    }
    if main_right as u16 != 0xFFFF {
        play(this, main_right, position(entity), bits(base), bits(main_roll), bits(speed), 0x84, Some(entity), minigun_event);
    }
    if tail as u16 == 0xFFFF || event == AE_WEAPON_FIRE_PLANE || event == AE_WEAPON_FIRE_MINIGUN_PLANE {
        return;
    }
    let p = ae_resolve_probability(0.5);
    let scaled = x(tail_freq).fmul_m(f32::from_bits(0x3F98_3809)).to_f32();
    let (speed_l, speed_r) = if p != 0 { (scaled, tail_freq) } else { (tail_freq, scaled) };
    let mut rel = [0u32; 3];
    let relp: GPtr<u8> = GPtr::from_raw(&mut rel as *mut [u32; 3]).cast();
    env_position_relative_to_camera(relp, entity);
    let tail_roll = x(roll_off).fmul_m(3.5).to_f32();
    let mag = X87::from_f64_exact(vector_magnitude_st0(relp));
    let d = mag.fdiv_m(tail_roll).to_f32();
    let a = X87::from_f64_exact(env_distance_attenuation_st0(d));
    let tv = a.fadd(x(volume).fsub_m(20.0));
    let tvol = tv.to_f32();
    let v1 = if tv.fcom(x(0.0)).test_parity(0x05) { 0.0 } else { tvol };
    let tail_event = if event == AE_WEAPON_FIRE_MINIGUN_AMMO { 7 } else { 1 };
    play(this, tail, left, bits(v1), bits(tail_roll), bits(speed_l), 0x1405, None, Some(tail_event));
    let v2 = if x(tvol).fcom(x(0.0)).test_parity(0x05) { 0.0 } else { tvol };
    play(this, tail, right, bits(v2), bits(tail_roll), bits(speed_r), 0x1405, None, Some(tail_event));
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PLAY_GUN_SOUNDS];
