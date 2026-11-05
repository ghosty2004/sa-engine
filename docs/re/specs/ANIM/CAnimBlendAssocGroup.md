# CAnimBlendAssocGroup (ANIM) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x4CE5C0 CAnimBlendAssocGroup::CreateAssociations(const char* blockName, RpClump* clump)  (catalogue: specified/REA, gr hook commented out, port level 4)
- Convention: `__thiscall void(const char* blockName [esp+4], RpClump* clump [esp+8])`, `ret 8`. SEH frame around the array
  construction. No static callers (catalogue); gr's hook line for this overload is commented out (`AnimBlendAssocGroup.cpp:34`).
- Layout (this): +0x0 m_AnimBlock (CAnimBlock*), +0x4 m_Anims (CAnimBlendStaticAssociation[], stride 0x14, preceded by an int32 count
  at [-4]), +0x8 m_NumAnims, +0xC m_IdOffset (int16 used), +0x10 m_GroupID. CAnimBlock: +0x14 FirstAnimIdx, +0x18 NumAnims.
- Behaviour:
  1. Inlined DestroyAssociations: if `m_Anims`: if `count = ((int32*)m_Anims)[-1]` != 0 call the first element's virtual slot 0 with 3
     (vector deleting destructor: destroys all `count` elements and frees `m_Anims - 4`); else `free(m_Anims - 4)` (0x8214BD).
     Then `m_Anims = 0; m_NumAnims = 0` (0x4CE5DA-0x4CE602).
  2. `m_AnimBlock = CAnimManager::GetAnimationBlock(blockName)` (0x4D3940); no null check (crashes if missing).
  3. `n = block->NumAnims`; `p = operator new(n*0x14 + 4)` (0x82119A); if p: `*(int32*)p = n`, `eh_vector_constructor_iterator(p+4, 0x14, n,
     CAnimBlendStaticAssociation ctor 0x4CE940, dtor 0x4CEC00)`, `m_Anims = p+4`; else `m_Anims = 0`. `m_NumAnims = 0`.
  4. for i = 0 .. block->NumAnims-1 (count re-read each iteration): `m_Anims[i].Init(clump, &CAnimManager::ms_aAnimations[block->FirstAnimIdx + i])`
     (0x4CEC20; ms_aAnimations 0xB4EA40, stride 0x18); `m_Anims[i].m_AnimId (+0x6, i16) = (int16)(m_IdOffset + i)`;
     `m_Anims[i].m_AnimGroupId (+0x8) = (int16)m_GroupID`; `m_NumAnims++` (0x4CE670-0x4CE6B6).
  5. `m_NumAnims = block->NumAnims` (0x4CE6C2).
