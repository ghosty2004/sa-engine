# CTaskSimpleCarWaitToSlowDown (AI) - RE-SPEC-3 group I4

## 0x64B440 CTaskSimpleCarWaitToSlowDown::`scalar deleting destructor'  (catalogue name: CTaskSimpleCarWaitToSlowDown::~CTaskSimpleCarWaitToSlowDown; catalogue: specified/REA, port level 2)
- NAME CORRECTION: catalogued as `CTaskSimpleCarWaitToSlowDown::~CTaskSimpleCarWaitToSlowDown`; it is slot 0 of the CTaskSimpleCarWaitToSlowDown vtable 0x86ED98 (vt lookup), i.e. the MSVC `scalar deleting
  destructor' (`??_G`), not the complete-object destructor.
- Convention: `__thiscall CTaskSimpleCarWaitToSlowDown* (this = ECX, uint32 flags [esp+4])`, returns this in EAX, `ret 4`. Callers: none direct (virtual only).
- Behaviour: `CTaskSimpleCarWaitToSlowDown::Destructor 0x646A00(this)` (thiscall, +3); `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`;
  `test byte [esp+8],1` at +8, call at +0x10); return this (`ret 4` at +0x1B).
- Complete dtor callee: 0x646A00 (gr-reversed).
- gta-reversed: compiler-generated (virtual dtor + pool operator delete); matches. Open questions: none.
