# FxInterpInfoU255_c (RENDER) - full specs

Layout (gr `Fx/Blueprints/Interp/FxInterpInfo.h`, `FxInterpInfoU255.h`): +0 vtable, +4 m_bLooped (bool), +5 m_nNumKeys (i8),
+6 m_nCount (accessed as a byte by the original readers, see FxInterpInfoFloat_c.md; gr `int16`), +8 m_pTimes (u16*),
+0xC m_Keys (u16**). Vtable 0x85A978.

## 0x4A87D0 FxInterpInfoU255_c::FxInterpInfoU255_c  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall` (ECX = this), no args, plain `ret` (0x4A87E3); returns `this` in EAX (0x4A87D0).
- Callers (8): FxInfoEmLife_c 0x4A4F50, FxInfoUnderwater_c 0x4A5E20, FxInfoColour_c 0x4A5EA0, FxInfoTrail_c 0x4A6220,
  FxInfoColourRange_c 0x4A6630, FxInfoSelfLit_c 0x4A6760, FxInfoColourBright_c 0x4A67D0, FxInfoSmoke_c 0x4A68E0 ctors.
- Behaviour (store order): `byte [this+4] = 0` (0x4A87D4), `[this+8] = 0` (0x4A87D7), `[this+0] = 0x85A978` (0x4A87DA),
  `[this+0xC] = 0` (0x4A87E0). +5 and +6 untouched.
- gta-reversed: same fields and values. Open questions: none.
