# CEventLowAngerAtPlayer (AI) - RE-SPEC-FINAL (helper B)

## 0x5FF5C0 CEventLowAngerAtPlayer::CEventLowAngerAtPlayer  (catalogue: unknown/REA, port level 1)
- `__thiscall CEventLowAngerAtPlayer* (this)`, plain `ret`. No callers (inlined at the use sites; out-of-line copy).
- Behaviour: `CEventEditableResponse::CEventEditableResponse(this)` 0x4AC450 (0x5FF5C3); `vtbl = 0x86CC30` (0x5FF5C8); return this.

## 0x5FF5E0 CEventLowAngerAtPlayer::~CEventLowAngerAtPlayer  (catalogue: unknown/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86CC30` (0x5FF5E0); tail-jump to base dtor CEventEditableResponse::~CEventEditableResponse 0x4AC480 (0x5FF5E6). No other member cleanup.
