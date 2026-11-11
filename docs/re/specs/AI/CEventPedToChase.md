# CEventPedToChase (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4AF190 CEventPedToChase::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 25 (0x19) (0x4AF190).

## 0x4AF1A0 CEventPedToChase::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AF1A0).

## 0x4AF1B0 CEventPedToChase::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 1 (0x1) (0x4AF1B0).

## 0x4B61C0 CEventPedToChase::~CEventPedToChase  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4AF1C0 CEventPedToChase::~CEventPedToChase (0x4B61C4); if `flags & 1` (0x4B61C9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B61D6-0x4B61EE); `pool->m_byteMap[idx] |= 0x80` (0x4B61F3); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B61F6-0x4B61FE). Returns this. Element size 0x44.

## 0x4B7360 CEventPedToChase::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B737F); if p == 0 return 0 (0x4B7392); else `CEventPedToChase::CEventPedToChase` 0x4AF130 on p with args (this+0xC) (0x4B739A) and return its EAX (the ctor returns p).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4AF130 CEventPedToChase::CEventPedToChase  (catalogue: specified/REA, port level 2)
- `__thiscall (this, CPed* ped [esp+4])`, `ret 4`, SEH: +4 = 0, +8 u8 = 0, vtbl = 0x85B138 (0x4AF162), +0xC m_ped = ped; if non-null `RegisterReference(ped, &m_ped)`
  0x571B70 (0x4AF16D). Returns this.

## 0x4AF1C0 CEventPedToChase::~CEventPedToChase  (catalogue: specified/REA, port level 1)
- Complete dtor, SEH: vtbl = 0x85B138; if +0xC `CleanUpOldReference(m_ped, &m_ped)` 0x571A00 (0x4AF1F6); vtbl = 0x85AB38.

## 0x4AF220 CEventPedToChase::AffectsPed  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this unused, CPed* ped [esp+4])`, `ret 4`: AL = `ped->IsAlive()` 0x5E0170 != 0 (`setne`, 0x4AF22B).
