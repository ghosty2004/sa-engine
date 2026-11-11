# CRemote (CALLED-ONLY) — RE-SPEC-REST

Player info: `CWorld::Players` at 0xB7CD98, stride 0x190, index `CWorld::PlayerInFocus` u8 [0xB7CD74]. Fields used (gr PlayerInfo.h):
+0xB0 m_pRemoteVehicle (table 0xB7CE48), +0xDD m_bAfterRemoteVehicleExplosion (0xB7CE75), +0xDE m_bCreateRemoteVehicleExplosion (0xB7CE76),
+0xDF m_bFadeAfterRemoteVehicleExplosion (0xB7CE77), +0xE0 m_nTimeOfRemoteVehicleExplosion (0xB7CE78).

## 0x45AB10 CRemote::GivePlayerRemoteControlledCar(f32 x, f32 y, f32 z, f32 angle, uint16 model)  (catalogue: specified/REA, port level 17)
- `__cdecl`, args [esp+4..+0x14] (model read as a u16 from its dword slot; the z and model slots are reused as scratch), plain `ret`;
  SEH frame. Caller: ProcessCommands1100To1199 0x48A320. Callees: CModelInfo::IsHeliModel 0x4C5B00 / IsPlaneModel 0x4C5B30 (cdecl),
  CVehicle::operator new 0x6E2D50, CHeli ctor 0x6C4190, CPlane ctor 0x6C8E20, CAutomobile ctor 0x6B0A90, CWorld::FindGroundZFor3DCoord
  0x5696C0, CEntity::GetDistanceFromCentreOfMassToBaseOfModel 0x536BE0, CMatrix::SetRotateZOnly 0x59B020, CCarCtrl::JoinCarWithRoadSystem
  0x42F5A0, CWorld::Add 0x563220, CWorld::FindPlayerVehicle 0x56E0D0, CEntity::RegisterReference 0x571B70, CCamera::TakeControl 0x50C7C0,
  CCamera::SetZoomValueCamStringScript 0x50C1B0.
- Behaviour:
  1. Create: heli → `new(0xA18)` + `CHeli(model, 2)`; else plane → `new(0xA04)` + `CPlane(model, 2)`; else `new(0x988)` +
     `CAutomobile(model, 2, 1)` (created-by 2 = mission). A NULL allocation gives v = NULL and the code below crashes (no check).
  2. `g = FindGroundZFor3DCoord(x, y, z + [0x858CA0]=2.0f, (bool*)&modelSlot, NULL)`; `z = v->GetDistanceFromCentreOfMassToBaseOfModel() + g`
     (stored f32).
  3. `v->m_matrix->SetRotateZOnly(angle)` (matrix pointer +0x14 used without a NULL check), then position = (x, y, z) into the matrix
     (+0x30..+0x38) if present, else into the placement (+4..+0xC).
  4. `v+0x36 = (v+0x36 & 7) | 0x40` (status 8, STATUS_PLAYER_REMOTE); `v+0x428 |= 8`; `JoinCarWithRoadSystem(v)`; bytes `+0x3BA, +0x3BB,
     +0x3B9 = 0`; `+0x3C8 = 9.0f` (immediate 0x41100000); `+0x3D0 = 9` (u8); `+0x3B7, +0x3B8 = 0`; then bit 0x10 of +0x428 = !(v+0x42D & 2).
  5. `CWorld::Add(v)`. If `FindPlayerVehicle(-1, false)` != NULL: its `+0x36 = (s & 7) | 0x48` (status 9).
  6. `Players[PlayerInFocus].m_pRemoteVehicle (0xB7CE48 + 0x190*idx) = v; v->RegisterReference(&that slot)`.
  7. `TheCamera (0xB6F028).TakeControl(v, 0x12, 1, 1)`; `TheCamera.SetZoomValueCamStringScript(1)`.
- gta-reversed: `plugin::Call` only.

## 0x45AE80 CRemote::TakeRemoteControlledCarFromPlayer(bool createExplosion)  (catalogue: specified/REA, port level 4)
- `__cdecl`, u8 at [esp+4], `ret`. Stub 0x45AE80 jumps to the .HOODLUM body 0x1562160. Callers: CPlayerInfo::BlowUpRCBuggy 0x56EA30,
  CAutomobile::ProcessAI 0x6B4800 (2 sites). Callees: CVehicle::SetVehicleCreatedBy 0x6D5D70 (thiscall, i32), CMissionCleanup::
  RemoveEntityFromList 0x4654B0 (thiscall on CTheScripts::MissionCleanUp 0xA90850, args (handle, type 1 = vehicle)).
- Behaviour (player P = Players[PlayerInFocus], re-indexed at every use):
  1. `v = P.m_pRemoteVehicle` (no NULL check). If `v.m_nCreatedBy (+0x4A4) == 2` (MISSION_VEHICLE): `v->SetVehicleCreatedBy(1)`;
     `handle = (index << 8) + pool.flags[index]` with `index = (v - pool.objects) / 0xA18` (signed magic-number divide 0x6572EC3,
     `sar 6`) on CPools::ms_pVehiclePool [0xB74494] (+0 objects, +4 byte flags); `MissionCleanUp.RemoveEntityFromList(handle, 1)`.
  2. `v.byte(+0x428) &= 0xF7` (clears bit 3 of that vehicle flag byte; gr name not verified).
  3. `P.m_nTimeOfRemoteVehicleExplosion = CTimer::m_snTimeInMilliseconds [0xB7CB84]`; `P.m_bAfterRemoteVehicleExplosion = 1`;
     `P.m_bCreateRemoteVehicleExplosion = createExplosion`; `P.m_bFadeAfterRemoteVehicleExplosion = 1` (0x15621FB-0x156220E).
  m_pRemoteVehicle itself is NOT cleared here.
- gta-reversed: `plugin::Call` only.
