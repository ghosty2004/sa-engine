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

## 0x4B9FF0 CEventHandler::ComputeKnockOffBikeResponse  (catalogue: specified/REA, port level 26)
- `__thiscall void (this: CEventHandler*, CEvent* e [esp+4], CTask* tactive [+8], CTask* tsimplest [+0xC])`, `ret 0xC`; SEH frame
  (the `push handler` is obfuscated through the stub 0x40749C, no behaviour), `sub esp,0xC4`. tactive/tsimplest are unused.
  P = `this->m_Ped` (+0, re-read from `[esi]` at every use). Callers: ComputeVehicleDiedResponse 0x4BA8B0, ComputeVehicleOnFireResponse
  0x4BB2E0, ComputeDamageResponse 0x4C0170, ComputeEventResponseTask 0x4C3870.
- Locals: `K` = CEventKnockOffBike at esp+0x54 (default ctor 0x4AFC70 at 0x4BA017, dtor 0x4AFE70 at exit 0x4BA785); `ev` (ebp) = the
  knock-off event used below, initialised to 0 (0x4BA015).
- CEventKnockOffBike layout used (gr EventKnockOffBike.h, verified by the 11-arg ctor 0x4AFCF0): +0xC moveSpeed, +0x18 collision
  impact velocity, +0x24 damage intensity, +0x28 field_28, +0x2C time, +0x30 m_ped (the knocker), +0x34 direction (i8), +0x35 bit0
  isVictimDriver, +0x36 knockOffType (u8), +0x38 vehicle. CEventDamage (gr EventDamage.h, size 0x44): +0x1C weaponType, +0x24 direction,
  +0x38 m_damageResponse.
- 1. Event type (virtual slot +4 GetEventType, called again for each compare):
  - == 8 (EVENT_KNOCK_OFF_BIKE) (0x4BA031): `ev = e` (0x4BA056). Build local CEventDamage D (esp+0x90) = ctor 0x4AD830(source = e->+0x38,
    time = CTimer::m_snTimeInMilliseconds [0xB7CB84], weapon 0x36 (54 WEAPON_FALL), piece 3 (TORSO), direction 2, a7 = 1, inVehicle = 0).
    If `D.AffectsPed(P)` 0x4B35A0: CPedDamageResponseCalculator C (esp+0x34) ctor 0x4AD3F0(e->+0x38, damage = e->+0x28 (f32 bits passed
    through), 0x36, 3, true); `C.ComputeDamageResponse(P, &D.+0x38, true)` 0x4B5AC0; C dtor 0x4AD420. D dtor 0x4AD960 (~CEventDamage:
    vtable 0x85AD78, CleanUpOldReference on +0x14, base vtable 0x85AB38). Go to step 2.
  - == 9 (EVENT_DAMAGE) (0x4BA0DE): V = P->+0x58C (m_pVehicle, no null check). `ms = (V->+0x44*0.75, V->+0x48*0.75, V->+0x4C*0.75)` with
    [0x858F34]=0.75f; the x87 order is z first (0x4BA0EF), then y, then x, stored x,y,z. `dir = e->+0x24` (u8). `M` = P->+0x14 (matrix,
    no null check). Collision vector `cv`:
    dir 0: `(M.fwd.x*-1, M.fwd.y*-1, M.fwd.z*-1)` (fwd = +0x10..+0x18, multiplied by [0x858C1C]=-1.0f, not fchs; z,y,x load order);
    dir 1: `M.right` (+0..+8) copied; dir 2: `M.fwd` copied; any other value: `0x40FE90(&tmp, -1.0f, &M.right)` (tmp = -1.0f * right,
    component order z,y,x computed, stored x,y,z) and copied.
    Temporary T (esp+0x90) = CEventKnockOffBike 11-arg ctor 0x4AFCF0(V, &ms, &cv, 50.0f, 0.0f, knockOffType = (u8)e->+0x1C (the damage
    weapon type), direction = dir, time 0, ped 0, isVictimDriver 0, force 0). Go to COPY.
  - == 0x23 (EVENT_VEHICLE_DIED) or == 0x4F (EVENT_VEHICLE_ON_FIRE) (0x4BA20C/0x4BA218): ms as in the damage case (0x4BA221-0x4BA24E,
    same order). `dir = _ftol2(((rand() 0x821B1E & 0xFFFF) as i32 -> f) * [0x858B14]=3.0517578e-05f * [0x858B90]=4.0f)` (0x821B40,
    truncation; fild of the masked int, so dir is 0..3). cv as in the damage case but selected by `dir` (0, 1, 2, else -1*right).
    T = ctor 0x4AFCF0(V, &ms, &cv, 50.0f, 0.0f, knockOffType 0x33 (51, EXPLOSION), dir, 0, 0, 0, 0). Go to COPY.
  - any other type: jump to step 2 with ev = 0 -> null dereference at 0x4BA364 (original bug; callers only pass the 4 types above).
  - COPY (0x4BA342): `K = T` via 0x4B4AA0 (operator=, calls CEventKnockOffBike::From 0x4AFDD0); T dtor 0x4AFE70; `ev = &K`.
- 2. (0x4BA364) `veh = ev->+0x38`, `downTime = ev->+0x2C`. If veh == 0 -> EXIT. If P is not one of veh->+0x460 (driver), +0x464, +0x468,
  +0x46C (passengers 0-2) (checked in that order) -> EXIT.
- 3. `g_InterestingEvents.Add(16, P)` 0x602590 (this = 0xC0B058). If `!ev->SetPedSafePosition(P)` 0x4B4AC0: if `P->+0x46C & 0x100`
  (bInVehicle) then `P->+0x1C &= ~1` (CEntity flag bit 0, bUsesCollision) (0x4BA765-0x4BA772); EXIT.
- 4. `ev->SetPedOutCar(P)` 0x4AFF60; `anim = ev->CalcForcesAndAnims(P)` 0x4B0020; `kt = ev->+0x36`.
  - kt == 0x35 (53) (0x4BA3D6): `P->+0x46C &= ~1`; `&= ~2`; `|= 0x200` (bIsInTheAir); `P->+0x40 |= 0x100`; `P->+0x40 |= 0x8000000` (each
    a separate read-modify-write, P re-read). If `P->m_fHealth (+0x540) > 0.0f [0x858B50]` (fcomp; `test ah,0x41; jne` -> NaN, ==0 and <0
    take the die branch): `this->+0x24 = new(0x10) ? CTaskComplexInAirAndLand(0, 0) 0x678C80 : 0`; else `this->+0x24 = new(0x28) ?
    CTaskComplexDie(weapon = (i8)kt sign-extended, animGroup 0, anim, blendDelta 4.0f, animSpeed 0.0f, 0, 0, 0, 0) 0x630040 : 0`.
    Go to DRAG.
  - else if NOT (`P->+0x540 > 0.0f` or unordered) (0x4BA4B4: `test ah,0x41; jp` -> the jump to ALIVE is taken for >0 and NaN):
    DEAD: if anim == 0xBF: switch on `(i8)ev->+0x34` via jump table 0x4BA7A8 (unsigned compare with 3, so negatives keep anim):
    0 -> anim = 0x6A; 1, 2 -> anim = 0x19; 3 -> anim = 0x1B. `this->+0x24 = new(0x28) ? CTaskComplexDie((i8)kt, 0, anim, 4.0f, 0.0f, 0, 0,
    0, 0) : 0`; `P->+0x46C &= ~1`; go to BIKEFLAG.
  - ALIVE (0x4BA557): if kt == 0 go to DRAG (no response task written). Else `this->+0x24 = new(0x18) ? CTaskComplexFallAndGetUp(anim,
    0, downTime) 0x6786C0 : 0`. `g = CPedGroups::GetPedsGroup(P)` 0x5F7E80; if g: `m = g+8` (membership):
    - if `m.IsLeader(P)` 0x5F69C0 or `ev->+0x35 & 1`: if `m.IsLeader(P)` (called again): `le = CEvent::new(0x14) ?
      CEventLeaderExitedCarAsDriver() 0x4B8300 : 0`; local CEventGroupEvent G(P, le) 0x4ADFD0; `(g+0x30)->AddEvent(&G)` 0x5F7470
      (CPedGroupIntelligence); G dtor 0x4AE070.
    - else `L = m.GetLeader()` 0x5F69A0; if L and `L->+0x46C & 0x100` and `L->m_pIntelligence(+0x47C)->+0x68.GetEventOfType(8)` 0x4AB650
      == 0: `seq = new(0x40) ? CTaskComplexSequence() 0x632BD0 : 0`; `seq->AddTask(this->+0x24)` 0x632D10; `seq->AddTask(new(0x50) ?
      CTaskComplexEnterCarAsPassenger(veh, 0, true) 0x640340 : 0)`; `this->+0x24 = seq`. (If the 0x40 allocation fails AddTask runs
      with this = 0: original bug, unreachable in practice.)
    Then (also when g == 0) `P->+0x46C &= ~1` (0x4BA6F8). Go to DRAG.
  - DRAG (0x4BA701): `k = ev->+0x30`; if k == 0 go to BIKEFLAG. Else local CEventDraggedOutCar E(veh, k, ev->+0x35 & 1) 0x4AD250;
    `P->m_pIntelligence(+0x47C)->m_eventGroup(+0x68).Add(&E, true)` 0x4AB420; if `veh->+0x590 == 9` (bike) `veh->+0x614 |= 0x10`;
    E dtor 0x4AD330; EXIT.
  - BIKEFLAG (0x4BA53E): if `veh->+0x590 == 9` `veh->+0x614 &= 0xEF`; EXIT.
- EXIT (0x4BA776): K dtor 0x4AFE70, restore SEH, `ret 0xC`.
- Allocations: CTask::operator new 0x61A5A0 (cdecl, size pushed), CEvent::operator new 0x4B5620.
- Notes: in the EVENT_KNOCK_OFF_BIKE case the damage is applied before the vehicle/occupant checks and `ev` is the caller's event (not
  the copy K); K stays default-constructed and is destroyed at exit.
- gta-reversed: plugin::CallMethod stub only (EventHandler.cpp:1469).
- Open questions: the meaning of P->+0x40 bits 0x100 / 0x8000000 (CPhysical flag word; gr bitfield names not verified) - behaviour is
  fully determined by the masks.
