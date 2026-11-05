# CTheZones (WORLD)

Globals: `CTheZones::ZonesVisited` 0xba3730 (`bool[10][10]`, 100 bytes; the code addresses it as
`0xba3739 + idx` = `ZonesVisited + 9 + idx`), `CTheZones::ZonesRevealed` 0xba372c (int32),
`CTheZones::m_CurrLevel` 0xba6718 (int32). Constants: `0x859a94` f32 3000.0, `0x865060` f32
0.0016666667 (0x3ada740e, i.e. ~1/600). `_ftol` = CRT 0x821b40: converts ST0 (pops it) to a 64-bit
integer by truncation toward zero, returns EDX:EAX; the code below uses only AL.

Zone index (shared by both functions):
`xi = (u8) _ftol((x + 3000.0f) * 0.0016666667f)`, `yi = (u8) _ftol((y + 3000.0f) * 0.0016666667f)`
(each: `fld x; fadd [0x859a94]; fmul [0x865060]` with 24-bit PC, then `_ftol`, then `movzx al`),
`idx = xi*10 - yi` (int32), byte address `0xba3739 + idx`.

## 0x572130 CTheZones::GetCurrentZoneLockedOrUnlocked  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl bool (float x [esp+4], float y [esp+8])` (a CVector2D by value),
  return AL = `byte != 0` (`setne`), plain `ret`.
- Callers: 3 (`SetCurrentZoneAsUnlocked` 0x572800, `CMenuManager::PrintMap` 0x575130,
  `CRadar::HasThisBlipBeenRevealed` 0x583af0). Callees: `_ftol` 0x821b40 (twice).
- Behaviour: computes `xi` (x first) then `yi`, returns `ZonesVisited_flat[9 + xi*10 - yi] != 0`.
- Edge cases: only the low byte of the `_ftol` result is used, so out-of-map coordinates wrap modulo 256
  (e.g. x < -3600 gives `_ftol = -1` → `xi = 255` → index up to 2559): out-of-bounds read, original bug,
  keep it. `yi > 9` reads before the row (into the previous row / before the array).
- gta-reversed: `GetZoneWasVisited` (TheZones.cpp:55) divides by `600.f` instead of multiplying by
  the f32 constant 0.0016666667, and casts to `size_t` (no 8-bit truncation). Both differ from the bytes:
  the multiply can round differently at zone borders, and the u8 truncation changes out-of-range
  indices. Port keeps the original.
- Open questions: none.

## 0x572800 CTheZones::SetCurrentZoneAsUnlocked  (catalogue: specified/REA, port level 2)
- Convention / signature: `__cdecl void (void)`, plain `ret`. 12-byte local CVector `pos` at `[esp]`.
- Callers: 1 (`CTheZones::Update` 0x572d10). Callees: `CWorld::FindPlayerCoors` 0x56e010
  (`__cdecl CVector* (CVector* out, int32 playerId = -1)`), `CTheZones::GetLevelFromPosition` 0x572300
  (`__cdecl int32 (const CVector*)`; one `add esp,0xc` pops the args of both calls),
  `GetCurrentZoneLockedOrUnlocked` 0x572130, `_ftol` 0x821b40.
- Globals: read `CTheScripts::bPlayerIsOffTheMap` 0xa444a4 (u8), `CGame::currArea` 0xb72914 (int32);
  write `m_CurrLevel` 0xba6718; read/write `ZonesRevealed` 0xba372c; write `ZonesVisited` byte.
- Behaviour:
  1. `FindPlayerCoors(&pos, -1)`; `m_CurrLevel = GetLevelFromPosition(&pos)`.
  2. If `bPlayerIsOffTheMap != 0` return. If `currArea != 0` return (inlined
     `CGame::CanSeeOutSideFromCurrArea`).
  3. If `GetCurrentZoneLockedOrUnlocked(pos.x, pos.y)` returns true (AL != 0) return.
  4. Recompute the index from `pos.x`, `pos.y` exactly as above (same instructions), then
     `ZonesRevealed += 1` (read, increment) and write byte `1` at `0xba3739 + idx`, then store
     `ZonesRevealed` (byte store precedes the counter store; no observable difference single-threaded).
- Edge cases: same index wrap as above (out-of-bounds write for off-map positions; guarded in practice by
  `bPlayerIsOffTheMap`).
- gta-reversed: `SetCurrentZoneAsUnlocked` is a raw call to 0x572800; its logic is also re-coded inline
  in `CTheZones::Update` (TheZones.cpp:356) with the `/600.f` index (see above). Port the bytes.
- Open questions: none.
