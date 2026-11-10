# CTaskComplexPartner (AI) - RE-SPEC-3 group I2
Vtable 0x870664 (Chat 0x8707C4, Deal 0x870754, Shove 0x870800 share several slots). Layout from the ctor bytes 0x681E70 (gr header
offsets are 8 lower from m_taskId on): +0x14 m_commandName[32], +0x34 m_taskId i32, +0x38 m_partner CPed* (registered ref), +0x3C
m_distanceMultiplier f32, +0x40 m_point CVector, +0x4C m_targetPoint CVector, +0x58 m_leadSpeaker u8, +0x59 m_partnerState i8
(starts 1), +0x5A m_firstToTargetFlag i8 (starts -1), +0x5B m_updateDirectionCount i8, +0x5C m_taskCompleted u8, +0x5D
m_makePedAlwaysFacePartner u8, +0x5E m_animBlockName[16], +0x6E m_requiredAnimsStreamedIn u8, +0x70 i16 go-to frame counter.
Virtual slot 11 (vtbl+0x2C) = StreamRequiredAnims, slot 12 = RemoveStreamedAnims, slot 13 (vtbl+0x34) = GetPartnerSequence.
Helpers (see _unattributed-I1.md): 0x6822B0 GetPartnerState() (partner's task m_partnerState), 0x681FE0 ComputeMeetingPositions,
0x66B150 (sets +0x34 of a wander task: next scan time). "SS50" below = `p = new(0x20)`, return `p ? CTaskSimpleStandStill(p, 50,
false, false, 8.0f) : 0` (shared tail 0x683C4E).

## 0x681F20 CTaskComplexPartner::CreateFirstSubTask  (catalogue: specified/REA, port level 5)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`, SEH. If `m_leadSpeaker && m_taskId == 0x4B4 && this->+0x74` (chat subclass
  field): if `CAEPedSpeechAudioEntity::RequestPedConversation(ped, m_partner)` 0x4E50E0 (cdecl) -> `+0x76 = 1`; else if `!+0x75`
  return 0; else `+0x74 = 0`. Then `ped->StopPlayingHandSignal()` 0x5E0480; return SS50. (Slot 9 of Partner/Chat/Shove; Deal jumps here.)
- gr: plugin::Call.

## 0x682310 CTaskComplexPartner::StreamRequiredAnims  (catalogue: specified/REA, port level 2)
- `__thiscall void ()`. If `!+0x6E`: if `m_animBlockName[0] != 0` (1-byte `repe cmpsb` against "" 0x858B54): `i =
  CAnimManager::GetAnimationBlockIndex(name)`; if `ms_aAnimBlocks[i].bLoaded` (byte 0xB5D4B0 + 32*i) == 1: `AddAnimBlockRef(i)`, `+0x6E = 1`;
  else `CStreaming::RequestModel(i + 0x63E7, 8)` and return (flag stays 0). Empty name: `+0x6E = 1`.

## 0x683A40 CTaskComplexPartner::~CTaskComplexPartner  (catalogue: specified/REA, port level 2)
- Complete dtor (SEH): vtbl = 0x870664; if m_partner `CleanUpOldReference(m_partner, &m_partner)`; if `+0x6E`: if name non-empty
  `RemoveAnimBlockRef(GetAnimationBlockIndex(name))`; `+0x6E = 0`; `~CTaskComplex` 0x61A3D0.

## 0x683AD0 CTaskComplexPartner::CreateNextSubTask  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`, SEH (`sub esp,0x20`); epilogue via an obfuscated jump (0x401A58).
- Return 0 unless m_partner, `t = m_partner TaskMgr.FindActiveTaskByType(m_taskId)` exists, and `t->m_partner == ped`. Then by m_partnerState:
  - 6: if `!+0x6E` or `m_updateDirectionCount == 0` return 0; `m_updateDirectionCount--`; return `this->GetPartnerSequence()` (vtbl+0x34).
  - 5: if `!m_makePedAlwaysFacePartner`: if `GetPartnerState() >= 5`: if the ped's primary task slot 4 (`intel->+0x14`) is a 0x390 wander
    task whose GetWanderType() (vtbl+0x2C) == 0: `a = ped->GetTopDirection()` and `b = ped->GetTopDirection()` (0x41CCB0, two calls);
    `dir = (u16)CGeneral::GetNodeHeadingFromVector(-a.x, -b.y)` 0x53CDC0; `w = new(0x38)`; `w ? CTaskComplexWanderStandard(w, 4, dir, true)`;
    `0x66B150(w, now + [0x86FCDC] (100000))` - called with ECX = w even when null (ORIGINAL BUG); `ped TaskMgr.SetTask(w, 4, false)`; then
    (if the state test passed) `m_partnerState = 6`. Return SS50.
    If m_makePedAlwaysFacePartner: if `m_firstToTargetFlag == 1`: if `GetPartnerState() == 6` set state 6; return SS50. Else:
    `d = P - Q` (2D, P = ped pos, Q = partner pos; stored f32); `len = sqrt(d.x*d.x + d.y*d.y)`; if `0.99f [0x862CD0] < len < 1.01f
    [0x870750]` (strict both, NaN fails): state = 6, return SS50. Otherwise: `d.Normalise()` (CVector2D 0x44E480); `M = (d.x + Q.x - P.x,
    d.y + Q.y - P.y)` (the y value is also written to the ped arg slot); if `|M| > 0.02f [0x858B38]` (strict): `M.Normalise(); M *= 0.02f`;
    with r = ped matrix right (+0), f = ped matrix forward (+0x10) (matrix not null-checked): `ped->+0x4D8 = r.x*M.x + r.y*M.y`,
    `ped->+0x4DC = f.x*M.x + f.y*M.y` (anim moving shift); return SS50.
  - 4: `s = GetPartnerState()`; if s == 4 or 5: `if (m_firstToTargetFlag == -1) m_firstToTargetFlag = 0`; partner task (re-found, no null
    check) `if (t->+0x5A == -1) t->+0x5A = 1`; state = 5; return SS50. Else: `if (m_firstToTargetFlag == -1) m_firstToTargetFlag = 1`;
    `if (t->+0x5A == -1) t->+0x5A = 0`; `t->m_point = this->m_targetPoint`; return SS50.
  - 3: subtask type 0x387 -> return `new(0x28) CTaskComplexTurnToFaceEntityOrCoord(m_partner, 0.5f [0x86FC7C], 0.2f [0x86FC80])` (or 0);
    type 0x398 -> state = 4, return SS50; else 0.
  - 2: if subtask type != 0xCB return 0; if `m_point.x == 0.0f && m_point.y == 0.0f` return SS50; else state = 3, `+0x70 = 0`, return
    `new(0x28) CTaskComplexGoToPointAndStandStill(4, &m_point, 0.1f, 0.0f, false, true)` (or 0).
  - 1: if subtask type != 0xCB return 0; if m_leadSpeaker: `ComputeMeetingPositions(ped, &m_point, &m_targetPoint)`; partner task (no
    null check) `t->m_point = this->m_targetPoint`. State = 2; return SS50.
  - other: 0.
- gr: plugin::Call.

## 0x6840D0 CTaskComplexPartner::ControlSubTask  (catalogue: specified/REA, port level 11)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`.
- `keep` = (m_partnerState <= 1 (signed) and m_partner and !m_taskCompleted) or (m_partner and its active m_taskId task has
  `+0x38 == ped` and !m_taskCompleted). If !keep: `if (m_pSubTask->MakeAbortable(ped, 1, nullptr)) return 0`.
- If `m_pSubTask->GetTaskType() == 0x387`: `++(i16)+0x70`; if > 150 (signed) and `MakeAbortable(ped, 1, nullptr)` return 0.
- `ped->DropEntityThatThisPedIsHolding(true)` 0x5E0360; `this->StreamRequiredAnims()` (vtbl+0x2C); if `IsLooking(ped)` `AbortLookAt(ped, 500)`;
  return m_pSubTask.
