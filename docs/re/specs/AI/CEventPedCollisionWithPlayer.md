# CEventPedCollisionWithPlayer (AI) - RE-SPEC-3 group I3

Vtable 0x86C970. Layout = CEventPedCollisionWithPed (gr EventPedCollisionWithPed.h, size 0x34): CEvent (0xC), +0xC m_pieceType (i16),
+0x10 m_damageIntensity (f32), +0x14 m_victim (CPed*), +0x18 m_collisionImpactVelocity (CVector), +0x24 m_collisionPos (CVector),
+0x30 m_movestate (i16), +0x32 m_victimMoveState (i16). EVENT_PED_COLLISION_WITH_PLAYER = 3.

## 0x5FED90 CEventPedCollisionWithPlayer::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,3; ret` -> 3. Slot 1 of 0x86C970. gr matches.

## 0x5FEDA0 CEventPedCollisionWithPlayer::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* (const this)`, plain `ret`, SEH frame. Slot 4.
- Behaviour: `p = CEvent::operator new(0x34)` 0x4B5620 (0x5FEDBC); if p == 0 return 0 (0x5FEDCA-0x5FEDD2).
  Else call `CEventPedCollisionWithPed::CEventPedCollisionWithPed(p, pieceType, damage, victim, &impactVel, &pos, moveState,
  victimMoveState)` 0x4AC990 (thiscall, 7 stack args) with pieceType = `(u16)[+0xC]` zero-extended (0x5FEDE8-0x5FEDEA),
  damage = `fld dword [+0x10]` / `fstp dword [esp]` (x87 round trip of an f32: value-exact; an SNaN would be quieted, 0x5FEDD8,
  0x5FEDF6), victim = [+0x14], &[+0x18], &[+0x24], moveState = `movsx [+0x30]`, victimMoveState = `movsx [+0x32]`
  (0x5FEDD4-0x5FEDDB). Then overwrite `p->vtbl = 0x86C970` (0x5FEDFF) and return p.
- gta-reversed: header inline `new CEventPedCollisionWithPlayer(...)` (its ctor calls the base ctor then sets the vtable) - matches.

## 0x5FEE20 CEventPedCollisionWithPlayer::`scalar deleting destructor'  (catalogue name ~CEventPedCollisionWithPlayer)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of 0x86C970. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this.
- Calls 0x5FED80 (0x5FEE23): `vtbl = 0x86C970; jmp 0x4ACA70` = ~CEventPedCollisionWithPed (SEH; `vtbl = 0x85AC68`; if
  `m_victim` (+0x14) != 0 `CEntity::CleanUpOldReference(m_victim, &m_victim)` 0x571A00; `vtbl = 0x85AB38`).
  Then `if (flags & 1) CEvent::operator delete(this)` 0x4B5630 (0x5FEE28-0x5FEE35).
- gta-reversed: `= default` + base dtor - matches.
