# FxInfoAttractLine_c (RENDER) - full specs

Layout (gr `Fx/Blueprints/FxInfo.h`, `FxInfoAttractLine.h`): +0 vtable, +4 m_nType (u16 eFxInfoType), +6 m_bTimeModeParticle (bool),
+8 m_InterpInfo (FxInterpInfo32_c, 0x10 bytes: +8 vtable, +0xC m_bLooped, +0xD m_nNumKeys, +0xE m_nCount (byte in the original),
+0x10 m_pTimes, +0x14 m_Keys). Object size 0x18, allocated by the caller from the Fx pool (FxMemoryPool_c::GetMem(0x18, 4),
memory not cleared). Vtable 0x85A878.

## 0x4A57C0 FxInfoAttractLine_c::FxInfoAttractLine_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (ECX = this), no args, plain `ret` (0x4A5824); returns `this` in EAX (0x4A5817). SEH frame
  (handler 0x839043, state 0 stored at 0x4A57E0, state 1 after the member ctor at 0x4A57FC; unwind funclets call FxInfo_c::~FxInfo_c 0x4A4940, FxInterpInfo32_c::~FxInterpInfo32_c 0x4A89B0).
- Callers: FxInfoManager_c::AddFxInfo 0x4A7B00 (only caller; hits=0). Callees: FxInterpInfo32_c ctor 0x4A8990 (thiscall on this+8), FxMemoryPool_c::GetMem 0x4A9CA0 (thiscall on g_fxMan.m_Pool 0xA9AF2C = g_fxMan 0xA9AE80 + 0xAC, args (size, align), `ret 8`).
- Behaviour (in order):
  1. `[this+0] = 0x85A878` (0x4A57E8; the FxInfo_c base ctor is inlined and only the final vtable is stored).
  2. FxInterpInfo32_c ctor on `this+8` (0x4A57EE): stores the interp vtable at +8 and zeroes byte +0xC (m_bLooped), dword +0x10 (m_pTimes)
     and dword +0x14 (m_Keys); +0xD (m_nNumKeys) is not touched.
  3. `word [this+4] = 0x2010` (0x4A5801); `byte [this+0xE] = 7` (0x4A5807, m_InterpInfo.m_nCount, only the low byte; +0xF stays as allocated).
  4. `[this+0x14] = GetMem(0x1C, 4)` (call 0x4A580B, store 0x4A5814): 7 key-array pointer(s), m_InterpInfo.m_Keys. Memory not cleared.
     +6 (m_bTimeModeParticle) is not written.
- gta-reversed: `m_nType = 0x2010; m_InterpInfo.Allocate(7)` - Allocate writes `m_nCount = 7` as int16 (two bytes, also clears +0xF)
  and `m_Keys = g_fxMan.Allocate<int16*>(7)` = GetMem(28, 4): same except the byte vs. word store (harmless: +0xF is not read
  by the original readers, which use `byte [+0xE]`). Open questions: none.

## 0x4A5830 FxInfoAttractLine_c::Load  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (this, FILESTREAM file [esp+4], int32 version [esp+8])`, `ret 8`.
- Behaviour: `this->m_InterpInfo (+8)` virtual Load (its vtable slot 1, +4) `(file)`; version unused.
- gta-reversed: Fx/Blueprints/FxInfoAttractLine.cpp:9 - same. Open questions: none.

## 0x4A5850 FxInfoAttractLine_c::GetValue  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall void (this, float currentTime [esp+4], float mult [esp+8], float totalTime [esp+0xC], float len [esp+0x10],
  bool useConst [esp+0x14], MovementInfo_t* info [esp+0x18])`, `ret 0x18`.
- Callees: FxInterpInfo32_c::GetVal 0x4A89C0 (thiscall (float* out, float t)); FindClosestPtOnLine 0x4A4880 (cdecl (out, lineA, lineB, point));
  RwV3dLength 0x7EDAC0; RwV3dNormalize 0x7ED9B0 (cdecl (out, in), returns length in ST0 - popped).
- Globals: 0.0f [0x858B50]=0.0f; 0.1f [0x858B1C]=0.1f.
- Behaviour: `t = this->m_bTimeModeParticle (+6) ? mult : currentTime / len` (x87 `fld currentTime; fdiv len; fstp` to an f32 slot); `GetVal(this+8, v[7], t)`; lineA = v[0..2], lineB = v[3..5] (copied to locals);
  `FindClosestPtOnLine(out, lineA, lineB, info->m_Pos (+0))`; `in = out - pos` (x: `out.x - pos.x`; y, z via fsubr = `out - pos`; f32);
  `dist = RwV3dLength(&in)` (f32); if `dist > 0` (`test ah,0x41; jne`) `RwV3dNormalize(&in, &in)`. If `dist > 0.1f`:
  `f = v[6] * totalTime` (st); `info->vel.x (+0xC) = in.x*f + vel.x`; `in.y' = f*in.y` (f32), `in.z' = in.z*f` (f32);
  `vel.y = in.y' + vel.y`; `vel.z = in.z' + vel.z`.
- gta-reversed: Fx/Blueprints/FxInfoAttractLine.cpp (GetValue) - same control flow, but gr computes `in * values[6] * totalTime`
  ((in*v6)*tt) while the original computes `in * (v6*tt)` (product first): rounding can differ - port the original order. gr uses
  Magnitude/Normalise instead of the RW functions. Open questions: none.
