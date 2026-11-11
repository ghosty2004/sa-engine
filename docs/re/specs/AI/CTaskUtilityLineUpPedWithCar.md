# CTaskUtilityLineUpPedWithCar (AI) - RE-SPEC-3 group I3 (written by the lead); ProcessPed 0x6513A0 added by RE-SPEC-AI-2
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

## 0x6513A0 CTaskUtilityLineUpPedWithCar::ProcessPed  (catalogue: specified/REA, port level 9) - RE-SPEC-AI-2 (lead)
- Convention: `__thiscall bool (this = ECX (EBX): CTaskUtilityLineUpPedWithCar*, CPed* ped [esp+4], CVehicle* veh [esp+8] (ESI),
  CAnimBlendAssociation* assoc [esp+0xC] (EBP, may be null))`, `ret 0xC`, SEH frame (temporary CMatrix objects). ALWAYS returns 0
  (0x65279B). Part of the body is relocated: 0x651888 `jmp 0x406B07` (`mov edi,2`; `jmp 0x65188D`). Locals relative to B = esp after the
  four register pushes (entry - 0x2A8): B+0x5B upside-down flag u8, B+0x3C progress f32 (passed to GetPositionToOpenCarDoor), B+0x14 t
  (height blend) f32, B+0x40 posA CVector (final ped position), B+0x4C posB CVector (door position at progress 1), B+0x10 r (anim phase),
  B+0x68 / B+0x78 RtQuat, B+0x94 RtQuatSlerpCache, B+0x2C result RtQuat, B+0xBC RwMatrix, B+0xFC CMatrix.
- Fields of this: +0 m_Offset CVector, +0xC m_fDoorOpenPosZ, +0x10 end time (u32 compared with CTimer::m_snTimeInMilliseconds; gr calls
  it `m_fTime`), +0x14 m_nDoorOpenPosType, +0x18 m_nDoorIdx. Assoc: +0x2C m_nAnimId (i16), +0x20 m_fCurrentTime, +0x14 hierarchy
  (+0x10 m_fTotalTime); "r" = currentTime / totalTime (x87 fdiv, stored f32 where noted). Veh: +0x14 matrix (right +0, forward +0x10,
  up +0x20, pos +0x30), +0x10 placement heading, +0x22 model index, +0x40 physical flags (bit 8 = bSubmergedInWater), +0x384 handling
  (+0xDE anim group u8), +0x590 vehicle type (5 boat, 9 bike), +0x594 sub type. Ped: +0x44 m_vecMoveSpeed, +0x558 m_fCurrentRotation,
  +0x55C m_fAimingRotation.
- Callees: GetPositionToOpenCarDoor 0x650A80, GetLocalPositionToOpenCarDoor 0x64FC10, 0x64E4B0 (cdecl `float (u8 group, i32 animId)`:
  `m_vehicleAnimGroups[group].GetGroup(animId)` (result unused) then returns `ComputeCriticalBlendTime(animId)` in ST0; catalogue FUN_0064e4b0,
  proposed `CVehicleAnimGroupData::ComputeCriticalBlendTime(group, animId)`), CVehicleAnimGroup::GetGroup 0x6E3B00 /
  ComputeCriticalBlendTime 0x6E3C80 (also inlined at 0x6517D3), veh vtable +0xD4 (GetDistanceFromCentreOfMassToBaseOfModel, ST0),
  CPedPlacement::FindZCoorForPed 0x616920, CVehicle::IsUpsideDown 0x6D1D90, CGeneral::LimitRadianAngle 0x53CB50, CMatrix copy ctor 0x59BCF0
  / dtor 0x59ACD0 / SetRotateZ 0x59B0E0 / SetRotateZOnly 0x59B020 / operator*= 0x411A80 / CopyToRwMatrix 0x59B8B0 / UpdateMatrix(RwMatrix*)
  0x59AD20, RtQuatConvertFromMatrix 0x7EB5C0, RtQuatSetupSlerpCache 0x7EC220, Multiply3x3 0x59C790, CPlaceable::SetMatrix 0x54F610,
  CPlaceable::SetOrientation 0x439A80.
- Behaviour:
  1. If m_nDoorOpenPosType == 0: ped->m_vecMoveSpeed = (0,0,0) (0x6513D6-0x651404).
  2. Target heading (0x651407-0x6514AC; veh->matrix is dereferenced at 0x651411 WITHOUT a null check; the later null tests fall back to
     placement heading): hd(m) = atan2(-m.forward.x, m.forward.y) (fpatan). If veh up.z <= -0.8f [0x858C9C] (`test ah,0x41; jp` -> NaN
     counts as not upside-down): flag = 1; heading = hd - 3.1415927f [0x858CB8] if m_nDoorIdx is 8 or 9, else hd. Else: m_nDoorIdx == 18
     -> hd + 3.1415927f [0x858CB8]; else if m_nDoorOpenPosType == 2 -> no store; else hd. Stored f32 into ped->m_fAimingRotation.
  3. progress = 0, t = 0. If assoc, by animId (switch 0x6514D1-0x651881; tables 0x6527B0/0x6527C0, 0x6528BC/0x6528C4, 0x6528D0/0x6528E0):
     c = ComputeCriticalBlendTime(group, animId); "AT400" = if veh model index == 0x241: t = 1.0f.
     - 0x6C-0x6D: progress = r; t = 0.
     - 0x15F-0x162: progress = 1.0f; if |c| < 10.0f [0x85862C]: t = 0; else a = |c| - 11.0f [0x858FBC], s = c - 11.0f [0x858FBC]:
       s > 0.0f [0x858B50]: t = (r >= a or NaN) ? 1.0f : r / a; s <= 0: t = (r >= a or NaN) ? (r - a) / (1.0f [0x858624] - a) : 0. AT400.
     - 0x163-0x166: progress = 1.0f; t = (|c| < 10.0f [0x85862C]) ? 0 : 1.0f; AT400.
     - 0x167-0x16B: progress = 1.0f [0x858624] - r; if |c| >= 10.0f or NaN: t = 1.0f; else with a = |c|: c > 0: t = (r >= a) ? 1.0f : r / a;
       c <= 0: t = (r >= a) ? (r - a) / (1.0f - a) : 0. AT400.
     - 0x16C-0x16E, 0x17C-0x17F: progress = 1.0f, t = 0.  0x16F-0x174, 0x182: progress = 0, t = 1.0f.
     - 0x185-0x186: progress = 1.0f; t = (m_vehicleAnimGroups[group] flags (0xC1CDC8 + 0x94*group) bit 5 (0x20)) ? 1.0f : 0.
     - 0x175-0x178, 0x17A-0x17B, 0x180-0x181, 0x183-0x184: progress = r; a = |c|: c > 0: t = (r >= a) ? 0 : 1.0f - r / a;
       c <= 0: t = (r >= a) ? 1.0f - (r - a) / (1.0f - a) : 1.0f (0x6517D3-0x651881).
     - any other id (incl. 0x179): unchanged (0, 0).
  4. posA = posB = (m_nDoorOpenPosType == 2) ? ped position : GetPositionToOpenCarDoor(veh, progress, assoc) (0x65188D-0x6518DF). If veh is
     not a bike (type != 9) and m_nDoorOpenPosType != 2: posB = GetPositionToOpenCarDoor(veh, 1.0f, assoc) (0x6518E5-0x65190D).
  5. If !(veh->+0x40 & 0x100) (not submerged): R = veh right vector (matrix ? m->right : (cos h, sin h, 0)); lat = R.z*dz + R.y*dy + R.x*dx
     with d = posB - vehPos (x87 order exactly: (Rz*dz + Ry*dy) + Rx*dx, 0x65196E-0x651999); zp = (vehPos.z - veh->vfunc_D4()) + R.z*lat +
     1.0f [0x858624] (0x6519B0-0x6519DA); zOld = posB.z; FindZCoorForPed(&posB); if zp - 0.5f [0x858B8C] > posB.z: posB.z = zOld
     (0x6519E3-0x6519FF). Else if veh type == 5 and IsUpsideDown(): posB.z += 1.0f [0x858624] (0x651A1B-0x651A39).
  6. m_fDoorOpenPosZ = posB.z (0x651A41).
  7. If m_nDoorOpenPosType is 1 or 2 (0x651A44-0x651ADF): vz = ped->m_vecMoveSpeed.z - ms_fTimeStep * 0.008f [0x863984] (GAME_GRAVITY);
     if vz + pedPos.z < posB.z (strict, NaN false): posA.z = posB.z; ped->m_vecMoveSpeed = (0,0,0). Else ped->m_vecMoveSpeed.z = vz;
     posA.z = pedPos.z.
  8. Height blend (0x651AE8-0x651BE6), Z0 = m_fDoorOpenPosZ: if Z0 > posA.z: if veh type == 9 and assoc and animId != 0x16B:
     k = (animId 0x175..0x178) ? 1.0f - progress : (animId 0x167..0x16A) ? ((1.0f < 2r) ? 1.0f : 2r) : 0; posA.z = ((vehPos.z -
     veh->vfunc_D4()) + 1.0f [0x858624] - Z0) * k + Z0. Else posA.z = (posA.z - Z0) * t + Z0. If Z0 <= posA.z (or NaN): the same
     `(posA.z - Z0) * t + Z0` only when m_nDoorOpenPosType == 0, otherwise unchanged.
  9. Heading/offset easing (0x651BEA-0x651D0F): if now < endTime (unsigned): aim = LimitRadianAngle(ped->m_fAimingRotation); cur =
     ped->m_fCurrentRotation; f = (float)(u32)(endTime - now) (fild, + 4294967296.0f [0x858C54] when negative as i32) * 0.0016666667f
     [0x865060]; (if f <= 0.0f: m_Offset.x = m_Offset.y = 0 - dead); m_Offset.z = 0; posA.x -= f * m_Offset.x; posA.y -= f * m_Offset.y;
     posA.z -= 0.0f [0x858B50] * f (all stored f32). If cur + 3.1415927f [0x858CB8] < aim: aim -= 6.2831855f [0x858CBC]; else if
     cur - 3.1415927f > aim: aim += 6.2831855f [0x858CBC]. ped->m_fCurrentRotation = cur - (cur - aim) * (1.0f [0x858624] - f).
     Else ped->m_fCurrentRotation = ped->m_fAimingRotation.
  10. If assoc and animId in 0x167..0x16B (0x651D43-0x6521CB): Mv = copy(veh matrix); if animId == 0x16B: Mv *= RotateZ(3.1415927f
      (imm 0x40490FDB)). qTo = quat(Mv), qFrom = quat(copy(ped matrix)) (CopyToRwMatrix + RtQuatConvertFromMatrix); RtQuatSetupSlerpCache(
      &qFrom, &qTo, &cache); r stored f32. SLERP: r <= 0.0f [0x858B50] -> q = qFrom; r >= 1.0f [0x858624] -> q = qTo; else inlined
      RtQuatSlerp: if cache.nearlyZeroOm: sF = 1 - r, sT = r; else sF = S((1 - r)*omega), sT = S(r*omega) with the RW sine polynomial
      S(x) = x + x^3*((((((x^2*1.589691e-10f [0x86D2B0] - 2.5050760e-08f [0x86D2AC])*x^2 + 2.7557314e-06f [0x86D2A8])*x^2 -
      0.0001984127f [0x86D2A4])*x^2 + 0.008333334f [0x864E0C])*x^2 - 0.16666667f [0x85F0A0]); q.x/y/z/w = to*sT + from*sF (cache raFrom at
      +0, raTo at +0x10, omega +0x20, flag +0x24). QUAT->MATRIX: s = 2.0f [0x858CA0] / (((y*y + x*x) + z*z) + w*w); xs, ys, zs; wx = w*xs,
      wy = w*ys, wz = w*zs, xx = x*xs, yy = y*ys, zz = z*zs, yz = y*zs, xz = z*xs, xy = x*ys; RwMatrix right = (1 - (zz + yy), xy + wz,
      xz - wy), flags = 3, up = (xy - wz, 1 - (zz + xx), yz + wx), at = (xz + wy, yz - wx, 1 - (yy + xx)), pos = 0 (all 1.0f [0x858624]).
      CMatrix M (attach 0) .UpdateMatrix(&rw); M.pos = posA; ped->SetMatrix(&M); return 0.
      If assoc and animId in 0x175..0x179 (0x6521F2-0x65263B): qFrom = quat(copy(veh matrix)); Mp = copy(ped matrix); Mp.SetRotateZOnly(
      ped->m_fCurrentRotation); qTo = quat(Mp); same SLERP (r <= 0 -> qFrom (vehicle), r >= 1 -> qTo) and QUAT->MATRIX; SetMatrix; return 0.
  11. Otherwise (0x652640): if progress <= 0.2f [0x858CC4] and flag == 0 and veh type != 9 and veh sub type != 2: M = copy(veh matrix);
      posA = GetLocalPositionToOpenCarDoor(veh, 0.0f, assoc); M.pos += Multiply3x3(M, posA) (x, y, z order); ped->SetMatrix(&M) (the ped
      takes the vehicle orientation); else ped position = posA (matrix pos or placement) and ped->SetOrientation(0, 0, m_fCurrentRotation).
      Return 0.
- Edge cases: the return value is always false (callers ignore it). Step 2 reads veh->matrix->up.z before any null check. Step 11 uses
  `progress > 0.2f or NaN` as the simple path.
- gta-reversed: plugin::CallMethodAndReturn (TaskUtilityLineUpPedWithCar.cpp:45), not reversed.
- Open questions: none affecting behaviour; the cache field layout (+0x20 omega, +0x24 nearlyZeroOm) is taken from RW's RtQuatSlerpCache
  and matches the reads at 0x651EC2/0x651EDB.
