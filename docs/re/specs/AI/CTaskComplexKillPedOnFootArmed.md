# CTaskComplexKillPedOnFootArmed (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x626FA0 CTaskComplexKillPedOnFootArmed::~CTaskComplexKillPedOnFootArmed  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x621250 CTaskComplexKillPedOnFootArmed::Destructor (0x626FA3); if `flags & 1` (0x626FA8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x626FB0); returns this (0x626FB8).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x626FC0 CTaskComplexKillPedOnFootArmed::CreateSubTask  (catalogue: specified/REA, port level 10)
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`; SEH frame (a local CTaskSimpleStandStill). Callers: CreateFirstSubTask 0x62BF00, CreateNextSubTask 0x62C190 (7), ControlSubTask 0x62CCE0. Fields (gr TaskComplexKillPedOnFootArmed.h): +0xC m_target, +0x10 m_shootTimer, +0x14 m_lastAttackTime, +0x20 m_lengthOfDuck, +0x35 m_aimImmediate, +0x50 m_losBlockedTime, +0x58 m_needToMoveInCloserTime. "Seek(r)" = `new(0x4C) CTaskComplexSeekEntity<Standard>(m_target, 50000, 1000, r, [0x859E30]=2.0f, [0x859E34]=2.0f, 1, 1)` 0x46AC10. All `new` failures yield 0.
- First: `wi = CWeaponInfo::GetWeaponInfo(ped's active weapon type, CPed::GetWeaponSkill(ped))` 0x743C60.
- By taskType:
  - `0x19F` (TASK_SIMPLE_DUCK): `new(0x28) CTaskSimpleDuck(0, (u16)m_lengthOfDuck, -1)` 0x691FC0.
  - `0xCB` (STAND_STILL): `new(0x20) CTaskSimpleStandStill([0x86DB24] = 20000 (int), false, false, 8.0f)`.
  - `0xCA` (PAUSE): a local `CTaskSimpleStandStill(0, false, false, 8.0f)` is constructed, `ProcessPed(local, ped)` 0x62F370 run once, destroyed (0x62F360); then `new(0x18) CTaskSimplePause(100)` 0x48E750.
  - `0x38B` (SEEK_ENTITY): `t = m_needToMoveInCloserTime`. If `t == 0` or `e < 3000` -> Seek(6.0f), where `e = (float)(u32)(now - t)` (fild, +2^32 if negative, saved f32) compared with `[0x859A94]=3000.0f` (`test ah, 5; jp`). Else if `e > [0x859C38]=8000.0f`: flank - `d = normalise(T.x - P.x, T.y - P.y, 0)` (T = target pos, P = ped pos); `a = -d.y * 1.5f`, `b = d.x * 1.5f` ([0x858CE8]); `P1 = (T.x + a, T.y + b, T.z)`, `P2 = (T.x - a, T.y - b, T.z)`; if `CWorld::GetIsLineOfSightClear(&T, &P1, 1, 1, 0, 0, 1, 0, 0)` 0x56A490 -> `new(0x28) CTaskComplexGoToPointAndStandStill(6, &P1, [0x86FC84]=0.5f, [0x86FC88]=2.0f, 0, 0)`; else if LOS(&T, &P2) clear -> same with P2; else Seek(1.0f). Else (3000 <= e <= 8000) -> Seek(`6.0f - (e - 3000.0f) * [0x858CDC]=0.001f` (fld e; fsub; fmul; fsubr [0x858B44]=6.0f)).
    After any 0x38B result (also NULL): if `m_target != NULL && CPed::IsPlayer(m_target)`: if `ped->m_nPedType (+0x598) == 6` (COP): `r = CPed::Say(ped, (FindPlayerWanted(-1)->m_nWantedLevel (+0x18, u8) > 1) ? 0x2B : 0xC7, 0, 1.0f, 0, 0, 0)`; if `(int16)r >= 0` -> `CPed::Say(m_target, 0x2C, 3500, 1.0f, 0, 0, 0)`. Else if `CPedGroups::GetPedsGroup(ped)` -> `Say(ped, 0x2B, 0, 1.0f, 0, 0, 0)`.
  - `0x3FC` (GUN_CTRL): if `(wi->m_nFlags (+0x18) >> 8) & 1` (throw) -> `new(0x1C) CTaskSimpleThrowControl(m_target, NULL)` 0x61F8B0. Else: `cmd = CTaskComplexKillPedOnFootArmed::LineOfSightClearForAttack(this, ped)` 0x621500 ? 3 : 0 - written into the low byte of the ped argument slot, whose upper 3 bytes (the ped pointer's) are passed along in the dword; `p = new(0x3C)`; `EAX = p ? CTaskSimpleGunControl(p, m_target, NULL, NULL, cmd-dword, 5, -1) : 0` 0x61F3F0. Then OUTSIDE the NULL check: `EAX->+0x3A = (m_aimImmediate != 0)` (original bug: NULL write at 0x3A on allocation failure); `m_aimImmediate = 0`; `m_shootTimer = now + CGeneral::GetRandomNumberInRange(4000, 8000)` 0x407180 (now read before the call); if `GetPedsGroup(ped)` -> `Say(ped, 0xCF, 0, 1.0f, 0, 0, 0)`. Both 0x3FC variants then set `m_lastAttackTime = now; m_losBlockedTime = 0`.
  - anything else -> 0.
- gta-reversed: plugin::CallMethodAndReturn stub (`.Reversed = false`); no body to compare.
