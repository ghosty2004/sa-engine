# CEntity (WORLD)

## 0x536c20 CEntity::vfunc_0 (scalar deleting destructor)  (catalogue: specified/REA, port level 13)
- Convention / signature: `__thiscall CEntity* (this = ECX, u32 flags [esp+4])`, returns this, `ret 4`.
  CEntity vtable slot 0; no direct callers.
- Callees: `CEntity::Destructor` 0x535e90, CRT `free` 0x8214bd (`__cdecl`, caller pops).
- Behaviour: `CEntity::Destructor(this)`; `if (flags & 1) free(this)` (CEntity has no pool operator
  delete: the global `operator delete` → `free`). Return this.
- Open questions: none.
