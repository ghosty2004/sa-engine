# FxEmitter_c (RENDER) - full specs

FxEmitter_c: +0x04 m_PrimBP (FxEmitterBP_c*; its FxInfoManager at +0x2C, m_nLodStart u16 at +0x38, m_nLodEnd u16 at +0x3A),
+0x08 m_System (FxSystem_c*, layout in FxSystem_c.md), +0x10 m_fEmissionIntensity (float). EmissionInfo_t local (0x68 bytes):
+0x00 m_fCount, +0x58 m_fMinWind, +0x5C m_fMaxWind, +0x60 m_fMinRain, +0x64 m_fMaxRain (other fields: FxEmitter_c.deviations.md).

## 0x4A41E0 FxEmitter_c::CreateParticles  (catalogue: specified/REA, port level 5)
- Convention: `__thiscall void (this = ECX, float currentTime [esp+4], float deltaTime [esp+8])`, `ret 8`.
- Callers: FxEmitter_c::Update 0x4A4460 (tail call). Callees: FxInfoManager_c::ProcessEmissionInfo 0x4A4960 (thiscall on
  m_PrimBP+0x2C, args (currentTime, deltaTime, m_System->m_SystemBP->m_fLength (+0xC), (u8)m_System->m_UseConstTime, &emission)),
  FxManager_c::FxRwMatrixCreate 0x4A9440 / FxRwMatrixDestroy 0x4A9460 (thiscall on g_fxMan 0xA9AE80; a stack allocator),
  RwMatrixUpdate 0x7F18A0, RwMatrixMultiply 0x7F18B0 (cdecl (dst, a, b)), FxPrimBP_c::GetRWMatrix 0x4A9DC0, FxPrtMult_c ctor
  0x4AB270, FxEmitter_c::CreateParticle 0x4A2580, `_ftol` 0x821B40.
- Globals: CWeather::Wind 0xC812F0, CWeather::Rain 0xC81324. Constants: 0.001f [0x858CDC], 0.015625f [0x85A7D4], 1.0f [0x858624],
  0.0f [0x858B50], 1.2f (imm 0x3F99999A).
- Behaviour:
  1. `ProcessEmissionInfo(...)` into the local `emission` (0x4A41E9-0x4A4214).
  2. `rate = (float)(u16)m_System->m_nRateMult (+0x5E) * 0.001f` (kept in ST0); `lodStart = (float)(u16)m_PrimBP->+0x38 *
     0.015625f`, `lodEnd = (float)(u16)m_PrimBP->+0x3A * 0.015625f` (both stored f32).
  3. `d = m_System->m_fCameraDistance (+0x58)`. `vis = 1.0f` if `d < lodStart` (`test ah,5; jp`); else `vis = 0.0f` if
     `d > lodEnd` (`test ah,0x41; jne` -> less, equal and NaN compute); else `vis = 1.0f - (d - lodStart) / (lodEnd - lodStart)`
     (0x4A4289-0x4A429A).
  4. `m_fEmissionIntensity = ((emission.m_fCount * vis) * rate) + m_fEmissionIntensity` (stored f32 via a local copy, 0x4A42A0-0x4A42B5).
  5. Return unless all hold (0x4A42BA-0x4A4329): `!(Wind < minWind)`, `!(Wind > maxWind)`, `!(Rain < minRain)`,
     `!(Rain > maxRain)` (NaN passes every test), `!(m_fEmissionIntensity < 1.0f)`.
  6. `mat = Create(); mat1 = Create(); RwMatrixUpdate(&m_System->m_LocalMatrix (+0x10))`; if `m_System->m_ParentMatrix (+0xC)`
     is null copy the 16 dwords of the local matrix into mat1, else `RwMatrixMultiply(mat1, &local, parent)`;
     `mat2 = Create(); m_PrimBP->GetRWMatrix(mat2); RwMatrixMultiply(mat, mat2, mat1); Destroy(mat2)` (0x4A4332-0x4A43AB).
  7. `i = 0`; while `i < ftol(m_fEmissionIntensity)` (signed; ftol re-evaluated before each iteration, first test `jle` at
     0x4A43C0): construct a fresh FxPrtMult_c `pm` (0x4A43C6); `t = ((float)i / m_fEmissionIntensity) * deltaTime`;
     `CreateParticle(&emission, mat, nullptr, t, &pm, 1.2f, (m_System->+0x62 >> 1) & 1 /*m_createLocal*/)`; `i++`.
  8. `m_fEmissionIntensity = m_fEmissionIntensity - (float)ftol(m_fEmissionIntensity)` (keeps the fractional part,
     0x4A4418-0x4A4431). `Destroy(mat1); Destroy(mat)` (reverse order of creation).
- Edge cases: intensity values above 2^31 make ftol return 0x80000000 (negative) -> no particles, and step 8 then subtracts
  -2^31. CreateParticle can return null (no free particle) - the loop does not check.
- gta-reversed: FxEmitter.cpp:121, hooked `.Reversed = false` (code after `plugin::CallMethod` is dead). Differences of that
  code from the bytes: it multiplies the rate by `1000.0f` (original `* 0.001f`), and it sets
  `m_fEmissionIntensity -= m_fEmissionIntensity` (= 0; original keeps the fraction). Both are gr errors; port the bytes.
  Open questions: none.

## 0x4A1920 FxEmitter_c::FxEmitter_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall FxEmitter_c* (this)`, plain `ret`, returns this. Body: `FxPrim_c::FxPrim_c(this)` 0x4A9F20; `vtable = 0x85A7A4`.
- gta-reversed: FxEmitter.h default ctor - matches. Open questions: none.
