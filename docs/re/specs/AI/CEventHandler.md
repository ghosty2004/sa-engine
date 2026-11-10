# CEventHandler (AI) - RE-SPEC-3 group I2

Layout (gr EventHandler.h): +0 m_Ped, +4 m_History, +0x20 m_PhysicalResponseTask, +0x24 m_EventResponseTask, +0x28 m_AttackTask,
+0x2C m_SayTask, +0x30 m_PartialAnimTask. Event CEventPedCollisionWithPed (+0x14 m_victim, +0x30 m_movestate i16 = this ped's,
+0x32 m_victimMoveState i16); CEventPedCollisionWithPlayer has the same layout (+0x14 = the player). Move states: 0 NONE, 1 STILL,
4 WALK, 6 RUN, 7 SPRINT. CPedStats +0x2C = m_fDefendWeakness (ped +0x59C = m_pStats). Common building blocks (exact arguments):
- SAY: `p = new(0x1C)`; `m_SayTask = p ? CTaskSimpleSay(p, 0x1C, -1) 0x48E360 : 0` (the store goes through an obfuscated jump at
  0x40235E in the first function; no effect).
- LOOK(name, a, b): `g_ikChainMan.LookAt(name, a, b, 2000, 5, nullptr, true, 0.25f, 500, 3, false)` 0x618970.
- ROUTE(tsimplest): only if `tsimplest && CTask::IsGoToTask(tsimplest)` 0x61A360: local CPointRoute `r` with `r.count = 0`;
  `CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox(m_Ped, other, &tsimplest->+0xC (goto target), &r, 0)` 0x5F6110;
  `R = (p = new(0x3C)) ? CTaskComplexFollowPointRoute(p, 4, &r, 0, 0.5f [0x86FC98], 5.0f [0x86FC9C], false, true, true) 0x671510 : 0`.
- HITSIDE = `CPedGeometryAnalyser::ComputeEntityHitSide(victim, m_Ped)` 0x5F3BC0 (cdecl (entity, ped) per the pushes).
- FALL = `R = (p = new(0x18)) ? CTaskComplexFallAndGetUp(p, HITSIDE, 0) 0x678700 : 0`; HIT = `R = (p = new(0x10)) ?
  CTaskComplexHitResponse(p, HITSIDE) 0x631D70 : 0`.
- LEADERFIX(f, seek, w, gp): `d = m_Ped pos - f->m_Leader (+0x10) pos` (VectorSub 0x40FE60), `d.z = 0`, `m = |d|` (CVector::Magnitude
  0x4082C0, stored f32); then flag/radius updates (listed per use); `seek->SetMoveStateRadius(m + 0.1f [0x858B1C])` 0x4BC470; if
  `m < 8.0f [0x86F828]` (strict) `f->+0x20 = d` (the follower's formation offset).
Task types: 0x4B7 GANG_FOLLOWER (f, FindTaskByType), 0x395 AVOID_OTHER_PED_WHILE_WANDERING (w, FindActiveTaskByType), 0x384 GO_TO_POINT
(gp, FindTaskByType), 0x38B SEEK_ENTITY (seek, FindTaskByType), 0xF3 BE_IN_GROUP, 0x391 FOLLOW_LEADER_IN_FORMATION.

## 0x4BDB80 CEventHandler::ComputePedCollisionWithPedResponse  (catalogue: specified/REA, port level 9)
- `__thiscall void (this, CEventPedCollisionWithPed* e [esp+4], CTask* tactive [+8], CTask* tsimplest [+0xC])`, `ret 0xC`, SEH
  (`sub esp,0xEC`). Strings: "CompPedCollPedResp" 0x85BA54. P = m_Ped, V = e->m_victim, R = response (local, starts 0).
- 1. If V == 0 return WITHOUT writing m_EventResponseTask. `V->intel->m_TaskMgr.GetSimplestActiveTask()` 0x6819D0 is called and its
  result discarded (keep the call only if it has side effects - it does not).
- 2. `d = Vpos - Ppos`; `inFront = !((d.z*Pf.z + d.y*Pf.y) + d.x*Pf.x < 0)` (Pf = P matrix forward; NaN -> false) and `vFacing =
  ((d.z*Vf.z + d.y*Vf.y) + d.x*Vf.x <= 0)` (NaN -> false) (0x4BDBE1-0x4BDC41). `msP = e->+0x30`, `msV = e->+0x32`.
- 3. If P is in a group g (CPedGroups::GetPedsGroup 0x5F7E80): `lA = g->GetLeader()`, `gv = GetPedsGroup(V)`, `lV = gv ? gv->GetLeader() : 0`;
  `same = (g == gv)` (this REPLACES inFront for the rest of the branch); if same: `if (V == lA) V->+0x474 |= 0x80000000`;
  `if (P == lA) P->+0x474 |= 0x80000000`.
  - If `V->IsPlayer()`: SAY; go to END (R = 0).
  - If gv == 0: if msP == 1 or msV == 1: EVADE1 -> END; else END.
  - If msP != 1 and msV != 1: END.
  - If !same: `r = CGeneral::GetRandomNumberInRange(0.0f, 1.0f)` 0x41BD90; if `r < 0.25f [0x858C84]`: `r2 = GetRandomNumberInRange(0.0f,
    1.0f)`; `r2 < 0.33f [0x85AD74]` -> LOOK("CompPedCollPedResp", P, V); else if `r2 < 0.66f [0x85BA50]` -> SAY. Continue at B.
    If same: if V == lV continue at B; else if lA == 0 END; else if `lA->intel->GetMoveStateFromGoToTask()` 0x601D70 != 1 END; else B.
  - B: f, w, gp, seek as above. If same and f and `f->m_Leader` and seek and (w or gp): LEADERFIX with `if (w) w->+0x5C |= 2` and
    `if (gp) gp->+0x18 = m + 0.1f` (both, independently); END. Else if f and same: END. Else EVADE1 -> END.
  - EVADE1: `v = (Pf.x, -Pf.y, 0)` (P's matrix forward with y negated - not a rotation; literal bytes 0x4BDF55-0x4BDF7F),
    `v.Normalise()`; `R = (p = new(0x1C)) ? CTaskComplexEvasiveStep(p, V, &v) : 0`.
- 4. P not in a group (0x4BE05D):
  - msP == 4: if msV in {0, 1}: if inFront { ROUTE(tsimplest) (R); SAY } else { LOOK(P, V); SAY }; END.
    If msV == 4: if inFront { ROUTE(tsimplest); LOOK(P, V); SAY } else if vFacing { LOOK(P, V); SAY }; END.
    Other msV: go to C.
  - msP in {6, 7} and msV in {6, 7} (0x4BE58D): if inFront: if vFacing: STATS; else { SAY; LOOK(P, V) }. If !inFront: if vFacing
    { SAY; LOOK(P, V) }. END.
  - msP in {0, 1} and msV in {4, 6, 7} and vFacing: if msV in {6, 7}: STATS. If msV == 4: SAY; LOOK(P, V); END.
  - otherwise C.
  - STATS: `sP = P->m_pStats->m_fDefendWeakness (+0x2C)`, `sV` likewise. If `sP > 1.5f [0x85AAE0]` and (`sV <= 1.5f` or NaN, or
    `sP > sV`): FALL (END). Else: SAY; LOOK(P, V); then (in the msP-moving case and the msV in {6,7} case) HIT; END.
    (All comparisons strict as written; the bytes test sP first, then sV, then sP > sV, 0x4BE365-0x4BE39E / 0x4BE5A5-0x4BE5DE.)
  - C (0x4BE4B7): if msV in {0, 1} and msP in {4, 6, 7} and vFacing: SAY; LOOK(P, V); if msP != 4: HIT. END.
- END (0x4BE78A): `m_EventResponseTask = R` (always written on this path, also when 0); if R and `V->IsPlayer()`:
  `CMentalState::IncrementAnger(ECX = P->m_pIntelligence, 1)` 0x421050.
- gta-reversed: `.Reversed = false` (plugin::Call). Confidence medium: the decision tree is long; every branch target above was
  traced from the jumps, but a second reader should re-verify the STATS / C entry conditions against 0x4BE313-0x4BE4D7.

## 0x4BE7D0 CEventHandler::ComputePedCollisionWithPlayerResponse  (catalogue: specified/REA, port level 9)
- `__thiscall void (this, CEventPedCollisionWithPlayer* e, CTask* tactive, CTask* tsimplest)`, `ret 0xC`, SEH (`sub esp,0xA0`). String
  "CompPedCollPlayerResp" 0x85BA70 (LOOK2 = LOOK with this name, (P, pl)). P = m_Ped, pl = e->+0x14 (the player), R = 0.
- 1. If pl == 0 return without writing. `front = !((d.z*Pf.z + d.y*Pf.y) + d.x*Pf.x < 0)` with `d = plPos - Ppos`; `plFacing = (d·plf <= 0)`;
  `plMoving = pl->m_nMoveState (+0x534) in {4, 6, 7}`; `msP = (i16)e->+0x30`. Locals inPlayerGroup = 0, groupFix = 0.
- 2. f, w, gp, seek as above; `b = P->intel->FindTaskByType(0xF3)`: if b and `b->+0xC == 0` (group index 0 = the player's group):
  inPlayerGroup = 1; if `CPedGroups::ms_groups[0].m_groupMembership.CountMembers()` 0x5F6A50 (ECX 0xC09928) <= 2: RETURN WITHOUT writing
  m_EventResponseTask; else groupFix = 1.
- 3. `P->Say(0x1C, 0, 1.0f, 0, 0, 0)` (always).
- 4. If tactive and `tactive->GetTaskType() == 0x391`: if `pl->IsPlayer()`: SAY. Else if pl's active task (GetActiveTask 0x681720) exists
  and is 0x391: `r = GetRandomNumberInRange(0.0f, 1.0f)`; if `r < 0.25f`: `r2 = GetRandomNumberInRange(0.0f, 1.0f)`; `r2 < 0.33f
  [0x85AD74]` -> LOOK2; else if `r2 < 0.66 [0x85BA68]` (f64 constant compared on the x87) -> SAY. Then: if plFacing and `pl ==
  tactive->+0x10` -> GROUP; else if groupFix -> GROUP; else END.
- 5. Else (0x4BEAC3): MOVEBLOCK = { if f and seek and (w or gp) and inPlayerGroup and !plMoving: LEADERFIX with `if (w) w->+0x5C |= 2;
  else if (gp) gp->+0x18 = m + 0.1f` (else-if, unlike 0x4BDB80); else if f == 0 or !(inPlayerGroup and !plMoving): { ROUTE(tsimplest); when
  tsimplest is a go-to task (route attempted, even if the allocation failed) groupFix = 0 } (when f != 0 and inPlayerGroup and
  !plMoving the route is skipped); then SAY; LOOK2 }; EVADE2 = `v = (-plf.y, plf.x, 0)` (player's forward rotated by +90 degrees),
  `Normalise`, `R = (p = new(0x1C)) ? CTaskComplexEvasiveStep(p, pl, &v) : 0`, store R and return.
  - plFacing and front: msP not in {4,6,7}: SAY; LOOK2; groupFix ? GROUP : EVADE2. msP moving: MOVEBLOCK, then groupFix ? GROUP : END.
  - plFacing and !front: SAY; LOOK2; groupFix ? GROUP : EVADE2.
  - !plFacing and front: msP not moving: SAY; LOOK2; groupFix ? GROUP : EVADE2. msP moving: MOVEBLOCK (duplicate code at 0x4BEF2E),
    then groupFix ? GROUP : END.
  - !plFacing and !front: groupFix ? GROUP : END.
- GROUP (0x4BF12E): if f and seek and (w or gp) and inPlayerGroup and !plMoving: LEADERFIX (w |= 2, else gp radius) -> END. Else if
  inPlayerGroup and !plMoving -> END. Else: `v = (-plf.y, plf.x, 0)`, Normalise, R = EvasiveStep(pl, &v) (or 0) -> store and return.
- END: `m_EventResponseTask = R`. (No anger increment in this function.) The epilogue is reached through an obfuscated jump (0x40399B).
- gta-reversed: `.Reversed = false`. Confidence medium (as above).
