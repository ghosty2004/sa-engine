# CTaskComplexFollowPointRoute (AI) - RE-SPEC-3 group I4

## 0x671610 CTaskComplexFollowPointRoute::`scalar deleting destructor'  (catalogue name: CTaskComplexFollowPointRoute::~CTaskComplexFollowPointRoute; catalogue: specified/REA, port level 3)
- NAME CORRECTION: catalogued as `CTaskComplexFollowPointRoute::~CTaskComplexFollowPointRoute`; it is slot 0 of the CTaskComplexFollowPointRoute vtable 0x870078 (vt lookup), i.e. the MSVC `scalar deleting
  destructor' (`??_G`), not the complete-object destructor.
- Convention: `__thiscall CTaskComplexFollowPointRoute* (this = ECX, uint32 flags [esp+4])`, returns this in EAX, `ret 4`. Callers: none direct (virtual only).
- Behaviour: `0x66E050(this)` (thiscall, +3); `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`;
  `test byte [esp+8],1` at +8, call at +0x10); return this (`ret 4` at +0x1B).
- Complete dtor callee: 0x66E050 (catalogue unknown; proposed CTaskComplexFollowPointRoute::~CTaskComplexFollowPointRoute): SEH frame; `vtbl = 0x870078` (0x66E06D); if `m_pRoute` (+0x20, CPointRoute*) != 0 `CPointRoute::operator delete(m_pRoute)` 0x41B7D0 (cdecl; no dtor call, CPointRoute is POD; 0x66E073-0x66E088); then `CTaskComplex::~CTaskComplex` 0x61A3D0 (0x66E095). gr TaskComplexFollowPointRoute dtor `delete m_route` matches.
- gta-reversed: compiler-generated (virtual dtor + pool operator delete); matches. Open questions: none.
