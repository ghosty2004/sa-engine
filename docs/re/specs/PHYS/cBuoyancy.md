# cBuoyancy (PHYS), full specs

## 0x6c2740 cBuoyancy::cBuoyancy  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall cBuoyancy* (cBuoyancy* this = ECX)`, returns EAX = this, plain `ret`.
- Callers: 1 (`__init_mod_Buoyancy` 0x853880: `ecx = 0xc1c890` (global `mod_Buoyancy`), call, then
  `atexit(0x856a20)`). Callees: none. Globals: none directly.
- Fields: `this+0x4c` and `this+0x50` are the inlined default constructor of the embedded `CMatrix
  m_EntityMatrix` (+0xc): `m_pAttachMatrix` (+0x40 of the matrix) and `m_bOwnsAttachedMatrix` (+0x44, written as a
  full dword).
- Behaviour: `[this+0x4c] = 0`; `[this+0x50] = 0` (dword); return this. Nothing else is initialised (the
  global lives in .bss, so the rest is zero anyway).
- gta-reversed: `cBuoyancy() = default;` (Buoyancy.h:70) with `CMatrix`'s default constructor; same effect.
- Open questions: none.
