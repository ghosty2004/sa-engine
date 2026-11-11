# CTaskSimpleUseGun (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x624DA0 CTaskSimpleUseGun::~CTaskSimpleUseGun  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x61DF30 CTaskSimpleUseGun::Destructor (0x624DA3); if `flags & 1` (0x624DA8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x624DB0); returns this (0x624DB8).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x62A380 CTaskSimpleUseGun::ProcessPed  (catalogue: specified/REA, port level 20)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`, SEH frame (0x74 locals; a CTaskSimpleFight lives on the stack at local+0x5C). Returns AL: 0 at 0x62A5B3,
  1 at 0x62AF3C. EBX = 0 for most of the body.
- this (gr TaskSimpleUseGun.h): +8 m_IsFinished, +9 m_IsInControl, +0xA m_HasMoveControl, +0xB m_HasFiredGun, +0xC m_IsLOSBlocked, +0xD bit0 fire-right-this-frame /
  bit1 fire-left-this-frame, +0xE m_SkipAim, +0xF m_NextCmd (int8), +0x10 m_LastCmd (int8), +0x14/+0x18 m_MoveCmd, +0x1C m_TargetEntity, +0x20 m_TargetPos,
  +0x2C m_Anim, +0x30 m_WeaponInfo, +0x34 u16 m_BurstLength, +0x36 i16 m_BurstShots, +0x38 u8 m_CountDownFrames, +0x39 m_IsArmIKInUse, +0x3A m_IsLookIKInUse.
  Commands: 0 NONE, 1 AIM, 2 FIRE, 3 FIREBURST, 4 RELOAD, 5 PISTOLWHIP, 6 END_LEISURE, 7 END_NOW. All cmd compares are SIGNED byte compares.
- Anim (CAnimBlendAssociation): +0x14 hierarchy (+0x10 total time), +0x18 blendAmount, +0x1C blendDelta, +0x20 currentTime, +0x28 timeStep, +0x2C u16 animId,
  +0x2E u16 flags (bit0 PLAYING, bit2 FREEZE_LAST_FRAME, bit10 tested at 0x62AC59). WeaponInfo: +0 m_nWeaponFire, +0x18 flags (bit1 bAimWithArm, bit2 b1stPerson,
  bit8 bThrow, bit10 bContinuosFire, bit11 bTwinPistol, bit18 per gr = bExpands), +0x40/+0x44/+0x48 loop start/end/fire, +0x4C/+0x50/+0x54 the ducking (loop2)
  versions, +0x58 m_fBreakoutTime. Ped: +0x480 m_pPlayerData (pd+0x20 u8 chosen weapon, pd+0x2C f32 attack-button counter), +0x718 i8 active slot, weapons at
  +0x5A0 stride 0x1C (+0 type, +4 state: 2 = RELOADING), +0x46C bit26 bIsDucking, +0x474 bits 15/17/18 and +0x476 bit0 (= +0x474 bit16), +0x528 bit2 (m_pedIK
  arm flag), +0x540 m_fHealth, +0x558 m_fCurrentRotation, +0x55C m_fAimingRotation.
- Helpers (unnamed, proposed): 0x608F00 `CWeaponInfo::GetAnimLoopStart(bool duck)` (duck ? +0x4C : +0x40, ST0), 0x608F20 `GetAnimLoopEnd(duck)` (+0x50 / +0x44),
  0x61C150 `GetAnimLoopFire(duck)` (+0x54 / +0x48); 0x406D70 `CVector::operator-=`; 0x40FE30 `operator+(out, a, b)` (computes z, y, x); 0x822130 = CRT `_CIpow`
  (x87 st1^st0); 0x6240B0 = a CTaskSimpleFight method `(ped, const CVector*)` (thiscall `ret 8`, pistol-whip hit test; name UNCLEAR).
- Behaviour (labels are addresses):
  1. IK bookkeeping: if +0x3A and `!g_ikChainMan.IsLooking(ped)` 0x6181A0 -> +0x3A = 0; if +0x39 and `!g_ikChainMan.IsArmPointing(0, ped)` 0x6182B0 -> +0x39 = 0;
     +0xD = 0 (whole byte, 0x62A3DD).
  2. Weapon info (0x62A3DA). `cur = CWeaponInfo::GetWeaponInfo(ped->weapons[slot].type, ped->GetWeaponSkill())` 0x743C60 / 0x5E6580.
     a. m_WeaponInfo == 0: if pd and `pd+0x20 != ped+0x718` (byte compare) -> return 0 (0x62A3F6). m_WeaponInfo = cur; if `cur->m_nWeaponFire == 0` or bThrow:
        m_IsFinished = 1, m_WeaponInfo = 0, go to step 12b (0x62AF27, no RemoveStanceAnims). Else m_MoveCmd = {0, 0} (int stores).
     b. m_WeaponInfo != 0 and != cur: `this->MakeAbortable(ped, 1, nullptr)` (vtbl+0x18, result ignored) (0x62A485).
  3. If !b1stPerson or pd == 0: `ped+0x474 |= 0x8000` (0x62A49E).
  4. If m_IsFinished -> step 12a.
  5. If pd and `pd+0x2C > 0.0` (`fcomp qword 0.0 [0x859EF8]`, `test ah,0x41; jne` skip on <=, NaN): `pd+0x2C = _CIpow(0.96f [0x8D2E64], CTimer::ms_fTimeStep
     [0xB7CB5C]) * pd+0x2C` (0x62A4CD-0x62A4E1; 0x8D2E64 is a .data variable initialised to 0.96f).
  6. If !m_IsInControl (0x62A4E9 -> 0x62AEFA): m_HasFiredGun = 0; m_MoveCmd = {0, 0}; `c = m_CountDownFrames; m_CountDownFrames = c - 1`; if c != 0 return 0;
     else m_IsFinished = 1 and step 12a.
  7. In control: m_CountDownFrames = 0xFF (0x62A4EF). duck = bIsDucking. Blocked test (0x62A4F3-0x62A52C):
     `blocked = duck ? (ped+0x474 & 0x40000) : ((ped+0x476 & 1) && !bAimWithArm)`.
     - blocked (0x62A532): if m_LastCmd == 5 -> 8. If m_NextCmd == 5 and m_Anim == 0 -> 8. Otherwise (0x62A606): if m_Anim: `anim->blendDelta = -4.0f`
       (0x62A623 immediate), `flags &= ~1`, `flags |= 4`; else if m_NextCmd not in 1..3 -> 8. Then (both) `AbortIK(ped)` 0x61DFA0, m_IsLOSBlocked = 1, if m_LastCmd == 3
       then m_LastCmd = 1; go to 11.
     - not blocked (0x62A5D0): if m_NextCmd == 5 and m_LastCmd <= 3: if m_Anim `anim->SetDeleteCallback(0x4D1390 CDefaultAnimCallback, 0)` 0x4CEBC0; m_Anim = 0;
       AbortIK(ped). Then 8.
  8. (0x62A54D) If m_Anim == 0: `StartAnim(ped)` 0x624F30; if still 0 `AbortIK(ped)`; go to 11. Else 9 with a = m_Anim.
  9. By m_LastCmd (0x62A657):
     - 4 RELOAD: if a->animId is 0xE2 or 0xE3 -> 10; else if `a->blendDelta >= 0.0f` (0x62AC82, `test ah,1`, NaN counts as <) `a->blendDelta = -4.0f`; -> 10.
       (gr's commented code tests the opposite: it fades the reload anims.)
     - 5 PISTOLWHIP (0x62A685): if `blendAmount > 0.9f [0x858C20]` (strict) and `blendDelta >= 0.0f [0x858B50]` and `cur > T[duck]` (T = f32 table 0xC17738,
       strict) and `cur - a->timeStep < T[duck]` (0x62A6CE-0x62A6E3; the mask in `test cl, ah` is CL = 5 = m_LastCmd, i.e. `test ah,5; jp`): `v = *ped->m_matrix *
       V[idx]` (CMatrix * vec 0x59C890, V = CVector table 0xC177D0 stride 12, idx = u8 table 0xC17780[duck]); stack fight = `CTaskSimpleFight(m_TargetEntity, 0xB,
       20000)` 0x61C470; fight+0x1C = m_Anim, +0x24 = 0x10, +0x25 = duck, +0x27 = 0xB; `0x6240B0(fight, ped, &v)`; fight+0x1C = 0; `~CTaskSimpleFight` 0x61C530.
       Then if m_TargetEntity: `d = VectorSub(targetPos, pedPos)` 0x40FE60; `ped->m_fAimingRotation = atan2(-d.x, d.y)` (`fchs; fpatan`, 0x62A7AD-0x62A7C7). -> 10.
     - <= 0: only for 0 (negative -> 10): if `blendAmount > 0.0f` (strict) and `blendDelta >= 0.0f` -> blendDelta = -4.0f (0x62AC72-0x62AC92). -> 10.
     - >= 6 -> 10. 1..3 with `blendDelta < 0.0f` -> 10. 1..3 otherwise: firing logic 9a-9f.
     9a. s = GetAnimLoopStart(duck), e = GetAnimLoopEnd(duck), f = GetAnimLoopFire(duck) (each stored f32), `f2 = (e - s) * 0.5f [0x858B8C] + f` (0x62A834-0x62A84B).
     9b. If m_LastCmd in {2,3} (0x62A855): if `blendAmount >= 0.99f [0x862CD0]` (NaN too) and weapon state != 2: if not PLAYING and `cur == s` exactly -> flags |= 1.
         Else (amount < 0.99 or reloading): if PLAYING and `cur >= s` and `cur - step < s` -> flags &= ~1, `SetCurrentTime(s)` 0x4CEA80.
     9c. bContinuosFire (0x62A8D9): if m_LastCmd in {2,3} and `cur > s` and `cur < e` (strict) and PLAYING: if m_HasFiredGun and !(m_LastCmd == 3 and
         m_BurstShots > 0) and m_NextCmd not in {2,3}: flags &= ~1, blendDelta = -4.0f. Otherwise: +0xD |= 1; m_HasFiredGun = 1; if m_NextCmd > m_LastCmd
         then m_LastCmd = m_NextCmd; m_NextCmd = 0; if m_LastCmd == 3 and m_BurstShots > 0 -> m_BurstShots-- else m_BurstShots = 0.
         Not continuous (0x62A99A): if PLAYING and `cur > f` and `cur - step <= f` and m_LastCmd in {2,3}: +0xD |= 1, m_HasFiredGun = 1, if m_BurstShots > 0 decrement.
         Then if bTwinPistol and !(ped+0x474 & 0x20000) and PLAYING and `cur > f2` and `cur - step <= f2` and m_LastCmd in {2,3}: +0xD |= 2, m_HasFiredGun = 1,
         if m_BurstShots > 0 decrement.
     9d. (0x62AA48) If NOT playing: unless (m_LastCmd == 1 and m_NextCmd <= 1): if `total > cur` or `blendDelta >= 0` (0x62AA68-0x62AA88): if m_LastCmd > 4 or
         m_NextCmd > 4: (bit 18 of wi flags clear -> blendDelta = -4.0f) else (flags |= 1; if `cur <= s` SetCurrentTime(e) - note: end time, 0x62AAFB);
         else if `blendAmount > 0.0f` and `blendDelta >= 0.0f` and weapon state != 2 -> flags |= 1. Then (0x62AB05): if m_NextCmd in {2,3}: m_LastCmd = m_NextCmd,
         m_NextCmd = 0, and for 3 m_BurstShots = m_BurstLength; else if m_LastCmd == 1 and m_NextCmd != 1 -> m_LastCmd = 0.
         If playing (or the AIM/<=1 case) (0x62AB35): if m_LastCmd == 1 and PLAYING: if `cur >= s`: stop when `cur - step < s`, else stop unless `cur + step < s`;
         if `cur < s`: stop unless `cur + step < s`. stop = flags &= ~1, SetCurrentTime(s).
     9e. (0x62AB80) If `cur > e` and `cur - step <= e` (crossed the loop end): n = m_NextCmd; refire if n in {2,3} or (m_LastCmd == 3 and m_BurstShots > 0 and
         n != 4): SetCurrentTime(s); PLAYING = (weapon state != 2); if n in {2,3}: if n > m_LastCmd m_LastCmd = n; if n == 3 and m_BurstShots == 0 m_BurstShots =
         m_BurstLength; m_NextCmd = 0. Else if n == 1: SetCurrentTime(s), flags &= ~1, m_LastCmd = 1, m_NextCmd = 0.
     9f. (0x62AC36) If `cur > wi->m_fBreakoutTime` (strict) and m_NextCmd == 7: m_IsFinished = 1; blendDelta = (anim flags bit10 ? -1.0f : -4.0f).
  10. Aim / skip-aim (0x62AC99): m_SkipAim = 0; dead = 0. If ped+0x528 bit2 is set:
     - target point P: m_TargetEntity is a ped (type & 7 == 3) -> `GetBonePosition(&P, 3, 0)` 0x5E4280 and dead = (`health <= 0.0f`, NaN false); other entity ->
       its position; no entity and ped->IsPlayer() and `TheCamera.Cams[0].m_nMode` (u16 0xB6F1A8) == 0x35 -> `TheCamera.Find3rdPersonCamTargetVector(20.0f,
       pedPos + (0, 0, 0.7f), &tmp, &P)` 0x514970; else P = m_TargetPos.
     - If `P.x != 0 or P.y != 0` (exact compares, NaN counts as non-zero): P -= pedPos; `P.Normalise()` 0x59C910; `h = atan2(-P.x, P.y) - m_fCurrentRotation`;
       if `h > 3.14159274f [0x858CB8]` h -= 6.28318548f [0x858CBC] else if `h < -3.14159274f [0x858CC0]` h += 6.28318548f [0x858CBC]. `k = (float)dead *
       0.69813174f [0x86D624]`; m_SkipAim = 1 if `h > 2.00712872f [0x86D61C] - k` or `h < -2.26892805f [0x86D620] + k` (strict). `at = ped->GetAtDirection()` 0x50E420
       (always called); if dead and `(P.y*at.y + P.z*at.z) + P.x*at.x < -0.8f [0x858C9C]` -> m_SkipAim = 1.
     - (0x62AEB6) If NOT ducking and ped+0x476 bit0 -> m_SkipAim = 1. (gr's comment says `bIsDucking && bDuckRightArmBlocked`; the bytes test `!ducking`.)
     Then if m_LastCmd in 1..3 and !m_SkipAim -> `AimGun(ped)` 0x61ED10 (0x62AEF0) else `AbortIK(ped)` (0x62A565).
  11. (0x62A56D) If !bAimWithArm or ducking or m_HasMoveControl -> `SetMoveAnim(ped)` 0x61E3F0. Then n = m_NextCmd: if n < 6 and NOT (n == 4 and m_LastCmd in {2,3})
     -> m_NextCmd = 0 (a RELOAD is kept only right after FIRE/FIREBURST; gr's comment code has this inverted). m_IsInControl = 0; return 0.
  12. a: `RemoveStanceAnims(ped, -4.0f)` 0x61E8E0; b: `AbortIK(ped)`; if pd `pd+0x2C = 0` (int store); return 1.
- gta-reversed: `plugin::CallMethodAndReturn` (cpp:425), with a commented-out partial reading that differs from the bytes at least in steps 7 (the anim branch also
  aborts IK and sets LOS-blocked), 9 RELOAD, 10 (ducking test) and 11 (RELOAD keep). Port from this spec.
- Open questions: 0x6240B0 semantics/name (pistol-whip strike against the target; specify separately); meaning of the u8/f32/CVector statics 0xC17780 / 0xC17738 /
  0xC177D0 (written by the melee loader 0x5BEDC0) - values are data, the port reads them; ped+0x474 bits 17/18 and +0x476 bit0 names (gr: see CPed flags).
