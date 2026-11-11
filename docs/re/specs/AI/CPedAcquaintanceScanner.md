# CPedAcquaintanceScanner (AI) - RE-SPEC-FINAL group A

## 0x603AF0 CPedAcquaintanceScanner::sub_603AF0  (catalogue: unknown/REA, port level 2)
Catalogue name: CPedAcquaintanceScanner::sub_603AF0, proposed: `CPedAcquaintanceScanner::IsRiotTarget(CPed* ped, CPed* other)` (static; evidence:
every caller (0x606CCD, 0x6075ED, 0x607BEE, 0x607C2E) calls it only after CGameLogic::LaRiotsActiveHere() returned true, and a true result
makes `other` hated).
- Convention: `__cdecl bool (CPed* ped [esp+4], CPed* other [esp+8])`, plain `ret`, AL (some paths set the whole EAX to 0/1).
  Entry is split: 0x603AF7 jumps to a HOODLUM chunk 0x406E8F (`pt = ped->m_nPedType` (+0x598)) which jumps back to 0x603AFC.
- Callees: CPed::IsPlayer 0x5DF8F0; CPedIntelligence::Respects 0x601C90 (thiscall (CPed*) -> AL); CPedGroups::GetPedsGroup 0x5F7E80 (cdecl);
  CPedGroupMembership::GetLeader 0x5F69A0; 0x5FE9C0 (IsGangPedType).
- Behaviour: return false if `pt == 6` (COP), `pt == 0x12` (18 MEDIC), `pt == 0x13` (19 FIREMAN), `ped->IsPlayer()`, or
  `ped->m_nCreatedBy (+0x484) == 2` (mission ped).
  If `other->IsPlayer()`: return `!ped->intel->Respects(other)` (`neg al; sbb eax,eax; inc eax`).
  `g = GetPedsGroup(other)`; if `g` and `g->membership (+8).GetLeader()` and that leader `IsPlayer()` (GetLeader called three times):
  return `!ped->intel->Respects(leader)`.
  Else if `IsGangPedType(pt)` and `IsGangPedType(ot = other->m_nPedType)` and `pt == ot` return 0 (same gang). Else return 1.
- gta-reversed: absent. Open questions: none.

## 0x606BA0 CPedAcquaintanceScanner::sub_606BA0  (catalogue: unknown/REA, port level 9)
Catalogue name: CPedAcquaintanceScanner::sub_606BA0, proposed: `CPedAcquaintanceScanner::AddAcquaintanceEvent(CPed* ped, int32 acqType,
CPed* other)` (evidence: switch on the acquaintance type building the matching CEventAcquaintancePed* event; caller 0x607560).
- Convention: `__thiscall bool (this /*unused*/, CPed* ped [esp+4], int32 acqType [esp+8], CPed* other [esp+0xC])`, `ret 0xC`, AL = event
  accepted. SEH frame (handler 0x83EA48), event buffer A [E-0x4C], buffer B [E-0x34], zero CVector [E-0x4C] (overlaps A, used only on the badly-lit path).
- Callees: CEventAcquaintancePed ctor 0x4AF820 (thiscall (CPed*), sets base vtable 0x85B250, registers the reference at +0x14) and its dtor
  0x4AF890; CEventAcquaintancePedHate ctor 0x420E70 (HOODLUM stub -> 0x1561070: base ctor then vtable 0x858E68); CEventGroup::Add 0x4AB420;
  CPedIntelligence::CanSeeEntityWithLights 0x605550; CGameLogic::LaRiotsActiveHere 0x441C10; 0x603AF0; CPedGroups::GetPedsGroup 0x5F7E80;
  CPedGroupIntelligence::AddEvent 0x5F7470 (thiscall (CEvent*) -> AL).
- Globals: 0.0f [0x858B50]=0.0f; CCheat::m_aCheatsActive[15] [0x96913F] (u8, CHEAT_HAVE_ABOUNTY_ON_YOUR_HEAD); now [0xB7CB84]=0. Vtables:
  0x86CA38 PedRespect (type 39), 0x86CA80 PedLike (38), 0x86CAC8 PedDislike (37), 0x86CB58 SeenCop (72), 0x858E68 PedHate (36), 0x86CB10
  PedHateBadlyLit (63).
- Behaviour: `result = 0`; if `(u32)acqType > 4` return 0. Jump table 0x606E58:
  - 0 (RESPECT, 0x606BD1): event A = CEventAcquaintancePed(other), vtable 0x86CA38, SEH 0; `ok = ped->intel->m_eventGroup.Add(&A, false)`.
  - 1 (LIKE, 0x606BFE): same with vtable 0x86CA80 (SEH 1).
  - 2 (IGNORE, 0x606C1F): only if `other->m_nPedType == 6` (cop), else return 0; vtable 0x86CB58 (CEventSeenCop, SEH 2), Add.
  - 3 (DISLIKE, 0x606C4D): vtable 0x86CAC8 (SEH 3), Add.
  - 4 (HATE, 0x606C91): `lit = ped->intel->CanSeeEntityWithLights(other, 0)`; `fcom 0.0f; test ah,0x41`:
    - lit > 0: if `m_aCheatsActive[15] != 0` or (`LaRiotsActiveHere() && 0x603AF0(ped, other)`): GROUP path - Hate event (SEH 4); if
      `g = GetPedsGroup(ped)`: `ev.m_TaskId (+0xE, i16) = 0x5DE` (1502 TASK_GROUP_KILL_THREATS_BASIC), `ok = g->intel (+0x30).AddEvent(&ev)`
      (AL); else `ev.m_TaskId = 0x3E8` (1000 TASK_COMPLEX_KILL_PED_ON_FOOT), `ok = Add(&ev, false) != 0`. Otherwise plain Hate event (SEH 5),
      `ok = Add(&ev, false) != 0`.
    - lit <= 0 or NaN (0x606D87): `fcomp 0.0f; test ah,5; jp return 0` -> continue only if lit < 0. Zero a local CVector, build
      CEventAcquaintancePed(other) in buffer B, then (store order) +0x1C = 0 (zero.x), +0x18 = now (u32), vtable = 0x86CB10, +0x20 = 0,
      +0x24 = 0, then +0x1C/+0x20/+0x24 = other's position x/y/z (matrix ? matrix->pos : placement.pos), SEH 6, `ok = Add(&B, false) != 0`.
      (CEventAcquaintancePedHateBadlyLit: +0x18 time, +0x1C CVector point.)
  For all: the vtable is re-stored before destruction, SEH -1, then the base dtor 0x4AF890. Return `ok` (AL; for Add results `!= 0`).
- gta-reversed: absent. Open questions: none.

## 0x607A90 CPedAcquaintanceScanner::sub_607A90  (catalogue: unknown/REA, port level 11)
Catalogue name: CPedAcquaintanceScanner::sub_607A90, proposed: `CPedAcquaintanceScanner::FindClosestAcquaintance(CPed* ped, int32 acqType,
CEntity** peds, int32 count, CPed** outPed, int32* outType)` (evidence: callers 0x607D80 (acquaintance scan of CEventScanner+0xAC, acqType
-1, list intel+0x130, 16) and 0x6221D0 (acqType 4 = hate); it fills `*outPed`/`*outType` through 0x607560).
- Convention: `__thiscall void (CPedAcquaintanceScanner* this, CPed* ped [esp+4], int32 acqType [esp+8], CPed** list [esp+0xC], int32 count
  [esp+0x10], CPed** outPed [esp+0x14], int32* outType [esp+0x18])`, `ret 0x18`. No SEH. Locals (E = esp at entry): n [E-0x6C], next index
  [E-0x68], this [E-0x64], dir CVector [E-0x60], eventTypes int32[4] [E-0x54..E-0x48], candidates CPed*[16] [E-0x40].
- Callees: CPed::IsAlive 0x5E0170; CVector::Normalise 0x59C910; CAcquaintance::GetAcquaintances 0x608970 (thiscall on ped+0x4E0, (type) -> u32
  mask); CPedType::GetPedFlag 0x608830 (cdecl (pedType) -> u32); CGameLogic::LaRiotsActiveHere 0x441C10; 0x603AF0 (riot hostility test, cdecl
  (ped, other)); CDecisionMakerTypes::GetInstance 0x4684F0; 0x6042B0 (CDecisionMakerTypes: any response to the given events); CPedGeometryAnalyser::
  CanPedTargetPed 0x5F1C40 (cdecl (ped, other, bool)); 0x607560 (per-candidate acquaintance event, returns acq type or -1).
- Globals: CPedAcquaintanceScanner::ms_fThresholdDotProduct [0xC0B034]=0 (f32, R; .bss here, set at run time).
- Behaviour:
  1. `*outPed = 0` (0x607A9F). If `count <= 0` skip to phase 2 with n = 0 (nothing happens).
  2. Phase 1, `for (i = 0; i < count; i++)`, `o = list[i]`; skip if `o == 0` or `!o->IsAlive()`.
     - `dir = oPos - pedPos` (components computed z, y, x; stored x, y, z), `dir.Normalise()`. `M = ped->m_matrix` (no null check):
       `dot = dir.z*M.fwd.z (+0x18) + dir.y*M.fwd.y (+0x14) + dir.x*M.fwd.x (+0x10)`; `fcomp ms_fThresholdDotProduct; test ah,0x41; je ok`
       -> ok if `dot > threshold`; otherwise ok only if `ped->flags (+0x46C) & 0x100` or `o->flags & 0x100` (either in a vehicle), else skip.
     - Match: `found = (acqType == -1 && o->m_nPedType (+0x598) == 6 /*COP*/)`. If not found: for `t = 4` down to `0`: if `acqType == -1 ||
       acqType == t`: `if (ped->m_acquaintance.GetAcquaintances(t) & CPedType::GetPedFlag(o->m_nPedType)) found = true`; stop at the first
       t that matches (the loop ends when found or t < 0). If still not found: found = `LaRiotsActiveHere() && 0x603AF0(ped, o)`. Not found -> skip.
     - Accept (0x607BFA): if `o->m_nPedType == 6` -> add. Else `types = {0x24, 0x25, 0x27, 0x28}` (36 PED_HATE, 37 PED_DISLIKE, 39 PED_RESPECT,
       40); if `LaRiotsActiveHere() && 0x603AF0(ped, o)` -> add; else add only if `0x6042B0(CDecisionMakerTypes::GetInstance(), ped, types,
       5)` returns true. Add = `cand[n++] = o`.
  3. Phase 2 (0x607C70-0x607CE9): `res = -1`; `for (k = 0; k < n; k++)`: if `res == 4` continue (no more work once a hate result exists);
     `onFoot = !(ped->flags & 0x100) && !(cand[k]->flags & 0x100)`; if `CanPedTargetPed(ped, cand[k], onFoot)`:
     `res = 0x607560(this, ped, acqType, res, cand[k], outPed, outType)`.
  4. Returns nothing meaningful (EAX = n at exit; callers ignore it - 0x607E16 / 0x62221A do not test EAX).
- Original bug: 0x6042B0 is called with count 5 (push 5, 0x607C3A) but only 4 event types are written; the 5th element [E-0x44] is never
  initialised in this frame (stale stack). It is read only when none of the first 4 events has a response in the ped's decision maker
  (and then used as an index into m_EventIndices).
- gta-reversed: absent (PedAcquaintanceScanner.h only). Spec from the bytes.
- Open questions: value of the uninitialised 5th event type at [E-0x44] (read in 0x6042B0 at 0x6042E8 when k = 4); `0x28` (40) has no gr
  eEventType name (between PED_RESPECT 39 and VEHICLE_DAMAGE_WEAPON 41).
