# CTaskAllocatorAttack (AI) - RE-SPEC-FINAL (helper B)

## 0x69DA00 CTaskAllocatorAttack::~CTaskAllocatorAttack  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor (vtable slot 0), `__thiscall void* (this, u8 flags [esp+4])`, `ret 4`; returns EAX = this. Called virtually from CPedGroupIntelligence (Flush 0x5F7350, SetPrimaryTaskAllocator 0x5F7410, SetEventResponseTaskAllocator 0x5F7440, ProcessIgnorePlayerGroup 0x5F87A0, Process 0x5FC4A0).
- Behaviour: complete dtor 0x69C2D0 (0x69DA03); `if flags & 1`: inlined CTaskAllocator::operator delete = pool free on `[0xB744BC ms_pTaskAllocatorPool]`: `idx = (int32)(this - pool->m_pObjects (+0)) >> 5` (object size 0x20, `sar`); `pool->m_byteMap (+4)[idx] |= 0x80`; `if idx < pool->m_nFirstFree (+0xC)` (signed) -> `m_nFirstFree = idx`.
