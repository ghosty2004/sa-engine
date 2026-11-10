# CAEAudioUtility (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/AEAudioUtility.h/.cpp` (pin 20d8d2a). `_rand` = 0x821B1E, `_ftol` = 0x821B40.

## 0x4D9C10 CAEAudioUtility::GetRandomNumberInRange<int32>  (gr_deviations = 2)
Original: `__cdecl i32 (i32 min [esp+4], i32 max [esp+8])`, plain `ret`.
- Original bytes: `r = _rand() & 0xFFFF` (0x4D9C12/0x4D9C1B); x87 `fild r; fmul 3.0517578125e-05f [0x858B14] (= 1/32768);
  fimul (max - min + 1)` (i32, 0x4D9C2C-0x4D9C39); `_ftol` (truncation); `+ min`. Result = min + trunc(r/32768 * (max-min+1)).
Row AEAudioUtility.h:19 (`CGeneral::GetRandomNumberInRange<T>(min, max + 1)`):
- Verdict: CONFIRMED - gr routes through its own CGeneral formula (docs/re/cat14-gr-deviations.md finding 1), not this one.
  Class: keep-original (port the formula above exactly; same RNG call count: one `_rand`).
Row AEAudioUtility.h:25 (float overload): belongs to 0x4D9C50, not to this function. For reference, 0x4D9C50 is
`__cdecl f32 (f32 min, f32 max)`: `fild _rand(); fmul 3.0518509e-05f [0x858C7C] (= 1/32767); fld max; fsub min; fmulp;
fadd min` -> ST0 (no mask on rand). Verdict for this row: CONFIRMED (gr's CGeneral float formula is not this one),
keep-original; it should be re-checked when 0x4D9C50 is ported.

## 0x4D9E80 CAEAudioUtility::GetCurrentTimeInMS  (gr_deviations = 2)
Original: `__cdecl u64 ()` (EDX:EAX from `_ftol`, tail `jmp 0x821B40`).
- Original bytes: `QueryPerformanceCounter(&q)` (IAT 0x8580C8); x87 `fild q (i64); fild qword [0xB610F0] (performance
  frequency, i64); fdivp` -> `q / freq` (seconds, floating); `fmul 1000.0f [0x858C4C]`; then subtracts `startTimeMs`
  (u64 at 0xB610F8) converted to floating via the MSVC u64->x87 sequence (low dword + (high & 0x7FFFFFFF) loaded as i64,
  plus 2^63 if the top bit is set, 0x4D9EA3-0x4D9ED6); `fsubp`; `_ftol` (truncation toward zero, 64-bit result).
  Under the game's 24-bit x87 precision control the division and products round to a 24-bit significand (when called on a
  thread with that control word), so millisecond resolution degrades as the counter grows.
Row AEAudioUtility.cpp:93 (NOTSA std::chrono): Verdict: CONFIRMED (different clock source; the value is still "ms since
StaticInitialise 0x5B97F0"). Class: keep-original (port uses QPC + the x87 formula above).
Row AEAudioUtility.cpp:102 ("original code" shown in `#if 0`): the commented reconstruction uses integer division
`counter / freq * 1000`; the bytes use floating division before the `* 1000` (0x4D9EA7). Verdict: CONFIRMED (gr's comment is
wrong). Class: keep-original.
