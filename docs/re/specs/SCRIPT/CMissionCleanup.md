# CMissionCleanup (SCRIPT) — RE-SPEC-REST

## 0x4637C0 CMissionCleanup::FindFree  (catalogue: specified/REA, port level 0)
- `__thiscall`, no args, plain `ret`; returns EAX = pointer to the first entry with `type (+0) == 0` among the 75 (stride 8, u16 counter),
  or NULL. Callers: none in game code.

## 0x4637E0 CMissionCleanup::AddEntityToList(int32 handle, MissionCleanUpEntityType type)  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = CMissionCleanup*; layout gr MissionCleanup.h: 75 entries {u8 type +0, i32 handle +4} stride 8, i8 m_Count at +0x258),
  handle [esp+4], u8 type [esp+8], `ret 8`. Stub 0x4637E0 → .HOODLUM body 0x1565D40. 16 callers (CCarCtrl::CreateCarForScript 0x431F80,
  script command switches 100-199, 200-299, 1800-1899, 2000-2099, 2200-2299, ...).
- Behaviour: first entry i (u16 counter, i < 0x4B) with `type == 0`; none → return. Else `entry.handle = handle; entry.type = type;
  m_Count++` (byte increment, wraps). Duplicates are not checked.
- gta-reversed: catalogue REA (gr C++ exists in MissionCleanup.cpp; not hooked); same logic.

## 0x4652D0 CMissionCleanup::CheckIfCollisionHasLoadedForMissionObjects  (catalogue: specified/REA, port level 9)
- `__thiscall` (ECX = this), no args, plain `ret`. Stub → .HOODLUM body 0x1569D40. Caller: CTheScripts::Process 0x46A000.
  Callees: CColStore::HasCollisionLoaded 0x410CE0 and CIplStore::HaveIplsLoaded 0x405600 (both cdecl (const CVector* pos, i32 area)),
  CEntity::GetIsStatic 0x4633E0, CPhysical::AddToMovingList 0x542800, CAutomobile::PlaceOnRoadProperly 0x6AF420,
  CBike::PlaceOnRoadProperly 0x6BEEB0.
- Behaviour: for each of the 75 entries (in order):
  - type 3 (object pool [0xB7449C], 0x19C) or type 2 (ped pool [0xB74490], 0x7C4): GetAt(handle) as in RemoveEntityFromList; if found and
    `m_nFlags (+0x1C) & 0x40000`: `pos = m_matrix (+0x14) ? &matrix.pos (+0x30) : &m_placement.pos (+0x4)`, `area = (u8)m_nAreaCode (+0x2F)`;
    if `HasCollisionLoaded(pos, area)` and then (pos re-derived) `HaveIplsLoaded(pos, area)`: clear 0x40000 and, if `!GetIsStatic()`,
    `AddToMovingList()`.
  - type 1 (vehicle pool [0xB74494], 0xA18): same checks and flag clear/AddToMovingList, then by `m_nVehicleSubType (+0x590)`: 0 or 11 →
    `CAutomobile::PlaceOnRoadProperly`; 9 → `CBike::PlaceOnRoadProperly`; others nothing (0x1569ED3-0x1569EE6, `sub 0; je`, `sub 9; je`,
    `sub 2; jne`).
  - other types: nothing.

## 0x4654B0 CMissionCleanup::RemoveEntityFromList(int32 handle, MissionCleanUpEntityType type)  (catalogue: specified/REA, port level 3)
- `__thiscall` (ECX = this → EBP), handle [esp+4], u8 type [esp+8], `ret 8`. Stub → .HOODLUM body 0x156E810. 12 callers
  (CRemote::TakeRemoteControlledCarFromPlayer 0x45AE80, CMissionCleanup::Process 0x468560, several command switches).
  Callees: CEntity::GetIsStatic 0x4633E0, CPhysical::AddToMovingList 0x542800.
- Behaviour: for ALL 75 entries (no early exit): if `entry.type == type && entry.handle == handle`:
  1. Entity lookup by type: 1 → vehicle pool [0xB74494] (size 0xA18), 2 → ped pool [0xB74490] (0x7C4), 3 → object pool [0xB7449C]
     (0x19C); pool GetAt: `idx = handle >> 8` (sar), valid when `flags[idx] == (u8)handle`, `ptr = objects + idx*size`. Other types: no
     lookup. If ptr != NULL and `ptr.m_nFlags (+0x1C) & 0x40000` (bIsStaticWaitingForCollision): clear that bit; then if
     `!ptr->GetIsStatic()` → `ptr->AddToMovingList()`.
  2. `entry.handle = 0; entry.type = 0; m_Count--` (byte).
- Edge: with type == 0 every empty slot whose handle equals the argument (normally 0) matches, so `RemoveEntityFromList(0, 0)` decrements
  m_Count once per empty slot (original behaviour).
