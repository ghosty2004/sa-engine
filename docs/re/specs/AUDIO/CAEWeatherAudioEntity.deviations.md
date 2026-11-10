# CAEWeatherAudioEntity (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Entities/AEWeatherAudioEntity.cpp` (pin 20d8d2a). Statics: m_sfRainVolume f32
0x8CC30C (initial -100.0), sfWindOffset 0xB6BAFC, sfWindFreq 0xB6BAF8, sfOldFreqLeft 0x8CC2C4 (initial 1.0), m_sRainSoundL
0xB6BB18 / m_sRainSoundR 0xB6BBC0 (their m_IsInUse u16 at 0xB6BBA0 / 0xB6BC48).

## 0x5052F0 CAEWeatherAudioEntity::Service  (gr_deviations = 1)
Row :274 (`m_sfRainVolume = notsa::step_to(m_sfRainVolume, targetVolume, 0.5f)`):
- Original bytes (target T in ST0, 0x505634-0x5056E9): if `T < rv`: `n = rv - 0.5f`; `rv = (n > T) ? n : T`. Else if `T > rv`:
  `n = rv + 0.5f`; `rv = (n < T) ? n : T`. Equal or unordered: no store. Then if `T == -100.0f` (exact, `test ah,0x44`) and
  `rv <= -50.0f`: `rv = -100.0f`, StopSoundAndForget on each rain twin-loop whose m_IsInUse != 0 (0x505682-0x5056B3).
- Verdict: REFUTED (equivalent to step_to except for NaN). Class: harmless.

## 0x505A00 CAEWeatherAudioEntity::UpdateParameters  (gr_deviations = 4)
Original: `__thiscall void (this; CAESound* sound [esp+4], i16 curPlayPos [esp+8])`, `ret 8`. If `curPlayPos <= 0` return
(0x505A6A). Dispatch on `sound->m_Event (+0xC)`: 1 -> thunder block (0x505A85); 4 or 5 -> wind block (0x506527); 3 -> city
noise (0x505BE2); anything else -> return (0x5067F3).
Row :224 (wind volume `step_to(vol, sfWindOffset - 33.f, 0.6f)`): target `= -33.0f [0x8CC2A4] + sfWindOffset`; step up by
0.6f [0x8CC2A8] / down by 0.6f [0x8CC2AC] with min/max against the target (0x5066AA-0x5066F6). Verdict: REFUTED, harmless.
Rows :227 / :228 (wind frequency):
- Original bytes (0x5066FD-0x50679B): if `old < windFreq`: `n = old + 0.1f [0x8CC2B8]`; `old = (n < windFreq) ? n : windFreq`.
  Else if `old > windFreq`: `n = old - 0.1f [0x8CC2BC]`; `old = (n > 1.0f) ? n : 1.0f` (floored at 1.0, NOT at windFreq).
  Equal/NaN: unchanged. Then `sound->m_Speed (+0x1C) = old`.
- Verdict: CONFIRMED - gr's condition is inverted (`old > windFreq ? step_up_to : ...`) and its step-down also clamps to
  windFreq (`max(1, step_down_to(old, wf, 0.1))`); the original steps up when below, and steps down without the wf floor.
  Class: keep-original (gr logic error).
Row :236 (`default: NOTSA_UNREACHABLE()`): CONFIRMED - events other than 1, 3, 4, 5 return silently (0x505BE5). Class: keep-original.
