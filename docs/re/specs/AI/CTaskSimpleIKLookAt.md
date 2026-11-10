# CTaskSimpleIKLookAt (AI) - RE-SPEC-3 group I4

## 0x639490 CTaskSimpleIKLookAt::`scalar deleting destructor'  (catalogue name: CTaskSimpleIKLookAt::~CTaskSimpleIKLookAt; catalogue: specified/REA, port level 6)
- NAME CORRECTION: catalogued as `CTaskSimpleIKLookAt::~CTaskSimpleIKLookAt`; it is slot 0 of the CTaskSimpleIKLookAt vtable 0x86E37C (vt lookup), i.e. the MSVC `scalar deleting
  destructor' (`??_G`), not the complete-object destructor.
- Convention: `__thiscall CTaskSimpleIKLookAt* (this = ECX, uint32 flags [esp+4])`, returns this in EAX, `ret 4`. Callers: none direct (virtual only).
- Behaviour: `CTaskSimpleIKLookAt::Destructor 0x633EF0(this)` (thiscall, +3); `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`;
  `test byte [esp+8],1` at +8, call at +0x10); return this (`ret 4` at +0x1B).
- Complete dtor callee: 0x633EF0 (gr-reversed).
- gta-reversed: compiler-generated (virtual dtor + pool operator delete); matches. Open questions: none.
