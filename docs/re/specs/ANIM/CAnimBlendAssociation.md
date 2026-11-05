# CAnimBlendAssociation (ANIM) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x4CEB60 CAnimBlendAssociation::GetNode  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CAnimBlendNode*(int32 index [esp+4])`, `ret 4`, return EAX.
- Callers: RpAnimBlendClumpUpdateAnimations 0x4D34F0, CTaskSimpleRunNamedAnim::FinishRunAnimMovePedCB 0x61AAA0.
- Fields: `this+0x10` = `m_BlendNodes` (CAnimBlendNode*, element size 0x18).
- Behaviour: `return (char*)this->m_BlendNodes + index*0x18` (0x4CEB64-0x4CEB6A: `lea eax,[i+i*2]; lea eax,[nodes+eax*8]`).
  No bounds check; index is used as signed 32-bit (wrapping multiply).
- gta-reversed: `GetNode(int32)` inline accessor, same.
