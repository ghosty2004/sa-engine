# CEventInWater (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x4B1370 CEventInWater::CEventInWater (proposed; catalogue: unnamed, unknown/REA, port level 0)
- Name evidence: the vtable stored at 0x4B137E is 0x85B4C8, whose slot 1 (0x4B1390, `mov eax, 0x42`) returns
  event type 66 = `EVENT_IN_WATER` (gr `Enums/eEventType.h:73`) and slot 3 (0x4B13B0) returns 62, the
  priority gr gives `CEventInWater` (`Events/EventInWater.h`). Size 0x10 matches gr `VALIDATE_SIZE(CEventInWater, 0x10)`.
  Callers: CPed::ProcessBuoyancy 0x5E1FA0 (2 sites), which constructs the event on the stack.
- Convention: `__thiscall CEventInWater*(float acceleration [esp+4])`, `ret 4`, returns `this` in EAX.
- Behaviour (inlined CEvent base ctor + derived ctor):
  1. `this->m_nTimeActive (+0x4, int32) = 0` (0x4B1374), `this->m_bValid (+0x8, u8) = 0` (0x4B1377).
  2. `this->vtable (+0x0) = 0x85B4C8` (0x4B137E).
  3. `this->m_acceleration (+0xC, f32) = arg` (raw dword copy via ECX, 0x4B137A/0x4B1384; no x87).
  The padding bytes +0x9..+0xB are not written.
- gta-reversed: `CEventInWater::CEventInWater(float)` in `Events/EventInWater.cpp:5` - same behaviour.
