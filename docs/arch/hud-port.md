# PORT-HUD: subsystem HUD, first pass (crate `sa-hud`)

First pass over the catalogue subsystem HUD (342 rows: `CRadar` 66, `CMenuManager` 61, `CFont` 39, `CHud` 38,
`CMessages` 28, `CMenuSystem` 21, `CLocalisation` 18, `CText` 9, `COnscreenTimer` 9, `CCredits` 6,
`CHudColours` 5, `CKeyArray` 4, ...). This pass ports leaf and near-leaf functions, chosen from the 154 HUD
rows of `catalog/samp-priority.csv` (SA:MP uses text draws, game text, radar, blips, menus and on-screen
timers heavily), smallest first, plus the small classes around them. Follows ADR-001 and the PORT-POP
pattern. The main session sets `catalog/state.csv`; the per-function verdicts are in
`docs/arch/hud-port-verdicts.tsv`.

Ported: **148 functions**: 139 `#[hook]` registrations and 9 ported but not hookable (see below). Rendering
into RenderWare / D3D (`CFont::RenderFontBuffer`, `PrintString`, `CHud::Draw*`, `CRadar::Draw*`,
`CSprite2d`) is not ported; the ported functions call such code through `orig!`.

## Crate

`crates/sa-hud` (PLAN §2 names it). Same rules as sa-pop: `#![no_std]`, `#![deny(unsafe_code)]`, game state
through `globals!`/`GPtr`, every other catalogued function through `orig!`, every port registered with
`#[hook]` and listed in its module's `HOOKS` (`sa_hud::hooks::ALL`, which sa-hook appends after
`sa_script::hooks::ALL`). Builds for host, i686 (`--features in-process`) and wasm32. Depends on sa-core,
sa-math, sa-stream, sa-world.

| file | contents |
|---|---|
| `common.rs` | sa-world's field helpers re-exported; CRT `strlen`, `sprintf` (1 and 2 int arguments) |
| `font.rs` | `CFont`: the 19 state setters (17 hooked), `InitPerFrame`, `GetLetterIdPropValue`, `FindSubFontCharacter`, `GetCharacterSize`, `GetNumberLines`; `CFontChar::Set` (22 hooks) |
| `hud.rs` | `CHud` message and name state (8), `CHudColours` (3 + `GetIntColour` unhookable), `CLocalisation` (14 + 4 unhookable), `CCredits::Start/Stop`, `CUserDisplay::Init/Process` (29 hooks) |
| `menu.rs` | `CMenuSystem`: `SetColumnWidth`, `CheckForSelected`, `SwitchOffMenu`, `ActivateOneItem`, `ActivateItems` |
| `radar.rs` | `CRadar`: blip handles and fields, coordinate transforms, `LimitRadarPoint`, `CalculateBlipAlpha`, `DisplayThisBlip`, `SetCoordBlip`/`SetEntityBlip`/`SetShortRangeCoordBlip`, `Initialise`, `StreamRadarSections` (31) |
| `messages.rs` | `CMessages`: string helpers, the 10 `Add*` queue functions, `Init`, `Process`, `Display`, `Clear*`, previous briefs (25) |
| `text.rs` | `CText` constructor / destructor / `Unload` / `GetNameOfLoadedMissionText`, `CKeyArray::Unload/BinarySearch`, `CData::Unload` (7 + 2 unhookable) |
| `timer.rs` | `COnscreenTimer` (Init, Process, ProcessForDisplay, Add/Clear clock and counter, flags), `COnscreenTimerEntry::Process`, `COnscreenCounterEntry` (12) |
| `statics.rs` | 8 HUD static initialisers (`__init_CHud__Sprites`, `__init_CFont__Sprite`/`ButtonSprite`, `__init_CRadar__*`, `__init_HudColour`, `__init_FrontEndMenuManager`, `__init_g_ba672c`) |

Shared-crate changes: sa-hook `table.rs` (sa-hud's hooks appended after sa-script's), `callers.rs` test,
sa-oracle `hook_preserve` and `tests/l1.rs` chain sa-hud's registrations; sa-oracle `specs_hud.rs`,
`specs_hud_radar.rs`, `specs_hud_msg.rs` (18 L1 specs). `tools/e2e/port-l2.sh`: one-line fix (below).

## Conventions and findings worth knowing

- Ported from the 1.0 US bytes (objdump of the exe incl. the `.HOODLUM` bodies); gta-reversed and the
  RE-SPEC-3 specs (`docs/re/specs/HUD/*.md`) as reading aids. Every `expect` string comes from the exe.
- `CFontDetails` (0xC71A60) is one 0x40-byte global with field offsets from the setters' stores; the
  render state `CFontChar` (0xC71AA0) likewise. `SetColor`/`SetDropColor` scale the alpha below full fade as
  `_ftol(fild(a) * AlphaFade * (1/255))` in x87 order; `SetDropColor` uses the *text* colour's alpha
  (0xC71A63), as the bytes. `SetScaleForCurrentLanguage`: languages 1..4 share one jump-table target
  (x 0.8). Setters taking floats store the argument words unchanged (no float round trip).
- `CHud::SetBigMessage`: for styles other than 5 the original clears the caller's text (`text[0] = 0`);
  style 5 restarts the odd-job-2 timer when the text differs from `LastBigMessage[5]`. `SetMessage` writes its
  terminator at index 400 when the text fills the buffer (one past `m_Message`). `CHud::GetYPosBasedOnHealth`
  returns `y - maxHeight * (1/448) * offset` when the player's maximum health is **below** 101, else `y`.
  `CHud::m_BigMessage[7][128]` is reached by address: sa-script declares its byte 0xBAAD40.
- `CHudColours::GetRGB/GetRGBA` are thiscall with a hidden return pointer (`ret 8` / `ret 0xC`); `GetRGB`
  passes the table's alpha, `GetRGBA` the argument word. `GetIntColour` *adds* `(g<<16|0xFF) + (b<<8|0xFF) +
  (r<<24|0xFF)` (the low byte ends up 0xFD; carries reach the colour bytes).
- `CMenuSystem::SwitchOffMenu` frees the panel but leaves `MenuNumber[menu]`; its "next menu" walk reads
  `MenuInUse[2]`, which is `CurrentMenuInUse` (just cleared). `CheckForSelected` returns AL = -99 for an unused
  menu. The selected-row walk of `ActivateItems`/`ActivateOneItem` wraps once.
- `CRadar` (helper findings): `CalculateBlipAlpha` clamps with `max(.., 70)` (off the map the alpha is never
  below 70); `ChangeBlipBrightness(1)` clears the bright bit; `SetCoordBlip`/`SetEntityBlip` ignore their colour
  argument (8, resp. 7 for types 1/2 else 1); `SetShortRangeCoordBlip` writes `ms_RadarTrace[-1]+0x25`
  (0xBA86ED) when the handle check fails; `DisplayThisBlip` reads `CGame::currArea` through a protection stub
  (`[0x15608B8] + [0x52F59D]` = 0xB72914). `LimitRadarPoint` and `DisplayThisBlip` entries are `jmp`s into
  `.text` chunks (0x401EC5, 0x408040), registered as inline hooks with 7-byte expects.
  `TransformRadarPointToScreenSpace` preserves EDX (callers 0x58563E, 0x5857DC, 0x585873, 0x586408,
  0x586ED8 read it).
- `CMessages` (helper findings, the spec's keep-original items): `InsertStringInString` replaces `"~a~"` (not
  gr's `"~1~"`), copies `total` bytes without a NUL and zero-fills the target up to 400 bytes;
  `InsertNumberInString` reuses n6 after six markers (its u8 counter wraps at 256); `AddToPreviousBriefArray`
  reads one entry past the array when all 20 are used. The ports' local buffers are larger than the
  original's 0x190 / 0x320 where a u16 index can overrun them (the original corrupts its stack there).
- `COnscreenTimer::Init` sets counter flash = 1 and leaves text and colour (gr differs);
  `ProcessForDisplay` prints the variable at `m_nMaxVarValue` for DOUBLE counters (gr prints the same
  variable twice); its `/ 1000` comes from a protection-VM constant (spec: high confidence, not byte-proven).
  `SetClockBeepCountdownSecs` walks 4 entries of 0x40 bytes from `this` (clock, then misaligned offsets), as
  the bytes.
- `CFont::FindSubFontCharacter` preserves ECX and EDX (its callers keep values there), with an L1 spec
  comparing both.
- Return registers: bool / byte ports return `u8` (AL). Several originals set only AL (`setne al`,
  `mov al, imm`); callers test AL (L2 compares AL).

### Not hookable (ported, `#[hook]` removed, reached through `orig!`)

| address | function | reason |
|---|---|---|
| 0x56D230 / 0x56D240 / 0x56D260 / 0x56D270 | `CLocalisation::Blood` / `Porn` / `Prostitutes` / `KickingWhenDown` | 3 bytes (`mov al, 1; ret`): `TooShort(3)` for the 5-byte patch |
| 0x58FD50 | `CHudColours::GetIntColour` | protection reads the entry bytes as data (`sbb ecx, [0x58FD50]` at 0x403CCE) |
| 0x6A0000 | `CKeyArray::Search` | entry bytes read as data at 0x522974, 0x5F7B7E, 0x62FDB5, 0x15648C5, 0x156DD95; 58 unlisted code-key readers |
| 0x719600 / 0x719610 | `CFont::SetJustify` / `SetOrientation` | `CHud::DrawHelpText` pushes ECX at 0x58BAC5 after them, only to reserve a stack slot, but sa-hook's callers test cannot prove that through the x87 branches that follow; a hook would need `preserve = "ecx"`, a preserving hook needs an L1 spec comparing ECX, and L1 cannot run a function that writes globals. `SetOrientation`'s original also returns ECX = 0, which `preserve` cannot model. While hooked (first build) both were L2 PASS on grove / pr / cheats (657 and 1000 cases on grove, 0 mismatches) and part of the identical all-ports runs |
| 0x6A0050 | `CText::Get` | protection reads the dword at 0x6A004D (covers the entry) at 0xD723B1 / 0xD72411; hookable only if keycells `READERS` gets these two entries (sa-hook, not changed here). Port note: the original fills the error buffer with `sprintf(buf, "")`, bytes 1..24 of 0xC1AEB8 = uninitialised stack; the port writes zeros |

`Porn`/`Prostitutes` share the `localisation_porn` port; the other four have their own functions.

## Verification (commands, results)

All game runs on lane 6 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=6 L2_PER_FRAME=1000 SPLIT_PER_FRAME=1000`,
`E2E_BIN` = a snapshot of the release DLL, under `systemd-run --user --scope -p MemoryMax=8G`), outputs in the
worktree's `target/e2e/lane-6/port-l2-hud-*`. Lists: `tools/e2e/lists/hud-all.txt` (139),
`hud-l2-a.txt` (39: fonts, CHud, CHudColours, CUserDisplay, CMenuSystem), `hud-l2-b.txt` (16: CLocalisation,
CHudColours setter, CCredits), `hud-l2-radar.txt` (29), `hud-l2-msg.txt` (39), `hud-split-a.txt` (16:
static initialisers, `SwitchOffMenu`, `StreamRadarSections`, the `CText`/`CKeyArray`/`CData` unload,
constructor and destructor: allocation, deallocation, streaming requests, CRT `atexit`), `hud-l2-more.txt` (52:
the hooks with no call in the first three recordings).

- **L1** (`SA_WINEPREFIX=/nonexistent sa-oracle l1 <key> --cases 1000000 --no-fixtures --threads 4`; 18 specs
  in `specs_hud*.rs`, rerun by the main pass after the helpers): 0 mismatches everywhere.
  `HudFindSubFontCharacter`, `HudMsgGetStringLength`, `HudMsgStringCopy`, `HudMsgStringCopyNull`,
  `HudMsgStringCompare`, `HudMsgInsertStringInString`, `HudMsgBinarySearch`, `HudRadarGetActualBlipArrayIndex`,
  `HudRadarClipRadarTileCoords`, `HudRadarGetTextureCorners`: 1,002,000 cases, PASS.
  `HudRadarLimitRadarPoint(+Map)` 1,003,364, `HudRadarTransformRealWorldPointToRadarSpace` 1,060,000,
  `HudRadarTransformRadarPointToScreenSpace(+Map)` 1,060,000 (EDX preserve compared),
  `HudRadarCalculateBlipAlpha(+Map)` 1,000,058, `HudRadarIsPointInsideRadar` 1,003,364: 0 mismatches,
  INCOMPLETE only by their signalling-NaN cases (native backend off, as COLL/PHYS/POP). The radar specs patch
  the globals they read with fixed values (the map flag in two variants); the ports have `_with` variants
  taking them as parameters.
  Harness finding: Unicorn loses EFLAGS across `rep movsb` (`cmp ax, 0x190` 0x69E11F, `rep movsb`, `jae`
  0x69E125 in `InsertStringInString`); the spec swaps the two instructions (`.patch(0x69E11F, F3 A4 66 3D 90
  01)`, the same result on real hardware). Other specs with a flag-setting instruction before a `rep` and a
  conditional jump after it may show false mismatches.
- **L2 / split** (per recording, 1000 cases per hook at most, `L2_PER_FRAME=1000`): every hook with calls has
  **0 mismatches and 0 undeclared writes** in its final run on grove walk (5200 frames), pause-restart (3700) and
  world-cheats (3276); the hooks with no call there were also run on c-bug, anim-input and weapon-combat
  (`hud-l2-more.txt`, 6 more got calls). Split lists: every hook PASS (`CText::Unload` PASS-FP: one
  fingerprint-paired heap page), 0 FAIL / DIVERGED, 0 undeclared after the extras. Tags:
  `port-l2-hud-{a-g2,a-p,a-c}` (fonts/CHud/menus), `hud-{r-g2,r-p,r-c}` (radar), `hud-{m-g2,m-p,m-c}`
  (messages/text/timer), `hud-{b-g,b-p,b-c}` (localisation/colours/credits), `hud-{s-g2,s-p,s-c,s-wc}` (split),
  `hud-n-{cb,ai,wc}` (the rest). In every run (b) l2 == plain and (e) splito/splitr == plain on all frames.
- **L3, HUD hooks on Rust**: every `rust` run of the lists above is frame-identical to the all-original trace
  (17 subsystems) on its recording.
- **L3, all engine ports on Rust** (`ports`: 1420 hooks incl. the 139 HUD ones, final build,
  `port-l2-hud-fports-*`): IDENTICAL over grove walk 5200, pause-restart 3700, world-cheats 3276, c-bug 3220,
  anim-input 3300 and weapon-combat 3500 frames; no crash / panic / install failure. (The same six were also
  identical with the earlier build that still hooked `SetJustify`/`SetOrientation`, 1422 hooks,
  `port-l2-hud-ports-*`.) Per-hook hook-off runs were not made, so no row claims L3.
- Tests: `cargo test -p sa-hud` 3 pass (list complete/unique against every other crate, globals declared once
  across sa-core/stream/world/pop/script, initial values match the exe); `cargo test --release -p sa-hook --lib`
  45 pass (`every_hook_installs_on_the_exe`, `matches_catalogue_and_exe`, `table_is_consistent`,
  `callers_reading_ecx_edx_after_a_hook_are_declared`, keycells `readers_match_the_exe` /
  `every_code_key_read_of_a_hooked_range_is_redirected`, INF-20 `data_reads`); `cargo test --release -p
  sa-oracle --test l1`: 15 pass, 1 fails on master's sa-script registration (see Open items; the test stops
  there, the two HUD preserving hooks both have specs comparing their registers). clippy `-D warnings`: sa-hud
  host (all targets), i686 (`--features in-process`), wasm32; sa-oracle host (all targets); sa-hook i686.

Verdict counts (139 hooks + 9 ported-unhookable): **L1 14** (12 also L2 PASS), **L2 78**,
**implemented-needs-L2 47** (no call in any of the six recordings: the `CLocalisation` setters/getters other
than `KnockDownPeds`/`StealFromDeadPed`/`KillPeds`, `CHudColours::SetRGBAValue`, `CCredits`, every
`CMenuSystem` port, the blip setters SA:MP uses for scripted blips, the `CMessages::Add*` variants with
numbers/strings, `COnscreenTimer` add/clear and counter functions, `CText` destructor / unloads: they need a
recording with script menus, timers and blips, e.g. a mission start or a SA:MP session), **ported-unhookable 9**.
No mismatch anywhere.

### Harness note

`tools/e2e/port-l2.sh` read `$E2E_OUT/wine.log` for a crash line before `e2e_game_start` truncated it, so
an exit-time Wine fault of the previous run (kept apart as `NAME.wine-exitfault.txt`) was taken as a crash of
the next run, which was then killed (`wineserver -k`) before its trace was written (first grove run: "rust
reached the end frame: 0 frames", `rust.trace` missing). Fix: `run_game` deletes the old `wine.log` first.

### L2 extra regions

`tools/e2e/l2-extras.txt`, section `# PORT-HUD`, each line from the first run of its list (the undeclared
writes, original and Rust alike, are quoted in the comments): `m_Message` and the help / big message
buffers, the `CRGBA` behind `GetRGB`/`GetRGBA`'s hidden return pointer (`arg0`), `CUserDisplay::OnscnTimer`
and `PlaceName`, `ProcessCurrentString`'s static 0xB71670, the CRT `atexit` byte 0xC9D428 and the objects the
static initialisers construct (`HudColour` 0x3C bytes, `FrontEndMenuManager` 0x1B78 plus the constructor's
writes to `AERadioTrackManager`+4 and `g_fx`+0x54), `ms_RadarTrace` (175 x 0x28) for the blip writers,
`gRadarTextures`, and the pointer outputs of the radar transforms.

## Open items

- Not ported in this pass (about 195 HUD rows): all drawing (`CHud::Draw*`, `DrawRadar`, `CRadar::Draw*`,
  `DrawBlips`, `DrawEntityBlip`, `CFont::PrintString`, `ProcessCurrentString`, `RenderFontBuffer`,
  `PrintChar`, `GetStringWidth`, `GetTextRect`, `ParseToken`), `CMenuManager` (61, incl. `Process`,
  `CheckForMenuClosing`, `LoadSettings`, `SmallMessageScreen`, `DrawWindow`, constructor), `CMenuSystem`
  creation / input / display, the GXT loaders (`CText::Load`, `CKeyArray::Load`, `CData::Load`,
  `CMissionTextOffsets::Load`), `CFont::Initialise`/`LoadFontValues`, `CHud::Initialise`/`ReInitialise`,
  `CRadar::CalculateCachedSinCos`, `LineRadarBoxCollision`, `ClipRadarPoly`, `AddBlipToLegendList`,
  `GetRadarTraceColour`, `ShowRadarTraceWithHeight`, `SetupAirstripBlips`, `CMessages::ClearThisPrint*`,
  `CPlaceName`, `CCredits` rendering.
- `CText::Get` becomes hookable with two keycells `READERS` entries; `CFont::SetJustify`/`SetOrientation`
  become hookable (without `preserve`) if sa-hook's callers test learns that the `push ecx` at 0x58BAC5 is a
  slot reservation (decisions for the sa-hook owner).
- L1 for the globals-reading leaves (`GetCharacterSize`, `GetLetterIdPropValue`) needs patched font data;
  covered by L2 only.
- Hook-off (per-hook L3) runs.
- The pre-existing `sa-oracle --test l1` failure `preserved_registers_match_the_hook_registrations`
  (`CRunningScript::GetPointerToLocalVariable` 0x463CA0, sa-script, preserves ECX without a spec comparing
  it) is not from this pass.

## Functions

One row per port (state and evidence per recording) in `docs/arch/hud-port-verdicts.tsv`.

---

# PORT-HUD-2: second pass (drawing orchestration, menus, fonts, GXT loaders)

Second pass over the HUD rows still `specified` after PORT-HUD (194 HUD rows + 66 HUD rows of the UNK
proposal), SA:MP priority first (`catalog/samp-priority.csv`), then the rows the corpus executes. Same rules
as the first pass (ADR-001, ported from the 1.0 US bytes, gta-reversed and `docs/re/specs/HUD/*.md` as
reading aids). RenderWare / D3D / CSprite2d / file IO / allocation stay `orig!` calls. Lead plus two helpers
(CHud drawing + CRadar drawing; CFont printing + GXT loaders + CMenuManager), disjoint new files.

Ported: **224 functions**: 220 `#[hook]` registrations and 4 ported but not hookable.

| file | contents |
|---|---|
| `hud_draw.rs` | `CHud`: `Draw`, `DrawAfterFade`, `DrawRadar`, `DrawAmmo`, `DrawWeaponIcon`, `DrawWanted`, `Render{Armor,Breath,Health}Bar`, `DrawHelpText`, `DrawSubtitles`, `DrawMissionTimers`, `DrawScriptText`, `DrawSuccessFailedMessage`, `DrawBustedWastedMessage`, `DrawOddJobMessage`, `DrawMissionTitle`, `DrawAreaName`, `DrawVehicleName` (ADR-002 partial, below), `DrawFadeState`, `DrawTripSkip`, `SetHelpMessage*`, `Initialise`, `ReInitialise`, `Shutdown`; `CHudColours::Constructor` (28) |
| `font_print.rs` | `CFont`: `PrintString` (both), `ProcessCurrentString`, `ProcessStringToDisplay`, `PrintStringFromBottom`, `GetTextRect`, `ParseToken`, `PrintChar`, `RenderFontBuffer`, `DrawFonts`, `Initialise`, `LoadFontValues`, `Shutdown`, `GetNextSpace`, `GetScriptLetterSize`; `GxtCharStrcat/Strcpy/Strlen`, the token stripper 0x719240 (19 hooks + `GetStringWidth` unhookable) |
| `text_load.rs` | `CText::Load`, `LoadMissionText`, `LoadMissionPackText`, `ReadChunkHeader`, `CKeyArray::Load`, `CData::Load`, `CMissionTextOffsets::Load`, `GxtCharToAscii`, the two unload copies (10) |
| `radar_draw.rs` | `CRadar`: `CalculateCachedSinCos`, `GetRadarTraceColour`, `AddBlipToLegendList`, `InitFrontEndMap`, `Request/RemoveMapSection`, `RemoveRadarSections`, `Shutdown`, `LineRadarBoxCollision`, `ClipRadarPoly`, `LoadTextures`, `LimitToMap`, `TransformRadarPointToRealWorldSpace`, `ShowRadarTrace`, `DrawRotatingRadarSprite`, `SetupAirstripBlips`, `DrawYouAreHereSprite`, `SetMapCentreToPlayerCoords`, `Load`, `Save` (20) |
| `radar_draw2.rs` | `CRadar`: `DrawBlips`, `DrawCoordBlip`, `DrawRadarSprite`, `DrawMap`, `DrawRadarMap`, `DrawRadarSection`, `DrawRadarSectionMap`, `DrawRadarMask`, `DrawRadarGangOverlay`, `DrawAreaOnRadar`, `Draw3dMarkers`, `ShowRadarTraceWithHeight`, `ShowRadarMarker` (13) |
| `menu2.rs` | `CMenuSystem`: `CreateNewMenu`, `InsertMenu`, `InsertOneMenuItemWithNumber`, `Process`, `Input`, `InputStandardMenu`, `CheckForAccept`, `HighlightOneItem`, `SetColumnOrientation`, `SetActiveMenuItem`, `GetCarColourFromGrid` (11 + `Initialise` unhookable) |
| `menu_mgr.rs` | `CMenuManager` (front end): the 8 SA:MP rows (`SetDefaultPreferences`, `LoadSettings`, constructor, `CheckForMenuClosing`, `Process`, `DrawWindow`, `SmallMessageScreen`, `MessageScreen`) and 37 more (input checks, settings, textures, screens, mission packs, `ProcessMenuOptions`, `ProcessPCMenuOptions`, `ProcessUserInput`, `ProcessFileActions`, controls), the `CPad` mouse / Esc readers, `GetCurrentKeyPressed` (51 + `StretchX`/`StretchY` unhookable) |
| `place.rs` | `CPlaceName::Process`, `CPlaceName::GetForMap`, `CZone::GetTranslatedName` (3) |
| `statics2.rs` | 51 header-constant `__xc` initialisers (generated by `tools/gen/hud_statics.py` from the exe bytes, each body exactly `fld; fmul/fdiv [; fdivr]; fstp; ret`), `__init_TheText`, the two initialisers that only register an empty destructor, 8 `atexit` destructors (62) |
| `messages.rs` | `CMessages::ClearThisPrint`, `ClearThisBigPrint`, `ClearThisPrintBigNow` (3 more) |

Shared-crate changes: sa-oracle `specs_hud2.rs` (radar: `LineRadarBoxCollision`, `ClipRadarPoly`,
`LimitToMap` x3, `TransformRadarPointToRealWorldSpace`), `specs_hud2_text.rs` (5 GXT string helpers),
`specs_hud2_menu.rs` (8 `CMenuSystem` panel specs); `tools/e2e/l2-extras.txt` section `# PORT-HUD-2`;
`tools/e2e/lists/hud2-*.txt`; `tools/gen/hud_statics.py`.

## Findings (second pass)

- **`ClipRadarPoly`** (0x585040): with no known exit side (-1) the corner walk starts at `corners[-1]`, which in
  the original is two stack locals (`&inside[-1]` and the `s2` output pointer): a stack address becomes a
  vertex. The port writes its own equivalents (`clip_radar_poly_with` takes the pair; the L1 spec passes the
  emulator's stack address and reaches 0 mismatches). In-process the value differs from the original
  whenever that path runs (not seen in the corpus; divergence candidate). Inside test: NaN coordinates count
  as inside.
- **`LineRadarBoxCollision`**: no early return at t = 0 and no `t >= 0` test (gta-reversed has both); the first
  edge compares against the constant 1.0, the y = 1 edge uses the unrounded parameter; temporaries are stored
  in the caller's argument slots (only caller `ClipRadarPoly` does not reread them).
- **`CMessages::ClearThisPrint`** scans briefs until the first empty slot, **`ClearThisBigPrint`** scans all 7x4
  big messages skipping empty ones; both repeat until nothing matches; the string test reads one byte past two
  empty strings. `ClearThisPrintBigNow` clears `BigMessageX[style]` and `m_BigMessage[style][0]`.
- **`CRadar::AddBlipToLegendList`** scans all 175 legend entries, the u16 counter is unbounded; arrow sprites
  -4/-3 (threat/friend), -2 object, -1 coord, -5 other.
- **`CRadar::SetupAirstripBlips`**: a changed nearest strip only updates `airstrip_location` when a blip
  exists; ties go to the later strip.
- **`CPlaceName::Process`**: with no zone the original passes an uninitialised stack dword to
  `CHud::SetZoneName` (stored outside interiors when no zone name is shown); the port passes 0 (divergence
  candidate; not seen in the corpus).
- **`CMenuSystem::Process`** indexes `MenuInUse` with the sign-extended and `MenuNumber` with the
  zero-extended menu byte; for -99 the loop index replaces only the low byte of the argument dword.
  `CreateNewMenu` passes `FillGridWithCarColours`/`ActivateItems` a dword whose upper bytes are uninitialised
  stack (callees read the byte). `InputStandardMenu` reads the alternative inputs from absolute pad words.
- Helper findings (CHud, CRadar drawing, CFont, GXT loaders, CMenuManager) are in the doc comments of each
  port; the ones that differ from gta-reversed: `ParseToken` searches the closing '~' without a NUL test;
  `CKeyArray::Load`/`CData::Load`/`ReadChunkHeader` read byte by byte and fail on a short read;
  `LoadMissionPackText` never closes its file; `DrawRadar`'s ring angle is `-atan2(-right.z, at.z) - pi/4`,
  and the altimeter shows for planes and helis that are not the Vortex; `SetHelpMessageStatUpdate` clears the
  buffers before its same-text test; `DrawMissionTimers` reads player 2's max health 0xB7D077 without a clock;
  `LoadSettings` reads brightness / draw mode into +0xCC / +0xD8 while `SaveSettings` writes +0xC8 / +0xD4;
  `CheckForMenuClosing` sets `DisablePlayerControls` on open; `DrawRadarSection` takes u/v from the clipped points.
- Uninitialised-stack values the ports replace by 0 (each in a doc comment): `DrawAreaOnRadar`'s u/v buffer
  (written to `CSprite2d::maVertices`, untextured draw), `DrawCoordBlip`/`DrawRadarGangOverlay` alpha upper
  bytes, `RenderFontBuffer` glyph dword upper bytes, `LoadFontValues`' `%d` slot, `CText::Load` file name for
  language > 4, `LoadSettings` check bytes, `DrawVehicleName`'s alpha for a state outside 0..4 (that path runs
  the original body: ADR-002 `fallback`).
- **`CFont::ProcessCurrentString`** returns the line count in AX; bits 16..31 of EAX are whatever the original's
  last computation left (e.g. 0xFFF1 in the split run). The port returns them as 0: split-report FAIL on EAX
  only (all memory outputs identical, splitr trace identical). Callers `GetNumberLines`/`ProcessStringToDisplay`
  pass EAX on, `GetTextRect` and the `PrintString`s use the low byte / discard it. Needs a decision (AX return
  kind in the harness, or a ledger entry).

### Not hookable (ported, reached through `orig!`)

| address | function | reason |
|---|---|---|
| 0x5822D0 | `CMenuSystem::Initialise` | loop head at +3 (`jb` 0x5822EA -> 0x5822D3) inside the 5-byte patch |
| 0x71A0E0 | `CFont::GetStringWidth` | protection reads the entry dword 0x71A0E2 at 0x40641C (needs a keycells `READERS` / patch_refs entry) |
| 0x5733E0 / 0x573410 | `CMenuManager::StretchX` / `StretchY` | callers keep EDX across the call (0x57804B, 0x5781B3, 0x57B906): `preserve = "edx"` + an L1 spec comparing EDX needed |

Not ported, too small to hook: 0x856B80 / 0x856B90 (`ret`), 0x43CE70 (`CRGBA` destructor, `ret`), 0x43E0E0
(`mov eax, ecx; ret`). Not ported (left for a third pass): `CHud::DrawPlayerInfo`, `DrawCrossHairs`,
`DrawVitalStats`, `CRadar::DrawEntityBlip`, `DrawLegend`, `CMenuManager::UserInput` and the drawing screens
(`DrawStandardMenus`, `DrawBackground`, `Print*`, `DisplaySlider`, `DrawWindowedText`, controller screens,
`RedefineScreenUserInput`, `SaveStatsToFile`, `AdditionalOptionInput`), `CMenuSystem::DisplayStandardMenu`,
`DisplayGridMenu`, `InputGridMenu`, `FillGridWithCarColours`, `CCredits` rendering, `DIReleaseMouse`, the two
unknown thunks 0x4410E0 / 0x745A70; 0x584C00 is a mis-split inside `RemoveRadarSections`.

Verification limit: for drawing functions L2/L3 compare memory only; the coordinates and colours passed to
CSprite2d / RenderWare are checked only where a callee writes them to memory (the sprite and Im2D vertex
buffers 0xC4B960 / 0xC5F950, which the split runs declare and compare). Pure draw-argument errors in calls
that render immediately need the F7 D3D call-stream diff.

## Verification (second pass)

Lane 6 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=6`, binary snapshots `E2E_BIN`/`CARGO_TARGET_DIR` per run,
`L2_PER_FRAME=1000`, `SPLIT_PER_FRAME` 20 (1 or 2 for the font / CHud / radar drawing lists, whose windows
include the 9 KB vertex buffer)). Lists: `tools/e2e/lists/hud2-l2-{a,b,c}.txt` (plain L2) and
`hud2-split-{a,b,c,s}.txt` (split). Outputs: the worktree's `target/e2e/lane-6/port-l2-hud2-*`.

- **L1** (`sa-oracle l1 KEY --cases 1000000 --no-fixtures --threads 4`): 0 mismatches on all 21 specs.
  `HudRadarLineRadarBoxCollision` 1,060,000, `HudRadarClipRadarPoly` 1,060,000 (signalling NaNs quieted in the
  inputs: they route to the native backend, whose stack differs from the emulator's that the `corners[-1]`
  quirk writes), `HudRadarLimitToMap{,Loaded,LoadedBig}` 3 x 1,003,364, `HudRadarTransformRadarPointToRealWorldSpace`
  1,060,000 (INCOMPLETE only by signalling-NaN inputs, as the first pass); `HudGxtCharStrcat/Strcpy/Strlen`,
  `HudFontGetNextSpace`, `HudGxtStripTokens` and the 8 `HudMenu*` specs 1,002,000 each, PASS.
- **L2 plain**: menus-hud 5700 frames (`hud2-a2-mh`, `hud2-c-mh`) and the grove walk 5200 (`hud2-a2-g`): every
  hook with calls 0 mismatches; undeclared writes only before the extras were added (help buffers,
  `CPlaceName` `this`, then 0 on the rerun). `SwitchToNewScreen` (audio fade thread state) and
  `Draw3dMarkers` (marker atomics allocated by `C3dMarkers::PlaceMarker`, collision model cache) differ when
  run twice in L2 mode (1 and 2 cases): moved to split (`hud2-s5-mh`). (b) l2 == plain and (c) rust == plain
  on all frames in every run.
- **Split** (`hud2-s-mh`, `hud2-s2a-mh`, `hud2-s2b-mh`, `hud2-s3-mh`, `hud2-s4-mh`, `hud2-s5-mh`): 0 mismatches on
  every compared case except `CFont::ProcessCurrentString` (EAX bits 16..31 only, above). DIVERGED rows are
  callees whose Rust caller passes a pointer into its own stack (`CText::Load` -> chunk readers,
  `CHud::Draw*` -> `PrintString`, `ProcessUserInput` -> `ProcessMenuOptions`, `RenderFontBuffer` -> `PrintChar`
  glyph dword); the callees were rerun with their callers original (`hud2-s3-mh`: all PASS). COUNT rows: the
  2 GiB spool quota stopped splito and splitr at different frames (0 mismatches on the compared cases). The
  undeclared writes the split runs report are callee writes (RenderWare / D3D state, sprite / Im2D / font
  buffers, texture dictionaries, CFileMgr, atexit), declared afterwards in `l2-extras.txt` (`hud2-s2b-mh`, run
  after its extras: 0 undeclared). splito == plain and splitr == plain on all frames in every split run.
- **L3, all ports on Rust** (`tools/e2e/verify-all.sh hud2va1 6`, final build incl. all 220 HUD-2 hooks): IDENTICAL on all 11 recordings (grove walk 5200, air-water 5800, anim-input 3300, car-ai 5000, c-bug 3220, driving 4338, menus-hud 5700, pause-restart 3700, police-gang 5200, weapon-combat 3500, world-cheats 3276 frames).
- Tests: `cargo test --release -p sa-hook --lib` 51 pass; `cargo test -p sa-hud` 3 pass; clippy `-D warnings`
  sa-hud host (all targets) / i686 (`--features in-process`) / wasm32, sa-oracle host (all targets): clean.
  `cargo test --release -p sa-oracle --test l1`: see the report (its first failure after this pass's specs is
  `RwMatrixRotate` 0x7F1FD0 on the native backend, an sa-rwcore spec).

Verdicts per function: `docs/arch/hud-port-2-verdicts.tsv`.

---

# PORT-HUD-3: third pass (the remaining HUD rows) and the harness decisions

Third pass over the HUD rows still `specified` after PORT-HUD-2 (29 rows) plus the two unknown HUD thunks.
Lead plus two helpers (A: `CHud` / `CRadar` drawing and `PrintMap`; B: `CMenuManager` input and screens),
disjoint new files. Same rules as the earlier passes (ADR-001, from the 1.0 US bytes, gta-reversed and
`docs/re/specs/HUD/*.md` as reading aids, RenderWare / CSprite2d / CFont / audio / file IO stay `orig!`).
Branch base: PORT-HUD-2 (`worktree-agent-a67124806e65588be`) merged into master `c1a27ca`.

Ported: **31 functions, all hooked** (one partial: `AdditionalOptionInput`, ADR-002 `fallback`), plus 3
earlier ports made hookable (`CFont::GetStringWidth`, `CMenuManager::StretchX` / `StretchY`).

| file | contents |
|---|---|
| `hud_draw3.rs` (A) | `CHud::DrawPlayerInfo` 0x58EAF0, `DrawCrossHairs` 0x58E020, `DrawVitalStats` 0x589650 |
| `radar_draw3.rs` (A) | `CRadar::DrawEntityBlip` 0x587000, `DrawLegend` 0x5828A0 |
| `menu_map.rs` (A) | `CMenuManager::PrintMap` 0x575130 |
| `menu_input.rs` (B) | `CMenuManager::UserInput` 0x57FD70, `AdditionalOptionInput` 0x5773D0 (partial: the map screen's pan / zoom / waypoint path runs the original body) |
| `menu_draw.rs` (B) | `CMenuManager::DrawStandardMenus` 0x5794A0, `DrawBackground` 0x57B750, `PrintStats` 0x574900, `PrintBriefs` 0x576320, `PrintRadioStationList` 0x5746F0, `DisplaySlider` 0x576860, `DrawWindowedText` 0x578F50 |
| `menu3.rs` | `CMenuSystem::DisplayStandardMenu` 0x580E00, `DisplayGridMenu` 0x5816E0, `InputGridMenu` 0x580BD0, `FillGridWithCarColours` 0x5820E0 |
| `credits.rs`, `credits_table.rs` | `CCredits::Render` 0x53D5B0, `RenderCredits` 0x5A87F0, `PrintCreditText` 0x5A8660, `PrintCreditSpace` 0x5A87C0; the 707-step line / space table generated from the exe by `tools/gen/hud_credits.py` |
| `menu_ctrl.rs` | `DIReleaseMouse` 0x746F70, 0x4410E0 (`CPad::IsDPadRightPressed`, a `.HOODLUM` stub), 0x745A70 (`jmp RwEngineGetNumVideoModes`) |

Not ported (left in `specified`): `CMenuManager::DrawControllerScreenExtraText` 0x57D8D0,
`DrawControllerBound` 0x57E6E0, `DrawControllerSetupScreen` 0x57F300, `RedefineScreenUserInput` 0x57EF50,
`SaveStatsToFile` 0x57DDE0 (the controller-setup screens and the stats file; off the boot -> main menu ->
start game path; `menu_ctrl.rs` is their intended home).

Catalogue naming for the main session: 0x4410E0 = `CPad::IsDPadRightPressed` (same shape as 0x53EE60
`IsDPadUpPressed`, offsets +0x16 / +0x46), 0x745A70 = a thunk to `RwEngineGetNumVideoModes`. Their hooks
use the Ghidra names (`thunk_FUN_015667a0`, `thunk_FUN_007f2cc0`) as the catalogue test requires.

## Harness decisions (implemented)

- **(a) AX return kind.** `sa_core::abi::AbiRet::Ax` and the newtype `sa_core::abi::Ax(u16)` (EAX =
  zero-extended AX); `sa-oracle-format` `Ret::Ax = 5` compares the low word only (L2 and split; unit
  test in `l2.rs`); `sa-hook` maps it; L1 already had `ExpRet::Ax`. No `sa-macros` change was needed (the
  return kind flows through `IntoAbi::RET`). `CFont::ProcessCurrentString` returns `Ax`. EAX[31:16] is
  **not derivable** from the inputs: the original ends with `mov ax, [esp+0xC]` and the upper half is the
  EAX residue of the last full write on the path (`RenderFontBuffer` / ranged `PrintString` /
  `GetStringWidth` / `ParseToken` / `sprintf` returns, a `lea eax` stack address, float bits), i.e. of
  callees that are themselves Rust ports. The port returns 0 there. **Ledger entry (proposed):**
  `ProcessCurrentString` EAX[31:16] = 0 instead of callee residue; every consumer reads AX only
  (`GetNumberLines` -> 0x5741FF / 0x57B24B / 0x58B7EA `movsx r32, ax`, 0x58C795 `cmp ax, 1`;
  `GetTextRect` 0x71A6B4 `movzx eax, al`; the `PrintString`s discard it), so no behaviour depends on it.
- **(b) `GetStringWidth` hookable.** `sa-hook` keycells `READERS` gets the protection read `mov ecx,
  [0x71A0E2]` at 0x40641C (value 0xFFCE75D3); `patch_refs` / `data_reads` exclude listed readers already,
  and the `or eax, 0x71A0E2` decode at 0x40641D is off-stream. `readers_match_the_exe`,
  `every_code_key_read_of_a_hooked_range_is_redirected`, INF-20 `data_reads` and `patch_refs` stay green.
- **(c) `StretchX` / `StretchY`** hooked with `preserve = "edx"` (entry 0x5733E0 = 5 `nop` + `jmp`; the
  inline hook takes the `nop`s). L1 specs `HudMenuStretchX{640,1024,1366}`, `HudMenuStretchY{448,768,1080}`
  (`specs_hud3.rs`, `RsGlobal.maximumWidth/Height` patched per spec, EDX compared): 1,000,058 cases each,
  0 mismatches (INCOMPLETE only by the signalling-NaN inputs, as the other HUD specs); the
  `preserved_registers_match_the_hook_registrations` test passes.
- **(d) Uninitialised stack.** The original's residue is *read* here (not left behind), which
  `footprint` (restore + record) does not model, and two of the functions nest (`DrawRadarGangOverlay`
  -> `DrawAreaOnRadar` -> `ClipRadarPoly`), which `footprint`'s single state forbids. New hook attribute
  **`#[hook(..., peek = N)]`** (`sa-macros`, `sa_core::abi::{PEEK, peek_entry_esp, peek_stack}`): a naked
  wrapper copies the N bytes below the entry ESP and the entry ESP into a per-depth frame (8 levels,
  N <= 0x200) before calling the shim; the port reads them only while its own frame is on top (`None` for
  a direct Rust call, phases B/C, host tests). Applied:
  - `ClipRadarPoly` (`peek = 4`): `corners[-1]` = `&inside[-1]` = `((entry ESP - 4) & !7) - 0x45` (the
    original's `and esp, -8` frame); faithful in-process.
  - `CPlaceName::Process` (`peek = 0x10`): the no-zone name = the dword at `entry ESP - 0x10`, which the
    original never writes; faithful.
  - `DrawAreaOnRadar` (`peek = 0xC0`): the `uv` buffer = 16 dwords from `entry ESP - 0xC0` (never written
    before the map scaling); faithful.
  - Ledger / not observable (argument-slot residue only; every callee reads the low byte): the alpha
    upper bytes of `DrawCoordBlip` / `DrawRadarGangOverlay` (`CRGBA` constructor and
    `ShowRadarTraceWithHeight` read bytes), the fade byte of `DisplayStandardMenu` / `DisplayGridMenu` to
    `DrawWindow` (`colour as u8`), the orientation byte to `SetOrientation`, B's `ProcessUserInput` /
    `ConstructStatLine` / `GetStatValue` arguments.
  - **Divergence candidate (ledger)**: `CRadar::DrawEntityBlip`, airstrip blip at distance >= 500: the
    original uses an uninitialised position local (residue of earlier callees deep in its own frame,
    overwritten by its callees on that path, so not recoverable by `peek`); the port uses (0, 0, 0). Not
    reached in the corpus.
  - Verification note: the split runs pass `SPLIT_STACKFILL=4096` (zeroed stack below the invoker), so
    both runs read the same residue; the plain all-ports L3 runs see the real residue.

## Merges

PORT-HUD-2's `orig!` declarations returning `X87` (16 bytes since the X87 change) broke the i686 build; this
branch first fixed them locally (`*_st0 -> f64` plus `X87` wrappers), then merged master `3b33f75`, which
carries the main session's equivalent fix: the sa-hud files keep this branch's version (same pattern, plus
the PORT-HUD-3 changes), `sa-veh` / `sa-render` take master's. `sa-macros` keeps both `peek` and master's
writer / verifier hints. Master was then rewritten; the final merge is master `32279fd` (sa-hud: this branch; `abi.rs`, `keycells.rs`
(now 27 readers), sa-oracle `lib.rs` / `specs.rs`: master plus this branch's additions; master's new
`footprint_residue_u8` reads a footprint snapshot but does not nest, which is why `peek` stays).

## Findings (third pass)

Each port's doc comment lists its byte findings and gta-reversed differences; the main ones:
- `RenderCredits` is 599 `PrintCreditText` calls and 108 inlined spaces; the line offset advances by
  `_ftol(float(offset) + scaleY * 20)` (gta-reversed: `(uint32)scaleY * 20`), spaces by
  `_ftol(float(offset) + C)` with C = 37.5 / 25 / 12.5; the credits stop once `float(offset + H) - scroll <
  -10`. `PrintCreditText` keeps `pos` and its CRGBAs in its own argument slots.
- `FillGridWithCarColours` (gta-reversed: unreversed) compares each candidate with the taken colours *and*
  the next, still black entry (within 15 per channel, exclusive), restarts the search at colour 0 for
  every slot and takes colour 127 when nothing fits. `DisplayStandardMenu` accumulates column offsets
  through `_ftol` with 16-bit truncation and shrinks the row scale from 0.52 by 0.05 until
  `GetStringWidth` fits.
- `DrawLegend`: sprites without a jump-table key pass the `y` argument as the text pointer (kept);
  `DrawEntityBlip` calls `FindPlayerVehicle` 4 times without a null test (kept); `DrawCrossHairs` reads
  address 0 for a TXD slot with a negative pool flag (kept, `black_box`); `PrintMap`: radar visibility in
  the absolute globals 0xBA8270 / 0xBA8274, legend window `{95, y, 555, 100}`.
- `UserInput`: screen 0x26 still runs the held left/right parts, slider repeat timers are globals
  0xBA82B0 / 0xBA82B4; `DrawStandardMenus` reads the `GetNumberLines` x through the protection chunk
  0x409B33 (`[0x15608E8] - [0x156B63F] - 1`, read at run time); `DrawBackground` shows errors through
  `RsCameraBeginUpdate` + `SmallMessageScreen`, not `MessageScreen`.

## Verification (third pass)

Lane 7 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=7`, binary snapshots `target/snap/h4` (pre-master-merge,
identical HUD code) and `h5` (final), `L2_PER_FRAME=20`, `SPLIT_PER_FRAME` 1 or 2, `SPLIT_STACKFILL=4096`,
`L2_SPOOL_MAX=1.5 GiB`). Lists: `tools/e2e/lists/hud3-h-l2.txt` (StretchX/Y, GetStringWidth, CPlaceName,
plain L2), `hud3-a.txt` (credits, CMenuSystem, thunks, plain L2), `hud3-x-split.txt` (DIReleaseMouse,
ProcessCurrentString and the `peek` hooks), `hud3-m1-split.txt` / `hud3-m2-split.txt` (CMenuManager),
`hud3-d-split.txt` (CHud / CRadar drawing, PrintMap). Outputs: the worktree's
`target/e2e/lane-7/port-l2-hud3-*` (spools deleted after the reports).

- **L1**: the six `HudMenuStretch*` specs, 1,000,058 cases each, 0 mismatches (EDX compared).
- **L2 / split**: **0 mismatches in every run** on menus-hud (5700 frames), the grove walk (5200) and
  pause-restart (3700). Undeclared writes (callee writes: RenderWare / D3D9 caches, font / sprite / Im2D
  buffers, audio front-end, aScreens, text buffers, the pools on a restart) were declared in
  `tools/e2e/l2-extras.txt` (section `# PORT-HUD-3`) and the reruns show 0 undeclared
  (`hud3-*-mh2`, `-gr`, `hud3-m1-pa2`, `hud3-d-gr2`, `-pa`). COUNT rows: the spool quota stopped splito and
  splitr at different frames (0 mismatches on the compared cases), as in PORT-HUD-2. `DIReleaseMouse`
  cannot run twice (COM `Release`): plain L2 crashed the first run, so it is verified in split only.
  (b) l2 == plain, (c) rust == plain and (e) splito / splitr == plain on all frames in every run.
- No call in the three recordings: the four `CCredits` functions, the four `CMenuSystem` ports (script
  menus), 0x745A70, `DrawVitalStats`, `DrawWindowedText`, `PrintRadioStationList`, `DrawAreaOnRadar`
  (its `peek` change) -> `implemented-needs-L2`.
- Tests: `cargo test --release -p sa-hook --lib` 53 pass (after the master merge); `cargo test -p sa-hud` 3;
  `sa-oracle-format` 27 (incl. the AX compare); `sa-oracle --test l1 preserved_registers_match_the_hook_registrations`
  pass; clippy `-D warnings` sa-hud / sa-core / sa-macros / sa-oracle-format / sa-oracle host (all targets),
  sa-hud i686 (`--features in-process`), sa-hook i686: clean.

Verdicts per function: `docs/arch/hud-port-3-verdicts.tsv` (**20 L2, 13 implemented-needs-L2**, 0 failing).

### All-ports finding: a Rust port reading GetStringWidth's entry bytes

With every port on Rust the first all-ports runs crashed at start (EIP 0x4FFFFFFF / 0xA4309A) while the
sa-hud hooks alone (`hud-all-3.txt`, 391 hooks on Rust, grove: IDENTICAL) and pure master were fine.
Cause: sa-stream's `CVehicleModelInfo::UseCommonVehicleTexDicationary` port reproduces the protection
chunk 0x406413 (`rol [0x1560810], cl = [0x71A0E2]`) by reading 0x71A0E2 live, i.e. code bytes of
`CFont::GetStringWidth`'s entry, which this pass hooks; it computed a wrong `FindTextureCB` pointer. The
port now uses the exe's dword (0xFFCE75D3) as a constant (`crates/sa-stream/src/modelinfo/vehicle.rs`).
A scan of every crate's address literals found no other read inside a newly hooked entry. Rule for
future hookability changes: keycells only redirect the original's readers; Rust ports that read code
bytes need the same check (a test like `every_code_key_read_of_a_hooked_range_is_redirected` over the
`globals!` declarations would catch it).

**L3, all ports on Rust** (`VERIFY_NO_BUILD=1 SA_ROOT=/home/ghosty2004/sa-engine E2E_BIN=<snapshot of this
branch's final build> tools/e2e/verify-all.sh hud3va3 7 <grove> <corpus/*.sarec>`, branch commit `7a3b4cf`
incl. master `32279fd`): **IDENTICAL on all 11 recordings** (grove walk 5200, air-water 5800, anim-input
3300, car-ai 5000, c-bug 3220, driving 4338, menus-hud 5700, pause-restart 3700, police-gang 5200,
weapon-combat 3500, world-cheats 3276 frames). Summary: the main repo's `target/verify-all/hud3va3/`.
Note: `cargo test --release -p sa-stream` segfaults in `misc::tests::shadow_vertices` (not touched here).
