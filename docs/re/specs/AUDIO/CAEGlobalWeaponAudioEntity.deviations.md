# CAEGlobalWeaponAudioEntity (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Entities/AEGlobalWeaponAudioEntity.cpp` (pin 20d8d2a). Statics: s_GunShots 0xB61318,
s_WeaponType 0xB6131C, s_Delay 0xB61320, s_State 0xB61324, s_LastTime 0xB61364 (guard bit 1 of 0xB61368), pWaterfall[3]
0xB612F0, pFogHorn 0xB612FC, fog-horn positions 0xB61358 / 0xB6134C, waterfall positions 0xB61340 / 0xB61334 / 0xB61328
(lazily initialised under guard bits 2..0x20 of 0xB61368, 0x4DF25E-0x4DF331). Tuning constants live in .data (0x8AE5B8..0x8AE5EC).

## 0x4DF210 CAEGlobalWeaponAudioEntity::ServiceAmbientGunFire  (gr_deviations = 7)
Original: `__thiscall void (this = ECX)`, plain `ret`; local CAESound (EH); the epilogue passes through the .text stub
0x402002 (`mov [esp+0xC0],-1; jmp 0x4DFA5E`). Switch on s_State (`cmp eax,5; ja`, table 0x4DFA7C).
Rows AEGlobalWeaponAudioEntity.cpp:134 / :135 (`CGeneral::GetRandomNumberInRange(0, TWO_PI)` / `(40, 80)`):
- Original bytes (LA, 0x4DF360): if `m_FrameCounter & 0x3FF` return. FIRST `m_Physical(+0xA0)->SetPosn(camera pos)` 0x4241C0
  (0x4DF38A), THEN `a = CGeneral::GetRandomNumberInRange<float>(0, 6.2831855f)` 0x41BD90 (stored f32), `r = ...(40.0f, 80.0f)`
  (kept in ST0, not rounded), `pos.x += cos(a) * r` (`fcos; fmul st1; fadd [x]`), `pos.y += sin(a) * r`, `pos.z = 0x404330(pos.z,
  20.0f)` (min) on the entity's matrix/placement position in place; `s_State = 1`; PickAmbientGunFire 0x4DEFE0.
- Verdict: CONFIRMED (gr's CGeneral float formula differs from 0x41BD90, docs/re/cat14 finding 1). Class: keep-original.
Row :150 (`all_of(pWaterfall, IsNull)`): original compares the three pointers explicitly (0x4DF5D0-0x4DF5EE). REFUTED, harmless.
(LV check order in the original: `distSq(camera - wf[1]) < 10000.0f [0x8AE5DC]` strict, then `IsSoundBankLoaded(0x27, 2)`, then
the null checks; gr tests bank, nulls, distance - side-effect free, harmless.)
Row :222 (`assert(type == PISTOL || AK47)`): original: type 0x16 -> `s_Delay = GetRandomNumberInRange<int32>(500, 1200)`,
0x1E -> `(90, 450)`, any other type: s_Delay unchanged (0x4DF862-0x4DF887). gr assigns the AK range to any non-pistol type.
Verdict: CONFIRMED (only for unexpected types). Class: keep-original.
Row :247 (BUG: FOGHORN_TTL == FOGHORN_FADE_OUT_DELAY): original FOGHORN_0 (0x4DF89B): `now > last + [0x8AE5C8] (5000)` ->
StopSound, `s_State = 3`; else `now > last + [0x8AE5BC] (5000)` -> `pFogHorn->m_Volume (+0x14) -= 3.3f [0x8AE5B8]` (unsigned
compares `jbe`). Unreachable fade as gr says. Verdict: REFUTED (gr reproduces it). Class: keep-original (bug kept).
Row :258 (FRONTEND bank check in STATE_3): original 0x4DF8C8: `if (now <= last + [0x8AE5C4] (6500)) return` (unsigned `jbe`);
then `IsSoundBankLoaded(0x3B, 0)`; then if `pFogHorn == 0` play. The bank check is in the original. Verdict: REFUTED (row).
BUT gr's time test is INVERTED: gr returns when `last + 6500 < now`; the original returns while `now <= last + 6500`.
CONFIRMED gr logic error, keep-original.
Row :289 (`default: NOTSA_UNREACHABLE()`): original `s_State > 5` jumps to the epilogue (0x4DF343). CONFIRMED, keep-original.
Found in passing (CONFIRMED, keep-original):
- Fog horn sounds (both 0x4DF4EB and 0x4DF8F8) are initialised with volume `[0x8AE5E8] = 12.0f` dB; gr passes no Volume (0 dB).
  Then rolloff (+0x18) = 60.0f [0x8AE5E0], speed (+0x1C) = 1.0f [0x8AE5E4] (first) / 1.2f [0x8AE5C0] (second), flags (+0x56) = 4.
- First fog horn (default region, 0x4DF462): checks `Foggyness_SF < 0.5` return, frame `& 0x3FF`, `distSq(camera - fog[0]) <
  4000000.0f [0x8AE5EC]` (strict), then `IsSoundBankLoaded(0x3B, 0)` (0x4DF4D2) - gr omits this bank check - then `pFogHorn == 0`.
- Region test: `WeatherRegion == 1` LA, `== 3` LV, `== 4` nothing, else fog horn (0x4DF356-0x4DF45C).
- Waterfall state 5 (0x4DF9D1): stops the three sounds but does NOT null the pointers (gr nulls them); they are cleared later by
  the entity's UpdateParameters callback. Distance compare `<= 10000.0f` keeps playing only when region is LV (else stop).
