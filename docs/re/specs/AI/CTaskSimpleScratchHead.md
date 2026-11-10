# CTaskSimpleScratchHead (AI) - RE-SPEC-3 group I4

Vtable 0x85A100 (set by Clone at 0x48DFAE). Derives from CTaskSimpleRunAnim (size 0x20).

## 0x48DF60 CTaskSimpleScratchHead::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (const this)`, plain `ret`, SEH frame (handler 0x8384FB). `this` is not read.
- Behaviour: `p = CTask::operator new(0x20)` 0x61A5A0 (0x48DF79). If p == 0 return 0 (0x48DF8F). Else
  `CTaskSimpleRunAnim::CTaskSimpleRunAnim(p, group = 0, anim = 0x87 (135), blendDelta = 4.0f (imm 0x40800000), taskType = 0x1A5
  (421), name = "ScratchHead" (0x85A128), holdLastFrame = 0)` 0x61A900 (arguments pushed right-to-left at 0x48DF91-0x48DFA6);
  then `vtbl = 0x85A100` (0x48DFAE); return p.
- gta-reversed: TaskSimpleScratchHead.h:7/10 `new CTaskSimpleScratchHead()` = RunAnim(ANIM_GROUP_DEFAULT, ANIM_ID_XPRESSSCRATCH,
  4.0, TASK_SIMPLE_SCRATCH_HEAD, "ScratchHead", false): matches.

## 0x48E910 CTaskSimpleScratchHead::`scalar deleting destructor'  (catalogue name: ~CTaskSimpleScratchHead; catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x85A100 -> scalar deleting destructor.
- `__thiscall this (uint32 flags [esp+4])`, `ret 4`. Calls 0x48E930 (a `jmp 0x61A6F0` thunk = CTaskSimpleAnim::~CTaskSimpleAnim;
  ScratchHead and RunAnim have no own dtor body), then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0; returns this.
- gta-reversed: `= default` dtor; matches.
