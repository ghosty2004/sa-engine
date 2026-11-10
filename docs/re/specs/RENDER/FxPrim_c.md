# FxPrim_c (RENDER) - full specs

## 0x4A9F50 FxPrim_c::Enable  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (FxPrim_c* this = ECX, bool enable [esp+4])`, `ret 4`.
- Callers: FxSystem_c::EnablePrim 0x4AA610. Callees: none. Globals: none.
- Behaviour: `this->+0x0C (u8, gr FxPrim_c::m_bEnabled) = enable` (0x4A9F50-0x4A9F54; the low byte of the stack slot is stored
  as is, no normalisation to 0/1).
- gta-reversed: not hooked as a separate function. Open questions: none.
