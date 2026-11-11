//! The AUDIO static initialisers (PORT-AUDIO): CRT `__init_*` functions that construct audio globals before
//! `WinMain`. Ported from the 1.0 US bytes. Constructors and the CRT helpers stay `orig!` calls.
//!
//! The `__init_g_*` functions compute derived float constants (mostly `hi - lo` range widths of the
//! audio entities' tuning tables). They run before D3D sets precision control 24, i.e. under the CRT's
//! PC=53; for one f32 subtraction / multiplication / division of f32 operands the double rounding
//! (to 53, then to 24 bits on `fstp m32`) equals the single PC=24 rounding `X87` models (53 >= 2*24+2).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, x};

/// No globals of its own (the constructed objects and the derived constants belong to their classes).
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// `CAESoundManager::CAESoundManager()` (0x4EFD30)
    pub fn sound_manager_ctor(this: GPtr<u8>) = thiscall 0x4EFD30 => unported;
    /// `CAETwinLoopSoundEntity::CAETwinLoopSoundEntity()` (0x4F63B0, not catalogued)
    pub fn twin_loop_ctor(this: GPtr<u8>) = thiscall 0x4F63B0 => unported;
    /// `CAEAudioHardware::Constructor` (0x4D83E0)
    pub fn audio_hardware_ctor(this: GPtr<u8>) = thiscall 0x4D83E0 => unported;
    /// `CAudioEngine::Constructor` (0x507670)
    pub fn audio_engine_ctor(this: GPtr<u8>) = thiscall 0x507670 => unported;
    /// `CAESmoothFadeThread::CAESmoothFadeThread()` (0x4EE9A0, not catalogued)
    pub fn smooth_fade_thread_ctor(this: GPtr<u8>) = thiscall 0x4EE9A0 => unported;
    /// `CAEStreamTransformer::Initialise` (0x4F1750; the catalogue's name for the constructor)
    pub fn stream_transformer_ctor(this: GPtr<u8>) = thiscall 0x4F1750 => unported;
}

/// `__init_AESoundManager` (0x84CD20): `AESoundManager.CAESoundManager()` (0xB62CB0), `atexit`.
#[hook(addr = 0x84CD20, conv = cdecl, name = "__init_AESoundManager", expect = "B9 B0 2C B6 00")]
pub fn init_ae_sound_manager() {
    sound_manager_ctor(g(0xB62CB0));
    atexit(0x856360);
}

/// `__init_m_sRainSoundL` (0x84D4D0): `CAEWeatherAudioEntity::m_sRainSoundL` (0xB6BB18, a
/// `CAETwinLoopSoundEntity`), `atexit`.
#[hook(addr = 0x84D4D0, conv = cdecl, name = "__init_m_sRainSoundL", expect = "B9 18 BB B6 00")]
pub fn init_rain_sound_l() {
    twin_loop_ctor(g(0xB6BB18));
    atexit(0x856380);
}

/// `__init_m_sRainSoundR` (0x84D4F0): `CAEWeatherAudioEntity::m_sRainSoundR` (0xB6BBC0), `atexit`.
#[hook(addr = 0x84D4F0, conv = cdecl, name = "__init_m_sRainSoundR", expect = "B9 C0 BB B6 00")]
pub fn init_rain_sound_r() {
    twin_loop_ctor(g(0xB6BBC0));
    atexit(0x856390);
}

/// `__init_AEAudioHardware` (0x84C270): `AEAudioHardware` constructor (0xB5F8B8), `atexit`.
#[hook(addr = 0x84C270, conv = cdecl, name = "__init_AEAudioHardware", expect = "B9 B8 F8 B5 00")]
pub fn init_ae_audio_hardware() {
    audio_hardware_ctor(g(0xB5F8B8));
    atexit(0x8562F0);
}

/// `__init_AudioEngine` (0x84D5D0): `AudioEngine` constructor (0xB6BC90), `atexit`.
#[hook(addr = 0x84D5D0, conv = cdecl, name = "__init_AudioEngine", expect = "B9 90 BC B6 00")]
pub fn init_audio_engine() {
    audio_engine_ctor(g(0xB6BC90));
    atexit(0x8563A0);
}

/// `__init_AESmoothFadeThread` (0x84C290): `AESmoothFadeThread` constructor (0xB608D0), `atexit`.
#[hook(addr = 0x84C290, conv = cdecl, name = "__init_AESmoothFadeThread", expect = "B9 D0 08 B6 00")]
pub fn init_ae_smooth_fade_thread() {
    smooth_fade_thread_ctor(g(0xB608D0));
    atexit(0x856300);
}

/// `__init_AEStreamTransformer` (0x84C500): `AEStreamTransformer` constructor (0xB612D8), `atexit`.
#[hook(addr = 0x84C500, conv = cdecl, name = "__init_AEStreamTransformer", expect = "B9 D8 12 B6 00")]
pub fn init_ae_stream_transformer() {
    stream_transformer_ctor(g(0xB612D8));
    atexit(0x856320);
}

/// `__init_CAEPedSpeechAudioEntity__s_PedSpeechSlots` (0x84C9A0): `eh vector constructor iterator(
/// 0xB61C38, 0x1C, 6, ctor 0x4E3330, dtor 0x4E3340)`, `atexit`.
#[hook(addr = 0x84C9A0, conv = cdecl, name = "__init_CAEPedSpeechAudioEntity__s_PedSpeechSlots", expect = "68 40 33 4E 00")]
pub fn init_ped_speech_slots() {
    eh_vector_ctor(g(0xB61C38), 0x1C, 6, 0x4E3330, 0x4E3340);
    atexit(0x856330);
}

/// `__init_g_b6ba54` (0x84CFC0): `[0xB6BA54] = [0xB6BA4C] / (float)(int16)[0x8CBC8C]`.
#[hook(addr = 0x84CFC0, conv = cdecl, name = "__init_g_b6ba54", expect = "51 0F BF 05 8C BC 8C 00")]
pub fn init_g_b6ba54() {
    let n = i32::from(g::<i16>(0x8CBC8C).read());
    let v = X87::from_i32(n).fdivr_m(g::<f32>(0xB6BA4C).read());
    g::<f32>(0xB6BA54).write(v.to_f32());
}

/// `__init_g_b613b0` (0x84C7E0): `[0xB613B0] = [0x8AE60C] - [0x8AE608]`.
#[hook(addr = 0x84C7E0, conv = cdecl, name = "__init_g_b613b0", expect = "D9 05 0C E6 8A 00")]
pub fn init_g_b613b0() {
    let v = x(g::<f32>(0x8AE60C).read()).fsub_m(g::<f32>(0x8AE608).read());
    g::<f32>(0xB613B0).write(v.to_f32());
}

/// `__init_g_b613bc` (0x84C840): `[0xB613BC] = [0x8AE65C] - [0x8AE658]`.
#[hook(addr = 0x84C840, conv = cdecl, name = "__init_g_b613bc", expect = "D9 05 5C E6 8A 00")]
pub fn init_g_b613bc() {
    let v = x(g::<f32>(0x8AE65C).read()).fsub_m(g::<f32>(0x8AE658).read());
    g::<f32>(0xB613BC).write(v.to_f32());
}

/// `__init_g_b613c0` (0x84C860): `[0xB613C0] = [0x8AE66C] - [0x8AE668]`.
#[hook(addr = 0x84C860, conv = cdecl, name = "__init_g_b613c0", expect = "D9 05 6C E6 8A 00")]
pub fn init_g_b613c0() {
    let v = x(g::<f32>(0x8AE66C).read()).fsub_m(g::<f32>(0x8AE668).read());
    g::<f32>(0xB613C0).write(v.to_f32());
}

/// `__init_g_b613cc` (0x84C8C0): `[0xB613CC] = [0x858624] - [0x8AE69C]`.
#[hook(addr = 0x84C8C0, conv = cdecl, name = "__init_g_b613cc", expect = "D9 05 24 86 85 00")]
pub fn init_g_b613cc() {
    let v = x(g::<f32>(0x858624).read()).fsub_m(g::<f32>(0x8AE69C).read());
    g::<f32>(0xB613CC).write(v.to_f32());
}

/// `__init_g_b6ba20` (0x84CE30): `[0xB6BA20] = [0x8CBBFC] - [0x8CBBF8]`.
#[hook(addr = 0x84CE30, conv = cdecl, name = "__init_g_b6ba20", expect = "D9 05 FC BB 8C 00")]
pub fn init_g_b6ba20() {
    let v = x(g::<f32>(0x8CBBFC).read()).fsub_m(g::<f32>(0x8CBBF8).read());
    g::<f32>(0xB6BA20).write(v.to_f32());
}

/// `__init_g_b6ba24` (0x84CE50): `[0xB6BA24] = [0xB6B9CC] - [0x8CBC00]`.
#[hook(addr = 0x84CE50, conv = cdecl, name = "__init_g_b6ba24", expect = "D9 05 CC B9 B6 00")]
pub fn init_g_b6ba24() {
    let v = x(g::<f32>(0xB6B9CC).read()).fsub_m(g::<f32>(0x8CBC00).read());
    g::<f32>(0xB6BA24).write(v.to_f32());
}

/// `__init_g_b6ba28` (0x84CE70): `[0xB6BA28] = [0x8CBC10] - [0x8CBC0C]`.
#[hook(addr = 0x84CE70, conv = cdecl, name = "__init_g_b6ba28", expect = "D9 05 10 BC 8C 00")]
pub fn init_g_b6ba28() {
    let v = x(g::<f32>(0x8CBC10).read()).fsub_m(g::<f32>(0x8CBC0C).read());
    g::<f32>(0xB6BA28).write(v.to_f32());
}

/// `__init_g_b6ba2c` (0x84CE90): `[0xB6BA2C] = [0xB6B9D0] - [0x8CBC14]`.
#[hook(addr = 0x84CE90, conv = cdecl, name = "__init_g_b6ba2c", expect = "D9 05 D0 B9 B6 00")]
pub fn init_g_b6ba2c() {
    let v = x(g::<f32>(0xB6B9D0).read()).fsub_m(g::<f32>(0x8CBC14).read());
    g::<f32>(0xB6BA2C).write(v.to_f32());
}

/// `__init_g_b6ba38` (0x84CEF0): `[0xB6BA38] = [0xB6BA34] * [0x858CC4]`.
#[hook(addr = 0x84CEF0, conv = cdecl, name = "__init_g_b6ba38", expect = "D9 05 34 BA B6 00")]
pub fn init_g_b6ba38() {
    let v = x(g::<f32>(0xB6BA34).read()).fmul_m(g::<f32>(0x858CC4).read());
    g::<f32>(0xB6BA38).write(v.to_f32());
}

/// `__init_g_b6ba3c` (0x84CF10): `[0xB6BA3C] = [0x8CBC68]` (a 32-bit copy).
#[hook(addr = 0x84CF10, conv = cdecl, name = "__init_g_b6ba3c", expect = "A1 68 BC 8C 00")]
pub fn init_g_b6ba3c() {
    g::<u32>(0xB6BA3C).write(g::<u32>(0x8CBC68).read());
}

/// `__init_g_b6ba40` (0x84CF20): `[0xB6BA40] = [0x8CBC74] - [0x8CBC70]`.
#[hook(addr = 0x84CF20, conv = cdecl, name = "__init_g_b6ba40", expect = "D9 05 74 BC 8C 00")]
pub fn init_g_b6ba40() {
    let v = x(g::<f32>(0x8CBC74).read()).fsub_m(g::<f32>(0x8CBC70).read());
    g::<f32>(0xB6BA40).write(v.to_f32());
}

/// `__init_g_b6ba44` (0x84CF40): `[0xB6BA44] = [0x8CBC7C] - [0x8CBC78]`.
#[hook(addr = 0x84CF40, conv = cdecl, name = "__init_g_b6ba44", expect = "D9 05 7C BC 8C 00")]
pub fn init_g_b6ba44() {
    let v = x(g::<f32>(0x8CBC7C).read()).fsub_m(g::<f32>(0x8CBC78).read());
    g::<f32>(0xB6BA44).write(v.to_f32());
}

/// `__init_g_b6ba50` (0x84CFA0): `[0xB6BA50] = [0xB6BA48] * [0x862D6C]`.
#[hook(addr = 0x84CFA0, conv = cdecl, name = "__init_g_b6ba50", expect = "D9 05 48 BA B6 00")]
pub fn init_g_b6ba50() {
    let v = x(g::<f32>(0xB6BA48).read()).fmul_m(g::<f32>(0x862D6C).read());
    g::<f32>(0xB6BA50).write(v.to_f32());
}

/// `__init_g_b6ba58` (0x84CFE0): `[0xB6BA58] = [0x8CBC6C]` (a 32-bit copy).
#[hook(addr = 0x84CFE0, conv = cdecl, name = "__init_g_b6ba58", expect = "A1 6C BC 8C 00")]
pub fn init_g_b6ba58() {
    g::<u32>(0xB6BA58).write(g::<u32>(0x8CBC6C).read());
}

/// `__init_g_b6ba5c` (0x84CFF0): `[0xB6BA5C] = [0x8CBC98] - [0x8CBC94]`.
#[hook(addr = 0x84CFF0, conv = cdecl, name = "__init_g_b6ba5c", expect = "D9 05 98 BC 8C 00")]
pub fn init_g_b6ba5c() {
    let v = x(g::<f32>(0x8CBC98).read()).fsub_m(g::<f32>(0x8CBC94).read());
    g::<f32>(0xB6BA5C).write(v.to_f32());
}

/// `__init_g_b6ba64` (0x84D030): `[0xB6BA64] = [0xB6B9DC] - [0x8CBCA8]`.
#[hook(addr = 0x84D030, conv = cdecl, name = "__init_g_b6ba64", expect = "D9 05 DC B9 B6 00")]
pub fn init_g_b6ba64() {
    let v = x(g::<f32>(0xB6B9DC).read()).fsub_m(g::<f32>(0x8CBCA8).read());
    g::<f32>(0xB6BA64).write(v.to_f32());
}

/// `__init_g_b6ba68` (0x84D050): `[0xB6BA68] = [0x8CBCB4] - [0x8CBCB0]`.
#[hook(addr = 0x84D050, conv = cdecl, name = "__init_g_b6ba68", expect = "D9 05 B4 BC 8C 00")]
pub fn init_g_b6ba68() {
    let v = x(g::<f32>(0x8CBCB4).read()).fsub_m(g::<f32>(0x8CBCB0).read());
    g::<f32>(0xB6BA68).write(v.to_f32());
}

/// `__init_g_b6ba6c` (0x84D070): `[0xB6BA6C] = [0x8CBD5C] - [0x8CBD58]`.
#[hook(addr = 0x84D070, conv = cdecl, name = "__init_g_b6ba6c", expect = "D9 05 5C BD 8C 00")]
pub fn init_g_b6ba6c() {
    let v = x(g::<f32>(0x8CBD5C).read()).fsub_m(g::<f32>(0x8CBD58).read());
    g::<f32>(0xB6BA6C).write(v.to_f32());
}

/// `__init_g_b6ba70` (0x84D090): `[0xB6BA70] = [0x8CBD68] - [0x8CBD64]`.
#[hook(addr = 0x84D090, conv = cdecl, name = "__init_g_b6ba70", expect = "D9 05 68 BD 8C 00")]
pub fn init_g_b6ba70() {
    let v = x(g::<f32>(0x8CBD68).read()).fsub_m(g::<f32>(0x8CBD64).read());
    g::<f32>(0xB6BA70).write(v.to_f32());
}

/// `__init_g_b6ba74` (0x84D0B0): `[0xB6BA74] = [0x8CBED0] - [0x858CC8]`.
#[hook(addr = 0x84D0B0, conv = cdecl, name = "__init_g_b6ba74", expect = "D9 05 D0 BE 8C 00")]
pub fn init_g_b6ba74() {
    let v = x(g::<f32>(0x8CBED0).read()).fsub_m(g::<f32>(0x858CC8).read());
    g::<f32>(0xB6BA74).write(v.to_f32());
}

/// `__init_g_b6ba78` (0x84D0D0): `[0xB6BA78] = [0x8CBF04] - [0x8CBF08]`.
#[hook(addr = 0x84D0D0, conv = cdecl, name = "__init_g_b6ba78", expect = "D9 05 04 BF 8C 00")]
pub fn init_g_b6ba78() {
    let v = x(g::<f32>(0x8CBF04).read()).fsub_m(g::<f32>(0x8CBF08).read());
    g::<f32>(0xB6BA78).write(v.to_f32());
}

/// `__init_g_b6ba80` (0x84D110): `[0xB6BA80] = [0x8CBF20] - [0x8CBF24]`.
#[hook(addr = 0x84D110, conv = cdecl, name = "__init_g_b6ba80", expect = "D9 05 20 BF 8C 00")]
pub fn init_g_b6ba80() {
    let v = x(g::<f32>(0x8CBF20).read()).fsub_m(g::<f32>(0x8CBF24).read());
    g::<f32>(0xB6BA80).write(v.to_f32());
}

/// `__init_g_b6ba84` (0x84D130): `[0xB6BA84] = [0x8CBF28] - [0x8CBF2C]`.
#[hook(addr = 0x84D130, conv = cdecl, name = "__init_g_b6ba84", expect = "D9 05 28 BF 8C 00")]
pub fn init_g_b6ba84() {
    let v = x(g::<f32>(0x8CBF28).read()).fsub_m(g::<f32>(0x8CBF2C).read());
    g::<f32>(0xB6BA84).write(v.to_f32());
}

/// `__init_g_b6ba88` (0x84D150): `[0xB6BA88] = [0x8CBF48] - [0x8CBF44]`.
#[hook(addr = 0x84D150, conv = cdecl, name = "__init_g_b6ba88", expect = "D9 05 48 BF 8C 00")]
pub fn init_g_b6ba88() {
    let v = x(g::<f32>(0x8CBF48).read()).fsub_m(g::<f32>(0x8CBF44).read());
    g::<f32>(0xB6BA88).write(v.to_f32());
}

/// `__init_g_b6ba90` (0x84D1B0): `[0xB6BA90] = [0x8CBFB0] - [0x8CBFAC]`.
#[hook(addr = 0x84D1B0, conv = cdecl, name = "__init_g_b6ba90", expect = "D9 05 B0 BF 8C 00")]
pub fn init_g_b6ba90() {
    let v = x(g::<f32>(0x8CBFB0).read()).fsub_m(g::<f32>(0x8CBFAC).read());
    g::<f32>(0xB6BA90).write(v.to_f32());
}

/// `__init_g_b6ba94` (0x84D1D0): `[0xB6BA94] = [0x8CBFB8] - [0x8CBFB4]`.
#[hook(addr = 0x84D1D0, conv = cdecl, name = "__init_g_b6ba94", expect = "D9 05 B8 BF 8C 00")]
pub fn init_g_b6ba94() {
    let v = x(g::<f32>(0x8CBFB8).read()).fsub_m(g::<f32>(0x8CBFB4).read());
    g::<f32>(0xB6BA94).write(v.to_f32());
}

/// `__init_g_b6ba9c` (0x84D210): `[0xB6BA9C] = [0x8CBFD4] - [0x8CBFD0]`.
#[hook(addr = 0x84D210, conv = cdecl, name = "__init_g_b6ba9c", expect = "D9 05 D4 BF 8C 00")]
pub fn init_g_b6ba9c() {
    let v = x(g::<f32>(0x8CBFD4).read()).fsub_m(g::<f32>(0x8CBFD0).read());
    g::<f32>(0xB6BA9C).write(v.to_f32());
}

/// `__init_g_b6baa0` (0x84D230): `[0xB6BAA0] = [0x8CBFFC] - [0x8CBFF8]`.
#[hook(addr = 0x84D230, conv = cdecl, name = "__init_g_b6baa0", expect = "D9 05 FC BF 8C 00")]
pub fn init_g_b6baa0() {
    let v = x(g::<f32>(0x8CBFFC).read()).fsub_m(g::<f32>(0x8CBFF8).read());
    g::<f32>(0xB6BAA0).write(v.to_f32());
}

/// `__init_g_b6baa4` (0x84D250): `[0xB6BAA4] = [0x8CC004] - [0x8CC000]`.
#[hook(addr = 0x84D250, conv = cdecl, name = "__init_g_b6baa4", expect = "D9 05 04 C0 8C 00")]
pub fn init_g_b6baa4() {
    let v = x(g::<f32>(0x8CC004).read()).fsub_m(g::<f32>(0x8CC000).read());
    g::<f32>(0xB6BAA4).write(v.to_f32());
}

/// `__init_g_b6baa8` (0x84D270): `[0xB6BAA8] = [0x8CC010] - [0x8CC00C]`.
#[hook(addr = 0x84D270, conv = cdecl, name = "__init_g_b6baa8", expect = "D9 05 10 C0 8C 00")]
pub fn init_g_b6baa8() {
    let v = x(g::<f32>(0x8CC010).read()).fsub_m(g::<f32>(0x8CC00C).read());
    g::<f32>(0xB6BAA8).write(v.to_f32());
}

/// `__init_g_b6baac` (0x84D290): `[0xB6BAAC] = [0x8CC018] - [0x8CC014]`.
#[hook(addr = 0x84D290, conv = cdecl, name = "__init_g_b6baac", expect = "D9 05 18 C0 8C 00")]
pub fn init_g_b6baac() {
    let v = x(g::<f32>(0x8CC018).read()).fsub_m(g::<f32>(0x8CC014).read());
    g::<f32>(0xB6BAAC).write(v.to_f32());
}

/// `__init_g_b6bab0` (0x84D2B0): `[0xB6BAB0] = [0x8CC024] - [0x8CC020]`.
#[hook(addr = 0x84D2B0, conv = cdecl, name = "__init_g_b6bab0", expect = "D9 05 24 C0 8C 00")]
pub fn init_g_b6bab0() {
    let v = x(g::<f32>(0x8CC024).read()).fsub_m(g::<f32>(0x8CC020).read());
    g::<f32>(0xB6BAB0).write(v.to_f32());
}

/// `__init_g_b6bab4` (0x84D2D0): `[0xB6BAB4] = [0x8CC02C] - [0x8CC028]`.
#[hook(addr = 0x84D2D0, conv = cdecl, name = "__init_g_b6bab4", expect = "D9 05 2C C0 8C 00")]
pub fn init_g_b6bab4() {
    let v = x(g::<f32>(0x8CC02C).read()).fsub_m(g::<f32>(0x8CC028).read());
    g::<f32>(0xB6BAB4).write(v.to_f32());
}

/// `__init_g_b6bab8` (0x84D2F0): `[0xB6BAB8] = [0x8CC034] - [0x8CC030]`.
#[hook(addr = 0x84D2F0, conv = cdecl, name = "__init_g_b6bab8", expect = "D9 05 34 C0 8C 00")]
pub fn init_g_b6bab8() {
    let v = x(g::<f32>(0x8CC034).read()).fsub_m(g::<f32>(0x8CC030).read());
    g::<f32>(0xB6BAB8).write(v.to_f32());
}

/// `__init_g_b6babc` (0x84D310): `[0xB6BABC] = [0x8CC03C] - [0x8CC038]`.
#[hook(addr = 0x84D310, conv = cdecl, name = "__init_g_b6babc", expect = "D9 05 3C C0 8C 00")]
pub fn init_g_b6babc() {
    let v = x(g::<f32>(0x8CC03C).read()).fsub_m(g::<f32>(0x8CC038).read());
    g::<f32>(0xB6BABC).write(v.to_f32());
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_AE_SOUND_MANAGER,
    HOOK_INIT_RAIN_SOUND_L,
    HOOK_INIT_RAIN_SOUND_R,
    HOOK_INIT_AE_AUDIO_HARDWARE,
    HOOK_INIT_AUDIO_ENGINE,
    HOOK_INIT_AE_SMOOTH_FADE_THREAD,
    HOOK_INIT_AE_STREAM_TRANSFORMER,
    HOOK_INIT_PED_SPEECH_SLOTS,
    HOOK_INIT_G_B6BA54,
    HOOK_INIT_G_B613B0,
    HOOK_INIT_G_B613BC,
    HOOK_INIT_G_B613C0,
    HOOK_INIT_G_B613CC,
    HOOK_INIT_G_B6BA20,
    HOOK_INIT_G_B6BA24,
    HOOK_INIT_G_B6BA28,
    HOOK_INIT_G_B6BA2C,
    HOOK_INIT_G_B6BA38,
    HOOK_INIT_G_B6BA3C,
    HOOK_INIT_G_B6BA40,
    HOOK_INIT_G_B6BA44,
    HOOK_INIT_G_B6BA50,
    HOOK_INIT_G_B6BA58,
    HOOK_INIT_G_B6BA5C,
    HOOK_INIT_G_B6BA64,
    HOOK_INIT_G_B6BA68,
    HOOK_INIT_G_B6BA6C,
    HOOK_INIT_G_B6BA70,
    HOOK_INIT_G_B6BA74,
    HOOK_INIT_G_B6BA78,
    HOOK_INIT_G_B6BA80,
    HOOK_INIT_G_B6BA84,
    HOOK_INIT_G_B6BA88,
    HOOK_INIT_G_B6BA90,
    HOOK_INIT_G_B6BA94,
    HOOK_INIT_G_B6BA9C,
    HOOK_INIT_G_B6BAA0,
    HOOK_INIT_G_B6BAA4,
    HOOK_INIT_G_B6BAA8,
    HOOK_INIT_G_B6BAAC,
    HOOK_INIT_G_B6BAB0,
    HOOK_INIT_G_B6BAB4,
    HOOK_INIT_G_B6BAB8,
    HOOK_INIT_G_B6BABC,
];
