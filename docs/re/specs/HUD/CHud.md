# CHud (HUD) - full specs (group H1)

Common notation (all functions below):
- `W` / `H`: `fild` of RsGlobal.maximumWidth (i32 0xC17044) / maximumHeight (i32 0xC17048), i.e. exact float of the int.
  `Wi`, `Hi` are the ints. `W/2i` = `(Wi - (Wi >> 31)) >> 1` (`cdq; sub eax, edx; sar eax, 1`, signed halving).
- `kx = 0.0015625f` (0x859520, 1/640), `ky = 0.0022321430f` (0x859524, 1/448). `sx = W * kx`, `sy = H * ky` (x87, kept on
  the stack unless stated). A "stored" value is rounded to f32 in memory.
- CRect fields by offset: +0 left, +4 (gr "top"), +8 right, +0xC (gr "bottom"). Rect `{a, b, c, d}` lists +0, +4, +8, +0xC.
- `CRGBA(r,g,b,a)` temporaries are built with the CRGBA ctor 0x7170C0 (`__thiscall (r, g, b, a)` pushed a, b, g, r; returns
  this in EAX); calls by value pass the dword.
- `RS(state, value)` = `RwRenderStateSet` through `RwEngineInstance (0xC97B24)->+0x20` (cdecl; the caller pops in batches).
  States: 1 TEXTURERASTER, 2 TEXTUREADDRESS, 6 ZTESTENABLE, 8 ZWRITEENABLE, 9 TEXTUREFILTER, 10 SRCBLEND, 11 DESTBLEND.
- `Sprites` = CHud::Sprites 0xBAB1FC (CSprite2d[6], 4 bytes each = RwTexture*): Sprites[1] 0xBAB200, Sprites[2] 0xBAB204.
  `CSprite2d::Draw(const CRect&, const CRGBA&)` 0x728350 is thiscall (ECX = sprite, 2 args, ret 8).
- Camera: `mode` = `TheCamera.m_aCams[TheCamera.ActiveCam].Mode` = i16 at 0xB6F1A8 + 0x238 * byte[0xB6F081]. Mode ids (gr
  eCamMode): 7 SNIPER, 8 ROCKETLAUNCHER, 0x10 1STPERSON, 0x22 M16_1STPERSON, 0x27 SNIPER_RUNABOUT, 0x28 ROCKETLAUNCHER_RUNABOUT,
  0x29 1STPERSON_RUNABOUT, 0x2A M16_1STPERSON_RUNABOUT, 0x2D HELICANNON_1STPERSON, 0x2E CAMERA, 0x33 ROCKETLAUNCHER_HS,
  0x34 ROCKETLAUNCHER_RUNABOUT_HS, 0x35 AIMWEAPON, 0x37 AIMWEAPON_FROMCAR, 0x41 AIMWEAPON_ATTACHED.
- Ped: active weapon = `ped + 0x5A0 + 0x1C * (int8)ped[+0x718]`, its type = dword at +0 of that CWeapon.

## 0x58E020 CHud::DrawCrossHairs  (catalogue: specified/REA, port level 4)
- Convention: `__cdecl void ()`, plain `ret`; frame `sub esp, 0x30` + ebx/edi (+esi in the texture path).
- Callers: CHud::Draw 0x58FAE0. Callees: CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (-1, 0)), CWorld::FindPlayerPed 0x56E210
  (cdecl (-1)), CWeapon::IsTypeMelee 0x73B1C0 (thiscall), CPedIntelligence::GetTaskUseGun 0x600F70 (thiscall),
  CPlayerPed::GetWeaponRadiusOnScreen 0x609CD0 (thiscall, ST0), CSprite2d::DrawRect 0x727B60 (cdecl (rect*, color*)),
  CSprite2d::Draw 0x728350, CWeaponInfo::GetWeaponInfo 0x743C60 (cdecl (type, skill 1)), CKeyGen::AppendStringToKey
  0x53CF70 (cdecl (key, str) -> key), RwTexDictionaryFindHashNamedTexture 0x734E50 (cdecl (dict, hash)),
  CSprite::RenderOneXLUSprite 0x70D000 (cdecl (x, y, z, halfW, halfH, r, g, b, intensity i16, recipZ, a, uFlip, vFlip)).
- Globals: CWorld::PlayerInFocus (byte 0xB7CD74), CWorld::Players 0xB7CD98 (stride 0x190, +0 m_pPed), camera mode (above),
  byte 0xB6F080 (TheCamera+0x58, gr m_bTransitionState), CTheScripts::bDrawCrossHair (i32 0xA44490),
  CCamera::m_f3rdPersonCHairMultX 0xB6EC14 / MultY 0xB6EC10, Sprites[1], Sprites[2], CModelInfo::ms_modelInfoPtrs 0xA9B0C8,
  CTxdStore::ms_pTxdPool 0xC8800C.
- Behaviour:
  1. `P = Players[PlayerInFocus].m_pPed` (edi). Flags `A = 0` ([esp+0xB]), `B = 0` (bl).
  2. If mode is one of {7, 0x10, 8, 0x33, 0x22, 0x2D, 0x2E} (tested in that order, 0x58E04E-0x58E07B):
     a. If mode == 0x10: if `FindPlayerVehicle(-1,0)` is non-null and its model (+0x22, i16) is 0x208 (HYDRA) or 0x1A9 (HUNTER)
        (vehicle fetched again for each test) -> `A = 1`.
     b. Then (all modes of the list): if mode != 0x10 (re-read) and `P != 0` and `!IsTypeMelee(active weapon of P)` -> `A = 1`.
  3. If mode is one of {0x2A, 0x28, 0x34, 0x27} -> `B = 1` (0x58E10E-0x58E126).
  4. If `P->m_pTargetedObject (+0x71C) == 0` and `P->m_pPlayerData (+0x480)->flags (+0x34) & 8` (m_bFreeAiming) and
     (`GetTaskUseGun(P->m_pIntelligence (+0x47C)) == 0` or that task's byte +0xE == 0) and mode is one of {0x35, 0x37, 0x41}
     and `P->m_nPedState (+0x530)` is neither 0x3B nor 0x39 and the active weapon type is in 0x16..0x1F or is 0x20, 0x21, 0x26
     or 0x25: `B = (mode == 0x35 && byte[0xB6F080] != 0) ? 0 : 1` (0x58E1CC-0x58E1DF). (P is dereferenced here without a
     null check.)
  5. If `!A && !B && bDrawCrossHair == 0` return (no render-state changes).
  6. `RS(9, 2)`, `RS(8, 0)`.
  7. Circle crosshair - if `B` and mode is one of {0x35, 0x37, 0x41} (0x58E224-0x58E4A1):
     `cx = W * MultX` (stored, s14), `cy = H * MultY` (stored, s10), `r = P->GetWeaponRadiusOnScreen()` (stored s18).
     If `r == 0.2f` (0x858CC4; exact equality, NaN fails): `DrawRect({cx - 1, cy + 1, cx + 1, cy - 1}, CRGBA(255,255,255,255))`
     (1.0f 0x858624; each field `cx/cy -/+ 1` from the stored values).
     `SX = ((W * kx) * 64.0f) * r` (64.0f 0x859A44; stored), `SY = ((H * ky) * 64.0f) * r` (stored), `HX = SX * 0.5f` (stored),
     `L1 = (HX + cx) - SX` (x87 SX; stored), `HY = SY * 0.5f` (stored), `T1 = (HY + cy) - SY` (stored), `R1 = HX + L1` (stored),
     `B1 = HY + T1` (stored). Four `Sprites[1].Draw(rect, CRGBA(255,255,255,255))` in this order:
     `{L1, B1, R1, T1}`, `{L1 + SX, B1, R1, T1}` (L1 + SX stored), `{L1, B1, R1, T1 + SY}` (T1 + SY stored),
     `{L1 + SX, B1, R1, T1 + SY}` (the 2nd and 4th have left > right = horizontally mirrored, the 3rd and 4th have +0xC > +4).
     Then go to 10.
  8. Else, if `bDrawCrossHair != 2` and mode is one of {0x22, 0x2A, 0x29, 0x2D} (0x58E4A6-0x58E6AD): fixed centred crosshair:
     `SX = (W * kx) * 64.0f` (stored), `SY = (H * ky) * 64.0f` (stored), `L = (float)W/2i - SX * 0.5f` (stored),
     `T = (float)H/2i - SY * 0.5f` (stored), `R = SX * 0.5f + L` (stored), `B = SY * 0.5f + T` (stored). Four Sprites[1] draws,
     colour (255,255,255,255): `{L, B, R, T}`, `{L + SX, B, R, T}`, `{L, B, R, T + SY}`, `{L + SX, B, R, T + SY}` (sums stored).
     Then go to 10.
  9. Otherwise (0x58E6B2; `push esi`): `RS(9, 2)`. Let `wt` = type of FindPlayerPed(-1)'s active weapon (fetched afresh for each
     test).
     a. If `wt != 0x2B` (CAMERA) and `wt != 0x22` (SNIPERRIFLE) and `bDrawCrossHair != 2`: only if mode is one of
        {8, 0x10, 0x28, 0x33, 0x34}, else go to 10 (0x58E749). `tex = Sprites[2].m_pTexture` (dword 0xBAB204);
        `hx = 24.0f * sx` (24.0f 0x85F114), `hy = 24.0f * sy`, `ox = sx * 20.0f` (20.0f 0x858BA4), `oy = sy * 20.0f` (all stored).
     b. Else: if `wt` (re-fetched) != 0x2B and `bDrawCrossHair != 2`: `hx = (W * kx) * 210.0f`, `hy = (H * ky) * 210.0f`
        (0x863B34); else `hx = (W * kx) * 256.0f` (0x858FB4), `hy = (H * ky) * 192.0f` (0x866C74). `ox = oy = 0`.
        `info = GetWeaponInfo(type of P's active weapon (edi ped, not FindPlayerPed), 1)`; `m = info->m_nModelId1 (+0xC)`;
        if `m <= 0` go to 10. `mi = ms_modelInfoPtrs[m]`; `t = (i16)mi->m_nTxdIndex (+0xA)`; slot = (pool flag byte `pool->+4[t]`
        has bit 7 set) ? NULL : `pool->objects + 12 * t`; `dict = *slot` - with a free slot this reads address 0 (original
        null dereference, 0x58E86F-0x58E87B). If `dict == 0` go to 10. `tex = RwTexDictionaryFindHashNamedTexture(dict,
        AppendStringToKey(mi->m_nKey (+4), "CROSSHAIR" 0x866C68))`.
     c. If `tex == 0` go to 10. `RS(6, 0)`, `RS(2, 3)`, `RS(1, tex->raster (dword at tex+0))`.
        `HY = hy * 0.5f` (stored), `HX = hx * 0.5f` (stored), `cxi = (float)W/2i`, `cyi = (float)H/2i`. Four
        RenderOneXLUSprite(x, y, 1.0f, HX, HY, 255, 255, 255, 255, 0.01f (imm 0x3C23D70A), 255, uFlip, vFlip) in this order:
        `x = (cxi - HX) - ox, y = (cyi - HY) - oy, flips (0, 0)`; `x = (cxi + HX) + ox, y = (cyi - HY) - oy, (1, 0)`;
        `x = (cxi - HX) - ox, y = (cyi + HY) + oy, (0, 1)`; `x = (cxi + HX) + ox, y = (cyi + HY) + oy, (1, 1)` (x87, left to
        right). Then `RS(8, 0)`.
  10. Common end 0x58EABE: `RS(10, 5)`, `RS(11, 6)`, `RS(8, 1)`; return. (ZTESTENABLE set to 0 in 9c is not restored here.)
- gta-reversed: `return plugin::Call<0x58E020>()` (Hud.cpp); the C++ after it is dead and differs from the bytes (red test
  colour, `bIsRestoringLook` instead of m_bFreeAiming, inverted transition-state test, different rect arithmetic) - do not
  use it.
- Open questions: none (flag meanings of +0x71C / +0x480 bit 3 / +0xB6F080 are gr names; behaviour is byte-exact).

## 0x58C250 CHud::DrawSubtitles  (catalogue: specified/REA, port level 11)
- Convention: `__cdecl void ()`, plain `ret`; `sub esp, 0x10`.
- Callers: CHud::DrawAfterFade 0x58D490, CHud::Draw 0x58FAE0. Callees (all CFont setters are cdecl): CGameLogic::IsCoopGameGoingOn
  0x441390 (AL), CFont::SetBackground 0x7195C0 (b, b), SetBackgroundColor 0x7195E0 (CRGBA), SetOrientation 0x719610, SetProportional
  0x7195B0, SetDropShadowPosition 0x719570, SetFontStyle 0x719490, SetColor 0x719430, SetDropColor 0x719510, SetCentreSize 0x7194E0
  (float), SetScale 0x719380 (float x, float y), SetScaleForCurrentLanguage 0x7193A0 (float x, float y), PrintString 0x71A700.
- Globals: R/W `CHud::m_Message` (GxtChar[], 0xBAB040), R byte 0xBAADC0, RW `CHud::m_VehicleState` (i32 0xBAA448), RW
  `CHud::m_ZoneState` (i32 0xBAA930), R byte 0xB6F065 (TheCamera+0x3D), RW byte 0xBAB214 (latched "subtitle shown while byte
  0xB6F065 set" flag), R byte 0xBA678C (FrontEndMenuManager+0x44), R `CCutsceneMgr::ms_running` (byte 0xB5F851),
  R `CTheScripts::bUseMessageFormatting` (byte 0xA44B66), `MessageWidth` (u16 0xA44B60), `MessageCentre` (u16 0xA44B64),
  R `CHud::bDrawingVitalStats` (byte 0xBAB1DE).
- Behaviour:
  1. If `m_Message[0] == 0` return. If byte 0xBAADC0 != 0 and `!IsCoopGameGoingOn()` return.
  2. If `m_VehicleState != 0` set it to 3; if `m_ZoneState != 0` set it to 3.
  3. `SetBackground(0, 0)`; `SetBackgroundColor(CRGBA(0,0,0,0x80))`; `SetOrientation(0)`; `SetProportional(1)`;
     `SetDropShadowPosition(0)`; `SetFontStyle(1)`; `SetColor(CRGBA(0xE1,0xE1,0xE1,0xFF))`; `SetDropShadowPosition(2)`;
     `SetDropColor(CRGBA(0,0,0,0xFF))`.
  4. If byte 0xB6F065 != 0 (0x58C333): byte 0xBAB214 = 1; if byte 0xBA678C == 0 and `ms_running != 0` go to 7 (no print).
     `SetCentreSize(W - (W * kx) * 60.0f)` (0x858B34; `fld kx; fmul W; fmul 60; fsubr W`);
     `SetScale((W * kx) * 0.58f, (H * ky) * 1.2f)` (0x866C24 = 0.58f, 0x858F08 = 1.2f);
     `PrintString((float)W/2i, H - (H * ky) * 80.0f, m_Message)` (80.0f 0x858A14). Go to 7.
  5. Else: if byte 0xBAB214 != 0 then `m_Message[0] = 0` (the message is cleared, the print below then does nothing);
     byte 0xBAB214 = 0. `SetScaleForCurrentLanguage((W * kx) * 0.58f, (H * ky) * 1.22f)` (0x866C20 = 1.22f, always, 0x58C42C).
  6. Position (`hy = H * ky` on x87; `y = (H - 105.0f * hy) - (hy + hy)` with 105.0f 0x866C1C, `fadd st0, st0` for 2*hy):
     a. `bUseMessageFormatting`: `SetCentreSize((W * kx) * (float)MessageWidth)` (fimul); print at
        x = `(W * kx) * (float)MessageCentre` (fimul), y as above.
     b. else if `bDrawingVitalStats`: a second `SetScaleForCurrentLanguage(((W * kx) * 0.58f) * 0.8f, (H * ky) * 1.22f)` (0.8f 0x858C98, 0x58C4F1).
        With `s = W * kx` on x87: `SetCentreSize((((W - 20*s) - 8*s) - (140*s + 8*s)) * 0.8f)` (20.0f 0x858BA4, 8.0f 0x859000,
        140.0f 0x865374; operand order exactly `((W - s*20) - s*8) - (s*140 + s*8)`).
        Print x: with `Wf = W` stored, `S = W * kx` stored, `S8 = S * 8` stored, `Q = S * 140 + S8` stored:
        `x = ((((Wf - S * 20) - S8) - Q) * 0.5f + Q) + S * 40.0f` (0.5f 0x858B8C, 40.0f 0x858A10); y as above.
     c. else: `SetCentreSize(((W - 20*s) - 8*s) - (140*s + 8*s))` (no 0.8), print x = `(((Wf - S * 20) - S8) - Q) * 0.5f + Q`,
        y as above.
     `PrintString(x, y, m_Message)`.
  7. `SetDropShadowPosition(0)`; return.
- gta-reversed: `plugin::Call<0x58C250>()` only (Hud.cpp). Open questions: none.

## 0x58C6A0 CHud::DrawSuccessFailedMessage  (catalogue: specified/REA, port level 11)
- Convention: `__cdecl void ()`, plain `ret`; `push ecx` (one local slot), esi saved in the draw path.
- Callers: CHud::Draw 0x58FAE0. Callees: CFont::GetNumberLines 0x71A5E0 (cdecl (float x, float y, const GxtChar*) -> AX),
  CFont setters (SetBackground, SetScale, SetProportional, SetJustify 0x719600, SetOrientation, SetCentreSize, SetFontStyle,
  SetEdge 0x719590, SetDropColor, SetColor), CHudColours::GetRGBA 0x58FEE0 (thiscall on HudColour 0xBAB22C: (CRGBA* out, u8 index,
  u8 alpha) -> EAX = out), `_ftol` 0x821B40 (truncation), PrintString 0x71A700.
- Globals: `CHud::m_BigMessage` (GxtChar[7][128] at 0xBAACC0; style 2 at 0xBAADC0, 3 at 0xBAAE40, 5 at 0xBAAF40),
  `CHud::BigMessageX` (f32 0xBAA3DC; style-0 entry), `CHud::BigMessageInUse` (f32 0xBAA3C0, used as the slide position),
  `CHud::BigMessageAlpha` (f32 0xBAA3A4), function statics: guard dword 0xBAB21C (bit 0) and `yPos` f32 0xBAB218,
  `CTimer::ms_fTimeStep` 0xB7CB5C.
- Behaviour:
  1. Static init (0x58C6A1): if `!(guard & 1)`: `guard |= 1`; `yPos = (float)H/2i - (H * ky) * 10.0f` (10.0f 0x85862C).
  2. If `m_BigMessage[0][0] == 0`: `BigMessageX = 0`; return.
  3. If `BigMessageX == 0.0f` (exact; NaN counts as non-zero) - start of a new message, no drawing this frame:
     `BigMessageInUse = -60.0f`, `BigMessageX = 1.0f`, `BigMessageAlpha = 0`. With `hy = H * ky`, `Hh = (float)H/2i`:
     a. if `m_BigMessage[3][0] != 0` or `m_BigMessage[5][0] != 0`: `yPos = (Hh - hy * 10) + hy * 25.0f` (0x858FE8); return.
     b. else if `m_BigMessage[2][0] != 0`: `yPos = Hh - hy * 10`; return.
     c. else: `n = GetNumberLines((float)W/2i, Hh - hy * 10, m_BigMessage[0])`; if `n > 1` (signed 16-bit)
        `yPos = (Hh - hy * 10) - hy * 15.0f` (0x858B48) else `yPos = Hh - hy * 10`; return.
  4. Draw (0x58C83D): `SetBackground(0, 0)`; `SetScale((W * kx) * 1.3, (H * ky) * 1.8)` - the factors are f64 constants
     (0x866C30 = 1.3, 0x866C38 = 1.8, `fmul qword`), result rounded to f32 by the store; `SetProportional(1)`; `SetJustify(0)`;
     `SetOrientation(0)`; `SetCentreSize((W * kx) * 590.0f)` (0x866C28); `SetFontStyle(3)`; `SetEdge(2)`;
     `SetDropColor(CRGBA(0, 0, 0, (u8)_ftol(BigMessageAlpha)))`; `SetColor(HudColour.GetRGBA(6, (u8)_ftol(BigMessageAlpha)))`.
  5. `t = (float)(u32)_ftol((ms_fTimeStep * 0.02f) * 1000.0f) * 0.3f` (0.02f 0x858B38, 1000.0f 0x858C4C; the int is reinterpreted
     unsigned by adding 4294967296.0f 0x858C54 when negative; 0.3f 0x858C24). (Computed separately in each branch below.)
     a. If `(float)(Wi - 20) > BigMessageInUse` (fcomp; equal or NaN go to b): `BigMessageInUse += t` (stored);
        `BigMessageAlpha = t + BigMessageAlpha` (stored); if `BigMessageAlpha > 255.0f` (0x859AAC) it is set to 255.0f. Go to 6.
     b. Else: `BigMessageX = ms_fTimeStep + BigMessageX` (stored); if `!(BigMessageX < 120.0f)` (0x858BB0; NaN counts as less):
        `BigMessageX = 120.0f`, `BigMessageAlpha = BigMessageAlpha - t` (stored). Then (also when X < 120): if
        `BigMessageAlpha <= 0.0f` (NaN excluded): `BigMessageAlpha = 0`, `m_BigMessage[0][0] = 0`.
  6. `PrintString((float)(Wi/2i), yPos, m_BigMessage[0])` (Wi read once at 0x58C905, before the timers).
- gta-reversed: `plugin::Call<0x58C6A0>()` only. Open questions: none.

## 0x58D9A0 CHud::DrawWanted  (catalogue: specified/REA, port level 11)
- Convention: `__cdecl void ()`, plain `ret`; `sub esp, 0x4C` + ebx/ebp/esi/edi.
- Callers: CHud::Draw 0x58FAE0. Callees: CWorld::FindPlayerWanted 0x56E230 (cdecl (-1) -> CWanted*), CHud::GetYPosBasedOnHealth
  0x588B60 (cdecl (u8 player, float pos, int8 offset) -> ST0), CHudColours::GetRGBA 0x58FEE0, Common::AsciiToGxtChar 0x718600,
  CFont setters, PrintString 0x71A700, `_ftol` 0x821B40.
- Globals: `CHud::m_WantedState` (i32 0xBAA414), `m_WantedFadeTimer` (i32 0xBAA418), `m_WantedTimer` (i32 0xBAA41C),
  `m_LastWanted` (i32 0xBAA420), byte 0xBAB228 (local static "level unchanged" flag), HudColour colour 6 raw bytes
  0xBAB244/0xBAB245/0xBAB246 (r, g, b), `CTimer::ms_fTimeStep` 0xB7CB5C, `CTimer::m_snTimeInMilliseconds` 0xB7CB84,
  `CTimer::m_FrameCounter` 0xB7CB4C, CWorld::PlayerInFocus 0xB7CD74, CPlayerInfo +0x14F (u8, gr m_nMaxHealth) at
  0xB7CD98 + 0x14F + 0x190 * focus.
- Notation: `dms = _ftol((ms_fTimeStep * 0.02f) * 1000.0f)` (0x858B38, 0x858C4C), `dneg = _ftol((ms_fTimeStep * 0.02f) *
  -1000.0f)` (0x859948); each is computed afresh where used.
- Behaviour:
  1. `L = FindPlayerWanted(-1)->+0x2C` (current stars, ebp), `P = FindPlayerWanted(-1)->+0x30` (stars before parole, second call).
     `alpha = 255.0f` (x87). Load `T = m_WantedTimer`, `F = m_WantedFadeTimer`, `S = m_WantedState`.
  2. If `m_LastWanted != L` (0x58D9DF):
     a. If S is 0 (then `F = 0`), 1 or 3: `T = 5`, `S = 2`.
     b. If S is 0 or 5 after a. (only possible for 5): skip c. and d.
     c. State step: S == 1: `F = 1000`, `alpha = 255`; if `(float)T > 10000.0f` (0x859AA4; T before this frame's advance):
        `S = 3`, `F = 3000`. S == 2: `F += dms`; if `(float)F > 1000.0f` then `F = 1000`, `S = 1`; `alpha = ((float)F * 0.001f) * 255.0f`
        (0x858CDC). S == 3: `F += dneg`; if `(float)F < 0` then `F = 0`, `S = 0`; `alpha = ((float)F * 0.001f) * 255.0f`. Other S
        (4): alpha stays 255.
     d. `T += dms`.
     e. Store `m_WantedState = S`, `m_WantedTimer = T`, `m_WantedFadeTimer = F`; clamp alpha: `< 0` -> 0, `> 255` -> 255 (NaN
        unchanged); `m_LastWanted = L`; byte 0xBAB228 = 0.
  3. Else (unchanged): if S is not 0 and not 5: steps 2c and 2d; then store the three globals, clamp alpha the same way, byte
     0xBAB228 = 1.
  4. `alpha` stored f32. If `S == 0` return.
  5. `SetBackground(0, 0)`; `SetScale((W * kx) * 0.605f, (H * ky) * 1.21f)` (0x866C60, 0x866C64); `SetOrientation(2)`;
     `SetProportional(1)`; `SetFontStyle(0)`; `AsciiToGxtChar("]", star)` (local 2-byte ASCII string 0x5D 0x00).
     `x = W - (W * kx) * 29.0f` (0x863210; stored).
  6. Draw only if `(L > 0 && byte 0xBAB228 != 0) || P > 0`; else return. `a = (u8)_ftol(alpha)`.
  7. For i = 0..5 (edi):
     a. `SetEdge(1)`; `SetDropColor(CRGBA(0, 0, 0, a))`; `SetScale((W * kx) * 0.605f, (H * ky) * 1.21f)`.
     b. If `L > i` and (`m_snTimeInMilliseconds > wanted->m_LastTimeWantedLevelChanged (+0xC) + 2000` (unsigned) or
        `m_FrameCounter & 4`): `SetColor(HudColour.GetRGBA(6, a))`; `hy = H * ky`; `yb = 114.0f * hy` (0x866C5C, stored);
        `y = ((float)maxHealth < 101.0f) ? yb - hy * 12.0f : yb` (inline, 0x866BB4 = 101.0f, 0x858CCC = 12.0f; equal -> yb);
        `PrintString(x, y, star)`.
     c. Else if `P > i` and `m_FrameCounter & 4` (also reached from b. when `L > i` but the blink test failed):
        `SetColor(CRGBA(_ftol(r6 * 0.8f), _ftol(g6 * 0.8f), _ftol(b6 * 0.8f), a))` (bytes 0xBAB244..46, computed b, g, r;
        0.8f 0x858C98); `PrintString(x, GetYPosBasedOnHealth(PlayerInFocus, (H * ky) * 114.0f, 12), star)`.
     d. Else if `L <= i`: `SetEdge(0)`; `SetColor(CRGBA(0, 0, 0, (u8)_ftol(alpha * 0.7f)))` (0x858CB0);
        `SetScale(((W * kx) * 0.605f) * 1.2f, ((H * ky) * 1.21f) * 1.2f)` (0x858F08); `hy = H * ky` (stored);
        `PrintString(x, GetYPosBasedOnHealth(PlayerInFocus, hy * 114.0f, 12) - (hy + hy), star)`.
     e. (else nothing: L > i, blink off, and not c.)
     f. `x = x - (W * kx) * 18.0f` (0x859008; stored).
  8. `SetEdge(0)`; return.
- gta-reversed: hooked with `.Reversed = false`; Hud.cpp body is `plugin::Call` (not a reference). Open questions: none.

## 0x58B180 CHud::DrawMissionTimers  (catalogue: specified/REA, port level 12)
- Convention: `__cdecl void ()`, plain `ret`; `sub esp, 0xF0` + ebx/esi/edi (+ebp in the counter loop).
- Callers: CHud::Draw 0x58FAE0. Callees: CHud::GetYPosBasedOnHealth 0x588B60 (cdecl (u8 player, float, int8) -> ST0),
  CHudColours::GetRGB 0x58FEA0 (thiscall (CRGBA* out, u8 index) -> EAX = out), CText::Get 0x6A0050, Common::AsciiToGxtChar 0x718600,
  `_atol` 0x82258E, CSprite2d::DrawBarChart 0x728640 (cdecl (float x, float y, u16 width, u8 height, float progress, int8 add,
  u8 showPercent, u8 border, CRGBA colour, CRGBA addColour)), CFont setters (SetProportional, SetBackground, SetScale, SetOrientation,
  SetRightJustifyWrap 0x7194F0, SetFontStyle, SetWrapx 0x7194D0, SetEdge, SetDropColor, SetColor), PrintString 0x71A700, `_ftol`.
- Globals: `CUserDisplay::OnscnTimer` (COnscreenTimer, base 0xBA1788; see COnscreenTimer.md for the layout: clock key 0xBA178C,
  clock text 0xBA1796, clock m_bEnabled 0xBA17C0, counter c at 0xBA17C8 + 0x44*i, m_bDisplay 0xBA18D8);
  `CHud::TimerMainCounterHideState` (i16 0xBAA388), `TimerMainCounterWasDisplayed` (byte 0xBAA38A),
  `TimerCounterHideState` (i16[4] 0xBAA38C), `TimerCounterWasDisplayed` (byte[4] 0xBAA394), byte 0xBAAEC0 (`m_BigMessage[4][0]`),
  `CHud::bScriptForceDisplayWithCounters` (byte 0xBAA3FA), `CGarages::MessageIDString` (byte 0x96C014),
  CWorld::Players[1].m_pPed (dword 0xB7CF28), Players[1] +0x14F (u8 0xB7D077, max health), CTimer::m_FrameCounter 0xB7CB4C.
- Behaviour:
  1. Return if (`m_BigMessage[4][0] != 0` and `!bScriptForceDisplayWithCounters`) or `MessageIDString[0] != 0`.
  2. `hy = H * ky` (stored). `Y1 = GetYPos(1, GetYPos(focus, hy * 148.0f, 12), 12)` (148.0f 0x859518; the inner result is
     passed through memory, f32). `Y2 = GetYPos(1, GetYPos(focus, hy * 20.0f + hy*148 (the stored value), 12), 12)`
     (20.0f 0x858BA4; `fmul 20; fadd y148`). If Players[1].m_pPed != 0: `Y1 = Y1 + hy * 72.0f` and `Y2 = hy*72 + Y2` (0x858C90;
     the same x87 `hy*72` reused, both stored).
  3. Font: `SetProportional(1)`; `SetBackground(0, 0)`; `SetScale((W * kx) * 0.5f, H * ky)`; `SetOrientation(2)`;
     `SetRightJustifyWrap(0)`; `SetFontStyle(2)`; `SetWrapx((W * kx) * 640.0f)` (0x85951C); `SetEdge(2)`;
     `SetDropColor(CRGBA(0, 0, 0, 255))`.
  4. `clockOn = byte 0xBA17C0`. If Players[1].m_pPed != 0 and `!clockOn`: `TimerMainCounterWasDisplayed = 0`. For each counter
     i = 0..3: if `!c[i].m_bEnabled` then `TimerCounterWasDisplayed[i] = 0`.
  5. If `OnscnTimer.m_bDisplay != 1` return.
  6. Clock: if `clockOn == 1`:
     a. `h = WasDisplayed ? TimerMainCounterHideState : (HideState = 1)`; `WasDisplayed = 1`; if `h != 0`: `h++`, store; if
        `h > 50` store 0.
     b. `SetColor(HudColour.GetRGB(4))`. If `(m_FrameCounter & 4) || HideState == 0`: `AsciiToGxtChar(clock text, buf)`;
        `PrintString(W - (W*kx) * 32.0f, Y1, buf)` (0x85950C); if clock key[0] != 0:
        `PrintString((W - s*32) - s * 90.0f, Y1, TheText.Get(key))` (s = W * kx on x87; 90.0f 0x85991C).
     Else (clock off, 0x58B42A): `hy` recomputed and stored; `r = GetYPos(focus, hy * 148, 12)`; if `(float)Players[1].maxHealth
     < 101.0f`: `r = r - hy * 12.0f` (inline, 0x858CCC); `Y2 = Players[1].m_pPed ? hy * 72 + r : r`.
  7. Counters, i = 0..3 (c = counter i):
     a. Skip unless `c.m_bEnabled == 1`. If `!TimerCounterWasDisplayed[i] && c.m_bFlashWhenFirstDisplayed == 1`:
        `TimerCounterHideState[i] = 1`. `WasDisplayed[i] = 1`; `h = HideState[i]`; if `h != 0`: `h++`, store, `if (h > 50)` store 0.
     b. Skip unless `(m_FrameCounter & 4) || HideState[i] == 0`. `SetColor(HudColour.GetRGB(c.m_nColourId))`.
     c. `hy = H * ky` (x87); `yi = (((20.0f * hy) * (float)i) * hy) + Y2` (stored; note hy is multiplied in twice - the spacing grows
        with the square of the vertical scale, an original bug; `fimul` with the loop index).
     d. If `c.m_nType != 1`: `AsciiToGxtChar(c.m_szDisplayedText, buf)`; `PrintString(W - s * 32, yi, buf)`.
        Else (LINE, bar): `v = (i16)atol(c.m_szDisplayedText)`; `DrawBarChart(x = (W - s*32) - 61*s, y = hy * 6.0f + yi,
        width = (u16)_ftol(61.0f * s), height = (u8)_ftol(hy * 9.0f), progress = ((float)v * 0.01f) * 100.0f, 0, 0, 1,
        HudColour.GetRGB(c.m_nColourId), CRGBA(0, 0, 0, 0))` (61.0f 0x866C10, 9.0f 0x85EED4, 6.0f 0x858B44, 0.01f 0x858C58,
        100.0f 0x858628; the height is converted first, then the width).
     e. If `c.m_szDescriptionTextKey[0] != 0`: `PrintString((W - s*32) - s*90, yi, TheText.Get(c.m_szDescriptionTextKey))`.
- gta-reversed: hooked `.Reversed = false`, body `plugin::Call<0x58B180>()`. Open questions: none.

## 0x58B6E0 CHud::DrawHelpText  (catalogue: specified/REA, port level 12)
- Convention: `__cdecl void ()`, plain `ret`; `sub esp, 0x18` + ebx/ebp/esi/edi (edi = 4, esi = 2, ebp = 0 constants).
- Callers: CHud::Draw 0x58FAE0. Callees: CMessages::StringCompare 0x69DBD0 (cdecl (a, b, u16 len) -> AL), CMessages::StringCopy 0x69DB70
  (cdecl (dst, src, len)), CAudioEngine::ReportFrontendAudioEvent 0x506EA0 (thiscall (id, i32 0, float 1.0)), CFont setters
  (SetOrientation, SetJustify, SetWrapx, SetFontStyle, SetBackground, SetDropShadowPosition, SetBackgroundColor, SetColor,
  SetAlphaFade 0x719500, SetProportional, SetScaleForCurrentLanguage), CFont::GetNumberLines 0x71A5E0, CFont::GetStringWidth 0x71A0E0
  (cdecl (text, 1, 0) -> ST0), PrintString 0x71A700, CHudColours::GetRGB 0x58FEA0 / GetRGBA 0x58FEE0 (thiscall (out, index, alpha)),
  CText::Get, Common::AsciiToGxtChar, CRT sprintf 0x821BB5, CStats::GetStatValue 0x558E40 (cdecl (u16) -> ST0), CWorld::FindPlayerPed,
  CPedGroupMembership::CountMembersExcludingLeader 0x5F6AA0 (thiscall -> EAX), `fmax2` 0x420800 (cdecl (float a, float b) -> ST0 =
  `a > b ? a : b`, equal/NaN -> b), CSprite2d::DrawBarChart 0x728640, `_ftol`.
- Globals: `CHud::m_pHelpMessage` (GxtChar[400] 0xBAA7A0), `m_pLastHelpMessage` (0xBAA610), `m_pHelpMessageToPrint` (0xBAA480),
  `m_nHelpMessageState` (i32 0xBAA474), `m_nHelpMessageFadeTimer` (i32 0xBAA478), `m_nHelpMessageTimer` (i32 0xBAA47C),
  `m_fHelpMessageTime` (f32 0xBAA460), `m_bHelpMessagePermanent` (byte 0xBAA464), `m_bHelpMessageQuick` (byte 0xBAA472),
  `m_nHelpMessageStatId` (u16 0xBAA470), `m_nHelpMessageMaxStatValue` (u16 0xBAA46C), `m_fHelpMessageStatUpdateValue` (f32 0xBAA468),
  `m_fHelpMessageBoxWidth` (f32 0x8D0934), `CHud::PagerXOffset` (f32 0x8D0938), byte 0xB6F065 (TheCamera+0x3D), byte 0xBA6793
  (FrontEndMenuManager+0x4B), `CCutsceneMgr::ms_running`, m_BigMessage[0][0] (0xBAACC0) and [4][0] (0xBAAEC0),
  `CGarages::MessageIDString[0]`, CPedGroups::ms_groups (0xC09920, stride 0x2D4; membership at +8).
- Notation: `dms = _ftol((ms_fTimeStep * 0.02f) * 1000.0f)`; `s = W * kx`, `hy = H * ky`.
- Behaviour:
  1. If `m_pHelpMessage[0] == 0`: `m_nHelpMessageState = 0`; return.
  2. If `!StringCompare(m_pHelpMessage, m_pLastHelpMessage, 400)` (message changed): switch on the state (table 0x58C050):
     a. state 0: `state = 2`, `timer = 0`, `fade = 0`; `StringCopy(ToPrint, m_pHelpMessage, 400)`; `SetOrientation(1)`;
        `SetJustify(0)`; `SetWrapx((34*s + 200*s) - 4*s)` (34.0f 0x86322C, 200.0f 0x858A48, 4.0f 0x858B90; operand order
        `(s*34 + s*200) - s*4`); `SetFontStyle(1)`; `SetBackground(1, 1)`; `SetDropShadowPosition(0)`;
        `n = GetNumberLines(s' * 34.0f, (H * ky) * 28.0f, ToPrint)` (s' recomputed; 28.0f 0x859AD8);
        `m_fHelpMessageTime = (float)((i16)n + 3)`; `SetWrapx(W)`; `ReportFrontendAudioEvent(0x20, 0, 1.0f)`.
     b. states 1..4: `state = 4`, `timer = 5`. (Out-of-range states: nothing.)
     c. Always: `StringCopy(m_pLastHelpMessage, m_pHelpMessage, 400)`.
  3. `alpha = 200.0f`. If `state == 0` return. Switch on `state - 1` (table 0x58C064):
     - state 1 (shown): `alpha = 200.0f`, `fade = 600`; if `!m_bHelpMessagePermanent`: `t = (float)timer`; if
       `t > m_fHelpMessageTime * 1000.0f`, or (`m_bHelpMessageQuick` and `t > 3000.0f` (0x859A94)): `state = 3`, `fade = 600`.
     - state 2 (fade in): if byte 0xB6F065 != 0 skip (alpha 200). Else `fade += 2 * dms`; if `(float)fade > 0`: `fade = 0`,
       `state = 1`; `alpha = ((float)fade * 0.001f) * 200.0f` (0x858CDC). (With the 0.0f limit the fade-in ends on its first frame
       with alpha 0 - as in the bytes.)
     - state 3 (fade out): `fade -= 2 * dms`; if `(float)fade < 0` or byte 0xB6F065 != 0: `fade = 0`, `state = 0`;
       `alpha = (fade * 0.001f) * 200.0f`.
     - state 4 (switch): `fade -= 2 * dms`; if `(float)fade < 0`: `fade = 0`, `state = 2`, `StringCopy(ToPrint, m_pLastHelpMessage, 400)`;
       `alpha = (fade * 0.001f) * 200.0f`.
  4. If `ms_running` return (alpha fade not reset). `timer += dms`. `SetAlphaFade(alpha)`; `SetProportional(1)`;
     `SetScaleForCurrentLanguage(s * 0.52f, hy * 1.1f)` (0x8665BC, 0x858F14).
  5. Plain help (`m_nHelpMessageStatId == 0`):
     a. If m_BigMessage[0][0], m_BigMessage[4][0] or MessageIDString[0] is non-zero go to 7.
     b. `SetOrientation(1)`; `SetJustify(0)`; wrap: if `s * m_fHelpMessageBoxWidth == 200 * s` (exact; NaN unequal):
        `(s*34 + 200*s) - s*4` else `(m_fHelpMessageBoxWidth - 4.0f) * s + s*34`; `SetWrapx(wrap)`; `SetFontStyle(1)`;
        `SetBackground(1, 1)`; `SetDropShadowPosition(0)`; `SetBackgroundColor(CRGBA(0, 0, 0, (u8)_ftol(alpha)))`;
        `SetColor(HudColour.GetRGB(4))`.
     c. `v = 150 + ((byte 0xB6F065 != 0 && byte 0xBA6793 == 0) ? 0x38 : 0)`; `y = (((float)v - PagerXOffset) * 0.6f) * hy + hy * 28.0f`
        (0.6f 0x858CC8); `PrintString(s * 34.0f, y, ToPrint)`; go to 6.
  6. Stat help (`statId != 0`): if byte 0xB6F065 != 0 go to 7.
     a. `sprintf(gString, statId < 10 ? "STAT00%d" : statId < 100 ? "STAT0%d" : "STAT%d", statId)` (0x864300 / 0x8642F8 / 0x8642F0).
        `SetOrientation(1)`; `SetJustify(0)`; `SetWrapx(W)`; `SetFontStyle(1)`; `SetBackground(1, 1)`; `SetDropShadowPosition(0)`;
        `a = (u8)_ftol(alpha)`; `SetBackgroundColor(CRGBA(0, 0, 0, a))`; `SetColor(HudColour.GetRGB(4))`.
     b. `S = W * kx` (stored); `Xw = (GetStringWidth(TheText.Get(gString), 1, 0) + S * 34) + S * 10.0f` (stored; 0x85862C);
        `SetWrapx(S * 75.0f + Xw)` (0x858C8C). `HY = H * ky` (stored). `PrintString(s * 34, ((150.0f - PagerXOffset) * 0.6f) * HY +
        HY * 28, TheText.Get(gString))` (150.0f 0x858A28).
     c. `AsciiToGxtChar("+", gGxtString)` (0x866C18). `V = (statId != 0x150) ? GetStatValue(statId) :
        (float)CountMembersExcludingLeader(&ms_groups[FindPlayerPed(-1)->m_pPlayerData->m_nPlayerGroup (+0x38)].membership)` (stored).
     d. `add = HudColour.GetRGBA(ToPrint[0] == '+' ? 1 : 0, a)`, `col = HudColour.GetRGBA(4, a)`. `inv = 1.0f / (float)maxStat`
        (stored). `DrawBarChart(Xw, (155.0f - PagerXOffset) * 0.6f + HY * 28 (NOT multiplied by HY - original bug; 155.0f 0x859990),
        (u16)_ftol(s * 62.0f) (0x86503C), (u8)_ftol(HY * 12.0f), fmax2((inv * V) * 100.0f, 2.0f) (2.0f imm),
        (int8)_ftol(fmax2((inv * m_fHelpMessageStatUpdateValue) * 100.0f, 3.0f)), 0, 0, col, add)`. Evaluation order: progressAdd,
        progress, height, width, y.
     e. `PrintString(s * 65.0f + Xw (0x866C14), ((150 - PagerXOffset) * 0.6f) * hy + hy * 28, ToPrint)`.
     Both 5c and 6e are followed by `SetWrapx(W)`.
  7. `SetAlphaFade(255.0f)`; return.
- gta-reversed: hooked `.Reversed = false`, body `plugin::Call<0x58B6E0>()`. Open questions: none.

## 0x58EAF0 CHud::DrawPlayerInfo  (catalogue: specified/REA, port level 13)
- Convention: `__cdecl void ()`, plain `ret`; `sub esp, 0x1A0` + ebx/ebp/esi/edi. Locals: `ped = Players[focus].m_pPed`
  (stored at entry), `focus` (int), `Wf` (f32 copy of W, re-stored before each use).
- Callers: CHud::Draw 0x58FAE0. Callees: CHud::RenderHealthBar 0x589270, RenderArmorBar 0x5890A0, RenderBreathBar 0x589190 (all cdecl
  (int player, int x, int y)), CHud::DrawWeaponIcon 0x58D7D0 and CHud::DrawAmmo 0x5893B0 (cdecl (CPed*, int x, int y, float alpha)),
  CHud::GetYPosBasedOnHealth 0x588B60, CPedIntelligence::GetTaskSwim 0x601070 (thiscall), CStats::GetFatAndMuscleModifier 0x559AF0
  (cdecl (8) -> ST0), CHudColours::GetRGB / GetRGBA, CFont setters, PrintString, Common::AsciiToGxtChar, CRT sprintf, `_ftol`.
- Globals: `CHud::bDrawClock` (i32 0xBAA400), CClock::ms_nGameClockHours (byte 0xB70153) / Minutes (byte 0xB70152), CPlayerInfo
  fields at 0xB7CD98 + 0x190*focus: +0x0 m_pPed, +0xBC (display money, i32, 0xB7CE54 for player 0), +0xE4 (last time energy lost,
  0xB7CE7C); Players[1].m_pPed 0xB7CF28; state machines `m_EnergyLost{State,FadeTimer,Timer}` 0xBAA434/38/3C + `m_LastTimeEnergyLost`
  0xBAA440, `m_DisplayScore{State,FadeTimer,Timer}` 0xBAA424/28/2C + `m_LastDisplayScore` 0xBAA430, `m_Weapon{State,FadeTimer,Timer}`
  0xBAA404/08/0C + `m_LastWeapon` 0xBAA410, `CHud::m_LastBreathTime` (u32 0xBAA3FC), CTimer::m_snTimeInMilliseconds 0xB7CB84.
- Fade state machine M(key, last, S, F, T, withAlpha) - identical to steps 2-3 of DrawWanted (same constants 0.02f, +/-1000.0f,
  1000.0f, 10000.0f, 3000, 5; alpha `((float)F * 0.001f) * 255.0f`, clamped to [0, 255] after the globals are stored):
  if `last != key` reset (S 0 -> F = 0; S in {0, 1, 3} -> T = 5, S = 2), step, `T += dms`, store S/T/F, then `last = key`;
  else (S not 0/5) step, `T += dms`, store. For the energy machine no alpha is computed (the x87 alpha code is absent; only S/F/T).
- Behaviour:
  1. Clock: if `bDrawClock == 1`: `SetBackground(0, 0)`; `SetScale((W * kx) * 0.55f, (H * ky) * 1.1f)` (0x866CAC, 0x858F14);
     `SetProportional(0)`; `SetFontStyle(3)`; `SetOrientation(2)`; `SetRightJustifyWrap(0)`; `SetEdge(2)`;
     `SetDropColor(CRGBA(0, 0, 0, 255))`; `sprintf(buf, "%02d:%02d" (0x859A6C), hours, minutes)`; `AsciiToGxtChar(buf, gbuf)`;
     `SetColor(HudColour.GetRGB(4))`; `PrintString(W - (W * kx) * 32.0f, (H * ky) * 22.0f (0x858F8C), gbuf)`; `SetEdge(0)`.
  2. Energy machine with key = Players[focus]+0xE4. If the resulting state is 0, skip to 6 (no health/armour/breath bars).
  3. Health: `RenderHealthBar(focus, _ftol(Wf - (Wf * kx) * 141.0f), _ftol(GetYPos(focus, (H * ky) * 77.0f, 10)))` (0x86535C, 0x866CA8;
     y converted first). If Players[1].m_pPed: `RenderHealthBar(1, x as before, _ftol(GetYPos(1, GetYPos(focus, hy * 194.0f, 12), 12)))`
     (0x866CA4).
  4. Armour: `RenderArmorBar(focus, _ftol(Wf - (Wf * kx) * 94.0f), _ftol(GetYPos(focus, hy * 48.0f, 3)))` (0x866B78, 0x862D38);
     player 2: `RenderArmorBar(1, same x, _ftol(GetYPos(1, GetYPos(focus, hy * 164.0f, 12), 12)))` (0x866CA0).
  5. Breath: `show1 = 0`, `show2 = 0`.
     a. Player 1 (`ped`): if `GetTaskSwim(ped->m_pIntelligence (+0x47C)) != 0`, or (`ped->+0x46C & 0x100` (in vehicle) and
        `v = ped->m_pVehicle (+0x58C)` non-null and `v->+0x40 & 0x100` and `byte v->+0x42B & 0x40`): `show1 = 1`,
        `m_LastBreathTime = now`. Else if `GetFatAndMuscleModifier(8) > ped->m_pPlayerData->+0x44` (breath not full; equal/NaN no)
        and `m_LastBreathTime + 500 > now` (unsigned): `show1 = 1`, `m_LastBreathTime = now`.
     b. Player 2 (if Players[1].m_pPed): same swim / vehicle tests, else `GetFatAndMuscleModifier(8) > ped2->m_pPlayerData->+0x44`
        - WITHOUT the 500 ms test; any hit: `show2 = 1`, `m_LastBreathTime = now`.
     c. If show1: `RenderBreathBar(focus, _ftol(Wf - (Wf * kx) * 94.0f), _ftol(GetYPos(focus, hy * 62.0f, 6)))` (0x86503C).
        If show2 and Players[1].m_pPed: `RenderBreathBar(1, same x, _ftol(GetYPos(1, GetYPos(focus, hy * 179.0f, 12), 12)))` (0x866C9C).
  6. Money: score machine with key = Players[focus]+0xBC (with alpha). If the state is not 0: `a = (u8)_ftol(alpha)`;
     if key >= 0: `SetColor(HudColour.GetRGBA(1, a))`, `sprintf(buf, "$%08d" (0x866C94), abs(money))`; else
     `SetColor(HudColour.GetRGBA(0, a))`, `sprintf(buf, "-$%07d" (0x866C8C), abs(money))` (money re-read; `neg` if negative);
     `AsciiToGxtChar(buf, gbuf)`; `SetProportional(0)`; `SetBackground(0, 0)`; `SetScale((W*kx) * 0.55f, (H*ky) * 1.1f)`;
     `SetOrientation(2)`; `SetRightJustifyWrap(0)`; `SetFontStyle(3)`; `SetDropShadowPosition(0)`; `SetEdge(2)`;
     `SetDropColor(CRGBA(0, 0, 0, a))`; `PrintString(Wf - (Wf * kx) * 32.0f, GetYPos(focus, hy * 89.0f, 12) (0x866C88), gbuf)`;
     `SetEdge(0)`.
  7. Weapon: weapon machine with key = type of `ped`'s active weapon (with alpha; `m_LastWeapon` = the type re-read after the step).
     If the state is 0 return. `alpha` stored f32 and passed as float bits.
     a. `DrawWeaponIcon(ped, _ftol(W - ((W * kx) * 32.0f + W * 0.17343046f)), _ftol(hy * 20.0f), alpha)` (0x866C84 = 0.17343046f;
        y converted first; the x expression is `W - (s*32 + W*0.17343046)` with s on x87).
     b. If Players[1].m_pPed (ped2): `DrawWeaponIcon(ped2, _ftol(Wf - ((Wf * kx) * 32.0f + 111.0f)), _ftol(GetYPos(focus, hy * 138.0f, 12)),
        alpha)` (111.0f 0x866C7C is not scaled - original quirk; 138.0f 0x866C80).
     c. With `Wf` stored and `S = W * kx` stored: `DrawAmmo(ped, _ftol((Wf - (Wf * 0.17343046f + S * 32)) + (S * 47.0f) * 0.5f),
        _ftol(hy * 20 + hy * 43.0f), alpha)` (47.0f 0x866C4C, 43.0f 0x866C78).
     d. If ped2: `DrawAmmo(ped2, same x formula, _ftol(GetYPos(focus, hy * 138 + hy * 43, 12)), alpha)`.
- gta-reversed: hooked `.Reversed = false`, body `return plugin::Call<0x58EAF0>()`. Open questions: none.

## 0x589650 CHud::DrawVitalStats  (catalogue: specified/REA, port level 12; never executed in the coverage corpus)
- Convention: `__cdecl void ()`, plain `ret`; `sub esp, 0x28` + ebx/ebp/esi/edi.
- Callers: CHud::Draw 0x58FAE0. Callees: CWorld::FindPlayerPed, CPedIntelligence::GetTaskSwim 0x601070, CMenuManager::DrawWindow 0x573EE0
  (thiscall on FrontEndMenuManager: (const CRect*, const char* title, u8 0, CRGBA, u8 0, u8 1)), CText::Get, CStats::GetStatValue 0x558E40
  (cdecl (u16) -> ST0), CWeaponInfo::GetSkillStatIndex 0x743CD0 (cdecl (type) -> EAX), `floorf` wrapper 0x406CE0 (cdecl (float) ->
  ST0 = CRT floor 0x8219F0 of the value widened to double; the x87 fallback uses control word 0x173F = round down), CSprite2d::DrawBarChart
  0x728640, CFont setters, PrintString, CRT sprintf, `_ftol`.
- Globals: `CReplay::Mode` (byte 0xA43088), byte 0xB6F065, CWorld::Players[focus].m_pPed, stat tables f32 at `0xB78EB4 + 4*idx` and
  `0xB78F40 + 4*(idx - 0x45)` (inside CStats::StatReactionValue 0xB78F10), `CClock::CurrentDay` (byte 0xB7014E).
- Behaviour:
  1. Return if `CReplay::Mode == 1` or byte 0xB6F065 != 0.
  2. `wt` = type of FindPlayerPed(-1)'s active weapon; if `wt == 0x20` then `wt = 0x1C`.
  3. `SetBackground(0, 0)`; `SetColor(CRGBA(0xE1, 0xE1, 0xE1, 0xFF))`; `SetWrapx((W * kx) * 640.0f)`; `SetRightJustifyWrap(0)`;
     `SetProportional(1)`. With `s = W * kx` on x87: `xL = _ftol(s * 10.0f + 40.0f)` (10.0f 0x85862C, 40.0f 0x858A10),
     `xB = _ftol(s * 90.0f + 40.0f)` (0x85991C), `gap = _ftol((H * ky) * 5.0f)` (0x858C80). (All later used as int16.)
  4. Window (`hy = H * ky`, `T = H - hy * 140.0f` (0x865374), on x87): if the focus player's GetTaskSwim() is null and `wt` is not in
     0x16..0x20: rect `{40.0f, hy * 127.0f + T (0x86532C), s * 170.0f (0x858F98) + 40.0f, hy * 15.0f + T (0x858B48)}`, and later
     `y = _ftol(T + (hy * 15) * 2)` (`fadd st0, st0`); else rect `{40.0f, hy*127 + T, s*170 + 40, T}` and `y = _ftol(T + hy * 15)`.
     `FrontEndMenuManager.DrawWindow(&rect, "FEH_STA" (0x86613C), 0, CRGBA(0, 0, 0, 0xBE), 0, 1)`.
  5. `SetFontStyle(1)`; `SetOrientation(1)`; `SetScale(s * 0.35f, hy * 0.9f)` (0x858F9C, 0x858C20); `SetColor(CRGBA(0xE1,0xE1,0xE1,0xFF))`;
     `SetEdge(0)`. `XL = (float)(i16)xL`, `XB = (float)(i16)xB`. A "row(label, p)" below is:
     `PrintString(XL, (float)(i16)y, TheText.Get(label))`; `DrawBarChart(XB, (float)((i16)gap + (i16)y), (u16)_ftol(s * 70.0f) (0x858CE0),
     (u8)_ftol(hy * 10.0f), p, 0, 0, 1, CRGBA(0xC8, 0xC8, 0xC8, 0xFF), CRGBA(0, 0, 0, 0))` (height converted before width);
     then `y = _ftol(hy * 15.0f + (float)(i16)y_prev)` (the float of the row's y, stored).
  6. Rows in order:
     a. row("STAT068" 0x866C00, `(GetStatValue(0x40) * 0.001f) * 100.0f`) (0.001f 0x858CDC, 100.0f 0x858628).
     b. If swimming: row("STAT225" 0x866BF8, `(GetStatValue(0xE1) * 0.001f) * 100.0f`).
        Else if `wt` in 0x16..0x20: label "CURWSKL" (0x866BF0) printed as in a row; `k = GetSkillStatIndex(wt)`;
        `maxV = f32[0xB78EB4 + 4k] * f32[0xB78F40 + 4(k - 0x45)]` (stored); `v = GetStatValue(k)`; if `v > 999.0f` (0x866BEC; equal
        or NaN go to the other arm) `r = 1.0f` else `r = (floorf(((f32[0xB78F40 + 4(k-0x45)] * 0.1f (0x858B1C) + v) / maxV) (the quotient
        stored f32 first)) * maxV) * 0.001f`; bar with `p = r * 100.0f`; then the y advance.
        Else: no row and no y advance.
     c. row("STAT022" 0x866BE4, stat 0x16); row("STAT023" 0x866BDC, stat 0x17); row("STAT021" 0x866BD4, stat 0x15);
        row("STAT025" 0x866BCC, stat 0x19) - each `p = (GetStatValue(id) * 0.001f) * 100.0f`. After the last bar the y advance is
        instead `yd = _ftol((hy * 15 + hy * 3.0f) + (float)(i16)y)` (3.0f 0x858B3C).
  7. Day: `SetFontStyle(3)`; `SetOrientation(2)`; `SetScale(s * 0.7f, hy * 0.7f)` (0x858CB0); `SetEdge(1)`;
     `SetColor(CRGBA(0xC8,0xC8,0xC8,0xFF))`; `SetDropColor(CRGBA(0,0,0,0xFF))`; `sprintf(gString, "DAY_%d" (0x866BC4), CurrentDay)`;
     `PrintString(s * 160.0f (0x858970) + 40.0f, (float)(i16)yd, TheText.Get(gString))`.
- gta-reversed: hooked `.Reversed = false`, body `plugin::Call<0x589650>()`.
- Open questions: the meaning of the two stat tables (only addresses are certain, 0x589B9A / 0x589BA4); 0x406CE0 is floor by its x87
  fallback control word (0x173F, round down) - the SSE2 path at 0x821A30 was not traced instruction by instruction.
