# CDummyPed (WORLD) - RE-SPEC-FINAL group A

## 0x5DE2E0 CDummyPed::CDummyPed  (catalogue: unknown/REA, port level 3)
- `__thiscall CDummyPed* (this)`, plain `ret`: `CDummy::CDummy(this)` 0x532540; `vtable = 0x86C198`; return this. gta-reversed: matches. Open questions: none.

## 0x5DE300 CDummyPed::~CDummyPed  (catalogue: unknown/REA, port level 14)
- `__thiscall void (this)`: `vtable = 0x86C198`; `jmp 0x532560` (CDummy::~CDummy). gta-reversed: `= default`. Open questions: none.

## 0x5DE310 CDummyPed::`scalar deleting destructor'  (catalogue: unknown/REA, port level 15)
- `__thiscall CDummyPed* (this, uint8 flags [esp+4])`, `ret 4`: dtor 0x5DE300; if `flags & 1` `CDummy::operator delete(this)` 0x5326F0. Returns this.
  Open questions: none.
