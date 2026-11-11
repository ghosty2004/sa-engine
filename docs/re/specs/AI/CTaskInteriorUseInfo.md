# CTaskInteriorUseInfo (AI) - RE-SPEC-FINAL (helper B)

## 0x675B60 CTaskInteriorUseInfo::CreateFirstSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped unused)`, `ret 4`; SEH frame.
- Behaviour: `info = this->+0xC`; if NULL return 0 (EAX = info). `info->+1 = 1` (u8, in-use flag; 0x675B95). `p = CTask::operator new(0x18)`; NULL -> 0. Else CTaskComplex ctor; `p+0x10 = this+0x10; p+0x14 = (u8)this+0x18; p+0xC = info; p->vtbl = 0x8702BC` (TASK_INTERIOR_GOTO_INFO, inline CTaskInteriorGoToInfo ctor); return p.
