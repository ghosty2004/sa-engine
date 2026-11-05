# CAnimBlendSequence: gta-reversed deviation notes (ANIM)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x4D0C50 CAnimBlendSequence::SetName  (gr_deviations = 1)
Row AnimBlendSequence.cpp:126 (bugfix `AnimBlendSequence_SetName_SetBoneTagFlag`: gr clears m_IsUsingBoneTag).
- Original bytes (`__thiscall void(const char* name [esp+4])`, `ret 4`): `this->m_NameHashKey (+0x0) = CKeyGen::GetUppercaseKey(name)`
  (0x53CF30, cdecl) (0x4D0C50-0x4D0C60). The flags word at +0x4 is not touched.
- Verdict: CONFIRMED (with the bugfix enabled gr clears bit 4 of +0x4; the original never does).
- Class: keep-original (do not touch the flags).

## 0x4D0CD0 CAnimBlendSequence::SetNumFrames  (gr_deviations = 1)
Row AnimBlendSequence.cpp:145 (NOTSA: `m_bUsingExternalMemory = frameData != nullptr`).
- Original bytes (`__thiscall void(int32 count [esp+4], bool hasTrans [esp+8], bool compressed [esp+0xC], void* frameData [esp+0x10])`,
  `ret 0x10`; flags byte +0x4: bit0 HasRotation, bit1 HasTranslation, bit2 IsCompressed, bit3 UsingExternalMemory):
  1. if hasTrans: `flags |= 3`, size = compressed ? count*16 : count*32 (0x4D0CE9-0x4D0D02);
     else: `flags |= 1`, size = compressed ? count*10 : count*20 (0x4D0D05-0x4D0D1D).
  2. if `frameData == null`: `frameData = CMemoryMgr::Malloc(size, 0)` (0x72F420, cdecl, second pushed value is the null frameData,
     0x4D0D21-0x4D0D26).
  3. `m_Frames (+0x8) = frameData`; `m_FramesNum (+0x6, u16) = (u16)count`; if compressed: `flags |= 4` (0x4D0D29-0x4D0D34).
  The flags are only ever OR-ed: bits that were already set stay set; bit3 is never written.
- Verdict: CONFIRMED - gr writes bit3 (external memory) and assigns (can clear) the translation/compressed bits; the original only
  sets bits 0/1/2 and leaves bit3 unchanged.
- Class: keep-original (affects MoveMemory/RemoveFrames decisions that read bit3; callers that pass external frame data set bit3
  themselves in the original, see CAnimManager::LoadAnimFile).
