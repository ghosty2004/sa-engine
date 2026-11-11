# CTaskComplexDie: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 helper A (groups J1/J6).

## 0x6302D0 CTaskComplexDie::CreateFirstSubTask  (gr_deviations = 1)
Original bytes (`__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH; this: +0xC m_nWeaponType, +0x10 m_nAnimGroup, +0x14 m_nAnimID, +0x18 m_fBlendDelta,
+0x1C m_fAnimSpeed, +0x20 flags (bit 1 m_bFallingToDeath, bit 2 m_bFallToDeathOverRailing), +0x24 m_nFallToDeathDir):
1. `SayDeathSample(ped)` 0x630100 (0x6302F2).
2. If weapon == 0x35 (DROWNING) and ped bInVehicle (+0x46C bit 8) and NOT bit 7 of ped+0x478 (`test al,al; js`, bForceDieInCar) (0x6302FC-0x630314):
   if ped->m_pVehicle != 0 and its sub type (+0x594) is 4 or 3 -> go to step 3 (0x630329/0x63032E). Otherwise `new(0x34) CTaskComplexLeaveCar(ped->m_pVehicle, 0, 0, 0, 1)`
   0x63B8C0 (0x630330-0x630358) and return it (0 if allocation failed).
3. `ped->SetPedState(0x36 /*DIE*/)` 0x5E4500; `ped->m_pIntelligence->ClearTasks(0, 1)` 0x601420 (0x630371-0x630383).
4. If bInVehicle: inline `new(0x1C) CTaskComplexDieInCar`: CTaskComplex ctor 0x61A3B0, +0xC = this->m_nWeaponType (0x6303BD), vtbl 0x86DE04, +0x10 = 0, +0x14 = 0,
   +0x18 = 0, +0x19 = 0 (0x6303B3-0x6303D0).
5. Else if weapon == 0x35: `new(0x2C) CTaskSimpleDrown()` 0x62FEC0.
6. Else if flags bit 1: direction table 0x63056C on m_nFallToDeathDir (`cmp 3; ja` unsigned): 0 -> ped matrix forward (+0x10), 1 -> right * -1.0f (helper 0x40FEC0
   = vector * scalar, computes z, y, x products), 2 -> forward * -1.0f, 3 -> right; > 3 -> the local vector stays UNINITIALISED (0x63043A `ja 0x6304C5`). Note the matrix
   pointer ped+0x14 is used without a null check. Then `new(0x24) CTaskComplexFallToDeath(m_nFallToDeathDir, &dir, (flags >> 2) & 1, 0)` 0x679040.
7. Else `new(0x28) CTaskSimpleDie(m_nAnimGroup, m_nAnimID, m_fBlendDelta, m_fAnimSpeed)` 0x62FA00 (raw dwords).
Row TaskComplexDie.cpp:135 (`default: NOTSA_UNREACHABLE("Originally not initialized")`):
- Original bytes: 0x630434-0x63043A `cmp eax,3; ja 0x6304C5` constructs CTaskComplexFallToDeath with the uninitialised stack vector [esp+0xC..0x14].
- Verdict: CONFIRMED (gr aborts). Class: keep-original (port: pass whatever the stack holds is not reproducible; use a zero vector and record it in the divergence
  ledger only if the case is ever reached - it needs m_nFallToDeathDir > 3, which no caller in the corpus produces).
Additional differences found (not catalogue rows), both CONFIRMED / keep-original:
- cpp:110: gr returns nullptr for a drowning ped in a plane/heli; the original skips only the leave-car task and continues with steps 3-7 (0x630329/0x63032E jump to 0x630371).
- cpp:118: gr builds `CTaskComplexDieInCar(WEAPON_UNARMED)`; the original stores this->m_nWeaponType (0x6303B3/0x6303BD).
