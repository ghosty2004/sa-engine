# FxInterpInfo255_c (RENDER) - full specs

Layout as FxInterpInfo32_c (gr `FxInterpInfo255.h`): +0 vtable, +4 m_bLooped, +5 m_nNumKeys, +6 m_nCount, +8 m_pTimes, +0xC m_Keys.
Vtable 0x85A988: slot 0 = scalar deleting destructor 0x4A8D90, slot 1 = Load 0x5C1D30.

## 0x4A8B50 FxInterpInfo255_c::FxInterpInfo255_c  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall` (ECX = this), no args, plain `ret`; returns `this` in EAX (0x4A8B50).
- Callers: FxInfoSpriteRect_c::FxInfoSpriteRect_c 0x4A60B0, FxInfoHeatHaze_c::FxInfoHeatHaze_c 0x4A61A0.
- Behaviour: `byte [this+4] = 0` (0x4A8B54), `[this+8] = 0` (0x4A8B57), `[this+0] = 0x85A988` (0x4A8B5A, vtable),
  `[this+0xC] = 0` (0x4A8B60). +5 / +6 not written. Identical to 0x4A8990 except the vtable.
- gta-reversed: same fields. Open questions: none.
