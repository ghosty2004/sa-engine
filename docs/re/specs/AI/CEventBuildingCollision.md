# CEventBuildingCollision (AI) - RE-SPEC-3 group I1

Vtable 0x85ACE8. Layout (gr EventBuildingCollision.h, size 0x30): CEvent (0xC), +0xC m_pieceType i16, +0xE m_moveState i16,
+0x10 m_damageIntensity f32, +0x14 m_building CBuilding* (registered ref), +0x18 m_impactNormal CVector, +0x24 m_impactPos CVector.
Ctor 0x4ACF00 (gr-reversed, hooked): `__thiscall (i16 pieceType, f32 damageIntensity, CBuilding*, const CVector* normal,
const CVector* pos, i16 moveState)`, `ret 0x18`.

## 0x4ACFB0 CEventBuildingCollision::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 6 (EVENT_BUILDING_COLLISION). Slot 1.

## 0x4ACFC0 CEventBuildingCollision::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `xor eax,eax` -> 0. Slot 3 (shared "return 0" body).

## 0x4ACFD0 CEventBuildingCollision::CanBeInterruptedBySameEvent  (catalogue: specified/REA, port level 0)
- `__thiscall bool ()`: `mov al,1` -> true. Slot 15.

## 0x4ACFE0 CEventBuildingCollision::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 0x3B (59). Slot 2.

## 0x4AD050 CEventBuildingCollision::TakesPriorityOver  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this, const CEvent* other [esp+4])`, `ret 4`. Slot 11.
- Behaviour: `o = other->GetEventType()` (vtbl+4, called first, 0x4AD05A); `t = this->GetEventType()` (0x4AD063);
  `return o != t` (`setne al`, 0x4AD066-0x4AD069). I.e. it never takes priority over another building collision and always
  over any other event type (priorities are not compared). Upper EAX bytes = garbage from the virtual call (AL only).
- gta-reversed: header inline `return GetEventType() != refEvent.GetEventType()` - same result.

## 0x4B5A20 CEventBuildingCollision::`scalar deleting destructor'  (catalogue: specified/REA, port level 2)
- `__thiscall this (uint32 flags [esp+4])`, `ret 4`. Slot 0.
- Calls the complete dtor 0x4ACFF0 (SEH frame; `vtbl = 0x85ACE8`; if `m_building` (+0x14) != 0
  `CEntity::CleanUpOldReference(m_building, &this->m_building)` 0x571A00; `vtbl = 0x85AB38`), then if `flags & 1` frees the
  slot with the inlined CEvent::operator delete on `ms_pEventPool` [0xB744AC] (same code as 0x4B5630: idx = (this-objects)/0x44,
  `byteMap[idx] |= 0x80`, `if (idx < firstFree) firstFree = idx`, 0x4B5A30-0x4B5A5E). Returns this.

## 0x4B6D40 CEventBuildingCollision::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* (const this)`, plain `ret`, SEH frame (unwind state 0 around the ctor -> on exception the slot is freed).
- Behaviour: `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B6D59-0x4B6D5F); if null return 0 (0x4B6DA7).
  Else `CEventBuildingCollision::CEventBuildingCollision(p, (u16)m_pieceType [+0xC], m_damageIntensity [+0x10] (raw dword),
  m_building [+0x14], &m_impactNormal [+0x18], &m_impactPos [+0x24], (i16)m_moveState [+0xE])` 0x4ACF00 and return its
  EAX (= p) (0x4B6D74-0x4B6D92). The new event's m_nTimeActive/m_bValid come from the ctor (0 / false), not copied.
- gta-reversed: header inline `new CEventBuildingCollision(m_pieceType, m_damageIntensity, m_building, m_impactNormal,
  m_impactPos, m_moveState)` - matches.
