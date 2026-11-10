# CCarAI (POP) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/CarAI.cpp` (pin 20d8d2a). Vehicle fields used below: autopilot at +0x390, so
+0x3AC m_nTimeToStartMission, +0x3B9 m_nCarDrivingStyle, +0x3BA m_nCarMission, +0x3BB m_nTempAction, +0x3BC
m_nTempActionTime, +0x3C0 m_LastUpdateTimeMs, +0x3C4 m_ucTempActionMode, +0x3D0 m_nCruiseSpeed, +0x3DD
m_nStraightLineDistance, +0x41C m_TargetEntity; +0x36 entity status in bits 3-7; +0x428 bit0 bIsLawEnforcer, bit4
(0x10) bEngineOn; +0x42D bit1 bEngineBroken. RNG background: docs/re/cat14-gr-deviations.md finding 1. `_rand` = 0x821B1E,
`_ftol` = 0x821B40. CWanted::m_nWantedLevel = +0x2C of the CWanted returned by FindPlayerWanted 0x56E230 (cdecl, arg -1).

## 0x41C8A0 CCarAI::TellCarToRamOtherCar  (gr_deviations = 1)
Original: `__cdecl void (CVehicle* v1 [esp+4], CVehicle* v2 [esp+8])`, plain `ret`.
Row CarAI.cpp:573 `NOTSA_UNREACHABLE("Unused");`
- Original bytes: `v1->m_TargetEntity (+0x41C) = v2` (0x41C8B0), then `v2->RegisterReference(&v1->m_TargetEntity)` (0x571B70,
  `__thiscall`, ECX = v2 loaded at 0x41C8A0, arg = &field; NO null check on v2, unlike gr's SafeRegisterRef in the
  commented code); `CCarCtrl::JoinCarWithRoadSystem(v1)` (0x42F5A0, cdecl); `m_nCarMission = 15` MISSION_RAMCAR_FARAWAY
  (0x41C8CE); `bEngineOn (+0x428 bit 0x10) = !(+0x42D & 2)` (0x41C8C6-0x41C8DD); `m_nCruiseSpeed = max(m_nCruiseSpeed, 6)`
  (unsigned byte, `cmp al,6; ja keep`, 0x41C8E9-0x41C8F5).
- Verdict: CONFIRMED - gr aborts; the original performs the body above (no caller found, but a port must implement it).
  Class: keep-original.

## 0x41C900 CCarAI::TellCarToBlockOtherCar  (gr_deviations = 1)
Original: same shape as 0x41C8A0 (`__cdecl (v1 [esp+4], v2 [esp+8])`), mission byte 0x11 = 17 MISSION_BLOCKCAR_FARAWAY (0x41C92E).
Row CarAI.cpp:545 `NOTSA_UNREACHABLE("Unused");`
- Original bytes: target = v2 (0x41C910), `v2->RegisterReference(&target)` (0x41C912, no null check), JoinCarWithRoadSystem
  (0x41C918), mission 17, bEngineOn = !bEngineBroken (0x41C926-0x41C93D), cruise = max(cruise, 6) (0x41C949-0x41C955).
- Verdict: CONFIRMED. Class: keep-original.

## 0x41C9D0 CCarAI::FindPoliceCarMissionForWantedLevel  (gr_deviations = 1)
Original: `__cdecl uint8 ()` in AL (upper EAX bits are leftover rand()/wanted-level bits; callers store AL, e.g. 0x41FA6E).
Switch on wanted level (`cmp eax,6; ja default`, unsigned, 0x41C9DD), jump table 0x41CA20 = {0x41CA1C, 0x41CA1C, 0x41C9E9,
0x41C9FA, 0x41CA0B, 0x41CA0B, 0x41CA0B}.
Row CarAI.cpp:215 `default: NOTSA_UNREACHABLE();`
- Original bytes: default (level > 6 or < 0) = same block as levels 0/1: `mov al,4` (MISSION_BLOCKPLAYER_FARAWAY, 0x41CA1C).
- Verdict: REFUTED for the marker (wanted level is 0..6). Class: harmless (port: default -> 4).
Found in passing (CONFIRMED, keep-original - gr logic error): levels 2/3/4-6 do `r = _rand() & 3` (low byte, 0x41C9EE/0x41C9FF/
0x41CA10) and return `(r < K) ? 4 : 2` with K = 3 / 2 / 1 (`setl; lea eax,[eax+eax+2]`), i.e. MISSION_BLOCKPLAYER_FARAWAY or
**MISSION_RAMPLAYER_FARAWAY (2)**. gr returns `RandomBool(chance) ? MISSION_BLOCKPLAYER_FARAWAY : MISSION_BLOCKPLAYER_HANDBRAKESTOP (6)`:
wrong mission for the false case, and `RandomBool` = `GetRandomNumberInRange(0,100) <= chance` instead of `rand() & 3`.

## 0x41CAA0 CCarAI::FindPoliceCarSpeedForWantedLevel  (gr_deviations = 2)
Original: `__cdecl int32 (CVehicle* veh [esp+4])`, EAX. Jump table 0x41CB50 = {0x41CABD, 0x41CAD4, 0x41CADA, 0x41CAE0, 0x41CAFB,
0x41CB16, 0x41CB31}; `cmp eax,6; ja 0x41CB4C` (unsigned).
Row CarAI.cpp:226 `case WANTED_CLEAN: return CGeneral::GetRandomNumberInRange<int32>(12, 16);`
- Original bytes: `CGeneral::GetRandomNumberInRange<float>(12.0f (push 0x41400000), 16.0f (push 0x41800000))` 0x41BD90, then tail
  `jmp _ftol` (0x41CACF): truncation of the float result.
- Verdict: CONFIRMED (gr int version = `(int)lerp(12, 15, rand()/32767)`, range 12..15 with different mapping; the original
  maps through the float 0x41BD90 formula and truncates). Class: keep-original.
Row CarAI.cpp:233 `default: NOTSA_UNREACHABLE();`
- Original bytes: default returns 0 (`xor eax,eax`, 0x41CB4C). Verdict: REFUTED (unreachable for 0..6). Class: harmless.
Found in passing (CONFIRMED, rounding-level): levels 3..6 return `_ftol(handling->m_fMaxVelocity (veh+0x384 -> +0x88) * K)` with
one f32 constant each: level 3 `0x858C94 = 54.0f` (0x41CAF0), level 4 `0x858C90 = 72.0f` (0x41CB0B), level 5 `0x858C8C = 75.0f`
(0x41CB26), level 6 `0x858C88 = 78.0f` (0x41CB41); gr computes `v * 60.0f * 0.90f` (two roundings, 0.9f/1.2f/1.25f/1.3f
inexact). Levels 1 and 2 return 25 (0x41CAD4) and 34 (0x41CADA) as gr.

## 0x41CB70 CCarAI::MellowOutChaseSpeedBoat  (gr_deviations = 1)
Original: `__cdecl void (CVehicle* veh [esp+4])`. Jump table 0x41CBE0; writes `+0x3D0` = 8, 10, 15, 20, 25, 30, 40 for
levels 0..6 (0x41CB8D..0x41CBD5). Row CarAI.cpp:518 (assert, deviates=no): no check in the original.
Row CarAI.cpp:528 `default: NOTSA_UNREACHABLE();`
- Original bytes: default (`ja 0x41CBDC`) just returns, cruise speed unchanged.
- Verdict: REFUTED (unreachable). Class: harmless (port: default = no write).

## 0x41D660 CCarAI::MakeWayForCarWithSiren  (gr_deviations = 1)
Original: `__cdecl void (CVehicle* siren [esp+4])`, plain `ret`. No calls at all.
Row CarAI.cpp:392 `for (auto& v : GetVehiclePool()->GetAllValid())` (pool-iteration)
- Original bytes: slot loop from `ms_pVehiclePool->m_nSize - 1` down to 0 (0x41D6D7-0x41D6DF, flags byte `js` = free,
  stride 0xA18), skipping free slots. Each iteration reads only `v`'s and the siren car's fields and writes only `v`'s temp
  action / time / status; `v == siren` is skipped (0x41D75C). So iteration order cannot change the result.
- Verdict: REFUTED (order-independent). Class: harmless. The four `notsa::contains` rows (deviates=no) are plain compares
  in the bytes (0x41D719/0x41D721, 0x41D916/0x41D91E, 0x41D948/0x41D94C, 0x41D9F8/0x41D9FC).
Found in passing (CONFIRMED, keep-original):
1. Early-out missing in gr: `s = sqrt(vx*vx + vy*vy)` of the siren car's move speed (+0x44/+0x48, 0x41D668-0x41D685, stored
   f32); `if (s < 0.1f [0x858B1C]) return` (0x41D68F-0x41D69A; NaN/equal continue). gr normalises without this check.
2. Moving check: original `sqrt(v.vx^2 + v.vy^2) <= 0.05f [0x858C28]` -> skip (0x41D86F-0x41D890, NaN skips); gr uses the
   SQUARED magnitude `<= 0.05f` (threshold 0.224 in speed instead of 0.05).
3. Swerve side: original `c = dx*my - dy*mx` (cross product; d = v.pos - siren.pos, m = siren dir normalised =
   `(1/s)*vx, (1/s)*vy`, 0x41D950-0x41D960); `c > 0` -> TEMPACT_SWIRVELEFT_STOP (20) else SWIRVERIGHT_STOP (21) (0x41D96A-0x41D978).
   gr uses `m.Dot(d) <= 0 ? RIGHT : LEFT` (dot product): different side. The later flip on `goingSameDirDot < 0`
   (0x41D97F-0x41D99D) and `21 -> TEMPACT_WAIT (1)` with time now + 2500 (0x41D9A3-0x41D9BD) match gr.
4. Status: original writes STATUS_PHYSICS (`(st & 7) | 0x18`, 0x41D9CA) unless the status byte saved at the top of the
   iteration equals 12 (0x41D9C3); it is always 2 or 3 there, so always written. gr's `!= STATUS_GHOST` is equivalent.

## 0x41DA30 CCarAI::UpdateCarAI  (gr_deviations = 4)
Original: `__cdecl void (CVehicle* veh [esp+4])` (ESI = veh in the body), SEH frame, `ret` at 0x420678.
Row CarAI.cpp:876 (escort -> FARAWAY lambda `default: NOTSA_UNREACHABLE();`)
- Original bytes: `if (!JoinCarWithRoadSystemGotoCoors(veh, FindPlayerCoors(-1), true, false)) m_nCarMission += 0x24`
  (`add byte [esi+0x3BA],0x24`, 0x41F874): no switch, plain +36 (29..32 -> 65..68).
- Verdict: REFUTED (same result for the four case values). Class: harmless.
Row CarAI.cpp:1034 (FARAWAY -> escort lambda default)
- Original bytes: `m_nCarMission += 0xDC` (-36, 0x41F765). Verdict: REFUTED. Class: harmless.
Row CarAI.cpp:1144 (`switch (m_ucTempActionMode)` default)
- Original bytes: `cmp eax,3; ja 0x41FCE7` (0x41FC79): default skips the temp-action write; jump table 0x42074C. The mode is
  `(mode + 1) % 4` (signed-remainder idiom on a zero-extended byte, 0x41FC11-0x41FC26) or 0 -> always 0..3.
- Verdict: REFUTED. Class: harmless.
Row CarAI.cpp:1323 (gang-area cruise speed, ped pool GetAllValid)
- Original bytes (0x4205AC-0x42065A): count peds, slot loop from `m_nSize - 1` down to 0 (stride 0x7C4), with
  `7 <= ped+0x598 (m_nPedType, signed) <= 17` (gang 1..10 or dealer, 0x4205EF-0x4205F7) and
  `|veh.pos - ped.pos| < 10.0f [0x85862C]` (VectorSub 0x40FE60 + CVector::Magnitude 0x4082C0, strict, 0x420630-0x42063B);
  then `m_nCruiseSpeed = max(10 - count, 1)` (0x420645-0x42065A). gr sets 10 and decrements per ped, stopping at 1: same
  final value; gr compares the squared distance against 100 (boundary rounding only).
- Verdict: REFUTED (order-independent count). Class: harmless.
Marker-only rows (deviates=no, notsa::contains / dyn_cast helpers at 672, 912, 1110, 1128, 1186, 1199, 1200, 1320): not
counted.
Found in passing (CONFIRMED, keep-original):
1. Stuck-reverse interval (gr CarAI.cpp:1128): original `interval = (m_nCarDrivingStyle == 0 || == 4) ?
   ((veh+0x20 byte & 0xF) + 40) * 500 : 1000` (`add eax,0x28; imul eax,eax,0x1F4`, 0x41FBDF-0x41FBEA) = 20000..27500 ms; gr
   computes `500 * (seed % 16) + 40` (40..7540 ms). gr logic error.
2. The same test `now - m_nTimeToStartMission > interval` is a SIGNED compare in the original (`cmp edi,eax; jle`, 0x41FBFA);
   gr compares unsigned (differs when the start time lies in the future).
3. Speed test before it: original `speed2DSq < 0.000144f [0x858CD4]` (strict, 0x41FBBA-0x41FBC5); gr `<= sq(0.012f)`.
4. Cruise-mission short reverse (gr :1146 `SetTempAction(TEMPACT_REVERSE, 750)`): original sets time `now + 0x2BC` = 700 ms
   (0x41FC5D) and action 3 (0x41FC69). gr constant error.
5. Escort near->far switch (gr :868, `SquaredMagnitude2D >= sq(Far)`): the bytes are `fcomp far vs d; test ah,5; jp skip`
   (0x41F83D-0x41F849): continue only if `far < d` (strict; NaN skips; d = sqrt of the 2D squared distance, 0x41F832). gr also continues on equality. Far->near (gr :1027
   `<= sq(Close)`): original `(float)m_nStraightLineDistance > d` strict (`fild; fcompp; test ah,0x41; jne skip`,
   0x41F754-0x41F75F). Boundary-only differences.
