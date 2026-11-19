# samp.dll's game patches and the samp mod

samp.dll changes `gta_sa.exe` with 87 memory patches at start-up (samp-rs's list:
`samp-rs/crates/samp-client/src/patches.rs`, each with its row of `samp-rs/docs/re/data/gta-patches-r4-annotated.csv`;
measured effects in `samp-rs/docs/phase3/trace-notes.md`, "SP-OFF baseline"). The samp mod (`mods/samp`) changes the
same game through the engine's generic switches, which name what they do in the game, never SA:MP:

* game options (`sa_types::GameOption`, mod API `set_game_option`; the port skips exactly the original call the
  option names, `sa_core::options::Feature`), switched off in the mod's `sa_init` and turned back on by the host
  when it disables the mod;
* game limits (`sa_types::GameLimit`, `set_game_limit`, only in `sa_init` / `sa_settings`;
  `sa_core::options::Limit`), read where the original sizes the pool or table from its constant;
* the game's script files (`set_script_file`, below) and the main script's global variables (`set_script_global`).

samp's object pool size (3000, patch row 113) is written by samp-rs's world hooks rather than this table; the mod
raises `GameLimit::Objects` to it as well.

With nothing switched or raised, every port runs as the original (the options are on and the limits the original's
by default; verify-all is unchanged).

## Summary

| category | patches | what it means |
|---|---:|---|
| covered | 12 | an option that existed before (start screens, traffic, peds, planes, cutscenes, replay, plants, sun and moon, idle camera, help boxes, zone names) |
| new option or limit | 25 | added here: 7 options, 2 extensions of `AmbientPeds`, 14 limits (13 pools and tables + the streaming budget) |
| needs an option (left) | 28 | a game change the mod should make but that has no switch yet (15 of them the `CFont` line buffer); listed in the ROADMAP's divergence ledger |
| DLL host only | 6 | patches of the Win32 / D3D / DirectInput host that the engine's platform layer replaces |
| not wanted | 16 | for remote players as `CPlayerPed`s (the mod streams other players in as actors) or for samp.saa's own game data and loading screen, which the mod does not serve |

## The story script

samp.dll serves its own `main.scm` and `script.img` from samp.saa: the story never starts, the main script creates
the player, sets the densities and the wanted level cap to 0 and runs samp's few threads (garages, the stunt bonus
`HJ`, ...). The engine has the generic counterpart: `set_script_file(file, data)` (permission `scripts`, only in
`sa_init` / `sa_settings`) gives the game a mod's `data\script\main.scm` or `data\script\script.img`; the platform's
file layer serves them whenever the game opens those paths (`sa_platform::vfs::set_redirect`, registered by sa-game's
mod host; every new game reads them again). The samp mod gives the game samp's own two files when the user copied
`samp.saa` into the mod's folder (read at run time like `sampgui.png`, never shipped), else samp-rs's minimal main
script (`samp_client::scm::build_main_scm`: the player at Grove Street, densities and wanted level cap 0, roads off,
then idle). Either way the per-frame densities / wanted-level stopgap of the mod is gone.

`--start-in-world` skips the install's intro mission (`START_MISSION 2` in main.scm's MAIN); with a mod's main script
it finds nothing to skip and changes nothing. The samp mod switches the start screens off, so the game loads straight
into its main script's world with or without the flag.

The stunt bonus (InitGame's flag, RPC 104 EnableStuntBonusForPlayer) is samp's main script's `$709`, which its `HJ`
thread tests before it awards a stunt (samp.dll writes `ScriptSpace + 0xB14`, 0xA4A474). The mod writes the same
variable through `set_script_global(709, on)` (`WorldState::set_stunt_bonus`); samp-rs's minimal script has no stunt
thread, so without samp.saa the switch has no effect, as with samp-rs's DLL without samp.saa.

## Every patch

`row`: the line of gta-patches-r4-annotated.csv. `engine`: the option or limit that does the same.

| row | address | samp.dll patch | category | engine | note |
|---:|---|---|---|---|---|
| 121 | 0x747483 | psInitialize: keep gGameState | covered | `GameOption::StartScreens` | with the logos / intro / menu skip of the same option |
| 157 | 0x5720A5 | CUserDisplay::Process: no CPlaceName::Process | covered | `GameOption::ZoneNames` |  |
| 158 | 0x53C159 | CGame::Process: no CPlantMgr::Update | covered | `GameOption::Plants` |  |
| 159 | 0x53C136 | CGame::Process: no CCoronas::DoSunAndMoon | covered | `GameOption::SunAndMoon` |  |
| 163 | 0x53C090 | CGame::Process: no CReplay::Update | covered | `GameOption::Replay` |  |
| 180 | 0x588BE0 | CHud::SetHelpMessage: ret | covered | `GameOption::HelpMessages` |  |
| 181 | 0x53C06A | CGame::Process: no CTheCarGenerators::Process | covered | `GameOption::CarGenerators` |  |
| 182 | 0x434272 | GenerateRandomCars: no plane generation | covered | `GameOption::AmbientPlanes` |  |
| 185 | 0x612710 | CPopulation::AddPed := return NULL | covered | `GameOption::AmbientPeds` |  |
| 189 | 0x53C1C1 | CGame::Process: no GenerateRandomCars | covered | `GameOption::RandomTraffic` |  |
| 244 | 0x524582 | Process_FollowPed_SA: no idle camera | covered | `GameOption::IdleCamera` |  |
| 246 | 0x53BF28 | CGame::Process: no CCutsceneMgr::Update | covered | `GameOption::Cutscenes` | also keeps the story script from taking over (now replaced, see "The story script") |
| 107 | 0x551024 | CPools::Initialise: vehicle pool 710 | new option or limit | `GameLimit::Vehicles` | 710 (samp also pushes a null pool name) |
| 108 | 0x55105F | CPools::Initialise: building pool 20000 | new option or limit | `GameLimit::Buildings` | 20000 |
| 109 | 0x5510CF | CPools::Initialise: dummy pool 4000 | new option or limit | `GameLimit::Dummies` | 4000 |
| 110 | 0x550F46 | CPools::Initialise: PtrNode Single 100000 | new option or limit | `GameLimit::PtrNodesSingle` | 100000 |
| 111 | 0x550F82 | CPools::Initialise: PtrNode Double 8000 | new option or limit | `GameLimit::PtrNodesDouble` | 8000 |
| 112 | 0x550FBA | CPools::Initialise: EntryInfoNode 5000 | new option or limit | `GameLimit::EntryInfoNodes` | 5000 |
| 114 | 0x550FF2 | CPools::Initialise: Peds 240 | new option or limit | `GameLimit::Peds` | 240 |
| 115 | 0x551283 | CPools::Initialise: PedIntelligence 240 | new option or limit | `GameLimit::PedIntelligence` | 240 |
| 116 | 0x551140 | CPools::Initialise: Task 1524 | new option or limit | `GameLimit::Tasks` | 1524 |
| 117 | 0x551178 | CPools::Initialise: Event 456 | new option or limit | `GameLimit::Events` | 456 |
| 118 | 0x54F3A1 | CPlaceable::InitMatrixArray: 6000 matrices | new option or limit | `GameLimit::Matrices` | 6000 |
| 119 | 0x551106 | CPools::Initialise: col-model pool 32511 | new option or limit | `GameLimit::ColModels` | 32511 |
| 127 | 0x5B8E6A | CStreaming::Init2: streaming memory budget | new option or limit | `GameLimit::StreamingMemory` | the mod has no RAM figure: it asks for samp's 1-2 GB budget (256 MiB), samp gives 1 GiB above 4 GB |
| 131 | 0x731F60 | CTxdStore::Initialise: txd pool 20000 | new option or limit | `GameLimit::TextureDictionaries` | 20000 |
| 155 | 0x4DF7E2 | ServiceAmbientGunFire: no ambient gunfire | new option or limit | `GameOption::AmbientGunfire` | a due burst resets the state with the current time (samp's mangled `mov` stores a status word: no audible difference) |
| 160 | 0x748063 | __MainWndProc: no InitAfterLostFocus | new option or limit | `GameOption::PauseMenuOnFocus` |  |
| 161 | 0x58FBE9 | CHud::Draw: no DrawVehicleName | new option or limit | `GameOption::VehicleNameOnHud` |  |
| 164 | 0x440833 | CEntryExit::TransitionFinished: no GenerateAmbientPeds | new option or limit | `GameOption::AmbientPeds` | added to the option: no ambient peds generated in interiors |
| 165 | 0x53EA08 | Idle: no CRealTimeShadowManager::Update | new option or limit | `GameOption::RealTimeShadows` | with row 184 |
| 172 | 0x5B47B0 | CFileLoader::LoadPickup := RET | new option or limit | `GameOption::SinglePlayerPickups` | IPL pickups |
| 179 | 0x441482 | SortOutStreamingAndMemory: keep RW objects after death | new option or limit | `GameOption::DeathModelCleanup` |  |
| 184 | 0x542485 | CPhysical dtor: no ReturnRealTimeShadow | new option or limit | `GameOption::RealTimeShadows` | with row 165 |
| 186 | 0x613BA7 | AddPedInCar: no SetPedInCarDirect | new option or limit | `GameOption::AmbientPeds` | added to the option: AddPedInCar would seat the NULL AddPed returns |
| 205 | 0x58FC2E | CHud::Draw: no vital stats | new option or limit | `GameOption::VitalStatsOnHud` | the radar is drawn instead, as with the patch |
| 208 | 0x593D7C | Interior_c::Init: no AddPickups | new option or limit | `GameOption::SinglePlayerPickups` | interior pickups |
| 162 | 0x86D1EC | playidles anim block name blanked | needs an option (left) | - | player idle animations ("playidles" blanked): a data patch read by two constructors; not done yet |
| 166 | 0x71162C | RenderStencilShadows: overlay alpha 80 | needs an option (left) | - | stencil shadow darkness 50 -> 80: a value, not a switch |
| 170 | 0x4BC6C1 | SetEventResponseTask: no editable response (facial expressions) | needs an option (left) | - | facial expressions of event responses |
| 187 | 0x609A4E | ProcessAnimGroups: no move-anim-group store | needs an option (left) | - | a skin's own walk style unless InitGame's CJ walk: tied to samp's gated call (row 43) and the InitGame flag |
| 192 | 0x609CB4 | ClearWeaponTarget: no ClearPlayerWeaponMode | needs an option (left) | - | the camera's weapon mode kept when the player's target is cleared |
| 199 | 0x53A984 | CFire::ProcessFire: skip the BMX branch | needs an option (left) | - | the BMX branch skipped: a fire near a BMX no longer sets its rider on fire or bursts a tyre |
| 202 | 0x58DB5F | CHud::DrawWanted: level from samp | needs an option (left) | - | the HUD's wanted stars as a value RPC 133 sets without a real wanted level: needs a HUD value, not a switch |
| 203 | 0x575B0E | PrintMap: no DrawRect | needs an option (left) | - | one `CSprite2d::DrawRect` of the pause menu's map left out |
| 212 | 0x5952A6 | InteriorGroup_c::Exit: no Interior_c::Exit | needs an option (left) | - | `Interior_c::Exit` not called when an interior group exits |
| 216 | 0x5E7847 | CPed::Render: weapon also while holding | needs an option (left) | - | `CPed::Render` draws the weapon also while the ped holds something |
| 223 | 0x6348F6 | 0x6348F0: phone model -1 | needs an option (left) | - | the cell phone's model removal (samp's phone special action) |
| 224 | 0x634DE2 | 0x634A40: phone model -1 | needs an option (left) | - | as row 223 |
| 225 | 0x71A223 | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | CFont::ProcessCurrentString's line buffer (0x12C -> 0x32C bytes) for long samp texts; the mod draws its texts itself and caps `game_text` at 400 bytes |
| 226 | 0x71A5D5 | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 227 | 0x71A247 | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 228 | 0x71A282 | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 229 | 0x71A26A | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 230 | 0x71A2E5 | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 231 | 0x71A3AC | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 232 | 0x71A4B9 | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 233 | 0x71A53C | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 234 | 0x71A37F | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 235 | 0x71A518 | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 236 | 0x71A525 | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 237 | 0x71A54B | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 238 | 0x71A5B6 | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 239 | 0x71A3BB | CFont::ProcessCurrentString: frame 0x32C | needs an option (left) | - | as row 225 |
| 245 | 0x712025 | CreateNumberOfBirds: LOS checks objects | needs an option (left) | - | birds test line of sight against objects (samp places many) |
| 124 | 0x745B87 | GetVideoModeList: no aspect filter | DLL host only | - | the engine's video modes: a mod's requested size is applied even when the original's list leaves it out (sa-game `mods::settings`, the resolution mod) |
| 125 | 0x7459E1 | FillAvailableVMs: no aspect filter | DLL host only | - | as row 124 |
| 126 | 0x561872 | CTimer::Initialise: no QPC | DLL host only | - | the engine's clock is the platform layer's (sa-platform `clock`), not QueryPerformanceCounter under Wine |
| 130 | 0x53E94C | Idle: frame limiter 2 ms | DLL host only | - | the frame limiter's busy wait; sa-game paces frames itself (`--fixed-step`, the window's present) |
| 178 | 0x4090A0 | CStreaming::DeleteAllRwObjects: ret | DLL host only | - | made RET only because samp's sector-array relocation would corrupt an instruction inside it; the engine relocates nothing (effect: `LoadScene` keeps RW objects, ledger) |
| 209 | 0x7469AF | diMouseInit: non-exclusive mouse | DLL host only | - | DirectInput's cooperative level; the platform layer owns the mouse (mod API `input_capture`) |
| 122 | 0x866CD8 | splash name title_pc_US -> title | not wanted | - | samp.saa's loadscs.txd (one `title` texture); the mod keeps the game's loading screens |
| 123 | 0x866CCC | splash name loadsc%d -> title | not wanted | - | as row 122 |
| 128 | 0x4083C0 | CStreaming::InitImageList := RET | not wanted | - | only with samp.saa's own default.dat (its image list); the mod serves no game data from samp.saa |
| 129 | 0x590099 | RenderSplash: no DrawRect | not wanted | - | a rectangle under samp.saa's splash `title`; the mod keeps the game's own loading screens (row 122) |
| 167 | 0x609C08 | CPlayerPed::ProcessAnimGroups: no anim block switch | not wanted | - | remote player peds share anim blocks; as row 183 |
| 173 | 0x5E63A6 | CPed::RemoveWeaponWhenEnteringVehicle: no player-info lookup | not wanted | - | as row 183 |
| 174 | 0x621AEA | 0x621960: no player-info lookup | not wanted | - | as row 183 |
| 175 | 0x62D331 | CTaskComplexKillPedOnFootArmed::ControlSubTask: no player-info lookup | not wanted | - | as row 183 |
| 176 | 0x741FFF | CVehicle::DoDriveByShootings: no player-info lookup | not wanted | - | as row 183 |
| 183 | 0x60D64E | CPlayerPed ctor: no ped group | not wanted | - | remote players as CPlayerPeds; the mod streams other players in as actors (`create_actor`), no extra player peds |
| 193 | 0x60F2C4 | CPlayerPed::ProcessControl: no FindPlayerSlotWithPedPointer | not wanted | - | as row 183 (Players[-1] for remote player peds) |
| 206 | 0x5E1E70 | CPed::UpdatePosition: ignore m_standingOnEntity | not wanted | - | remote players' sync (standing-on-entity ignored for every ped); needs player peds first, as row 183 |
| 213 | 0x6884C4 | PlayerControlZelda: no m_fAimingRotation store | not wanted | - | remote players' aiming (restored around the local player only); as row 183 |
| 214 | 0x688200 | PlayerControlDucked: no m_fAimingRotation store | not wanted | - | as row 213 |
| 215 | 0x570535 | CPlayerInfo::Process: no hold/fight test | not wanted | - | a CPlayerInfo::Process block samp's own sync replaces (hold / fight test) (row 183) |
| 217 | 0x570546 | CPlayerInfo::Process: no script-command task | not wanted | - | as row 215 (the script-command task block) |
