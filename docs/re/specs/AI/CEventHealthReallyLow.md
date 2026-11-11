# CEventHealthReallyLow (AI) - RE-SPEC-FINAL (helper B)

## 0x5FF500 CEventHealthReallyLow::CEventHealthReallyLow  (catalogue: unknown/REA, port level 1)
- `__thiscall CEventHealthReallyLow* (this)`, plain `ret`. No callers (inlined at the use sites; out-of-line copy).
- Behaviour: `CEventEditableResponse::CEventEditableResponse(this)` 0x4AC450 (0x5FF503); `vtbl = 0x86CBE8` (0x5FF508); return this.

## 0x5FF520 CEventHealthReallyLow::~CEventHealthReallyLow  (catalogue: unknown/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86CBE8` (0x5FF520); tail-jump to base dtor CEventEditableResponse::~CEventEditableResponse 0x4AC480 (0x5FF526). No other member cleanup.
