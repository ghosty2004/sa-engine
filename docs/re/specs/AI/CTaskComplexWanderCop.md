# CTaskComplexWanderCop (AI) - RE-SPEC-3 group I4

Vtable 0x859C80 (gr TaskComplexWanderCop.cpp:9). Layout (gr TaskComplexWanderCop.h, size 0x50): CTaskComplexWander (0x28),
+0x28 m_pGoToPointAndStandStillTask (CTask*, owned), timers / counters after it (not touched here).

## 0x460D50 CTaskComplexWanderCop::GetWanderType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x1` (0x460D50); `ret` -> returns 1. Vtable: slot 11 (+0x2C) of vtable 0x859C80 (CTaskComplexWanderCop); slot 11 (+0x2C) of vtable 0x85A428.

## 0x460D60 CTaskComplexWanderCop::`scalar deleting destructor'  (catalogue name: ~CTaskComplexWanderCop; catalogue: specified/REA, port level 5)
- NAME CORRECTION: slot 0 of vtable 0x859C80 -> `CTaskComplexWanderCop::`scalar deleting destructor'`.
- `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this. Calls 0x460D80 (the complete dtor, see below; 0x460D63), then
  `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (0x460D68-0x460D75). Callers: none direct.
- gta-reversed: TaskComplexWanderCop.cpp:32 labels 0x460D60 as `~CTaskComplexWanderCop` (the deleting wrapper); behaviour equivalent.

## 0x460D80 CTaskComplexWanderCop::~CTaskComplexWanderCop  (catalogue name: CTaskComplexWanderCop::CTaskComplexWanderCop; catalogue: specified/REA, port level 4)
- NAME CORRECTION: catalogued as the constructor (gr TaskComplexWanderCop.cpp:24 comment `// 0x460D80` sits on the ctor), but the
  bytes are the complete-object DESTRUCTOR: it stores the class vtable, deletes an owned subtask and calls the base dtor; it
  takes no arguments (plain `ret`, 0x460DD6) and is called from the deleting dtor 0x460D60 and tail-jumped to from 0x4994E0
  (a derived class dtor that stores vtbl 0x85A428 first). The real ctor lives elsewhere (not this group).
- `__thiscall void (this = ECX)`, SEH frame (handler 0x837B88).
- Behaviour: `vtbl = 0x859C80` (0x460D9D); `if (m_pGoToPointAndStandStillTask (+0x28) != 0) m_pGoToPointAndStandStillTask->vtbl[0](1)`
  (virtual scalar deleting dtor, 0x460DA3-0x460DB6); then `CTaskComplexWander::~CTaskComplexWander` 0x669D20 (0x460DC2), which is a
  thunk: `vtbl = 0x86FE84` (0x40387C) then `jmp 0x61A3D0` (CTaskComplex dtor: deletes m_pSubTask, CTask vtbl). The +0x28 pointer
  is not cleared.
- Callers: 0x460D60 (call), 0x4994E0 (tail-jump; derived class with vtable 0x85A428, proposed CTaskComplexWanderCriminal-like
  subclass, not in this group).
- gta-reversed: dtor body `delete m_pGoToPointAndStandStillTask;` (cpp:33) matches the bytes; only the address labels are swapped
  (gr says ctor = 0x460D80, dtor = 0x460D60). Open questions: identity of the class with vtable 0x85A428 (0x4994E0).

## 0x460CE0 CTaskComplexWanderCop::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x50) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexWanderCop::CTaskComplexWanderCop(moveState = this+0xC (i32), wanderDir = this+0x10 (u8 zero-extended)) 0x460C80 (thiscall, 2 args); return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
