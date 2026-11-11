# CTaskAllocatorPlayerCommandAttack (AI) - RE-SPEC-FINAL (helper B)

## 0x69C480 CTaskAllocatorPlayerCommandAttack::CTaskAllocatorPlayerCommandAttack  (catalogue: specified/REA, port level 3)
- `__thiscall T* (this, a1 [esp+4], a2 [esp+8], a3 [esp+0xC])`, `ret 0xC`. Caller: CGroupEventHandler::ComputeResponsePlayerCommand 0x5FB470.
- Behaviour: `player = CWorld::FindPlayerPed(-1)` 0x56E210 (cdecl, `push -1`); `CTaskAllocatorAttack::CTaskAllocatorAttack(this, a1, a2, player)` 0x69C240; `vtbl = 0x870E78; +0x1C = a3`; return this.

## 0x69DB00 CTaskAllocatorPlayerCommandAttack::~CTaskAllocatorPlayerCommandAttack  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor (vtable slot 0), `__thiscall void* (this, u8 flags [esp+4])`, `ret 4`; returns EAX = this. Called virtually from CPedGroupIntelligence (Flush 0x5F7350, SetPrimaryTaskAllocator 0x5F7410, SetEventResponseTaskAllocator 0x5F7440, ProcessIgnorePlayerGroup 0x5F87A0, Process 0x5FC4A0).
- Behaviour: complete dtor 0x69DB40 (0x69DB40 = `vtbl = 0x870E78` then tail-jump to CTaskAllocatorAttack::Destructor 0x69C2D0) (0x69DB03); `if flags & 1`: inlined CTaskAllocator::operator delete = pool free on `[0xB744BC ms_pTaskAllocatorPool]`: `idx = (int32)(this - pool->m_pObjects (+0)) >> 5` (object size 0x20, `sar`); `pool->m_byteMap (+4)[idx] |= 0x80`; `if idx < pool->m_nFirstFree (+0xC)` (signed) -> `m_nFirstFree = idx`.
