# CPedATMAttractor (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x5EE4D0 CPedATMAttractor::GetType  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x5EE4D0).

## 0x5EE4E0 CPedATMAttractor::~CPedATMAttractor  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5EE530 FUN_005ee530 (unnamed complete dtor: stores vtbl 0x86C550) (0x5EE4E4); if `flags & 1` (0x5EE4E9) frees the slot in the event pool `*(CPool*)[0xB744C4]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0xC4` (signed magic division 0x5397829D >> 6, rounding toward zero, 0x5EE4F6-0x5EE50E); `pool->m_byteMap[idx] |= 0x80` (0x5EE513); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x5EE516-0x5EE51E). Returns this. Element size 0xC4.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5EE470 CPedATMAttractor::CPedATMAttractor  (catalogue: specified/REA, port level 4)
- `__thiscall (this, C2dEffectPedAttractor* fx [esp+4], CEntity* entity [esp+8], int32 moveState [esp+0xC])`, `ret 0xC`: calls `CPedAttractor::Constructor` 0x5EDFB0 (0x5ee4b4) with (fx=fx [esp+4], entity=entity [esp+8], moveState=moveState [esp+0xC], maxNoOfPeds=5, spacing=1.0f [0x86C3EC], achieveQueueTime=30000.0f [0x86C3F0], achieveQueueShuffleTime=3000.0f [0x86C3F4], arriveRange=0.200000002f [0x86C3F8], headingRange=0.150000005f [0x86C3FC], deltaPos=0.100000001f [0x86C400], deltaHeading=0.100000001f [0x86C404]) - float args are pushed as raw f32 dwords loaded from .rdata; then `this->vtbl = 0x86C550`; returns this.
