# CAutoPilot (VEH) - full specs (writer group E)

Layout: gr `AutoPilot.h` / plugin-sdk `CAutoPilot.h` (size 0x98, offsets validated there).

## 0x6D5E20 CAutoPilot::CAutoPilot  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CAutoPilot* (this = ECX)`, returns `this` in EAX, plain `ret`.
- Callers: CVehicle::Constructor 0x6D5F10. Callees: `rand` 0x821B1E.
- Globals: `CTimer::m_snTimeInMilliseconds` 0xB7CB84 (read once, 0x6D5EA1).
- Behaviour (store order of the bytes):
  1. u16 0xFFFF to +0x00, +0x04, +0x08 (areaId of m_currentAddress / m_startingRouteNode / m_endingRouteNode; the
     nodeId halves +0x02/+0x06/+0x0A are NOT written), +0x14, +0x16, +0x18 (path link addresses), and +0x68 + 4*i
     for i = 0..7 (areaId of m_aPathFindNodesInfo; nodeId halves untouched).
  2. +0x26 = 1, +0x25 = 1.
  3. `+0x4B (m_nCarCtrlFlags) &= 0xFC` then `&= 0x07` (0x6D5E63-0x6D5EA7): net `flags &= 0x04` - bit 2
     (bAvoidLevelTransitions) keeps whatever was in memory, all other bits cleared.
  4. +0x0C = 0; +0x10 = 1000; +0x28 = 0; +0x27 = 0; +0x29 = 0; +0x2A = 0; +0x2B = 0; +0x40 (cruise speed) = 10;
     +0x38 = 10.0f; +0x88 (u16 path node count) = 0; +0x8C = null; +0x1C = +0x20 = now; +0x41 = 0; +0x44 = 1.0f;
     +0x94 = 0xFF (-1, vehicle recording id); +0x35 = 0; +0x4D = 20; +0x30 = 0; +0x34 = 0; +0x49 = 0; +0x4A = 0;
     +0x4E = 10; +0x4F = 10; +0x95 = 0; `+0x4C &= 0xFC` (clear bIsStopped, bIsParked; other bits kept).
  5. `+0x50 = (rand() & 7) + 2` (byte); +0x90 = null; +0x3C = 0.
  Untouched: +0x02/+0x06/+0x0A, +0x1A-0x1B, +0x24, +0x2C, +0x36-0x37, +0x42-0x43, +0x48, +0x51-0x67 (incl.
  m_vecDestinationCoors), node-id halves of +0x68.., +0x8A, +0x96.
- gta-reversed (AutoPilot.cpp:6): `m_nCarCtrlFlags = 0` clears bit 2 as well -> differs from the bytes (the
  original leaves bit 2 = previous memory content of the pool slot). Keep-original for a 1:1 port (bit 2 is
  then whatever the vehicle pool memory held). gr `field_50 = GetRandomNumber() % 8 + 2` equals `(rand() & 7) + 2`.
  gr's CNodeAddress default constructor must also leave the nodeId half alone to match.
