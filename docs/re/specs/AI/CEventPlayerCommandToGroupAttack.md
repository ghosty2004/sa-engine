# CEventPlayerCommandToGroupAttack (AI) - RE-SPEC-3 group I2

## 0x5F6360 CEventPlayerCommandToGroupAttack::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 0x47 (71 EVENT_PLAYER_COMMAND_TO_GROUP). Slot 1 of vtable 0x86C710 (the attack command shares the
  base type value).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5F63D0 CEventPlayerCommandToGroupAttack::~CEventPlayerCommandToGroupAttack  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5F63F0 thunk_FUN_0040a472 (0x5F63D3); if `flags & 1` (0x5F63D8) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x5F63E0); returns this (0x5F63E8). Proposed name: `thunk_FUN_0040a472::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_0040a472).

## 0x5F6400 CEventPlayerCommandToGroupAttack::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 44 (0x2C) (0x5F6400).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5F6370 CEventPlayerCommandToGroupAttack::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall`, `ret`, SEH: `p = CEvent::operator new(0x1C)` 0x4B5620; null -> 0; else `CEventPlayerCommandToGroup::Constructor(p, 0 /*command: EAX is 0 here,
  0x5F639A*/, this+0x18 target)` 0x4B23D0 (0x5F63A4-0x5F63AB), `p->vtbl = 0x86C710` (0x5F63B0); return p. Note: the command argument is the zeroed EAX (0), not
  this->m_command.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x5F63F0 CEventPlayerCommandToGroupAttack::~CEventPlayerCommandToGroupAttack  (catalogue: unknown/REA, port level 2)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86C710` (done in the .HOODLUM detour 0x40A472, entry `nop; jmp 0x40A472`), then tail-jump to the base dtor 0x4B2470 (CEventPlayerCommandToGroup complete dtor) (0x5F63F6). Caller: scalar deleting dtor 0x5F63D0.
