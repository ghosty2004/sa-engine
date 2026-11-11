# CEventOnFire (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x5FF770 CEventOnFire::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 58 (0x3A) (0x5FF770).

## 0x5FF780 CEventOnFire::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x5FF780).

## 0x5FF7E0 CEventOnFire::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: returns ST0 = 60.0f [0x858B34] (0x5FF7E0).

## 0x5FF7F0 CEventOnFire::~CEventOnFire  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5FF760 FUN_005ff760 (unnamed complete dtor: stores vtbl 0x86CCC0) (0x5FF7F3); if `flags & 1` (0x5FF7F8) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x5FF800); returns this (0x5FF808).

## 0x5FF890 CEventOnFire::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 66 (0x42) (0x5FF890).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5FF790 CEventOnFire::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CEvent* ()`, `ret`, SEH: `p = CEvent::operator new(0xC)` 0x4B5620; null -> 0; else `CEvent::CEvent(p)` 0x4ABFC0, `p->vtbl = 0x86CCC0` (0x5FF7C8); return p.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x5FF740 CEventOnFire::CEventOnFire  (catalogue: unknown/REA, port level 1)
- `__thiscall CEventOnFire* (this)`, plain `ret`. No callers (inlined at the use sites; out-of-line copy).
- Behaviour: `CEvent::CEvent(this)` 0x4ABFC0 (0x5FF743); `vtbl = 0x86CCC0` (0x5FF748); return this.

## 0x5FF760 CEventOnFire::~CEventOnFire  (catalogue: unknown/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86CCC0` (CEventOnFire) (0x5FF760); tail-jump to base dtor FUN_004ac010 0x4AC010 (0x5FF766). No other member cleanup.
