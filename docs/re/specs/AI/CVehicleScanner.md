# CVehicleScanner (AI) - RE-SPEC-3 group I4

Derives from CEntityScanner (gr EntityScanner.h, size 0x50: +0x4C m_pClosestEntityInRange).

## 0x5FFE60 CVehicleScanner::ScanForVehiclesInRange  (catalogue: specified/REA, port level 3)
- `__thiscall void (CVehicleScanner* this, CPed* ped [esp+4])`, `ret 4`.
- Callers: CPedIntelligence::Process 0x608260. Callees: CPed::IsAlive 0x5E0170, CEntityScanner::ScanForEntitiesInRange 0x5FFA20
  (`(this, int32 sectorList = 0, CPed* ped)`, 0 = REPEATSECTOR_VEHICLES), CEntity::CleanUpOldReference 0x571A00,
  CEntity::RegisterReference 0x571B70.
- Behaviour:
  1. If `!ped->IsAlive()` return (0x5FFE6A-0x5FFE71).
  2. If `ped->m_pVehicle (+0x58C) != 0` AND `ped flags (+0x46C) bit 8` (bInVehicle; `test ah,1`, 0x5FFE7D-0x5FFE86):
     a. if `ped->m_nCreatedBy (byte +0x484) == 2` (PED_MISSION) `ScanForEntitiesInRange(0, ped)` (0x5FFE88-0x5FFE96);
     b. if `m_pClosestEntityInRange (+0x4C) != 0` `CleanUpOldReference(m_pClosest, &m_pClosest)` (0x5FFE9B-0x5FFEA6);
     c. `m_pClosest = ped->m_pVehicle` (re-read, 0x5FFEAB-0x5FFEB3); if non-null `RegisterReference(m_pClosest, &m_pClosest)` (0x5FFEB8).
  3. Else `ScanForEntitiesInRange(0, ped)` (0x5FFEC2-0x5FFEC7).
- gta-reversed: VehicleScanner.cpp:6 matches the bytes. Open questions: none.
