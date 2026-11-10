# CEventScriptCommand (AI) - RE-SPEC-3 group I4

Vtable 0x85B378. Layout (gr EventScriptCommand.h): CEvent (0xC), +0xC m_primaryTaskIndex (i32), +0x10 m_task (CTask*, owned),
+0x14 m_affectsDeadPeds (bool).

## 0x4B6440 CEventScriptCommand::`scalar deleting destructor'  (catalogue name: ~CEventScriptCommand; catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of vtable 0x85B378 -> scalar deleting destructor.
- `__thiscall this (uint32 flags [esp+0xC] after two pushes = [esp+4] on entry)`, `ret 4`, returns this.
- Behaviour: call 0x4B0A50 (complete dtor, catalogue unknown; proposed CEventScriptCommand::~CEventScriptCommand): SEH frame;
  `vtbl = 0x85B378`; `if (m_task (+0x10)) m_task->vtbl[0](1)`; `vtbl = 0x85AB38` (CEvent); m_task not cleared (0x4B0A6D-0x4B0A8C).
  Then if `flags & 1` (0x4B6449) the inlined event-pool delete on `CPools::ms_pEventPool` [0xB744AC]: `idx = (this - objects) / 0x44`
  (magic 0x78787879, `sar edx,5`, add sign bit: truncation toward zero, 0x4B6456-0x4B646E); `byteMap[idx] |= 0x80` (0x4B6473);
  `if (idx < firstFree (+0xC)) firstFree = idx` (signed `jge` skip, 0x4B6476-0x4B647E). Identical to CEvent::operator delete 0x4B5630.
- gta-reversed: dtor `delete m_task` + pool delete; matches.
