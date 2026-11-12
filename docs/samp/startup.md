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
| 0x866CD8 "title_pc_US" → "title", 0x866CCC "loadsc%d" → "title" (`74 69 74 6C 65 00`) | `CLoadingScreen::LoadSplashes` string data | not yet: needs samp.saa's `loadscs.txd` served at run time from the user's SA:MP install (samp-rs `saafs` / `samp-saa`); without it the engine shows GTA's own splash | open |
| 0x590099 `E8 C2 7A 19 00` → 5×NOP (`CSprite2d::DrawRect`) | `CLoadingScreen::RenderSplash` | not yet (goes with the splash names) | open |
| not-connected scene every frame until InitGame (samp.dll 0x100089B0): weather 1, HUD off, ped at (1133.05, -2038.40, 69.1), fixed camera (1093, -2036, 90) pointing at (384, -1557, 20) jump cut | script opcodes 015F / 0160 / 0373 | `samp_net.rs::connecting_scene` through the adapter (`set_fixed_position`, `point_at`, `set_weather`, `set_hud_visible`, `set_ped_position`) | done |
| chat lines `Started` (netgame created), `Connecting to H:P...` (connect tick > 3000 ms, samp.dll 0x10008930), `Connected. Joining the game...` (0x22), `Connected to {B9C9BF}name` (InitGame), debug colour 0xA9C4E4 | samp chat | `samp_net.rs::debug_line` + samp-rs `lifecycle::NetGame` | done |
| class selection: RPC 128 on entry, Left/Right/Shift polled from the keyboard state, sounds 1052/1053, HUD off while no menu | samp's class selection (samp-rs `classsel.rs`, `spawn.rs`) | `samp_net.rs::class_selection_tick` with samp-rs `classsel::ClassSelection`; keys from `NewKeyState` 0xB73190 (+0x22A left, +0x22C right, +0x260 shift) | done (keyboard); mouse panel open |
| spawn: restore camera, behind player, HUD on, control on, teleport z+0.5, heading, RPC 52 | samp's local spawn routine | `samp_net.rs::spawn_local` (existing) | done |

Headless runs (offscreen sa-game, no keyboard) set `SA_SAMP_AUTO_SPAWN=1`: Shift is then held synthetically;
samp's 2 s guard and the class-info wait still apply (`tools/e2e/samp/native-join.sh` sets it).
