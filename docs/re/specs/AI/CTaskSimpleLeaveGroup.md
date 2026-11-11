# CTaskSimpleLeaveGroup (AI) - RE-SPEC-3 group I3

Vtable 0x859CFC.

## 0x4635C0 CTaskSimpleLeaveGroup::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0xC8; ret` -> 200 (0xC8). Slot 4. gr matches.

## 0x4635F0 CTaskSimpleLeaveGroup::`scalar deleting destructor'  (catalogue name CTaskSimpleLeaveGroup::~CTaskSimpleLeaveGroup)  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of vtable 0x859CFC. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x463560 (0x4635F3): split by the no-CD build: `nop; jmp 0x40155E`: `vtbl = 0x859CFC; jmp 0x463566` -> `jmp 0x61A3A0` (~CTaskSimple: CTask vtbl). Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x4635F8, call at 0x463600; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.

## 0x463570 CTaskSimpleLeaveGroup::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this)`, plain `ret`, SEH frame. `p = CTask::operator new(8)` 0x61A5A0; if p: `CTaskSimple::CTaskSimple(p)` 0x61A390,
  `p->vtbl = 0x859CFC` (CTaskSimpleLeaveGroup); return p (0 if the allocation failed). `this` is not read (no members copied).
