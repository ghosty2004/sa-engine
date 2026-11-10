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
