# CObjectScanner (AI) - RE-SPEC-FINAL group A

## 0x5FFF30 CObjectScanner::ScanForObjectsInRange  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (CObjectScanner* this, CPed* ped [esp+4])`, `ret 4`. Caller 0x606890.
- Behaviour: if `ped->m_pVehicle (+0x58C) != 0` and `ped->flags (+0x46C) & 0x100` (bInVehicle) return. If `!ped->IsAlive()` (0x5E0170) return.
  Else `CEntityScanner::ScanForEntitiesInRange(this, 2 /*REPEATSECTOR_OBJECTS*/, ped)` 0x5FFA20 called directly (non-virtual, `ret 8`).
- gta-reversed: ObjectScanner.cpp:5 - same logic. Open questions: none.

## 0x606010 CObjectScanner::CObjectScanner  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CObjectScanner* (this)`, plain `ret`, returns this (EAX). Caller 0x606890 (stack object).
- Behaviour (store order): +4 = 0, +8 = 0, +0xC..+0x48 (16 dwords m_apEntities) = 0 (`rep stosd`), +0x4C m_pClosestEntityInRange = 0,
  +8 = 0x10 (16, second dword of the CTickCounter m_timer), vtable = 0x86CD8C.
- gta-reversed: ObjectScanner.h:7 `CEntityScanner()` (inline; its +8 = 16 value comes from the CEntityScanner ctor body, check against gr
  EntityScanner.cpp when porting). Open questions: none.

## 0x606050 CObjectScanner::~CObjectScanner  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall void (this)`; vtable = 0x86CD40 (CEntityScanner base vtable), then tail-jump to CEntityScanner::Clear 0x5FF9D0
  (releases the 16 m_apEntities references and +0x4C via CEntity::CleanUpOldReference, zeroing each). Used as an SEH unwind action of 0x606890.
- gta-reversed: ObjectScanner.h:8 `= default` (CEntityScanner dtor calls Clear) - matches. Open questions: none.
