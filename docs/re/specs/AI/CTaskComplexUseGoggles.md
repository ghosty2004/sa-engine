# CTaskComplexUseGoggles (AI) - RE-SPEC-AI-2 group J5

## 0x634F20 CTaskComplexUseGoggles::~CTaskComplexUseGoggles  (catalogue: specified/REA, port level 2)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86E4A8` (CTaskComplexUseGoggles) (0x634F20); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x634F26). No other member cleanup.

## 0x639680 CTaskComplexUseGoggles::~CTaskComplexUseGoggles  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexUseGoggles::~CTaskComplexUseGoggles 0x634F20 (0x639683); if `flags & 1` (0x639688) calls CTask::operator delete 0x61A5B0 (this) (0x639690, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86E4A8 (CTaskComplexUseGoggles).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x634EF0 CTaskComplexUseGoggles::CTaskComplexUseGoggles  (catalogue: specified/REA, port level 1)
- `__thiscall T* (this)`, plain `ret`. Callers: CRunningScript::ProcessCommands2500To2599 0x47A760, CTaskSimplePlayerOnFoot::ProcessPlayerWeapon 0x6859A0.
- Behaviour: CTaskComplex ctor 0x61A3B0; `vtbl = 0x86E4A8`; return this.
