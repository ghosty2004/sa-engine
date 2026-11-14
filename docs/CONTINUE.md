# Where to continue (state at 2026-11-14)

**Newest tree = branch `worktree-agent-ab80f5d4fe8147e79` (INTRO-2, docs/verify/integration-intro2.md): master
`58a1c75b` + AUDIO-STREAM + INTRO-1 + SCRIPT-5 + the intro ports up to the BMX ride; verify-all IDENTICAL 13/13;
master fast-forwards to it.**
New Game now runs the intro natively (no `--start-in-world`, no `SA_GAME_SCRIPT_SKIP`, no stubs) through the airport
cutscene, the police-car ride and the drop-off in Jefferson ("Get on the bike."), 0 blocked to frame 6000; with keys
CJ rides the BMX into East Los Santos until the bike hits a ped (`CPed::KillPedWithCar` 0x5F0360, next blocker).
`--fixed-step` runs are deterministic again (the audio threads run cooperatively natively). Queue: 2395 unverified
hooks in 376 batches.

Previous state (2026-11-12 ~15:00):

**Integrated tree = branch `worktree-agent-a623a7f9d5dd59528` (INTEGRATE-10, docs/verify/integration-10.md), verified
IDENTICAL on all 13 recordings; master fast-forwards to it.** It is master `d60bbee9` (INTEGRATE-9) plus SAMP-FX:
- everything master had (INTEGRATE-7: native Linux game, AI-7/8, VERIFIER-9, VEH-3/4, WORLD, WRITER-SAMP, SAMP-UI;
  INTEGRATE-6: PORT-AI-7, VERIFIER-10/11 with 393 promotions; README + GETTING-STARTED);
- INTEGRATE-8 (WEB-3's tree): web build (wasm32, WebGL2/WebGPU, input, full window, menu cursor), SA:MP natively and
  in the browser via sa-udp-proxy, SAMP-4 (sa-samp write side, API v3) + SAMP-START + SAMP-UI-2, STD-15 driving,
  WRITER-NATIVE / WRITER-CLIMB picks, world6-1/2/3, WEB-AUDIO;
- samp-rs in `samp-rs/` (own Cargo workspace; sa-game takes samp-client/samp-proto/samp-saa from it by path,
  samp-rs takes sa-samp-api/sa-net from `crates/` by path; no external ~/samp-rs reference left in code or
  scripts; third-party auth table at `re/samp/auth_table.txt`, SA:MP client files at `re/samp/client`);
- the generator fix: `ai_patterns.py` keeps `unverified` for listed hooks until L1/L2/L3 (catalogue state
  `implemented` = written, not verified);
- SAMP-FX (INTEGRATE-10): skin change / class skins, animations, frontend sounds, hands up / special actions,
  give-weapon + HUD icon, HUD clock toggle, samp's main.scm / script.img by default in samp mode (no "Fat +" box;
  `SA_SAMP_SCM=0` keeps GTA's), `serve.py --samp-dir` (samp-demo.sh defaults `SAMP_DIR` to `re/samp/client`),
  WEB-AUDIO latency, SAMP-SCM, HELI-CONE (CHeli::SearchLightCone 0x6C58E0). Queue: 2329 unverified hooks in 344
  batches.

Branches below marked "in INTEGRATE-9" are done; the rest are still to integrate (each = its own commits; integrate
with picks, regenerate generated files with `--check` clean, then verify-all).

Branches:

| branch (worktree-agent-…) | head | content |
|---|---|---|
| a51e2199757a44c0b (WEB-3) **in INTEGRATE-9** | 421ba072 | the most complete combined tree: web build (wasm32, WebGL2/WebGPU, input, full window, menu cursor), SA:MP in the browser via the proxy, SAMP-4 + SAMP-START + SAMP-UI-2, STD-15 driving, WRITER-NATIVE / CLIMB picks, WEB-AUDIO. |
| a2a63a1cdd7fd3b05 (STD-15) **in INTEGRATE-9** | 804331df | enter car / drive / exit natively; stream-thread lock release; 0x5F3730 family hand ports |
| a6131240cce7c696b (WRITER-CLIMB) | ce867d6e | climb chain, garage doors |
| a6d319b53fd3185ec (WRITER-NATIVE) | 6367086e | sprint, fists, jump, collision freezes, garages |
| a33f0e44a9994a468 (WRITER-WORLD-2) | 0174d246 | 74 WORLD ports incl. TriggerExplosionSectorList, protection chunks, interiors (not in WEB-3 except world6-2) |
| aa6be1d1b3cc0f218 (WRITER-SAMP-2) | 855a5ba0 | FireInstantHit (detwin scrub must skip it), projectiles, water, checkpoint render |
| af11bd66b65e2aabf (SAMP-4) **in INTEGRATE-9** | 6b6d5048 | sa-samp-api v2/v3, sa-samp write side, samp mode toggles, native client samp_net.rs; (its samp-rs dependency is now `samp-rs/` by path) |
| afb099518e063a1b1 (SAMP-START) **in INTEGRATE-9** | cdf5dc34 | SA:MP startup 1:1 via samp.dll patch points (docs/samp/startup.md) |
| a0fec7b0bdae3c86c (SAMP-UI-2) **in INTEGRATE-9** | 23ee4454 | SA:MP UI compared with the original client |
| abbdcb1da87a27a3c (SAMP-SCM) **in INTEGRATE-10** | 646e0f7b | samp's main.scm behind SA_SAMP_SCM=1 (blockers listed in docs/samp/startup.md) |
| a2404a204209682e4 (WEB-AUDIO) **in INTEGRATE-9** | e477afe4+ | browser audio (AudioWorklet + SharedArrayBuffer ring); latency tuning in progress |
| ab013435f0f3841aa (SAMP-FX) **in INTEGRATE-10** | f10a8428+ | sa-samp: skin change, one-off sounds, animations (in progress at hand-off) |
| aa4c0e8384f020c37 (INTEGRATE-6) **in INTEGRATE-9** | b766ecc3 | PORT-AI-7 + VERIFIER-10 on master + SA:MP verification (did not finish) |

Added after 11:45:

| branch (worktree-agent-…) | head | content |
|---|---|---|
| ab013435f0f3841aa (SAMP-FX) **in INTEGRATE-10** | e6e0528e | on WEB-3 421ba072: SA:MP files served to the browser (serve.py --samp-dir), skin change / class skins, animations, 17 frontend sounds, hands up, give-weapon + HUD icon, HUD clock toggle, samp's main.scm default (no "Fat +" box; includes HELI-CONE c47670e6 and WRITER-SAMP's search lights). Serves 127.0.0.1:8644 with open.mp 7813 + proxy 7781. The newest SA:MP tree. |
| a6df07424cbb59315 (HELI-CONE) **in INTEGRATE-10** | c47670e6 | CHeli::SearchLightCone 0x6C58E0 (in SAMP-FX's tree) |
| acc62cadcdbd3be53 (AUDIO-STREAM) **in INTRO-2** | 22e226f8 | CAEDataStream + lewton vorbisfile replacement, GetStreamID, 16-tap sinc mixer, native WAV tap; streams decode but the game holds the stream buffer at -10000 (open); web streams off by default (hang) |
| a43f6b653f2e96d67 (SCRIPT-5) **in INTRO-2** | 17a4881f | 31 callees of the first missions' opcodes (LOCATE helpers, CleanUpThisPed, AddPassenger …); per-mission remaining list in its hand-back (05EB StartPlaybackRecordedCar, 070A, 0637, 0723, 07F5 …) |
| newgame-1 (NEWGAME) (not in INTRO-2) | 5752d60e | New Game from the pause menu (CMenuSystem::Initialise native, CTaskComplexWaitForPed); its CAEDataStream port duplicates AUDIO-STREAM's — keep AUDIO-STREAM's |
| a2c16c35b48eab431 (INTRO-1) **in INTRO-2** | a87cf585 | New Game runs the real intro cutscene natively (no --start-in-world, no SCRIPT_SKIP); stops at the police-car ride: StartPlaybackRecordedCar 0x45A980, CleanUpThisPed (SCRIPT-5 has it), CTaskComplexDriveToPoint::IsTargetBlocked 0x6452C0 |
| ac8a20a070c956b08 (INTEGRATE-8) **in INTEGRATE-9** | f94f1ee3 | WEB-3's whole tree on b239e888; combined with master in INTEGRATE-9 |
| a1c6b8a6b0d220dec (FRAMELIMIT) | see branch | on master 0e66feb2: the Display Setup frame-limiter toggle works natively (ON/OFF/ON/OFF, then 2,600 iterations of the Grove Street walk with 0 blocked, offscreen and in a window; the setting is saved and changes the pacing from 25 fps to uncapped). Ports (unverified): CRealTimeShadowManager::ReInit 0x706870 (the toggle resets the D3D device through the vsync flag), then 0x4AF700, 0x649020, 0x664590, 0x6648C0, 0x4AC920. CEventVehicleCollision::AffectsPed 0x4B2EE0 is CARJACK's file taken unchanged; CARJACK also has 0x4AC920, so keep one of each. SA_GAME_KEYS accepts `esc`. GetBeatInfo/GetTrackInfo run each iteration with the limiter off (0 blocked) |

Next steps, in order:
1. Fast-forward master to INTRO-2 (`worktree-agent-ab80f5d4fe8147e79`). Then integrate NEWGAME (without its
   CAEDataStream copy), WRITER-WORLD-2, WRITER-SAMP-2 (FireInstantHit needs detwin to skip its scrub; it also ports
   0x6E91D0, which INTRO-2 has: keep one); verify-all 13/13; record promotions.
1b. Intro: port `CPed::KillPedWithCar` 0x5F0360, script the BMX ride east to Grove Street (follow the CJ icon), then
   the arrival cutscene and free roam with the Big Smoke marker (integration-intro2.md §2 / §4; the run script and
   key timings are there).
2. SA:MP: adapter steps 5b–12 in `samp-rs/`, sa-samp write side (non-frontend sounds through
   ProcessMissionAudioEvent, money, trains, proofs, bones for attached objects), unapplied RPCs (e.g. 60), audio streams (CAEDataStream 0x4DC620 +
   a Rust Vorbis decoder), the ~200 SA:MP-executed functions still unported (docs/samp/native-unported.csv),
   class-selection panel.
3. Native: burning/wrecked car path (0x4B1F10, 0x4DCBE0, 0x4B0DA0, 0x6375F0), remaining blockers from det runs.
4. Verification: the ~7,500 implemented-but-unverified hooks, SA:MP priority first; new recordings (driving,
   interiors, climbing, SA:MP sessions) so the gen2 hooks without callers get evidence.
5. Web: real viewport resolution, OPFS settings/saves, audio latency, streams.

Known issues: 5 doubled phase B registrations (0x5B0440, 0x618E70, 0x84A540, 0x84E300, 0x84F320); 0x60A820 needs a
preserve-edx spec; `docs/samp/samp_usage.py` outputs and `catalog/progress.md` are stale against the catalogue; `inflight.py` does not
see `#[sa_core::native(` registrations (0x4413C0 listed as an exclusion);
stream volume -10000; web stream hang; ProcessMissionAudioEvent 0x4ECCF0 (all non-frontend PlayerPlaySound), phone
task, class-selection << >> Spawn panel, CEventDamage::AffectsPedGroup 0x4B38D0, burning-car path. The wasm build
needs wasm-bindgen 0.2.130 (`cargo binstall --root target/tools wasm-bindgen-cli@0.2.130`).
