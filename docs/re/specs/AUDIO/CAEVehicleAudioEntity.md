# CAEVehicleAudioEntity (AUDIO) - full specs (RE-SPEC-3, group U1)

Fields (gr AEVehicleAudioEntity.h): +0xA9 i8 m_State, +0xD8 f32 m_EventVolume, +0xDC u16 m_DummyEngineBank, +0xE0 i16 m_DummySlot.
tVehicleParams (vp): +0x10 CVehicle*, +0x14 cTransmission* (+0x58 f32 m_MaxVelocity), +0x1C f32 Speed. Statics:
s_DummyEngineSlots 0xB6B9A0 (10 entries x 4 bytes, +0 u16 bank), 0xB6BA6C (= 1.05f - 0.95f, _static-init-U1.md), 0xB6BA70
(= 1.1f - 0.4f), 0xB6BAC8 u8 (bicycle "was pedalling" last frame).
Callees used below: RequestBankSlot 0x4F4D10 (cdecl `i16 (u16 bank)`), AEAudioHardware.IsSoundBankLoaded 0x4D88C0
`(u16 bank, i16 slot)`, AudioLog10 0x4D9E50 (cdecl f32 -> ST0), GetBaseVolumeForBicycle 0x4F60B0 (thiscall `(f32)` -> ST0),
PlayBicycleSound 0x4F9710 (thiscall `(i16 st, i16 slot, i16 sfx, f32 volume, f32 speed)`, see the deviation note).

## 0x4FFDC0 CAEVehicleAudioEntity::ProcessDummyBicycle  (catalogue: specified/REA, port level 5)
- Convention: `__thiscall void (this = ECX; tVehicleParams* vp [esp+4])`, `ret 4`. Caller: ProcessVehicle 0x501E10. Frame:
  `sub esp,0xC` (locals L0 [E-0xC], L1 [E-8], L2 [E-4]); the vp argument slot [E+4] is reused as a float temporary.
- Behaviour:
  1. Dummy slot (0x4FFDC6): if `+0xE0 == -1`: `+0xE0 = RequestBankSlot(+0xDC)`; if still -1 return.
     If `!IsSoundBankLoaded(+0xDC, +0xE0)`: `k = +0xE0 - 7`; if `0 <= k < 10` and `s_DummyEngineSlots[k].bank == +0xDC` return
     (still loading); else `+0xE0 = RequestBankSlot(+0xDC)`, `+0xA9 = 0`, return.
  2. `veh = vp->+0x10`; `r = vp->Speed / vp->Transmission->m_MaxVelocity` (fdiv); `s = min(|r|, 1.0)` (compare with f64 1.0
     [0x85A310]; |r| by `fchs` when < 0; equal gives 1.0); stored f32 into [E+4]; if `!(s > 0.0f)` s = 0 (0x4FFEC4-0x4FFED1).
  3. `w = (f32)(i16)(u8)veh->+0x804` (stored L0). `vol1 = AudioLog10(w * 0.25f [0x858C84] * s) * 20.0f [0x858BA4] + m_EventVolume
     + (-8.0f) [0x8CBEBC]` (stored L2; operand order: `fld w; fmul 0.25; fmul s`; then `fmul 20; fadd +0xD8; fadd -8`).
  4. `f = (w * [0xB6BA6C] * 0.125f [0x858C48]) + [0xB6BA6C] * s + 1.05f [0x8CBD5C]` (x87 order: `fld w; fmul K; fmul 0.125; fld K;
     fmul s; faddp; fadd 1.05`); `lean = |sin(veh->+0x648)|` (fsin, fchs if < 0); `freq1 = (lean * 0.2f [0x8CBD60] + 1.0f) * f`
     (stored L1).
  5. If `veh->+0x834 != 0` (pedalling): `L0 = GetBaseVolumeForBicycle(s)`, `ped = 1`; else `L0 = 0.0f`, `ped = 0`.
     `vol2 = AudioLog10(L0) * 20.0f + m_EventVolume + (-15.0f) [0x8CBEC0]` (stored L0); `freq2 = [0xB6BA70] * s + 0.4f [0x8CBD64]`
     (stored into [E+4]).
  6. `PlayBicycleSound(1, +0xE0, 0, vol1, freq1)`; `PlayBicycleSound(2, +0xE0, 1, vol2, freq2)`.
  7. If `!ped && *(u8*)0xB6BAC8 != 0`: `PlayBicycleSound(3, +0xE0, 1, vol1, 1.0f)` (freewheel click when pedalling stops).
     Finally `*(u8*)0xB6BAC8 = ped` (also when step 7 is skipped). The early-return paths (step 1) do not touch 0xB6BAC8.
- Edge cases: `w = 0` or `s = 0` gives AudioLog10(0) (whatever 0x4D9E50 returns for 0). 0xB6BAC8 is a single global shared by
  all dummy bicycles (original behaviour).
- gta-reversed: `plugin::CallMethod` only. Open questions: none.

## 0x500040 CAEVehicleAudioEntity::ProcessPlayerBicycle  (catalogue: specified/REA, port level 5)
- Convention: `__thiscall void (this = ECX; tVehicleParams* vp [esp+4])`, `ret 4`. Caller: ProcessVehicle 0x501E10. Statics:
  0xB6BACC f32 (bump timer), 0xB6BAD0 f32 (previous `veh->+0x820`), 0xB6BAC9 u8 (player "was pedalling").
- Behaviour:
  1. If `!IsSoundBankLoaded(+0xDC, +0xE0)` return (no slot request, unlike the dummy version).
  2. Bump timer (0x500075-0x5000E8), `veh = vp->+0x10`: if `veh->+0x818 > 5.0f` [0x858C80] (strict) and `[0xB6BAD0] * veh->+0x820 <
     0.0f` (sign change, strict) and the timer at 0xB6BACC is exactly 0.0f: store 6.0f into it. Then `t = [0xB6BACC] - 0.7f [0x858CB0]`;
     `[0xB6BACC] = (t < 0.0f) ? 0.0f : t`. The timer is not used anywhere else in this function.
  3. Steps 2-6 of ProcessDummyBicycle 0x4FFDC0 identically (same constants, same x87 order, same PlayBicycleSound calls st 1 and
     2 with sfx 0 and 1), with the vp argument slot again reused for `s` and `freq2`.
  4. If `!ped && *(u8*)0xB6BAC9 != 0`: `PlayBicycleSound(3, +0xE0, 1, vol1, 1.0f)`.
  5. `*(u8*)0xB6BAC9 = ped`; `*(f32*)0xB6BAD0 = veh->+0x820` (dword copy, 0x50029F-0x5002B0).
- gta-reversed: `plugin::CallMethod` only. Open questions: meaning of `veh->+0x818` / `+0x820` (bike fields; read only) and of
  the unused 0xB6BACC timer (no reader found in this function).

## 0x500F50 CAEVehicleAudioEntity::ProcessDummyHovercraft  (catalogue: specified/REA, port level 5)
- Convention: `__thiscall void (this = ECX; tVehicleParams* vp [esp+4])`, `ret 4`. Caller: ProcessSpecialVehicle 0x501AB0. Frame:
  local L at [E-4] (first the vehicle pointer, later the volume), the vp slot [E+4] is reused for `R` and then the frequency.
- Callees: CPad::GetPad 0x53FB70 (cdecl `(0)`), CPad::GetAccelerate 0x5403F0, CPad::GetBrake 0x540080 (thiscall -> i16 in AX),
  CancelVehicleEngineSound 0x4F55C0 (thiscall `(i16 st)`), UpdateGenericVehicleSound 0x4FAD40 (thiscall `(i16 st, i16 slot, u16 bank,
  i16 sfx, f32 speed, f32 volume, f32 rollOff)`).
- Fields: veh +0x36 status bits, +0x428 flags (0x10 = engine on), +0x988 f32, +0x9C4 f32; this +0xA8 m_IsWreckedVehicle, +0x22C f32
  (current dummy frequency). Statics 0xB6BA90 (= -6.0f - (-12.0f)), 0xB6BA94 (= 1.15f - 0.8f).
- Behaviour:
  1. `pad = CPad::GetPad(0)` (always, first). Dummy-slot handling exactly as ProcessDummyBicycle step 1 (request slot, loading
     check against s_DummyEngineSlots, re-request + `+0xA9 = 0`).
  2. `veh = vp->+0x10`; if `!(veh->+0x428 & 0x10)` or `+0xA8 != 0`: `CancelVehicleEngineSound(i)` for i = 0..11, return.
  3. `x = veh->+0x988 * 0.6666667f [0x862D68]`; `R = min(|x|, 1.0f)` (`1.0 < |x|` -> 1.0; NaN -> 0), stored into [E+4].
  4. Throttle offset `v0`: if `(veh->+0x36 & 0xF8) == 0` (player-controlled status): if `vp->Speed < 0.001f` [0x858CDC] (strict):
     `a = pad->GetAccelerate() - pad->GetBrake()` (i16 difference in DI), `b = 0`; else `b = pad->GetBrake()`, `a =
     pad->GetAccelerate()`. Otherwise (not player): `a = b = the raw dword of R` (original bug: the float bits of R are used as
     the accelerate/brake values, 0x501115-0x501119). Then `v0 = ((i16)a > 0) ? 0.1f [0x858B1C] : R`; if `(i16)b > 0` `v0 = -0.05f
     [0x85901C]`.
  5. `c = clamp(veh->+0x9C4 * 2.9411764f [0x862D18], 0, 1)` (`1.0 < y` -> 1; `!(0 <= y)` -> 0); `vol = [0xB6BA90] * c + (-12.0f)
     [0x8CBFAC]` (stored L); `f = (([0xB6BA94] * c) + 0.15f [0x8CBFBC] * R) + 0.8f [0x8CBFB4] + v0` (that x87 order; stored f32).
  6. `cur = +0x22C`; if `cur < 0.0f` `cur = f` (dword copy into +0x22C). If `f < cur`: `f = max(cur - 0.0053333323f [0x862D04], f)`;
     else `f = min(cur + 0.0053333323f, f)` (equality keeps f). `+0x22C = f`.
  7. `UpdateGenericVehicleSound(1, +0xE0, +0xDC, 0, f, vol, 3.5f)`.
- Edge cases: for AI hovercraft (status != 0) the accelerate/brake decisions depend on the bit pattern of R (positive R gives a
  positive low word unless the low 16 bits are 0 or >= 0x8000) - reproduce bit-exactly.
- gta-reversed: `plugin::CallMethod` only. Open questions: meaning of veh +0x988 / +0x9C4 (hovercraft fields, read only).
