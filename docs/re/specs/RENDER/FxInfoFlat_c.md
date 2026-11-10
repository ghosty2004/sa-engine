# FxInfoFlat_c (RENDER) - full specs

Layout (gr `Fx/Blueprints/FxInfo.h`, `FxInfoFlat.h`): +0 vtable, +4 m_nType (u16 eFxInfoType), +6 m_bTimeModeParticle (bool),
+8 m_InterpInfo (FxInterpInfoFloat_c, 0x10 bytes: +8 vtable, +0xC m_bLooped, +0xD m_nNumKeys, +0xE m_nCount (byte in the original),
+0x10 m_pTimes, +0x14 m_Keys). Object size 0x18, allocated by the caller from the Fx pool (FxMemoryPool_c::GetMem(0x18, 4),
memory not cleared). Vtable 0x85A908.

## 0x4A6320 FxInfoFlat_c::FxInfoFlat_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (ECX = this), no args, plain `ret` (0x4A6384); returns `this` in EAX (0x4A6377). SEH frame
  (handler 0x8391C3, state 0 stored at 0x4A6340, state 1 after the member ctor at 0x4A635C; unwind funclets call FxInfo_c::~FxInfo_c 0x4A4940, FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4A8460).
- Callers: FxInfoManager_c::AddFxInfo 0x4A7B00 (only caller; hits=2). Callees: FxInterpInfoFloat_c ctor 0x4A8440 (thiscall on this+8), FxMemoryPool_c::GetMem 0x4A9CA0 (thiscall on g_fxMan.m_Pool 0xA9AF2C = g_fxMan 0xA9AE80 + 0xAC, args (size, align), `ret 8`).
- Behaviour (in order):
  1. `[this+0] = 0x85A908` (0x4A6348; the FxInfo_c base ctor is inlined and only the final vtable is stored).
  2. FxInterpInfoFloat_c ctor on `this+8` (0x4A634E): stores the interp vtable at +8 and zeroes byte +0xC (m_bLooped), dword +0x10 (m_pTimes)
     and dword +0x14 (m_Keys); +0xD (m_nNumKeys) is not touched.
  3. `word [this+4] = 0x4020` (0x4A6361); `byte [this+0xE] = 9` (0x4A6367, m_InterpInfo.m_nCount, only the low byte; +0xF stays as allocated).
  4. `[this+0x14] = GetMem(0x24, 4)` (call 0x4A636B, store 0x4A6374): 9 key-array pointer(s), m_InterpInfo.m_Keys. Memory not cleared.
     +6 (m_bTimeModeParticle) is not written.
- gta-reversed: `m_nType = 0x4020; m_InterpInfo.Allocate(9)` - Allocate writes `m_nCount = 9` as int16 (two bytes, also clears +0xF)
  and `m_Keys = g_fxMan.Allocate<float*>(9)` = GetMem(36, 4): same except the byte vs. word store (harmless: +0xF is not read
  by the original readers, which use `byte [+0xE]`). Open questions: none.
