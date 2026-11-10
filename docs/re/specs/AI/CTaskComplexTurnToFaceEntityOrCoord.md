# CTaskComplexTurnToFaceEntityOrCoord (AI) - RE-SPEC-3 group I4

## 0x670830 CTaskComplexTurnToFaceEntityOrCoord::`scalar deleting destructor'  (catalogue name: CTaskComplexTurnToFaceEntityOrCoord::~CTaskComplexTurnToFaceEntityOrCoord; catalogue: specified/REA, port level 3)
- NAME CORRECTION: catalogued as `CTaskComplexTurnToFaceEntityOrCoord::~CTaskComplexTurnToFaceEntityOrCoord`; it is slot 0 of the CTaskComplexTurnToFaceEntityOrCoord vtable 0x86FF94 (vt lookup), i.e. the MSVC `scalar deleting
  destructor' (`??_G`), not the complete-object destructor.
- Convention: `__thiscall CTaskComplexTurnToFaceEntityOrCoord* (this = ECX, uint32 flags [esp+4])`, returns this in EAX, `ret 4`. Callers: none direct (virtual only).
- Behaviour: `CTaskComplexTurnToFaceEntityOrCoord::Destructor 0x66B960(this)` (thiscall, +3); `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`;
  `test byte [esp+8],1` at +8, call at +0x10); return this (`ret 4` at +0x1B).
- Complete dtor callee: 0x66B960 (gr-reversed).
- gta-reversed: compiler-generated (virtual dtor + pool operator delete); matches. Open questions: none.
