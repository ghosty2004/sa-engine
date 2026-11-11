# CTaskSimpleGangDriveBy (AI) - RE-SPEC-AI-2 group J4

## 0x6218B0 CTaskSimpleGangDriveBy::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3FE` (0x6218B0); `ret` -> returns 1022 (TASK_SIMPLE_GANG_DRIVEBY). Vtable: slot 4 (+0x10) of vtable 0x86D944.

## 0x6275E0 CTaskSimpleGangDriveBy::~CTaskSimpleGangDriveBy  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006218c0 0x6218C0 (0x6275E3); if `flags & 1` (0x6275E8) calls CTask::operator delete 0x61A5B0 (this) (0x6275F0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D944.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x6218C0 sub_6218c0 (proposed: CTaskSimpleGangDriveBy::~CTaskSimpleGangDriveBy (non-deleting))  (catalogue: unknown/REA, port level 1)
- Evidence: stores vtable 0x86D944; callers 0x6275E0 (scalar deleting dtor), CCam::Process_Cam_TwoPlayer_InCarAndShooting 0x519810 and CTaskAllocatorKillThreatsDriveby::AllocateTasks 0x69CC30 (stack instances).
- `__thiscall void (this)`, plain `ret`; SEH frame.
- Behaviour: `vtbl = 0x86D944`; `if m_bAnimsReferenced (+9)` -> `CAnimManager::RemoveAnimBlockRef((ms_aAnimAssocGroups[+0x28].m_pAnimBlock - 0xB5D4A0) >> 5)` 0x4D3FD0 (group stride 0x14; flag not cleared); `if m_pAnimAssoc (+0x2C)` -> `SetDeleteCallback(0x4D1390 (empty), NULL)` 0x4CEBC0; `if m_pTargetEntity (+0x34)` -> CleanUpOldReference(target, &+0x34) 0x571A00; CTaskSimple::~CTaskSimple 0x61A3A0.

## 0x621960 CTaskSimpleGangDriveBy::sub_621960 (proposed: CTaskSimpleGangDriveBy::PlayerTarget)  (catalogue: unknown/REA, port level 4)
- Name proposal: player-aim branch of CTaskSimpleGangDriveBy (caller CTaskSimpleGangDriveBy::ProcessPed 0x62D3B0): chooses the drive-by anim from the camera aim direction relative to the vehicle. Fields: +0xA m_bSeatRHS, +0xB m_bInRangeToShoot, +0xC m_bInWeaponRange, +0x24 m_nRequiredAnimID.
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`.
- Behaviour:
  1. `i = TheCamera.m_nActiveCam` (u8 [0xB6F081]); `mode = TheCamera.m_aCams[i].m_nMode` (u16 at 0xB6F1A8 + i*0x238). If `mode != 0x37 && mode != 0x31`: `+0xB = 0; +0xC = 1`; go to 5.
  2. `+0xB = 1; +0xC = 1`; `L = TheCamera.m_aCams[i].m_vecFront` (0xB6F32C + i*0x238, 3 dwords); if `mode == 0x31` -> `CCam::Get_TwoPlayer_AimVector(&TheCamera.m_aCams[i] (0xB6F19C + i*0x238), &L)` 0x513E40.
  3. x87 (0x6219E1-0x621A4C): `c = fpatan(st1 = -L.x, st0 = L.y)` = atan2(-L.x, L.y). `veh = ped->m_pVehicle (+0x58C)` (no NULL check); `h = veh->m_matrix ? atan2(-m.forward.x (+0x10), m.forward.y (+0x14)) : veh->m_placement.m_fHeading (+0x10)`; `a = c - h` (`fsubp st(1)`). `fcom [0x858CB8]=pi; test ah, 0x41; jne`: if `a > pi` -> `a -= [0x858CBC]=2pi` and skip the next test; else `fcom [0x858CC0]=-pi; test ah, 5; jp`: if `a < -pi` -> `a += 2pi`. Then `a += [0x859AB0]=pi/4`; `fcom [0x858B50]=0.0; test ah, 5; jp`: if `a < 0` -> `a += 2pi`. `a *= [0x858FB8]=2/pi (0.63661975)`; `k = (int32)(int8)_ftol2(a)` 0x821B40 (truncation, low byte sign-extended).
  4. If `(u32)k <= 3` (jump table 0x621B00): k = 0 -> `+0x24 = seatRHS ? 0xED : 0xE9`; k = 1 -> `seatRHS ? 0xEC : 0xE7`; k = 2 -> `seatRHS ? 0xEE : 0xEA`; k = 3 -> `seatRHS ? 0xEB : 0xE8` (seatRHS = +0xA != 0). Otherwise +0x24 unchanged.
  5. `if !CGameLogic::IsCoopGameGoingOn()` 0x441390 -> `CCamera::SetNewPlayerWeaponMode(&TheCamera 0xB6F028, 0x37, 0, 0)` 0x50BFB0. `if ped->m_pPlayerData (+0x480)` -> `m_pPlayerData->+0x34 |= 8`. `if CPed::IsPlayer(ped)` -> `CPlayerPed::GetPlayerInfoForThisPlayerPed(ped)->+0x153 = 0` (u8).

## 0x621B10 CTaskSimpleGangDriveBy::sub_621B10 (proposed: CTaskSimpleGangDriveBy::CheckLineOfSight)  (catalogue: unknown/REA, port level 5)
- `__thiscall bool (this, CPed* ped [esp+4], const CVector* aimPos [esp+8])`, `ret 8`; returns AL = 1 when the line of sight is NOT blocked. Caller: 0x627600. Fields: +0x18 m_nLOSCheckTime, +0x1C m_nLOSBlocked, +0x34 m_pTargetEntity.
- Behaviour:
  1. `if (u32)now <= (u32)m_nLOSCheckTime` (`jbe` 0x621B1E; now = [0xB7CB84]) -> skip to 4 (cached result).
  2. `m_nLOSBlocked = 0`. If `m_pTargetEntity != NULL`: `p = ped->m_matrix ? &m_matrix->pos : &ped->m_placement.pos`; local `s = (p.x, p.y, p.z + [0x858B8C]=0.5f)` (x copied as raw dword; y via fld/fstp; z = fld/fadd/fstp, f32 store); `CWorld::pIgnoreEntity ([0xB7CD68]) = m_pTargetEntity`; `clear = CWorld::GetIsLineOfSightClear(&s, aimPos, 1, 0, 0, 0, 0, 1, 0)` 0x56A490 (buildings only, see-through check on); `if !clear` -> `m_nLOSBlocked = 1`; `CWorld::pIgnoreEntity = 0`.
  3. Re-arm (also when the target is NULL): `r = _rand() & 0xFFFF` 0x821B1E; `t = (float)r` (fild) `* [0x858B14]=3.0517578e-05f * [0x85AB78]=-500.0f`; `k = _ftol2(t)` 0x821B40 (truncation toward zero); `m_nLOSCheckTime = now - k + 0x6D6` (now re-read; i.e. now + 1750 + trunc(500*r/32768)).
  4. Return `AL = (m_nLOSBlocked == 0)` (`sete`).

## 0x621BE0 CTaskSimpleGangDriveBy::sub_621BE0 (proposed: CTaskSimpleGangDriveBy::FinishAnimCB / delete-callback)  (catalogue: unknown/REA, port level 0)
- `static void __cdecl (CAnimBlendAssociation* assoc [esp+4], void* data [esp+8] = CTaskSimpleGangDriveBy*)`, plain `ret`. Referenced as a callback by 0x627B20.
- Behaviour: `if data->+0x2C (anim) == assoc` -> `data->+0x2C = 0` (0x621BE7-0x621BED). Nothing else.

## 0x6236D0 CTaskSimpleGangDriveBy::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame.
- Behaviour: `p = CTask::operator new(0x44)`; if p != NULL: `EAX = CTaskSimpleGangDriveBy::CTaskSimpleGangDriveBy(p, m_pTargetEntity (+0x34), &m_vecCoords (+0x38), m_fAbortRange (+0x20, raw), (u32)(u8)m_nFrequencyPercentage (+0x13), (u32)(u8)m_nDrivebyStyle (+0x12), (u32)(u8)m_bSeatRHS (+0xA))` 0x6217D0; else EAX = 0. Then, OUTSIDE the NULL check (0x62372C): `EAX->+0xE (m_bFromScriptCommand) = this->+0xE`. Original bug (keep): on allocation failure this writes to address 0xE. Return EAX.

## 0x627600 CTaskSimpleGangDriveBy::sub_627600 (proposed: CTaskSimpleGangDriveBy::TargetInRangeDriveBy)  (catalogue: unknown/REA, port level 6)
- Name proposal: AI targeting step of CTaskSimpleGangDriveBy (caller ProcessPed 0x62D3B0): computes distance/angle to the target, range flags and the drive-by anim. Fields: +0xA m_bSeatRHS, +0xB m_bInRangeToShoot, +0xC m_bInWeaponRange, +0xD m_bReachedAbortRange, +0xF m_nNextCommand, +0x12 m_nDrivebyStyle (int8), +0x20 m_fAbortRange, +0x24 m_nRequiredAnimID, +0x30 m_pWeaponInfo, +0x34 m_pTargetEntity, +0x38 m_vecCoords.
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`.
- Behaviour:
  1. `range = m_pWeaponInfo->+8 (m_fTargetRange)`. Target point: if `m_pTargetEntity`: if it has no matrix -> `CPlaceable::AllocateMatrix(t)` 0x54F560 and `CSimpleTransform::UpdateMatrix(&t->m_placement, t->m_matrix)` 0x54F1B0; `W = (*t->m_matrix) * m_vecCoords` (CMatrix::operator* 0x59C890, result by hidden pointer); else `W = m_vecCoords`. `d = W - P` (P = ped position), each component stored f32.
  2. `dist = fsqrt((d.z*d.z + d.x*d.x) + d.y*d.y)` (f32 store). `c = atan2(-d.x, d.y)` (fchs/fpatan); `h = ped->m_pVehicle (+0x58C)` heading (`m_matrix ? atan2(-fwd.x, fwd.y) : m_placement.m_fHeading`, no NULL check on the vehicle); `a = c - h` (f32). If `a > [0x858CB8]=pi` -> `a -= [0x858CBC]=2pi`; else if `a < [0x858CC0]=-pi` -> `a += 2pi` (each result stored f32).
  3. `m_bInRangeToShoot = 0`; `m_bInWeaponRange = (dist < range)` (only C0). Abort range: if `!m_bReachedAbortRange`: if `dist < m_fAbortRange` -> `m_bReachedAbortRange = 1`. Else (already reached) if `dist > m_fAbortRange` (neither C0 nor C3) -> `m_nNextCommand = 7`; return.
  4. Dead target: if the target exists: entity type (`+0x36 & 7`) 3 (ped) with `m_fHealth (+0x540) <= 0` (`test ah, 0x41; jnp`), or type 2 (vehicle) with `m_fHealth (+0x4C0) <= 0` -> `m_nNextCommand = 7`; return.
  5. `aim = P + d` (z, y, x each fld/fadd/fstp); `if !CheckLineOfSight(this, ped, &aim)` 0x621B10 -> `m_bInRangeToShoot = 0`; return.
  6. Side values (only when style is not 0, 1, 5 or 6; otherwise `q = -1`, `fb = lr = 0`): `q = (int8)_ftol2((a + [0x859AB0]=pi/4 (+2pi if the sum < 0)) * [0x858FB8]=2/pi)`; `fb = -1` if `pi/12 ([0x858F1C]) < a < 11pi/12 ([0x86DB10])`, else `fb = 1` if `-11pi/12 ([0x86DB0C]) < a < -pi/12 ([0x858F18])`, else 0; `lr = -1` if `-5pi/12 ([0x86DB04]) < a < 5pi/12 ([0x86DB08])`; the alternative `lr = 1` test requires `a > 7pi/12 ([0x86DB00])` AND `a < -7pi/12 ([0x86DAFC])` - never true (original logic bug, keep), so lr is -1 or 0.
  7. Quadrant k by style (jump table 0x627AE8; anim 0xBF = "no anim yet"): 0 -> 1; 1 -> 3; 2 -> `animId == 0xBF ? 1 : Q13`; 3 -> `animId == 0xBF ? 3 : Q13`; 4 -> `animId == 0xBF ? (fb == -1 ? 1 : fb == 1 ? 3 : -1) : Q13`; 5 -> 0; 6 -> 2; 7 -> `animId == 0xBF ? (lr == -1 ? 0 : lr == 1 ? 2 : -1) : (q == 0 ? 0 : q == 2 ? 2 : -1)`; 8 -> q; other -> -1. `Q13 = (q == 1 ? 1 : q == 3 ? 3 : -1)`.
  8. By k (jump table 0x627B0C; `in = dist < range`): k 0 -> `m_nRequiredAnimID = seatRHS ? 0xED : 0xE9`; `m_bInRangeToShoot = 1` if `in && a >= -pi/4 ([0x86DAF8]) && a <= pi/4`. k 1 -> `seatRHS ? 0xEC : 0xE7`; set if `in && a >= pi/4 && a <= 3pi/4 ([0x86DAF4])`. k 2 -> `seatRHS ? 0xEE : 0xEA`; set if `in && (a >= -3pi/4 ([0x86DAF0]) || a <= 3pi/4)` - always true for non-NaN a (original logic: the back quadrant test accepts every angle). k 3 -> `seatRHS ? 0xEB : 0xE8`; set if `in && a >= -3pi/4 && a <= -pi/4`. Other k: nothing. (Each "a <= x" uses `test ah, 0x41; jp` -> false for NaN; each "a >= x" uses `test ah, 1`.)

## 0x627B20 CTaskSimpleGangDriveBy::sub_627B20 (proposed: CTaskSimpleGangDriveBy::PlayerAnim / StartAnim)  (catalogue: unknown/REA, port level 9)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`; returns AL = 1 when the drive-by anim is fully blended in. Callers: CTaskSimpleGangDriveBy::MakeAbortable 0x62D290, ProcessPed 0x62D3B0. Fields (gr names): +9 m_bAnimsReferenced, +0xC m_bInWeaponRange, +0xF m_nNextCommand, +0x10 m_nLastCommand, +0x11 m_nBurstShots, +0x16 m_nAttackTimer (int16), +0x24 m_nRequiredAnimID, +0x28 m_nRequiredAnimGroup, +0x2C m_pAnimAssoc, +0x30 m_pWeaponInfo.
- Behaviour:
  1. `blk = ms_aAnimAssocGroups ([0xB4EA34])[m_nRequiredAnimGroup].m_pAnimBlock` (dword at +0 of a 0x14-byte group); if NULL -> `blk = CAnimManager::GetAnimationBlock(CAnimManager::GetAnimBlockName(group))` 0x4D3A30/0x4D3940. `idx = (blk - 0xB5D4A0) >> 5`.
  2. `if blk->m_bLoaded (+0x10, u8) != 1` -> `CStreaming::RequestModel(idx + 0x63E7, 8)` 0x4087E0 and return 0.
  3. `if !m_bAnimsReferenced` -> `CAnimManager::AddAnimBlockRef(idx)` 0x4D3FB0; `m_bAnimsReferenced = 1`.
  4. `id = m_nRequiredAnimID`; if `id == 0xBF && m_pAnimAssoc == NULL` -> return 0.
  5. Decide whether to (re)start the anim: with `a = m_pAnimAssoc`: if `a == NULL`: start iff `(int16)m_nAttackTimer <= 0 && m_bInWeaponRange != 0`; else skip to 7 (returns 0). If `a != NULL`: if `(int32)(int16)a->m_AnimId (+0x2C) == id` -> skip to 7; else start iff `m_nNextCommand in {1, 2, 3}` and `(int8)m_nLastCommand < 4`; otherwise skip to 7.
  6. Start: `had = (a != NULL)`; if had: `a->SetDeleteCallback(0x4D1390 (empty), NULL)` 0x4CEBC0 and `a->m_Flags &= ~1`. `a = CAnimManager::BlendAnimation(ped->m_pRwClump, m_nRequiredAnimGroup, id, 4.0f)` 0x4D4610; `m_pAnimAssoc = a`; `a->SetDeleteCallback(0x621BE0, this)`. If `m_nNextCommand == 3 && (int8)m_nBurstShots <= 1` -> `m_nBurstShots = (u8)CGeneral::GetRandomNumberInRange(2, (int32)(int16)m_pWeaponInfo->+0x20)` 0x407180 (original int version). If had: `a->SetCurrentTime([0x8D2E7C])` 0x4CEA80 (an f32 in .data, initial value 0.2667f - a tunable global, read at run time), `a->m_Flags &= ~1`, `m_nLastCommand = 1`, `m_nNextCommand = 0`, and `if !CPed::IsPlayer(ped)` -> `m_nAttackTimer = 100`. Else: `m_nLastCommand = m_nNextCommand; m_nNextCommand = 0`.
  7. `a = m_pAnimAssoc`; if NULL -> 0. Return 1 iff `a->m_BlendAmount (+0x18) > [0x858C20]=0.9f` (`test ah, 0x41; jne` -> less/equal/NaN give 0) and `a->m_BlendDelta (+0x1C) >= [0x858B50]=0.0f` (`test ah, 1; jne` -> less or NaN give 0); else 0.

## 0x627CC0 CTaskSimpleGangDriveBy::sub_627CC0 (proposed: CTaskSimpleGangDriveBy::FireGun)  (catalogue: unknown/REA, port level 21)
- Name proposal: fires one drive-by shot (callers CTaskSimpleGangDriveBy::ProcessPed 0x62D3B0 and CCam::Process_Cam_TwoPlayer_InCarAndShooting 0x519810). Fields: +0x14 m_nFakeShootDirn (int8), +0x2C m_pAnimAssoc, +0x30 m_pWeaponInfo, +0x34 m_pTargetEntity, +0x38 m_vecCoords.
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`; 0x3C bytes of locals (the ped argument slot is reused for a float range).
- Behaviour:
  1. Muzzle: `M = m_pWeaponInfo->+0x24 (m_vecFireOffset)`; `h = GetAnimHierarchyFromSkinClump(ped->m_pRwClump)`; `i = RpHAnimIDGetIndex(h, ((frame data*)ped->+0x4A0)->+0x14)`; `RwV3dTransformPoints(&M, &M, 1, RpHAnimHierarchyGetMatrixArray(h) + i*0x40)` 0x7EDD90 (in place, world space). `veh = ped->m_pVehicle (+0x58C)`; `bb = CModelInfo::ms_modelInfoPtrs[veh->m_nModelIndex]->m_pColModel` (+0 min, +0xC max); `V = M - vehPos` (f32 per component).
  2. Aim direction `D`:
     - Player (`CPed::IsPlayer`): `i = TheCamera.m_nActiveCam`; `mode = m_aCams[i].m_nMode`; `range = 20.0f` (0x41A00000); if `mode == 0x31`: `range = fsqrt((dz*dz + dy*dy) + dx*dx) + [0x858BA4]=20.0f` with `d = pedPos - m_aCams[i].m_vecSource (0xB6F338 + i*0x238)`; else if `mode != 0x37` -> return (no shot). `CCamera::Find3rdPersonCamTargetVector(&TheCamera, range, M (by value), &camPos, &B)` 0x514970; `D = B - M`; `ped->m_pPlayerData->+0x2C = 0`.
     - AI: `W = m_pTargetEntity ? (*target->m_matrix) * m_vecCoords : m_vecCoords` (matrix allocated/updated first if missing, as in 0x627600); `D = W - M`.
     `CVector::Normalise(&D)` 0x59C910.
  3. Origin `O` (default `O = M`), by the side the anim/fake direction points to; `R` = vehicle matrix right (+0..+8), `F` = vehicle matrix forward (+0x10..+0x18); dot products are x87 `((a.z*b.z + a.y*b.y) + a.x*b.x)` for `s` and `((V.y*b.y + V.z*b.z) + V.x*b.x)` for `q`:
     - Left (anim id 0xE7 or 0xEC, else m_nFakeShootDirn == 1): `s = -dot(D, R)`; if `s > [0x858B1C]=0.1f` (`fcom; test ah, 0x41; jne` keeps M for <=, NaN): `t = ((bb.min.x - q) - [0x858CC4]=0.2f) / s` (fsubr/fsub/fdivrp); `O = M + D*t` (per component fmul/fadd, f32 stores).
     - Right (0xEB or 0xE8, else fake 3): `s = dot(D, R)`; if `s > 0.1`: `t = ((bb.max.x - q) + 0.2f) / s`; `O = M + D*t`.
     - Back (0xEA or 0xEE, else fake 2): `s = -dot(D, F)`; if `s > 0.1`: `t = ((bb.min.y - q) - 0.2f) / s` (f32); `O = 0x40FE30(M, 0x40FE90(D, t))` (CVector operator* then operator+, cdecl helpers with hidden result pointers).
     - Front (otherwise): `s = dot(D, F)`; if `s > 0.1`: `t = ((bb.max.y - q) + 0.2f) / s`; `O = M + D*t` via the same helpers.
  4. Fire: `ped->+0x528 |= 1` (in a .HOODLUM detour 0x405AF8). If `m_vecCoords.x == 0 && .y == 0 && .z == 0` (each `test ah, 0x44; jp` - NaN counts as non-zero): `CWeapon::Fire(&ped->m_aWeapons[slot], ped, &M, &M, m_pTargetEntity, NULL, &O)` 0x742300. Else `T = m_vecCoords`, transformed by the target's matrix when a target entity exists; `CWeapon::Fire(weapon, ped, &M, &M, m_pTargetEntity, &T, &O)`.
  5. `CPed::DoGunFlash(ped, (u16)[0x8D2E90] (= 250), false)` 0x5DF340.

## 0x628350 CTaskSimpleGangDriveBy::sub_628350 (proposed: CTaskSimpleGangDriveBy::AimGun)  (catalogue: unknown/REA, port level 7)
- Name proposal: aiming step (caller CTaskSimpleGangDriveBy::ProcessPed 0x62D3B0): points the ped's arm IK at the target. Fields: +0x2C m_pAnimAssoc, +0x34 m_pTargetEntity, +0x38 m_vecCoords.
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`; 0x34 bytes of locals; the ped argument slot is reused for a float.
- Behaviour:
  1. `ik = &ped->m_pedIK (+0x50C)`; `ik->m_nFlags (+0x1C, = ped+0x528) &= ~4`. Target point `T = (0, 0, 0)`.
  2. Player (`CPed::IsPlayer`): camera mode test as in 0x627CC0 (`0x31`: `range = fsqrt((dz*dz + dy*dy) + dx*dx) + 20.0f` from ped to `m_aCams[i].m_vecSource`; `0x37`: `range = 20.0f`; other mode -> return without aiming). `S = (P.x, P.y, P.z + [0x858CB0]=0.7f)`; `CCamera::Find3rdPersonCamTargetVector(&TheCamera, range, S (by value), &camPos, &T)` 0x514970.
     AI: if `m_pTargetEntity`: if `m_vecCoords` is all zero (three `test ah, 0x44; jp` checks; NaN counts as non-zero): `T = target->GetBonePosition(3, false)` 0x5E4280 when the target is a ped (`(+0x36 & 7) == 3`), else `T = target position`; else `T = (*target->m_matrix) * m_vecCoords` (matrix allocated/updated first if missing). Without a target entity: `T = m_vecCoords` unless all three components are zero (then T stays 0).
  3. `H = CPed::GetBonePosition(ped, 0x16, false)` (into a zeroed local); then `H.x = P.x`, `H.y = P.y` (ped position; only the bone z is kept). `h = CGeneral::GetRadianAngleBetweenPoints(T.x, T.y, P.x, P.y)` 0x53CBE0 (stored f32 in the argument slot). `pitch = 0`.
  4. Bike correction - if `ped->m_pVehicle && m_pVehicle->m_nVehicleType (+0x590) == 9`: `dd = fsqrt((P.y - T.y)^2 + (P.x - T.x)^2)` (x87: `dy*dy + dx*dx`); `pr = GetRadianAngleBetweenPoints(T.z, dd, H.z, 0.0f)` (ST0); `vh` = vehicle heading (`m_matrix ? atan2(-fwd.x, fwd.y) : m_placement.m_fHeading`); `pitch = pr + sin(h - vh) * veh->+0x648` (fsubr/fsin/fmul/fadd, f32); `c = cos(h - vh)` (vh recomputed, f32); `v = clamp(m_matrix->fwd.z (+0x18), -1, 1)` (if `fwd.z > 1.0` -> 1.0; else if `fwd.z < -1.0` -> -1.0; else fwd.z; tests use `test ah, 0x41` / `test ah, 5`); `pitch = asin(v) * c + pitch` (0x821E70 = CRT asin intrinsic on ST0).
  5. By `m_pAnimAssoc->m_AnimId` (u16; m_pAnimAssoc is dereferenced without NULL check): 0xE7/0xEC -> `flag = 1; h -= [0x858FE4]=pi/2; pitch *= -1.0f`; 0xEB/0xE8 -> `flag = 1; h += pi/2`; 0xEA/0xEE -> `h -= [0x858CB8]=pi; pitch *= -1.0f` (flag 0); other ids -> unchanged (flag 0). (flag is the low byte of a local dword whose upper bytes may hold leftovers of `v`.)
  6. `CPedIK::PointGunInDirection(ik, h, pitch, flag, -1.0f)` 0x5FDC00.

## 0x62D290 CTaskSimpleGangDriveBy::MakeAbortable  (catalogue: specified/REA, port level 10)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`.
- Behaviour:
  1. If `priority != 1 && priority != 2`: `m_nNextCommand (+0xF) = 6`; return 0.
  2. If `event != NULL && event->GetEventType() == 9` (EVENT_DAMAGE, vslot +4): continue only if `event->+0x40 (u8) != 0 && event->+0xC (u8, m_bAddToEventGroup) != 0`; otherwise return 0. (The +0x40 byte lies in the CEventDamage damage-response block; exact gr field unverified.)
  3. `m_nNextCommand = 7`; `if PlayerAnim(this, ped)` 0x627B20: `m_nLastCommand (+0x10) = m_nNextCommand`; if `priority == 2` -> `m_pAnimAssoc->m_BlendDelta (+0x1C) = -1000.0f`; else `if !(m_pAnimAssoc->m_Flags & 1)` -> `m_Flags |= 1` (16-bit or).
  4. `if ped->m_pPlayerData (+0x480)` -> `+0x34 |= 8`. `if CPed::IsPlayer(ped)` -> `GetPlayerInfoForThisPlayerPed(ped)->+0x153 = 1`.
  5. `a = m_pAnimAssoc`; if a: if `!(a->m_Flags & 1)`: `if a->m_BlendDelta > 0` (`test ah, 0x41; je`) -> `m_BlendDelta = -8.0f`; else if `a->m_BlendAmount (+0x18) > 0` and `a->m_BlendDelta >= 0` (`test ah, 1; jne` skip on less/NaN) -> `m_BlendDelta = -8.0f`. Then (regardless of the flag) `a->SetDeleteCallback(0x4D1390 (empty), NULL)` 0x4CEBC0; `m_pAnimAssoc = 0`.
  6. `m_bIsFinished (+8) = 1`; return 1.
