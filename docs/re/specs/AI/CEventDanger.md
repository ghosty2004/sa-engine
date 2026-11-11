# CEventDanger (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B2670 CEventDanger::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 75 (0x4B) (0x4B2670).

## 0x4B2680 CEventDanger::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B2680).

## 0x4B2690 CEventDanger::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 20 (0x14) (0x4B2690).

## 0x4B6AD0 CEventDanger::~CEventDanger  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B26A0 FUN_004b26a0 (unnamed complete dtor: stores vtbl 0x85B848 then 0x85AB38) (0x4B6AD4); if `flags & 1` (0x4B6AD9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B6AE6-0x4B6AFE); `pool->m_byteMap[idx] |= 0x80` (0x4B6B03); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B6B06-0x4B6B0E). Returns this. Element size 0x44.

## 0x4B7C10 CEventDanger::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B7C2F); if p == 0 return 0 (0x4B7C42); else `CEventDanger::Constructor` 0x4B2600 on p with args (this+0x14, this+0x18) (0x4B7C4E) and return its EAX (the ctor returns p).
