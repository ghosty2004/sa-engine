# CEventDraggedOutCar (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4AD2E0 CEventDraggedOutCar::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 7 (0x7) (0x4AD2E0).

## 0x4AD2F0 CEventDraggedOutCar::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AD2F0).

## 0x4AD300 CEventDraggedOutCar::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret`: returns the dword at this+0x14 (0x4AD300).

## 0x4AD310 CEventDraggedOutCar::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: returns ST0 = 100.0f [0x858628] (0x4AD310).

## 0x4AD320 CEventDraggedOutCar::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 40 (0x28) (0x4AD320).

## 0x4B5A70 CEventDraggedOutCar::~CEventDraggedOutCar  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4AD330 CEventDraggedOutCar::~CEventDraggedOutCar (0x4B5A74); if `flags & 1` (0x4B5A79) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B5A86-0x4B5A9E); `pool->m_byteMap[idx] |= 0x80` (0x4B5AA3); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B5AA6-0x4B5AAE). Returns this. Element size 0x44.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B6DC0 CEventDraggedOutCar::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B6DDF); if p == 0 return 0 (0x4B6DF2); else `CEventDraggedOutCar::CEventDraggedOutCar` 0x4AD250 on p with args (this+0x18, this+0x14, (u8)this+0x1C (zero-extended) (byte, mov)) (0x4B6E04) and return its EAX (the ctor returns p).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4AD250 CEventDraggedOutCar::CEventDraggedOutCar  (catalogue: specified/REA, port level 2)
- `__thiscall CEventDraggedOutCar* (this, CVehicle* vehicle [esp+4], CPed* carjacker [esp+8], bool isDriverSeat [esp+0xC])`, `ret 0xC`, SEH.
- Base inline: +4 = 0, +8 u8 = 0, +0xC u8 = 1, +0xE i16 = 200, +0x10 i16 = -1 (0x4AD26E-0x4AD283); +0x14 = carjacker (0x4AD29A); vtbl = 0x85AD30; +0x18 = vehicle;
  +0x1C u8 = arg3 (0x4AD2AF). `RegisterReference` 0x571B70 for the vehicle (+0x18) first if non-null (0x4AD2B5), then for the carjacker (+0x14) if non-null (0x4AD2C1).
  Returns this. (gr field names: +0x14 m_CarJacker, +0x18 m_Vehicle, +0x1C m_IsDriverSeat.)

## 0x4AD330 CEventDraggedOutCar::~CEventDraggedOutCar  (catalogue: specified/REA, port level 1)
- Complete dtor, SEH: vtbl = 0x85AD30; `CleanUpOldReference` 0x571A00 on +0x18 (vehicle) if non-null (0x4AD366), then on +0x14 (carjacker) if non-null (0x4AD376); vtbl = 0x85AB38.

## 0x4AD3A0 CEventDraggedOutCar::AffectsPed  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`: if `!ped->IsAlive()` 0x5E0170 return its AL (0, 0x4AD3B1); else return `this+0x14 (carjacker) != 0` (`setne al`, 0x4AD3B9).

## 0x4AD3C0 CEventDraggedOutCar::AffectsPedGroup  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this unused, CPedGroup* group [esp+4])`, `ret 4`: `leader = (group+8 membership).GetLeader()` 0x5F69A0; `pl = CWorld::FindPlayerPed(-1)`
  0x56E210 (cdecl, 0x4AD3CD-0x4AD3D8); returns AL = (leader == pl) (`sub; neg; sbb; inc`, 0x4AD3D6-0x4AD3E0).
