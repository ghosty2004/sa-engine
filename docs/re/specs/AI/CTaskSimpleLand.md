# CTaskSimpleLand (AI) - RE-SPEC-3 group I3

Vtable 0x8704FC (gr TaskSimpleLand.h, size 0x14): +8 m_pAnim (CAnimBlendAssociation*), +0xC m_nAnimId, +0x10 flags.

## 0x678ED0 CTaskSimpleLand::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0xF2; ret` -> 242 (TASK_SIMPLE_LAND). Slot 4. gr matches.

## 0x678EE0 CTaskSimpleLand::~CTaskSimpleLand  (catalogue: specified/REA, port level 1)
- `__thiscall void (this)`, plain `ret`, SEH. Complete-object dtor.
- Behaviour: `vtbl = 0x8704FC`; if `m_pAnim` (+8) != 0 `CAnimBlendAssociation::SetFinishCallback(m_pAnim, 0x4D1390 (DefaultAnimCB), 0)` 0x4CEBE0
  (0x678F03-0x678F19); then ~CTaskSimple 0x61A3A0.
- gta-reversed: TaskSimpleLand.cpp:30 - matches.

## 0x67D360 CTaskSimpleLand::`scalar deleting destructor'  (catalogue name ~CTaskSimpleLand (second row))  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of vtable 0x8704FC. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x678EE0 (0x67D363): see above. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x67D368, call at 0x67D370; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x67C3F0 CTaskSimpleLand::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame.
- Behaviour: `p = CTask::operator new(0x14)`; NULL -> 0. Else `id = this->+0xC (anim id)`; CTaskSimple ctor 0x61A390; `p+0x10 = (p+0x10 & 0xFC) | ((((id == -1) & 1) | 2) << 1)` - i.e. bit 0 cleared, bit 1 = (id == -1), bit 2 set, bits 3-7 left as the uninitialised heap byte (bit 2 of the old byte is overwritten by the OR); `p+0xC = id; p->vtbl = 0x8704FC; p+8 = 0`; return p.
