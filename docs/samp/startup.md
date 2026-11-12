# SA:MP start-up: samp.dll's patches and the engine's hook points

What a player sees with `gta_sa.exe` 1.0 US + `samp.dll` 0.3.7-R4 from launch to spawn, and how native
`sa-game --samp` (shared with the web build) reproduces each step. The single-player start-up (logos, intro
movie, legal screen, main menu, loading) is unchanged when SA:MP is off: every engine change below is gated on
samp mode (`sa_core::samp::toggle::SAMP_STARTUP`, set by `sa-game/src/samp.rs::install` before WinMain's
boot steps) and sits at the function / address the original patch touches.

Reference recording: `tools/e2e/samp-start-ref.sh` (original exe + samp.dll under Wine, lane-locked, offscreen
gamescope, private open.mp with `tools/e2e/samp/sampcov.pwn`; one screenshot about every 200 ms). Sources for
the patch list: samp-rs `docs/re/subsystems/gta-patches.md` ("Startup / intro"), samp-rs `patches.rs`,
`client/session.rs`, `client/spawn.rs`, `lifecycle.rs`, `classsel.rs`.

## Observed sequence (reference run, 1280x720)

| t (s) | what the original shows |
|---|---|
| 0 – 2 | black (window creation, `gGameState` 5 `InitialiseEssentialsAfterRW`) |
| ~2.4 | loading screen: the SA-MP logo (`title` texture from samp.saa's `loadscs.txd`) with the EAX / NVIDIA / RakNet logos |
| ~3.3 | in game, connecting scene: camera above Verona Beach looking at downtown, HUD off, weather 1; chat `SA-MP 0.3.7-R4 Started` |
| ~6.3 | chat `Connecting to 127.0.0.1:PORT...` (3 s connect tick) |
| ~7.6 | chat `Connected. Joining the game...`, `Connected to <hostname>`; class selection: the gamemode's camera (`SetPlayerCameraPos/LookAt`) on the class ped |
| class selection | no `<< >> Spawn` panel visible with keyboard input; Left/Right change class (2 s guard, 250 ms repeat, sounds 1053/1052), Shift spawns |
| spawn | camera behind the player, HUD and radar back |

## Patch → engine hook

| samp.dll patch (address, bytes) | original function | engine hook / option | state |
|---|---|---|---|
| `gGameState` 0xC8D4C0 := 5 at attach; 0x747483 `89 35 C0 D4 C8 00` (`mov [gGameState],esi`) → 6×NOP | `Ps::psInitialize` 0x747420 | `sa_core::psinit::ps_initialize` writes 5 instead of 0 when `SAMP_STARTUP` is set | done |
| end of samp's init-main (runs at `gGameState` 7): `FrontEndMenuManager` +0xE9 (0xBA6831) := 1, +0x5C (0xBA67A4) := 0, +0x33 (0xBA677B) := 0, `gGameState` := 8 | WinMain loop, state 7 (frontend idle) | `sa-game/src/winmain.rs` state 7 applies the same writes when `SAMP_STARTUP` is set | done |
| `CreateFileA` hook: files whose lower-case basename is a samp.saa entry are served from memory | kernel32 (the game's every file open) | `sa_platform::vfs::set_redirect`, consulted by `Vfs::read` / `open_read` (where the engine's file opens reach the platform); `sa-game/src/samp_saa.rs` reads samp.saa from the game folder (or `SA_SAMP_DIR` natively) with samp-rs's `samp-saa` and serves `loadscs.txd` | done for `loadscs.txd`; samp's `main.scm` / `script.img` are served only with `SA_SAMP_SCM=1` (blockers in "samp's main.scm" below), the IDE/DAT entries need samp's `SAMP` folder |
| 0x866CD8 "title_pc_US" → "title", 0x866CCC "loadsc%d" → "title" (`74 69 74 6C 65 00`) | `CLoadingScreen::LoadSplashes` 0x5900B0 string data | `samp_saa::install` writes the same bytes into the mapped .rdata (`arena::patch_image`, over the original bytes only) | done |
| 0x590099 `E8 C2 7A 19 00` → 5×NOP (`CSprite2d::DrawRect`) | `CLoadingScreen::RenderSplash` 0x58FF60 | `sa_stream::loading_screen::render_splash` skips that call when `SAMP_STARTUP` is set | done |
| not-connected scene every frame until InitGame (samp.dll 0x100089B0): weather 1, HUD off, ped at (1133.05, -2038.40, 69.1), fixed camera (1093, -2036, 90) pointing at (384, -1557, 20) jump cut | script opcodes 015F / 0160 / 0373 | `samp_net.rs::connecting_scene` through the adapter (`set_fixed_position`, `point_at`, `set_weather`, `set_hud_visible`, `set_ped_position`) | done |
| chat lines `Started` (netgame created), `Connecting to H:P...` (connect tick > 3000 ms, samp.dll 0x10008930), `Connected. Joining the game...` (0x22), `Connected to {B9C9BF}name` (InitGame), debug colour 0xA9C4E4 | samp chat | `samp_net.rs::debug_line` + samp-rs `lifecycle::NetGame` | done |
| class selection: RPC 128 on entry, Left/Right/Shift polled from the keyboard state, sounds 1052/1053, HUD off while no menu | samp's class selection (samp-rs `classsel.rs`, `spawn.rs`) | `samp_net.rs::class_selection_tick` with samp-rs `classsel::ClassSelection`; keys from `NewKeyState` 0xB73190 (+0x22A left, +0x22C right, +0x260 shift) | done (keyboard); mouse panel open |
| spawn: restore camera, behind player, HUD on, control on, teleport z+0.5, heading, RPC 52 | samp's local spawn routine | `samp_net.rs::spawn_local` (existing) | done |

Headless runs (offscreen sa-game, no keyboard) set `SA_SAMP_AUTO_SPAWN=1`: Shift is then held synthetically;
samp's 2 s guard and the class-info wait still apply (`tools/e2e/samp/native-join.sh` sets it).

## Comparison (2026-11-12, original 1280x720 under Wine, sa-game 800x600 offscreen)

| phase | status |
|---|---|
| logos / intro / legal / menu skipped | same: `gGameState` FRONTEND_LOADING → FRONTEND_LOADED → FRONTEND_IDLE → LOADING_STARTED → IDLE in 4 iterations |
| loading screen | same art (SA-MP logo, EAX / NVIDIA / RakNet); sa-game also shows GTA's loading bar on the last slides (not seen in the original's 200 ms samples: to check) |
| connecting scene | same camera, weather and chat lines and the same 3 s connect tick; sa-game shows a "Fat" stat box at the top left (a stat message from GTA's main.scm; samp runs its own main.scm, see below). Clock 12:00 every frame before InitGame like samp (`connecting_scene`, samp's server-settings default) |
| class selection | same camera (the gamemode's), HUD off. Real key events through the engine's input path (`SA_GAME_KEYS='right:300:306,right:360:366,left:420:426,shift:500:620'` with native-join.sh and `SA_SAMP_AUTO_SPAWN=0`): the server sees classes 0 → 1 → 2 → 1 and the spawn, as with the original's Right, Right, Left, Shift. The class skin changes with the class (`Peds::set_ped_model`: model loaded, then 09C7; skins 105 / 285 seen in the PNGs; skin 0 keeps CJ). The arrow sounds 1052 / 1053 play: `one_off_sound` sends 018C's frontend-only cases (0x41C → frontend event 0x28, 0x41D → 0x0E, read from `ProcessMissionAudioEvent` 0x4ECCF0) to the ported `ReportFrontendAudioEvent` while 0x4ECCF0 is unported. Open: the mouse panel (`<< >> Spawn`; the original showed none with keyboard input) |
| spawn | same position and camera behind the player; the HUD clock is off and GTA's clock stopped like samp's (`RuntimeToggle::GameClock(false)` at install, toggle bit `NO_GAME_CLOCK` in the `CClock::Update` and `CHud::DrawPlayerInfo` ports; RPC 30 TogglePlayerClock turns it back on) |

Browser (wasm32, headless Chromium `--gpu native`, private open.mp + sa-udp-proxy on other ports via `SAMP_LOCK`): the
same start-up up to class selection: no logos or menu, samp's loading screen from samp.saa (served from the page's game
folder), connect, class 0 requested. The class view then blocks every iteration on the unported FX getter
`info_vget_value` 0x4A56E0 (white frames, WEB-SAMP's known blocker), so `CPad::UpdatePads` never sees the Shift key and
no spawn follows there; natively the same keys spawn.

## samp's main.scm (2026-11-12, SAMP-SCM)

samp.dll serves samp.saa's `main.scm` (103876 bytes) and `script.img` (194560 bytes) in place of GTA's. With
`SA_SAMP_SCM=1` sa-game serves both through the same redirect (`samp_saa.rs`); the default stays GTA's main.scm
until the blockers below are ported. samp's main thread (`MAIN` at 0xBE64) fades, sets the clock to 01:00
(`00C0 1 0`), and starts the threads `initial`, `initil2`, `intman`, `hj`, `sa-smp`.

Native run (`native-join.sh` with `SA_SAMP_DIR=re/samp/client`, `SA_SAMP_SCM=1`, port 7818, lock samp-scm.lock):

1. `initil2` opcode 06B1 (CREATE_SEARCHLIGHT) calls `CTheScripts::AddScriptSearchLight` 0x493000 (unported). The
   iteration is abandoned in the middle of the command, so the thread's IP lands inside the parameters and every
   following iteration blocks on garbage opcodes (`call_command_handler`, `locate_char_char_command` 0x4870F0,
   `locate_char_car_command` 0x487420, `char_in_angled_area_check_command` 0x487F60), then a SIGSEGV (null page,
   frame 82). These are consequences, not separate blockers.
2. With 0x493000 stubbed (exploration only): `CTheScripts::AttachSearchlightToSearchlightObject` 0x4934F0 (unported).
3. With both stubbed: no blocked iteration in 1600 frames; connect, class selection, spawn and every sampcov command as
   with GTA's main.scm; the "Fat" box is gone during connecting.

To make samp's main.scm the default: port 0x493000 and 0x4934F0 (gta-reversed `TheScripts.cpp`; the slot uses
`tScriptSearchlight` 0x7C bytes at 0xA94D68, `NumberOfScriptSearchLights` 0xA90830, and `RemoveScriptSearchLight`).
Once a light exists, `NumberOfScriptSearchLights` != 0 makes the render path call the unported
`CTheScripts::RenderAllSearchLights` 0x493E30 and the CHeli cone functions 0x6C4650 / 0x6C7C50 / 0x6C46E0
(`sa-core/src/mvpb_app.rs`), and the per-frame search-light update has to be checked too. With samp's 01:00 the
connecting scene was night before the 12:00 clock above (the original shows day).
