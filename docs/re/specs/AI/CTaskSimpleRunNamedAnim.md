# CTaskSimpleRunNamedAnim (AI) - RE-SPEC-3 group I3

Vtable 0x86D54C.

## 0x61AA90 CTaskSimpleRunNamedAnim::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x191; ret` -> 401 (TASK_SIMPLE_NAMED_ANIM). Slot 4. gr matches.

## (sections below added by the lead for addresses the I3 writer left unwritten)
Layout (gr, size 0x64): CTaskSimpleAnim (+8 m_pAnim, +0xC flags: bit0 finished, bit1 dontInterrupt, bit2 holdLastFrame, bit4
runInSequence, bit5 offsetPed, bit6 offset pending), +0x10 m_animName[24], +0x28 m_animGroupName[16], +0x38 m_fBlendDelta, +0x3C
m_pAnimHierarchy, +0x40 m_Time (i32, < 0 = no timer), +0x44 CTaskTimer {+0x48 interval, +0x4C started}, +0x50 m_vecOffsetAtEnd, +0x5C
m_animFlags, +0x60 m_nAnimId i16.

## 0x61B770 CTaskSimpleRunNamedAnim::Clone  (catalogue: specified/REA, port level 3)
- `p = new(0x64)`; if p: `CTaskSimpleRunNamedAnim::CTaskSimpleRunNamedAnim(p, m_animName, m_animGroupName, m_animFlags, m_fBlendDelta, m_Time,
  (flags>>1)&1, (flags>>4)&1, (flags>>5)&1, (flags>>2)&1)` 0x61A990; return p / 0. gr header inline matches.

## 0x61BB10 CTaskSimpleRunNamedAnim::StartAnim  (catalogue: specified/REA, port level 8)
- `__thiscall void (this, CPed* ped)`, `ret 4`. If `m_Time >= 0` (signed): timer start = now, interval = m_Time, started = 1. If
  `!(m_animFlags & 8)`: `m_animFlags |= 4`. `m_pAnim = CAnimManager::BlendAnimation(ped->m_pRwClump, m_pAnimHierarchy, m_animFlags,
  m_fBlendDelta)` 0x4D4410. Callback: flags bit5 -> `SetFinishCallback(FinishRunAnimMovePedCB 0x61AAA0, this)`; else if `(m_animFlags & 8)`
  or flags bit4 -> `SetFinishCallback(CTaskSimpleAnim::FinishRunAnimCB 0x61A8A0, this)`; else `SetDeleteCallback(0x61A8A0, this)` 0x4CEBC0.
  Then `m_nAnimId = m_pAnim->+0x2C` (i16).
- gta-reversed: hook commented out; not reversed.

## 0x61BF20 CTaskSimpleRunNamedAnim::ProcessPed  (catalogue: specified/REA, port level 11)
- `__thiscall bool (this, CPed* ped)`, `ret 4`.
- If flags bit6: `OffsetPedPosition(ped)` 0x61AB00.
- If finished (bit0): if bit5 return 1. `a = RpAnimBlendClumpGetAssociation(ped->clump, true, m_pAnimHierarchy)` 0x4D68E0; `b = (0,0,0)`;
  `ped->GetBonePosition(&b, 3, false)` 0x5E4280 (always called); if `m_fBlendDelta > 100.0f [0x858628]` (strict) and a and
  `sqrt((dz*dz + dy*dy) + dx*dx) > 1.0f` with `d = pedPos - b`: `a->m_nFlags (+0x2E) |= 4`; `a->m_fBlendDelta (+0x1C) = m_fBlendDelta *
  -1.0f [0x858C1C]`. Return 1.
- Else: return 1 if `m_pAnimHierarchy == 0` or `m_pAnimHierarchy->+4 == 0` (not loaded). If bit1 (dontInterrupt) and (`!(m_animFlags & 8)`
  or `(m_animFlags & 2)`): `ped->intel->m_eventGroup.RemoveInvalidEvents(true)` 0x4AB760; `.Reorganise()` 0x4AB700.
  If the timer is started and `IsOutOfTime(&+0x44)`: if `m_animFlags & 8`: `this->MakeAbortable(ped, 0, nullptr)` (vtbl+0x18, result
  ignored) and continue; else { if m_pAnim `SetFinishCallback(0x4D1390, nullptr)`; flags |= 1; `m_pAnim = 0`; return 1 }.
  If `m_pAnim == 0`: `a = GetAssociation(clump, true, hier)`; if a and `a->+0x38 != 0` return 1; else `StartAnim(ped)`.
  Return `flags & 1`.
- gta-reversed: VMT hook, body plugin::Call.
