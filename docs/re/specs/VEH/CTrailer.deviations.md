# CTrailer: gta-reversed deviation notes (VEH)

Source: `re/gta-reversed/source/game_sa/Entity/Vehicle/Trailer.cpp` (pin 20d8d2a). CTrailer fields: +0x988
`m_supportCPs[2]` (CColPoint), +0x9E0 `m_supportRatios[2]`, +0x9E8 `m_fHeight`, +0x9EC `m_fTrailerTowedRatio`,
+0x9F0 `m_fTrailerTowedRatio2`.

## 0x6CFFD0 CTrailer::ProcessEntityCollision  (gr_deviations = 2)
Convention: `__thiscall int32 (CTrailer* this, CEntity* entity [esp+4], CColPoint* outColPoints [esp+8])`, `ret 8`.
Original order: `if (m_fTrailerTowedRatio == -1000.0f)` (dword compare with 0xC47A0000, 0x6CFFD9) -> return
`CAutomobile::ProcessEntityCollision(entity, outColPoints)`. Else bVehicleColProcessed if status != SIMPLE; `tcd` read
without a null check; local touch distances = 4 wheel compressions + 2 support ratios; `tcd->m_nNumLines = 0` when
physicalFlags & 0x9000 or entity type is 3 (ped) or 2 (vehicle); triangle hiding for tractor/trailer as in
CAutomobile; ProcessColModels into a local 6-entry colpoint array (esp+0x4C); restore triangles.

Row Trailer.cpp:297 (FIX_BUGS `if (!tcd || !ocd) return 0;`).
- Original bytes: no such test; `tcd` is dereferenced at 0x6D0087 / 0x6D014B, `ocd` only in the tow case
  (0x6D00AE-0x6D00C8).
- Verdict: CONFIRMED. Class: keep-original.

Row Trailer.cpp:304 ("Later reset back to original value").
- Original bytes: if `tcd->m_nNumLines == 0` after the collision call it is set to the constant 6 (0x6D014B-0x6D0152)
  and the line processing is skipped; gr does the same (`NUM_TRAILER_SUSP_LINES`).
- Verdict: REFUTED. Class: harmless.

Found in passing:
1. Support lines: the original processes the 2 support lines once, after the 4 wheels (0x6D02BF-0x6D0334:
   `if (t < 1.0f && t < m_supportRatios[k]) { count++; m_supportRatios[k] = t; m_supportCPs[k] = cp[4+k]; }`). gr nests
   that loop inside the wheel loop (it runs 4 times). After the first pass the later passes fail the `t < ratio`
   test, so only the processed-line counter differs and it is only tested for > 0. REFUTED as a behavioural
   difference; port the original structure.
2. Wheel entity reference: `slot = entity; entity->RegisterReference(&slot)` without CleanUpOldReference
   (0x6D01FB-0x6D0200); gr uses ChangeEntityReference. CONFIRMED, keep-original.
3. The original never copies the other vehicle's collision lighting (no vehicle branch after 0x6D0245); gr does. With
   vehicle entities the lines are disabled anyway (type 2 zeroes m_nNumLines), so this is unreachable. Harmless.
