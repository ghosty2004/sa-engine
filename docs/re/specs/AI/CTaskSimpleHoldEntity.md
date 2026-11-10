# CTaskSimpleHoldEntity (AI) - RE-SPEC-3 group I3

Vtable 0x870B2C.

## 0x691460 CTaskSimpleHoldEntity::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x133; ret` -> 307 (TASK_SIMPLE_HOLD_ENTITY). Slot 4. gr matches.

## 0x692FD0 CTaskSimpleHoldEntity::`scalar deleting destructor'  (catalogue name CTaskSimpleHoldEntity::~CTaskSimpleHoldEntity)  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of vtable 0x870B2C. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x691630 (0x692FD3): SEH; `vtbl = 0x870B2C`; if byte +0x35 == 0 and the held entity [+8] != 0: `entity+0x1C &= ~0x400`, CleanUpOldReference, `entity+0x1C |= 0x800`, `[+8] = 0` (0x691653-0x691685); if the anim [+0x38] != 0 `SetFinishCallback(anim, 0x4D1390 DefaultAnimCB, 0)` 0x4CEBE0; if the anim block [+0x2C] != 0 `CAnimManager::RemoveAnimBlockRef((p - 0xB5D4A0) >> 5)` 0x4D3FD0; then ~CTaskSimple 0x61A3A0 (complete dtor itself not in this group; gr TaskSimpleHoldEntity.cpp). Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x692FD8, call at 0x692FE0; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
