# CTaskSimpleWaitForPizza (AI) - RE-SPEC-FINAL (helper B)

## 0x632B30 CTaskSimpleWaitForPizza::CTaskSimpleWaitForPizza  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this)`, plain `ret`. No callers.
- Behaviour: CTaskSimple ctor 0x61A390; `vtbl = 0x86E1DC` (TASK_SIMPLE_WAIT_FOR_PIZZA); timer `+8 = 0; +0xC = 0; +0x10 = 0 (u8 started); +0x11 = 0 (u8 reset)`; return this.

## 0x632B70 CTaskSimpleWaitForPizza::~CTaskSimpleWaitForPizza  (catalogue: unknown/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86E1DC` (0x632B70); tail-jump to base dtor CTaskSimple::~CTaskSimple 0x61A3A0 (0x632B76). No other member cleanup.
