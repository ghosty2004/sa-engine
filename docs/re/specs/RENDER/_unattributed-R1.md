# RENDER unattributed functions (group R1)

Unnamed or helper functions of the RENDER subsystem; each heading gives a proposed Class::Method with the evidence.

## 0x407000 CEntity::sub_407000 (proposed: CEntity::IsInCurrentArea)  (catalogue: unknown/REA, port level 0)
- Name evidence: same test as gr `CEntity::IsInCurrentArea()` (Entity.h:428, inline in gr): `area == CGame::currArea || area == AREA_CODE_13`;
  only caller CRenderer::SetupEntityVisibility 0x554230 (ECX = entity).
- Convention: `__thiscall bool (CEntity* this = ECX)`, result in AL, plain `ret`. Entry 0x407000 is `jmp 0x1562960` (.HOODLUM
  body, ordinary code, no decryption). Clobbers EAX, ECX, EDX.
- Globals: `CGame::currArea` 0xB72914 (u32, read at 0x1562963).
- Fields: `this+0x2F` u8 m_nAreaCode (gr `m_nAreaCode`).
- Behaviour: `a = (u32)(u8)this->m_nAreaCode` (zero-extended, 0x1562960/0x1562969); if `a == currArea` (full 32-bit compare,
  0x156296C) return 1; else if `(u8)a == 13` (0x1562970, AREA_CODE_13 "all areas") return 1; else return 0 (0x1562974).
- Edge: currArea is compared as a full dword, so a currArea value >= 256 never matches an entity area (only 13 does).
- gta-reversed: inline `CEntity::IsInCurrentArea` matches the bytes. Open questions: none.

## 0x4A1900 FxEmitterBP_c::sub_4A1900 (proposed: FxEmitterBP_c::operator new)  (catalogue: unknown/REA, port level 1)
- Name evidence: only caller FxSystemBP_c::Load 0x5C05F0 does `push 0x40; call 0x4A1900` (0x5C089F) and passes the result as
  ECX to FxEmitterBP_c::Constructor 0x4A18D0 (0x5C08BE) when non-null - the class `operator new(size)` of FxEmitterBP_c (same
  shape as FxEmitterPrt_c::operator new[] 0x4A18B0 and FxSystemBP_c::operator new 0x4AA100 in gr).
- Convention: `__cdecl void* (uint32 size [esp+4])`, plain `ret`, caller pops (0x5C08A6 `add esp,4`). Returns EAX.
- Callees: FxMemoryPool_c::GetMem 0x4A9CA0 (`__thiscall (int32 size, int32 align)`, `ret 8`).
- Globals: the pool `g_fxMan.m_Pool` at 0xA9AF2C (g_fxMan + 0xAC), passed as ECX (0x4A1907).
- Behaviour: `return g_fxMan.m_Pool.GetMem(size, 4)` (push 4 at 0x4A1904, push size at 0x4A1906). No null check, no ctor call.
- Finding: the single call site allocates 0x40 bytes for an FxEmitterBP_c (0x5C089F), while gr declares
  `VALIDATE_SIZE(FxEmitterBP_c, 0x48)` (FxEmitterBP.h:36, fields m_nLodStart.. after FxPrimBP_c's 0x40). Since the compiler
  passes sizeof, the original class is 0x40 bytes; gr's layout of the extra fields is not confirmed by this function.
- gta-reversed: absent (gr `new FxEmitterBP_c()` at FxSystemBP.cpp:102 uses the global heap, not the Fx pool - a deviation the
  FxSystemBP_c::Load port must not copy). Open question: where the original keeps FxEmitterBP_c's LOD fields (0x5C089F size 0x40).

## 0x4A5270 FxInfoNoise_c::sub_4A5270 (proposed: FxInfoNoise_c::FxInfoNoise_c)  (catalogue: unknown/REA, port level 1)
- Name evidence: only caller FxInfoManager_c::AddFxInfo 0x4A7B00 allocates `GetMem(0x18, ..)` from the Fx pool (0x4A7DAB) and calls
  it on the block (0x4A7DCD); it stores type 0x2001 = FX_INFO_NOISE_DATA (eFxInfoType.h:22) and allocates one interp key list,
  matching gr `FxInfoNoise_c::FxInfoNoise_c()` (FxInfoNoise.cpp:5: `m_nType = FX_INFO_NOISE_DATA; m_InterpInfo.Allocate(1)`).
- Convention: `__thiscall FxInfoNoise_c* (this = ECX)`, returns `this` in EAX, plain `ret`. Has an SEH frame (handler 0x838FC3,
  state 0 set at 0x4A5290 for the FxInterpInfo32_c member; unwind funclets call FxInfo_c::~FxInfo_c 0x4A4940 and
  FxInterpInfo32_c::~FxInterpInfo32_c 0x4A89B0 - only reached if GetMem threw, which it cannot).
- Callees: FxInterpInfo32_c::FxInterpInfo32_c 0x4A8990 (on this+8), FxMemoryPool_c::GetMem 0x4A9CA0 (ECX = 0xA9AF2C, pool of g_fxMan).
- Fields written, in order: `+0` vtable = 0x85A848 (FxInfoNoise_c vtable; the FxInfo_c base ctor is inlined: no base-vtable store,
  0x4A5298); member ctor on `+8` (writes +8 vtbl 0x85A980, byte +0xC = 0 m_bLooped, dword +0x10 = 0 m_pTimes, dword +0x14 = 0 m_Keys);
  `+4` u16 = 0x2001 (m_nType, `mov word` at 0x4A52B2); `+0xE` u8 = 1 (interp m_nCount, BYTE store at 0x4A52B8); `+0x14` =
  `GetMem(4, 4)` (m_Keys, room for 1 pointer; 0x4A52A5..0x4A52C4).
- Not written: `+6` (gr m_bTimeModeParticle), `+0xD` (m_nNumKeys), `+0xF` (high byte of gr's int16 m_nCount). These keep the pool
  memory contents; Load fills them later.
- gta-reversed: gr's ctor is equivalent except that gr's `m_nCount` is an int16 (FxInterpInfo.h) and Allocate writes all 16 bits;
  the original writes only the low byte (0x4A52B8). The pool memory is not zeroed by GetMem, so a port must store exactly one byte.
  Open questions: none.

## 0x4AB240 FxManager_c::sub_4AB240 (proposed: Fx file-open thunk to CFileMgr::OpenFile)  (catalogue: unknown/REA, port level 1)
- Single instruction `jmp 0x538900` (CFileMgr::OpenFile, itself `jmp 0x8232D8` = CRT fopen). Convention therefore that of
  OpenFile: `__cdecl FILE* (const char* path [esp+4], const char* mode [esp+8])`, caller pops.
- Caller: FxManager_c::LoadFxProject 0x5C2420 (`push 0x85A53C ("r"); push esi(path); call 0x4AB240` at 0x5C24AB..0x5C24B1).
- gta-reversed: calls `CFileMgr::OpenFile(path, "r")` directly (FxManager.cpp:142), same behaviour. Open questions: none.

## 0x4AB260 FxManager_c::sub_4AB260 (proposed: Fx file-close thunk to CFileMgr::CloseFile)  (catalogue: unknown/REA, port level 1)
- Single instruction `jmp 0x5389D0` (CFileMgr::CloseFile). `__cdecl int (FILE* file [esp+4])`, caller pops (0x5C25C5 `add esp,4`).
- Caller: FxManager_c::LoadFxProject 0x5C2420 (0x5C25C0).
- gta-reversed: calls `CFileMgr::CloseFile(file)` directly (FxManager.cpp:160). Open questions: none.

## 0x54EEF0 TransformPoints  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl void (RwV3d* out [esp+4], int32 count [esp+8], const CMatrix& m [esp+0xC], const RwV3d* in [esp+0x10])`,
  plain `ret`. (Stack offsets: `[esp+0xC]` read before any push at 0x54EEF5; `[esp+0x10]` after one push = count at 0x54EF0C;
  out/in read after 3 pushes at 0x54EF18 `[esp+0xC]` and 0x54EF1D `[esp+0x1C]`.)
- Callers (4): CPlantMgr::_ProcessEntryCollisionDataSections_AddLocTris 0x5DC8B0, CPlantMgr::_ProcessEntryCollisionDataSections
  0x5DCD80, CStencilShadows::RenderForVehicle 0x70FAE0, CStencilShadows::RenderForObject 0x710310.
- Callees: CMatrix::CopyToRwMatrix 0x59B8B0 (thiscall, ret 4), RwMatrixUpdate 0x7F18A0 (cdecl), RwV3dTransformPoint 0x7EDD60
  (cdecl (RwV3d* out, const RwV3d* in, const RwMatrix* m)).
- Globals: `CGame::m_pWorkingMatrix1` 0xB72920 (RwMatrix*), read three times (0x54EEF0, 0x54EF00, and every loop pass 0x54EF21).
- Behaviour: `m.CopyToRwMatrix(m_pWorkingMatrix1)` (0x54EEFB); `RwMatrixUpdate(m_pWorkingMatrix1)` (0x54EF07); if `count != 0`
  (0x54EF13, `test ebx,ebx`): loop `do { RwV3dTransformPoint(out, in, m_pWorkingMatrix1); out++; in++; } while (--count != 0)`
  (0x54EF21..0x54EF39, stride 12 bytes).
- Edge: count is tested only for zero; a negative count loops ~4G times (gr's `for (i < count)` does nothing). Callers pass 2 or 3.
  In-place use (out == in) is fine because points are transformed one at a time.
- gta-reversed: common.cpp:64, same behaviour for count >= 0. Open questions: none.

## 0x5A2E60 CMatrix::sub_5A2E60 (proposed: CMatrix::ScaleXYZ, column-wise)  (catalogue: unknown/REA, port level 0)
- Name evidence: callers are the two places where gr uses its NOTSA `CMatrix::ScaleXYZ(x, y, z)`: C3dMarker::Render 0x7223D0
  (`push 20.0f, 1.0f, 1.0f` = 0x41A00000/0x3F800000/0x3F800000 at 0x7225AA..0x7225B4, ECX = local CMatrix) and
  ProcSurfaceInfo_c::AddObject 0x5A32D0 (`push z, xy, xy` at 0x5A37E9..0x5A37EB). ECX is a CMatrix (rows right +0, forward +0x10,
  up +0x20, 16-byte stride).
- Convention: `__thiscall void (CMatrix* this = ECX, float x [esp+4], float y [esp+8], float z [esp+0xC])`, `ret 0xC`.
- Behaviour: for each of the three rows r = right, forward, up (in that order; ECX counter 3 at 0x5A2E63, pointer step 0x10 at
  0x5A2E6C): `r.x = x * r.x; r.y = y * r.y; r.z = z * r.z` (each `fld arg; fmul [row]; fstp [row]`, f32 store, 0x5A2E68..0x5A2E87).
  The position row (+0x30) is untouched.
- I.e. it scales the x/y/z COMPONENTS of every axis vector (a world-axis scale, M' = M * diag(x,y,z) in row-vector convention),
  not each axis vector by its own factor.
- gta-reversed LOGIC ERROR: gr `CMatrix::ScaleXYZ` (Matrix.cpp:336) does `m_right *= x; m_forward *= y; m_up *= z` (row-wise).
  The two agree only for x == y == z, or for matrices whose rows have the special zero pattern (e.g. a pure Z-rotation with
  (x,y,z) = (s,s,z')). ProcSurfaceInfo_c::AddObject applies it to an entity-derived matrix with xy != z, so gr's placement of
  procedural objects differs from the original. Also gr's 3dMarker.cpp:112-116 replaces the original sequence `CMatrix::Scale(m_fSize)` (0x72259E, always) followed by this
  function with (1, 1, 20) when `marker+0x50 (m_nType) == 2` (MARKER3D_TUBE, 0x7225A3) by a single row-wise `ScaleXYZ(size, size,
  size*20)` for tubes and `Scale(size)` otherwise; equal results only when the attach matrix is a pure Z-rotation. Port keeps the original column-wise scaling. Open questions: none.

## 0x5DD1E0 GeometrySetPrelitConstantColor  (catalogue: specified/REA, port level 8)
- Convention (non-standard): `RpGeometry* geometry` in ESI (register arg), `RwRGBA color` packed as a dword at [esp+4]; returns
  bool in AL; plain `ret` (caller pops). Preserves ESI/EDI; clobbers EAX, ECX, EDX. gr notes "retarded calling conv" (PlantMgr.cpp:16).
- Caller: CPlantMgr::LoadModels 0x5DD220 (ESI = atomic->geometry loaded at 0x5DD2BA; color dword from a CRGBA(255,255,255,255)
  temporary via 0x7170C0, pushed at 0x5DD2FC).
- Callees: RpGeometryLock 0x74C7D0 (cdecl (geo, 0xFFF = rpGEOMETRYLOCKALL)), RpGeometryUnlock 0x74C800 (cdecl).
- Fields: RpGeometry +0x08 flags (bit 0x8 = rpGEOMETRYPRELIT), +0x14 numVertices (i32), +0x30 preLitLum (RwRGBA*).
- Behaviour: if `!(geo->flags & 8)` return 0 (0x5DD1E0..0x5DD1E8, no lock). Else `RpGeometryLock(geo, 0xFFF)` (0x5DD1F0);
  `p = geo->preLitLum` (read after the lock, 0x5DD1F5); if `p != NULL && numVertices > 0` (signed, 0x5DD204 `jle`) fill
  `numVertices` dwords with `color` (`rep stosd`, 0x5DD20A); `RpGeometryUnlock(geo)` (0x5DD20D); return 1.
- gta-reversed: PlantMgr.cpp:17 static, same behaviour (not hooked). Open questions: none.

## 0x6E5750 CWaterLevel::sub_6E5750 (proposed: CWaterLevel::AddToQuadsAndTrianglesList)  (catalogue: unknown/REA, port level 0)
- Name evidence: gr lists `AddToQuadsAndTrianglesList(int32, int32, int32, uint32)` as missing (WaterLevel.h:323); the only
  caller CWaterLevel::FillQuadsAndTrianglesList 0x6E7B30 calls it as `(blockX, blockY, quadIdx, 1)` (0x6E7C0D push 1) and
  `(blockX, blockY, triIdx, 2)` (0x6E7CE8 push 2); it writes `m_BlockPolyInfo` 0xC21B70 and `m_PolyCombos` 0xC215F8
  (gr names for `m_QuadsAndTrianglesInEachBlock` / `m_QuadsAndTrianglesList`).
- Convention: `__cdecl void (int32 x [esp+4], int32 y [+8], int32 id [+0xC], uint32 type [+0x10])`, plain `ret`.
  No callees.
- Globals: `m_BlockPolyInfo` 0xC21B70 (u16[12][12], read/written), `NumWaterZonePolys` 0xC215F0 (u32, gr name; it is
  the used length of `m_PolyCombos`), `m_PolyCombos` 0xC215F8 (u16[700]; the code also addresses it as 0xC215F6 =
  element -1, 0xC215FA = +1, 0xC215FC = +2).
- Entry format (u16): bits 0-13 id, bits 14-15 type (0 none, 1 quad, 2 triangle, 3 combo = id is first index into
  `m_PolyCombos`, list terminated by a 0 entry). New entries are built as `(u16)((type << 14) | id)` (0x6E577A-0x6E577D,
  0x6E57A1, 0x6E57D5); `id` is NOT masked to 14 bits (an id >= 0x4000 would corrupt the type bits).
- Behaviour:
  1. `slot = &m_BlockPolyInfo[x*12 + y]` (0x6E5758 `eax = x*3`, 0x6E575B `ecx = y + eax*4`; word index), `old = *slot`,
     `t = old >> 14` (0x6E5769).
  2. `t == 0` (0x6E5770): `*slot = (type << 14) | id`; return.
  3. `t == 3` (neither 2 nor 1, 0x6E5788/0x6E578E): append to the LAST combo list in the array, not to this block's list:
     `n = NumWaterZonePolys; m_PolyCombos[n-1] = new` (0x6E57A6 writes 0xC215F6 + 2n, overwriting the terminator),
     `m_PolyCombos[n] = 0` (0x6E57AE), `NumWaterZonePolys = n + 1` (0x6E57B9). `*slot` unchanged.
  4. `t == 1 or 2`: convert into a combo: `n = NumWaterZonePolys; m_PolyCombos[n] = old` (0x6E57C4),
     `m_PolyCombos[n+1] = new` (0x6E57DA), `m_PolyCombos[n+2] = 0` (0x6E57E4), `NumWaterZonePolys = n + 3` (0x6E57F7),
     `*slot = (u16)n | 0xC000` (0x6E57EE/0x6E57FD; n not masked).
- Edge cases / original quirks: step 3 is only correct when the block being extended owns the most recently created
  combo (true for the caller's block-by-block fill order); otherwise it silently extends another block's list. No bounds
  check on `NumWaterZonePolys` against 700.
- gta-reversed: absent (only the "Missing" comment). Open questions: none.

## 0x6E5BB0 CWaterLevel::sub_6E5BB0 (proposed: CWaterLevel::TestQuadToGetWaterLevel)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller CWaterLevel::GetWaterLevelNoWaves 0x6E8580 (2 sites, 0x6E863A / 0x6E870E) passes
  `&WaterQuads[id & 0x3FFF]` (0x6E8632 `lea ecx, [eax*2 + 0xC21C90]`, stride 10) for a type-1 entry; the triangle twin is
  0x6E5E90. The name is the one used by other-platform symbol sets for this pair (not provable from these bytes).
- Convention: `__cdecl bool (CWaterQuad* quad [esp+4], float x [+8], float y [+0xC], float z [+0x10], float* outLevel
  [+0x14], float* outBigWaves [+0x18] (nullable), float* outSmallWaves [+0x1C])`, result in AL, plain `ret`.
  The `quad` argument slot is reused as an f32 scratch (0x6E5BD3).
- Layouts: CWaterQuad (0xA): `u16 verts[4]` +0 (read with `movsx`, i.e. as signed i16), flags byte +8 (bit 2 = 0x4
  bLimitedDepth). `m_aVertices` 0xC22910, CWaterVertex 0x14: +0 i16 x, +2 i16 y, +4 f32 z, +8 f32 bigWaves, +0xC f32
  smallWaves (addressed as 0xC22910/0xC22912/0xC22914/0xC22918/0xC2291C + 20*idx). V0..V3 = m_aVertices[verts[0..3]].
- Behaviour (all compares via `fcomp` + `fnstsw`):
  1. `X0 = (float)V0.x` (fild, stored f32 0x6E5BD3). If `x < X0` -> false (0x6E5BDB, `test ah,1`; NaN x -> false).
  2. If `(float)V1.x < x` -> false (0x6E5BFF). `Y0 = (float)V0.y` (f32, 0x6E5C21). If `y < Y0` -> false (0x6E5C25).
     If `(float)V2.y < y` -> false (0x6E5C4D). (V0 = min corner, V1 gives max x, V2 gives max y; boundaries inclusive.)
  3. `tx = (x - X0) / (int)(V1.x - V0.x)` (x87 `fsub` then `fidiv` of the int difference, stored f32 0x6E5C80);
     `ty = (y - Y0) / (int)(V2.y - V0.y)` (0x6E5C84-0x6E5C8C, kept in ST0, not rounded to f32).
  4. If NOT `tx + ty > 1.0` (`fld tx; fadd st1; fcomp [0x858624] = 1.0`, `test ah,0x41; jp` 0x6E5CA1; i.e. sum <= 1)
     -> first triangle V0,V1,V2:
     `*outLevel = ((V2.z - V0.z) * ty + (V1.z - V0.z) * tx) + V0.z` (0x6E5CA7-0x6E5CD7; ty from ST, tx from the f32 slot;
     `faddp` adds the tx term to the ty term).
     If `outBigWaves == null` -> return TRUE immediately (0x6E5CD9 -> 0x6E5E7A), skipping step 6.
     `*outBigWaves = ((V2.big - V0.big) * ty + (V1.big - V0.big) * tx) + V0.big` (0x6E5CEC-0x6E5D1E);
     `*outSmallWaves` likewise with +0xC (0x6E5D2D-0x6E5D63; no null check on this pointer). Go to step 6.
  5. Else (sum > 1.0 or NaN) -> second triangle V3,V1,V2 (0x6E5D6A): `a = 1.0 - ty` (`fsubr`), `b = 1.0 - tx` (computed from
     the f32 tx); `*outLevel = ((V2.z - V3.z) * b + (V1.z - V3.z) * a) + V3.z` (0x6E5D81-0x6E5DB2).
     If `outBigWaves == null` -> return TRUE (0x6E5DB4 -> 0x6E5E78). Big: `((V1.big - V3.big) * a + (V2.big - V3.big) * b)
     + V3.big` (0x6E5DC8-0x6E5DF8); small: same order with +0xC (0x6E5E08-0x6E5E3C). Note the z term order (V2 first) differs
     from the big/small order (V1 first); result identical only up to addition commutativity (exact in IEEE).
  6. Depth tests on the stored f32 `*outLevel` (0x6E5E42): if `level - 0x858B44 (6.0) > z` (strict; `test ah,0x41; jne`
     skips on <=, == or NaN) AND `quad->flags & 4` (0x6E5E55) -> false. Then if `level + 0x858BA4 (20.0) < z` (strict;
     `test ah,5; jp` 0x6E5E6C takes TRUE on >= or NaN) -> false; else TRUE.
- Edge cases: zero-width quad (`V1.x == V0.x` or `V2.y == V0.y`) divides by integer 0 in x87 (`fidiv`) -> inf/NaN, NaN goes
  to the second triangle. The level/waves are written even when the function then returns false (step 6).
  Inconsistency with the triangle twin 0x6E5E90: here a null `outBigWaves` returns true WITHOUT the depth tests.
- gta-reversed: absent (no counterpart; gr's GetWaterLevelNoWaves is plugin::Call). Open questions: none.

## 0x6E5E90 CWaterLevel::sub_6E5E90 (proposed: CWaterLevel::TestTriangleToGetWaterLevel)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller CWaterLevel::GetWaterLevelNoWaves 0x6E8580 (2 sites, 0x6E8678 / 0x6E86E9) passes
  `&WaterTriangles[id & 0x3FFF]` (0x6E8670 `lea edx, [eax*8 + 0xC22854]`, stride 8); twin of 0x6E5BB0.
- Convention: `__cdecl bool (CWaterTriangle* tri [esp+4], float x [+8], float y [+0xC], float z [+0x10], float* outLevel
  [+0x14], float* outBigWaves [+0x18] (nullable), float* outSmallWaves [+0x1C])`, AL, plain `ret`. The `tri` slot and the
  `x` slot are reused as scratch (0x6E5F13, 0x6E5F6F).
- Layouts: CWaterTriangle (8): `u16 verts[3]` +0 (read `movsx`), flags byte +6 (bit 2 = 0x4 bLimitedDepth). Vertex
  layout as in 0x6E5BB0. V0, V1, V2 = m_aVertices[verts[0..2]]. The triangle is assumed axis-aligned: V0-V1 horizontal (x),
  and either V2 directly above/below V0 (same x) or above/below V1.
- Behaviour:
  1. `X0 = (float)V0.x` (f32, 0x6E5EB3). If `x < X0` -> false (0x6E5EBB). If `(float)V1.x < x` -> false (0x6E5EDF).
  2. `Y0 = (float)V0.y` stored f32 (0x6E5F21 / 0x6E5F33, both paths). Signed i16 compare `V0.y < V2.y` (0x6E5F0A):
     `lo = min(V0.y, V2.y)`, `hi = max(...)` (as floats, `hi` stored f32). If `y < lo` -> false (0x6E5F3B-0x6E5F48);
     if `y > hi` or NaN -> false (0x6E5F4E, `test ah,0x41; jp`).
  3. `tx = (x - X0) / (int)(V1.x - V0.x)` (fidiv, stored f32 0x6E5F8F); `ty = (y - Y0) / (int)(V2.y - V0.y)` (stored f32
     0x6E5F9F; may be negative-denominator, giving ty in [0,1] either way).
  4. If `V0.x == V2.x` (16-bit compare, 0x6E5F88; right angle at V0):
     reject unless `ty + tx <= 1.0` (`fld ty; fadd tx; fcomp [0x858624] = 1.0; test ah,0x41; jp` 0x6E5FBC; NaN rejects).
     `*outLevel = ((V2.z - V0.z) * ty + (V1.z - V0.z) * tx) + V0.z` (0x6E5FC2-0x6E5FF4).
     If `outBigWaves != null` (0x6E5FCC/0x6E5FF6): `*outBigWaves = ((V1.big - V0.big) * tx + (V2.big - V0.big) * ty) + V0.big`
     (0x6E6009-0x6E603D, note V1 term first here), `*outSmallWaves = ((V2.small - V0.small) * ty + (V1.small - V0.small) * tx)
     + V0.small` (0x6E604C-0x6E6084).
  5. Else (right angle at V1, 0x6E608B): reject if `tx < ty` (`fld tx; fcomp ty; test ah,1`; NaN rejects). `b = 1.0 - tx`
     (x87, kept on stack); `*outLevel = ((V2.z - V1.z) * ty + (V0.z - V1.z) * b) + V1.z` (0x6E60AC-0x6E60D8).
     If `outBigWaves != null`: big `((V0.big - V1.big) * b + (V2.big - V1.big) * ty) + V1.big` (0x6E60ED-0x6E611F); small
     `((V0.small - V1.small) * b + (V2.small - V1.small) * ty) + V1.small` (0x6E612E-0x6E6164).
  6. Depth tests (ALWAYS reached, also when `outBigWaves == null`, unlike 0x6E5BB0): if `level - 0x858B44 (6.0) > z` and
     `tri->flags & 4` (0x6E617B) -> false; if `level + 0x858BA4 (20.0) < z` -> false (`test ah,5; jnp` 0x6E6192); else TRUE.
- Edge cases: integer-zero denominators give inf/NaN (x87 fidiv). Outputs are written before the depth rejection.
- gta-reversed: absent. Open questions: none.

## 0x6E8550 CWaterLevel::sub_6E8550 (proposed: CWaterLevel::CalculateWavesOnlyForCoordinate2 argument-order thunk)  (catalogue: unknown/REA, port level 1)
- Convention: `__cdecl void (int32 x [esp+4], int32 y [+8], float* level [+0xC], float bigAmpl [+0x10],
  float smallAmpl [+0x14])`, plain `ret`. Pure forwarding thunk.
- Callers: CShadows::RenderStoredShadows 0x70A960 (4 sites, e.g. 0x70B0E4 pushes `small, big, &height, y, x`).
- Callees: CWaterLevel::CalculateWavesOnlyForCoordinate2 0x6E7210 (cdecl, real order `(x, y, bigAmpl, smallAmpl, float* level)`:
  0x6E7210 reads its pointer at [esp+0x18] after one push, i.e. the 5th argument).
- Behaviour: `0x6E7210(x, y, bigAmpl, smallAmpl, level)` (0x6E8550-0x6E8569: pushes arg3 (+0xC) first, then arg5,
  arg4, arg2, arg1), `add esp, 0x14`, return. No return value.
- gta-reversed: absent as a function; gr's declared signature of `CalculateWavesOnlyForCoordinate2(x, y, float*, big, small)`
  (WaterLevel.h:274) is exactly this thunk's order, and gr's `plugin::Call<0x6E7210>(x, y, big, small, ptr)` (WaterLevel.cpp:771)
  reorders correctly, so gr's Shadows.cpp:947 (`..._Direct`, calls 0x6E7210 directly) is behaviourally equivalent.
  Open questions: none.

## 0x6E8780 CWaterLevel::sub_6E8780 (proposed: CWaterLevel::RenderHighDetailWaterTriangle_OneLayer)  (catalogue: unknown/REA, port level 7)
- Name evidence: gr lists `RenderHighDetailWaterTriangle_OneLayer(int32, int32, CRenPar, int32, int32, CRenPar, int32,
  int32, CRenPar, int32, int32, int32, int32)` as missing (WaterLevel.h:339); the only caller
  CWaterLevel::RenderHighDetailWaterTriangle 0x6EDDC0 calls it twice with layer 0 (0x6EDEC1 `push 0`, call 0x6EDF43) and
  layer 1 (0x6EDF5D `push 1`, call 0x6EDFDF), mirroring the flat `RenderFlatWaterTriangle_OneLayer` 0x6E8ED0 pair.
- Convention: `__cdecl void`, plain `ret`, 0x58 bytes of arguments (caller cleans):
  `X1 [esp+4], Y1 [+8], CRenPar P1 [+0xC..+0x1B], X2 [+0x1C], Y2 [+0x20], P2 [+0x24..], X3 [+0x34], Y3 [+0x38],
  P3 [+0x3C..], int32 layer [+0x4C], int32 numTris [+0x50], int32 numVerts [+0x54], int32 N [+0x58]`.
  CRenPar (0x10, by value): +0 f32 z, +4 f32 bigWaves, +8 f32 smallWaves, +0xC flow bytes (unused here).
  `numTris` (caller passes N*N, 0x6EDE7E) and `numVerts` (caller passes 1+2+..+(N+1), 0x6EDE91 loop) are NEVER read.
  `N` = number of subdivisions (caller passes `(span) / 2`, 0x6EDE71-0x6EDE78). Frame: `sub esp,0x9C` + esi/edi, later
  ebx/ebp (0x6E8A1C); offsets below are given as in the instruction text.
- Callees: 0x6E7680 (catalogued as RenderBuffer::RenderIfDoesntFit, but the bytes take NO arguments and unconditionally
  flush: if `uiTempBufferVerticesStored` 0xC4B950 != 0 -> RwIm3DTransform(0xC4D958, n, 0, 1) and, on success,
  RwIm3DRenderIndexedPrimitive(3, 0xC4B958, uiTempBufferIndicesStored) + RwIm3DEnd; then both counters = 0. So it is
  "render and empty the buffer"), CWaterLevel::CalculateWavesOnlyForCoordinate 0x6E6EF0 (cdecl, 8 args), floor 0x8219F0
  (double on stack -> ST0, round-down), `_ftol` 0x821B40 (truncation).
- Globals read: N/A args aside: TheCamera 0xB6F028 (`m_matrix` at 0xB6F03C, placement pos 0xB6F02C), DETAILEDWATERDIST
  0x8D37D0 (int, 48), TextureShiftFirstV/U 0xC21178/0xC2117C, TextureShiftSecondV/U 0xC21180/0xC21184, WaterColor 0xC2116C
  (r,g,b bytes), WaterLayerAlpha[0..2] 0x8D3808 / 0x8D380C / 0x8D3810 (u32: 0xA0, 0x5A, 0xFF initially), f32 globals 0x8D3834
  (0x8D3834 = 0.1 initially), 0xC21170, 0xC21174 (layer-2 UV shift, only in the dead layer-2 path). Written: 0xC1F960 (u32 per-call
  vertex colour cache index), colour caches 0xC20968 (r), 0xC20168 (g), 0xC1F968 (b) (u8[0x800] each), TempVertexBuffer
  0xC4D958 (RwIm3DVertex stride 0x24: +0 x, +4 y, +8 z, +0x18 ARGB colour, +0x1C u, +0x20 v), aTempBufferIndices 0xC4B958
  (u16), uiTempBufferVerticesStored 0xC4B950, uiTempBufferIndicesStored 0xC4B954, CVector 0xC278D4 (wave normal sink).
- Behaviour:
  1. `colourIdx [0xC1F960] = 0` (0x6E878A); flush 0x6E7680 (0x6E8790). `inv = 0x858624 (1.0) / (float)N` (fild + fdivr,
     kept in ST0, not rounded to f32 until used as multiplier).
  2. Base vertex selection (0x6E87AA `cmp X1, X3`):
     - `X1 == X3` (V3 straight above/below V1): base B = (X1, Y1, P1), along-x partner = P2:
       `dX = (X2 - X1) / N` (idiv, 0x6E87E6), `dY = (Y3 - Y1) / N` (0x6E8851).
     - else: B = (X2, Y2, P2), along-x partner = P1: `dX = (X1 - X2) / N` (0x6E88AA), `dY = (Y3 - Y2) / N` (0x6E8915).
     Per-step increments (each `(Pa.c - B.c) * inv`, stored f32): `dzX` (partner.z), `dzY` (P3.z), `dbigX`, `dbigY`,
     `dsmallX`, `dsmallY` (0x6E87EF-0x6E8948 / 0x6E88B3-0x6E8948). `idiv` truncates toward zero; `N == 0` faults (idiv by 0).
  3. UV origin by `layer` (0x6E893F, `sub eax,0`):
     - layer 0: `u0 = (float)B.x * 0x859018 (0.08) + TextureShiftSecondU`, `v0 = (float)B.y * 0.08 + TextureShiftSecondV`
       (0x6E8981-0x6E89A5);
     - layer 1: same with 0x858CEC (0.04) and TextureShiftFirstU/V (0x6E895B-0x6E897F);
     - both: `u0 = u0 - floor(u0)`, `v0 = v0 - floor(v0)` (0x6E89A9-0x6E89D3; value passed as double, `fsubr` of the f32 slot,
       stored f32). Other layers: u0/v0 left uninitialised (only the dead layer-2 path follows and it does not read them).
  4. `cam = TheCamera.m_matrix ? m_matrix->pos : TheCamera.placement.pos` (0x6E89D7-0x6E8A0B, copied). If `N < 0` return.
  5. Rows `j = 0 .. N` (0x6E8A40, `jle` 0x6E8EB5, signed); `curY = B.y + j*dY`, `yAcc = j*dY` (ints, incremented by dY).
     Per row: `zj = dzY * (float)j`, `bigj = dbigY * j`, `smallj = dsmallY * j` (f32, 0x6E8A4E-0x6E8A79); `Yf = (float)curY`.
     Columns `i = 0 .. N - j` (inner bound kept as `N - j`, 0x6E8E43 `jle`): `curX = B.x + i*dX`, `xAcc = i*dX`.
     a. `z = (dzX * i + zj) + B.z`; `big = (dbigX * i + bigj) + B.big`; `small = (dsmallX * i + smallj) + B.small`
        (0x6E8A90-0x6E8ACA; each stored f32; i is fild'ed once and reused).
     b. `Xf = (float)curX` (stored f32), `dx = cam.x - Xf`, `dy = cam.y - Yf` (stored f32);
        `t = sqrt(dy*dy + dx*dx) / (float)DETAILEDWATERDIST` (`fidiv` of the int global, 0x6E8B27; stored f32).
        Wave fade `f`: if `t > 1.0` -> 0x858B50 (0.0); else if `t > 0x858F34 (0.75)` -> `(1.0 - t) * 0x858B90 (4.0)`; else 1.0
        (NaN t -> 1.0) (0x6E8B31-0x6E8B65).
     c. layer 0 (0x6E8C7D): `CalculateWavesOnlyForCoordinate(curX, curY, f * big, small * f, &z, &colorMult, &glare,
        *(CVector*)0xC278D4)` (z is in/out: the wave height is added to it). Vertex `v = uiTempBufferVerticesStored`:
        `u = (float)xAcc * 0.08 + u0`, `vv = (float)yAcc * 0.08 + v0`, pos `(Xf, Yf, z)`. Colour (the wave colorMult output is
        ignored; the slot is overwritten with 0.577f at 0x6E8CE6): `r = _ftol((float)WaterColor.r * 0x872004 (0.577))`,
        same for g, b; `colour = (WaterLayerAlpha[0] << 24) | (r << 16) | (g << 8) | b` (built by shifts, so alpha bits above 8
        are shifted out). Cache `0xC20968[k] = r, 0xC20168[k] = g, 0xC1F968[k] = b`, `k = ++[0xC1F960]` (0x6E8D93-0x6E8DB8).
     d. layer 1 (0x6E8C13): ONLY UV and colour are written (position is NOT written: it reuses the position the layer-0 call
        left in the same buffer slot, valid because each call starts at slot 0 after the flush): `u = (float)xAcc * 0.04 + u0`,
        `vv = (float)yAcc * 0.04 + v0`, `colour = (WaterLayerAlpha[1] << 24) | (r[k] << 16) | (g[k] << 8) | b[k]` from the caches,
        `k++` (0x6E8C61).
     e. layer 2 (0x6E8B8B, unreachable from the only caller): `u = (float)xAcc * [0x8D3834] + [0xC21174]`, `vv = (float)yAcc *
        [0x8D3834] + [0xC21170]`, pos `(Xf, Yf, z + 0x858B1C (0.1))`, `a = _ftol((float)WaterLayerAlpha[2] * glare)` where `glare`
        ([esp+0x9C]) is never written in a layer-2 call (stale stack); colour `0xFF000000 | a<<16 | a<<8 | a`. Other layers:
        no vertex data written.
     f. Indices (0x6E8DC2), when `j != 0 && i != 0`: with `v` = current vertex, `w = v + j - N`:
        `(v, v-1, w-3)` and `(v, w-2, w-3)` (u16 stores), indices += 6. Then `uiTempBufferVerticesStored = v + 1` (always,
        all layers).
     After each row (0x6E8E55), when `j != 0`: one more triangle `(v'-1, w'-2, w'-3)` with `v'` = next free vertex,
     `w' = v' + j - N`; indices += 3.
- Edge cases / quirks: no buffer-capacity check here (the caller rejects `3*N*N >= 0x1000` indices or `verts >= 0x800`,
  0x6EDE9F/0x6EDEAA); colour cache index unbounded beyond 0x800. Rows always start at B.x (right angle at B).
  Different from the rectangle twin 0x6E91D0: the triangle ignores the wave colorMult (constant 0.577).
- gta-reversed: absent (listed as missing; caller is plugin::Call). Open questions: none for reachable paths; layer 2 reads
  uninitialised stack (0x6E8BE7) but is never called with layer 2.

## 0x6E91D0 CWaterLevel::sub_6E91D0 (proposed: CWaterLevel::RenderHighDetailWaterRectangle_OneLayer)  (catalogue: unknown/REA, port level 7)
- Name evidence: gr lists `RenderHighDetailWaterRectangle_OneLayer(int32, int32, int32, int32, CRenPar, CRenPar, CRenPar,
  CRenPar, int32, int32, int32, int32, int32)` as missing (WaterLevel.h:336); the only caller
  CWaterLevel::RenderHighDetailWaterRectangle 0x6EB810 calls it with layer 0 (0x6EB96E `push 0`, call 0x6EBA1D) and layer 1
  (0x6EBA49 `push 1`, call 0x6EBAF8).
- Convention: `__cdecl void`, plain `ret`, 0x64 bytes of arguments: `minX [esp+4], maxX [+8], Y1 [+0xC], Y2 [+0x10],
  CRenPar P1 [+0x14], P2 [+0x24], P3 [+0x34], P4 [+0x44] (z, bigWaves, smallWaves, flow each), int32 layer [+0x54],
  int32 numTris [+0x58], int32 numVerts [+0x5C], int32 NX [+0x60], int32 NY [+0x64]`. `numTris` (caller: 2*NX*NY,
  0x6EB911) and `numVerts` (caller: (NX+1)*(NY+1), 0x6EB91B) are never read. Corner meaning from the bytes:
  P1 = (minX, Y1), P2 = (maxX, Y1), P3 = (minX, Y2), P4 = (maxX, Y2) (P2-P1 and P3-P4 are the x steps, P3-P1 and P2-P4 the
  y steps). Frame `sub esp,0xC4` + 4 pushes; stack offsets below as in the instruction text.
- Callees / globals: as 0x6E8780 (flush 0x6E7680, CalculateWavesOnlyForCoordinate 0x6E6EF0, floor 0x8219F0, `_ftol`
  0x821B40, TheCamera, DETAILEDWATERDIST, TextureShift*, WaterColor, WaterLayerAlpha[0..1], colour caches 0xC20968 /
  0xC20168 / 0xC1F968 with index 0xC1F960, temp vertex/index buffers, normal sink 0xC278D4). Extra: byte 0xC278E0 (read only
  at 0x6E9859; no code in the image writes it, so it is always 0).
- Behaviour:
  1. `[0xC1F960] = 0` (0x6E91DA); flush 0x6E7680. `invNX = 0x858624 (1.0) / (float)NX`, `invNY = 1.0 / (float)NY` (stored f32).
     `dX = (maxX - minX) / NX`, `dY = (Y2 - Y1) / NY` (idiv). Also `dXn = (minX - maxX) / NX`, `dYn = (Y1 - Y2) / NY`
     (separate idivs, 0x6E92C6 / 0x6E9325; equal to -dX / -dY because idiv truncates toward zero).
     Increments (f32, each `(Pa.c - Pb.c) * inv`): from P1: `dz1x = (P2.z-P1.z)*invNX`, `dz1y = (P3.z-P1.z)*invNY`, same for big
     and small; from P4: `dz4x = (P3.z-P4.z)*invNX`, `dz4y = (P2.z-P4.z)*invNY`, same for big and small (0x6E925C-0x6E93CA).
     `NX == 0` or `NY == 0` faults (idiv).
  2. UV origin by layer (0x6E9366 `sub eax,0`): layer 0 `u0 = (float)minX * 0x859018 (0.08) + TextureShiftSecondU`, `v0 =
     (float)Y1 * 0.08 + TextureShiftSecondV`; layer 1: 0x858CEC (0.04) and TextureShiftFirstU/V; both `u0 -= floor(u0)`,
     `v0 -= floor(v0)` (0x6E9431-0x6E945B). Other layers: uninitialised (unused).
  3. Camera pos as in 0x6E8780 (0x6E945F-0x6E948C). If `NY < 0` return.
  4. Rows `j = 0 .. NY` (signed, 0x6E992A); `yAcc = j*dY`; P4-side `y4 = Y2 + NY*dYn` then `+= -dYn` per row; `rj = NY - j`.
     Columns `i = 0 .. NX` (0x6E98EB, against arg NX); `xAcc = i*dX`; P4-side `x4 = maxX + NX*dXn`, `+= -dXn` per column;
     `ri = NX - i`.
     a. Split test (0x6E9511): `s = (float)i * invNX + (float)j * invNY` (`fild i; fmul invNX; fadd [j*invNY]` where
        `j*invNY` was stored f32 per row). If `s < 1.0` (`test ah,5; jp` takes the else on >= or NaN):
        P1 plane: `X = minX + xAcc`, `Y = Y1 + yAcc`, `z = (j*dz1y + i*dz1x) + P1.z`, `big = (j*dbig1y + i*dbig1x) + P1.big`,
        `small = (i*dsmall1x + j*dsmall1y) + P1.small` (0x6E9531-0x6E959C; i, j as floats, z stored f32, big/small on stack).
        Else P4 plane (0x6E95A5): `X = x4`, `Y = y4`, `a = (float)rj`, `b = (float)ri`:
        `z = (a*dz4y + b*dz4x) + P4.z`, `big = (a*dbig4y + b*dbig4x) + P4.big`, `small = (a*dsmall4y + b*dsmall4x) + P4.small`.
     b. `Xf = (float)X`, `Yf = (float)Y` (stored f32), `dx = cam.x - Xf`, `dy = cam.y - Yf`,
        `t = sqrt(dy*dy + dx*dx) / (float)DETAILEDWATERDIST`; fade `f` exactly as 0x6E8780 (1.0 / 0.75 / 4.0 / 0.0 constants
        0x858624, 0x858F34, 0x858B90, 0x858B50; 0x6E967F-0x6E96B3).
     c. layer 0 (0x6E9733): `CalculateWavesOnlyForCoordinate(X, Y, f * big, small * f, &z, &colorMult, &glare, *(CVector*)0xC278D4)`;
        vertex at `v = uiTempBufferVerticesStored`: `u = (float)xAcc * 0.08 + u0`, `vv = (float)yAcc * 0.08 + v0`, pos (Xf, Yf, z);
        colour channels `_ftol((float)WaterColor.c * colorMult)` (the wave output IS used here, 0x6E97C9/0x6E97E3/0x6E97FF),
        `colour = WaterLayerAlpha[0] << 24 | r << 16 | g << 8 | b`; cache r/g/b at 0xC20968/0xC20168/0xC1F968[k]; `k++`.
     d. layer 1 (0x6E96D0): only UV (`xAcc * 0.04 + u0`, `yAcc * 0.04 + v0`) and colour `WaterLayerAlpha[1] << 24 | cached
        r,g,b` are written (position reused from the layer-0 call); `k++`.
     e. other layers: nothing written, `k` not incremented.
     f. If byte `[0xC278E0] != 0` (0x6E9860) skip the rest of this column (no indices, vertex count NOT incremented).
        Else, when `j != 0 && i != 0`: with `v` current vertex: `(v-NX-2, v-NX-1, v-1)` and `(v, v-NX-1, v-1)` (0x6E9872-0x6E98BA),
        indices += 6; then `uiTempBufferVerticesStored = v + 1`.
- Edge cases / quirks: grid diagonal runs from the vertex above to the vertex left (matches the s < 1 split). P1-side and
  P4-side positions coincide only when the spans divide evenly by NX/NY (truncated steps; the P4 side is anchored at maxX/Y2
  minus whole steps). No capacity check here (caller checks 0x6EB925 / 0x6EB935).
- gta-reversed: absent (listed as missing; the caller is plugin::Call). Open questions: purpose of byte 0xC278E0 (never
  written; treat as constant 0) - 0x6E9859.

## 0x6FBA20 CCoronas::sub_6FBA20 (proposed: CCoronas::GetSunReflectionDistanceFade)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller CCoronas::RenderSunReflection 0x6FBAA0 (5 sites, 0x6FBBC5..0x6FBC7D), each result multiplied into the
  reflection intensity (`fmul [esp+0x34]`); gr inlines it as `std::clamp(invLerp(0, 100, dist2D - 250), 0, 1)` (Coronas.cpp:485).
- Convention: `__cdecl float (CVector point [esp+4..0xC] by value, float radius [esp+0x10])`, result in ST0, plain `ret`, caller
  pops. Entry 0x6FBA20 is `jmp 0x4064A7`, a .HOODLUM stub: `eax = rol(dword[0x1560818]=0x1E005B78, cl=0xE9&31=9)` = 0xB6F03C
  (`TheCamera.m_matrix`, CPlaceable+0x14 of TheCamera 0xB6F028), `eax = [0xB6F03C]`, then back at 0x6FBA25. EFLAGS/ECX restored.
- Globals: TheCamera.m_matrix 0xB6F03C (via the stub), TheCamera placement pos 0xB6F02C. Constants: 0x858628 = 100.0f,
  0x858B50 = 0.0f, 0x858C58 = 0.01f, 0x858624 = 1.0f.
- Behaviour: `P = TheCamera.m_matrix ? &m_matrix->pos (+0x30) : (CVector*)0xB6F02C` (0x6FBA25..0x6FBA2E; no allocation).
  `dx = P.x - point.x; dy = P.y - point.y` (x87); `d = sqrt(dx*dx + dy*dy)` (the sum is `dy*dy + dx*dx` folded as faddp; 0x6FBA40..
  0x6FBA4A), stored f32 into the point.x arg slot (0x6FBA50). Then:
  1. if `!(radius + 100.0f > d)` (fcomp + `test ah,0x41`, 0x6FBA54..0x6FBA67; equal or NaN also take it) return 1.0f (0x6FBA8E);
  2. else if `d <= radius` return 0.0f (0x6FBA78): fcomp d vs radius, `test ah,0x41; jp 0x6FBA7F` (0x6FBA69..0x6FBA76) jumps to
     step 3 when neither C0 nor C3 is set (d > r) or both are (unordered; unreachable since NaN already left in step 1);
  3. else return `(d - radius) * 0.01f` (0x6FBA7F..0x6FBA87, d as stored f32).
- gta-reversed LOGIC ERROR (in RenderSunReflection, not this helper): gr uses radius 250 for all five points; the original passes
  350.0f (`push 0x43AF0000`, 0x6FBC35) for the 4th point (2372, -1854, 0) and 250.0f (0x437A0000) for the other four.
  gr also computes `(d-250)/100` instead of `(d - r) * 0.01f` (rounding differs by an ulp). Open questions: none.

## 0x6FE260 RoadsignGetLineAndRow  (catalogue: specified/REA, port level 0)
- Convention (non-standard): `char c` in AL (register arg), `int32* line [esp+4]` (charset column), `int32* row [esp+8]`; plain
  `ret` (cdecl, caller pops); returns nothing meaningful (EAX/ECX/EDX clobbered). gr notes the AL argument (CustomRoadsignMgr.h:32).
- Caller: CCustomRoadsignMgr::RoadsignGenerateTextRaster 0x6FEB70 (one site).
- Behaviour: `idx = (u32)(u8)c - 0x21` (`movzx eax,al; add eax,-0x21`, 0x6FE260/0x6FE263, so c is treated as UNSIGNED); if
  `idx > 0x5D` (unsigned, 0x6FE266) go to the default case 0x6FE9DF; else `jmp [0x6FE9F4 + idx*4]` (94-entry jump table directly
  after the code). Every case stores `*line = L; *row = R` (line first) and returns. Table (char = line,row (case address)):
  `!`=0,0 (0x6FE276); `#`=3,21 (0x6FE944); `$`=0,22 (0x6FE95B); `%`=2,21 (0x6FE92D); `&`=2,0 (0x6FE287); `(`=0,1 (0x6FE29B);
  `)`=1,1 (0x6FE2AF); `*`=1,22 (0x6FE96F); `+`=2,1 (0x6FE2C3); `-`=0,2 (0x6FE2DA); `.`=1,2 (0x6FE2EE); `0`=3,2 (0x6FE305);
  `1`=0,3 (0x6FE31C); `2`=1,3 (0x6FE330); `3`=2,3 (0x6FE347); `4`=3,3 (0x6FE35E); `5`=0,4 (0x6FE372); `6`=1,4 (0x6FE386);
  `7`=2,4 (0x6FE39D); `8`=3,4 (0x6FE3B4); `9`=0,5 (0x6FE3CB); `:`=2,5 (0x6FE3F6); `;`=1,5 (0x6FE3DF); `<`=2,20 (0x6FE8D4);
  `>`=3,20 (0x6FE8EB); `?`=3,5 (0x6FE40D); `@`=2,22 (0x6FE986); `A`=0,6 (0x6FE424); `B`=1,6 (0x6FE438); `C`=2,6 (0x6FE44F);
  `D`=3,6 (0x6FE466); `E`=0,7 (0x6FE47D); `F`=1,7 (0x6FE491); `G`=2,7 (0x6FE4A8); `H`=3,7 (0x6FE4BF); `I`=0,8 (0x6FE4D6);
  `J`=1,8 (0x6FE4EA); `K`=2,8 (0x6FE501); `L`=3,8 (0x6FE518); `M`=0,9 (0x6FE52F); `N`=1,9 (0x6FE543); `O`=2,9 (0x6FE55A);
  `P`=3,9 (0x6FE571); `Q`=0,10 (0x6FE588); `R`=1,10 (0x6FE59C); `S`=2,10 (0x6FE5B3); `T`=3,10 (0x6FE5CA); `U`=0,11 (0x6FE5E1);
  `V`=1,11 (0x6FE5F5); `W`=2,11 (0x6FE60C); `X`=3,11 (0x6FE623); `Y`=0,12 (0x6FE63A); `Z`=1,12 (0x6FE64E); `[`=2,12 (0x6FE665);
  `]`=0,13 (0x6FE67C); `^`=0,21 (0x6FE902); `a`=1,13 (0x6FE690); `b`=2,13 (0x6FE6A7); `c`=3,13 (0x6FE6BE); `d`=0,14 (0x6FE6D5);
  `e`=1,14 (0x6FE6E9); `f`=2,14 (0x6FE700); `g`=3,14 (0x6FE717); `h`=0,15 (0x6FE72E); `i`=1,15 (0x6FE742); `j`=2,15 (0x6FE759);
  `k`=3,15 (0x6FE770); `l`=0,16 (0x6FE787); `m`=1,16 (0x6FE79B); `n`=2,16 (0x6FE7B2); `o`=3,16 (0x6FE7C9); `p`=0,17 (0x6FE7E0);
  `q`=1,17 (0x6FE7F4); `r`=2,17 (0x6FE80B); `s`=3,17 (0x6FE822); `t`=0,18 (0x6FE839); `u`=1,18 (0x6FE84D); `v`=2,18 (0x6FE864);
  `w`=3,18 (0x6FE87B); `x`=0,19 (0x6FE892); `y`=1,19 (0x6FE8A6); `z`=2,19 (0x6FE8BD); `{`=0,23 (0x6FE9B4); `|`=3,22 (0x6FE99D);
  `}`=2,23 (0x6FE9C8); `~`=1,21 (0x6FE916)
  Default (0x6FE9DF; every other byte, including ' ', 0x00-0x20, 0x7F-0xFF and the printable characters
  `"` `'` `,` `/` `=` `\` `_` and backtick (0x60)): `*line = 0; *row = 26`.
- gta-reversed: CustomRoadsignMgr.cpp:385, the 86 cases and the default were compared mechanically against the jump-table cases:
  identical. (gr takes `char` signed, but every non-ASCII byte reaches the default in both.) Open questions: none.

## 0x70FA70 CStencilShadows::sub_70FA70 (proposed: CStencilShadows::ToggleSilhouetteEdge)  (catalogue: unknown/REA, port level 0)
- Name evidence: callers CStencilShadows::RenderForVehicle 0x70FAE0 and RenderForObject 0x710310 call it 3 times in a row per
  shadow triangle with the vertex-index pairs (a,b), (b,c), (c,a) (0x70FEFC/0x70FF10/0x70FF22): an edge shared by two
  back-facing triangles cancels out, leaving the silhouette edge list.
- Convention (custom, register args): ECX = `uint16 (*edges)[2]` (array of 4-byte {u16 v0, u16 v1} pairs), EDX = `uint16* count`,
  DI (low 16 bits of EDI) = vertex a, `uint16 b` at [esp+4] (read as a word, 0x70FA75; pushed as a dword); plain `ret`, caller pops.
  ECX, EDX and EDI are PRESERVED - the third call site (0x70FF22) relies on ECX still holding `edges` (no reload). EAX clobbered;
  EBX/ESI/EBP saved/restored.
- Behaviour:
  1. `n = *count` (zero-extended, captured once in EBX at 0x70FA71). Search `i = 0 .. ` while `i < *count` (the bound is re-read
     from memory each pass, 0x70FA9D): match if `(edges[i].v0 == a && edges[i].v1 == b)` (0x70FA85/0x70FA8A) or
     `(edges[i].v0 == b && edges[i].v1 == a)` (0x70FA91/0x70FA96), 16-bit compares. If `n == 0` the search is skipped (0x70FA7F).
  2. Found at i (0x70FAB8): if `*count > 1` (unsigned word compare, `jbe` at 0x70FABC) copy the LAST pair into slot i:
     `edges[i].v0 = edges[n-1].v0` (uses the entry snapshot n, 0x70FABE) and `edges[i].v1 = edges[*count-1].v1` (re-reads
     *count, 0x70FAC7). Then `--*count` (word, 0x70FAD4). Unordered removal.
  3. Not found (0x70FAA5): `edges[n].v0 = a; edges[*count].v1 = b; ++*count` (word increment, 0x70FAB2). No capacity check.
- Edge: no bound on the edge array (callers size it); n and *count are equal throughout (only this function writes count), so the
  snapshot/re-read mix has no visible effect.
- gta-reversed: absent (RenderForVehicle/RenderForObject are `.Reversed = false` stubs, StencilShadows.cpp:21-22). Open questions: none.

## 0x710BA0 CStencilShadows::sub_710BA0 (proposed: CStencilShadows::FindActiveShadowObject)  (catalogue: unknown/REA, port level 0)
- Name evidence: walks the active CStencilShadowObject list; only caller CStencilShadows::RegisterStencilShadows 0x711760
  (`push ebx(entity); call` at 0x7118C8, result tested for null before creating a new shadow object). The same loop is inlined
  in RegisterStencilShadows at 0x711BF3 and 0x711D10.
- Convention: `__cdecl CStencilShadowObject* (CEntity* owner [esp+4])`, plain `ret`, caller pops (0x7118CE). Returns EAX (null if none).
  Clobbers ECX.
- Globals: `CStencilShadows::pFirstActiveStencilShadowObject` 0xC6A16C (read once, 0x710BA0).
- Fields: CStencilShadowObject +0x00 m_pOwner, +0x14 m_pNext (gr StencilShadowObject.h).
- Behaviour: `for (o = pFirstActive; o; o = o->m_pNext) if (o->m_pOwner == owner) return o; return NULL;` (0x710BB0..0x710BBB).
- gta-reversed: absent as a function (RegisterStencilShadows is an unreversed stub). Open questions: none.

## 0x711160 CStencilShadows::sub_711160 (proposed: CStencilShadows::GetSignedSqDistToBoundSphere_Object)  (catalogue: unknown/REA, port level 2)
- Name evidence: only caller CStencilShadows::RegisterStencilShadows 0x711760, used for shadow type 1 (object; dispatch on
  `obj+6` at 0x711816..0x711834 and on the create paths, sites 0x711834, 0x711BFE, 0x711D1B); the result is compared with 0x8598B0 = 2500.0f (= 50^2).
  Byte-for-byte the same body as 0x7111F0 (only addresses differ) - two compiler copies of one inline helper.
- Convention (custom): EBX = `CEntity* entity`, ESI = `const CVector* point` (register args, no stack args); result in ST0;
  plain `ret`. EDI preserved (saved/restored); EAX, ECX, EDX clobbered; EBX/ESI unchanged.
- Callees: CEntity::GetColModel 0x535300 (thiscall ECX = entity), CEntity::TransformFromObjectSpace 0x5334F0
  (thiscall (CVector& out, const CVector& in), returns out; out = local at [esp+8] of the 0x14-byte frame).
- Constants: 0x858624 = 1.0f, 0x858C1C = -1.0f.
- Fields: CColModel +0x18 bounding-sphere centre (CVector), +0x24 radius (f32) (gr CColModel m_boundSphere).
- Behaviour: `cm = entity->GetColModel()` (0x711166); `W = entity->TransformFromObjectSpace(cm->boundSphere.center)` (0x711178);
  `r = cm->boundSphere.radius` (copied to a local, 0x711183/0x711189);
  `dx = P.x - W.x; dy = P.y - W.y; dz = P.z - W.z` (P = *point); `d = sqrt((dz*dz + dy*dy) + dx*dx)` (x87 order 0x711199..
  0x7111A9), stored f32 (0x7111B1). `s = 1.0f`; if `d < r` (fcomp d vs r, `test ah,5; jp` at 0x7111BF..0x7111C8: the jump that keeps
  1.0 is taken when C0 and C2 are both clear (d >= r) or both set (unordered)) `s = -1.0f`. Return `s * (d - r) * (d - r)`
  (computed as `t = d - r` from the stored d; `s * t` then `* t`, 0x7111D2..0x7111DE).
- I.e. the signed squared distance from the point to the bounding-sphere surface (negative inside).
- gta-reversed: absent (RegisterStencilShadows is `.Reversed = false`). Open questions: none.

## 0x7111F0 CStencilShadows::sub_7111F0 (proposed: CStencilShadows::GetSignedSqDistToBoundSphere_Vehicle)  (catalogue: unknown/REA, port level 2)
- Name evidence: only caller CStencilShadows::RegisterStencilShadows 0x711760, used for shadow type 2 (vehicle; dispatch on
  `obj+6` at 0x711816..0x711834 and on the create paths, sites 0x711820, 0x7118D5); the result is compared with 0x8598B0 = 2500.0f (= 50^2).
  Byte-for-byte the same body as 0x711160 (only addresses differ) - two compiler copies of one inline helper.
- Convention (custom): EBX = `CEntity* entity`, ESI = `const CVector* point` (register args, no stack args); result in ST0;
  plain `ret`. EDI preserved (saved/restored); EAX, ECX, EDX clobbered; EBX/ESI unchanged.
- Callees: CEntity::GetColModel 0x535300 (thiscall ECX = entity), CEntity::TransformFromObjectSpace 0x5334F0
  (thiscall (CVector& out, const CVector& in), returns out; out = local at [esp+8] of the 0x14-byte frame).
- Constants: 0x858624 = 1.0f, 0x858C1C = -1.0f.
- Fields: CColModel +0x18 bounding-sphere centre (CVector), +0x24 radius (f32) (gr CColModel m_boundSphere).
- Behaviour: `cm = entity->GetColModel()` (0x7111F6); `W = entity->TransformFromObjectSpace(cm->boundSphere.center)` (0x711208);
  `r = cm->boundSphere.radius` (copied to a local, 0x711213/0x711219);
  `dx = P.x - W.x; dy = P.y - W.y; dz = P.z - W.z` (P = *point); `d = sqrt((dz*dz + dy*dy) + dx*dx)` (x87 order 0x711229..
  0x711239), stored f32 (0x711241). `s = 1.0f`; if `d < r` (fcomp d vs r, `test ah,5; jp` at 0x71124F..0x711258: the jump that keeps
  1.0 is taken when C0 and C2 are both clear (d >= r) or both set (unordered)) `s = -1.0f`. Return `s * (d - r) * (d - r)`
  (computed as `t = d - r` from the stored d; `s * t` then `* t`, 0x711262..0x71126E).
- I.e. the signed squared distance from the point to the bounding-sphere surface (negative inside).
- gta-reversed: absent (RegisterStencilShadows is `.Reversed = false`). Open questions: none.

## 0x711280 CStencilShadowObject::sub_711280 (proposed: CStencilShadowObject::Create / CStencilShadows::ActivateShadowObject)  (catalogue: unknown/REA, port level 2)
- Name evidence: only caller CStencilShadows::RegisterStencilShadows 0x711760 (3 sites: 0x7118F8 with type 2 for vehicles,
  0x711C21/0x711D3A with type 1 for objects), each with `ECX = pFirstAvailableStencilShadowObject` (0x7118E7/0x711C10/0x711D2D)
  after the owner was not found in the active list; it fills the free object and moves it to the active list.
- Convention: `__thiscall CStencilShadowObject* (this = ECX, CEntity* owner [esp+4], uint8 type [esp+8])`, `ret 8`. Returns this,
  or NULL when rejected. Part of the body is a .HOODLUM detour: 0x7112CB `jmp 0x401ABC` (`mov dword [esi+0xC], 0; jmp 0x7112D0`).
- Callees: CEntity::GetColModel 0x535300, CMemoryMgr::Malloc 0x72F420 (cdecl (size, hint=0), `add esp,8`), CEntity::RegisterReference
  0x571B70 (thiscall ECX = owner, arg `CEntity** ref` = &this->m_pOwner (this+0)), CStencilShadows::UpdateHierarchy 0x710BC0 (cdecl
  (&pFirstAvailable 0xC6A168, &pFirstActive 0xC6A16C, this), caller pops 0xC).
- Fields (CStencilShadowObject, 0x1C): +0 m_pOwner, +4 i16 number of shadow triangles (gr m_NumShadowFaces), +6 u8 type,
  +8 u32 (= tris*15, see below; gr calls +8 `m_FaceID`), +0xC u32 (zeroed; gr `m_SizeOfShadowFacesData`), +0x10 CVector* face data.
  CColModel +0x2C m_pColData; CCollisionData +7 flags (bit 0x4 bHasShadowInfo), +0x20 u32 m_nNumShadowTriangles.
- Behaviour:
  1. If `type == 0` (`test bl,bl; jbe`, 0x71128B) or `type > 2` (unsigned, 0x711290) return NULL.
  2. `cd = owner->GetColModel()->m_pColData` (0x711298/0x71129D); NULL -> return NULL; `!(cd->flags7 & 4)` -> return NULL (0x7112A4);
     `n = cd->m_nNumShadowTriangles`; if `n <= 0` (signed, 0x7112AF) return NULL.
  3. `this+4 = (i16)n` (0x7112B1); `this+8 = n*15` (0x7112B5/0x7112B8); `this+0 = owner` (0x7112C4); `this+6 = type` (0x7112C6);
     `this+0xC = 0` (0x401ABC); `this+0x10 = CMemoryMgr::Malloc(n*15*12, 0)` (size = n*180 bytes = 15 CVectors per triangle,
     0x7112BB..0x7112DB; result not checked).
  4. `owner->RegisterReference(&this->m_pOwner)` (0x7112DE); `UpdateHierarchy(&pFirstAvailableStencilShadowObject,
     &pFirstActiveStencilShadowObject, this)` (0x7112EE); return this.
- Rejected paths leave the object untouched. Note gr's field names for +8/+0xC look swapped relative to this use (+8 holds the
  capacity in CVectors, +0xC the used size, reset to 0); gr's RenderStencilShadows reads +0xC as the data size (StencilShadows.cpp:70).
- gta-reversed: absent. Open questions: none (the +8/+0xC naming is a gr-layout remark, behaviour is from the bytes).

## 0x734EA0 sub_734EA0 (proposed: RwIm2DRenderIndexedPrimitive, HOODLUM-wrapped RW device call)  (catalogue: unknown/REA, port level 0)
- Name evidence: tail-jumps through `RwEngineInstance->dOpenDevice.fpIm2DRenderIndexedPrimitive`; callers CPostEffects::HeatHazeFX
  0x701780, CShadowCamera::DrawOutlineBorder 0x705790, CSprite::FlushSpriteBuffer 0x70CF20 (2), CMirrors::RenderMirrorBuffer
  0x726090, CSprite2d::RenderVertexBuffer 0x7273D0 (e.g. `push numIdx, 0xC4B958 indices, numVtx, 0xC5F958 vertices, 3` at
  0x7273F1..0x7273FD), matching gr's `RwIm2DRenderIndexedPrimitive(rwPRIMTYPETRILIST, aRadiosityVertexBuffer, n, aTempBufferIndices, m)`
  (Sprite2d.cpp:202).
- Convention: `__cdecl RwBool (RwPrimitiveType type [esp+4], RwIm2DVertex* verts [+8], RwInt32 numVerts [+0xC], RwImVertexIndex*
  indices [+0x10], RwInt32 numIndices [+0x14])` - arguments are passed through untouched to the device function (tail jump).
- Body: 0x734EA0 `nop; jmp 0x403927` (.HOODLUM stub): `pushfd; edx = dword[0x1560778] = 0x92F64801; ecx saved; cl = byte[0x698EFA]
  = 0xE9; ror edx, cl` (0xE9 & 31 = 9) -> 0xC97B24 (`RwEngineInstance`); `edx = [0xC97B24]`; restore ECX/EFLAGS; `jmp 0x734EA6`:
  `jmp dword ptr [edx + 0x34]` = RwGlobals.dOpenDevice (+0x10) .fpIm2DRenderIndexedPrimitive (+0x24). EDX is clobbered.
- Port: call the RW device's Im2D indexed-primitive entry (or the project's RwIm2DRenderIndexedPrimitive). gta-reversed: uses the
  RW macro/function of the same name. Open questions: none.
