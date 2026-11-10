# CAESound (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/AESound.cpp` (pin 20d8d2a). Fields: +0x24 m_CurrPos, +0x56 u16 m_Flags,
+0x5C i16 m_PlayTime, +0x5E i16 m_IsPhysicallyPlaying, +0x64 f32 m_ListenerSpeed, +0x68 i16 m_HasRequestedStopped, +0x70 i16
m_Length.

## 0x4EF2E0 CAESound::UpdatePlayTime  (gr_deviations = 2)
Original: `__thiscall void (this; i16 soundLength [esp+4], i16 loopStart [esp+8], i16 progress [esp+0xC])`, `ret 0xC`.
- Original bytes: `+0x70 = soundLength` (always, 0x4EF2EE). If `+0x5E != 0` return. If `+0x68 != 0`: `+0x5C = -1`, return.
  `t = _ftol((f32)progress * +0x64 + (f32)(i16)+0x5C)` - x87 `fild progress; fmul [+0x64]; fiadd playTime` then truncation of
  the whole sum (0x4EF313-0x4EF322); `+0x5C = (i16)t`. If `(i16)t < soundLength` (signed) return. If `loopStart == -1`:
  `+0x5C = -1`, return. Else `+0x5C = loopStart + (i32)(i16)t % (i32)soundLength` (`cdq; idiv`, 0x4EF340) - no zero check.
Rows AESound.cpp:148 / :149 (division-by-zero bugfix):
- Verdict: CONFIRMED - with soundLength 0 the original executes `idiv 0` (integer divide fault); gr skips the modulo when
  its bugfix switch is on. Class: keep-original semantics; the port cannot fault - record the ledger entry (a length of 0
  with t >= 0 crashes the original).
Found in passing (CONFIRMED, keep-original): gr computes `m_PlayTime += (int16)((float)progress * m_ListenerSpeed)` (truncates
the product, then adds); the original truncates `product + playTime` once. Results differ when the sum is negative and the
product has a fraction (e.g. playTime -5, product 2.5: original -2, gr -3).

## 0x4EF350 CAESound::GetRelativePosition (original convention)  (gr_deviations = 2)
Original: `__thiscall void (this; CVector* out [esp+4])`, `ret 4`. If `m_Flags & 1` (front-end): `*out = m_CurrPos` (3 dword
copies); else `CAEAudioEnvironment::GetPositionRelativeToCamera(out, &m_CurrPos)` 0x4D80B0 (cdecl, caller pops 8).
Row :154 (NOTSA simplified overload) - harmless wrapper. REFUTED.
Row :161 (original-convention overload): matches the bytes. REFUTED. Class: harmless.
