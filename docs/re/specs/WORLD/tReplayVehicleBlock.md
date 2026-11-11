# tReplayVehicleBlock (WORLD) - RE-SPEC-FINAL group A

## 0x45B730 tReplayVehicleBlock::MakeVehicleUpdateData  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (tReplayVehicleBlock* this, CVehicle* veh [esp+4], int32 poolIdx [esp+8])`, `ret 8`. The original fills an
  existing block in place (gr declares a static function returning a new zero-initialised block - see gta-reversed below).
- Callees: CCompressedMatrixNotAligned::CompressFromFullMatrix 0x59BAD0 (thiscall on this+4, (CMatrix*)); _ftol 0x821B40 (truncation to int32;
  every byte field below stores the LOW 8 BITS of that int32); CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (-1, false)).
- Globals: 1000.0f [0x858C4C]=1000.0f; 0.25f [0x858C84]=0.25f; 100.0f [0x858628]=100.0f; 4.0f [0x858B90]=4.0f; -4.0f [0x859C3C]=-4.0f;
  8000.0f [0x859C38]=8000.0f; 40.764328f [0x859C34]=40.764328f; 50.0f [0x858B40]=50.0f; 20.222929f [0x859C30]=20.222929f;
  gbFirstPersonRunThisFrame [0xB6EC20] (u8).
- Behaviour (store order):
  1. `+1 = (u8)poolIdx`; `CompressFromFullMatrix(this+4, veh->m_matrix (+0x14))`.
  2. `h = veh->m_fHealth (+0x4C0)`; `fld h; fld 1000; fcomp st(1); test ah,5; jp` -> if `1000 < h` h = 1000 (NaN keeps h); `+2 = _ftol(h * 0.25f)`.
  3. `+3 = _ftol(veh->m_fGasPedal (+0x49C) * 100.0f)`.
  4. `+0x1C = veh->m_nVehicleType (+0x590) == 0 ? veh->+0x5B4 (damage manager panels dword) : 0`.
  5. Move speed, for c in x (+0x44 -> +0x20), y (+0x48 -> +0x21), z (+0x4C -> +0x22): `v' = v > 4 ? 4 : (v < -4 ? -4 : v)` (NaN -> v; tests:
     `fld 4; fcomp v; test ah,5; jnp` then `fld -4; fcomp st(1); test ah,0x41; jne`, then `fld 4; fcomp v; test ah,5; jp`);
     `byte = _ftol(v' * 8000.0f)` (values beyond +-127 wrap: only the low byte is kept).
  6. `+0x1A = veh->m_nModelIndex (+0x22, u16)`; `+0x2D = veh->+0x434` (primary colour); `+0x2E = veh->+0x435` (secondary).
  7. Flags byte +0x2F, starting from its CURRENT content in the block (not cleared): bit1 = entity flags (+0x1C) bit 7 (bIsVisible), stored;
     bit2 = flags bit 30, stored; bit3 = flags bit 31, stored (xor/and/xor merges keep the other bits).
  8. If `veh == FindPlayerVehicle(-1, false)` and `gbFirstPersonRunThisFrame` -> `+0x2F &= ~2`.
  9. `+0x23 = _ftol(model == 0x1B0 (432 RHINO) ? veh->+0x94C (m_fDoomVerticalRotation) * 40.764328f : veh->m_fSteerAngle (+0x494) * 50.0f)`.
  10. Only if `veh->m_nVehicleType == 0` (automobile, re-read): for w = 0..3 interleaved: `+0x24+w = _ftol(veh->+0x7D4+4w * 50.0f)` (wheel
      suspension compression) then `+0x28+w = _ftol(veh->+0x828+4w * 40.764328f)` (wheel rotation); `+0x18 = _ftol(veh->+0x5F4 * 20.222929f)`,
      `+0x19 = _ftol(veh->+0x60C * 20.222929f)` (front door angles); `+0x2C = 0`, then bit k (k = 0..5) set if door-status byte
      `veh->+0x5A9+k == 4` (BL holds 4 from step 7). For other vehicle types these fields keep the block's previous content.
  11. `+0x2F` bit0 = `veh->m_nPhysicalFlags (+0x40)` bit 29 (merge, other bits kept). `+0x30 = veh->+0x594` (sub type, u8).
  Not written: +0 (block type), +0x31 (vehicleType) and the padding.
- gta-reversed: ReplayBlockType.cpp:6 (not hooked) DIFFERS: (a) gr zero-initialises the whole block and sets `type`; the original writes only
  the fields above, so +0/+0x31/padding - and for non-automobiles +0x18/+0x19/+0x24..+0x2C - keep stale contents; (b) gr seeds physicalFlags
  with `(uint8)veh.m_nPhysicalFlags` before the bit merges, the original merges into the block's old +0x2F byte (bits 4..7 stale);
  (c) gr uses `/ 4.0f` where the original multiplies by 0.25f (same value). The callers (CReplay::StoreVehicle-like code) write the type byte
  themselves; check before relying on (a). Port the bytes.
- Open questions: none.
