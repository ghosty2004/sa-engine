# CStats (WORLD)

All 19 functions here are gta-reversed hooks with `.Reversed = false`; gr provides only
`plugin::Call` wrappers for them (Stats.cpp:54-72), so every statement below comes from the bytes.
All are `__cdecl` statics.

## Shared definitions

Globals (gr `StaticRef`s, Stats.h):
- `StatTypesFloat` 0xb79380 (`float[82]`, stat ids 0..81): `F[id]` = `0xb79380 + 4*id`.
- `StatTypesInt` 0xb79000 (`int32[223]`, stat ids 120..342): `I[id]` = `0xb79000 + 4*(id-120)`.
- `StatReactionValue` 0xb78f10 (`float[59]`, indexed by `eStatsReactions`): `R[name]`.
- `FavoriteRadioStationList` 0xb78e58 (`int32[14]`), `StatMessage` 0xb78200 (`tStatMessage[128]`, 0x10
  bytes: `+0 int16 stat_num`, `+2 u8 displayed`, `+3 u8 condition`, `+4 float value`, `+8 char text_id[8]`),
  `TotalNumStatMessages` 0xb794d0 (u32), `bStatUpdateMessageDisplayed` 0xb794d4 (u8),
  `bShowUpdateStats` 0x8cde56 (u8, `.data` init 1).
- Counters (u32): `m_CycleStaminaCounter` 0xb794dc, `m_CycleSkillCounter` 0xb794e0, `m_SwimStaminaCounter`
  0xb794e4, `m_SwimUnderWaterCounter` 0xb794e8, `m_DrivingCounter` 0xb794ec, `m_FlyingCounter` 0xb794f0,
  `m_FatCounter` 0xb794fc, `m_WeaponCounter` 0xb79504, `m_DeathCounter` 0xb79508, `m_MaxHealthCounter`
  0xb7950c, `m_AddToHealthCounter` 0xb79510, `m_LastWeaponTypeFired` 0xb79514.
- Unnamed file statics used here (no gr name): `g_LastSexAppealVehicle` 0xb79530 (`CVehicle*`, registered
  reference), `g_FatMuscleMsgState` 0xb79534 (u8), `g_RespectLast` 0x8cdec8 and `g_RespectThreshold`
  0x8cdec4 (f32, `.data` init -99.0f), and the `.data` (writable, not `.rdata`) float tuning table
  0x8cde58..0x8cdec0 used by GetFatAndMuscleModifier (initial values listed there). The names are
  proposals for the port's `globals!`; the addresses are what matters.
- `CTimer::ms_fTimeStep` 0xb7cb5c.

Constants (all f32 `.rdata`): `K1000` 0x858c4c = 1000.0, `K0` 0x858b50 = 0.0, `K1` 0x858624 = 1.0,
`K002` 0x858b38 = 0.02, `K2_32` 0x858c54 = 4294967296.0, `K0001` 0x858cdc = 0.001.

Recurring idioms (spelled out once):
- **`u32f(x)`**: `fild` of the dword `x` (exact), then if the dword is negative as int32
  `fadd K2_32` (this add is rounded to 24 bits under in-game PC). I.e. the u32 value as float.
- **`dtMs()`** = `_ftol((ms_fTimeStep * K002) * K1000)`: `fld; fmul; fmul` (24-bit PC rounding after each
  multiply) then CRT `_ftol` 0x821b40 (truncation toward zero of ST0, 64-bit result, EAX = low dword).
- **`Inc(stat, v)`** = `CStats::IncrementStat` 0x55c180 `(eStats stat (dword), float v)`; **`Msg(state,
  stat, v)`** = `CStats::DisplayScriptStatUpdateMessage` 0x55b980 `(int32 state, eStats stat, float v)`.
  The reaction value `v` is re-read from memory for each call (raw dword push).
- **"if `c > limit*1000`"**: `fld R[limit]; fmul K1000; fcompp` (ST0 = limit*1000, ST1 = c) then
  `test ah,5; jp` → the "else" branch is taken when `limit*1000 > c`, `==`, or unordered; the "then"
  branch only when `c > limit*1000` strictly.
- Compare spelling: `test ah,0x41; je X` jumps when ST0 > operand; `test ah,0x41; jne X` jumps when ST0 <
  operand, equal, or unordered; `test ah,5; jp X` jumps when ST0 > operand, equal, or unordered;
  `test ah,1; jne X` jumps when ST0 < operand or unordered.

## 0x558ec0 CStats::PopulateFavoriteRadioStationList  (catalogue: specified/REA, port level 2)
- Signature: `bool ()`, return AL; plain `ret`.
- Callers: `ConstructStatLine` (2), `CStats::Init` 0x55c0c0, `CStats::Save` 0x5d3b40.
  Callee: `CAudioEngine::GetRadioStationListenTimes` 0x507020 (`__thiscall`, ECX = `&AudioEngine` 0xb6bc90,
  returns `int32*` to 14 listen times).
- Behaviour: `src = GetRadioStationListenTimes(); allZero = true;` then for `i = 0..13` in order:
  `FavoriteRadioStationList[i] = src[i]; if (src[i] != 0) allZero = false;` (unrolled 7 per iteration,
  2 iterations; each element read once, stored, then tested). Return `allZero`.
- Open questions: none.

## 0x558fa0 CStats::FindMostFavoriteRadioStation  (catalogue: specified/REA, port level 0)
- Signature: `int32 ()` (eRadioID), EAX; plain `ret`. Callers: script commands 2500-2599 0x47a760,
  `CAEAmbienceTrackManager::UpdateAmbienceTrackAndVolume` 0x4d6e60, `ConstructStatLine`.
- Behaviour: `best = 0 (int32); res = 1; for (i = 1; i <= 12; i++) if (List[i] > best) { best =
  List[i]; res = i; } return res;` Signed compare, strict (first maximum wins). Indices 0 and 13 are
  never examined; if no entry is > 0 the result is 1.
- Open questions: none.

## 0x559010 CStats::FindLeastFavoriteRadioStation  (catalogue: specified/REA, port level 0)
- Signature: `int32 ()`, EAX. Caller: `ConstructStatLine`.
- Behaviour: `least = List[1]; res = 1; for (i = 1; i <= 12; i++) if (List[i] < least) { least =
  List[i]; res = i; } return res;` Signed, strict (first minimum wins); indices 0 and 13 ignored.
- Open questions: none.

## 0x559230 CStats::BuildStatLine  (catalogue: specified/REA, port level 4)
- Signature: `void (const char* key [ebp+8], void* p1 [ebp+0xc], int32 metrics [ebp+0x10], void* p2
  [ebp+0x14], int32 type [ebp+0x18])`, frame `ebp`, plain `ret`. Caller: `ConstructStatLine` (4 sites).
- Callees: `CText::Get` 0x6a0050 (`__thiscall`, ECX = `&TheText` 0xc1b340, `(const char* key)`, ret 4),
  `GxtCharToAscii` 0x69f7e0 (`__cdecl char* (const GxtChar*, u8 start = 0)`), CRT `sprintf` 0x821bb5
  (`__cdecl`), `CLocalisation::Metric` 0x56d220 (`bool ()`), `GxtCharStrcpy` 0x718660 (`(dst, src)`),
  `0x719240` (`__cdecl void (char* s)`: removes every `~...~` token from `s` in place; uncatalogued name),
  `Common::AsciiToGxtChar` 0x718600 (`(const char* src, GxtChar* dst)`).
- Globals: `gString2` 0xb71510 (char buffer, written), `gGxtString` 0xc1b100, `gGxtString2` 0xc1aed8.
- HOODLUM note: the common `sprintf` tail at 0x5594ca jumps to `0x401711`, an obfuscated
  `push 0xb71510` (`[0x156066c] + [0x62f393] + CF(1) = 0xb71510`), then back to 0x5594cf `call sprintf;
  add esp,0x10`. Treat it as `sprintf(gString2, fmt, <8 bytes of args>)`.
- Behaviour (`FEST_OO`, `ST_MILE`, `ST_SECS` are GXT keys; `A(key)` = `GxtCharToAscii(TheText.Get(key), 0)`,
  evaluated before the sprintf; float values are `(double)*(float*)p` = exact widening):
  1. `if (key == NULL) return;` `gString2[0] = 0`.
  2. If `type == 1` (time): `v1 = *(int32*)p1; v2 = *(int32*)p2;` `sprintf(gString2, v2 < 10 ? "%d:0%d" :
     "%d:%d", v1, v2)` (signed compare). No null checks.
  3. Else if `p2 != NULL` (range "x to y"):
     - `metrics == 0`: `sprintf(gString2, " %d %s %d", *(int*)p1, A("FEST_OO"), *(int*)p2)`.
     - `metrics == 1`: `sprintf(gString2, "%.2f %s %.2f", f(p1), A("FEST_OO"), f(p2))`.
     - `metrics == 3`: `sprintf(gString2, "$%.2f %s $%.2f", f(p1), A("FEST_OO"), f(p2))`.
     - any other value: nothing (gString2 stays "").
  4. Else if `p1 != NULL` and `(u32)metrics <= 9` (jump table 0x559510):
     0 `"%d"` int; 1 `"%.2f"` float; 2 `"%0.2f%%"` float; 3 `"$%.2f"` float; 4 `"%d|"` int;
     5 weight: `v = *(int*)p1; if (CLocalisation::Metric()) sprintf(gString2, "%dkgs", (double)((float)v *
       0.4536f [0x863f0c]))` **(original bug: a double is passed for `%d`; sprintf prints the low 32 bits of
       the double. With 24-bit PC the product has a 24-bit significand, so those bits are 0 and the
       output is "0kgs" for any v; reproduce by formatting the low dword of the f64 bits)**; else
       `sprintf(gString2, "%dlbs", v)`;
     6 `"%.2f %s"` float, `A("ST_MILE")`; 7 `"%.2fm"` float; 8 `"%.2fft"` `(double)(f * 3.3333333f
       [0x859f60])` (product rounded to 24 bits); 9 `"%d %s"` int, `A("ST_SECS")`.
     metrics > 9 or p1 == NULL: nothing.
  5. Always (when key != NULL): `GxtCharStrcpy(gGxtString, TheText.Get(key)); 0x719240(gGxtString);
     AsciiToGxtChar(gString2, gGxtString2)`.
- Edge cases: type 1 is tested before p2, so type 1 dereferences p2 even if NULL. All sprintf targets are
  unbounded.
- Open questions: none.

## 0x559760 CStats::CheckForStatsMessage  (catalogue: specified/REA, port level 8)
- Signature: `void ()`, plain `ret`. 11 callers (IncrementStat, DecrementStat, SetStatValue, ... ,
  several as tail calls).
- Callees: `CPad::GetPad` 0x53fb70 (`(int32 0)`), `CHud::HelpMessageDisplayed` 0x588b50 (`bool ()`),
  `CStats::GetStatValue` 0x558e40 (`float (eStats)`, result ST0), `CText::Get` 0x6a0050,
  `CHud::SetHelpMessage` 0x588be0 (`(const GxtChar*, bool quick, bool permanent, bool addToBrief)`).
- Globals: `TheCamera+0x3d` 0xb6f065 (u8 `m_bWideScreenOn`), `CCutsceneMgr::ms_cutsceneProcessing`
  0xb5f852 (u8), `CMenuSystem::num_menus_in_use` 0xba82e3 (u8), StatMessage/Total/bStatUpdateMessageDisplayed.
- Behaviour:
  1. Return if `GetPad(0)->JustOutOfFrontEnd (byte +0x117) != 0`; if `bShowUpdateStats == 0`; if
     `m_bWideScreenOn != 0`; if `ms_cutsceneProcessing != 0`; if `CHud::HelpMessageDisplayed()`; if
     `num_menus_in_use != 0` — in this order.
  2. `for (i = 0; i < 128; i++)`: `if (i >= TotalNumStatMessages) break` (unsigned, re-read each pass);
     `m = &StatMessage[i]`; skip if `m->displayed`. `v = GetStatValue((u16) m->stat_num)` (zero-extended).
     - `condition != 0` ("morethan"): show if `v >= m->value` (skip when `v < value` or NaN).
     - `condition == 0` ("lessthan"): show if `v <= m->value` (skip when `v > value` or NaN).
     Show: `m->displayed = 1; CHud::SetHelpMessage(TheText.Get(m->text_id), false, false, false);
     bStatUpdateMessageDisplayed = 1;` and **continue the loop** (several messages can be issued in one
     call; each later one sees `HelpMessageDisplayed` only at the next call).
- Open questions: none.

## 0x559860 CStats::LoadStatUpdateConditions  (catalogue: specified/REA, port level 2)
- Signature: `void ()`; `ebp` frame, 8-aligned stack, locals: `id` (int32, init 0), `value` (f32 bits,
  init 0), `count` (int32), buffers `name` (at B+0x38, 0x50 bytes), `cond` (B+0x2c, 12 bytes), `text`
  (B+0x24, 8 bytes) — adjacent on the stack in the order text, cond, name.
- Callees: `CFileMgr::SetDir("")` 0x5387d0, `CFileMgr::OpenFile("DATA\\STATDISP.DAT", "rb")` 0x538900,
  `CFileLoader::LoadLine(FILE)` 0x536f80, CRT `sscanf` 0x8220ad, `CFileMgr::CloseFile` 0x5389d0.
- Behaviour:
  1. `SetDir(""); f = OpenFile("DATA\\STATDISP.DAT", "rb"); TotalNumStatMessages = 0; count = 0;
     p = &StatMessage[0];` (no check that `f` is valid).
  2. `for (line = LoadLine(f); line; line = LoadLine(f))`: skip if `line[0] == '#'` or `line[0] == 0`.
     `sscanf(line, "%d %s %s %f %s", &id, name, cond, &value, text)` (result ignored; fields not
     matched keep the previous line's values).
     `p->stat_num = (int16) id; p->displayed = 0;`
     `if (strcmp(cond, "lessthan") == 0) p->condition = 0; else if (strcmp(cond, "morethan") == 0)
     p->condition = 1;` (otherwise `condition` keeps its old byte). Comparisons are exact 9-byte compares
     including the terminator.
     `p->value = value (raw bits); strcpy(p->text_id, text)` (byte loop until NUL, unbounded);
     `count++; p++`.
  3. `TotalNumStatMessages = count; CloseFile(f)`.
- Edge cases: no limit of 128 entries; text longer than 7 chars overflows into the next entry.
- Open questions: none.

## 0x559af0 CStats::GetFatAndMuscleModifier  (catalogue: specified/REA, port level 1)
- Signature: `float (eStatModAbilities m [esp+4])`, result **ST0**, plain `ret`. 27 callers.
- Callees: `0x420800` = `max0(a, b)`: `__cdecl float (float a, float b)` returning `(a > b) ? a : b`
  (NaN → b), called as `(0.0f, x)`; `0x404330` = `min1(a, b)`: returns `(a < b) ? a : b` (NaN → b),
  called as `(1.0f, x)`. Both return ST0 loaded from their f32 argument slot; the caller stores `x` to
  an f32 slot first (rounding).
- Notation: `FAT = F[21]`, `STA = F[22]`, `MUS = F[23]`, `MAXH = F[24]`, `DRV = I[160]`, `LUNG = I[225]`,
  `BIKE = I[229]`, `CYC = I[230]` (ints loaded with `fild`). Tuning floats (`.data`, re-read each use):
  `FB` 0x8cdec0 = 200.0, `MB` 0x8cdeb8 = 50.0; others named by address below with their initial value.
  `fatN(k) = ((FAT - FB) * k) / (K1000 - FB)`; `musN(k) = ((MUS - MB) * k) / (K1000 - MB)` — operation
  order exactly: `fld X; fsub B; fmul k; fld 1000; fsub B; fdivp` (= first / second).
  Sums are left to right as written. All x87 with 24-bit PC.
- Behaviour: `ST0 = 1.0` is loaded first; `if ((u32)m > 15) return 1.0` (that value). Otherwise the
  1.0 is popped and (jump table 0x559f5c):
  - 0: `if (FAT > 800.0f [0x8594dc]) return 2.0f; if (FAT > 400.0f [0x85a700]) return 1.0f; return 0.0f`.
  - 1: `t = (f32)((FAT - FB) / (K1000 - FB)); r = max0(0, t) * [0x8cdebc]=-0.4 + 1.0 + musN([0x8cdeb4]=0.2)`;
       `return (r < 0.7f [0x858cb0]) ? 0.7f : r` (NaN → r).
  - 2: same shape with `[0x8cdeb0]=-0.2` and `musN([0x8cdeac]=-0.1)`; floor `0.8f` [0x858c98].
  - 3: `r = fatN([0x8cdea8]=-0.2) + 1.0 + musN([0x8cdea4]=-0.1)` (no max0); floor `0.8f`.
  - 4: `r = fatN([0x8cdea0]=0.5) + 1.0 + musN([0x8cde9c]=1.0)`; `return (r > 2.0f) ? 2.0f : r`.
  - 5: `r = fatN([0x8cde98]=-1.0) + 1.0 + musN([0x8cde94]=0.5) + (STA * K0001) * [0x8cde90]=0.5
       + ((float)CYC * K0001) * [0x8cde8c]=1.0`; `if (r > 2.0f) return 2.0f; if (r < 0.25f [0x858c84])
       return 0.25f; return r`.
  - 6: `r = fatN([0x8cde88]=-0.5) + 1.0 + ((float)CYC * K0001) * [0x8cde84]=1.0`; `> 2.0 → 2.0`;
       `< 0.5f [0x858b8c] → 0.5`.
  - 7 (TIME_CAN_RUN): `return (STA * K0001) * [0x8cde80]=3000.0 + 150.0f [0x858a28]`.
  - 8 (AIR_IN_LUNG): `return (((float)LUNG + STA) * 0.0005f [0x859978]) * [0x8cde7c]=3000.0 + 1000.0f`.
  - 9 (MAX_HEALTH): `return (MAXH * K0001) * [0x8cde78]=176.0`.
  - 10: `return [0x8cde78]` (176.0).
  - 11: `return min1(1.0, (f32)((float)BIKE * K0001)) * [0x8cde74]=0.5 + 1.0`.
  - 12: same with `[0x8cde70]=0.3`. 13: same with `[0x8cde6c]=0.25`.
  - 14 (DRIVING_SKILL): `r = fatN([0x8cde68]=0.5) + (((float)DRV * K0001) * [0x8cde64]=0.5 + 1.0)`;
       `return (r > 1.0f) ? 1.0f : r`.
  - 15: `r = fatN([0x8cde60]=-0.5) + 1.0 + musN([0x8cde5c]=-0.25)`; `return (r < [0x8cde58]=0.5) ?
       [0x8cde58] : r`.
  Clamp compares are against the unrounded register value; the returned value is the register (callers
  round when they store).
- Note `BIKE` = `I[229]` (0xb791b4) is `STAT_BIKE_SKILL` per the gr enum.
- Open questions: none.

## 0x55a210 CStats::FindCriminalRatingString  (catalogue: specified/REA, port level 4)
- Signature: `const GxtChar* ()`, EAX = `TheText.Get(key)` for the selected key; plain `ret`.
  Callers: `CMenuManager::PrintStats` 0x574900, `CMenuManager::SaveStatsToFile` 0x57dde0.
- Callees: `CStats::FindCriminalRatingNumber` 0x559080 (`int32 ()`), `CText::Get`, `_ftol`.
- Behaviour: `r = FindCriminalRatingNumber()` (signed).
  - `r < 0`: `r > -500` → "RATNG53"; `r > -2000` → "RATNG54"; `r > -4000` → "RATNG55"; `r > -6000` →
    "RATNG56"; else "RATNG57".
  - `r >= 0`: first row with `r < bound` wins:
    20 RATNG1, 50 RATNG2, 75 RATNG3, 100 RATNG4, 120 RATNG5, 150 RATNG6, 200 RATNG7, 240 RATNG8,
    270 RATNG9, 300 RATNG10, 335 RATNG11, 370 RATNG12, 400 RATNG13, 450 RATNG14, 500 RATNG15,
    550 RATNG16, 600 RATNG17, 610 RATNG18, 650 RATNG19, 700 RATNG20, 850 RATNG21, 1000 RATNG22,
    1005 RATNG23, 1150 RATNG24,
    1300 → `(I[STAT_TIMES_BUSTED] (0xb79034) > 0) ? "RATNG25" : "RATNG24"` (int compared as float with 0.0),
    1500 RATNG26, 1700 RATNG27, 2000 RATNG28, 2100 RATNG29, 2300 RATNG30, 2500 RATNG31, 2750 RATNG32,
    3000 RATNG33, 3500 RATNG34, 4000 RATNG35, 5000 RATNG36, 7500 RATNG37, 10000 RATNG38, 20000 RATNG39,
    30000 RATNG40, 40000 RATNG41, 50000 RATNG42, 65000 RATNG43, 80000 RATNG44, 100000 RATNG45,
    150000 RATNG46, 200000 RATNG47, 300000 RATNG48, 375000 RATNG49,
    500000 → `k = _ftol(((float)I[STAT_FLIGHT_TIME] (0xb790c4) * 1.6666667e-05f [0x863fd0]) *
    0.016666668f [0x859044]); (k > 10) ? "RATNG50" : "RATNG49"`,
    1000000 → "RATNG51";
    `r >= 1000000`: `(Players[PlayerInFocus].m_nDisplayMoney (dword 0xb7ce54 + PlayerInFocus*0x190) >
    10000000) ? "RATNG52" : "RATNG51"` (`CWorld::PlayerInFocus` 0xb7cd74 u8; signed compare).
- Open questions: none.

## 0x55a780 CStats::ConstructStatLine  (catalogue: specified/REA, port level 5)
Confidence: **low** — the skeleton, the generic formats and every special handler below were traced
from the bytes, but this 1039-instruction function was specified under time pressure; the open
questions list the places that need a second read before porting.

- Signature: `int32 (int32 lineIndex [esp+4], u8 category [esp+8])`, return EAX, plain `ret`.
  Entry `0x55a780: jmp 0x40211f` (`movzx eax, byte [esp+8]; jmp 0x55a785`) — HOODLUM detour, no effect.
  Return value: **0** when the requested line was built (output in `gGxtString` = label,
  `gGxtString2` = value); otherwise, after the table ends, the **number of lines** of the category.
  The arg slot `[esp+8]` is reused as a byte temp.
- Callers: `CMenuManager::PrintStats` 0x574900, `CMenuManager::SaveStatsToFile` 0x57dde0 (2 each).
- Callees: see catalogue list (GetStatValue 0x558e40, BuildStatLine 0x559230, ConvertToMins 0x559540,
  ConvertToSecs 0x559560, GetPercentageProgress 0x5591e0, PopulateFavoriteRadioStationList,
  Find{Most,Least}FavoriteRadioStation, `CAudioEngine::GetRadioStationNameKey` 0x507010 (`__thiscall`,
  ECX = &AudioEngine, `(int32 id, char* outKey)`), CLocalisation::GermanGame 0x56d200 / Metric 0x56d220,
  CWorld::FindPlayerPed 0x56e210 (`(int32 -1)`), `CPed::GetWeaponSkill(eWeaponType)` 0x5e3b60
  (`__thiscall`, returns i8), CText::Get, GxtCharToAscii 0x69f7e0, GxtCharStrcpy 0x718660, 0x719240
  (strip `~..~`), AsciiToGxtChar 0x718600, sprintf 0x821bb5, `_ftol`).
- HOODLUM: `0x55b088: jmp 0x401240` is an obfuscated `push 0x859508` ("%d"; `[0x1560610] -
  [0x594972] - CF(0)`), then `0x55b08d: push gString2; gString2[0]=0; sprintf; add esp,0xc; jmp 0x55a8d4`.
  So "INT(v)" below = `sprintf(gString2, "%d", v)`.
- Data: category tables (6-byte entries `{int16 id; u8 type; u8 alwaysShow; u8 special; u8 pad}`,
  terminated by `id == -99`): category 0 → 0x8cd8a0, 1 → 0x8cd938, 2 → 0x8cd9b0, 3 → 0x8cd9d8,
  4 → 0x8cda70, 5 → 0x8cdac8, 6 → 0x8cdbf8, any other → 0x8cde08 (`.data`; read from the image).
  `gString` 0xb71670, `gString2` 0xb71510, `gGxtString` 0xc1b100, `gGxtString2` 0xc1aed8,
  `CStats::m_ThisStatIsABarChart` 0xb794cc (int16, written).
- Notation: `n` = running line counter (ESI, starts 0). "**Line(L, V)**" = the standard check
  `if (n != lineIndex) { n++; next entry; }` else produce the line and return 0. Produce forms:
  - **STD**: `gString2` formatted as stated, then (0x55a8d4) `GxtCharStrcpy(gGxtString, TheText.Get(gString));
    0x719240(gGxtString); AsciiToGxtChar(gString2, gGxtString2)`.
  - **HDR** (0x55b60d): `gString2 = ""` then STD tail (label = stat name, empty value).
  - **TXT(key)** (0x55b68f): `gGxtString[0] = 0` (empty label) and `GxtCharStrcpy(gGxtString2,
    TheText.Get(key))`.
  `V(id)` = `GetStatValue(id)`; `IV(id)` = `_ftol((float) I[id])`; `A(key)` = GxtCharToAscii(TheText.Get(key), 0).
- Behaviour:
  1. Select the table; `idx = 0`. Loop while `entry->id != -99` (checked before the first entry too):
  2. **Visibility**: if `entry->alwaysShow == 0`: `v = ((u16)id < 82) ? F[id] : (float)*(int32*)(0xb78e20
     + 4*(u16)id)`; if `!(v > 0.0f)` → skip entry (no count).
  3. `gString = sprintf(id < 10 ? "STAT00%d" : id < 100 ? "STAT0%d" : "STAT%d", id)` (signed, id
     sign-extended).
  4. If `GermanGame()` and the **low byte** of id is one of 167, 168, 177, 205, 206, 207, 208, 209
     (rampage stats) → skip entry.
  5. `m_ThisStatIsABarChart = 0`.
  6. If `entry->special == 0` — generic by `entry->type`:
     1 `"%.2f"` (double)V; 2 and 8 and any type outside 1..10: INT(`id < 82 ? _ftol(F[id]) : IV(id)`);
     3 `"$%.2f"` V; 4 `"%d|"` _ftol(V); 5 weight: `w = _ftol(V)`, `Metric() ? "%dkgs" with (double)((float)w *
     0.4536f) (same low-dword bug as BuildStatLine → "0kgs") : "%dlbs" w`;
     6 `"%.2f %s"` V, A("ST_MILE"); 7 `Metric() ? "%.2fm" V : "%.2fft" (V * 3.3333333f)` (Metric is
     called **before** the Line check here);
     9 time: `mins = ConvertToMins(_ftol(V)); secs = ConvertToSecs(_ftol(V))` (GetStatValue called twice),
     `BuildStatLine(gString, &mins, 0, &secs, 1)`, return 0 (BuildStatLine does the gxt conversion);
     10 bar chart: `m_ThisStatIsABarChart = (i16) id` (**before** the Line check), then INT(_ftol(V)).
     All via Line(…, STD) except 9.
  7. If `entry->special != 0` — by id:
     - `0xfc <= id <= 0x101`: visible only if `v > 0` (re-tested as in step 2); Line: `"%0.2f%%"` V.
     - `0xf4..0xfb`, `0x102..0x13f`, `> 0x151`: skip entry.
     - 0 PROGRESS_MADE: `"%0.2f%%"` GetPercentageProgress().
     - 164 ARMOR: INT(`_ftol(CWorld::Players[0].m_pPed->m_fArmour (+0x548))`) (`[0xb7cd98]` = ped ptr).
     - pairs, `" %d %s %d"` (first, A("FEST_OO"), second): 213 (IV(213), 30); 174 (IV(174), 12);
       175 (IV(175), 25); 231 (IV(231), IV(232)); 243 (IV(243), IV(244)); 144 (IV(144),
       CStuntJumpManager::m_iNumJumps 0xa9a89c); 145 (IV(145), m_iNumJumps); 241 (IV(241), IV(242));
       322 TAGS_SPRAYED (CTagManager::ms_numTagged 0xa9ad74, ms_numTags 0xa9ad70).
     - 143 BEST_INSANE_STUNT: two lines: `if (n == lineIndex) HDR; n++;` `k = IV(143) - 1`; Line → TXT of
       {0:"INSTUN",1:"PRINST",2:"DBINST",3:"DBPINS",4:"TRINST",5:"PRTRST",6:"QUINST",7:"PQUINS"}[k],
       any other k → "NOSTUC".
     - 169 FLIGHT_TIME: `t = _ftol((float)I[169] * 1.6666667e-05f)`; `q = t / 60, r = t % 60` (signed
       idiv); Line → `BuildStatLine(gString, &q, 0, &r, 1)`.
     - 173 TIME_ON_JETPACK: same with I[173], but skip the entry if `q <= 0 && r <= 0`.
     - 320 PLAYING_TIME: `m = (u32)CTimer::m_snTimeInMilliseconds / 60000` (magic 0x45e7b273, `shr 14`),
       `h = m / 60, mm = m % 60` (signed); Line → `BuildStatLine(gString, &h, 0, &mm, 1)`.
     - 323 LEAST_FAVORITE_GANG: `best = 0; g = 0; for (i = 7; i < 15; i++) if ((u32)PedsKilledOfThisType[i]
       > best) { best = value; g = i; }`; if `g == 0` skip; `if (n == lineIndex) HDR; n++;` Line →
       TXT("ST_GNG" + (g-7)) i.e. ST_GNG0..ST_GNG7.
     - 324 GANG_MEMBERS_WASTED: INT(sum of PedsKilledOfThisType[7..15]) (int32 wrapping).
     - 325 CRIMINALS_WASTED: INT(PedsKilledOfThisType[20]) (0xb78ee0).
     - 326 / 327 radio: `if (PopulateFavoriteRadioStationList()) skip` (all zero); `if (n == lineIndex) HDR;
       n++; GetRadioStationNameKey(FindMost…() / FindLeast…(), localKey)` (called even if the next line is
       not requested); Line → TXT(localKey).
     - 328 CURRENT_WEAPON_SKILL: `p = FindPlayerPed(-1); w16 = (u16) p->m_aWeapons[(i8)p->m_nActiveWeaponSlot
       (+0x718)].m_eWeaponType (+0x5a0 + slot*0x1c)`; `if (w16 == 32) b = 7 else if (w16 < 22 || w16 > 32)
       skip; else b = (u8)(w16 - 21)`. If `n == lineIndex`: INT(_ftol(GetStatValue(b + 68))) with label
       gString (the entry's STATxxx), STD, return 0. Else `n++`; re-read the weapon type as a **dword**;
       if it equals 32 → `b = 11`; `gString = sprintf(b < 10 ? "STWE00%d" : b < 100 ? "STWE0%d" : "STWE%d", b)`;
       Line → TXT(gString).
     - 329 WEAPON_SKILL_LEVELS: `if (n == lineIndex) HDR (via 0x55b626, same as HDR); n++;` then
       `for (i = 1; i <= 10; i++)`: `gString = sprintf("STWE00%d"/"STWE0%d"/"STWE%d", i)`;
       `s = (i8) FindPlayerPed(-1)->GetWeaponSkill(i + 21)`; `gString2 = s == 1 ? "WS_STD" : s == 2 ?
       "WS_PRO" : "WS_POOR"` (sprintf with the key as format); if `n == lineIndex`: `gGxtString =
       TheText.Get(gString)` (no strip), `gGxtString2 = TheText.Get(gString2)`, return 0; else `n++`.
       After the loop continue with the next entry (no extra increment).
     - 330 PILOT_RANKING: `t = _ftol((float)I[169] * 1.6666667e-05f); q = t / 60; r = t % 60`.
       If `q > 0 || r >= 5`: `if (n == lineIndex) HDR; n++`. Then the rank (each a Line → TXT):
       `q <= 0`: `r < 5` → skip; `r < 10` ST_PR01; `r < 20` ST_PR02; `r < 30` ST_PR03; else fall through;
       `q <= 1`: `r < 0` ST_PR04; `r < 30` ST_PR05; `q <= 2`: `r < 0` ST_PR06; `r < 30` ST_PR07;
       `q <= 3`: `r < 0` ST_PR08; `r < 30` ST_PR09; `q <= 4 && r < 0` ST_PR10; `q <= 5 && r < 0` ST_PR11;
       `q <= 10 && r < 0` ST_PR12; `q <= 20 && r < 0` ST_PR13; `q <= 25 && r < 0` ST_PR14;
       `q <= 30 && r < 0` ST_PR15; `q <= 49 && r < 2` ST_PR16; `q <= 50 && r < 0` ST_PR17;
       `q <= 100 && r < 0` ST_PR18; else ST_PR19. (Each test falls through to the next on failure; the
       `r < 0` tests can only succeed for negative t — original logic, keep it.)
     - 331 / 332 / 333 STRONGEST / 2ND / 3RD GANG: `gang = *(int32*)(0x96a60c + 4*id)`, `val =
       *(int32*)(0x96a600 + 4*id)`, `j = id - 331`. If `gang < 0` skip. `gString2 = sprintf("ST_GNG%d", gang)`;
       `gString = sprintf("ST_LAB%d", j)`; `lab = TheText.Get(gString)`. If `n == lineIndex`:
       `GxtCharStrcpy(gGxtString, lab)`, `gGxtString2 = TheText.Get(gString2)`, return 0. Else
       `gString = sprintf("%d", val); n++; AsciiToGxtChar(gString, gGxtString2)`; if `n == lineIndex`:
       `gGxtString[0] = 0; GxtCharStrcpy(gGxtString2, gGxtString2)` (self copy), return 0; else `n++`.
     - 334 MONEY_LOST_GAMBLING: `d = F[MONEY_SPENT_GAMBLING] - F[MONEY_WON_GAMBLING]; if (d < 0) d = 0`
       (NaN kept); Line: `"$%.2f"` (double)d.
     - 337 TERRITORY_UNDER_CONTROL: Line: `"%0.2f%%"` (double)(TerritoryUnderControlPercentage * 100.0f).
     - other ids in 0x140..0x151 (321, 335, 336, 338..341): skip.
  8. Next entry: `n` already incremented where stated; `idx++` (u16), `entry = table + idx*6`.
  9. End: return `n`.
- gta-reversed: `plugin::CallAndReturn` only.
- Open questions (all need a second byte-level read before porting):
  - 0x55a7e6-0x55a817 vs 0x55ae07: confirm that the `0xfc..0x101` re-test uses the same int-path
    address `0xb78e20 + 4*id` (it does at 0x55ae19) and that `alwaysShow` does not bypass it.
  - 0x55aa63: type 7 calls `Metric()` before the `n == lineIndex` check — confirm no other side effect
    ordering matters (Metric is pure).
  - 0x55b3cc-0x55b40f: first weapon-type read is a 16-bit `mov ax, word` and the `== 32` case sets
    `eax = 0x1c` so `b = 0x1c - 0x15 = 7`; double-check against the second read's `b = 11` mapping.
  - 0x55b528: confirm the identity of the tables 0x96ab38/0x96ab2c (gang ids / strengths; no gr name).
  - The exact per-handler placement of `n++` versus the HDR check was traced for 143, 323, 326/327,
    328, 329, 330, 331-333; the remaining handlers use the plain Line pattern (`jne 0x55b5a5`).

## 0x55bc50 CStats::UpdateRespectStat  (catalogue: specified/REA, port level 9)
- Signature: `void (u8 reset [esp+4])`, plain `ret`; 0x10 bytes of locals; **the argument slot is reused
  as an f32 temporary `X`** (`[esp+0x14]`/`[esp+0x18]`).
- Callers: `CGameLogic::Update` 0x442ad0, `CMenuManager::DoSettingsBeforeStartingAGame` 0x573330.
- Callees: `CTagManager::GetPercentageTagged` 0x49cda0 (`int32 ()`), `_ftol`, `CheckForStatsMessage`,
  `CStats::CheckForThreshold` 0x5595f0 (`bool (float* p, float v)`), `Msg`.
- Globals: `F[STAT_RESPECT=68]` 0xb79490, `I[STAT_RESPECT_MISSION]` 0xb791a0, `I[STAT_RESPECT_MISSION_TOTAL]`
  0xb791b0, `F[STAT_GIRLFRIEND_RESPECT]` 0xb79484, `F[STAT_CLOTHES_RESPECT]` 0xb79488, `F[STAT_MUSCLE]`,
  `CGangWars::TerritoryUnderControlPercentage` 0x96ab9c (f32), `CWorld::Players[PlayerInFocus].m_nMoney`
  (dword 0xb7ce50 + idx*0x190), `CCheat::m_aCheatsActive[CHEAT_MAX_RESPECT=79]` 0x96917f,
  write `F[STAT_TOTAL_RESPECT=64]` 0xb79480, `g_RespectLast` 0x8cdec8, `g_RespectThreshold` 0x8cdec4.
- Behaviour:
  1. If `reset != 0`: `g_RespectLast = -99.0f; g_RespectThreshold = -99.0f;` (0x8cdec8 first) return.
  2. x87 (24-bit PC), exact order:
     `a = F[68] * 0.4f [0x858ee8]`; `b = (float)I[RESPECT_MISSION] * 1000.0f`;
     `c = (float)I[RESPECT_MISSION_TOTAL]; if (1.0f > c) c = 1.0f`;
     `s1 = (a + (b / c) * 0.36f [0x863e10]) + F[GIRLFRIEND_RESPECT] * 0.03f [0x858b10]`;
     `t = TerritoryUnderControlPercentage - 0.2f [0x858cc4]; if (0.0f > t) t = 0.0f` (NaN kept);
     `X = (f32)(((t * 1250.0f [0x864314]) * 0.05f [0x858c28]) + s1)`.
  3. `m = (float)Players[PlayerInFocus].m_nMoney * 1e-7f [0x864310]; L0 = (1.0f < m) ? 1.0f : (f32) m`
     (NaN → m). `mus = F[MUSCLE]`, `clo = F[CLOTHES_RESPECT]` read now (before the call).
  4. `p = GetPercentageTagged()`; `k1 = (float)(p * 10) * 0.05f` (int multiply, wrapping).
     `T = k1 + ((((L0 * 1000.0f) * 0.05f) + X) + mus * 0.03f) + clo * 0.03f`; `X = (f32) T;`
     `if (T < 0.0f) X = 0.0f` (compare on the register value; NaN keeps X).
  5. If cheat MAX_RESPECT active: `X = 1000.0f`.
  6. `if (_ftol(g_RespectLast) == _ftol(X)) return;` (both truncated to int; `g_RespectLast` first).
  7. `F[TOTAL_RESPECT] = X; CheckForStatsMessage();`
  8. If `X > g_RespectThreshold` → **up path** (step 10). Else if `g_RespectLast < 2.0f` → up path.
     Else (**down path**): if `g_RespectLast != -99.0f` and `g_RespectThreshold != -99.0f` (exact
     compares with f32 -99.0 [0x86430c]; NaN counts as "not equal"): `if (CheckForThreshold(&g_RespectThreshold,
     X) || X < 2.0f) Msg(0, 64, X)`. Then `if (X > g_RespectLast) { g_RespectThreshold = X; g_RespectLast
     = X; return; }` else `g_RespectLast = X; return`.
  10. Up path: if both statics != -99.0f: `if (CheckForThreshold(&g_RespectThreshold, X) || g_RespectLast
     < 2.0f) Msg(1, 64, X)`. Then `if (X < g_RespectLast) g_RespectThreshold = X;` `g_RespectLast = X`.
     (The `|| ... < 2.0f` operand is skipped when CheckForThreshold returns true; NaN → false.)
- Open questions: none.

## 0x55bf20 CStats::UpdateSexAppealStat  (catalogue: specified/REA, port level 9)
- Signature: `void ()`; locals `L0` (f32, init 0.0), `L4` (f32); ends with a tail `jmp CheckForStatsMessage`.
- Callees: `CWorld::FindPlayerVehicle` 0x56e0d0 (`(int32 -1, bool 0)`), `CEntity::RegisterReference`
  0x571b70 (`__thiscall (CEntity** ref)`), `CWorld::FindPlayerCoors` 0x56e010 (`(CVector* out, int32 -1)`).
- Behaviour:
  1. `L4 = (f32)(F[STAT_APPEARANCE=80] (0xb794c0) * 0.5f)`; `L0 = 0.0f`.
  2. `if (FindPlayerVehicle(-1, 0)) { v = FindPlayerVehicle(-1, 0); g_LastSexAppealVehicle = v;
     v->RegisterReference(&g_LastSexAppealVehicle); }` (called twice; the stored pointer is the second).
  3. `v = g_LastSexAppealVehicle`. If `v != NULL`:
     `pos = v->m_matrix ? &v->m_matrix->pos (+0x30) : &v->m_placement.pos (+4)`; `pc = FindPlayerCoors(-1)`;
     `dx = pos.x - pc.x; dy = pos.y - pc.y; d = sqrt(dy*dy + dx*dx)` (x87 `fsqrt`, sum order `dy² + dx²`).
     If `d < 35.0f [0x85f070]` (strict; NaN fails):
       `L0 = (f32)((v->m_fHealth (+0x4c0) + same) - 1000.0f)` (`fadd st0,st0`);
       `h = (1000.0f < L0) ? 1000.0f : ((0.0f > L0) ? 0.0f : L0)`;
       `cls = (i8) CModelInfo::ms_modelInfoPtrs[(i16) v->m_nModelIndex]->m_nVehicleClass (+0x4d)`;
       factor by class: POORFAMILY(1), BIG(5), MOPED(7), BICYCLE(11) → `h *= 0.1f`; RICHFAMILY(2),
       EXECUTIVE(3), LEISUREBOAT(9) → no multiply; WORKER(4), TAXI(6), WORKERBOAT(10) → `h *= 0.3f`;
       MOTORBIKE(8) and every other value (incl. NORMAL 0, IGNORE -1) → `h *= 0.5f`. Go to step 4 with h.
     Else (no vehicle, or `d >= 35`, or NaN): `h = L0` (= 0.0f).
  4. `r = h * 0.5f + L4; if (r < 0.0f) r = 0.0f; else if (r > 1000.0f) r = 1000.0f;` (NaN stays).
     If cheat `CHEAT_MAX_SEX_APPEAL` (0x969180) active: `r = 1000.0f`.
     `F[STAT_SEX_APPEAL=25] (0xb793e4) = (f32) r`; tail-call `CheckForStatsMessage()`.
- Edge cases: `g_LastSexAppealVehicle` is never cleared here; it keeps pointing at the last vehicle (the
  registered reference nulls it when that vehicle is destroyed).
- Open questions: none.

## 0x55c470 CStats::UpdateFatAndMuscleStats  (catalogue: specified/REA, port level 10)
- Signature: `void (u32 value [esp+4])`. Callers: UpdateStatsWhenSprinting/Running/Cycling/Swimming/Fighting.
- Behaviour:
  1. If `u32f(m_FatCounter) > R[TIMELIMIT_FAT_ADJUST]*1000`:
     `m_FatCounter = 0`. If `FAT > 0.0f`:
       if `R[DEC_FAT] > 0.0f`: `f = FAT - R[DEC_FAT]; FAT = (f > 0.0f) ? f : 0.0f` (NaN → 0);
       `CheckForStatsMessage()`.
       `Inc(23 MUSCLE, R[INC_BODY_MUSCLE])`.
       `if (g_FatMuscleMsgState == 1) { g_FatMuscleMsgState = 2; Msg(1, 23, R[INC_BODY_MUSCLE]); }
       else { g_FatMuscleMsgState = 1; Msg(0, 21 FAT, R[DEC_FAT]); }`
     Else (`FAT <= 0` or NaN): if `R[DEC_BODY_MUSCLE] > 0.0f`: `m = MUS - R[DEC_BODY_MUSCLE];
       MUS = (m > 0) ? m : 0; CheckForStatsMessage();` then `g_FatMuscleMsgState = 3;
       Msg(0, 23, R[DEC_BODY_MUSCLE])`.
     Else (counter not over the limit): `m_FatCounter += (u32)(dtMs() * value) / 10` (32-bit wrapping
     multiply, unsigned divide).
  2. Then always: if `u32f(m_MaxHealthCounter) > R[TIMELIMIT_MAX_HEALTH]*1000`:
     `Inc(24, R[INC_MAX_HEALTH]); Msg(1, 24, R[INC_MAX_HEALTH]); m_MaxHealthCounter = 0;`
     else `m_MaxHealthCounter += dtMs()`.
- Open questions: none.

## 0x55c780 CStats::UpdateStatsWhenCycling  (catalogue: specified/REA, port level 11)
- Signature: `void (bool sprint [esp+4], CBmx* bmx [esp+8])`. No callers recorded (reached via
  gr-known caller paths not in the call graph).
- Callees: `UpdateFatAndMuscleStats`, `Inc`, `Msg`, `_ftol`, CRT `ceil` 0x823820 (`double (double)`).
- Behaviour:
  1. `g = fabs(bmx->m_fGasPedal (+0x49c))`. If `g > 0.0f`: `rate = sprint ? R[EXERCISE_RATE_CYCLE_SPRINT]
     : R[EXERCISE_RATE_CYCLE]`. Else (0 or NaN): if `!sprint` skip to step 3; else `rate =
     R[EXERCISE_RATE_CYCLE_SPRINT]`. `UpdateFatAndMuscleStats(_ftol(rate))`.
  2. If `u32f(m_CycleStaminaCounter) > R[TIMELIMIT_CYCLE_STAMINA]*1000`: `m_CycleStaminaCounter = 0;
     Inc(22, R[INC_CYCLE_STAMINA]); Msg(1, 22, R[INC_CYCLE_STAMINA])`; else `m_CycleStaminaCounter += dtMs()`.
  3. If `u32f(m_CycleSkillCounter) > R[TIMELIMIT_CYCLE_SKILL]*1000`: `m_CycleSkillCounter = 0;
     Inc(230, R[INC_CYCLE_SKILL]); Msg(1, 230, R[INC_CYCLE_SKILL]); return`.
  4. `s2 = (x*x + y*y) + z*z` of `bmx->m_vecMoveSpeed` (+0x44/+0x48/+0x4c; no sqrt).
     If `(u8) bmx->m_nNumContactWheels (+0x804) < 2` and `0.05f*0.05f < s2` (`fld 0.05; fmul 0.05`
     [0x863e58], rounded product): `k = 3.0f`.
     Else: recompute `s2`; if `0.2f*0.2f [0x863e54] < s2`: `k = sprint ? 1.5f : 1.0f`; else return.
  5. `if (k > 0.0f)` (always true): `m_CycleSkillCounter += _ftol(ceil((double)(k * u32f(dtMs()))))`
     — product `fmulp` = `k * n` rounded to 24 bits, widened to f64, `ceil`, `_ftol`.
- Open questions: none.

## 0x55c990 CStats::UpdateStatsWhenSwimming  (catalogue: specified/REA, port level 11)
- Signature: `void (bool underWater [esp+4], bool sprint [esp+8])`. Caller: `CTaskSimpleSwim::ProcessSwimAnims`
  0x6899f0.
- Behaviour:
  1. `rate = (underWater || sprint) ? R[EXERCISE_RATE_SWIM_SPRINT] : R[EXERCISE_RATE_SWIM]`;
     `UpdateFatAndMuscleStats(_ftol(rate))`.
  2. If `u32f(m_SwimStaminaCounter) > R[TIMELIMIT_SWIM_STAMINA]*1000`: `m_SwimStaminaCounter = 0;
     Inc(22, R[INC_SWIM_STAMINA]); Msg(1, 22, R[INC_SWIM_STAMINA])`; else `+= dtMs()`.
  3. If `underWater`: if `u32f(m_SwimUnderWaterCounter) > R[TIMELIMIT_BREATH_UNDERWATER]*1000`:
     `m_SwimUnderWaterCounter = 0; Inc(225 LUNG_CAPACITY, R[INC_BREATH_UNDERWATER]); Msg(1, 225, ...)`;
     else `+= dtMs()`.
- Note: the first argument's role is inferred from step 3 (gr names it `arg0`).
- Open questions: none.

## 0x55cac0 CStats::UpdateStatsWhenDriving  (catalogue: specified/REA, port level 10)
- Signature: `void (CVehicle* v [esp+4])`; the argument slot is reused as f32 temp `dc`.
  Caller: `CPlayerInfo::Process` 0x56f8d0 (2 sites).
- Behaviour:
  1. `car = (v->m_nVehicleType (+0x590) == 0) ? v : NULL` (CAutomobile or derived).
  2. `dc = (f32) u32f(m_DrivingCounter)` (stored, rounded). If `dc > R[TIMELIMIT_DRIVING_SKILL]*1000`
     (`fcomp` against the stored `dc`): `m_DrivingCounter = 0; Inc(160, R[INC_DRIVING_SKILL]);
     Msg(1, 160, R[INC_DRIVING_SKILL]); return`.
  3. `s = sqrt((x*x + y*y) + z*z)` of `v->m_vecMoveSpeed`.
     If `s > 0.8f [0x863e70]` → fast. Else if `car != NULL && car->m_nNumContactWheels (u8 +0x960) == 0`
     → fast. Else if `s > 0.2f [0x863e68]` → slow. Else return.
     fast: `m_DrivingCounter = _ftol(u32f(dtMs()) * 1.5f [0x863e74] + dc)`;
     slow: `m_DrivingCounter = _ftol(u32f(dtMs()) * 0.5f [0x863e6c] + dc)` (dtMs stored to a local first,
     then `u32f`; sum `product + dc`).
- Open questions: none.

## 0x55cc00 CStats::UpdateStatsWhenFlying  (catalogue: specified/REA, port level 10)
- Signature: `void (CVehicle* v [esp+4])`, arg slot reused as f32 `fc`. Caller: `CPlayerInfo::Process` (2).
- Behaviour:
  1. `if (v->m_nVehicleType (+0x590) != 0) return`.
  2. `fc = (f32) u32f(m_FlyingCounter)`; if `fc > R[TIMELIMIT_FLYING_SKILL]*1000`: `m_FlyingCounter = 0;
     Inc(223, R[INC_FLYING_SKILL]); Msg(1, 223, R[INC_FLYING_SKILL]); return`.
  3. `if (v->m_nNumContactWheels (u8 +0x960) != 0) return`.
  4. `s = sqrt((x*x + y*y) + z*z)` of move speed. If `s > 1.3f [0x863e80]` → fast. Else if
     `v->m_matrix->at.z (+0x14 → +0x28) < 0.0f` → fast (no null check on m_matrix). Else if `s > 0.5f
     [0x863e78]` → slow. Else return.
     fast: `m_FlyingCounter = _ftol(u32f(dtMs()) * 1.5f [0x863e84] + fc)`; slow: same with `0.5f [0x863e7c]`.
- Open questions: none.

## 0x55ceb0 CStats::UpdateStatsWhenWeaponHit  (catalogue: specified/REA, port level 10)
- Signature: `void (eWeaponType w [esp+4])`. Caller: `CWeapon::DoBulletImpact` 0x73b550.
- Callees: `CWeaponInfo::GetSkillStatIndex` 0x743cd0 (`int32 (eWeaponType)`), `CGameLogic::IsCoopGameGoingOn`
  0x441390, `Inc`, `Msg`.
- Behaviour:
  1. `st = GetSkillStatIndex(w)` (full EAX kept); `k = st - 69` (32-bit; skill index relative to
     STAT_PISTOL_SKILL). `v = ((u16)st < 82) ? F[(u16)st] : (float) I[(u16)st]` (int path reads
     `0xb78e20 + 4*(u16)st`), stored to an f32 local.
  2. `if (IsCoopGameGoingOn()) return;` `if (!(v < 1000.0f)) return` (≥ or NaN returns).
  3. `inc = &R[STAT_INC_PISTOL_SKILL + k]` (0xb78f40 + 4k). `Inc(st, *inc)`.
  4. If `m_LastWeaponTypeFired != k`: `m_LastWeaponTypeFired = k; m_WeaponCounter = 0; return`.
  5. Else: `c = u32f(m_WeaponCounter)`; if `c > R[STAT_TIMELIMIT_PISTOL_SKILL + k]` (0xb78fc8 + 4k; no
     ×1000 here): `Msg(1, st, (f32)(c * *inc)); m_WeaponCounter = 0;` else `m_WeaponCounter++`.
- Note: `m_LastWeaponTypeFired` actually stores the skill index `k`, not a weapon type.
- Open questions: none.

## 0x55cfc0 CStats::UpdateStatsOnRespawn  (catalogue: specified/REA, port level 10)
- Signature: `void ()`. Caller: `CGameLogic::RestorePlayerStuffDuringResurrection` 0x442060.
- Behaviour: if `u32f(m_DeathCounter) > R[TIMELIMIT_DEATH_HEALTH]` (no ×1000): `if (F[24 MAX_HEALTH] >
  400.0f) { Inc(24, R[DEC_MAX_HEALTH]); Msg(0, 24, R[DEC_MAX_HEALTH]); } m_DeathCounter = 0;` else
  `m_DeathCounter++`.
- Open questions: none.

## 0x55d030 CStats::UpdateStatsAddToHealth  (catalogue: specified/REA, port level 9)
- Signature: `void (u32 add [esp+4])` (arg slot reused). Callers: `CPickups::GivePlayerGoodiesWithPickUpMI`
  0x4564f0, `CStats::IncrementStat` 0x55c180.
- Behaviour: `c = m_AddToHealthCounter + add` (wrapping); `m_AddToHealthCounter = c`; if `u32f(c) >
  R[TIMELIMIT_ADD_TO_HEALTH]` (no ×1000): `Inc(24, R[INC_MAX_HEALTH]); Msg(1, 24, R[INC_MAX_HEALTH]);
  m_AddToHealthCounter = 0`.
- Note: re-entrancy — `IncrementStat` calls this function; the counter is reset only after both calls
  return (order as above).
- Open questions: none.
