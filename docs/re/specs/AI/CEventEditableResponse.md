# CEventEditableResponse (AI) - RE-SPEC-3 group I3

## 0x4AC480 CEventEditableResponse::~CEventEditableResponse  (catalogue: specified/REA, port level 0)
- `__thiscall void (this)`, plain `ret`. Complete-object (non-deleting) dtor; 47 callers, mostly EH-unwind funclets of event ctors.
- Body is split by the no-CD build: 0x4AC480 `nop; jmp 0x402129`; 0x402129 `mov [ecx], 0x85AB38` (vtbl = CEvent vtable);
  `jmp 0x4AC486`; 0x4AC486 `ret`. Net effect: `this->vtbl = 0x85AB38`. Nothing else (no reference cleanup).
- gta-reversed: EventEditableResponse.h:13 `= default` (0x4AC480) - matches.
