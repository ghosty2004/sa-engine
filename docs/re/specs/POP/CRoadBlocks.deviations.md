# CRoadBlocks (POP) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/RoadBlocks.cpp` (pin 20d8d2a). Globals: InOrOut 0xA43438 (bool[325]), NumRoadBlocks
0xA43580 (int32), GenerateDynamicRoadBlocks 0xA43584 (bool), RoadBlockNodes 0xA435A0 (CNodeAddress[325]), aScriptRoadBlocks
0xA43AB8 (16 x 0x1C, IsActive at +0x18). RNG background: docs/re/cat14-gr-deviations.md finding 1. `_rand` 0x821B1E, `_ftol` 0x821B40.

## 0x461020 CRoadBlocks::ClearSpaceForRoadBlockObject  (gr_deviations = 1)
Original: `__cdecl bool (CVector cornerA [esp+4..], CVector cornerB [esp+0x10..])`, AL.
`CWorld::FindObjectsIntersectingCube(&cornerA, &cornerB, &num (int16), 2, list[2], buildings 0, vehicles 1, peds 1, objects 1,
dummies 0)` 0x568DD0 (0x461023-0x461043). `if (num > 2) return 0` (signed, 0x461050); `if (num <= 0) return 1` (0x461061).
For i = 0 .. num-1 (int16): e = list[i]; type = `+0x36 & 7`:
vehicle (2) and `CVehicle::CanBeDeleted` 0x6D1180: if `+0x42A & 0x10` (bCreateRoadBlockPeds) keep it and continue
(0x46108D), else remove; ped (3) and `CPed::CanBeDeleted` 0x5DF100: remove; object (4) and `+0x13C != 1` (not OBJECT_GAME,
tested BEFORE `CObject::CanBeDeleted` 0x59F120) and CanBeDeleted: remove; every other case returns 0 (0x4610EC), including a
non-deletable vehicle/ped (falls through the later type tests). Remove = `CWorld::Remove(e)` 0x563280 then `e->vtbl[0](1)`
(scalar deleting destructor, 0x4610DA). Return 1 after the loop (0x4610E5).
Row RoadBlocks.cpp:96 `default: NOTSA_UNREACHABLE();`
- Original bytes: any other entity type returns 0 (`cmp cl,4; jne 0x4610EC`, 0x4610B2). The search excludes buildings and
  dummies, so only types 2/3/4 come back.
- Verdict: REFUTED (unreachable in practice). Class: harmless (port: return false).

## 0x461100 CRoadBlocks::Init  (gr_deviations = 2)
Original: `__cdecl void ()`; entry `jmp 0x1568D90` (.HOODLUM). `InOrOut[0..324] = 1` (`rep stosd` 0x51 dwords of 0x01010101 +
one `stosb`, 0x1568D92-0x1568DB4); GenerateDynamicRoadBlocks = 0 (0x1568DAD); `f = CFileMgr::OpenFile("data\\paths\\roadblox.dat"
[0x859CBC], "rb" [0x859C6C])` 0x538900; `CFileMgr::Read(f, &NumRoadBlocks, 4)`; `CFileMgr::Read(f, RoadBlockNodes, 0x514)` (1300 bytes =
325 entries; the whole array, independent of NumRoadBlocks); `CFileMgr::CloseFile(f)`; then aScriptRoadBlocks[i].IsActive = 0
for the 16 entries (byte at 0xA43AD0 + 0x1C*i while `< 0xA43C90`, signed `jl`, 0x1568DEC-0x1568DF7; inlined ClearScriptRoadBlocks).
Row RoadBlocks.cpp:28 `if (notsa::File rbx(...); rbx)` and row :33 `NOTSA_UNREACHABLE("roadblox.dat couldn't be opened!")`
- Original bytes: no check of the file handle; both reads and the close run unconditionally (0x1568DB5-0x1568DDA).
- Verdict: REFUTED when the file exists (same reads). When it cannot be opened the original passes 0 to CFileMgr::Read (CRT
  failure), gr aborts. Class: harmless (file is mandatory game data).
Row :30 (assert, deviates=no): no check in the original.

## 0x461170 CRoadBlocks::GenerateRoadBlockPedsForCar  (gr_deviations = 1)
Original: `__cdecl void (CVehicle* veh [esp+4], int32 posType [esp+8], int32 pedType [esp+0xC])`.
Row RoadBlocks.cpp:153 `new CTaskComplexWanderCop(PEDMOVE_STILL, CGeneral::GetRandomNumberInRange(8ui8))`
- Original bytes (only for pedType == 6 COP, 0x461627): `dir = _ftol(((_rand() & 0xFFFF) * 3.0517578e-05f [0x858B14]) * 8.0f
  [0x859000])` (0x461658-0x46167E: `fild; fmul 1/32768; fmul 8`), passed to CTaskComplexWander::Constructor 0x66F450
  (`__thiscall (moveState 1, dir, 1, 0.5f [0x86FCB0])`), then vtable 0x859C80 and fields +0x28..+0x4D zeroed (inlined
  CTaskComplexWanderCop ctor). Index 0..7.
- Verdict: CONFIRMED (gr `(uint8)lerp(0, 7, rand()/32767)`, different mapping). Class: keep-original.
Found in passing (CONFIRMED, keep-original - gr logic error): non-cop peds (pedType != 6, 0x461795) pick the gang weapon inline,
not via gr's `GetRandomWeapon(false)` (uniform over the 3 slots): `r = _ftol(((_rand() & 0xFFFF) * 3.0517578e-05f) * 100.0f
[0x858628])` (0x46179E-0x4617BC), g = &Gang[pedType - 7] (weapons w1 +4, w2 +8, w3 +0xC; 0xC091F4/F8/FC):
if `w3 != 0`: `r < 33` -> w1, `r >= 66` -> w3, else w2 (0x4617CD-0x4617DB); else if `w2 != 0 && r >= 50` -> w2 else w1
(0x4617E3-0x4617F2). If the result != 0: `GiveDelayedWeapon(w, 25001)` 0x5E89B0 then
`SetCurrentWeapon(CWeaponInfo::GetWeaponInfo(w, 1)->m_nSlot (+0x14))` (0x461809-0x461818).

## 0x4629E0 CRoadBlocks::GenerateRoadBlocks  (gr_deviations = 1)
Original: `__cdecl void ()`, SEH frame.
Row RoadBlocks.cpp:239 `if (CGeneral::GetRandomNumberInRange(128u) >= FindPlayerWanted()->m_ChanceOnRoadBlock) continue;`
- Original bytes: `c = (u16)FindPlayerWanted(-1)->+0x1C` (0x462BDD), `r = _rand() & 0x7F` (0x462BE4-0x462BE9); `if (r >= c)`
  skip (signed `jge` 0x462BEE). This runs after `InOrOut[k] = 1` (0x462BD1), as in gr.
- Verdict: CONFIRMED (gr `(uint)lerp(0, 127, rand()/32767)` vs `rand() & 127`). Class: keep-original.
Found in passing (CONFIRMED, keep-original - gr logic errors):
1. Loop range: `a = (frame & 0xF) * 325` (0x462A74-0x462A77), `start = a / 16` (cdq/and/sar idiom = truncating division,
   0x462A7F-0x462A95), `end = min(NumRoadBlocks, (a + 325) / 16)` (NumRoadBlocks read through the obfuscated .text stub at
   0x402033, which loads `*(int*)0xA43580`; min at 0x462AAB-0x462AB3, signed). gr computes `end = (c + c % 16) / 16` with
   c = 325*(f%16+1): e.g. f%16 = 1 gives 41 instead of 40, so gr visits extra entries.
2. Pairing loop (node `k` = main, `i` = partner, 0x462EBC-0x4632D6): after the first successful
   `CreateRoadBlockBetween2Points(...)` + `InOrOut[i] = 1` (0x4631E4-0x4631EC) the original stops searching (`cmp ebx,
   NumRoadBlocks; jne next-main` 0x4631F7-0x4631FD, i < Num there). Only when NO partner was found (i reached NumRoadBlocks,
   also when NumRoadBlocks == 0) it creates the fallback single roadblock `CreateRoadBlockBetween2Points(main + dir*(w*0.5f),
   main - dir*(w*0.5f), 0)` (w = width from GetRoadBlockNodeInfo, dir = its direction, 0.5f [0x858B8C]; 0x463203-0x4632AE).
   gr keeps looping after a success (more than one roadblock per main node) and its fallback (`if (i == NumRoadBlocks)` inside
   the loop) can never run.
3. Range test: keep iff `-90 < d.x < 90`, `-90 < d.y < 90` (d = player - node, 0x462B1D-0x462B96; `0x859CE8 = -90.0f`,
   `0x85991C = 90.0f`) and `sqrt(d.y*d.y + d.x*d.x) < 90.0f` (y term first, 0x462B9C-0x462BBB); otherwise `InOrOut[k] = 0`
   (0x4632DB). gr equivalent up to the sum order.
