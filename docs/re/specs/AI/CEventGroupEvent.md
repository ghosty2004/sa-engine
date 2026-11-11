# CEventGroupEvent (AI) - RE-SPEC-3 group I1

Vtable 0x85AE48 (catalogue data-tables lists it unnamed). Layout (gr EventGroupEvent.h, size 0x14): CEvent (0xC),
+0xC m_ped CPed* (registered reference), +0x10 m_event CEvent* (owned). EVENT_GROUP_EVENT = 47 (0x2F).

## 0x4ADFD0 CEventGroupEvent::CEventGroupEvent  (catalogue: specified/REA, port level 2)
- `__thiscall CEventGroupEvent* (this, CPed* ped [esp+4], CEvent* event [esp+8])`, `ret 8`, returns this. SEH frame.
- Behaviour: `m_nTimeActive = 0` (+4), `m_bValid = 0` (+8 byte) (0x4ADFF0-0x4ADFF3); `vtbl = 0x85AE48` (0x4AE007);
  `m_ped = ped` (+0xC); `m_event = event` (+0x10) (0x4AE00D-0x4AE00F); if ped != 0 `CEntity::RegisterReference(ped, &m_ped)`
  0x571B70 (thiscall on ped, arg = slot address, 0x4AE014). The event is not cloned (ownership is taken).
- gta-reversed: EventGroupEvent.cpp:5 matches.

## 0x4AE030 CEventGroupEvent::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 0x2F (47, EVENT_GROUP_EVENT). Slot 1.

## 0x4AE070 CEventGroupEvent::~CEventGroupEvent  (catalogue: specified/REA, port level 1)
- `__thiscall void (this)`, plain `ret`. Complete-object dtor (called by 0x4B5E50).
- Oddity: the SEH prologue is obfuscated by the no-CD build: after `push -1` it jumps to 0x409BA4, which computes
  `[0x15608F4] - [0x7320B5] - 1` (= 0x839B68, the SEH handler address) with `stc; sbb`, pushes it via `xchg [esp],esi`,
  and jumps back to 0x4AE077. Net effect = `push 0x839B68`; no game-visible behaviour (the port ignores it).
- Behaviour: `vtbl = 0x85AE48`; if `m_ped` != 0 `CEntity::CleanUpOldReference(m_ped, &m_ped)` 0x571A00 (0x4AE093-0x4AE0A6);
  if `m_event` != 0 `m_event->vtbl[0](1)` (scalar deleting dtor with flags 1 = delete, 0x4AE0AB-0x4AE0B6); `vtbl = 0x85AB38`.
  m_event is not nulled. gr EventGroupEvent.cpp:12 matches.

## 0x4AE0E0 CEventGroupEvent::AffectsPedGroup  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this, CPedGroup* group [esp+4])`, `ret 4`.
- `return CPedGroupMembership::IsMember(&group->m_groupMembership (group+8), m_ped (+0xC))` 0x5F6A10 normalised with
  `test al,al; setne al` (0x4AE0E0-0x4AE0F2). Slot 6. gr header inline: `pedGroup->GetMembership().IsMember(m_ped)` - matches.

## 0x4AE100 CEventGroupEvent::BaseEventTakesPriorityOverBaseEvent  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, const CEventGroupEvent* other [esp+4])`, `ret 4` (or tail-jump, see below).
- Behaviour:
  1. `a = IsPriority(this->m_event->GetEventType())` (vtbl+4, 0x4AE10A), where IsPriority(t) = t in {0x36, 0x37, 0x39, 0x47,
     0x4C, 7, 0x4D} (54 LEADER_ENTERED_CAR_AS_DRIVER, 55 LEADER_EXITED_CAR_AS_DRIVER, 57 LEADER_QUIT_ENTERING_CAR_AS_DRIVER,
     71 PLAYER_COMMAND_TO_GROUP, 76 PLAYER_COMMAND_TO_GROUP_GATHER, 7 DRAGGED_OUT_CAR, 77 LEADER_ENTRY_EXIT), tested in that
     order (0x4AE10D-0x4AE134).
  2. `b = IsPriority(other->m_event->GetEventType())` - evaluated UNCONDITIONALLY, before `a` is tested (0x4AE13A-0x4AE169).
  3. if a return 1 (0x4AE16B-0x4AE174); else if b return 0 (0x4AE177-0x4AE180); else tail-call
     `this->m_event->TakesPriorityOver(other->m_event)` (vtbl+0x2C, slot 11, via `jmp` with the arg slot overwritten,
     0x4AE183-0x4AE192): its AL is returned.
- gta-reversed: EventGroupEvent.cpp:33 `IsPriorityEvent() || !other.IsPriorityEvent() && ...` - same result; gr short-circuits
  (skips other's GetEventType when a is true) - harmless (GetEventType has no side effects).

## 0x4B5E50 CEventGroupEvent::`scalar deleting destructor'  (catalogue name ~CEventGroupEvent)  (catalogue: specified/REA, port level 2)
- `__thiscall this (uint32 flags [esp+4])`, `ret 4`. Slot 0 of 0x85AE48. Calls 0x4AE070, then if `flags & 1` the inlined event
  pool delete on `ms_pEventPool` [0xB744AC] (identical to 0x4B5630, 0x4B5E60-0x4B5E8E). Returns this.

## 0x4B6EE0 CEventGroupEvent::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* (const this)`, plain `ret`, SEH frame.
- Behaviour: `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B6F00); if p == 0 return 0 (0x4B6F13, EAX = 0).
  Else `e = this->m_event->Clone()` (vtbl+0x10, 0x4B6F1A) - note: the inner Clone runs AFTER the outer slot is taken;
  then `CEventGroupEvent::CEventGroupEvent(p, m_ped [+0xC], e)` 0x4ADFD0, return its EAX (0x4B6F1D-0x4B6F24).
  If the pool is full the inner event is not cloned. If the inner Clone returns null, the group event holds a null m_event.
- gta-reversed: EventGroupEvent.h:22 `new CEventGroupEvent(m_ped, m_event->Clone())` - C++ evaluation order: MSVC allocates
  first as well; matches.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4AE040 CEventGroupEvent::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AE040).

## 0x4AE050 CEventGroupEvent::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: returns ST0 = 100.0f [0x858628] (0x4AE050).

## 0x4AE060 CEventGroupEvent::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 41 (0x29) (0x4AE060).

## 0x4AE0D0 CEventGroupEvent::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns false (AL=0; upper EAX bits unchanged) (0x4AE0D0).
