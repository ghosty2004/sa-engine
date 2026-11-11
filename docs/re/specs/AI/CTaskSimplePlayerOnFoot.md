# CTaskSimplePlayerOnFoot (AI) - RE-SPEC-3 group I4

## 0x68B0C0 CTaskSimplePlayerOnFoot::`scalar deleting destructor'  (catalogue name: CTaskSimplePlayerOnFoot::~CTaskSimplePlayerOnFoot; catalogue: specified/REA, port level 2)
- NAME CORRECTION: catalogued as `CTaskSimplePlayerOnFoot::~CTaskSimplePlayerOnFoot`; it is slot 0 of the CTaskSimplePlayerOnFoot vtable 0x8708EC (vt lookup), i.e. the MSVC `scalar deleting
  destructor' (`??_G`), not the complete-object destructor.
- Convention: `__thiscall CTaskSimplePlayerOnFoot* (this = ECX, uint32 flags [esp+4])`, returns this in EAX, `ret 4`. Callers: none direct (virtual only).
- Behaviour: `CTaskSimplePlayerOnFoot::Destructor 0x6857D0(this)` (thiscall, +3); `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`;
  `test byte [esp+8],1` at +8, call at +0x10); return this (`ret 4` at +0x1B).
- Complete dtor callee: 0x6857D0 (gr-reversed). Note gr TaskSimplePlayerOnFoot.h:59 labels 0x68B0C0 as the destructor; it is the deleting wrapper.
- gta-reversed: compiler-generated (virtual dtor + pool operator delete); matches. Open questions: none.

## 0x687530 CTaskSimplePlayerOnFoot::PlayerControlFighter  (catalogue: specified/REA, port level 8)
- `__thiscall void (this: CTaskSimplePlayerOnFoot*, CPlayerPed* player [esp+4])`, `ret 4`, `sub esp,0x28`. Caller: ProcessPed 0x688810.
  `FC(cmd)` = `T->ControlFight(nullptr, cmd)` 0x61C5E0 (thiscall, 2 stack args) followed by return. PD = `player->m_pPlayerData (+0x480)`
  (+0xC = move X, +0x10 = move Y, both f32). Camera heading `CH = [0xB6F178]` (TheCamera+0x150, f32).
- 1. `T = player->m_pIntelligence->GetTaskFighting()` 0x600F30; if T == 0 return. `armed = (GetWeaponInfo(active weapon type, 1)
  0x743C60 ->+0 != 0)` (first dword = fire type; non-zero = not melee). `pad = CPlayerPed::GetPadFromPlayer(player)` 0x609560.
- 2. `LR = (f32)(i16)pad->GetPedWalkLeftRight()` 0x53FC90, `UD = (f32)(i16)pad->GetPedWalkUpDown()` 0x53FD30 (fild of the
  sign-extended short); `X = LR * [0x858B88]=0.0078125f`, `Y = UD * 0.0078125f`; `step = CTimer::ms_fTimeStep [0xB7CB5C] * [0x858CA8]=0.07f`.
  `m` (f32 local, stored over the argument slot) is only written in the branches below.
- 3. If `CGameLogic::IsPlayerUse2PlayerControls(player)` 0x442020 (cdecl): `m = sqrt(X*X + Y*Y)` (X^2 first); if `m > 0.0f` (strict):
  `a = CGeneral::LimitRadianAngle(GetRadianAngleBetweenPoints(0, 0, -X, Y) 0x53CBE0 - CH)` 0x53CB50; `dir = (-sin a, cos a, 0)` (fsin,
  fchs; fcos). If `CGameLogic::IsPlayerAllowedToGoInThisDirection(player, dir (by value), 0.0f)` 0x441E10: with `R = player->+0x14`
  (right +0..+8, forward +0x10..+0x18): `X = ((cos a*R.right.y + (-sin a)*R.right.x) + R.right.z*0.0f) * m`;
  `Y = -(((cos a*R.fwd.y + (-sin a)*R.fwd.x) + R.fwd.z*0.0f) * m)` ([0x858B50]=0.0f, fmul kept); else `X = Y = 0`.
  Else (one-player): if `player->+0x71C` (targeted entity) != 0 or (CCamera::m_bUseMouse3rdPerson [0xB6EC2E] and `pad->GetTarget()`
  0x540670): `m = sqrt(X*X + Y*Y)`; if `m > 0`: if `m > 1.0f` `m = 1.0f`; `a` as above (no direction check); X and Y transformed with
  the same formula.
- 4. TARGETING = `player->+0x71C != 0` or (m_bUseMouse3rdPerson and pad->GetTarget()) (GetTarget called again).
- 5. FREE (not TARGETING, 0x6877D5): `a2 = GetRadianAngleBetweenPoints(0, 0, -LR, UD)` (raw stick values). `m = sqrt(X*X + Y*Y)`.
  If `m > 1.0f`: `m = 1.0f` and go to SMOOTH. Else if `m == 0.0f` (`test ah,0x44; jp` = not equal or NaN goes to SMOOTH): `PD->+0xC = 0;
  PD->+0x10 = 0`; go to TIMER.
  SMOOTH (0x687821): skipped (go to TIMER) when `Y > 0` or Y is NaN (`test ah,0x41; jp`). Otherwise
  `v = -m`; `t = v - PD->+0x10`; if `fabs(t) > step` (strict; else and NaN -> `PD->+0x10 = v`): `PD->+0x10 = (t > 0) ? step + PD->+0x10 :
  PD->+0x10 - step`. Then `PD->+0xC = 0`; `player->+0x55C = LimitRadianAngle(a2 - CH)` (m_fAimingRotation).
  TIMER (0x6878D3): if `player->+0x71C == 0` and `!(Y > 0 or NaN)` and `(i8)T->+0x27 < 0xB`: `this->+0x10 += _ftol(ms_fTimeStep *
  [0x858B38]=0.02f * [0x858C4C]=1000.0f)` (0x821B40); else `this->+0x10 = 0`.
  If `!pad->GetSprint()` 0x5407A0 and `!pad->GetDuck()` 0x540700 and !armed and `!(Y > 0)` (`test ah,0x41; je`; NaN continues) and
  `(f64)(u32)this->+0x10 < 2000.0f [0x870870]` (fild + 2^32 [0x858C54] when negative; `test ah,1`): go to DIRCMD (0x687B68).
  Otherwise: `pad->GetSprint()` (called, result ignored); `cmd = (m > 0.5f [0x858B8C]) ? 0x10 : 0xF` (NaN -> 0xF). If
  `pad->DuckJustDown()` 0x540720 and `CTaskSimpleDuck::CanPedDuck(player)` 0x692610: `player->m_pIntelligence->SetTaskDuckSecondary(0)`
  0x601230; FC(0x12). Else FC(cmd) (the `cmd > 0` test at 0x687B4E is always true).
- 6. TARGETING (0x6879D1): `t = Y - PD->+0x10`; if `fabs(t) > step`: `PD->+0x10 = (t > 0) ? step + PD->+0x10 : PD->+0x10 - step` else
  (incl. NaN) `PD->+0x10 = Y`. Same for X with `PD->+0xC`.
  If `e = player->+0x71C`: `d = epos - ppos` (matrix pos or +4 for both); `dsq = dz*dz + dy*dy + dx*dx`; if `dsq > 16.0f [0x87086C]`
  (strict; `test ah,0x41; jne`): if `(i8)T->+0x27 < 0xB` go to SPRINTCMD else go to CHK.
  CHK (0x687ACE): if `pad->GetSprint()` or armed go to SPRINTCMD; else DIRCMD.
  SPRINTCMD (0x687AE1): if `pad->GetSprint()` FC(0x11); else if `Y < -0.5f [0x858F40]` (`test ah,5; jp`) FC(0x10); else FC(0xF).
- 7. DIRCMD (0x687B6C): `ay = fabs(PD->+0x10)`, `ax = fabs(PD->+0xC)`. If `ay > 0` and `ay > ax` (strict): FC(PD->+0x10 < 0 ? 3 : 5)
  (`test ah,5; jp`: >= 0 and NaN give 5). Else if `ax > 0`: FC(PD->+0xC > 0 ? 6 : 4) (<= 0 and NaN give 4). Else return with no command.
- gta-reversed: plugin::CallMethod stub only (TaskSimplePlayerOnFoot.cpp:762). Open questions: none (the 2-player transform writes X
  from the right vector and Y from the negated forward dot product, exactly as listed at 0x687684-0x6877AB).
