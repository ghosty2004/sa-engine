# CPickup (WORLD) - full specs

## 0x454920 CPickup::sub_454920 (proposed: `IsWeaponSlotWithSharedAmmo` - weapon slot is 3, 4 or 5)  (catalogue: unknown/REA, port level 0)
- Evidence: only caller CPickup::Update 0x457410 (two sites, 0x457ABD and 0x457AD8); the argument there is
  `CWeaponInfo::GetWeaponInfo(weapon, 1)->m_nSlot` (+0x14 of the 0x743C60 result, loaded into ESI at 0x457A66).
  Slots 3/4/5 = shotgun, SMG, assault-rifle slots (the slots whose ammo is merged when picking up). gr has the same
  test inlined in `CPickup::ExtractAmmoFromPickup` (where it misleadingly uses PICKUP_ONCE* enum names for 3/4/5).
  The proposed name is a guess at intent; the behaviour is exact.
- Convention: `__cdecl bool (int32 slot [esp+4])`, plain `ret`; body in .HOODLUM at 0x156F730 (stub `jmp`).
- Behaviour: `return (slot == 3 || slot == 4 || slot == 5) ? 1 : 0;` - EAX is fully written (`xor eax,eax` / `mov eax,1`).
- gr: not present as a function.
