# CEventKnockOffBike (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4AFCB0 CEventKnockOffBike::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 8 (0x8) (0x4AFCB0).

## 0x4AFCC0 CEventKnockOffBike::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AFCC0).

## 0x4AFCD0 CEventKnockOffBike::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: returns ST0 = 60.0f [0x858B34] (0x4AFCD0).

## 0x4AFCE0 CEventKnockOffBike::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 70 (0x46) (0x4AFCE0).

## 0x4B06F0 CEventKnockOffBike::IsCriminalEvent  (catalogue: specified/REA, port level 0)
- `__thiscall bool (0 stack args)`, `ret`: returns true (AL=1; upper EAX bits unchanged) (0x4B06F0).

## 0x4B63A0 CEventKnockOffBike::~CEventKnockOffBike  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4AFE70 FUN_004afe70 (unnamed complete dtor: stores vtbl 0x85B2E0 then 0x85AB38) (0x4B63A4); if `flags & 1` (0x4B63A9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B63B6-0x4B63CE); `pool->m_byteMap[idx] |= 0x80` (0x4B63D3); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B63D6-0x4B63DE). Returns this. Element size 0x44.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B7590 CEventKnockOffBike::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH: `p = CEventPool::New([0xB744AC])`; null -> 0 (0x4B75C2). Else (0x4B75C4-0x4B7603) calls 0x4AFCF0 (11 args, in order):
  `(this+0x38 m_vehicle, &this+0xC m_moveSpeed, &this+0x18 m_collisionImpactVelocity, this+0x24 m_damageIntensity (raw f32), this+0x28 field_28 (raw f32),
  (int32)(int8)this+0x36 m_knockOffType (movsx), (u8)this+0x34 m_knockOffDirection, this+0x2C m_time, this+0x30 m_ped, this+0x35 & 1 (m_isVictimDriver),
  (this+0x35 & 2) >> 1 (m_forceKnockOff))`. Returns p. Note: knockOffType is sign-extended by the clone (0x4B75E2) but the ctor takes a byte; no effect.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x4B4AA0 CEventKnockOffBike::CEventKnockOffBike(const CEventKnockOffBike&)  (catalogue: unknown/REA, port level 3)
- Name correction: behaves as `CEventKnockOffBike::operator=(const CEventKnockOffBike&)` (no vtable store, no base init): `__thiscall CEventKnockOffBike* (this, const CEventKnockOffBike* other [esp+4])`, `ret 4`. Caller: CEventHandler::ComputeKnockOffBikeResponse 0x4B9FF0.
- Behaviour: `CEventKnockOffBike::From(this, other)` 0x4AFDD0 (0x4B4AA8); return this (0x4B4AAD).
