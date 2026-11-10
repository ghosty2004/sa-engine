# FxSystem_c (RENDER) - full specs

FxSystem_c layout (FxSystem.h, offsets confirmed by the bytes): +0x00/+0x04 list links, +0x08 m_SystemBP, +0x0C m_ParentMatrix
(RwMatrix*), +0x10 m_LocalMatrix (RwMatrix, 0x40), +0x50 m_nPlayStatus (0 PLAYING, 1 STOPPED, 2 T2), +0x51 m_nKillStatus,
+0x52 m_UseConstTime, +0x54 m_fCurrentTime, +0x58 m_fCameraDistance, +0x5C m_nConstTime (u16), +0x5E m_nRateMult (u16),
+0x60 m_nTimeMult (u16), +0x62 flag byte (0x08 m_stopParticleCreation, 0x10 m_prevCulled, 0x20 m_MustCreateParticles),
+0x64 m_LoopInterval, +0x68 m_VelAdd, +0x74 m_BoundingSphere, +0x78 m_Prims (FxPrim_c**).
FxSystemBP_c: +0x0C m_fLength, +0x10 m_fLoopIntervalMin, +0x14 m_fLoopLength, +0x18 m_nCullDist (u16, 8.8 fixed),
+0x1A m_nPlayMode, +0x1B m_nNumPrims (i8). FxPrim_c: +0x0C m_bEnabled; vtable +0x08 Update(time, dt),
+0x10 AddParticle(RwMatrix&...), +0x14 AddParticle(CVector&...).
`g_fx` = 0xA9AE00, `g_fxMan` = 0xA9AE80.

## 0x4AA440 FxSystem_c::AddParticle (CVector* pos)  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (this = ECX, const CVector* pos [esp+4], const CVector* vel [+8], float timeSince [+0xC],
  FxPrtMult_c* mults [+0x10], float rotZ [+0x14], float lightMult [+0x18], float lightMultLimit [+0x1C], bool createLocal
  [+0x20])`, `ret 0x20`.
- Callers: 42 (Fx_c::AddBlood/AddWood/AddSparks/AddBulletImpact/..., CGarage::Update, script 2300-2399). Callees: `rand`
  0x821B1E, `_ftol` 0x821B40, Fx_c::GetFxQuality 0x49EA50 (thiscall, ECX = &g_fx, returns int), prim vfunc +0x14.
- Behaviour:
  1. `r = ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14 = 2^-15] * 100.0f [0x858628])` (0..99), always (one rand()
     per call, 0x4AA44B-0x4AA469).
  2. `q = GetFxQuality()`; if `q == 0` and `r < 50` return (0x4AA47A-0x4AA481). `q = GetFxQuality()` (second call); if `q == 1`
     and `r < 25` return (0x4AA48C-0x4AA499).
  3. `b = 1.0f`; if `lightMult < lightMultLimit` (`test ah,5; jp`, NaN -> 1.0): `b = (1.0f [0x858624] - lightMultLimit) +
     lightMult` (stored f32, 0x4AA49F-0x4AA4C4).
  4. For `i = 0; i < (i8)m_SystemBP->+0x1B; i++` (count re-read each iteration, 0x4AA521-0x4AA52F): `p = m_Prims[i]`; if
     `p->+0x0C != 0`: `p->vfunc+0x14(pos, vel, timeSince, mults, rotZ, b, createLocal)` (thiscall).
- Notes: the loop counter is kept in the caller's `lightMultLimit` stack slot ([esp+0x34]); harmless.
- gta-reversed: FxSystem.cpp:184, hooked `.Reversed = false`. gr's `CanAddParticle` uses `CGeneral::RandomBool(50/75)`
  (`GetRandomNumberInRange(0, 100) <= chance`, different RNG mapping and pass condition: the original passes on `r >= 50`
  (LOW) / `r >= 25` (MEDIUM) and calls GetFxQuality twice). Rest matches. Open questions: none.

## 0x4AA540 FxSystem_c::AddParticle (RwMatrix* mat)  (catalogue: specified/REA, port level 1)
- Convention: as 0x4AA440 with `const RwMatrix* mat` as the first argument; `ret 0x20`. No callers in the binary.
- Behaviour: steps 1-3 identical to 0x4AA440 (0x4AA545-0x4AA5BA, same constants). Loop (0x4AA5D4-0x4AA604): for every
  prim (no m_bEnabled test) `p->vfunc+0x10(mat, vel, timeSince, mults, rotZ, b, createLocal)`.
- gta-reversed: FxSystem.cpp:197 (also no enabled check), same RNG difference as above. Open questions: none.

## 0x4AA9A0 FxSystem_c::UpdateBoundingBoxCB  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl (Particle_c* p [esp+4], int32 a2 [+8], FxBox_c** data [+0xC])`, plain `ret`. Used as a callback by
  FxSystem_c::GetBoundingBox 0x4AAA40 (via ForAllParticles). Return value in EAX is not meaningful: `a2` when `a2 != 0`,
  otherwise the last `fnstsw` word or a copied float.
- Behaviour: if `a2 != 0` return (0x4AA9A6). `box = *data` (re-read before every test). With `P = p->+0x10/+0x14/+0x18`:
  `if (P.x < box->minX (+0)) box->minX = P.x`; `if (P.x > box->maxX (+4)) box->maxX = P.x`; same for y with +8/+0xC and z
  with +0x10/+0x14 (order x-min, x-max, y-min, y-max, z-min, z-max). Mins use `test ah,5; jp` (strict <), maxes use
  `test ah,0x41; jne` (strict >); NaN never updates. Values are copied as raw dwords.
- gta-reversed: FxSystem.cpp:306 calls the original. Open questions: none.

## 0x4AAF70 FxSystem_c::Update  (catalogue: specified/REA, port level 7)
- Convention: `__thiscall bool (this = ECX, RwCamera* cam [esp+4], float timeStep [esp+8])`, `ret 8`. Returns AL = 1 only when
  the system is dead (kill status 3), else 0.
- Callers: FxManager_c::Update 0x4A9A80. Callees: FxManager_c::FxRwMatrixCreate 0x4A9440 / FxRwMatrixDestroy 0x4A9460
  (thiscall on g_fxMan), RwMatrixMultiply 0x7F18B0 (cdecl (dst, a, b)), RwV3dLength 0x7EDAC0 (-> ST0), FxSystem_c::IsVisible
  0x4AAF30, Play 0x4AA2F0, Stop 0x4AA390, DoFxAudio 0x4AAC90 (thiscall, CVector by value), `rand`, prim vfunc +8.
- Constants: 0.00390625f [0x859AA0], 0.001f [0x858CDC], 0.0f [0x858B50], 9.9999997e-05f [0x858FC4], 0.25f [0x858C84].
- Behaviour:
  1. `if (kill == 3) return 1`; `if (kill == 2) { kill = 3; return 0; }` (0x4AAF77-0x4AAF9D).
  2. `m = FxRwMatrixCreate()`; if `m_ParentMatrix == 0` copy the 16 dwords of m_LocalMatrix into m, else
     `RwMatrixMultiply(m, &m_LocalMatrix, m_ParentMatrix)` (0x4AAFA0-0x4AAFCC).
  3. `prevDist = m_fCameraDistance`; `FxRwMatrixDestroy(m)` (0x4AAFE9) - the position `m->pos` (m+0x30) is read AFTER this call
     (here and for DoFxAudio below: use after release, see open questions). `camPos = ((RwFrame*)cam->+4)->+0x40` (frame
     modelling-matrix pos). `d = camPos - m->pos` (x, y, z, each stored f32); `m_fCameraDistance = RwV3dLength(&d)` (fst, value
     stays in ST0).
  4. `cull = (float)(u16)BP->+0x18 * 0.00390625f` (stored f32). `culled = 0`.
     If `BP->m_nPlayMode (+0x1A) == 0`: drop ST0 and go to step 6 (never culled).
     Else: if `dist < cull` (`test ah,5; jp`; NaN = not less) and `IsVisible()` -> step 6; otherwise `culled = 1`, step 5.
  5. Culled / not-playing path (0x4AB058, also entered from step 6 with culled = 0 when play status != 0):
     if `playStatus == 1` and `m_fCameraDistance < cull` and `!(prevDist < cull)` (`test ah,1; jne`) and `BP->+0x1A == 3`:
     `Play()`. Then (0x4AB08B) set flag bit 0x10 (m_prevCulled) to `culled` (`flags ^= ((culled << 4) ^ flags) & 0x10`);
     if `playStatus (re-read) == 0`: `DoFxAudio(m->pos)` (by value). Return 0.
  6. Not culled (0x4AB0CE): if `playStatus != 0` go to step 5 with culled = 0. Otherwise:
     a. `dt = ((float)(u16)m_nTimeMult (+0x60) * 0.001f) * timeStep` (stored f32 in the timeStep slot).
     b. `m_fCurrentTime = m_UseConstTime ? (float)(u16)m_nConstTime (+0x5C) * 0.00390625f : dt + m_fCurrentTime`.
     c. `mode = BP->+0x1A`. If `mode == 2` and `BP->m_fLoopIntervalMin (+0x10) > 0` (`test ah,0x41; jne` -> not taken only for
        > 0): if `cur > BP->m_fLength (+0xC)` set flag 0x08; `lim = BP->m_fLength + m_LoopInterval (+0x64)`; if `lim < cur`:
        `cur = cur - lim` (stored), `r = rand() % 10000` (signed idiv), clear flag 0x08,
        `m_LoopInterval = (float)r * 9.9999997e-05f * (BP->+0x14 - BP->+0x10) + BP->+0x10` (0x4AB161-0x4AB18F).
     d. Else (0x4AB194): if `cur > BP->m_fLength` (NaN no): mode 0: `Stop()`, and if `kill == 1` also `kill = 2`
        (Stop first, 0x4AB1AD-0x4AB1B2); mode 1: `cur = BP->m_fLength`; mode 2: `cur = cur - BP->m_fLength`; mode 3: `Stop()`;
        other modes: nothing.
     e. If flag 0x10 (prevCulled from the previous frame): `dt += 0.25f` (stored).
     f. For `i = 0 .. (i8)BP->+0x1B - 1` (re-read each iteration): `m_Prims[i]->vfunc+8(m_fCurrentTime, dt)` (all prims, no
        enabled test).
     g. Go to 0x4AB08B with culled = 0 (clears m_prevCulled; DoFxAudio if still playing). Return 0.
- Edge cases: the cull distance compares use the f32 stored distance in step 5 but the ST0 value (same) in step 4.
- gta-reversed: FxSystem.cpp:417 calls the original (no C++ body). Open questions: none. Note: FxRwMatrixDestroy(m) runs
  (0x4AAFE9) before `m->pos` is read (0x4AAFEE) and copied for DoFxAudio (0x4AB0A4); this is safe because the matrix pool
  is a stack: Destroy 0x4A9460 only does `g_fxMan.+0x88--` and Create 0x4A9440 returns `g_fxMan.+0x8C[+0x88++]`, so the
  memory is untouched until the next Create. A port must keep this stack discipline.
