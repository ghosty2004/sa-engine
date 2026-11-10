# CTaskComplexGangFollower (AI) - RE-SPEC-3 group I1

Vtable 0x86F938, task type 0x4B7 (1207 TASK_COMPLEX_GANG_FOLLOWER), size 0x4C (`operator new(0x4C)` in Clone).
Layout (gr TaskComplexGangFollower.h, offsets confirmed by the bytes; meaning from the bytes):
+0xC m_PedGroup CPedGroup*, +0x10 m_Leader CPed* (registered ref), +0x14 m_LeaderPosAtOffsetCalc CVector (gr m_PedPosn),
+0x20 m_Offset CVector (gr dword20; formation offset, also the out-param of CalculateOffsetPosition), +0x2C CVector (gr dword2C;
ctor `pos`, not read by the functions below), +0x38 f32 (gr dword38; ctor arg 6, copied by Clone), +0x3C u8 m_SlotIndex (index into
the formation tables), +0x3D u8 m_Flags, +0x40 CTaskTimer m_ExhaleTimer {+0x40 start i32, +0x44 interval i32, +0x48 started u8},
+0x49 u8 (timer "stopped" byte, zeroed by the ctor only).
m_Flags bits (from the bytes): 0x01 = holds a ref on anim block "gangs" (0x86F92C); 0x02 = "signal at leader" requested;
0x04 = follow with seek task (set by the ctor; copied by Clone); 0x08 = leader is the player (ctor); 0x10 = offset cached (non-moving).
Ped fields: +0x18 m_pRwClump, +0x47C m_pIntelligence (+4 CTaskManager, +0x68 CEventGroup, +0x274 m_AnotherStaticCounter i32),
+0x474 third flag dword (bit 22 = 0x400000 bDontAcceptIKLookAts), +0x478 fourth flag dword (bit 16 = 0x10000
bMoveAnimSpeedHasBeenSetByTask), +0x46C bit 8 bInVehicle, +0x58C m_pVehicle, +0x534 m_nMoveState (4 WALK, 6 RUN, 7 SPRINT, 1 STILL).
Constant 0x86C938 (i32 = 30, .rdata): static-counter limit used by Create*SubTask. Task types: 0x38B COMPLEX_SEEK_ENTITY,
0x384 (go-to-point simple task found with FindActiveTaskByType; 0x645700 is CTaskSimpleGoToPoint::UpdatePoint), 0x38A,
0x4BA COMPLEX_SIGNAL_AT_PED, 0x2C5 SIMPLE_CAR_DRIVE, 0xCB SIMPLE_STAND_STILL, 0x1AA COMPLEX_HANDSIGNAL_ANIM, 0x4BB, 0x133 (hold-entity).
`GetRandomNumberInRange(int,int)` = ORIGINAL 0x407180 (gr's differs, cat14 finding 1). `_rand` 0x821B1E, `_ftol` 0x821B40.

## 0x65EAA0 CTaskComplexGangFollower::Constructor  (catalogue: specified/REA, port level 2)
- `__thiscall this (CPedGroup* group [esp+4], CPed* leader [+8], u8 slot [+0xC], CVector pos (by value, [+0x10..+0x18]),
  float f [+0x1C])`, `ret 0x1C`, returns this. SEH frame.
- Behaviour (0x65EAC0-0x65EB80): `CTaskComplex()` 0x61A3B0; vtbl = 0x86F938; +0x40 = 0, +0x44 = 0, +0x48 = 0, +0x49 = 0;
  +0xC = group; +0x3C = slot; +0x2C = pos; +0x20 = pos (same vector, 0x65EB05-0x65EB12); +0x38 = f;
  `m_Flags = m_Flags | 0x14` (0x65EB1C-0x65EB2D): the flag byte is READ BEFORE IT WAS EVER INITIALISED (pool memory) - ORIGINAL
  BUG: bits 5..7 keep whatever the task-pool slot held; port must keep the read-modify-write on the raw slot byte (or document it in
  the divergence ledger); +0x10 = leader; if leader: `RegisterReference(leader, &m_Leader)` 0x571B70 and +0x14 = leader position
  (matrix ? matrix+0x30 : +4) (if leader is null +0x14 stays uninitialised); `m_Flags &= 0xFC`; then
  `m_Flags bit3 = (leader == CWorld::FindPlayerPed(0))` 0x56E210 (cdecl, arg 0) (0x65EB5C-0x65EB80).
- gta-reversed: `.Reversed = false` (plugin::CallMethod) - not reversed.

## 0x65EBA0 CTaskComplexGangFollower::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 0x4B7.

## 0x65EBB0 CTaskComplexGangFollower::Destructor  (catalogue: specified/REA, port level 2)
- Complete dtor (thiscall, plain ret, SEH): vtbl = 0x86F938; if m_Leader `CleanUpOldReference(m_Leader, &m_Leader)` 0x571A00;
  if `m_Flags & 1`: `CAnimManager::RemoveAnimBlockRef(CAnimManager::GetAnimationBlockIndex("gangs" 0x86F92C))` 0x4D3990/0x4D3FD0
  (cdecl) and `m_Flags &= 0xFE`; then `~CTaskComplex` 0x61A3D0 (0x65EBCD-0x65EC16). gr: `.Reversed = false`.

## 0x65EC30 CTaskComplexGangFollower::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [+8], const CEvent* event [+0xC])`, `ret 0xC`.
- If m_pSubTask: `r = m_pSubTask->MakeAbortable(ped, priority, event)` (vtbl+0x18); if !r return 0 (flags untouched). Then (also
  when there is no subtask): `ped->+0x474 &= ~0x400000` (bDontAcceptIKLookAts = 0) and `ped->+0x478 &= ~0x10000`
  (bMoveAnimSpeedHasBeenSetByTask = 0) and return 1 (0x65EC50-0x65EC73 / 0x65EC7F-0x65ECA7). gr: `.Reversed = false`.

## 0x65ECB0 CTaskComplexGangFollower::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (const this)`, plain ret, SEH.
- `p = CTask::operator new(0x4C)`; if p: `Constructor(p, m_PedGroup, m_Leader, (u8)m_SlotIndex, m_Offset (+0x20, NOT +0x2C, by
  value), +0x38)` (0x65ECE4-0x65ED10); else p = 0. Then `p->m_Flags = (p->m_Flags & ~4) | (this->m_Flags & 4)` (xor/and/xor
  idiom, 0x65ED19-0x65ED26) - executed EVEN WHEN p == 0 (reads and writes address 0x3D: ORIGINAL BUG, crash when the task pool is
  full). Returns p. Note the clone gets this->m_Offset as its `pos`.
- gta-reversed: `.Reversed = false`.

## 0x65ED40 CTaskComplexGangFollower::CalculateOffsetPosition  (catalogue: specified/REA, port level 1)
- `__thiscall void (this, CVector* out [esp+4])`, `ret 4`. The function returns through `out`; EAX on return is NOT `out`
  (it holds the last copied dword, offset.z bits, 0x65EE7C) - callers must not use EAX. (This also resolves the gr-deviations
  row, see below.)
- Tables: `CTaskComplexFollowLeaderInFormation::ms_offsets` 0xC196E8: 8 CVector2D (stride 8) for a standing leader, followed at
  0xC19728 by 8 CVector2D for a moving leader (gr struct Offsets {Offsets[8]; MovingOffsets[8]}).
- Behaviour:
  1. `moving = leader->m_nMoveState (+0x534) in {4, 6, 7}` (WALK, RUN, SPRINT; JOG 5 is NOT included) (0x65ED46-0x65ED62).
  2. `d = leaderPos - m_LeaderPosAtOffsetCalc` (x, y, z, kept on the x87 stack, 0x65ED75-0x65ED83). No null check on m_Leader.
  3. If moving (0x65EE22): discard d; `v = (MovingOffsets[slot].x, MovingOffsets[slot].y, 0)`; `m_Offset =
     CVector::FromMultiply3x3(leader->m_matrix, &v)` 0x59C6D0 (thiscall ECX = &m_Offset, args (matrix, &v); the matrix pointer is
     not null-checked); `m_Flags &= ~0x10` (0x65EE63-0x65EE66). m_LeaderPosAtOffsetCalc is not updated.
  4. Else: `distSq = (dz*dz + dy*dy) + dx*dx` (z term first, 0x65ED8C-0x65ED9C); if `distSq > 9.0f [0x86F838]` (`test ah,0x41; je`)
     -> recompute; else (<=, NaN) if `m_Flags & 0x10` -> keep the cached m_Offset (0x65EDB1); else recompute.
     Recompute (0x65EDBA): `m_LeaderPosAtOffsetCalc = leaderPos`; `m_Offset = (Offsets[slot].x, Offsets[slot].y, 0)`; `m_Flags |= 0x10`.
  5. `*out = m_Offset` (0x65EE6B-0x65EE7F).
- gta-reversed: CalculateOffsetPosition is plain plugin::Call; gr-deviations row TaskComplexGangFollower.cpp:11 (hook commented out
  because gr returns CVector by value while the original takes `CVector&`): Verdict CONFIRMED as a signature note only - the
  original is `void (CVector* out)` with `ret 4`, out-param only (and EAX is not the out pointer, so a by-value-return hook would
  be wrong). Class: harmless for a Rust port that follows this spec.

## 0x6629F0 CTaskComplexGangFollower::`scalar deleting destructor'  (catalogue: specified/REA, port level 3)
- Slot 0 of 0x86F938: call 0x65EBB0; `if (flags & 1) CTask::operator delete(this)` 0x61A5B0; return this; `ret 4`.

## 0x662A10 CTaskComplexGangFollower::ControlSubTask  (catalogue: specified/REA, port level 9)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. SEH frame (`sub esp,0x40`); epilogue jumps through an obfuscation stub
  (0x663493 -> 0x401F25 restores fs:[0] -> 0x663498) - no behavioural effect. Frame base B = esp after `push ebx; push edi`
  (0x662ACD): ped arg slot [B+0x60] (reused for several temporaries, listed where used), SEH state [B+0x58].
  Locals: B+0x12 u8 speedSetByTask (init 0, 0x662B06), B+0x13 u8 staticTooLong, B+0x14 pOffset (=&m_Offset, later the held entity),
  B+0x18 walkAssoc (later the anim-0x12D assoc), B+0x1C goToTask, B+0x20 f32 distSq2D (later r1), B+0x24 CVector d,
  B+0x30 CVector target, B+0x3C CVector v / CEventPassObject.
- Block A, early exits (0x662A2C-0x662A90):
  1. `ped->+0x474 &= ~0x400000` (bDontAcceptIKLookAts = 0) - always, first.
  2. If m_Leader == 0: `if (m_pSubTask->MakeAbortable(ped, 1 URGENT, nullptr))` { `ped->+0x478 &= ~0x10000`; return 0 } else return
     m_pSubTask. (No null check on m_pSubTask.)
  3. If `m_pSubTask->GetTaskType() == 0x2C5` (CAR_DRIVE) return m_pSubTask.
- Block B, setup (0x662A93-0x662B57): `walkAssoc = RpAnimBlendClumpGetAssociation(ped->m_pRwClump, 0)` 0x4D68B0 (anim id 0 = walk)
  -> [B+0x18]; `ms = m_Leader->m_pIntelligence->GetMoveStateFromGoToTask()` 0x601D70; `leaderMoving = ms in {4,6,7}` stored as a BYTE
  into the low byte of the ped arg slot (0x662AC0/0x662AC7); `CalculateOffsetPosition(&m_Offset)` 0x65ED40 (pOffset [B+0x14]);
  `d = leaderPos - pedPos` computed z, y, x; stored `d = (dx, dy, 0)` [B+0x24]; `distSq2D = dy*dy + dx*dx` (dy term first, from the
  f32 stores) -> [B+0x20].
- Block C, seek-task steering - only if `m_pSubTask->GetTaskType() == 0x38B` (else go to Block E with speedSet = 0)
  (0x662B5B-0x662E0B):
  1. `staticTooLong = ped->m_pIntelligence->m_AnotherStaticCounter (+0x274) > 8` (signed) -> [B+0x13]. If `!(m_Flags & 4)` go to E.
  2. `seek = m_pSubTask`: `seek->+0x44 = m_Offset` (the XY-offset seek calculator's vector); `seek->+0x20 = 2.0f` (raw 0x40000000).
  3. `goTo = ped TaskMgr.FindActiveTaskByType(0x384)` 0x681740 -> [B+0x1C]; `x = FindActiveTaskByType(0x38A)`.
     If goTo == 0 or x != 0: skip to step 7 (no UpdatePoint).
  4. `target = leaderPos + m_Offset` (0x40FE30) [B+0x30]; `v = target - pedPos` (VectorSub 0x40FE60) [B+0x3C], then `v.z = 0`.
  5. If leaderMoving (byte read at 0x662C23): with `f = m_Leader->m_matrix->forward` (+0x10; matrix not null-checked):
     `a = -((target.y*f.y + target.z*f.z) + target.x*f.x)` stored f32 to the arg slot [B+0x60]; `s = DotProduct(pedPos, f)` 0x40FDB0
     (`(P.z*f.z + P.y*f.y) + P.x*f.x`) `+ a` (0x662C3C-0x662C75). If `s < 0.0f [0x858B50]` (strict; equal/greater/NaN -> skip to
     step 6 with speedSet = 0) (0x662C7C-0x662C87), i.e. the ped is behind the target plane:
       - `m = v.x*v.x + v.y*v.y + v.z*v.z` (0x406DA0, order (x*x + y*y) + z*z); `r = goTo->+0x18`; `lim = (1.0f + r) * (r + 1.0f)`
         (constant 0x858624 = 1.0f, 0x662C96-0x662CAC). If `m <= lim` (fcompp, `test ah,0x41; jp`: greater/NaN go to the else):
         `target += f * 2.0f` (0x40FEC0 then CVector::operator+= 0x411A00); if walkAssoc: `walkAssoc->m_fSpeed (+0x24) =
         max(0.85f, speed - 0.0125f [0x85A5C4])` via 0x420800 (cdecl (a=0.85, b), returns `a > b ? a : b`, ties/NaN -> b);
         speedSet = 1 (0x662CBB-0x662D0A). (walkAssoc null -> speedSet = 0.)
       - else if walkAssoc (0x662D0E): `old = walkAssoc->m_fSpeed` (arg slot [B+0x60]); `ped->SetMoveAnimSpeed(walkAssoc)` 0x5DEC10;
         `new = walkAssoc->m_fSpeed` ([B+0x18], overwriting walkAssoc which stays in EDI);
         if `approxEqual(old, new, 0.013f)` 0x4EEA80 (`|old-new| < eps`, strict): speed = old (undo);
         else if `old > new` (strict) speed = old - 0.0125f; else (old <= new or NaN) speed = old + 0.0125f; store f32 to +0x24;
         speedSet = 1 (0x662D16-0x662D6E). (walkAssoc null -> speedSet = 0.)
     If !leaderMoving: speedSet = 0.
  6. `goTo->UpdatePoint(&target, 0.5f [0x86FC78], false)` 0x645700 (0x662D76-0x662D88).
  7. If staticTooLong and `distSq2D < 64.0f [0x86F82C]` (strict, 0x662D9F-0x662DAE) and `m_pSubTask->MakeAbortable(ped, 1, nullptr)`:
     `m_Offset = d` (dx, dy, 0); `p = new(0x20)`; return p ? `CTaskSimpleStandStill(p, 500, false, false, 8.0f)` 0x62F310 : 0
     (0x662DC1-0x662E0B). (bMoveAnimSpeedHasBeenSetByTask is NOT updated on this return path.)
- Block E (0x662E10-0x662EDD):
  1. `ped->+0x478 bit16 = speedSet` (bit-insert via xor/and/xor).
  2. Anim block "gangs": if !(m_Flags & 1): if `CTaskComplexGangLeader::ShouldLoadGangAnims()` 0x65E7F0: `idx =
     GetAnimationBlockIndex("gangs")`; if `CAnimManager::ms_aAnimBlocks[idx].bLoaded` (array 0xB5D4A0 stride 0x20, byte +0x10) == 1:
     `AddAnimBlockRef(idx)` 0x4D3FB0 and `m_Flags |= 1`; else `CStreaming::RequestModel(idx + 0x63E7, 8)` 0x4087E0.
     Else (flag set) if !ShouldLoadGangAnims(): RemoveAnimBlockRef(GetAnimationBlockIndex("gangs")), `m_Flags &= ~1`.
  3. If `!m_PedGroup->m_bIsMissionGroup (+0x2D0)` and `!(m_Flags & 8)` and `m_PedGroup->m_groupMembership.CountMembers() > 3`
     0x5F6A50 and `GetRandomNumberInRange(0, 2000) == 500`: `m_Flags |= 2` (short-circuit order as listed; the RNG is consumed only
     when the first three hold).
- Block F, exhale FX (0x662EDD-0x662F47): if `m_ExhaleTimer.started (+0x48)` and `CTaskTimer::IsOutOfTime(&+0x40)` 0x420E30 and
  `ped->m_pRwClump` and `(clump->parentFrame + 0x10) != 0`: `fx = g_fxMan.CreateFxSystem("exhale" 0x86FBF0, &(0, 0.1f, 0),
  frameMatrix, false)` 0x4A9BE0 (thiscall ECX 0xA9AE80; the vector is built in [B+0x30..0x38]); if fx: `fx->AttachToBone(ped, 5)`
  0x4AA400, `fx->PlayAndKill()` 0x4AA3D0; then `+0x48 = 0` (only inside this condition).
- Block G (0x662F4B-0x662FF5): if `!ped->IsVisible()` 0x536BC0 return m_pSubTask. If `!g_ikChainMan.IsLooking(ped)` 0x6181A0 and
  `GetRandomNumberInRange(0, 100) > 95`: `r1 = GetRandomNumberInRange(3000, 5000)`, converted to f32 into [B+0x20];
  `r2 = GetRandomNumberInRange(0, 8)`; `m = membership.GetMember(_ftol((float)r2))` 0x5F69B0; if m == ped use m_Leader; if m:
  `g_ikChainMan.LookAt("TaskGangFollower" 0x86FBF8, ped, m, _ftol(r1f), 5 BONE_HEAD, nullptr, true, 0.15f, 500, 3, false)` 0x618970.
  (The int->float->_ftol round trips are exact for these ranges.)
- Block H, only if `(m_Flags & 1)` and `ped->m_nMoveState < 6` (else return m_pSubTask) (0x662FFA-0x66300B):
  `held = ped->GetEntityThatThisPedIsHolding()` 0x5E02E0 -> [B+0x14]. If held == 0 go to Block J.
  1. `a1..a4 = RpAnimBlendClumpGetAssociation(clump, 0x12B / 0x12C / 0x12D / 0x12E)` (DRNKBR_PRTL, SMKCIG_PRTL, DRNKBR_PRTL_F,
     SMKCIG_PRTL_F; a1 in the arg slot [B+0x60], a2 EDI, a3 [B+0x18], a4 EBX).
  2. If any a1..a4: if IsLooking(ped) `AbortLookAt(ped, 250)` 0x618280; `ped->+0x474 |= 0x400000`; if `(a2 && a2->m_fCurrentTime
     (+0x20) < 0.5f [0x858B8C])` or `(a4 && a4->+0x20 < 0.5f)` (strict, NaN false): if !m_ExhaleTimer.started:
     `+0x40 = CTimer::m_snTimeInMilliseconds [0xB7CB84]`, `+0x44 = 2700`, `+0x48 = 1` (0x6630A5-0x6630F6).
  3. If `ped TaskMgr.FindActiveTaskByType(0x4BB)` return m_pSubTask. `r = GetRandomNumberInRange(0, 500)`.
  4. If r == 200 (0x663123-0x663280): if any a1..a4 return m_pSubTask; `rcv = CTaskComplexGangLeader::TryToPassObject(ped,
     m_PedGroup)` 0x65EA50 (cdecl); return m_pSubTask if rcv == 0, if `rcv->GetEntityThatThisPedIsHolding()`, or if rcv's active weapon
     type `rcv->+0x5A0 + 0x1C * (i8)rcv->+0x718` != 0. `mi = (i16)held->+0x22`:
     - mi == MI_GANG_DRINK (u16 [0x8CD728]): `q = GetRandomNumberInRange(0, 500)`; q < 250 ? `rcv->Say(0x18, 0, 1.0f, 0, 0, 0)` :
       `ped->Say(0x17, 1500, 1.0f, 0, 0, 0)` (CPed::Say 0x5EFFE0);
     - else mi == MI_GANG_SMOKE (u16 [0x8CD72C]): q < 250 ? `rcv->Say(0xC9, 0, 1.0f, ...)` : `ped->Say(0xC8, 1500, 1.0f, ...)`;
     - then `CEventPassObject ev(ped, false)` 0x65DC70 (stack object at [B+0x3C]); `rcv->m_pIntelligence->m_eventGroup.Add(&ev, false)`
       0x4AB420; `p = new(0x30)`; `p ? CTaskComplexPassObject(p, rcv, true)` 0x6611B0 : 0; `ev.~CEventPassObject()` 0x65DCF0;
       RETURN p (the new subtask, possibly 0).
  5. Else if `GetRandomNumberInRange(0, 100) == 50` and `hold = FindActiveTaskByType(0x133)`: `hold->PlayAnim(
     CTaskComplexGangLeader::GetRandomGangAmbientAnim(ped, held) 0x65E730, 0x34)` 0x691720. Return m_pSubTask.
- Block J, not holding (0x6632D4-0x663481): return m_pSubTask if `ped->IsPlayingHandSignal()` 0x5E0460 or
  `TaskMgr.GetTaskSecondary(4)` 0x681810. `r = GetRandomNumberInRange(0, 500)`:
  - 50 < r < 56 (signed): `p = new(0x20)`; if p: `i = _ftol(((float)(_rand() & 0xFFFF) * 3.0517578e-05f [0x858B14]) * 8.0f [0x859000])`
    (fild, fmul, fmul, `_ftol`), `CTaskSimpleRunAnim(p, 0x34, 0x117 + i, 4.0f, false)` 0x61A8B0 (anims 279..286 PRTIAL_GNGTLK*);
    `TaskMgr.SetTaskSecondary(p, 4)` 0x681B60 (p may be 0). Then if `ped->m_nMoveState == 1`: say-switch.
  - r == 100 and `ped->m_nMoveState == 1`: `p = new(0x18)`; `p ? CTaskComplexPlayHandSignalAnim(p, -1, 4.0f)` 0x61B2B0; SetTaskSecondary(p, 4);
    say-switch.
  - say-switch: `k = GetRandomNumberInRange(0, 10)`; k in 0..2 -> `ped->Say(0x2D, 0, 1.0f, 0, 0, 0)`; 3..7 -> Say(0xA6, ...);
    8 -> Say(0x18, ...); 9 -> Say(0xC9, ...) (byte tables 0x6634B0 / 0x6634CC = {0,0,0,1,1,1,1,1,2,3}, jump tables 0x6634A0 /
    0x6634BC; `ja` for k > 9 unsigned -> nothing).
- Return m_pSubTask (0x663486) on all paths not listed above.
- gta-reversed: `.Reversed = false` (plugin::Call). gr's CTaskComplexGangLeader::ControlSubTask (reversed) has a similar exhale /
  pass-object structure but is a different function - do not reuse it for this one.
- Task type names (gr eTaskType): 0x384 TASK_SIMPLE_GO_TO_POINT, 0x38A TASK_COMPLEX_FOLLOW_NODE_ROUTE, 0x4BB TASK_COMPLEX_PASS_OBJECT,
  0x133 TASK_SIMPLE_HOLD_ENTITY. Open questions: none.

## 0x665E00 CTaskComplexGangFollower::CreateNextSubTask  (catalogue: specified/REA, port level 6)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH frame (`sub esp,0x10`).
- Behaviour:
  1. If m_Leader == 0: `ped->+0x478 &= ~0x10000`; return 0 (0x665E1B-0x665E4D).
  2. `staticLong = ped->m_pIntelligence->+0x274 > [0x86C938] (= 30)` (signed setg, BL).
  3. If `(m_Flags & 2)` and `m_pSubTask->GetTaskType() == 0x4BA` (SIGNAL_AT_PED finished): `m_PedGroup->m_groupMembership
     .RemoveMember(ped)` 0x5FB210; `p = new(0x38)`; if p: `d = GetRandomNumberInRange(0, 8)`, `CTaskComplexWanderGang(p, 4, (u8)d,
     30000, true, 0.5f [0x86FCB0])` 0x66F5C0; `ped TaskMgr.SetTask(p (or 0), 4 /*TASK_PRIMARY_DEFAULT*/, false)` 0x681AF0;
     `ped->+0x478 &= ~0x10000`; return 0 (0x665E78-0x665EFA).
  4. If staticLong -> step 8.
  5. `t = m_pSubTask->GetTaskType()` (re-called for each compare): if t == 0x38B (SEEK) and `m_Leader->m_nMoveState < 4` -> step 8.
     If t == 0xCB (STAND_STILL) or t == 0x1AA (HANDSIGNAL_ANIM) -> step 7. If t == 0x2C5 (CAR_DRIVE) return
     `this->CreateFirstSubTask(ped)` (virtual, vtbl+0x24).
  6. (other types, 0x665F77) if `m_Flags & 4`: `p = new(0x58)`; if p: `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>(p,
     m_Leader, 50000, 1000, 0.5f, 5.0f, 2.0f, false, false)` 0x661DC0 where 0.5f = `1.0f [0x86FC2C] * 0.5f [0x858B8C]` (fld/fmul,
     stored f32), 5.0f = [0x86F830], 2.0f = [0x86FC30]; then (also when p == 0: ORIGINAL BUG, writes to null+0x44) build a temporary
     seek calculator from m_Offset (0x65DE60: vtbl 0x86F8F8 + vector) and copy its vector to `p->+0x44`; `p->+0x54 |= 8`;
     `p->+0x50 = 7`; return p. Else `p = new(0x20)` -> step 8's StandStill construction (shared tail 0x66613F).
  7. (0x666023) If `m_Flags & 2`: `p = new(0x18)`; return p ? `CTaskComplexSignalAtPed(p, m_Leader, -1, false)` 0x660A30 : 0.
     Else `r = GetRandomNumberInRange(0, 30)`; if r == 20: `n = membership.CountMembers()`; `m = membership.GetMember(
     GetRandomNumberInRange(0, n))`; if m == ped `m = membership.GetLeader()` 0x5F69A0; if m: `p = new(0x28)`; return p ?
     `CTaskComplexTurnToFaceEntityOrCoord(p, m, 0.5f [0x86FC7C], 0.2f [0x86FC80])` 0x66B890 : 0. Otherwise (r != 20 or m == 0):
     `p = new(0x18)`; return p ? `CTaskSimplePause(p, 50)` 0x48E750 : 0.
  8. (0x666107) If m_Leader and `m_Leader->IsPlayer()` 0x5DF8F0: `ped->Say(0x5A, 0, 0.3f, 0, 0, 0)`. `p = new(0x20)`; return p ?
     `CTaskSimpleStandStill(p, 500, false, false, 8.0f)` : 0.
- gta-reversed: `.Reversed = false`.

## 0x666160 CTaskComplexGangFollower::CreateFirstSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH.
- Behaviour:
  1. If m_Leader == 0: `ped->+0x478 &= ~0x10000`; return 0.
  2. If ped bInVehicle (+0x46C bit 8) and `ped->m_pVehicle` and `m_Leader->m_pVehicle == ped->m_pVehicle`: `p = new(0x60)`;
     return p ? `CTaskSimpleCarDrive(p, ped->m_pVehicle, nullptr, false)` 0x63C340 : 0 (0x6661A3-0x666203).
  3. Else if ped bInVehicle and ped->m_pVehicle: `p = new(0x34)`; return p ? `CTaskComplexLeaveCar(p, ped->m_pVehicle, 0, 0, true,
     false)` 0x63B8C0 : 0.
  4. Else if `(m_Flags & 4)` and `!(ped->m_pIntelligence->+0x274 > [0x86C938] = 30)`: `p = new(0x58)`; if p the same SeekEntity XYOffset
     construction as CreateNextSubTask step 6 but with the last bool = true (pushes `1, 0`, 0x6662B8); then `p->+0x44 = m_Offset`
     (direct copy), `p->+0x54 |= 8`, `p->+0x50 = 7` - again executed with p == 0 when the pool is full (ORIGINAL BUG); return p.
  5. Else `p = new(0x20)`; return p ? `CTaskSimpleStandStill(p, 500, false, false, 8.0f)` : 0.
- gta-reversed: `.Reversed = false`.
