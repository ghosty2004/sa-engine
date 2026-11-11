# FxInterpInfoU255_c (CALLED-ONLY) — RE-SPEC-REST

Layout as FxInterpInfo_c (see CALLED-ONLY/FxInterpInfoFloat_c.md): looped +4 (u8), nk +5 (i8), count +6 (read as i8 low byte),
times u16* +8, keys +0xC (pointers to **u16** arrays; gr Load 0x5C18F0 stores `value * 256`). `_ftol` 0x821B40.

## 0x4A8800 FxInterpInfoU255_c::GetVal(float* out, float time)  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this → ESI), `out` [esp+4], f32 `time` [esp+8], `ret 8`, void. Callers (6): FxInfoEmLife_c 0x4A4FE0,
  FxInfoColour_c 0x4A5F30, FxInfoTrail_c 0x4A62B0, FxInfoColourRange_c 0x4A66C0, FxInfoColourBright_c 0x4A6860, FxInfoSmoke_c 0x4A6970
  (`GetValue`).
- Scale: `S = [0x859AA0]=0.00390625f` (1/256) for both times and keys. Key decode = `(float)(u16)key * S` (`movzx` + `fild dword`).
- Behaviour (same control flow as FxInterpInfoFloat_c::GetVal 0x4A8470, step numbers as there):
  1. nk == 1 (0x4A8808): for i < (i8)this[+6]: `out[i] = (u16)m_Keys[i][0] * S` (0x4A8820-0x4A8838); return.
  2. Looped (0x4A884A): `P = (u16)times[nk-1] * S`; `n = _ftol(time / P)`; `time = (f32)(time - n * P)` (0x4A8851-0x4A8883).
  3. Search k = 1.. while k < nk for the first `time < (u16)times[k] * S` (`fcomp; test ah, 5; jnp`, 0x4A88A0-0x4A88C6).
  4. Not found: `out[i] = (u16)m_Keys[i][nk-1] * S` (0x4A88C8-0x4A88FD).
  5. Found (0x4A8906): `t0 = (u16)times[k-1] * S`; `frac = (time - t0) / (tk - t0)` stored f32 in a local ([esp+0xC], 0x4A892E).
     Per i (0x4A8940-0x4A8982): `a = (u16)keys[i][k-1] * S`; `b = (u16)keys[i][k] * S`; `out[i] = ((b - a) * frac) + a` (`fsub st1;
     fmul frac; fadd st1`), a popped after the store.
- gta-reversed: `plugin::CallMethod<0x4A8800>` only (FxInterpInfoU255.cpp).
