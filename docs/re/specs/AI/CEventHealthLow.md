# CEventHealthLow (AI) - RE-SPEC-FINAL (helper B)

## 0x5FF440 CEventHealthLow::CEventHealthLow  (catalogue: unknown/REA, port level 1)
- `__thiscall CEventHealthLow* (this)`, plain `ret`. No callers (inlined at the use sites; out-of-line copy).
- Behaviour: `CEventEditableResponse::CEventEditableResponse(this)` 0x4AC450 (0x5FF443); `vtbl = 0x86CBA0` (0x5FF448); return this.

## 0x5FF460 CEventHealthLow::~CEventHealthLow  (catalogue: unknown/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`: `vtbl = 0x86CBA0` (in the .HOODLUM detour 0x401A42; entry `nop; jmp 0x401A42`), then tail-jump to CEventEditableResponse::~CEventEditableResponse 0x4AC480 (0x5FF466). Caller: scalar deleting dtor 0x5FF4E0.
