# CTaskComplexWanderFlee (AI) - RE-SPEC-3 group I4

## 0x65D0C0 CTaskComplexWanderFlee::`scalar deleting destructor'  (catalogue: specified/REA, port level 4)
- Name already correct in the catalogue (slot 0 of vtable 0x86F690).
- Convention: `__thiscall CTaskComplexWanderFlee* (this = ECX, uint32 flags [esp+4])`, returns this in EAX, `ret 4`. Callers: none direct (virtual only).
- Behaviour: `CTaskComplexWanderFlee::Destructor 0x65B370(this)` (thiscall, +3); `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`;
  `test byte [esp+8],1` at +8, call at +0x10); return this (`ret 4` at +0x1B).
- Complete dtor callee: 0x65B370 (gr-reversed).
- gta-reversed: compiler-generated (virtual dtor + pool operator delete); matches. Open questions: none.
