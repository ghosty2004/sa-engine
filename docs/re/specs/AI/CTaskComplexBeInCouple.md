# CTaskComplexBeInCouple (AI) - RE-SPEC-3 group I1

Vtable 0x870724, task type 0x4BF (1215 TASK_COMPLEX_BE_IN_COUPLE). Layout (gr TaskComplexBeInCouple.h, size 0x20; offsets
confirmed by the bytes): CTaskComplex (0xC), +0xC (gr m_walkOnSide; never written by the ctor or by any function below),
+0x10 m_partner CPed* (registered ref), +0x14 m_isLeader u8, +0x15 m_holdHands u8, +0x16 m_lookAtEachOther u8,
+0x18 m_giveUpDist f32, +0x1C m_prevSide u8. Ped fields: +0x14 m_matrix (CMatrix*, pos at +0x30) / +0x4 placement pos,
+0x47C m_pIntelligence (CPedIntelligence*; +4 = CTaskManager, +0x68 = CEventGroup).
IK manager `g_ikChainMan` 0xC15448 (thiscall object). Strings: 0x870848 "CTaskComplexBeInCouple" (IK purpose for PointArm),
0x870838 "TaskBeInCouple" (purpose for LookAt). `CGeneral::GetRandomNumberInRange(int,int)` 0x407180 is the ORIGINAL
int version (see docs/re/cat14-gr-deviations.md finding 1; gr's differs) - call it via orig!.

## 0x6836F0 CTaskComplexBeInCouple::CTaskComplexBeInCouple  (catalogue: specified/REA, port level 2)
- `__thiscall this (CPed* partner [esp+4], bool isLeader [+8], bool holdHands [+0xC], bool lookAtEachOther [+0x10],
  float giveUpDist [+0x14])`, `ret 0x14`, returns this. SEH frame whose handler push is obfuscated (0x408055: `[0x1560A27] xor
  [0x15608BC]` pushed via xchg; no game-visible effect).
- Behaviour: `CTaskComplex::CTaskComplex()` 0x61A3B0; +0x14 = isLeader, +0x15 = holdHands, +0x16 = lookAtEachOther (bytes,
  0x683720-0x683741); vtbl = 0x870724; m_partner = partner (0x68373F); m_giveUpDist = giveUpDist (raw dword, 0x683744);
  m_prevSide = 0 (0x683747); if partner `CEntity::RegisterReference(partner, &m_partner)` 0x571B70. +0xC left as-is.
- gta-reversed: TaskComplexBeInCouple.cpp:9 equivalent (gr default-initialises +0xC = m_walkOnSide{} to 0: harmless, unused).

## 0x683770 CTaskComplexBeInCouple::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 0x4BF. Slot 4.

## 0x683780 CTaskComplexBeInCouple::~CTaskComplexBeInCouple  (catalogue: specified/REA, port level 2)
- `__thiscall void (this)`, complete dtor. vtbl = 0x870724; if m_partner `CEntity::CleanUpOldReference(m_partner, &m_partner)`
  0x571A00; then `CTaskComplex::~CTaskComplex` 0x61A3D0 (deletes the subtask). gr cpp:27 matches.

## 0x6837E0 CTaskComplexBeInCouple::CreateFirstSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped)`: tail-jumps through `vtbl+0x20` (slot 8, CreateNextSubTask = 0x684840) with the same
  stack arg (virtual dispatch, not a direct call). gr cpp:32 calls it non-virtually - same result for this class.

## 0x6837F0 CTaskComplexBeInCouple::AbortArmIK  (catalogue: specified/REA, port level 2)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`; `this` unused.
- Behaviour: for arm 0 then arm 1: `if (g_ikChainMan.IsArmPointing(arm, ped))` 0x6182B0 (thiscall ECX = 0xC15448, args
  (arm, ped)) then `AbortPointArm(arm, ped, 250)` 0x6182F0 (called with ECX = 0xC15448 and 3 stack args; gr declares it static
  `__stdcall` - ECX is ignored) (0x6837F5-0x683836).
- gta-reversed: cpp:37 matches (RIGHT = 0 first, then LEFT = 1).

## 0x6847A0 CTaskComplexBeInCouple::`scalar deleting destructor'  (catalogue: specified/REA, port level 3)
- Slot 0 of 0x870724: call 0x683780, `if (flags & 1) CTask::operator delete(this)` 0x61A5B0, return this, `ret 4`.

## 0x6847C0 CTaskComplexBeInCouple::MakeAbortable  (catalogue: specified/REA, port level 9)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [+8] (unused), CEvent* event [+0xC])`, `ret 0xC`, always returns AL = 1.
- Behaviour: if `m_partner && event && event->HasEditableResponse()` (vtbl+0x24, 0x6847C3-0x6847DC):
  `saved = event->m_bAddToEventGroup` (+0xC byte); set it to 0 (0x6847E1-0x6847E6); `t = event->GetEventType()` is called up
  to three times (once per compare: ==9 DAMAGE, ==0x1F GUN_AIMED_AT, ==0xF SHOT_FIRED, 0x6847EA-0x684808); on a match
  `CEventGroup::Add(&m_partner->m_pIntelligence->m_eventGroup (+0x47C then +0x68), event, false)` 0x4AB420 (0x68480A-0x684819);
  restore `+0xC = saved` (0x68481E). Then always `AbortArmIK(ped)` 0x6837F0 and return 1.
- gta-reversed: cpp:48 matches (switch on one GetEventType call; harmless difference).

## 0x684840 CTaskComplexBeInCouple::CreateNextSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH frame (pool slot freed if a ctor throws).
- Behaviour:
  1. If `m_partner == 0`: `AbortArmIK(ped)`, return 0 (0x684858-0x684869).
  2. If `m_isLeader` (+0x14): `AbortArmIK(ped)`; `p = CTask::operator new(0x38)` 0x61A5A0; if null return 0; else
     `dir = CGeneral::GetRandomNumberInRange(0, 8)` 0x407180 (cdecl int, AFTER the allocation), then
     `CTaskComplexWanderStandard::CTaskComplexWanderStandard(p, moveState 4, dir (u8), wanderSensibly 1)` 0x48E4F0; return p
     (0x684884-0x6848D3).
  3. Else: `AbortArmIK(ped)`; `p = operator new(0x20)`; if null return 0; else
     `CTaskComplexWalkAlongsidePed::CTaskComplexWalkAlongsidePed(p, m_partner, m_giveUpDist (raw f32))` 0x683240; return p
     (0x6848D6-0x68491E).
- gta-reversed: plugin::Call (cpp:68) - not reversed.

## 0x684930 CTaskComplexBeInCouple::ControlSubTask  (catalogue: specified/REA, port level 7)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. SEH frame, `sub esp,0x1C`. Frame base B = esp after the 4 register
  pushes: ped arg at [B+0x3C] (later REUSED as the f32 distSq local), locals dx [B+0x14], dy [B+0x18], side [B+0x10], mid vector
  [B+0x20..0x28]. "fail" = `AbortArmIK(ped)` then return 0 (0x684CE6 / 0x68498F->0x684CEB).
- Helper used: 0x683840 (catalogue unknown; proposed `CTaskComplexBeInCouple::GetSideOfPartner(CPed* ped)`), `__thiscall`,
  `ret 4`: `r = m_partner->m_matrix->right` (reads the matrix WITHOUT a null check, 0x683843-0x683855); P = partner position
  (matrix ? matrix+0x30 : +4), Q = ped position; `a = -((r.y*P.y + r.z*P.z) + r.x*P.x)` (0x68386C-0x68388D);
  `s = ((r.y*Q.y + r.z*Q.z) + r.x*Q.x) + a` (0x683899-0x6838B1); `return (s < 0.0f [0x858B50] or NaN) ? 2 : 1` (`test ah,1`, 0x6838B3-0x6838C7).
- Behaviour:
  1. If `m_partner == 0` or `!m_partner->IsAlive()` 0x5E0170 -> fail (0x68494D-0x684960).
  2. `t = m_partner->m_pIntelligence->FindTaskByType(0x4BF)` 0x600EE0 (thiscall on CPedIntelligence). If m_isLeader: require
     t != 0 and `t->m_isLeader (+0x14) == 0`; else require t != 0 and `t->m_isLeader != 0`; otherwise fail (0x684966-0x6849AC).
     Then require `t->m_partner (+0x10) == ped`, else fail (0x6849AE).
  3. `mine = ped->m_pIntelligence->m_TaskMgr.FindActiveTaskByType(0x4BF)` 0x681740 (ECX = intel+4), then
     `theirs = m_partner->...FindActiveTaskByType(0x4BF)` (0x6849B3-0x6849D9); `side = GetSideOfPartner(ped)` 0x683840.
  4. If !m_isLeader: `w = ped->m_pIntelligence->FindTaskByType(0x4B8 WALK_ALONGSIDE_PED)`; if w: write the CVector at w+0x14 =
     (side == 2 ? -1.05f : 1.05f, 0, 0) (immediates 0xBF866666 / 0x3F866666, 0x6849F3-0x684A5A).
  5. Positions: Q = ped pos (edx), P = partner pos (eax). `dy = P.y - Q.y; dx = P.x - Q.x` (computed y first, x stored f32
     [B+0x14], y stored f32 [B+0x18]); `distSq = dy*dy + dx*dx` (from the stored f32 values, dy term first), stored f32 to
     [B+0x3C] (0x684A7C-0x684A9C). `g = m_giveUpDist; if (distSq > g*g)` (fcompp distSq vs g*g, `test ah,0x41; je`: strictly
     greater fails, NaN continues) fail (0x684AA0-0x684AB4). If `mine == 0` or `theirs == 0` fail (0x684ABA-0x684AC4).
  6. If `distSq > 4.0f [0x858B90]` (strict; NaN -> not) and m_isLeader (0x684ACA-0x684AE0): `AbortArmIK(ped)`; if
     `ped->m_pIntelligence->FindTaskByType(0x4BD WAIT_FOR_PED) == 0`: `p = operator new(0x2C)`; if null return 0, else
     `0x683340(p, m_partner, 0.75f, 20000, false)` (catalogue unknown; proposed CTaskComplexWaitForPed::CTaskComplexWaitForPed,
     vtbl 0x8706F8, thiscall `ret 0x10`) and RETURN p as the new subtask (0x684AFE-0x684B33). If such a task already exists, fall
     through to step 7.
  7. If m_holdHands (+0x15) (0x684B38): `mid = (dx*0.5f [0x858B8C], dy*0.5f, 0)` (f32 stores); `hand = Q + mid` via 0x40FE30
     (cdecl CVector add: out[esp+4], a[+8], b[+0xC]; computes z, y, x sums) into [B+0x14..0x1C] (overwrites dx/dy).
     If `distSq < 2.25f [0x86E104]` (`test ah,5; jp`: equal, greater and NaN go to the else) (0x684B86-0x684B98):
       - if `side != m_prevSide` (u8 zero-extended) `AbortArmIK(ped)`;
       - if side == 2: `g_ikChainMan.PointArm("CTaskComplexBeInCouple", arm 0, ped, nullptr, bone -1, &hand, 0.5f, 250, 30.0f)`
         0x618B60, `m_prevSide = 2`; else if side == 1: same with arm 1, `m_prevSide = 1` (0x684BAE-0x684C10).
     else `AbortArmIK(ped)` (0x684C15).
  8. If m_lookAtEachOther (+0x16) (0x684C1D):
       - if `!g_ikChainMan.IsLooking(ped)` 0x6181A0 and `GetRandomNumberInRange(0, 100) > 80`: `time = GetRandomNumberInRange(2000,
         4000)`; `g_ikChainMan.LookAt("TaskBeInCouple", ped, m_partner, time, bone 5, nullptr, false, 0.25f, 500, 3, false)` 0x618970
         (0x684C28-0x684C7E);
       - then the same for the partner: `!IsLooking(m_partner)` and rand(0,100) > 80 -> `LookAt("TaskBeInCouple", m_partner, ped,
         rand(2000,4000), 5, nullptr, false, 0.25f, 500, 3, false)` (0x684C83-0x684CDC).
     RNG calls happen only when the IsLooking test passes (order: test-rand, then time-rand).
  9. Return `m_pSubTask` (+8) (0x684CE1).
- Edge cases: the ped arg slot is overwritten by distSq, so the step-1/2 fail path must use the original ped (it does: edi).
  In step 2 the leader/non-leader asymmetry: exactly one of the two tasks must be leader. `side` is never 0, so m_prevSide
  starts at 0 -> first PointArm always preceded by AbortArmIK.
- gta-reversed: plugin::Call (cpp:87) - not reversed. Open questions: none.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x6839C0 CTaskComplexBeInCouple::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* ()`, `ret`, SEH frame: `p = CTask::operator new(0x20)` 0x61A5A0 (cdecl, task pool, 0x6839DB); if p == 0 return 0 (0x6839F1); else `CTaskComplexBeInCouple::CTaskComplexBeInCouple` 0x6836F0 on p with args (this+0x10, (u8)this+0x14 (zero-extended) (byte, mov), (u8)this+0x15 (zero-extended) (byte, mov), (u8)this+0x16 (zero-extended) (byte, mov), this+0x18) (0x683A0F) and return its EAX (the ctor returns p).

## 0x683840 CTaskComplexBeInCouple::sub_683840  (catalogue: unknown/REA, port level 0)
Proposed: `int32 CTaskComplexBeInCouple::GetSideOfPartner(CPed* ped) const` (only caller ControlSubTask 0x684930; name as used in the
ControlSubTask section of this file).
- Convention: `__thiscall int32 (this, CPed* ped [esp+4])`, `ret 4`, EAX = 1 or 2. 12-byte local copy of the right vector.
- Behaviour: `A = this->m_partner (+0x10)`; `r = A->m_matrix->right` (matrix +0/+4/+8, copied to locals; no null check on m_matrix, 0x683846);
  `P = A->m_matrix ? matrix+0x30 : A+4` (pointer re-tested); x87: `a = r.y*P.y; a += r.z*P.z; a += r.x*P.x; a = -a` (fchs, kept in st);
  `Q` = ped position likewise; `s = r.y*Q.y; s += r.z*Q.z; s += r.x*Q.x; s += a` (`fadd st(1)`); `fcomp [0x858B50]=0.0f; test ah,1`:
  return 2 if `s < 0` or NaN (C0 set), else 1.
- gta-reversed: absent. Open questions: none.
