# FxInfoAnimTexture_c (RENDER) - full specs

Layout as FxInfoAttractPt_c (see FxInfoAttractPt_c.md): +4 m_nType (u16), +6 m_bTimeModeParticle, +8 m_InterpInfo
(FxInterpInfo32_c; +0xE m_nCount byte, +0x14 m_Keys). Vtable 0x85A920.

## 0x4A6550 FxInfoAnimTexture_c::FxInfoAnimTexture_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (ECX = this), no args, plain `ret` (0x4A65B4); returns `this` in EAX (0x4A65A7). SEH frame
  (handler 0x839203).
- Callers: FxInfoManager_c::AddFxInfo 0x4A7B00. Callees: FxInterpInfo32_c ctor 0x4A8990 (this+8), FxMemoryPool_c::GetMem
  0x4A9CA0 (ECX = 0xA9AF2C, g_fxMan.m_Pool).
- Behaviour: `[this] = 0x85A920` (0x4A6578); FxInterpInfo32_c ctor on this+8 (0x4A657E); `word [this+4] = 0x4080`
  (0x4A6592, FX_INFO_ANIMTEX_DATA); `byte [this+0xE] = 1` (0x4A6598, m_nCount); `[this+0x14] = GetMem(4, 4)` (0x4A6585-0x4A65A4,
  one key-array pointer).
- gta-reversed: `m_nType = FX_INFO_ANIMTEX_DATA; m_InterpInfo.Allocate(1)`: same (word vs byte store of m_nCount only).
  Open questions: none.

## 0x4A65E0 FxInfoAnimTexture_c::GetValue  (catalogue: specified/REA, port level 1)
- Convention: as FxInfoAttractLine_c::GetValue (`ret 0x18`), info = RenderInfo_t*.
- Behaviour: `t = this->m_bTimeModeParticle (+6) ? mult : currentTime / len` (x87 `fld currentTime; fdiv len; fstp` to an f32 slot); `FxInterpInfo32_c::GetVal(this+8, v, t)` 0x4A89C0; `info->m_bHasAnimTextures (+0x34) = 1`; `info->m_nCurrentTexId (+0x35) =
  (u8)_ftol(v[0])` (0x821B40, truncation, low byte).
- gta-reversed: FxInfoAnimTexture.cpp - same. Open questions: none.
