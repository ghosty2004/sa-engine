# RENDER: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x5A2E90 ProcSurfaceInfo_c::ProcSurfaceInfo_c  (catalogue: specified/REA, port level 1)
- gr ProcSurfaceInfo.h:33 (`// 0x5A2E90`). Element ctor (size 0x48, count 0x80) used by ProcObjectMan_c ctor 0x5A3E20.
- Convention: `__thiscall ProcSurfaceInfo_c* (this = ECX)`, returns this. Behaviour: `List_c::List_c(this+0x3C)` (0x4A8DD0); return this. Other fields untouched. Open questions: none.

## 0x5A3070 ProcObjectListItem_c::ProcObjectListItem_c (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: element ctor (size 0x14, count 0x200, dtor 0x5A3080) used by ProcObjectMan_c ctor for `this+0x2408` (gr ProcObjectMan.h `ProcObjectListItem_c m_objects[512]`). Body = ListItem_c base ctor only (byte-identical to InteriorGroup_c::Constructor 0x597FE0).
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: `ListItem_c::ListItem_c(this)` (0x4A8DB0). Open questions: none.

## 0x5A3E20 ProcObjectMan_c::ProcObjectMan_c  (catalogue: specified/REA, port level 1)
- gr ProcObjectMan.h:30 (`// 0x5A3E20`). Only caller __init_g_procObjMan 0x84FDC0. SEH frame handler 0x83D404.
- Convention: `__thiscall ProcObjectMan_c* (this = ECX)`, returns this.
- Behaviour: `eh_vector_constructor_iterator(this+8, 0x48, 0x80, ProcSurfaceInfo_c ctor 0x5A2E90, dtor 0x5A2EA0)`; state 0; `eh_vector_constructor_iterator(this+0x2408, 0x14, 0x200, 0x5A3070, 0x5A3080)`; state 1; `List_c::List_c(this+0x4C08)` (0x4A8DD0); return this. `this+0`/`+4` (m_numSurfaceInfos etc.) are not written.
- Open questions: none.

## 0x5C16F0 FxInterpInfoFloat_c::Load  (catalogue: specified/REA, port level 2)
- gr Fx/Blueprints/Interp/FxInterpInfoFloat.cpp:11 (comment 0x5C16F0).
- Convention: `__thiscall void (this = ECX, FILESTREAM file [esp+4])`, `ret 4`; vtable slot 1 (no direct callers). 0x188-byte stack frame with a 256-byte line buffer.
- Callees: 0x4AB250 (Fx line reader thunk: `(file, buf, 0x100)`, thunk to 0x5389B0), CRT sscanf 0x8220AD, CRT _ftol 0x821B40 (truncation), FxMemoryPool_c::GetMem 0x4A9CA0 with ECX = g_fxMan pool 0xA9AF2C (`GetMem(size, align)`).
- Fields: +4 u8 m_bLooped, +5 i8 m_nNumKeys, +6 i8 m_nCount, +8 u16* m_pTimes, +0xC T** m_Keys.
- Behaviour: `for (i = 0; i < m_nCount (signed byte); i++)`: read line, `sscanf(buf, "%s" [0x85C714], tmp)` (blank/name line); read line, `sscanf(buf, "%s", tmp)` ("FX_INTERP_DATA:"); read line, `sscanf(buf, "%s %d" [0x86A6AC], tmp, &v); m_bLooped = (u8)v`; read line, `sscanf(buf, "%s %d", tmp, &v); m_nNumKeys = (u8)v`; `if (i == 0) m_pTimes = GetMem(m_nNumKeys*2, 2)` (sign-extended count); `m_Keys[i] = GetMem(m_nNumKeys*4, 4)`; then `for (j = 0; j < m_nNumKeys; j++)`: read line + `sscanf("%s")` (key header); read line, `sscanf(buf, "%s %f" [0x86AA68], tmp, &t); m_pTimes[j] = (u16)_ftol(t * 256.0f [0x858FB4])`; read line, `sscanf(buf, "%s %f", tmp, &m_Keys[i][j])` (float stored directly).
- Edge: m_pTimes is allocated only for i == 0 with that iteration's key count; later channels with more keys overflow it (original behaviour). Return value of sscanf is not checked (unparsed fields keep stale stack values).
- gta-reversed: matches the structure; the key conversion above is what the bytes do. Open questions: none.

## 0x5C18F0 FxInterpInfoU255_c::Load  (catalogue: specified/REA, port level 2)
- gr Fx/Blueprints/Interp/FxInterpInfoU255.cpp:11 (comment 0x5C18F0).
- Convention: `__thiscall void (this = ECX, FILESTREAM file [esp+4])`, `ret 4`; vtable slot 1 (no direct callers). 0x188-byte stack frame with a 256-byte line buffer.
- Callees: 0x4AB250 (Fx line reader thunk: `(file, buf, 0x100)`, thunk to 0x5389B0), CRT sscanf 0x8220AD, CRT _ftol 0x821B40 (truncation), FxMemoryPool_c::GetMem 0x4A9CA0 with ECX = g_fxMan pool 0xA9AF2C (`GetMem(size, align)`).
- Fields: +4 u8 m_bLooped, +5 i8 m_nNumKeys, +6 i8 m_nCount, +8 u16* m_pTimes, +0xC T** m_Keys.
- Behaviour: `for (i = 0; i < m_nCount (signed byte); i++)`: read line, `sscanf(buf, "%s" [0x85C714], tmp)` (blank/name line); read line, `sscanf(buf, "%s", tmp)` ("FX_INTERP_DATA:"); read line, `sscanf(buf, "%s %d" [0x86A6AC], tmp, &v); m_bLooped = (u8)v`; read line, `sscanf(buf, "%s %d", tmp, &v); m_nNumKeys = (u8)v`; `if (i == 0) m_pTimes = GetMem(m_nNumKeys*2, 2)` (sign-extended count); `m_Keys[i] = GetMem(m_nNumKeys*2, 2)`; then `for (j = 0; j < m_nNumKeys; j++)`: read line + `sscanf("%s")` (key header); read line, `sscanf(buf, "%s %f" [0x86AA68], tmp, &t); m_pTimes[j] = (u16)_ftol(t * 256.0f [0x858FB4])`; read line, `sscanf(buf, "%s %f", tmp, &v); m_Keys[i][j] = (u16)_ftol(v * 256.0f [0x858FB4])` (the U255 class also scales by 256, not 255).
- Edge: m_pTimes is allocated only for i == 0 with that iteration's key count; later channels with more keys overflow it (original behaviour). Return value of sscanf is not checked (unparsed fields keep stale stack values).
- gta-reversed: matches the structure; the key conversion above is what the bytes do. Open questions: none.

## 0x5C1B10 FxInterpInfo32_c::Load  (catalogue: specified/REA, port level 2)
- gr Fx/Blueprints/Interp/FxInterpInfo32.cpp:11 (comment 0x5C1B10).
- Convention: `__thiscall void (this = ECX, FILESTREAM file [esp+4])`, `ret 4`; vtable slot 1 (no direct callers). 0x188-byte stack frame with a 256-byte line buffer.
- Callees: 0x4AB250 (Fx line reader thunk: `(file, buf, 0x100)`, thunk to 0x5389B0), CRT sscanf 0x8220AD, CRT _ftol 0x821B40 (truncation), FxMemoryPool_c::GetMem 0x4A9CA0 with ECX = g_fxMan pool 0xA9AF2C (`GetMem(size, align)`).
- Fields: +4 u8 m_bLooped, +5 i8 m_nNumKeys, +6 i8 m_nCount, +8 u16* m_pTimes, +0xC T** m_Keys.
- Behaviour: `for (i = 0; i < m_nCount (signed byte); i++)`: read line, `sscanf(buf, "%s" [0x85C714], tmp)` (blank/name line); read line, `sscanf(buf, "%s", tmp)` ("FX_INTERP_DATA:"); read line, `sscanf(buf, "%s %d" [0x86A6AC], tmp, &v); m_bLooped = (u8)v`; read line, `sscanf(buf, "%s %d", tmp, &v); m_nNumKeys = (u8)v`; `if (i == 0) m_pTimes = GetMem(m_nNumKeys*2, 2)` (sign-extended count); `m_Keys[i] = GetMem(m_nNumKeys*2, 2)`; then `for (j = 0; j < m_nNumKeys; j++)`: read line + `sscanf("%s")` (key header); read line, `sscanf(buf, "%s %f" [0x86AA68], tmp, &t); m_pTimes[j] = (u16)_ftol(t * 256.0f [0x858FB4])`; read line, `sscanf(buf, "%s %f", tmp, &v); m_Keys[i][j] = (i16)_ftol(v * 1000.0f [0x858C4C])`.
- Edge: m_pTimes is allocated only for i == 0 with that iteration's key count; later channels with more keys overflow it (original behaviour). Return value of sscanf is not checked (unparsed fields keep stale stack values).
- gta-reversed: matches the structure; the key conversion above is what the bytes do. Open questions: none.

## 0x5C1D30 FxInterpInfo255_c::Load  (catalogue: specified/REA, port level 2)
- gr Fx/Blueprints/Interp/FxInterpInfo255.cpp:11 (comment 0x5C1D30).
- Convention: `__thiscall void (this = ECX, FILESTREAM file [esp+4])`, `ret 4`; vtable slot 1 (no direct callers). 0x188-byte stack frame with a 256-byte line buffer.
- Callees: 0x4AB250 (Fx line reader thunk: `(file, buf, 0x100)`, thunk to 0x5389B0), CRT sscanf 0x8220AD, CRT _ftol 0x821B40 (truncation), FxMemoryPool_c::GetMem 0x4A9CA0 with ECX = g_fxMan pool 0xA9AF2C (`GetMem(size, align)`).
- Fields: +4 u8 m_bLooped, +5 i8 m_nNumKeys, +6 i8 m_nCount, +8 u16* m_pTimes, +0xC T** m_Keys.
- Behaviour: `for (i = 0; i < m_nCount (signed byte); i++)`: read line, `sscanf(buf, "%s" [0x85C714], tmp)` (blank/name line); read line, `sscanf(buf, "%s", tmp)` ("FX_INTERP_DATA:"); read line, `sscanf(buf, "%s %d" [0x86A6AC], tmp, &v); m_bLooped = (u8)v`; read line, `sscanf(buf, "%s %d", tmp, &v); m_nNumKeys = (u8)v`; `if (i == 0) m_pTimes = GetMem(m_nNumKeys*2, 2)` (sign-extended count); `m_Keys[i] = GetMem(m_nNumKeys*2, 2)`; then `for (j = 0; j < m_nNumKeys; j++)`: read line + `sscanf("%s")` (key header); read line, `sscanf(buf, "%s %f" [0x86AA68], tmp, &t); m_pTimes[j] = (u16)_ftol(t * 256.0f [0x858FB4])`; read line, `sscanf(buf, "%s %f", tmp, &v); m_Keys[i][j] = (i16)_ftol(v * 128.0f [0x858BF4])`.
- Edge: m_pTimes is allocated only for i == 0 with that iteration's key count; later channels with more keys overflow it (original behaviour). Return value of sscanf is not checked (unparsed fields keep stale stack values).
- gta-reversed: matches the structure; the key conversion above is what the bytes do. Open questions: none.

## 0x5DB290 CPlantColEntEntry::CPlantColEntEntry (proposed)  (catalogue: unknown/REA, port level 0)
- Empty element ctor (`mov eax, ecx; ret`) referenced only by __init_CPlantMgr__m_ColEntCacheTab 0x850970; writes nothing. Open questions: none.

## 0x5DB2B0 CPlantLocTri::CPlantLocTri (proposed)  (catalogue: unknown/REA, port level 0)
- Empty element ctor referenced only by __init_CPlantMgr__m_LocTrisTab; writes nothing. Open questions: none.

## 0x5DD0B0 CPlantMgr::SetPlantModelTextureCB (proposed)  (catalogue: unknown/REA, port level 4)
- Name evidence: material callback referenced only by CPlantMgr::LoadModels 0x5DD220.
- Convention: `__cdecl RpMaterial* (RpMaterial* mat [esp+4], void* data [esp+8])`. Globals: tex_gras07Si 0xC09174 (RwTexture*, read first).
- Behaviour: `mat->color (dword +4) = *(u32*)data` (RwRGBA copied as a dword); `RpMaterialSetTexture(mat, tex_gras07Si)` (0x74DBC0); `return mat`.
- gta-reversed: CPlantMgr::LoadModels lambda (equivalent). Open questions: none.

## 0x6E5280 lerp (CRenPar)  (catalogue: specified/REA, port level 0)
- gr WaterLevel.h:18 (`static CRenPar lerp(CRenPar rp1, CRenPar rp2, float t)`). Callers: CWaterLevel::RenderHighDetailWaterRectangle (4), RenderFlatWaterRectangle, ...
- Convention: `__cdecl CRenPar* (CRenPar* ret [esp+4] hidden, CRenPar a [esp+8..+0x14] by value (16 bytes), CRenPar b [esp+0x18..+0x24], float t [esp+0x28])`, returns hidden pointer in EAX, plain `ret`. CRenPar = {float z, bigWaves, smallWaves; u8 flags[2]...}.
- Constants: `[0x858624]=1.0f`.
- Behaviour (x87): `u = 1.0f - t` (kept in ST1 throughout); `ret.byte+0xC = 0; ret.byte+0xD = 0`; `ret.z = a.z*u + b.z*t`; `ret.bigWaves = a.bigWaves*u + b.bigWaves*t`; `ret.smallWaves = a.smallWaves*u + b.smallWaves*t` (each: product with u first, then b*t, faddp). Bytes +0xE/+0xF of ret are not written.
- gta-reversed: gr uses `lerp(a,b,t) = a + (b-a)*t`-style helpers per component (not the `a*(1-t)+b*t` form); port must use the bytes' form. Open questions: none.

## 0x6E6870 CWaterLevel::RenderSeaBedSegment (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: gr WaterLevel.h:341 `RenderSeaBedSegment(int32, int32, float, float, float, float)` (unhooked); only caller CWaterLevel::RenderWater 0x6EF650 (2 sites); writes one sea-bed quad (z = -70, grey) into the water temp buffers.
- Convention: `__cdecl void (int32 x [esp+4], int32 y [esp+8], float u0 [esp+0xC], float u1 [esp+0x10], float v0 [esp+0x14], float v1 [esp+0x18])`, plain `ret`. The argument slots are reused as scratch.
- Globals: uiTempBufferVerticesStored 0xC4B950 (n), uiTempBufferIndicesStored 0xC4B954 (m), TempVertexBuffer 0xC4D958 (RwIm3DVertex, stride 0x24: +0 x, +4 y, +8 z, +0x18 colour, +0x1C u, +0x20 v), aTempBufferIndices 0xC4B958 (u16).
- Constants: `8.0f [0x859000]`, `500.0f [0x858B58]`, `3000.0f [0x859A94]`; immediates z = -70.0f (0xC28C0000), colour 0xFF505050.
- Behaviour (each coordinate: `X(u) = ((float)x + u) * 500.0f - 3000.0f`, `Y(v) = ((float)y + v) * 500.0f - 3000.0f`, i.e. fild, fadd, fmul, fsub; uv = value * 8.0f): vertices n..n+3 = V0 (X(u0), Y(v0)) uv (u0*8, v0*8); V1 (X(u0), Y(v1)) uv (u0*8, v1*8); V2 (X(u1), Y(v0)) uv (u1*8, v0*8); V3 (X(u1), Y(v1)) uv (u1*8, v1*8); all z = -70.0f, colour 0xFF505050; normals (+0xC..+0x17) not written. Indices at m: n, n+1, n+2, n+3, n+1, n+2. Then `n += 4; m += 6`. No overflow check against the buffer sizes.
- Open questions: none.

## 0x6EA8A0 CWaterLevel::GetGroundLevel (proposed)  (catalogue: unknown/REA, port level 8)
- Name evidence: gr WaterLevel.h:320 `GetGroundLevel(const CVector&, float*, ColData*, float)` (unhooked); only caller CWaterLevel::GetWaterDepth 0x6EA960.
- Convention: `__cdecl bool (const CVector* pos [esp+4], float* outZ [esp+8], ColData* outData [esp+0xC] (nullable), float range [esp+0x10])`, result AL, plain `ret`.
- Behaviour: `p = (pos.x, pos.y, pos.z + range)`; `hit = CWorld::ProcessVerticalLine(p, -range, colPoint, &outEntity /* range arg slot reused */, 1 /*buildings*/, 0 /*vehicles*/, 0 /*peds*/, 0 /*objects*/, 1 /*dummies*/, 0, 0)` (0x5674E0; 11 args, popped 0x2C). If hit: `*outZ = colPoint.m_vecPoint.z`; `if (outData) { outData[0] = colPoint byte +0x23 (m_nSurfaceTypeB); outData[1] = colPoint byte +0x24 (m_nPieceTypeB); }` return 1. Else return 0 (outputs untouched).
- Edge: the line's lower end is the absolute z = -range (not pos.z - range). Open questions: none.

## 0x6FE220 CCustomRoadsignMgr::SetMaterialAlphaCB (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: material callback referenced by CCustomRoadsignMgr::SetAtomicAlpha 0x6FE240 and RenderRoadsignAtomic 0x6FF350 (2).
- Convention: `__cdecl RpMaterial* (RpMaterial* mat [esp+4], void* data [esp+8])`; data is the alpha value passed by value in the pointer slot (low byte used).
- Behaviour: `c = mat->color (dword +4); c.alpha (byte 3) = (u8)data; mat->color = c` (done through the arg slot as scratch); returns mat (EAX unchanged). Open questions: none.

## 0x706770 CRealTimeShadowManager::CRealTimeShadowManager (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller __init_g_realTimeShadowMan 0x854980; gr RealTimeShadowManager.h: `bool m_bInitialised; bool m_bNotReInitialised; CRealTimeShadow* m_apShadows[40]? ...` (layout below is what the bytes write).
- Convention: `__thiscall T* (this = ECX)`, returns this.
- Behaviour: dwords +0x44, +0x48, +0x4C, +0x50 = 0; byte +0 = 0; byte +1 = 0; `rep stosd` 16 dwords of 0 at +4..+0x43. Net: bytes 0..1 and 4..0x53 zero; bytes 2..3 untouched.
- Open question: gr declares 40 shadow pointers; the bytes clear only 16 (+4..+0x43) plus four camera pointers +0x44..+0x50 - verify gr layout against 0x70678E.

## 0x71D780 CMBlur::ClearDrunkBlur  (catalogue: specified/REA, port level 0)
- gr MBlur.cpp:49. Convention: `__cdecl void ()`. Callers: CGameLogic::RestorePlayerStuffDuringResurrection, scripts 1300-1399, PED (2).
- Body: 5 `nop`s then `jmp 0x401190` (.HOODLUM stub): stores 1.0f (bits 0x3F800000) into CTimer::ms_fTimeScale (global 0xB7CB64), then computes the resume address 0x71D78A (adc of 0x1C7A02A1 and 0xE3F7D4E9 with CF=0, truncated) and returns into it; 0x71D78A: `CMBlur::Drunkness [0xC73C58] = 0` (dword store); `ret`.
- Net behaviour: `ms_fTimeScale := one; Drunkness := zero` (gr: ResetTimeScale(); Drunkness = 0). Open questions: none.

## 0x7204C0 CRubbish::SetVisibility  (catalogue: specified/REA, port level 0)
- gr Rubbish.cpp:4. Empty function: single `ret` (cdecl `void (bool)`; caller pops). Callers CEntryExit::TransitionFinished, scripts 900-999, ANIM (2). Open questions: none.

## 0x722220 MarkerAtomicCB  (catalogue: specified/REA, port level 0)
- RpClumpForAllAtomics callback used by C3dMarker::AddMarker 0x722230 (2 refs). `__cdecl RpAtomic* (RpAtomic* atomic [esp+4], void* data [esp+8])`.
- Behaviour: `*(RpAtomic**)data = atomic; return atomic;` (EAX = atomic: iteration continues, so the last atomic wins). Open questions: none.
