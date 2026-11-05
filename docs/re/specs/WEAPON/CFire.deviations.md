# CFire (WEAPON) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Fire.cpp` (pin 20d8d2a). CFire (0x28 bytes): +0 flags u8 (bit0 active, bit1 script,
bit2 makes noise, bit3 being extinguished, bit4 first generation), +2 script ref i16, +4 position, +0x10 entity on fire,
+0x14 entity that started it, +0x18 time to burn u32, +0x1C strength f32, +0x20 generations u8, +0x21 removal distance u8,
+0x24 FxSystem_c*. gFireManager = 0xB71F80 (60 fires).

## 0x538B30 CFire::Initialise  (gr_deviations = 1)
Row Fire.cpp:30 (original-comment "Originally m_nFlags = (m_nFlags & 0xF4) | 0x14").
- Original bytes: `__thiscall`, plain `ret`. Position (+4/+8/+0xC) = 0 via stack temps (0x538B33-0x538B5F);
  flags byte +0: `dl = (flags & 0xF4) | 0x14` (0x538B62-0x538B69), stored at 0x538B82; `+2 (i16) = 1`
  (0x538B6C); +0x18 (TimeToBurn), +0x10 (EntityOnFire), +0x14 (EntityStartedFire), +0x24 (FxSystem) = 0;
  +0x1C (Strength) = 1.0f (0x3F800000, 0x538B7B); +0x20 = 100 (0x64), +0x21 = 60 (0x3C) (0x538B87/0x538B8B).
  Mask 0xF4 clears bit0 (IsActive), bit1 (IsCreatedByScript), bit3 (IsBeingExtinguished), keeps bits 2 and 4-7;
  OR 0x14 sets bit2 (MakesNoise) and bit4 (IsFirstGeneration). Bits 5-7 are preserved.
- gr sets the five named 1-bit fields (false,false,true,false,true) and leaves the 3 padding bits alone: same byte.
- Verdict: REFUTED (same result; the comment documents the original, not a change).
- Class: harmless.

## 0x5393F0 CFire::Extinguish  (gr_deviations = 1)
Row Fire.cpp:225 (original-comment "Originally m_nFlags = (m_nFlags & 0xF6) | 0x10").
- Original bytes: `__thiscall`, plain `ret`. `if (!(flags & 1)) return` (0x5393F5). `+0x18 TimeToBurn = 0`
  (0x539405); `flags = (flags & 0xF6) | 0x10` (0x5393FC/0x539401/0x539408): clears IsActive (bit0) and
  IsBeingExtinguished (bit3), sets IsFirstGeneration (bit4), keeps bit1 (script), bit2 (noise), bits 5-7.
  If `+0x24 FxSystem` non-null: `FxSystem_c::Kill` 0x4AA3F0 (thiscall) then `+0x24 = 0` (0x53940C-0x539411; the
  pointer is cleared only when it was non-null). Then `e = +0x10 EntityOnFire`; if non-null:
  `t = e->m_nType (byte +0x36 & 7)`: t == 3 (ped) -> `*(e+0x730) = 0` (CPed::m_pFire); t == 2 (vehicle) ->
  `*(e+0x490) = 0` (CVehicle::m_pFire); then if `this->+0x10` still non-null `CEntity::CleanUpOldReference(&this->+0x10)`
  0x571A00 (thiscall on the entity, arg = address of the field), and `+0x10 = 0` (0x53943C-0x539448).
- gr: clears the same three bits individually (IsActive=false, IsBeingExtinguished=false, IsFirstGeneration=true);
  identical byte result. `DestroyFx()` and `CEntity::ClearReference` match the bytes.
- Verdict: REFUTED.
- Class: harmless.

## 0x53A570 CFire::ProcessFire  (gr_deviations = 7)
Original: `__thiscall void ()` (ESI = this in the body). CFire layout as in gr (flags +0 bit0 active, bit1 script; pos +4;
+0x10 entity on fire; +0x14 entity started fire; +0x18 time to burn u32; +0x1C strength f32; +0x20 generations u8;
+0x21 removal distance u8; +0x24 FxSystem). `t = (float)rand() * 3.0518509e-05f [0x858C7C]` below (raw rand, 1/32767).
Rows Fire.cpp:327 (original-comment "Original loop was backwards [not that it matters]") and :327 (pool-iteration).
- Original bytes (0x53A8C5-0x53A9FF): `if ((rand() & 0x1F) == 0)` walk the vehicle pool from slot size-1 DOWN to 0; per
  vehicle: skip if `veh->m_pFire (+0x490) != 0` or `veh->m_nModelIndex == 0x197` (407, fire truck); `|veh.pos - m_Position| < 2.0f`
  (3D sqrt, (z^2+y^2)+x^2); if `veh+0x594 == 10` (BMX): `FindPlayerPed(-1)->DoStuffToGoOnFire()`,
  `gFireManager.StartFire(FindPlayerPed(-1), m_EntityStartedFire, 0.8f, 1, 7000, 100)`,
  `veh->vtbl[0xB0/4](FindTyreNearestPoint(m_Position.x, m_Position.y) + 13, false)` (BurstTyre); else
  `gFireManager.StartFire(veh, m_EntityStartedFire, 0.8f, 1, 7000, 0)` (0x53A9E3: numGenerations 0).
- Verdict (both rows): CONFIRMED - the order is observable (each StartFire takes the next free CFire slot), so "not that it
  matters" is wrong. Class: keep-original (descending).
- Found in passing (CONFIRMED, keep-original): gr omits the `m_pFire == 0` and `model != 407` filters and passes 100 instead of
  0 generations for the non-BMX StartFire.
Rows Fire.cpp:342 (original-comment) and :342 (pool-iteration), objects.
- Original bytes (0x53AA05-0x53AAA9): `if ((rand() & 3) == 0)`: object pool from size-1 DOWN to 0; `|obj.pos - m_Position| < 3.0f`
  (3D) -> `obj->ObjectFireDamage(ms_fTimeStep * 8.0f, m_EntityStartedFire)` (0x5A1580).
- Verdict (both): CONFIRMED (ObjectFireDamage can start fires/explosions, so the order is observable). Class: keep-original.
Rows Fire.cpp:352 and :353 (rng-helper; creeping fire direction and distance).
- Original bytes (0x53AAAB-0x53AC1F): only if `(int8)m_NumGenerationsAllowed > 0` (signed byte test, values >= 128 skip) and
  `(rand() & 0x7F) == 0` and the count of active fires (bit0 over all 60 CFire slots of gFireManager 0xB71F80, stride 0x28)
  is `< 25`. Then `t1 = t` (first rand), `t2 = t` (second rand); `dir = ((t2 + t2) - 1.0f, (t1 + t1) - 1.0f, 0)` (the FIRST
  random value goes to y); `dir.Normalise()`; `d = t3 + 2.0f` (third rand); `pos = (dir.x*d + m_Position.x, dir.y*d + m_Position.y,
  dir.z*d + m_Position.z + 2.0f)`; `CCreepingFire::TryToStartFireAtCoors(pos, m_NumGenerationsAllowed - 1, 0, (flags >> 1) & 1, 10.0f)`.
- Verdict (both): CONFIRMED - besides the RNG formula, gr assigns the random values to x/y in the other order, does not
  normalise, omits the `+2.0f` on z and passes `m_NumGenerationsAllowed` instead of `m_NumGenerationsAllowed - 1` (and its
  boolean argument order is (false, IsScript()) where the original passes (0, script)). Class: keep-original.
Row Fire.cpp:373 (original-comment "R* way: m_fStrength - (float)(int)m_fStrength").
- Original bytes (0x53AD72-0x53ADD5): if `m_FxSystem`: `f = s - (float)_ftol2(s)` (fisub of the truncated int);
  `k = (float)(u32)(m_TimeToBurn - now) * 0.00028571428f [0x863A48]`; `v = (f < k) ? f : k`; `fx->SetConstTime(1, v)`.
- Verdict: REFUTED for the fraction (modf gives the same value for finite s). Class: harmless.
- Found in passing (CONFIRMED, gr LOGIC ERROR, keep-original): the second operand is the REMAINING burn time
  `(m_TimeToBurn - now) / 3500` (unsigned difference), not gr's `CTimer::GetTimeInMS() / 3500.0f`.
