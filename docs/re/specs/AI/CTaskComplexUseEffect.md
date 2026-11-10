# CTaskComplexUseEffect (AI) - RE-SPEC-3 group I4

## 0x638570 CTaskComplexUseEffect::`scalar deleting destructor'  (catalogue name: CTaskComplexUseEffect::~CTaskComplexUseEffect; catalogue: specified/REA, port level 16)
- NAME CORRECTION: catalogued as `CTaskComplexUseEffect::~CTaskComplexUseEffect`; it is slot 0 of the CTaskComplexUseEffect vtable 0x86E0D8 (vt lookup), i.e. the MSVC `scalar deleting
  destructor' (`??_G`), not the complete-object destructor.
- Convention: `__thiscall CTaskComplexUseEffect* (this = ECX, uint32 flags [esp+4])`, returns this in EAX, `ret 4`. Callers: none direct (virtual only).
- Behaviour: `CTaskComplexUseEffect::Destructor 0x632270(this)` (thiscall, +3); `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`;
  `test byte [esp+8],1` at +8, call at +0x10); return this (`ret 4` at +0x1B).
- Complete dtor callee: 0x632270 (gr-reversed).
- gta-reversed: compiler-generated (virtual dtor + pool operator delete); matches. Open questions: none.
