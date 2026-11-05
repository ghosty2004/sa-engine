# BreakObject_c (WORLD)

## 0x59d170 BreakObject_c::BreakObject_c  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall BreakObject_c* (this = ECX)`, returns this in EAX, plain `ret`.
- Callers: only as the element ctor passed by `0x59e620` (BreakManager_c ctor) to the CRT
  `eh vector constructor iterator` (64 elements, stride 0x20; see `_static-init.md`, 0x84fa80). Callees: none.
- Behaviour: `byte[this+0] = 0` (`m_JustFaces`), `byte[this+1] = 0` (`m_bActive`),
  `dword[this+8] = 0` (`m_BreakGroups`), in this order. Nothing else is written.
- gta-reversed: provided (BreakObject_c.cpp:22), identical.
- Open questions: none.
