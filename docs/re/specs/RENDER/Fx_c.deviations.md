# Fx_c (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Fx/Fx.cpp` (pin 20d8d2a).

## 0x49EAE0 Fx_c::Reset  (gr_deviations = 1)
Row Fx.cpp:155 (`InitEntitySystems(); // NOTSA`).
- Original bytes (thiscall, ECX = Fx_c*): `FxManager_c::DestroyAllFxSystems(&g_fxMan 0xA9AE80)` 0x4A98B0 (0x49EAE3-0x49EAE8), then
  `ecx = this; jmp 0x49E660` (Fx_c::InitStaticSystems, tail call). No call to InitEntitySystems.
- Verdict: CONFIRMED - gr re-creates the entity effect systems on Reset; the original does not. Class: keep-original.
