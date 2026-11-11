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

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x48E8E0 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x48E900 CTaskSimpleCower::Destructor (0x48E8E3); if `flags & 1` (0x48E8E8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x48E8F0); returns this (0x48E8F8). Proposed name: `CTaskSimpleCower::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCower::Destructor).

## 0x48E940 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x48E960 CTaskSimpleUseAtm::Destructor (0x48E943); if `flags & 1` (0x48E948) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x48E950); returns this (0x48E958). Proposed name: `CTaskSimpleUseAtm::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleUseAtm::Destructor).

## 0x61BA80 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x61A6F0 CTaskSimpleAnim::~CTaskSimpleAnim (0x61BA83); if `flags & 1` (0x61BA88) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x61BA90); returns this (0x61BA98).

## 0x61BBC0 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x48E090 CTaskSimpleRunTimedAnim::Destructor (0x61BBC3); if `flags & 1` (0x61BBC8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x61BBD0); returns this (0x61BBD8). Proposed name: `CTaskSimpleRunTimedAnim::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleRunTimedAnim::Destructor).

## 0x637300 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x637320 thunk_FUN_0061a6f0 (0x637303); if `flags & 1` (0x637308) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x637310); returns this (0x637318). Proposed name: `thunk_FUN_0061a6f0::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_0061a6f0).

## 0x637330 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x637350 thunk_FUN_0061a6f0 (0x637333); if `flags & 1` (0x637338) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x637340); returns this (0x637348). Proposed name: `thunk_FUN_0061a6f0::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_0061a6f0).

## 0x637360 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x637380 thunk_FUN_0061a6f0 (0x637363); if `flags & 1` (0x637368) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x637370); returns this (0x637378). Proposed name: `thunk_FUN_0061a6f0::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_0061a6f0).

## 0x637390 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6373B0 thunk_FUN_0061a6f0 (0x637393); if `flags & 1` (0x637398) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6373A0); returns this (0x6373A8). Proposed name: `thunk_FUN_0061a6f0::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_0061a6f0).

## 0x6373C0 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6373E0 thunk_FUN_0061a6f0 (0x6373C3); if `flags & 1` (0x6373C8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6373D0); returns this (0x6373D8). Proposed name: `thunk_FUN_0061a6f0::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_0061a6f0).

## 0x6373F0 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x637410 thunk_FUN_0061a6f0 (0x6373F3); if `flags & 1` (0x6373F8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x637400); returns this (0x637408). Proposed name: `thunk_FUN_0061a6f0::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_0061a6f0).

## 0x637420 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x637440 thunk_FUN_0061a6f0 (0x637423); if `flags & 1` (0x637428) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x637430); returns this (0x637438). Proposed name: `thunk_FUN_0061a6f0::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_0061a6f0).

## 0x637450 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x637470 thunk_FUN_0061a6f0 (0x637453); if `flags & 1` (0x637458) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x637460); returns this (0x637468). Proposed name: `thunk_FUN_0061a6f0::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_0061a6f0).

## 0x655DB0 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x655DD0 thunk_FUN_0061a6f0 (0x655DB3); if `flags & 1` (0x655DB8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x655DC0); returns this (0x655DC8). Proposed name: `thunk_FUN_0061a6f0::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_0061a6f0).

## 0x655DE0 CTaskSimpleAnim::~CTaskSimpleAnim  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x655E00 thunk_FUN_0061a6f0 (0x655DE3); if `flags & 1` (0x655DE8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x655DF0); returns this (0x655DF8). Proposed name: `thunk_FUN_0061a6f0::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_0061a6f0).
