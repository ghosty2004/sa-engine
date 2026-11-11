# CRepeatSector (WORLD) - RE-SPEC-FINAL group A

## 0x5631B0 CRepeatSector::~CRepeatSector  (catalogue: unknown/REA, port level 0)
- Convention: `__thiscall void (CRepeatSector* this)`, plain `ret`. Element dtor of CWorld::ms_aRepeatSectors (0x84E9E0 / 0x856540).
- Behaviour: `` `eh vector destructor iterator'(this, 4, 3, CPtrListDoubleLink::~CPtrListDoubleLink 0x5630F0) `` (CRT 0x821E02, __stdcall,
  destroys the 3 lists in reverse order: +8, +4, +0).
- gta-reversed: compiler-generated - matches. Open questions: none.
