# CEventLeanOnVehicle (AI) - RE-SPEC-3 group I3

Vtable 0x86F878 (gr EventLeanOnVehicle.h, size 0x14): CEvent (0xC), +0xC m_vehicle (CVehicle*, registered reference),
+0x10 m_leanAnimDurationInMs (i32). EVENT_LEAN_ON_VEHICLE = 92 (0x5C).

## 0x65DB50 CEventLeanOnVehicle::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x5C; ret` -> 92. Slot 1 of 0x86F878. gr matches.

## 0x65DB70 CEventLeanOnVehicle::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* (const this)`, plain `ret`, SEH frame. Slot 4.
- Behaviour: `p = CEvent::operator new(0x14)` 0x4B5620 (0x65DB8B); if p == 0 return 0 (0x65DB97-0x65DBA1, 0x65DBC6).
  Else `return CEventLeanOnVehicle::CEventLeanOnVehicle(p, m_vehicle [+0xC], m_leanAnimDurationInMs [+0x10])` 0x65DAF0
  (thiscall, args pushed +0x10 then +0xC, 0x65DBA3-0x65DBAD; its EAX is returned).
- gta-reversed: header inline `new CEventLeanOnVehicle(m_vehicle, m_leanAnimDurationInMs)` - matches.

## 0x65DBF0 CEventLeanOnVehicle::`scalar deleting destructor'  (catalogue name ~CEventLeanOnVehicle)  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of 0x86F878. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this.
- Calls complete dtor 0x65DC10 (0x65DBF3): SEH frame; `vtbl = 0x86F878`; if `m_vehicle` (+0xC) != 0
  `CEntity::CleanUpOldReference(m_vehicle, &m_vehicle)` 0x571A00 (0x65DC33-0x65DC46); then `CEvent::~CEvent` 0x4AC010
  (`vtbl = 0x85AB38`). Then `if (flags & 1) CEvent::operator delete(this)` 0x4B5630 (0x65DBF8-0x65DC05).
- gta-reversed: EventLeanOnVehicle.cpp dtor cleans the vehicle reference - matches.

## 0x65DE10 CEventLeanOnVehicle::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0xC; ret` -> 12. Slot 2. gr matches.
