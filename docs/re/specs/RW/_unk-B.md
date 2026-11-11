# RW: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x5A47E0 CClothesBuilder::FindAtomicFromNameCB (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: gr ClothesBuilder.cpp:103 (lambda passed to RpClumpForAllAtomics, comment 0x5A47E0); referenced by CClothesBuilder::GetAtomicWithName 0x5A4810, StoreBoneArray 0x5A48B0, BlendGeometry (3).
- Convention: `__cdecl RpAtomic* (RpAtomic* atomic [esp+4], void* data [esp+8])`; data points at `{ const char* name; RpAtomic* found; }`.
- Callees: NodeNamePlugin::GetFrameNodeName 0x72FB30 (`(RwFrame*)`, on `atomic->object.object.parent` = atomic+4), CRT `_stricmp` 0x8229B6.
- Behaviour: `if (_stricmp(data->name, GetFrameNodeName(atomic->parent)) == 0) { data->found = atomic; return nullptr; /* EAX keeps stricmp's 0 */ } return atomic;` (return value at the match is EAX = 0 from stricmp, i.e. stop iteration).
- Open questions: none.

## 0x5D5B30 CCarFXRenderer::CustomCarPipeAtomicSetupCB (proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: RpClumpForAllAtomics callback referenced only by CCarFXRenderer::CustomCarPipeClumpSetup 0x5D5B40.
- Convention: `__cdecl RpAtomic* (RpAtomic* atomic [esp+4], void* data)`. Behaviour: `return CCustomCarEnvMapPipeline::CustomPipeAtomicSetup(atomic)` (0x5DA610; its EAX is passed through as the CB result). Open questions: none.

## 0x5D6830 CCustomBuildingRenderer::sub_5D6830 (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller CCustomBuildingRenderer::Update 0x5D8050. Globals: s_Magic1 0xC02C14 (int32, read+written), s_Magic2 0xC02C18 (written).
- Convention: `__cdecl void (u32 value [esp+4])`. Behaviour: `s_Magic1 = (s_Magic1 + 1) & 0xF; s_Magic2 = value`.
- Open question: purpose / gr name (gr inlines this in CCustomBuildingRenderer::Update).

## 0x5D72A0 CCustomBuildingDNPipeline::PreRenderUpdateCB (proposed)  (catalogue: unknown/REA, port level 10)
- gr CustomBuildingDNPipeline.cpp:151 (lambda, comment 0x5D72A0) passed to RpClumpForAllAtomics by PreRenderUpdate(RpClump*) 0x5D72C0.
- Convention: `__cdecl RpAtomic* (RpAtomic* atomic [esp+4], void* data [esp+8])`. Behaviour: `PreRenderUpdate(atomic, *(bool*)data /* dword read */)` (0x5D7200, cdecl 2 args); `return atomic`. Open questions: none.

## 0x5D84C0 CCustomCarEnvMapPipeline::CustomPipeRenderCB_CalculatePos (proposed)  (catalogue: unknown/REA, port level 4)
- Name evidence: gr CustomCarEnvMapPipeline.cpp:298 (`const auto CalculatePos = [](float p, float s) { // @ 0x5D84C0`); only caller CCustomCarEnvMapPipeline::CustomPipeRenderCB 0x5D9900.
- Convention: register arguments, custom: EAX = CustomEnvMapPipeMaterialData* (i8 TranslationScale.x at +2, .y at +3), ECX = RpAtomic*, ESI = float out[2]; plain `ret`, returns EAX = 1. Callee-preserved: ESI.
- Callees: RwFrameGetLTM 0x7F0990, CRT _ftol 0x821B40 (truncation). Constants: `0.125f [0x858C48]`, `50.0f [0x858B40]`, `1.0f [0x858624]`.
- Behaviour (x87 order): `sx = (float)tx * 0.125f * 50.0f; sy = (float)ty * 0.125f * 50.0f` (tx/ty sign-extended bytes); `frame = atomic->clump (+0x3C) ? atomic->clump->parentFrame (+4) : atomic->parentFrame (+4)`; `p = RwFrameGetLTM(frame)->pos` (+0x30). Then `ix = 1.0f / sx; n = _ftol(p.x * ix); out[0] = -((p.x - (float)n * sx) * ix)`; `iy = 1.0f / sy; m = _ftol(p.y * iy); out[1] = -((p.y - (float)m * sy) * iy)` (the reciprocal is computed first and kept in ST1; n*sx uses fild then fmul by sx, fsubr from p).
- Edge: tx or ty == 0 gives a division by zero (inf), _ftol of inf/NaN returns 0x80000000 (integer indefinite) and the output becomes NaN. Open questions: none.

## 0x5D8980 IsEnvironmentMappingSupported  (catalogue: specified/REA, port level 1)
- gr CustomCarEnvMapPipeline.cpp:89. `__cdecl bool ()`, result in EAX (0/1). Only caller CCustomCarEnvMapPipeline::CreatePipe 0x5DA020.
- Callee: RwD3D9GetCaps 0x7FAD20 (returns const D3DCAPS9*).
- Behaviour: `caps = RwD3D9GetCaps()`; `if (caps->MaxTextureBlendStages /* +0x94 */ <= 1) return 0` (signed); `okStages = caps->MaxSimultaneousTextures /* +0x98 */ >= 2` (unsigned compare via sbb); `cubeTex = (caps->TextureOpCaps /* +0x90 */ >> 13) & 1`; `blendTex = (TextureOpCaps >> 24) & 1`; `if (!okStages || !cubeTex || !blendTex) return 0`; `return (caps->TextureCaps? /* +0x38 */ >> 9) & 1` (low byte of the shifted value, bit 0).
- Field names are D3DCAPS9 offsets as read; check gr for the exact flags (D3DTEXOPCAPS_BLENDFACTORALPHA bit 13? / D3DTEXOPCAPS_MULTIPLYADD bit 24, +0x38 bit 9). Open question: confirm the D3DCAPS9 member names for +0x38/+0x90/+0x94/+0x98 (offsets are certain).

## 0x5D8BE0 CustomEnvMapPipeMaterialData::FromBuffer  (catalogue: specified/REA, port level 0)
- gr CustomCarEnvMapPipeline.h:33. Only caller CCustomCarEnvMapPipeline::pluginEnvMatStreamReadCB 0x5D9660.
- Convention: `__cdecl void (const EnvMapPipeMaterialDataBuffer* src [esp+4], CustomEnvMapPipeMaterialData* dst [esp+8])`, plain `ret`; EAX on return = src->Texture (last load), not dst.
- Callee: CRT `_ftol` 0x821B40 (truncation toward zero; result low byte stored).
- Constants: `8.0f [0x859000]`, `255.0f [0x859AAC]`.
- Behaviour (in order): `dst[0] = (i8)_ftol(src.Scale.x * 8.0f)`; `dst[1] = _ftol(src.Scale.y * 8.0f)`; `dst[2] = _ftol(src.TranslationScale.x * 8.0f)`; `dst[3] = _ftol(src.TranslationScale.y * 8.0f)`; `dst[4] = _ftol(src.Shininess * 255.0f)`; `dst+8 = src.Texture` (dword +0x14); `word dst+6 = 0`. Byte +5 is not written. Values outside the byte range wrap (low 8 bits of the int).
- gta-reversed: FixedFloat conversion; port must truncate with _ftol and keep the low byte. Open questions: none.

## 0x5D8F70 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8214BD, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5D8FC0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8214BD, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5D9010 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8214BD, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5D9060 CPool<CustomEnvMapPipeMaterialData>::CPool (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller CCustomCarEnvMapPipeline::CreatePipe 0x5DA020; allocates `n*12` objects and `n` flag bytes = gr `CPool<T>::CPool(int32 size)` (Pool.h) with sizeof(T) = 12 (gr CustomEnvMapPipeMaterialData). Byte-identical to 0x5D91B0 (the second 12-byte pool, CustomEnvMapPipeAtomicData).
- Convention: `__thiscall CPool* (this = ECX, int32 size [esp+4], const char* name [esp+8] (unused))`, `ret 8`, returns this.
- Callees: CRT operator new 0x82119A (twice, cdecl, popped together at 0x5D907F).
- Behaviour: `m_pObjects = new(size*12)` (lea/shl: size*3*4); `m_byteMap = new(size)`; `m_bOwnsAllocations (byte +0x10) = 1`; `m_nSize = size`; `m_nFirstFree = -1`; `for (i = 0; i < size; i++) { map[i] |= 0x80; map[i] &= 0x80; }` (free flag set, id cleared; loop skipped when size <= 0, signed). No null check on the allocations.
- gta-reversed: matches gr CPool ctor. Open questions: none.

## 0x5D90C0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8214BD, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5D9110 CPool<CustomEnvMapPipeMaterialData>::New (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: CPool::New shape returning `objs + idx*12`; callers CCustomBuildingDNPipeline::SetFxEnvTexture 0x5D9570, CCustomCarEnvMapPipeline::pluginEnvMatCopyConstructorCB 0x5D9600, pluginEnvMatStreamReadCB 0x5D9660, ... (4).
- Convention: `__thiscall T* (CPool* this = ECX)`, plain `ret`, EAX = slot or null.
- Behaviour: identical to CPool<CQuadTreeNode>::New 0x552B50 with element size 12 (`lea eax,[eax+eax*2]; lea eax,[ecx+eax*4]`). The wrap-around store `m_nFirstFree = 0` is relocated to .HOODLUM stub 0x401F94 (plain `mov [ecx+0xC],0; jmp 0x5D912D`); flags from the `test al,al` at 0x5D9124 survive the jump, so the logic is unchanged: on the second wrap return null with m_nFirstFree = 0.
- Open questions: none.

## 0x5D9180 CPool<CustomEnvMapPipeMaterialData>::Delete (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: CPool::Delete shape with element size 12; only caller CCustomCarEnvMapPipeline::SetFxEnvTexture 0x5DA230 (frees material data back into the env-map pool).
- Convention: `__thiscall void (CPool* this = ECX, void* p [esp+4])`, `ret 4`.
- Behaviour: `idx = (p - m_pObjects) / 12` (signed magic imul 0x2AAAAAAB, sar 1, add sign); `m_byteMap[idx] |= 0x80`; `if (idx < m_nFirstFree) m_nFirstFree = idx`. Open questions: none.

## 0x5D91B0 CPool<CustomEnvMapPipeAtomicData>::CPool (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller CCustomCarEnvMapPipeline::CreatePipe 0x5DA020; allocates `n*12` objects and `n` flag bytes = gr `CPool<T>::CPool(int32 size)` (Pool.h) with sizeof(T) = 12 (gr CustomEnvMapPipeMaterialData). Byte-identical to 0x5D9060 (the material-data pool); this one builds the atomic-data pool (12-byte CustomEnvMapPipeAtomicData, gr).
- Convention: `__thiscall CPool* (this = ECX, int32 size [esp+4], const char* name [esp+8] (unused))`, `ret 8`, returns this.
- Callees: CRT operator new 0x82119A (twice, cdecl, popped together at 0x5D91CF).
- Behaviour: `m_pObjects = new(size*12)` (lea/shl: size*3*4); `m_byteMap = new(size)`; `m_bOwnsAllocations (byte +0x10) = 1`; `m_nSize = size`; `m_nFirstFree = -1`; `for (i = 0; i < size; i++) { map[i] |= 0x80; map[i] &= 0x80; }` (free flag set, id cleared; loop skipped when size <= 0, signed). No null check on the allocations.
- gta-reversed: matches gr CPool ctor. Open questions: none.

## 0x5D9210 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8214BD, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5D9260 CPool<CustomEnvMapPipeAtomicData>::New (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: byte-identical to CPtrNodeDoubleLinkPool::New 0x5522E0 (CPool::New, element size 12); callers CCustomCarEnvMapPipeline::AllocEnvMapPipeAtomicData 0x5D96F0, pluginEnvAtmCopyConstructorCB 0x5D9780, CustomPipeRenderCB 0x5D9900.
- Convention / behaviour: as 0x552B50 (`__thiscall T* (CPool*)`), element size 12, no .HOODLUM relocation. Open questions: none.

## 0x5D9300 CPool<CustomSpecMapPipeMaterialData>::CPool (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller CCustomCarEnvMapPipeline::CreatePipe; same shape as 0x5D9060 with element size 8 (gr CustomSpecMapPipeMaterialData = float + RwTexture*).
- Convention: `__thiscall CPool* (this = ECX, int32 size [esp+4], const char* name [esp+8] unused)`, `ret 8`, returns this.
- Behaviour: `m_pObjects = new(size*8)`; `m_byteMap = new(size)`; `byte +0x10 = 1`; `m_nSize = size`; `m_nFirstFree = -1`; for each i < size: `map[i] |= 0x80; map[i] &= 0x80`. Open questions: none.

## 0x5D9360 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8214BD, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5D93B0 CPool<CustomSpecMapPipeMaterialData>::New (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: byte-identical to CPtrNodeSingleLinkPool::New 0x552240 (CPool::New, element size 8); callers pluginSpecMatCopyConstructorCB 0x5D9830, pluginSpecMatStreamReadCB 0x5D9880.
- Convention / behaviour: as 0x552B50 with element size 8. Open questions: none.

## 0x5D9450 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x5D94B0 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x5D9510 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x5DA6A0 RxD3D9InstanceDataRenderLighting  (catalogue: specified/REA, port level 2)
- gr PipelinesCommon.hpp:39. Convention: `__cdecl void (RxD3D9ResEntryHeader* header [esp+4], RxD3D9InstanceData* mesh [esp+8], u32 flags [esp+0xC], RwTexture* texture [esp+0x10])`, plain `ret`. Callers: CCustomBuildingDNPipeline::CustomPipeRenderCB, CCustomBuildingPipeline::CustomPipeRenderCB, ... (3).
- Callees: RwD3D9SetTexture 0x7FDE70, RwD3D9SetTextureStageState 0x7FC340 (stage, type, value), _rwD3D9SetPixelShader 0x7F9FF0, _rwD3D9SetVertexShader 0x7F9FB0, _rwD3D9DrawIndexedPrimitive 0x7FA320, _rwD3D9DrawPrimitive 0x7FA360.
- Behaviour: if `flags & 0x84` (rxGEOMETRY_TEXTURED | TEXTURED2): `RwD3D9SetTexture(texture, 0)`; TSS(0, COLOROP(1), 4 /*MODULATE*/); TSS(0, COLORARG1(2), 2 /*TEXTURE*/); TSS(0, COLORARG2(3), 0 /*DIFFUSE*/); TSS(0, ALPHAOP(4), 4); TSS(0, ALPHAARG1(5), 2); TSS(0, ALPHAARG2(6), 0). Else: `RwD3D9SetTexture(nullptr, 0)`; TSS(0, COLOROP, 3 /*SELECTARG2*/); TSS(0, COLORARG2, 0); TSS(0, ALPHAOP, 3); TSS(0, ALPHAARG2, 0). Then `_rwD3D9SetPixelShader(nullptr)`; `_rwD3D9SetVertexShader(mesh->vertexShader (+0x10))`. If `header->indexBuffer (+8)`: `_rwD3D9DrawIndexedPrimitive(header->primType (+0xC), mesh->baseIndex (+0x14), 0, mesh->numVertices (+0x18), mesh->startIndex (+0x1C), mesh->numPrimitives (+0x20))`; else `_rwD3D9DrawPrimitive(header->primType, mesh->baseIndex (+0x14), mesh->numPrimitives (+0x20))`.
- Open questions: none.

## 0x5DAAC0 CCustomCarEnvMapPipeline::ResetSurfaceProperties (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: only caller CCustomCarEnvMapPipeline::CustomPipeRenderCB 0x5D9900.
- Convention: `__cdecl void ()`. Behaviour: `RwD3D9SetSurfaceProperties(&0x8D131C, &0x8D1328, 0x20)` (0x7FC4D0, cdecl, popped). The two .data blocks are near gSpecIntensity (+0x4C / +0x58): a static RwSurfaceProperties and a static RwRGBA/colour block; 0x20 = flags (rxGEOMETRY_MODULATE).
- Open question: names of the statics at 0x8D131C / 0x8D1328 (gr: a default surface-properties object reset after rendering).

## 0x72F4A0 sub_72F4A0 (empty)  (catalogue: unknown/REA, port level 0)
- Single `ret`; called by cb_RSCOLLISION_RpClump_StreamRead 0x41B1D0 and CVehicleModelInfo::ReduceMaterialsInVehicle 0x4C8BD0 (a compiled-out debug/profiling hook). Callers pop their own args (cdecl). Open question: original name.

## 0x72F4B0 sub_72F4B0 (empty)  (catalogue: unknown/REA, port level 0)
- Single `ret`; same two callers as 0x72F4A0 (paired begin/end hook). Open question: original name.

## 0x72F9C0 NodeNameConstructor  (catalogue: specified/REA, port level 0)
- gr NodeName.cpp:29 (cb_RSNODENAME_RwFrame_Ctor). RwFrame plugin ctor registered by NodeNamePlugin::NodeNamePluginAttach 0x72FAB0.
- Convention: `__cdecl void* (void* frame [esp+4], int32 offset, int32 size)`; returns frame. Globals: gNodeNamePluginOffset 0xC87C5C (int32).
- Behaviour: `if (gNodeNamePluginOffset > 0) ((char*)frame)[gNodeNamePluginOffset] = 0` (signed `jle`; offset 0 or negative skips); return frame. Open questions: none.

## 0x72F9E0 NodeNameDestructor  (catalogue: specified/REA, port level 0)
- gr NodeName.cpp:38. `__cdecl void* (void* frame, int32, int32)`: returns frame (`mov eax,[esp+4]; ret`), no other effect. Open questions: none.

## 0x72F9F0 NodeNameCopy  (catalogue: specified/REA, port level 0)
- gr NodeName.cpp:44. `__cdecl void* (void* dst [esp+4], const void* src [esp+8], int32, int32)`, returns dst.
- Behaviour: `strncpy(dst + gNodeNamePluginOffset, src + gNodeNamePluginOffset, 23)` (CRT 0x821F40; count 0x17; no explicit terminator and no offset > 0 check, unlike the ctor). Open questions: none.

## 0x72FA50 NodeNameStreamRead  (catalogue: specified/REA, port level 2)
- gr NodeName.cpp:51. `__cdecl RwStream* (RwStream* stream [esp+4], int32 length [esp+8], void* frame [esp+0xC], int32, int32)`, returns stream.
- Behaviour: `p = frame + gNodeNamePluginOffset; RwStreamRead(stream, p, length)` (0x7EC9D0; result ignored); `p[length] = 0`; return stream. No length clamp: a chunk longer than the 24-byte plugin slot overflows (original behaviour). Open questions: none.

## 0x72FB50 PipelineConstructor  (catalogue: specified/REA, port level 0)
- gr PipelinePlugin.cpp:28. RpAtomic plugin ctor. `__cdecl void* (void* atomic [esp+4], int32, int32)`, returns atomic. Globals: gPipelinePluginOffset 0x8D6080 (int32, initial -1).
- Behaviour: `if (gPipelinePluginOffset > 0) *(u32*)(atomic + offset) = 0`. Open questions: none.

## 0x72FB70 PipelineCopy  (catalogue: specified/REA, port level 0)
- gr PipelinePlugin.cpp:37. `__cdecl void* (void* dst [esp+4], const void* src [esp+8], int32, int32)`; behaviour `*(u32*)(dst+off) = *(u32*)(src+off)` with off = gPipelinePluginOffset (no > 0 check); returns dst. Open questions: none.

## 0x72FE80 D3DTextureBuffer::Setup  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (D3DTextureBuffer* this = ECX, u32 format [esp+4], u32 width [esp+8], u32 levels [esp+0xC], u32 capacity [esp+0x10])`, `ret 0x10`. Only caller D3DResourceSystem::DestroyTexture 0x730B70 (lazy setup).
- Behaviour: `+0 = arg0; +4 = arg1; +8 = arg2`; `+0x18 = operator new(capacity*4)` (CRT 0x82119A); `+0xC = capacity; +0x14 = 0 (count); +0x10 = 0 (counter)`.
- Open question: gr field names for +0..+8 (the matching keys used by Pop 0x72FF60); the bytes only copy the three args.

## 0x72FF60 D3DTextureBuffer::Pop  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall IDirect3DTexture9* (D3DTextureBuffer* this = ECX, D3DFORMAT format [esp+4], u32 width [esp+8], u32 height [esp+0xC], u32 levels [esp+0x10])`, `ret 0x10`, EAX = texture or null. Only caller D3DResourceSystem::CreateTexture 0x730510 (on TextureBuffer[0]).
- Fields: +0x14 count (int32), +0x18 IDirect3DTexture9** array, +0x10 not touched here.
- Behaviour: `for (i = count; i != 0; ) { tex = arr[i-1]; i--; isSingle = (tex->GetLevelCount() /* vtbl+0x34 */ == 1); if (isSingle != levels) continue; tex->GetLevelDesc(0, &desc) /* vtbl+0x44 */; if (desc.Format == format && desc.Width (+0x18) == width && desc.Height (+0x1C) == height) goto found; } return null;`
- found: `count--; if (count > 0 && count != i) arr[i] = arr[count]; return tex;` (swap-remove; the slot is not cleared).
- Edge: `levels` is compared with the boolean "has exactly one level", so callers pass 0 (full chain) or 1 (CreateTexture clamps to that). Open questions: none.

## 0x72FFF0 D3DTextureBuffer::Push  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall bool (D3DTextureBuffer* this = ECX, IDirect3DTexture9* tex [esp+4])`, `ret 4`, result AL.
- Behaviour: `this->m_nCounter (+0x10)++` (always, before the check); `if (count (+0x14) < capacity (+0xC)) { array (+0x18)[count] = tex; count++; return 1; } return 0;` (signed compare). Open questions: none.

## 0x730020 D3DTextureBuffer::Resize  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (D3DTextureBuffer* this = ECX, u32 newCapacity [esp+4])`, `ret 4`. Caller D3DResourceSystem::DestroyTexture 0x730B70 (2 sites).
- Behaviour: `if (newCapacity == capacity /* +0xC */) return;` `p = operator new(newCapacity*4)` (0x82119A); `for (i = 0; i < count && i < newCapacity; i++) p[i] = arr[i]` (count re-read each iteration); `for (i = newCapacity; i < count; i++) arr[i]->Release()` (vtbl+8; signed compares); `free(arr)` (0x8214BD); `arr = p; capacity = newCapacity`.
- Edge (original bug): `count` (+0x14) is NOT reduced when shrinking, so after a shrink count can exceed capacity and still index released textures in the new (shorter) array. Port must keep or consciously document this. Open questions: none.

## 0x730270 D3DIndexDataBuffer::Pop  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall IDirect3DIndexBuffer9* (D3DIndexDataBuffer* this = ECX, u32 numIndices [esp+4])`, `ret 4`. Only caller D3DResourceSystem::CreateIndexBuffer 0x7306A0. Fields: +0x10 count, +0x14 IDirect3DIndexBuffer9** array.
- Behaviour: `need = numIndices*2` (16-bit indices); `best = -1; bestSize = 0x7FFFFFFF;` `for (i = count; i != 0; ) { ib = arr[--i]; ib->GetDesc(&desc) /* vtbl+0x34 */; if (desc.Size (+0x10) < bestSize && desc.Size >= need) { bestSize = desc.Size; best = i; } }` (unsigned compares; full scan, ties keep the later-scanned = lower index). `if (best == -1) return null;` `ib = arr[best]; count--; if (count > 0 && count != best) arr[best] = arr[count]; return ib;`
- Open questions: none.

## 0x730330 D3DIndexDataBuffer::Resize  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (D3DIndexDataBuffer* this = ECX, u32 newCapacity [esp+4])`, `ret 4`. Caller D3DResourceSystem::DestroyIndexBuffer 0x730D30 (2). Fields: +8 capacity, +0x10 count, +0x14 array.
- Behaviour: identical in structure to D3DTextureBuffer::Resize 0x730020 with these offsets: return if equal; allocate `newCapacity*4`; copy `min(count, newCapacity)` entries; `Release()` (vtbl+8) entries `newCapacity..count-1`; free old; `array = p; capacity = newCapacity`. Count is not reduced on shrink (same original bug). Open questions: none.

## 0x730510 D3DResourceSystem::CreateTexture  (catalogue: specified/REA, port level 1)
- gr D3DResourceSystem.cpp:57 (plugin::Call; gr's 4-arg signature is wrong: the bytes take 5 args).
- Convention: `__cdecl HRESULT (u32 width [esp+4], u32 height [esp+8], u32 levels [esp+0xC], D3DFORMAT format [esp+0x10], IDirect3DTexture9** out [esp+0x14])`, plain `ret`. Callers _rwD3D9NativeTextureRead 0x4CD820, 0x4CB7C0.
- Globals: FreeTextureBufferIndex 0xC87C60 (int32), TextureBuffer array 0xC87C68 (D3DTextureBuffer, stride 0x1C: +0 format, +4 size, +8 levels, +0xC capacity, +0x10 counter, +0x14 count, +0x18 array), _RwD3DDevice 0xC97C28.
- Behaviour: `if (levels > 1) levels = 0` (signed). If `width == height && FreeTextureBufferIndex > 1`: `w = (float)width` (fild/fstp into the arg slot); for `i = 1 .. FreeTextureBufferIndex-1`: if `(float)TB[i].size == w` (fild of the int, fcomp; equal when C3 set and not unordered) and `levels == TB[i].levels` and `format == TB[i].format` then: `if (TB[i].count) { TB[i].counter--; TB[i].count--; tex = TB[i].array[TB[i].count]; } else tex = null;` goto store. If no square bucket matched (or not square): `tex = TextureBuffer[0].Pop(format, width, height, levels)` (0x72FF60).
- store: `*out = tex; if (tex) return 0 (D3D_OK);` else `return _RwD3DDevice->CreateTexture(width, height, levels, 0 /*usage*/, format, 1 /*D3DPOOL_MANAGED*/, out, nullptr)` (vtbl +0x5C).
- Edge: the matching bucket path does not fall back to Pop on buffer 0 when empty; it goes straight to device creation. Open questions: none.

## 0x7306A0 D3DResourceSystem::CreateIndexBuffer  (catalogue: specified/REA, port level 1)
- gr D3DResourceSystem.cpp:52. Convention: `__cdecl HRESULT (u32 numIndices [esp+4], u32 format [esp+8] (unused), IDirect3DIndexBuffer9** out [esp+0xC])`, plain `ret`. Only caller RwD3D9IndexBufferCreate 0x4C9970.
- Globals: IndexDataBuffer array 0xC87E48 (stride 0x18: +8 capacity, +0xC counter, +0x10 count, +0x14 array), entry 16 at 0xC87FC8 (general pool), _RwD3DDevice 0xC97C28.
- Behaviour: `bucket = (int32)(numIndices - 1) / 100` (signed magic divide 0x51EB851F, sar 5: truncation toward zero, so 0 -> 0); `rounded = (bucket + 1) * 100`.
  - If `bucket < 16`: `e = &IndexDataBuffer[bucket]; if (e->count) { e->counter--; e->count--; ib = e->array[e->count]; } else ib = null; *out = ib;` if ib: `ib->GetDesc(&localDesc)` (vtbl+0x34; result unused) and return 0.
  - Else: `ib = IndexDataBuffer[16].Pop(rounded)` (0x730270); `*out = ib`; if ib return 0 (no GetDesc on this path).
  - Fallback: `return _RwD3DDevice->CreateIndexBuffer(rounded*2, 8 /*D3DUSAGE_WRITEONLY*/, 0x65 /*D3DFMT_INDEX16*/, 1 /*D3DPOOL_MANAGED*/, out, nullptr)` (vtbl +0x6C).
- Edge: the `format` argument is ignored (always INDEX16); the stray GetDesc call is a no-op for the port. Open questions: none.

## 0x731800 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8214BD, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x731B30 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8214BD, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x731C20 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x733910 CLinkList<CVisibilityPlugins::AlphaObjectInfo>::InsertSorted (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: gr Core/LinkList.h:61 `InsertSorted`; 11 callers (CVisibilityPlugins::InsertEntityIntoUnderwaterList, InsertObjectIntoSortedList, ...). Link = 12-byte data + prev (+0xC) + next (+0x10); list = usedListHead (+0), usedListTail (+0x14), freeListHead (+0x28), freeListTail (+0x3C).
- Convention: `__thiscall CLink* (CLinkList* this = ECX, const T* data [esp+4])`, `ret 4`, EAX = new link or null.
- Behaviour: `i = usedListHead.next; while (i != &usedListTail && i->data.key /* f32 +8 */ < data->key) i = i->next;` (fld i.key; fcomp data.key; continue while C0 set, i.e. strictly less; NaN compares as unordered: C0 set, so a NaN key keeps walking); `link = freeListHead.next; if (link == &freeListTail) return null;` copy 3 data dwords into link; unlink link from the free list (`link->next->prev = link->prev; link->prev->next = link->next`); insert before `i`: `link->next = i->prev->next; i->prev->next->prev = link; link->prev = i->prev; i->prev->next = link` (in that store order, 0x733962..0x733975); return link.
- gta-reversed: gr compares `>=` to break (same as continue on `<`, except NaN: gr breaks never on NaN either). Open questions: none.

## 0x733F40 CLinkList<CVisibilityPlugins::AlphaObjectInfo>::Clear (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: gr LinkList.h:77 `Clear`; callers CVisibilityPlugins::InitAlphaAtomicList (tail) and InitAlphaEntityList (3 refs).
- Convention: `__thiscall void (CLinkList* this = ECX)`, plain `ret`.
- Behaviour: `while ((l = usedListHead.next) != &usedListTail) { l->next->prev = l->prev; l->prev->next = l->next; l->next = freeListHead.next; freeListHead.next->prev = l; l->prev = &freeListHead; freeListHead.next = l; }` (each used link is pushed to the front of the free list). Open questions: none.

## 0x734930 GetFirstTextureCallback  (catalogue: specified/REA, port level 0)
- gr rwtexdict.h:9. `__cdecl RwTexture* (RwTexture* tex [esp+4], void* data [esp+8])`: `*(RwTexture**)data = tex; return nullptr;` (stops after the first texture). Open questions: none.

## 0x734940 GetFirstTexture  (catalogue: specified/REA, port level 1)
- gr rwtexdict.h:6. `__cdecl RwTexture* (RwTexDictionary* txd [esp+4])`. Callers: CRadar::DrawRadarSection, DrawRadarSectionMap, ... (6).
- Behaviour: local `tex = nullptr`; `RwTexDictionaryForAllTextures(txd, GetFirstTextureCallback 0x734930, &tex)` (0x7F3730); return tex. Open questions: none.

## 0x734970 RpClumpGetBoundingSphere_AtomicCB1 (proposed)  (catalogue: unknown/REA, port level 4)
- Name evidence: RpClumpForAllAtomics callback referenced by RpClumpGetBoundingSphere 0x734FC0 (sums atomic bounding-sphere centres).
- Convention: `__cdecl RpAtomic* (RpAtomic* atomic [esp+4], RwV3d* sum [esp+8])`, returns atomic.
- Globals: byte 0x8D60BC (read first; a "use LTM" flag next to CVisibilityPlugins::ms_clumpPluginOffset, initial value 1).
- Behaviour: `clump = atomic->clump (+0x3C)`; `if (atomic byte +0x4C & 2) _rpAtomicResyncInterpolatedSphere(atomic)` (0x7491F0; done in both branches); `m = flag ? RwFrameGetLTM(clump->parentFrame (+4)) : &clump->parentFrame->modelling (+0x10)`; `RwV3dTransformPoints(&c, &atomic->boundingSphere.center (+0x1C), 1, m)` (0x7EDD90); then `sum->x = c.x + sum->x; sum->y = c.y + sum->y; sum->z = c.z + sum->z` (f32, each fld c / fadd sum / fstp).
- Open questions: none (0x734ED0 is the second callback of the same function; not yet specified).

## 0x734A70 RwHelper::GetAnimHierarchyFromFrameCB (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: frame callback referenced by RwHelper::GetAnimHierarchyFromFrame 0x734AB0 (2 refs) and recursively by itself.
- Convention: `__cdecl RwFrame* (RwFrame* frame [esp+4], void* data [esp+8])`.
- Behaviour: `h = RpHAnimFrameGetHierarchy(frame)` (0x7C5160); `if (h) { *(RpHAnimHierarchy**)data = h; return nullptr; }` else `RwFrameForAllChildren(frame, 0x734A70, data)` (0x7F0DC0, recursive) and `return frame`. Open questions: none.

## 0x734E90 RwIm2DRenderPrimitive (proposed)  (catalogue: unknown/REA, port level 0)
- Body: `mov eax, [RwEngineInstance 0xC97B24]; jmp [eax+0x30]` = `RWSRCGLOBAL(dOpenDevice).fpIm2DRenderPrimitive` (dOpenDevice at +0x10 of RwGlobals, fpIm2DRenderPrimitive at +0x20 of RwDevice). Arguments/return pass through unchanged: `__cdecl RwBool (RwPrimitiveType, RwIm2DVertex*, RwInt32)`.
- Callers: CRadar::DrawAreaOnRadar, DrawRadarMask (2), DrawRadarSection, ... (24). Class RW (RenderWare API macro compiled out of line). Open questions: none.

## 0x734EC0 RwIm2DRenderLine (proposed)  (catalogue: unknown/REA, port level 0)
- Body: `mov ecx, [RwEngineInstance 0xC97B24]; jmp [ecx+0x28]` = `dOpenDevice.fpIm2DRenderLine` (RwDevice +0x18). `__cdecl RwBool (RwIm2DVertex*, RwInt32 numVerts, RwInt32 v1, RwInt32 v2)` passed through.
- Callers: CTheScripts::ScriptDebugCircle2D, CLines::ImmediateLine2D, CShadowCamera::MakeGradientRaster. Open questions: none.

## 0x734ED0 RpClumpGetBoundingSphere_AtomicCB2 (proposed)  (catalogue: unknown/REA, port level 4)
- Name evidence: second RpClumpForAllAtomics callback of RpClumpGetBoundingSphere 0x734FC0 (grows the radius around the centre computed with CB1 0x734970).
- Convention: `__cdecl RpAtomic* (RpAtomic* atomic [esp+4], RwSphere* sphere [esp+8])`, returns atomic.
- Behaviour: `clump = atomic->clump (+0x3C)`; `if (atomic byte +0x4C & 2) _rpAtomicResyncInterpolatedSphere(atomic)`; then the flag byte is re-read and the same test/call is repeated inside whichever branch follows (so a resync that leaves bit 1 set is called twice). Centre transform as in CB1: `m = byte[0x8D60BC] ? RwFrameGetLTM(clump->parentFrame) : &clump->parentFrame->modelling`; `RwV3dTransformPoints(&c, &atomic->boundingSphere.center, 1, m)`.
- Radius (x87 order): `dx = c.x - s.x; dy = c.y - s.y; dz = c.z - s.z` (s copied to locals first); `d = sqrt((dz*dz + dy*dy) + dx*dx)`; `r = d + atomic->boundingSphere.radius (+0x28)`; `if (!(r <= sphere->radius)) sphere->radius = r` (fcom; skip when C0 or C3 set, i.e. r < radius, equal, or unordered: NaN never stored).
- Open questions: none.
