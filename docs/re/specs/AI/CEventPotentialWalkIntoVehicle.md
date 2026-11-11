# CEventPotentialWalkIntoVehicle (AI) - RE-SPEC-3 group I3

Vtable 0x85AED0 (gr EventPotentialWalkIntoVehicle.h, size 0x1C): CEventEditableResponse (0x14), +0x14 m_vehicle (CVehicle*,
registered reference), +0x18 m_moveState (i32). EVENT_POTENTIAL_WALK_INTO_VEHICLE = 56 (0x38).

## 0x4AE390 CEventPotentialWalkIntoVehicle::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x38; ret` -> 56. Slot 1 of 0x85AED0. gr matches.

## 0x4AE3B0 CEventPotentialWalkIntoVehicle::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x1D; ret` -> 29. Slot 2. gr matches.

## 0x4B5EF0 CEventPotentialWalkIntoVehicle::`scalar deleting destructor'  (catalogue name ~CEventPotentialWalkIntoVehicle)  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of 0x85AED0. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this.
- Calls complete dtor 0x4AE3C0 (0x4B5EF4): SEH; `vtbl = 0x85AED0`; if `m_vehicle` (+0x14) != 0
  `CEntity::CleanUpOldReference(m_vehicle, &m_vehicle)` 0x571A00 (0x4AE3E3-0x4AE3F6); `vtbl = 0x85AB38`.
  Then if `flags & 1` the inlined event-pool delete on `ms_pEventPool` [0xB744AC] (same 30-insn shape as 0x4B6260/0x4B59D0;
  stride 0x44, magic 0x78787879).
- gta-reversed: matches.

## 0x4B6FB0 CEventPotentialWalkIntoVehicle::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall CEventEditableResponse* (const this)`, plain `ret`, SEH. Slot 16.
- Behaviour: `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B6FC9-0x4B6FCF); if p == 0 return 0 (0x4B7007).
  Else `return CEventPotentialWalkIntoVehicle::CEventPotentialWalkIntoVehicle(p, m_vehicle [+0x14], m_moveState [+0x18])` 0x4AE320
  (0x4B6FE4-0x4B6FEE). Editable-response fields of the clone are the ctor defaults.
- gta-reversed: header inline `new CEventPotentialWalkIntoVehicle(m_vehicle, m_moveState)` - matches.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4AE3A0 CEventPotentialWalkIntoVehicle::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AE3A0).

## 0x4AE5C0 CEventPotentialWalkIntoVehicle::TakesPriorityOver  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4AE5C0).
