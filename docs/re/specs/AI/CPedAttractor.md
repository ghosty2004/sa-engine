# CPedAttractor (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x5EC4B0 CPedAttractor::~CPedAttractor  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5EC410 CPedAttractor::Destructor (0x5EC4B4); if `flags & 1` (0x5EC4B9) frees the slot in the event pool `*(CPool*)[0xB744C4]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0xC4` (signed magic division 0x5397829D >> 6, rounding toward zero, 0x5EC4C6-0x5EC4DE); `pool->m_byteMap[idx] |= 0x80` (0x5EC4E3); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x5EC4E6-0x5EC4EE). Returns this. Element size 0xC4.

## 0x5EE830 CPedAttractor::~CPedAttractor  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5EE880 FUN_005ee880 (unnamed complete dtor: stores vtbl 0x86C5B4) (0x5EE834); if `flags & 1` (0x5EE839) frees the slot in the event pool `*(CPool*)[0xB744C4]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0xC4` (signed magic division 0x5397829D >> 6, rounding toward zero, 0x5EE846-0x5EE85E); `pool->m_byteMap[idx] |= 0x80` (0x5EE863); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x5EE866-0x5EE86E). Returns this. Element size 0xC4.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5EAFE0 CPedAttractor::operator delete  (catalogue: specified/REA, port level 0)
- `__cdecl void (void* p [esp+4])`, `ret`: frees the slot in the attractor pool `*[0xB744C4]` (ms_pPedAttractorPool): `idx = (int32)(p - pool->m_pObjects) / 0xC4` (signed
  magic 0x5397829D >> 6, 0x5EAFEB-0x5EAFFF), `pool->m_byteMap[idx] |= 0x80` (0x5EB004), `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (0x5EB007-0x5EB00C).
