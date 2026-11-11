# CPedGeometryAnalyser (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

Budget note: these four functions (226, 883, 26 and 1900 instructions) were NOT verified against the bytes in this pass. Every row below is UNCLEAR
pending a byte check; the port must follow the original, so do not take gr's bug-fix branches. Rows (gr file source/game_sa/PedGeometryAnalyser.cpp):

## 0x5F1890 CPedGeometryAnalyser::ComputeRouteRoundSphere  (gr_deviations = 2)
- cpp:1024 `notsa::bugfixes::CPedGeometryAnalyser_ComputeRouteRoundSphere_IncorrectDetourPosition`: UNCLEAR (not checked). Class: keep-original (original = bugfix off).
- cpp:1036 NOTE/BUG (Pirulax) (deviates=no): UNCLEAR (not checked).
- cpp:1041 `..._AmbigousDetourTarget` bugfix: UNCLEAR (not checked). Class: keep-original (original = bugfix off).

## 0x5F1FA0 CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached  (gr_deviations = 3)
- cpp:314 BUGFIX, cpp:318 BUGFIX, cpp:391 "differs a bit from the original code ... bb's two axes": UNCLEAR (not checked). Class: keep-original.
- cpp:340 "original code ... simplified", cpp:350 assert, cpp:409 grid vectors (classified harmless): UNCLEAR (not checked).

## 0x5F3B70 CPedGeometryAnalyser::ComputeClosestSurfacePoint-ped  (gr_deviations = 1)
- cpp:229 `bugfixes::GenericUB: return (!GenericUB || computed) && ComputeClosestSurfacePoint(...)`: UNCLEAR (not checked; the original presumably ignores the
  `computed` result - verify at the call of ComputeEntityBoundingBoxCorners inside 0x5F3B70). Class: keep-original.

## 0x5F3DD0 CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox-2  (gr_deviations = 2)
- cpp:727 "For completeness, the original code:" (maybe), cpp:871 assert (harmless), cpp:995 `default: NOTSA_UNREACHABLE_CASE(forceDirection)` (maybe): UNCLEAR (not checked).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5F1B00 CPedGeometryAnalyser::CanPedJumpObstacle-LoS  (gr_deviations = 1)
Original bytes (`__cdecl bool (CPed* ped [esp+4], CEntity* obstacle [esp+8])`, `ret`): if `obstacle->m_nFlags (+0x1C) & 0x400000` -> return 0 (0x5F1B0A-0x5F1B12).
Else start = ped position (matrix+0x30 or placement +4), end = start + ped->m_matrix->forward (+0x10..+0x18; the matrix pointer is used for the forward vector even
when it is null - no check, 0x5F1B2B); returns `CWorld::GetIsLineOfSightClear(&start, &end, 1, 0, 0, 1, 0, 0, 0)` 0x56A490 (buildings, objects only) in AL.
Row PedGeometryAnalyser.cpp:118 (`if (!pedCM) return false; // BUGFIX`):
- Original bytes: this overload never reads a collision model; the row's code belongs to the contact-normal overload of CanPedJumpObstacle (gr cpp:105-130), not to 0x5F1B00.
- Verdict: REFUTED for 0x5F1B00 (row mis-attributed). Class: harmless. (For the other overload the original has no null check - keep-original there.)

## 0x5F2F00 CPedGeometryAnalyser::GetIsLineOfSightClear-v3d  (gr_deviations = 1)
Original bytes (`__cdecl bool (const CVector* start [esp+4], const CVector* target [esp+8], CEntity* entity [esp+0xC])`, `ret`): `CColLine line(start, target)` 0x40EF50;
`cm = CModelInfo::ms_modelInfoPtrs[entity->m_nModelIndex (+0x22, movsx)]->m_pColModel (+0x14)` (0x5F2F1C-0x5F2F27, NO null check); entity matrix: if +0x14 == 0
`AllocateMatrix` 0x54F560 + `m_placement.UpdateMatrix(matrix)` 0x54F1B0; `r = CCollision::TestLineOfSight(&line, matrix, cm, 0, 0)` 0x417730; returns EAX = (r == 0) ? 1 : 0
(`neg al; sbb eax,eax; inc eax`, 0x5F2F5A-0x5F2F60).
Row PedGeometryAnalyser.cpp:1121 (`BUGFIX: Check for null`):
- Original bytes: no null check of cm (0x5F2F27) - passes a null col model to TestLineOfSight.
- Verdict: CONFIRMED. Class: keep-original.
- Also: gr returns `cm && TestLineOfSight(...)` while the bytes return the NEGATION of TestLineOfSight's result (0x5F2F5A). Verdict CONFIRMED (keep-original), unless gr's
  TestLineOfSight already returns "clear" - check that callee's spec before porting (open question at 0x5F2F5A).

## 0x5F3590 CPedGeometryAnalyser::GetNearestPed  (gr_deviations = 1)
Original bytes (`__cdecl CPed* (const CVector* point [esp+4])`, `ret`): min = FLT_MAX (0x7F7FFFFF), best = 0; walks the ped pool [0xB74490] DOWNWARDS (i = size-1 .. 0, `dec edx`
0x5F35C5, free slots skipped by `js` 0x5F35CE, element 0x7C4); d2 = `(dz*dz + dy*dy) + dx*dx` with `d = point - pedPos` (0x5F35E5-0x5F3604); if `d2 < min` (strict, `test ah,5; jp`)
-> min = d2 (f32 store), best = ped (0x5F3617).
Row PedGeometryAnalyser.cpp:1129 (GetAllValid ascending):
- Verdict: CONFIRMED - on equal distances the original keeps the HIGHEST pool index, gr the lowest. Class: keep-original.

## 0x5F36F0 CPedGeometryAnalyser::ComputeClosestSurfacePoint-pos  (gr_deviations = 1)
Original bytes (`__cdecl bool (const CVector* pos [esp+4], CEntity* entity [esp+8], CVector* out [esp+0xC])`, `ret`): `ComputeEntityBoundingBoxCornersUncached(pos->z (f32 by value),
entity, corners)` 0x5F1FA0 - result IGNORED (0x5F3709); returns `ComputeClosestSurfacePoint(pos, corners, out)` 0x5F2C10 (0x5F3719).
Row PedGeometryAnalyser.cpp:236 (`(!GenericUB || computed) && ...`): Verdict CONFIRMED (with the fix gr skips when the corners were not computed; the original uses whatever
the 0x30-byte stack array holds). Class: keep-original.

## 0x5F3970 CPedGeometryAnalyser::IsEntityBlockingTarget  (gr_deviations = 1)
Original bytes (`__cdecl bool (CEntity* entity [esp+4], const CVector* point [esp+8], float radius [esp+0xC])`; entry obfuscated: `nop; nop; jmp 0x4096F8` loads ESI = entity and
returns to 0x5F397B): `d = entityPos - point` (x, y stored f32; 0x5F3992-0x5F39A7); if `|d.z| > 3.0f [0x858B3C]` (strict, `test ah,0x41; je`) -> 0x5F3AAA (return false).
`r = colModel(+0x14 of model info)->boundSphere.radius (+0x24)`; `dist = sqrt(d.y*d.y + d.x*d.x)` (NOT squared); if `r*r + radius*radius < dist` (`fcompp; test ah,5; jnp`,
0x5F39E2-0x5F39FF) -> return false. Then plane tests (not re-specified here).
Row PedGeometryAnalyser.cpp:1149 (bugfix: compare (r+radius)^2 with the squared distance):
- Verdict: CONFIRMED - the original compares `r^2 + radius^2` with the unsquared 2D distance (gr's non-fix branch matches the bytes). Class: keep-original.
