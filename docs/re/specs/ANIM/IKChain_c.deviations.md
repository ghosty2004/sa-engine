# IKChain_c: gta-reversed deviation notes (ANIM)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x6178B0 IKChain_c::MoveBonesToTarget  (gr_deviations = 1)
Row IKChain.cpp:259 (`bone->Limit(m_Blend); // Originally this was in an if, but the variable that was checked was always true`).
- Original bytes: per bone, after `RtQuatRotate(&bone->m_Orientation, &axis, angle, 2)` (0x617BBD): `if (byte [0x8D2BD2] != 0)
  bone->Limit(m_Blend (+0x18))` (0x617BC2-0x617BD4); then `bone->CalcWldMat(bone->m_Parent (+0x2C) ? &parent->m_WorldMat (+0x3C)
  : m_PivotBoneMatrix (+0x14))` (0x616CD0). The flag byte 0x8D2BD2 (next to s_TestSkipClampDefault/Current at 0x8D2BD0/1) is 1 in the
  shipped .data and no code writes it.
- Verdict: REFUTED (with the shipped data the call always happens; same as gr).
- Class: harmless (a port may still read the global to stay exact).

## 0x617CA0 IKChain_c::SetupBones  (gr_deviations = 2)
Original bytes (`__thiscall void(eBoneTag32 effector [esp+4], CVector effectorPos [esp+8..0x10] (unused), eBoneTag32 pivot [esp+0x14],
AnimBlendFrameData* frames [esp+0x18])`, `ret 0x18`; locals: `BoneNode_c* bones[32]` at esp+0x10, no overflow check):
  1. `m_BonesCount (+0xC) = 0`; `h = GetAnimHierarchyFromSkinClump(m_Ped (+0x8)->m_pRwClump (+0x18))` (0x734A40);
     `m_PivotBoneMatrix (+0x14) = RpHAnimHierarchyGetMatrixArray(h) + RpHAnimIDGetIndex(h, pivot) * 64`; `m_PivotBone (+0x2C, w) = pivot`
     (0x617CAB-0x617CF4).
  2. `tag = effector`; while `tag != pivot` (only this test, 0x617CF2/0x617D53): `node = g_boneNodeMan.GetBoneNode()` (0x617460);
     `node->Init(tag, frames[RpHAnimIDGetIndex(h, tag)].KeyFrame (+0x10 of the 0x18-byte entry))` (0x6177B0);
     `bones[m_BonesCount++] = node`; `tag = ms_boneInfos[GetIdFromBoneTag(tag)].ParentBoneTag` (int16 at 0x8D26D2 + id*0x28,
     sign-extended; GetIdFromBoneTag 0x617050 returns -1 for an unknown tag, so the read is at 0x8D26AA = int16 0).
  3. `m_Bones (+0x10) = operator new(m_BonesCount*4)` (0x821195); copy bones[0..count).
  4. For each i: `p = ms_boneInfos[GetIdFromBoneTag(m_Bones[i]->m_BoneTag (+0x8))].ParentBoneTag`; if `p > -1` (signed): first j with
     `m_Bones[j]->m_BoneTag == p`; if found and non-null: `m_Bones[j]->AddChild(m_Bones[i])` (0x616BD0) (0x617D84-0x617DEC).
Row IKChain.cpp:275 (BUGFIX: info null -> BONE_UNKNOWN):
- Verdict: CONFIRMED - the original never gets null: an unknown tag indexes entry -1 and reads ParentBoneTag = 0 (BONE_ROOT) from the
  bytes preceding the table; gr returns BONE_UNKNOWN (after NOTSA_UNREACHABLE in GetBoneInfoFromTag). Class: keep-original.
Row IKChain.cpp:281 (BUGFIX: stop the walk at BONE_UNKNOWN):
- Verdict: CONFIRMED - the original only stops at `pivot`. If pivot is not an ancestor of effector the walk goes root (0) -> parent -1
  -> entry -1 -> 0 -> -1 ... without end and overruns the 32-entry stack array (original bug; callers always pass a valid chain).
  gr stops at -1. Class: keep-original (valid inputs behave the same).
