# FxMemoryPool_c (RENDER) - full specs

FxMemoryPool_c (gr `Fx/FxMemoryPool.h`): +0 m_pData, +4 m_nSize, +8 m_nPosition.

## 0x4A9C10 FxMemoryPool_c::FxMemoryPool_c  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall` (ECX = FxMemoryPool_c*), no args, `mov eax, ecx; ret` (0x4A9C10): returns `this`, initialises nothing.
- Caller: FxManager_c::Constructor 0x4A9470. gta-reversed: `= default` (same; no member initialisers). Open questions: none.

## 0x4A9C20 FxMemoryPool_c::~FxMemoryPool_c  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall`, body `ret` (0x4A9C20); frees nothing (Exit() does that elsewhere). Caller: FxManager_c::Destructor 0x4A90A0.
- gta-reversed: `= default` (same). Open questions: none.
