# CEventSoundQuiet (AI) - RE-SPEC-3 group I1

Vtable 0x86C2B0. Layout (gr EventSoundQuiet.h): CEventEditableResponse (0x14), +0x14 m_entity CEntity* (registered ref),
+0x18 m_fLocalSoundLevel f32, +0x1C m_startTimeInMs u32, +0x20 m_position CVector. EVENT_SOUND_QUIET = 62 (0x3E).

## 0x5DEA60 CEventSoundQuiet::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 0x3E (62).

## 0x5DEA80 CEventSoundQuiet::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `__thiscall CEntity* ()`: `return m_entity` (+0x14).

## 0x5DEA90 CEventSoundQuiet::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`: `fld dword [ecx+0x18]` -> returns m_fLocalSoundLevel in ST0.

## 0x5DEAB0 CEventSoundQuiet::`scalar deleting destructor'  (catalogue name ~CEventSoundQuiet)  (catalogue: specified/REA, port level 2)
- `__thiscall this (uint32 flags [esp+4])`, `ret 4`. Slot 0 of 0x86C2B0.
- Calls the complete dtor 0x5DEA00 (SEH frame; `vtbl = 0x86C2B0`; if `m_entity` != 0 `CEntity::CleanUpOldReference(m_entity,
  &m_entity)` 0x571A00; then `CEventEditableResponse::~CEventEditableResponse` 0x4AC480), then `if (flags & 1)
  CEvent::operator delete(this)` 0x4B5630 (0x5DEAB3-0x5DEAC5). Returns this.

## 0x5DEAD0 CEventSoundQuiet::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 8.
