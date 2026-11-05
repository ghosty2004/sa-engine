# CShopping helpers (WORLD)

Free functions used only by CShopping (Shopping.cpp in gta-reversed). Field names follow gta-reversed;
offsets were read from the bytes.

## 0x49abd0 GetChangingStatIndex  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl int32 GetChangingStatIndex(const char* name)`, arg `[esp+4]`, return
  EAX, plain `ret`. Stub `jmp 0x1565360` (`.HOODLUM` body). Preserves ESI/EDI; clobbers ECX, EDX.
- Callers: 1 (`CShopping::LoadStats` 0x49b6a0, 2 sites). Callees: none.
- Globals: string literals in `.rdata`: `"-"` 0x85a50c, `"fat"` 0x85a508, `"respect"` 0x85a500,
  `"sexy"` 0x85a4f8, `"health"` 0x85a4f0, `"stamina"` 0x85a4e8, `"calories"` 0x85a4dc.
- Behaviour: each test is `repe cmpsb` over `strlen(literal)+1` bytes (terminator included), i.e. exact
  string equality; the comparison stops at the first differing byte, so it never reads past the first
  mismatch of `name`. Tests in this order, first match wins:
  `"-"` → -1; `"fat"` → 0; `"respect"` → 1; `"sexy"` → 2; `"health"` → 3; `"stamina"` → 4;
  `"calories"` → 5; no match → -1 (computed as `((ne ? 1 : 0) - 1 & 6) - 1`).
- Edge cases: `name == NULL` crashes (reads byte 0). The explicit `"-"` test returns -1 like the
  no-match case (same result).
- gta-reversed: provided (Shopping.cpp:55), a loop with `strcmp` over the six names; same results (it
  lacks the explicit `"-"` test, which yields -1 either way). Equivalent.
- Open questions: none.

## 0x49b010 IsValidModForVehicle  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl bool IsValidModForVehicle(int32 modelId [esp+4], CVehicle* veh [esp+8])`,
  return AL (upper EAX bytes undefined: the false paths do `xor al, al` only; one true path leaves
  EAX = loop index with AL forced to 1 — callers test AL only). Plain `ret`. Stub `jmp 0x15691d0`.
- Callers: 1 (`CShopping::LoadShop` 0x49bbe0). Callees: `CVehicleModelInfo::GetNumWheelUpgrades`
  0x4c8740 (`__cdecl int32 (int32 wheelSet)`), `CVehicleModelInfo::GetWheelUpgrade` 0x4c8750
  (`__cdecl int32 (int32 wheelSet, int32 n)`).
- Note on the bytes: the first call to 0x4c8740 at 0x1569216..0x1569235 is HOODLUM-obfuscated
  (`call $+5; push esi; mov esi,[0x156053c]; not esi; xor esi,[0x665da4]; xchg [esp],esi;
  add [esp+4],0x1b; ret`): target = `~0x006ace49 ^ 0xffd9b6f6 = 0x4c8740`, return address 0x1569236,
  ESI restored to 0. It is an ordinary `GetNumWheelUpgrades(wheelSet)` call. The catalogue extent
  omits 0x1569236-0x1569259; that range is part of the function (disassembled for this spec).
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xa9b0c8 (read, indexed).
- Fields: `upg = ms_modelInfoPtrs[modelId]` (no range check), `flags = (u16) upg+0x12` (CBaseModelInfo
  `m_nFlags`; bit 0x100 = `bUsesVehDummy`, bits 0x7c00 = `CarMod` (5 bits at bit 10)).
  `vmi = ms_modelInfoPtrs[(i16) veh+0x22]` (`m_nModelIndex`, sign-extended). `vmi+0x4f` (i8,
  sign-extended: `m_nWheelUpgradeClass`), `vmi+0x2d6` (`int16 m_anUpgrades[18]`).
  `veh+0x1d3` (u8, `m_vehicleAudio.m_AuSettings.RadioType`), `veh+0x1be` (u8,
  `m_vehicleAudio.m_AuSettings.BassSetting`), `veh+0x42e` bit 0x10 (`bUpgradedStereo` in the vehicle flag
  bytes at 0x428..0x42f).
- Behaviour:
  1. If `flags & 0x100`:
     - if `(flags & 0x7c00) == 0x800` (dummy component `"wheel_"`, CarMod 2): `ws = (i8) vmi[0x4f]`;
       if `GetNumWheelUpgrades(ws) <= 0` (signed) return false; `i = 0`; loop:
       `if GetWheelUpgrade(ws, i) == modelId return true; i++; if i < GetNumWheelUpgrades(ws) (re-called
       every iteration, signed) continue; else return false`.
     - otherwise go to step 3.
  2. Else if `(flags & 0x7c00) == 0x4400` (chassis component `"stereo"`, CarMod 0x11):
     `if veh[0x1d3] != 0 return false; if veh[0x1be] != 1 return true; if veh[0x42e] & 0x10 return true;
     return false`.
  3. List check: for `i = 0 .. 17` ascending: `if (i32)(i16) vmi->m_anUpgrades[i] == modelId return true`;
     return false.
- Edge cases: `flags & 0x100` with CarMod != 2 goes straight to the list check (the `test ax,ax` at
  0x156925f is always true there). No null checks on either model info.
- gta-reversed: `plugin::CallAndReturn` wrapper only (Vehicle.cpp:5835). Nothing to compare.
- Open questions: none.

## 0x49b0e0 CShopping::sub_49b0e0 (proposed: `CShopping::AddUpgradeToPlayerVehicleAndFix`)  (catalogue: unknown/REA, port level 8)
- Evidence for the name: only caller is `CShopping::Buy` (0x49bf70); the body is exactly the block that
  gta-reversed inlines in `CShopping::Buy` for `PRICE_SECTION_CAR_MODS` (Shopping.cpp:113-148).
- Convention / signature: non-standard. **Register argument EDI = upgrade model id (int32)**; no stack
  arguments; plain `ret`; returns nothing meaningful (EAX = garbage). Preserves EBX, EBP, ESI, EDI
  (EDI is never written). A port hooked here must take the model id from EDI.
- Callees: `CWorld::FindPlayerVehicle` 0x56e0d0 (`__cdecl (int32 playerId = -1, bool = 0)`, caller pops
  8), `CVehicle::AddVehicleUpgrade` 0x6e3290 (`__thiscall (int32 modelId)`), `CAutomobile::FixTyre`
  0x6a3580 (`__thiscall (int32 wheel)`), `CAutomobile::FixPanel` 0x6a3670 (`__thiscall (int32 node,
  int32 panel)`), `CAutomobile::FixDoor` 0x6a35a0 (`__thiscall (int32 node, int32 door)`).
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xa9b0c8. Jump tables at 0x49b1bc (6 dwords) and 0x49b1d4
  (16 bytes) (data inside the function).
- Behaviour:
  1. `veh = FindPlayerVehicle(-1, false)` (pushes 0 then -1). No null check.
  2. `upg = ms_modelInfoPtrs[EDI]`; `vmi = ms_modelInfoPtrs[(i16) veh+0x22]` (both read before the next call).
  3. `veh->AddVehicleUpgrade(EDI)`.
  4. If `dword veh+0x590 != 0` (`m_nVehicleType`, 0 = automobile) return.
  5. `f = (u16) upg+0x12`. If `f & 0x100` (`bUsesVehDummy`): `id = (f >> 10) & 0x1f`; else
     `id = *(i32*)(vmi->m_pVehicleStruct(+0x5c) + ((f >> 10) & 0x1f) * 0x20 + 0xd0)`
     (`m_aUpgrades[CarMod].m_nParentComponentId`).
  6. `switch (u32)(id - 2)` (values above 15 → return):
     - id 2 (CAR_WHEEL_RF): `FixTyre(0); FixTyre(2); FixTyre(1); FixTyre(3)` in this order.
     - id 12: `FixPanel(12, 5)`. id 13: `FixPanel(13, 6)`.
     - id 16: `FixDoor(16, 0)`. id 17: `FixDoor(17, 1)`.
     - anything else: return.
- gta-reversed: inlined in `CShopping::Buy`; same calls and order (wheel order FL=0, FR=2, RL=1, RR=3).
  gr checks `veh->IsAutomobile()` which is the same `m_nVehicleType == 0` test.
- Open questions: none.
