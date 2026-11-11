# CHandObject (WORLD) - RE-SPEC-FINAL group A

## 0x59EFE0 CHandObject::`scalar deleting destructor'  (catalogue: specified/REA, port level 16)
- `__thiscall CHandObject* (this, uint8 flags [esp+4])`, `ret 4`: `~CHandObject()` 0x59F000; if `flags & 1` CObject pool `operator delete(this)`
  0x5A1F20. Returns this. gta-reversed: compiler-generated. Open questions: none.

## 0x59F000 CHandObject::~CHandObject  (catalogue: unknown/REA, port level 15)
- Thunk: `jmp CObject::~CObject` 0x59F660 (CHandObject adds nothing to destroy; its own vtable is NOT re-stored). gta-reversed: `= default`. Open questions: none.
