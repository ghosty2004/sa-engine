# CTaskComplexKillCriminal (AI) - RE-SPEC-AI-2

## 0x68DC60 CTaskComplexKillCriminal::CreateFirstSubTask  (catalogue: specified/REA, port level 15) - RE-SPEC-AI-2 (lead)
- Convention: `__thiscall CTask* (this = ECX (ESI): CTaskComplexKillCriminal*, CPed* ped [esp+4] (EBX, the cop))`, `ret 4`, SEH frame
  (states only guard the `operator new` + ctor pairs and two stack events). Vtable slot CreateFirstSubTask (gr RH_ScopedVMTInstall).
  Part of the body is relocated: 0x68E2DF `jmp 0x407651` (`mov eax,[ebx+0x58C]`; `jmp 0x68E2E4`).
- Fields of this (gr TaskComplexKillCriminal.h): +8 m_pSubTask, +0xC m_Criminal CPed*, +0x10 m_Cop CCopPed* (registered ref),
  +0x18 m_Randomize u8, +0x1B m_OrigDrivingMode, +0x1C m_OrigMission, +0x1D m_OrigCruiseSpeed (u8 each), +0x1E m_IsSetUp u8.
  CCopPed (gr CopPed.h): +0x7A8 m_pCopPartner, +0x7C0 m_isTheDriver u8. CPed: +0x22 model index, +0x46C bit 0x100 bInVehicle,
  +0x46C bit 0x20000000 (set here), +0x470 bit 0x40 (set here), +0x478 bit 0x800 (tested), +0x47C m_pIntelligence (event group at +0x68),
  +0x480 m_pPlayerData (+0 CWanted*; CWanted +0x2C m_WantedLevel), +0x540 m_fHealth, +0x54C u32 (set to now + 300000), +0x58C m_pVehicle,
  +0x598 m_nPedType (6 = COP), +0x790 f32 (set 0.3f). CVehicle: +0x3B9/+0x3BA/+0x3D0 autopilot driving style / mission / cruise speed
  (u8), +0x42C/+0x42D/+0x42E flag bytes, +0x460 m_pDriver, +0x464 m_apPassengers[], +0x484 m_nNumPassengers u8, +0x488 m_nMaxPassengers u8,
  +0x4A6 u16, +0x590 m_nVehicleType (9 = bike).
- Globals: 0xB72994 u8 ambient-crime enable flag (written by script 09D2 ENABLE_AMBIENT_CRIME 0x47AB20, no gr name), CTimer::m_snTimeInMilliseconds
  0xB7CB84, g_InterestingEvents 0xC0B058.
- Callees: CPed::IsPlayer 0x5DF8F0, CWorld::FindPlayerPed 0x56E210, CEntity::RegisterReference 0x571B70, CGeneral::GetRandomNumberInRange(int,int)
  0x407180, CCopPed::AddCriminalToKill 0x5DDEB0, CPedIntelligence::SetPedDecisionMakerType 0x600B50, CCopPed::SetPartner 0x5DDE80,
  CEventAcquaintancePedHate ctor 0x420E70 / base dtor 0x4AF890 (vtable reset to 0x858E68 first), CEventGroup::Add 0x4AB420,
  CPedIntelligence::FindTaskByType 0x600EE0, CTask::operator new 0x61A5A0, CTaskComplexLeaveCar ctor 0x63B8C0, CTaskComplexCarDriveMission
  ctor 0x63CC30, CTaskSimpleCarDrive ctor 0x63C340, CTaskComplexKillPedOnFoot ctor 0x620E30, CPed::SetCurrentWeapon(eWeaponType) 0x5E6280,
  CPed::GiveWeapon 0x5E6080, `rand` 0x821B1E, CInterestingEvents::Add 0x602590, m_pSubTask->GetTaskType (vtbl+0x10).
- Behaviour (C = m_Criminal re-read from this each time; "reuse T" = if m_pSubTask && m_pSubTask->GetTaskType() == T then task = m_pSubTask):
  1. Return 0 if: C == 0; C->IsPlayer(); FindPlayerPed(-1)->m_pPlayerData->m_pWanted->m_WantedLevel != 0 (a null m_pPlayerData is
     dereferenced as address 0x2C - original); [0xB72994] == 0; ped->m_nPedType != 6 (0x68DC7D-0x68DCD5).
  2. If C->m_pVehicle: return 0 if its driver IsPlayer(), or any non-null m_apPassengers[i] for i < m_nMaxPassengers IsPlayer()
     (0x68DCDB-0x68DD53).
  3. m_Cop = ped; ped->RegisterReference(&m_Cop) (0x68DD55-0x68DD5E).
  4. If m_Randomize and !(C->+0x478 & 0x800): return 0 unless C->bInVehicle and C->m_pVehicle and (veh->+0x42E & 8) and
     GetRandomNumberInRange(0, 3) == 0 (0x68DD63-0x68DDB2; the failure path stores C's model index into the dead arg slot, 0x68DE60).
  5. m_Cop->AddCriminalToKill(C); if C->bInVehicle: C->m_pIntelligence->SetPedDecisionMakerType(6) (0x68DDB8-0x68DDD9).
  6. If m_Cop->bInVehicle and m_Cop->m_pVehicle (else step 8), v = that vehicle:
     a. v->m_pDriver == ped (0x68DE01): m_Cop->m_isTheDriver = 1; if m_Cop->m_pCopPartner == 0: first non-null passenger i < m_nMaxPassengers
        with m_nPedType == 6 -> m_Cop->SetPartner(it) (0x68DE10-0x68DE82). Then if m_Cop->m_pCopPartner: event hate = CEventAcquaintancePedHate(C)
        with m_taskId (+0xE) = 0x451, partner->m_pIntelligence->eventGroup.Add(&hate, false) (0x68DE87-0x68DEE9). -> step 7.
     b. driver != ped: if driver && driver->m_nPedType == 6: m_Cop->SetPartner(driver); driver->m_isTheDriver = 1 (0x68DEF3-0x68DF15).
        If m_Cop->m_pCopPartner (p):
        - p->m_isTheDriver: if p->m_pIntelligence->FindTaskByType(0x451) exists and its +0xC == C: m_Cop->m_isTheDriver = 0;
          m_Cop->+0x46C |= 0x20000000 -> step 7. Else send the same hate event (taskId 0x451) to p and RETURN 0 (0x68DF75-0x68DFC8).
        - else -> step 7.
        Else (no partner, 0x68DFCD): m_Cop->m_isTheDriver = 1; reuse 0x2C0, else new(0x34) CTaskComplexLeaveCar(ped->m_pVehicle, 0, 0, 1, 0);
        a non-null task goes to TAIL, a failed allocation falls into step 7.
  7. (0x68E036) If C->bInVehicle and C->m_pVehicle:
     - m_Cop->m_isTheDriver: reuse 0x2D4; else if ped->m_pVehicle == 0 -> task 0; else (C still in vehicle, always true here):
       new(0x2C) CTaskComplexCarDriveMission(ped->m_pVehicle, C->m_pVehicle, mission, 2, (float)C->m_pVehicle->cruiseSpeed(+0x3D0, u8) +
       10.0f [0x85862C]) with mission 0x35 if ped->m_pVehicle->m_nVehicleType == 9 else 0x12 (0x68E0B2-0x68E178). The unreachable else
       (0x68E17D) builds CarDriveMission(ped->m_pVehicle, C, 0x37, 2, 20.0f (imm 0x41A00000)).
     - else: reuse 0x2C5, else new(0x60) CTaskSimpleCarDrive(ped->m_pVehicle, 0, 0) (0x68E1C1-0x68E20F).
     Else (C on foot, 0x68E225): reuse 0x3E8 (no weapon change), else new(0x38) CTaskComplexKillPedOnFoot(C, -1, 0, 0, 0, 1) and
     ped->SetCurrentWeapon(22) (also when the allocation failed, 0x68E2C9).
  8. Cop on foot (0x68E273): if m_Cop->m_pCopPartner == 0 or partner->m_fHealth <= 0.0f [0x858B50] (NaN: not set): m_Cop->m_isTheDriver = 1.
     Then as the C-on-foot branch of step 7 (reuse 0x3E8 / new KillPedOnFoot + SetCurrentWeapon(22)).
  9. TAIL (0x68E2DA): if ped->m_pVehicle and m_Cop->m_isTheDriver: m_OrigDrivingMode/m_OrigMission/m_OrigCruiseSpeed = vehicle +0x3B9/+0x3BA/
     +0x3D0; m_IsSetUp = 1 (0x68E2E4-0x68E31C).
     Weapons: if C->m_pVehicle: if (veh->+0x42E & 8): if veh->m_nNumPassengers > 0: C->GiveWeapon(22, 1000, 1), C->SetCurrentWeapon(22), and
     for each non-null passenger i < m_nMaxPassengers: GiveWeapon(22, 1000, 1), SetCurrentWeapon(22), m_Cop->AddCriminalToKill(it);
     else if (rand() & 1): C->GiveWeapon(22, 1000, 1), SetCurrentWeapon(22) (0x68E320-0x68E401).
     If C has no vehicle: if (rand() & 1): C->GiveWeapon(22, 1000, 1), SetCurrentWeapon(22), and C receives CEventAcquaintancePedHate(ped)
     with m_taskId 0x3E8 (0x68E403-0x68E46C).
     ped->+0x54C = now + 300000; ped->+0x470 |= 0x40; ped->+0x790 = 0.3f (imm 0x3E99999A); if ped->m_pVehicle: veh->+0x4A6 = 0xFF,
     veh->+0x42C |= 0x40; if ped->m_pVehicle and ped->bInVehicle: veh->+0x42D |= 0x80; g_InterestingEvents.Add(0x19, ped); return task
     (0x68E471-0x68E4EA).
- Edge cases: returning 0 after step 3 leaves m_Cop registered. The hate event of 6b returns null while step 6a continues. The step-9
  weapon/flag block runs for every non-null-return path, including reused subtasks.
- gta-reversed: plugin::CallMethodAndReturn (TaskComplexKillCriminal.cpp:338, "Good luck!"), not reversed.
- Open questions: names of ped +0x54C / +0x790 / +0x470 bit 0x40 / +0x478 bit 0x800 and vehicle +0x42C..+0x42E bits, +0x4A6 (offsets exact,
  names not checked against gr flag unions).
