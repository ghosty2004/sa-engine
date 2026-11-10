# C3dMarkers (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/3dMarkers.cpp`, `3dMarker.cpp` (pin 20d8d2a). C3dMarker stride 0xA0 (m_aMarkerArray, 32
entries ending at 0xC7F158; +0x48 m_Atomic, +0x50 m_Type (u16), +0x52 m_IsInUse, +0x53 m_IsActive, +0x54 m_ID, +0x60 m_StartTime).
C3dMarkers::m_pRpClumpArray 0xC7C6DC, 7 entries (end 0xC7C6F8).

## 0x722710 C3dMarkers::Shutdown  (gr_deviations = 1)
Original: `__cdecl void ()`, plain `ret` (0x7227AF).
Row 3dMarkers.cpp:82 ("Original code is retarded, this does the same, but better.").
- Original bytes:
  1. Markers (0x722713-0x72275A, esi = &marker.m_StartTime, stride 0xA0): if `m_Atomic != NULL`: `m_ID = 0` (0x722728),
     `m_StartTime = 0` (0x72272B), `m_IsInUse = 0` (0x72272D), `m_IsActive = 0` (0x722730), `m_Type = 0x101` (MARKER3D_NA, 0x722733),
     `frame = atomic->object.parent (+4)` (0x722739), `RpAtomicDestroy(atomic)` 0x749DC0, `RwFrameDestroy(frame)` 0x7F05A0,
     `m_Atomic = NULL` (0x72274B).
  2. Clumps (0x72275C-0x7227AA), for i = 0..6: if `clump = m_pRpClumpArray[i]` != NULL: `RpClumpForAllAtomics(clump, RemoveRefsCB
     0x7226D0, NULL)` (0x722770), `RpClumpDestroy(clump)` (0x722778, re-read from the array); then for j = 0..i-1: if
     `m_pRpClumpArray[j] == m_pRpClumpArray[i]` set `m_pRpClumpArray[j] = NULL` (0x722782-0x72279C); finally `m_pRpClumpArray[i] = NULL`.
- Verdict: REFUTED. The inner j-loop can never match: every earlier entry is NULL by then (each processed entry is set to NULL at
  0x72279E), and entry i is non-NULL. gr's `std::exchange` loop gives the same calls in the same order. Later duplicates (j > i) are
  not cleared in either version (a clump stored twice would be destroyed twice in both). Class: harmless.
Found in passing (marker part, gr C3dMarker::DeleteMarkerObject 3dMarker.cpp:82): gr adds `m_Mat.Detach()` under
`notsa::bugfixes::GenericCrashing` between RpAtomicDestroy and RwFrameDestroy; the original has no such step - CONFIRMED,
keep-original (port must not detach the matrix).
