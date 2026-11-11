# CTaskComplexAttractorPartnerWait (AI) - RE-SPEC-AI-2 group J5

## 0x633290 CTaskComplexAttractorPartnerWait::~CTaskComplexAttractorPartnerWait  (catalogue: specified/REA, port level 2)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86E284` (CTaskComplexAttractorPartnerWait) (0x633290); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x633296). No other member cleanup.

## 0x638F00 CTaskComplexAttractorPartnerWait::~CTaskComplexAttractorPartnerWait  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexAttractorPartnerWait::~CTaskComplexAttractorPartnerWait 0x633290 (0x638F03); if `flags & 1` (0x638F08) calls CTask::operator delete 0x61A5B0 (this) (0x638F10, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86E284 (CTaskComplexAttractorPartnerWait).
