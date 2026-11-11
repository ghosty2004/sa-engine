# CEventStuckInAir (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B14F0 CEventStuckInAir::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 93 (0x5D) (0x4B14F0).

## 0x4B1500 CEventStuckInAir::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B1500).

## 0x4B1510 CEventStuckInAir::CanBeInterruptedBySameEvent  (catalogue: specified/REA, port level 0)
- `__thiscall bool (0 stack args)`, `ret`: returns true (AL=1; upper EAX bits unchanged) (0x4B1510).

## 0x4B6660 CEventStuckInAir::~CEventStuckInAir  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B1520 FUN_004b1520 (unnamed complete dtor: stores vtbl 0x85B508 then 0x85AB38) (0x4B6664); if `flags & 1` (0x4B6669) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B6676-0x4B668E); `pool->m_byteMap[idx] |= 0x80` (0x4B6693); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B6696-0x4B669E). Returns this. Element size 0x44.

## 0x4B7840 CEventStuckInAir::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B785F); if p == 0 return 0 (0x4B7872); else `CEventStuckInAir::Constructor` 0x4B1490 on p with args (this+0xC) (0x4B787A) and return its EAX (the ctor returns p).
