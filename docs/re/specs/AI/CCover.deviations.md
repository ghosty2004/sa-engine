# CCover: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

CCover::m_Points 0xC197C8: 100 entries of 0x1C (+0 type i8 (gr CoverPoint.h: 0 NONE, 1 OBJECT, 2 VEHICLE,
3 POINTONMAP), +1 usage u8, +2 dir u8, +4 CVector pointOnMap, +0x10 CEntity* coverEntity, +0x14/+0x18 occupant peds);
m_NumPoints 0xC197A4; m_ListOfProcessedBuildings 0xC1A2B8 (CPtrNodeDoubleLink: +0 item, +4 next, +8 prev).

## 0x698F30 CCover::AddCoverPoint  (gr_deviations = 4: 2 yes, 2 no)
Original bytes (`__cdecl void (int32 type [esp+4], CEntity* entity [esp+8], const CVector* pos [esp+0xC], uint8 usage [esp+0x10], uint8 dir
[esp+0x14])`, plain `ret`; EAX on return is garbage):
  1. Return if `m_NumPoints >= 100` (signed, 0x698F38).
  2. For each of the 100 points (0x698F51-0x699071): if `(int8)cpt.type == type`: type 1 or 2 -> return if `entity == cpt.coverEntity`
     (0x698F94); type 3 -> return if `pos->x == cpt.x && pos->y == cpt.y && pos->z == cpt.z` (exact, NaN unequal; 0x698F68-0x698F8C).
     Then, if `cpt.type != 0` (0x698F9D): `at = pos ? *pos : entity ? entity position : (stack garbage)`; `d = sqrt(dz*dz + dy*dy + dx*dx)`
     with `dc = cpt.c - at.c` (fsqrt; 0x698FDD-0x69900B); return if `d < 0.8f [0x858C98]` (0x699013-0x69901E); else recompute the same d
     (dx kept on the x87 stack, dy/dz reloaded from their f32 stores) and if `d < 2.0f [0x858CA0]` (NaN: no) and
     `(uint8)(cpt.dir - dir)` is < 0x20 or > 0xDF (unsigned) -> return (0x699044-0x699062).
  3. Find the first point with type == 0 (unbounded scan from index 0, 0x699077-0x699090); set type, usage (+1), dir (+2), occupants
     (+0x14, +0x18) = 0, `m_NumPoints++`, coverEntity (+0x10) = entity; if entity != 0 and NOT (`(entity+0x36 & 7) == 1` (building) and
     `byte entity+0x2E == 0`) -> `entity->RegisterReference(&cpt.coverEntity)` (0x6990DA-0x6990EF); if pos != 0 copy *pos to +4.
Row Cover.cpp:177 (`NOTE: Original function didn't return the (possibly) created point, we do`):
- Original: void (EAX not meaningful). Verdict: CONFIRMED (signature only). Class: harmless (no original caller reads EAX: callers
  0x699120 and 0x6997E0 ignore it).
Row Cover.cpp:180 (assert): REFUTED (debug only). Class: harmless.
Row Cover.cpp:208 (bug note): the original compares the point-on-map of every non-NONE point (0x698F9D) - note is accurate. REFUTED.
Row Cover.cpp:209 (`notsa::IsFixBugs() ? type != POINTONMAP : type == NONE`):
- Original: skips only type 0 (`test cl,cl; je` at 0x698F9D), so entity cover points are also distance-tested using their (+4) field.
- Verdict: CONFIRMED (with FixBugs gr skips entity points). Class: keep-original.
Additional (no row): the original compares `sqrt(d2) < 0.8` / `< 2.0`, gr `d2 < sq(0.8f)` / `sq(2.f)` - rounding-level difference.

## 0x6997E0 CCover::Update  (gr_deviations = 6: 5 yes/maybe, 1 no)
Rows Cover.cpp:82 / :101 (pool iteration with GetAllValid, ascending):
- Original: both loops walk the pools DOWNWARD: vehicles `i = size-1 .. 0` (`dec ebp` 0x6998DE, stride 0xA18, slot skipped when
  `byteMap[i] & 0x80`), objects `i = size-1 .. 0` (`dec ebx` 0x6999DB, stride 0x19C). AddCoverPoint is order dependent (first free
  slot, 100-point limit, nearby-duplicate test).
- Verdict: CONFIRMED (x2). Class: keep-original.
Row Cover.cpp:120 (bug note): REFUTED (comment only).
Rows Cover.cpp:127 / :132 / :155 (FixBugs use-after-free handling):
- Original: processed-building loop (0x699A7D-0x699ADB) calls ShouldThisBuildingHaveItsCoverPointsCreated(item) 0x699230 with no null
  check; if false: RemoveCoverPointsForThisEntity(item) 0x698740, unlink and `delete node` 0x5523D0. New list nodes (0x699D69-0x699D9A):
  `new(0xC)`, item = building, prev = 0, pushed at the head; no RegisterReference (and a failed allocation writes through null at
  0x699D7D). No references are registered or cleaned for list items.
- Verdict: CONFIRMED (x3; gr's FixBugs path registers/cleans references). Class: keep-original.
