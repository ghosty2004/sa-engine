# CSector (WORLD) - RE-SPEC-FINAL group A

## 0x563130 CSector::~CSector  (catalogue: unknown/REA, port level 2)
- Convention: `__thiscall void (CSector* this)`, plain `ret`. SEH frame (handler 0x83CCA8). Used as the element dtor of CWorld::ms_aSectors
  (__init/__dtor 0x84E9B0 / 0x856520).
- Behaviour: SEH 0; `CPtrListDoubleLink::Flush(this+4)` 0x552470 (m_dummies); SEH -1; `CPtrListSingleLink::Flush(this)` 0x552400 (m_buildings).
- gta-reversed: CSector dtor (members destroyed in reverse order) - matches. Open questions: none.
