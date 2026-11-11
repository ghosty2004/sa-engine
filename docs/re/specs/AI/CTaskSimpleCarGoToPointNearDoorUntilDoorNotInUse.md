# CTaskSimpleCarGoToPointNearDoorUntilDoorNotInUse (AI) - RE-SPEC-FINAL (helper B)

## 0x646F40 CTaskSimpleCarGoToPointNearDoorUntilDoorNotInUse::ComputeTarget  (catalogue: specified/REA, port level 7)
- `__thiscall void (this, CVector* out [esp+4])`, `ret 4`. Caller: ProcessPed 0x64B4A0. `this->+0xC` points to a struct with +8 = CVehicle*, +0xC = target door.
- Behaviour: `d = CCarEnterExit::GetPositionToOpenCarDoor(&tmp, veh, door)` 0x64E740 (cdecl, result in local `D`); `V = veh->m_matrix ? m_matrix->pos : veh->m_placement.pos` (copied to locals as raw dwords). x87, in this order: `dx = D.x - V.x`; `dy = D.y - V.y`; `dz = D.z - V.z`, stored to f32 local; `2dx = dx + dx`, stored to f32 local; `2dy = dy + dy` (kept in ST); `2dz = (f32 dz) + (f32 dz)` (reloaded); `out.x = (f32 2dx) + V.x`; `out.y = V.y + 2dy`; `out.z = V.z + 2dz` (each fstp f32). I.e. `out = V + 2*(D - V)` (the door point mirrored away from the vehicle centre).
