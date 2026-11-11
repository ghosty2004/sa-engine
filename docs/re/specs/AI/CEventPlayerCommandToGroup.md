# CEventPlayerCommandToGroup (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B2440 CEventPlayerCommandToGroup::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B2440).

## 0x4B2450 CEventPlayerCommandToGroup::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns false (AL=0; upper EAX bits unchanged) (0x4B2450).

## 0x4B2460 CEventPlayerCommandToGroup::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret`: returns the dword at this+0x18 (0x4B2460).

## 0x4B2520 CEventPlayerCommandToGroup::TakesPriorityOver  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4B2520).

## 0x4B6A80 CEventPlayerCommandToGroup::`scalar deleting destructor'  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B2470 FUN_004b2470 (unnamed complete dtor: stores vtbl 0x85B800 then 0x85AB38) (0x4B6A84); if `flags & 1` (0x4B6A89) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B6A96-0x4B6AAE); `pool->m_byteMap[idx] |= 0x80` (0x4B6AB3); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B6AB6-0x4B6ABE). Returns this. Element size 0x44.
