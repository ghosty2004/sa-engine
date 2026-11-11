# CTaskSimpleWaitUntilLeaderAreaCodesMatch (AI) - RE-SPEC-FINAL (helper B)

## 0x5F67E0 CTaskSimpleWaitUntilLeaderAreaCodesMatch::~CTaskSimpleWaitUntilLeaderAreaCodesMatch  (catalogue: unknown/REA, port level 3)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86C7E8` (CTaskSimpleWaitUntilLeaderAreaCodesMatch) (0x5F67E0); tail-jump to base dtor CTaskSimpleWaitUntilAreaCodesMatch::Destructor 0x6355E0 (0x5F67E6). No other member cleanup.

## 0x5F68A0 CTaskSimpleWaitUntilLeaderAreaCodesMatch::sub_5F68A0 (proposed: CPedList::CPedList)  (catalogue: unknown/REA, port level 1)
- Name proposal: CPedList default ctor (out-of-line). Evidence: only calls `CPedList::Empty(this)` 0x699DB0 on ECX and returns this; nothing task-specific. No callers.
- `__thiscall CPedList* (this)`, plain `ret`: `CPedList::Empty(this)` (0x5F68A3); return this (0x5F68A8).
