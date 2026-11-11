# WEAPON: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x441e00 CWeapon::CWeapon (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CWeapon* ()`, EAX = this: `byte[+0x14] = 0; byte[+0x15] = 0; [+0x18] = 0` (0x441e04-0x441e0a).
- Type, state, ammo and timer fields (+0..+0x10) are NOT initialised; gr (`CWeapon() = default` with `{}` member initialisers, Weapon.h:61/104) zeroes everything. A port keeps the original (uninitialised fields are overwritten by Initialise/Load in practice).

## 0x538ba0 CFire::~CFire (empty) (proposed)  (catalogue: specified/REA, port level 0)
- Empty: single `ret` (0x538ba0). Port: no-op.
