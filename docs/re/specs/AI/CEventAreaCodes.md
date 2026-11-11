# CEventAreaCodes (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x43E2C0 CEventAreaCodes::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 55 (0x37) (0x43E2C0).

## 0x4B21F0 CEventAreaCodes::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 67 (0x43) (0x4B21F0).

## 0x4B2200 CEventAreaCodes::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B2200).

## 0x4B6A30 CEventAreaCodes::~CEventAreaCodes  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B2210 FUN_004b2210 (unnamed complete dtor: stores vtbl 0x85B7C0 then 0x85AB38) (0x4B6A34); if `flags & 1` (0x4B6A39) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B6A46-0x4B6A5E); `pool->m_byteMap[idx] |= 0x80` (0x4B6A63); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B6A66-0x4B6A6E). Returns this. Element size 0x44.

## 0x4B7BA0 CEventAreaCodes::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B7BBF); if p == 0 return 0 (0x4B7BD2); else `CEventAreaCodes::Constructor` 0x4B2190 on p with args (this+0xC) (0x4B7BDA) and return its EAX (the ctor returns p).
