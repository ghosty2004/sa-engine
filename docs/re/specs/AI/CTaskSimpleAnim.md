# CTaskSimpleAnim and anim-task deleting destructors (AI) - RE-SPEC-3 group I2
CTaskSimpleAnim layout: +8 m_pAnim (CAnimBlendAssociation*), +0xC flags (bit0 m_bIsFinished, bit2 m_bHoldLastFrame...). Vtable 0x86D504.

## 0x61A6F0 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 1)
- Complete dtor (SEH): vtbl = 0x86D504; if m_pAnim: `m_pAnim->SetFinishCallback(0x4D1390 CDefaultAnimCallback::DefaultAnimCB, nullptr)`
  0x4CEBE0; `m_pAnim->m_nFlags (+0x2E, u16) |= 4`; if `!(flags & 4)` and `m_pAnim->m_fBlendAmount (+0x18) > 0.0f` (strict) and
  `m_pAnim->m_fBlendDelta (+0x1C) >= 0.0f` (`test ah,1`: not less; NaN counts as less): `m_fBlendDelta = -4.0f`; then `m_pAnim = 0`.
  Finally `~CTaskSimple` 0x61A3A0. gr TaskSimpleAnim.cpp:19 matches (flag 4 = ANIMATION_IS_BLEND_AUTO_REMOVE).

## 0x61A8A0 CTaskSimpleAnim::FinishRunAnimCB  (catalogue: specified/REA, port level 0)
- `__cdecl void (CAnimBlendAssociation* a, void* task)`: `task->+0xC |= 1` (finished); `task->+8 = 0`. gr cpp:86 matches.

## Deleting destructors (catalogue name `CTaskSimpleAnim::~CTaskSimpleAnim`; NAME CORRECTION)
Same shape as CTaskSimple.md (call the complete dtor, `if (flags & 1) CTask::operator delete`, return this, `ret 4`):
| address | vtable (slot 0) | proposed name | complete dtor |
|---|---|---|---|
| 0x48EA70 | 0x85A164 | CTaskSimpleLookAbout::`scalar deleting destructor' | 0x48EA90 |
| 0x5F7FB0 | 0x86C78C | CTaskSimpleChat::`scalar deleting destructor' | 0x5F7FD0 |
| 0x61BAA0 | 0x86D528 | CTaskSimpleRunAnim::`scalar deleting destructor' | 0x48DE60 |
| 0x61BEF0 | 0x86D54C | CTaskSimpleRunNamedAnim::`scalar deleting destructor' | 0x61BF10 |
