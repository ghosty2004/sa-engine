# FxInfoHeatHaze_c (RENDER) - full specs

Layout (gr `Fx/Blueprints/FxInfo.h`, `FxInfoHeatHaze.h`): +0 vtable, +4 m_nType (u16 eFxInfoType), +6 m_bTimeModeParticle (bool),
+8 m_InterpInfo (FxInterpInfo255_c, 0x10 bytes: +8 vtable, +0xC m_bLooped, +0xD m_nNumKeys, +0xE m_nCount (byte in the original),
+0x10 m_pTimes, +0x14 m_Keys). Object size 0x18, allocated by the caller from the Fx pool (FxMemoryPool_c::GetMem(0x18, 4),
memory not cleared). Vtable 0x85A8F0.

## 0x4A61A0 FxInfoHeatHaze_c::FxInfoHeatHaze_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (ECX = this), no args, plain `ret` (0x4A61EE); returns `this` in EAX (0x4A61E1). SEH frame
  (handler 0x839178, state 0 stored at 0x4A61C0; unwind funclets call FxInfo_c::~FxInfo_c 0x4A4940).
- Callers: FxInfoManager_c::AddFxInfo 0x4A7B00 (only caller; hits=2). Callees: FxInterpInfo255_c ctor 0x4A8B50 (thiscall on this+8).
- Behaviour (in order):
  1. `[this+0] = 0x85A8F0` (0x4A61C8; the FxInfo_c base ctor is inlined and only the final vtable is stored).
  2. FxInterpInfo255_c ctor on `this+8` (0x4A61CE): stores the interp vtable at +8 and zeroes byte +0xC (m_bLooped), dword +0x10 (m_pTimes)
     and dword +0x14 (m_Keys); +0xD (m_nNumKeys) is not touched.
  3. `word [this+4] = 0x4008` (0x4A61D7); `byte [this+0xE] = 0` (0x4A61DD, m_InterpInfo.m_nCount).
  No GetMem call: m_InterpInfo.m_Keys (+0x14) stays NULL from step 2; +0xD, +0xF and +6 are not written (pool memory contents).
- gta-reversed: `m_nType = 0x4008; m_InterpInfo.Allocate(0)` - DIFFERS: gr's Allocate(0) calls `g_fxMan.Allocate<...>(0)` =
  GetMem(0, 4) and stores the returned (non-NULL) pool pointer into +0x14 where the original leaves NULL; it also writes
  m_nCount as int16. GetMem(0, 4) only realigns the pool position to 4 (0x4A9CA9-0x4A9CB7), which is a no-op here because
  AddFxInfo's GetMem(0x18, 4) just left it aligned. Harmless (count 0: no reader dereferences m_Keys), but the port keeps
  the original (no pool call, m_Keys NULL). Open questions: none.
