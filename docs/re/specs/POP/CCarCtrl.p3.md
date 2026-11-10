# CCarCtrl (POP) - full specs, part 3 (group P3)

Conventions as CCarCtrl.md (P1): entity position = `m_matrix (+0x14) ? &m_matrix->pos (+0x30) : &placement.pos (+0x04)`; ThePaths 0x96F050 (m_pPathNodes[] 0x96F854, CPathNode stride 0x1C; m_pNaviNodes[] 0x96F974, CCarPathLink stride 0xE; m_pNodeLinks[] 0x96FA94; m_pNaviLinks[] 0x96FDF4); CVehicle autopilot at +0x390 (gr CAutoPilot offset = vehicle offset - 0x390). CCarPathLinkAddress (u16): area = v >> 10, index = v & 0x3FF. `_rand` 0x821B1E, `_ftol` 0x821B40 (truncation), CRT floor 0x8219F0. Recurring x87 idioms: "normalised into [-pi, pi]" = `while (a < -pi) a += 2pi` (`test ah,5; jp` exit on >=/NaN) then `while (a > pi) a -= 2pi` (`test ah,0x41; jne` exit on <=/NaN), constants 0x858CC0/0x858CBC/0x858CB8; `min1(g) = (1.0f < g) ? 1.0f : g` (NaN keeps g). gta-reversed deviation notes for this part: CCarCtrl.p3.deviations.md.

## 0x430050 CCarCtrl::GenerateOneRandomCar  (catalogue: gr-unreversed, port level 33)
Structured by address block (2171 instructions). Precision is spent on branch conditions, constants, RNG order and float order.
- Convention / signature: `__cdecl void ()`, plain `ret`. Entry `nop; jmp 0x403AEE` -> `sub esp,0x12C; jmp 0x430056` (.text detour stub,
  same behaviour); then ebx/ebp/esi/edi pushed; E = esp after these four pushes (all locals below are E-relative; the listing offsets
  differ by 4 per outstanding push). Main locals: total cars E+0x10 (int) / E+0x54 (float), density E+0x14 (then dirX, then search
  radius, then lanes, then halfLen), dirY / outFrac / frac E+0x24, dotLimit E+0x20 (later frac2), wantAhead byte E+0x2C (later the i16
  object count), model E+0x28, type E+0x58 (int, written by ChooseModel), isBoat E+0x1F, gangZone E+0x36, topDown E+0x37, madDriver E+0x47,
  N2 (CNodeAddress) E+0x48, N1 E+0x4C, creation coors / curve position E+0x90..0x98, player centre C E+0xC4..0xCC, player speed
  E+0xAC..0xB4. Dead stores at entry: `w[E+0x3C] = w[E+0x38] = 0xFFFF` (0x43005D/0x430062).
- Callers: CCarCtrl::GenerateRandomCars 0x4341C0 (3 sites).
- Callees (in first-use order; cdecl unless noted): FindPlayerCentreOfWorld 0x56E250, FindPlayerSpeed 0x56E090, CCullZones::FewerCars 0x72DD80,
  CPopulation::FindCarMultiplierMotorway 0x611B60 (-> ST0), FindPlayerWanted 0x56E230 (-> CWanted*), CGangWars::GangWarFightingGoingOn 0x443AC0,
  ChoosePoliceCarModel 0x421980, ChooseModel 0x424CE0, CGameLogic::LaRiotsActiveHere 0x441C10, `_rand` 0x821B1E, FindPlayerVehicle 0x56E0D0,
  GenerateCarCreationCoors2 0x424210 (signature in CCarCtrl.md), CStreaming::RequestModel 0x4087E0, CLoadedCarGroup::PickLeastUsedModel 0x611E90
  (thiscall), CWorld::FindObjectsKindaColliding 0x568B80 (11 args), CNodeAddress operators (thiscall, `ret 4`): `!=` 0x420950 (thunk to
  0x156AB40), `==` 0x420920 (-> 0x1565640), `<` 0x420980 (-> 0x156E7E0: area less, or same area and node less), CTheZones::GetZoneInfo
  0x572400, GetNewVehicleDependingOnCarModel 0x421440, CGeneral::GetRandomNumberInRange<float> 0x41BD90 (pushes max then min; ST0),
  `_ftol` 0x821B40, CCarAI::FindPoliceBoatMissionForWantedLevel 0x41CA50, FindPoliceCarSpeedForWantedLevel 0x41CAA0, FindPoliceBikeMission...
  0x41CA40, FindPoliceCarMission... 0x41C9D0, CVehicle::GetVehicleAppearance 0x6D1080 (thiscall), CGeneral::GetRandomNumberInRange(int,int)
  0x407180 (pushes max then min), CPathNode::GetPosition 0x420A10 (21 sites), VectorSub 0x40FE60 (out = a - b), CCarPathLink::GetNodeCoors
  0x420A60 (thiscall (CVector* out) -> out = (x*0.125f, y*0.125f, 0)), CVector::Normalise 0x59C910, PickNextNodeRandomly 0x42DE80,
  CCarPathLink::OneWayLaneOffset 0x44DB00, FindSpeedMultiplierWithSpeedFromNodes 0x424130 (cdecl (int8) -> ST0), CCurves::CalcSpeedScaleFactor
  0x43C710 (cdecl (const CVector* start, const CVector* end, float sdx, float sdy, float edx, float edy) -> ST0), CCurves::CalcCurvePoint 0x43C900,
  CVector::Magnitude 0x4082C0, 0x40FEC0 (`out = v * s`, cdecl (out, v, float s)), 0x40FE30 (`out = a + b`), 0x4119D0 (`out = v * (1/d)`, cdecl
  (out, v, float d), 1/d stored f32 first), CWaterLevel::GetWaterLevel 0x6EB690, CWorld::ProcessVerticalLine 0x5674E0, CModelInfo::IsBoatModel
  0x4C5A70, vtable +0xD4 GetHeightAboveRoad (thiscall -> ST0), CPlaceable::SetPosn 0x4241C0, CVisibilityPlugins::SetClumpAlpha 0x732B00,
  CVehicle::AddVehicleUpgrade 0x6E3290, CEntity::GetIsOnScreen 0x534540, 0x420800 (`max(a, b)` cdecl floats: returns a if a > b else b),
  CVehicleModelInfo::ChooseVehicleColour 0x4C8500 (thiscall (u8*, u8*, u8*, u8*, int32 1)), CWorld::Add 0x563220, CAutomobile::SetRandomDamage
  0x6A2530 (thiscall (bool)), FindPlayerPed 0x56E210, CPlayerPed::GetWantedLevel 0x41BE60, CreatePoliceChase 0x42C2B0 (cdecl (veh, type, N1) -> AL),
  CreateConvoy 0x42C740 (cdecl (veh, type) -> AL), SetUpDriverAndPassengersForVehicle 0x4217C0, CPlaceable::GetTopDirection 0x41CCB0,
  CCarAI::AddPoliceCarOccupants 0x41C070 (cdecl (veh, false)), CVehicle::ChangeLawEnforcerState 0x6D2330 (thiscall (1)),
  CStreaming::PossiblyStreamCarOutAfterCreation 0x40BA70, 0x421120 (thunk to 0x1563560: thiscall on the model info, `b[this+0x50] =
  min(b[this+0x50] + 1, 120)` with a signed byte read), virtual slot 0 scalar deleting destructor with arg 1 (the "delete" paths).
- Globals: CCarCtrl counters NumRandomCars 0x969094, NumLawEnforcerCars 0x969098, NumMissionCars 0x96909C, NumAmbulancesOnDuty 0x9690A8,
  NumFireTrucksOnDuty 0x9690AC, CarDensityMultiplier 0x8A5B20 (f32), MaxNumberOfCarsInUse 0x8A5B24 (i32), LastTimeLawEnforcerCreated 0x9690B8 (RW),
  TimeNextMadDriverChaseCreated 0x9690BC (RW), bCarIsBeingCreated 0x9690CC (W); CPopCycle::m_NumOther_Cars 0xC0BC30, m_NumCops_Cars 0xC0BC34,
  m_NumGangs_Cars 0xC0BC38, m_NumDealers_Cars 0xC0BC3C (f32), m_pCurrZone 0xC0BC64, m_nCurrentZoneType 0xC0BC6C; CGame::currArea 0xB72914;
  gbLARiots 0xB72958, gbLARiots_NoPoliceCars 0xB72959; CWorld::PlayerInFocus 0xB7CD74; CTimer m_FrameCounter 0xB7CB4C, m_snTimeInMilliseconds
  0xB7CB84; CCheat::m_aCheatsActive[31] 0x96914F (AGGRESSIVE_DRIVERS), [70] 0x969176 (FUNHOUSE_THEME); MI_HYDRAULICS 0x8CD76C (u16);
  CStreaming::ms_aInfoForModel load state 0x8E4CD0 + model*0x14 (0x8E6E68 = model 0x1AE); CPopulation::m_LoadedBoats 0xC0E998; TheCamera 0xB6F028:
  +0x14 matrix 0xB6F03C, +0x04 pos 0xB6F02C, +0xDC/+0xE0 0xB6F104/0xB6F108 (2D direction used when the player is slow or on foot),
  +0xF4 0xB6F11C (f32 distance multiplier; gr name not resolved here), +0x98C 0xB6F9B4 (forward.z of the camera vector at +0x984, see
  RENDER/CWeather.deviations.md); ThePaths tables 0x96F854 (nodes), 0x96F974 (navi nodes), 0x96FA94 (node links), 0x96FDF4 (navi links);
  ms_modelInfoPtrs 0xA9B0C8. Jump tables 0x431F3C, 0x431F4C (4 entries each), 0x431F5C + byte index 0x431F68 (24 entries).
- Constants (f32): `0x858CC8 = 0.6000000238418579`, `0x858CAC = -0.8999999761581421`, `0x858EE8 = 0.4000000059604645`, `0x858624 = 1.0`,
  `0x858B1C = 0.10000000149011612`, `0x858970 = 160.0`, `0x85862C = 10.0`, `0x858B8C = 0.5`, `0x858B50 = 0.0`, `0x858C58 = 0.009999999776482582`,
  `0x858C50 = 5.400000095367432`, `0x859010 = 1.4580000638961792`, `0x858C48 = 0.125`, `0x858C4C = 1000.0`, `0x858C54 = 4294967296.0`,
  `0x858CA0 = 2.0`, `0x858FEC = 1000000000.0`, `0x858F48 = 7.0`, `0x858A28 = 150.0`, `0x858BB0 = 120.0`, `0x858F94 = 0.0058823530562222`,
  `0x858CB4 = 45.0`. Immediates: 0.707f (0x3F34FDF4), -1.0f, 0.85f (0x3F59999A), 38.0f, 40.0f, 8.0f, 21/13, 27/18, 15/10, 24/15, 35/25,
  18/14, 24/18 (float range pairs), 1000.0f/-1000.0f, 1e9f (0x4E6E6B28), 60.0f, 170.0f, 480/240, 1200/600, 0.02f, 0x1388 = 5000,
  0x1F40 = 8000, 0x63 = 99, 0x3E8 = 1000.

Block A - budget (0x430056-0x430164):
  1. `C = FindPlayerCentreOfWorld(PlayerInFocus)`, `S = FindPlayerSpeed(-1)` (copied).
  2. `total = ((((FireTrucks + Ambulances) + MissionCars) + LawEnforcerCars) + RandomCars)` (int). `dens = CarDensityMultiplier`; if
     `FewerCars()` then `dens *= 0.6f` (stored). `tf = (float)total` (stored).
  3. Return unless `Motorway() * (float)MaxNumberOfCarsInUse (fimul) * dens > tf` (`test ah,0x41; jnp` returns on `<` or `==`; NaN continues,
     0x430132). Return unless `Motorway() * (((Other + Cops) + Gangs) + Dealers) * dens > tf` (Motorway called again, 0x430164).
Block B - model choice (0x43016A-0x430294):
  4. Police case: `w = FindPlayerWanted(-1)` (re-called for each field). If `w->+0x2C (wanted level) > 1` and `NumLawEnforcerCars < (u8)w->+0x1A`
     (m_MaxCopCarsInPursuit, signed int compare) and `(u8)w->+0x18 < (u8)w->+0x19` (cops in pursuit < max) and `currArea == 0` and
     `!GangWarFightingGoingOn()` and (`level > 3`, or `level > 2 && now > Last + 5000`, or `now > Last + 8000`; unsigned compares, 0x4301D7-0x43020A):
     `model = ChoosePoliceCarModel(0)`, `type = 13`.
  5. Otherwise `model = ChooseModel(&type)`; return if `model == -1`; return if (`type == 13 || type == 24`) and wanted level >= 1.
  6. If `LaRiotsActiveHere()` and `!gbLARiots_NoPoliceCars` and `(int8)(rand() & 0x7F) < 0x37`: `model = ChoosePoliceCarModel(0)`, `type = 13`.
Block C - search direction (0x430298-0x4303E1):
  7. If `[0xB6F9B4] < -0.9f` (camera looking down; `test ah,5; jp`): topDown = 1, `dirX = dirY = 0.707f`, `dotLimit = -1.0f`, wantAhead = 1.
  8. Else `pv = FindPlayerVehicle(-1, false)`; if pv: `s = sqrt(mv.y*mv.y + mv.x*mv.x)` (pv +0x44/+0x48):
     - `s > 0.4f`: `dirX = mv.x * (1/s)`, `dirY = (1/s) * mv.y` (stored); by `FrameCounter & 3` (table 0x431F3C): 0,1 -> dotLimit 0.85f, wantAhead 1;
       2 -> 0.707f, 1; 3 -> 0.707f, 0.
     - else `s > 0.1f`: same normalisation; table 0x431F4C: 0 -> 0.85f/1; 1 -> 0.707f/1; 2,3 -> 0.707f/0.
     - else (incl. NaN): dir = (TheCamera+0xDC, +0xE0), then as "no vehicle".
     No vehicle: dir = camera 2D direction; `FrameCounter & 1`: 0 -> 0.707f/1, 1 -> 0.707f/0.
Block D - creation point (0x4303E6-0x43045F):
  9. `restrict = !(type == 13 && wantedLevel >= 1)`. `ok = GenerateCarCreationCoors2(C, dirX, dirY, dotLimit, wantAhead, TheCamera+0xF4 * 160.0f,
     38.0f, &coors, &N1, &N2, &frac, restrict, 0)`; return if false.
Block E - node checks, boats, clearance, lanes (0x430465-0x43069E):
  10. `prob = min(N1.spawnProb, N2.spawnProb)` (node +0x1A & 0xF); return if `(rand() & 0xF) > prob` (unsigned).
  11. If N1 is a water node (node +0x18 bit 7): isBoat = 1; if `type == 13`: `model = 0x1AE` (430 PREDATOR), `type = 24`, and if its load
      state (0x8E6E68) != 1: `RequestModel(0x1AE, 8)` and return (vehicle not created). Else `model = m_LoadedBoats.PickLeastUsedModel(1)`;
      return if -1 or its load state != 1. Clearance radius `r = 40.0f`. Land node: `r = 8.0f`.
  12. `FindObjectsKindaColliding(coors, r, true, &cnt, 2, null, buildings 0, vehicles 1, peds 1, objects 0, dummies 0)`; return if `cnt != 0`.
  13. Link to N2: `i = 0`; while `i < N1.numLinks` and `nodeLinks[N1.area][N1.baseLink + i] != N2` (0x420950) `i++` (i as i16). `nav =
      naviLinks[N1.area][N1.baseLink + i]` (u16; read even when no link matched, i == numLinks). L = that navi link; `lanes = (L.attachedTo == N2)
      ? (L.+0xB & 7) : ((L.+0xB >> 3) & 7)` (0x430603-0x430621).
  14. If `lanes > 1`: return if the model's vehicle type (+0x3C) == 10 (BMX). Else: return if model is 0x1B5 (437 COACH) or 0x1AF (431 BUS).
      Return if `lanes == 0`.
  15. If `m_pCurrZone`: `z = GetZoneInfo(&coors, 0)->+0xF & 0x1F`; if `17 <= z <= 19`: return if `z != m_nCurrentZoneType`, else gangZone = 1.
Block F - vehicle creation and driving parameters (0x43069E-0x430943):
  16. `veh = GetNewVehicleDependingOnCarModel(model, 1)` (NO null check: a full pool crashes on the following writes). Autopilot:
      `w[+0x398] = 0xFFFF`, `+0x390 = N1`, `+0x394 = N2`.
  17. Cruise speed (`+0x3D0`, u8 = ftol result), mission (+0x3BA), temp action (+0x3BB), style (+0x3B9):
      - type 13: tempAction 0; if wanted level == 0: cruise = ftol(Rand(18, 24)), style 0, mission 1; else cruise =
        FindPoliceCarSpeedForWantedLevel(veh), mission = (GetVehicleAppearance() == 2) ? FindPoliceBikeMission() : FindPoliceCarMission(),
        style 2. If model == 0x1EA (490 FBIRANCH): colours +0x434 = +0x435 = 0. Then `+0x42C |= 1` (bCreatedAsPoliceVehicle).
      - type 24: tempAction 0, cruise = ftol(Rand(14, 18)), style 2, mission = FindPoliceBoatMissionForWantedLevel(); `+0x42C |= 1`.
      - else: cruise = ftol(Rand(13, 21)) (always consumed); then type 3 -> ftol(Rand(18, 27)), type 1 -> ftol(Rand(10, 15)). If
        `colModel.bbox.max.y - min.y > 10.0f` (`test ah,0x41; je`, model info +0x14 -> col +0x10 / +0x04) or `type == 5`:
        `cruise = (3 * cruise) / 4` (signed idiv-by-4 idiom). If isBoat: model 0x1BE (446 SQUALO), 0x1C4 (452 SPEEDER), 0x1ED (493 JETMAX)
        -> ftol(Rand(25, 35)), other boats -> ftol(Rand(15, 24)). Then mission 1, tempAction 0, style 0 (no bCreatedAsPoliceVehicle).
  18. If model == 0x1A7 (423 MRWHOOP): `+0x42D |= 0x80` (bSirenOrAlarm). `+0x3A6 = nav`; `lane = rand() % (int16)lanes`; `+0x3B7 = +0x3B8 = lane`.
  19. Mad driver: `n = LaRiots ? 80 : (appearance 2 -> 50, 4 -> 10, else 200)`. If `!isBoat && type != 13 && !gangZone` and
      (`GetRandomNumberInRange(0, n) == 0` or cheat 31): madDriver = 1, `frac = 1.0f` (0x430909-0x43093B).
Block G - position on the link (0x430943-0x430FD5):
  20. `half = (bbox.max.y - bbox.min.y) * 0.5f + 1.0f`; `len = sqrt((N1.y - N2.y)^2 + (N1.x - N2.x)^2)` (y first; 8 GetPosition calls,
      0x43097E-0x430A43). If `0.5f * len < half`: `frac = 0.5f`. Else `q = half / len`; `if (!(frac > q)) frac = q`; `if (!(frac < 1 - q))
      frac = 1 - q` (0x430A7D-0x430AA5).
  21. `+0x3B6 = (N1 < N2) ? -1 : 1` (0x430A62-0x430AAB).
  22. If `N1.numLinks == 1` -> delete veh, return (0x430AC7). Repeat `k = rand() % N1.numLinks; a = naviLinks[N1.area][N1.baseLink + k]` until
      `a != +0x3A6`; `+0x3A4 = a`; delete+return if `m_pPathNodes[a >> 10] == null`. `+0x3B5 = (nodeLinks[N1.area][N1.baseLink + k] < N1) ? -1 : 1`.
  23. Orientation: `d = pos(N2) - pos(N1)` (copy 3D at E+0xB8). 2D `l = sqrt(d.y*d.y + d.x*d.x)`; if `l == 0` (`test ah,0x44; jp`): `d2 =
      (1.0f, d.y)`; else `d2 = (d.x * (1/l), d.y * (1/l))`. 3D copy `.Normalise()`. Matrix (pointer re-read per store, unchecked): `fwd = d3n`,
      `right = (d2.y, -d2.x, 0)`, `up = (0, 0, 1.0f)` (0x430C26-0x430C79).
  24. frac2 (0x430C80-0x430FD5), with `Ln` = next link (+0x3A6), `Lc` = current link (+0x3A4), `Nc` = node +0x390, `Nn` = node +0x394, all
      2D magnitudes `sqrt(x*x + y*y)`: `dA = |Ln.coors - Nc|`, `dB = |Ln.coors - Nn|`. If `dA / (dB + dA) > frac` (strict): `dC = |Lc.coors - Nc|`,
      `e = |coors - Nc|`, `frac2 = (e + dC) / (dC + dA)`. Else: `PickNextNodeRandomly(veh)`, re-read Nc, Ln, Lc; `v = Ln.coors - Nc`,
      `dC = |Lc.coors - Nc|`, `e = |coors - Nc|`, `frac2 = (dC - e) / (|v| + dC)` (0x430E62-0x430FD3). Clamp: `if (0 > frac2) frac2 = 0; else if
      (1.0f < frac2) frac2 = 1.0f` (NaN kept).
Block H - curve setup (0x43100D-0x4315CB):
  25. `D1 = (Lc.dir * 0.01f * s1)`, `D2 = (Ln.dir * 0.01f * s2)` (s = (float)(i8)+0x3B5 / +0x3B6). `lo1 = (Lc.OneWayLaneOffset() + curLane) * 5.4f`,
      `lo2 = (Ln.OneWayLaneOffset() + nextLane) * 5.4f`, both `+= 1.458f` if `subType == 10`.
  26. `+0x3D1 = (Nn +0x19 >> 4) & 3`; `m = FindSpeedMultiplierWithSpeedFromNodes(that)`; `+0x3D4 = m` (m_SpeedMult); `+0x3C8 = (float)cruise * m` (m_speed).
  27. `End = (Ln.x*0.125f + lo2*D2.y, Ln.y*0.125f - lo2*D2.x, 0)`, `Start = (Lc.x*0.125f + lo1*D1.y, Lc.y*0.125f - lo1*D1.x, 0)` (products stored
      f32 first). `ss = ftol(CalcSpeedScaleFactor(&Start, &End, D1.x, D1.y, D2.x, D2.y) * (1000.0f / m_speed))` (D recomputed identically);
      `+0x3A0 = ss`; `+0x39C = ftol((float)(u32)now - frac2 * (float)ss)`.
  28. `t = (float)(u32)(now - +0x39C) / (float)ss`; `CalcCurvePoint(&Start, &End, &(D1,0), &(D2,0), t, ss, &coors, &speed)` (Start/End recomputed
      identically) - `coors` is overwritten with the curve point.
Block I - final position (0x4315D0-0x431866):
  29. `dir = pos(N1) - pos(N2)`; `P = coors + dir * (2.0f / |dir|)` (0x40FEC0 then 0x40FE30); `P.z = frac * pos(N2).z + (1 - frac) * pos(N1).z`
      (uses `frac`, not frac2).
  30. `g = 1e9f`. Boat: `if (!GetWaterLevel(P.x, P.y, P.z, &g', true, null))` delete+return (0x431B2D); `g = g'`. Land: `ProcessVerticalLine(P, 1000.0f,
      &cp, &ent, 1, 0, 0, 0, 1, 0, 0)`: hit -> `g = cp.z`; then `ProcessVerticalLine(P, -1000.0f, ...same flags)`: hit and `|cp.z - P.z| < |g - P.z|`
      -> `g = cp.z`.
  31. Delete+return if `g == 1e9f` (0x4317EE) or `|g - P.z| > 7.0f` (0x431819). `P.z = IsBoatModel(model) ? g : g + veh->GetHeightAboveRoad()`;
      boats also set `w[+0x4A6] = 0xFF`. `SetPosn(&P)`; `m_vecMoveSpeed = 0`.
  32. `rel = speed * (1/60.0f) (0x4119D0) - S`; `cp2 = (coors.x - C.x, coors.y - C.y)` (stored over S.x/S.y). `type` reloaded into ebx.
  33. Status (byte +0x36 bits 3-7): type 13 -> mission == 1 ? 2 : 3; type 24 -> 3; isBoat -> 3; else 2 unless already 3.
  34. `SetClumpAlpha(+0x18 clump, 0)`. If cheat 70 and `+0x590 == 0`: `AddVehicleUpgrade(MI_HYDRAULICS)`.
Block J - visibility and acceptance (0x43199D-0x431BE6); `k = TheCamera+0xF4`, `vmax = max(170.0f, (float)(int16)+0x4A6)`, `dp` = 2D distance
  from C to the vehicle position:
  35. On screen: delete if `vmax * k < dp`, delete if `k * 150.0f > dp`, delete (0x431F28) if `k * 120.0f > |camPos - vehPos|` (2D), or topDown,
      or model == 0x1E4 (484 MARQUIS). Off screen: if `vmax * 0.005882353f * 45.0f < dp` (`test ah,5; jp` keeps on `>=`/NaN): delete unless topDown.
  36. `FindObjectsKindaColliding(vehPos, colModel boundSphere radius (+0x24), true, &cnt, 2, null, 0, 1, 1, 0, 0)`; delete if `cnt != 0`.
      Delete unless `rel.x * cp2.x + rel.y * cp2.y < 0.0f` (approaching the player; 0x431B8E-0x431BB1).
  37. `ChooseVehicleColour(mi, +0x434, +0x435, +0x436, +0x437, 1)`; `CWorld::Add(veh)`.
Block K - post-add (0x431BEB-0x431F1D):
  38. Model 0x213 (531 TRACTOR) / 0x214 (532 COMBINE) or subType 10: `cruise /= 3` (signed). LaRiots: `m_fHealth (+0x4C0) = (float)(rand() % 1000)`.
      type 13: `LastTimeLawEnforcerCreated = now`. Model 0x1C9 (457 CADDY): status 3, style 2.
  39. If `+0x590 == 0` and `type <= 23` (table 0x431F5C/0x431F68): types 0, 4, 5, 6, 13 -> `if (Rand(0, 20) == 0) SetRandomDamage(false)`;
      types 1, 14..23 -> `if (Rand(0, 8) == 0) SetRandomDamage(true)`; types 2, 3, 7..12 -> nothing.
  40. If `subType == 9` (BIKE) and style == 0: status 3, style 6.
  41. Occupants:
      - If `!isBoat && type != 13 && FindPlayerPed(-1)->GetWantedLevel() == 0` and (cheat 31 or `TimeNextMadDriverChaseCreated <= 0.0f`) and `!gangZone`
        and `CreatePoliceChase(veh, type, N1)`: `TimeNextMadDriverChaseCreated = LaRiots ? Rand(240, 480) : Rand(600, 1200)`; go to 42.
      - Else if madDriver: if model in {0x1CF FREEWAY, 0x1CD PCJ600, 0x209 FCR900, 0x20A NRG500, 0x245 BF400, 0x24A WAYFARER} and `!gbLARiots` and
        `Rand(0, 7) == 0` and `CreateConvoy(veh, type)`: `SetUpDriverAndPassengersForVehicle(veh, type, 1, 1, 0, 99)`. Otherwise:
        `SetUpDriverAndPassengersForVehicle(veh, type, 1, 1, 0, 99)`, status 3, style 2, `cruise = ftol((float)cruise + 10.0f)`,
        `m_vecMoveSpeed = (GetTopDirection() * (float)cruise) * 0.02f` (two 0x40FEC0 calls, each stored f32); if (LaRiots or cheat 31) and
        driver (+0x460): `driver->+0x478 |= 0x800`; `+0x42E |= 8` (bMadDriver).
      - Else type 13 or 24: `AddPoliceCarOccupants(veh, false)`; other types: `bCarIsBeingCreated = 1; SetUpDriverAndPassengersForVehicle(veh, type,
        0, 0, 0, 99); bCarIsBeingCreated = 0`.
  42. type 13 or 24: `ChangeLawEnforcerState(1)`. `PossiblyStreamCarOutAfterCreation(model)`; model-info use counter (0x421120). Return.
- Original bugs / oddities (keep): no null check after GetNewVehicleDependingOnCarModel (16); step 13 reads a navi link one past the node's links
  when N2 is not linked; step 24 uses the 2D creation point `coors` from GenerateCarCreationCoors2 but z from `frac` (29); the speed vector
  computed in step 28 is only used for the approach test, the vehicle starts with zero speed unless it is a mad driver.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:470). Open questions: (1) exact gr names of TheCamera +0xDC/+0xE0/+0xF4 and of vehicle
  +0x4A6 (i16, compared against 170 as a distance in step 35) - behaviour does not depend on them; (2) CPathNode +0x18 bit 7 is used as
  "water node" per gr's m_bWaterNode (0x4304DA).

## 0x432420 CCarCtrl::SlowCarDownForCarsSectorList<CPtrListDoubleLink<CPhysical*>>  (catalogue: gr-unreversed, port level 4)
- Convention / signature: `__cdecl void (CPtrListDoubleLink* list [esp+4], CVehicle* veh [+8], float minX [+0xC], float minY [+0x10],
  float maxX [+0x14], float maxY [+0x18], float* pSpeed [+0x1C], float cruise [+0x20])`, plain `ret`. Same frame as the objects
  variant 0x42D4F0 (CCarCtrl.p2.md): `sub esp,0x18` + 4 pushes, E = esp after the pushes, bound centre C at [E+0x10..0x18], top
  direction temp [E+0x1C..0x24]; the list arg slot [E+0x2C] is reused for `d`.
- Callers: CCarCtrl::FindMaximumSpeedForThisCarInTraffic 0x434400 (1). Callees: CEntity::GetBoundCentre 0x534250 (thiscall (CVector* out)),
  CCollision::DistAlongLine2D 0x412A80 (cdecl (px, py, dirX, dirY, qx, qy) -> ST0 = `(qx - px)*dirX + (qy - py)*dirY`),
  CPlaceable::GetTopDirection 0x41CCB0 (thiscall (CVector* out) -> EAX), CCarCtrl::SlowCarDownForOtherCar 0x42D0E0 (cdecl (CEntity* other,
  CVehicle* veh, float* pSpeed, float cruise)).
- Globals: CWorld::ms_nCurrentScanCode 0xB7CD78 (u16). Constants (f32): `0x858B50 = 0.0`, `0x85862C = 10.0`, `0x858B3C = 3.0`.
- Fields: list node +0 item, +4 next; entity +0x1C flags dword bit 0 (gr m_bUsesCollision), +0x2C m_nScanCode (u16); veh +0x14 m_matrix
  (fwd.x +0x10, fwd.y +0x14, pos +0x30), +0x04 placement pos.
- Behaviour: `for (node = list->head; node; node = next)` (`next = node->next` read before processing, 0x432444):
  1. `e = node->item`; skip if `e == veh` (0x432447), skip if `e->m_nScanCode == ms_nCurrentScanCode` (0x432457), skip if
     `!(e->+0x1C & 1)` (0x432461); only then `e->m_nScanCode = ms_nCurrentScanCode` (0x43246E; entities without collision are NOT marked).
  2. `C = e->GetBoundCentre()` (0x432472). Require `C.x > minX` (`test ah,0x41; jne` skip, 0x432484), `C.x < maxX` (`test ah,5; jp`,
     0x432497), `C.y > minY` (0x4324AA), `C.y < maxY` (0x4324BD); NaN fails all.
  3. `t = C.z - V.z` (V = veh pos, matrix +0x30 or placement +4, re-read for every use); `|t|` computed as `t < 0 ? -(C.z - V.z) : (C.z - V.z)`
     (recomputed, 0x4324D0-0x432504; NaN takes the non-negated branch); require `|t| < 10.0f` (strict, 0x432512).
  4. `d = DistAlongLine2D(V.x, V.y, vm.fwd.x, vm.fwd.y, C.x, C.y)` (0x432542; vm = veh->m_matrix read raw: a null matrix reads
     address 0x10/0x14 -> crash, 0x432538/0x43253B), stored f32 in the list arg slot.
  5. `P = veh pos`, `F = veh->GetTopDirection()` (0x432562); `h = C.z - (d * F.z + P.z)` (`fld d; fmul F.z; fadd P.z; fsubr C.z`,
     0x432567-0x432571); `|h|` by `fcom 0.0; test ah,5; jp` (negated when h < 0, 0x432575-0x432582); require `|h| < 3.0f` (strict,
     0x43258F).
  6. `SlowCarDownForOtherCar(e, veh, pSpeed, cruise)` (0x43259D).
- Edge cases: bounds strict, NaN rejected; scan code is written only for entities with collision (differs from the objects variant,
  which marks before its model filter).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:869-871). Open questions: none.

## 0x4325C0 CCarCtrl::FindAngleToWeaveThroughTraffic  (catalogue: gr-unreversed, port level 4)
- Convention / signature: `__cdecl float (CVehicle* veh [esp+4], CPhysical* exclude [+8], float targetAngle [+0xC], float otherAngle [+0x10],
  float distScale [+0x14])` (gr: `(CVehicle*, CPhysical*, float arg3, float arg4, float arg5)`), returns ST0, plain `ret`.
  Frame `sub esp,0x2C` + ebx/ebp/esi/edi; E = esp after the pushes (args at E+0x40..E+0x50). Locals: minX E+0x18 (later the x counter),
  minY E+0x14, maxX E+0x28 (later the y counter), maxY E+0x10, xMin E+0x20/esi, yMin E+0x34, xMax E+0x1C, yMax E+0x24, prevR E+0x30,
  prevL E+0x2C. Arg slots reused: E+0x40 (veh slot) = angleR, E+0x50 (distScale slot) = angleL; targetAngle slot E+0x48 = mid angle at the end.
- Callers: SteerAICarWithPhysicsFollowPreRecordedPath 0x432DD0, SteerAICarWithPhysicsHeadingForTarget 0x433280, SteerAICarTowardsPointInEscort
  0x4336D0, SteerAICarWithPhysicsFollowPath 0x434900, SteerAICarWithPhysicsFollowPath_Racing 0x435830.
- Callees: CRT floor 0x8219F0 / `_ftol` 0x821B40 (8 each), CWorld::ClearScanCodes 0x563470, WeaveThroughCarsSectorList 0x42D680 (2 sites),
  WeaveThroughPedsSectorList 0x42D7E0, WeaveThroughObjectsSectorList 0x42D950 (signatures in CCarCtrl.p2.md; the two angle pointers are
  passed as `(&angleL, &angleR)`, i.e. angleL is the callee's "angleA"/left pointer, decremented by WeaveForOtherCar's left sweep).
- Globals: CWorld::ms_nCurrentScanCode 0xB7CD78, CWorld::ms_aRepeatSectors 0xB992B8 (stride 12). Constants (f32): `0x858FA0 = 2.5`,
  `0x858624 = 1.0`, `0x858CA0 = 2.0`, `0x858CCC = 12.0`, `0x858B38 = 0.019999999552965164`, `0x858B34 = 60.0`,
  `0x858CC0 = -3.1415927410125732`, `0x858CB8 = 3.1415927410125732`, `0x858CBC = 6.2831854820251465`, `0x858B8C = 0.5`, `0x858B50 = 0.0`,
  `0x858FE4 = 1.5707963705062866`, `0x859018 = 0.07999999821186066`; immediate 0xC61C3F9A = -9999.900390625f; 0x77 = 119.
- Fields: veh +0x44/+0x48 m_vecMoveSpeed.x/.y, +0x14/+0x04 position, +0x3B9 m_nCarDrivingStyle.
- Behaviour:
  1. `s = sqrt(mv.x*mv.x + mv.y*mv.y)` (x term first, 0x4325CB-0x4325DB); `k = s * 2.5f + 1.0f`; if `!(k < 2.0f)` (incl. NaN) `k = 2.0f`
     (0x4325ED-0x4325FC). `R = k * distScale * 12.0f` (x87, kept in ST0, 0x432613-0x432617).
  2. P = veh pos (re-read per use): `minX = P.x - R`, `maxX = R + P.x`, `minY = P.y - R`, `maxY = R + P.y` (stored f32, 0x43261D-0x43265A).
  3. Sector bounds exactly as FindMaximumSpeedForThisCarInTraffic step 3 (`s(v) = v * 0.02f + 60.0f`, floor + ftol twice for kept values,
     min clamp 0 for minX/minY via `jle`, max clamp 119 for maxX/maxY via `jge`; 0x43265E-0x432790).
  4. Scan code: `>= 0xFFFF` -> ClearScanCodes + `= 1`, else `++` (0x432794-0x4327BB). Done ONCE, before the loop below.
  5. `angleR = angleL = targetAngle` (0x43279D-0x4327A7); `prevR = prevL = -9999.9f`.
  6. Repeat (0x4327E0): if `prevL == angleL` and `prevR == angleR` (fcomp + `test ah,0x44`; unordered counts as not equal) leave the loop.
     Else `prevR = angleR`, `prevL = angleL` and scan `for (y = yMin; y <= yMax; y++) for (x = xMin; x <= xMax; x++)` (signed), with
     `R = &ms_aRepeatSectors[(y & 0xF) * 16 + (x & 0xF)]`:
     a. `WeaveThroughCarsSectorList(&R->vehicles, veh, exclude, minX, minY, maxX, maxY, &angleL, &angleR)` (0x432870);
     b. if drivingStyle (re-read) != 6: `WeaveThroughPedsSectorList(&R->peds, veh, exclude, ...same...)` (0x4328A2);
     c. `WeaveThroughObjectsSectorList(&R->objects, veh, minX, minY, maxX, maxY, &angleL, &angleR)` (0x4328C5);
     d. `WeaveThroughCarsSectorList(&R->vehicles, ...same as a...)` AGAIN (0x4328E7).
  7. Result selection (0x432922 onwards; all x87, values reloaded from the f32 slots):
     a. `d = otherAngle - targetAngle`; `while (d < -pi) d += 2pi` (test first, `test ah,5; jp` exits on >= / NaN); `while (d > pi) d -= 2pi`
        (`test ah,0x41; jne` exits on <= / NaN).
     b. `m = d * 0.5f + targetAngle`, stored f32 to the targetAngle slot; normalised with the same two loops (stored again after each loop
        that ran).
     c. `dL = angleL - m` (`fsubr`), `dR = angleR - m`; dL normalised (same loops) then `dL = |dL|` (negated when `< 0`); then dR normalised
        and `dR = |dR|` (0x4329D7-0x432AA0).
     d. If `dL > pi/2` and `dR > pi/2` (`test ah,0x41; jne` -> either `<=` or NaN goes on): return `m` (reloaded f32, 0x432AC2).
     e. If `|dL - dR| < 0.08f` (0x432ADD; NaN fails): return `angleR` (0x432AEE).
     f. If `dL < dR` (0x432AF8 `fcomp st(1)` with st0 = dL; `test ah,5; jp` -> `>=`/NaN returns angleR): return `angleL` (0x432B03); else
        return `angleR`.
- Edge cases / oddities (keep):
  - The scan code is set once (step 4), and the sector-list walkers mark every entity with collision they visit, so the second pass of
    step 6 skips everything already seen and normally changes nothing; the loop therefore normally runs exactly two passes (the second one
    a full no-op walk). Step 6d's second vehicles-list call per cell is likewise a no-op for already marked entities.
  - Normalisation loops are unbounded `while` loops: an infinite angle hangs (inf + 2pi = inf).
  - Returned angles are not normalised (angleL/angleR are whatever the weave functions stored).
- gta-reversed: `.Reversed = false`, `plugin::CallAndReturn` only. Open questions: none.

## 0x432B10 CCarCtrl::PickNextNodeAccordingStrategy  (catalogue: gr-unreversed, port level 7)
- Convention / signature: `__cdecl bool (CVehicle* veh [esp+4])`, AL, plain `ret`. Frame `sub esp,0x24` + esi/edi (scratch CVector
  buffers for FindPlayerCoors). The veh arg slot is overwritten with the node address (0x432B34, harmless).
- Callers: SteerAICarWithPhysicsFollowPath 0x434900, UpdateCarOnRails 0x436540.
- Callees: CCarCtrl::PickNextNodeToChaseCar 0x426EF0 (2 sites), PickNextNodeToFollowPath 0x427740 (cdecl (veh) -> AL),
  PickNextNodeRandomly 0x42DE80 (cdecl (veh), result ignored), CWorld::FindPlayerCoors 0x56E010 (cdecl (CVector* out, int32 player = -1)
  -> EAX = out; 3 calls), CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (-1, false)).
- Globals: ThePaths.m_pPathNodes[] 0x96F854. Jump table 0x432C54 = {0x432B94, 0x432C31, 0x432BE7, 0x432C40}, byte index table 0x432C64
  (67 entries for mission-2 = 0..0x42; read from the image).
- Fields: veh +0x394 m_autoPilot.m_startingRouteNode (CNodeAddress, used as "next node"), +0x3BA m_nCarMission (read as i8),
  +0x3D1 m_autoPilot.field_41 (gr unnamed byte; written here), +0x41C m_autoPilot.m_TargetEntity. Node +0x19 byte bits 4-5 (gr
  m_bNotHighway, m_bHighway), +0x1A bits 4-7 (m_nBehaviourType).
- Behaviour:
  1. `N = m_pPathNodes[next.area] + next.node * 0x1C` (no validity check of the address or the area pointer, 0x432B18-0x432B30).
     `veh->+0x3D1 = (N.+0x19 >> 4) & 3` (0x432B4F). If `(N.+0x1A >> 4)` is 8 or 9 (unsigned, 0x432B63-0x432B6A): `veh->+0x3D1 = 0xFF`.
  2. `m = (int8)mission - 2`; if `(uint32)m > 0x42` -> default (0x432B80, also for missions 0, 1 and negative values).
  3. Cases:
     - Missions 2, 4, 0x2B (RAMPLAYER_FARAWAY, BLOCKPLAYER_FARAWAY, APPROACHPLAYER_FARAWAY) -> 0x432B94: `pA = FindPlayerCoors(&a, -1)`,
       `pv = FindPlayerVehicle(-1, false)`, `z = FindPlayerCoors(&b, -1)->z`, `y = FindPlayerCoors(&c, -1)->y`, then
       `return PickNextNodeToChaseCar(veh, pA->x, y, z, pv)` (five dwords pushed, caller cleans 0x14, 0x432BD9).
     - Missions 8, 0xC (GOTOCOORDINATES, GOTOCOORDINATES_ACCURATE) -> 0x432C31: `return PickNextNodeToFollowPath(veh)`.
     - Missions 0xF, 0x11, 0x34, 0x36, 0x3D, 0x41, 0x42, 0x43, 0x44 (RAMCAR_FARAWAY, BLOCKCAR_FARAWAY, FOLLOWCAR_FARAWAY, KILLPED_FARAWAY,
       DO_DRIVEBY_FARAWAY, ESCORT_*_FARAWAY) -> 0x432BE7: `t = veh->m_TargetEntity` (no null check); `return PickNextNodeToChaseCar(veh,
       T.x, T.y, T.z, t)` (T = t's matrix pos or placement pos, evaluated three times, 0x432BE7-0x432C23).
     - All other values -> 0x432C40: `PickNextNodeRandomly(veh)`; return 0.
- Edge cases: the chase calls pass a FIFTH argument (player vehicle / target entity) that gr's 4-argument declaration of
  PickNextNodeToChaseCar (CarCtrl.h:98) does not have; CCarCtrl.md 0x426EF0 also lists 4 args. Port: pass it (cdecl, harmless if the
  callee ignores it). Step 1 dereferences the node table with whatever is in +0x394 (an invalid area 0xFFFF reads table index 0xFFFF).
- gta-reversed: `plugin::CallAndReturn` only (CarCtrl.cpp:672). Open questions: whether 0x426EF0 reads the fifth argument ([esp+0x14]
  at entry) - to be settled in that function's spec, not here.

## 0x432DD0 CCarCtrl::SteerAICarWithPhysicsFollowPreRecordedPath  (catalogue: gr-unreversed, port level 10)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], float* pSteer [+8], float* pGas [+0xC], float* pBrake [+0x10], bool* pHandbrake [+0x14])`,
  plain `ret`. Frame `sub esp,0x1C` + esi/edi (E = esp after these; args at E+0x28..E+0x38), later ebx (= frame pointer into the buffer) and ebp
  (= &PlaybackIndex[rid]). Locals: P0.y/P0.z E+0x10/+0x14 (later own speed at E+0x08), P1.y/P1.z E+0x1C/+0x20, angTarget E+0x08. The veh arg slot
  E+0x28 holds rid, then angHeading, then steer.
- Callers: SteerAICarWithPhysics_OnlyMission 0x436A90. Callees: CGeneral::GetATanOfXY 0x53CC70 (x2), FindAngleToWeaveThroughTraffic 0x4325C0,
  FindMaxSteerAngle 0x427FE0, CVehicleRecording::StopPlaybackWithIndex 0x459440 (cdecl (int32)).
- Globals (indexed by rid): CVehicleRecording::pPlaybackBuffer 0x97D800 (u8* per slot), PlaybackIndex 0x97D7C0 (byte offset, u32), PlaybackBufferSize
  0x97D780 (u32 bytes), PlaybackSpeed 0x97D700 (f32), bPlaybackPaused 0x97D6D0 (u8). Frame record (gr CVehicleStateEachFrame) = 0x20 bytes:
  +0x04/+0x06 i16 velocity x/y, +0x14/+0x18/+0x1C f32 position.
- Constants (f32): `0x85862C = 10.0`, `0x858CC0 = -3.1415927410125732`, `0x858CBC = 6.2831854820251465`, `0x858CB8 = 3.1415927410125732`,
  `0x858EAC = 6.103701889514923e-05` (1/16383), `0x858B34 = 60.0`, `0x858C80 = 5.0`, `0x858B50 = 0.0`, `0x858CA0 = 2.0`, `0x858C84 = 0.25`,
  `0x858624 = 1.0`, `0x858C48 = 0.125`, `0x85901C = -0.05000000074505806`, `0x858B8C = 0.5`; immediates 2.0f, 0x20, 0x320 = 800.
- Fields: veh +0x424 = autopilot m_vehicleRecordingId (i8), +0x3BA mission, +0x14/+0x04 position, +0x44/+0x48 move speed.
- Behaviour:
  1. `rid = (int8)veh->+0x424`. If `rid < 0` or `pPlaybackBuffer[rid] == null`: `*pBrake = 0, *pGas = 0, *pSteer = 0, *pHandbrake = 0`, mission = 0xB,
     return (0x432DE8 / 0x432E25).
  2. If `bPlaybackPaused[rid]`: `*pGas = 0, *pSteer = 0, *pBrake = 0.5f, *pHandbrake = 0`, return (mission unchanged).
  3. `F = buffer + PlaybackIndex[rid]` (kept in EBX for the rest of the function). `d0 = |V - F.pos|`, `d1 = |V - (F+1).pos|` (3D; per component
     `V.c - P.c`, sum `(dz*dz + dy*dy) + dx*dx`, 0x432EAE-0x432F2A).
  4. Advance loop: while `d0 < 10.0f` or `d1 < d0` (`test ah,5; jnp`, 0x432F39 / 0x432F48): `PlaybackIndex[rid] += 0x20`; if the new index
     `>= PlaybackBufferSize[rid] - 0x20` (unsigned): `StopPlaybackWithIndex(rid)`, `veh->+0x424 = -1`, `*pBrake = *pGas = *pSteer = 0`,
     `*pHandbrake = 0`, mission = 0xB, return (0x433233). Else recompute d0/d1 from the NEW index (0x43303B-0x4330B8) and test again.
     EBX (F) is NOT updated by the loop: steps 5-7 use the frame that was current on entry (one-frame lag, original behaviour; keep).
  5. `angHeading = GetATanOfXY(m.fwd.x, m.fwd.y)` (raw, unnormalised forward; matrix not null-checked), `angT = GetATanOfXY(F.pos.x - V.x,
     F.pos.y - V.y)`; `steer = FindAngleToWeaveThroughTraffic(veh, null, angT, angHeading, 2.0f) - angHeading` (always weaves, distScale 2.0f),
     normalised into [-pi, pi] and clamped to `+-FindMaxSteerAngle(veh)` (patterns as 0x433280 steps 3 and 5).
  6. `vx = F.+4 * k`, `vy = F.+6 * k` (k = 1/16383; vy computed first); `target = sqrt(vx*vx + vy*vy) * PlaybackSpeed[rid] * 60.0f`. If
     `PlaybackIndex[rid] <= 0x320` (unsigned; the possibly advanced index) and `!(target > 5.0f)`: `target = 5.0f`.
  7. `own = sqrt(mv.x*mv.x + mv.y*mv.y) * 60.0f` (stored); `*pBrake = 0`; `diff = target - own`. If `diff > 0`: `*pGas = min1(diff * (own < 2.0f ?
     0.25f : 0.125f))` with `min1(g) = (1.0f < g) ? 1.0f : g`. Else `*pGas = 0`, `b = diff * -0.05f`, `*pBrake = (0.5f < b) ? 0.5f : b`.
     `*pSteer = steer`, `*pHandbrake = 0`.
- gta-reversed: `.Reversed = false`, plugin call only. Open questions: none.

## 0x433280 CCarCtrl::SteerAICarWithPhysicsHeadingForTarget  (catalogue: gr-unreversed, port level 5)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], CPhysical* exclude [+8], float x [+0xC], float y [+0x10], float* pSteer [+0x14],
  float* pGas [+0x18], float* pBrake [+0x1C], bool* pHandbrake [+0x20])` (gr: `(CVehicle*, CPhysical* target, float, float, float*, float*,
  float*, bool*)`), plain `ret`. Frame `sub esp,0x10` + esi/edi; E = esp after both pushes (args at E+0x1C..E+0x38, locals E+0x08..E+0x14).
  Arg slots reused: E+0x1C (veh) = fwdN.x then angHeading; E+0x38 (pHandbrake, kept in EDI) = fwdN.y then steer; E+0x24 (x) = cruise int;
  E+0x28 (y) = speed. Locals: angTarget E+0x08, move speed copy E+0x0C..E+0x14.
- Callers: SteerAICarWithPhysicsTryingToBlockTarget 0x4335E0, SteerAICarParkParallel 0x433BA0, SteerAICarParkPerpendicular 0x433EA0,
  SteerAICarWithPhysicsFollowPath 0x434900 (4), SteerAICarWithPhysicsFollowPath_Racing 0x435830, SteerAICarWithPhysics_OnlyMission 0x436A90 (8).
- Callees: CGeneral::GetATanOfXY 0x53CC70 (x3), FindAngleToWeaveThroughTraffic 0x4325C0, FindMaxSteerAngle 0x427FE0, FindSpeedMultiplier 0x4224E0.
- Constants (f32): `0x858B50 = 0.0`, `0x858624 = 1.0`, `0x858CC0 = -3.1415927410125732`, `0x858CBC = 6.2831854820251465`,
  `0x858CB8 = 3.1415927410125732`, `0x858C24 = 0.30000001192092896`, `0x858CB0 = 0.699999988079071`, `0x858B34 = 60.0`, `0x858FE8 = 25.0`,
  `0x858B1C = 0.10000000149011612`, `0x858B3C = 3.0`, `0x85901C = -0.05000000074505806`, `0x858B8C = 0.5`; immediates 0.4f, 1.2f, 1.0f, 10.0f.
- Fields: veh +0x14 m_matrix (fwd +0x10/+0x14; dereferenced without null check at 0x433290), +0x04, +0x44..+0x4C m_vecMoveSpeed, +0x3B9 style,
  +0x3D0 cruise (u8), +0x594 subType, +0x818 (CBmx, see 0x434900).
- Behaviour:
  1. `*pHandbrake = 0` (0x43328D). Forward normalisation exactly as SteerAICarWithPhysicsFollowPath step 2 (`len = sqrt(fy*fy + fx*fx)`;
     `len == 0` -> `fwdN = (1.0f, fy)`).
  2. `angTarget = GetATanOfXY(x - V.x, y - V.y)` (V = matrix pos or placement pos), `angHeading = GetATanOfXY(fwdN.x, fwdN.y)`.
     `a = (style in {2,5,6}) ? FindAngleToWeaveThroughTraffic(veh, exclude, angTarget, angHeading, 1.0f) : angTarget`.
  3. `steer = a - angHeading` normalised into [-pi, pi] (`while (< -pi) += 2pi; while (> pi) -= 2pi`, stored f32, 0x43335B-0x4333B3).
  4. `sp = sqrt((mv.x*mv.x + mv.y*mv.y) + mv.z*mv.z)` (0x4333B7-0x4333D0). If `sp > 0.3f` (`test ah,0x41; jne`) and `|steer| > 0.7f`
     (|x| via `fcom 0; jp; fchs` on the x87 copy of steer): `*pHandbrake = 1` (0x433401).
  5. `mx = FindMaxSteerAngle(veh)`; `if (steer < -mx) steer = -mx; if (steer > mx) steer = mx` (stored f32, NaN unchanged).
  6. `m = FindSpeedMultiplier(GetATanOfXY(x - V.x, y - V.y) - angHeading, 0.4f, 1.2f, 0.4f)` (angle recomputed, unnormalised difference).
  7. `*pBrake = 0`. `spd = sqrt((mv.z*mv.z + mv.y*mv.y) + mv.x*mv.x) * 60.0f` (stored f32; note the different sum order from step 4).
     `diff = m * (int)cruise (fimul) - spd` (x87).
  8. If `diff > 0`: `*pGas = (spd < 25.0f) ? ((1.0f < diff * 0.1f) ? 1.0f : diff * 0.1f) : 1.0f` (`spd >= 25` or NaN -> 1.0f). If `subType == 10`
     and `diff > 3.0f` and `+0x818 <= 0.0f`: `+0x818 = 10.0f`. Else (`diff <= 0` or NaN): `*pGas = 0`, `b = diff * -0.05f`,
     `*pBrake = (0.5f < b) ? 0.5f : b`.
  9. `*pSteer = steer` (all paths, 0x433580 / 0x4335BA / 0x4335CC).
- Edge cases: cruise 0 gives `diff = -spd` (brakes). The function never touches the autopilot.
- gta-reversed: `.Reversed = false`, plugin call only. Open questions: none.

## 0x4335E0 CCarCtrl::SteerAICarWithPhysicsTryingToBlockTarget  (catalogue: gr-unreversed, port level 6)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], CEntity* unused [+8], float tx [+0xC], float ty [+0x10], float tvx [+0x14],
  float tvy [+0x18], float* pSteer [+0x1C], float* pGas [+0x20], float* pBrake [+0x24], bool* pHandbrake [+0x28])` (gr: same 10 args,
  `Unusued` entity - it is indeed never read), plain `ret`. No frame; esi/edi pushed after the first float ops. The tvx/tvy arg slots receive
  the aim point X/Y.
- Callers: SteerAICarWithPhysics_OnlyMission 0x436A90 (2). Callees: SteerAICarWithPhysicsHeadingForTarget 0x433280.
- Constants (f32): `0x859020 = 0.12999999523162842`, `0x858B34 = 60.0`, `0x858FE8 = 25.0`.
- Behaviour:
  1. `len = sqrt(tvx*tvx + tvy*tvy)` (0x4335E0-0x4335F2). If `len > 0.13f` (`test ah,0x41; jne` keeps on `<=`/NaN): `k = 0.13f / len`,
     `vx = tvx * k`, `vy = tvy * k` (x87, not stored); else `vx = tvx`, `vy = tvy`.
  2. `X = vx * 60.0f + tx` (stored f32 in the tvx slot), `Y = vy * 60.0f + ty` (stored in the tvy slot) (0x433611-0x43364E).
  3. `veh->m_nCarDrivingStyle (+0x3B9) = 2` (0x433647, before the call). `SteerAICarWithPhysicsHeadingForTarget(veh, null, X, Y, pSteer, pGas,
     pBrake, pHandbrake)` (0x43365B).
  4. V = veh pos (matrix +0x30 or placement +4, read after the call). If `(Y - V.y)*(Y - V.y) + (X - V.x)*(X - V.x) < 25.0f` (strict, NaN no;
     0x433680-0x4336AB): `m_nCarMission (+0x3BA) = (mission == 0x12) ? 0x13 : 6` (BLOCKCAR_CLOSE -> BLOCKCAR_HANDBRAKESTOP, anything else ->
     BLOCKPLAYER_HANDBRAKESTOP; branchless `setne/dec/and 0xD/add 6`, 0x4336AD-0x4336BD).
- gta-reversed: `.Reversed = false`, plugin call only. Open questions: none.

## 0x4336D0 CCarCtrl::SteerAICarTowardsPointInEscort  (catalogue: gr-unreversed, port level 5)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], CVehicle* escorted [+8], float offX [+0xC], float offY [+0x10], float* pSteer [+0x14],
  float* pGas [+0x18], float* pBrake [+0x1C], bool* pHandbrake [+0x20])` (gr: `(CVehicle* vehicle1, CVehicle* vehicle2, float, float, float*, float*,
  float*, bool*)`), plain `ret`. Frame `sub esp,0x24` + ebx (= escorted) / esi (= veh); E = esp after both pushes (args at E+0x30..E+0x4C).
  Locals: T E+0x08/+0x0C, W.z E+0x10, A.x E+0x14 (first the offset vector O at E+0x14..+0x1C), product out E+0x20. Arg slots reused:
  E+0x34 = fwdN.x then `dist * 3.5f`; E+0x3C = fwdN.y then speeds; E+0x38 = angHeading then steer; E+0x4C = angA (after *pHandbrake was written).
- Callers: SteerAICarWithPhysics_OnlyMission 0x436A90. Callees: CMatrix::operator* 0x59C890 (cdecl (CVector* out, const CMatrix*, const CVector*),
  full transform with translation), CGeneral::GetATanOfXY 0x53CC70 (x3, one result discarded), FindAngleToWeaveThroughTraffic 0x4325C0,
  FindMaxSteerAngle 0x427FE0.
- Constants (f32): `0x858B50 = 0.0`, `0x858624 = 1.0`, `0x858B3C = 3.0`, `0x858CC0 = -3.1415927410125732`, `0x858CBC = 6.2831854820251465`,
  `0x858CB8 = 3.1415927410125732`, `0x858B8C = 0.5`, `0x858B34 = 60.0`, `0x858B48 = 15.0`, `0x858B1C = 0.10000000149011612`, `0x858B90 = 4.0`,
  `0x859028 = 3.5`, `0x858FD8 = 300.0`, `0x85862C = 10.0`, `0x859024 = -3.0`, `0x859000 = 8.0`, `0x858FE8 = 25.0`, `0x85901C = -0.05000000074505806`;
  immediates 1.0f, 0.1f (0x3DCCCCCD).
- Behaviour:
  1. `W = escorted->m_matrix * (offX, offY, 0)` (0x4336FF; escorted matrix not null-checked). `T.x = escorted.mv.x + W.x`, `T.y = escorted.mv.y + W.y`
     (raw move speed added, no scaling; stored f32). `*pHandbrake = 0` (0x43372C).
  2. Forward of veh normalised as in SteerAICarWithPhysicsFollowPath step 2 (`len == 0` -> `(1.0f, fy)`).
  3. `A.x = escorted.fwd.x * 3.0f + T.x` (stored), `A.y = escorted.fwd.y * 3.0f + T.y` (x87). `angA = GetATanOfXY(A.x - V.x, A.y - V.y)`,
     `angHeading = GetATanOfXY(fwdN.x, fwdN.y)`. `steer = FindAngleToWeaveThroughTraffic(veh, null, angA, angHeading, 1.0f) - angHeading` (always
     weaves, no driving-style test), normalised into [-pi, pi] and clamped to `+-FindMaxSteerAngle(veh)` exactly as in 0x433280 steps 3 and 5.
  4. `GetATanOfXY(T.x - V.x, T.y - V.y)` is called and its result discarded (0x4338D5-0x4338DD; keep the call, it has no side effects).
  5. `ahead = (T.y - V.y) * fwdN.y + (T.x - V.x) * fwdN.x` (x87); `dist = sqrt((T.y - V.y)^2 + (T.x - V.x)^2)` (stored f32 in E+0x34).
  6. If `ahead > 0.5f` (`test ah,0x41; jne`, 0x433961):
     - `tSpd = sqrt((emv.x^2 + emv.y^2) + emv.z^2) * 60.0f` (escorted move speed, stored).
     - If `dist >= 15.0f` (or NaN): `desired = 300.0f`.
     - Else: `x = (ahead - 0.5f) - 0.1f`; `y = (4.0f < x) ? 4.0f : x`; `k = dist * 3.5f` (stored); if `tSpd + y > k`: `desired = ((4.0f < x) ? 4.0f : x)
       + tSpd`, else `desired = k`.
     - `lim = tSpd + 10.0f` (stored); `if (!(desired < lim)) desired = lim`.
     Else (`ahead <= 0.5f`): if `dist >= 15.0f` (or NaN): `desired = 8.0f`; else: `*pSteer = 0`, `*pGas = 0`, `*pBrake = (ahead < -3.0f) ? 1.0f : 0.1f`,
     return (0x433A64-0x433AA4).
  7. `*pBrake = 0`; `sp = sqrt((mv.z*mv.z + mv.y*mv.y) + mv.x*mv.x) * 60.0f` (own speed, stored); `diff = desired - sp`.
     If `diff > 0`: `*pGas = (sp < 25.0f) ? ((1.0f < diff * 0.1f) ? 1.0f : diff * 0.1f) : 1.0f`. Else: `*pGas = 0`, `b = diff * -0.05f`,
     `*pBrake = (0.5f < b) ? 0.5f : b`. Finally `*pSteer = steer`.
- gta-reversed: `.Reversed = false`, plugin call only. Open questions: none.

## 0x433BA0 CCarCtrl::SteerAICarParkParallel  (catalogue: gr-unreversed, port level 6)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], float* pSteer [+8], float* pGas [+0xC], float* pBrake [+0x10], bool* pHandbrake [+0x14])`,
  plain `ret`. MSVC SEH frame (handler 0x8374E6; state slot set to 0 around `new`/constructor and -1 after SetTask) only to free the task if the
  constructor throws - no observable behaviour. Then `sub esp,0x18` + ebx/ebp/esi/edi.
- Callers: SteerAICarWithPhysics_OnlyMission 0x436A90. Callees: CPathNode::GetPosition 0x420A10, CVector::Normalise 0x59C910,
  SteerAICarWithPhysicsHeadingForTarget 0x433280, CTask::operator new 0x61A5A0 (cdecl (0x14)), CTaskComplexLeaveAnyCar::CTaskComplexLeaveAnyCar
  0x421150 (thiscall (int32 delay, bool sensible, bool forceGetOut); thunk to 0x15667C0: CTaskComplex ctor 0x61A3B0, vtable 0x858EB0, +0xC delay,
  +0x10 sensible, +0x11 forceGetOut), CTaskComplex::CTaskComplex 0x61A3B0, CTaskManager::SetTask 0x681AF0 (thiscall (CTask*, int32 slot, bool)).
- Constants (f32): `0x858B90 = 4.0`, `0x858CA0 = 2.0`.
- Fields: veh +0x390 / +0x394 autopilot current / next CNodeAddress, +0x3BA mission, +0x3D0 cruise, +0x428 vehicleFlags byte 0, +0x460 m_pDriver,
  +0x464..+0x480 m_apPassengers[8]; ped +0x47C m_pIntelligence (task manager at +4).
- Behaviour:
  1. If `m_pPathNodes[next.area] == null` or `m_pPathNodes[cur.area] == null` (next tested first, 0x433BD2 / 0x433BEA): `mission = 0xB` (STOP_FOREVER),
     `*pBrake = 0`, `*pGas = 0`, `*pSteer = 0` (in that order; handbrake untouched), return.
  2. Target T: if `mission == 0x2E` (PARK_PARALLEL): `d = pos(next) - pos(cur)` (components computed z, y, x), `d.Normalise()`, `T = d + pos(next)`
     (z first, each stored f32; 1 m beyond the next node). Else `T = pos(next)`.
  3. `SteerAICarWithPhysicsHeadingForTarget(veh, null, T.x, T.y, pSteer, pGas, pBrake, pHandbrake)`; `cruise = min(cruise, 8)` (u8).
  4. `dist = sqrt(dx*dx + dy*dy)` with `d = T - vehPos` (2D, x term first in the sum, 0x433D0C-0x433D2F).
     - mission 0x2E: if `dist < 4.0f` -> `mission = 0x32` (PARK_PARALLEL_2). Return.
     - other missions: if `dist < 2.0f`: `+0x428 &= 0xAF` (clears bEngineOn bit 4 and bLightsOn bit 6), `mission = 0xB`; if a driver exists:
       `driver->m_pIntelligence->m_TaskMgr.SetTask(new CTaskComplexLeaveAnyCar(0, true, false), 3, false)`; then for each of the 8 passenger
       slots in order, if non-null: the same with an inlined CTaskComplexLeaveAnyCar (vtable 0x858EB0, delay 0, sensible 1, forceGetOut 0).
       A null `new` result is passed to SetTask as null.
- Differences from SteerAICarParkPerpendicular: no bIsParked flag, the 4 m switch radius, and the target offset.
- gta-reversed: `.Reversed = false`, plugin call only. Open questions: none.

## 0x433EA0 CCarCtrl::SteerAICarParkPerpendicular  (catalogue: gr-unreversed, port level 6)
- Convention / signature: same as SteerAICarParkParallel (`__cdecl void (veh, float* pSteer, float* pGas, float* pBrake, bool* pHandbrake)`), SEH
  frame (handler 0x837506) + `sub esp,0x30`. E+0x50 (veh arg slot) is reused as scratch.
- Callers: SteerAICarWithPhysics_OnlyMission 0x436A90. Callees: as 0x433BA0 (without Normalise) plus CCollision::DistToMathematicalLine 0x412970
  (cdecl (const CVector* lineStart, const CVector* lineEnd, const CVector* point) -> ST0).
- Constants (f32): `0x858B44 = 6.0`, `0x858CA0 = 2.0`.
- Behaviour:
  1. Same invalid-area exit as 0x433BA0 step 1 (0x434188).
  2. If `mission == 0x2D` (PARK_PERPENDICULAR): `A = (pos(next).x, pos(next).y, 0)`, `B = (pos(cur).x, pos(cur).y, 0)`, `V = (vehPos.x, vehPos.y, 0)`
     (y read first for each); if `DistToMathematicalLine(&A, &B, &V) < 6.0f` -> `mission = 0x31` (PARK_PERPENDICULAR_2). Then `T = pos(cur)` if the
     mission is still 0x2D, else `T = pos(next)` (0x433FEC-0x434009). Other missions: `T = pos(next)`.
  3. `SteerAICarWithPhysicsHeadingForTarget(veh, null, T.x, T.y, pSteer, pGas, pBrake, pHandbrake)`; `cruise = min(cruise, 8)`.
  4. `dist = sqrt(dx*dx + dy*dy)`, `d = pos(next) - vehPos` (next node re-read, for every mission value). If `dist < 2.0f`: `+0x3DC |= 2` (autopilot
     movementFlags bIsParked), `+0x428 &= 0xAF`, `mission = 0xB`, then the same driver/passenger CTaskComplexLeaveAnyCar(0, true, false) tasks as
     0x433BA0 step 4.
- Edge case: in mission 0x2D the car steers toward the CURRENT node until it is within 6 m of the node line, but the 2 m stop test always uses
  the NEXT node.
- gta-reversed: `.Reversed = false`, plugin call only. Open questions: none.

## 0x4342A0 CCarCtrl::SetCoordsOfScriptCar  (catalogue: gr-unreversed, port level 9)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], float x [+8], float y [+0xC], float z [+0x10], bool clearOrientation [+0x14],
  bool addCentreOfMassOffset [+0x18])` (gr: `(CVehicle*, float, float, float, uint8 arg5, uint8 arg6)`; only the low bytes are used), plain
  `ret`. Frame `sub esp,0xC` + esi/edi; the z arg slot is updated in place.
- Callers: CStuckCarCheck::AttemptToWarpVehicle 0x463A60, CRunningScript::ProcessCommands100To199 0x466DE0, ProcessCommands2100To2199 0x470A90.
- Callees: CWorld::FindGroundZForCoord 0x569660 (cdecl (x, y) -> ST0), CEntity::GetDistanceFromCentreOfMassToBaseOfModel 0x536BE0 (thiscall -> ST0),
  virtual +0x10 CEntity::SetIsStatic (thiscall (false)), CStuckCarCheck::ClearStuckFlagForCar 0x463C40 (thiscall on CTheScripts::StuckCars
  0xA90AB0, (int32 handle)), virtual +0x38 CEntity::Teleport (thiscall (CVector by value, bool), callee cleans 0x10), CAutomobile::PlaceOnRoadProperly
  0x6AF420, CBike::PlaceOnRoadProperly 0x6BEEB0, CTheScripts::ClearSpaceForMissionEntity 0x486B00 (cdecl (const CVector*, CEntity*)),
  CCarCtrl::JoinCarWithRoadAccordingToMission 0x432CB0 (cdecl (veh)).
- Globals: CPools::ms_pVehiclePool 0xB74494. Constant `0x859014 = -100.0` (f32).
- Behaviour:
  1. `handle = (index << 8) + pool->byteMap[index]` with `index = (veh - pool->objects) / 0xA18` (signed magic divide 0x6572EC3, `sar 6`, 0x4342B9-0x4342DC;
     the CPool::GetRef idiom), computed before anything else.
  2. If `z <= -100.0f` (`fcomp`, `test ah,0x41; jp` -> only `<` or `==` take it; NaN does not): `z = FindGroundZForCoord(x, y)` (stored in the arg slot).
  3. If `addCentreOfMassOffset`: `z = veh->GetDistanceFromCentreOfMassToBaseOfModel() + z` (stored).
  4. `veh->SetIsStatic(false)`; `StuckCars.ClearStuckFlagForCar(handle)`.
  5. `pos = (x, y, z)`; `veh->Teleport(pos, clearOrientation)`.
  6. If `veh->m_nVehicleType (+0x590) == 5` (BOAT): `ClearSpaceForMissionEntity(&pos, veh)`; `m_nTempAction (+0x3BB) = 0`; return (no road join).
     Else: type 0 (AUTOMOBILE) or 11 (TRAILER) -> `CAutomobile::PlaceOnRoadProperly(veh)`; type 9 (BIKE) -> `CBike::PlaceOnRoadProperly(veh)`; other
     types nothing. Then `ClearSpaceForMissionEntity(&pos, veh)`, `JoinCarWithRoadAccordingToMission(veh)`, `m_nTempAction = 0`.
- Edge cases: `pos.z` passed to ClearSpaceForMissionEntity is the adjusted z, not the post-PlaceOnRoadProperly height.
- gta-reversed: `.Reversed = false`, plugin call only. Open questions: none.

## 0x434400 CCarCtrl::FindMaximumSpeedForThisCarInTraffic  (catalogue: gr-unreversed, port level 10)
- Convention / signature: `__cdecl float (CVehicle* veh [esp+4])`, returns ST0, plain `ret`. Frame `sub esp,0x24` + esi (early path) or
  esi/ebx/ebp/edi; E = esp after the four pushes. Locals: minX E+0x28 (later reused for `(y & 0xF) << 4`), maxX E+0x14, minY E+0x18,
  maxY E+0x10, sector bounds xMin E+0x20/ebp, xMax E+0x1C, yMin edi -> y counter E+0x2C, yMax E+0x24; the veh arg slot E+0x38 is
  reused as scratch and then as `maxSpeed` (its address is passed to the sector-list functions).
- Callers: SlowCarOnRailsDownForTrafficAndLights 0x434790, SteerAICarWithPhysicsFollowPath 0x434900.
- Callees: CRT `floor` 0x8219F0 (double on stack -> ST0, 8 calls), `_ftol` 0x821B40 (8 calls), CWorld::ClearScanCodes 0x563470,
  SlowCarDownForCarsSectorList 0x432420, SlowCarDownForPedsSectorList 0x425440, SlowCarDownForObjectsSectorList 0x42D4F0 (all cdecl,
  8 args `(list, veh, minX, minY, maxX, maxY, float* pSpeed, float cruise)`).
- Globals: CWorld::ms_nCurrentScanCode 0xB7CD78 (u16, RW), CWorld::ms_aRepeatSectors 0xB992B8 (16x16, stride 12: +0 vehicles, +4 peds,
  +8 objects lists). Constants (f32): `0x859030 = 14.0`, `0x858B38 = 0.019999999552965164`, `0x858B34 = 60.0`, `0x858B8C = 0.5`;
  immediate 0x77 = 119.
- Fields: veh +0x3B9 m_autoPilot.m_nCarDrivingStyle (u8), +0x3D0 m_nCruiseSpeed (u8), +0x3D4 m_SpeedMult (f32), +0x429 vehicleFlags
  byte 1 (bit 5 = bWarnedPeds, flag bit 13), +0x14/+0x04 position.
- Behaviour:
  1. If drivingStyle is 2 (AVOID_CARS), 3 (PLOUGH_THROUGH) or 5 (AVOIDCARS_OBEYLIGHTS) (0x434408-0x434420): return
     `(float)cruise * speedMult` (fild of the zero-extended u8, fmul, 0x43476A-0x434779; not rounded to f32 before return).
  2. Box (P = veh pos, re-read per use): `minX = P.x - 14.0f`, `maxX = P.x + 14.0f`, `minY = P.y - 14.0f`, `maxY = P.y + 14.0f` (each
     stored f32, 0x43443A-0x434489).
  3. Sector bounds; for each `s(v) = v * 0.02f + 60.0f` stored f32 (`fst`) and passed as double to floor (the second floor call reloads
     the stored f32):
     `xMin = ftol(floor(s(minX))) > 0 ? ftol(floor(s(minX))) : 0` (0x43448D-0x4344D8), `yMin` likewise from minY (0x4344DA-0x43451C),
     `xMax = ftol(floor(s(maxX))) < 119 ? that : 119` (0x43451E-0x43456B), `yMax` likewise from maxY (0x43456F-0x4345B7).
     Signed compares (`jle` / `jge`). floor+ftol are called twice for every value that is kept.
  4. Scan code: `if (ms_nCurrentScanCode >= 0xFFFF) { ClearScanCodes(); ms_nCurrentScanCode = 1; } else ms_nCurrentScanCode++` (0x4345BB-0x4345D4).
  5. `maxSpeed = (float)cruise * speedMult` (stored f32 in the arg slot, 0x4345DD-0x4345FE).
  6. `for (y = yMin; y <= yMax; y++) for (x = xMin; x <= xMax; x++)` (signed, outer test 0x434602 / 0x434717, inner 0x434614 / 0x4346FE):
     `R = &ms_aRepeatSectors[(y & 0xF) * 16 + (x & 0xF)]`;
     - if drivingStyle != 6: `SlowCarDownForCarsSectorList(&R->vehicles, veh, minX, minY, maxX, maxY, &maxSpeed, cruiseNow)` (0x43467A);
     - `SlowCarDownForPedsSectorList(&R->peds, ...same...)` (0x4346B5, always);
     - `SlowCarDownForObjectsSectorList(&R->objects, ...same...)` (0x4346EF, always);
     where `cruiseNow = (float)veh->cruise * veh->speedMult` is recomputed (fild/fmul/fstp f32) before each call (cruise is re-read).
  7. `veh->+0x429 |= 0x20` (bWarnedPeds = 1, 0x43472A-0x434730).
  8. If drivingStyle (re-read) is 0 (STOP_FOR_CARS) or 4 (STOP_FOR_CARS_IGNORE_LIGHTS): return `maxSpeed` (f32 from memory, 0x434761).
     Else: return `((float)cruise * speedMult + maxSpeed) * 0.5f` (x87, not stored, 0x43473D-0x434757).
- Edge cases: drivingStyle 6 skips only the cars list. The callees may lower `maxSpeed` through the pointer; their `cruise` argument is
  the unmodified cruise speed. Sector indices wrap with `& 0xF` (repeat sectors).
- gta-reversed: `.Reversed = false`, no body to compare (plugin call). Open questions: none.

## 0x434900 CCarCtrl::SteerAICarWithPhysicsFollowPath  (catalogue: gr-unreversed, port level 11)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], float* pSteer [+8], float* pGas [+0xC], float* pBrake [+0x10],
  bool* pHandbrake [+0x14])` (gr: `(CVehicle*, float*, float*, float*, bool*)`), plain `ret`. Frame `sub esp,0x60` + ebp/esi, then edi
  at 0x43497E; E = esp after the three pushes (args at E+0x70..E+0x80; before 0x43497E all offsets are E+4).
  Locals (E-relative, f32): lo1 +0x0C, dist +0x10, D1.y +0x14, D1.x +0x18, fwdN.y +0x1C (later angHeading, then m1, then M, then diff),
  P1.x +0x20, D2.y +0x24, D2.x +0x28, P1.y +0x2C (later dot, then angTarget), fwdN.x +0x30 (later fwdSpeed / distance to destination),
  P2 +0x34/+0x38 (+0x38 later segLen, scratch, linkAngle1, m2), target T +0x3C/+0x40/+0x44, X1 +0x48, Y1 +0x4C.
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90.
- Callees: StopCarIfNodesAreInvalid 0x422590 (cdecl (veh) -> AL), CCarPathLink::OneWayLaneOffset 0x44DB00 (thiscall -> ST0, 4 sites),
  PickNextNodeAccordingStrategy 0x432B10 (-> AL), SteerAICarWithPhysicsHeadingForTarget 0x433280 (cdecl (veh, CEntity* = 0, float x,
  float y, pSteer, pGas, pBrake, pHandbrake), 4 sites), CWorld::FindPlayerCoors 0x56E010 (cdecl (CVector*, -1)), CGeneral::GetATanOfXY
  0x53CC70 (cdecl (x, y) -> ST0, 5 sites), FindAngleToWeaveThroughTraffic 0x4325C0, FindMaxSteerAngle 0x427FE0 (cdecl (veh) -> ST0),
  FindMaximumSpeedForThisCarInTraffic 0x434400, CTrafficLights::ShouldCarStopForLight 0x49D610 (cdecl (veh, 0) -> AL),
  CTrafficLights::ShouldCarStopForBridge 0x49D420 (cdecl (veh) -> AL), CCarAI::CarHasReasonToStop 0x41C050 (cdecl (veh), 2 sites),
  FindSpeedMultiplier 0x4224E0 (cdecl (float angleChange, float minAngle, float maxAngle, float minMult) -> ST0, 2 sites),
  VectorSub 0x40FE60 (cdecl (CVector* out, const CVector* a, const CVector* b) -> EAX), CVector::Magnitude 0x4082C0 (thiscall -> ST0),
  CVector::DotProduct 0x40FDB0 (cdecl (const CVector*, const CVector*) -> ST0).
- Globals: ThePaths.m_pPathNodes[] 0x96F854 (area-loaded test), m_pNaviNodes[] 0x96F974, CTimer::m_snTimeInMilliseconds 0xB7CB84.
  Jump tables: mission switch 0x43579C / byte index 0x4357C0 (mission-2, bound 0x42); driving-style switches 0x435804 / 0x43580C and
  0x435814 / 0x43581C (bound 6). Constants (f32): `0x858B50 = 0.0`, `0x858624 = 1.0`, `0x858C58 = 0.009999999776482582`,
  `0x858C50 = 5.400000095367432`, `0x859010 = 1.4580000638961792`, `0x858C48 = 0.125`, `0x858C80 = 5.0`, `0x859000 = 8.0`,
  `0x858CB0 = 0.699999988079071`, `0x858F9C = 0.3499999940395355`, `0x858A10 = 40.0`, `0x858CC0 = -3.1415927410125732`,
  `0x858CBC = 6.2831854820251465`, `0x858CB8 = 3.1415927410125732`, `0x858B34 = 60.0`, `0x859038 = 0.02500000037252903`,
  `0x858C28 = 0.05000000074505806`, `0x858B10 = 0.029999999329447746`, `0x858CA0 = 2.0`, `0x858C84 = 0.25`, `0x858B3C = 3.0`,
  `0x859034 = -0.0833333358168602`, `0x858B8C = 0.5`, `0x858CC4 = 0.20000000298023224`; immediates 0x3ECCCCCD = 0.4f,
  0x3F99999A = 1.2f, 0x3DCCCCCD = 0.1f, 0x3F800000 = 1.0f, 0x7D0 = 2000.
- Fields (veh): +0x14 m_matrix (fwd +0x10/+0x14/+0x18, pos +0x30; read WITHOUT null check at 0x43491A, 0x4352C2, 0x4356EB), +0x04 placement
  pos, +0x44..+0x4C m_vecMoveSpeed, +0x594 m_nVehicleSubType, +0x818 (CBmx: gr `m_fControlPedaling`, gr's "always 0" comment is contradicted
  by 0x435636). Autopilot: +0x3A4/+0x3A6 current/next CCarPathLinkAddress, +0x3B5/+0x3B6 direction signs (i8), +0x3B7/+0x3B8 current/next
  lane (i8), +0x3B9 m_nCarDrivingStyle, +0x3BA m_nCarMission, +0x3BB m_nTempAction, +0x3BC m_nTempActionTime, +0x3D0 m_nCruiseSpeed (u8),
  +0x3EC/+0x3F0 m_vecDestinationCoors.x/.y, +0x41C m_TargetEntity.
- Behaviour:
  1. If `StopCarIfNodesAreInvalid(veh)` return immediately (outputs untouched, 0x434914).
  2. Forward: `fx = m.fwd.x`, `fy = m.fwd.y`; `len = sqrt(fy*fy + fx*fx)`; if `len == 0` (`test ah,0x44; jnp`, NaN is "not equal"):
     `fwdN = (1.0f, fy)` (fy unnormalised); else `inv = 1.0f / len`, `fwdN.x = inv * fx`, `fwdN.y = inv * fy` (0x43491A-0x43495E).
  3. If `m_pPathNodes[cur >> 10] == null` or `m_pPathNodes[next >> 10] == null` (link addresses used as area index; 0x434975-0x43499B):
     `*pBrake = 1.0f; *pGas = 0; *pSteer = 0; *pHandbrake = 0` (0x43576B), return.
  4. Link geometry (L1 = cur link, L2 = next link): `D1 = (L1.dirX * 0.01f * s1, L1.dirY * 0.01f * s1)`, `D2` likewise with s2
     (`s = (float)(i8)` sign byte; same op order as UpdateCarOnRails step 6). `lo1 = (L1.OneWayLaneOffset() + curLane) * 5.4f` (stored);
     `lo2 = (L2.OneWayLaneOffset() + nextLane) * 5.4f` (x87). If `subType == 10`: `lo1 += 1.458f` (stored), `lo2 += 1.458f` (0x434AE9-0x434B0A).
     `P1 = (L1.x * 0.125f + lo1 * D1.y, L1.y * 0.125f - lo1 * D1.x)`, `P2 = (L2.x * 0.125f + lo2 * D2.y, L2.y * 0.125f - lo2 * D2.x)` (stored).
  5. `d = V - P1` (V = veh pos): `dist = sqrt(d.y*d.y + d.x*d.x)`; `e = P2 - P1`: `segLen = sqrt(e.y*e.y + e.x*e.x)`; `dot = e.y*d.y + e.x*d.x`
     (0x434C05-0x434C54).
  6. Advance test (0x434C5C): take step 7 if `dist < 5.0f`, or (`dot > 0` and `dist < 8.0f`), or `dot / (segLen * dist) > 0.7f`
     (`fld segLen; fmul dist; fdivr dot`), or `next == cur` (u16, 0x434CA8). Otherwise go to step 9 with the values of steps 4-5.
  7. If `PickNextNodeAccordingStrategy(veh)` returns true, switch on mission (0x434CC2):
     2 -> mission = 3 and head for the player; 4 -> 5 and player; 0x2B -> 0x2C and player (player = `FindPlayerCoors(&tmp, -1)`, x/y);
     8 -> 9 and 0xC -> 0xD, heading for (destX, destY); 0xF -> 0x10, 0x11 -> 0x12, 0x41..0x44 -> mission - 0x24 (= 0x1D..0x20), heading for the
     target entity position (`m_TargetEntity`, matrix pos or placement pos, no null check). Each case stores the new mission byte then
     `SteerAICarWithPhysicsHeadingForTarget(veh, 0, x, y, pSteer, pGas, pBrake, pHandbrake)` and returns. Other missions fall through.
  8. (0x434E63, after a pick that returned false or an unlisted mission) Re-check both areas as in step 3 (failure -> 0x43573C, same outputs).
     `lo1 = (L1.OneWayLaneOffset() + curLane) * 5.4f` with the NEW links and WITHOUT the BMX +1.458. `dist = sqrt(((L1.y*0.125f - lo1*D1.x) - V.y)^2
     + ((L1.x*0.125f + lo1*D1.y) - V.x)^2)` (y term first) using the OLD D1 of step 4 (original quirk, keep). Then D1 and D2 are recomputed
     from the new links (0x434F9A-0x43504F). P2/segLen/dot are not recomputed (not used further).
  9. Target (0x435055): `t1 = lo1 * D1.y` (x87), `X1 = L1.x*0.125f + t1`, `Y1 = L1.y*0.125f - lo1*D1.x` (stored);
     `T = (X1 - dist * D1.x * 0.35f, Y1 - dist * D1.y * 0.35f, 0)`. If `dist > 40.0f` (0x435125; NaN no): `T = (L1.x*0.125f + t1,
     L1.y*0.125f - lo1*D1.x, 0)` (recomputed, equals X1/Y1).
  10. `angTarget = GetATanOfXY(T.x - V.x, T.y - V.y)`; `angHeading = GetATanOfXY(fwdN.x, fwdN.y)` (0x4351D6, 0x4351E9).
      `a = (drivingStyle in {2, 5, 6}) ? FindAngleToWeaveThroughTraffic(veh, 0, angTarget, angHeading, 1.0f) : angTarget`.
      `steer = a - angHeading`, normalised: `while (steer < -pi) steer += 2pi; while (steer > pi) steer -= 2pi` (same compare pattern as
      0x4325C0 step 7a, stored f32). `mx = FindMaxSteerAngle(veh)`; `if (steer < -mx) steer = -mx;` then `if (steer > mx) steer = mx`
      (NaN keeps steer).
  11. `fwdSpeed = ((mv.z*fwd.z + mv.y*fwd.y) + mv.x*fwd.x) * 60.0f` (stored, 0x4352C2-0x4352ED).
      `sf = (style in {0,1,4,6}) ? FindMaximumSpeedForThisCarInTraffic(veh) / (int)cruise (fidiv) : 1.0f` (style > 6 unsigned -> 1.0f).
      If style in {0,1,5,6} and `ShouldCarStopForLight(veh, 0)`: `CarHasReasonToStop(veh); sf = 0`. Then always: if
      `ShouldCarStopForBridge(veh)`: `CarHasReasonToStop(veh); sf = 0` (0x435341-0x435377).
  12. Curve multipliers (0x43537F): `lo1' = (L1.OneWayLaneOffset() + curLane) * 5.4f` (no BMX term);
      `m1 = FindSpeedMultiplier(GetATanOfXY((L1.x*0.125f + lo1'*D1.y) - V.x, (L1.y*0.125f - lo1'*D1.x) - V.y) - angHeading, 0.4f, 1.2f, 0.4f)`;
      `m2 = FindSpeedMultiplier(GetATanOfXY(D1.x, D1.y) - GetATanOfXY(D2.x, D2.y), 0.1f, 1.2f, 0.4f)` (D1 angle computed first).
      `m3 = 1.0f` if `dist > 40.0f` or `cruise < 12` (u8); else `m3 = 1.0f - (1.0f - dist * 0.025f) * (1.0f - m2)` (0x4354CE-0x4354EA).
      `M = (min_a < sf) ? (m1 < m3 ? m1 : m3) : sf` where `min_a = (m1 < m3) ? m1 : m3` (NaN compares fall to the second operand / sf,
      0x435500-0x43553B).
  13. `*pBrake = 0` (0x43554C). `desired = (float)cruise * M`; `diff = desired - fwdSpeed` (stored).
      - If `desired < 0.05f` and `diff < 0.03f`: `*pBrake = 1.0f`, `*pGas = 0`.
      - Else if `diff > 0`: `g = diff * (fwdSpeed < 2.0f ? 0.25f : 0.125f)`; `*pGas = (1.0f < g) ? 1.0f : g`. If `subType == 10`, `diff > 3.0f`
        and `veh->+0x818 <= 0.0f` (`test ah,0x41; jp`): `veh->+0x818 = 10.0f` (0x435609-0x435636).
      - Else (`diff <= 0` or NaN): `*pGas = 0`; `b = diff * -0.083333336f`; `*pBrake = (0.5f < b) ? 0.5f : b`.
  14. `*pSteer = steer`, `*pHandbrake = 0` (0x43566F-0x435680).
  15. If mission is 8 or 0xC and `m_nTempAction == 0`: `d = |V - dest|` (VectorSub(tmp, pos, &veh->+0x3EC) then Magnitude, stored);
      if `d < 8.0f`: `k = |DotProduct(&V, &fwd)|` - NOTE: the dot product of the vehicle POSITION with its forward vector, not of the
      offset (original bug, 0x4356EE-0x4356F0); if `k / d < 0.2f`: `m_nTempAction = 3` (REVERSE), `m_nTempActionTime = now + 2000` (0x43571C).
- Edge cases: cruise == 0 makes `sf` a division by zero (inf/NaN) in step 11. The matrix pointer is dereferenced unchecked (steps 2, 11, 15).
- gta-reversed: `.Reversed = false`, plugin call only. Open questions: none.

## 0x435830 CCarCtrl::SteerAICarWithPhysicsFollowPath_Racing  (catalogue: gr-unreversed, port level 6)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], float* pSteer [+8], float* pGas [+0xC], float* pBrake [+0x10], bool* pHandbrake [+0x14])`,
  plain `ret`. Frame `sub esp,0x68` + ebx/ebp/esi/edi; E = esp after the four pushes (args at E+0x7C..E+0x8C).
  Locals: clampedFlag byte E+0x13, dist / bend outAngle / steer limit / diff E+0x14, fwdN.y then steer E+0x18, fwdSpeed E+0x1C, outAngleDiff then
  angHeading E+0x20, P1.x then dot then outSpeedMult (init 1.0) E+0x24, fwdN.x then -limit E+0x28, P2.x then outDist E+0x2C, P2.y then segLen then
  scratch E+0x30, signs[8] (i8) E+0x34..E+0x3B, outPos (CVector2D) E+0x3C, links[8] (u16) E+0x48..E+0x57, nodes[8] (CNodeAddress) E+0x58..E+0x77.
- Callers: SteerAICarWithPhysics_OnlyMission 0x436A90.
- Callees: StopCarIfNodesAreInvalid 0x422590, CPathFind::DoPathSearch 0x4515D0 (thiscall ThePaths 0x96F050, gr parameter order), CAutoPilot::RemoveOnePathNode
  0x41B950 (thiscall on veh+0x390, 3 sites), CNodeAddress `==` 0x420920 / `<` 0x420980 (thiscall, see 0x430050), SteerAICarWithPhysicsHeadingForTarget
  0x433280, 0x403D60 = MSVC `eh vector constructor iterator` (__stdcall (void* array, size, count, ctor)) with ctors 0x4241B0 (CCarPathLinkAddress:
  `w[this] = 0xFFFF`) and 0x4241A0 (CNodeAddress: `w[this] = 0xFFFF`, node half untouched), CPathFind::FindLinkBetweenNodes 0x451350 (thiscall, hidden
  return pointer, (CNodeAddress a, CNodeAddress b) -> u16), DealWithBend_Racing 0x428040 (15 args, CCarCtrl.md), CGeneral::GetATanOfXY 0x53CC70,
  ClitargetOrientationToLink 0x422760 (CCarCtrl.md; 4 sites), FindPercDependingOnDistToLink 0x422620 (cdecl (veh, CCarPathLinkAddress) -> ST0, 2 sites),
  FindAngleToWeaveThroughTraffic 0x4325C0, CTrafficLights::ShouldCarStopForBridge 0x49D420, CCarAI::CarHasReasonToStop 0x41C050.
- Globals: ThePaths 0x96F050 (tables 0x96F854, 0x96F974, 0x96FA94, 0x96FDF4), 0x8A5F44 (CNodeAddress constant passed as forbiddenNodeAddr).
- Constants (f32): `0x858B34 = 60.0`, `0x858B50 = 0.0`, `0x858624 = 1.0`, `0x858C48 = 0.125`, `0x859000 = 8.0`, `0x858CCC = 12.0`,
  `0x858CB0 = 0.699999988079071`, `0x858CA0 = 2.0`, `0x859040 = 0.3333333432674408`, `0x858CC0 = -3.1415927410125732`, `0x858CBC = 6.2831854820251465`,
  `0x858CB8 = 3.1415927410125732`, `0x858CC4 = 0.20000000298023224`, `0x858C20 = 0.8999999761581421`, `0x858C28 = 0.05000000074505806`,
  `0x858B10 = 0.029999999329447746`, `0x858F48 = 7.0`, `0x858C84 = 0.25`, `0x858B3C = 3.0`, `0x85903C = -0.06666667014360428`,
  `0x85901C = -0.05000000074505806`, `0x858B8C = 0.5`; immediates 999999.875f (0x497423FE), 2.0f, 0.7f (0x3F333333), 15.0f.
- Fields (veh): autopilot +0x390/+0x394/+0x398 current/next/ending node, +0x3A4/+0x3A6/+0x3A8 current/next/previous link, +0x3B4/+0x3B5/+0x3B6 signs
  (prev/cur/next), +0x3B7/+0x3B8 lanes, +0x3BA mission, +0x3D0 cruise, +0x3DB carCtrlFlags (bit 6 bCantGoAgainstTraffic), +0x3EC destination,
  +0x3F8 m_aPathFindNodesInfo[8], +0x418 m_nPathFindNodesCount (i16); +0x22 model, +0x44.. move speed, +0x594 subType, +0x818 (CBmx).
- Behaviour:
  1. `clamped = 0`. If `StopCarIfNodesAreInvalid(veh)` return (outputs untouched).
  2. `fwdSpeed = ((mv.z*fwd.z + mv.y*fwd.y) + mv.x*fwd.x) * 60.0f` (stored); forward normalised as in 0x434900 step 2 (fwdN.x E+0x28, fwdN.y E+0x18).
  3. If `m_pPathNodes[cur >> 10]` or `m_pPathNodes[next >> 10]` is null: `*pBrake = 1.0f, *pGas = 0, *pSteer = 0, *pHandbrake = 0`, return (0x4364FF).
  4. `P1 = (Lc.x*0.125f, Lc.y*0.125f)`, `P2 = (Ln.x*0.125f, Ln.y*0.125f)` (no lane offset); `d = V - P1`: `dist = sqrt(d.y^2 + d.x^2)`; `e = P2 - P1`:
     `segLen = sqrt(e.y^2 + e.x^2)`; `dot = e.y*d.y + e.x*d.x`. Advance when `dist < 8.0f`, or (`dot > 0` and `dist < 12.0f`), or
     `dot / (segLen * dist) > 0.7f`, or `next link == cur link`; otherwise skip to step 6.
  5. Advance (0x435A63):
     a. If `count (+0x418) < 4`: `ThePaths.DoPathSearch(0, V, +0x394, dest (+0x3EC), &+0x3F8, &count, 8, null, 999999.875f, null, 999999.875f,
        (+0x3DB >> 6) & 1, [0x8A5F44], model == 0x21B (539 VORTEX), false)`; then if `count > 0`: `RemoveOnePathNode()`, and if still `count > 0` and
        `+0x394 == +0x3F8[0]`: `RemoveOnePathNode()` again. If now `count < 1`: `mission = 9`, `SteerAICarWithPhysicsHeadingForTarget(veh, null, dest.x,
        dest.y, pSteer, pGas, pBrake, pHandbrake)`, return.
     b. Shift (0x435B84): `+0x398 = +0x390; +0x390 = +0x394; +0x394 = +0x3F8[0]; RemoveOnePathNode(); +0x3A8 = old +0x3A4; +0x3A4 = old +0x3A6;
        +0x3B4 = old +0x3B5; +0x3B7 = +0x3B8; +0x3B5 = old +0x3B6`. Find `k` with `nodeLinks[cur.area][cur.baseLink + k] == +0x394` (UNBOUNDED loop,
        0x435C20-0x435C44); `+0x3A6 = naviLinks[cur.area][cur.baseLink + k]`; `+0x3B6 = (+0x390 < +0x394) ? -1 : 1`.
  6. Lookahead tables (0x435C8E): links[0..7] = 0xFFFF, nodes[0..7].area = 0xFFFF (vector ctors); `links[0] = +0x3A4, links[1] = +0x3A6,
     signs[0] = +0x3B5, signs[1] = +0x3B6, nodes[0] = +0x390, nodes[1] = +0x394`. For j = 0..3: if `j < count` and `+0x3F8[j].area != 0xFFFF`:
     `nodes[2+j] = +0x3F8[j]`, `links[2+j] = FindLinkBetweenNodes(nodes[1+j], nodes[2+j])`, `signs[2+j] = (nodes[1+j] < nodes[2+j]) ? -1 : 1`;
     else `links[2+j] = 0xFFFF`, `nodes[2+j].area = 0xFFFF` (signs[2+j] left uninitialised). nodes[6], nodes[7] areas = 0xFFFF.
  7. Bend search: `outSpeedMult = 1.0f`, `chosen = 0xFFFF`; for j = 0..3: if `DealWithBend_Racing(veh, links[j], links[j+1], links[j+2], links[j+3],
     signs[j], signs[j+1], signs[j+2], signs[j+3], fwdSpeed, &outAngle, &outSpeedMult, &outAngleDiff, &outDist, &outPos)`: `chosen = links[j]`, stop.
  8. `angHeading = GetATanOfXY(fwdN.x, fwdN.y)` (overwrites outAngleDiff).
     - chosen == 0xFFFF: `angle = angHeading`; for idx = 5, 4, 3, 2 with `links[idx] != 0xFFFF`: `ClitargetOrientationToLink(veh, links[idx],
       signs[idx], &angle, L.x*0.125f, L.y*0.125f)`; then the same with (+0x3A6, +0x3B6, Ln coords); `p = FindPercDependingOnDistToLink(veh, +0x3A4)`;
       target `X = (Lc.x*0.125f)*p + (Ln.x*0.125f)*(1-p)`, `Y = (Lc.y*0.125f)*p + (Ln.y*0.125f)*(1-p)` (y computed first).
     - chosen != 0xFFFF: `angle = outAngle` (from the bend); if `+0x3A4 != chosen`: ClitargetOrientationToLink(veh, +0x3A6, +0x3B6, &angle, Ln coords);
       `q = (FindPercDependingOnDistToLink(veh, +0x3A4) + 2.0f) * 0.33333334f`; `X = (Ln.x*0.125f)*(1-q) + (Lc.x*0.125f)*q`, `Y` likewise (y first).
     Then `ClitargetOrientationToLink(veh, +0x3A4, +0x3B5, &angle, X, Y)` (the two dwords left on the stack by the FindPerc call are reused as X/Y).
  9. `steer = FindAngleToWeaveThroughTraffic(veh, null, angle, angHeading, 2.0f) - angHeading`, normalised into [-pi, pi] (stored E+0x18).
  10. Steer limit: `sp = sqrt((mv.x^2 + mv.y^2) + mv.z^2)`; `lim = (sp > 0.7f) ? 0.2f : min(0.9f - sp, 0.7f)` (`0.7f < x -> 0.7f`). If `steer < -lim`:
      `steer = -lim`, clamped = 1; then if `steer > lim`: `steer = lim`, clamped = 1.
  11. `m = 1.0f`; if `ShouldCarStopForBridge(veh)`: `CarHasReasonToStop(veh)`, `m = 0`. `if (!(m < outSpeedMult)) m = outSpeedMult`.
      `*pBrake = 0`; `desired = m * (int)cruise` (fimul); `diff = desired - fwdSpeed` (stored).
      - `desired < 0.05f` and `diff < 0.03f`: `*pBrake = 1.0f`, `*pGas = 0`.
      - else `diff > 0`: if clamped and `fwdSpeed > 7.0f`: `*pGas = 0`. Else `*pGas = min1(diff * (fwdSpeed < 2.0f ? 0.25f : 0.125f))`; if `subType == 10`,
        `diff > 3.0f` and `+0x818 <= 0.0f`: `+0x818 = 15.0f` (15 here, 10 in the non-racing functions).
      - else: `*pGas = 0`; `*pBrake = (subType == 10) ? min1(diff * -0.06666667f) : ((0.5f < diff * -0.05f) ? 0.5f : diff * -0.05f)`.
  12. `*pSteer = steer`, `*pHandbrake = 0`.
- Edge cases / original bugs: the link search in 5b has no bound (relies on +0x394 being a neighbour of +0x390); signs[2+j] of unused slots are
  uninitialised stack bytes passed to DealWithBend_Racing (their links are 0xFFFF).
- gta-reversed: `.Reversed = false`, plugin call only. Open questions: (1) 0x435D63: for skipped slots the sign byte is never written - whether
  DealWithBend_Racing reads it for an invalid link is a question for 0x428040's spec.

## 0x436540 CCarCtrl::UpdateCarOnRails  (catalogue: gr-unreversed, port level 12)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4])`, plain `ret`. Frame `sub esp,0x64` + esi/edi; E = esp after both pushes.
  The veh arg slot E+0x70 is reused as int/float scratch (fild source, then laneOff2).
  Locals (E-relative, f32): laneOff1 +0x08, D2 = (+0x0C, +0x10), D1 = (+0x14, +0x18), t +0x20, resultSpeed +0x24..+0x2C, endDir +0x30..+0x38,
  startDir +0x3C..+0x44, End +0x48..+0x50, Start +0x54..+0x5C, resultCoor +0x60..+0x68.
- Detour oddity (keep the semantics): 0x43654F `nop; jmp 0x401284` -> 0x401284 `mov al,[esi+0x3DC]; jmp 0x436555`. The 6-byte instruction
  `mov al,[esi+0x3DC]` was moved out of line, so AFTER calling StopCarIfNodesAreInvalid the code tests veh+0x3DC, not the call's AL.
- Callers: CAutomobile::ProcessAI 0x6B4800, CBike::ProcessAI 0x6BC930, CBmx::ProcessAI 0x6C1470.
- Callees: StopCarIfNodesAreInvalid 0x422590 (cdecl (veh), result ignored), SlowCarOnRailsDownForTrafficAndLights 0x434790 (cdecl (veh)),
  PickNextNodeAccordingStrategy 0x432B10 (result ignored), CCarPathLink::OneWayLaneOffset 0x44DB00 (thiscall (link) -> ST0, 2 sites),
  CVector::Normalise 0x59C910 (thiscall, 2), CCurves::CalcCurvePoint 0x43C900 (cdecl (const CVector* start, const CVector* end,
  const CVector* startDir, const CVector* endDir, float t, int32 timeMs, CVector* outPos, CVector* outSpeed)), CCarCtrl::DragCarToPoint
  0x42EC90 (cdecl (veh, CVector*)), CAutoPilot::ModifySpeed 0x41B980 (thiscall (this = veh+0x390, float), 2 sites).
- Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84 (u32), ThePaths.m_pNaviNodes[] 0x96F974 (CCarPathLink stride 0xE). Constants (f32):
  `0x858C54 = 4294967296.0`, `0x858C58 = 0.009999999776482582`, `0x858C50 = 5.400000095367432`, `0x859010 = 1.4580000638961792`,
  `0x859048 = 0.008999999612569809`, `0x858C48 = 0.125`, `0x859044 = 0.01666666753590107`; immediate 0x41700000 = 15.0f.
- Fields (veh): +0x20 m_nRandomSeed (u16), +0x36 status bits 3-7, +0x44..+0x4C m_vecMoveSpeed, +0x594 m_nVehicleSubType (i32);
  autopilot (veh+0x390): +0x39C field_C (i32, curve start time), +0x3A0 m_nSpeedScaleFactor (i32, curve duration ms), +0x3A4 / +0x3A6
  m_nCurrentPathNodeInfo / m_nNextPathNodeInfo (u16 CCarPathLinkAddress: area = v >> 10, index = v & 0x3FF), +0x3AC m_nTimeToStartMission,
  +0x3B0 m_nTimeSwitchedToRealPhysics, +0x3B5 _smthCurr (i8 direction sign of the current link), +0x3B6 _smthNext (i8), +0x3B7 m_nCurrentLane
  (i8), +0x3B8 m_nNextLane (i8), +0x3BB m_nTempAction (i8), +0x3BC m_nTempActionTime (u32), +0x3DC m_nMovementFlags (bit 0 bIsStopped).
  CCarPathLink: +0 / +2 i16 x/y (x8), +8 / +9 i8 dir x/y (x100).
- Behaviour:
  1. `StopCarIfNodesAreInvalid(veh)`; if `veh->+0x3DC & 1` return (0x43654A-0x43655A, see detour note).
  2. `ta = (int8)veh->m_nTempAction`: if `ta == 1` (WAIT) or `ta == 24` (BRAKE) -> step 9; if `ta == 12` (STUCKINTRAFFIC) -> step 10
     (0x436560-0x43657A).
  3. `SlowCarOnRailsDownForTrafficAndLights(veh)` (0x436581). `end = field_C + speedScale` (i32 add). If `(int64)(uint32)now >=
     (int64)(int32)end` (cdq / two-part compare, 0x4365A0-0x4365A8; i.e. a negative `end` always passes) call
     `PickNextNodeAccordingStrategy(veh)` (0x4365AB).
  4. If `(veh->+0x36 & 0xF8) == 0x18` (status == STATUS_PHYSICS) return (0x4365BE).
  5. `t = (float)(uint32)(now - field_C) / (float)speedScale` (fild of the i32 difference + 4294967296.0 if negative, then `fidiv` by the
     i32; stored f32, 0x4365C6-0x436614). `now` is the value read before the PickNextNode call if it was not called, else re-read.
  6. Directions: L1 = link `cur`, L2 = link `next` (each looked up again per field read). `s1 = (float)_smthCurr`, `s2 = (float)_smthNext`;
     `D1.x = (L1.dirX * 0.01f) * s1`, `D1.y = (L1.dirY * 0.01f) * s1`, `D2.x = (L2.dirX * 0.01f) * s2`, `D2.y = (L2.dirY * 0.01f) * s2` (stored
     f32, 0x436614-0x4366EC).
     `laneOff1 = (L1.OneWayLaneOffset() + (float)m_nCurrentLane) * 5.4f`, `laneOff2 = (L2.OneWayLaneOffset() + (float)m_nNextLane) * 5.4f`
     (`fiadd` of the i8, 0x4366F2-0x43674D). If `m_nVehicleSubType == 10`: `laneOff1 += 1.458f`, `laneOff2 += 1.458f` (0x43673C-0x43676B).
  7. Random jitter: `a = (u16)seed + (u16)cur`, `b = (u16)seed + (u16)next` (int adds):
     `startDir = (((a & 7) - 3) * 0.009f + D1.x, (((a >> 3) & 7) - 3) * 0.009f + D1.y, 0)`,
     `endDir = (((b & 7) - 3) * 0.009f + D2.x, (((b >> 3) & 7) - 3) * 0.009f + D2.y, 0)` (jitter computed first, then `fadd D`, stored f32,
     0x43676F-0x436823); `startDir.Normalise()`, `endDir.Normalise()` (0x436827, 0x436830).
     `End = (L2.x * 0.125f + laneOff2 * D2.y, L2.y * 0.125f - laneOff2 * D2.x, 0)` (y computed first, 0x43685F-0x4368B6);
     `Start = (L1.x * 0.125f + laneOff1 * D1.y, L1.y * 0.125f - laneOff1 * D1.x, 0)` (0x4368DD-0x436953).
  8. `CalcCurvePoint(&Start, &End, &startDir, &endDir, t, speedScale, &resultCoor, &resultSpeed)` (0x43695B); then `resultCoor.z = 15.0f`
     and `DragCarToPoint(veh, &resultCoor)` (0x436969-0x436974). `resultSpeed *= 0.016666668f` per component (stored f32, 0x436979-0x4369B0).
     If `cur != next` (u16, 0x43698D): `veh->m_vecMoveSpeed = resultSpeed` (0x4369C6-0x4369CF); else leave it. Return.
  9. (ta 1 / 24, 0x436A19) `m_vecMoveSpeed = (0,0,0)`; `autopilot.ModifySpeed(0.0f)`; if `now > m_nTempActionTime` (unsigned, `jbe` skip):
     `m_nTempAction = 0`, `+0x3AC = now`, `+0x3B0 = now` (now re-read for each store). Return.
  10. (ta 12, 0x4369D7) `m_vecMoveSpeed = (0,0,0)`; `autopilot.ModifySpeed(0.0f)`; return.
- Edge cases: no null checks on m_pNaviNodes[area]; speedScale == 0 divides by zero (x87 inf/NaN, no trap with default masks).
- gta-reversed: `.Reversed = false`, plugin call only. Open questions: none.

