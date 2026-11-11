# CEventPotentialWalkIntoObject (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4AE640 CEventPotentialWalkIntoObject::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 44 (0x2C) (0x4AE640).

## 0x4AE650 CEventPotentialWalkIntoObject::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AE650).

## 0x4AE660 CEventPotentialWalkIntoObject::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 27 (0x1B) (0x4AE660).

## 0x4AE6D0 CEventPotentialWalkIntoObject::TakesPriorityOver  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4AE6D0).

## 0x4B5F40 CEventPotentialWalkIntoObject::~CEventPotentialWalkIntoObject  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4AE670 FUN_004ae670 (unnamed complete dtor: stores vtbl 0x85AF18 then 0x85AB38) (0x4B5F44); if `flags & 1` (0x4B5F49) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B5F56-0x4B5F6E); `pool->m_byteMap[idx] |= 0x80` (0x4B5F73); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B5F76-0x4B5F7E). Returns this. Element size 0x44.

## 0x4B7020 CEventPotentialWalkIntoObject::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B703F); if p == 0 return 0 (0x4B7052); else `CEventPotentialWalkIntoObject::Constructor` 0x4AE5D0 on p with args (this+0x14, this+0x18) (0x4B705E) and return its EAX (the ctor returns p).
