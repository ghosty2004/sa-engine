# Where to continue (state at 2026-11-12 ~11:45)

master = 14f4b7a3 (INTEGRATE-7, verified IDENTICAL on all 13 recordings): native Linux game (walk, sprint, jump,
fists, window loop, deterministic mode, attractors, traffic, shadow fix), AI-7/8, VERIFIER-9 promotions,
VEH-3/4, WORLD, WRITER-SAMP, SAMP-UI, sa-samp (read side), sa-udp-proxy, sa-net.

Unmerged branches (each = its own commits; integrate with picks, regenerate generated files, then verify-all):

| branch (worktree-agent-…) | head | content |
|---|---|---|
| a51e2199757a44c0b (WEB-3) | 421ba072 | the most complete combined tree: web build (wasm32, WebGL2/WebGPU, input, full window, menu cursor), SA:MP in the browser via the proxy, SAMP-4 + SAMP-START + SAMP-UI-2, STD-15 driving, WRITER-NATIVE / CLIMB picks, WEB-AUDIO. Start the next integration from here. |
| a2a63a1cdd7fd3b05 (STD-15) | 804331df | enter car / drive / exit natively; stream-thread lock release; 0x5F3730 family hand ports |
| a6131240cce7c696b (WRITER-CLIMB) | ce867d6e | climb chain, garage doors |
| a6d319b53fd3185ec (WRITER-NATIVE) | 6367086e | sprint, fists, jump, collision freezes, garages |
| a33f0e44a9994a468 (WRITER-WORLD-2) | 0174d246 | 74 WORLD ports incl. TriggerExplosionSectorList, protection chunks, interiors (not in WEB-3 except world6-2) |
| aa6be1d1b3cc0f218 (WRITER-SAMP-2) | 855a5ba0 | FireInstantHit (detwin scrub must skip it), projectiles, water, checkpoint render |
| af11bd66b65e2aabf (SAMP-4) | 6b6d5048 | sa-samp-api v2/v3, sa-samp write side, samp mode toggles, native client samp_net.rs; Cargo.toml must point at file:///home/ghosty2004/samp-rs rev fb89010 (samp-rs commits are on its sa-engine/adapter branch) |
| afb099518e063a1b1 (SAMP-START) | cdf5dc34 | SA:MP startup 1:1 via samp.dll patch points (docs/samp/startup.md) |
| a0fec7b0bdae3c86c (SAMP-UI-2) | 23ee4454 | SA:MP UI compared with the original client |
| abbdcb1da87a27a3c (SAMP-SCM) | 646e0f7b | samp's main.scm behind SA_SAMP_SCM=1 (blockers listed in docs/samp/startup.md) |
| a2404a204209682e4 (WEB-AUDIO) | e477afe4+ | browser audio (AudioWorklet + SharedArrayBuffer ring); latency tuning in progress |
| ab013435f0f3841aa (SAMP-FX) | f10a8428+ | sa-samp: skin change, one-off sounds, animations (in progress at hand-off) |
| aa4c0e8384f020c37 (INTEGRATE-6) | b766ecc3 | PORT-AI-7 + VERIFIER-10 on master + SA:MP verification (did not finish) |

Added after 11:45:

| branch (worktree-agent-…) | head | content |
|---|---|---|
| ab013435f0f3841aa (SAMP-FX) | e6e0528e | on WEB-3 421ba072: SA:MP files served to the browser (serve.py --samp-dir), skin change / class skins, animations, 17 frontend sounds, hands up, give-weapon + HUD icon, HUD clock toggle, samp's main.scm default (no "Fat +" box; includes HELI-CONE c47670e6 and WRITER-SAMP's search lights). Serves 127.0.0.1:8644 with open.mp 7813 + proxy 7781. The newest SA:MP tree. |
| a6df07424cbb59315 (HELI-CONE) | c47670e6 | CHeli::SearchLightCone 0x6C58E0 (in SAMP-FX's tree) |
| acc62cadcdbd3be53 (AUDIO-STREAM) | 22e226f8 | CAEDataStream + lewton vorbisfile replacement, GetStreamID, 16-tap sinc mixer, native WAV tap; streams decode but the game holds the stream buffer at -10000 (open); web streams off by default (hang) |
| a43f6b653f2e96d67 (SCRIPT-5) | 17a4881f | 31 callees of the first missions' opcodes (LOCATE helpers, CleanUpThisPed, AddPassenger …); per-mission remaining list in its hand-back (05EB StartPlaybackRecordedCar, 070A, 0637, 0723, 07F5 …) |
| newgame-1 (NEWGAME) | 5752d60e | New Game from the pause menu (CMenuSystem::Initialise native, CTaskComplexWaitForPed); its CAEDataStream port duplicates AUDIO-STREAM's — keep AUDIO-STREAM's |
| a2c16c35b48eab431 (INTRO-1) | a87cf585 | New Game runs the real intro cutscene natively (no --start-in-world, no SCRIPT_SKIP); stops at the police-car ride: StartPlaybackRecordedCar 0x45A980, CleanUpThisPed (SCRIPT-5 has it), CTaskComplexDriveToPoint::IsTargetBlocked 0x6452C0 |
| ac8a20a070c956b08 (INTEGRATE-8) | f94f1ee3 | DONE and verified (verify-all int8 IDENTICAL 13/13 + int8b recheck): WEB-3's whole tree merged onto b239e888, duplicates settled, samp-rs dep → file:///home/ghosty2004/samp-rs rev fb890107. NOT yet combined with INTEGRATE-6 (now on master cff47307): next step = rebase INTEGRATE-6's commits (or master since b239e888) onto f94f1ee3, regenerate, verify-all, fast-forward. Then SAMP-FX, INTRO-1, SCRIPT-5, AUDIO-STREAM, NEWGAME (prefer AUDIO-STREAM's CAEDataStream), WRITER-WORLD-2, WRITER-SAMP-2. |

Next steps, in order:
1. INTEGRATE-8 (or its successor): bring SAMP-FX's tree (newest SA:MP), STD-15, SCRIPT-5, AUDIO-STREAM, NEWGAME, INTRO-1 and WEB-3's tree (and the branches above not in it) onto master; verify-all 13/13; record promotions.
2. SA:MP: adapter steps 5b–12 in samp-rs (sa-engine/adapter), sa-samp write side (skin, anims, sounds, money,
   trains, proofs, bones for attached objects), unapplied RPCs (e.g. 60), audio streams (CAEDataStream 0x4DC620 +
   a Rust Vorbis decoder), the ~200 SA:MP-executed functions still unported (docs/samp/native-unported.csv),
   samp's main.scm (search-light render path, CHeli cone functions), class-selection panel, HUD clock toggle.
3. Native: burning/wrecked car path (0x4B1F10, 0x4DCBE0, 0x4B0DA0, 0x6375F0), remaining blockers from det runs.
4. Verification: the ~7,500 implemented-but-unverified hooks, SA:MP priority first; new recordings (driving,
   interiors, climbing, SA:MP sessions) so the gen2 hooks without callers get evidence.
5. Web: real viewport resolution, OPFS settings/saves, audio latency, streams.

Known issues to fix first: the generators (ai_patterns.py, patterns.py, inflight.py) treat catalogue state `implemented` as verified — fix before any regeneration (see integration-6.md); duplicate #[hook] registrations in the WEB-3-based trees (e.g. RenderWeaponCB 0x733670,
CTaskComplexPassObject::ControlSubTask) make `cargo test -p sa-hook --lib` fail there; stream volume -10000;
web stream hang; ProcessMissionAudioEvent 0x4ECCF0 (all non-frontend PlayerPlaySound), phone task, class-selection
<< >> Spawn panel, CEventDamage::AffectsPedGroup 0x4B38D0, burning-car path.

samp-rs is now part of this repository (`samp-rs/`, 2026-11-12, user decision: all SA:MP work happens here). It takes
sa-samp-api and sa-net by `path` from `crates/` (master's sa-samp-api was raised to API v3 for it: PadState
`chat_indicated` / `ped_walk`, `force_weather`, `destroy_script_thread`; master's sa-samp answers the new write calls
with Unsupported until the integration branch's write side arrives). Still to do at the next integration: point
sa-game's samp-client/samp-proto dependency at `path = "../../samp-rs/crates/…"` (the integration branch uses
`git = file:///home/ghosty2004/samp-rs`).
