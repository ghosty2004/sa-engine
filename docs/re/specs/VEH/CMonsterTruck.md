# CMonsterTruck (VEH) - full specs

Layout (gr `MonsterTruck.h`, size 0x99C, derives CAutomobile 0x988): +0x988 `field_988[4]` f32 (gr: "unused"; the
original writes 1.0 there in ResetSuspension), +0x998 `m_fSuspensionRadius` f32 (written by SetupSuspensionLines =
front wheel size * 0.5, read by ExtendSuspension).
CAutomobile fields used (gr `Automobile.h`, offsets derived from the VALIDATE_OFFSETs): +0x724 `m_wheelColPoint[4]`
(CColPoint, stride 0x2C: +0x00 point, +0x10 normal, +0x24 pieceTypeB), +0x7D4 `m_fWheelsSuspensionCompression[4]`,
+0x838 `m_wheelPosition[4]`, +0x878 `m_aSuspensionSpringLength[4]`, +0x888 `m_aSuspensionLineLength[4]`,
+0x898 `m_fFrontHeightAboveRoad`, +0x89C `m_fRearHeightAboveRoad`, +0x8E8 `m_apWheelCollisionEntity[4]`,
+0x8F8 `m_vWheelCollisionPos[4]` (CVector). CVehicle +0x384 `m_pHandlingData` (tHandlingData: +0xAC
`m_fSuspensionForceLevel`, +0xB0 `m_fSuspensionDampingLevel`, +0xB8 `m_fSuspensionUpperLimit`, +0xBC
`m_fSuspensionLowerLimit`, +0xC0 `m_fSuspensionBiasBetweenFrontAndRear`). CPhysical +0x44 moveSpeed, +0x50 turnSpeed,
+0x8C `m_fMass`. CEntity +0x14 `m_matrix` (CMatrix*: up at +0x20, pos at +0x30), +0x04 placement pos, +0x1C flags,
+0x22 model index, +0x36 type (bits 0-2) / status (bits 3-7).
Globals (data, gr `MonsterTruck.cpp`/`.h`): 0x8D33A8 `DUMPER_COL_ANGLEMULT` = 0.0002f, 0x8D33AC `fWheelExtensionRate` =
0.1f, 0x8D33B0 u8 = 1 (unnamed, proposed `bMonsterTruckClampCompression`), 0x8D33B4 f32 = -0.05f, 0x8D33B8 f32 = 0.05f
(both unnamed). Vtable 0x8717D8 (71 slots).

Note on names: in this class the slots +0x878 / +0x888 do not hold lengths. SetupSuspensionLines stores wheel-centre
z + upper limit / + lower limit there (heights in model space). The specs keep the gr names for the offsets.

## 0x6C7D40 CMonsterTruck::ResetSuspension  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (CMonsterTruck* this)`, plain `ret`. Virtual slot 67 (+0x10C).
- Callers: CAutomobile::Teleport 0x6A9CA0 (virtual). Callees: CAutomobile::ResetSuspension 0x6A2AE0.
- Behaviour: `CAutomobile::ResetSuspension(this)` (0x6C7D43); then for i = 0..3 (0x6C7D60 loop, ascending):
  `m_wheelPosition[i] (+0x838) = m_aSuspensionLineLength[i] (+0x888)` (dword copy, 0x6C7D60-0x6C7D63) and
  `field_988[i] (+0x988) = 1.0f` (0x6C7D65, `[eax+0x150]`).
- gta-reversed: `plugin::CallMethod` followed by dead fallback code that fills `m_wheelPosition` with 1.0f. That
  fallback is wrong against the bytes: the original copies the line length into the wheel position and writes 1.0f to
  `field_988`, not to `m_wheelPosition`.

## 0x6C7D80 CMonsterTruck::ExtendSuspension  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (CMonsterTruck* this)`, plain `ret`. 4-byte local (temp f32).
- Callers: ProcessControl 0x6C8250 (tail call), ProcessControlCollisionCheck 0x6C8330. Callees: none.
- Globals read: `CTimer::ms_fTimeStep` 0xB7CB5C (f32), `fWheelExtensionRate` 0x8D33AC (f32 0.1).
- Behaviour, for i = 0..3 ascending (edx = &m_wheelPosition[i]):
  1. `v = m_wheelPosition[i] - ((ms_fTimeStep * m_fSuspensionRadius[+0x998]) * 0.1f)` (0x6C7D90-0x6C7DA2: fld timestep,
     fmul [this+0x998], fmul [0x8D33AC], fsubr [edx]); stored as f32 both to the local and to `m_wheelPosition[i]`.
  2. `if (v < m_aSuspensionLineLength[i] (+0x888))` (fcomp, `test ah,5; jp` at 0x6C7DAE-0x6C7DB6: taken when
     v >= line or unordered): `m_wheelPosition[i] = m_aSuspensionLineLength[i]` (0x6C7DB8, 0x6C7DCE).
     Else `if (v > m_aSuspensionSpringLength[i] (+0x878))` (0x6C7DBD-0x6C7DC9, `test ah,0x41; jne` skips when
     v < spring, v == spring or unordered): `m_wheelPosition[i] = m_aSuspensionSpringLength[i]`.
     NaN keeps NaN.
  3. `m_fWheelsSuspensionCompression[i] (+0x7D4) = 1.0f` (0x6C7DD0, unconditional).
- Oddity: the clamp is "below the line value -> line value, above the spring value -> spring value", in that order.
  Reproduce exactly (do not reorder into a std::clamp).
- gta-reversed: `plugin::CallMethod` only (no C++).

## 0x6C7F90 CMonsterTruck::vfunc_0  (catalogue: specified/REA, port level 17)
- Convention: `__thiscall CMonsterTruck* (CMonsterTruck* this, u8 flags [esp+4])`, `ret 4`. Virtual slot 0 (scalar
  deleting destructor).
- Callees: 0x6C7D10 (CMonsterTruck::~CMonsterTruck: `mov [ecx], 0x8717D8` then `jmp CAutomobile::Destructor 0x6A61E0`),
  `operator delete` 0x6E2D90 (cdecl).
- Behaviour: call 0x6C7D10 on this; `if (flags & 1) delete(this)`; return this (EAX).
- Name: proposed `CMonsterTruck::scalar_deleting_destructor`.

## 0x6C7FB0 CMonsterTruck::SetupSuspensionLines  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (CMonsterTruck* this)`, plain `ret`. Locals 0x28. Virtual slot 48 (+0xC0).
- Callers: CMonsterTruck::Constructor 0x6C8D60, virtual dispatch (e.g. CVehicle::ClearVehicleUpgradeFlags 0x6D3210).
- Callees: CVehicleModelInfo::GetWheelPosn 0x4C7D20 (thiscall (int32 wheel, CVector& out, bool local), callee-cleaned),
  CColDisk::Set 0x40FD50 (thiscall, ret 0x1C: (f32 radius -> +0xC, const CVector& centre -> +0x0, const CVector& axis
  -> +0x14, f32 thickness -> +0x20, u8 material -> +0x10, u8 piece -> +0x11, u8 lighting -> +0x12)),
  CMemoryMgr::Malloc 0x72F420 (cdecl (size, hint), only `size` used: plain `malloc`), CMemoryMgr::Free 0x72F430.
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8. Constants: 0x858B8C = 0.5f, 0x858CC8 = 0.6f, 0x858B90 = 4.0f,
  0x858624 = 1.0f.
- Fields: model info `mi` (+0x14 `m_pColModel`, +0x40 `m_fWheelSizeFront`); CColModel `cm` (+0x00 bbox min, +0x0C bbox
  max, +0x24 bounding-sphere radius, +0x2C `m_pColData`); CCollisionData `cd` (+0x06 `m_nNumLines` u8, +0x07 flags
  bit0 `bUsesDisks`, +0x10 `m_pLines`/`m_pDisks`).
- Behaviour:
  1. `mi = ms_modelInfoPtrs[(i16)this->m_nModelIndex]`, `cm = mi->m_pColModel`, `cd = cm->m_pColData`.
     `m_fSuspensionRadius (+0x998) = mi->m_fWheelSizeFront * 0.5f` (0x6C7FC7-0x6C7FDD).
  2. Disk storage (0x6C7FE3-0x6C802F):
     - if `cd->m_pDisks == nullptr`: `cd->flags |= 1` (bUsesDisks), `cd->m_nNumLines = 4`, `cd->m_pDisks = Malloc(0x90)`.
     - else if `(cd->flags & 1) == 0`: `Free(cd->m_pDisks)`, then the same three writes (flags |= 1, numLines = 4,
       new Malloc(0x90)). Order inside: flags written before numLines, both before the Malloc call.
     - else: keep the existing array (numLines is NOT rewritten).
  3. Local `axis = (0,0,0)` (0x6C8038-0x6C8048). Loop i = 0..3 (ebx; `ebp = i*0x24` into the disk array):
     - `mi->GetWheelPosn(i, pos, false)`.
     - `axis.x = (i < 2) ? -1.0f : 1.0f` (signed 16-bit compare of the counter with 2, 0x6C8071-0x6C8081). y,z stay 0.
     - `cd->m_pDisks[i].Set(radius, pos, axis, radius * 0.6f, 0x3C /*material*/, 0x0D /*piece*/, 0xFF /*lighting*/)`
       with `radius = m_fSuspensionRadius` (raw dword), thickness computed `fld [+0x998]; fmul 0.6f` stored f32.
       The disk pointer is re-read from `cd->m_pDisks` for each call.
     - piece override (+0x11): i == 1 -> 0x0F, i == 2 -> 0x0E, i == 3 -> 0x10 (i == 0 keeps 0x0D) (0x6C80BD-0x6C80DF).
     - `m_aSuspensionSpringLength[i] (+0x878) = pos.z + handling->m_fSuspensionUpperLimit (+0xB8)`;
       `m_aSuspensionLineLength[i] (+0x888) = pos.z + handling->m_fSuspensionLowerLimit (+0xBC)` (both f32 stores,
       handling pointer re-read each time, 0x6C80E4-0x6C811F).
  4. `h = (1.0f - 1.0f / (handling->m_fSuspensionForceLevel (+0xAC) * 4.0f)) * (spring[0] - line[0])
     + (m_fSuspensionRadius - spring[0])` (0x6C8132-0x6C816F; x87 order: `t = force*4; t = 1/t; t = 1 - t;
     u = spring0 - line0; p = t*u; w = radius - spring0; h = p + w`, no intermediate stores). Store h (f32) to both
     `m_fFrontHeightAboveRoad` (+0x898) and `m_fRearHeightAboveRoad` (+0x89C).
  5. For i = 0..3: `m_fWheelsSuspensionCompression[i] = 1.0f`; `m_wheelPosition[i] = mi->m_fWheelSizeFront * 0.5f -
     m_fFrontHeightAboveRoad` (front size used for all four wheels, front height for all four; 0x6C8180-0x6C819E).
  6. `t = m_fFrontHeightAboveRoad - m_fSuspensionRadius` (0x6C81A3-0x6C81AD); `if (t < cm->bbox.min.z)` (jp skip on
     >= or NaN, 0x6C81B9-0x6C81C1) `cm->bbox.min.z = t`.
  7. `a = |cm->bbox.min|` (sqrt((x*x + y*y) + z*z), x87, fsqrt), `b = |cm->bbox.max|` same order. `r = (a > b) ? a : b`
     (fcompp + `test ah,0x41; je`: a is kept only when a > b; equal or NaN -> b, recomputed from memory at
     0x6C8216-0x6C8237). `if (r > cm->boundSphere.radius (+0x24))` (`test ah,0x41; jne` skips on <, ==, NaN) store r
     (f32) to +0x24. The magnitudes are not rounded to f32 before the compares.
- gta-reversed: `plugin::CallMethod` only.
- Open questions: none.

## 0x6C8250 CMonsterTruck::ProcessControl  (catalogue: specified/REA, port level 26)
- Convention: `__thiscall void (CMonsterTruck* this)`, plain `ret` (or tail jump to ExtendSuspension). Virtual slot 10.
- Callees: CAutomobile::ProcessControl 0x6B1880, CMonsterTruck::ExtendSuspension 0x6C7D80 (tail call, `jmp`).
- Globals: u8 0x8D33B0 (=1, read once into BL at entry, 0x6C8251). Constants 0x858624 = 1.0f, 0x858B50 = 0.0f.
- Behaviour:
  1. For i = 0..3 (ecx = &m_fWheelsSuspensionCompression[i]):
     - `if (comp[i] < 1.0f)` (`test ah,5; jp` -> else-branch on >= or NaN):
       `comp[i] = (spring[i] - wheelPos[i]) / (spring[i] - line[i])` (fld [+0x878]; fsub [+0x838]; fld [+0x878];
       fsub [+0x888]; fdivp; `fst` to comp[i], 0x6C8274-0x6C828B); then `if (comp[i] < 0.0f && flag_8D33B0 != 0)`
       `comp[i] = 0` (the compare uses the unrounded x87 quotient, 0x6C828D-0x6C829E).
     - else `comp[i] = 1.0f` (0x6C82A6).
  2. `CAutomobile::ProcessControl(this)` (0x6C82B4).
  3. If entity flag bit 6 of the dword at +0x1C (`m_bWasPostponed`) is set: return (0x6C82B9-0x6C82C1).
  4. If any of moveSpeed.x/y/z (+0x44/+0x48/+0x4C) or turnSpeed.x/y/z (+0x50/+0x54/+0x58) compares not-equal to 0.0f
     (in that order; NaN counts as not-equal, `test ah,0x44; jp`): tail-call `ExtendSuspension()`. Otherwise return.
- gta-reversed: `plugin::CallMethod` only.

## 0x6C8330 CMonsterTruck::ProcessControlCollisionCheck  (catalogue: specified/REA, port level 23)
- Convention: `__thiscall void (CMonsterTruck* this, bool applySpeed [esp+4])`, `ret 4`. Virtual slot 23.
- Callees: ExtendSuspension 0x6C7D80, CAutomobile::ProcessControlCollisionCheck 0x6A29C0 (the arg dword is forwarded).
- Behaviour: `ExtendSuspension(); CAutomobile::ProcessControlCollisionCheck(applySpeed);` then for i = 0..3:
  `if (comp[i] < 1.0f)` (jp on >= / NaN) `comp[i] = (spring[i] - wheelPos[i]) / (spring[i] - line[i])` (same x87 order
  as ProcessControl, 0x6C8373-0x6C837F) else `comp[i] = 1.0f`. No clamp to 0 here.
- gta-reversed: `plugin::CallMethod` only.

## 0x6C83A0 CMonsterTruck::ProcessSuspension  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (CMonsterTruck* this)` (edi = this), plain `ret`. Locals 0xE8. Virtual slot 70 (+0x118).
- Callers: CAutomobile::ProcessControl 0x6B1880 (virtual).
- Callees: CPhysical::ApplySpringCollisionAlt 0x543D60 (thiscall (f32 springConst, CVector& dir, CVector& point,
  f32 springRatio, f32 bias, CVector& normal, f32& outSpringForce)), CPhysical::GetSpeed 0x542CE0 (thiscall, hidden
  return pointer first, then CVector point by value, callee-cleaned), CPhysical::ApplySpringDampening 0x543E90
  (thiscall (f32 damping, f32 springForce, CVector& dir, CVector& point, CVector& speed)), CPhysical::ApplyForce
  0x542B50 (thiscall (CVector force, CVector point, bool updateTurnSpeed) by value), virtual slot 56 (+0xE0)
  `CVehicle::VehicleDamage` on another vehicle.
- Constants: 0x858C1C = -1.0f, 0x858624 = 1.0f, 0x8717D4 = 1.0f, 0x858F9C = 0.35f, 0x858B8C = 0.5f, 0x858C84 = 0.25f,
  0x8D33B8 = 0.05f, 0x8D33B4 = -0.05f.
- Locals (names for the port): `dir[4]` (esp+0x70, CVector), `pt[4]` (esp+0x40), `force[4]` (esp+0xB8, f32),
  `speed[4]` (esp+0xC8, CVector).
- Behaviour:
  1. For i = 0..3 (unrolled, 0x6C83AC-0x6C86F1), with `m = this->m_matrix` (read once, used without a null check for
     the up vector):
     - `dir[i] = m->up * -1.0f` (each component `fld; fmul -1.0f; fstp f32`).
     - `if (comp[i] < 1.0f)` (jp on >= / NaN): `pt[i] = m_wheelColPoint[i].point - GetPosition()` (position =
       `m ? &m->pos : &this->placement.pos`, componentwise f32 subtraction); else `pt[i] = (0,0,0)`.
  2. Spring loop, i = 0..3 (0x6C8702-0x6C8779): if `comp[i] < 1.0f`: `bias = handling->m_fSuspensionBiasBetweenFrontAndRear`;
     if i == 1 or i == 3 (rear wheels): `bias = 1.0f - bias` (f32 store). Then
     `ApplySpringCollisionAlt(handling->m_fSuspensionForceLevel, dir[i], pt[i], comp[i], bias,
     m_wheelColPoint[i].normal, force[i])`. `force[i]` is left unwritten (stack garbage) when comp[i] >= 1, but is
     then never read.
  3. Speed loop, i = 0..3 (0x6C87A0-0x6C88B2): `speed[i] = this->GetSpeed(pt[i])`; if `m_apWheelCollisionEntity[i]`
     (+0x8E8) is non-null: `speed[i] -= entity->GetSpeed(m_vWheelCollisionPos[i] (+0x8F8))` (componentwise). Then if
     `comp[i] < 1.0f` and `m_wheelColPoint[i].normal.z > 0.35f` (`test ah,0x41; jne` skips <, ==, NaN):
     `dir[i] = -m_wheelColPoint[i].normal` (fchs per component).
  4. Damping loop (0x6C88D0-0x6C8915): if `comp[i] < 1.0f`:
     `ApplySpringDampening(handling->m_fSuspensionDampingLevel (+0xB0), force[i], dir[i], pt[i], speed[i])`.
  5. Contact loop, i = 0..3 (0x6C8930-0x6C8ACE; ebx = &m_apWheelCollisionEntity[i], esi = &colpoint[i].point.z):
     - if `comp[i] < 1.0f`, entity `e` non-null and `(e->type & 7) == 2` (vehicle):
       a. if `comp[i] < 0.5f` (jp on >= / NaN): `e->VehicleDamage((1.0f - comp[i]) * this->m_fMass * 0.05f [0x8D33B8],
          pieceTypeB, this, &colpoint[i].point, &colpoint[i].normal, 0x31 /*WEAPON_RAMMEDBYCAR*/)`.
          x87 order `fld 1.0; fsub comp; fmul [this+0x8C]; fmul 0.05`. The component arg is pushed as `edx` after
          `movzx dx, byte [colpoint+0x24]`: only the low 16 bits are defined, the upper 16 bits hold bits of an address
          (0x6C898B-0x6C899D). The callee must only use the low 16 bits.
       b. if `colpoint[i].normal.z > 0.5f` (`test ah,0x41; jne` skips <, ==, NaN; independent of a):
          `k = 1.0f - comp[i]` (f32 store), `mass = e->m_fMass`,
          `X = f32(((normal.x * 0.25f) * -0.05f) * k)`, `Y = (-0.05f * f32(normal.y * 0.25f)) * k`,
          `Z = (f32(normal.z * -0.05f)) * k` (note: z is NOT scaled by 0.25),
          force = `(f32(X * mass), f32(Y * mass), f32(Z * mass))`, point = `colpoint[i].point - e->GetPosition()`
          (f32 per component), `e->ApplyForce(force, point, true)` (0x6C89BC-0x6C8ABC). f32() marks a store to memory;
          other intermediates stay in x87 registers.
     - Always (whether or not the above ran): `m_apWheelCollisionEntity[i] = nullptr` (plain store, no
       CleanUpOldReference, 0x6C8AC1).
- Oddities: the z component of the push force misses the 0.25 factor (original behaviour, keep). The VehicleDamage
  component argument has undefined upper bits.
- gta-reversed: `plugin::CallMethod` only.
- Open questions: none.
