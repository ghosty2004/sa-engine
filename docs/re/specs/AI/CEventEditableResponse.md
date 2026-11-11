# CEventEditableResponse (AI) - RE-SPEC-3 group I3

## 0x4AC480 CEventEditableResponse::~CEventEditableResponse  (catalogue: specified/REA, port level 0)
- `__thiscall void (this)`, plain `ret`. Complete-object (non-deleting) dtor; 47 callers, mostly EH-unwind funclets of event ctors.
- Body is split by the no-CD build: 0x4AC480 `nop; jmp 0x402129`; 0x402129 `mov [ecx], 0x85AB38` (vtbl = CEvent vtable);
  `jmp 0x4AC486`; 0x4AC486 `ret`. Net effect: `this->vtbl = 0x85AB38`. Nothing else (no reference cleanup).
- gta-reversed: EventEditableResponse.h:13 `= default` (0x4AC480) - matches.

## 0x4B5670 CEventEditableResponse::~CEventEditableResponse  (catalogue: specified/REA, port level 1)
- Scalar deleting destructor (slot 0). `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls the non-deleting dtor 0x4AC480
  (ECX is not saved: the code relies on it still being `this` after the call); if `flags & 1`: return the object to CEventPool `P = *(0xB744AC)` (ms_pEventPool): `idx = (this - P->+0) / 0x44` (signed division via the magic multiply 0x78787879, `sar 5`, rounding toward zero); `P->+4[idx] |= 0x80` (free flag); if `idx < P->+0xC` `P->+0xC = idx` (first-free hint). Returns this in EAX.
