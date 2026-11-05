# CAnimBlendStaticAssociation (ANIM) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x4CEC00 CAnimBlendStaticAssociation::~CAnimBlendStaticAssociation  (catalogue name "CAnimBlendStaticAssociation": specified/REA, port level 1)
- Name correction: the body is the (non-deleting) destructor, not a constructor: it stores the class vtable and frees the
  sequence array. gr `AnimBlendStaticAssociation.cpp:23` labels 0x4CEC00 as the destructor.
- Convention: `__thiscall void()`, plain `ret`. Callers: deleting destructor 0x4CDF50, and as the element destructor of
  `eh_vector_*_iterator` in CAnimBlendAssocGroup::CreateAssociations* (0x4CE220, 0x4CE3B0, 0x4CE5C0, 0x4CE6E0).
- Behaviour: `this->vtable = 0x85C6CC` (0x4CEC00); `p = this->m_BlendSeqs (+0xC)`; if p != null: `CMemoryMgr::Free(p)` (0x72F430,
  cdecl) (0x4CEC06-0x4CEC13). The field is not cleared afterwards. (Same as the inlined FreeSequenceArray 0x4CE9A0.)
- gta-reversed: same.
