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
