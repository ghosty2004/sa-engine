# CEventScanner (AI) - RE-SPEC-3 group I3

Layout (gr EventScanner.h, size 0xD4; CTaskTimer = {+0 m_nStartTime u32, +4 m_nInterval i32, +8 m_bStarted u8, +9 m_bStopped u8}, 0xC):
+0 m_nNextScanTime (u32), +4 m_vehiclePotentialCollisionScanner (timer), +0x10 m_objectPotentialCollisionScanner (timer),
+0x1C m_attractorScanner (CAttractorScanner, 0x90: +0 field_0 u8, +4 CTaskTimer, +0x10 m_pEffectInUse, +0x14 field_14, three 10-int arrays),
+0xAC m_pedAcquaintanceScanner (timer + flags +0xB8/+0xB9/+0xBA), +0xBC m_sexyPedScanner (timer), +0xC8 m_nearbyFireScanner (timer).
Embedded in CPedIntelligence (ctor caller 0x607140). Static `m_sDeadPedWalkingTimer` 0xC0B038 (u32).
CPed fields used: +0x22 m_nModelIndex (i16), +0x46C ped flags dword (bit0 bIsStanding, bit8 bInVehicle, bit9 bIsInTheAir), +0x47C m_pIntelligence,
+0x540 m_fHealth (f32), +0x584 m_pContactEntity, +0x58C m_pVehicle, +0x730 m_pFire (plugin-sdk CPed.h offsets). CPedIntelligence: +4 m_TaskMgr,
+0x38 m_eventHandler history, +0x68 m_eventGroup, +0xE0 vehicle-scanner m_apEntities[16], +0x130 ped-scanner m_apEntities[16],
+0x170 ped-scanner m_pClosestEntityInRange, +0x174 m_mentalState.

## 0x605300 CEventScanner::CEventScanner  (catalogue: specified/REA, port level 1)
- `__thiscall CEventScanner* (this)`, plain `ret`, returns this. Called once from CPedIntelligence ctor 0x607140.
- Behaviour (stores in this order, all with 0 unless noted, 0x605307-0x605392):
  vehicle timer +4, +8 (dwords), +0xC, +0xD (bytes); object timer +0x10, +0x14, +0x18, +0x19; attractor scanner: +0x1C byte = 1, +0x20, +0x24,
  +0x28, +0x29, +0x2C, +0x30; then `CAttractorScanner::Clear(this + 0x1C)` 0x5FFF90 (0x605338); acquaintance scanner +0xAC, +0xB0, +0xB4, +0xB5,
  +0xB8 byte = 1, +0xB9 = 0, +0xBA = 0; sexy-ped timer +0xBC, +0xC0, +0xC4, +0xC5; fire timer +0xC8, +0xCC, +0xD0, +0xD1.
  Then `now = CTimer::m_snTimeInMilliseconds` [0xB7CB84] (read BEFORE rand, 0x605398); `r = rand() & 0xFFFF` (_rand 0x821B1E);
  x87: `fild r; fmul 3.0517578e-05f [0x858B14]; fmul -3000.0f [0x859A90]` (two f32 multiplies, 24-bit PC), `k = _ftol(...)` 0x821B40 (truncation,
  k in -2999..0); `m_nNextScanTime = now - k` (0x6053C1-0x6053C3), i.e. now + 0..2999.
- gta-reversed (EventScanner.cpp:7): `GetTimeInMS() + CGeneral::GetRandomNumberInRange(3000u)` - DIFFERS (gr helper formula, finding 1 of
  docs/re/cat14-gr-deviations.md; the original's inline formula is `-trunc(r/32768 * -3000)`). Port the original. The gr comment
  "Originally should be -3000.0f" describes exactly this inline constant.

## 0x6053D0 CEventScanner::ScanForEventsNow  (catalogue: specified/REA, port level 10)
- `__thiscall void (this, const CPed* ped [esp+4], bool bDontScan [esp+8])`, `ret 8`. Callers: CTaskSimpleGoToPoint::ProcessPed 0x66D710 (2 sites).
- Behaviour: `intel = ped->m_pIntelligence` (+0x47C, loaded first, 0x6053D4). If the FULL dword at [esp+8] != 0 return (`mov esi,[esp+0xC];
  test esi,esi`, 0x6053DB-0x6053E1: the callers' upper bytes matter; MSVC pushes a zero-extended value).
  Timer = this+4: if `m_bStarted` (+0xC byte) != 0 then `m_nStartTime (+4) = now [0xB7CB84]`, `m_nInterval (+8) = -1`, `m_bStarted = 1`
  (0x6053E7-0x6053FE; restart only an already-started timer).
  Then `CVehiclePotentialCollisionScanner::ScanForVehiclePotentialCollisionEvents(this+4, ped, intel + 0xE0, 16)` 0x603720 (thiscall, `ret 0xC`).
- gta-reversed: EventScanner.cpp:19 - same logic (the `todo: inlined?` comment is not a behavioural difference).

## 0x607E30 CEventScanner::ScanForEvents  (catalogue: specified/REA, port level 16)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. SEH frame (state var [esp+0x7C] after pushes), locals: one event buffer at [esp+0x10]
  (reused for every event) and a CEventDamage at [esp+0x30]. Caller: CPedIntelligence::Process 0x608260.
- Callees: 0x603720 vehicle-potential-collision scan; 0x606580 (unnamed; proposed CPedPotentialCollisionScanner::ScanForPedPotentialCollisionEvents,
  ECX = address of this function's own `ped` argument slot, i.e. a stateless dummy object, 0x607E83); 0x606890 (proposed
  CObjectPotentialCollisionScanner::ScanForObjectPotentialCollisionEvents); 0x607D80 (proposed CPedAcquaintanceScanner::FindClosestAcquaintance);
  0x6060A0 (proposed CAttractorScanner::ScanForAttractorsInRange); 0x603E70 (proposed CNearbyFireScanner::ScanForNearbyFires); 0x603BF0 (proposed
  CSexyPedScanner::ScanForSexyPedEvents) - names proposed from the `this` member they receive; CMentalState::Process 0x6008A0;
  CPedGeometryAnalyser::IsInAir 0x5F1CB0; CEventInAir ctor 0x4B0CB0 / dtor 0x4B0D00; CEventGroup::Add 0x4AB420 (thiscall (event*, bool));
  CEventHandlerHistory::GetCurrentEvent 0x4B8CA0; CEvent::CEvent 0x4ABFC0 / ~CEvent 0x4AC010; CPed::IsPlayer 0x5DF8F0;
  CTaskManager::GetSimplestActiveTask 0x6819D0; CWeapon::GenerateDamageEvent 0x73A530 (cdecl, 6 args); CTaskManager::FindTaskByType 0x6817D0
  (thiscall (slot, type)); CEventAreaCodes ctor 0x4B2190 / dtor 0x4B2210; CEventPedEnteredMyVehicle ctor 0x4AEAC0 (vtbl 0x85AFE8, type 17;
  args ped, vehicle, int; registers BOTH references without null checks) / dtor 0x4AEB90; CWorld::FindPlayerPed 0x56E210; CPed::IsAlive 0x5E0170;
  CEventDamage ctor 0x4AD830 / dtor 0x4AD960; CEventGroup::HasEventOfType 0x4AB5E0.
- Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84 (R), MI_ESCALATORSTEP 0x8CD674 / MI_ESCALATORSTEP8 0x8CD678 (u16, R),
  m_sDeadPedWalkingTimer 0xC0B038 (R/W), 0.0f [0x858B50].
- Behaviour:
  1. Gate (0x607E3E-0x607E53): `if (now <= m_nNextScanTime) return` (unsigned `jbe`). m_nNextScanTime is NOT updated here (after the initial
     random delay set by the ctor the scanner runs every call).
  2. Scanners in this order (0x607E63-0x607EC9), `intel = ped->m_pIntelligence`: 0x603720(this+4; ped, intel+0xE0, 16);
     0x606580(ECX = &arg slot; ped, intel->[+0x170]); 0x606890(this+0x10; ped); 0x607D80(this+0xAC; ped, intel+0x130, 16);
     0x6060A0(this+0x1C; ped); 0x603E70(this+0xC8; ped); CMentalState::Process(intel+0x174; ped).
  3. In-air (0x607ECE-0x607F50): if `!(flags & 1)` and (`flags & 0x200` or `CPedGeometryAnalyser::IsInAir(ped)` (cdecl)): build CEventInAir in the
     buffer, `intel->m_eventGroup.Add(&ev, false)` (0x607F0D), destroy (0x4B0D00), go to 4. Otherwise, if `flags & 0x200`: if
     `history(intel+0x38).GetCurrentEvent()` is null or its GetEventType() (vtbl+4) != 0x21 (EVENT_IN_AIR) then `flags &= ~0x200` (0x607F50).
     GetCurrentEvent is called twice (0x607F33, 0x607F3F). Flags are re-read from memory before each test.
  4. On fire (0x607F5A-0x607FEB): if `ped->m_pFire` (+0x730) != 0: CEvent::CEvent(buffer), vtbl = 0x86CCC0 (CEventOnFire). If `!ped->IsPlayer()`
     and `t = GetSimplestActiveTask(intel+4)` != 0 and `!t->MakeAbortable(ped, 1, &ev)` (vtbl+0x18, 0x607FA4): call
     `CWeapon::GenerateDamageEvent(ped, ped->m_pFire->m_pEntityCreator (+0x14), 37 (0x25, flamethrower), 5, 3 (torso), 0)` and do NOT add the
     event; otherwise (player, no simplest task, or abort succeeded) `intel->m_eventGroup.Add(&ev, false)` (0x607FD2). Then vtbl = 0x86CCC0 and
     ~CEvent 0x4AC010 (0x607FE3-0x607FEB).
  5. `0x603BF0(this+0xBC; ped, intel+0x130, 16)` (0x607FF0-0x607FFA) - runs every time (after the fire block).
  6. Area codes (0x607FFF-0x6080AD): `k = FindTaskByType(intel+4, slot, 1000 TASK_COMPLEX_KILL_PED_ON_FOOT)` trying slots 4, 3, 1, 2 in that
     order, first non-null wins. If found: `target = k->[+0x10]` (the victim ped); if target != 0 and target->m_pContactEntity (+0x584) != 0 and
     ped->m_pContactEntity != 0 and `target->m_pContactEntity->m_nAreaCode (+0x2F byte) != ped->m_pContactEntity->m_nAreaCode`:
     CEventAreaCodes ctor(buffer, target) 0x4B2190, Add(&ev, false) (re-reading ped->m_pIntelligence, 0x60807D), dtor 0x4B2210.
     If the first found task has a null target the remaining slots are NOT tried.
  7. Escalator (0x6080B2-0x60810F): if ped->m_pContactEntity != 0 and (`(u16)MI_ESCALATORSTEP == (i32)(i16)contact->m_nModelIndex` or the same with
     MI_ESCALATORSTEP8) (zero-extended MI vs sign-extended model: an unset MI 0xFFFF never matches model -1): CEvent::CEvent(buffer), vtbl = 0x86CD00
     (CEventEscalator), Add(&ev, false), ~CEvent.
  8. Boat passenger (0x608114-0x60819F): if `flags & 0x100` (bInVehicle) and `v = ped->m_pVehicle` != 0 and `v->m_nVehicleClass (+0x590) == 5`
     (boat) and `!ped->IsPlayer()` and `FindPlayerPed(-1)->m_pContactEntity (+0x584) == v`: CEventPedEnteredMyVehicle(buffer, FindPlayerPed(-1),
     v, 10) (FindPlayerPed called again, 0x60815E), then `ev.m_TaskId (+0xE i16) = 0x2C2` (706 TASK_COMPLEX_LEAVE_CAR_AND_FLEE, written after the
     Add arguments are pushed but before the call, 0x60818B), Add(&ev, false), dtor 0x4AEB90.
  9. Dead ped walking (0x6081A4-0x608239): `fld m_fHealth; fcomp 0.0f [0x858B50]; test ah,0x41; jp skip` -> continue only if health <= 0.0f
     (NaN skips). Then if `ped->IsAlive()` and `now > m_sDeadPedWalkingTimer` (unsigned): construct CEventDamage(buffer2 [esp+0x30], source 0,
     startTime 0, weapon 0x37 (55), piece 3, direction 0, a7 0, inVehicle 0) 0x4AD830; if `!intel->m_eventGroup.HasEventOfType(&dmg)` (0x608203):
     `CWeapon::GenerateDamageEvent(ped, 0, 0x36 (54, fall), 10, 3, 0)` and `m_sDeadPedWalkingTimer = now + 2000` (now re-read, 0x60821C-0x60822B).
     Destroy dmg (0x4AD960).
- Edge cases: the in-air "else" branch can only clear bit 9 when the ped is standing or the bit is clear (it is gated by `flags & 0x200`).
  The timer global is shared by all peds (only one dead-walking damage per 2 s game-wide).
- gta-reversed: `plugin::CallMethod<0x607E30>` (EventScanner.cpp:16) - not reversed; this spec is from the bytes.
- Open questions: exact names/semantics of the unnamed scanner callees 0x606580, 0x606890, 0x607D80, 0x6060A0, 0x603E70, 0x603BF0 (call sites
  0x607E8A, 0x607E93, 0x607EA8, 0x607EB1, 0x607EBD, 0x607FFA); they are ported as `orig!` calls, so only their arguments matter here.

## 0x603570 CEventScanner::sub_603570  (catalogue: unknown/REA, port level 10)
Catalogue name: CEventScanner::sub_603570, proposed: `CAttractorScanner::GetClosestPedForScriptedEffect(C2dEffect* fx)` (static; evidence: only
caller 0x6060A0 compares its result with the scanning ped before raising CEventScriptedAttractor; it reads ms_userLists of the effect).
- Convention: `__cdecl CPed* (C2dEffect* fx [esp+4])`, plain `ret`, result EAX (0 if none). SEH frame (handler 0x83E878), local CMatrix.
- Callees: CTaskManager::GetActiveTask 0x681720; CScripted2dEffects::GetIndex 0x6F9F60; CMatrix::SetScale(float) 0x59AED0;
  CPedAttractorManager::IsApproachable 0x5EA220 (cdecl (fx, const CMatrix&, 0, ped)); CMatrix::~CMatrix 0x59ACD0.
- Globals: CPools::ms_pPedPool [0xB74490]=0 (R; pool +0 objects, +4 flags bytes, +8 size); ms_userLists 0xC3A200 (+0x20 at 0xC3A220).
- Behaviour:
  1. `best = FLT_MAX (0x7F7FFFFF)`, `bestPed = 0`. If pool size == 0 return 0.
  2. Iterate slots DOWNWARD: `for (i = size-1; i >= 0; i--)`; skip if `flags[i] & 0x80` (free); `ped = objects + i*0x7C4` (skip if that is 0).
  3. Skip if `t = GetActiveTask(ped->intel+4)` is non-null and `t->GetTaskType() == 0xE9` (TASK_COMPLEX_USE_EFFECT).
  4. `p = ped->m_matrix ? matrix->pos : placement.pos`; x87: `dx = fx.x - p.x; dy = fx.y - p.y; dz = fx.z - p.z` (effect minus ped);
     `s = dz*dz; s += dy*dy; s += dx*dx`, stored f32 (0x60363B), reloaded and compared with `best`: continue only if `s < best`
     (`test ah,5; jp skip`; NaN skips).
  5. `ul = &ms_userLists[GetIndex(fx)]`: eligible if `ul->m_bUseList == 0`, or `(i32)(i16)ped->m_nModelIndex` equals one of `m_UserTypes[0..3]`,
     or (m_bUseList re-tested, nonzero) some k in 0..3 has `m_UserTypes[k] == -2 && m_UserTypesByPedType[k] == ped->m_nPedType (+0x598)`
     (inlined copy of 0x5FE960). Not eligible -> next ped.
  6. Local CMatrix (+0x40/+0x44 zeroed), `SetScale(1.0f)` (identity); `if (IsApproachable(fx, mat, 0, ped)) { best = s; bestPed = ped; }`;
     destroy matrix.
  7. Return bestPed. Ties: a later (lower index) ped with exactly the same distance does not replace (strict `<`).
- Note: the effect position is used as-is (scripted effects are world-space; no entity transform).
- gta-reversed: absent. Open questions: none.

## 0x603BF0 CEventScanner::sub_603BF0  (catalogue: unknown/REA, port level 9)
Catalogue name: CEventScanner::sub_603BF0, proposed: `CSexyPedScanner::ScanForSexyPedEvents(const CPed* ped, CEntity** peds, int32 count)`
(evidence: `this` = CEventScanner+0xBC m_sexyPedScanner (call 0x607FFA); raises vtable 0x85B0B0 whose GetEventType 0x4AEE60 returns 20 =
EVENT_SEXY_PED; gr InterestingEvents.h:25 attributes interesting-event 9 to "CSexyPedScanner::ScanForSexyPedEvents").
- Convention: `__thiscall void (CTaskTimer* this, CPed* ped [esp+4], CPed** list [esp+8], int32 count [esp+0xC])`, `ret 0xC`. SEH frame
  (handler 0x83E8B8). Caller: CEventScanner::ScanForEvents 0x607E30 with (ped, intel+0x130 ped-scanner entities, 16).
- Callees: CInterestingEvents::Add 0x602590 (thiscall on g_InterestingEvents 0xC0B058, (type, CEntity*)); CWorld::GetIsLineOfSightClear
  0x56A490 (cdecl, 9 args); 0x4AEDF0 CEventSexyPed ctor (thiscall (CPed*), `ret 4`); CEventGroup::Add 0x4AB420; 0x4AEEA0 CEventSexyPed dtor.
- Globals: CTimer::m_snTimeInMilliseconds [0xB7CB84]=0; 10000.0f [0x859AA4]=10000.0f; 0.0f [0x858B50]=0.0f.
- Fields: this = CTaskTimer {+0 start, +4 interval, +8 bStarted, +9 bStopped}. ped/other: +0x14 m_matrix (forward = matrix+0x10),
  +4 placement pos, +0x46C flags (bit 8 bInVehicle), +0x47C intelligence (+0x68 event group), +0x480 m_pPlayerData, +0x484 m_nCreatedBy
  (u8, 2 = mission), +0x598 m_nPedType (i32), +0x59C m_pStats (CPedStat*, +0x27 sexiness, signed i8).
- Behaviour:
  1. Timer (0x603C0B-0x603C69): if `!bStarted` { start = now; bStarted = 1; interval = 500 (0x1F4) } (bStarted re-tested, always 1).
     If `bStopped` { start = now; bStopped = 0 }. Fresh `now` (0x603C4A): if `now < start + interval` (unsigned) return. Else start = now,
     interval = 500, bStarted = 1.
  2. If `ped->m_nCreatedBy != 2` and `ped->flags & 0x100` (in vehicle) return. (Mission peds scan even inside vehicles.)
  3. `bestDist = 1.0e10f (0x501502F9)`, `best = 0`; `myPos` = copy of (matrix ? matrix->pos : placement.pos). If `count <= 0` return.
  4. For `i = 0 .. count-1` (0x603CC2-0x603DFB), `o = list[i]`, skip unless all of: `o != 0`; `o->m_nPedType == 5` (PED_TYPE_CIVFEMALE);
     `(i8)o->m_pStats->sexiness > (i8)ped->m_pStats->sexiness` (signed `jle` skip); `!(o->flags & 0x100)`.
     - `oPos` = copy of o's position; `dx = oPos.x - myPos.x` -> f32 local, `dy` -> f32, `dz` -> f32 (`fst`); `d2 = dz*dz` (dz from memory),
       `d2 += dy*dy; d2 += dx*dx` (f32 locals); `fst` d2 -> f32 (the `ped` arg slot). `fcomp 10000.0f; test ah,5; jp next` -> need `d2 < 10000`.
     - If `ped->m_pPlayerData != 0` (player): `g_InterestingEvents.Add(9, o)` (0x603D87) - before the best/visibility tests.
     - `if !(d2_f32 < bestDist) next` (strict; d2 reloaded from the f32 slot).
     - `fwd = ped->m_matrix + 0x10` (no null check, 0x603D9B): `dot = dz*fwd.z; dot += dy*fwd.y; dot += dx*fwd.x` (f32 locals);
       `fcomp 0.0f; test ah,0x41; jne next` -> need `dot > 0` (in front; NaN rejects).
     - `if GetIsLineOfSightClear(myPos, oPos, buildings 1, vehicles 0, peds 0, objects 1, dummies 0, seeThrough 0, ignoreSomeObjects 0)`:
       `best = o; bestDist = d2` (dword copy).
  5. If `best != 0` (0x603E05): CEventSexyPed ev(best) (0x4AEDF0), `ped->m_pIntelligence->m_eventGroup.Add(&ev, false)` (result ignored);
     then timer start = now (fresh), interval = 3000 (0xBB8), bStarted = 1 (0x603E31-0x603E44); destroy ev (0x4AEEA0).
- Edge cases: the forward-vector read dereferences `ped->m_matrix` without the null check used for the position. The interesting event is
  added for every candidate within 100 m regardless of visibility.
- gta-reversed: absent (CSexyPedScanner has only a header). Spec from the bytes.
- Open questions: none.

## 0x606580 CEventScanner::sub_606580  (catalogue: unknown/REA, port level 9)
Catalogue name: CEventScanner::sub_606580, proposed: `CPedPotentialCollisionScanner::ScanForPedPotentialCollisionEvents(const CPed* ped,
CPed* closestPed)` (evidence: raises CEventPotentialWalkIntoPed (ctor 0x4AE6E0) for the closest ped blocking the go-to line; `this` is unused -
the caller 0x607E83 passes the address of its own argument slot as a stateless object).
- Convention: `__thiscall void (void* this /*unused*/, CPed* ped [esp+4], CEntity* closest [esp+8])`, `ret 8`. SEH frame (handler 0x83E9D8).
  Locals (E = esp at entry): bestPed [E-0x78], &target [E-0x74], intel then later sqLen1 [E-0x70], diff/v1 CVector [E-0x6C], v2 CVector
  [E-0x60], out1 [E-0x54], CColSphere [E-0x48], event [E-0x34], SEH state [E-4]; arg slots reused: [E+4] = loop index i then sqLen2+0.25,
  [E+8] = bestSq.
- Callees: CTaskManager::GetSimplestActiveTask 0x6819D0; CTask::IsGoToTask 0x61A360; CPedIntelligence::GetMoveStateFromGoToTask 0x601D70
  (simplest task if go-to: task+8, else 1); CPed::IsAlive 0x5E0170; CVector::Difference 0x40FE00 (thiscall out = a - b, `ret 8`);
  CColSphere::Set 0x40FD10 (radius, center, surface, piece, lighting); CColSphere::IntersectEdge 0x4100E0 (start, end, out1, out2) -> AL;
  0x406DA0 CVector::SquaredMagnitude (thiscall, ST0 = (x*x + y*y) + z*z); CVector::DotProduct 0x40FDB0 (cdecl (a*, b*): ST0 = a.z*b.z +
  a.y*b.y + a.x*b.x in that order); 0x40FEC0 (cdecl out = v * s, components computed z, y, x); approxEqual 0x4EEA80; CEventPotentialWalkIntoPed
  ctor 0x4AE6E0 (thiscall (CPed* other, const CVector& target, int32 moveState)) / dtor 0x4AE7A0; CEventGroup::Add 0x4AB420.
- Globals: 2.5f [0x86C8C0]=2.5f; 0.0f [0x858B50]=0.0f; 0.923f [0x86CD98]=0.923f; 0.25f [0x86CD94]=0.25f; 1.0f [0x86CD90]=1.0f; immediates
  0.7f (sphere radius, 0x6066BB), 50.0f (0x60678F, 0x6067A2), 0.01f (0x606806).
- Behaviour:
  1. Return if `closest == 0` (0x60659C; the second argument only gates, it is not used afterwards); if `!(ped->m_nFlags (+0x1C) & 1)`
     (bUsesCollision); if `st = GetSimplestActiveTask(intel+4)` is 0 or `!IsGoToTask(st)`; if `GetMoveStateFromGoToTask(intel) == 1`
     (PEDMOVE_STILL).
  2. `target = &st->+0xC` (go-to target point), `bestSq = 2.5f*2.5f` (x87 product stored f32), `best = 0`.
  3. For `i = 0..15` (0x606630-0x60672D), `o = intel->m_pedScanner.m_apEntities[i]` (intel+0x130+4i, intel re-read from the local): skip
     if `o == 0`, `!o->IsAlive()`, or `!(o->m_nFlags & 1)`.
     - `pp = ped->m_matrix ? &m->pos : &ped->placement.pos`, `op` likewise; `diff = op - pp` (Difference(op, pp)).
     - `M = ped->m_matrix` (used WITHOUT a null check even when pp came from placement): `dot = diff.y*M.fwd.y` (+0x14) `+ diff.z*M.fwd.z`
       (+0x18) `+ diff.x*M.fwd.x` (+0x10) in that order; need `dot > 0` (`test ah,0x41; jne skip`; NaN skips).
     - `sphere.Set(0.7f, op, 0, 0, 0xFF)`; `if (!sphere.IntersectEdge(pp_fresh, *target, out1, out2)) skip` (pp re-derived, 0x6066C9).
     - `q = diff.SquaredMagnitude()`; `fcom bestSq; test ah,5; jp` -> if `q < bestSq` { bestSq = q (f32); best = o } else pop.
  4. If `best == 0` return.
  5. Same-direction filter (0x606741-0x60682A): `bt = GetSimplestActiveTask(best->intel+4)`; if `bt && IsGoToTask(bt)`:
     `d = DotProduct(&ped->m_matrix->fwd, &best->m_matrix->fwd)`; `fcomp 0.923f; test ah,1; jne raise` (d < 0.923 or NaN -> raise).
     Else: `v1 = ped->m_vecMoveSpeed (+0x44) * 50.0f`, `v2 = best->m_vecMoveSpeed * 50.0f` (0x40FEC0), then `v1.z = 0; v2.z = 0`;
     `s1 = v1.SquaredMagnitude()` -> f32; `s2 = v2.SquaredMagnitude() + 0.25f` -> f32. If `!(bestSq > 1.0f)` (`test ah,0x41; jne`; NaN ->
     raise) raise. If `approxEqual(s2, 0.0f, 0.01f)` raise. If `s2 > s1` (`fld s2; fcomp s1; test ah,0x41; je return`) return without an event
     (the other ped walks the same way and is faster); otherwise raise. (Note: s1/s2 are squared speeds; the +0.25 is added to a squared value.)
     If `bt` is 0 or not a go-to task: raise.
  6. Raise (0x60682C-0x606873): `ms = GetMoveStateFromGoToTask(ped->intel)`; `CEventPotentialWalkIntoPed ev(best, *target, ms)`;
     `ped->m_pIntelligence->m_eventGroup.Add(&ev, false)`; destroy (0x4AE7A0).
- Edge cases: possible null dereference of `ped->m_matrix` / `best->m_matrix` (0x606681, 0x606768) for peds without a matrix (never the case
  for live peds). `approxEqual(s2, 0, 0.01)` can only be true for negative-ish values since s2 >= 0.25: effectively dead, keep it.
- gta-reversed: absent. Open questions: none.

## 0x607560 CEventScanner::sub_607560  (catalogue: unknown/REA, port level 10)
Catalogue name: CEventScanner::sub_607560, proposed: `CPedAcquaintanceScanner::ScanAcquaintanceTypes(CPed* ped, int32 acqType, int32 prevResult,
CPed* other, CPed** outPed, int32* outType)` (evidence: only caller 0x607A90 passes its `this` (acquaintance scanner) and its out pointers;
`this` is used as a CTaskTimer).
- Convention: `__thiscall int32 (CTaskTimer* this, CPed* ped [esp+4], int32 acqType [esp+8], int32 prevResult [esp+0xC], CPed* other [esp+0x10],
  CPed** outPed [esp+0x14], int32* outType [esp+0x18])`, `ret 0x18`, result EAX. The `other` arg slot is reused as the f32 `lit` local.
- Callees: CPedIntelligence::CanSeeEntityWithLights 0x605550 (thiscall (CEntity*, int32 0) -> ST0 f32); CAcquaintance::GetAcquaintances 0x608970;
  CPedType::GetPedFlag 0x608830; CGameLogic::LaRiotsActiveHere 0x441C10; 0x603AF0; 0x606BA0 (raise acquaintance event, thiscall (ped, type,
  other) -> AL).
- Globals: 0.0f [0x858B50]=0.0f; 0.0 (f64) [0x859EF8]=0.0; interval words [0x8D235C]=3000 and [0x8D2360]=200 (.data i32, R); now [0xB7CB84]=0.
- Behaviour:
  1. `lit = ped->m_pIntelligence->CanSeeEntityWithLights(other, 0)` -> f32 (0x60757C).
  2. `for (t = 4; t >= 0; t--)` (`dec; jns`, 0x607683):
     - if `t == prevResult` return -1 (0x607589; checked first in every iteration).
     - if `acqType == -1`: if `t == 2 && other->m_nPedType == 6` -> go to LIT (skips the mask test). Else MASK.
       if `acqType != -1`: if `acqType != t` continue; else MASK.
     - MASK: `if (ped->m_acquaintance (+0x4E0).GetAcquaintances(t) & GetPedFlag(other->m_nPedType))` -> LIT; else if `LaRiotsActiveHere() &&
       0x603AF0(ped, other)` -> LIT; else continue.
     - LIT: `fld lit; fcomp 0.0f; test ah,0x41; je ACCEPT` (lit > 0). Else if `t != 4` continue; else `fcomp 0.0f; test ah,0x44; jnp continue`
       -> continue only when lit == 0; lit < 0 or NaN -> ACCEPT.
     - ACCEPT: `*outPed = other; *outType = t`. If `acqType != -1` return acqType (EAX = ecx = acqType, 0x607697). Else if `*outPed == 0` continue;
       `ok = 0x606BA0(this, ped, t, *outPed)`; interval = (`*outType == 4` and `lit < 0.0` vs the f64 constant: `fcomp qword; test ah,5; jnp`)
       ? [0x8D2360] (200) : [0x8D235C] (3000); timer: start = now, interval, bStarted = 1 (always, before testing ok); if `ok` return
       `*outType`; else continue.
  3. Loop exhausted: return -1.
- gta-reversed: absent. Open questions: names of the .data words 0x8D235C/0x8D2360 (gr has none; values 3000/200 from the image).
