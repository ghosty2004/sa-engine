# CEventAttractor (AI) - RE-SPEC-3 group I3

Vtable 0x85B1B8 (gr EventAttractor.h, size 0x20): CEventEditableResponse (0x14), +0x14 m_2dEffect (C2dEffectPedAttractor*),
+0x18 m_entity (CEntity*, registered reference), +0x1C m_bAvoidLookingAtAttractor (u8). EVENT_ATTRACTOR = 27 (0x1B).

## 0x4AF3D0 CEventAttractor::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x1B; ret` -> 27. Slot 1 of 0x85B1B8. gr matches.

## 0x4AF3F0 CEventAttractor::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `xor eax,eax; ret` -> 0. Slot 2 of 0x85B1B8 AND of CEventScriptedAttractor 0x86C9F0 (shared body). gr matches.

## 0x4B6260 CEventAttractor::`scalar deleting destructor'  (catalogue name ~CEventAttractor)  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of 0x85B1B8. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Behaviour: call complete dtor 0x4AF400 (0x4B6264): SEH frame; `vtbl = 0x85B1B8`; if `m_entity` (+0x18) != 0
  `CEntity::CleanUpOldReference(m_entity, &m_entity)` 0x571A00 (0x4AF423-0x4AF436); `vtbl = 0x85AB38` (CEvent vtable; the
  CEventEditableResponse level is folded away, 0x4AF43F). m_entity is not nulled.
  Then if `flags & 1` (0x4B6269) the inlined event-pool delete on `ms_pEventPool` [0xB744AC], identical to CEvent::operator delete
  0x4B5630 (`idx = (this - objects) / 0x44` via magic 0x78787879 + `sar 5` + sign fix, `byteMap[idx] |= 0x80`,
  `if (idx < firstFree) firstFree = idx` signed, 0x4B6270-0x4B629E).
- gta-reversed: EventAttractor.cpp dtor `if (m_entity) m_entity->CleanUpOldReference(&m_entity)` + pool delete - matches.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4AF3E0 CEventAttractor::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AF3E0).
