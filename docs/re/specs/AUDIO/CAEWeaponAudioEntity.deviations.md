# CAEWeaponAudioEntity (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Entities/AEWeaponAudioEntity.cpp` (pin 20d8d2a).

## 0x504F80 CAEWeaponAudioEntity::WeaponFire  (gr_deviations = 1)
Original: `__thiscall void (this; i32 weaponType [esp+4], CPhysical* entity [esp+8], i32 event [esp+0xC])`, `ret 0xC`.
Row :278 (`default: NOTSA_UNREACHABLE()` after the explicit melee/thrown `return` cases):
- Original bytes: `if (entity == 0) return` (0x504F89); `type - 0x16 > 0x17` (unsigned) -> return (0x504F99); otherwise jump table
  0x505244 over types 0x16..0x2D, each case calling PlayGunSounds 0x503CE0 with per-weapon constants (e.g. type 0x16 pistol:
  `(entity, 0x34, 0x35, 6, 7, 8, event, 0.0f, 1.4142099f, 1.0f)`, 0x504FA6). Every type outside 0x16..0x2D (all melee, thrown,
  >= 0x2E and negative) returns silently.
- Verdict: CONFIRMED (gr aborts for types not listed in its switch, e.g. >= 47; the original ignores them). Class: keep-original.
