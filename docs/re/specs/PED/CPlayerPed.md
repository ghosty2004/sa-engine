# CPlayerPed (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x609380 sub_609380 (proposed: CPlayerPed helper "AreAnyRecruitCheatsActive"; catalogue: unnamed, unknown/REA, port level 0)
- Name evidence: reads `CCheat::m_aCheatsActive` (0x969130) entries 0x4C/0x4D/0x4E = 76 CHEAT_WANNA_BE_IN_MY_GANG,
  77 CHEAT_NO_ONE_CAN_STOP_US, 78 CHEAT_ROCKET_MAYHEM (gr `Enums/eCheats.h`), the three "recruit anyone" cheats; callers
  CPlayerPed::MakeThisPedJoinOurGroup 0x60C840 and CPlayerPed::ProcessGroupBehaviour 0x60D350.
- Convention: `__cdecl bool()` (no args), return AL (only AL defined), plain `ret`.
- Behaviour: `return active[76] || active[77] || active[78]` (byte tests in that order, 0x609380-0x6093A0; returns 1 or 0).

## 0x6093B0 CPlayerPed::~CPlayerPed (proposed; catalogue: unnamed, unknown/REA, port level 16)
- Evidence: called only from the deleting destructor 0x60A9A0 and stores the CPlayerPed vtable 0x86D168 first; ends in CPed::Destructor.
- Convention: `__thiscall void()`, plain `ret`. SEH frame (state 0 while the CPed part is alive, so an exception unwinds into
  CPed::Destructor).
- Fields: `m_pPlayerData` +0x480 (CPlayerData: +0x38 m_nPlayerGroup, +0x48 m_nMeleeWeaponAnimReferenced,
  +0x4C m_nMeleeWeaponAnimReferencedExtra - both are anim-group ids, 0 = none); flags3 +0x478 bit 21 (0x200000) bUsedForReplay.
- Behaviour:
  1. `vtable = 0x86D168`.
  2. If `pd->+0x48 != 0`: `RemoveAnimBlockRef((ms_aAnimAssocGroups[g].m_AnimBlock - ms_aAnimBlocks) >> 5)` with
     `ms_aAnimAssocGroups` = dword at 0xB4EA34 (array, stride 0x14, field +0x0 = CAnimBlock*), `ms_aAnimBlocks` = 0xB5D4A0 (stride 0x20,
     signed `sar 5`); then `pd->+0x48 = 0` (0x6093D3-0x60940D). The pd pointer is re-read from +0x480 each time.
  3. Same for `pd->+0x4C` (0x609414-0x609445).
  4. If `!(flags3 & 0x200000)`: `CPedGroups::RemoveGroup(pd->m_nPlayerGroup)` (0x5FB870, cdecl) (0x60944C-0x609462).
  5. `CPed::Destructor()` 0x5E8620 (0x609474).
- gta-reversed: no CPlayerPed destructor body in gr (the class uses the compiler default); port from the bytes.

## 0x6098F0 CPlayerPed::ProcessAnimGroups  (catalogue: specified/REA, gr `.Reversed = false`, port level 9)
- Convention: `__thiscall void()`, plain `ret`. Caller: CPlayerPed::ProcessControl 0x60EA90.
- Notation: pd = `m_pPlayerData` (+0x480, re-read): +0x4 m_pPedClothesDesc, +0x48 m_nMeleeWeaponAnimReferenced, +0x4C
  m_nMeleeWeaponAnimReferencedExtra, +0x50 m_fFPSMoveHeading. `cur = m_nAnimGroup` (+0x4D4). `blockIdx(g)`: `b = ms_aAnimAssocGroups[g]`
  (dword 0xB4EA34 -> array stride 0x14, +0x0 CAnimBlock*); if `b == null` `b = CAnimManager::GetAnimationBlock(CAnimManager::GetAnimBlockName(g))`;
  index = `(b - 0xB5D4A0) >> 5` (signed). `Def() = CClothes::GetDefaultPlayerMotionGroup()` (0x5A81B0).
- Part A - motion group (0x6098F5-0x609A54):
  1. If `a = pd->m_fFPSMoveHeading` satisfies `a <= -0.87266463f` (0x86D1D4) or NaN, or `a >= 0.87266463f` (0x86D1D0): and
     `TheCamera.m_aCams[TheCamera.m_nActiveCam].Using3rdPersonMouseCam()` (cam array 0xB6F19C, stride 0x238, index byte 0xB6F081) and
     `CanStrafeOrMouseControl()` (0x5DF090): `g = cur`; if `g != 54` -> ADJ(g); else -> P2 check (step 5). Otherwise continue at 2.
  2. `wt = 0`; if `m_pWeaponObject (+0x4F4)` and `mi = CVisibilityPlugins::GetClumpModelInfo(obj)` and `mi->GetModelType() == 4` (weapon):
     `wt = mi->m_weaponInfo (+0x24)`; if `wt` is 35 or 36 -> ADJ(57).
  3. if `m_pIntelligence->GetTaskJetPack()` -> SET(70).
  4. `wt` in {5, 6, 7} -> ADJ(63); {9, 37, 38} -> ADJ(66); {31, 30, 27, 25, 34, 33} -> ADJ(60) (test order as listed: 5,6,7,9,37,38,31,30,27,25,34,33).
     Else if `pd->m_pPedClothesDesc->GetIsWearingBalaclava()` -> SET(69); else g = 54 -> step 5.
  5. P2 check: if `m_nPedType == 1` and `ms_modelInfoPtrs[m_nModelIndex]->m_nAnimType (+0x24) != 54` -> SET(that anim type); else ADJ(54).
  ADJ(g): SET(`g + Def() - 54`). SET(v): if `cur != v`: `m_nAnimGroup = v; ReApplyMoveAnims()` (0x609650).
- Part B - melee anim blocks (0x609A59-0x609C78): `info = GetWeaponInfo(W[act].type, GetWeaponSkill())`; `relA = relB = false`.
  1. If `info->m_eFireType (+0x0) != 0` or `flags1 & 0x100` (bInVehicle): `relB = true` and go straight to the release of +0x48 (step 4, without
     testing relA).
  2. `c = (int8)info->m_nBaseCombo (+0x6E)`: if `c == 4`: `relA = (pd->+0x48 != 0)`. Else `g = CTaskSimpleFight::m_aComboData[c - 4].+0x0`
     (0xC170D0, stride 0x88); if `g != pd->+0x48`: `if (pd->+0x48 != 0) relA = true`; `b` = block of g; if `!b->IsLoaded (+0x10 != 1)`:
     `CStreaming::RequestModel(blockIdx + 25575, 8)`; else if `pd->+0x48 == 0`: `AddAnimBlockRef(blockIdx)` (0x4D3FB0), `pd->+0x48 = g`.
  3. Same with `c2 = (uint8)m_nFightingStyle (+0x72D)` (zero-extended), field +0x4C and relB.
  4. if relA and `pd->+0x48 != 0`: `RemoveAnimBlockRef(blockIdx'(pd->+0x48))` (index from ms_aAnimAssocGroups only, no GetAnimationBlock
     fallback), `pd->+0x48 = 0`. if relB and `pd->+0x4C != 0`: same for +0x4C.

## 0x60A610 CPlayerPed::ControlButtonSprint  (catalogue: specified/REA, gr `.Reversed = false` (plugin::Call), port level 3)
- Convention: `__thiscall float(eSprintType type [esp+4])`, return **ST0**, `ret 4`. Callers: CTaskSimpleGoToPoint::ProcessPed 0x66D710,
  CTaskSimplePlayerOnFoot::PlayerControlZelda 0x6883D0, CTaskSimpleSwim::ProcessControlInput 0x688A90 (2).
- Data: sprint set table 0x8D2460 (.data, stride 0x20, see 0x60A820): S = table[type]: +0x0 add-on-tap (4.0), +0x4 hold decay (0.7),
  +0x8 release decay (0.2), +0xC threshold (5.0), +0x10 max (10.0), +0x14 walk energy cost (1.0/0.0/1.0/0.0), +0x18 sprint energy cost
  (0.5/0.4/0.3/0.0), +0x1C result scale (0.3/1.0/0.3/1.0). `CTimer::ms_fTimeStep` 0xB7CB5C. pd = `m_pPlayerData` (+0x480; +0x18
  m_fTimeCanRun, +0x1C m_fMoveSpeed, +0x84 m_bPlayerSprintDisabled); pd is re-read from +0x480 at most uses.
- Behaviour:
  1. if pd == null return 0.0f (0x60A61E).
  2. `pad = m_nPedType == 0 ? CPad::GetPad(0) : m_nPedType == 1 ? CPad::GetPad(1) : null` (no null check later).
  3. `ok = !pd->m_bPlayerSprintDisabled && (pd->m_fMoveSpeed > 0.0f || pd->m_fTimeCanRun > 0.0f)` (NaN = false each; 0x60A657-0x60A689).
  4. Update m_fMoveSpeed (x87, result stored f32 at 0x60A757):
     - if `pad->SprintJustDown()` (0x5407F0) && ok: `v = S.f0 + speed`; `if (S.f10 < v) v = S.f10` (NaN v kept).
     - else if `pad->GetSprint()` (0x5407A0) && ok: `v = speed - ts * S.f4`; `if (1.0f > v) v = 1.0f` (NaN kept).
     - else: if `speed > 0.0f`: `v = speed - ts * S.f8`; `if (0.0f > v) v = 0.0f`; store. (speed <= 0 or NaN: no store.)
  5. `s = pd->m_fMoveSpeed`:
     - if `s > S.fC`: `r = s / S.fC` (kept as f32 copy), `cost = S.f18`; if NOT `r > 0` return 0.0f; go to 6.
     - else if `s > 0.0f && ok`: `r = 1.0f`, `cost = S.f14`; go to 6. Else return 0.0f.
  6. if `!this->HandleSprintEnergy(true, cost)` (0x60A550, thiscall(bool, float)): `pd->m_fMoveSpeed = 0`; return 0.0f.
     Else `t = r - 1.0f`; `if (0.0f > t) t = 0`; return `t * S.f1C + 1.0f` (ST0, unrounded) (0x60A7A9-0x60A7D3).

## 0x60A820 CPlayerPed::GetButtonSprintResults  (catalogue: specified/REA, gr hook `.Reversed = false`, port level 0)
- Convention: `__thiscall float(eSprintType type [esp+4])`, return **ST0**, `ret 4`. Clobbers EAX, ECX (ECX := m_pPlayerData);
  **EDX is preserved** and callers rely on it: CPlayerPed::SetRealMoveAnim 0x60A9C0 keeps an anim-assoc pointer in EDX across the
  call (gr comment at `PlayerPed.cpp:700`, call site 0x60B44C). Callers: SetRealMoveAnim, CTaskSimpleSwim::ProcessSwimAnims 0x6899F0 (3).
- Globals: `PLAYER_SPRINT_THRESHOLD` 0x8D2458 (f32 in **.data**, 5.0f - read from memory, not a constant); sprint set table
  0x8D2460 (stride 0x20, field +0x1C read at 0x8D247C + type*0x20: GROUND 0.3, BMX 1.0, WATER 0.3, UNDERWATER 1.0). Constants
  1.0f 0x858624, 0.0f 0x858B50.
- Fields: `this->m_pPlayerData` (+0x480); `pd->m_fMoveSpeed` (+0x1C, f32).
- Behaviour:
  1. `v = pd->m_fMoveSpeed`. If NOT (`v > threshold`) (`test ah,0x41; jne` - <, == or NaN; 0x60A826-0x60A837):
     return `(v > 0.0f) ? 1.0f : 0.0f` (NaN -> 0.0f; 0x60A872-0x60A88E).
  2. Else `t = v / threshold - 1.0f` (fdiv, fsub, x87); if `0.0f > t` then `t = 0.0f` (`fld 0; fcomp st(1); test ah,0x41; jne`,
     so t = NaN or t >= 0 is kept) (0x60A839-0x60A856).
  3. return `t * table[type].f1C + 1.0f` (fmul f32 memory, fadd 1.0f; left in ST0 unrounded) (0x60A85C-0x60A86F).
  `type` is not range-checked.
- gta-reversed: unhooked C++ (`PlayerPed.cpp:705`) matches the formula but uses constexpr copies of the threshold and table;
  the original reads the .data globals.

## 0x60A9A0 CPlayerPed::vfunc_0 = scalar deleting destructor  (catalogue: specified/REA, port level 17)
- Convention: virtual slot 0, `__thiscall CPlayerPed*(uint8 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `CPlayerPed::~CPlayerPed` 0x6093B0; if `flags & 1`: `CPed::operator delete(this)` 0x5E4760. Caller (virtual):
  CPopulation::AddDeadPedInFrontOfCar 0x612CD0.

## 0x60A9C0 CPlayerPed::SetRealMoveAnim  (catalogue: specified/REA, gr `.Reversed = false`, port level 12)
- Convention: `__thiscall void()`, plain `ret`. Callers: SetPlayerMoveBlendRatio 0x60C520 (5), SetInitialState 0x60CD20, CTaskSimpleGoToPoint::ProcessPed
  0x66D710 (3), CTaskSimpleJetPack::ProcessAnims 0x67B5C0, CTaskSimplePlayerOnFoot::PlayerControlZelda 0x6883D0. The common exit is a split chunk
  (0x60B399 -> 0x409B28 -> 0x60B39F) that **reads the SPRINT association from EDX**; it relies on GetButtonSprintResults 0x60A820 preserving EDX.
- Notation: assocs fetched at entry with `RpAnimBlendClumpGetAssociation(clump, id)` (0x4D68B0) in this order: WALK(0), RUN(1), SPRINT(2), WALK_START(5),
  IDLE(3), RUN_STOP(6), RUN_STOPR(7), TURN_L(137), TURN_R(138), IDLE_TIRED(10). Assoc fields: +0x14 hierarchy (+0x10 total time), +0x18 blendAmount (amt),
  +0x1C blendDelta (dlt), +0x20 currentTime, +0x24 speed, +0x28 timeStep, +0x2E flags (bit0 playing/STARTED, bit2 auto-remove). `DEL(a)` = virtual
  slot 0 with 1 (delete). `BLEND(g, id, d)` = `CAnimManager::BlendAnimation(clump, g, id, d)` (0x4D4610); `ADD(g, id)` = AddAnimation (0x4D3AA0).
  `G` = m_nAnimGroup (+0x4D4). pd = m_pPlayerData; `ratio` = pd->m_fMoveBlendRatio (+0x14). ms = m_nMoveState (+0x534). END = exit tail.
- Behaviour:
  1. If `F2 & 1` (bResetWalkAnims): SetCurrentTime(0) on WALK, RUN, SPRINT (each if non-null); clear the bit.
  2. If (RUN_STOP playing) or (RUN_STOPR playing): `ms = 6`; if RUN_STOP exists, is not playing and `total > currentTime`: set its playing bit. END.
  3. Else if (RUN_STOP and its dlt >= 0) or (RUN_STOPR and dlt >= 0): on RUN_STOP (else RUN_STOPR): flags |= 4, amt = 1.0, dlt = -8.0;
     `RestoreHeadingRate()`; `i = IDLE ?: BLEND(G, 3, 8.0f)`; `i.amt = 0; i.dlt = 8.0f`. END.
  4. Else if `ratio == 0` and SPRINT == null (standing):
     a. if `GetPadFromPlayer()->GetForceCameraBehindPlayer()` and `pad->AimWeaponLeftRight(0) != 0` and active cam mode == 4: `v = AimWeaponLeftRight(0)`
        (int16 -> fild); `t = (v <= 0) ? TURN_L : TURN_R`; re-blend (`BLEND(0, 137|138, 16.0f)`) if t is null, or `t.dlt < 0`, or (`t.amt < 1.0f` and
        `t.dlt <= 0`); `t.speed = |AimWeaponLeftRight(0)| * dword[0x8D24EC] (1.0f, .data) * 0.0078125f`; if IDLE and `IDLE.amt <= 0.01f`: DEL(IDLE).
        Else: if IDLE == null: `BLEND(G, 3, 4.0f)`.
     b. If `pd->m_fTimeCanRun (+0x18) < 0` and no GetTaskFighting / GetTaskUseGun / GetTaskDuck(1) / GetTaskThrow / GetTaskJetPack and
        `!CWorld::TestSphereAgainstWorld(pos, 0.5f, null, 1, 0, 0, 0, 0, 0)` (0x569E20): if IDLE_TIRED == null:
        `BLEND(GetDefaultPlayerMotionGroup() == 55 ? 76 : 0, 10, 4.0f)` and set its playing bit. Else (not tired): if IDLE_TIRED with `amt > 0` and
        `dlt >= 0`: clear its playing bit, `dlt = -2.0f`. In all cases `ms = 1`; END.
  5. Moving (ratio != 0 or SPRINT exists):
     a. If IDLE: `WALK_START` = existing (amt = 1.0, dlt = 0) or `ADD(G, 5)`; SetCurrentTime(0) on WALK and RUN; DEL(IDLE); if IDLE_TIRED `dlt = -4.0f`;
        if SPRINT: DEL(SPRINT); SPRINT = null; `ms = 4`.
     b. DEL(RUN_STOP) + RestoreHeadingRate() if it exists; same for RUN_STOPR; DEL(TURN_L), DEL(TURN_R) if they exist.
     c. If WALK == null: `WALK = ADD(G, 0)`, amt = 0. If RUN == null: `RUN = ADD(G, 1)`, amt = 0.
     d. If WALK_START and (not playing or `timeStep + currentTime >= total`): DEL(WALK_START); set playing on WALK and RUN; WALK_START = null.
     e. If `ms == 7` and WALK_START: `ms = 1`.
     f. If SPRINT and (`ms != 7` or `ratio < 0.4f`):
        - `SPRINT.amt == 0`: flags |= 4, dlt = -1000.
        - else if `SPRINT.dlt <= 0` and `SPRINT.amt < 0.8f`: if `ratio < 1.0f`: `SPRINT.dlt = -8.0f; RUN.dlt = 8.0f`.
        - else if `ratio < 0.4f`: `a = ADD(0, currentTime/total < 0.5 (f64) ? 6 : 7)`, `a.amt = 1.0f`, `a.SetDeleteCallback(RestoreHeadingRateCB 0x5DFD70, this)`;
          `m_fHeadingChangeRate = 0`; SPRINT: flags |= 4, dlt = -1000; WALK/RUN: clear playing, amt = 0, dlt = 0.
        - else if `SPRINT.dlt >= 0`: SPRINT flags |= 4, `dlt = -1.0f`, `RUN.dlt = 1.0f`.
        Then `ms = (ratio > 1.0f) ? 6 : 4`. END.
     g. Else if WALK_START: clear playing on WALK and RUN, both amt = 0. END.
     h. Else if `ms == 7`: if SPRINT == null: if `RUN.amt < 1.0f`: if `WALK.amt == 0 && RUN.amt == 0` WALK.amt = 1.0; if `RUN.dlt <= 0` `RUN = BLEND(G, 1, 4.0f)`;
        `ratio = RUN.dlt + 1.0f`; END. Else `SPRINT = BLEND(G, 2, 2.0f)` then `CStats::UpdateStatsWhenSprinting()`; END.
        If SPRINT exists: if `SPRINT.dlt < 0`: `SPRINT.dlt = 2.0f; RUN.dlt = -2.0f`; `UpdateStatsWhenSprinting()`; END.
     i. Else: `ratio < 1`: WALK.amt = 1, RUN.amt = 0, both dlt 0, `ms = 4`. `ratio < 2`: `WALK.amt = 2.0f - ratio; RUN.amt = ratio - 1.0f`, dlt 0, `ms = 6`.
        Else: WALK.amt = 0, RUN.amt = 1, dlt 0, `ms = 6`, `CStats::UpdateStatsWhenRunning()`. END.
  6. END tail (EDX = SPRINT): if `pd->+0x34 & 2` (m_bAdrenaline): if `time > pd->m_nAdrenalineEndTime (+0x3C)` (unsigned) and not cheat 71 (byte 0x969177):
     clear the bit, `CTimer::ms_fTimeScale = 1.0f`, s = 1.0f; else `ms_fTimeScale = 0.33333334f`, s = 2.0f; set `speed = s` on WALK_START, WALK, RUN (if
     non-null) and SPRINT (if null, return). Then if SPRINT: if active cam mode == 15: `SPRINT.speed = 0.7f`; else `SPRINT.speed = max(GetButtonSprintResults(0), 1.0f)`
     (`1.0f > r` -> 1.0; NaN keeps r).
- Open question: the exact meaning of camera modes 4 and 15 (TheCamera.m_aCams[m_nActiveCam].m_nMode at 0xB6F1A8 + i*0x238) is not needed for porting.

## 0x60B650 CPlayerPed::Compute3rdPersonMouseTarget  (catalogue: specified/REA, gr `.Reversed = false`, port level 9)
- Convention: `__thiscall void(bool useCamTargetVector [esp+4])`, `ret 4`. SEH frame. Caller: CTaskSimplePlayerOnFoot::ProcessPlayerWeapon 0x6859A0.
- Fields: +0x79C mouse target (CEntity*, registered reference; gr m_p3rdPersonMouseTarget), +0x7A0 its expiry time (u32).
  CCam (TheCamera.m_aCams[TheCamera.m_nActiveCam], base 0xB6F19C, stride 0x238): +0x190 m_vecFront, +0x19C m_vecSource.
- Behaviour:
  1. If `!CCamera::m_bUseMouse3rdPerson` (0xB6EC2E) -> step 6.
  2. `R = GetWeaponInfo(W[act].type, GetWeaponSkill())->m_fWeaponRange (+0x4)`; `P` = this position (copy).
     - if the argument is true: `TheCamera.Find3rdPersonCamTargetVector(R, P /*by value*/, &S, &T)` (0x514970).
     - else: `S = cam.m_vecSource`, `F = cam.m_vecFront`; `d = (F.y*(S.y-P.y) + F.z*(S.z-P.z)) + (S.x-P.x)*F.x` (x87; differences formed first);
       if `d < 0` (NaN/>= skip): `S.x -= F.x*d; S.y -= F.y*d; S.z -= F.z*d` (stored f32 each). `T.x = S.x + F.x*R; T.y = F.y*R + S.y; T.z = F.z*R + S.z`.
  3. `hit = null`; `CWorld::pIgnoreEntity = this`; `CWorld::bIncludeBikers = 1`;
     `CWorld::ProcessLineOfSight(S, T, cp, hit, 0, 0, 1 /*peds only*/, 0, 0, 0, 0, 0)` (0x56BA00); if it returns true and `hit != this` and
     `((CPed*)hit)->IsAlive()` (0x5E0170): `tgt = hit`. `CWorld::ResetLineTestOptions()` (0x5631C0). If `tgt == null` -> step 6.
  4. if `tgt != +0x79C`: if old non-null `CEntity::CleanUpOldReference(&+0x79C)` (on the old entity); `+0x79C = tgt`; `tgt->RegisterReference(&+0x79C)`.
     `+0x7A0 = CTimer::m_snTimeInMilliseconds + 1000`.
  5. Return if any holds (in order): `GetWeaponInfo(W[act].type, 1)->m_eFireType == 0`; `!(GetWeaponInfo(W[act].type, skill)->m_nFlags (+0x18) & 1)`
     (bCanAim); `!tgt->m_pIntelligence->IsInSeeingRange(this position)` (0x600C60); the first non-null of `tgt->m_pIntelligence->m_TaskMgr`
     primary task slots 0, 1, 2 (+0x4/+0x8/+0xC of the intelligence) has `GetTaskType()` (vtable +0x10) == 601.
     Then: if `W[act].type != 23`: `this->Say(0xB0, 0, 1.0f, 0, 0, 0)`.
     `g = CPedGroups::GetPedsGroup(tgt)`: if g: if `CPedGroups::AreInSameGroup(tgt, this)` return; `e = new CEventGunAimedAt(this)` (0x18 bytes, 0x4B0700,
     null-safe); stack `CEventGroupEvent ge(tgt, e)`; `g->m_groupIntelligence (+0x30).AddEvent(&ge)`; `~ge`. Else: stack `CEventGunAimedAt ev(this)`;
     `tgt->m_pIntelligence->m_eventGroup (+0x68).Add(&ev, 0)`; `~ev` (0x4B07B0). Return.
  6. Timeout: if `+0x79C != 0` and `+0x7A0 < CTimer::m_snTimeInMilliseconds` (unsigned): `+0x79C = 0` (**no CleanUpOldReference** - the
     registered reference is left in the entity's list).

## 0x60BA80 CPlayerPed::DrawTriangleForMouseRecruitPed  (catalogue: specified/REA, gr `.Reversed = false`, port level 6)
- Convention: `__thiscall void()`, plain `ret`. Caller: Game::RenderEffects 0x53E170.
- `RS(s, v)` = `RwEngineInstance->dOpenDevice.fpRenderStateSet(s, v)` (call `[RwEngineInstance(0xC97B24)] + 0x20`, args (state, value)).
- Behaviour:
  1. `t = +0x79C` (mouse target ped); null -> return.
  2. RS(8, 0), RS(6, 1), RS(10, 5), RS(11, 6), RS(12, 1), RS(1, 0), RS(29, 8) (in this order).
  3. Colour: `r = t->m_fHealth (+0x540) / t->m_fMaxHealth (+0x544)`; if `r > 1` -> r = 1; if `r > 0` (after clamping): `inv = 1 - r`;
     `R = _ftol2(inv*255.0f + r*0.0f)`, `G = _ftol2(r*255.0f + inv*0.0f)`, `B = _ftol2(inv*0.0f + r*0.0f)` (x87, in this order; low bytes kept);
     else (`r <= 0` or NaN) R = G = B = 0. Constants 255.0f 0x859AAC, 0.0f 0x858B50.
  4. Size: `d = |t.pos - this.pos|` (3D: `(dz*dz + dy*dy) + dx*dx`, sqrt); `d -= 10.0f`; if NOT `d > 0` d = 0; `d *= 0.02f`; if `1.0f < d` d = 1;
     `s = d * 0.825f [0x86D1DC] + 0.175f [0x85A5C8]` (x87).
  5. Side vector: if `TheCamera.m_matrix` (0xB6F03C): `Rv = matrix->right` (x, y, z); else `Rv = (cos(h), sin(h), 0)` with h = placement heading
     (0xB6F038; fcos/fsin). `Rv *= s` (x, y, z stored f32). `U = (0*s, 0*s)` (both 0).
  6. `P = t.pos; P.z += 1.0f`. If `t->m_nPedType != 7`: `V0 = (P.x + U.x, U.y + P.y, P.z + s)`, `V1 = P + Rv`, `V2 = P - Rv` (component-wise,
     f32 stores). If `t->m_nPedType == 7` (inverted triangle): `V0 = P`, `V1 = (P.x - Rv.x + U.x, U.y + (P.y - Rv.y), (P.z - Rv.z) + s)`,
     `V2 = (P.x + Rv.x + U.x, U.y + (P.y + Rv.y), (P.z + Rv.z) + s)`.
  7. For V0, V1, V2 in order: `D = camPos - V` (z, y, x computed; stored), `D.Normalise()` (0x59C910), `V += D` (pull 1 unit toward the camera).
  8. `TempVertexBuffer` (0xC4D958, stride 0x24): positions of vertices 0..2 = V0, V1, V2; colours (+0x18): v0 = `0xFF000000 | R<<16 | G<<8 | B`,
     v1 = v2 = `R<<16 | G<<8 | B` (alpha 0). Other vertex fields untouched. `aTempBufferIndices` (0xC4B958) = {0, 1, 2} (u16).
     `if (RwIm3DTransform(TempVertexBuffer, 3, null, 0x18))` { `RwIm3DRenderIndexedPrimitive(3, aTempBufferIndices, 3)`; `RwIm3DEnd()` }.
  9. Restore: RS(1, 0), RS(8, 1), RS(6, 1), RS(10, 5), RS(11, 6), RS(12, 0), RS(29, 5).

## 0x60C1E0 CPlayerPed::KeepAreaAroundPlayerClear  (catalogue: specified/REA, gr `.Reversed = false`, port level 9)
- Convention: `__thiscall void()`, plain `ret`. Callers: CAutomobile/CBike/CHeli/CPlane::ProcessControlInputs (0x6AD690, 0x6BE310, 0x6C4830, 0x6CADD0).
- Prologue oddity: `push -1; jmp 0x401771`, where a protection stub decodes the SEH handler address (`ror [0x1560678], cl=[0x615957]`, flags,
  ECX and EDX preserved) and jumps back to 0x60C1E7. Behaviourally just the SEH frame setup; a port ignores it.
- Behaviour:
  1. Peds (0x60C210-0x60C357): for i = 0..15: `ped = m_pIntelligence->m_pedScanner` entity slot i (dword at intelligence+0x130+i*4); skip unless
     `ped && ped->m_nCreatedBy (+0x484) == 1 && !(ped->flags1 & 0x100) && ped->IsAlive() && !CPedGroups::ms_groups[0].m_groupMembership.IsMember(ped)`
     (group 0 at 0xC09928 fixed, not the player's group index).
     - if `ped->GetIsOnScreen()` and `!(ped->flags3 (+0x478) & 0x10)` (bKeepTasksAfterCleanUp):
       skip if `t = ped->m_pIntelligence->FindTaskByType(911)` exists with `t->+0xC == this`; skip if `e = ped->intel->m_eventGroup.GetEventOfType(32)`
       has `e->+0x10` (task) non-null with `GetTaskType() == 911`. Else `t = new CTaskComplexSmartFleeEntity(this, 0, 1000.0f, 100000, dword[0x86F678],
       dword[0xC18CF0])` (0x40 bytes, 0x65C430) and **`t->+0x30 = 4` is written even when the allocation returned null** (0x60C314);
       stack `CEventScriptCommand ev(3, t, 0)`; `ped->intel->m_eventGroup.Add(&ev, 0)`; destroy (0x4B0A50).
     - else: `ped->FlagToDestroyWhenNextProcessed()` (virtual slot 21, vtable +0x54).
  2. `P` = position of `m_pVehicle` (+0x58C) if `flags1 & 0x100` and the vehicle is non-null, else this ped's position. `C` = this ped's position.
     `CWorld::FindObjectsInRange(C, 15.0f, true, &n, 6, list, 0, 1, 0, 0, 0)` (vehicles only, 2D).
  3. For each `v = list[k]`, k < (int16)n: skip if `v->m_nCreatedBy (+0x4A4) == 2` or `status = byte[+0x36] >> 3` is 0 or 9.
     `d2 = (dz*dz + dy*dy) + dx*dx` (v - P, 3D); if `d2 > 25.0f`: `v->m_autoPilot.m_nTempAction (+0x3BB) = 1`, `m_nTempActionTime (+0x3BC) = time + 5000`;
     else (<=, NaN): `dot = (P.y - v.y) * v->m_matrix->forward.y + (P.x - v.x) * v->m_matrix->forward.x` (m_matrix used even when null -> crash in
     the original if a vehicle has no matrix); `temp action = (dot > 0) ? 3 : 9`, time + 2000. Then `CCarCtrl::PossiblyRemoveVehicle(v)` (0x424F80).

## 0x60C840 CPlayerPed::MakeThisPedJoinOurGroup  (catalogue: specified/REA, gr `.Reversed = false`, port level 36)
- Convention: `__thiscall void(CPed* ped [esp+4])`, `ret 4`. SEH frame for the stack events. Caller: CPlayerPed::ProcessGroupBehaviour 0x60D350 (2).
- Notation: `pd = this->m_pPlayerData` (+0x480, re-read every use), `G = CPedGroups::ms_groups[pd->m_nPlayerGroup (+0x38)]` (0xC09920,
  stride 0x2D4; membership at +0x8, intelligence at +0x30). `Say(ctx, delay, prob, overrideSilence, forceAudible, frontEnd)` = CPed::Say 0x5EFFE0.
- Behaviour:
  1. if `ped->flags3 (+0x478) & 0x400` (bit 10, gr name bDruggedUp): **`this->Say(0x52, 0, 1.0f, 0, 0, 0)`** (ECX is still the player) and return.
  2. if `ped->m_pIntelligence->m_TaskMgr.FindActiveTaskByType(1000 /*TASK_COMPLEX_KILL_PED_ON_FOOT*/)` != null return.
  3. if `ped->m_nPedType != 8` (GANG2) and `!sub_609380()` (recruit cheats 76/77/78, inlined byte tests) return.
  4. if `G.membership.IsMember(ped)` return.
  5. `CAEPedSpeechAudioEntity::SetCJMood(-1, 10000, 1, -1, -1)` (0x4E3ED0, cdecl); `this->Say(0x7F, 0, 1.0f, 1, 0, 0)`.
  6. `max = CStats::FindMaxNumberOfGroupMembers()`; `if (max >= pd->m_nScriptLimitToGangSize (+0x43, u8)) max = limit` (signed compare);
     if `CStats::GetStatValue(181) == 1.0f` or `== 2.0f` (exact, two separate calls): `max = 0` (0x60C93E-0x60C972).
  7. join = (`sub_609380()` && `G.membership.CountMembersExcludingLeader() < 7`) || `Count() < max` - where the `< max` test is done
     **twice** with two Count calls (0x60C99C-0x60C9DA) when the cheat branch did not decide.
  8. Join (0x60C9E0-0x60CBE1):
     a. `old = CPedGroups::GetPedsGroup(ped)`; if old: `old->membership.RemoveMember(ped)` (0x5FB210).
     b. `task = new CTaskComplexBeInGroup(FindPlayerPed(-1)->m_pPlayerData->m_nPlayerGroup, 0)` (CTask::operator new(0x28); null-safe);
        stack `CEventScriptCommand ev(3, task, 0)` (0x4B0A00); `ped->m_pIntelligence->m_eventGroup (+0x68).Add(&ev, 0)` (0x4AB420).
     c. `G.membership.AddFollower(ped)` (0x5F8020); `G.Process()` (0x5FC7E0); `ped->GiveWeaponWhenJoiningGang()` (0x5E8BE0).
     d. `e = new CEventNewGangMember(ped)` (CEvent::operator new(0x10), 0x608F70; null if allocation fails); stack
        `CEventGroupEvent ge(this, e)` (0x4ADFD0); `G.intelligence.AddEvent(&ge)` (0x5F7470).
     e. `ped->flags2 (+0x470) &= ~0x100000` (bit 20, bDrownsInWater).
     f. `CStats::IncrementStat(238, 1.0f)`; `CStats::DisplayScriptStatUpdateMessage(1, 336, 1.0f)`.
     g. `colour = (b[0x8D1345] << 24) | (b[0x8D1351] << 16) | (b[0x8D135D] << 8) | 0xFF` (three bytes of the HUD colour table);
        `h = CPools::ms_pPedPool->GetRef(ped)` (thunk 0x4442D0 -> .HOODLUM 0x156CD40: `(idx << 8) + slotByte`, idx = (ped - storage)/0x7C4);
        `blip = CRadar::SetEntityBlip(2, h, colour, 2, "CODEPLR")`; `ChangeBlipScale(blip, 2)`; `ChangeBlipColour(blip, colour)`;
        `SetBlipFriendly(blip, 1)`.
     h. `ped->flags (+0x474) |= 0x2000` (bit 13 of the third flag dword, bClearRadarBlipOnDeath).
     i. `G.membership.m_fMaxSeparation (+0x24 of membership) = CPedGroupMembership::ms_fPlayerGroupMaxSeparation` (0x86C6C0, 120.0f).
     j. `ped->Say(0x7E, 2500, 1.0f, 1, 0, 0)`; destroy `ge` then `ev`.
  9. Refuse (0x60CBE3): `ped->Say(0x7D, 2500, 1.0f, 1, 0, 0)`; stack `CEventDontJoinPlayerGroup ev(this)` (0x6090E0);
     `ped->m_pIntelligence->m_eventGroup.Add(&ev, 0)`; destroy (0x609140).
- Open question: what the stat-181 test means and the colour-table entry name (0x8D1345 etc.); behaviour is fully determined by the bytes.

## 0x60CC50 CPlayerPed::PlayerWantsToAttack  (catalogue: specified/REA, gr `.Reversed = false`, port level 7)
- Convention: `__thiscall void()`, plain `ret`. Caller: CPlayerPed::PlayerHasJustAttackedSomeone 0x60D5A0 (tail call).
- Data: group `g = CPedGroups::ms_groups[pd->m_nPlayerGroup]` (0xC09920 + id*0x2D4; +0x4 m_bMembersEnterLeadersVehicle (u8),
  +0x8 m_groupMembership, +0x30 m_groupIntelligence); `pd = m_pPlayerData` (+0x480, re-read each time).
- Behaviour:
  1. if `g.m_groupMembership.CountMembersExcludingLeader() < 1` return (0x60CC53-0x60CC70).
  2. if `g.m_bMembersEnterLeadersVehicle (+0x4) == 0` return (0x60CC85-0x60CC8D).
  3. `g.m_groupIntelligence.ReportAllBarScriptTasksFinished()` 0x5F8780.
  4. `t = m_pTargetedObject (+0x71C)`; if `CCamera::m_bUseMouse3rdPerson` (0xB6EC2E) and `t == null`: `t = +0x79C` (mouse-lock target).
  5. if `t != null`: if t is a ped (type bits `[+0x36] & 7 == 3`): `target = t`, go to 7. Else if `CTagManager::IsTag(t)` (0x49CCE0) return;
     else if t is an object (type 4) and `CObject::CanBeTargetted(t)` (0x59F320) return.
  6. `target = FindPedToAttack()` (0x60C5F0, thiscall).
  7. if `target != null`: `g.PlayerGaveCommand_Attack(this, target)` (0x5F7CC0, thiscall on the group).

## 0x60CD20 CPlayerPed::SetInitialState  (catalogue: specified/REA, gr `.Reversed = false`, port level 13)
- Convention: `__thiscall void(bool bGroupCreated [esp+4])`, `ret 4`. SEH frame (heap task/event constructions and two stack events).
  Callers: CGameLogic::RestorePlayerStuffDuringResurrection 0x442060, CPlayerPed constructor 0x60D5B0.
- Behaviour (in order):
  1. `CMBlur::ClearDrunkBlur()` (0x71D780); `CTimer::ms_fTimeScale (0xB7CB64) = 1.0f`.
  2. `m_nFlags (+0x1C) |= 1` (m_bUsesCollision); `physicalFlags (+0x40) |= 2` (bApplyGravity) (both written from values read before).
  3. `ClearAimFlag()` (0x5DEF20); `ClearLookFlag()` (0x5E1950); `flags1 (+0x46C) |= 0x2000` (bRenderPedInCar).
  4. if `m_pFire (+0x730)`: `CFire::Extinguish(m_pFire)` (0x5393F0, thiscall).
  5. `SetPedState(1)` (0x5E4500); `SetMoveState(1)` (0x5DEC00).
  6. `flags3 (+0x474) = (flags3 & ~0x42) | 0x04000000` (clear bDontRender bit1 and bIsBeingArrested bit6, set bCanExitCar bit26);
     `flags1 &= ~0x04000000` (bIsDucking) (values read before the two calls above? no: re-read at 0x60CD96).
  7. `m_pIntelligence->FlushIntelligence()` (0x601DA0); `RpAnimBlendClumpRemoveAllAssociations(m_pRwClump)` (0x4D6C00).
  8. `t = new CTaskSimplePlayerOnFoot()` (CTask::operator new(0x1C), ctor 0x685750; null-safe);
     `m_pIntelligence->m_TaskMgr.SetTask(t, 4, 0)` (0x681AF0).
  9. `flags1 &= ~0x100000` (bIsPedDieAnimPlaying); `m_nAnimGroup (+0x4D4) = 54`; if `m_pPlayerData`: `pd->+0x34 &= ~2` (m_bAdrenaline).
  10. `SetRealMoveAnim()` (0x60A9C0); `m_pStats (+0x59C)->m_nTemper (+0x25) = 50`.
  11. if `m_pAttachedTo (+0xFC)` and `!(m_nFlags & 1)`: `m_nFlags |= 1`. Then `m_pAttachedTo = 0`; `+0x784 = 0` (gr name m_nTurretAmmo).
  12. `f = new CTaskComplexFacial()` (0x20, 0x690D20); `m_TaskMgr.SetTaskSecondary(f, 3)` (0x681B60).
  13. if `!bGroupCreated` and `!(pd->+0x34 & 0x400)` (m_bGroupNeverFollow): with `G = ms_groups[pd->m_nPlayerGroup]`:
      `G.m_bMembersEnterLeadersVehicle (+0x4) = 1`; `G.intelligence.SetDefaultTaskAllocatorType(5)` (0x5FBB70);
      stack `ev = CEventPlayerCommandToGroup(0, 0)` (0x4B23D0) with its vtable then set to 0x86C710 (catalogue: CEventPlayerCommandToGroupAttack);
      `ev.ComputeResponseTaskType(&G)` (0x4B57A0); if `ev.WillRespond()` (0x4AC490): `e = new CEventPlayerCommandToGroup(2, 0)` (CEvent::operator new(0x1C))
      with vtable set to 0x86D120 (CEventPlayerCommandToGroupGather); `e->+0xE (int16) = ev.+0xE` (the computed response task type; written
      through `eax` even if the allocation failed and eax = 0); stack `CEventGroupEvent ge(this, e)`; `G.intelligence.AddEvent(&ge)`; `~ge`.
      Finally `ev.vtable = 0x86C710` and its destructor 0x4B2470.
  14. if `m_pPlayerData`: `CPlayerPedData::SetInitialState()` (0x56F210).
- Open question: the stack event's class (vtable 0x86C710, catalogue "CEventPlayerCommandToGroupAttack") vs the heap one (Gather) - behaviour is
  fully determined by the vtable addresses, which a port must reproduce.

## 0x60D1C0 CPlayerPed::EvaluateNeighbouringTarget  (catalogue: specified/REA, gr `.Reversed = false`, port level 4)
- Convention: `__thiscall void(CEntity* target [esp+4], CEntity** outTarget [esp+8], float* outScore [esp+0xC], float maxDist [esp+0x10],
  float baseHeading [esp+0x14], bool leftSide [esp+0x18])`, `ret 0x18`. Caller: CPlayerPed::FindNextWeaponLockOnTarget 0x60E530 (3).
- Constants: pi 0x858CB8, 2pi 0x858CBC, -pi 0x858CC0, 0.87266463f (50 deg) 0x86D1D0, -100000.0f 0x86D1E8.
- Behaviour:
  1. `d = sqrt((dz*dz + dy*dy) + dx*dx)` with `dk = target.k - this.k` (positions via matrix or placement); continue only if
     `d <= maxDist` (`test ah,0x41; jp`: > and NaN return; equality continues) (0x60D1C0-0x60D212).
  2. if `this->DoesTargetHaveToBeBroken(target, &m_aWeapons[(int8)m_nActiveWeaponSlot])` (0x60C0C0) return.
  3. `a = CGeneral::GetATanOfXY(target.x - cam.x, target.y - cam.y) - baseHeading` (0x53CC70, cdecl(float x, float y) -> ST0; cam =
     TheCamera matrix pos or placement) (0x60D26F-0x60D288).
  4. `while (a > pi) a -= 2pi;` then `while (a < -pi) a += 2pi;` (x87, unrounded).
  5. if NOT `|a| < 0.87266463f` return (0x60D2D3-0x60D2EF).
  6. score: leftSide ? (`a > 0` or NaN ? `-a` : -100000.0f) : (`a < 0` or NaN ? `a` : -100000.0f) (0x60D2F1-0x60D321).
  7. if `score > *outScore` (NaN/<= skip): `*outTarget = target; *outScore = score` (f32 store) (0x60D323-0x60D338).

## 0x60D350 CPlayerPed::ProcessGroupBehaviour  (catalogue: specified/REA, port level 37)
- Convention: `__thiscall void(CPad* pad [esp+4])`, `ret 4`. Caller: CPlayerPed::ProcessControl 0x60EA90.
- Fields: pd = `m_pPlayerData` (+0x480; re-read): +0x34 flags (bit 8 = 0x100 m_bGroupStuffDisabled), +0x88 u16
  m_nPadDownPressedInMilliseconds, +0x8A u16 m_nPadUpPressedInMilliseconds. `m_pTargetedObject` +0x71C, mouse target +0x79C.
  `ms(dt) = (uint16)_ftol2(CTimer::ms_fTimeStepNonClipped * 0.02f * 1000.0f)` (x87, two f32 fmuls, truncation).
- Behaviour:
  1. `t = m_pTargetedObject`; if `CCamera::m_bUseMouse3rdPerson` and t == null: `t = +0x79C`.
  2. If `CWorld::FindPlayerVehicle(-1, 0) == null`:
     - if `pad->GetGroupControlForward()` (0x541190): `pd->+0x8A += ms(dt)` (16-bit add); if `pd->+0x8A == 1200` (**exact equality**)
       and !GroupStuffDisabled: `DisbandPlayerGroup()` (0x60A0A0).
     - else: `v = pd->+0x8A`; if `v != 0 && v < 1200` (unsigned): if `t` is a ped (type 3) and (`t->m_nPedType == 8` or `sub_609380()`
       (recruit cheats)): if !GroupStuffDisabled `MakeThisPedJoinOurGroup(t)` (else nothing); otherwise `TellGroupToStartFollowingPlayer(1, 1, 0)`
       (0x60A1D0). Then `pd->+0x8A = 0`.
  3. If GroupStuffDisabled: return.
  4. If `FindPlayerVehicle(-1, 0) == null` (called again):
     - if `pad->GetGroupControlBack()` (0x5411B0): `pd->+0x88 += ms(dt)`; if `pd->+0x88 >= 1200` (unsigned) `DisbandPlayerGroup()`.
     - else: `v = pd->+0x88`; if `v != 0 && v < 1200`: if `t` is a ped with `m_nPedType == 8` (no cheat check here):
       `MakeThisPedJoinOurGroup(t)`, else `TellGroupToStartFollowingPlayer(0, 1, 0)`. Then `pd->+0x88 = 0`.
  5. If `(CTimer::m_FrameCounter & 0x1F) == 6`: for i = pool size-1 down to 0 over `CPools::ms_pPedPool` (stride 0x7C4, slot byte sign bit =
     empty): for each ped with `m_nPedType == 8`: `pdp = FindPlayerPed(-1)->m_pPlayerData`; `w = pdp ? pdp->m_pWanted : null` (no null check
     afterwards); if `w->m_nWantedLevel (+0x2C) > 0` (signed): `ped->m_acquaintance (+0x4E0).SetAsAcquaintance(4, CPedType::GetPedFlag(6))`
     (0x608DA0) else `ClearAsAcquaintance(4, GetPedFlag(6))` (0x608980) (0x60D4F8-0x60D594).
- Note the asymmetry kept from the original: the "forward" hold disbands only when the counter hits exactly 1200, the "back" hold at >= 1200.

## 0x60D850 CPlayerPed::ProcessWeaponSwitch  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall void(CPad* pad [esp+4])`, `ret 4`. Caller: CPlayerPed::ProcessControl 0x60EA90.
- Notation: `W[i] = m_aWeapons[i]` (+0x5A0 + i*0x1C: +0 type, +0xC totalAmmo); `act = (int8)m_nActiveWeaponSlot (+0x718)`;
  `C = pd->m_nChosenWeapon` (int8 at pd+0x20, pd = m_pPlayerData re-read each time); `mode = TheCamera.m_PlayerWeaponMode.m_nMode`
  (int16 at 0xB6F858). `ok(i) = W[i].type != 0 && W[i].HasWeaponAmmoToBeUsed() (0x73B2A0) && (!CGameLogic::IsCoopGameGoingOn() ||
  W[i].CanBeUsedFor2Player() (0x73DEF0))` (evaluated in this order).
- Behaviour:
  1. if `CDarkel::FrenzyOnGoing()` or `m_pAttachedTo (+0xFC)` or `m_pIntelligence->GetTaskJetPack()` -> go to 6.
  2. if `m_pTargetedObject (+0x71C)` or `pd->+0x34 & 8` (m_bFreeAiming) or `pd->m_bDontAllowWeaponChange (+0x85)` or `pd->+0x34 & 0x800`
     (m_bInVehicleDontAllowWeaponChange) -> go to 4.
  3. a. if `pad->CycleWeaponRightJustDown()` (0x540640): if mode in {34, 42, 7, 39, 8, 40, 51, 52, 46} -> 4. Else `C = act + 1`; while `C < 13`
        (signed int8): if `ok(C)` -> 4; `C++`. If the loop ends: `C = 0`. -> 4.
     b. else if `pad->CycleWeaponLeftJustDown()` (0x540610): if mode in {34, 7, 8, 51, 46} -> 4. Else `C = act - 1`; loop: if `C < 0` C = 12;
        if `C == 0` -> 4; if `ok(C)` -> 4; `C--`.
  4. `info = CWeaponInfo::GetWeaponInfo(W[act].type, 1)`; if `info->m_eFireType (+0x0) == 0` (melee) -> 6.
     if `pad->GetWeapon(this)` (0x540180) != 0 and `W[act].type == 38` (minigun) -> 6. if `W[act].totalAmmo > 0` -> 6.
     (out of ammo:) if mode in {34, 7, 8, 51} -> **return** (skips 6).
     if `W[act].type == 40` (detonator) and `W[8].type == 39` (satchel): `C = 8`; else `C = act - 1`.
  5. Search down (0x60DB65): while `C >= 0`: accept if (`C == 5` and `W[5].type == 5`) or (`W[C].totalAmmo > 0` and C not in {18, 17, 16});
     else `C--`. If C becomes negative: `C = 0`.
  6. if `C != act`: `t = m_pIntelligence->GetTaskUseGun()` (called up to three times); if t and `t->+0x10` (byte) is 2 or 3 -> return;
     if t and `t->+0x10 == 4` and `t->+0x2C != 0` -> return. `RemoveWeaponAnims(act, -1000.0f)` (0x5F0250); if `C != -1`:
     `MakeChangesForNewWeapon(W[C].type)` (0x60B460).

## 0x60DC50 CPlayerPed::FindWeaponLockOnTarget  (catalogue: specified/REA, gr `.Reversed = false`, port level 10)
- Convention: `__thiscall bool()`, return AL, plain `ret`. Caller: CTaskSimplePlayerOnFoot::ProcessPlayerWeapon 0x6859A0.
- Notation: `info = GetWeaponInfo(W[act].type, GetWeaponSkill())`, `R = info->m_fWeaponRange (+4)`; `SET(e)` = `CleanUpOldReference(&+0x71C)` if the old
  m_pTargetedObject is non-null, `+0x71C = e`, `e->RegisterReference(&+0x71C)`, `pd->+0x85 = 1`, return true.
- Behaviour:
  1. Existing target `t = m_pTargetedObject (+0x71C)`: `d = sqrt(dy*dy + dx*dx)` (2D, t - this); if `TargetWeaponRangeMultiplier(t, this) * R >= d`
     (`test ah,1; je`: NaN fails) return true; else `CleanUpOldReference` (if non-null), `+0x71C = 0`, return false.
  2. No target: `best = null`, `bestScore = -10000.0f`; `H = GetATanOfXY(m_matrix->forward.x, m_matrix->forward.y)`.
     `pad = pedType 0 ? GetPad(0) : pedType 1 ? GetPad(1) : null`; if `|pad->GetPedWalkLeftRight()| > 60` or `|pad->GetPedWalkUpDown()| > 60`
     (int16 -> float): `H = LimitRadianAngle(GetRadianAngleBetweenPoints(0, 0, -LeftRight, UpDown) - dword[0xB6F178] + pi/2)` (0x53CBE0; the
     camera float at TheCamera+0x150; LeftRight/UpDown are re-read).
  3. Spraycan (`W[act].type == 41`): `A = m_matrix ? atan2(-fwd.x, fwd.y) : m_placement heading (+0x10)`; `P = pos + fwd * 8.0f` (fwd read from m_matrix
     before its null test); `bestAng = pi`; `CWorld::FindObjectsInRange(P, 8.0f, false, &n, 15, list, 1, 0, 0, 0, 0)` (buildings only). For each e:
     if `CTagManager::IsTag(e)` and `CTagManager::GetAlpha(e) < 255`: `d = e - this` (3D); require `(dx*dx + dz*dz) + dy*dy < R*R`;
     `ang = atan2(-dx, dy) - A`, wrapped once (`< -pi` -> +2pi; else `> pi` -> -2pi); take e if `ang < bestAng` (signed, not absolute) or nothing taken yet
     (`bestAng = ang`). If a tag was taken and `!LOSBlockedBetweenPeds(this, tag)`: SET(tag).
  4. Peds (pool top-down): same exclusions as FindNextWeaponLockOnTarget (null, self, state 54/55, vehicle rule, bNeverEverTargetThisPed, players when
     bPlayersCannotTargetEachOther, AreInSameGroup(p, this)). `a = GetATanOfXY(p.x - this.x, p.y - this.y) - H`, wrapped with while loops into [-pi, pi];
     require `|a| < PLAYER_MAX_TARGET_VIEW_ANGLE (0x8D243C, 140.0f) * 0.008726646f`. `n = normalised m_matrix->forward`;
     `Q = pos - n * dword[0x8D2440]`; `a2 = GetATanOfXY(p.x - Q.x, p.y - Q.y) - H` wrapped; require `|a2| < dword[0x8D2438] (90.0f) * 0.008726646f`.
     `d = |p - this|` (3D Magnitude); `m = TargetWeaponRangeMultiplier(p, this) * R`; if `d < m`: `EvaluateTarget(p, &best, &bestScore, m, H, false)` (0x60D020).
  5. Objects (top-down): skip empty/null, `!CanBeTargetted()`, `m_nObjectFlags & 0x40`, no RwObject, `!CanIKReachThisTarget(o.pos, &W[act], true)`;
     `EvaluateTarget(o, &best, &bestScore, R, H, true)`.
  6. If coop: vehicles (top-down): skip empty/null, `physicalFlags & 0x20000000`, `+0x594 == 10`, CanIKReach fails; `EvaluateTarget(v, ..., R, H, true)`.
  7. If best: SET(best) (identical code for peds and others). Else return false.

## 0x60E530 CPlayerPed::FindNextWeaponLockOnTarget  (catalogue: specified/REA, gr `.Reversed = false`, port level 9)
- Convention: `__thiscall bool(CEntity* curTarget [esp+4], bool leftSide [esp+8])`, `ret 8`, return AL. SEH frame; prologue goes through a
  protection stub at 0x4038AB that computes the SEH handler (`[0x58713F] ^ [0x156076C]`) and returns to 0x60E537 (flags/EDI preserved).
  Caller: CTaskSimplePlayerOnFoot::ProcessPlayerWeapon 0x6859A0 (5). The curTarget argument slot is reused as a loop temporary later.
- Behaviour:
  1. `range = GetWeaponInfo(W[act].type, GetWeaponSkill())->m_fWeaponRange (+4)`; `best = null`; `bestScore = -10000.0f`.
     `CGeneral::GetATanOfXY(m_matrix->forward.x, m_matrix->forward.y)` is called and its result **discarded** (0x60E58F-0x60E594).
  2. `heading = curTarget ? GetATanOfXY(curTarget.x - cam.x, curTarget.y - cam.y) : GetATanOfXY(dword[0xB6F9AC], dword[0xB6F9B0])`
     (cam = TheCamera matrix pos or placement; the two globals are CCamera fields at +0x984/+0x988) (stored f32).
  3. Peds (`CPools::ms_pPedPool`, from the highest slot down, stride 0x7C4): `p` = slot ped; EBP is set to 0 for empty slots. Skip if p is null,
     `this`, curTarget, state 54/55; if `flags1 & 0x100` (in vehicle) skip unless `m_pVehicle` is non-null and (`m_nVehicleClass == 9` or vehicle byte
     +0x42D & 4); skip if `flags2 (+0x470) & 0x10000000` (bNeverEverTargetThisPed); skip if `CPedGroups::AreInSameGroup(p, this)`; skip if p is a
     player (type 0/1) and `CGameLogic::bPlayersCannotTargetEachOther` (0x96A8B2); skip if `CPlayerPed::LOSBlockedBetweenPeds(this, p)` (0x60B550);
     skip if `!this->CanIKReachThisTarget(p.pos /*by value*/, &W[act], true)` (0x609F80). Else
     `this->EvaluateNeighbouringTarget(p, &best, &bestScore, CWeapon::TargetWeaponRangeMultiplier(p, this) * range, heading, leftSide)`.
  4. Objects (`ms_pObjectPool`, top-down, stride 0x19C): skip empty/null, `!CObject::CanBeTargetted()`, `m_nObjectFlags & 0x40`, `m_pRwObject == null`,
     `!CanIKReachThisTarget(o.pos, &W[act], true)`; else EvaluateNeighbouringTarget(o, ..., `TargetWeaponRangeMultiplier(EBP, this) * range`, ...).
     **EBP is stale here**: it still holds the ped from slot 0 of the ped loop (0 if that slot was empty or null; `curTarget` if the ped pool has
     size 0) - the original passes that as the "victim" to TargetWeaponRangeMultiplier. Keep.
  5. Only if `CGameLogic::IsCoopGameGoingOn()`: vehicles (`ms_pVehiclePool`, top-down, stride 0xA18): skip empty/null, `physicalFlags & 0x20000000`,
     `+0x594 == 10`; CanIKReach test; EvaluateNeighbouringTarget with the same stale-EBP multiplier.
  6. If `best == null` return false. If best is a ped and `GetWeaponInfo(W[act].type, 1)->m_eFireType != 0`: `g = GetPedsGroup(best)`; if g:
     if `!AreInSameGroup(best, this)`: `e = new CEventGunAimedAt(this)`; stack `CEventGroupEvent(best, e)`; `g->m_groupIntelligence.AddEvent`; destroy.
     Else (no group): stack `CEventGunAimedAt(this)` added to `best->m_pIntelligence->m_eventGroup` (Add(&ev, 0)); destroy.
  7. `m_pTargetedObject (+0x71C)`: CleanUpOldReference on the old one if non-null; set best; RegisterReference. `pd->+0x85 = 1`
     (m_bDontAllowWeaponChange). Return true.
