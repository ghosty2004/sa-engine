# CTaskSimpleGoToPoint (AI) - RE-SPEC-3 group I4

## 0x66D6F0 CTaskSimpleGoToPoint::`scalar deleting destructor'  (catalogue name: CTaskSimpleGoToPoint::~CTaskSimpleGoToPoint; catalogue: specified/REA, port level 2)
- NAME CORRECTION: catalogued as `CTaskSimpleGoToPoint::~CTaskSimpleGoToPoint`; it is slot 0 of the CTaskSimpleGoToPoint vtable 0x86FD50 (vt lookup), i.e. the MSVC `scalar deleting
  destructor' (`??_G`), not the complete-object destructor.
- Convention: `__thiscall CTaskSimpleGoToPoint* (this = ECX, uint32 flags [esp+4])`, returns this in EAX, `ret 4`. Callers: none direct (virtual only).
- Behaviour: `0x667D50(this)` (thiscall, +3); `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`;
  `test byte [esp+8],1` at +8, call at +0x10); return this (`ret 4` at +0x1B).
- Complete dtor callee: 0x667D50 (catalogue unknown): `vtbl = 0x86FD1C` then tail-jump to 0x61A3A0 (`vtbl = 0x86D48C`, the CTaskSimple dtor). 0x86FD1C is not CTaskSimpleGoToPoint's vtable (0x86FD50) but its base's (CTaskSimpleGoTo, gr TaskSimpleGoTo.cpp:8 RH_ScopedVirtualClass 0x86fd1c; so 0x667D50 = CTaskSimpleGoTo::~CTaskSimpleGoTo; CTaskSimpleGoToPoint has no own dtor body, its trivial dtor was folded into the base call). Port: Drop of GoToPoint = base GoTo drop = nothing beyond vtable writes.
- gta-reversed: compiler-generated (virtual dtor + pool operator delete); matches. Open questions: none.
