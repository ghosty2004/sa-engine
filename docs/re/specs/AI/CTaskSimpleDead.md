# CTaskSimpleDead (AI) - RE-SPEC-AI-2 group J3

Vtable 0x86DEA4 (9 slots), task type from GetTaskType 0x6305D0. Layout (gr TaskSimpleDead.h, size 0x10): +0x8 m_nDeathTimeMS u32, +0xC flags u8:
bit0 m_bFirstTime, bit1 m_bHasDrowned, bit2 m_bBloodPuddleCreated.
Ped fields used: +0x14 m_matrix, +0x18 m_pRwClump, +0x1C CEntity flags (bit0 bUsesCollision), +0x44 m_vecMoveSpeed, +0x46C ped flags
(bit0 bIsStanding, bit1 bWasStanding, bit8 bInVehicle, bit28 bDoBloodyFootprints), +0x47C m_pIntelligence, +0x518 m_pedIK.m_fSlopePitch,
+0x520 m_pedIK.m_fSlopeRoll, +0x528 m_pedIK.m_nFlags, +0x540 m_fHealth, +0x568 m_standingOnEntity, +0x578 field_578 (ground normal CVector),
+0x58C m_pVehicle, +0x5A0 m_aWeapons (stride 0x1C), +0x718 m_nActiveWeaponSlot (i8), +0x71C m_pTargetedObject, +0x750 (bloody footprint
counter, set to 200).

## 0x630590 CTaskSimpleDead::CTaskSimpleDead  (catalogue: specified/REA, port level 1)
- `__thiscall CTaskSimpleDead* (this, u32 deathTime [esp+4], bool hasDrowned [esp+8])`, `ret 8`.
- Calls CTaskSimple::CTaskSimple 0x61A390 (0x630593); `+0x8 = deathTime` (raw dword, 0x6305AD); `+0xC = (old & 0xF9) | ((hasDrowned & 1) << 1) | 1`
  (0x630598-0x6305B9: sets m_bFirstTime, copies hasDrowned, clears bit2, keeps bits 3..7); vtbl = 0x86DEA4 (0x6305B3); returns this.
- gr deviation row (TaskSimpleDead.cpp:11 "NOTSA: deathTime originally int32"): type-only note. All later uses of +0x8 in ProcessPed are
  unsigned (`sub` then `jae`/`jbe`, 0x63095B-0x630967), so uint32 matches; REFUTED as a behavioural difference (harmless).
- Note (not this function): gr's inline Clone (TaskSimpleDead.h:19) passes m_bFirstTime as hasDrowned; check against 0x636100 before porting.

## 0x6305D0 CTaskSimpleDead::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xDA` (0x6305D0); `ret` -> returns 218 (TASK_SIMPLE_DEAD). Vtable: slot 4 (+0x10) of vtable 0x86DEA4 (CTaskSimpleDead).

## 0x6305E0 CTaskSimpleDead::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns true (1) in AL only (`mov al, 1` 0x6305E0; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x86DEA4 (CTaskSimpleDead).

## 0x6305F0 CTaskSimpleDead::~CTaskSimpleDead  (catalogue: specified/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86DEA4` (CTaskSimpleDead) (0x6305F0); tail-jump to base dtor CTaskSimple::~CTaskSimple 0x61A3A0 (0x6305F6). No other member cleanup.

## 0x630600 CTaskSimpleDead::ProcessPed  (catalogue: specified/REA, port level 20)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`, SEH frame (stack CEventDeadPed, unwound with 0x4ADF70). Always returns AL = 0 (0x630F02).
  Registers: EBP = this, ESI = ped, EDI = 0xFFFFFFFE (mask), BL = "player is targeting this ped" (0 unless set in step 1), local L13 = 0.
- Callees: CCarEnterExit::ComputeTargetDoorToExit 0x64F110, CAnimManager::BlendAnimation 0x4D4610 (clump, group, anim, delta), CPed::SetPedState
  0x5E4500, CWorld::FindPlayerPed 0x56E210 (cdecl int), CWeaponInfo::GetWeaponInfo 0x743C60, CPed::RemoveWeaponModel 0x5E3990, CPed::IsPlayer
  0x5DF8F0, CPed::RemoveWeaponAnims 0x5F0250, CPed::CreateDeadPedWeaponPickups 0x4591D0, CPed::CreateDeadPedMoney 0x4590F0, CEventDeadPed ctor
  0x4ADEA0 / dtor 0x4ADF70, GetEventGlobalGroup 0x4ABA50, CEventGroup::Add 0x4AB420, CAccidentManager::GetInstance 0x56CF20 / ReportAccident
  0x56CE80, CRT `_CIasin` 0x821E70 (ST0 -> ST0), CPed::DeadPedMakesTyresBloody 0x6B4200, CLocalisation::Blood 0x56D230,
  CWaterLevel::GetWaterLevelNoWaves 0x6E8580, CShadows::StoreStaticShadow 0x70BA00, CShadows::AddPermanentShadow 0x706F60.
- Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84, CTimer::m_snPreviousTimeInMilliseconds 0xB7CB78, gpBloodPoolTex 0xC40400; constants
  -1.0 [0x858C1C], 1.0 [0x858624], 0.0 [0x858B50], 0.75000006 [0x85B900], 4294967296.0 [0x858C54], 0.00015 [0x86DECC], -0.00015 [0x86DEC8].
- Step 1, first time (`flags & 1`, 0x630626; skipped otherwise):
  a. If ped bInVehicle (+0x46C bit 8, 0x630643): `door = ComputeTargetDoorToExit(ped->m_pVehicle, ped)`; anim = (door == 10 || door == 11) ? 0xBB : 0xBC
     (0x630658-0x630681); `BlendAnimation(ped->m_pRwClump, 0, anim, 4.0f)` (delta pushed as immediate).
  b. `ped->SetPedState(0x37 /*PEDSTATE_DEAD*/)` (0x630690); `flags &= ~1` (0x630695).
  c. L13 = 1 if ped bIsStanding or bWasStanding (+0x46C bit0 / bit1, 0x630699-0x6306AE).
  d. BL = 1 if `FindPlayerPed(0)->m_pTargetedObject == ped` or (`FindPlayerPed(1) != 0` and `FindPlayerPed(1)->m_pTargetedObject == ped`)
     (0x6306B3-0x6306E9; FindPlayerPed(1) is called twice).
  e. If `!(flags & 2)` and `ped->m_standingOnEntity == 0` and BL == 0: `ped->+0x1C &= ~1` (bUsesCollision off, 0x6306FF).
  f. `ped->m_fHealth = 0` (dword 0, 0x63070C); `RemoveWeaponModel(GetWeaponInfo(ped->m_aWeapons[(i8)activeSlot].m_eWeaponType, 1)->+0xC model)`
     (0x630702-0x63072E); `ped->m_nActiveWeaponSlot = 0` (0x630735).
  g. If `!ped->IsPlayer()`: `RemoveWeaponAnims(0, -1000.0f)`, `CreateDeadPedWeaponPickups()`, `CreateDeadPedMoney()` (0x630745-0x63075C).
  h. `CEventDeadPed ev(ped, (flags >> 1) & 1, m_nDeathTimeMS)` (0x630761-0x630777); `GetEventGlobalGroup()->Add(&ev, false)`;
     `CAccidentManager::GetInstance()->ReportAccident(ped)` (0x630797-0x63079F); `~ev`.
- Step 2, collision/anim (0x6307B5; AL = flags):
  a. If `!(flags & 1) && (flags & 2) && ped bIsStanding`: `ped->+0x1C &= ~1`; `flags &= ~2`; `a = BlendAnimation(clump, 0, 0x27, 8.0f)`;
     `a->+0x2E &= ~8` (byte, 0x6307EA; no null check) -> step 3.
  b. Else if `(ped->+0x1C & 1) && !(flags & 2) && ped bIsStanding && m_standingOnEntity == 0 && FindPlayerPed(0)->m_pTargetedObject != ped &&
     !(FindPlayerPed(1) && FindPlayerPed(1)->m_pTargetedObject == ped) && BL == 0` (0x6307F0-0x630845): `ped->+0x1C &= ~1` -> step 3.
  c. Else: if L13 == 0 skip step 3 (0x630852 -> 0x630932).
- Step 3, slope (0x630858-0x63092C), m = ped->m_matrix (no null check), n = ped->field_578:
  `f = (n.z*m.fwd.z + n.y*m.fwd.y) + n.x*m.fwd.x` stored f32 [local]; `r = (n.z*m.right.z + n.y*m.right.y) + n.x*m.right.x` (kept on st).
  Clamp(x): `if (-1.0 > x) x = -1.0; else if (1.0 < x) x = 1.0` (`test ah,0x41; je` then `test ah,5; jp`; NaN passes through unchanged).
  `ped->+0x520 (m_fSlopeRoll) = asin(clamp(r))` (0x6308DA-0x6308DF), then `ped->+0x518 (m_fSlopePitch) = asin(clamp(f))` (0x630927-0x63092C).
- Step 4: `ped->DeadPedMakesTyresBloody()` (0x630934). If `CLocalisation::Blood()` and `!(flags & 2)` (0x630939-0x63094E):
  a. `dt = now - m_nDeathTimeMS` (u32). radius `s` = dt < 2000 -> 0.0; dt > 7000 -> 0.75000006 [0x85B900]; else `(float)(u32)(dt - 2000) * 0.00015
     [0x86DECC]` (fild + 4294967296.0 when the sign bit is set, 0x630980-0x630997).
  b. For the 16 entity pointers at `ped->m_pIntelligence + 0x130 .. +0x16C` (gr: ped scanner entity array; 2 iterations x 8 unrolled, 0x6309C0-0x630D1B),
     each non-null e: `dx = e.x-p.x, dy, dz` (positions via matrix +0x30 or placement +0x4); `d2 = (dz*dz + dy*dy) + dx*dx`; if `s*s > d2` (strict,
     `fcompp; test ah,0x41; jne skip`): `e->+0x750 = 200`, `e->+0x46C |= 0x10000000` (bDoBloodyFootprints).
  c. If dt > 2000 (0x630D2C `jbe`) and `!(flags & 4)`: `pos` = ped position copy; `dt2 = dt - 2000`. If `dt2 <= now - previousTime` (unsigned,
     0x630D7B `ja skip`): if `GetWaterLevelNoWaves(pos.x, pos.y, pos.z, &w, 0, 0)` and NOT (`ped.z > w` or unordered) (0x630DCE-0x630DDA,
     `test ah,0x41; jp`): `flags |= 4`.
     Then if `!(flags & 4)` and `CLocalisation::Blood()` (called again, 0x630DF0):
     - dt2 < 5000: `t = (float)(u32)dt2`; `StoreStaticShadow(id = (u32)this + 0x11, 1, gpBloodPoolTex, &pos, t*0.00015 [0x86DECC], 0, 0,
       -0.00015 [0x86DEC8]*t, 255, 200, 0, 0, 4.0f, 1.0f, 40.0f, 0, 0.0f)` (0x630E08-0x630E63; sideY computed first as `-0.00015*t`).
     - else: `AddPermanentShadow(1, gpBloodPoolTex, &pos, 0.75000006, 0, 0, -0.75000006, 255, 200, 0, 0, 4.0f, 40000, 1.0f)` (0x630E6D-0x630EA2,
       immediates 0x3F400001 / 0xBF400001); `flags |= 4`.
- Step 5 (0x630EB2): if `flags & 2`: `ped->+0x46C &= ~3`; else `ped->+0x528 |= 8` and `ped->m_vecMoveSpeed = (0,0,0)` (0x630EC5-0x630EF8). Return 0.
- gta-reversed: plugin::Call (cpp:24), not reversed. Open questions: meaning of `ped->+0x528` bit 3 and anim assoc `+0x2E` bit 3 (0x6307EA) not named.

## 0x637930 CTaskSimpleDead::~CTaskSimpleDead  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleDead::~CTaskSimpleDead 0x6305F0 (0x637933); if `flags & 1` (0x637938) calls CTask::operator delete 0x61A5B0 (this) (0x637940, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DEA4 (CTaskSimpleDead).

## 0x636100 CTaskSimpleDead::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x10) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else bit = (this+0xC >> 1) & 1, ped = this+8 read first; p->CTaskSimple::CTaskSimple() 0x61A390; p+0xC = (p+0xC & 0xF9) | (bit << 1) | 1 (bit0 set, bit1 copied, bit2 cleared, bits 3-7 uninitialised heap); vtbl = 0x86DEA4 (CTaskSimpleDead); p+8 = ped; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
