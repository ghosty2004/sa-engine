# CTaskSimpleRunAnim (AI) - RE-SPEC-3 group I3

## 0x48DEA0 CTaskSimpleRunAnim::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `movzx eax, word [ecx+0x1C]; ret` -> the u16 task type stored at +0x1C (gr m_TaskType), ZERO-extended.
  Shared (ICF) slot 4 of many CTaskSimpleRunAnim-derived vtables (0x86D528 CTaskSimpleRunAnim, CTaskSimpleCower 0x85A0D0, CTaskSimpleUseAtm
  0x85A134, CTaskSimpleHitFromBack 0x86DB70, CTaskSimpleHitFromLeft 0x86DBA4, ...). gr returns the stored type - matches.
