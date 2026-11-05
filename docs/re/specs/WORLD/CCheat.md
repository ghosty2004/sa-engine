# CCheat (WORLD) - full specs

Source of truth: the 1.0 US bytes (bundles in RE-SPEC-1). gta-reversed (gr) has `source/game_sa/Cheat.cpp`
code for every function below, but none of these addresses is hooked by gr; every gr statement below was
checked against the disassembly and differences are listed per function.

## Common facts (apply to every function in this file)

- Convention: all cheat handlers are `__cdecl void Fn(void)`, plain `ret` (no stack args), no register args.
  Callers: none direct (catalogue callers = 0) unless stated; they are reached through the function-pointer
  table `CCheat::m_aCheatFunctions` at 0x8A5B58 (92 entries, index = `eCheats` id) called by
  `CCheat::ApplyCheat` (0x438370). Table index for each function is given as `[id N]`.
- `CCheat::m_aCheatsActive` = `bool[92]` at 0x969130; entry for cheat id N is the byte at 0x969130+N.
  "toggle X" below always means the exact sequence `new = (X == 0) ? 1 : 0; X = new;` (`test/sete`), then
  the branch tests `new` (not a re-read).
- `CWorld::FindPlayerPed(int32 id)` 0x56E210 and `CWorld::FindPlayerVehicle(int32 id, bool includeRemote)`
  0x56E0D0 are cdecl, caller cleans. `FindPlayerPed(-1)` is called again at every use site (the original
  never caches the pointer); a port must keep the call count/order only where the callee has side effects
  (it has none), so caching is behaviourally equivalent, but the result is never null-checked unless stated.
- CPed fields used: +0x47C `m_pIntelligence` (CPedIntelligence*), +0x480 `m_pPlayerData` (CPlayerPedData*),
  +0x4E0 `m_acquaintance` (CAcquaintance), +0x530 `m_nPedState` (int32; 0x32 = PEDSTATE_DRIVING),
  +0x540 `m_fHealth` f32, +0x548 `m_fArmour` f32, +0x5A0 `m_aWeapons[13]` (CWeapon, 0x1C each, `m_Type` at +0).
  CPlayerPedData: +0 `m_pWanted` (CWanted*), +4 `m_pPedClothesDesc`, +0x34 `m_nPlayerFlags` (bit 1 =
  `m_bAdrenaline`). CWanted +0x2C `m_nWantedLevel` int32. CPedIntelligence +0x68 `m_eventGroup`,
  +0x124 `m_pedScanner`.
- "RebuildIfNotDriving" below = the exact sequence
  `if (FindPlayerPed(-1)->m_nPedState != 0x32) CClothes::RebuildPlayer(FindPlayerPed(-1), false);`
  (`CClothes::RebuildPlayer` 0x5A82C0, cdecl (CPlayerPed*, bool)).
- "SetClothes(tex, model, part)" = `FindPlayerPed(-1)->m_pPlayerData->m_pPedClothesDesc->SetTextureAndModel(tex, model, part)`
  (`CPedClothesDesc::SetTextureAndModel(const char*, const char*, eClothesTexturePart)` 0x5A8080, thiscall).
  Strings are .rdata constants (address given); `nullptr` where the original pushes 0.
- "StreamPeds[a,b,...]" = build an `int32[8]` on the stack with the listed ids (-2 = UNLOAD_MODEL padding) and
  call `CStreaming::StreamPedsIntoRandomSlots(int32*)` 0x40BDA0 (cdecl).
- `CStats::SetStatValue(eStats, float)` 0x55A070 cdecl; 1000.0 is pushed as immediate 0x447A0000.
- Pool iteration (`BlowUpCarsCheat`, `MayhemCheat`, `EverybodyAttacksPlayerCheat`): the original reads the
  pool pointer once, reads `size = pool->m_nSize` (+8) once, then walks slots from `size-1` DOWN to 0. For
  each slot it re-reads `pool->m_byteMap` (+4) and tests the flag byte: slot skipped if bit 7 set (`js`,
  free). Object address = `pool->m_pObjects (+0, re-read) + slot*stride`; skipped if that sum is 0. gr uses
  `GetAllValid()` (ascending); the descending order is what the port must reproduce (it determines event
  insertion / explosion order).

## 0x4385B0 CCheat::WeaponCheat1  (catalogue: specified/REA, port level 16)
- `[id 0]` CHEAT_WEAPON_SET1. Callees: CStreaming::RequestModel 0x4087E0 (cdecl (int32 model, int32 flags)),
  LoadAllRequestedModels 0x40EA10 (cdecl (bool)), FindPlayerPed, CPed::GiveWeapon 0x5E6080
  (thiscall (eWeaponType, uint32 ammo, bool) ret value ignored), CStreaming::SetModelIsDeletable 0x409C10 (cdecl (int32)).
- Behaviour:
  1. RequestModel(m, 2) for m in order: 0x14B BRASSKNUCKLE, 0x150 BAT, 0x158 MOLOTOV, 0x15A COLT45,
     0x15D CHROMEGUN, 0x160 MICRO_UZI, 0x163 AK47, 0x165 CUNTGUN, 0x167 ROCKETLA, 0x16D SPRAYCAN.
  2. LoadAllRequestedModels(false).
  3. For (weapon, ammo) in order (1,1) (5,1) (18,10) (22,100) (25,50) (28,150) (30,120) (33,25) (35,200)
     (41,200): `FindPlayerPed(-1)->GiveWeapon(weapon, ammo, true)` (fresh FindPlayerPed per call).
  4. `if (FindPlayerPed(1) != nullptr)`: same 10 GiveWeapon calls on `FindPlayerPed(1)` (fresh call each).
  5. SetModelIsDeletable(m) for, in order: 0x14B, 0x160, 0x150, 0x158, 0x15D, 0x163, 0x165, 0x167, 0x16D.
- Oddities: COLT45 (0x15A) is requested but never marked deletable (original bug, keep). Release order is
  not the request order.
- gr: uses `CPed::GiveWeaponSet1()` (notsa helper, same 10 calls/ammo); differences: gr gives player 1's set
  to player 0 (`player->GiveWeaponSet1()` inside the player1 branch; original gives to FindPlayerPed(1));
  gr marks all 10 models deletable including COLT45 and in request order.

## 0x438890 CCheat::WeaponCheat2  (catalogue: specified/REA, port level 16)
- `[id 1]`. Same callees and shape as WeaponCheat1.
- Request (flag 2), in order: 0x14F KNIFECUR, 0x156 GRENADE, 0x15C DESERT_EAGLE, 0x15E SAWNOFF, 0x174 TEC9,
  0x164 M4, 0x166 SNIPER, 0x169 FLAME, 0x16E FIRE_EX; LoadAllRequestedModels(false).
- Give (weapon, ammo, true) to FindPlayerPed(-1): (4,0) (16,10) (24,40) (26,40) (32,150) (31,150) (34,21)
  (37,500) (42,200); then if FindPlayerPed(1) non-null the same 9 to FindPlayerPed(1).
- SetModelIsDeletable in the request order (all 9).
- gr: matches (GiveWeaponSet2, player1 branch correct here).

## 0x438B30 CCheat::WeaponCheat3  (catalogue: specified/REA, port level 16)
- `[id 2]`. Same shape.
- Request (flag 2): 0x155 CHNSAW, 0x15B SILENCED, 0x15F SHOTGSPA, 0x161 MP5LNG, 0x164 M4, 0x168 HEATSEEK,
  0x16B SATCHEL, 0x16C BOMB; LoadAllRequestedModels(false).
- Give: (9,0) (39,5) (23,40) (27,30) (29,100) (31,150) (36,200) to FindPlayerPed(-1), then same to
  FindPlayerPed(1) if non-null.
- SetModelIsDeletable in request order (all 8).
- gr: matches (GiveWeaponSet3).

## 0x438E40 CCheat::MoneyArmourHealthCheat  (catalogue: specified/REA, port level 1)
- `[id 3]`. Catalogue extent includes 0x438D60-0x438E3F, the body of gr's `CCheat::HealthCheat` (0x438D60),
  which is NOT a separate catalogue row: 0x438E40 tail-jumps into it (`jmp 0x438d60`). The port should
  implement HealthCheat as a private helper of this function (no other caller is catalogued).
- Globals: `CWorld::PlayerInFocus` 0xB7CD74 (u8); `CWorld::Players` 0xB7CD98 (CPlayerInfo[2], 0x190 each):
  +0xB8 `m_nMoney` int32, +0x14F `m_nMaxHealth` u8, +0x150 `m_nMaxArmour` u8.
- Behaviour (0x438E40):
  1. `Players[PlayerInFocus].m_nMoney += 250000` (0x3D090, int add, wraps).
  2. `t = (float)(int32)Players[0].m_nMaxArmour` (byte 0xB7CEE8, zero-extended, `fild`/`fstp` f32) - note the
     armour cap is read from Players[0] regardless of PlayerInFocus.
  3. `FindPlayerPed(-1)->m_fArmour = t`.
  4. fall into HealthCheat.
- HealthCheat body (0x438D60):
  1. `h = (float)Players[0].m_nMaxHealth` (byte 0xB7CEE7, same conversion); `FindPlayerPed(-1)->m_fHealth = h`.
  2. `if (FindPlayerVehicle(-1, false) == nullptr) return;`
  3. `FindPlayerVehicle(-1,false)->m_fHealth (+0x4C0) = 1000.0f` (imm 0x447A0000).
  4. `t = FindPlayerVehicle(-1,false)->m_nVehicleType (+0x590)`; if `t != 0`: re-read via a new
     FindPlayerVehicle; if `!= 9` return.
  5. Re-read type via new FindPlayerVehicle: if `== 0` (AUTOMOBILE): `FindPlayerVehicle()->(+0x8E4 CAutomobile::m_fBurnTimer) = 0`
     (int store 0), then tail-call virtual slot +0xC8 (gr: `Fix()`) on a fresh FindPlayerVehicle(-1,false).
     Else (BIKE, 9): `(+0x7BC CBike::m_BlowUpTimer) = 0`, then the same virtual +0xC8 call.
- Oddities: type test is the exact value 0 or 9 (MTRUCK/QUAD/HELI/PLANE/BMX etc. only get health 1000, no
  fix). FindPlayerVehicle is called up to 7 times. No null check of FindPlayerPed.
- gr: `MoneyArmourHealthCheat` uses `GetPlayerInfoForThisPlayerPed()` for money AND armour; the original uses
  `Players[PlayerInFocus]` for money but hard-coded `Players[0]` for max armour/health. gr HealthCheat uses
  `IsBike()/IsAutomobile()` (same type tests) - otherwise matches.

## 0x438E90 CCheat::WantedLevelUpCheat  (catalogue: specified/REA, port level 14)
- `[id 4]`. Callees: FindPlayerPed, `CPlayerPed::CheatWantedLevel(int32)` 0x609F50 (thiscall).
- Behaviour: `lvl(p) = (p->m_pPlayerData ? p->m_pPlayerData->m_pWanted : nullptr)->m_nWantedLevel` (reads
  address 0x2C if m_pPlayerData is null: crash; no null check of the ped either).
  1. `if (lvl(FindPlayerPed(-1)) + 2 >= 6)` (signed `jge`) -> `FindPlayerPed(-1)->CheatWantedLevel(6)`.
  2. else `n = lvl(FindPlayerPed(-1)) + 2` (re-read), `FindPlayerPed(-1)->CheatWantedLevel(n)`.
- gr: adds `if (!player) return;` (original has none); otherwise `min(level+2, 6)` matches.

## 0x438F20 CCheat::WantedLevelDownCheat  (catalogue: specified/REA, port level 14)
- `[id 5]`. `FindPlayerPed(-1)->CheatWantedLevel(0)`. gr matches.

## 0x438F40..0x438F80, 0x439590 weather cheats  (catalogue: specified/REA, port level 1)
Each is `CWeather::ForceWeatherNow(id)` (0x72A4F0, cdecl, arg pushed as dword, `pop ecx` cleanup). gr matches.

| address | name | [id] | arg |
|---|---|---|---|
| 0x438F40 | SunnyWeatherCheat | 6 | 1 (WEATHER_SUNNY_LA) |
| 0x438F50 | ExtraSunnyWeatherCheat | 7 | 0 (WEATHER_EXTRASUNNY_LA) |
| 0x438F60 | CloudyWeatherCheat | 8 | 4 (WEATHER_CLOUDY_LA) |
| 0x438F70 | RainyWeatherCheat | 9 | 0x10 (WEATHER_RAINY_COUNTRYSIDE) |
| 0x438F80 | FoggyWeatherCheat | 10 | 9 (WEATHER_FOGGY_SF) |
| 0x439590 | SandstormCheat | 58 | 0x13 (WEATHER_SANDSTORM_DESERT) |

## 0x438F90 CCheat::FastTimeCheat  (catalogue: specified/REA, port level 0)
- `[id 12]`. Globals: `CTimer::ms_fTimeScale` 0xB7CB64 f32 (R/W); const 0x858B90 = 4.0f.
- Behaviour: `fld ts; fcomp 4.0f; test ah,5; jp skip` -> body runs only when `ts < 4.0` (ts == 4.0, ts > 4.0
  and NaN skip). Body: `ts = ts + ts` (`fld ts; fadd st0,st0; fstp` f32).
- gr: `ts < 4.0f -> ts*2.0f` - equivalent.

## 0x438FC0 CCheat::SlowTimeCheat  (catalogue: specified/REA, port level 0)
- `[id 13]`. const 0x858C84 = 0.25f, 0x858B8C = 0.5f.
- Behaviour: `fcomp 0.25f; test ah,0x41; jne skip` -> body only when `ts > 0.25` (equal / less / NaN skip).
  Body: `ts = ts * 0.5f` (`fmul dword [0x858B8C]`).
- gr: `ts / 2.0f` - same value (exact halving). Matches.

## 0x438FF0 CCheat::SuicideCheat  (catalogue: specified/REA, port level 9)
- `[id 29]`. SEH frame (unwind 0x837520). Locals: `calc` CPedDamageResponseCalculator at [esp+0],
  `ev` CEventDamage at [esp+0x14] (size 0x44 at least; ends before [esp+0x58]).
- Callees: CPedDamageResponseCalculator ctor 0x4AD3F0 / dtor 0x4AD420; CEventDamage ctor 0x4AD830
  (the 7-arg overload) / dtor 0x4AD960; CEventDamage::AffectsPed 0x4B35A0; ComputeDamageResponse 0x4B5AC0;
  CEventGroup::Add 0x4AB420. Globals: `CTimer::m_snTimeInMilliseconds` 0xB7CB84 (R).
- Behaviour:
  1. `calc.ctor(nullptr /*entity*/, 1000.0f, 0 /*WEAPON_UNARMED*/, 3 /*PED_PIECE_TORSO*/, false)`.
  2. `ev.ctor(nullptr, CTimer::m_snTimeInMilliseconds, 0, 3, 0, false, false)`.
  3. `if (ev.AffectsPed(FindPlayerPed(-1)))` -> `calc.ComputeDamageResponse(FindPlayerPed(-1), &ev.m_damageResponse (ev+0x38), true)`;
     else `ev.m_damageResponse.m_bDamageCalculated (ev+0x42) = 1`.
  4. `FindPlayerPed(-1)->m_pIntelligence->m_eventGroup.Add(&ev, false)` (return ignored).
  5. `ev.~CEventDamage()` then `calc.~CPedDamageResponseCalculator()`.
- gr: matches.

## 0x4390D0 CCheat::PinkCarsCheat  (catalogue: specified/REA, port level 0)
- `[id 32]`. Toggle 0x969150 (PINK_TRAFFIC); if new != 0: 0x969151 (BLACK_TRAFFIC) = 0. gr matches.

## 0x4390F0 CCheat::BlackCarsCheat  (catalogue: specified/REA, port level 0)
- `[id 33]`. Toggle 0x969151; if new != 0: 0x969150 = 0. gr matches.

## 0x439110 CCheat::FatCheat  (catalogue: specified/REA, port level 21)
- `[id 36]`. `CStats::SetStatValue(0x15 STAT_FAT, 1000.0f)`; RebuildIfNotDriving. gr matches.

## 0x439150 CCheat::MuscleCheat  (catalogue: specified/REA, port level 21)
- `[id 37]`. `SetStatValue(0x17 STAT_MUSCLE, 1000.0f)`; RebuildIfNotDriving. gr matches (gr calls
  FindPlayerPed before SetStatValue; no side effect, harmless).

## 0x439190 CCheat::SkinnyCheat  (catalogue: specified/REA, port level 21)
- `[id 38]`. `SetStatValue(0x15, 0.0f)`, `SetStatValue(0x17, 0.0f)` (pushes int 0 = +0.0f); RebuildIfNotDriving.
  gr matches.

## 0x4391D0 CCheat::ElvisLivesCheat  (catalogue: specified/REA, port level 10)
- `[id 39]`. Toggle 0x969157; if new != 0: StreamPeds[0x52 VHMYELV, 0x53 VBMYELV, 0x54 VIMYELV, -2 x5].
  gr matches.

## 0x439230 CCheat::BeachPartyCheat  (catalogue: specified/REA, port level 21)
- `[id 41]`. Callers: HandleSpecialCheats 0x439A10, NinjaCheat 0x439E50, CountrysideInvasionCheat 0x439F60.
- Code layout oddity: 0x439230 = `nop; jmp 0x404790`; 0x404790 loads `dl = [0x969159]` and jumps back to
  0x439236. Behaviourally just the start of the function.
- Behaviour: toggle 0x969159 (BEACH_PARTY). If new == 0 return. Else:
  1. StreamPeds[0x12 BMYBE, 0x2D HMYBE, 0x8A WFYBE, 0x8B BFYBE, 0x8C HFYBE, -2, -2, -2].
  2. SetClothes("torso" 0x8590CC, nullptr, 0 TORSO); SetClothes("shortskhaki" 0x8590B8, "shorts" 0x8590C4, 2 LEGS);
     SetClothes("flipflop" 0x8590AC, "flipflop" 0x8590AC, 3 SHOES);
     SetClothes("glasses04dark" 0x859090, "glasses04" 0x8590A0, 15 GLASSES).
  3. RebuildIfNotDriving.
  4. `CWeather::ForceWeatherNow(0)` (inlined ExtraSunnyWeatherCheat).
  5. `CCheat::HandleSpecialCheats(0x29)`; tail-jump `CStreaming::ReclassifyLoadedCars()` 0x40AFA0.
- gr: FIX_BUGS adds a first `SetTextureAndModel(nullptr, nullptr, CLOTHES_TEXTURE_SPECIAL)` that the original
  does not have (keep original: 4 calls). Rest matches.

## 0x439360 CCheat::GangsCheat  (catalogue: specified/REA, port level 10)
- `[id 42]`. Toggle 0x96915A; if new != 0: StreamPeds[0x66 BALLAS1, 0x67 BALLAS2, 0x68 BALLAS3, 0x69 FAM1,
  0x6A FAM2, 0x6C LSV1, 0x6D LSV2, 0x6E LSV3]. gr matches.

## 0x4393D0 CCheat::GangLandCheat  (catalogue: specified/REA, port level 0)
- `[id 43]`. Toggle 0x96915B; `CPopulation::m_bOnlyCreateRandomGangMembers` (0xC0FCB3, u8) = (new != 0).
  gr matches.

## 0x4393F0 CCheat::LoveConquersAllCheat  (catalogue: specified/REA, port level 21)
- `[id 45]`. Toggle 0x96915D; if new == 0 return. StreamPeds[0xF9 BMYPIMP, 0x3F BFYPRO, 0x40 HFYPRO,
  0x4B SWFOPRO, 0xEE SBFYPRO, 0x55 VWFYPRO, 0xCF VHFYPRO, -2]; SetClothes("gimpleg" 0x8590D4, "gimpleg", 0x11 SPECIAL);
  RebuildIfNotDriving. gr matches.

## 0x4394B0 CCheat::AllCarsAreShitCheat  (catalogue: specified/REA, port level 22)
- `[id 46]`. Toggle 0x96915E; if new != 0: `HandleSpecialCheats(0x2E)`, tail-jump ReclassifyLoadedCars. gr matches.

## 0x4394E0 CCheat::AllCarsAreGreatCheat  (catalogue: specified/REA, port level 22)
- `[id 47]`. Toggle 0x96915F; if new != 0: `HandleSpecialCheats(0x2F)`, tail-jump ReclassifyLoadedCars. gr matches.

## 0x439510 CCheat::MidnightCheat  (catalogue: specified/REA, port level 0)
- `[id 55]`. Toggle 0x969167; if new != 0: `CClock::ms_nGameClockHours` (0xB70153 u8) = 0,
  `ms_nGameClockMinutes` (0xB70152 u8) = 0, `ms_nGameClockSeconds` (0xB70150 u16) = 0 (that store order).
  gr matches.

## 0x439540 CCheat::DuskCheat  (catalogue: specified/REA, port level 0)
- `[id 56]`. Toggle 0x969168; if new != 0: hours = 21 (0x15), minutes = 0, seconds (u16) = 0.
- gr DIFFERS: gr sets the clock when the cheat becomes INACTIVE (`if (!IsActive)`); the original sets it when
  it becomes active. Port follows the bytes.

## 0x439570 CCheat::StormCheat  (catalogue: specified/REA, port level 1)
- `[id 57]`. `CWeather::ForceWeatherNow(0x10)`; `CWeather::Rain` (0xC81324 f32) = 1.0f; `CWeather::Wind`
  (0xC812F0 f32) = 1.0f (immediate 0x3F800000 stores). gr matches.

## 0x4395A0 CCheat::PredatorCheat  (catalogue: specified/REA, port level 0)
- `[id 59]` (CHEAT_UNUSED). Body is a single `ret`: does nothing.
- gr DIFFERS: gr calls `VehicleCheat(MODEL_PREDATOR)`; the original is empty.

## 0x4395B0 CCheat::ParachuteCheat  (catalogue: specified/REA, port level 16)
- `[id 63]`. `RequestModel(0x173 GUN_PARA, 2)`; `LoadAllRequestedModels(false)`;
  `FindPlayerPed(-1)->GiveWeapon(0x2E PARACHUTE, 0, true)`; `FindPlayerPed(-1)->SetCurrentWeapon(0x2E)`
  (`CPed::SetCurrentWeapon(eWeaponType)` 0x5E6280 thiscall). gr matches.

## 0x439600 CCheat::JetpackCheat  (catalogue: specified/REA, port level 9)
- `[id 64]`. SEH frame (0x837543). Callees: CPedIntelligence::GetTaskJetPack 0x601110 (thiscall),
  `CTask::operator new` 0x61A5A0 (cdecl(size)), CTaskSimpleJetPack ctor 0x67B4E0, CEventScriptCommand ctor
  0x4B0A00, dtor 0x4B0A50, CEventGroup::Add 0x4AB420.
- Behaviour:
  1. `if (FindPlayerPed(-1)->m_pIntelligence->GetTaskJetPack() != nullptr) return;`
  2. `mem = CTask::operator new(0x70)`; `task = mem ? CTaskSimpleJetPack::ctor(mem, nullptr /*target*/, 10.0f, 0, nullptr) : nullptr`.
  3. Local `CEventScriptCommand ev(3 /*TASK_PRIMARY_PRIMARY*/, task, false)` at [esp+4].
  4. `FindPlayerPed(-1)->m_pIntelligence->m_eventGroup.Add(&ev, false)`; then `ev` dtor (0x4B0A50).
- gr matches.

## 0x4396C0 CCheat::NotWantedCheat  (catalogue: specified/REA, port level 14)
- `[id 65]`. `FindPlayerPed(-1)->CheatWantedLevel(0)`; then toggle 0x969171 (I_DO_AS_I_PLEASE).
- gr DIFFERS: gr also does `player->bWantedByPolice = false;` - the original writes no ped flag. Port: no
  such write.

## 0x4396F0 CCheat::WantedCheat  (catalogue: specified/REA, port level 14)
- `[id 66]`. `FindPlayerPed(-1)->CheatWantedLevel(6)`. gr matches.

## 0x439710 CCheat::RiotCheat  (catalogue: specified/REA, port level 0)
- `[id 69]`. Toggle 0x969175 only. gr matches.

## 0x439720 CCheat::FunhouseCheat  (catalogue: specified/REA, port level 21)
- `[id 70]`. Callers: HandleSpecialCheats, NinjaCheat, CountrysideInvasionCheat. Global written:
  `CPostEffects::m_bHeatHazeFX` 0xC402BA (u8).
- Behaviour: toggle 0x969176.
  - new == 0: `m_bHeatHazeFX = 0`; return.
  - new != 0:
    1. StreamPeds[0xA7 WMYBELL, 0xCD WFYBURG, 0x108 WMOICE, -2 x5].
    2. SetClothes("torso", "torso" (both 0x8590CC), 0); SetClothes("legsheart" 0x859118, "legs" 0x859124, 2);
       SetClothes("timberhike" 0x859104, "bask1" 0x859110, 3); SetClothes("groucho" 0x8590F0, "grouchos" 0x8590F8, 15);
       SetClothes("hairpink" 0x8590DC, "head" 0x8590E8, 1).
    3. RebuildIfNotDriving.
    4. `HandleSpecialCheats(0x46)`; `CStreaming::ReclassifyLoadedCars()`; `CWeather::ForceWeatherNow(0)`.
    5. `m_bHeatHazeFX = 1`.
- gr DIFFERS: (a) branch inverted - gr does the theme setup when the cheat becomes INACTIVE and clears heat
  haze when it becomes active; (b) FIX_BUGS extra `SetTextureAndModel(nullptr, nullptr, SPECIAL)` first.
  Port follows the bytes (5 clothes calls, setup on activation).

## 0x439880 CCheat::AdrenalineCheat  (catalogue: specified/REA, port level 14)
- `[id 71]`. Global: `MI_PICKUP_ADRENALINE` 0x8CD5BC (u16, initial 0xFFFF).
- Behaviour: toggle 0x969177.
  - new != 0: `CPickups::GivePlayerGoodiesWithPickUpMI((uint32)(uint16)MI_PICKUP_ADRENALINE, 0)` (0x4564F0,
    cdecl; the model index is zero-extended, so an unset 0xFFFF is passed as 65535, not -1).
  - new == 0: `FindPlayerPed(-1)->m_pPlayerData->m_nPlayerFlags &= ~2` (clear m_bAdrenaline only; 32-bit RMW).
- gr DIFFERS: FIX_BUGS calls `FindPlayerPed()->ClearAdrenaline()` instead of just clearing the bit.

## 0x4398D0 CCheat::DrivebyCheat  (catalogue: specified/REA, port level 16)
- `[id 73]`. Toggle 0x969179; if new == 0 return. If `FindPlayerPed(-1)->m_aWeapons[4].m_Type` (+0x610)
  `!= 0` return. Else `FindPlayerPed(-1)->GiveDelayedWeapon(0x1C MICRO_UZI, 150)` (0x5E89B0 thiscall) and
  `FindPlayerPed(-1)->SetCurrentWeapon(0x1C)`.
- gr: same behaviour (gr indexes `m_aWeapons[WEAPON_KNIFE]`, i.e. slot 4 by value - slot 4 is the SMG slot,
  the name is misleading but the index matches).

## 0x439930 CCheat::StaminaCheat  (catalogue: specified/REA, port level 10)
- `[id 81]`. `SetStatValue(0x16 STAT_STAMINA, 1000.0f)`. gr matches.

## 0x439940 CCheat::WeaponSkillsCheat  (catalogue: specified/REA, port level 10)
- `[id 82]`. `SetStatValue(s, 1000.0f)` for s = 0x45, 0x46, ..., 0x4F (69..79, ascending; PISTOL_SKILL ..
  RIFLE_SKILL). gr matches.

## 0x4399D0 CCheat::VehicleSkillsCheat  (catalogue: specified/REA, port level 10)
- `[id 83]`. `SetStatValue(s, 1000.0f)` for s = 0xA0 (160 DRIVING), 0xDF (223 FLYING), 0xE5 (229 BIKE),
  0xE6 (230 CYCLING), in that order. gr matches.

## 0x439B20 CCheat::MayhemCheat  (catalogue: specified/REA, port level 9)
- `[id 14]` PEDS_ATTACK_OTHER_WITH_GOLFCLUB. SEH frame 0x837558. Globals: `CPools::ms_pPedPool` 0xB74490
  (stride 0x7C4). Callees: CPedType::SetPedTypeAsAcquaintance 0x608E20 (cdecl (int32 type, int32 pedType,
  int32 flags)), CPed::IsPlayer 0x5DF8F0 (thiscall), CPedType::GetPedFlag 0x608830 (cdecl (int32) -> uint32),
  CAcquaintance::SetAsAcquaintance 0x608DA0 (thiscall (int32 type, uint32 flag)), CPedScanner::GetClosestPedInRange
  0x5FFF20 (thiscall), CEventAcquaintancePed ctor 0x4AF820 (thiscall (CPed*)), dtor-body 0x4AF890,
  CEventGroup::Add 0x4AB420. Vtable const 0x858E68 = CEventAcquaintancePedHate vtable.
- Behaviour: toggle 0x96913E. If new == 0: return (nothing else).
  1. for pt = 4 .. 22 inclusive (ascending): `SetPedTypeAsAcquaintance(4 /*HATE*/, pt, 0xFFFFF)`.
  2. Walk the ped pool (descending, see Common facts). For each live ped `p` with `!p->IsPlayer()`:
     a. for pt = 4 .. 22: `p->m_acquaintance.SetAsAcquaintance(4, GetPedFlag(pt))`.
     b. `c = p->m_pIntelligence->m_pedScanner.GetClosestPedInRange()`; if null continue.
     c. Local event `ev` (0x18 bytes): `CEventAcquaintancePed::ctor(&ev, c)`; `ev.vtable = 0x858E68`;
        `ev.m_TaskId (int16 at ev+0xE) = 1000` (TASK_COMPLEX_KILL_PED_ON_FOOT);
        `p->m_pIntelligence->m_eventGroup.Add(&ev, false)`; `ev.vtable = 0x858E68`; call 0x4AF890 on &ev.
        (`m_pIntelligence` is re-read from the ped for Add.)
- gr DIFFERS: (a) NOTSA `std::swap(CPedType::ms_apPedTypesOld, ms_apPedTypes)` on activation (not in
  original); (b) the whole else-branch (swap back + `Remove` events, FIX_BUGS) does not exist - the original
  does nothing on deactivation; (c) ascending pool order. Keep original.

## 0x439C70 CCheat::EverybodyAttacksPlayerCheat  (catalogue: specified/REA, port level 9)
- `[id 15]`. Callers: VillagePeopleCheat 0x439DD0 (call and tail-call). SEH frame 0x837578.
- Behaviour: toggle 0x96913F. If new == 0 return. Walk ped pool descending; for each live ped `p` with
  `!p->IsPlayer()`:
  1. `p->m_acquaintance.SetAsAcquaintance(4, GetPedFlag(0 /*PLAYER1*/))` (GetPedFlag called per ped).
  2. `CEventAcquaintancePed::ctor(&ev, FindPlayerPed(-1))` (FindPlayerPed called per ped); `ev.vtable = 0x858E68`;
     `ev.m_TaskId = 1000`; `p->m_pIntelligence->m_eventGroup.Add(&ev, false)`; `ev.vtable = 0x858E68`; 0x4AF890(&ev).
- gr: same except ascending pool order.

## 0x439D80 CCheat::BlowUpCarsCheat  (catalogue: specified/REA, port level 0)
- `[id 26]`. Global `CPools::ms_pVehiclePool` 0xB74494 (stride 0xA18). Walk descending (Common facts);
  for each live vehicle `v`: virtual call `v->vtable[+0xA4](nullptr, false)` (gr: `BlowUpCar(CEntity*, bool)`).
- gr: same except ascending order (`GetAllValid`). Order matters for explosion/event creation; keep descending.

## 0x439DD0 CCheat::VillagePeopleCheat  (catalogue: specified/REA, port level 10)
- `[id 40]` PEDS_ATTACK_YOU_WITH_ROCKETS. Toggle 0x969158.
  - new != 0: StreamPeds[0x1B WMYCON, 0xA1 CWMYFR, 0x11F ARMY, 0x11C LAPDM1, 0xF8 BIKERB, -2, -2, -2];
    `EverybodyAttacksPlayerCheat()`; then `0x969140 (EVERYONE_ARMED) = 1`.
  - new == 0: `0x969140 = 0`; tail-jump `EverybodyAttacksPlayerCheat()`.
- gr DIFFERS: branches inverted (gr streams peds and arms everyone when the cheat becomes inactive). Note
  EverybodyAttacksPlayerCheat toggles its own flag each call.

## 0x439E50 CCheat::NinjaCheat  (catalogue: specified/REA, port level 22)
- `[id 44]`. Toggle 0x96915C.
  - new == 0: `if (0x969151 != 0) 0x969151 = 0;` return.
  - new != 0:
    1. StreamPeds[0x75 TRIADA, 0x76 TRIADB, 0x78 TRIBOSS, -2 x5].
    2. Inlined HandleSpecialCheats-like block, in order: if [0x969159] BeachPartyCheat(); if [0x96915E] it = 0;
       if [0x96915F] it = 0; if [0x969176] FunhouseCheat(); if [0x96917B] CountrysideInvasionCheat().
       (Each flag read just before its test, so a call earlier in the list can affect later tests.)
    3. `CStreaming::ReclassifyLoadedCars()`.
    4. `if (0x969151 == 0) { 0x969151 = 1; 0x969150 = 0; }`.
    5. `RequestModel(0x153 KATANA, 2)`; `LoadAllRequestedModels(false)`;
       `FindPlayerPed(-1)->GiveWeapon(8 KATANA, 0, true)`; `FindPlayerPed(-1)->SetCurrentWeapon(8)`.
- gr DIFFERS: branches inverted (gr disables black traffic on activation and does the theme on deactivation).

## 0x439F60 CCheat::CountrysideInvasionCheat  (catalogue: specified/REA, port level 21)
- `[id 75]`. Callers: HandleSpecialCheats (tail-call), NinjaCheat. Toggle 0x96917B; if new == 0 return.
  1. StreamPeds[0xC4 CWFOFR, 0xC5 CWFOHB, 0xC6 CWFYFR1, 0x9D CWFYHB, 0x9E CWMOFR, 0x9F CWMOHB1, 0xA0 CWMOHB2, 0xA1 CWMYFR].
  2. SetClothes("timberfawn" 0x859144, "bask1" 0x859110, 3); SetClothes("captruck" 0x859138, "captruck", 0x10 HATS);
     SetClothes("countrytr" 0x85912C, "countrytr", 0x11 SPECIAL).
  3. RebuildIfNotDriving.
  4. if [0x969159] BeachPartyCheat(); if [0x96915E] = 0; if [0x96915F] = 0; if [0x969176] FunhouseCheat().
  5. tail-jump ReclassifyLoadedCars().
- gr matches.

## Vehicle spawn cheats  (catalogue: specified/REA, port level 18)
Each is exactly `push imm32; call CCheat::VehicleCheat (0x43A0B0); pop ecx; ret` - the returned CVehicle* is
discarded. `VehicleCheat` is `__cdecl CVehicle* (int32 modelId)` (not in this file). All gr model ids match.

| address | name | [id] | VehicleCheat arg |
|---|---|---|---|
| 0x43A4A0 | TankCheat | 17 | 0x1B0 = 432 RHINO |
| 0x43A4B0 | StockCarCheat | 18 | 0x1F8 = 504 BLOODRA |
| 0x43A4C0 | StockCar2Cheat | 19 | 0x1F9 = 505 RNCHLURE |
| 0x43A4D0 | StockCar3Cheat | 20 | 0x1F6 = 502 HOTRINA |
| 0x43A4E0 | StockCar4Cheat | 21 | 0x1F7 = 503 HOTRINB |
| 0x43A4F0 | HearseCheat | 22 | 0x1BA = 442 ROMERO |
| 0x43A500 | LovefistCheat | 23 | 0x199 = 409 STRETCH |
| 0x43A510 | TrashmasterCheat | 24 | 0x198 = 408 TRASH |
| 0x43A520 | GolfcartCheat | 25 | 0x1C9 = 457 CADDY |
| 0x43A530 | FlyboyCheat | 50 | 0x208 = 520 HYDRA |
| 0x43A540 | VortexCheat | 51 | 0x21B = 539 VORTEX |
| 0x43A550 | ApacheCheat | 84 | 0x1A9 = 425 HUNTER |
| 0x43A560 | QuadCheat | 85 | 0x1D7 = 471 QUAD |
| 0x43A660 | DozerCheat | 87 | 0x1E6 = 486 DOZER |
| 0x43A670 | StuntPlaneCheat | 88 | 0x201 = 513 STUNT |
| 0x43A680 | MonsterTruckCheat | 89 | 0x22C = 556 MONSTERA |

## 0x43A570 CCheat::TankerCheat  (catalogue: specified/REA, port level 18)
- `[id 86]`. SEH frame 0x83760B. Callees: VehicleCheat 0x43A0B0, RequestModel, LoadAllRequestedModels,
  `operator new` 0x6E2D50 (cdecl(size)), CTrailer ctor 0x6D03A0 (thiscall (int32 model, int32 createdBy)),
  CPlaceable::SetPosn(const CVector&) 0x4241C0, CPlaceable::SetOrientation(float,float,float) 0x439A80,
  CWorld::Add 0x563220 (cdecl (CEntity*)). Global read: `CStreaming::ms_aInfoForModel[584].m_nLoadState`
  (byte 0x8E7A70 = 0x8E4CC0 + 584*0x14 + 0x10).
- Behaviour:
  1. `v = VehicleCheat(0x202 /*514 PETRO*/)`; if null return.
  2. `RequestModel(0x248 /*584 PETROTR*/, 0)`; `LoadAllRequestedModels(false)`.
  3. `if (load state byte != 1 /*LOADSTATE_LOADED*/) return`.
  4. `mem = operator new(0x9F4)`; `t = mem ? CTrailer::ctor(mem, 0x248, 1 /*RANDOM_VEHICLE*/) : nullptr`; if
     `t == nullptr` return.
  5. `t->SetPosn(v->m_matrix ? &v->m_matrix->pos (matrix+0x30) : &v->m_placement.pos (v+4))` (`m_matrix` at +0x14).
  6. `t->SetOrientation(0.0f, 0.0f, 3.4906585f)` (imm 0x405F66F3 = 200 degrees in radians, f32).
  7. `t->(byte +0x36) = (byte & 7) | 0x20` (m_nStatus bits 3..7 := 4 = STATUS_ABANDONED).
  8. `CWorld::Add(t)`; virtual `t->vtable[+0xF4](v, true)` (gr: `SetTowLink(CVehicle*, bool)`).
- gr: matches (gr computes `DegreesToRadians(200)`; the port must use the f32 bit pattern 0x405F66F3).
