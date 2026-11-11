# CEventSeenPanickedPed (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B20F0 CEventSeenPanickedPed::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 65 (0x41) (0x4B20F0).

## 0x4B2100 CEventSeenPanickedPed::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B2100).

## 0x4B2110 CEventSeenPanickedPed::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret`: returns the dword at this+0x14 (0x4B2110).

## 0x4B2120 CEventSeenPanickedPed::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 13 (0xD) (0x4B2120).

## 0x4B69E0 CEventSeenPanickedPed::~CEventSeenPanickedPed  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B2130 FUN_004b2130 (unnamed complete dtor: stores vtbl 0x85B778 then 0x85AB38) (0x4B69E4); if `flags & 1` (0x4B69E9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B69F6-0x4B6A0E); `pool->m_byteMap[idx] |= 0x80` (0x4B6A13); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B6A16-0x4B6A1E). Returns this. Element size 0x44.

## 0x4B7B30 CEventSeenPanickedPed::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B7B4F); if p == 0 return 0 (0x4B7B62); else `CEventSeenPanickedPed::Constructor` 0x4B2080 on p with args (this+0x14) (0x4B7B6A) and return its EAX (the ctor returns p).
