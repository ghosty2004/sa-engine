# C3dMarker (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/3dMarker.cpp` (pin 20d8d2a).

## 0x722390 C3dMarker::DeleteMarkerObject  (gr_deviations = 2)
Rows 3dMarker.cpp:82 (`if (notsa::bugfixes::GenericCrashing) {`) and :83 (`m_Mat.Detach(); // FIX(#1514): Otherwise
C3dMarkers::Init writes into the destroyed frame's matrix`).
- Original bytes (0x722390-0x7223C4, thiscall, plain `ret`): `[this+0x54] = 0` (m_ID), `[this+0x60] = 0` (m_StartTime),
  `byte [this+0x52] = 0` (m_IsInUse), `byte [this+0x53] = 0` (m_IsActive), `word [this+0x50] = 0x101` (m_Type = MARKER3D_NA 257),
  `frame = [m_Atomic(+0x48) + 4]`, `RpAtomicDestroy(m_Atomic)` 0x749DC0, `RwFrameDestroy(frame)` 0x7F05A0, `[this+0x48] = 0`.
  There is no Detach of m_Mat: m_Mat.m_pAttachMatrix (+0x40) keeps pointing into the destroyed frame.
- Verdict (both rows): CONFIRMED - gr adds a Detach that the original does not do. Class: keep-original (the port must leave
  +0x40/+0x44 untouched; the dangling attach pointer is original behaviour).
