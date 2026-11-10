# CTaskComplexWander (AI) - RE-SPEC-3 group I3

## 0x460CD0 CTaskComplexWander::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x390; ret` -> 912 (TASK_COMPLEX_WANDER). Shared slot 4 of every wander vtable (CTaskComplexWander 0x86FE84,
  CTaskComplexWanderCop 0x859C80, CTaskComplexWanderFlee 0x86F690, 0x85A200, 0x85A23C criminal, 0x86F48C, 0x8700D4 gang). gr matches.

## (sections below added by the lead for addresses the I3 writer left unwritten)
Layout (gr): +0xC m_nMoveState, +0x10 m_nDir u8, +0x14 m_fTargetRadius, +0x18 m_LastNode (u16 area +0x18, u16 node +0x1A), +0x1C
m_NextNode (+0x1C area, +0x1E node), +0x24 flags (bit0 m_bWanderSensibly, bit3 "scratch head next"). Virtual slots: 9 CreateFirstSubTask
(vtbl+0x24), 13 UpdateDir (vtbl+0x34), 14 UpdatePathNodes (vtbl+0x38: `(ped, u8 dir, CNodeAddress* origin, CNodeAddress* target, u8* outDir)`).
`ThePaths.m_pPathNodes[area]` at 0x96F854 (node stride 0x1C), CPathNode::GetPosition 0x420A10 (thiscall, out CVector*).
"SCRATCH" = `p = new(0x20)`; `p ? CTaskSimpleScratchHead(p)` (0x48DF30, a thunk to code that builds a CTaskSimpleRunAnim(0, 0x87, 4.0f,
0x1A5, 0x85A128, 0) and sets vtbl 0x85A100) `: 0`.

## 0x669FF0 CTaskComplexWander::GetDistSqOfClosestPathNodeToPed  (catalogue: specified/REA, port level 0)
- `__thiscall float (this, CPed* ped)`, `ret 4`, ST0.
- `dNext`: if `m_NextNode.area != 0xFFFF` and `m_pPathNodes[area] != 0`: `p = node(area, m_NextNode.node).GetPosition()`, `d = p - pedPos`,
  `dNext = (dz*dz + dy*dy) + dx*dx` (stored f32); else `dNext = 999999.0f` (immediate 0x497423F0).
- `dLast`: the same for m_LastNode (kept on the x87 stack), else `999999.0f [0x86FEC0]`.
- Return `dLast < dNext ? dLast : dNext` (`fcom dNext; test ah,5; jnp`: only a strictly smaller dLast wins; NaN -> dNext).
- gta-reversed (cpp:340, not hooked: "OG name unknown") uses rng::min over {next, last}: ties/NaN pick the first (next) - same result.

## 0x674140 CTaskComplexWander::CreateNextSubTask  (catalogue: specified/REA, port level 9)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`, SEH (epilogue via an obfuscated jump at 0x407E8D). `CS(type)` = `CreateSubTask(ped, type)`
  0x671CB0 (thiscall `(ped, type)`).
- `t = m_pSubTask->GetTaskType()`:
  - 0xE3 (CROSS_ROAD_LOOK_AND_ACHIEVE_HEADING) or 0xE1 (OBSERVE_TRAFFIC_LIGHTS_AND_ACHIEVE_HEADING): return CS(0x384).
  - 0x1A5 (SCRATCH_HEAD): `m_nDir++` (u8 wrap); `UpdatePathNodes(ped, m_nDir, &m_LastNode, &m_NextNode, &m_nDir)`; if both areas != 0xFFFF and
    the two node addresses differ (area or node word): return CS(0x384); else return SCRATCH.
  - 0xF4 (SEQUENCE) or 0x384 (GO_TO_POINT):
    - if `flags & 8`: clear bit 3; return SCRATCH.
    - `UpdateDir(ped)`; `old = m_LastNode` (both words); `UpdatePathNodes(ped, m_nDir, &m_LastNode, &m_NextNode, &m_nDir)`.
    - If the new `m_NextNode == old` (area and node equal - the ped turned back): `seq = new(0x40) CTaskComplexSequence()` (null seq -> AddTask
      with ECX = 0, ORIGINAL BUG); AddTask(new(0x20) CTaskSimpleStandStill(500, false, false, 8.0f)); AddTask(new(0x20) CTaskSimpleRunAnim(
      ped->+0x4D4, 4, 4.0f, false)); AddTask(SCRATCH); `T = ComputeTargetPos(ped, &T, &m_NextNode)` 0x669F60; AddTask(new(0x30)
      CTaskSimpleGoToPoint(m_nMoveState, &T, m_fTargetRadius, false, false)) (0 if that allocation fails); return seq.
    - Else if `!ValidNodes()` 0x669F30: return SCRATCH.
    - Else if `flags & 1`: if `ThePaths.TestForPedTrafficLight(m_LastNode, m_NextNode)` 0x44D480 (ECX 0x96F050, nodes by value): return CS(0xE1);
      else if `ThePaths.TestCrossesRoad(m_LastNode, m_NextNode)` 0x44D790: `p = new(0x14)`; return p ?
      `CTaskComplexCrossRoadLookAndAchieveHeading(p, 2000, ComputeTargetHeading(ped) 0x66F530)` 0x631AE0 : 0.
    - Return CS(0x384).
  - 0x2C0 (LEAVE_CAR): if ped bInVehicle: `p = new(0x70)`; return p ? `CTaskSimpleCarDriveTimed(p, ped->m_pVehicle, 2000)` : 0. Else return
    `this->CreateFirstSubTask(ped)` (vtbl+0x24).
  - 0x2CF (DRIVE_TIMED): return `this->CreateFirstSubTask(ped)`.
  - other: 0.
- gta-reversed: plugin::Call.
