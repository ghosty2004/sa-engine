# CAEPedWeaponAudioEntity (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x4E6AA0 CAEPedWeaponAudioEntity::Initialise  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall void(CPed* ped [esp+4])`, `ret 4`. Caller: CPed::Constructor 0x5E8030.
- Behaviour: `this->m_pPed (+0xA4) = ped`; `CAEWeaponAudioEntity::Initialise()` 0x503450 (thiscall, same this);
  `this->m_bInitialised (+0xA0, u8) = 1` (0x4E6AA7-0x4E6AB2). The ped pointer is stored before the base Initialise runs.

## 0x4E6AE0 CAEPedWeaponAudioEntity::Service  (catalogue: specified/REA, port level 5)
- Convention: `__thiscall void()`, plain `ret` (or tail jump). Caller: CPed::ProcessControl 0x5E8CD0.
- Behaviour: `ped = m_pPed (+0xA4)`. If `ped != null && !(ped->flags1 (+0x46C) & 0x100 /*bInVehicle*/) &&
  !ped->m_pIntelligence (+0x47C)->GetTaskSwim()` (0x601070) and the active weapon `w = &ped->m_aWeapons[(int8)ped->m_nActiveWeaponSlot (+0x718)]`
  (+0x5A0, stride 0x1C; the `w != null` test at 0x4E6B1E is always true) has `w->m_Type == 37` (flamethrower):
  `PlayFlameThrowerIdleGasLoop(ped)` (0x503870, thiscall, arg = ped); otherwise tail-jump `StopFlameThrowerIdleGasLoop()` 0x5034E0.
  The ped pointer is re-read from +0xA4 after GetTaskSwim (0x4E6B07).
