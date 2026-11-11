# CEventFireNearby (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B1F50 CEventFireNearby::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 59 (0x3B) (0x4B1F50).

## 0x4B1F60 CEventFireNearby::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B1F60).

## 0x4B1F70 CEventFireNearby::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 17 (0x11) (0x4B1F70).

## 0x4B1F80 CEventFireNearby::~CEventFireNearby  (catalogue: specified/REA, port level 0)
- Complete destructor, `__thiscall void ()`: only `this->vtbl = 0x85AB38` (0x4B1F80); `ret`.

## 0x4B1FC0 CEventFireNearby::TakesPriorityOver  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4B1FC0).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B1F90 CEventFireNearby::AffectsPed  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this unused, CPed* ped [esp+4])`, `ret 4`: if `ped->m_pIntelligence (+0x47C)->m_TaskMgr (+4).FindActiveTaskByType(0x25C)` 0x681740
  (TASK_COMPLEX_EXTINGUISH_FIRES = 604) != 0 -> return 0 (AL, 0x4B1FAC); else return `ped->IsAlive()` 0x5E0170 (AL passed through, 0x4B1FB4).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B6940 CEventFireNearby::~CEventFireNearby  (catalogue: specified/REA, port level 1)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls 0x4B1F80 (complete dtor: `vtbl = 0x85AB38` only) with ECX = this; ECX is not
  saved and is reused as `this` after the call (0x4B6956). If flags & 1: event-pool free inlined (pool [0xB744AC]; `idx = (int32)(this - m_pObjects) / 0x44`, magic
  0x78787879 >> 5; `m_byteMap[idx] |= 0x80`; `if (idx < m_nFirstFree) m_nFirstFree = idx`, 0x4B694D-0x4B6980). Returns this.

## 0x4B7A70 CEventFireNearby::CloneEditable  (catalogue: specified/REA, port level 1)
- `__thiscall CEventEditableResponse* ()`, `ret`, no SEH: `p = CEventPool::New([0xB744AC])` 0x4B5570; null -> 0 (0x4B7A82). Inline ctor: p+4 = 0, p+8 = 0, p+0xC = 1,
  p+0xE i16 = 200, p+0x10 i16 = -1, vtbl = 0x85B6E8, p+0x14..0x1C = this+0x14..0x1C (CVector m_position copied as 3 dwords) (0x4B7A84-0x4B7AB3). Returns p.
