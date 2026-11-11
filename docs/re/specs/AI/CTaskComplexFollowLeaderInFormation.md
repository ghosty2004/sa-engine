# CTaskComplexFollowLeaderInFormation (AI) - RE-SPEC-AI-2 group J2

Vtable 0x870C3C, size 0x28. Layout: +0xC CPedGroup* (not ref-counted), +0x10 leader CPed* (registered ref), +0x14 CVector offset, +0x20 i32 (= 4 at
construction, move state), +0x24 f32/i32 (4th ctor arg, copied raw).

## 0x6949A0 CTaskComplexFollowLeaderInFormation::Constructor  (catalogue: specified/REA, port level 2)
- `__thiscall (this, CPedGroup* group [esp+4], CPed* leader [+8], const CVector* offset [+0xC], f32 a4 [+0x10])`, `ret 0x10`, SEH frame; returns this.
  Entry is obfuscated (`jmp 0x4015C3`: pushes `[0x1560658] - [0x5CFF64]` with EDI/flags preserved, i.e. an obfuscated `push <SEH handler>`) - no effect.
- CTaskComplex::CTaskComplex 0x61A3B0; +0xC = group; vtbl = 0x870C3C; +0x10 = leader; +0x14 = *offset (3 dwords); +0x20 = 4; +0x24 = a4 (raw dword)
  (0x6949C4-0x694A06); if leader `leader->RegisterReference(&+0x10)` 0x571B70.

## 0x694A30 CTaskComplexFollowLeaderInFormation::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x391` (0x694A30); `ret` -> returns 913 (TASK_COMPLEX_FOLLOW_LEADER_IN_FORMATION). Vtable: slot 4 (+0x10) of vtable 0x870C3C (CTaskComplexFollowLeaderInFormation).

## 0x695740 CTaskComplexFollowLeaderInFormation::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* ()`, plain `ret`, SEH: `p = new(0x28)`; if p: `Constructor(+0xC, +0x10, &+0x14, +0x24)` 0x6949A0; returns p or 0 (+0x20 is not
  copied; the ctor resets it to 4).

## 0x695BC0 CTaskComplexFollowLeaderInFormation::~CTaskComplexFollowLeaderInFormation  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexFollowLeaderInFormation::Destructor 0x694A40 (0x695BC3); if `flags & 1` (0x695BC8) calls CTask::operator delete 0x61A5B0 (this) (0x695BD0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870C3C (CTaskComplexFollowLeaderInFormation).
