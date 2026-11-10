# FxInfoAttractPt_c (RENDER) - full specs

Layout (gr `Fx/Blueprints/FxInfo.h`, `FxInfoAttractPt.h`): +0 vtable, +4 m_nType (u16 eFxInfoType), +6 m_bTimeModeParticle
(bool), +8 m_InterpInfo (FxInterpInfo32_c, 0x10 bytes: +8 vtable, +0xC m_bLooped, +0xD m_nNumKeys, +0xE m_nCount (byte),
+0x10 m_pTimes, +0x14 m_Keys). Vtable 0x85A86C.

## 0x4A5650 FxInfoAttractPt_c::FxInfoAttractPt_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (ECX = this), no args, plain `ret` (0x4A56B4); returns `this` in EAX (0x4A56A7). SEH frame
  (handler 0x839023, state 0 at 0x4A5670, state 1 after the member ctor at 0x4A568C; unwind calls FxInterpInfo32_c dtor
  0x4A89B0 and FxInfo_c dtor 0x4A4940).
- Callers: FxInfoManager_c::AddFxInfo 0x4A7B00. Callees: FxInterpInfo32_c ctor 0x4A8990 (on this+8),
  FxMemoryPool_c::GetMem 0x4A9CA0 (thiscall on g_fxMan.m_Pool 0xA9AF2C = g_fxMan 0xA9AE80 + 0xAC, args (size, align), ret 8).
- Behaviour (in order):
  1. `[this+0] = 0x85A86C` (0x4A5678; the FxInfo_c base ctor is inlined and only the final vtable is stored).
  2. FxInterpInfo32_c ctor on `this+8` (0x4A567E).
  3. `word [this+4] = 0x2008` (0x4A5691, FX_INFO_ATTRACTPT_DATA); `byte [this+0xE] = 4` (0x4A5697, m_InterpInfo.m_nCount, only
     the low byte is written; +0xF stays as allocated).
  4. `[this+0x14] = GetMem(0x10, 4)` (0x4A5683-0x4A56A4): 4 key-array pointers, m_InterpInfo.m_Keys. Memory content is not
     cleared. +6 (m_bTimeModeParticle) is not written here.
- gta-reversed: `m_nType = FX_INFO_ATTRACTPT_DATA; m_InterpInfo.Allocate(4)` - Allocate writes `m_nCount = 4` as int16
  (two bytes) and `m_Keys = g_fxMan.Allocate<int16*>(4)` = GetMem(16, 4): same except the byte vs. word store (harmless if +0xF
  is never read; it is not in the original readers). Open questions: none.
