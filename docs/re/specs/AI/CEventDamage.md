# CEventDamage (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B5CF0 CEventDamage::~CEventDamage  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4AD960 FUN_004ad960 (unnamed complete dtor: stores vtbl 0x85AD78 then 0x85AB38) (0x4B5CF4); if `flags & 1` (0x4B5CF9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B5D06-0x4B5D1E); `pool->m_byteMap[idx] |= 0x80` (0x4B5D23); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B5D26-0x4B5D2E). Returns this. Element size 0x44.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4ADD60 CEventDamage::WasHeadShot  (catalogue: specified/REA, port level 0)
- `__thiscall void (this, bool* out [esp+4])`, `ret 4`. `*out = 0` (0x4ADD64); `w = this+0x1C` (m_weaponType); if w > 0x34 (unsigned, 0x4ADD6D) return.
  Index table 0x4ADDAC (53 bytes) -> dword table 0x4ADDA0:
  - w in {0..15, 46} -> 0x4ADD7D: if this+0x41 (m_damageResponse.m_bForceDeath) `*out = 1`.
  - w in {22..34, 38, 52} -> 0x4ADD8A: if this+0x41 and `this+0x20 == 9` (m_pedPieceType == PED_PIECE_HEAD, 0x4ADD91) `*out = 1`.
  - all other w (16..21, 35..37, 39..45, 47..51) -> return.
- gta-reversed: not hooked; gr inline helpers are not a reference for this. Open questions: none.
