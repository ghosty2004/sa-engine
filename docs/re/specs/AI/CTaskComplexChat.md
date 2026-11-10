# CTaskComplexChat (AI) - RE-SPEC-3 group I4

## 0x6846D0 CTaskComplexChat::`scalar deleting destructor'  (catalogue: specified/REA, port level 3)
- Name already correct in the catalogue (slot 0 of vtable 0x8706A0).
- Convention: `__thiscall CTaskComplexChat* (this = ECX, uint32 flags [esp+4])`, returns this in EAX, `ret 4`. Callers: none direct (virtual only).
- Behaviour: `CTaskComplexChat::Destructor 0x682D40(this)` (thiscall, +3); `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`;
  `test byte [esp+8],1` at +8, call at +0x10); return this (`ret 4` at +0x1B).
- Complete dtor callee: 0x682D40 (gr-reversed CTaskComplexChat::~CTaskComplexChat, ported from gr).
- gta-reversed: compiler-generated (virtual dtor + pool operator delete); matches. Open questions: none.
