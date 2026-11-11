# FxInfoEmPos_c (RENDER) - full specs

Layout (gr `Fx/Blueprints/FxInfo.h`, `FxInfoEmPos.h`): +0 vtable, +4 m_nType (u16 eFxInfoType), +6 m_bTimeModeParticle (bool),
+8 m_InterpInfo (FxInterpInfoFloat_c, 0x10 bytes: +8 vtable, +0xC m_bLooped, +0xD m_nNumKeys, +0xE m_nCount (byte in the original),
+0x10 m_pTimes, +0x14 m_Keys). Object size 0x18, allocated by the caller from the Fx pool (FxMemoryPool_c::GetMem(0x18, 4),
memory not cleared). Vtable 0x85A824.

## 0x4A5010 FxInfoEmPos_c::FxInfoEmPos_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (ECX = this), no args, plain `ret` (0x4A5074); returns `this` in EAX (0x4A5067). SEH frame
  (handler 0x838F63, state 0 stored at 0x4A5030, state 1 after the member ctor at 0x4A504C; unwind funclets call FxInfo_c::~FxInfo_c 0x4A4940, FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4A8460).
- Callers: FxInfoManager_c::AddFxInfo 0x4A7B00 (only caller; hits=2). Callees: FxInterpInfoFloat_c ctor 0x4A8440 (thiscall on this+8), FxMemoryPool_c::GetMem 0x4A9CA0 (thiscall on g_fxMan.m_Pool 0xA9AF2C = g_fxMan 0xA9AE80 + 0xAC, args (size, align), `ret 8`).
- Behaviour (in order):
  1. `[this+0] = 0x85A824` (0x4A5038; the FxInfo_c base ctor is inlined and only the final vtable is stored).
  2. FxInterpInfoFloat_c ctor on `this+8` (0x4A503E): stores the interp vtable at +8 and zeroes byte +0xC (m_bLooped), dword +0x10 (m_pTimes)
     and dword +0x14 (m_Keys); +0xD (m_nNumKeys) is not touched.
  3. `word [this+4] = 0x1080` (0x4A5051); `byte [this+0xE] = 3` (0x4A5057, m_InterpInfo.m_nCount, only the low byte; +0xF stays as allocated).
  4. `[this+0x14] = GetMem(0xC, 4)` (call 0x4A505B, store 0x4A5064): 3 key-array pointer(s), m_InterpInfo.m_Keys. Memory not cleared.
     +6 (m_bTimeModeParticle) is not written.
- gta-reversed: `m_nType = 0x1080; m_InterpInfo.Allocate(3)` - Allocate writes `m_nCount = 3` as int16 (two bytes, also clears +0xF)
  and `m_Keys = g_fxMan.Allocate<float*>(3)` = GetMem(12, 4): same except the byte vs. word store (harmless: +0xF is not read
  by the original readers, which use `byte [+0xE]`). Open questions: none.

## 0x4A50A0 FxInfoEmPos_c::GetValue  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (FxInfoEmPos_c* this, float currentTime [esp+4], float mult, float totalTime, float len, bool useConst,
  void* info [esp+0x18])`, `ret 0x18`. Local float buffer of 0x40 bytes.
- Behaviour: `FxInterpInfoFloat_c::GetVal(this+8, values, currentTime)` 0x4A8470 (thiscall (float* out, float t)); then
  `info->+0x44 = values[0]` (via `fld; fstp`, f32 -> f32), `info->+0x48 = values[1]`, `info->+0x4C = values[2]` (dword moves)
  (EmissionInfo_t::m_Pos).
- gta-reversed: Fx/Blueprints/FxInfoEmPos.cpp:13 - same. Open questions: none.
