# CEventPedCollisionWithPed (AI) - RE-SPEC-3 group I2
Vtable 0x85AC68, type 2. Layout (gr, size 0x34): +0xC m_pieceType i16, +0x10 m_damageIntensity f32, +0x14 m_victim CPed*,
+0x18 m_collisionImpactVelocity CVector, +0x24 m_collisionPos CVector, +0x30 m_movestate i16, +0x32 m_victimMoveState i16.

## 0x4ACA40 CEventPedCollisionWithPed::GetEventType  (catalogue: specified/REA, port level 0)
- returns 2.

## 0x4ACA50 CEventPedCollisionWithPed::GetLifeTime  (catalogue: specified/REA, port level 0)
- returns 0 (`xor eax,eax`). Shared slot 3 with CEventPedCollisionWithPlayer 0x86C970 and CEventPlayerCollisionWithPed.

## 0x4ACA60 CEventPedCollisionWithPed::GetEventPriority  (catalogue: specified/REA, port level 0)
- returns 0x3C (60). Shared slot 2 with the two other collision-with-ped vtables.

## 0x4B5980 CEventPedCollisionWithPed::`scalar deleting destructor'  (catalogue: specified/REA, port level 2)
- call 0x4ACA70 (complete dtor); `if (flags & 1)` inlined event-pool delete; return this; `ret 4`.

## 0x4B6C40 CEventPedCollisionWithPed::Clone  (catalogue: specified/REA, port level 3)
- `p = CEventPool::New(...)`; if null return 0; else `CEventPedCollisionWithPed::CEventPedCollisionWithPed(p, (u16)m_pieceType,
  m_damageIntensity (raw), m_victim, &m_collisionImpactVelocity, &m_collisionPos, (i16)m_movestate, (i16)m_victimMoveState)` 0x4AC990;
  return its EAX. gr header inline Clone - matches.
