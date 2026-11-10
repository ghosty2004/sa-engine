# POP unattributed / unnamed functions (group P2)

Headings name the proposed class and method; evidence is listed in each section.

## 0x407670 CDirectory::`scalar deleting destructor' (proposed)  (catalogue: unknown/REA, port level 2)
- Convention / signature: `__thiscall CDirectory* (CDirectory* this = ECX, uint32 flags [esp+4])`, returns `this` in EAX, `ret 4`.
- Callers: none (no call sites and no dword reference to 0x407670 anywhere in the image): dead code (MSVC-generated deleting
  destructor that the linker kept). Callees: CDirectory::Destructor 0x5322D0 (thiscall: `if (m_pEntries (+0) && m_bOwnsEntries
  (+0xC byte)) free(m_pEntries)`), `_free` 0x8214BD.
- Behaviour: `CDirectory::~CDirectory(this)` (0x407673); `if (flags & 1) free(this)` (0x407678-0x407680, cdecl, `add esp,4`);
  return this.
- Evidence for the name: standard MSVC `??_G` shape (dtor call, `test [esp+8],1`, operator delete = `_free`, `ret 4`) around the
  CDirectory destructor. gta-reversed: absent. Open questions: none.

## 0x407D30 CStreaming::GetDefaultAmbulanceModel (proposed)  (catalogue: unknown/REA, port level 0)
- Entry `jmp 0x15674B0` (.text detour stub; same behaviour). `__cdecl int32 ()`: `return CStreaming::ms_aDefaultAmbulanceModel
  [CTheZones::m_CurrLevel]` (`eax = [0xBA6718]; eax = [eax*4 + 0x8A5AB4]`, 0x15674B0-0x15674BC). No bounds check.
- Callers: CCarCtrl::GenerateEmergencyServicesCar 0x42F9C0 (1). gta-reversed: no function (inlined as
  `ms_aDefaultAmbulanceModel[CTheZones::m_CurrLevel]`, Streaming.cpp:3097); name by analogy with GetDefaultMedicModel. Open questions: none.

## 0x407DC0 CStreaming::GetDefaultFireEngineModel (proposed)  (catalogue: unknown/REA, port level 0)
- Entry `jmp 0x156CE80` (detour stub). `__cdecl int32 ()`: `return CStreaming::ms_aDefaultFireEngineModel[CTheZones::m_CurrLevel]`
  (`[0xBA6718]`, table 0x8A5AD4, 0x156CE80-0x156CE8C).
- Callers: CCarCtrl::GenerateEmergencyServicesCar 0x42F9C0 (1). gta-reversed: inlined (Streaming.cpp:3168). Open questions: none.

## 0x426970 CCarCtrl::WeaveForPed (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: only caller CCarCtrl::WeaveThroughPedsSectorList 0x42D7E0; same shape as CCarCtrl::WeaveForObject 0x426BC0 (P1
  spec in CCarCtrl.md) and WeaveForOtherCar 0x426350 (`(CEntity*, CVehicle*, float*, float*)`, widens a blocked heading interval);
  `WeaveForPed` is the missing member of the WeaveFor* family (gr CarCtrl.h:156-163 declares WeaveForObject / WeaveForOtherCar and
  the three WeaveThrough*SectorList callers but no ped variant). Name is a proposal.
- Convention / signature: `__cdecl void (CPed* ped [esp+4], CVehicle* veh [+8], float* angleA [+0xC], float* angleB [+0x10])`,
  plain `ret` (0x426BAA / 0x426BB2). Frame `sub esp,8` + ebx/edi; below B = esp after `push edi` (0x42697F) = esp0-0x10:
  locals ang [B+8], dx [B+0xC]; the veh arg slot [B+0x18] is reused as scratch (dy, then the half-arc).
- Callers: 1 (0x42D7E0). Callees: CWorld::FindPlayerPed 0x56E210 (cdecl (-1)), CGeneral::GetATanOfXY 0x53CC70 (cdecl (x, y) -> ST0).
- Globals: CModelInfo::ms_modelInfoPtrs 0xA9B0C8. Constants (f32): 0x858624 = 1.0, 0x858FB0 = 2.4, 0x858C98 = 0.8, 0x858CC0 = -3.1415927,
  0x858CB8 = 3.1415927, 0x858CBC = 6.2831855, 0x858B50 = 0.0, 0x858B8C = 0.5.
- Fields: veh +0x3BA m_autoPilot.m_nCarMission (u8), +0x41C m_autoPilot.m_pTargetEntity (gr name m_TargetEntity), +0x22 model
  index (i16), +0x14 m_matrix / +0x04 placement (GetPosition); ped GetPosition. ColModel (modelinfo +0x14) +0x0C = boundBox.max.x.
- Behaviour:
  1. Early-outs: if `veh.mission == 3` (RAMPLAYER_CLOSE) and `ped == FindPlayerPed(-1)` return (0x426978-0x426992); if
     `veh.mission == 0x37` (KILLPED_CLOSE) and `ped == veh->+0x41C` return (0x426998-0x4269A7).
  2. `dx = Ped.x - Veh.x` stored f32 [B+0xC] (0x4269CA-0x4269D2); `dy = Ped.y - Veh.y` stored f32 [B+0x18] (0x4269E5-0x4269EF).
     `ang = GetATanOfXY(dx, dy)` stored f32 [B+8] (0x4269F9-0x4269FE).
  3. `dist = sqrt(dy*dy + dx*dx)` (y term first, from the f32 locals, kept in ST0, 0x426A02-0x426A17). If `dist < 1.0f` return
     (0x426A19-0x426A24, `test ah,5; jnp`: equal or NaN continue).
  4. `half = (veh colmodel boundBox.max.x * 2.4f + 0.8f) / dist` stored f32 [B+0x18] (0x426A2A-0x426A4D); dist popped.
  5. A side (ecx = angleA): `d = ang - *angleA` (0x426A53); `while (d < -pi) d += 2pi` (0x426A59-0x426A81; first test `test ah,5; jp`
     skips when not less or NaN); `while (d > pi) d -= 2pi` (0x426A83-0x426AA1, `test ah,0x41`: equal/NaN exit); `if (d < 0) d = -d`
     (0x426AA3-0x426AB0). Then `half = half * 0.5f` stored f32 (0x426AB2-0x426ABC; done here, once, before the first compare).
     If `d < half` (0x426AC0, strict, NaN false): `*angleA = ang - half` (fst 0x426AD3); if that value `< -pi` (0x426AD5): reload
     `*angleA` and `do x += 2pi while (x < -pi)`, store (0x426AE2-0x426B03).
  6. B side (ecx = angleB, 0x426B09): same normalisation of `d = ang - *angleB` (0x426B05-0x426B60). If `d < half` (0x426B62):
     `*angleB = half + ang` (fld half; fadd ang; fst, 0x426B6D-0x426B75); if that value `> pi` (0x426B77, `test ah,0x41`): reload and
     `do x -= 2pi while (x > pi)`, store (0x426B84-0x426BA4). Otherwise return.
- Differences from WeaveForObject: early-outs on mission/target instead of model, margin 0.8 (not 0.3), the `dist < 1.0` early return
  (so no division by zero), `ang` is stored to f32 before use, and the 0.5 scaling happens once before the A-side compare.
- Edge cases: an adjusted angle is wrapped only when it crosses the limit (single check, then loop); `ang - half` may be left below
  -pi only if the loop is skipped by NaN. Matrices are not allocated (GetPosition only).
- gta-reversed: absent (no function at 0x426970). Open questions: none.

## 0x610DB0 CPopulation::sub_610DB0 (proposed: CPedGroupPlacer::GetRandomGangSize)  (catalogue: unknown/REA, port level 0)
- Convention / signature: `__cdecl int32 ()`, EAX, plain `ret`. Frame `push ecx` (temp) + esi/edi.
- Callers: CPopulation::AddToPopulation 0x614720 (1 site, 0x614AC6, on the path where a gang group of 7..16 is being placed).
- Globals read: CPedGroupPlacer::ms_maxGangSize 0x86C6C8 (i32 = 5, .rdata), CPedGroupPlacer::ms_minGangSize 0x86C6C4 (i32 = 1, .rdata).
  Constant 0x858B14 = 3.0517578e-05f (1/32768).
- Behaviour: `r = rand() & 0xFFFF` (0x610DBF-0x610DC4); `return _ftol(((float)r * 3.0517578e-05f) * (float)(max - min)) + min`
  (fild r; fmul 0x858B14; fimul (max - min) int; `_ftol` truncation; `add eax, min`; 0x610DCD-0x610DE6) -> 1..4 with the shipped
  values (max itself is never returned).
- Evidence for the name: it is the original's inlined `CGeneral::GetRandomNumberInRange(ms_minGangSize, ms_maxGangSize)` as an
  out-of-line helper; gta-reversed has no function here (AddToPopulation 0x614720 is unhooked) - name is a proposal only.
- gta-reversed: absent. Open questions: none.
