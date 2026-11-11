# CEventCarUpsideDown (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B1D20 CEventCarUpsideDown::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 45 (0x2D) (0x4B1D20).

## 0x4B1D30 CEventCarUpsideDown::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B1D30).

## 0x4B1D40 CEventCarUpsideDown::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 18 (0x12) (0x4B1D40).

## 0x4B67F0 CEventCarUpsideDown::~CEventCarUpsideDown  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B1D50 FUN_004b1d50 (unnamed complete dtor: stores vtbl 0x85B660 then 0x85AB38) (0x4B67F4); if `flags & 1` (0x4B67F9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B6806-0x4B681E); `pool->m_byteMap[idx] |= 0x80` (0x4B6823); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B6826-0x4B682E). Returns this. Element size 0x44.

## 0x4B79D0 CEventCarUpsideDown::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B79EF); if p == 0 return 0 (0x4B7A02); else `CEventCarUpsideDown::Constructor` 0x4B1CC0 on p with args (this+0xC) (0x4B7A0A) and return its EAX (the ctor returns p).
