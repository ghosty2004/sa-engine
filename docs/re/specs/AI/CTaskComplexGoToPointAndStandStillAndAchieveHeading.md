# CTaskComplexGoToPointAndStandStillAndAchieveHeading (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x66DFB0 CTaskComplexGoToPointAndStandStillAndAchieveHeading::~CTaskComplexGoToPointAndStandStillAndAchieveHeading  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x668D40 CTaskComplexGoToPointAndStandStillAndAchieveHeading::Destructor (0x66DFB3); if `flags & 1` (0x66DFB8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x66DFC0); returns this (0x66DFC8).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x668E80 CTaskComplexGoToPointAndStandStillAndAchieveHeading::ControlSubTask  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`.
- Behaviour: `if +0x2C & 1` (new target flag) -> return `this->CreateFirstSubTask(ped)` (vslot +0x24, 0x668E92). Else: `if m_pSubTask->GetTaskType() == 0x384` (TASK_SIMPLE_GO_TO_POINT) -> `m_pSubTask->+8 = this->+0xC` (the move state is pushed down into the subtask, 0x668EAE). Return m_pSubTask.
