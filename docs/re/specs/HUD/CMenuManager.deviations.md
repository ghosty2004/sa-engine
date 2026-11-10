# CMenuManager (HUD) - gta-reversed deviation notes (group H1)

Sources: `re/gta-reversed/source/game_sa/MenuManager.cpp`, `Frontend/MenuManager_{Input,Process,Draw,Pages}.cpp`.
`this` = FrontEndMenuManager 0xBA6748 (some members are accessed by absolute address, e.g. 0xBA827C = this+0x1B34
m_StatusDisablePlayerControls, 0xBA67CC = this+0x84 m_nPrefsLanguage). Menu item row: `aScreens` 0x8CE008, screen stride
0xE2, item stride 0x12, item action type byte at 0x8CE012 + screen*0xE2 + item*0x12. Action ids (gr MenuManager_Internal.h,
build without RU/JP): 2 BACK, 5 MENU, 7 CTRLS_JOYPAD, 8 FOOT, 9 CAR, 21 STAT, 27 BRIGHTNESS, 28 RADIO_VOL, 29 SFX_VOL,
32 RADIO_STATION, 57 RESET_CFG, 61 DRAW_DIST, 62 MOUSE_SENS.

## 0x572EC0 CMenuManager::LoadAllTextures  (gr_deviations = 1)
Row MenuManager.cpp:250 (`LoadTexture(0x6800, "frontend3", ...) // NOTSA`):
- Original bytes 0x572EC0-0x573099: if `m_bTexturesLoaded (+0x15C)` return; `CTimer::Stop`; then three blocks, each
  `MakeSpaceFor(size); ImGonnaUseStreamingMemory(); [TidyUpMemory(0,1)]; PushCurrentTxd(); slot = FindTxdSlot(name);
  if (slot == -1) slot = AddTxdSlot(name); LoadTxd(slot, file); AddRef(slot); SetCurrentTxd(slot); for each sprite:
  SetTexture(name, alpha) 0x7272B0 + SetAddressing(3) 0x727320; PopCurrentTxd(); IHaveUsedStreamingMemory()`:
  1. 0x21000, "frontend1", "MODELS\FRONTEN1.TXD", sprites +0xF8.. (13; names 0x8CDF28..0x8CDF90), with TidyUpMemory.
  2. 0x3000, "frontend_pc", "MODELS/FRONTEN_pc.TXD", sprites +0x154.. (2; names 0x8CDFE0..0x8CDFF0), with TidyUpMemory.
  3. 0x4C000, "frontend2", "MODELS\FRONTEN2.TXD", sprites +0x12C.. (8; names 0x8CDF90..0x8CDFD0), NO TidyUpMemory.
  Then `m_bTexturesLoaded = 1`, tail-jump CTimer::Update.
  gr loads frontend1 (0x21000), frontend2 (0x3000), frontend3 (0x6800, NOTSA), frontend_pc (0x4C000), TidyUpMemory each time.
- Verdict: CONFIRMED (extra frontend3 load; also order 1/pc/2, the sizes of frontend2 / frontend_pc swapped, and no
  TidyUpMemory before frontend2). Class: keep-original.

## 0x573440 CMenuManager::CheckSliderMovement  (gr_deviations = 3)
Original: `__thiscall void (int8 lr [esp+4])` (read with `movsx byte`), `ret 4`; switch on action - 27 (0x57345E,
table 0x573658/0x573640); unknown actions return without SaveSettings (0x57363B).
Row MenuManager_Input.cpp:889 (`assert(false && "value invalid ...")`): no check in the bytes. Verdict: REFUTED
(debug assert only). Class: harmless.
Row :917 (BUG comment, mouse sensitivity): comment only; the bytes 0x5735D1-0x573634 compute what gr computes in shape
(see below). Verdict: REFUTED. Class: harmless.
Row :919 (`#ifdef FIX_BUGS m_fMouseAccelVertical = horz * 0.6f`): the original writes only `CCamera::m_fMouseAccelHorzntl`
0xB6EC1C. Verdict: CONFIRMED. Class: keep-original.
Found in passing (CONFIRMED, keep-original):
- Brightness: `v = _ftol((float)lr * 24.19f (0x865258) + m_PrefsBrightness (+0x3C, fiadd))` (0x573481-0x57348E); gr uses
  363/15 = 24.2f. Clamp: `v = max(v, 0)`, then `if (v > 384.0f (0x865254)) v = 384` via x87, then
  `CGamma::SetGamma(&gamma 0xC92134, v * 0.001953125f (0x863A44), 0)` 0x747200; +0x3C is written 3 times (intermediates).
- Draw distance: step 0.0546874963f (f32 at 0x865250 = 0x3D5FFFFF), not 7/128 (0x3D600000) as in gr. `d = d +/- step`
  (lr > 0 adds), stored; `if (d > 1.8f) d = 1.8f` (0x859BB4; NaN keeps), `if (d < 0.925f) d = 0.925f` (0x86524C);
  `CRenderer::ms_lodDistScale (0x8CD800) = d`.
- Mouse: `h = ((float)lr * 0.005f (0x8651A8)) * 0.06666667f (0x863E0C) + h` (gr divides by 15.0f: different rounding);
  `if (!(minSens [0xBA672C] < h) && !(minSens == h)) h = minSens` i.e. `if (minSens > h) h = minSens`; then
  `if (!(h < 0.005f)) h = 0.005f` (0x57361B-0x57362E: equal or NaN also store 0.005f).
- Radio/SFX volume (+0x50 / +0x4F, int8): `v = (int8)(v + (int8)(lr << 2))`; if the byte add result is negative
  (sign flag) v = 0; `if (v >= 64) v = 64`; then CAudioEngine::SetMusicMasterVolume 0x506DE0 / SetEffectsMasterVolume 0x506E10.

## 0x574010 CMenuManager::SmallMessageScreen  (gr_deviations = 1)
Row MenuManager.cpp:1100 (`GxtCharToUTF8(TheText.m_szCdErrorText, text)`):
- Original bytes 0x57426F-0x5742DC: `text = TheText.Get(key)`; if `TheText.m_bCdErrorLoaded` (byte 0xC1B362) == 0 and
  `_stricmp("NOCD", key) == 0 || _stricmp("OPENCD", key) == 0 || _stricmp("WRONGCD", key) == 0 || _stricmp("CDERROR", key)
  == 0` (0x8229B6, short-circuit in that order): `AsciiToGxtChar(TheText.m_szCdErrorText (0xC1B36C), text)` 0x718600 -
  converts the ASCII CD-error text INTO the buffer returned by TheText.Get (overwriting the GXT string in memory).
  gr converts in the opposite direction (from `text` into m_szCdErrorText) and prints `text` unchanged.
- Verdict: CONFIRMED. Class: keep-original.
Found in passing: the y position is converted to int with `_ftol` (0x57426A) and back with `fild` (0x574305) before
PrintString (whole-pixel y); gr keeps it float. CONFIRMED, keep-original.

## 0x574630 CMenuManager::UnloadTextures  (gr_deviations = 1)
Row MenuManager.cpp:314 (`Unload("frontend3", ...) // NOTSA`):
- Original bytes 0x574630-0x5746E1: if `m_bTexturesLoaded`: `SwapTexturesRound(0)` 0x5730A0; then for ("frontend1",
  13 sprites at +0xF8), ("frontend_pc", 2 at +0x154), ("frontend2", 8 at +0x12C): `slot = FindTxdSlot(name)` FIRST,
  then `Delete()` each sprite, then `RemoveTxd(slot)` 0x731E90; finally `m_bTexturesLoaded = 0`. No frontend3 unload.
- Verdict: CONFIRMED (extra frontend3 unload in gr; order 1/pc/2 vs gr 1/2/3/pc). Class: keep-original.

## 0x576B70 CMenuManager::CheckForMenuClosing  (gr_deviations = 4)
Rows MenuManager_Input.cpp:1029, :1123 (`#ifndef NOTSA_USE_SDL3` around `if (IsVideoModeExclusive()) { DIReleaseMouse();
InitialiseMouse(false); }`):
- Original bytes 0x576EB3-0x576EC7 (menu entering) and 0x576F83-0x576F97 (save-done path): `VideoMode::IsVideoModeExclusive
  0x745CA0`; if true `DIReleaseMouse 0x746F70; Input::diMouseInit(0) 0x7469A0` (ebx = 0). Matches gr with SDL3 off.
- Verdict: REFUTED (the code exists in the original; a port must keep it regardless of gr's SDL3 build option). Class: harmless.
Row :1055 (`#ifndef NOTSA_USE_SDL3` on the closing path) and row :1058 (`#ifdef FIX_BUGS InitialiseMouse(false) #else
InitialiseMouse(true)`):
- Original bytes 0x576CC5-0x576CDA: `if (IsVideoModeExclusive()) { DIReleaseMouse(); diMouseInit(1); }` (`push 1`).
- Verdict: :1055 REFUTED (block present), :1058 CONFIRMED (original passes 1 = the #else branch). Class: keep-original.
Found in passing (CONFIRMED, keep-original): on menu entering the original, after saving `pad->DisablePlayerControls`
(+0x10E) into 0xBA827C, sets `pad->DisablePlayerControls = 1` (0x576E67) before `pad->Clear(0, 1)`; gr
(MenuManager_Input.cpp:1024) omits the assignment.

## 0x576FE0 CMenuManager::ProcessMenuOptions  (gr_deviations = 1)
Row MenuManager_Process.cpp:212 (MENU_ACTION_MPACK, NOTSA_UNREACHABLE when `m_nCurrentScreenItem - 2` is out of range):
- Original bytes 0x57724F-0x577266: `m_nMissionPackGameId (+0x160) = *(u8*)(this + item*0x105 - 0xA9)` (=
  `m_MissionPacks[item - 2].m_Id`, packs at +0x161, stride 0x105) with no range check, then `SwitchToNewScreen(8)`.
- Verdict: CONFIRMED (out-of-range only). Class: keep-original (unchecked read).

## 0x5773D0 CMenuManager::AdditionalOptionInput  (gr_deviations = 7)
Rows MenuManager_Input.cpp:367, :440, :485 (comments about the 20 ms tick): the original tests
`(u32)(CTimer::m_snTimeInMillisecondsPauseMode (0xB7CB7C) - lastActionTime (0xBA8280)) > 20` (`cmp eax, 0x14; jbe`,
0x577656). Verdict: REFUTED (comments describe the original). Class: harmless.
Rows :438 and :483 (`if (notsa::bugfixes::GenericFrameRate)` scaled zoom steps):
- Original bytes (zoom in) 0x5775E5-0x5776A1: condition `(LS2 (+0xA) != 0 && RS2 (+0xE) == 0) || NewMouse.wheelUp (byte
  0xB7341B) || NewKeyState.pgup (0xB733B2)`; if the 20 ms test passes: if `!(zoom (+0x64) < 1100.0f)` (0x864F94, also NaN)
  -> clamp branch 0x5776C8; else `zoom += 7.0f` (0x858F48), and `zoom += 21.0f` (0x8653E4) if wheelUp; fixed steps only.
  Zoom out mirrors it from 0x577918 (300.0f limit, -7 / -21).
- Verdict: CONFIRMED (gr's runtime fix scales by elapsed time; original fixed +/-7, +/-21). Class: keep-original (fix off).
Row :670 (NOTSA lambda INSERT_BLIP_TOGGLE_ROWS):
- Original bytes 0x578944-0x578A2E and again before 0x578B4A: five local 10-byte buffers filled with "FEM_ON"
  (0x86539C) or "FEM_OFF" (0x8653A4) from the bytes +0x45..+0x49, then `CMenuSystem::InsertMenu(m_nSysMenu (+0x30), 1, 0,
  buf1..buf5, 0 x7)` 0x581E00; the label column is `InsertMenu(.., 0, 0, "FED_BL1".."FED_BL5")` at 0x57893F.
- Verdict: REFUTED (same calls and order, only factored into a lambda). Class: harmless.
Row MenuManager_Pages.cpp:51 (`CMenuManager::RadarZoomIn`, NOTSA helper "0x5775E5 to 0x577918"): the helper ANDs all
four conditions, unlike the original OR (above), but it is never called in gr (only declared, MenuManager.h:389).
Verdict: REFUTED as a behaviour difference of 0x5773D0 (dead gr code). Class: harmless (do not port the helper).

## 0x578D60 CMenuManager::ProcessFileActions  (gr_deviations = 1)
Row MenuManager_Process.cpp:143 (`CFileMgr::SetDir(""); // FIX_BUGS` after the mission-pack probe):
- Original bytes 0x578DA4-0x578DEA: `SetDirMyDocuments(); sprintf(gString, "MPACK//MPACK%d//SCR.SCM" (0x859D78),
  bMissionPackGame); f = OpenFile(gString, "rb")`; `if ((int)f > 0) CloseFile(f)` (signed test, 0x578DD4 `jle`); no
  SetDir("") afterwards (the current directory stays My Documents). On failure it re-tests bMissionPackGame and goes to
  the "FES_NIM" error screen (0x578E3D).
- Verdict: CONFIRMED. Class: keep-original.

## 0x5794A0 CMenuManager::DrawStandardMenus  (gr_deviations = 5)
Row MenuManager_Draw.cpp:517 (MPACK entry index out of range -> NOTSA_UNREACHABLE):
- Original bytes 0x579CD8-0x579D4E (item type 9): `id = *(u8*)(this + i*0x105 - 0xA9)` (= m_MissionPacks[i - 2].m_Id), no
  range check.
- Verdict: CONFIRMED (out-of-range only). Class: keep-original.
- Found in passing (CONFIRMED, keep-original): the "empty pack" test is `m_Id == 0` (0x579CF0 `test cl, cl; jbe`), not
  `m_Name` empty as in gr (MenuManager_Draw.cpp:507). Non-zero id -> `AsciiToGxtChar(m_Name (this + i*0x105 - 0xA8),
  gGxtString 0xC1B100)`, action byte = 0xB (MPACK); zero id -> action byte = 0x14 (SKIP), text = null, `item.m_Y =
  previous item's m_Y` (word copy 0x579D3B-0x579D47).
Row :543 (NOTSA: save name truncated to 32 chars with "..."):
- Original bytes 0x579BE0-0x579C19: `name = CGenericGameStorage::GetNameOfSavedGame(i - 1)` 0x5D0F40; if
  `GxtCharStrlen(name) (0x718690) >= 0xFE`: `AsciiToGxtChar("..." 0x86550C, gGxtString2 0xC1AED8)` and
  `GxtCharStrcat(name, gGxtString2)` 0x718630 - appends "..." to the stored name itself, no copy, no truncation; the
  displayed text is `name`. Names shorter than 254 are shown in full.
- Verdict: CONFIRMED (gr truncates at 32; visible for names of 32..253 chars). Class: keep-original.
Rows :607 (radar mode), :617 (language), :626 (FX quality) `default: NOTSA_UNREACHABLE()`:
- Original bytes: radar 0x579E37-0x579E62 (0 "FED_RDM", 1 "FED_RDB", 2 "FEM_OFF", other -> 0x57A168), language
  0x579DEE (`cmp eax, 4; ja 0x57A168`, table 0x57B3AC), FX 0x579EB3 (`cmp eax, 3; ja 0x57A168`, table 0x57B3C0): out of
  range jumps past the `TheText.Get` at 0x57A15C, leaving the right-column text unchanged.
- Verdict: CONFIRMED (out-of-range only). Class: keep-original.

## 0x57B750 CMenuManager::DrawBackground  (gr_deviations = 1)
Row MenuManager_Draw.cpp:185 (`default: assert(true); // Bad R*`, returns `{}`):
- Original bytes 0x57B7AC-0x57B7E1: the rect is pre-set to (1000000.0, -1000000.0, -1000000.0, 1000000.0) and the switch
  on `sprite - 13` (0..6, table 0x57C274) has no default store, so other sprites keep the default rect - identical to
  gr's default-constructed CRect.
- Verdict: REFUTED. Class: harmless.

## 0x57C520 CMenuManager::CentreMousePointer  (gr_deviations = 2)
Rows MenuManager.cpp:629 (`#ifdef FIX_BUGS if (!IsForegroundApp()) return;`) and :630 (its comment):
- Original bytes 0x57C520-0x57C5D9: no foreground test. `x = W * 0.5f`, `y = H * 0.5f` (fild of 0xC17044 / 0xC17048,
  0.5f 0x858B8C); if `x == 0.0f` or `y == 0.0f` (0x858B50, `test ah, 0x44; jnp`) return; else (inlined RsMouseSetPos)
  `pt = {W / 2, H / 2}` (signed int halving: `cdq; sub eax, edx; sar eax, 1`), `ClientToScreen(RsGlobal.ps->window, &pt)`,
  `SetCursorPos(pt.x, pt.y)`, then `ps->lastMousePos.x (+0xC) = (float)(W / 2)`, `ps->lastMousePos.y (+0x10) = (float)(H / 2)`.
- Verdict: :629 CONFIRMED, :630 REFUTED (comment of the same block). Class: keep-original.

## 0x57CD50 CMenuManager::ProcessPCMenuOptions  (gr_deviations = 7)
Original: `__thiscall bool (int8 lr [esp+4], bool accept [esp+8])`, `ret 8`; switch on action - 6 (table 0x57D4DC/0x57D454);
handled actions return AL = 1, unhandled AL = 0 (0x57D447).
Row MenuManager_Process.cpp:361 (CTRLS_JOYPAD default unreachable): 0x57D2FA: `m_ControlMethod (+0xD0) == 1 ?
SwitchToNewScreen(0x28) : SwitchToNewScreen(0x27)` - any value other than 1 goes to the mouse screen.
Verdict: CONFIRMED (invalid values only). Class: keep-original.
Row :563 (CONTROL_TYPE default unreachable): 0x57CF5B: `if (+0xD0 == 1) { +0xD0 = 0; m_bUseMouse3rdPerson (0xB6EC2E) = 1 }
else { +0xD0 = 1; m_bUseMouse3rdPerson = 0 }`, SaveSettings. Verdict: CONFIRMED (invalid values only). Class: keep-original.
Row :379 (radar mode `+ (IsFixBugs() ? lr : 1)`): 0x57D3A5: `m_nRadarMode (+0x24) += 1; if (> 2) = 0` (lr ignored).
Verdict: CONFIRMED. Class: keep-original (+1).
Row :409 (FX quality step): 0x57D185-0x57D212: `q = g_fx.GetFxQuality()`; 3 -> Set(0); 2 -> Set(3); 1 -> Set(2); 0 -> Set(1)
(each followed by SaveSettings and return 1); other values -> SaveSettings only (0x57D438). lr ignored.
Verdict: CONFIRMED. Class: keep-original.
Row :582 (user-tracks play mode): 0x57D21F: `m = (int8)(m (+0xB0) + lr)`; if negative (sign flag of the byte add) m = 2;
if `m > 2` m = 0. With lr == 0 the mode is unchanged (gr's fix turns 0 into +1). Verdict: CONFIRMED. Class: keep-original.
Rows :510 and :526 (FIX_BUGS bounded video-mode scans): 0x57CF9B-0x57D073 (when !accept): `list = GetVideoModeList()`
0x745AF0; lr > 0: `do ++mode (+0xD8) while (list[mode] == 0)` with no upper bound, then if `mode >= GetNumVideoModes()`
(via thunk 0x745A70): mode = 0 and `while (list[mode] == 0) ++mode`; lr <= 0: `do --mode while (list[mode] == 0)` with no
lower bound, then if `mode < 0`: mode = num - 1 and `while (list[mode] == 0) --mode`. Verdict: CONFIRMED (unbounded scans =
the #else branch). Class: keep-original.

## 0x57D520 CMenuManager::ProcessMissionPackNewGame  (gr_deviations = 1)
Row MenuManager_Process.cpp:624 (NOTSA: `fscanf_s("%hhu#%[^\n\r#]#")` into `m_MissionPacks[i]`):
- Original bytes 0x57D520-0x57D71A: all 25 `m_MissionPacks[k].m_Id = 0` (+0x161, stride 0x105). `found = 0`, `k = 0` (u8,
  shared across files). `SetDirMyDocuments()` inside the loop (0x57D563); for file i = 0..24:
  `sprintf(gString, "MPACK//MPACK%d//MPACK.DAT", i)`, `f = OpenFile(gString, "rb")`; if `(int)f <= 0` (signed, 0x57D592)
  skip. Else `found = 1`, `size = CFileMgr::GetTotalSize(f)`, `isId = 1`, `len = 0` (u8), `pos = 0` (u8); while
  `pos < size` (pos zero-extended; pos wraps at 256, so a file of 256+ bytes never ends the loop): read 1 byte c;
  if c == '#': if len > 0 { buf[len] = 0; if isId { m_MissionPacks[k].m_Id = (u8)atol(buf) (0x82258E) } else
  { strcpy(m_MissionPacks[k].m_Name, buf); k++ } isId = !isId; len = 0 }; else if c is '\n' or '\r' ignore;
  else buf[len++] = c (256-byte local buffer, u8 index). pos++. Then CloseFile.
  After the loop: found -> SwitchToNewScreen(7); else SwitchToNewScreen(6) and `strncpy(aScreens[cur].m_aItems[0].m_szName
  (0x8CE013 + cur*0xE2), bMissionPackGame ? "FESZ_MR" : "FESZ_QR", 8)` (0x821F40).
- Verdict: CONFIRMED (hand parser; entries stored at a running index k, not at the file index i; several "id#name#"
  pairs per file allowed; k unbounded; signed FILE* test; u8 pos wrap). Class: keep-original.

## 0x57DB20 CMenuManager::CheckCodesForControls  (gr_deviations = 2)
Row MenuManager_Input.cpp:1220 (invalid keyboard keys "Fixed from v1.01"):
- Original bytes 0x57DC21-0x57DC63: invalid when key is 0, 0x3E9 (F1), 0x3EA (F2), 0x3EB (F3), 0x3F1 (F9), 0x41D (LWIN),
  0x408 (PAD5), 0x41E (RWIN), 0x41C (RALT); gr rejects only 0, F1, F2, F3, LWIN, RWIN.
- Verdict: CONFIRMED. Class: keep-original.
Row :1288 (`default: NOTSA_UNREACHABLE()` in the assignment switch): 0x57DD2E-0x57DD7B: type 0 -> keyboard path, 2 -> mouse
(+0x1AE0), 3 -> joystick (+0x1AE4); any other type assigns nothing but still resets the state and calls SaveSettings.
Verdict: CONFIRMED (invalid types only). Class: keep-original.
Found in passing (CONFIRMED, keep-original): after classification (0x57DB78) the original does: if
`m_MenuIsAbleToQuit (+0x1AE8)` (joystick combo action): escape -> reset, else return WITHOUT assigning; else if escape ->
reset (`+0x1B0B = 0, +0xF0 = 0, +0x1B09 = 0, +0x38 = -1, +0x1B08 = 0`) and return; else if the key was invalid -> return
WITHOUT the reset (gr resets on invalid keys too, and gr assigns for combo actions).

## 0x57DDE0 CMenuManager::SaveStatsToFile  (gr_deviations = 2)
Row MenuManager.cpp:834 (`CFileMgr::CloseFile(file); // FIX_BUGS` inside End()):
- Original bytes: the file is opened with CRT `fopen("stats.html", "w")` (0x8232D8, not CFileMgr) at 0x57DE30; on success
  the end is `fclose(f)` (0x57E1E6) then `CFileMgr::SetDir("")` (0x57E1F7); on failure (f == 0) it goes straight to
  SetDir("") with no close. Both paths then set `m_HelperText (+0x1B18) = 6` and `m_nHelperTextFadingAlpha (+0x1AEC) = 300`.
- Verdict: REFUTED (the original does close the file on success; the FIX_BUGS label is wrong). gr's End() additionally calls
  CloseFile(nullptr) on the failure path, which the original never does. Class: keep-original.
Row :850 (FIX_BUGS: `<meta charset="UTF-8"/>` and UTF-8 text):
- Original bytes 0x57DE42: the first line written is "<title>Grand Theft Auto San Andreas Stats</title>\n" (0x86636C), no meta
  line; GXT text is converted with 0x69F7E0 (GxtChar to ASCII) and upper-cased with CRT 0x836FBA. Writes use CRT fprintf
  0x823A30 (FID label `_fwprintf`, narrow format strings).
- Verdict: CONFIRMED. Class: keep-original.

## 0x57E240 CMenuManager::DisplayHelperText  (gr_deviations = 1)
Row MenuManager.cpp:983 (comment "ORIGINAL SETTING RESTORED" on `case FET_RSO`):
- Original bytes 0x57E35A-0x57E3DC: `switch (m_HelperText (+0x1B18) - 1)`, table 0x57E46C: 1 "FET_APP", 2 "FET_HRD",
  3 "FET_RSO", 4 "FEA_SCF", 5 "FEA_SCS", 6 "FET_STS" - same mapping as gr.
- Verdict: REFUTED. Class: harmless. (Entry 0x57E240 is a `nop; nop; jmp 0x40165E` detour stub returning to 0x57E24A.)

## 0x57FD70 CMenuManager::UserInput  (gr_deviations = 7)
Row MenuManager_Input.cpp:15 (comment "Vanilla bug: analog directionals ..."): comment only. Verdict: REFUTED. Class: harmless.
Rows :70, :177, :186, :202, :228, :229 (`notsa::contains` sets), checked against the bytes:
- :70 (0x57FF63, table 0x580630/0x580628): actions {27, 28, 29, 61, 62} skip `m_MouseInBounds (+0x1B00) = 2`. Matches SLIDER_ACTIONS.
- :177 (0x58031A, table 0x580674/0x580668): {27, 28, 61, 62} -> ReportFrontendAudioEvent(1); 29 -> event 5. Matches.
- :186 (0x58038A-0x5803A2): screens 0x22, 0x29, 0x10, 0x16, 0x12, 0x0E, 0x2B. Matches mainMenu.
- :202 (0x58049A / 0x5804EF, tables 0x5806A0 / 0x5806D4): {21, 27, 28, 29, 61, 62} set sliderMove. Matches SLIDER_ACTIONS + STAT.
- :228 (0x5805A8-0x5805BC): screens 3, 0x1A, 4, 0x1B, 0x24, 0x27. Matches specialScreens.
- :229 (0x5805D7, table 0x580708/0x580700): {2, 5, 7, 8, 9, 27, 28, 29, 32, 57, 61, 62} suppress the select sound. Matches.
- Verdict: REFUTED (x6). Class: harmless.
Found in passing (CONFIRMED, keep-original): in the slider repeat code (0x580463-0x58050A) the original stores the
current pause-mode time into the left/right repeat timer (0xBA82B0 left, 0xBA82B4 right) whenever more than 200 ms
(`cmp edx, 0xC8; jbe`, unsigned) have passed, even when the action is not a slider; gr updates the timer only for slider
actions.
