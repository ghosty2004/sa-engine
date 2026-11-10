# FxInfoEmRate_c (RENDER) - full specs

Layout (gr `Fx/Blueprints/FxInfo.h`, `FxInfoEmRate.h`): +0 vtable, +4 m_nType (u16 eFxInfoType), +6 m_bTimeModeParticle (bool),
+8 m_InterpInfo (FxInterpInfoFloat_c, 0x10 bytes: +8 vtable, +0xC m_bLooped, +0xD m_nNumKeys, +0xE m_nCount (byte in the original),
+0x10 m_pTimes, +0x14 m_Keys). Object size 0x18, allocated by the caller from the Fx pool (FxMemoryPool_c::GetMem(0x18, 4),
memory not cleared). Vtable 0x85A95C.

## 0x4A6A00 FxInfoEmRate_c::FxInfoEmRate_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (ECX = this), no args, plain `ret` (0x4A6A64); returns `this` in EAX (0x4A6A57). SEH frame
  (handler 0x8392A3, state 0 stored at 0x4A6A20, state 1 after the member ctor at 0x4A6A3E; unwind funclets call FxInfo_c::~FxInfo_c 0x4A4940, FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4A8460).
- Callers: FxInfoManager_c::AddFxInfo 0x4A7B00 (only caller; hits=2). Callees: FxInterpInfoFloat_c ctor 0x4A8440 (thiscall on this+8), FxMemoryPool_c::GetMem 0x4A9CA0 (thiscall on g_fxMan.m_Pool 0xA9AF2C = g_fxMan 0xA9AE80 + 0xAC, args (size, align), `ret 8`).
- Behaviour (in order):
  1. `[this+0] = 0x85A95C` (0x4A6A28; the FxInfo_c base ctor is inlined and only the final vtable is stored).
  2. FxInterpInfoFloat_c ctor on `this+8` (0x4A6A2E): stores the interp vtable at +8 and zeroes byte +0xC (m_bLooped), dword +0x10 (m_pTimes)
     and dword +0x14 (m_Keys); +0xD (m_nNumKeys) is not touched.
  3. `al = 1` (0x4A6A33); `word [this+4] = 0x1001` (0x4A6A42); `byte [this+0xE] = al = 1` (0x4A6A48, m_InterpInfo.m_nCount, low byte only).
  4. `[this+0x14] = GetMem(0x4, 4)` (call 0x4A6A4B, store 0x4A6A54): 1 key-array pointer(s), m_InterpInfo.m_Keys. Memory not cleared.
     +6 (m_bTimeModeParticle) is not written.
- gta-reversed: `m_nType = 0x1001; m_InterpInfo.Allocate(1)` - Allocate writes `m_nCount = 1` as int16 (two bytes, also clears +0xF)
  and `m_Keys = g_fxMan.Allocate<float*>(1)` = GetMem(4, 4): same except the byte vs. word store (harmless: +0xF is not read
  by the original readers, which use `byte [+0xE]`). Open questions: none.
