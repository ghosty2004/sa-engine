# CTaskSimplePlayerOnFoot (AI) - RE-SPEC-3 group I4

## 0x68B0C0 CTaskSimplePlayerOnFoot::`scalar deleting destructor'  (catalogue name: CTaskSimplePlayerOnFoot::~CTaskSimplePlayerOnFoot; catalogue: specified/REA, port level 2)
- NAME CORRECTION: catalogued as `CTaskSimplePlayerOnFoot::~CTaskSimplePlayerOnFoot`; it is slot 0 of the CTaskSimplePlayerOnFoot vtable 0x8708EC (vt lookup), i.e. the MSVC `scalar deleting
  destructor' (`??_G`), not the complete-object destructor.
- Convention: `__thiscall CTaskSimplePlayerOnFoot* (this = ECX, uint32 flags [esp+4])`, returns this in EAX, `ret 4`. Callers: none direct (virtual only).
- Behaviour: `CTaskSimplePlayerOnFoot::Destructor 0x6857D0(this)` (thiscall, +3); `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`;
  `test byte [esp+8],1` at +8, call at +0x10); return this (`ret 4` at +0x1B).
- Complete dtor callee: 0x6857D0 (gr-reversed). Note gr TaskSimplePlayerOnFoot.h:59 labels 0x68B0C0 as the destructor; it is the deleting wrapper.
- gta-reversed: compiler-generated (virtual dtor + pool operator delete); matches. Open questions: none.
