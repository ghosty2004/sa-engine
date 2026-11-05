# CAutomobile (VEH) - full specs (group C)

Layout used (gr `Automobile.h`; offsets derived from its VALIDATE_OFFSETs, CAutomobile size 0x988): +0x5A0
`m_damageManager` (0x18), +0x5B8 `m_doors[6]` (CDoor 0x18), +0x648 `m_aCarNodes[25]` (RwFrame*), +0x6AC `m_panels[3]`
(CBouncingPanel 0x20), +0x70C `m_swingingChassis` (CDoor), +0x724 `m_wheelColPoint[4]` (CColPoint 0x2C: +0x00 point,
+0x10 normal, +0x20 surfaceA, +0x21 pieceA, +0x23 surfaceB, +0x24 pieceB, +0x25 lightingB, +0x28 depth), +0x7D4
`m_fWheelsSuspensionCompression[4]`, +0x7E4 `...Prev[4]`, +0x7F4 `m_WheelCounts[4]`, +0x828 `m_wheelRotation[4]`,
+0x838 `m_wheelPosition[4]`, +0x848 `m_wheelSpeed[4]` (+0x84C heli rotor speed), +0x868 `autoFlags` (bit 5
bTankExplodesCars, bit 7 bIsMonsterTruck), +0x86C/+0x86E misc component angle (cur/prev), +0x878
`m_aSuspensionSpringLength[4]`, +0x888 `m_aSuspensionLineLength[4]`, +0x898 front / +0x89C rear height above road,
+0x8A8 `m_fAircraftGoToHeading`, +0x8CC `DoorRotation[6]`, +0x8E8 `m_apWheelCollisionEntity[4]`, +0x8F8
`m_vWheelCollisionPos[4]`, +0x928 `m_pExplosionVictim`, +0x960 `m_nNumContactWheels`, +0x978
`m_exhaustNitroFxSystem[2]`, +0x980 `m_harvesterParticleCounter`, +0x984 `m_heliDustFxTimeConst`.
CVehicle: +0x138 `m_vehicleAudio`, +0x384 `m_pHandlingData`, +0x38C handling flags copy, +0x390 `m_autoPilot`,
+0x428 vehicleFlags (8 bytes), +0x458 `m_fWheelScale`, +0x494 `m_fSteerAngle`, +0x498 `m_f2ndSteerAngle`,
+0x4C0 `m_fHealth`, +0x4C4 `m_pTractor`, +0x4C8 `m_pTrailer`, +0x4D0 `m_nTimeTillWeNeedThisCar`, +0x574 collision
lighting[4], +0x580 `m_pDustParticle`, +0x584 `m_renderLights`, +0x590 `m_nVehicleType`, +0x594 `m_nVehicleSubType`
(plugin-sdk VALIDATE_OFFSETs). CPhysical: +0x40 physicalFlags, +0x44 moveSpeed, +0x50 turnSpeed, +0x8C mass, +0x90
turn mass, +0x98 air resistance, +0xA0 buoyancy, +0xA4 centre of mass, +0xB8 fake physics, +0xB9 number of collided
entities, +0xBA contact surface, +0xBC collided entities[6], +0xD4 moving speed. CEntity: +0x14 matrix, +0x1C flags,
+0x22 model, +0x36 type/status.
Shared helpers: `_ftol` 0x821B40 (truncation), `rand` 0x821B1E, 0x40FE90 `out = s * v`, 0x40FE30 `out = a + b`,
0x40FE60 `out = a - b` (all cdecl (CVector* out, ...), x87 per component starting with z).

## 0x44D520 CAutomobile::HasCarStoppedBecauseOfLight  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall bool (CAutomobile* this)`, result in AL only (`xor al,al` / `mov al,1`; upper EAX bits are
  garbage), plain `ret`. 0x44D520 is a `jmp` stub; the body is at .HOODLUM 0x156E9C0-0x156EC1C. Locals 8.
- Callers: CAutomobile::PlayHornIfNecessary 0x6A3820. Callees: none.
- Globals (ThePaths = CPathFind 0x96F050): `m_pPathNodes[]` 0x96F854 (CPathNode*, node stride 0x1C: +0x10
  `m_wBaseLinkId` i16, +0x18 low nibble `m_nNumLinks`), `m_pNaviNodes[]` 0x96F974 (CCarPathLink*, stride 0xE; +0xC
  bits 0-1 `m_nTrafficLightState`), `m_pNodeLinks[]` 0x96FA94 (CNodeAddress* = {u16 area, u16 node}),
  `m_pNaviLinks[]` 0x96FDF4 (u16 navi address: bits 10-15 area, bits 0-9 index).
- Fields: +0x36 entity status (bits 3-7); autopilot (+0x390): +0x390/+0x392 current node (area, node),
  +0x394/+0x396 starting route node, +0x398/+0x39A ending route node (gr `m_currentAddress`,
  `m_startingRouteNode`, `m_endingRouteNode`).
- Behaviour:
  1. `st = byte[+0x36] >> 3`; if st != 2 (SIMPLE) and st != 3 (PHYSICS): return false (0x156E9C3-0x156E9D7).
  2. `cur = dword[+0x390]`. If `m_pPathNodes[cur.area] == 0` or `m_pPathNodes[word[+0x394]] == 0`: return false.
  3. Loop A, i = 0 .. numLinks(cur)-1 (count re-read every iteration, signed `jl`; skipped when the nibble is 0):
     `n = m_pPathNodes[cur.area][cur.node]`; `k = n.m_wBaseLinkId + i`; if `m_pNodeLinks[cur.area][k]` equals
     (`+0x394`, `+0x396`) (two 16-bit compares, area first): `nav = m_pNaviLinks[cur.area][k]`; if
     `m_pPathNodes[nav >> 10] != 0` and `(m_pNaviNodes[nav >> 10][nav & 0x3FF].byte[0xC] & 3) != 0`: return true.
  4. If `word[+0x398] == 0xFFFF`: return false. `end = dword[+0x398]`; if `m_pPathNodes[end.area] == 0` return false.
  5. Loop B, same as A over the links of node `end`, matching link == (`+0x390`, `+0x392`), using
     `m_pNodeLinks[end.area]` / `m_pNaviLinks[end.area]`; same traffic-light test -> return true.
  6. return false.
- Oddities: the area indices in steps 2/3 are not validated: `word[+0x390]` / `word[+0x394]` = 0xFFFF (invalid
  address) index `m_pPathNodes` far past its 72 entries (up to 0x96F854 + 0x3FFFC). Ports must read the same global
  memory (through the globals window) rather than bounds-check, or prove those values cannot occur.
- gta-reversed: `plugin::CallMethod<0x44D520>` only (no C++).

## 0x6A2530 CAutomobile::SetRandomDamage  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall void (CAutomobile* this, bool bRemoveStuff [esp+4])`, `ret 4`. Locals 0x1C; the argument
  slot is reused as an int ("numPanels", later "remaining panels").
- Callers: CCarCtrl::GenerateOneRandomCar 0x430050.
- Callees: rand 0x821B1E, `_ftol` 0x821B40 (truncation), CDamageManager::GetCarNodeIndexFromDoor 0x6C26F0 (cdecl),
  ::GetDoorStatus 0x6C2230, ::SetDoorStatus 0x6C21C0 (thiscall (door, status)), ::GetCarNodeIndexFromPanel 0x6C26A0
  (cdecl), ::GetPanelStatus 0x6C2180, ::SetPanelStatus 0x6C2150 (thiscall (panel, status)),
  CVehicle::SetComponentVisibility 0x6D2700 (thiscall on this (RwFrame*, u32 state)).
- Globals/constants: `ms_modelInfoPtrs` 0xA9B0C8; 0x858B14 = 3.0517578e-05f (1/32768), 0x858B44 = 6.0f,
  0x859024 = -3.0f, 0x858B90 = 4.0f.
- Fields: +0x5A0 m_damageManager, +0x648 `m_aCarNodes[]` (RwFrame*), model info +0x5C `m_pVehicleStruct`
  (+0x310 `m_nMaskComponentsDamagable` u32).
- Notation: `R() = (float)(rand() & 0xFFFF) * 3.0517578e-05f` (fild, fmul f32, kept in x87).
- Behaviour:
  1. Local `doorPicked[6] = {0}` (esp+0x1C), `panelPicked[7]` (esp+0x24) = {0,0,1,1,1,0,0} (panels 2,3,4 =
     rear-left, rear-right, windscreen pre-marked, 0x6A2548-0x6A2567).
  2. Counts (0x6A2569-0x6A2638):
     - bRemoveStuff != 0: `numDoors = _ftol(R() * 6.0f)` (0..5). If numDoors == 0:
       `numPanels = 1 - _ftol(R() * -3.0f)` (1..3); else `numPanels = _ftol(R() * 4.0f)` (0..3).
     - bRemoveStuff == 0: `numDoors = _ftol(R())` (always 0, one rand() consumed). Then numPanels = 1 (no rand()).
       (The `numDoors != 0` sub-branch, `numPanels = _ftol(R())`, is unreachable since rand() <= 32767.)
  3. Doors, i = 0 .. numDoors-1 (signed, skipped when numDoors <= 0) (0x6A2650-0x6A2706):
     - `r = _ftol(R() * (6 - i))` (`fimul` with the int 6-i).
     - if i == 0: `d = r`; else find the r-th (0-based) entry of doorPicked that is 0: scan k = 0..5 counting zeros,
       stop at the first zero that makes the count reach r; if the scan ends, `d = 6` (0x6A2684-0x6A26A1).
     - `node = GetCarNodeIndexFromDoor(d)`. If `GetDoorStatus(d) == 0` and `(mask & (1 << (node & 31))) != 0`:
       `SetDoorStatus(d, 2 /*DAMAGED*/)`; `SetComponentVisibility(m_aCarNodes[node], 2 /*ATOMIC_DAMAGED*/)`.
     - `doorPicked[d] = 1` (always).
  4. Panels: `remaining = 4`, count = numPanels; skipped when numPanels <= 0; loop `do { ... } while (--count != 0)`
     (0x6A2730-0x6A27DF):
     - `r = _ftol(R() * remaining)` (`fimul`); find the r-th zero of panelPicked (k = 0..6; `p = 7` if the scan ends).
     - `node = GetCarNodeIndexFromPanel(p)`. If `GetPanelStatus(p) == 0` and mask bit `node` set:
       `SetPanelStatus(p, 1 /*DAMSTATE_OPENED*/)`; `SetComponentVisibility(m_aCarNodes[node], 2)`.
     - `panelPicked[p] = 1`; `remaining--`.
- Edge cases: the scans cannot run out with these counts (doors <= 5 of 6, panels <= 3 of 4), so d = 6 / p = 7 do not
  occur in practice.
- gta-reversed: C++ exists (Automobile.cpp:3954) and is NOT equivalent (see CAutomobile.deviations.md for the rows):
  (a) gr builds every range with its lerp-based GetRandomNumberInRange and, under FIX_BUGS, widens them by one
  (doors 0..6, panels 1..4 / 0..4; with bRemoveStuff == false doors become 1 when rand() == 32767);
  (b) gr's per-pick roll is `GetRandomNumberInRange(0, numToDmg - i)`; the original uses the number of remaining
  candidates (`6 - i` for doors, `4 - i` for panels);
  (c) gr sets damaged panels to DAMSTATE_DAMAGED (2); the original writes 1 (DAMSTATE_OPENED).

## 0x6A2B10 CAutomobile::Render  (catalogue: specified/REA, port level 7)
- Convention: `__thiscall void (CAutomobile* this)`, plain `ret`. Virtual slot 18 (+0x48). Locals 0x74.
- Callees: RW render state through `RwEngineInstance` 0xC97B24 (`+0x24` fpRenderStateGet, `+0x20` fpRenderStateSet,
  cdecl (state, value)), RwHelper::GetFirstObject 0x7348C0 (cdecl (RwFrame*) -> RwObject*), RpAtomic render callback
  (`call [atomic+0x48]`, cdecl (RpAtomic*)), CVehicle::Render 0x6D0E60, CEntity::GetColModel 0x535300,
  RwMatrixTranslate 0x7F2450 (cdecl (RwMatrix*, const RwV3d*, int combine)), RwFrameUpdateObjects 0x7F0910,
  CVehicle::GetVehicleAppearance 0x6D1080, CVehicle::DoHeadLightBeam 0x6E0E20 (thiscall (int lightId, CMatrix&,
  bool isRight)).
- Globals: `CTimer::m_snTimeInMilliseconds` 0xB7CB84, `CCheat::m_aCheatsActive[27]` 0x96914B (INVISIBLE_CAR).
  Constants 0x858B50 0.0f, 0x858624 1.0f, 0x858C1C -1.0f.
- Fields: +0x4D0 `m_nTimeTillWeNeedThisCar`, +0x648 car nodes (+0x650 RF, +0x654 RM, +0x658 RB, +0x65C LF,
  +0x660 LM, +0x664 LB), door 4/5 fields +0x618/+0x61C/+0x624/+0x628/+0x630/+0x634/+0x63C/+0x640 (RHINO extra wheel
  compressions), +0x878 spring[0], +0x888 line[0], +0x42C bit 7 (vehicleFlags bit 39 `bIsRCVehicle`), +0x584
  `m_renderLights` (bit 0, bit 1); RwFrame +0x10 modelling matrix (pos at frame+0x40).
- Behaviour:
  1. `m_nTimeTillWeNeedThisCar = m_snTimeInMilliseconds + 3000`.
  2. `saved = 1; RwRenderStateGet(30 /*ALPHATESTFUNCTIONREF*/, &saved); RwRenderStateSet(30, 1)`.
  3. If INVISIBLE_CAR is active: for the frames at +0x658, +0x664, +0x650, +0x65C (in that order, no null check)
     and then +0x654 and +0x660 (each only if non-null): `a = GetFirstObject(frame)` (called twice, once for the
     callback pointer and once for the argument) and `a->renderCallBack(a)`. Else `CVehicle::Render(this)`.
  4. If model == 0x1B0 (RHINO) (0x6A2C29-0x6A2E80): `cd = GetColModel()->m_pColData`; copy the 8 door floats to a
     local array `ex[8]`. For k = 0..7 (line index 4+k, `cd->m_pLines + 0x80 + 0x20*k`):
     - `c = ex[k] - (line[0] - spring[0]) / line[0]` (x87: fld line0; fsub spring0; fdiv line0; fsubr ex[k]);
       `if (c < 0) c = 0` (0 > c test; NaN kept); `t = 1 - c`.
     - `P.x = c*end.x + f32(t*start.x)`, `P.y = c*end.y + f32(t*start.y)`, `P.z = f32(c*end.z) + t*start.z`
       (each stored f32; f32() marks intermediate stores).
     - frame = k < 4 ? node at +0x664 (LB) : node at +0x658 (RB). `D = P - frame->modelling.pos` (componentwise f32).
     - `RwMatrixTranslate(&frame->modelling, &D, 2 /*POSTCONCAT*/)`; `RwFrameUpdateObjects(frame)`;
       `a = GetFirstObject(frame); a->renderCallBack(a)`; then `D = D * -1.0f` (in place, per component) and
       `RwMatrixTranslate(&frame->modelling, &D, 2)` (no RwFrameUpdateObjects after the second translate).
  5. `RwRenderStateSet(30, saved)`.
  6. Head-light beams: skip if model is 0x1B9 (RCBANDIT), 0xFFFE or 0x1B0, if bIsRCVehicle, or if
     `GetVehicleAppearance() == 3` (HELI). Else if `m_renderLights & 1`: `DoHeadLightBeam(0, *m_matrix, true)`; if
     `m_renderLights & 2`: `DoHeadLightBeam(0, *m_matrix, false)`.
- Oddities: the RHINO extra wheels are drawn by moving the rear wheel frames there and back, so float error
  accumulates in those frame matrices each frame (keep). The first four wheel frames in the invisible-car path are
  not null-checked.
- gta-reversed: `plugin::CallMethod<0x6A2B10>` only.

## 0x6A2F80 CAutomobile::DebugCode  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (CAutomobile* this)`, plain `ret`.
- Callers: none in the catalogue (virtual / debug). Callees: CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (int32 = -1,
  bool = false)).
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8, `CWorld::PlayerInFocus` 0xB7CD74 (u8), `CWorld::Players`
  (stride 0x190; +0xB0 `m_pRemoteVehicle` at 0xB7CE48), handling array `gHandlingDataMgr.m_aVehicleHandling`
  0xC2B9DC (gHandlingDataMgr + 0x14, stride 0xE0). Constants 0x858C58 = 0.01f, 0x870FCC = 1000.0f, 0x858B8C = 0.5f.
- Behaviour:
  1. `mi = ms_modelInfoPtrs[(i16)m_nModelIndex]`; `v = FindPlayerVehicle(-1, false)`.
  2. `remote = Players[PlayerInFocus].m_pRemoteVehicle` (PlayerInFocus read once, after the call).
     Proceed iff `(v == this && remote == nullptr) || remote == this` (0x6A2F9B-0x6A2FC6); else return.
  3. `h = &m_aVehicleHandling[mi->m_nHandlingId (+0x4A, u8)]`. `m_fMass (+0x8C) = h->m_fMass (+4)`,
     `m_fTurnMass (+0x90) = h->m_fTurnMass (+0xC)`, `m_fBuoyancyConstant (+0xA0) = h->m_fBuoyancyConstant (+0x24)`
     (dword copies).
  4. `if (h->m_fDragMult (+0x10) > 0.01f)` (`test ah,0x41; jne` -> else on <=, NaN):
     `m_fAirResistance (+0x98) = h->m_fDragMult / 1000.0f * 0.5f` (fdiv then fmul, stored f32); else
     `m_fAirResistance = h->m_fDragMult` (dword copy).
- gta-reversed: hooked/reversed C++ exists (Automobile.cpp:4133). Difference: gr computes the air resistance with
  `GetDefaultAirResistance()`, which reads `this->m_pHandlingData`, while the original reads the handling entry selected
  by the model info's handling id (the same entry as for mass). Equal unless `m_pHandlingData` points elsewhere.
  `/1000/2` vs `/1000*0.5` gives the same f32.

## 0x6A3030 CAutomobile::RemoveRefsToVehicle  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (CAutomobile* this, CEntity* ent [esp+4])`, `ret 4`. Virtual slot 40 (+0xA0) of the
  CAutomobile vtable 0x871120.
- Behaviour: for i = 0..3 ascending: `if (m_apWheelCollisionEntity[i] (+0x8E8 + 4i) == ent) m_apWheelCollisionEntity[i]
  = nullptr` (plain store, no CleanUpOldReference) (0x6A3034-0x6A304E).
- gta-reversed: no hook found for this slot (catalogue name from an address comment); behaviour is fully given above.

## 0x6A3060 CAutomobile::SetUpWheelColModel  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall bool (CAutomobile* this, CColModel* wheelCol [esp+4])`, AL result, `ret 4`. SEH frame
  (handler 0x847EE8) only to destroy the local CMatrix. Virtual slot 43 (+0xAC).
- Callees: CEntity::GetColModel 0x535300, CMatrix::Attach 0x59BD10 (thiscall (RwMatrix*, bool owns)),
  CColSphere::Set 0x40FD10 (thiscall (f32 radius -> +0xC, const CVector& centre -> +0x0, u8 material -> +0x10,
  u8 piece -> +0x11, u8 lighting -> +0x12), ret 0x14), CMatrix::~CMatrix 0x59ACD0.
- Fields: +0x22 model, +0x648 `m_aCarNodes[]` (index 2 WHEEL_RF, 3 WHEEL_RM, 4 WHEEL_RB, 5 WHEEL_LF, 6 WHEEL_LM,
  7 WHEEL_LB; RwFrame modelling matrix at frame+0x10); model info +0x40 `m_fWheelSizeFront`, +0x44
  `m_fWheelSizeRear`; CColModel +0x00 bbox (0x18 bytes), +0x18 bounding sphere (0x10 bytes), +0x2C `m_pColData`
  (CCollisionData +0x00 `m_nNumSpheres` u16, +0x08 `m_pSpheres`, stride 0x14). Constant 0x858B8C = 0.5f.
- Behaviour:
  1. `mi = ms_modelInfoPtrs[model]`, `cm = GetColModel()`, `cd = wheelCol->m_pColData` (read before the model test).
  2. If model is 0x213 (531, TRACTOR), 0x214 (532, COMBINE) or 0x23B (571, KART): return false (nothing written).
  3. Copy `cm` bounding sphere (+0x18, 4 dwords) to `wheelCol` +0x18, then the bbox (6 dwords, `rep movsd`).
  4. Local CMatrix `mat` (attach = null, owns = false). For each (node, sphere k, size, piece):
     (5 LF, 0, front, 0x0D), (7 LB, 1, rear, 0x0F), (2 RF, 2, front, 0x0E), (4 RB, 3, rear, 0x10):
     `mat.Attach(&m_aCarNodes[node]->modelling, false)`; `cd->m_pSpheres[k].Set(size * 0.5f, mat.pos, 0x43
     /*SURFACE_RUBBER*/, piece, 0xFF)` (radius = `fld size; fmul 0.5f` stored f32). `cd` pointer (`ebp`) re-reads
     `m_pSpheres` before each call.
  5. If `m_aCarNodes[6]` (LM) and `m_aCarNodes[3]` (RM) are both non-null (0x6A31DC-0x6A31EC): spheres 4 (LM, rear,
     0x0F) and 5 (RM, rear, 0x10) the same way, then `cd->m_nNumSpheres = 6`; else `cd->m_nNumSpheres = 4`.
  6. Destroy `mat`, return true.
- gta-reversed: C++ (Automobile.cpp:2447, hooked) matches the bytes (it copies the box before the sphere; no
  observable difference).

## 0x6A32B0 CAutomobile::BurstTyre  (catalogue: specified/REA, port level 10)
- Convention: `__thiscall bool (CAutomobile* this, u8 tyreComponentId [esp+4], bool bPhysicalEffect [esp+8])`, AL,
  `ret 8`. Virtual slot 44 (+0xB0).
- Callers: CVehicle::InflictDamage 0x6D7C90 (virtual, 3 sites).
- Callees: CDamageManager::GetWheelStatus 0x6C21B0 / SetWheelStatus 0x6C21A0, CStats::IncrementStat 0x55C180
  (cdecl (u16 stat, f32)), CAEVehicleAudioEntity::AddAudioEvent 0x4F6420 (thiscall on +0x138 (int event, f32)),
  CCarCtrl::SwitchVehicleToRealPhysics 0x423FC0 (cdecl), CGeneral::GetRandomNumberInRange<float> 0x41BD90
  (cdecl (min, max) -> ST0 = `(max - min) * ((float)rand() * 3.0518509e-05f [0x858C7C]) + min`, rand() not masked),
  0x40FE90 `CVector operator*(float s, const CVector& v)` (cdecl (CVector* out, f32 s, const CVector* v) -> EAX = out;
  computes `s*v.z, s*v.y, s*v.x`), CPhysical::ApplyMoveForce 0x5429F0 (thiscall (CVector by value)),
  CPhysical::ApplyTurnForce 0x542A50 (thiscall (CVector force, CVector point) by value).
- Globals: f32 0x8D3168 = 0.03f (data, read twice; gr treats it as constexpr 0.03f).
- Behaviour:
  1. Return false if model == 0x1B0 (432, RHINO), if bit 7 of byte +0x42B (vehicleFlags `bTyresDontBurst`) is set,
     or if physicalFlags (+0x40) bit 0x20000000 (`bRenderScorched`) is set.
  2. Wheel: piece 13 -> 0, 14 -> 2, 15 -> 1, 16 -> 3 (table 0x6A3430); any other value is used unchanged as the wheel
     index (`movzx edi, al`).
  3. If `GetWheelStatus(wheel) != 0`: return false. Else `SetWheelStatus(wheel, 1 /*BURST*/)`,
     `CStats::IncrementStat(0x81, 1.0f)`, `m_vehicleAudio.AddAudioEvent(0x5A, 0.0f)`; if `(byte[0x36] & 0xF8) == 0x10`
     (STATUS_SIMPLE): `SwitchVehicleToRealPhysics(this)`.
  4. If bPhysicalEffect:
     - `s1 = f32(GetRandomNumberInRange(-g, g) * m_fMass (+0x8C))` with `g = [0x8D3168]` (min = `-g` via fchs);
       `ApplyMoveForce(s1 * m_matrix->right)`.
     - `s2 = f32(GetRandomNumberInRange(-g, g) * m_fTurnMass (+0x90))`; `ApplyTurnForce(s2 * m_matrix->right,
       m_matrix->forward)`. (The matrix pointer is read before each random call; no null check.)
     - `m_nFakePhysics (+0xB8) = 0`.
  5. Return true.
- gta-reversed: C++ (Automobile.cpp:2486). Differences: the float RNG helper (gr `lerp` = `max*t + min*(1-t)`,
  original `(max-min)*t + min`), and gr multiplies `right * rnd` first and then `* mass` (vector * float twice, each
  component rounded in between) whereas the original forms `rnd * mass` first and multiplies the vector once.
  Both change the f32 results. Port the original order.

## 0x6A3BD0 CAutomobile::DoNitroEffect  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall void (CAutomobile* this, f32 power [esp+4])`, `ret 4`. Locals 0x2C.
- Callers: CAutomobile::NitrousControl 0x6A3EA0 (3 sites).
- Callees: `CMatrix operator*(const CMatrix&, const CVector&)` 0x59C890 (cdecl (CVector* out, CMatrix*, CVector*)),
  CWaterLevel::GetWaterLevel 0x6EB690 (cdecl (f32 x, f32 y, f32 z, f32* level, bool, CVector* = null) -> AL),
  FxManager_c::CreateFxSystem 0x4A9BE0 (thiscall on g_fxMan 0xA9AE80: (const char* name, CVector* pos, RwMatrix*,
  bool)), FxSystem_c::SetLocalParticles 0x4AA910, ::Play 0x4AA2F0, ::Stop 0x4AA390, ::SetConstTime 0x4AA6C0
  (thiscall (int, f32)), ::GetPlayStatus 0x4AA900 (0 = PLAYING, 1 = STOPPED).
- Globals/constants: `ms_modelInfoPtrs`; string "nitro" 0x871114; 0x858C1C = -1.0f.
- Fields: +0x40 physicalFlags (bit 0x08000000 `bTouchingWater`), +0x14 m_matrix, +0x18 m_pRwObject (frame at
  object+4, modelling matrix at frame+0x10), +0x384 handling (+0xCC model flags bit 0x2000 `DOUBLE_EXHAUST`),
  +0x978 / +0x97C `m_exhaustNitroFxSystem[0/1]`; model info +0x5C vehicle struct, exhaust dummy at struct+0x48.
- Behaviour:
  1. `e1 = vehStruct->dummy[+0x48]` (exhaust position, local CVector). `sub1 = false`, `sub2 = false`.
  2. If bTouchingWater: `p = m_matrix * e1`; if `GetWaterLevel(p.x, p.y, p.z, &level, true, nullptr)` and NOT
     `level < p.z` (`fcomp; test ah,1; jne`: so level >= p.z, NaN -> false): `sub1 = true`.
  3. If DOUBLE_EXHAUST: `e2 = (f32(e1.x * -1.0f), e1.y, e1.z)`; and if bTouchingWater (`test [+0x40], edi; je` at
     0x6A3C82-0x6A3C91 skips when the flag is clear): same water test on `m_matrix * e2` -> `sub2`.
  4. `rwm = m_pRwObject ? &RwFrame(parent)->modelling : nullptr`.
  5. First system (+0x978):
     - null: if `!sub1 && rwm`: `fx = g_fxMan.CreateFxSystem("nitro", &e1, rwm, true)`, store; if non-null:
       `fx->SetLocalParticles(true); fx->Play()`. (Then continue with step 6.)
     - non-null: `SetConstTime(1, fabs(power))`; if `GetPlayStatus() == 0 && sub1`: `Stop()`; else if
       `GetPlayStatus() == 1 && !sub1`: `Play()` (status re-read for the second test).
  6. If DOUBLE_EXHAUST (handling re-read): second system (+0x97C):
     - null: if `!sub1 && rwm` (note: tests `sub1`, not `sub2`): create with `&e2`, store; if non-null
       `SetLocalParticles(true); Play()`.
     - non-null: `SetConstTime(1, fabs(power))`; `GetPlayStatus() == 0 && sub2` -> `Stop()`; else
       `GetPlayStatus() == 1 && !sub2` -> `Play()`.
- gta-reversed: C++ (Automobile.cpp:4265, hooked). Logic error vs the bytes: gr runs the second-exhaust water test
  when `!physicalFlags.bTouchingWater`; the original runs it only when bTouchingWater is set (0x6A3C82). Everything
  else (including the use of `sub1` when creating the second system) matches.

## 0x6AA290 CAutomobile::UpdateWheelMatrix  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (CAutomobile* this, int32 nodeIndex [esp+4], int32 flags [esp+8])`, `ret 8`.
  0x6AA290 starts with `nop; jmp 0x401A84` (an inlined `mov eax, fs:[0]` trampoline) and continues at 0x6AA296;
  SEH frame (handler 0x8480AB) for the local CMatrix. Locals 0xD0.
- Callers: CAutomobile::PreRender 0x6AAB50 (6), CHeli::PreRender 0x6C5420 (4), CPlane::PreRender 0x6C94A0 (4).
- Callees: CMatrix::Attach 0x59BD10, CMatrix::SetScale(fff) 0x59AF00, CMatrix::Rotate(x,y,z) 0x59B460,
  CMatrix::RotateY 0x59B2C0, CMatrix::UpdateRW 0x59BBB0, ~CMatrix 0x59ACD0, CDamageManager::GetWheelStatus 0x6C21B0,
  CVehicleModelInfo::GetWheelSize 0x6A06F0 (thiscall (bool front) -> ST0), SurfaceInfos_c::IsWater 0x55E710 /
  IsSand 0x55E6F0 (thiscall on g_surfaceInfos 0xB79538), FxPrtMult_c::FxPrtMult_c 0x4AB290 (thiscall (r, g, b, a,
  size, unused, life)), FxSystem_c::AddParticle 0x4AA440 (thiscall (CVector* pos, CVector* vel, f32 time,
  FxPrtMult_c*, f32 zRot, f32 lightMult, f32 lightMultLimit, u8 local)), 0x40FE90 (`out = s * v`), 0x40FE30
  (`out = a + b`, cdecl (out, a*, b*)), CRT `_CIasin` 0x821E70 (ST0 -> ST0).
- Globals: `ms_modelInfoPtrs` 0xA9B0C8; `CCheat::m_aCheatsActive[34]` 0x969152 (CARS_ON_WATER); data f32 0x8D31D4 =
  1.7f and 0x8D31D0 = 0.45f (combine wheels); `CTimer::m_FrameCounter` 0xB7CB4C; FxSystem pointers g_fx+0x18
  (0xA9AE18), +0x20 (0xA9AE20), +0x38 (0xA9AE38).
- Constants: 0x858B50 0.0f, 0x858624 1.0f, 0x870FD0 1.0f, 0x858CB8 3.1415927f, 0x871048 0.6f, 0x858CB0 0.7f,
  0x858B8C 0.5f, 0x858F50 0.65f, 0x858C98 0.8f, 0x858F14 1.1f, 0x8595F0 1.25f, 0x87104C 3.0f, 0x858C24 0.3f,
  0x859998 -1.5707964f, 0x858CA0 2.0f, 0x871044 -1.0f, 0x858C1C -1.0f.
- Fields: +0x22 model, +0x38C handling flags copy (u32), +0x384 handling (+0xCC model flags), +0x42B bit 0x40
  (vehicleFlags bit 30 `bIsDrowning`), +0x458 `m_fWheelScale`, +0x494 `m_fSteerAngle`, +0x498 `m_f2ndSteerAngle`,
  +0x594 `m_nVehicleSubType`, +0x648 `m_aCarNodes[]`, +0x724 wheel colpoints (stride 0x2C, +0x23 surfaceTypeB),
  +0x7E4 `m_fWheelsSuspensionCompressionPrev[]`, +0x828 `m_wheelRotation[]`, +0x838 `m_wheelPosition[]`,
  +0x868 autoFlags (bit 7 `bIsMonsterTruck`), +0x898 `m_fFrontHeightAboveRoad`; model info +0x40/+0x44 wheel sizes,
  +0x48 `m_nWheelModelIndex` (i16).
- Locals: `steer` (esp+0x10), `isFront` (byte esp+0x14), `isRear` (BL), `side` (esp+0x18, -1 left / +1 right),
  `scale` (esp+0x1C), `pos` (esp+0x24, CVector), `mat` (CMatrix at esp+0x48), `w` (x87 width factor).
- Behaviour:
  1. `frame = m_aCarNodes[nodeIndex]`; null -> return. `mat` attach = null / owns = false.
  2. Wheel selection by nodeIndex (table 0x6AAB34; nodeIndex outside 2..7 -> destroy mat and return without any
     change):
     - 5 (LF): wheel 0, side -1, isFront. steer = (hflags & 0x20) ? -> PI : m_fSteerAngle.
     - 2 (RF): wheel 2, side +1, isFront. steer = (hflags & 0x20) ? -> 0 : m_fSteerAngle.
     - 6, 7 (LM, LB): wheel 1, side -1, isRear. steer = (hflags & 0x20) ? -m_fSteerAngle : (hflags & 0x40) ?
       m_f2ndSteerAngle : -> PI.
     - 3, 4 (RM, RB): wheel 3, side +1, isRear. steer = (hflags & 0x20) ? -m_fSteerAngle : (hflags & 0x40) ?
       m_f2ndSteerAngle : -> 0.
     ("-> PI / -> 0" = the value is set directly and step 3 is skipped.)
  3. For a steer value taken from a field (0x6AA3CB): if `steer == 0` (exact; NaN is not equal) or `(flags & 2)`:
     `steer = (side < 0) ? PI : 0`. Otherwise reduce the inner wheel and add PI on the left:
     left (side < 0): rear -> `if (steer > 0) steer = 0.6f*steer`; front -> `if (steer < 0) steer = 0.6f*steer`;
     then `steer = steer + PI` (f32). Right (side >= 0 or NaN): rear -> `if (steer < 0) steer *= 0.6f`; front ->
     `if (steer > 0) steer *= 0.6f` (all as `fld 0.6; fmul steer`, stored f32).
  4. `mat.Attach(&frame->modelling, false)`; `pos = mat.pos`; `pos.z = m_wheelPosition[wheel]`;
     `scale = m_fWheelScale`, `w = 1.0f`.
     - If `mi->m_nWheelModelIndex == -1`: if isRear `scale = (mi->rear / mi->front) * scale` (f32);
       `w = mi->front / (m_fWheelScale * 0.7f)` (x87, not stored).
     - Else: `scale = isRear ? mi->rear : mi->front` (m_fWheelScale is discarded), w stays 1.0.
  5. Model 0x214 (532 COMBINE) and nodeIndex 6 or 3: `scale = 1.7f[0x8D31D4] / mi->front`;
     `pos.z = (1.7f - mi->rear) * 0.45f[0x8D31D0] + pos.z`; `steer = ((side < 0) ? steer - PI : steer) * 0.5f`
     then `if (side < 0) steer += PI`. Else model 0x201 (513 STUNT) and nodeIndex 5: `steer = 0`.
  6. Width: if subtype is 1 (MTRUCK) or 2 (QUAD): `w = 1.0f`. Else if isRear and `hflags & 0xF000`: first match of
     0x1000 -> w*0.65f, 0x2000 -> w*0.8f, 0x4000 -> w*1.1f, 0x8000 -> w*1.25f. Else if isFront and `hflags & 0x0F00`:
     0x100 -> 0.65f, 0x200 -> 0.8f, 0x400 -> 1.1f, 0x800 -> 1.25f. Then model 0x23B (571 KART): `w *= 3.0f`.
  7. `mat.SetScale(f32(w * scale), scale, scale)`.
  8. Rotation: subtype 3 (HELI): `mat.Rotate(0, 0, steer)`. Otherwise `r = side * m_wheelRotation[wheel]`; if
     `GetWheelStatus(wheel) == 1` (burst): `r` stored f32, `mat.Rotate(r, 0, f32(fsin(r) * 0.3f + steer))` (fsin on the
     unrounded x87 r); else `mat.Rotate(f32(r), 0, steer)`.
  9. Tilt selection (0x6AA6D6):
     - Solid-axle tilt (step 11) if autoFlags bit 7 (monster truck), or hflags & 0x20000, or (isFront and
       model flags & 0x20000), or (isRear and model flags & 0x200000).
     - Else hover (step 10) if (CARS_ON_WATER cheat active or model == 0x21B VORTEX) and
       `m_fWheelsSuspensionCompressionPrev[wheel] < 1.0f` and !bIsDrowning.
     - Else camber (step 12).
  10. Hover (0x6AA75E): if `!(flags & 4)`: `mat.RotateY(side * -1.5707964f)`. If `flags & 8`: go to step 13.
      Else `prt = FxPrtMult_c(1, 1, 1, 0.2f, 0.5f, 0, 0.05f)`; `p = colpoint[wheel].point`;
      `vel = (0.5f * moveSpeed) + (side * m_matrix->right)` (two 0x40FE90 products, then 0x40FE30 sum: x87 per
      component, z first). If `IsWater(surfaceB)`: if `((m_FrameCounter - 1) & 1) != 0` (`dec eax; test al,1`):
      `vel.z += 2.0f` and emit through `g_fx+0x38`, else emit through `g_fx+0x20` (unchanged vel); emit =
      `AddParticle(&p, &vel, 0.0f, &prt, -1.0f, 1.2f, 0.6f, 0)`. Else if `IsSand(surfaceB)`: second
      `FxPrtMult_c(0.81f, 0.67f, 0.57f, 0.35f, 0.5f, 0, 0.05f)` and `AddParticle` through `g_fx+0x18` with the same
      other arguments. Then step 13.
  11. Solid-axle tilt (0x6AAA8F): `other = (wheel > 1) ? wheel - 2 : wheel + 2`;
      `a = fabs(pos.y) + fabs(pos.y)` (`fadd st0,st0`; note it is the y coordinate of the wheel frame);
      `b = ((m_wheelPosition[wheel] - m_wheelPosition[other]) * side) * -1.0f`; `mat.RotateY(f32(fpatan(b / a)))`
      (atan2(b, a)). Then step 13.
  12. Camber (0x6AA904): skip to step 13 if `flags & 1`, or isFront and model flags & 0x10000, or isRear and model
      flags & 0x100000. Else `t = pos.z + m_fFrontHeightAboveRoad` (f32, stored in the `scale` slot);
      `h = t - mi->GetWheelSize(isFront) * 0.5f` (x87); `h = (h * -1.0f) * side`; clamp: `if (h < -1) x = -1; else if
      (h > 1) x = 1; else x = h` (x stored f32 and reloaded; NaN passes as NaN); `ang = _CIasin(x)`; if (isFront and
      model flags & 0x80000) or (isRear and model flags & 0x800000) `ang = -ang`; `mat.RotateY(f32(ang))`.
  13. `mat.pos += pos` (componentwise, x then y then z, f32); `mat.UpdateRW()`; destroy `mat`.
- Oddities: step 2 sends middle wheels (3, 6) through the rear wheel indices 3 and 1. Step 11 uses `pos.y`, not `pos.x`.
  In the hover path `esi` (this) is clobbered after the particle setup; nothing after it uses `this`.
- gta-reversed: raw call to 0x6AA290 only (TODO: Reverse); no C++ to compare.
- Open questions: none (handling flag names: 0x20 STEER_REARWHEELS, 0x40 HB_REARWHEEL_STEER, 0x100-0x800 front width,
  0x1000-0x8000 rear width, 0x20000 HYDRAULIC_INST; model flags 0x10000 AXLE_F_NOTILT, 0x20000 AXLE_F_SOLID, 0x80000
  AXLE_F_REVERSE, 0x100000 AXLE_R_NOTILT, 0x200000 AXLE_R_SOLID, 0x800000 AXLE_R_REVERSE per gr tHandlingData bit
  order).

## 0x6AAB50 CAutomobile::PreRender  (catalogue: specified/REA, port level 17)
- Convention: `__thiscall void (CAutomobile* this)` (esi = this throughout), plain `ret` (0x6ACCBA). Virtual slot 17
  (+0x44). Entry is obfuscated: `push -1; nop; jmp 0x408C8C` (`mov eax, fs:[0]`; `jmp 0x6AAB58`), SEH handler
  0x8480D6, `sub esp, 0x178`. Two more jmp chunks: 0x6AB5C7 -> 0x401740 (time-pointer decode, below) -> 0x6AB5CC, and
  0x6ACBC8 -> 0x40649C (`cmp word [esi+0x22], 0x1EF`) -> 0x6ACBCD. Extent 0x6AAB50-0x6ACCBB (8592 bytes, 2286 insns).
  Offsets `[orig X]` below are esp-relative at zero push depth (i.e. after the prologue), so they can be checked
  against the disassembly after subtracting pushes.
- Locals: `mat1` CMatrix at [orig 0x110] (block D only; constructed inline at 0x6AAE4A: attach ptr [0x150] = 0, owns
  [0x154] = 0; SEH state [0x190] = 0; destroyed by ~CMatrix 0x59ACD0 at 0x6AB069 with state -1). `mat2` CMatrix at
  [orig 0x94] (constructed inline at 0x6ABCFD: [0xD4] = [0xD8] = 0; state 1 at 0x6ABD1D; destroyed at 0x6ACC9D,
  state -1). Both are only ever `Attach(..., false)`ed, so the destructors free nothing. `cd` = col data pointer at
  [orig 0x70] (written 0x6AAB9A, re-read at 0x6AC364 / 0x6ACB2A; not overwritten in between - checked every write
  to `[esp+0x70]`). `mi` = `CModelInfo::ms_modelInfoPtrs[model]` at [orig 0x38] (0x6ABD28).
- Callers: CMonsterTruck::PreRender 0x6C7DE0, CQuadBike::PreRender 0x6CEAD0, CTrailer::PreRender 0x6CFAC0, virtual.
- Callees: CVehicle::PreRender 0x6D6480; virtual +0x114 DoHoverSuspensionRatios (0x6A45C0); virtual +0xD0
  DoBurstAndSoftGroundRatios (0x6A47F0); virtual +0xEC GetTowHitchPos / +0xF0 GetTowBarPos (gr `bool (CVector& out,
  bool bCheckModelInfo, CVehicle* veh)`); CEntity::GetColModel 0x535300 (3); CMatrix::Attach 0x59BD10 (10);
  `operator*(CMatrix, CVector)` 0x59C890 (5, cdecl (CVector* out, CMatrix*, CVector*)); Multiply3x3 0x59C790 (3, same
  shape); CMatrix::UpdateRW 0x59BBB0 (5); SetRotate(x,y,z) 0x59B120; RotateX 0x59B1E0; SetTranslateOnly 0x59AF80;
  ~CMatrix 0x59ACD0 (2); CDamageManager::GetWheelStatus 0x6C21B0 (3, thiscall on this+0x5A0);
  CVehicle::AddSingleWheelParticles 0x6DE880 (3); CCullZones::CamNoRain 0x72DDB0 / PlayerNoRain 0x72DDC0;
  CVehicle::AddWaterSplashParticles 0x6DDF60; AddExhaustParticles 0x6DE240; AddDamagedVehicleParticles 0x6D2A80;
  CPointLights::AddLight 0x7000E0 (2, cdecl, caller pops 0x38); CCoronas::RegisterCorona 0x6FC580 (3, cdecl, 21
  args, caller pops 0x54; this is the **eCoronaType** overload - gr labels it "texture" in its install list but its
  signature there is `(..., eCoronaType, eCoronaFlareType, eCoronaReflType, eCoronaLOSCheck, eCoronaTrail, float
  normalAngle, bool neonFade, float pullTowardsCam, bool fullBrightAtStart, float fadeSpeed, bool onlyFromBelow, bool
  whiteCore)`); CPlaceable::GetTopDirection 0x41CCB0; CVehicle::GetVehicleAppearance 0x6D1080;
  CVehicle::DoVehicleLights 0x6E1A60; CWorld::FindPlayerVehicle 0x56E0D0; CCamera::GetLookingForwardFirstPerson
  0x50AED0; CShadows::StoreShadowForVehicle 0x70BDA0; CPhysical::GetSpeed 0x542CE0 (thiscall, hidden out ptr + CVector
  by value); CVehicle::ProcessWheelRotation 0x6D1230 (thiscall, returns ST0); CAutomobile::UpdateWheelMatrix 0x6AA290
  (6); ProcessSwingingDoor 0x6A9D70 (6); CBouncingPanel::ProcessPanel 0x6F49A0; CDoor::Process 0x6F4040;
  CVehicle::SetComponentRotation 0x6DBA30 (14; `(RwFrame*, eRotationAxis 0=X 1=Y 2=Z, float angle, bool resetPos)`);
  SetTransmissionRotation 0x6DBBB0 (3; `(RwFrame*, float angleL, float angleR, CVector posn by value, bool isFront)`);
  CVehicleModelInfo::GetWheelPosn 0x4C7D20 (4; thiscall `(int wheel, CVector& out, bool local)`);
  CClumpModelInfo::GetFrameFromId 0x4C53C0 (cdecl); `_ftol` 0x821B40 (11); `_CIasin` 0x821E70 (3); `_CIpow` 0x822130
  (st1 ^ st0).
- Globals: `CCheat::m_aCheatsActive[34]` 0x969152 (u8); f32 0xC1BFF0 (set by `__init_g_c1bff0` 0x853520 to
  `0.277778f / 50.0f`); `CWeather::Rain` 0xC81324; `CGame::currArea` 0xB72914; `CTimer::m_snTimeInMilliseconds`
  0xB7CB84 (read directly at 0x6AB8E4 / 0x6ABBC1, and through the jmp chunk 0x401740: `edi = *(ror(dword[0x1560674],
  byte[0x587B84] & 31))`; 0x6F970801 ror 9 = 0xB7CB84; preserves flags and ecx, clobbers edi only);
  `CTimer::ms_fTimeStep` 0xB7CB5C; `CTimeCycle::m_CurrentColours.m_fSpriteBrightness` 0xB7C4E4 (CColourSet +0x44);
  `TheCamera` 0xB6F028, `TheCamera.m_fLODDistMultiplier` 0xB6F118 (+0xF0); `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8
  and its element [530] (FORKLIFT) at 0xA9B910. .data (mutable, read as globals, not constants): 0x8D313C u16 = 20000
  (gr constexpr TOWTRUCK_HOIST_DOWN_LIMIT), 0x8D3144 f32 0.4, 0x8D3148 f32 1.0, 0x8D314C i16 = 2500 (gr constexpr
  DEFAULT_COLLISION_EXTENDLIMIT; read with movsx), 0x8D31D8 u32[4] = {0x14, 0x16, 0x15, 0x17}, 0x8D31E8 f32 0.95,
  0x8D31EC f32 0.5235988, 0x8D31F0 u32[3] = {0x15, 0, 0x16}.
- .rdata constants used: 0x858624 1.0f, 0x858B50 0.0f, 0x858F34 0.75f, 0x858CE8 1.5f, 0x858BA4 20.0f, 0x858B38 0.02f,
  0x871250 130.0f, 0x858C54 4294967296.0f, 0x858C58 0.01f, 0x858B4C 0.005f, 0x858B1C 0.1f, 0x858B3C 3.0f, 0x859040
  0.33333334f, 0x858A28 150.0f, 0x85862C 10.0f, 0x858B8C 0.5f, 0x858C24 0.3f, 0x87100C -0.0001f, 0x871010 0.0002f,
  0x871014 0.0006f, 0x871050 -0.17453294f (passed as raw bits), 0x871054 0.02f, 0x858C1C -1.0f, 0x858CBC 6.2831855f,
  0x858CB8 3.1415927f.
- Fields of this (gr names, layout header of this file): CEntity +0x14 m_matrix (right +0x00, forward +0x10, up +0x20,
  pos +0x30), +0x04 placement pos (used when m_matrix is null), +0x22 model (i16), +0x36 status (bits 3-7);
  CPhysical +0x44 moveSpeed, +0x50 turnSpeed, +0x5C frictionMoveSpeed, +0x68 frictionTurnSpeed; CVehicle +0x384
  m_pHandlingData (+0xB8 m_fSuspensionUpperLimit, +0xCC model flags), +0x38C handling flags, vehicleFlags bytes
  +0x428 (0x10 bEngineOn), +0x42B (0x01 bVehicleColProcessed, 0x40 bIsDrowning), +0x42C (0x80 bIsRCVehicle), +0x42D
  (0x80 bSirenOrAlarm), +0x42E (0x80 bDisableParticles); +0x486 m_nGettingInFlags, +0x487 (next byte, gr
  m_nGettingOutFlags), +0x494 m_fSteerAngle, +0x498 m_f2ndSteerAngle, +0x49C m_fGasPedal, +0x4C8 m_pTrailer, +0x594
  m_nVehicleSubType; CAutomobile +0x5A0 m_damageManager, +0x648 m_aCarNodes[25] (node n at 0x648 + 4n: 1 CHASSIS
  0x64C, 2 WHEEL_RF 0x650, 3 RM 0x654, 4 RB 0x658, 5 LF 0x65C, 6 LM 0x660, 7 LB 0x664, 16 BONNET 0x688, 20 MISC_A
  0x698, 21 MISC_B 0x69C, 23 MISC_D 0x6A4, 24 MISC_E 0x6A8), +0x6AC m_panels[3] (stride 0x20: +0 i16/u16 m_nFrameId,
  +8 m_vecRotation), +0x70C m_swingingChassis (CDoor: +0xA m_axis u8 (0x716), +0xB m_doorState u8 (0x717), +0xC
  m_angle (0x718)), +0x724 m_wheelColPoint[4] (stride 0x2C), +0x7D4 compression[4], +0x7E4 compressionPrev[4], +0x7F4
  m_WheelCounts[4], +0x810 skidmarkType[4], +0x820 bloodState[4] (u8), +0x824 muddy[4] (u8), +0x828 wheelRotation[4],
  +0x838 wheelPosition[4], +0x848 wheelSpeed[4], +0x868 autoFlags (bit 0 bTaxiLight), +0x86C m_wMiscComponentAngle
  (u16), +0x878 springLength[4], +0x888 lineLength[4], +0x8B4 m_moveForce, +0x8C0 m_turnForce, +0x94C
  m_fDoomVerticalRotation, +0x950 m_fDoomHorizontalRotation, +0x958 m_fPropRotate, +0x968 m_WheelStates[4].
  CVehicleModelInfo +0x1C m_pRwObject (clump), +0x40 m_fWheelSizeFront, +0x44 m_fWheelSizeRear. Wheel index i: 0 FL,
  1 RL, 2 FR, 3 RR.
- Model ids: 0x197 FIRETRUK, 0x1A0 AMBULAN, 0x1A4 TAXI, 0x1A8 BFINJECT, 0x1AB ENFORCER, 0x1B0 RHINO, 0x1B2 HOTKNIFE,
  0x1B6 CABBIE, 0x1B9 RCBANDIT, 0x1BB PACKER, 0x1D7 QUAD, 0x1DD ZR350, 0x1E6 DOZER, 0x1EA FBIRANCH, 0x1EF SANDKING,
  0x20C CEMENT, 0x20D TOWTRUCK, 0x212 FORKLIFT, 0x213 TRACTOR, 0x214 COMBINE, 0x21B VORTEX, 0x220 FIRELA, 0x238
  BANDITO, 0x254-0x256 COPCARLA/SF/VG, 0x257 COPCARRU, 0x259 SWATVAN; 0xFFFE is compared as a model value.

### Behaviour (blocks in address order)

**A. 0x6AAB71-0x6AAB83** `CVehicle::PreRender()`; if byte 0x969152 != 0: `this->vtbl[+0x114]()`.

**B. 0x6AAB89-0x6AADC9 suspension visual position.** `cd = GetColModel()->m_pColData (+0x2C)`, saved [orig 0x70]. If
(byte +0x42B & 1) and dword +0x594 != 1: `this->vtbl[+0xD0]()`, then for i = 0..3 (unrolled; lines = `cd->+0x10`,
line i start.z at `lines + 0x20*i + 8`; the lines pointer is re-read each i):
`a = spring[i] / line[i]` (fld +0x878+4i, fdiv +0x888+4i); `r = 1.0f - a` (fsubr [0x858624]);
`c = (comp[i] - r) / (1.0f - r)` (comp = +0x7D4+4i; fdivp = st1/st0), stored f32 [orig 0x3C]; r popped;
`p = start.z`; if `c > 0.0f` (fcomp 0x858B50, `test ah,0x41; jne` skips on <=, NaN): `p = p - c * spring[i]`
(c reloaded f32). Then `fld comp[i]; fcomp 1.0f; test ah,5; jp` -> blend when comp >= 1.0 or NaN; otherwise
`fcom wheelPos[i]` (+0x838+4i) and if `p > wheelPos[i]` (`test ah,0x41; je`) store p directly; else blend:
`wheelPos[i] = (p - wheelPos[i]) * 0.75f + wheelPos[i]` (fsub, fmul [0x858F34], fadd; fstp f32).

**C. 0x6AADCF-0x6AAE1B** `fwd = ((ms.z*f.z + ms.y*f.y) + ms.x*f.x) / [0xC1BFF0]` (ms = +0x44, f = matrix forward) ->
f32 [orig 0x50]; `spd = sqrt((ms.x*ms.x + ms.y*ms.y) + ms.z*ms.z)` -> f32 [orig 0x40].

**D. 0x6AAE1F-0x6AB069.** If model == 0xFFFE, 0x21B or 0x1B9: go to F (0x6AB154). If `(byte +0x36 & 0xF8) == 0x10`
(SIMPLE): construct mat1; for (node, wheel, colpoint) in (+0x658, 3, +0x7A8), (+0x664, 1, +0x750), (+0x650, 2, +0x77C),
(+0x65C, 0, +0x724): `mat1.Attach(&frame->modelling (frame+0x10), false)`; `v = (mat1.pos.x, mat1.pos.y,
f32(wheelPos[wheel] * 1.5f))` ([0x858CE8]); `cp[wheel].point = *m_matrix * v` (0x59C890); `cp[wheel].surfaceTypeB
(+0x23) = 0`. Destroy mat1. (No null check on the node pointers.)

**E. 0x6AB06E-0x6AB14E** if status (byte +0x36 >> 3) is 3, 0, 1 or 2 (checked in that order): `spin = (ws[1] (+0x96C)
== 2 || ws[3] (+0x974) == 2)` (byte [orig 0x13]). For i = 0..3: `flags = ((i == 0 || i == 2) && !spin) ? 4 : 0`;
`+1` if bloodState[i]; `+2` if muddy[i]; `side = (i == 0 || i == 1) ? -1.0f : 1.0f`; call
`AddSingleWheelParticles(ws[i], dm.GetWheelStatus(i), compPrev[i] (+0x7E4), spd, &cp[i], &cp[i], side, i,
skidType[i] (+0x810), &bloodState[i] (+0x820+i), flags)`.

**F. 0x6AB154-0x6AB2B7** if node[3] (+0x654) != 0: wheel 1 then wheel 3:
`flags = (blood[w] ? 1 : 0) + (muddy[w] ? 2 : 0)` (bytes +0x821/+0x825, then +0x823/+0x827);
`p.z = (f.z + f.z) + cp[w].z; p.y = (f.y + f.y) + cp[w].y; p.x = (f.x + f.x) + cp[w].x` (x87 order z, y, x; f =
matrix forward; the doubled y and x are stored f32 first; p at [orig 0x44]);
`AddSingleWheelParticles(ws[w], GetWheelStatus(w), compPrev[w], spd, &cp[w], &p, 1.0f, id, skidType[w],
&bloodState[w], flags)` with (w, id) = (1, 5) and (3, 6). Note side is +1.0f for both (wheel 1 is a left wheel).

**G. 0x6AB2BC-0x6AB2FF** if `!CamNoRain() && !PlayerNoRain() && fabs(fwd) < 20.0f` (`test ah,5; jp` skips >= /
NaN) `&& Rain > 0.02f` (`test ah,0x41; jne`) `&& currArea == 0`: `AddWaterSplashParticles()`.

**H. 0x6AB304-0x6AB34B** if (byte +0x428 & 0x10) && !(handling->modelFlags (+0xCC) & 0x1000) && `fwd < 130.0f`
(jp skips >= / NaN) && !(byte +0x42B & 0x40) && !(byte +0x42E & 0x80): `AddExhaustParticles()`. Always
`AddDamagedVehicleParticles()`.

**I. 0x6AB350-0x6AB5C7 light switch.** `idx = model - 0x197` (movsx); if `(unsigned)idx > 0xC0` -> K (0x6ABC71).
Byte table 0x6ACCD0[idx] -> jump table 0x6ACCBC: 0 -> 0x6AB374 (models 0x197, 0x1A0, 0x1AB, 0x254, 0x255, 0x256,
0x257), 1 -> 0x6ABA84 (0x1A4), 2 -> 0x6ABAA3 (0x1B6), 3 -> 0x6ABBB3 (0x1EA), 4 -> K (all other 183 values).
Emergency (0x6AB374): if !(byte +0x42D & 0x80) -> K. Init `r1 = g2 = b2 = r2 = 0` (bytes: r1 = al -> [orig 0x50],
b2 = cl -> [orig 0x3C], r2 = dl -> [orig 0x38], g2 = [orig 0x40]). Second byte table 0x6ACDAC[idx] -> jump table
0x6ACD94 sets `P+ = (x, y, z)` at [orig 0x44] and `P- = (-x, y, z)` at [orig 0x54] (f32 immediates) and colours:

| model | case | (x, y, z) | r1 | (r2, g2, b2) |
|---|---|---|---|---|
| 0x197 FIRETRUK | 0 -> 0x6AB54E | (0.9, 3.2, 1.3) | 0xFF | (0xFF, 0xFF, 0) |
| 0x1A0 AMBULAN | 1 -> 0x6AB4E4 | (0.6, 0.9, 1.2) | 0xFF | (0xFF, 0xFF, 0xFF) |
| 0x1AB ENFORCER | 2 -> 0x6AB47A | (0.55, 1.1, 1.4) | 0xFF | (0, 0, 0xFF) |
| 0x254-0x256 cop cars | 3 -> 0x6AB410 | (0.7, -0.4, 1.0) | 0xFF | (0, 0, 0xFF) |
| 0x257 COPCARRU | 4 -> 0x6AB3A6 | (0.7, -0.1, 1.2) | 0xFF | (0, 0, 0xFF) |

(Case 5 -> 0x6AB5C6 is the switch default; unreachable because only the seven models above reach 0x6AB374. Cases
0/1 write [orig 0x38] = dl and [orig 0x40] = 0xFF at 0x6AB5B5; cases 2-4 jump to 0x6AB5BE and keep the zero-init.)
`g1` and `b1` are compile-time 0. Then 0x6AB5C7 `jmp 0x401740` -> `edi = m_snTimeInMilliseconds` -> 0x6AB5CC.

**J1. 0x6AB5CC-0x6AB711 point-light colour ramp.** `t = edi & 0x3FF`. If `t < 0x200`: `R = r1/3, G = 0, B = 0`;
else `R = r2/3, G = g2/3, B = b2/3` (each `imul 0x2AAAAAAB` + sign fix = signed /3 of the zero-extended byte;
R in bl, G byte [orig 0x13], B byte [orig 0x1F]). `t2 = t & 0x1FF`. If `t2 < 100`: `k = t2`; else if `t2 > 0x19C`:
`k = 0x200 - t2`; else (100..412) no ramp. With a ramp: `fild k` (the `+4294967296.0f` [0x858C54] unsigned fix-up
is guarded by `jge` and never taken), then for R, G, B in that order: `C = _ftol(f32(int(C) * k) * 0.01f)`
(`fild C; fmul st(1); fmul [0x858C58]`; low byte kept); k popped at 0x6AB70F.

**J2. 0x6AB715-0x6AB820 AddLight.** `pos = m_matrix ? &m_matrix->pos : &this->placement.pos` (note `fld
[m_matrix+0x28]` is executed before/regardless of the null test, 0x6AB722). `lp.z = (up.z + up.z) + pos.z`,
`lp.y = pos.y + (up.y + up.y)`, `lp.x = pos.x + (up.x + up.x)` (x87 order: up.z doubled, up.y doubled and stored
f32, up.x doubled and stored f32, then sums z, y, x). Call `CPointLights::AddLight(0, lp, (0,0,0), 10.0f,
f32(R)*0.005f, f32(G)*0.005f, f32(B)*0.005f, 0, true, nullptr)` (each colour `fild` of the zero-extended byte, `fmul
[0x858B4C]`, stored f32; the dir vector is built from three stored 0 dwords).

**J3. 0x6AB823-0x6AB8D4 corona colours.** With `S = [0xB7C4E4]`: `r1' = _ftol(f32(int(r1)) * S * 0.1f)`;
`g1' = b1' = _ftol(S * 0.0f)` (`fld S; fmul [0x858B50]`, one call, stored to both [orig 0x14] and [orig 0x74]);
`r2' = _ftol(int(r2) * S * 0.1f)`, `g2' = ...g2...`, `b2' = ...b2...` (x87: `fild c; fmul S; fmul [0x858B1C]`;
`_ftol` truncation; low byte). ebp = r2' (dword from [orig 0x38]; only the low byte is meaningful).

**J4. 0x6AB8E0-0x6ABA7F 4 coronas.** For k = 0..3 (counter [orig 0x18] = 4 down; `id = (uint32)this + 0x15 + k`):
`phase = ((m_snTimeInMilliseconds + 0x40*k) >> 8) & 3` (time read directly from 0xB7CB84). Position (x87 order,
`w = 3.0f - f32(k)`): `tz = P-.z * w` (stored f32), `ty = P-.y * w` (stored), `tx = w * P-.x` (kept), `lz = P+.z * k`
(kept), `ly = P+.y * k` (stored), `lx = P+.x * k` (stored); `sz = lz + tz` (stored), `sy = ly + ty` (stored),
`sx = lx + tx` (stored); `pos = (sx, sy, sz) * 0.33333334f` ([0x859040], computed z, y, x; stored f32 to [orig
0xF8]). So k = 0 is P-, k = 3 is P+. Phase 0: `RegisterCorona(id, this, r1', g1', b1', 0xFF, &pos, 0.4f,
[0xB6F118] * 150.0f, CORONATYPE_SHINYSTAR(0), flare 0, refl 0, LOS 0, trail 0, 0.0f, false, 1.5f, false, 15.0f,
false, true)`; phase 2: same with `(r2', g2', b2')`; phases 1 and 3: nothing. Then -> K.

**J5. 0x6ABA84 / 0x6ABAA3 taxi light** (TAXI / CABBIE): if !(byte +0x868 & 1) -> K. `v = (0, -0.4f, 0.95f)` (TAXI)
or `(0, 0, 0.85f)` (CABBIE) at [orig 0x20]; `w = *m_matrix * v` ([orig 0x44]); `c = _ftol(S * 10.0f)` (fld
0xB7C4E4, fmul [0x85862C]); `RegisterCorona((uint32)this + 0x11, this, c, c, 0, 0xFF, &v (the local offset, not w),
0.8f, [0xB6F118] * 150.0f, CORONATYPE_HEADLIGHT(1), flare 0, refl 1, LOS 0, trail 0, 0.0f, false, 1.5f, false, 15.0f,
false, true)`; `AddLight(0, w, (0,0,0), 10.0f, 0.1f, 0.1f, 0.05f, 0, true, nullptr)`. -> K.

**J6. 0x6ABBB3 FBIRANCH**: if (byte +0x42D & 0x80) and (`m_snTimeInMilliseconds & 0x100`): `v = (0, 1.2f, 0.5f)`
([orig 0x2C], written before the time test); `u = TheCamera.GetTopDirection()`; `d = (f.z*u.z + f.y*u.y) + f.x*u.x`
(f = m_matrix->forward); if `d < 0.0f` (`test ah,5; jp` skips >= / NaN): `RegisterCorona((uint32)this + 0x15, this,
0, 0, 0xFF, 0xFF, &v, 0.4f, [0xB6F118] * 150.0f, 0, 0, 0, 0, 0, 0.0f, false, 1.5f, false, 15.0f, false, true)`.

**K. 0x6ABC71-0x6ABCFA lights and shadow.** If model != 0x1B9 && != 0xFFFE && != 0x1B0 && !(byte +0x42C & 0x80)
&& `GetVehicleAppearance() != 3`: `DoVehicleLights(*m_matrix, (model == 0x214 || model == 0x1D7) ? 0 : 3)`.
Then unless (`this == FindPlayerVehicle(-1, false)` && `TheCamera.GetLookingForwardFirstPerson()`):
`StoreShadowForVehicle(this, ((byte +0x42C & 0x80) && subtype +0x594 != 0) ? 4 (VEH_SHD_RC) : 0)`.

**L1. 0x6ABCFD-0x6ABE7C wheel directions.** Construct mat2; `mi` saved. If model == RHINO: `SetComponentRotation(
node[16] (+0x688), 2, m_fDoomVerticalRotation (+0x94C), true)`. `isHeli = (handling->modelFlags & 0x2000000) != 0`
(byte [orig 0x13]). `s = m_fSteerAngle`; `v = (-sin(s), cos(s), 0)` (fsin/fcos of the f32 field; sin stored f32,
negated with fchs); `frontDir = Multiply3x3(m_matrix, v)` [orig 0xEC]; `rearDir = m_matrix->forward` [orig 0x7C].
Then with `hf = dword +0x38C`: if `hf & 0x20` (STEER_REARWHEELS): `frontDir = m_matrix->forward`, `v = (+sin(s),
cos(s), 0)` (no fchs), `rearDir = Multiply3x3(m_matrix, v)`; else if `hf & 0x40` (HB_REARWHEEL_STEER) and
`m_f2ndSteerAngle != 0.0f` (`test ah,0x44; jnp` skips only on equal; NaN computes): `v = (-sin(s2), cos(s2), 0)`,
`rearDir = Multiply3x3(m_matrix, v)`.

**L2. 0x6ABE83-0x6ABFC0 wheel rotation**, i = 0..3 (u16 compare, `jb 4`): skip if `m_WheelCounts[i] (+0x7F4) <= 0`
(or NaN); skip if isHeli and i != 0 && i != 2. `pos` as in J2 (null -> placement). `off[i] = cp[i].point - pos`
(x87 z, y, x; stored f32 to [orig 0x110 + 0xC*i]); `vel[i] = GetSpeed(off[i])` ([orig 0x158 + 0xC*i]);
front (i == 0 || i == 2): `radius = mi->m_fWheelSizeFront (+0x40) * 0.5f`, dir = frontDir; else `m_fWheelSizeRear
(+0x44) * 0.5f`, dir = rearDir; `wheelSpeed[i] = ProcessWheelRotation(ws[i], dir, vel[i], radius)` (fstp f32);
`wheelRotation[i] = ms_fTimeStep * wheelSpeed[i] + wheelRotation[i]` (fld timestep, fmul stored f32, fadd).

**M. 0x6ABFC6-0x6AC020** `flags = (model == RHINO) ? 7 : 0`; `UpdateWheelMatrix(node, flags)` for node 2, 5, 4, 7;
then node 3 if +0x654 != 0; node 6 if +0x660 != 0.

**N. 0x6AC025-0x6AC06E** if model == RHINO -> skip N **and O** (jump to 0x6AC232). Else `ProcessSwingingDoor(node,
door)` for (10, 2), (8, 3), (11, 4), (9, 5), (16, 0), (17, 1).

**O. 0x6AC073-0x6AC22D panels and swinging chassis.** For the 3 panels (stride 0x20): if `(int16)frameId > -1`
(`cmp ax, 0xFFFF; jle` - signed, so ids >= 0x8000 are also skipped): `panel.ProcessPanel(this,
node[(int16)frameId], m_moveForce (+0x8B4, by value), m_turnForce (+0x8C0, by value), -1.0f, -1.0f)`.
Chassis: `n = (model == 0x220 FIRELA) ? 21 : 1`; if `byte +0x717 == 1` (DOOR_HIT_MAX_END) and node[n] != 0:
`mat2.Attach(&node[n]->modelling, false)`; `v = mat2.pos` [orig 0x64]; `rot = (0,0,0)` [orig 0x44];
`w = Multiply3x3(m_matrix, v)` [orig 0x88]; `m_swingingChassis.Process(this, m_moveForce, m_turnForce, w)` (result
ignored); `rot[m_axis (byte +0x716)] = m_angle (+0x718)` (index not range-checked); `mat2.SetRotate(rot.x, rot.y,
rot.z)`; if FIRELA: `mat2.RotateX(f32(int(misc u16 +0x86C)) * -0.0001f)` ([0x87100C]); `mat2.pos.x = v.x +
pos.x; mat2.pos.y = v.y + pos.y; mat2.pos.z = pos.z + v.z`; `mat2.UpdateRW()`.

**O2. 0x6AC232-0x6AC2A7** (all models, RHINO included): `m_moveForce = moveSpeed + frictionMoveSpeed` and
`m_turnForce = turnSpeed + frictionTurnSpeed` (x87 per component z, y, x; stored via f32 temps).

**P. 0x6AC29F-0x6ACBC2 model components** (if/else chain on model; every branch ends at 0x6ACBC7). "SCR(n, ax, a, r)"
= `SetComponentRotation(node[n], ax, a, r)`; `misc = f32(int(u16 +0x86C))` (fild of the zero-extended value).
- 0x1A8 BFINJECT: `a = ms_fTimeStep * (fabs(gas +0x49C) > 0.0f ? 0.5f : 0.3f)` (compare done first; `test
  ah,0x41; jne` -> 0.3 on <= / NaN) stored f32; SCR(20, 1, a, false); SCR(24, 1, f32(a * 1.5f), false);
  GetColModel() (result unused); for k = 0..2 over table 0x8D31F0 = {0x15, 0, 0x16} = nodes {21, -, 22} for wheels 0 and 2 (entry 1 is 0, skipped): if entry != 0 and node != 0:
  `mat2.Attach(&node->modelling, false)`; (copies mat2.pos to [orig 0x64], unused); `L = cd->lines[k]` (cd from
  [orig 0x70]); `mat2.forward.z (orig 0xAC) = (wheelPos[k] - (L.start.z - handling->m_fSuspensionUpperLimit))
  / (L.start.y - mat2.pos.y)`; UpdateRW. Then `GetWheelPosn(1, &p, false)` (mi); `SetTransmissionRotation(node[23],
  wheelPos[1], wheelPos[3], p, false)`.
- 0x1E6 DOZER: SCR(20, 0, misc * 0.0002f, true).
- 0x20C CEMENT: if status == 5 ((byte +0x36 & 0xF8) == 0x28) nothing. Else SCR(20, 0, -0.17453294f, true);
  `m_fPropRotate = ((misc / int(i16 [0x8D314C])) + 0.5f) * ms_fTimeStep * 0.02f + m_fPropRotate` (fild, fidiv,
  fadd [0x858B8C], fmul timestep, fmul [0x871054], fadd field; fst/fstp); SCR(21, 1, m_fPropRotate, true);
  SCR(21, 0, -0.17453294f, false).
- 0x1BB PACKER: SCR(20, 0, misc * -0.0001f, true).
- 0x20D TOWTRUCK: `a = misc * [0x8D3144] / int(u16 [0x8D313C])` (fild, fmul, fidiv) stored f32 [orig 0x50];
  SCR(20, 0, a, true). If m_pTrailer != 0: [orig 0x50] = `-a` (a * -1.0f); if
  `trailer->vtbl[+0xEC](&hitch [orig 0x44], true, this)` and `this->vtbl[+0xF0](&bar [orig 0x2C], true, trailer)`:
  `d = hitch - bar` (x, y, z, each stored f32 in place); `x1 = f32(((d.z*f.z + d.y*f.y) + d.x*f.x) / [0x8D3148])`
  (f = m_matrix->forward); SCR(21, 0, asin(x1) + (-a), true); `x2 = f32((-1.0f / [0x8D3148]) * ((d.z*r.z + d.y*r.y)
  + d.x*r.x))` (r = m_matrix->right; the quotient is computed at run time); SCR(21, 1, f32(asin(x2)), false). If
  either virtual returns false: nothing more. Without trailer: SCR(21, 0, -a, true); then if `m_panels[0].frameId
  (+0x6AC, u16) == 0x15`: `mat2.Attach(&node[21]->modelling, false)`; `mat2.up.y += m_panels[0].m_vecRotation.x
  (+0x6B4)`; `mat2.up.x += m_panels[0].m_vecRotation.y (+0x6B8)`; UpdateRW (sic: swapped components on the up
  vector - keep).
- 0x213 TRACTOR: SCR(20, 0, f32(misc * [0x8D3144] / int(u16 [0x8D313C])), true).
- 0x212 FORKLIFT: if node[20] != 0: `f = GetFrameFromId(ms_modelInfoPtrs[530]->m_pRwObject, 0x14)` (cdecl, no null
  check); `t = f->modelling.pos` (frame +0x40); `t.z = misc * 0.0006f + t.z`; `mat2.Attach(&node[20]->modelling,
  false)`; `mat2.SetTranslateOnly(t.x, t.y, t.z)`; UpdateRW.
- 0x214 COMBINE: if `m_fPropRotate > 6.2831855f`: `-= 6.2831855f` (once). `s = (ms.z*f.z + ms.y*f.y) + ms.x*f.x`
  (recomputed at each use); `c = s > 0.1f ? 0.1f : (s < 0.0f ? 0.0f : s)` (NaN passes through as s);
  `m_fPropRotate = m_fPropRotate - c * ms_fTimeStep` (fst); SCR(21, 0, m_fPropRotate, true);
  `GetWheelPosn(0, &q [orig 0xE0], false)`; `mat2.Attach(&node[20]->modelling, false)`; `avg = (wp[0] + wp[2]) *
  0.5f`, `diff = wp[0] - wp[2]`; `A = -atan2(avg - q.z, mat2.pos.y - q.y)` (fpatan, fchs, stored f32);
  `B = atan2(diff, |q.x| + |q.x|)` (stored f32); SCR(20, 0, A + 3.1415927f, true); SCR(20, 1, B, false).
  Tilt: `t = 0`; if `m_matrix->up.z > 0.0f` && byte +0x486 == 0 && byte +0x487 == 0: `t = asin(f32
  m_matrix->right.z)` (fst f32), clamped: if `t > [0x8D31EC]` t = [0x8D31EC]; else if `t < -[0x8D31EC]` t =
  -[0x8D31EC] (NaN kept). `p = pow([0x8D31E8], ms_fTimeStep)`; `m_fDoomVerticalRotation = (1.0f - p) * t + p *
  m_fDoomVerticalRotation` (x87: `(1-p)*t` first, then `p*doom`, faddp; fst/fstp); SCR(1, 1, that, true).
- 0x238 BANDITO, 0x1B2 HOTKNIFE: GetColModel() (unused); for k = 0..3 over table 0x8D31D8 = {20, 22, 21, 23} (no
  zero check on the entry): if node != 0: Attach; `mat2.right.z (orig 0x9C) = (wheelPos[k] - (L.start.z -
  m_fSuspensionUpperLimit)) / (L.start.x - mat2.pos.x)`; UpdateRW. Then `a = ms_fTimeStep * (fabs(gas) > 0 ? 0.5f :
  0.3f)` (stored f32 into [orig 0x70]); SCR(24, 1, a, false).
- 0x1B0 RHINO, 0x259 SWATVAN: SCR(20, 2, m_fDoomVerticalRotation, true); SCR(21, 0, m_fDoomHorizontalRotation, true).
- 0x197 FIRETRUK: SCR(20, 0, m_fDoomHorizontalRotation, true); SCR(20, 2, m_fDoomVerticalRotation, false).
- 0x1DD ZR350: SCR(20, 0, m_fPropRotate, true).
- others: nothing.

**P2. 0x6ACBC7-0x6ACC86 transmission.** If model == 0x1EF (SANDKING, via chunk 0x40649C) or ((hf & 0x10000) &&
(hf & 0x20000) && subtype == 0): `GetWheelPosn(0, &p, false)`; `SetTransmissionRotation(node[20], wheelPos[0],
wheelPos[2], p, true)`; `GetWheelPosn(1, &p, false)`; `SetTransmissionRotation(node[21], wheelPos[1],
wheelPos[3], p, false)`.

**Q. 0x6ACC8B** destroy mat2 (state -1), restore fs:[0], return.

### Oddities / original bugs (keep)
- 0x6AC316 / 0x6ACAE0: GetColModel() result discarded; the loops use the col data pointer read at 0x6AAB90.
- 0x6AC6A0-0x6AC6C8 TOWTRUCK: panel rotation x is added to mat2.up.y and y to mat2.up.x.
- F (0x6AB1A6 / 0x6AB248): middle wheels pass side +1.0f for both sides.
- J2 / L2: `fld [m_matrix+0x28]` / matrix use happens before the null fallback; only the position pointer falls
  back.
- O: `rot[m_axis]` written without range check; panel id compare is signed 16-bit.
- J5: the taxi corona gets the local offset (attached to this), the AddLight gets the world position.
- COMBINE reuses m_fDoomVerticalRotation as smoothed tilt; RHINO additionally rotates node[16] (BONNET) at
  0x6ABD42 besides the node[20]/node[21] turret pair at 0x6ACAAE.
- DoVehicleLights flag value 3 includes bit 1, which gr's eVehicleLightsFlags does not name.
- The .data values 0x8D313C/0x8D3144/0x8D3148/0x8D314C/0x8D31D8.../0x8D31E8/0x8D31EC are mutable globals in the
  original (gr uses constexpr for the first and fourth); a port must read the globals.

### gta-reversed
`CAutomobile::PreRender` (Automobile.cpp:6437) is only `plugin::CallMethod<0x6AAB50>`; no C++ body exists in the
pinned gr tree, so there is nothing to diff. Related gr notes: the Coronas install list names 0x6FC580 "texture"
although its signature is the eCoronaType overload (the bytes pass type ids 0/1 here); gr ProcessWheelRotation
(0x6D1230, hooked) returns `-dot(arg1, arg2) / arg3`, -1.1 when spinning, 0 when fixed.

### Open questions
- 0x6AB5C6: switch default of the second table would read uninitialised P+/P- - unreachable as far as the tables
  show; no behaviour depends on it.
- 0x6AC5A4-0x6AC64F: `_CIasin` of a value outside [-1, 1] gives NaN, which is passed on to SetComponentRotation;
  not otherwise handled (original behaviour, not a doubt about the bytes).

## 0x6B1330 CAutomobile::vfunc_0  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall CAutomobile* (CAutomobile* this, u8 flags [esp+4])`, `ret 4`. Slot 0 of vtable 0x871120
  (scalar deleting destructor; proposed name `CAutomobile::scalar_deleting_destructor`).
- Callers: virtual (CCarCtrl::CreatePoliceChase 0x42C2B0, CreateConvoy 0x42C740, GenerateOneRandomCar 0x430050,
  CSetPiece::Update 0x499A80). Callees: CAutomobile::Destructor 0x6A61E0, CVehicle::operator delete 0x6E2D90.
- Behaviour: `~CAutomobile(this)`; `if (flags & 1) CVehicle::operator delete(this)`; return `this` (EAX).

