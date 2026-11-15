//! FRONTIER-2 (batch `frontier2-1`): `CAEExplosionAudioEntity::AddAudioEvent` (0x4DCBE0), the explosion sounds (user-hit
//! with the burning car). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::field::gf;
use sa_core::hook;

use crate::bindings::{
    ae_resolve_probability, env_distance_attenuation_st0, hw_is_sound_bank_loaded, hw_load_sound_bank,
    position_relative_to_camera_vec, sm_request_new_sound, sound_initialise,
};
use crate::common::{GPtr, audio_hardware, g, rd, sound_manager, wr, x, xi};
use sa_world::bindings::sound_dtor;

/// The per-event volume table pointer (`[0xBD00F8]`, one int8 per event).
const EVENT_VOLUMES: usize = 0xBD_00F8;
/// The three explosion speeds (`float[3]` at 0x8AE58C).
const SPEEDS: usize = 0x8A_E58C;

/// `CAEExplosionAudioEntity::AddAudioEvent(int32 event, CVector& pos, float volume)` (0x4DCBE0, `ret 0xC`; its SEH
/// handler push runs in the no-CD chunk 0x40A265).
///
/// Only event 0x81; bank 0x34 must be loaded in slot 4 (else it is requested). The speed index (+0x7C) advances
/// modulo 3 (signed `idiv`). Three sounds of slot 4 at `pos` (sfx 4 / 3 / 2, max distance 2 / 4 / 7.5, volume
/// `(int8)[[0xBD00F8] + 0x81] + volume`, speed `SPEEDS[i]`, flags 4, variance 0.06). Then the camera-relative distance
/// `|rel| * [0x85F0FC]` gives `attenuation + volume - [0x858B3C]` for two frontend sounds (flags 0x1405, sfx 1, max
/// distance 12) at (-1, 0, 0) and (1, 0, 0); one of them (by `ResolveProbability(0.5)`) plays at `speed *
/// [0x85F0F8]`. A stack `CAESound` (only +8 cleared, destroyed on every path).
#[hook(addr = 0x4DCBE0, conv = thiscall, name = "CAEExplosionAudioEntity::AddAudioEvent", expect = "6A FF E9 7E D6 F2 FF",
       unverified = "frontier2-1", route = "L2", hazards = "uninit-locals,entry-protection-read")]
pub fn explosion_audio_add_event(this: GPtr<u8>, event: i32, pos: GPtr<u8>, volume: f32) {
    let left = [0xBF80_0000u32, 0, 0];
    let right = [0x3F80_0000u32, 0, 0];
    let mut buf = [0u32; 0x74 / 4];
    let sound = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(sound, 8, 0);
    let req = |s: GPtr<u8>| {
        sm_request_new_sound(sound_manager().addr() as u32, s.addr() as u32);
    };
    if event == 0x81 {
        if hw_is_sound_bank_loaded(audio_hardware(), 0x34, 4) == 0 {
            hw_load_sound_bank(audio_hardware(), 0x34, 4);
        } else {
            let base = rd::<i8>(g::<GPtr<u8>>(EVENT_VOLUMES).read(), 0x81);
            let i = (i32::from(rd::<i8>(this, 0x7C)) + 1) % 3;
            let vol = xi(i32::from(base)).fadd_m(volume).to_f32();
            wr(this, 0x7C, i as u8);
            let i = i32::from(i as i8) as usize;
            let speed = || g::<u32>(SPEEDS + 4 * i).read();
            let at = |off: isize| rd::<u32>(pos, off);
            for (sfx, dist) in [(4u32, 0x4000_0000u32), (3, 0x4080_0000), (2, 0x40F0_0000)] {
                sound_initialise(sound, 4, sfx, this.addr() as u32, at(0), at(4), at(8), vol.to_bits(), dist, speed(),
                                 0x3F80_0000, 0, 4, 0x3D75_C28F, 0);
                req(sound);
            }
            let mut rel = [0f32; 3];
            position_relative_to_camera_vec(GPtr::from_raw(rel.as_mut_ptr()).cast(), pos);
            let d = x(rel[2])
                .fmul_m(rel[2])
                .fadd(x(rel[1]).fmul_m(rel[1]))
                .fadd(x(rel[0]).fmul_m(rel[0]))
                .fsqrt()
                .fmul_m(gf(0x85_F0FC))
                .to_f32();
            let v2 = x(env_distance_attenuation_st0(d)).fadd_m(vol).to_f32();
            let p = ae_resolve_probability(0.5);
            let s = g::<f32>(SPEEDS + 4 * i).read();
            let (sa, sb) = if p != 0 {
                (x(s).fmul_m(gf(0x85_F0F8)).to_f32(), x(s).to_f32())
            } else {
                (x(s).to_f32(), x(s).fmul_m(gf(0x85_F0F8)).to_f32())
            };
            let v3 = x(v2).fsub_m(gf(0x85_8B3C)).to_f32().to_bits();
            for (p, sp) in [(left, sa), (right, sb)] {
                sound_initialise(sound, 4, 1, this.addr() as u32, p[0], p[1], p[2], v3, 0x4140_0000, sp.to_bits(),
                                 0x3F80_0000, 0, 0x1405, 0, 0);
                req(sound);
            }
        }
    }
    sound_dtor(sound);
}

pub const HOOKS: &[HookReg] = &[HOOK_EXPLOSION_AUDIO_ADD_EVENT];
