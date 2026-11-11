# CTaskComplexFallToDeath (AI) - RE-SPEC-AI-2 group J2

## 0x6790A0 CTaskComplexFallToDeath::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x115` (0x6790A0); `ret` -> returns 277 (TASK_COMPLEX_FALL_TO_DEATH). Vtable: slot 4 (+0x10) of vtable 0x870520 (CTaskComplexFallToDeath).

## 0x6790B0 CTaskComplexFallToDeath::~CTaskComplexFallToDeath  (catalogue: specified/REA, port level 2)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x870520` (CTaskComplexFallToDeath) (0x6790B0); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x6790B6). No other member cleanup.

## 0x67C480 CTaskComplexFallToDeath::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* ()`, plain `ret`, SEH: `p = new(0x24)`; if p: `CTaskComplexFallToDeath((u8)+0x20 zero-extended, &+0xC, (+0x21 >> 3) & 1,
  (+0x21 >> 4) & 1)` 0x679040 (0x67C4B3-0x67C4DB; the two flag bits are extracted from byte +0x21 as `and 0xFFFFFF01` of the shifted byte);
  returns p or 0.

## 0x67D550 CTaskComplexFallToDeath::~CTaskComplexFallToDeath  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexFallToDeath::~CTaskComplexFallToDeath 0x6790B0 (0x67D553); if `flags & 1` (0x67D558) calls CTask::operator delete 0x61A5B0 (this) (0x67D560, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870520 (CTaskComplexFallToDeath).
