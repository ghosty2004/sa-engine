# FxInfoSmoke_c (RENDER) - full specs

Layout (gr `Fx/Blueprints/FxInfo.h`, `FxInfoSmoke.h`): +0 vtable, +4 m_nType (u16 eFxInfoType), +6 m_bTimeModeParticle (bool),
+8 m_InterpInfo (FxInterpInfoU255_c, 0x10 bytes: +8 vtable, +0xC m_bLooped, +0xD m_nNumKeys, +0xE m_nCount (byte in the original),
+0x10 m_pTimes, +0x14 m_Keys). Object size 0x18, allocated by the caller from the Fx pool (FxMemoryPool_c::GetMem(0x18, 4),
memory not cleared). Vtable 0x85A950.

## 0x4A68E0 FxInfoSmoke_c::FxInfoSmoke_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (ECX = this), no args, plain `ret` (0x4A6944); returns `this` in EAX (0x4A6937). SEH frame
  (handler 0x839283, state 0 stored at 0x4A6900, state 1 after the member ctor at 0x4A691C; unwind funclets call FxInfo_c::~FxInfo_c 0x4A4940, FxInterpInfoU255_c::~FxInterpInfoU255_c 0x4A87F0).
- Callers: FxInfoManager_c::AddFxInfo 0x4A7B00 (only caller; hits=0). Callees: FxInterpInfoU255_c ctor 0x4A87D0 (thiscall on this+8), FxMemoryPool_c::GetMem 0x4A9CA0 (thiscall on g_fxMan.m_Pool 0xA9AF2C = g_fxMan 0xA9AE80 + 0xAC, args (size, align), `ret 8`).
- Behaviour (in order):
  1. `[this+0] = 0x85A950` (0x4A6908; the FxInfo_c base ctor is inlined and only the final vtable is stored).
  2. FxInterpInfoU255_c ctor on `this+8` (0x4A690E): stores the interp vtable at +8 and zeroes byte +0xC (m_bLooped), dword +0x10 (m_pTimes)
     and dword +0x14 (m_Keys); +0xD (m_nNumKeys) is not touched.
  3. `word [this+4] = 0x8001` (0x4A6921); `byte [this+0xE] = 8` (0x4A6927, m_InterpInfo.m_nCount, only the low byte; +0xF stays as allocated).
  4. `[this+0x14] = GetMem(0x20, 4)` (call 0x4A692B, store 0x4A6934): 8 key-array pointer(s), m_InterpInfo.m_Keys. Memory not cleared.
     +6 (m_bTimeModeParticle) is not written.
- gta-reversed: `m_nType = 0x8001; m_InterpInfo.Allocate(8)` - Allocate writes `m_nCount = 8` as int16 (two bytes, also clears +0xF)
  and `m_Keys = g_fxMan.Allocate<uint16*>(8)` = GetMem(32, 4): same except the byte vs. word store (harmless: +0xF is not read
  by the original readers, which use `byte [+0xE]`). Open questions: none.

## 0x4A6950 FxInfoSmoke_c::Load  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (this, FILESTREAM file [esp+4], int32 version [esp+8])`, `ret 8`.
- Behaviour: `this->m_InterpInfo (+8)` virtual Load (its vtable +4) `(file)`; version unused. Same code as FxInfoAttractLine_c::Load 0x4A5830.
- gta-reversed: FxInfoSmoke_c Load - same. Open questions: none.

## 0x4A6970 FxInfoSmoke_c::GetValue  (catalogue: specified/REA, port level 1)
- Convention: as FxInfoAttractLine_c::GetValue (`ret 0x18`), info = RenderInfo_t*.
- Behaviour: `t = this->m_bTimeModeParticle (+6) ? mult : currentTime / len` (x87 `fld currentTime; fdiv len; fstp` to an f32 slot); `FxInterpInfoU255_c::GetVal(this+8, v[8], t)` 0x4A8800; `info->m_SmokeType (+0x7B) = (u8)_ftol(v[0])`; dword copies
  `+0x88..+0xA0 = v[1..7]` (m_SmokeColor r,g,b,a, m_SmokeBrightness, m_SmokeSize, m_SmokeLife).
- gta-reversed: FxInfoSmoke.cpp - same. Open questions: none.
