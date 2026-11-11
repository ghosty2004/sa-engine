# CTaskComplexFallAndStayDown (AI) - RE-SPEC-FINAL (helper B)

## 0x6789D0 sub_6789d0 (proposed: CTaskComplexFallAndStayDown::CTaskComplexFallAndStayDown(int32 animId, int32 time))  (catalogue: unknown/REA, port level 1)
- Evidence: vtable 0x870480 (TASK_COMPLEX_FALL_AND_STAY_DOWN). No callers.
- `__thiscall T* (this, int32 a1 [esp+4], int32 a2 [esp+8])`, `ret 8`: CTaskComplex ctor 0x61A3B0; `+0xC = a1; vtbl = 0x870480; +0x10 = a2`; return this.

## 0x678A10 CTaskComplexFallAndStayDown::sub_678A10 (proposed: CTaskComplexFallAndStayDown::CTaskComplexFallAndStayDown(int32 direction))  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this, int32 dir [esp+4])`, `ret 4`. No callers.
- Behaviour: CTaskComplex ctor 0x61A3B0; `vtbl = 0x870480`; switch on dir (unsigned `cmp 3; ja`, jump table 0x678A80 = {0x678A2E, 0x678A56, 0x678A42, 0x678A6A}): 0 -> `+0xC = 0x18`; 1 -> `+0xC = 0x19`; 2 -> `+0xC = 0x1A`; 3 -> `+0xC = 0x1B`; any other value -> `+0xC` left uninitialised. In all cases `+0x10 = 0`; return this.

## 0x678A90 CTaskComplexFallAndStayDown::~CTaskComplexFallAndStayDown  (catalogue: unknown/REA, port level 2)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x870480` (0x678A90); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x678A96). No other member cleanup.

## 0x678BD0 CTaskComplexFallAndStayDown::CreateSubTask  (catalogue: unknown/REA, port level 2)
- `__thiscall CTask* (this, int32 taskType [esp+4])`, `ret 4`; SEH frame. Caller: vt_870480::CreateNextSubTask 0x67CBE0.
- Behaviour: `taskType == 0xCF` (TASK_SIMPLE_FALL) -> same allocation and inline CTaskSimpleFall init as 0x678B20 (`+0xC = this+0xC; +0x18 = +0x1C = 0x5F5E0FF; +0x10 = this+0x10; vtbl = 0x870430; +8 = 0; +0x14 = 0`); return p (0 on allocation failure). Any other type (including 0x516) -> return 0.
