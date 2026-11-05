# CModelInfo (STREAM) - full specs, writer A (RE-SPEC-1)

## 0x4C5A60 CModelInfo::Get2dEffectStore  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl CStore<C2dEffect, N>* (void)`; returns EAX = 0xB4C2D8 (`CModelInfo::ms_2dFXInfoStore`;
  layout: +0x0 i32 count, +0x4 C2dEffect[N], 0x40 bytes each); plain `ret`.
- Callers: 3 (CBaseModelInfo::Get2dEffect 0x4C4C70, CBaseModelInfo::Add2dEffect 0x4C4D20,
  CFileLoader::Load2dEffect 0x5B7670).
- gta-reversed: ModelInfo.h:77 (wrapper only). Open questions: none.
