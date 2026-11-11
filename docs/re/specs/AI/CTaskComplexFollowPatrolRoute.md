# CTaskComplexFollowPatrolRoute (AI) - RE-SPEC-FINAL (helper B)

## 0x6709F0 CTaskComplexFollowPatrolRoute::~CTaskComplexFollowPatrolRoute  (catalogue: unknown/REA, port level 2)
- Complete dtor, `__thiscall void (this)`, plain `ret`; SEH frame. Caller: 0x672BB0 (scalar deleting dtor, slot 0 of 0x870114).
- Behaviour: `vtbl = 0x870114`; `r = this->+0x1C (CPatrolRoute*)`; `if r`: inlined ~CPatrolRoute = `eh_vector_destructor_iterator(r + 4, 0x28, 8, 0x46A380)` 0x821E02, then `0x41B820(r)` (CPatrolRoute::operator delete, pool free; cdecl, 0x670A36). Then CTaskComplex::~CTaskComplex 0x61A3D0. The pointer is not nulled.

## 0x670A70 CTaskComplexFollowPatrolRoute::sub_670A70 (proposed: CTaskComplexFollowPatrolRoute::GetNextSubTaskType)  (catalogue: unknown/REA, port level 1)
- `__thiscall int32 (this)`, plain `ret`. Callers: vt_870114::CreateNextSubTask 0x670D80, CreateFirstSubTask 0x670E20. Fields: +0xC (int16) route mode, +0xE (int16) lap counter, +0x12 (int16) current point index, +0x1C CPatrolRoute* (count at +0).
- Behaviour: `n = route->count`. If `n == 0` -> return 0x516 (TASK_FINISHED). `i = (int32)(int16)+0x12`. If `i + 1 < n` (signed) -> return 0x384 (TASK_SIMPLE_GO_TO_POINT). If `i + 1 == n` -> return 0x387 (TASK_COMPLEX_GO_TO_POINT_AND_STAND_STILL). If `i != n` -> return 0xC8 (TASK_NONE). Else (`i == n`, route completed): `+0xE += 1` (u16); `mode = (int32)(int16)+0xC`; if `(u32)mode > 3` -> return 0xC8; jump table 0x670AEC: mode 0 -> return 0x516; mode 1 -> if the new lap counter != 1 return 0x516, else fall into mode 2; mode 2 -> `0x66D550(route)` (thiscall on the CPatrolRoute: reverses it) then mode 3; mode 3 -> `+0x12 = 0` and tail-call `GetNextSubTaskType(this)` again (`jmp 0x670A70`).

## 0x670B00 CTaskComplexFollowPatrolRoute::CreateSubTask  (catalogue: unknown/REA, port level 3)
- `__thiscall CTask* (this, int32 taskType [esp+4])`, `ret 4`; SEH frame. Fields: +0xC (int16) route mode, +0xE (int16) laps, +0x10 (int16) move state, +0x12 (int16) point index i, +0x14 target radius, +0x18 slow-down distance, +0x1C CPatrolRoute* (`pos[i]` at +0x144 + i*0xC, names at +4 + i*0x28). Allocation failures -> 0.
- By taskType:
  - `0x384` (GO_TO_POINT): `new(0x30) CTaskSimpleGoToPoint((int16)+0x10, &pos[i], +0x14, false, false)` 0x667CD0.
  - `0x191` (NAMED_ANIM): `e = route + 4 + i*0x28`; `new(0x64) CTaskSimpleRunNamedAnim(e (anim name), e + 0x18 (block name), 0x58 (flags), 4.0f, -1, false, false, false, false)` 0x61A990.
  - `0xC8` (TASK_NONE): `new(8)` CTaskSimple + vtable 0x859CFC (CTaskSimpleNone).
  - `0x387` (GO_TO_POINT_AND_STAND_STILL): `d = +0x18`, but `d = 0` when `(mode == 1 && laps == 0) || mode == 2 || mode == 3` (decrement chain 0x670C44-0x670C5F; for mode 1 the laps word +0xE is tested); `new(0x28) CTaskComplexGoToPointAndStandStill((int16)+0x10, &pos[i], +0x14, d, false, false)`.
  - `0x38A` (FOLLOW_NODE_ROUTE): if `route->count == 0` -> 0; if `(int16)i == count` -> `+0x12 -= 1` (u16); `P = pos[i]` (copied); `new(0x60) CTaskComplexFollowNodeRoute((int16)+0x10, &P, [0x86FCA0]=0.5f, [0x86FCA4]=3.0f, [0x86FCA8]=2.0f, false, -1, true)` 0x66EA30.
  - `0x516` or anything else -> 0.

## 0x674930 CTaskComplexFollowPatrolRoute::CTaskComplexFollowPatrolRoute  (catalogue: unknown/REA, port level 2)
- `__thiscall T* (this, int16 moveState [esp+4], const CPatrolRoute* route [esp+8], int16 mode [esp+0xC], float targetRadius [esp+0x10], float slowDownDist [esp+0x14])`, `ret 0x14`; SEH frame. Callers: CRunningScript::ProcessCommands2000To2099 0x472310, Clone 0x674BB0.
- Behaviour: CTaskComplex ctor 0x61A3B0; `+0x10 = (u16)moveState; vtbl = 0x870114; +0xC = (u16)mode; +0x20 &= ~2`; `r = 0x41B810(0x1A4)` (CPatrolRoute::operator new, pool alloc, cdecl); `if r` -> `0x66D440(r)` (CPatrolRoute ctor) else r = 0; `+0x1C = r`; `CPatrolRoute::operator=(r, route)` 0x66D4B0 - called even when r == NULL (original bug: NULL write on pool exhaustion); `+0x14 = targetRadius; +0x18 = slowDownDist` (raw dwords; names per arg position, unverified); `+0x20 |= 1`; `+0x12 = 0 (u16); +0xE = 0 (u16)`; return this.
