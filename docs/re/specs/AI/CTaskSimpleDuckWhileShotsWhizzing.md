# CTaskSimpleDuckWhileShotsWhizzing (AI) - RE-SPEC-FINAL (helper B)

## 0x692680 sub_692680 (proposed: CTaskSimpleDuckWhileShotsWhizzing::CTaskSimpleDuckWhileShotsWhizzing)  (catalogue: unknown/REA, port level 1)
- Evidence: CTaskSimple ctor, then the CTaskSimpleDuck vtable 0x870BC4 (inline CTaskSimpleDuck ctor), then vtable 0x870BEC (TASK_SIMPLE_DUCK_WHILE_SHOTS_WHIZZING); caller vt_870bec::Clone 0x692D60.
- `__thiscall T* (this, uint16 lengthOfDuck [esp+4])`, `ret 4`.
- Behaviour: CTaskSimple ctor 0x61A390; `vtbl = 0x870BC4`; `+0x18 = 0; +0x19 = 0; +0x1A = 1; +0x1B = 1; +0x1C = 0; +0x20 = 0; +0x24 = 0; +0x25 = 0xFF` (u8s/dwords as written, 0x692695-0x6926AC); `+0x10 = 0; +0x14 = 0; +8 = now ([0xB7CB84], start time); +0xC = (u16)arg; +0xE = 0xFFFF (u16)`; `vtbl = 0x870BEC`; return this.
- gta-reversed: CTaskSimpleDuck(eDuckControlType, uint16 lengthOfDuck, int16 shotWhizzingCounter = -1) - the inline base init matches DuckControlType = 0 (+0x18) per these stores.

## 0x6926F0 CTaskSimpleDuckWhileShotsWhizzing::~CTaskSimpleDuckWhileShotsWhizzing  (catalogue: unknown/REA, port level 2)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x870BEC` (0x6926F0); tail-jump to base dtor CTaskSimpleDuck::Destructor 0x692030 (0x6926F6). No other member cleanup.
