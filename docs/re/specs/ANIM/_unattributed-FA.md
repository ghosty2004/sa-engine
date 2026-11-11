# _unattributed-FA (ANIM) - RE-SPEC-FINAL group A

## 0x4CEF30 CAnimBlendHierarchy-like dtor (vtable 0x85c6cc)  (catalogue: unknown/REA, port level 1)
Name correction: vtable 0x85C6CC is CAnimBlendStaticAssociation (catalog/data-tables.csv, gr RH_ScopedVirtualClass) -> proposed
`CAnimBlendStaticAssociation::scalar deleting destructor` (inlined dtor body). No code callers (virtual slot 0).
- Convention: `__thiscall CAnimBlendStaticAssociation* (this, uint8 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `vtable = 0x85C6CC`; if `this->+0xC` (m_pNodeArray) != 0 `CMemoryMgr::Free(+0xC)` 0x72F430 (pointer not zeroed); if `flags & 1`
  `operator delete(this)` 0x8214BD.
- gta-reversed: CAnimBlendStaticAssociation dtor frees the node array - same. Open questions: none.

## 0x4CFA90 compressed rotation decode (i16 * 1/4096)  (catalogue: unknown/REA, port level 0)
Catalogue name: <none>, proposed: `KeyFrameTransCompressed::GetRotation`-style static helper `DecompressRotation(CQuaternion* out, const int16 q[4])`
(evidence: callers CAnimBlendNode::CalcDeltasCompressed 0x4D0350 and UpdateCompressed 0x4D08D0; scale 1/4096 is the compressed quaternion scale).
- Convention: `__cdecl void (CQuaternion* out [esp+4], const int16* in [esp+8])`, plain `ret`. The `in` argument slot is overwritten and used as
  the 32-bit `fild` scratch.
- Behaviour: for k = 0..3 in order: `out[k] = (float)(i32)(i16)in[k] * [0x858BFC]=0.000244140625f` (`movsx`, `fild dword`, `fmul`, `fstp`
  to f32). Exact (power-of-two scale).
- gta-reversed: inline in the AnimBlendSequence headers (not hooked). Open questions: none.

## 0x4CFB10 compressed translation decode (i16 * 1/1024)  (catalogue: unknown/REA, port level 0)
Catalogue name: <none>, proposed: static helper `DecompressTranslation(CVector* out, const int16 t[3])` (callers CAnimBlendNode::
GetCurrentTranslationCompressed 0x4CFE60, GetEndTranslationCompressed 0x4D0000, UpdateCompressed 0x4D08D0).
- Convention: `__cdecl void (CVector* out [esp+4], const int16* in [esp+8])`, plain `ret`; `in` arg slot reused as `fild` scratch.
- Behaviour: for k = 0..2: `out[k] = (float)(i16)in[k] * [0x85C6D4]=0.0009765625f`.
- gta-reversed: inline (not hooked). Open questions: none.

## 0x4D40A0 member buffer free (sub_4d40a0)  (catalogue: unknown/REA, port level 1)
Catalogue name: <none>; owner UNCLEAR (no code callers). `__thiscall void (this)`: `operator delete(this->+0x30)` 0x8214BD; `+0x30 = 0`.
(Same shape as 0x4161D0 but with operator delete instead of free.) gta-reversed: n/a. Open questions: owner class.

## 0x4D6240 ForAllFramesF  (catalogue: specified/REA, port level 1)
Proposed: RpAnimBlendClumpFindFrame skinned-clump callback `static void FindFrameSkinnedCB(AnimBlendFrameData* fd, void* name)` (address taken in
RpAnimBlendClumpFindFrame 0x4D62A0).
- Convention: `__cdecl void (AnimBlendFrameData* fd [esp+4], const char* name [esp+8])`, plain `ret` (no return value).
- Behaviour: `n = ConvertBoneTag2BoneName(fd->m_nNodeId (+0x14))` 0x4D56F0; if `n != 0` and `_stricmp(name, n) == 0` (CRT 0x8229B6,
  case-insensitive) store `fd` into the global result [0xB5F87C] (gr RpAnimBlendClumpFindFrame's static "found" pointer). No early exit.
- gta-reversed: RpAnimBlend.cpp lambda in RpAnimBlendClumpFindFrame - same. Open questions: none.

## 0x4D6270 RpAnimBlendClumpFindFrame non-skinned lambda  (catalogue: unknown/REA, port level 1)
- Convention: `__cdecl void (AnimBlendFrameData* fd [esp+4], const char* name [esp+8])`, plain `ret`.
- Behaviour: `n = NodeNamePlugin::GetFrameNodeName(fd->m_pFrame (+0x10))` 0x72FB30; if `_stricmp(name, n) == 0` (0x8229B6; n is NOT null-checked)
  `[0xB5F87C] = fd`.
- gta-reversed: RpAnimBlend.cpp - same. Open questions: none.

## 0x4D6430 ForAllFramesF  (catalogue: specified/REA, port level 1)
Proposed: RpAnimBlendClumpFillFrameArray callback `static void FillFrameArrayCB(AnimBlendFrameData* fd, void* array)` (address taken in 0x4D64A0).
- Convention: `__cdecl void (AnimBlendFrameData* fd [esp+4], AnimBlendFrameData** arr [esp+8])`, plain `ret`.
- Behaviour: `arr[CVisibilityPlugins::GetFrameHierarchyId(fd->m_pFrame (+0x10))] = fd` (0x732A20, no bounds check).
- gta-reversed: RpAnimBlend.cpp - same. Open questions: none.

## 0x72F500 sub_72F500 (MoveMemory no-op)  (catalogue: unknown/REA, port level 0)
- `__cdecl void* (void* p [esp+4])`: returns its argument (EAX = p), plain `ret`. Called by CAnimBlendHierarchy::MoveMemory 0x4CF800 and
  CAnimBlendSequence::MoveMemory 0x4D1150 (the memory-move hook is a no-op in this build). Open questions: none.
