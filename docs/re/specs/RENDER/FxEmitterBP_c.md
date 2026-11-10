# FxEmitterBP_c (RENDER) - full specs

Layout (bytes, gr `Fx/FxPrimBP.h` + `FxEmitterBP.h`, size 0x48): +0 vtable (0x85A788), +4 m_Type (i8), +5 m_nSrcBlendId (i8),
+6 m_nDstBlendId (i8), +7 m_bAlphaOn (bool), +8 m_pMatrixBuffered, +0xC..+0x18 m_apTextures[4] (RwTexture*, texture +0 = raster),
+0x1C m_TxdIndex, +0x20 m_Particles (TList_c: +0x20 head, +0x24 tail, +0x28 count), +0x2C m_FxInfoManager (FxInfoManager_c:
+0x2C m_nNumInfos (i32), +0x30 m_pInfos (FxInfo_c**), +0x34 m_MovementOffset (i8), +0x35 m_RenderOffset, +0x38 m_nLodStart (u16),
+0x3A m_nLodEnd (u16), +0x3C m_bHasFlatParticleEmitter, +0x3D m_bHasHeatHazeParticleEmitter), +0x40 gr m_nLodStart / +0x42 m_nLodEnd
(never written by the original Load, see 0x5C25F0), +0x44 m_bHasInfoFlatData, +0x45 m_bHasInfoHeatHazeData.
FxInfo_c: +4 m_nType (u16).

Particle (FxEmitterPrt_c, 0x3C bytes; list item): +0 prev, +4 next, +8 m_fTotalLife, +0xC m_fCurrentLife, +0x10 m_Pos, +0x1C m_Velocity,
+0x28 m_System (FxSystem_c*), +0x2C..+0x2F m_MultColor r,g,b,a (u8), +0x30 m_MultSize (u8), +0x31 m_MultRot (u8), +0x32/+0x33/+0x34
random bytes (gr m_RandR/G/B), +0x35 m_Brightness (u8, percent), +0x36 m_RotZ (u8, 0xFF = none), +0x37 m_bLocalToSystem, +0x38
m_CurrentRotation (f32, degrees). gr declares the class with a second `ListItem_c` base, which would shift these fields by 8 and makes
gr's VALIDATE_SIZE 0x44; the original element is 0x3C (see FxEmitterPrt_c.md) and all offsets above are from the bytes.
FxSystem_c: +8 m_SystemBP (FxSystemBP_c*, +0xC m_fLength), +0x50 m_nPlayStatus (u8), +0x51 m_nKillStatus (u8), +0x54 m_fCurrentTime,
+0x60 m_nTimeMult (u16).
TList_c<T>::GetNumItems 0x4A8EC0 = `return [ecx+8]`; TList_c::RemoveItem 0x4A8E30 (thiscall, item at [esp+4], ret 4): unlinks
(`item->next ? next->prev = item->prev : list->tail = item->prev`; `item->prev ? prev->next = item->next : list->head = item->next`;
`count--`), does not clear the item's links.
RenderInfo_t (gr `Fx/RenderInfo.h`, 0xA4) offsets used below: +0 m_Color1 (r,g,b,a u8), +4 m_Color2, +8 m_nColorType, +0xC m_fSizeX,
+0x10 m_fSizeY, +0x14 m_fSizeXBias, +0x18 m_fSizeYBias, +0x1C m_fSpriteTop, +0x20 m_fSpriteBottom, +0x24 m_fSpriteLeft, +0x28
m_fSpriteRight, +0x2C m_nTrailScreenMode (i32), +0x30 m_fTrailTime, +0x34 m_bHasAnimTextures, +0x35 m_nCurrentTexId, +0x37 m_bSelfLit,
+0x38 m_FlatMatrix (RwMatrix 0x40), +0x78 m_bIsFlat, +0x79 m_bHasDir, +0x7A m_bUseVel, +0x7B m_SmokeType (i8), +0x7C m_Direction,
+0x88 m_SmokeColor (4 x f32), +0x98 m_SmokeBrightness, +0x9C m_SmokeSize, +0xA0 m_SmokeLife.

Shared sub-steps of RenderHeatHaze and Render (described once here, referenced as S1..S4):
- S1 particle position: if `prt->m_bLocalToSystem` (+0x37): `m = g_fxMan.FxRwMatrixCreate()` 0x4A9440 (ECX = g_fxMan 0xA9AE80);
  `prt->m_System->GetCompositeMatrix(m)` 0x4AA8C0; `RwV3dTransformPoints(&P, &prt->m_Pos, 1, m)` 0x7EDD90; `FxRwMatrixDestroy(m)`
  0x4A9460. Otherwise `P = prt->m_Pos`.
- S2 render info: `FxInfoManager_c::ProcessRenderInfo(this+0x2C; currentTime = system+0x54, mult = prt->m_fCurrentLife /
  prt->m_fTotalLife (x87 fdiv, stored f32 as the arg), totalTime = 0.0f (int 0 pushed), length = system->m_SystemBP->m_fLength,
  useConst = 0, &ri)` 0x4A4A80 (thiscall, 6 args).
- S3 identity matrix: `M = FxRwMatrixCreate()`; right = (1,0,0), up = (0,1,0), at = (0,0,1), pos = (0,0,0) (dwords),
  `M->flags |= 0x20003` (rwMATRIXTYPEORTHONORMAL | rwMATRIXINTERNALIDENTITY).
- Camera matrix `CM = &RwCameraGetFrame(camera)->modelling` = `*(camera+4) + 0x10`.
- Cross products below are written component-wise in the exact x87 order: for `A x B` the code computes
  `x = A.y*B.z - A.z*B.y` etc. but loads the operands as stated per site.
- S4 rotation (0x4A1C4E-0x4A1D61 and 0x4A3604-0x4A3711, identical code): if `prt->m_RotZ` (+0x36) != 0xFF:
  `prt->m_CurrentRotation = (float)(u8)RotZ; += itself` (fild, `fadd st0,st0`, stored f32 = 2*RotZ). Then normalise:
  if `rot < 0` (`test ah,5; jp` falls through only for "less"): loop `rot += 360.0f [0x859E2C]` while `rot < 0` (x87, stored once at
  the end). If `!(rot < 360.0f)` (`test ah,1`; NaN counts as "less" and skips): loop `rot -= 360.0f` while `!(rot < 360.0f)`.
  Then if `rot > 0.0f` (strict; NaN false): `RotateVecAboutVec(&Rr, &M->right, &U, rot * 0.017453279f [0x85A7BC])` 0x4A1780
  (cdecl, 4 args; note the constant is 0x3C8EFA2E, slightly smaller than pi/180 = 0x3C8EFA35); then `Up = U x Rr` written as
  `Up.x = U.y*Rr.z - U.z*Rr.y`, `Up.y = U.z*Rr.x - U.x*Rr.z`, `Up.z = U.x*Rr.y - U.y*Rr.x`. Else `Rr = M->right`, `Up = M->up`.
  Finally `FxRwMatrixDestroy(M)`.
- S5 size: `ri.sizeX = (((float)rnd32 * (1/255 = 0.0039215689f [0x859A3C])) - 0.5f [0x858B8C]) * ri.sizeXBias + ri.sizeX`
  (rnd32 = prt byte +0x32), stored f32; `ri.sizeY` likewise with byte +0x33 and sizeYBias. If `prt->m_MultSize` (+0x30) < 0xFF
  (unsigned): `ri.sizeX = (b30 * (1/255)) * ri.sizeX`, then `ri.sizeY = (b30 * (1/255)) * ri.sizeY` (b30 re-read).
- S6 quad: `T = ri.top * ri.sizeY`, `B = ri.bottom * ri.sizeY`, `L = ri.left * ri.sizeX`, `Rt = ri.right * ri.sizeX` (each stored
  or kept as noted). Two `RenderAddTri_` 0x4A1410 calls (cdecl, 27 args = 9 floats xyz, 6 floats uv, 12 ints rgba, `add esp,0x6c`):
  tri 1: v1 = P + (L*Rr + T*Up), uv (0,0); v2 = P + (Rt*Rr + B*Up), uv (1,1); v3 = P + (Rt*Rr + T*Up), uv (1,0).
  tri 2: v1 = P + (Rt*Rr + B*Up), uv (1,1); v2 = P + (L*Rr + T*Up), uv (0,0); v3 = P + (L*Rr + B*Up), uv (0,1).
  All three vertices of both triangles use the same colour (r,g,b,a) of `ri.m_Color1` (as bytes, zero-extended).
  Exact float order per vertex component c (heat haze addresses / Render addresses):
  Rounding: stored as f32 are `Up.c*B` (c = x,y,z), `Rr.c*L` (x,y,z), `Rr.x*Rt`, `Rr.y*Rt`; kept in x87 registers are `Up.c*T`
  (x,y,z) and `Rr.z*Rt`. Pair sums stored as f32: `LT.c = Rr.c*L + Up.c*T` (all c), `RB.z = Up.z*B + Rr.z*Rt`,
  `RB.x = Rr.x*Rt + Up.x*B`, `RB.y = Rr.y*Rt + Up.y*B`. tri1 v3 = `(Rr.c*Rt + Up.c*T) + P.c` computed on the fly (z, then y, then x);
  tri2 v3 = `(Rr.c*L + Up.c*B) + P.c` (from the stored products). All other components are `pairsum + P.c`. Order of argument stores in each
  call: z3, y3, x3, z2, y2, x2, z1, y1, x1.

## 0x4A1940 FxEmitterBP_c::RenderHeatHaze  (catalogue: specified/REA, port level 7)
- Convention: `__thiscall` (ECX = this), args `RwCamera* camera` [esp+4], `uint32 txdHashKey` [esp+8], `float brightness` [esp+0xC],
  `ret 0xC` (0x4A21CC). txdHashKey and brightness are never read. Frame `sub esp,0x12C`.
- Callers: FxEmitterBP_c::Render 0x4A2C40 (only when its doHeatHaze arg is true and +0x3D is set).
- Callees: TList_c::GetNumItems 0x4A8EC0 (x2), RwRenderStateSet via `RwEngineInstance` (0xC97B24) `+0x20` (x3),
  Fx_c::RenderBegin 0x4A13B0 (raster, transform, flags), RenderEnd 0x4A1600, RenderAddTri_ 0x4A1410, RotateVecAboutVec 0x4A1780,
  ProcessRenderInfo 0x4A4A80, FxRwMatrixCreate 0x4A9440 / FxRwMatrixDestroy 0x4A9460, FxSystem_c::GetCompositeMatrix 0x4AA8C0,
  RwV3dNormalize 0x7ED9B0 (x2), RwV3dTransformPoints 0x7EDD90.
- Behaviour:
  1. If `GetNumItems(this+0x20) == 0` return (0x4A195A). `curRaster = m_apTextures[0]->raster` ([this+0xC] then [tex+0], 0x4A1960).
     If `GetNumItems == 0` (again) return.
  2. `RwRenderStateSet(12 VERTEXALPHAENABLE, 1)`, `(10 SRCBLEND, 5 SRCALPHA)`, `(11 DESTBLEND, 2 ONE)` (0x4A1980-0x4A199D);
     `RenderBegin(curRaster, nullptr, 1)` (0x4A19A6). `CM` from `camera` (0x4A19AB).
  3. For each particle `prt` from the list head (+0x20) following `next` (+4) until null (0x4A21AF):
     a. S1 -> P (local); S2 -> ri; S3 -> M.
     b. Orientation (0x4A1A8F):
        - if `ri.m_bHasDir`: `D = ri.m_bUseVel ? prt->m_Velocity : ri.m_Direction`; `RwV3dNormalize(&D, &D)` (result discarded);
          `V = P - CM.pos` (each `P.c - CM.pos.c`, stored f32); `RwV3dNormalize(&V, &V)`;
          `R.x = V.z*D.y - V.y*D.z`, `R.y = V.x*D.z - V.z*D.x`, `R.z = V.y*D.x - V.x*D.y` (= D x V; R is NOT normalised);
          `U.x = R.z*D.y - R.y*D.z`, `U.y = R.x*D.z - R.z*D.x`, `U.z = R.y*D.x - R.x*D.y` (= D x R; R.z is used from the register
          before its f32 store, same value under 24-bit precision); `M->right = R`, `M->up = D`, `M->at = U` (pos/flags keep S3 values).
        - else if `ri.m_bIsFlat`: copy the 64 bytes of `ri.m_FlatMatrix` over `*M` (all 16 dwords, flags and pos included);
          `U = ri.m_FlatMatrix.at`.
        - else: copy the 64 bytes of `*CM` over `*M`; `U = CM->at`.
     c. S4 rotation -> Rr, Up; M destroyed.
     d. S5 size.
     e. `ri.m_Color1.r = g = b = 0` (0x4A1E19-0x4A1E29; alpha keeps the ProcessRenderInfo value).
     f. Raster (0x4A1E10): `if (!ri.m_bHasAnimTextures || ri.m_nCurrentTexId == 1) r = m_apTextures[0]->raster`;
        `else if id == 2/3/4: r = m_apTextures[id-1] ? m_apTextures[id-1]->raster : m_apTextures[0]->raster`;
        `else r = r_prev` (local [esp+0x2C] keeps the value from the previous particle; uninitialised stack for the first particle -
        original bug, only reachable with an anim-texture id outside 1..4). If `r != curRaster`: `RenderEnd(); RenderBegin(r, nullptr, 1);
        curRaster = r`.
     g. S6 quad with colour (0,0,0,ri.alpha).
  4. After the loop `RenderEnd()` (0x4A21BD).
- Edge cases: the first early return (empty list) skips RenderBegin/RenderEnd entirely. The render states set in step 2 are not restored.
- gta-reversed: `plugin::CallMethod` only. Open questions: none.

## 0x4A21D0 FxEmitterBP_c::UpdateParticle  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall` (ECX = this), args `float deltaTime` [esp+4], `FxEmitterPrt_c* prt` [esp+8], `ret 8`; returns AL
  (1 = particle dead / remove, 0 = alive). Both arg slots are reused as locals (dt' and the water level).
- Callers: FxEmitter_c::CreateParticle 0x4A2580, FxEmitterBP_c::Update 0x4A2BC0. Callees: ProcessMovementInfo 0x4A4A10
  (thiscall on this+0x2C, args (currentTime, mult, totalTime, length, useConst, MovementInfo_t*), ret 0x18; it zeroes the info's
  m_Rot[4] and the two flags first), CWaterLevel::GetWaterLevel 0x6EB690 (cdecl (x, y, z, float* out, bool touching, CVector* n)).
- Behaviour:
  1. `dt' = ((float)system->m_nTimeMult (u16 +0x60, fild) * 0.001f [0x858CDC]) * deltaTime` (stored f32 into the deltaTime slot);
     `life = dt' + prt->m_fCurrentLife` (x87); `prt->m_fCurrentLife = life` (f32). If `!(life < prt->m_fTotalLife)` (`test ah,1`;
     NaN counts as "less" and continues) return 1 (0x4A2207).
  2. `pos.x = dt' * vel.x + pos.x`, then y, then z (each `fld dt'; fmul vel.c; fadd pos.c; fstp`).
  3. Local MovementInfo_t mi (0x2C): mi.pos = prt->m_Pos, mi.vel = prt->m_Velocity (rot/flags left to the callee).
     `ProcessMovementInfo(system->m_fCurrentTime, life / prt->m_fTotalLife (life from the x87 register), dt', system->m_SystemBP->m_fLength,
     false, &mi)` (0x4A228C). Copy back `prt->m_Pos = mi.pos`, `prt->m_Velocity = mi.vel`.
  4. Water (0x4A22B8): if `mi.m_bHasFloatInfo` (+0x28) or `mi.m_bHasUnderwaterInfo` (+0x29):
     `found = GetWaterLevel(pos.x, pos.y, pos.z, &w, true, nullptr)`.
     - if float && found && `pos.z < w` (strict, `test ah,5; jp`): `pos.z = w`.
     - if underwater: if `!found` return 1; if `pos.z > w` (strict; NaN false) return 1.
  5. Rotation speed (0x4A2328), with `r0..r3 = mi.m_Rot[0..3]` and `k = 1/255`, `b31 = prt->m_MultRot`, `b32 = prt byte +0x32`:
     `cw = r0 > 0 || r1 > 0`, `ccw = r2 > 0 || r3 > 0` (each test strict, NaN false, evaluated in this order; the code re-tests
     r0, r1 after a false `ccw`).
     - cw && ccw: if `b32 < 0x80`: `rot += ((((float)b32 * 0.0078125f [0x858B88]) * (r1 - r0) + r0) * b31 (fimul)) * dt' * k`
       else: `rot -= (((((float)b32 - 128.0f [0x858BF4]) * 0.0078125f) * (r3 - r2) + r2) * b31) * dt' * k` (`fsubr`: rot - value).
     - cw only: `rot += ((((float)b32 * k) * (r1 - r0) + r0) * b31) * dt' * k`.
     - ccw only: `rot -= ((((float)b32 * k) * (r3 - r2) + r2) * b31) * dt' * k`.
     - neither: no change. (rot = prt->m_CurrentRotation +0x38, read-modify-write f32; x87 order exactly: `fild b32; fmul const;
       fld rB; fsub rA; fmulp; fadd rA; fimul b31; fmul dt'; fmul k; fadd/fsubr rot`.)
     Return 0.
- gta-reversed: `plugin::CallMethodAndReturn` only. Open questions: none.

## 0x4A24D0 FxEmitterBP_c::IsFxInfoPresent  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall`, arg `int type` [esp+4] (compared as a full dword against the zero-extended u16 m_nType), `ret 4`, returns AL.
- Caller: FxSystemBP_c::Load 0x5C05F0 (2 sites).
- Behaviour: `n = m_FxInfoManager.m_nNumInfos` (+0x2C); if `n <= 0` (signed) return 0; for i = 0..n-1: if
  `(u16)m_pInfos[i]->m_nType == type` return 1. Return 0.
- gta-reversed: same logic (FxEmitterBP.cpp, hooked). Open questions: none.

## 0x4A2510 FxEmitterBP_c::FreePrtFromPrim  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall`, arg `FxSystem_c* system` [esp+4], `ret 4`, returns AL.
- Callers: vtable slot only. Callees: TList_c::RemoveItem 0x4A8E30, FxManager_c::ReturnParticle 0x4A93B0 (ECX = g_fxMan; it is
  `ecx += 0x1C; jmp 0x4A8DF0`, i.e. adds the particle to g_fxMan's free list).
- Behaviour: walk `m_Particles` from head (+0x20) via `next` (+4); at the FIRST particle with `prt->m_System (+0x28) == system`:
  `RemoveItem(prt)`, `ReturnParticle(prt)`, return 1. If none: return 0. Only one particle is freed per call.
- gta-reversed: `plugin::CallMethodAndReturn`. Open questions: none.

## 0x4A2B40 FxEmitterBP_c::CreateInstance  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall`, no args, plain `ret`; returns EAX = new FxEmitter_c* or null. SEH frame (handler 0x838E9B, unwind frees).
- Behaviour: `p = operator new(0x14)` 0x82119A; if null return null; `FxPrim_c::FxPrim_c(p)` 0x4A9F20 (sets [p] = 0x85A9E0,
  [p+4] = 0, [p+8] = 0, byte [p+0xC] = 1); `[p] = 0x85A7A4` (FxEmitter_c vtable). `p+0x10` (m_fEmissionIntensity) is NOT initialised.
- gta-reversed: `return new FxEmitter_c();` with a defaulted, non-user-provided ctor: value-initialisation zero-fills +0x10 first,
  so gr writes 0 where the original leaves heap garbage (harmless if FxEmitter_c::Init writes it before use; not verified here).
  gr also uses its own allocator. Open questions: none.

## 0x4A2BC0 FxEmitterBP_c::Update  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall`, arg `float deltaTime` [esp+4], `ret 4`. (EAX on return = the item count read at entry; callers ignore it.)
- Callees: GetNumItems 0x4A8EC0, UpdateParticle 0x4A21D0, RemoveItem 0x4A8E30, ReturnParticle 0x4A93B0.
- Behaviour: if `GetNumItems() == 0` return. `prt = head`; while prt:
  `sys = prt->m_System`; if `sys->m_nKillStatus (+0x51) == 3` set it to 2 (0x4A2BEF);
  if `sys->m_nPlayStatus (+0x50) != 2` and `UpdateParticle(deltaTime, prt)`: `next = prt->next` (saved BEFORE removal, 0x4A2C0D);
  `RemoveItem(prt)`; `g_fxMan.ReturnParticle(prt)`; `prt = next`. Else `prt = prt->next`.
- gta-reversed: provided but unhooked (FxEmitterBP.cpp:59, row "wrong casts or smth"): the loop advances with
  `it = m_Particles.GetNext(it)` AFTER `RemoveItem(it)` + `ReturnParticle(it)`, i.e. it reads the next pointer of an item that has
  just been returned to the free list (ReturnParticle relinks it), so gr can skip or wander into the free list. The original saves
  `next` first. Port the bytes. Open questions: none.

## 0x4A2C40 FxEmitterBP_c::Render  (catalogue: specified/REA, port level 8)
- Convention: `__thiscall`, args `RwCamera* camera` [esp+4], `uint32 txdHashKey` [esp+8], `float brightness` [esp+0xC],
  `bool doHeatHaze` [esp+0x10], `ret 0x10`. Frame `sub esp,0x1E8`. The brightness arg slot is overwritten (see step 5j).
- Callers: vtable slot. Callees: as RenderHeatHaze plus RenderHeatHaze 0x4A1940, FxSystem_c::AddParticle (pos variant) 0x4AA440,
  FxPrtMult_c ctor 0x4AB270, RwV3dLength 0x7EDAC0, `_rand` 0x821B1E, `_ftol` 0x821B40.
- Globals: `RwEngineInstance` 0xC97B24; blend table 0x8A6230 (11 dwords 1..11: rwBLENDZERO..rwBLENDSRCALPHASAT); g_fx.m_SmokeHuge
  (dword at 0xA9AE20 = g_fx+0x20); `g_fxMan + 0xB8` byte 0xA9AF38 (gr m_bHeatHazeEnabled; written 1); `CTimer::ms_fTimeStep` 0xB7CB5C;
  CVector at 0xB6FA14/18/1C (= TheCamera 0xB6F028 + 0x9EC; see open questions).
- Behaviour:
  1. `hh = IsFxInfoPresent(0x4008)` inlined (0x4A2C4A-0x4A2CB5, same loop as 0x4A24D0).
  2. If `doHeatHaze`: if `m_FxInfoManager.m_bHasHeatHazeParticleEmitter` (+0x3D) call `RenderHeatHaze(camera, txdHashKey, brightness)`;
     return (0x4A2C7B-0x4A2CB0).
  3. If `hh`: if `GetNumItems() > 0` (signed) set byte 0xA9AF38 = 1; return.
  4. If `GetNumItems() == 0` return; `curRaster = m_apTextures[0]->raster`; if `GetNumItems() == 0` return.
     `RwRenderStateSet(12, (u8)m_bAlphaOn)`; if alphaOn: `RwRenderStateSet(10, table[(i8)m_nSrcBlendId])`,
     `RwRenderStateSet(11, table[(i8)m_nDstBlendId])` (no bounds check); else `(10, table[1] = 2 ONE)`, `(11, table[0] = 1 ZERO)`.
     `RenderBegin(curRaster, nullptr, 1)`. CM from camera. If the list head is null jump to RenderEnd (0x4A3E87).
  5. Per particle (head, `next` +4):
     a. S1 -> P; S2 -> ri.
     b. Smoke (0x4A2E36): if `(i8)ri.m_SmokeType > -1`: `nb = -ri.m_SmokeBrightness` (f32);
        `rnd = ((((float)rand() * 3.0518509e-05f [0x858C7C]) * (bright - nb)) + nb) * (1/255)` (stored f32);
        FxPrtMult_c m (ctor 0x4AB270 on the local); `m.r = ri.m_SmokeColor.r * (1/255) + rnd` then clamp: if `1.0f < v` -> 1.0f
        (0x858624), else store v and if `0.0f > v` -> 0.0f; same for g, b; `m.a = ri.m_SmokeColor.a * (1/255)` (no rnd, no clamp);
        `m.size (+0x10) = ri.m_SmokeSize`, `m.rot (+0x14) = 1.0f`, `m.life (+0x18) = ri.m_SmokeLife`.
        `g_fx.m_SmokeHuge->AddParticle(&p, &vel, 0.0f, &m, -1.0f, 1.2f, 0.6f, 0)` 0x4AA440 with `p = prt->m_Pos` (the
        untransformed particle position, not P) and `vel = (0,0,0)`.
     c. S3 -> M.
     d. Orientation (0x4A308C), `tm = ri.m_nTrailScreenMode` (signed):
        - tm > 0 (trail): `p = prt->m_Pos` (untransformed).
          - tm == 2: `d = p - CM.pos` (x87, d.z stored f32, d.x/d.y kept in registers); `s = ms_fTimeStep * 0.02f [0x858B38]`;
            `q = p - vel*s` per component (vel.y*s and vel.z*s stored f32; vel.x*s kept); `O = (d - (q - G)) * ri.m_fTrailTime`
            with G = CVector 0xB6FA14 (per component `(q.c - G.c)` stored f32 for x and z, kept for y; then `d.c - that`; then
            `* trailTime`, stored f32).
          - tm != 2: `A = p - trailTime * vel` (`fld tt; fmul vel.c; fsubr p.c`, A.z stored f32), `O = p - A` per component.
          `D = O`; if `D.x == 0 && D.y == 0 && D.z == 0` exactly (`test ah,0x44; jp`; NaN counts as non-zero) `D.z = 1.0f` (no
          normalise) else `RwV3dNormalize(&D, &D)`. `V = p - CM.pos`, normalised. R = D x V and U = D x R with the same component
          formulas as RenderHeatHaze 3b; `M->right = R, M->up = D, M->at = U`.
        - else if `ri.m_bHasDir`: if useVel: `D = prt->m_Velocity`, if exactly zero `D.z = 1.0f`; else `D = ri.m_Direction`;
          `RwV3dNormalize(&D, &D)` always; `V = P - CM.pos` (transformed P), normalised; R, U, M as above.
        - else if `ri.m_bIsFlat`: `*M = ri.m_FlatMatrix` (64 bytes), `U = flat.at`; else `*M = *CM`, `U = CM.at`.
     e. S4 rotation -> Rr, Up. f. S5 size.
     g. Colour (0x4A37C0), floats: `r = (float)c1.r`, `g = (float)c1.g`, `b = (float)c1.b` (kept in st0), `a = (float)c1.a`.
        - colorType == 1 (RANGE): `r = (c2.r - r) * (b32 * k) + r`; `g = (c2.g - g) * (b33 * k) + g`; `b = b + (c2.b - b) * (b34 * k)`
          (k = 1/255; bytes +0x32/+0x33/+0x34).
        - colorType == 2 (BRIGHT): `x = ((b32 * 0.0078125f) - 1.0f) * (int)c2.a` (fimul); `r += x; g += x; b += x`; then each of r, g, b:
          if `0.0f > v` -> 0.0f; then each: if `255.0f < v` -> 255.0f [0x859AAC] (all lower clamps first, then the upper ones).
        - Multiply by m_MultColor: for r (+0x2C), g (+0x2D), b (+0x2E), a (+0x2F): if byte < 0xFF: `v = (byte * k) * v`.
     h. If `ri.m_bSelfLit` (+0x37): `c1.r = (u8)_ftol(r)`, `c1.g = (u8)_ftol(g)`, `c1.b = (u8)_ftol(b)`.
        Else: if `prt->m_Brightness` (+0x35) <= 100: `brightness = (float)b35 * 0.01f [0x858C58]` written INTO THE ARG SLOT
        [esp+0x204] (persists for the following particles, original quirk); `c1.r = _ftol(r * brightness)`, g, b likewise.
        Both: `c1.a = _ftol(a)`. (`_ftol` truncates toward zero; low byte stored.)
     i. Raster selection as RenderHeatHaze 3f, except: id == 4 with null texture falls back to `m_apTextures[0]->raster`
        (0x4A3B14 -> 0x4A3AB8, same as the others); the "other id" case keeps the previous particle's raster (local [esp+0x38],
        uninitialised for the first particle).
     j. If `tm > 0`: `ri.top = RwV3dLength(&O)`, `ri.bottom = 0` (0x4A3B5E-0x4A3B72).
     k. S6 quad with `ri.m_Color1` bytes as colour and P (transformed) as the centre.
  6. `RenderEnd()`; return.
- Edge cases: the trail branch builds the orientation from the untransformed `prt->m_Pos` but the quad is centred on the transformed P
  (differs for m_bLocalToSystem particles). colorType values other than 1/2 leave the colours unchanged.
- gta-reversed: `plugin::CallMethod` plus a commented-out partial draft (FxEmitterBP.cpp) that omits the trail/dir/rotation/colour logic;
  the draft also tests `m_FxInfoManager.m_bHasHeatHazeParticleEmitter` like the bytes. Port the bytes.
- Open questions: 0x4A318E/0x4A319B/0x4A31A8: the global CVector at 0xB6FA14 (TheCamera + 0x9EC) is not named here; by use it is a
  previous-frame camera position (gr name not resolved from the headers in this pass).

## 0x5C25F0 FxEmitterBP_c::Load  (catalogue: specified/REA, port level 5)
- Convention: `__thiscall`, args `FILESTREAM file` [esp+4], `int version` [esp+8], `FxName32_t* textureNames` [esp+0xC], `ret 0xC`;
  returns AL = 1 always.
- Callees: FxPrimBP_c::Load 0x5C2010 (thiscall (file, version, textureNames); result ignored), CFileMgr::ReadLine via thunk 0x4AB250 ->
  0x5389B0 (cdecl (file, buf, 256) = fgets), `sscanf` 0x8220AD with format 0x86AA68 = "%s %f", `_ftol` 0x821B40.
- Behaviour: `FxPrimBP_c::Load(file, version, textureNames)`; `ReadLine(file, line, 0x100)`; `sscanf(line, "%s %f", name, &f)`;
  `word [this+0x38] = (u16)_ftol(f * 64.0f [0x859A44])`; again for the second line -> `word [this+0x3A]`. The sscanf result is not
  checked (an unparsable line leaves `f` as stale stack data).
- gta-reversed: unhooked (`.Reversed = false`) code writes `this->m_nLodStart` / `m_nLodEnd`, which in gr's layout are the
  FxEmitterBP_c members at +0x40/+0x42, while the original writes +0x38/+0x3A = `m_FxInfoManager.m_nLodStart/m_nLodEnd`
  (the fields FxEmitter.cpp:127 reads). gr logic error (would leave the LOD distances 0 if hooked); port the original offsets.
  gr's ReadField also checks the field name; the original ignores the name. Open questions: none.
