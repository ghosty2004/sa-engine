# CEventObjectCollision (AI) - RE-SPEC-3 group I3

Vtable 0x85ACA8 (gr EventObjectCollision.h, size 0x30): CEvent (0xC), +0xC m_pieceType (i16), +0xE m_moveState (i16),
+0x10 m_damageIntensity (f32), +0x14 m_object (CObject*, registered reference), +0x18 m_impactNormal (CVector),
+0x24 m_impactPos (CVector). EVENT_OBJECT_COLLISION = 5.

## 0x4ACDA0 CEventObjectCollision::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,5; ret` -> 5. Slot 1 of 0x85ACA8. gr matches.

## 0x4ACDC0 CEventObjectCollision::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x39; ret` -> 57. Slot 2. gr matches.

## 0x4B59D0 CEventObjectCollision::`scalar deleting destructor'  (catalogue name ~CEventObjectCollision)  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of 0x85ACA8. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this.
- Calls complete dtor 0x4ACDD0 (0x4B59D4): SEH frame (prologue obfuscated by the no-CD build: `push -1; jmp 0x406A96`, which
  pushes an XOR-decoded handler and returns to 0x4ACDD7; no game-visible effect); `vtbl = 0x85ACA8`; if `m_object` (+0x14) != 0
  `CEntity::CleanUpOldReference(m_object, &m_object)` 0x571A00 (0x4ACDF3-0x4ACE06); `vtbl = 0x85AB38`.
  Then if `flags & 1` (0x4B59D9) the inlined event-pool delete on `ms_pEventPool` [0xB744AC] (same code as 0x4B5630,
  0x4B59E0-0x4B5A0E).
- gta-reversed: EventObjectCollision.cpp dtor - matches.

## 0x4B6CC0 CEventObjectCollision::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* (const this)`, plain `ret`, SEH frame. Slot 4.
- Behaviour: `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (operator new inlined, 0x4B6CD9-0x4B6CDF); if p == 0
  return 0 (0x4B6CE8-0x4B6CF2, 0x4B6D2B). Else `return CEventObjectCollision::CEventObjectCollision(p, pieceType, damage, object,
  &normal, &pos, moveState)` 0x4ACCF0 with: pieceType = `(u16)[+0xC]` zero-extended into a dword (0x4B6D08-0x4B6D0A),
  damageIntensity = raw dword copy of [+0x10] (no x87, bit-exact, 0x4B6D04), object = [+0x14], &this->m_impactNormal (+0x18),
  &this->m_impactPos (+0x24), moveState = `movsx [+0xE]` (0x4B6CF4). Push order 0x4B6CF8-0x4B6D11 (right to left).
- gta-reversed: header inline `new CEventObjectCollision(m_pieceType, m_damageIntensity, m_object, m_impactNormal, m_impactPos,
  m_moveState)` - matches.
