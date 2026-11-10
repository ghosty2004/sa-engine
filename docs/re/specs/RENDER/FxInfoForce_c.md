# FxInfoForce_c (RENDER) - full specs

Layout (gr `Fx/Blueprints/FxInfo.h`, `FxInfoForce.h`): +0 vtable, +4 m_nType (u16 eFxInfoType), +6 m_bTimeModeParticle (bool),
+8 m_InterpInfo (FxInterpInfo32_c, 0x10 bytes: +8 vtable, +0xC m_bLooped, +0xD m_nNumKeys, +0xE m_nCount (byte in the original),
+0x10 m_pTimes, +0x14 m_Keys). Object size 0x18, allocated by the caller from the Fx pool (FxMemoryPool_c::GetMem(0x18, 4),
memory not cleared). Vtable 0x85A854.

## 0x4A5450 FxInfoForce_c::FxInfoForce_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (ECX = this), no args, plain `ret` (0x4A54B4); returns `this` in EAX (0x4A54A7). SEH frame
  (handler 0x838FE3, state 0 stored at 0x4A5470, state 1 after the member ctor at 0x4A548C; unwind funclets call FxInfo_c::~FxInfo_c 0x4A4940, FxInterpInfo32_c::~FxInterpInfo32_c 0x4A89B0).
- Callers: FxInfoManager_c::AddFxInfo 0x4A7B00 (only caller; hits=2). Callees: FxInterpInfo32_c ctor 0x4A8990 (thiscall on this+8), FxMemoryPool_c::GetMem 0x4A9CA0 (thiscall on g_fxMan.m_Pool 0xA9AF2C = g_fxMan 0xA9AE80 + 0xAC, args (size, align), `ret 8`).
- Behaviour (in order):
  1. `[this+0] = 0x85A854` (0x4A5478; the FxInfo_c base ctor is inlined and only the final vtable is stored).
  2. FxInterpInfo32_c ctor on `this+8` (0x4A547E): stores the interp vtable at +8 and zeroes byte +0xC (m_bLooped), dword +0x10 (m_pTimes)
     and dword +0x14 (m_Keys); +0xD (m_nNumKeys) is not touched.
  3. `word [this+4] = 0x2002` (0x4A5491); `byte [this+0xE] = 3` (0x4A5497, m_InterpInfo.m_nCount, only the low byte; +0xF stays as allocated).
  4. `[this+0x14] = GetMem(0xC, 4)` (call 0x4A549B, store 0x4A54A4): 3 key-array pointer(s), m_InterpInfo.m_Keys. Memory not cleared.
     +6 (m_bTimeModeParticle) is not written.
- gta-reversed: `m_nType = 0x2002; m_InterpInfo.Allocate(3)` - Allocate writes `m_nCount = 3` as int16 (two bytes, also clears +0xF)
  and `m_Keys = g_fxMan.Allocate<int16*>(3)` = GetMem(12, 4): same except the byte vs. word store (harmless: +0xF is not read
  by the original readers, which use `byte [+0xE]`). Open questions: none.
