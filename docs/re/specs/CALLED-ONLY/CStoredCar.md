# CStoredCar (CALLED-ONLY) — RE-SPEC-REST

Layout (gr `StoredCar.h`, size 0x40): +0x0 CVector pos, +0xC u32 handling flags, +0x10 u8 storedCarFlags, +0x11 pad, +0x12 u16 model,
+0x14 i16 mods[15], +0x32..+0x35 colours 1-4, +0x36 radio station, +0x37/+0x38 comps, +0x39 bomb type, +0x3A paintjob, +0x3B nitro,
+0x3C..+0x3E packed forward x/y/z (u8), +0x3F pad.

## 0x449760 CStoredCar::StoreCar(CVehicle* veh)  (catalogue: specified/REA, port level 2)
- `__thiscall` (ECX = this → EDI), veh [esp+4] (ESI), `ret 4`. Callers: CGarage::StoreAndRemoveCarsForThisHideOut 0x449900, 0x449A50,
  CGarages::StoreCarInNearestImpoundingGarage 0x44A3C0. Callees: `_ftol` 0x821B40 (x3), CVehicle::GetRemapIndex 0x6D0B70.
- Behaviour in store order:
  1. `model (+0x12) = veh.m_nModelIndex (+0x22, u16)`.
  2. `pos.x/y/z` = `veh.m_matrix (+0x14) ? matrix.pos (+0x30) : veh.m_placement.pos (+0x4)`, the NULL test repeated per component.
  3. `packedForward x (+0x3C) / y (+0x3D) / z (+0x3E) = (u8)_ftol(matrix.forward.x/y/z (+0x10/+0x14/+0x18) * [0x858628]=100.0f)` —
     the matrix pointer is used here WITHOUT a NULL check (crash if NULL). Negative values truncate toward zero, then the low byte
     is stored (e.g. -0.5 → -50 → 0xCE).
  4. Colours: +0x32..+0x35 ← veh+0x434..+0x437; radio +0x36 ← veh+0x1D2; handling flags +0xC ← veh+0x38C (u32); comps +0x37/+0x38 ←
     veh+0x438/+0x439.
  5. Flags (u16 write at +0x10 first clears flags AND the pad byte): `= 1` if physical flags (veh+0x40) & 0x40000 (bBulletProof);
     `|= 2` if & 0x80000 (bFireProof); `|= 4` if & 0x800000 (bExplosionProof); `|= 8` if & 0x100000 (bCollisionProof); `|= 0x10` if
     & 0x200000 (bMeleeProof, a 16-bit OR); `|= 0x20` if byte veh+0x42E & 0x10; `|= 0x40` if handling (veh+0x38C) & 0x20000; `|= 0x80`
     if handling & 0x80000 (0x449832-0x4498A7).
  6. If `veh.m_nVehicleSubType (+0x590)` is 0 or 9 (`test; je` / `cmp 9; jne`): `bombType (+0x39) = veh+0x4A8 & 7`; otherwise +0x39 is
     left unchanged (stale value from the previous use of this slot — keep).
  7. `mods[0..14] (+0x14) = veh+0x43A[0..14]` (u16 copy loop, 15 iterations).
  8. `paintjob (+0x3A) = (u8)veh->GetRemapIndex()`; `nitro (+0x3B) = veh+0x48A`.
- gta-reversed: `plugin::CallMethod` only.
