# CEventSeenCop (AI) - RE-SPEC-3 group I3

Vtable 0x86CB58. Layout = CEventAcquaintancePed (size 0x18; +0x14 m_AcquaintancePed CPed*). EVENT_SEEN_COP = 72 (0x48).

## 0x4AFB40 CEventSeenCop::AffectsPed  (catalogue: specified/REA, port level 3)
- `__thiscall bool (this, CPed* ped [esp+4])`: single `jmp 0x4AFA30` (CEventAcquaintancePed::AffectsPed; the callee's `ret 4`
  returns). Slot 5 of 0x86CB58. Port: call CEventAcquaintancePed::AffectsPed with the same args.
- gta-reversed: header inline `return CEventAcquaintancePed::AffectsPed(ped)` - matches.

## 0x5FF3B0 CEventSeenCop::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x48; ret` -> 72. Slot 1. gr matches.

## 0x5FF3C0 CEventSeenCop::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall CEventEditableResponse* (const this)`, plain `ret`, SEH. Slot 16.
- Behaviour: `p = CEvent::operator new(0x18)` 0x4B5620 (0x5FF3DC); if p == 0 return 0 (0x5FF3EC-0x5FF3F2). Else
  `CEventAcquaintancePed::CEventAcquaintancePed(p, m_AcquaintancePed [+0x14])` 0x4AF820 (0x5FF3F4-0x5FF3FA), then
  `p->vtbl = 0x86CB58` (0x5FF3FF), return p. Same shape as CEventAcquaintancePedHate::CloneEditable 0x420F50.
- gta-reversed: header inline `new CEventSeenCop(m_AcquaintancePed)` - matches.

## 0x5FF880 CEventSeenCop::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x15; ret` -> 21. Slot 2. gr matches.
