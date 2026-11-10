# CEventAcquaintancePedHate (AI) - RE-SPEC-3 group I3

Vtable 0x858E68 (gr EventAcquaintancePedHate.h). Layout = CEventAcquaintancePed (size 0x18): CEventEditableResponse
(+0xC m_bAddToEventGroup u8 = 1, +0xE m_TaskId i16 = 200 (TASK_NONE), +0x10 m_FacialExpressionType i16 = -1, per ctor 0x4AF845-0x4AF84F), +0x14 m_AcquaintancePed CPed*
(registered reference). EVENT_ACQUAINTANCE_PED_HATE = 36 (0x24).

## 0x420F40 CEventAcquaintancePedHate::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x24; ret` -> 36 (EVENT_ACQUAINTANCE_PED_HATE). Slot 1 of 0x858E68. gr matches.

## 0x420F50 CEventAcquaintancePedHate::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall CEventEditableResponse* (const this)`, plain `ret`, SEH frame (0x420F50-0x420F5E). Slot 16 (called by
  CEventEditableResponse::Clone 0x420ED0).
- Behaviour: `p = CEvent::operator new(0x18)` 0x4B5620 (cdecl, size pushed at 0x420F68, `add esp,4`); if p == 0 return 0
  (0x420F7C-0x420F82, EAX = 0). Else `CEventAcquaintancePed::CEventAcquaintancePed(p, this->m_AcquaintancePed [+0x14])` 0x4AF820
  (thiscall, `ret 4`; sets time 0, valid 0, +0xC = 1, +0xE = 200, +0x10 = -1, vtbl 0x85B250, ped + RegisterReference when non-null),
  then overwrites `p->vtbl = 0x858E68` (0x420F8F), returns p.
- Only the ped is copied; m_nTimeActive / m_bValid / editable-response fields of the clone are the ctor defaults.
- gta-reversed: header inline `new CEventAcquaintancePedHate(m_AcquaintancePed)` - matches.

## 0x420FD0 CEventAcquaintancePedHate::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x1A; ret` -> 26. Slot 2. gr matches.
