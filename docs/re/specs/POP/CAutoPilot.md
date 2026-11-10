# CAutoPilot (POP) - full specs

CAutoPilot (0x98, gr AutoPilot.h): +0x0C field_C (int32; time the car entered the current path segment / curve, ms),
+0x10 m_nSpeedScaleFactor (int32; time in ms the segment takes at the current speed), +0x14 m_nCurrentPathNodeInfo,
+0x16 m_nNextPathNodeInfo (CCarPathLinkAddress u16: low 10 bits = link id, high 6 bits = area id), +0x25 _smthCurr,
+0x26 _smthNext (int8 direction +1/-1 along the current / next link), +0x27 m_nCurrentLane, +0x28 m_nNextLane (int8),
+0x38 m_speed (f32), +0x68 m_aPathFindNodesInfo[8] (CNodeAddress, 4 bytes each), +0x88 m_nPathFindNodesCount (int16).
ThePaths 0x96F050: m_pPathNodes[area] at 0x96F854 (+0x804), m_pNaviNodes[area] (CCarPathLink*, 0xE bytes each) at
0x96F974 (+0x924). CCarPathLink: +0 posn.x, +2 posn.y (int16, /8), +8 dir.x, +9 dir.y (int8, /100).

## 0x41B950 CAutoPilot::RemoveOnePathNode  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (CAutoPilot* this = ECX)`, plain `ret` (0x156202C). Entry `jmp 0x1562000` (.HOODLUM body,
  same behaviour). Clobbers EAX, EDX; ESI saved.
- Callers: CCarCtrl::PickNextNodeToFollowPath 0x427740 (3 sites), CCarCtrl::ReconsiderRoute 0x42FC40,
  CCarCtrl::SteerAICarWithPhysicsFollowPath_Racing 0x435830 (3 sites). Callees: none. Globals: none.
- Behaviour: `count = --m_nPathFindNodesCount` (int16 at +0x88, `dec word` 0x1562000, stored first);
  `for (i = 0 (int16); i < count (signed, re-read from +0x88 each iteration, 0x1562022); i++)
  m_aPathFindNodesInfo[i] = m_aPathFindNodesInfo[i + 1]` (dword copy +0x6C+4i -> +0x68+4i, 0x1562019/0x156201E).
  If count <= 0 after the decrement (0x1562013 `jle`) nothing is copied.
- Edge cases: no underflow guard: called with count 0 it stores -1. Slot `count` (the old last entry) is not cleared.
- gta-reversed: `plugin::CallMethod<0x41B950>` only (AutoPilot.cpp:51). Open questions: none.

## 0x41B980 CAutoPilot::ModifySpeed  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (CAutoPilot* this = ECX, float target [esp+4])`, `ret 4` (0x41BC6D).
- Callers: CCarCtrl::PickNextNodeRandomly 0x42DE80 (2), CCarCtrl::SlowCarOnRailsDownForTrafficAndLights 0x434790 (5),
  CCarCtrl::UpdateCarOnRails 0x436540 (2).
- Callees: CCarPathLink::OneWayLaneOffset 0x44DB00 (`__thiscall`, ECX = CCarPathLink*, no stack args, result ST0; 2 sites),
  CCurves::CalcSpeedScaleFactor 0x43C710 (`__cdecl float (const CVector* start, const CVector* end, float startDirX,
  float startDirY, float endDirX, float endDirY)` -> ST0, caller pops 0x18 at 0x41BC25), `_ftol` 0x821B40 (2 sites,
  truncation toward zero of ST0, pops).
- Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84 (u32, read twice: 0x41B9A0, 0x41BC3B/0x41BC41), ThePaths.m_pPathNodes
  0x96F854, ThePaths.m_pNaviNodes 0x96F974. Constants: 0x858C58 = 0.01f (f32 0x3C23D70A), 0x858C54 = 4294967296.0f,
  0x858C50 = 5.4f, 0x858C48 = 0.125f, 0x858C4C = 1000.0f.
- Behaviour:
  1. Clamp (0x41B980-0x41B99A): `fld target; fcom 0.01f; test ah,0x41; je keep` - keep `target` only if `target > 0.01f`;
     otherwise (also NaN) `target = 0.01f [0x858C58]`.
  2. Progress (0x41B9A0-0x41B9CB): `dt = now - field_C` (u32 subtraction); `fild dt` and, if dt as int32 < 0, `fadd 4294967296.0f
     [0x858C54]` (i.e. (float)(uint32)dt); `fidiv m_nSpeedScaleFactor` (+0x10, signed int32 divisor; 0 gives inf/NaN, no
     check). The quotient is stored f32 to a local `progress` (0x41B9C7), THEN `m_speed (+0x38) = target` (0x41B9CB). So
     m_speed is written even when the function returns early below.
  3. Guards: `cur = +0x14`; if `m_pPathNodes[cur >> 10] == null` return (0x41B9D7). `nxt = +0x16`; if
     `m_pPathNodes[nxt >> 10] == null` return (0x41B9F1). (Checks the path-node area pointer, not the navi-node one.)
     `C = &m_pNaviNodes[cur >> 10][cur & 0x3FF]`, `N = &m_pNaviNodes[nxt >> 10][nxt & 0x3FF]` (stride 0xE, 0x41BA01/0x41BA1A).
  4. Directions (each `fild int8; fmul 0.01f; fmul dirSign`, stored f32): `A = (int8)+0x25`, `B = (int8)+0x26` (as ints via fild).
     `cdx = (C.dir.x * 0.01f) * A` (0x41BA5E), `cdy = (C.dir.y * 0.01f) * A` (0x41BA76), `ndx = (N.dir.x * 0.01f) * B`
     (0x41BAAB), `ndy = (N.dir.y * 0.01f) * B` (0x41BABB).
  5. Lane offsets: `lc = (C->OneWayLaneOffset() + (int8)m_nCurrentLane (+0x27, fiadd)) * 5.4f` stored f32 (0x41BAF3);
     `ln = (N->OneWayLaneOffset() + (int8)m_nNextLane (+0x28, fiadd)) * 5.4f` kept in ST0 (not stored).
  6. Points (z = 0 for both, 0x41BB45/0x41BB4D):
     `end.x = (N.posn.x * 0.125f) + ln * ndy` (0x41BB34, stored f32); `end.y = (N.posn.y * 0.125f) - ln * ndx` (0x41BB69);
     `start.x = (C.posn.x * 0.125f) + lc * cdy` (0x41BB89); `start.y = (C.posn.y * 0.125f) - lc * cdx` (0x41BBA9)
     (posn components are int16, `fild` then `fmul 0.125f`; the product `ln*nd?` / `lc*cd?` is computed second and
     added to / subtracted from the scaled position).
  7. `sf = CalcSpeedScaleFactor(&start, &end, cdx', cdy', ndx', ndy')` where the four direction args are recomputed from
     the bytes with the same formula and order as step 4 (0x41BBAF-0x41BC17; identical values).
  8. `m_nSpeedScaleFactor = _ftol( sf * (1000.0f [0x858C4C] / m_speed) )` (x87: `fld 1000; fdiv [+0x38]; fmulp` = sf * q,
     0x41BC28-0x41BC38).
  9. `field_C = _ftol( (float)(uint32)now - (float)m_nSpeedScaleFactor_new * progress )` (now re-read 0x41BC3B, +2^32 if
     negative as int32 0x41BC4F; `fild newFactor; fmul progress; fsubp`, 0x41BC55-0x41BC64). Under 24-bit precision control
     the subtraction rounds to 24 significant bits, so for now > 2^24 ms the stored time is quantised.
- Edge cases: target is clamped before it is stored, so m_speed >= 0.01 (no division by 0 in step 8). Division by a zero
  m_nSpeedScaleFactor in step 2 is unchecked (inf/NaN progress, then `_ftol` of NaN/inf yields 0x80000000).
- gta-reversed: `plugin::CallMethod<0x41B980>` only (AutoPilot.cpp:46). Open questions: none.
