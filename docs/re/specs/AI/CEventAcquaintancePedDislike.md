# CEventAcquaintancePedDislike (AI) - RE-SPEC-FINAL group A

## 0x5FF1B0 CEventAcquaintancePedDislike::~CEventAcquaintancePedDislike  (catalogue: unknown/REA, port level 2)
- Convention: `__thiscall void (this)`. Body: `this->vtable = 0x86CAC8` (the class's own vtable, GetEventType = 37), then tail-jump to the
  CEventAcquaintancePed destructor 0x4AF890 (sets base vtable 0x85B250, releases the +0x14 ped reference via CEntity::CleanUpOldReference
  0x571A00 if non-null, then CEvent vtable 0x85AB38). Used as an SEH unwind action (e.g. in 0x606BA0) and by the deleting destructor.
- gta-reversed: class declared but the dtor is not hooked. Open questions: none.
