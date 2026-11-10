# AI unattributed / vtable-slot functions (group I1)

Headings name the proposed class and method with evidence. Event-vtable classes are identified from their GetEventType value
(gr eEventType) and gr layouts:
- vt 0x85AE88 = CEventPotentialGetRunOver (type 12; +0x14 CVehicle* m_Vehicle (registered ref)).
- vt 0x85B070 = CEventChatPartner (type 0x13 = 19; +0xC m_leadSpeaker u8, +0x10 m_partner CPed*), size 0x14.
- vt 0x85B0B0 = CEventSexyPed (type 0x14 = 20; editable; +0x14 m_SexyPed CPed*), size 0x18.
- vt 0x85B0F8 = CEventSexyVehicle (type 0x15 = 21; +0xC m_SexyVehicle CVehicle*).
- vt 0x86C6D0 = CEventCreatePartnerTask (type 0x46 = 70; +0xC i32 m_partnerType, +0x10 CPed* m_partner, +0x14 u8, +0x18 f32), size 0x1C.
- vt 0x86CA38 = CEventAcquaintancePedRespect (type 0x27 = 39), 0x86CA80 = CEventAcquaintancePedLike (0x26 = 38),
  0x86CAC8 = CEventAcquaintancePedDislike (0x25 = 37); all CEventAcquaintancePed layout (size 0x18, +0x14 ped).
Shared tails: "pool delete" = the inlined CEvent::operator delete on ms_pEventPool [0xB744AC] (see CEvent.md 0x4B5630);
"deleting dtor shape" = call complete dtor, `if (flags & 1)` free, return this, `ret 4`.

## Event vtable slots - constant getters  (all catalogue: specified/REA, port level 0)
`__thiscall int32 ()` returning an immediate (`mov eax, imm; ret`):
| address | proposed name | returns |
|---|---|---|
| 0x4AE210 | CEventPotentialGetRunOver::GetEventType | 0xC |
| 0x4AE260 | CEventPotentialGetRunOver::GetEventPriority | 0x33 |
| 0x4AED30 | CEventChatPartner::GetEventType | 0x13 |
| 0x4AED50 | CEventChatPartner::GetEventPriority | 5 |
| 0x4AEE60 | CEventSexyPed::GetEventType | 0x14 |
| 0x4AEE90 | CEventSexyPed::GetEventPriority | 4 |
| 0x4AF070 | CEventSexyVehicle::GetEventType | 0x15 |
| 0x4AF090 | CEventSexyVehicle::GetEventPriority | 3 |
| 0x5F6260 | CEventCreatePartnerTask::GetEventType | 0x46 |
| 0x5F6410 | CEventCreatePartnerTask::GetEventPriority | 6 |
| 0x5FF040 | CEventAcquaintancePedRespect::GetEventType | 0x27 |
| 0x5FF100 | CEventAcquaintancePedLike::GetEventType | 0x26 |
| 0x5FF1C0 | CEventAcquaintancePedDislike::GetEventType | 0x25 |
| 0x5FF8B0 | CEventAcquaintancePedRespect::GetEventPriority | 0x17 |
| 0x5FF8C0 | CEventAcquaintancePedLike::GetEventPriority | 0x16 |
| 0x5FF8D0 | CEventAcquaintancePedDislike::GetEventPriority | 0x18 |
Evidence: each address is slot 1 (GetEventType) or slot 2 (GetEventPriority) of the listed vtable (data-tables). gr headers give the
same values where present.

## 0x4AE230 CEventPotentialGetRunOver::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `__thiscall CEntity* ()`: `v = [this+0x14]`; return v ? `v->m_pDriver (+0x460)` : nullptr (0x4AE230-0x4AE240).

## 0x4AE2D0 CEventPotentialGetRunOver::AffectsPed  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this, CPed* ped)`, `ret 4`. Returns 0 if `!ped->IsAlive()` 0x5E0170, if `ped->+0xFC` (m_pAttachedTo) != 0, if
  ped bInVehicle (+0x46C bit 8), or if `m_Vehicle` == 0; else `m_Vehicle->m_nVehicleType (+0x590) != 5` (not a boat) (0x4AE2DA-0x4AE30E).

## 0x4AEDC0 CEventChatPartner::AffectsPed  (catalogue: specified/REA, port level 1)
- `ped->IsAlive() && m_partner (+0x10) != 0` -> 1 else 0; `ret 4`.

## 0x4AEE80 CEventSexyPed::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `return [this+0x14]` (m_SexyPed).

## 0x4AEF00 CEventSexyPed::AffectsPed  (catalogue: specified/REA, port level 2)
- `__thiscall bool (this, CPed* ped)`, `ret 4`.
- Return 0 if `!ped->IsAlive()`, if m_SexyPed == 0, if `!m_SexyPed->IsAlive()`, or if `g_ikChainMan.IsLooking(ped)` 0x6181A0 and
  `g_ikChainMan.GetLookAtEntity(ped)` 0x6181D0 == m_SexyPed.
- `a` = first non-null of ped's `FindActiveTaskByType(0x4B2 PARTNER_DEAL)`, `(0x4BF BE_IN_COUPLE)`, `(0x4B3 PARTNER_GREET)` (on
  `ped->m_pIntelligence->m_TaskMgr`, 0x681740); if none return 1. `b` = the same search on m_SexyPed; if none return 1.
- Return `b->GetTaskType() != a->GetTaskType()` (b's queried first) (0x4AEFEC-0x4AEFFE). gr: CEventSexyPed::AffectsPed exists in gr
  only if reversed elsewhere - not in this group's evidence.

## 0x4AF100 CEventSexyVehicle::AffectsPed  (catalogue: specified/REA, port level 1)
- `ped->IsAlive() && m_SexyVehicle (+0xC) != 0` -> 1 else 0; `ret 4`.

## 0x5F62F0 CEventCreatePartnerTask::AffectsPed  (catalogue: specified/REA, port level 0)
- `mov al,1; ret 4` -> true (own copy, not shared with 0x4ABFF0).

## Clone / CloneEditable  (all catalogue: specified/REA, port level 3; `__thiscall CEvent* (const this)`, plain `ret`, SEH frame)
Pattern: `p = CEventPool::New(ms_pEventPool)` 0x4B5570 (or `CEvent::operator new(size)` 0x4B5620); if p == 0 return 0; else construct
and return p.
| address | proposed name | construction |
|---|---|---|
| 0x4B7210 | CEventChatPartner::Clone | `0x4AECD0(p, (u8)m_leadSpeaker [+0xC], m_partner [+0x10])` (CEventChatPartner ctor) |
| 0x4B7280 | CEventSexyPed::CloneEditable | `0x4AEDF0(p, m_SexyPed [+0x14])` (CEventSexyPed ctor) |
| 0x4B72F0 | CEventSexyVehicle::Clone | `0x4AF010(p, m_SexyVehicle [+0xC])` (CEventSexyVehicle ctor) |
| 0x5F6280 | CEventCreatePartnerTask::Clone | `operator new(0x1C)`; `0x5F6190(p, [+0xC], [+0x10], (u8)[+0x14], [+0x18] raw f32)` |
| 0x5FF050 | CEventAcquaintancePedRespect::CloneEditable | `operator new(0x18)`; `CEventAcquaintancePed::CEventAcquaintancePed(p, [+0x14])` 0x4AF820; `p->vtbl = 0x86CA38` |
| 0x5FF110 | CEventAcquaintancePedLike::CloneEditable | same with vtbl 0x86CA80 |
| 0x5FF1D0 | CEventAcquaintancePedDislike::CloneEditable | same with vtbl 0x86CAC8 |
The cloned event's m_nTimeActive / m_bValid / editable fields come from the ctor (not copied).

## Deleting destructors  (catalogue: specified/REA, port level 2-3)
| address | proposed name | complete dtor | free |
|---|---|---|---|
| 0x4B60D0 | CEventChatPartner::`scalar deleting destructor' | 0x4AED60 | inlined pool delete |
| 0x4B6120 | CEventSexyPed::`scalar deleting destructor' | 0x4AEEA0 | inlined pool delete |
| 0x4B6170 | CEventSexyVehicle::`scalar deleting destructor' | 0x4AF0A0 | inlined pool delete |
| 0x5F6300 | CEventCreatePartnerTask::`scalar deleting destructor' | 0x5F6200 | CEvent::operator delete 0x4B5630 |
| 0x5FF0B0 | CEventAcquaintancePedRespect::`scalar deleting destructor' | 0x5FF030 | 0x4B5630 |
| 0x5FF170 | CEventAcquaintancePedLike::`scalar deleting destructor' | 0x5FF0F0 | 0x4B5630 |
| 0x5FF230 | CEventAcquaintancePedDislike::`scalar deleting destructor' | 0x5FF1B0 | 0x4B5630 |

## 0x46FEE0 CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>::GetTargetPos (proposed)  (catalogue: unknown/REA, port level 0)
- Evidence: only caller CTaskComplexSeekEntityStandard::ControlSubTask 0x46F800 (2 sites); `this+0x40` is the seek-pos calculator
  object (vtable call slot 0), `this+0xC` the seek entity.
- `__thiscall CVector* (this, CVector* out [esp+4], CPed* ped [esp+8])`, `ret 8`, returns out.
- `calc = this+0x40; calc->vtbl[0](ped, m_entity (+0xC), &tmp)` (thiscall ComputeEntitySeekPos(ped, entity, CVector* out)); `*out = tmp`
  (3 dword copies) (0x46FEE6-0x46FF13). gr: absent (inlined in gr).

## 0x665DC0 CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::GetTargetPos (proposed)  (catalogue: unknown/REA, port level 0)
- Byte-for-byte the same body as 0x46FEE0 (separate template instance); caller CTaskComplexSeekEntity<XYOffset>::ControlSubTask 0x665760.

## 0x48DE60 CTaskSimpleRunAnim::~CTaskSimpleRunAnim (proposed)  (catalogue: unknown/REA, port level 2)
- `jmp 0x61A6F0` (CTaskSimpleAnim::~CTaskSimpleAnim): the subclass dtor has no body of its own. Caller 0x61BAA0 = slot 0 of
  CTaskSimpleRunAnim vtable 0x86D528 (deleting dtor).

## 0x48EA90 CTaskSimpleLookAbout::~CTaskSimpleLookAbout (proposed)  (catalogue: unknown/REA, port level 2)
- Same `jmp 0x61A6F0`; caller 0x48EA70 = slot 0 of CTaskSimpleLookAbout vtable 0x85A164.

## 0x48E8A0 CTaskSimplePause::~CTaskSimplePause (proposed)  (catalogue: unknown/REA, port level 1)
- `vtbl = 0x85A278` then `jmp 0x61A3A0` (~CTaskSimple). Caller 0x48E880 (CTaskSimplePause deleting dtor).

## 0x4ACE80 CEventObjectCollision::IsHeadOnCollision (proposed)  (catalogue: unknown/REA, port level 1)
- Evidence: caller CEventHandler::ComputeObjectCollisionResponse 0x4B92B0; `this+0x18` = m_impactNormal (CEventObjectCollision layout
  as CEventBuildingCollision: +0x18 normal); same shape as CEventBuildingCollision::IsHeadOnCollision 0x4AD1E0 (gr).
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`.
- `v = (n.x, n.y, 0)` (z forced 0); `v.Normalise()` 0x59C910; `f = ped->m_matrix->forward` (+0x10, matrix NOT null-checked);
  `d = -((v.y*f.y + v.z*f.z) + v.x*f.x)`; return `d > 0.9397f [0x85AAE4]` (strict; NaN false) (0x4ACEB6-0x4ACEDC).

## 0x4BC470 CTaskComplexSeekEntity<...>::SetMoveStateRadius (proposed)  (catalogue: unknown/REA, port level 0)
- Evidence: the caller CEventHandler::ComputePedCollisionWithPedResponse 0x4BDB80 passes ECX = the ped's TASK_COMPLEX_SEEK_ENTITY (0x38B)
  task found with FindTaskByType (0x4BDF0C), arg = distance-to-leader + 0.1f; +0x18 of the seek task is its move-state radius and +0x28
  its CTaskTimer.
- `__thiscall void (this, float a [esp+4])`, `ret 4`: if `a != this->+0x18` (fcomp; `test ah,0x44; jnp`: exactly equal skips, NaN
  counts as different): `+0x18 = a` (raw dword), timer start `+0x28 = CTimer::m_snTimeInMilliseconds [0xB7CB84]`, interval `+0x2C = 0`,
  started `+0x30 = 1`.

## 0x59EEB0 CHandObject::CHandObject (proposed)  (catalogue: unknown/REA, port level 15)
- Evidence: writes vtbl 0x866EE0 (gr RH_ScopedVirtualClass(CHandObject, 0x866EE0)); caller CTaskSimplePlayHandSignalAnim::StartAnim
  0x61AF60 (6 sites). gr HandObject.cpp has the C++ ctor (not hooked) - reading aid only.
- `__thiscall this (int32 modelIndex [esp+4], CPed* ped [+8], bool leftHand [+0xC])`, `ret 0xC`, returns this. SEH prologue
  obfuscated (0x407615: `stc; adc` of two dwords pushed as the handler; no effect).
- Behaviour (0x59EECF-0x59EFC1): `CObject::CObject()` 0x5A1D10; vtbl = 0x866EE0; `h = GetAnimHierarchyFromSkinClump(ped->m_pRwClump)`
  0x734A40; `+0x17C = ped`; `ped->UpdateRpHAnim()` 0x532B20; `+0x180 = RpHAnimIDGetIndex(h, leftHand ? 0x21 : 0x17)` 0x7C51A0 (bone 33
  BONE_L_FORE_ARM / 23 BONE_R_FORE_ARM); `SetModelIndex(modelIndex)` 0x532AE0; `RpAnimBlendClumpInit(this->m_pRwClump)` 0x4D6720;
  `mi = ms_modelInfoPtrs[ped->m_nModelIndex]`; txd slot `mi->+0xA` (i16) in `CTxdStore::ms_pTxdPool` [0xC8800C] (slot is null if its
  byteMap flag has 0x80, else objects + 12*idx; NO null check before `dict = *slot`); `+0x184 = RwTexDictionaryFindHashNamedTexture(dict,
  mi->m_nKey (+4))` 0x734E50, or if null `GetFirstTexture(dict)` 0x734940; entity flags `+0x1C = (+0x1C & ~1) | 0x10000400`;
  `+0x36 = (+0x36 & 7) | 0x10`; `+0x13C (m_nObjectType) = 5`; `SetIsStatic(false)` 0x5A0760; `ms_modelInfoPtrs[modelIndex]->+0xC = 0xFF`;
  `+0x188 = 1`.

## 0x5BB9F0 CDecisionMakerTypes::LoadEventIndices helper (proposed: ReadEventIndexFile)  (catalogue: unknown/REA, port level 1)
- Evidence: only caller CDecisionMakerTypes::LoadEventIndices 0x600840.
- `__stdcall void (int32* out [esp+4], const char* fileName [esp+8])`, `ret 8` (no `this`). Stack buffer 0x204.
- `memset(out, 0, 0x60 dwords)` (`rep stosd`, 96 ints); `CFileMgr::SetDir("")` 0x5387D0; `SetDir("data\\decision\\" 0x86A6B4)`;
  `f = CFileMgr::OpenFile(fileName, "r" 0x85A53C)` 0x538900; `SetDir("")`; `i = 0`; while `CFileMgr::ReadLine(f, line, 256)` 0x5389B0:
  if `line[0] != 0 && line[0] != '\n'`: `sscanf(line, "%s %d" 0x86A6AC, name, &idx)` 0x8220AD; `out[idx] = i`; `i++` (0x5BBA56-0x5BBA88).
  `CFileMgr::CloseFile(f)` 0x5389D0. No bounds check on idx; f is not null-checked.

## 0x5BD360 CTaskSimpleFight::LoadMeleeData helper (proposed: GetHitLevelFromString)  (catalogue: unknown/REA, port level 0)
- `__cdecl u8 (const char* s)`; only the FIRST character is examined: 'H' -> 0, 'L' -> 1, 'G' -> 2, 'B' -> 3, otherwise 7.
- Oddity (keep): after the single-char tests the code compares the zero-extended first byte with 0x484C, 0x4C4C and 0x474C ("LH", "LL",
  "LG" as 16-bit values); these can never match (value <= 0xFF), so results 4/5/6 are unreachable and any other string returns 7
  (`setne al; add al,6`, 0x5BD382-0x5BD3A1).

## 0x5BD3B0 CTaskSimpleFight::LoadMeleeData helper (proposed: GetComboAnimGroup)  (catalogue: unknown/REA, port level 0)
- `__cdecl int32 (int32 i)`: for 1 <= i <= 8 returns `60 + i` (0x3D..0x44; jump table 0x5BD3F4), any other value returns 0x3E (62).

## SArray (MSVC std::vector) instances used by CPedAttractor  (catalogue: unknown/REA)
gr Attractors/PedAttractor.h: `SArray<CPed*> m_AttractPeds, m_ArrivedPeds; SArray<CPedTaskPair> m_PedTaskPairs` (CPedTaskPair = 0x14
bytes). MSVC vector layout: +0 allocator (unused), +4 first, +8 last, +0xC end. All are standard-library code; a port may use Vec
provided the observable results (element order, copies) match. Growth rule (both _Insert_n): if `size + n > capacity`:
`newcap = (MAX - cap/2 < cap) ? 0 : cap + cap/2`; if `newcap < size + n` then `newcap = size + n`; MAX = 0x3FFFFFFF (4-byte) /
0x0CCCCCCC (20-byte); `MAX - size < n` calls the length-error thrower (0x5ECBB0 / 0x5ECB30). New storage via `operator new` 0x82119A,
old freed with `free` 0x8214BD.
| address | proposed name | behaviour |
|---|---|---|
| 0x5E9800 | SArray<CPedTaskPair>::size | `first ? (last - first) / 20 : 0` (signed magic divide), thiscall |
| 0x5EC220 | SArray<CPed*>::erase(iterator* ret, iterator where) | shift [where+1, last) down by one dword, `last -= 4`, `*ret = where`; `ret 8` (caller CPedAttractor::BroadcastArrival) |
| 0x5ED1B0 | SArray<CPedTaskPair>::_Insert_n(where, n, const T& val) | MSVC _Insert_n for 20-byte elements (reallocate / move-up / fill as below); `ret 0xC` |
| 0x5ED4B0 | SArray<CPed*>::_Insert_n(where, n, const T& val) | same for 4-byte elements; val is copied first; SEH prologue obfuscated (0x4019E5) |
| 0x5EDD90 | SArray<CPedTaskPair>::insert(iterator* ret, iterator where, const T& val) | `off = (first && size) ? (where-first)/20 : 0`; `_Insert_n(where, 1, val)`; `*ret = first + off*20` (re-read after realloc); `ret 0xC` |
| 0x5EE200 | SArray<CPed*>::push_back(const T& val) | if first and size < capacity: copy-construct at last (0x5EB2A0), `last += 4`; else `_Insert_n(last, 1, val)` 0x5ECF10; `ret 4` (callers CPedAttractor::RegisterPed / BroadcastArrival) |
| 0x5EE270 | SArray<CPedTaskPair>::push_back(const T& val) | same pattern with 0x5EB2E0 / `last += 0x14` / insert 0x5EDD90 |
| 0x5EE300 | SArray<CPed*>::push_back (second instance) | same pattern with 0x5EB310 / insert 0x5ED4B0 (caller CPedAttractor::SetTaskForPed) |
_Insert_n paths (both): n == 0 -> nothing. Reallocate: copy [first, where) (0x5EAE70/0x5EAE40), fill n copies of val, copy [where, last),
free old, set first/last/end. Else if `last - where < n`: copy [where, last) to where+n, fill the gap past old last with n-(last-where)
copies, `last += n`, fill [where, old last) with val. Else: copy [last-n, last) to last, `last += n`, copy_backward [where, old last-n)
to old last, fill [where, where+n) with val.

## 0x5FFEE0 CEntityScanner::GetClosestEntityInRange (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CEntity* ()`: `return [this+0x4C]`. Evidence: callers pass `ECX = ped->m_pIntelligence + 0xD4` (the vehicle scanner;
  entity array at scanner+0xC, closest at +0x4C), e.g. 0x643535 in CTaskComplexEnterAnyCarAsDriver::CreateFirstSubTask.

## 0x606060 CDecisionMaker::MakeDecision (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall int32 (this, int32 eventIdx [esp+4], a2..a9 [esp+8..+0x24])`, `ret 0x24`: tail-like forward
  `return ((CDecision*)(this + eventIdx*0x3C))->MakeDecision(a2, a3, a4, a5, a6, a7, a8, a9)` 0x6040D0 (8 args re-pushed in order).
  Caller CDecisionMakerTypes::MakeDecision 0x606E70 (2 sites).

## 0x61A2E0 GetPlayerRelationshipRecorder  (catalogue: specified/REA, port level 0)
- `__cdecl CPlayerRelationshipRecorder* ()`. If `g_sPlayerRelationshipRecorder` [0xC17084] != 0 return it. Else `p = operator new(0x80)`
  0x82119A; if p: for i = 0..15: `*(u32*)(p + 8i) = 0; *(u8*)(p + 8i + 4) = 0`; store p and return p. If allocation fails store 0 and
  return 0. gr PlayerRelationshipRecorder.cpp:82 equivalent.

## 0x61AEB0 CTaskSimplePlayHandSignalAnim::~CTaskSimplePlayHandSignalAnim (proposed)  (catalogue: unknown/REA, port level 3)
- Complete dtor (SEH): vtbl = 0x86D5B8; for the hand objects at +0x1C then +0x20: if non-null: `CWorld::Remove(obj)` 0x563280; if the
  (re-read) pointer is non-null `obj->vtbl[0](1)` (delete); `CObject::nNoTempObjects` (i16 [0xBB4A70]) -= 1 (inside the outer if, even
  if the re-read was null). Pointers are not cleared. Then `CTaskSimpleAnim::~CTaskSimpleAnim` 0x61A6F0.

## 0x63BA00 CTaskComplexLeaveCar::SetVehicleFlags (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this, CPed* ped)`, `ret 4`: `m_nDoorFlagsSet = CCarEnterExit::ComputeDoorFlag(veh, m_nTargetDoor, true)` 0x64E550 (cdecl);
  `veh->SetGettingOutFlags(flag)` 0x6D3020; `m_nNumGettingInSet = 1`; `veh->m_nNumGettingIn++`; if `veh->m_pDriver` and
  `!driver->IsPlayer()` and `ped == driver` and `m_bSensibleLeaveCar`: `veh->+0x3D0 = 0` (byte, autopilot field - name UNCLEAR) and
  `veh->+0x3BA = 0` (m_autoPilot.m_nCarMission = MISSION_NONE); then if `ped->IsPlayer()` and `ped == veh->m_pDriver`:
  `veh->+0x36 = (veh->+0x36 & 7) | 0x48` (status 9). Evidence: callers CTaskComplexLeaveCar::CreateNextSubTask (2 sites).

## 0x63BAB0 CTaskComplexLeaveCar::ComputeTargetDoor (proposed)  (catalogue: unknown/REA, port level 1)
- As in CTaskComplexLeaveCar.md (header "Helpers"): if `m_nTargetDoor == 0` then `m_nTargetDoor = CCarEnterExit::ComputeTargetDoorToExit(veh,
  ped)` 0x64F110. `ret 4`.

## 0x63BAE0 CTaskComplexLeaveCar::CreateLineUpTask (proposed)  (catalogue: unknown/REA, port level 1)
- As in CTaskComplexLeaveCar.md: SEH frame; `p = operator new(0x1C)` 0x82119A; `+0x1C = p ? CTaskUtilityLineUpPedWithCar(p, &zeroVector
  (local), 0, 0, m_nTargetDoor) 0x64FBB0 : 0` (the previous pointer is overwritten, not freed). `ret 4`.

## 0x646CF0 CCarEnterExit::IsCarDoorInUse (proposed)  (catalogue: unknown/REA, port level 0)
- As in CTaskComplexLeaveCar.md: `__cdecl void (CVehicle*, int32 door, bool* in, bool* out)`; door 8/9/10/11 -> bit 4/8/1/2 (jump table
  0x646D58); `*in = 1` if `veh->+0x486 & bit`, `*out = 1` if `veh->+0x487 & bit`; other doors: nothing.

## 0x63D100 CTaskComplexEnterAnyCarAsDriver::~CTaskComplexEnterAnyCarAsDriver (proposed)  (catalogue: unknown/REA, port level 2)
- `vtbl = 0x86EA14; jmp 0x61A3D0` (~CTaskComplex). Caller 0x6434F0 (its deleting dtor, see CTaskComplex.md).

## 0x6550D0 CTaskComplexWalkRoundObject::~CTaskComplexWalkRoundObject (proposed)  (catalogue: unknown/REA, port level 2)
- Complete dtor (SEH): vtbl = 0x86F364; if `+0x1C` (object) `CleanUpOldReference(obj, &+0x1C)`; if `+0x20` (CPointRoute*)
  `CPointRoute::operator delete` 0x41B7D0 (no dtor call); `~CTaskComplex` 0x61A3D0. Caller 0x657200 (deleting dtor).

## 0x6551D0 CTaskComplexWalkRoundObject::ComputeRouteAndGetHitSideDistance (proposed)  (catalogue: unknown/REA, port level 10)
- Evidence: caller CTaskComplexWalkRoundObject::CreateFirstSubTask 0x657380; uses +0x10 target pos, +0x1C entity, +0x20 route.
- `__thiscall float (this, CPed* ped [esp+4])`, `ret 4`, result in ST0. Locals: dists f32[4] at [B+0xC], normals CVector[4] at [B+0x1C].
- Behaviour: `save = CPedGeometryAnalyser::ms_fPedNominalRadius` [0x8D22B0]; set it to 0.7f (immediate 0x3F333333);
  `CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox(ped, m_entity, &m_targetPos (+0x10), m_route (+0x20), 0)` 0x5F6110;
  restore the radius (0x6551D3-0x655206). `ComputeEntityBoundingBoxPlanes(pedPos.z, m_entity, &normals, &dists)` 0x5F3660;
  `s = ComputeEntityHitSide(pedPos, m_entity)` 0x5F3AC0; return `((normals[s].z*P.z + normals[s].y*P.y) + normals[s].x*P.x) + dists[s]`
  (0x655266-0x655281; s is not range-checked).

## 0x65DC10 CEventLeanOnVehicle::~CEventLeanOnVehicle (proposed)  (catalogue: unknown/REA, port level 1)
- Complete dtor: vtbl = 0x86F878; if `+0xC` (vehicle) `CleanUpOldReference(veh, &+0xC)`; then 0x4AC010 (base CEvent dtor). Caller 0x65DBF0.

## 0x65DE60 CEntitySeekPosCalculatorXYOffset::CEntitySeekPosCalculatorXYOffset (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall this (const CVector* offset [esp+4])`, `ret 4`: `vtbl = 0x86F8F8; +4 = *offset` (3 dwords); returns this.
  Caller CTaskComplexGangFollower::CreateNextSubTask (stack temporary).

## 0x65E710 CTaskComplexGangLeader::GetRandomGangAmbientAnim helper (proposed: GetSmokeAnimForPed)  (catalogue: unknown/REA, port level 0)
- `__cdecl int32 (CEntity* e [esp+4])`: `m = (i16)e->m_nModelIndex (+0x22)`; returns 0x12E (SMKCIG_PRTL_F) if m == 0x67 or m == 0x69,
  else 0x12C (SMKCIG_PRTL).

## 0x65EE90 CTaskComplexGangFollower::SetFollowWithSeek (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void (this, bool b [esp+4])`, `ret 4`: `m_Flags (+0x3D) bit 2 = b & 1` (xor/and/xor). Caller
  CPedGroupDefaultTaskAllocatorRandom::AllocateDefaultTasks 0x5F6E90. (Bit meaning: CTaskComplexGangFollower.md.)

## 0x664770 CTaskGoToVehicleAndLean::CalcTargetPos  (catalogue: unknown/REA, port level 1)
- Name from gr TaskGoToVehicleAndLean.h:28 comment. `__thiscall CVector* (this, CVector* out [esp+4], CPed* ped [esp+8])`, `ret 8`,
  returns out. `veh = this+0xC`.
- `bb = ms_modelInfoPtrs[veh->m_nModelIndex]->m_pColModel (+0x14)`: min (+0), max (+0xC). `L = (0, 0, 0)`.
  `d = pedPos - vehPos` (x, y, z). `r` = veh matrix right (+0) if veh->m_matrix else `(cos h, sin h, 0)` with h = placement heading
  (+0x10) (x87 `fcos`/`fsin` on the f32). `dot = (r.z*d.z + r.y*d.y) + r.x*d.x`. If `dot > 0` (strict): `L.x = bb.max.x + 0.5f [0x858B8C]`,
  `this+0x15 = 1`; else (<= or NaN): `L.x = bb.min.x - 0.5f`, `this+0x15 = 0`. `*out = CMatrix::operator*(veh->m_matrix, &L)` 0x59C890
  (cdecl (out, matrix, vec); matrix not null-checked) (0x664773-0x664885).

## 0x665270 CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::CreateSubTask (proposed)  (catalogue: unknown/REA, port level 3)
- Evidence: 3 callers in the same template (CreateNextSubTask 0x665080, ...). Layout used: +0xC entity, +0x18 f32, +0x1C f32, +0x24 f32,
  +0x40 seek-pos calculator (vtbl), +0x50 moveState, +0x54 flags.
- `__thiscall CTask* (this, int32 type [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH; entry obfuscated (`nop; jmp 0x402328`).
- Cases (0 when new fails unless noted):
  - 0xCB STAND_STILL: `new(0x20) CTaskSimpleStandStill(2000, false, false, 8.0f)`.
  - 0xDB TIRED: `new(0x1C) CTaskSimpleTired(1000)` 0x630F20.
  - 0xF4 SEQUENCE: `s = new(0x40) CTaskComplexSequence()` 0x632BD0 (s may be 0); if `flags & 2`: `s->AddTask(new(0x28)
    CTaskComplexTurnToFaceEntityOrCoord(m_entity, 0.5f [0x86FC7C], 0.2f [0x86FC80]))` 0x632D10; then `s->AddTask(new(0x20)
    CTaskSimpleStandStill(100, false, false, 8.0f))`; return s. ORIGINAL BUG: AddTask is called with ECX = 0 when the sequence
    allocation failed.
  - 0x2C0 LEAVE_CAR: `new(0x34) CTaskComplexLeaveCar(ped->m_pVehicle, 0, 0, true, false)`.
  - 0x2CF DRIVE_TIMED: `p = new(0x70)`; `CTaskSimpleCarDrive(p, ped->m_pVehicle, nullptr, false)` 0x63C340; then `vtbl = 0x859E50`,
    `+0x60 = 2000`, `+0x64 = 0`, `+0x68 = 0`, `+0x6C = 0`, `+0x6D = 0` (inlined CTaskSimpleCarDriveTimed(veh, 2000)).
  - 0x387 GO_TO_POINT_AND_STAND_STILL: `p = new(0x28)`; if p: `t = calc->vtbl[0](ped, m_entity, &tmp)` (target), then
    `CTaskComplexGoToPointAndStandStill(p, m_moveState (+0x50), &t, +0x18, +0x1C, false, false)` 0x668120. Then
    `p->+0x24 bit 3 = this->+0x54 bit 3` - executed also when p == 0 (ORIGINAL BUG, null+0x24 access).
  - 0x38A FOLLOW_NODE_ROUTE: `p = new(0x60)`; target as above; `CTaskComplexFollowNodeRoute(p, +0x50, &t, +0x18, +0x1C, +0x24, false,
    -1, true)` 0x66EA30.
  - 0x398 TURN_TO_FACE_ENTITY: `new(0x28) CTaskComplexTurnToFaceEntityOrCoord(m_entity, 0.5f, 0.2f)` 0x66B890.
  - 0x516 and anything else: 0.

## 0x667310 CPathNode::GetNodeCoors (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void (this, CVector* out [esp+4])`, `ret 4` (EAX = last copied dword, not out). For x, y, z: `(f32)((i16)this->+8/+0xA/+0xC)
  * 0.125f [0x858C48]` (fild of the sign-extended value, fmul, fstp f32) stored to out. Caller CTaskComplexFollowNodeRoute::ComputePathNodes
  (4 sites).

## 0x668240 CTaskComplexGoToPointAndStandStill::~CTaskComplexGoToPointAndStandStill (proposed)  (catalogue: unknown/REA, port level 2)
- `vtbl = 0x86FDA8; jmp 0x61A3D0`.

## 0x669FB0 inlined CTaskTimer::IsOutOfTime on this+0x28 (proposed: CTaskComplexGangLeader::IsScanTimeUp)  (catalogue: unknown/REA, port level 0)
- `__thiscall bool ()`: if !started (+0x30) return 0; if stopped (+0x31): start (+0x28) = now [0xB7CB84], stopped = 0; return
  `now >= start + interval (+0x2C)` (unsigned, `cmp; sbb; inc`). Caller CTaskComplexGangLeader::ScanForStuff 0x65E200.

## 0x66B150 sub_66B150 (setter of +0x34)  (catalogue: unknown/REA, port level 0)
- `__thiscall void (this, int32 v)`, `ret 4`: `this->+0x34 = v`. Caller CTaskComplexPartner::CreateNextSubTask 0x683AD0; the class of
  `this` is UNCLEAR (a subtask created there).

## 0x66E050 CTaskComplexFollowPointRoute::~CTaskComplexFollowPointRoute (proposed)  (catalogue: unknown/REA, port level 2)
- Complete dtor (SEH): vtbl = 0x870078; if `+0x20` `CPointRoute::operator delete` 0x41B7D0; `~CTaskComplex` 0x61A3D0.

## 0x678CC0 CTaskComplexInAirAndLand::~CTaskComplexInAirAndLand (proposed)  (catalogue: unknown/REA, port level 2)
- `vtbl = 0x8704AC; jmp 0x61A3D0`.

## 0x681FE0 CTaskComplexPartner::ComputeMeetingPositions (proposed)  (catalogue: unknown/REA, port level 1)
- Evidence: caller CTaskComplexPartner::CreateNextSubTask 0x683AD0; layout from the ctor 0x681E70 bytes: +0x34 m_taskId, +0x38 m_partner,
  +0x3C m_distanceMultiplier f32, +0x5C m_taskCompleted u8 (gr header offsets are 8 lower than the bytes from m_taskId on).
- `__thiscall void (this, CPed* ped [esp+4], CVector* out1 [esp+8], CVector* out2 [esp+0xC])`, `ret 0xC`. Frame base B = esp after
  `push ebx; push esi`.
- Behaviour:
  1. P = ped pos [B+0x28], Q = m_partner pos [B+0x34] (partner not null-checked). `D = Q - P` stored f32 [B+0x10] (x, y, z each
     `fld Q; fsub P; fstp`). `k = m_distanceMultiplier`; `M = P + D*k` per component (`D.c*k` then `+ P.c`, stored f32 [B+0x1C]).
     `D.Normalise()` 0x59C910 (in place) (0x681FEB-0x68209B).
  2. If `this->GetTaskType() != 0x4B4` (not PARTNER_CHAT): relax M (x87 registers hold Mx, My, Mz) for i = 0..10 (`cmp i,10; jg` ->
     failure):
     - `d1 = sqrt((Mx-Px)^2... )` computed as `sqrt(ay*ay + ax*ax)` with ax = Mx-Px, ay = My-Py (2D). If `d1 < 0.7f [0x858CB0]` (strict):
       `s = 0.75f [0x858F34] - d1`; `M += D*s` (x, y, z; products stored f32 first) and mark movedP.
     - `d2` likewise against Q; if `d2 < 0.7f`: `s = 0.75f - d2`; `M -= D*s`; continue looping.
     - else if movedP continue looping; else converged -> leave the loop.
     Converged: store Mz [B+0x24] and My [B+0x20] (Mx stays on the x87 stack). Failure (i > 10, 0x68225C): store Mz, My, discard Mx,
     set `m_taskCompleted (+0x5C) = 1` and RETURN without writing out1/out2.
  3. Output (also the direct path for type 0x4B4, which uses the unrelaxed M): `h = D * 0.5f [0x858B8C]` (per component);
     `out1 = (Mx - h.x, My - h.y, Mz - h.z)`; `out2 = (h.x + Mx, h.y + My, h.z + Mz)` (Mx from the x87 register, My/Mz from the f32
     stores) (0x6821A0-0x682253).
- gr: absent (the gr CreateNextSubTask is plugin::Call).

## 0x6822B0 CTaskComplexPartner::GetPartnerState (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall int32 ()`: `t = m_partner->m_pIntelligence->m_TaskMgr.FindActiveTaskByType(m_taskId (+0x34))` 0x681740 (partner read from
  +0x38; obfuscated jump via 0x40806B loads the intelligence); return t ? `(i8)t->+0x59` (m_partnerState, sign-extended) : 0.

## 0x690C70 CTaskSimpleFacial::CTaskSimpleFacial  (catalogue: specified/REA, port level 1)
- `__thiscall this (int32 type [esp+4], int32 durationMs [esp+8])`, `ret 8`: `CTaskSimple()` 0x61A390; vtbl = 0x870AB8; timer
  `+8 = 0, +0xC = 0, +0x10 = 0, +0x11 = 0`; `+0x18 = durationMs`; `+0x14 = type`. gr TaskSimpleFacial.cpp:16 matches.

## 0x691630 CTaskSimpleHoldEntity::~CTaskSimpleHoldEntity (proposed)  (catalogue: unknown/REA, port level 1)
- Complete dtor (SEH): vtbl = 0x870B2C; if `!this->+0x35` and entity `+8` != 0: `entity->+0x1C &= ~0x400`; `CleanUpOldReference(entity,
  &+8)`; `entity (re-read)->+0x1C |= 0x800`; `+8 = 0`. If anim assoc `+0x38`: `assoc->SetFinishCallback(0x4D1390, nullptr)` 0x4CEBE0. If
  anim block `+0x2C`: `CAnimManager::RemoveAnimBlockRef((+0x2C - 0xB5D4A0) >> 5)` 0x4D3FD0. Then `~CTaskSimple` 0x61A3A0. Callers:
  deleting dtor 0x692FD0 and two tail calls (0x6935B0, 0x6935E0).
