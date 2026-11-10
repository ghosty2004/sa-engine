# CTaskSimplePlayHandSignalAnim (AI) - RE-SPEC-3 group I2
Vtable 0x86D5B8, type 0x1A9 (425), size 0x24. Layout (gr): CTaskSimpleAnim {+8 m_pAnim, +0xC flags byte (bit0 m_bIsFinished, bit1,
bit2 m_bHoldLastFrame)}, +0x10 m_PlayerIdlesAnimBlockId (i32, -1 = no hand objects), +0x14 m_AnimBlenDelta f32, +0x18 m_DoUseFatHands u8,
+0x1C m_LeftHand CHandObject*, +0x20 m_RightHand CHandObject*.

## 0x61AE50 CTaskSimplePlayHandSignalAnim::CTaskSimplePlayHandSignalAnim  (catalogue: specified/REA, port level 1)
- `__thiscall this (int32 animId [esp+4], f32 blend [+8], bool fatHands [+0xC], bool holdLastFrame [+0x10])`, `ret 0x10`.
- `CTaskSimple()` 0x61A390; `flags(+0xC) = (flags & 0xF8) | ((holdLastFrame & 1) << 2)` (upper 5 bits keep uninitialised pool contents;
  bits 0/1 cleared); `+0x10 = animId`; `+8 = 0`; `+0x18 = (fatHands != 0)`; `+0x1C = 0`; `+0x20 = 0`; vtbl = 0x86D5B8; `+0x14 = blend`.
  gr cpp:15 equivalent (gr goes through the CTaskSimpleAnim ctor; same fields).

## 0x61AEA0 CTaskSimplePlayHandSignalAnim::GetTaskType  (catalogue: specified/REA, port level 0)
- returns 0x1A9.

## 0x61B980 CTaskSimplePlayHandSignalAnim::Clone  (catalogue: specified/REA, port level 2)
- `p = CTask::operator new(0x24)`; if p: ctor 0x61AE50 with `(+0x10, +0x14 raw, (u8)+0x18, (flags >> 2) & 1)`; return p or 0.

## 0x61BD80 CTaskSimplePlayHandSignalAnim::`scalar deleting destructor'  (catalogue name ~CTaskSimplePlayHandSignalAnim)  (catalogue: specified/REA, port level 3)
- call 0x61AEB0 (complete dtor, see _unattributed-I1.md); `if (flags & 1) CTask::operator delete(this)`; return this; `ret 4`.
