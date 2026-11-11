# CEventHighAngerAtPlayer (AI) - RE-SPEC-FINAL (helper B)

## 0x5FF680 CEventHighAngerAtPlayer::CEventHighAngerAtPlayer  (catalogue: unknown/REA, port level 1)
- `__thiscall CEventHighAngerAtPlayer* (this)`, plain `ret`. No callers (inlined at the use sites; out-of-line copy).
- Behaviour: `CEventEditableResponse::CEventEditableResponse(this)` 0x4AC450 (0x5FF683); `vtbl = 0x86CC78` (0x5FF688); return this.

## 0x5FF6A0 CEventHighAngerAtPlayer::~CEventHighAngerAtPlayer  (catalogue: unknown/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86CC78` (0x5FF6A0); tail-jump to base dtor CEventEditableResponse::~CEventEditableResponse 0x4AC480 (0x5FF6A6). No other member cleanup.
