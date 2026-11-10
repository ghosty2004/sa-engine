# CEventPotentialWalkIntoPed (AI) - RE-SPEC-3 group I2
Vtable 0x85AF60, type 0xD (13). Layout (gr, size 0x28): CEventEditableResponse (0x14), +0x14 m_targetPoint CVector, +0x20 m_ped CPed*,
+0x24 m_moveState i32.

## 0x4AE760 CEventPotentialWalkIntoPed::GetEventType  (catalogue: specified/REA, port level 0)
- returns 0xD.

## 0x4AE770 CEventPotentialWalkIntoPed::GetLifeTime  (catalogue: specified/REA, port level 0)
- returns 0.

## 0x4AE780 CEventPotentialWalkIntoPed::GetSourceEntity  (catalogue: specified/REA, port level 0)
- returns `[this+0x20]` (m_ped).

## 0x4AE790 CEventPotentialWalkIntoPed::GetEventPriority  (catalogue: specified/REA, port level 0)
- returns 0x1C (28).

## 0x4B5F90 CEventPotentialWalkIntoPed::`scalar deleting destructor'  (catalogue: specified/REA, port level 2)
- call 0x4AE7A0; `if (flags & 1)` inlined event-pool delete; return this; `ret 4`.

## 0x4B7090 CEventPotentialWalkIntoPed::CloneEditable  (catalogue: specified/REA, port level 3)
- `p = CEventPool::New(...)`; if null return 0; else `CEventPotentialWalkIntoPed(p, m_ped, &m_targetPoint, m_moveState)` 0x4AE6E0 (args
  pushed: moveState, &point, ped -> (ped, &point, moveState)); return it. Slot 16. gr inline - matches.
