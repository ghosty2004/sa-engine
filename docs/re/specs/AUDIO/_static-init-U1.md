# AUDIO static initialisers and atexit destructors (RE-SPEC-3, group U1)

All `__init_*` are `__cdecl void ()` run by the CRT dynamic-initialiser table; `__dtor_*` are `__cdecl void ()` registered with
`_atexit` 0x821D1E (`push dtor; call 0x821D1E; pop ecx`). Float initialisers compute `a - b` (or `a * b`) on the x87 at
CRT-init time and store an f32 (`fld a; fsub b; fstp dst`). The x87 precision control at that time is the CRT default
(53-bit), so the difference is computed exactly enough and rounded once by the f32 store. "(.bss, 0 in the image)" marks a
source operand that is zero unless an earlier initialiser wrote it (none of the U1 initialisers do).

## Object constructors / destructors
| addr | name | behaviour |
|---|---|---|
| 0x84C270 | __init_AEAudioHardware | `CAEAudioHardware::Constructor(&AEAudioHardware 0xB5F8B8)` 0x4D83E0; `_atexit(__dtor_AEAudioHardware 0x8562F0)`. |
| 0x84C290 | __init_AESmoothFadeThread | ctor 0x4EE9A0 on 0xB608D0 (`+0 = -1, +4 = 0, +0x808 = 0 (u8), +0x809 = 0 (u8), +0x80A = 0 (u16), +0x80C = 0 (u16), +0x810 = 0, +0x818 = 0`, returns this); `_atexit(0x856300)`. |
| 0x84C500 | __init_AEStreamTransformer | `CAEStreamTransformer::Initialise(&AEStreamTransformer 0xB612D8)` 0x4F1750 (called as the ctor); `_atexit(0x856320)`. |
| 0x84C9A0 | __init_CAEPedSpeechAudioEntity__s_PedSpeechSlots | `eh_vector_constructor_iterator(0xB61C38, 0x1C, 6, ctor 0x4E3330, dtor 0x4E3340)` 0x821D30 (stdcall). The element ctor 0x4E3330 is `mov eax,ecx; ret` (no init); dtor 0x4E3340 is `ret`. `_atexit(0x856330)`. |
| 0x84CD20 | __init_AESoundManager | `CAESoundManager::CAESoundManager(&AESoundManager 0xB62CB0)` 0x4EFD30; `_atexit(0x856360)`. |
| 0x84D4D0 | __init_m_sRainSoundL | CAETwinLoopSoundEntity default ctor 0x4F63B0 on 0xB6BB18 (`+0x10 = 0, +0 = 0x85F438, +0x88 = 0 (u16), +0xA0 = 0, +0xA4 = 0`); `_atexit(0x856380)`. |
| 0x84D4F0 | __init_m_sRainSoundR | same ctor 0x4F63B0 on 0xB6BBC0; `_atexit(0x856390)`. |
| 0x84D5D0 | __init_AudioEngine | `CAudioEngine::Constructor(&AudioEngine 0xB6BC90)` 0x507670; `_atexit(0x8563A0)`. |
| 0x8562F0 | __dtor_AEAudioHardware | tail `CAEAudioHardware::Destructor(0xB5F8B8)` 0x4D83A0. |
| 0x856300 | __dtor_AESmoothFadeThread | `ecx = 0xB608D0; jmp 0x4EE9E0` (0x4EE9E0 is a bare `ret`) - no effect. |
| 0x856320 | __dtor_AEStreamTransformer | bare `ret` - no effect. |
| 0x856330 | __dtor_CAEPedSpeechAudioEntity__s_PedSpeechSlots | `eh_vector_destructor_iterator(0xB61C38, 0x1C, 6, 0x4E3340)` 0x821E02 (element dtor is `ret`) - no effect. |
| 0x856360 | __dtor_AESoundManager | `ecx = 0xB62CB0; jmp 0x4EFD60` = CAESoundManager dtor body: `eh_vector_destructor_iterator(this + 4, 0x74, 0x12C, ~CAESound 0x4EF660)`. |
| 0x856380 | __dtor_m_sRainSoundL | tail `~CAETwinLoopSoundEntity(0xB6BB18)` 0x4F2B80. |
| 0x856390 | __dtor_m_sRainSoundR | tail `~CAETwinLoopSoundEntity(0xB6BBC0)` 0x4F2B80. |
| 0x8563A0 | __dtor_AudioEngine | tail `CAudioEngine::Destructor(0xB6BC90)` 0x506CD0. |

## Float constant initialisers (`dst = a - b` unless stated)
| addr | dst | a | b | result (nominal) |
|---|---|---|---|---|
| 0x84C7E0 | 0xB613B0 | 0x8AE60C = 0.93 | 0x8AE608 = 1.0 | -0.07 (ped audio range) |
| 0x84C840 | 0xB613BC | 0x8AE65C = 1.2 | 0x8AE658 = 0.8 | 0.4 (in-vehicle shirt-flap speed range, used by 0x4E304B) |
| 0x84C860 | 0xB613C0 | 0x8AE66C = 1.6 | 0x8AE668 = 0.8 | 0.8 (on-foot shirt-flap speed range, 0x4E322E) |
| 0x84C8C0 | 0xB613CC | 0x858624 = 1.0 | 0x8AE69C = 0.5 | 0.5 (divisor at 0x4E30ED) |
| 0x84CE30 | 0xB6BA20 | 0x8CBBFC = 1.2 | 0x8CBBF8 = 0.85 | 0.35 |
| 0x84CE50 | 0xB6BA24 | 0xB6B9CC (.bss, 0 in the image) | 0x8CBC00 = -3.0 | 3.0 |
| 0x84CE70 | 0xB6BA28 | 0x8CBC10 = 1.5 | 0x8CBC0C = 0.9 | 0.6 |
| 0x84CE90 | 0xB6BA2C | 0xB6B9D0 (.bss, 0) | 0x8CBC14 = -4.5 | 4.5 |
| 0x84CEF0 | 0xB6BA38 | `0xB6BA34 (.bss, 0) * 0.2f [0x858CC4]` (fmul) | - | 0.0 |
| 0x84CF10 | 0xB6BA3C | dword copy of 0x8CBC68 = -2.0 (`mov eax; mov`) | - | -2.0 |
| 0x84CF20 | 0xB6BA40 | 0x8CBC74 = 1.5 | 0x8CBC70 = 0.85 | 0.65 |
| 0x84CF40 | 0xB6BA44 | 0x8CBC7C = -1.5 | 0x8CBC78 = -3.0 | 1.5 |
| 0x84CFA0 | 0xB6BA50 | `0xB6BA48 (.bss, 0) * 0.006666667f [0x862D6C]` | - | 0.0 |
| 0x84CFC0 | 0xB6BA54 | `0xB6BA4C (.bss, 0) / (f32)(i16)[0x8CBC8C]` (i16 = 150; `fild; fdivr`) | - | 0.0 |
| 0x84CFE0 | 0xB6BA58 | dword copy of 0x8CBC6C = 1.0 | - | 1.0 |
| 0x84CFF0 | 0xB6BA5C | 0x8CBC98 = 1.25 | 0x8CBC94 = 0.75 | 0.5 |
| 0x84D030 | 0xB6BA64 | 0xB6B9DC (.bss, 0) | 0x8CBCA8 = -4.0 | 4.0 |
| 0x84D050 | 0xB6BA68 | 0x8CBCB4 = 0.4 | 0x8CBCB0 = 0.2 | 0.2 |
| 0x84D070 | 0xB6BA6C | 0x8CBD5C = 1.05 | 0x8CBD58 = 0.95 | 0.1 |
| 0x84D090 | 0xB6BA70 | 0x8CBD68 = 1.1 | 0x8CBD64 = 0.4 | 0.7 |
| 0x84D0B0 | 0xB6BA74 | 0x8CBED0 = 1.3 | 0x858CC8 = 0.6 | 0.7 |
| 0x84D0D0 | 0xB6BA78 | 0x8CBF04 = 1.2 | 0x8CBF08 = 0.8 | 0.4 |
| 0x84D110 | 0xB6BA80 | 0x8CBF20 = 1.1 | 0x8CBF24 = 0.9 | 0.2 |
| 0x84D130 | 0xB6BA84 | 0x8CBF28 = 1.2 | 0x8CBF2C = 0.8 | 0.4 |
| 0x84D150 | 0xB6BA88 | 0x8CBF48 = 1.0 | 0x8CBF44 = 0.68 | 0.32 |
| 0x84D1B0 | 0xB6BA90 | 0x8CBFB0 = -6.0 | 0x8CBFAC = -12.0 | 6.0 |
| 0x84D1D0 | 0xB6BA94 | 0x8CBFB8 = 1.15 | 0x8CBFB4 = 0.8 | 0.35 |
| 0x84D210 | 0xB6BA9C | 0x8CBFD4 = 1.2 | 0x8CBFD0 = 0.85 | 0.35 |
| 0x84D230 | 0xB6BAA0 | 0x8CBFFC = -21.0 | 0x8CBFF8 = -24.0 | 3.0 |
| 0x84D250 | 0xB6BAA4 | 0x8CC004 = 1.1 | 0x8CC000 = 0.9 | 0.2 |
| 0x84D270 | 0xB6BAA8 | 0x8CC010 = -18.0 | 0x8CC00C = -24.0 | 6.0 |
| 0x84D290 | 0xB6BAAC | 0x8CC018 = 1.2 | 0x8CC014 = 0.9 | 0.3 |
| 0x84D2B0 | 0xB6BAB0 | 0x8CC024 = 2.0 | 0x8CC020 = -6.0 | 8.0 |
| 0x84D2D0 | 0xB6BAB4 | 0x8CC02C = 0.84 | 0x8CC028 = 0.75 | 0.09 |
| 0x84D2F0 | 0xB6BAB8 | 0x8CC034 = -9.0 | 0x8CC030 = -36.0 | 27.0 |
| 0x84D310 | 0xB6BABC | 0x8CC03C = 1.0 | 0x8CC038 = 0.8 | 0.2 |
Port note: compute each value from the f32 operands (exact f32 subtraction in double, then round to f32) - do not
hard-code the nominal decimal. The 0xB6BA20..0xB6BABC block are CAEVehicleAudioEntity "range" statics (gr
AEVehicleAudioEntity.h tables, e.g. 0xB6B9CC VolumeMax = 0.0f). gta-reversed: these are static-storage initialisers of the
gr headers (constexpr in gr); no deviation recorded.
