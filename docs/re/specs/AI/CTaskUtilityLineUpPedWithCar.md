# CTaskUtilityLineUpPedWithCar (AI) - RE-SPEC-3 group I3 (written by the lead; ProcessPed 0x6513A0 is NOT written yet)
Layout (gr, size 0x1C): +0 m_Offset CVector, +0xC m_fDoorOpenPosZ, +0x10 m_fTime, +0x14 m_nDoorOpenPosType, +0x18 m_nDoorIdx (8..11, 18).
Vehicle anim groups: `CVehicleAnimGroupData::m_vehicleAnimGroups` 0xC1CDC0 (stride 0x94, index = handling +0xDE), flag dword at +8
(0xC1CDC8 + 0x94*g); `CVehicleAnimGroup::ComputeAnimDoorOffsets(CVector* out, int32 type)` 0x6E3D10 (thiscall on the group).
Vehicle model info (ms_modelInfoPtrs) +0x3C vehicle type, +0x5C vehicle structure (door dummy positions at +0x00/+0x30 and +0x3C).

## 0x64FC10 CTaskUtilityLineUpPedWithCar::GetLocalPositionToOpenCarDoor  (catalogue: specified/REA, port level 4)
- `__thiscall CVector* (this, CVector* out [esp+4], CVehicle* veh [+8], float t [+0xC], CAnimBlendAssociation* a [+0x10])`, `ret 0x10`, returns out.
- 1. If `veh->+0x429 & 1` and m_nDoorIdx is 11 or 9: `t = 0`; else `t *= veh->m_pHandlingData->+0xD4` (f32).
- 2. Door-anim type `k` from `a` (`id = (i16)a->+0x2C`): a == 0 -> 0; id 0x6C..0x6D -> 3; 0x15F..0x174 -> 0; 0x175..0x178 -> 3; 0x17A..0x17B -> 5;
  0x17C..0x17F -> 3; 0x183..0x184 -> 3; 0x185..0x186 -> 0 (byte table 0x6500F8, jump table 0x6500EC); every other id (0x179, 0x180..0x182,
  > 0x186, < 0x6C ...) -> k = the raw VALUE OF THE veh ARGUMENT SLOT (`mov esi,[esp+0x54]` at 0x64FCA2: the vehicle pointer reinterpreted as
  an int) - ORIGINAL BUG (open question: whether such anims ever reach here; ComputeAnimDoorOffsets then indexes with a pointer).
- 3. If k == 0 or k == 3: door 10 or 8 -> k = 0; door 11 or 9 -> k = 1; door 18 -> k = 2; else unchanged. If k == 5 -> k = 0.
- 4. `D`: if a and id is 0x185/0x186 and the group flag bit 5 is set: `O1 = group[g].ComputeAnimDoorOffsets(k)`; `f = 1.0f [0x858624] -
  a->+0x18`; `A = O1 * f`; `O2 = group[0].ComputeAnimDoorOffsets(k)` (ECX = 0xC1CDC0, i.e. anim group 0 regardless of the vehicle - literal
  bytes 0x64FD36); `D = O2 * a->+0x18 + A` (per component, B first). Otherwise `D = group[g].ComputeAnimDoorOffsets(k)`.
- 5. `L` by door (jump table 0x650104, byte table 0x65011C over door-8 = 0..10): C1 = vehicle structure +0x00 if model type == 5 else +0x30;
  C2 = structure +0x3C.
  - door 8: `L = ((C1.x + t) + D.x, C1.y - D.y, C1.z - D.z)` (computed as `(C1.x + t) - (-D.x)`).
  - door 9: same with C2.
  - door 10: `L = (-(C1.x + t) - D.x, C1.y - D.y, C1.z - D.z)`.
  - door 11: same with C2.
  - door 18: `L = (D.x + C1.x, D.y + C1.y, D.z + C1.z)`.
  - doors 12..17: `L = C1` (D ignored).
- 6. If `veh->m_nVehicleType == 9` (bike) and door != 18: `W = CBike::GetCorrectedWorldDoorPosition(veh, &W, D, C)` 0x6BF230 (two CVectors by
  value: D, then the door dummy C used above); `L = 0x59C810(&L, &(W - vehPos), veh->m_matrix)` (inverse-rotate the world offset into vehicle
  space).
- 7. `*out = L`.
- gta-reversed: `.Reversed = false`. Confidence medium (register reuse across the jump tables; re-verify step 6 argument order).

## 0x650A80 CTaskUtilityLineUpPedWithCar::GetPositionToOpenCarDoor  (catalogue: specified/REA, port level 1)
- `__thiscall CVector* (this, CVector* out [esp+4], CVehicle* veh [+8], float t [+0xC], CAnimBlendAssociation* a [+0x10])`, `ret 0x10`, SEH.
- `M = CMatrix(*veh->m_matrix)` 0x59BCF0 (local copy; no null check); `L = GetLocalPositionToOpenCarDoor(&L, veh, t, a)`; `W = Multiply3x3(&W, &M, &L)`
  0x59C790; `*out = (P.x + W.x, P.y + W.y, P.z + W.z)` with P = veh position (matrix ? +0x30 : +4); `M.~CMatrix()` 0x59ACD0; return out.
