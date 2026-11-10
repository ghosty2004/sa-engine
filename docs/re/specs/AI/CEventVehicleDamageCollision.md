# CEventVehicleDamageCollision (AI) - RE-SPEC-3 group I3

Vtable 0x8710D0. Layout = CEventVehicleDamage (gr EventVehicleDamage.h): CEventEditableResponse (0x14), +0x14 m_vehicle,
+0x18 m_attacker (both registered references, cleaned by 0x4B1990). EVENT_VEHICLE_DAMAGE_COLLISION = 73 (0x49).

## 0x6A0660 CEventVehicleDamageCollision::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x49; ret` -> 73. Slot 1 of 0x8710D0. gr matches.

## 0x6A06D0 CEventVehicleDamageCollision::`scalar deleting destructor'  (catalogue name ~CEventVehicleDamageCollision)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of 0x8710D0. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this.
- Calls 0x6A0650 (0x6A06D3): `vtbl = 0x8710D0; jmp 0x4B1990` = ~CEventVehicleDamage (SEH; `vtbl = 0x85B588`; if [+0x14] != 0
  `CEntity::CleanUpOldReference([+0x14], this+0x14)` 0x571A00; if [+0x18] != 0 same for +0x18; `vtbl = 0x85AB38`).
  Then `if (flags & 1) CEvent::operator delete(this)` 0x4B5630 (0x6A06D8-0x6A06E5).
- gta-reversed: `= default` + base dtor - matches.
