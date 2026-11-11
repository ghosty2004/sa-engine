# CPedSaveStructure (SAVE) - RE-SPEC-REST group B

## 0x5D43D0 CPedSaveStructure::Construct  (catalogue: specified/REA, port level 0)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`.
- +0..+8 = ped position (matrix +0x30 if ped->+0x14 else ped->+4; 3 dwords); +0xC = ped->+0x540 (health); +0x10 = ped->+0x548 (armour);
  +0x180 u8 = ped->+0x484 (m_nCreatedBy); +0x181 u8 = ped->+0x718 (active weapon slot); +0x182 u8 = ped->+0x2F (area code);
  +0x188 u8 = ped->+0x72D; +0x189 u8 = ped->+0x72E; +0x184 = -1.
- If `ee = ped->+0x78C` (CEntryExit*): `t = ee->+0x38 ? ee->+0x38 : ee`; if `t->+0x32 != 0` (u8): `+0x184 = (ee - pool->objects) / 0x3C`
  (pool = CEntryExitManager::mp_poolEntryExits [0x96A7D8]; signed division via magic 0x88888889, `sar 5`; the index is of `ee`, not `t`).
- `memcpy(this+0x14, ped+0x5A0, 0x5B*4)` (rep movsd: the 13 CWeapon slots, 0x1C each).

## 0x5D44B0 CPedSaveStructure::Extract  (catalogue: specified/REA, port level 16)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`.
- 1. Position: x87 loads +8, +4, +0; if ped->+0x14 stores x, y, z to matrix +0x30/+0x34/+0x38, else to ped->+4/+8/+0xC.
- 2. ped->+0x540 = +0xC; ped->+0x548 = +0x10; ped->+0x718 = +0x181 (direct byte store); `CPed::SetCharCreatedBy(ped, (u8)+0x180)` 0x5E47E0;
  ped->+0x72D = +0x188; ped->+0x72E = +0x189.
- 3. For k = 0..12 (save weapon `w = this+0x14 + 0x1C*k`, ped weapon `pw = ped+0x5A0 + 0x1C*k`): if `w.type (+0)` != 0:
  `wi = GetWeaponInfo(w.type, 1)` 0x743C60; if `wi->+0xC != -1` { `CStreaming::RequestModel(wi->+0xC, 8)` 0x4087E0;
  `CStreaming::LoadAllRequestedModels(false)` 0x40EA10 }; same for `wi->+0x10`; `ped->GiveWeapon(w.type, w.+0xC (total ammo), false)`
  0x5E6080; `pw.+8 = w.+8` (ammo in clip).
- 4. `CPed::SetCurrentWeapon(ped, (i8)+0x181)` 0x5E61F0; ped->+0x2F = +0x182.
- 5. `idx = +0x184`: if idx == -1 or the pool flag byte `pool->+4[idx]` is negative (slot free): ped->+0x78C = 0; else ped->+0x78C =
  `pool->+0 + 0x3C*idx` (pool = [0x96A7D8]).
