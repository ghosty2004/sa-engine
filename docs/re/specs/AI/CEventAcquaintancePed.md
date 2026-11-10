# CEventAcquaintancePed family (AI) - RE-SPEC-3 group I1

Layout (gr EventAcquaintancePed.h): CEventEditableResponse (0x14 bytes: CEvent 0xC + +0xC m_TaskId i16, +0xE m_DirectionalTaskId?
etc.) then +0x14 m_AcquaintancePed (CPed*, registered reference). Vtables: CEventAcquaintancePed 0x85B250,
CEventAcquaintancePedHate 0x858E68, CEventSeenCop 0x86CB58 (all share slot 10 GetSourceEntity 0x420F10).
Helper (not in this group, needed by the dtors below): 0x4AF890 (proposed name: CEventAcquaintancePed::~CEventAcquaintancePed,
complete-object dtor, catalogue unknown/FUN_004af890): SEH frame; `this->vtbl = 0x85B250`; if `[this+0x14] != 0`
`CEntity::CleanUpOldReference([this+0x14], &this->+0x14)` 0x571A00 (thiscall on the ped, arg = slot address); `this->vtbl =
0x85AB38` (CEvent; the trivial CEventEditableResponse dtor is folded away) (0x4AF8AD-0x4AF8CF).

## 0x420F10 CEventAcquaintancePed::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `__thiscall CEntity* (this)`: `return *(CEntity**)(this+0x14)` (m_AcquaintancePed). Slot 10 of 7 vtables (AcquaintancePed,
  ...Hate, ...HateBadlyLit, SeenCop, 0x86CA38/0x86CA80/0x86CAC8 = the other acquaintance subclasses). gr matches.

## 0x420FB0 CEventAcquaintancePedHate::`scalar deleting destructor'  (catalogue name: CEventAcquaintancePed::~CEventAcquaintancePed; NAME CORRECTION)  (catalogue: specified/REA, port level 3)
- Evidence: the only reference is slot 0 of vtable 0x858E68 (CEventAcquaintancePedHate).
- `__thiscall CEvent* (this, uint32 flags [esp+4])`, `ret 4`, returns this in EAX.
- Behaviour: call 0x420F30 = CEventAcquaintancePedHate::~CEventAcquaintancePedHate (`vtbl = 0x858E68` then tail-jump to
  0x4AF890 above); `if (flags & 1) CEvent::operator delete(this)` 0x4B5630 (cdecl, `add esp,4`) (0x420FB3-0x420FC5).
- gta-reversed: compiler-generated in gr; behaviour matches.

## 0x5FF420 CEventSeenCop::`scalar deleting destructor'  (catalogue name: CEventAcquaintancePed::~CEventAcquaintancePed; NAME CORRECTION)  (catalogue: specified/REA, port level 3)
- Evidence: only referenced from slot 0 of vtable 0x86CB58 (CEventSeenCop).
- Same shape as 0x420FB0: call 0x5FF3A0 (`vtbl = 0x86CB58`, jmp 0x4AF890), then `if (flags & 1) CEvent::operator delete(this)`
  0x4B5630; return this; `ret 4` (0x5FF423-0x5FF43B).
