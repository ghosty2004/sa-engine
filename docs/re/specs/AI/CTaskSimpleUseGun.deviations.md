# CTaskSimpleUseGun: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 helper A (groups J1/J6).

## 0x61E3F0 CTaskSimpleUseGun::SetMoveAnim  (gr_deviations = 2)
Original bytes: `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. Only the rows' region is re-checked here.
Row TaskSimpleUseGun.cpp:660 (`if (!notsa::IsFixBugs()) { m_MoveCmd = {0,0}; }`) and row cpp:661 (bug note "at high FPS m_MoveCmd can never accumulate past 0.1"):
- Original bytes: end of the `moveCmdAbsSum < 0.1f` branch: after the optional SetCurrentTime(0) resets of the four move anims (only when the new anim's blend amount
  `> 0.95f [0x858EF0]`, `test ah,0x41; jne`, 0x61E5A0-0x61E5AE; 0x4CEA80 calls 0x61E5B5-0x61E5D9), the original ALWAYS stores `this+0x14 = 0`, `this+0x18 = 0`
  (m_MoveCmd.x/.y) and `this+0xA = 0` (m_HasMoveControl) (0x61E5E4-0x61E5EB), then returns (0x61E5F3).
- Verdict row 660: CONFIRMED (gr skips the reset when FixBugs is on). Class: keep-original.
- Verdict row 661: CONFIRMED - the frame-rate-dependent bug is the original's behaviour. Class: keep-original (original bug to keep).

## 0x61E8E0 CTaskSimpleUseGun::RemoveStanceAnims  (gr_deviations = 1)
Row TaskSimpleUseGun.cpp:165 (`std::abs(pad->GetPedWalkUpDown()) > 50 || std::abs(pad->GetPedWalkLeftRight()) > 50 // NOTSA: They didn't use abs`):
- Original bytes (0x61E9EC-0x61EA64): four separate pad queries, each value `movsx` int16 -> `fild` and compared with f32 constants:
  `UD > 50.0f [0x858B40]` (`test ah,0x41; je walk`), else `UD < -50.0f [0x859B60]` (`test ah,5; jnp walk`), else `LR > 50.0f [0x858B40]`, else `LR < -50.0f [0x859B60]`
  (`jp skip`). GetPedWalkUpDown 0x53FD30 / GetPedWalkLeftRight 0x53FC90 are each called twice (re-read, not cached). Walk branch: `CAnimManager::BlendAnimation(ped+0x18 clump,
  ped+0x4D4 anim group, 0 /*ANIM_ID_WALK*/, 8.0f)` 0x4D4610, `assoc->m_nFlags (+0x2E) |= 1` (0x61EA7D), then `SetMoveState` 0x5DEC00.
- Verdict: REFUTED - for int16 inputs `v > 50 || v < -50` equals `abs(v) > 50` (the float compare is exact, no NaN possible) and the pad getters have no side
  effects, so the double reads give the same value within a frame. Class: harmless.

## 0x61EB10 CTaskSimpleUseGun::FireGun  (gr_deviations = 1)
Row TaskSimpleUseGun.h:42 (declaration: "Originally returned a bool ... we're going to make it void"):
- Original bytes: `__thiscall (this, CPed* ped [esp+4], bool [esp+8])`, `ret 8`, single exit 0x61ECFF; EAX on return is the AL result of `CWeapon::Fire` 0x742300 (0x61ECCD),
  or, when the active weapon's state (`ped+0x5A4 + 0x1C*slot`) == 1, the return of `CPed::DoGunFlash(ped, [0x8D2E88] u16, arg2)` 0x5DF340 (0x61ECF5). The only caller,
  SetPedPosition 0x624ED0, ignores EAX after both call sites (0x624F12 / 0x624F22 do not read AL).
- Verdict: REFUTED (no observable difference: the return value is never consumed). Class: harmless.
