# CTaskSimpleInAir (AI) - RE-SPEC-3 group I3

Vtable 0x8704D8 (gr TaskSimpleInAir.h, size 0x3C): +0x1C m_pAnim (CAnimBlendAssociation*), +0x38 m_pClimbEntity (registered reference).

## 0x678D40 CTaskSimpleInAir::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0xF1; ret` -> 241 (TASK_SIMPLE_IN_AIR). Slot 4. gr matches.

## 0x678D50 CTaskSimpleInAir::~CTaskSimpleInAir  (catalogue: specified/REA, port level 1)
- `__thiscall void (this)`, plain `ret`, SEH. Complete-object dtor.
- Behaviour: `vtbl = 0x8704D8`; if `m_pAnim` (+0x1C) != 0 `CAnimBlendAssociation::SetDeleteCallback(m_pAnim, 0x4D1390 (CDefaultAnimCallback::DefaultAnimCB), 0)`
  0x4CEBC0 (0x678D73-0x678D89); if `m_pClimbEntity` (+0x38) != 0 `CEntity::CleanUpOldReference(m_pClimbEntity, &m_pClimbEntity)` 0x571A00
  (0x678D8E-0x678D99); then ~CTaskSimple 0x61A3A0.
- gta-reversed: TaskSimpleInAir.cpp:40 - matches (SetDeleteCallback, not SetFinishCallback).

## 0x67D340 CTaskSimpleInAir::`scalar deleting destructor'  (catalogue name ~CTaskSimpleInAir (second row))  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of vtable 0x8704D8. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x678D50 (0x67D343): see above. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x67D348, call at 0x67D350; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x67C360 CTaskSimpleInAir::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame. `p = CTask::operator new(0x3C)`; NULL -> 0; else return `CTaskSimpleInAir::CTaskSimpleInAir(p, (+0x24 & 1), (+0x24 >> 1) & 1, (+0x24 >> 2) & 1)` 0x678CD0 (each flag zero-extended to a dword; gr names: bUsingJumpGlide, bUsingFallGlide, bUsingClimbJump).
