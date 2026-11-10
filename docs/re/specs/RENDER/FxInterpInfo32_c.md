# FxInterpInfo32_c (RENDER) - full specs

Layout (gr `Fx/Blueprints/Interp/FxInterpInfo.h`, `FxInterpInfo32.h`): +0 vtable, +4 m_bLooped (bool), +5 m_nNumKeys (i8),
+6 m_nCount (i16), +8 m_pTimes (u16*), +0xC m_Keys (i16**). Vtable 0x85A980: slot 0 = scalar deleting destructor 0x4A8D70,
slot 1 = Load 0x5C1B10.

## 0x4A8990 FxInterpInfo32_c::FxInterpInfo32_c  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall` (ECX = this), no args, plain `ret`; returns `this` in EAX (0x4A8990).
- Callers: 12 FxInfo*_c constructors (FxInfoEmSize_c 0x4A4C20, EmSpeed 0x4A4D00, EmDir 0x4A4DC0, 0x4A5270, Force 0x4A5450,
  Friction 0x4A5550, AttractPt 0x4A5650, AttractLine 0x4A57C0, GroundCollide 0x4A5980, Wind 0x4A5A10, Jitter 0x4A5B20, Size 0x4A5FA0).
- Behaviour (store order): `byte [this+4] = 0` (0x4A8994, m_bLooped), `[this+8] = 0` (0x4A8997, m_pTimes),
  `[this+0] = 0x85A980` (0x4A899A, vtable), `[this+0xC] = 0` (0x4A89A0, m_Keys). m_nNumKeys (+5) and m_nCount (+6) are NOT written.
  The base FxInterpInfo_c ctor is inlined (no separate base-vtable store).
- gta-reversed: FxInterpInfo_c() sets m_bLooped/m_pTimes, derived sets m_Keys: same fields. Open questions: none.
