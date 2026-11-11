# CPedPizzaAttractor (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x5EE740 CPedPizzaAttractor::GetType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 3 (0x3) (0x5EE740).

## 0x5EE760 CPedPizzaAttractor::~CPedPizzaAttractor  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5EE7B0 FUN_005ee7b0 (unnamed complete dtor: stores vtbl 0x86C598) (0x5EE764); if `flags & 1` (0x5EE769) frees the slot in the event pool `*(CPool*)[0xB744C4]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0xC4` (signed magic division 0x5397829D >> 6, rounding toward zero, 0x5EE776-0x5EE78E); `pool->m_byteMap[idx] |= 0x80` (0x5EE793); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x5EE796-0x5EE79E). Returns this. Element size 0xC4.
> QA (2026-11-11): mislabelled pool. [0xB744C4] is `CPools::ms_pPedAttractorPool` (gta-reversed Pools.cpp:41), not the event pool (ms_pEventPool is 0xB744AC, Pools.cpp:38); the inlined delete is the CPedAttractor pool's operator delete, not CEvent::operator delete. Address, element size 0xC4, magic 0x5397829D/sar 6, `|= 0x80` and the signed `jge` first-free update are correct as written; a port must free into the ped-attractor pool.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5EE6E0 CPedPizzaAttractor::CPedPizzaAttractor  (catalogue: specified/REA, port level 4)
- `__thiscall (this, C2dEffectPedAttractor* fx [esp+4], CEntity* entity [esp+8], int32 moveState [esp+0xC])`, `ret 0xC`: calls `CPedAttractor::Constructor` 0x5EDFB0 (0x5ee724) with (fx=fx [esp+4], entity=entity [esp+8], moveState=moveState [esp+0xC], maxNoOfPeds=5, spacing=1.0f [0x86C44C], achieveQueueTime=30000.0f [0x86C450], achieveQueueShuffleTime=3000.0f [0x86C454], arriveRange=0.200000002f [0x86C458], headingRange=0.100000001f [0x86C45C], deltaPos=0.100000001f [0x86C460], deltaHeading=0.100000001f [0x86C464]) - float args are pushed as raw f32 dwords loaded from .rdata; then `this->vtbl = 0x86C598`; returns this.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x5EE750 CPedPizzaAttractor::GetHeadOfQueueWaitTime  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, plain `ret`: `fld [0x86C468]=2000.0f` -> returns 2000.0f in ST0 (0x5EE750). Vtable slot of CPedPizzaAttractor (no direct callers).

## 0x5EE7B0 CPedPizzaAttractor::~CPedPizzaAttractor  (catalogue: unknown/REA, port level 3)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86C598` (0x5EE7B0); tail-jump to base dtor CPedAttractor::Destructor 0x5EC410 (0x5EE7B6). No other member cleanup.
