# FxInterpInfoFloat_c (RENDER) - full specs

Layout (gr `Fx/Blueprints/Interp/FxInterpInfo.h`, `FxInterpInfoFloat.h`): +0 vtable, +4 m_bLooped (bool), +5 m_nNumKeys (i8),
+6 m_nCount, +8 m_pTimes (u16*), +0xC m_Keys (float**). The original only ever accesses +6 as a byte (e.g. `mov cl, byte ptr
[esi+6]` 0x4A847D / `movsx eax, byte ptr [esi+6]` 0x5C18D6 in the Load/GetVal readers), so m_nCount is effectively an i8 and +7
is padding; gr declares it `int16` (harmless as long as +7 stays unused). Vtable 0x85A970: slot 0 = 0x4A8D30 (scalar deleting
destructor), slot 1 = Load 0x5C16F0 (`dis.py d0x85A970:3`).

## 0x4A8440 FxInterpInfoFloat_c::FxInterpInfoFloat_c  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall` (ECX = this), no args, plain `ret` (0x4A8453); returns `this` in EAX (`mov eax, ecx` 0x4A8440).
  The body is split by a .text detour: `nop; jmp 0x406A71` (0x4A844A/0x4A844B) -> `mov [eax], 0x85A970; jmp 0x4A8450`
  (0x406A71/0x406A77). Same behaviour as a straight-line body.
- Callers (9): FxInfoEmAngle_c 0x4A4E90, FxInfoEmPos_c 0x4A5010, FxInfoEmWeather_c 0x4A50E0, FxInfoEmRotation_c 0x4A51B0,
  FxInfoRotSpeed_c 0x4A5CB0, FxInfoFloat_c 0x4A5DA0, FxInfoFlat_c 0x4A6320, FxInfoDir_c 0x4A6440, FxInfoEmRate_c 0x4A6A00 ctors.
  Callees: none.
- Behaviour (store order): `byte [this+4] = 0` (0x4A8444, m_bLooped), `[this+8] = 0` (0x4A8447, m_pTimes),
  `[this+0] = 0x85A970` (0x406A71, vtable), `[this+0xC] = 0` (0x4A8450, m_Keys). m_nNumKeys (+5) and m_nCount (+6) are not
  written (the owning FxInfo*_c ctor writes the +6 byte afterwards). The base FxInterpInfo_c ctor is inlined.
- gta-reversed: FxInterpInfo_c() sets m_bLooped/m_pTimes, derived sets m_Keys: same fields, same values. Open questions: none.
