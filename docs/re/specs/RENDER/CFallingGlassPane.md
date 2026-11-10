# CFallingGlassPane (RENDER) - full specs

CFallingGlassPane (gr `FallingGlassPane.h`): +0x00 CMatrix m_Matrix (0x48; +0x40 m_pAttachMatrix, +0x44 m_bOwnsAttachedMatrix),
+0x48 m_Velocity, ... Array `CGlass::aGlassPanes` is built by `__init_CGlass__aGlassPanes` 0x854EE0 (see `_static-init-R1.md`).

## 0x71A8B0 CFallingGlassPane::CFallingGlassPane  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall` (ECX = CFallingGlassPane*), no args, plain `ret`, returns `this` in EAX (0x71A8B0).
- Callers: `__init_CGlass__aGlassPanes` 0x854EE0 (element ctor). Callees: none.
- Behaviour: inlined `CMatrix()` of m_Matrix only: `[this+0x40] = 0` (0x71A8B4), `[this+0x44] = 0` (0x71A8B7, dword). Byte-identical
  to C3dMarker::C3dMarker 0x720F60.
- gta-reversed: `= default`; the gr class has no other default member initialisers relevant here. Open questions: none.

## 0x71A8C0 CFallingGlassPane::~CFallingGlassPane  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall`, tail jump `jmp 0x59ACD0` (0x71A8C0) = CMatrix::~CMatrix on m_Matrix (offset 0).
- Callers: 0x854EE0 (dtor argument), `__dtor_CGlass__aGlassPanes` 0x856BA0.
- Behaviour: if `[this+0x44] != 0` and `[this+0x40] != 0`: `RwMatrixDestroy([this+0x40])` 0x7F2A20 (0x59ACD0-0x59ACE5).
- gta-reversed: `= default` (same). Open questions: none.
