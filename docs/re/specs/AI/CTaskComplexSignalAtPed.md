# CTaskComplexSignalAtPed (AI) - RE-SPEC-3 group I3 (written by the lead after the I3 writer ran out of time)
Layout (gr, confirmed): +0xC m_pedToSignalAt, +0x10 m_initialPause (i32, -1 = random), +0x14 m_playAnimAtEnd u8, +0x15
m_areAnimsReferenced u8 ("gangs" 0x86F92C). gr hooks these three slots but the bodies are plugin::Call (not reversed).

## 0x660B30 CTaskComplexSignalAtPed::CreateNextSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`, SEH. `t = m_pSubTask->GetTaskType()` (re-queried per test):
  - 0x398 (TURN_TO_FACE_ENTITY): `ms = m_initialPause`; if ms == -1: `ms = _ftol(((float)(_rand() & 0xFFFF) * 3.0517578e-05f [0x858B14]) *
    2000.0f [0x8631F0])` (drawn before the allocation); `p = new(0x20)`; return p ? `CTaskSimpleStandStill(p, ms, false, false, 8.0f)` : 0.
  - 0xCB (STAND_STILL): `p = new(0xC)`; if p: `CTaskSimple()` 0x61A390, vtbl = 0x86FA68 (CTaskSimpleDoHandSignal), `+8 = 0`; return p / 0.
  - 0x4BE (DO_HAND_SIGNAL): if `m_areAnimsReferenced` and `m_playAnimAtEnd`: `p = new(0x20)`; if p: `id = 0x117 - _ftol(((float)(_rand() &
    0xFFFF) * 3.0517578e-05f) * -8.0f [0x85B320])` (279..286; drawn after the allocation); return `CTaskSimpleRunAnim(p, 0x34, id, 4.0f, false)`.
    Else 0.
  - other: 0.

## 0x660CC0 CTaskComplexSignalAtPed::CreateFirstSubTask  (catalogue: specified/REA, port level 3)
- If `m_pedToSignalAt == 0` or `!CTaskComplexGangLeader::ShouldLoadGangAnims()` 0x65E7F0 return 0. `ped->StopPlayingHandSignal()` 0x5E0480;
  `g_ikChainMan.LookAt("TaskSignalAtPed" 0x86FAB8, ped, m_pedToSignalAt, 5000, 5, nullptr, true, 0.15f, 500, 3, false)`; return
  `(p = new(0x28)) ? CTaskComplexTurnToFaceEntityOrCoord(p, m_pedToSignalAt, 0.5f [0x86FC7C], 0.2f [0x86FC80]) : 0`.

## 0x660D80 CTaskComplexSignalAtPed::ControlSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`.
- "gangs" anim block on +0x15: if !flag and ShouldLoadGangAnims: loaded -> AddAnimBlockRef, flag = 1; else RequestModel(i + 0x63E7, 8).
  If flag and !ShouldLoadGangAnims: RemoveAnimBlockRef, flag = 0.
- If m_pedToSignalAt: `ped->m_fAimingRotation (+0x55C) = CGeneral::LimitRadianAngle(CGeneral::GetRadianAngleBetweenPoints(T.x - P.x,
  T.y - P.y, 0, 0))` (T = target pos, P = ped pos; differences stored f32; 0x53CBE0 then 0x53CB50).
- Return m_pSubTask.
