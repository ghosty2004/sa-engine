# CAEScriptAudioEntity (AUDIO) - RE-SPEC-REST group A

## 0x4ECCF0 CAEScriptAudioEntity::ProcessMissionAudioEvent  (catalogue: specified/REA, port level 10)
- Convention: `__thiscall void (this = ECX: CAEScriptAudioEntity*, int32 event [esp+4], const CVector* pos [esp+8], CPhysical* phys
  [esp+0xC], float volume [esp+0x10], float speed [esp+0x14])`, `ret 0x14`. `sub esp,0x18` + 4 pushes; ESI = this, EDI = event
  (full dword, passed on to the callees as `ev`), EBX = (u16)event for the dispatch (`movzx ebx, di`, 0x4ECCFB). Executed under SA:MP.
- Callers: ReportMissionAudioEvent 0x4EE940 (physical variant) and 0x4EE960 (position variant).
- Callees: PlayMissionBankSound 0x4EC6D0 (`PMB(sfx, linkId, checkPlaying, volume, maxDistance, speed)` below = `this->
  PlayMissionBankSound(ev, pos, phys, sfx, linkId, checkPlaying, volume, maxDistance, speed)`, thiscall, 9 dwords),
  PlayResidentSoundEvent 0x4EC550 (`PRS(slot, bank, sfx, ev, pos, phys, vol, speed, playPos, maxDist)`, thiscall, 10 dwords),
  CAudioEngine (AudioEngine 0xB6BC90) ::ReportFrontendAudioEvent 0x506EA0 (event, volume, speed) / ::ReportWeaponEvent 0x506F40 /
  ::ReportWaterSplash 0x506F00 (physical) and 0x506F10 (CVector by value), CAEAmbienceTrackManager (AEAmbienceTrackManager 0x8AC15C)
  ::PlaySpecialMissionAmbienceTrack 0x4D6D50 (id) / ::StopSpecialMissionAmbienceTrack 0x4D6D60, CAESoundManager (0xB62CB0)
  ::AreSoundsOfThisEventPlayingForThisEntity 0x4EF570, ...AndPhysical 0x4EF5D0, ::CancelSoundsOfThisEventPlayingForThisEntity
  0x4EFB90, ...AndPhysical 0x4EFBF0, ::CancelSoundsInBankSlot 0x4EFC60 (slot, bool), ::AreSoundsPlayingInBankSlot 0x4EF520,
  CAEAudioHardware (0xB5F8B8) ::IsSoundBankLoaded 0x4D88C0 / ::LoadSoundBank 0x4D88A0, GetRandomNumberInRange<int32> 0x4D9C10
  (`Rand(a, b)`, inclusive), CAEDoorAudioEntity::AddAudioEvent 0x4DC860 (this+0x114), CAEExplosionAudioEntity::AddAudioEvent
  0x4DCBE0 (this+0x19C), CAEPedAudioEntity::AddAudioEvent 0x4E2BB0, CLocalisation::Blood 0x56D230,
  CAEVehicleAudioEntity::DisableHelicoptors 0x4F4EE0 / EnableHelicoptors 0x4F4EF0.
- Globals (R): m_pAudioEventVolumes 0xBD00F8, CTimer::m_snTimeInMilliseconds 0xB7CB84.
- Fields of this: +0x7C u8 (cleared by ROULETTE_SPIN), +0x7D u8 crane bank loaded flag, +0x7E int8 mesh-gate alternation toggle,
  +0x80 u32 last event time ms, +0x84 f32 volume, +0x88 f32 speed, +0x8C f32 gym machine state (1.0 running / 2.0 stopping),
  +0x110 CPhysical* crane entity, +0x114 CAEDoorAudioEntity, +0x19C CAEExplosionAudioEntity.
- Dispatch: compare tree on EBX (0x4ECCFE-...) with five dense jump tables (0x4EE668 for 0x3EF..0x3F2, 0x4EE678 for 0x3F4..0x401,
  0x4EE6B0 for 0x403..0x41A, 0x4EE710 for 0x41C..0x44A, 0x4EE7CC for 0x44D..0x4A6). Any event not listed below -> plain return
  (0x4EE65B). Every case returns right after the listed actions.
- Obfuscated constant: the relocated block 0x405BA1-0x405BB4 (entered from 0x4EE64D) computes [0x15607F8] - [0x156491C] (.HOODLUM
  data, = 0x8F) with `sbb` after `clc` and swaps it onto the stack: it is just `push 0x8F` (bank argument of the following PRS
  call at 0x4EE656 with slot 5). Shown as 0x8F in the table.
- Floats in the table are the f32 immediates pushed (0.79 = 0x3F4A3D71, 0.67 = 0x3F2B851F, 0.38 = 0x3EC28F5C, 0.94 = 0x3F70A3D7,
  1.12 = 0x3F8F5C29, 1.26 = 0x3FA147AE, 2.5 = 0x40200000, -18 = 0xC1900000, -3 = 0xC0400000, -6 = 0xC0C00000, 5 = 0x40A00000).
  `0` in a volume slot is integer 0 (= 0.0f).

| event(s) | case | behaviour |
|---|---|---|
| 0x3E8 AE_SCRIPT_FIRST | 0x4ECD7D | CAEVehicleAudioEntity::DisableHelicoptors() 0x4F4EE0 |
| 0x68 AE_CRANE_WINCH_MOVE | 0x4ECD8C | if this+0x7D == 0 return. If !AreSoundsOfThisEventPlayingForThisEntityAndPhysical(ev, this, phys): PRS(0x28, 0x2C, 1, ev, pos, phys, volume, speed, 0, 2.5). Then always (0x4ECDD2): this+0x88 = speed; this+0x84 = (float)(int32)(int8)m_pAudioEventVolumes[ev] (0xBD00F8) + volume (fild, fadd f32, fstp); this+0x80 = m_snTimeInMilliseconds |
| 0x3E9 AE_SCRIPT_ENABLE_HELI_AUDIO | 0x4ECE0C | CAEVehicleAudioEntity::EnableHelicoptors() 0x4F4EF0 |
| 0x3EE AE_SCRIPT_CLAXON_STOP | 0x4ECE73 | CancelSoundsOfThisEventPlayingForThisEntity(0x3ED, this) |
| 0x3F2 AE_SCRIPT_BASKETBALL_BOUNCE | 0x4ECEA3 | PMB(Rand(0, 2), 3, 0, 0, 2.0, 1.0) |
| 0x3F0 AE_SCRIPT_BLAST_DOOR_SLIDE_STOP | 0x4ECEDC | CancelSoundsOfThisEventPlayingForThisEntity(0x3EF, this) |
| 0x3F6 AE_SCRIPT_POOL_HIT_WHITE | 0x4ECF0C | PMB(0xa, 3, 0, 0, 2.0, 1.0) |
| 0x3F9 AE_SCRIPT_POOL_BALL_POT | 0x4ECF3A | PMB(Rand(3, 5), 3, 0, 0, 2.0, 1.0) |
| 0x3FA AE_SCRIPT_POOL_CHALK_CUE, 0x49C AE_SCRIPT_TEMPEST_SELECT | 0x4ECF73 | PMB(7, 3, 0, 0, 2.0, 1.0) |
| 0x3F4 AE_SCRIPT_BASKETBALL_SCORE | 0x4ECFA1 | if AreSoundsOfThisEventPlayingForThisEntity(ev, this) return; PMB(4, 3, 0, 0, 2.0, 1.0) |
| 0x3FB AE_SCRIPT_CRANE_ENTER | 0x4ECFCD | if !IsSoundBankLoaded(0x2C, 0x28) { if AreSoundsPlayingInBankSlot(0x28) CancelSoundsInBankSlot(0x28, 0); LoadSoundBank(0x2C, 0x28) }; this+0x110 = phys; this+0x7D = 1 |
| 0x3FE AE_SCRIPT_CRANE_EXIT | 0x4ED024 | if this+0x7D == 0 return; CancelSoundsInBankSlot(0x28, 1); PRS(0x28, 0x2C, 3, ev, pos, phys, 0.0, 1.0, 0, 2.5); this+0x110 = 0; this+0x7D = 0 |
| 0x3FC AE_SCRIPT_CRANE_MOVE_START | 0x4ED07B | if this+0x7D == 0 return; if AreSoundsOfThisEventPlayingForThisEntityAndPhysical(ev, this, phys) return; PRS(0x28, 0x2C, 1, ev, pos, phys, 0.0, 1.0, 0, 2.5) |
| 0x3FD AE_SCRIPT_CRANE_MOVE_STOP | 0x4ED0CC | if this+0x7D == 0 return; if phys: CancelSoundsOfThisEventPlayingForThisEntityAndPhysical(0x3FC, this, phys) else CancelSoundsOfThisEventPlayingForThisEntity(0x3FC, this); PRS(0x28, 0x2C, 2, ev, pos, phys, 0.0, 1.0, 0, 2.5) |
| 0x401 AE_SCRIPT_VIDEO_POKER_PAYOUT | 0x4ED129 | PMB(1, 3, 0, 0, 2.0, 1.0) |
| 0x402 AE_SCRIPT_VIDEO_POKER_BUTTON | 0x4ED157 | PMB(0, 3, 0, 0, 2.0, 1.0) |
| 0x412 AE_SCRIPT_DUAL_THRUST | 0x4ED19B | PMB(0, 3, 1, 0, 2.0, 1.0); this+0x80 = m_snTimeInMilliseconds |
| 0x413 AE_SCRIPT_DUAL_EXPLOSION_SHORT, 0x462 AE_SCRIPT_OTB_LOSE | 0x4ED1D5 | PMB(2, 3, 1, 0, 2.0, 1.0) |
| 0x414 AE_SCRIPT_DUAL_EXPLOSION_LONG | 0x4ED203 | PMB(1, 3, 1, 0, 2.0, 1.0) |
| 0x415 AE_SCRIPT_DUAL_MENU_SELECT, 0x431 AE_SCRIPT_BEE_SELECT | 0x4ED231 | PMB(5, 3, 0, 0, 2.0, 1.0) |
| 0x416 AE_SCRIPT_DUAL_MENU_DESELECT, 0x42F AE_SCRIPT_BEE_PICKUP, 0x49B AE_SCRIPT_TEMPEST_HIGHLIGHT | 0x4ED248 | PMB(4, 3, 0, 0, 2.0, 1.0) |
| 0x417 AE_SCRIPT_DUAL_GAME_OVER | 0x4ED276 | PMB(3, 3, 1, 0, 2.0, 1.0) |
| 0x418 AE_SCRIPT_DUAL_PICKUP_LIGHT | 0x4ED2A4 | PMB(7, 3, 1, 0, 2.0, 1.0) |
| 0x419 AE_SCRIPT_DUAL_PICKUP_DARK | 0x4ED2BB | PMB(6, 3, 1, 0, 2.0, 1.0) |
| 0x41A AE_SCRIPT_DUAL_TOUCH_DARK | 0x4ED2E9 | PMB(9, 3, 1, 0, 2.0, 1.0) |
| 0x3F8 AE_SCRIPT_POOL_HIT_CUSHION, 0x411 AE_SCRIPT_DUAL_SHOOT | 0x4ED317 | PMB(8, 3, 0, 0, 2.0, 1.0) |
| 0x40F AE_SCRIPT_PARACHUTE_OPEN | 0x4ED32B | PRS(5, 0x8f, 0x41, ev, pos, phys, 0, 1.0, 0, 1.0) |
| 0x403 AE_SCRIPT_WHEEL_OF_FORTUNE_CLACKER | 0x4ED34B | PRS(2, 0x27, 0x13, ev, pos, phys, 0, 1.0, 0x46, 1.0) |
| 0x404 AE_SCRIPT_KEYPAD_BEEP | 0x4ED37B | if IsSoundBankLoaded(0xF2, 0x1C) PMB(0, 2, 0, 0, 2.0, 1.0); else if IsSoundBankLoaded(0x159, 0x1C) PMB(3, 2, 0, 0, 2.0, 1.0) (0x4ED3BE); else nothing |
| 0x3EF AE_SCRIPT_BLAST_DOOR_SLIDE_START | 0x4ED390 | PMB(0, 2, 0, 0, 2.0, 1.0) |
| 0x406 AE_SCRIPT_KEYPAD_FAIL, 0x457 AE_SCRIPT_DA_NANG_HEAVY_DOOR_OPEN, 0x484 AE_SCRIPT_CAT2_WOODEN_DOOR_BREACH | 0x4ED405 | PMB(1, 2, 0, 0, 2.0, 1.0) |
| 0x408 AE_SCRIPT_SHOOTING_RANGE_TARGET_DROP | 0x4ED433 | PMB(1, 3, 0, 0, 2.0, 1.0) |
| 0x409 AE_SCRIPT_SHOOTING_RANGE_TARGET_MOVE_START | 0x4ED447 | PMB(0, 3, 0, 0, 2.0, 1.0) |
| 0x40A AE_SCRIPT_SHOOTING_RANGE_TARGET_MOVE_STOP | 0x4ED475 | if phys: CancelSoundsOfThisEventPlayingForThisEntityAndPhysical(0x409, this, phys) else CancelSoundsOfThisEventPlayingForThisEntity(0x409, this) |
| 0x407 AE_SCRIPT_SHOOTING_RANGE_TARGET_SHATTER | 0x4ED4AD | PMB(Rand(2, 6), 3, 0, 0, 2.0, 1.0) |
| 0x41B AE_SCRIPT_DUAL_TOUCH_LIGHT | 0x4ED4E6 | PMB(0xa, 3, 1, 0, 2.0, 1.0) |
| 0x42C AE_SCRIPT_DUAL_TRACK_START | 0x4ED52A | AEAmbienceTrackManager.PlaySpecialMissionAmbienceTrack(0x96) |
| 0x423 AE_SCRIPT_GOGO_PLAYER_FIRE | 0x4ED543 | PMB(5, 3, 0, 0, 2.0, 1.0) |
| 0x424 AE_SCRIPT_GOGO_ENEMY_FIRE | 0x4ED571 | PMB(2, 3, 0, 0, 2.0, 1.0) |
| 0x3F3 AE_SCRIPT_BASKETBALL_HIT_HOOP, 0x425 AE_SCRIPT_GOGO_EXPLOSION, 0x43F AE_SCRIPT_BANDIT_INSERT_COIN, 0x486 AE_SCRIPT_OTB_NO_CASH | 0x4ED588 | PMB(3, 3, 0, 0, 2.0, 1.0) |
| 0x428 AE_SCRIPT_GOGO_SELECT, 0x492 AE_SCRIPT_TEMPEST_PLAYER_SHOOT | 0x4ED5B6 | PMB(6, 3, 0, 0, 2.0, 1.0) |
| 0x429 AE_SCRIPT_GOGO_ACCEPT | 0x4ED5E4 | PMB(0, 3, 0, 0, 2.0, 1.0) |
| 0x42A AE_SCRIPT_GOGO_DECLINE, 0x432 AE_SCRIPT_BEE_ACCEPT | 0x4ED5F9 | PMB(1, 3, 0, 0, 2.0, 1.0) |
| 0x42B AE_SCRIPT_GOGO_GAME_OVER | 0x4ED627 | PMB(4, 3, 1, 0, 2.0, 1.0) |
| 0x426 AE_SCRIPT_GOGO_TRACK_START | 0x4ED655 | AEAmbienceTrackManager.PlaySpecialMissionAmbienceTrack(0x99) |
| 0x42E AE_SCRIPT_BEE_ZAP | 0x4ED66E | PMB(6, 3, 0, 0, 2.0, 1.0) |
| 0x430 AE_SCRIPT_BEE_DROP | 0x4ED685 | PMB(2, 3, 0, 0, 2.0, 1.0) |
| 0x433 AE_SCRIPT_BEE_DECLINE | 0x4ED6B3 | PMB(1, 3, 0, 0, 2.0, 0.79) |
| 0x436 AE_SCRIPT_BEE_GAME_OVER, 0x49A AE_SCRIPT_TEMPEST_GAME_OVER | 0x4ED6E1 | PMB(3, 3, 1, 0, 2.0, 1.0) |
| 0x434 AE_SCRIPT_BEE_TRACK_START | 0x4ED6F8 | AEAmbienceTrackManager.PlaySpecialMissionAmbienceTrack(0x8d) |
| 0x441 AE_SCRIPT_BANDIT_WHEEL_START | 0x4ED711 | PMB(0, 3, 0, 0, 2.0, 1.0) |
| 0x440 AE_SCRIPT_BANDIT_WHEEL_STOP | 0x4ED728 | if phys: CancelSoundsOf...AndPhysical(0x441, this, phys) else CancelSoundsOfThisEventPlayingForThisEntity(0x441, this); PMB(2, 3, 0, 0, 2.0, 1.0) |
| 0x442 AE_SCRIPT_BANDIT_PAYOUT | 0x4ED778 | PMB(1, 3, 0, 0, 2.0, 1.0) |
| 0x43B AE_SCRIPT_ROULETTE_ADD_CASH | 0x4ED78F | AudioEngine.ReportFrontendAudioEvent(1, 0, 1.0) |
| 0x43C AE_SCRIPT_ROULETTE_REMOVE_CASH | 0x4ED7AC | AudioEngine.ReportFrontendAudioEvent(2, 0, 1.0) |
| 0x43D AE_SCRIPT_ROULETTE_NO_CASH | 0x4ED7C9 | AudioEngine.ReportFrontendAudioEvent(4, 0, 1.0) |
| 0x43E AE_SCRIPT_ROULETTE_SPIN | 0x4ED7E6 | PMB(0, 3, 0, 0, 2.0, 1.0); this+0x80 = m_snTimeInMilliseconds; this+0x7C = 0 |
| 0x447 AE_SCRIPT_BIKE_PACKER_CLUNK | 0x4ED823 | PRS(2, 0x27, 0x41, ev, pos, phys, 0, 1.0, 0x10, 1.0) |
| 0x438 AE_SCRIPT_FREEZER_CLOSE | 0x4ED853 | PMB(1, 2, 1, 0, 3.0, 1.0) |
| 0x437 AE_SCRIPT_FREEZER_OPEN | 0x4ED86A | PMB(2, 2, 1, 0, 3.0, 1.0) |
| 0x439 AE_SCRIPT_MEAT_TRACK_START | 0x4ED898 | PMB(3, 2, 1, 0, 3.0, 1.0); PMB(0, 2, 0, 0, 3.0, 1.0) |
| 0x43A AE_SCRIPT_MEAT_TRACK_STOP | 0x4ED8E2 | CancelSoundsOfThisEventPlayingForThisEntity(0x439, this); PMB(4, 2, 1, 0, 3.0, 1.0) |
| 0x449 AE_SCRIPT_AWARD_TRACK_START | 0x4ED920 | AEAmbienceTrackManager.PlaySpecialMissionAmbienceTrack(0x91) |
| 0x41C AE_SCRIPT_AMMUNATION_BUY_WEAPON, 0x41E AE_SCRIPT_SHOP_BUY | 0x4ED939 | AudioEngine.ReportFrontendAudioEvent(0x28, 0, 1.0) |
| 0x41D AE_SCRIPT_AMMUNATION_BUY_WEAPON_DENIED, 0x41F AE_SCRIPT_SHOP_BUY_DENIED | 0x4ED956 | AudioEngine.ReportFrontendAudioEvent(0xe, 0, 1.0) |
| 0x420 AE_SCRIPT_RACE_321 | 0x4ED973 | AudioEngine.ReportFrontendAudioEvent(0x2c, 0, 1.0) |
| 0x421 AE_SCRIPT_RACE_GO | 0x4ED990 | AudioEngine.ReportFrontendAudioEvent(0x2d, 0, 1.0) |
| 0x44C AE_SCRIPT_MESH_GATE_OPEN_START | 0x4ED9AD | if (int8)this+0x7E != 0: PRS(2, 0x27, 0x45, ev, pos, phys, 0.0, 1.0, 0x3F, 1.0) else PRS(2, 0x27, 0x46, ev, pos, phys, 0.0, 1.0, 0x3E, 1.0); then this+0x7E = (int8)(((int32)(int8)this+0x7E + 1) % 2) (C remainder, 0x4ED9F6-0x4EDA03) |
| 0x460 AE_SCRIPT_OTB_BET_ZERO | 0x4EDA27 | PMB(0, 3, 0, 0, 2.0, 1.0) |
| 0x461 AE_SCRIPT_OTB_INCREASE_BET, 0x493 AE_SCRIPT_TEMPEST_ENEMY_SHOOT | 0x4EDA55 | PMB(1, 3, 0, 0, 2.0, 1.0) |
| 0x463 AE_SCRIPT_OTB_PLACE_BET | 0x4EDA6C | PMB(4, 3, 0, 0, 2.0, 1.0) |
| 0x464 AE_SCRIPT_OTB_WIN | 0x4EDA9A | PMB(5, 3, 1, 0, 2.0, 1.0) |
| 0x476 AE_SCRIPT_OTB_TRACK_START | 0x4EDAB1 | AEAmbienceTrackManager.PlaySpecialMissionAmbienceTrack(0xa0) |
| 0x48F AE_SCRIPT_BEE_BUZZ | 0x4EDACA | PMB(0, 3, 1, 0, 2.0, 1.0); this+0x80 = m_snTimeInMilliseconds |
| 0x45B AE_SCRIPT_GYM_BOXING_BELL, 0x494 AE_SCRIPT_TEMPEST_EXPLOSION | 0x4EDB04 | PMB(2, 3, 0, 0, 2.0, 1.0) |
| 0x495 AE_SCRIPT_TEMPEST_PICKUP1 | 0x4EDB32 | PMB(5, 3, 0, 0, 2.0, 0.67) |
| 0x496 AE_SCRIPT_TEMPEST_PICKUP2 | 0x4EDB3C | PMB(5, 3, 0, 0, 2.0, 0.79) |
| 0x497 AE_SCRIPT_TEMPEST_PICKUP3 | 0x4EDB53 | PMB(5, 3, 0, 0, 2.0, 1.0) |
| 0x498 AE_SCRIPT_TEMPEST_WARP | 0x4EDB81 | PMB(8, 3, 1, 0, 2.0, 0.38) |
| 0x499 AE_SCRIPT_TEMPEST_SHIELD_GLOW | 0x4EDB98 | PMB(0, 3, 1, 0, 2.0, 1.0); this+0x80 = m_snTimeInMilliseconds |
| 0x49D AE_SCRIPT_TEMPEST_TRACK_START | 0x4EDBD1 | AEAmbienceTrackManager.PlaySpecialMissionAmbienceTrack(0xac) |
| 0x40B AE_SCRIPT_SHUTTER_DOOR_START, 0x481 AE_SCRIPT_GARAGE_DOOR_START | 0x4EDBEA | p = phys ? (phys->m_matrix (+0x14) ? &matrix->pos (+0x30) : &phys->placement.pos (+0x4)) : pos; m_DoorAudio (this+0x114).AddAudioEvent(0x9F, p, 0.0, 1.0) (0x4DC860) |
| 0x40C AE_SCRIPT_SHUTTER_DOOR_STOP, 0x482 AE_SCRIPT_GARAGE_DOOR_STOP | 0x4EDC39 | p as for 0x40B; m_DoorAudio.AddAudioEvent(0xA0, p, 0.0, 1.0) |
| 0x485 AE_SCRIPT_MINITANK_FIRE | 0x4EDC55 | if phys == 0 return; AudioEngine.ReportWeaponEvent(0x91, 0x1F, phys) (0x506F40) |
| 0x46C AE_SCRIPT_CAMERA_SHOT | 0x4EDC7D | if phys: AudioEngine.ReportWeaponEvent(0x91, 0x2B, phys) else PRS(5, 0x8F, 0x2D, ev, pos, 0, 0.0, 1.0, 0, 1.0) (0x4EDCA1) |
| 0x487 AE_SCRIPT_EXPLOSION | 0x4EDCBE | p as for 0x40B; m_ExplosionAudio (this+0x19C).AddAudioEvent(0x81, p, 0.0) (0x4DCBE0) |
| 0x46B AE_SCRIPT_AMMUNATION_GUN_COLLISION | 0x4EDCFB | PRS(2, 0x27, 0x21, ev, pos, phys, 0, 1.0, 0, 1.0); PRS(2, 0x27, 0x32, ev, pos, phys, 0, 0.79, 0x16, 1.0) |
| 0x479 AE_SCRIPT_RESTAURANT_TRAY_COLLISION | 0x4EDD49 | PRS(2, 0x27, 0x13, ev, pos, phys, 0, 1.0, 0x41, 1.0) |
| 0x44D AE_SCRIPT_MESH_GATE_OPEN_STOP | 0x4EDD79 | if (int8)this+0x7E != 0: PRS(2, 0x27, 0x45, ev, pos, phys, 0.0, 0.94, 0x21, 1.0) else PRS(2, 0x27, 0x46, ev, pos, phys, 0.0, 1.0, 0x19, 1.0); then this+0x7E = (this+0x7E + 1) % 2 as above |
| 0x46A AE_SCRIPT_PUNCH_PED | 0x4EDDDD | PRS(5, 0x8f, 0x3a, ev, pos, phys, 0, 1.0, 0, 1.0); PRS(5, 0x8f, 0x28, ev, pos, phys, 0, 1.0, 0, 1.0) |
| 0x46F AE_SCRIPT_BASEBALL_BAT_HIT_PED | 0x4EDDFA | PRS(5, 0x8f, 0x22, ev, pos, phys, 5.0, 1.0, 0, 1.0); PRS(5, 0x8f, 0x28, ev, pos, phys, 0, 1.0, 0, 1.0) |
| 0x470 AE_SCRIPT_STAMP_PED | 0x4EDE3E | PRS(5, 0x8f, 0x52, ev, pos, phys, -3.0, 1.0, 0, 1.0) |
| 0x4A5 AE_SCRIPT_PED_DEATH_CRUNCH | 0x4EDE61 | if !CLocalisation::Blood() return; if phys == 0 return; if (phys->m_nType (+0x36) & 7) != 3 (ped) return; ped->m_pedAudio (phys+0x138).AddAudioEvent(0x77, 0.0, 1.0, phys, 0, 0, 0) (0x4E2BB0) |
| 0x468 AE_SCRIPT_VERTICAL_BIRD_LIFT_START | 0x4EDEAE | PMB(2, 2, 0, 0, 2.0, 1.0); PMB(0, 2, 0, 0, 2.0, 1.0) |
| 0x469 AE_SCRIPT_VERTICAL_BIRD_LIFT_STOP | 0x4EDEF8 | if phys: CancelSoundsOf...AndPhysical(0x468, this, phys) else CancelSoundsOfThisEventPlayingForThisEntity(0x468, this); PMB(3, 2, 0, 0, 2.0, 1.0) |
| 0x489 AE_SCRIPT_VERTICAL_BIRD_ALARM_START | 0x4EDF36 | PMB(1, 2, 0, 0, 2.0, 1.0) |
| 0x48A AE_SCRIPT_VERTICAL_BIRD_ALARM_STOP | 0x4EDF64 | if phys: CancelSoundsOf...AndPhysical(0x489, this, phys) else CancelSoundsOfThisEventPlayingForThisEntity(0x489, this) |
| 0x454 AE_SCRIPT_CARGO_PLANE_DOOR_START | 0x4EDF9C | PMB(1, 2, 0, 0, 2.0, 1.0); PMB(0, 2, 0, 0, 2.0, 1.0) |
| 0x455 AE_SCRIPT_CARGO_PLANE_DOOR_STOP | 0x4EDFB3 | CancelSoundsOfThisEventPlayingForThisEntity(0x454, this); PMB(2, 2, 0, 0, 2.0, 1.0) |
| 0x451 AE_SCRIPT_HEAVY_DOOR_START | 0x4EDFDA | PMB(2, 2, 0, 0, 2.0, 1.0) |
| 0x453 AE_SCRIPT_SHOOT_CONTROLS | 0x4EDFF1 | if IsSoundBankLoaded(0xA7, 0x1C) PMB(3, 2, 0, 0, 2.0, 1.0) (0x4ED3D7); else if IsSoundBankLoaded(0x159, 0x1C) PMB(4, 2, 0, 0, 2.0, 1.0); else nothing |
| 0x48C AE_SCRIPT_AIR_HORN | 0x4EE03A | local L = (-1, 0, 0): PlayMissionBankSound(ev, &L, NULL, 0, 3, 0, 0.0, 2.0, 1.0); L = (1, 0, 0): PlayMissionBankSound(ev, &L, NULL, 1, 3, 0, 0.0, 2.0, 1.0) |
| 0x48D AE_SCRIPT_SHUTTER_DOOR_SLOW_START | 0x4EE0E6 | p as for 0x40B; m_DoorAudio.AddAudioEvent(0x9F, p, 0.0, 0.79) |
| 0x48E AE_SCRIPT_SHUTTER_DOOR_SLOW_STOP | 0x4EE145 | p as for 0x40B; m_DoorAudio.AddAudioEvent(0xA0, p, 0.0, 1.0) |
| 0x48B AE_SCRIPT_PED_COLLAPSE | 0x4EE18C | PRS(0x29, 0, 0, ev, pos, phys, 0, 1.0, 0, 1.0) |
| 0x459 AE_SCRIPT_GYM_BIKE_START | 0x4EE1BC | PMB(0, 3, 1, -18.0, 2.0, 1.0); this+0x8C = 1.0f |
| 0x45C AE_SCRIPT_GYM_INCREASE_DIFFICULTY | 0x4EE1DD | PMB(3, 3, 0, 0, 2.0, 1.0) |
| 0x45D AE_SCRIPT_GYM_REST_WEIGHTS | 0x4EE1F4 | PMB(7, 3, 0, 0, 2.0, 1.0) |
| 0x45E AE_SCRIPT_GYM_RUNNING_MACHINE_START | 0x4EE20B | PMB(1, 3, 1, -18.0, 2.0, 1.0); this+0x8C = 1.0f |
| 0x45A AE_SCRIPT_GYM_BIKE_STOP, 0x45F AE_SCRIPT_GYM_RUNNING_MACHINE_STOP | 0x4EE246 | this+0x8C = 2.0f |
| 0x3F1 AE_SCRIPT_BONNET_DENT, 0x474 AE_SCRIPT_CAR_SMASH_CAR, 0x47C AE_SCRIPT_MAGNET_VEHICLE_COLLISION | 0x4EE25A | PRS(2, 0x27, Rand(0x14, 0x1C), ev, pos, phys, 0.0, 1.0, 0, 1.0) |
| 0x475 AE_SCRIPT_CAR_SMASH_GATE | 0x4EE295 | PRS(2, 0x27, Rand(0x14, 0x1C), ev, pos, phys, 0.0, 1.0, 0, 1.0); PRS(2, 0x27, 0x41, ev, pos, phys, 0.0, 1.0, 0, 1.0) |
| 0x44E AE_SCRIPT_OGLOC_DOORBELL | 0x4EE2EE | PMB(0, 2, 0, 0, 2.0, 1.0) |
| 0x44F AE_SCRIPT_OGLOC_WINDOW_RATTLE_BANG | 0x4EE31C | PMB(1, 2, 0, 0, 2.0, 1.0) |
| 0x465 AE_SCRIPT_STINGER_FIRE | 0x4EE333 | PMB(0, 2, 0, 0, 2.0, 1.0) |
| 0x450 AE_SCRIPT_STINGER_RELOAD, 0x47F AE_SCRIPT_MECHANIC_SLIDE_OUT | 0x4EE361 | PMB(1, 2, 0, 0, 2.0, 1.0) |
| 0x456 AE_SCRIPT_DA_NANG_CONTAINER_OPEN, 0x480 AE_SCRIPT_MECHANIC_ATTACH_CAR_BOMB, 0x483 AE_SCRIPT_CAT2_SECURITY_ALARM | 0x4EE38F | PMB(0, 2, 0, 0, 2.0, 1.0) |
| 0x466 AE_SCRIPT_HEAVY_GATE_START | 0x4EE3A6 | PMB(1, 2, 0, 0, 2.0, 1.0); PMB(0, 2, 0, 0, 2.0, 1.0) |
| 0x467 AE_SCRIPT_HEAVY_GATE_STOP | 0x4EE3F0 | CancelSoundsOfThisEventPlayingForThisEntity(0x466, this); PMB(2, 2, 0, 0, 2.0, 1.0) |
| 0x405 AE_SCRIPT_KEYPAD_PASS | 0x4EE400 | PMB(2, 2, 0, 0, 2.0, 1.0) |
| 0x47B AE_SCRIPT_SWEETS_HORN | 0x4EE42E | if AreSoundsOfThisEventPlayingForThisEntity(ev, this) return; PRS(0x11, 0x4A, 7, ev, pos, phys, 0.0, 1.0, 0, 1.0); this+0x80 = m_snTimeInMilliseconds |
| 0x478 AE_SCRIPT_PED_HIT_WATER_SPLASH | 0x4EE47F | if phys: AudioEngine.ReportWaterSplash(phys, -6.0, 0) (0x506F00) else AudioEngine.ReportWaterSplash(CVector *pos by value, -6.0) (0x506F10) |
| 0x490 AE_SCRIPT_RESTAURANT_CJ_EAT | 0x4EE4D5 | PMB(0, 3, 0, 0, 2.0, 1.0) |
| 0x491 AE_SCRIPT_RESTAURANT_CJ_PUKE | 0x4EE4EC | PMB(1, 3, 0, 0, 2.0, 1.0) |
| 0x49F AE_SCRIPT_DRIVING_AWARD_TRACK_START | 0x4EE51A | AEAmbienceTrackManager.PlaySpecialMissionAmbienceTrack(0x95) |
| 0x4A1 AE_SCRIPT_BIKE_AWARD_TRACK_START | 0x4EE533 | AEAmbienceTrackManager.PlaySpecialMissionAmbienceTrack(0x8e) |
| 0x4A3 AE_SCRIPT_PILOT_AWARD_TRACK_START | 0x4EE54C | AEAmbienceTrackManager.PlaySpecialMissionAmbienceTrack(0xa1) |
| 0x427 AE_SCRIPT_GOGO_TRACK_STOP, 0x42D AE_SCRIPT_DUAL_TRACK_STOP, 0x435 AE_SCRIPT_BEE_TRACK_STOP, 0x44A AE_SCRIPT_AWARD_TRACK_STOP, 0x477 AE_SCRIPT_OTB_TRACK_STOP, 0x49E AE_SCRIPT_TEMPEST_TRACK_STOP, 0x4A0 AE_SCRIPT_DRIVING_AWARD_TRACK_STOP, 0x4A2 AE_SCRIPT_BIKE_AWARD_TRACK_STOP, 0x4A4 AE_SCRIPT_PILOT_AWARD_TRACK_STOP | 0x4EE565 | AEAmbienceTrackManager.StopSpecialMissionAmbienceTrack() (0x4D6D60) |
| 0x471 AE_SCRIPT_CHECKPOINT_AMBER | 0x4EE579 | AudioEngine.ReportFrontendAudioEvent(0x2b, 0, 1.12) |
| 0x473 AE_SCRIPT_CHECKPOINT_RED | 0x4EE596 | AudioEngine.ReportFrontendAudioEvent(0x2b, 0, 1.26) |
| 0x46D AE_SCRIPT_BUY_CAR_MOD | 0x4EE5B3 | AudioEngine.ReportFrontendAudioEvent(0x2e, 0, 1.0) |
| 0x46E AE_SCRIPT_BUY_CAR_RESPRAY | 0x4EE5D0 | AudioEngine.ReportFrontendAudioEvent(0x10, 0, 1.0) |
| 0x422 AE_SCRIPT_PART_MISSION_COMPLETE, 0x472 AE_SCRIPT_CHECKPOINT_GREEN, 0x47D AE_SCRIPT_PROPERTY_PURCHASED | 0x4EE5ED | AudioEngine.ReportFrontendAudioEvent(0x2b, 0, 1.0) |
| 0x47E AE_SCRIPT_PICKUP_STANDARD | 0x4EE60A | AudioEngine.ReportFrontendAudioEvent(7, 0, 1.0) |
| 0x4A6 ? | 0x4EE627 | PRS(5, 0x8F, Rand(0x4E, 0x50), ev, pos, phys, 0.0, 1.0, 0, 1.0) |

- gta-reversed: plugin::CallMethod stub (AEScriptAudioEntity.cpp:262) followed by a commented-out draft marked "Untested / Need to
  fix case's" (AE_CAS4_* placeholder names; catalogue gr-deviations row: bugfix-comment, keep-original). Not usable; port from the
  table above.
- Open questions: none affecting behaviour; parameter meanings of the external callees follow gr names.

## 0x4EC970 CAEScriptAudioEntity::UpdateParameters  (catalogue: specified/REA, port level 5)
- Convention: virtual (CAEAudioEntity slot 1), `__thiscall void (this = ECX, CAESound* sound [esp+4], int16 curPlayPos [esp+8])`,
  `ret 8`. `sub esp,0x14`: local CVector L = (-1000, -1000, -1000) (imm 0xC47A0000 x3, 0x4EC981-0x4EC991).
- Callees: CAESound::SetPosition 0x4EF880 (CVector by value), CAESound::StopSoundAndForget 0x4EF850, PlayResidentSoundEvent 0x4EC550,
  PlayMissionBankSound 0x4EC6D0, GetRandomNumberInRange<int32> 0x4D9C10 (inclusive).
- Globals (R): CTimer::m_snTimeInMilliseconds 0xB7CB84 (u32, re-read once per loop iteration at 0x4ECA03), m_pAudioEventVolumes 0xBD00F8.
- Fields: +0x7C u8 m_MiniGameOneShotTriggered, +0x80 u32 m_LastMiniGameLoopTriggerTimeMs (T below), +0x84 f32 m_CraneVolume, +0x88 f32
  m_CraneFrequencyScaling, +0x8C f32 m_MissionSoundEnvelopePhase; m_WavLinks[4] at +0x90 (stride 0x20): link+0xC m_pEntity, link+0x10
  m_Sound. sound+8 physical entity, +0xC m_Event, +0x14 m_Volume, +0x1C m_Speed, +0x24 m_CurrPos.
- Behaviour: if sound == NULL return (0x4EC999). For i = 0..3 (link = this+0x90+0x20*i):
  1. If link.m_Sound == sound: if curPlayPos == -1 { link.m_Sound = NULL (store at this+0xA0+0x20*i); RETURN (whole function) }.
     Else if link.m_pEntity != NULL: sound->SetPosition(entity matrix ? matrix->pos (+0x14 -> +0x30) : placement pos (+4)) (copied into a
     12-byte stack CVector). Then next i.
  2. Otherwise (link does NOT hold this sound) the per-event update below runs on `sound`, keyed by sound->m_Event, and then next i.
     So for a sound not held by any link it runs 4 times per call; for a held sound it runs once per other link (original behaviour).
     now = m_snTimeInMilliseconds; all time compares are unsigned (`jbe` = skip when now <= limit).
     - 0x68 (AE_CRANE_WINCH_MOVE): if now > T + 300 (0x12C): StopSoundAndForget(); T = 0; PlayResidentSoundEvent(0x28, 0x2C, 2, 0x3FD,
       &sound->m_CurrPos, sound+8, -12.0f (imm 0xC1400000), 1.0f, 0, 2.5f) (0x4ECA47-0x4ECA79). Else: sound.m_Volume = this+0x84;
       sound.m_Speed = this+0x88 (fld/fstp copies).
     - 0x43E (AE_SCRIPT_ROULETTE_SPIN): if now > T + 4500 (0x1194): v = m_Volume; if v > -40.0f [0x85A708] (`fcom; test ah,0x41; jne`):
       m_Volume = v - 0.1f [0x858B1C]; else (v <= -40 or NaN) { StopSoundAndForget(); T = 0 }. Then s = m_Speed; if s > 0.0f [0x858B50]:
       s = s - 0.001f [0x858CDC]; m_Speed = (0.0 > s) ? 0.0f : s (`fld 0; fcomp st1; test ah,0x41; jne` keeps s when 0 <= s or NaN).
       Then (also when now <= T + 4500; T re-read) if now > T + 4800 (0x12C0) and +0x7C == 0: PlayMissionBankSound(0x488, &L, NULL,
       Rand(1, 3), 3, 0, 0.0f, 2.0f, 1.0f); +0x7C = 1.
     - 0x459 / 0x45E (GYM_BIKE_START / GYM_RUNNING_MACHINE_START): base = (float)(int32)(int8)m_pAudioEventVolumes[event] (fild, stored
       f32). If dword +0x8C == 0x3F800000 (1.0f, integer compare): v = m_Volume; if v < base (`test ah,5; jp` exits otherwise / NaN):
       v += 0.1f; m_Volume = (v < base) ? v : base. Else if +0x8C == 0x40000000 (2.0f): lim = base - 18.0f [0x859008] (rounded to f32);
       v = m_Volume; if v > lim: v -= 0.1f; m_Volume = (v > lim) ? v : lim; else (v <= lim or NaN) StopSoundAndForget(). Other +0x8C:
       nothing.
     - 0x47B (AE_SCRIPT_SWEETS_HORN): if now > T + 500 (0x1F4): StopSoundAndForget(); T = 0.
     - 0x412 (DUAL_THRUST), 0x48F (BEE_BUZZ), 0x499 (TEMPEST_SHIELD_GLOW): if now > T + 300: StopSoundAndForget(); T = 0.
     - other events: nothing.
- gta-reversed: `#if 0` body (AEScriptAudioEntity.cpp, "partially reversed"), RH_ScopedVMTInstall with .Reversed = false. It differs from
  the bytes: it switches on the link's m_nAudioEvent instead of sound->m_Event, its event arithmetic covers only 0x47B/0x48F/0x499,
  and the crane, roulette and gym envelopes are missing. Port from the bytes.
- Open question: whether running the per-event block once per non-matching link (up to 4x per call) is intended; the bytes do it
  (loop back-edge 0x4ECCAD-0x4ECCC4 is reached from every event path).
