# CBike: gta-reversed deviation notes (VEH)

Source: `re/gta-reversed/source/game_sa/Entity/Vehicle/Bike.cpp`. Layout and compare idioms: see CBike.md.

## 0x6BDEA0 CBike::ProcessEntityCollision  (gr_deviations = 2)
Convention: virtual `__thiscall int32 (CEntity* entity [esp+4], CColPoint* outColPoints [esp+8])`, `ret 8`, EAX = number of col points.

Row Bike.cpp:409 (fix-bugs-ifdef: `#ifdef FIX_BUGS if (!tcd || !ocd) return 0;`).
- Original bytes: `tcd = GetColModel()->m_pColData` is loaded at 0x6BDED3-0x6BDEED and dereferenced without any null test
  (`mov byte [ecx+6],0` at 0x6BDF31, `mov cl,[eax+6]` at 0x6BE15E); the entity's col data is never fetched separately.
- Verdict: CONFIRMED (the early return does not exist in the original).
- Class: keep-original (no null check; a port must not return early).

Row Bike.cpp:416 (original-comment: `tcd->m_nNumLines = 0; // Later reset back to original value`).
- Original bytes: if `m_nPhysicalFlags & 0x9000` (bSkipLineCol bit 12 | bProcessingShift bit 15; `test ah,0x90` 0x6BDF07) or
  `(entity->m_nType & 7) == 3` (ped): `tcd->m_nNumLines = 0` (0x6BDF31). After the col-model test: `if (tcd->m_nNumLines == 0)
  { tcd->m_nNumLines = 4; skip the wheel loop }` (0x6BE15A-0x6BE169). gr does the same (`else tcd->m_nNumLines = NUM_SUSP_LINES`);
  the "original value" is the constant 4, not a saved value.
- Verdict: REFUTED (gr matches the bytes; the marker is only a comment).
- Class: harmless.

Additional differences found in the bytes (not rows of gr-deviations.csv; gr hooks this function as reversed):
1. Driver seat position. Original (0x6BDFF8-0x6BE090): `driverMat = CMatrix(*m_matrix)`; `seat = mi->m_pVehicleStruc->m_avDummyPos[
   (mi->m_nVehicleType (+0x3C) == 5) ? 0 : 4]`; `driverMat.pos += Multiply3x3(*m_matrix, seat)` (rotation only, 0x59C790).
   gr: `driverMat.GetPosition() += GetDriverSeatDummyPositionWS()` = `m_matrix.TransformPoint(seat)`, which includes the translation,
   so gr adds the vehicle position twice. CONFIRMED logic error in gr; keep-original.
2. Copied points. Original: the driver/entity test writes into a function-local static `CColPoint[32]` at 0xC1C280 (constructed once,
   guard bit 0 of 0xC1C800, `eh vector constructor iterator` 0x403D60 with ctor 0x4217B0) and the copy loop reads that same array
   (0x6BE108 `ebp = 0xC1C280`). gr fills `pedEntityColPts` but copies from the never-filled `pedCPs[i]` (and tests its
   `m_nPieceTypeA`). CONFIRMED logic error in gr; keep-original (copy the filled points).
3. Copy bound. Original loop (0x6BE110): `if (numColPts >= 31) stop` (`cmp [esp+0x18],0x1F; jge`), so at most 31 points total;
   gr uses `numColPts < 32`. Copy condition: `piece = cp.m_nPieceTypeA (+0x21); if (piece == 0) skip` (`test cl,cl; jbe`), same as
   gr's `== PED_COL_SPHERE_LEG (0)`. CONFIRMED (off by one); keep-original.
4. Ground reference. Original (0x6BE1DC-0x6BE1E1): `m_aGroundPhysicalPtrs[i] = entity; entity->RegisterReference(&m_aGroundPhysicalPtrs[i])`
   with no CleanUpOldReference of the previous pointer. gr's `ChangeEntityReference` first clears the old reference. CONFIRMED;
   keep-original (do not unregister the old entity).
5. Order: original stores `m_anCollisionLighting[i] = cp.m_nLightingB` first, then the per-type block, then `m_nContactSurface =
   cp.m_nSurfaceTypeB` (0x6BE247); gr sets the contact surface before the switch. No observable difference (harmless).
Everything else (status != SIMPLE -> `byte +0x42B |= 1`; `m_nTestPedCollision == 1 -> 0` when the driver test hits; wheel loop
condition `ratio[i] < 1.0f && ratio[i] < saved[i]`; tunnel-flag copy for buildings; collision records; bHasHitWall `|= 0x1000` for
buildings or objects with physical flag 0x4) matches gr.
