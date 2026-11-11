# ANIM: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x4cecf0 CAnimBlendAssociation::~CAnimBlendAssociation (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`.
- `[this] = 0x85c6d0`; `if ([+0x10]) CMemoryMgr::FreeAlign([+0x10])` 0x72f4f0 (blend node array).
- Unlink (CAnimBlendLink at +4 next, +8 prev): `if (prev [+8]) prev->next = next [+4]` (0x4ced10); `if (next) next->prev = prev`; then `[+4] = 0; [+8] = 0`.
- `if ((u16)[+0x2e] >> 14 & 1)` (0x4ced32-0x4ced39: flag 0x4000, "uses anim block") `CAnimManager::RemoveAnimBlockRef(m_pHierarchy [+0x14] -> [+0xc] block id)` 0x4d3fd0.

## 0x4cefa0 CAnimBlendAssociation::scalar deleting destructor (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `~CAnimBlendAssociation()` 0x4cecf0; `if (flags & 1) free(this)` 0x8214bd; EAX = this.

## 0x4d1390 empty function (ret) (proposed)  (catalogue: unknown/REA, port level 0)
- Single `ret` (0x4d1390). Port: no-op. Open question: which inline compiled to nothing (callers' context).

## 0x4d1570 RpAnimBlendNodeUpdateKeyFrames (proposed)  (catalogue: specified/REA, port level 3)
- Convention: custom - `c` in EAX, cdecl stack args `frames` [esp+4], `nFrames` [esp+8]; void.
- `for (it = &c->nodeArrays[+4]; *it; ++it)`: `assoc = (*it)[0].+0x14`; `for (n = 0; n < nFrames; ++n)` (skipped when nFrames <= 0, signed): `if ((frames[n].flags [stride 0x18] & 8) && gpAnimBlendClump [0xb4ea0c]->+0xc) continue`; `node = &(*it)[n]` (stride 0x18); `if (node+0x10 == 0) continue`; `node->FindKeyFrame(assoc->m_fCurrentTime [+0x20])` 0x4d0240 (thiscall, float arg).
- Matches gr.

## 0x4d2e10 FrameUpdateCallBackOffscreen (proposed)  (catalogue: specified/REA, port level 5)
- `__cdecl void (AnimBlendFrameData* fd, void* data)`: `if ((fd->flags & 8) && gpAnimBlendClump [0xb4ea0c]->m_PedPosition [+0xc])` call `FrameUpdateCallBackSkinnedWithVelocityExtraction` 0x4d1680 with EAX = data and stack arg fd (pops it).

## 0x4d56f0 ConvertBoneTag2BoneName (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl const char* (i32 boneTag)`; returns EAX = string pointer or 0.
- `if (tag > 0xc9)` (signed `jg`, 0x4d56f4): 301 -> "R Breast" (0x85ed68), 302 -> "L Breast" (0x85ed74), else 0. `tag == 0xc9` (201) -> "Belly" (0x85ed80). Else `if ((u32)tag > 0x36)` -> 0 (negative tags also 0); else via the tables:
- 0 Root, 1 Pelvis, 2 Spine, 3 Spine1, 4 Neck, 5 Head, 6 L Brow, 7 R Brow, 8 Jaw; 21 Bip01 R Clavicle, 22 R UpperArm, 23 R Forearm, 24 R Hand, 25 R Fingers, 26 R Finger01; 31 Bip01 L Clavicle, 32 L UpperArm, 33 L Forearm, 34 L Hand, 35 L Fingers, 36 L Finger01; 41 L Thigh, 42 L Calf, 43 L Foot, 44 L Toe; 51 R Thigh, 52 R Calf, 53 R Foot, 54 R Toe; all other tags 0..54 -> 0.
- String addresses: Root 0x85ee74, Pelvis 0x85ee7c, Spine 0x85ee84, Spine1 0x85ee8c, Neck 0x85ee94, Head 0x85ee9c, L Brow 0x85ee6c, R Brow 0x85ee64, Jaw 0x85ee60, then 0x85ee4c..0x85ed88 descending in the order listed.

## 0x4d5f20 CAnimBlendClumpData::scalar deleting destructor (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `CAnimBlendClumpData::Destructor 0x4cf100(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4d5f20).

## 0x4d6310 RpAnimBlendClumpFindFrameFromHashKey skinned lambda (ForAllFramesF 0x4D6310) (proposed)  (catalogue: specified/REA, port level 1)
- `__cdecl void (AnimBlendFrameData* fd, void* key)`: `name = ConvertBoneTag2BoneName(fd->BoneTag [+0x14])` 0x4d56f0; `if (name && CKeyGen::GetUppercaseKey(name) 0x53cf30 == (u32)key) [0xb5f87c] = fd` (found-frame global next to ClumpOffset).

## 0x4d6340 RpAnimBlendClumpFindFrameFromHashKey non-skinned lambda (ForAllFramesF 0x4D6340) (proposed)  (catalogue: specified/REA, port level 1)
- `__cdecl void (AnimBlendFrameData* fd, void* key)`: `if (CKeyGen::GetUppercaseKey(NodeNamePlugin::GetFrameNodeName(fd->Frame [+0x10]) 0x72fb30) == (u32)key) [0xb5f87c] = fd`.

## 0x4d63e0 RpAnimBlendClumpFindBone lambda (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl void (AnimBlendFrameData* fd, void* boneTag)`: `if ((u32)boneTag == fd->BoneTag [+0x14]) [0xb5f87c] = fd`.

## 0x4d6450 FillFrameArrayIndiciesSkinned (proposed)  (catalogue: unknown/REA, port level 2)
- Convention: EAX = clump (register), cdecl stack arg `AnimBlendFrameData** frames` [esp+4]; void.
- `bd = *(clump + [ClumpOffset 0xb5f878])`; `h = RwHelper::GetAnimHierarchyFromSkinClump(clump)` 0x734a40; `for (i = 1; i < 0x13; ++i) frames[i] = bd->m_FrameDatas [+0x10] + RpHAnimIDGetIndex(h, ConvertPedNode2BoneTag(i)) * 0x18` (0x4d6471-0x4d6492). frames[0] untouched; no -1 index check.

## 0x4d6500 RpAnimBlendClumpInitSkinned frame-init lambda (FrameInitCBskin) (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl void (AnimBlendFrameData* fd, void*)`: `byte[fd] = 0` (flags).

## 0x4d6510 RpAnimBlendClumpInitSkinned (proposed)  (catalogue: specified/REA, port level 3)
- Convention: EAX = clump (register arg), void; frame aligned to 8, 0x314 bytes of locals.
- `RpAnimBlendAllocateData(clump)` 0x4d5f50 (gr moved this to RpAnimBlendClumpInit); `bd = *(clump + [ClumpOffset 0xb5f878])`.
- `n = RpSkinGetNumBones(RpSkinGeometryGetSkin(RwHelper::GetFirstAtomic(clump)->geometry [+0x18]))` (0x734820, 0x7c7550, 0x7c77e0); `bd->SetNumberOfBones(n)` 0x4cf140; `h = RwHelper::GetAnimHierarchyFromSkinClump(clump)` 0x734a40.
- `pos[64]` (CVector, 0x300 bytes) zero-filled (`[esp+0x24] = 0` + `rep stosd` 0xbf dwords, 0x4d6564-0x4d6578); `SkinGetBonePositionsToTable(clump, pos)` 0x735360 (gr inlines this).
- `for (i = 0; i < n; ++i)` (signed, skipped if n <= 0): `fd = &bd->m_FrameDatas[i]` (stride 0x18); `fd->KeyFrame (+0x10) = h->currentAnim [+0x20] + 0x4c + h->currentAnim->interpFrameSize [+0x24] * i`; `fd->BoneTag (+0x14) = h->pNodeInfo [+0x10][i].nodeID` (stride 0x10); `fd->BonePos (+4..+0xc) = pos[i]` (integer copies).
- `bd->ForAllFrames(0x4d6500, 0)` 0x4cf190 (flags = 0); then `byte[bd->m_FrameDatas[0]] |= 8` (0x4d6606), same as the non-skinned init 0x4d66a0.

## 0x4d6610 frame count callback (RwFrameForAllChildren, ++*count) (proposed)  (catalogue: unknown/REA, port level 1)
- `__cdecl RwFrame* (RwFrame* f, void* data)`: `++*(i32*)data` (0x4d661c/0x4d6623); `RwFrameForAllChildren(f, 0x4d6610, data)` 0x7f0dc0; return f. (gr replaces it with RwFrameCount - 1.)

## 0x4d6640 RpAnimBlendClumpInitNotSkinned frame-init lambda (FrameInitCBnonskin) (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl void (AnimBlendFrameData* fd, void*)`: `byte[fd] = 0`; `fd->FramePos (+4..+0xc) = RwFrameGetMatrix(fd->Frame [+0x10])->pos` (frame +0x40..+0x48, integer copy); `fd->BoneTag [+0x14] = -1`.

## 0x4d6670 SetFrameDataFrameCB (proposed)  (catalogue: unknown/REA, port level 1)
- `__cdecl RwFrame* (RwFrame* f, void* data)`: `(*(fd**)data)->Frame [+0x10] = f`; `*(u8**)data += 0x18`; `RwFrameForAllChildren(f, 0x4d6670, data)`; return f.

## 0x4d66a0 RpAnimBlendClumpInitNotSkinned (proposed)  (catalogue: unknown/REA, port level 2)
- Convention: EAX = clump, void.
- `RpAnimBlendAllocateData(clump)` 0x4d5f50; `bd = *(clump + [0xb5f878])`; `count = 0; RwFrameForAllChildren(clump->frame [+4], 0x4d6610, &count)`; `bd->SetNumberOfBones(count)` 0x4cf140; `fd = bd->m_FrameDatas [+0x10]; RwFrameForAllChildren(frame, 0x4d6670, &fd)`; `bd->ForAllFrames(0x4d6640, 0)` 0x4cf190; `byte[bd->m_FrameDatas[0]] |= 8` (0x4d670c: first frame gets the velocity-extraction flag).
- gta-reversed RpAnimBlendClumpInitNonSkinned omits the final `|= 8` in the shown code (open question: check gr RpAnimBlendClumpInit, 0x4d670c).
