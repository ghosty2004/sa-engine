# CEventVehicleDamageWeapon (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x61C320 CEventVehicleDamageWeapon::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 41 (0x29) (0x61C320).

## 0x61C390 CEventVehicleDamageWeapon::~CEventVehicleDamageWeapon  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x61C310 FUN_0061c310 (unnamed complete dtor: stores vtbl 0x86D640) (0x61C393); if `flags & 1` (0x61C398) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x61C3A0); returns this (0x61C3A8).
