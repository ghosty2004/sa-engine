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
