# CTaskComplexPresentIDToCop (AI) - RE-SPEC-FINAL (helper B)

## 0x6591D0 CTaskComplexPresentIDToCop::CTaskComplexPresentIDToCop  (catalogue: unknown/REA, port level 2)
- `__thiscall T* (this, CPed* cop [esp+4])`, `ret 4`; SEH frame. Caller: vt_86f544::Clone 0x659B60.
- Behaviour: CTaskComplex ctor 0x61A3B0; `vtbl = 0x86F544` (TASK_COMPLEX_PRESENT_ID_TO_COP); `+0xC = cop`; `CEntity::RegisterReference(cop, &this->+0xC)` 0x571B70 (0x65920A) - NOT null-checked (ECX = cop).

## 0x659240 CTaskComplexPresentIDToCop::~CTaskComplexPresentIDToCop  (catalogue: unknown/REA, port level 2)
- Complete dtor, `__thiscall void (this)`, plain `ret`; SEH frame. Caller: scalar deleting dtor 0x65A0F0.
- Behaviour: `vtbl = 0x86F544`; `if +0xC (cop)` -> CleanUpOldReference(cop, &this->+0xC) 0x571A00; CTaskComplex::~CTaskComplex 0x61A3D0.

## 0x65A110 sub_65a110 (proposed: CTaskComplexPresentIDToCop::CreateSubTask)  (catalogue: unknown/REA, port level 2)
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`; SEH frame. Callers: vt_86f544 CreateNextSubTask 0x65AFE0 (2), CreateFirstSubTask 0x65B050, ControlSubTask 0x65B070.
- Behaviour: `taskType == 0x19D` (TASK_SIMPLE_HANDS_UP): `p = CTask::operator new(0x34)`; NULL -> 0; `CTaskSimpleRunTimedAnim::CTaskSimpleRunTimedAnim(p, 0, 0x8E, 4.0f, -4.0f, 3000 (0xBB8), 0x19D, "HandsUp" [0x85A2C4], false)` 0x61ABE0; `p->vtbl = 0x85A29C` (CTaskSimpleHandsUp); return p. `taskType == 0x386` (TASK_SIMPLE_ACHIEVE_HEADING): `p = CTask::operator new(0x18)`; NULL -> 0; `h = 0x6592A0(this, ped)` (heading to the cop, ST0) stored as the first f32 arg; return `CTaskSimpleAchieveHeading::CTaskSimpleAchieveHeading(p, h, [0x86FC7C]=0.5f, [0x86FC80]=0.2f)` 0x667E20. Any other value (including 0x516) -> 0.
