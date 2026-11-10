# CAEPedWeaponAudioEntity (AUDIO) - full specs (RE-SPEC-3, group U1)

Derives from CAEWeaponAudioEntity (methods called with the same ECX). +0xA0 u8 m_bInitialised, +0xA4 CPed* m_Ped. Ped
+0x5A0 CWeapon m_aWeapons[] (stride 0x1C, +0 type), +0x718 i8 m_nActiveWeaponSlot.

## 0x4E69F0 CAEPedWeaponAudioEntity::AddAudioEvent  (catalogue: specified/REA, port level 8)
- Convention: `__thiscall void (this = ECX; i32 event [esp+4])`, `ret 4`. Callers (11): CPed::ProcessControl 0x5E8CD0,
  CPlayerPed::ProcessControl 0x60EA90 (2), CTaskSimpleFight::ProcessPed 0x629920, 0x61CBA0, 0x61D0B0, 0x61D400, ...
- Behaviour: if `+0xA0 == 0` or `+0xA4 == 0` return. `w = &ped->m_aWeapons[(i8)ped->+0x718]` (pointer test only, never null;
  NO weapon-type test). `event - 0x91` > 0xB (unsigned) -> return. Table 0x4E6A88/0x4E6A74:
  - 0x91 FIRE, 0x96 MINIGUN_AMMO, 0x97 MINIGUN_NO_AMMO: `WeaponFire(w->type, ped, event)` 0x504F80.
  - 0x92 RELOAD_A, 0x93 RELOAD_B: `WeaponReload(w->type, ped, event)` 0x503690.
  - 0x99, 0x9A, 0x9B (chainsaw idle/active/cutting): `ReportChainsawEvent(ped, event)` 0x503910.
  - 0x9C STEALTH_KILL: `ReportStealthKill(w->type, ped, event)` 0x503B20.
  - 0x94, 0x95, 0x98: nothing.
  All callees are `__thiscall` on this, arguments pushed right-to-left as listed.
- gta-reversed: unhooked C++ adds `if (weapon.m_Type == WEAPON_UNARMED) return;` - NOT in the original (CONFIRMED,
  keep-original: unarmed events are forwarded with type 0). Event mapping matches. Open questions: none.

## 0x4E6AC0 CAEPedWeaponAudioEntity::Terminate  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (this = ECX)`; tail `jmp CAEWeaponAudioEntity::Terminate` 0x503480. Caller: CPed::Destructor 0x5E8620.
- Behaviour: `+0xA0 = 0` (u8), `+0xA4 = 0`, then tail-call the base Terminate.
- gta-reversed: same. Open questions: none.
