# FxInterpInfoFloat_c (CALLED-ONLY) — RE-SPEC-REST

Layout (gr `Fx/Blueprints/Interp/FxInterpInfo.h`, confirmed by the accesses): vtable +0x0; `m_bLooped` u8 +0x4; `m_nNumKeys` i8 +0x5;
`m_nCount` +0x6 (gr: int16, but every GetVal reads only the LOW BYTE, sign-extended: `movsx edx, byte ptr [esi+6]`);
`m_pTimes` u16* +0x8 (key times in 1/256 s units); `m_Keys` +0xC (array of `m_nCount` pointers to per-parameter key arrays).
Helper `_ftol` 0x821B40 = MSVC CRT float→int64 truncation toward zero (pops ST0, result EAX).

## 0x4A8470 FxInterpInfoFloat_c::GetVal(float* out, float time)  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this → ESI), `float* out` [esp+4], f32 `time` [esp+8]; `ret 8`; void. The `time` argument slot is reused as
  scratch (holds the wrapped time, later the blend fraction). Callers (8): FxInfoEmRate_c 0x4A4B60, FxInfoEmAngle_c 0x4A4F20,
  FxInfoEmPos_c 0x4A50A0, FxInfoEmWeather_c 0x4A5170, FxInfoEmRotation_c 0x4A5240, FxInfoRotSpeed_c 0x4A5D40, ... (all `GetValue`).
  Keys are f32 (`m_Keys[i][k]` is a float array).
- Behaviour:
  1. `nk = (i8)this[+5]` (0x4A8475). If nk == 1 (0x4A8478): for i = 0 while i < (i8)this[+6] (tested before the first iteration,
     `jle` on the byte, 0x4A8482): `out[i] = m_Keys[i][0]` as a raw dword copy (0x4A8490-0x4A84A2); return.
  2. If `m_bLooped` (+4) != 0 (0x4A84AA): `P = (float)(u16)m_pTimes[nk-1] * [0x859AA0]=0.00390625f` (fild of the zero-extended
     u16, 0x4A84B7-0x4A84C4); `n = _ftol(time / P)` (0x4A84CA-0x4A84D0); `time = (f32)(time - (float)n * P)` (`fild n; fmul st1;
     fsubr time; fstp time`, 0x4A84D9-0x4A84E3). P = 0 gives time/0 = ±inf or NaN → `_ftol` returns 0x80000000 (integer
     indefinite), and time becomes NaN/inf; no guard.
  3. Search (0x4A84E9-0x4A8526): for k = 1 while k < nk: `tk = (float)(u16)m_pTimes[k] * 0.00390625f`; compare `fld time; fcomp tk;
     test ah, 5; jnp found` — breaks at the first k with **time < tk** (only C0 set). time >= tk or NaN → next k.
  4. Not found (time past the last key, or nk <= 1): for i < (i8)this[+6]: `out[i] = m_Keys[i][nk-1]` (raw dword, 0x4A8528-0x4A854D).
     Note nk <= 0 here reads m_Keys[i][-1] (out of bounds; only with corrupt data).
  5. Found at k (0x4A8556): `t0 = (float)(u16)m_pTimes[k-1] * 0.00390625f`; `frac = (time - t0) / (tk - t0)` (x87: `fld time; fsub st1`
     then `fxch st2; fsub st1; fdivp st2`), stored as f32 into the time slot (0x4A857E). Then for i < (i8)this[+6] (0x4A8590-0x4A85B1):
     `out[i] = ((m_Keys[i][k] - m_Keys[i][k-1]) * frac) + m_Keys[i][k-1]` (`fld [k]; fsub [k-1]; fmul frac; fadd [k-1]; fstp`).
- Edge cases: with time below the first key time (k = 1 found with time < t0 possible when m_pTimes[0] > 0) frac is negative
  → linear extrapolation; equal times[k-1] == times[k] cannot be picked (time < tk and time >= t0 are needed only for k > 1, so a
  zero span can still occur at k = 1 when time < t0 == tk → division by zero → ±inf/NaN). The original has no guards.
- gta-reversed: `plugin::CallMethod<0x4A8470>` only (FxInterpInfoFloat.cpp).
