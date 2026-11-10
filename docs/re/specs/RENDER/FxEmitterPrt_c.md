# FxEmitterPrt_c (RENDER) - full specs

FxEmitterPrt_c is the 0x3C-byte particle element of `FxManager_c::m_FxEmitters` (1000 = 0x3E8 elements).

## 0x4A94F0 FxEmitterPrt_c::~FxEmitterPrt_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (ECX = FxEmitterPrt_c*), `jmp 0x4A8DC0` (0x4A94F0); 0x4A8DC0 is a lone `ret` (shared empty base/ListItem
  destructor). Net effect: nothing.
- Callers (as a function pointer, not called directly): FxManager_c::Init 0x4A98E0 at 0x4A9946 and FxManager_c::UnloadFxProject
  0x4A9AE0 at 0x4A9B4C both push `0x4A94F0, 0x4A94E0 (ctor), 0x3E8, 0x3C` to the eh-vector-constructor helper after
  `FxEmitterPrt_c::operator new[](0xEA64)` 0x4A18B0 (0xEA64 = 4 + 1000 * 0x3C: count cookie + elements).
- gta-reversed: no explicit dtor (implicit, empty). Same. Open questions: none.
