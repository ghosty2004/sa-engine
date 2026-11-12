# Where to continue (state at 2026-11-12 ~14:30)

**Integrated tree = branch `worktree-agent-ab58bdb347d7ad153` (INTEGRATE-9, docs/verify/integration-9.md), verified
IDENTICAL on all 13 recordings; master fast-forwards to it.** It holds:
- everything master had (INTEGRATE-7: native Linux game, AI-7/8, VERIFIER-9, VEH-3/4, WORLD, WRITER-SAMP, SAMP-UI;
  INTEGRATE-6: PORT-AI-7, VERIFIER-10/11 with 393 promotions; README + GETTING-STARTED);
- INTEGRATE-8 (WEB-3's tree): web build (wasm32, WebGL2/WebGPU, input, full window, menu cursor), SA:MP natively and
  in the browser via sa-udp-proxy, SAMP-4 (sa-samp write side, API v3) + SAMP-START + SAMP-UI-2, STD-15 driving,
  WRITER-NATIVE / WRITER-CLIMB picks, world6-1/2/3, WEB-AUDIO;
- samp-rs in `samp-rs/` (own Cargo workspace; sa-game takes samp-client/samp-proto/samp-saa from it by path,
  samp-rs takes sa-samp-api/sa-net from `crates/` by path; no external ~/samp-rs reference left in code or
  scripts; third-party auth table at `re/samp/auth_table.txt`);
- the generator fix: `ai_patterns.py` keeps `unverified` for listed hooks until L1/L2/L3 (catalogue state
  `implemented` = written, not verified). Queue: 2328 unverified hooks in 343 batches.

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
| abbdcb1da87a27a3c (SAMP-SCM) | 646e0f7b | samp's main.scm behind SA_SAMP_SCM=1 (blockers listed in docs/samp/startup.md) |
| a2404a204209682e4 (WEB-AUDIO) **in INTEGRATE-9** | e477afe4+ | browser audio (AudioWorklet + SharedArrayBuffer ring); latency tuning in progress |
| ab013435f0f3841aa (SAMP-FX) | f10a8428+ | sa-samp: skin change, one-off sounds, animations (in progress at hand-off) |
| aa4c0e8384f020c37 (INTEGRATE-6) **in INTEGRATE-9** | b766ecc3 | PORT-AI-7 + VERIFIER-10 on master + SA:MP verification (did not finish) |

Added after 11:45:

| branch (worktree-agent-…) | head | content |
|---|---|---|
| ab013435f0f3841aa (SAMP-FX) | e6e0528e | on WEB-3 421ba072: SA:MP files served to the browser (serve.py --samp-dir), skin change / class skins, animations, 17 frontend sounds, hands up, give-weapon + HUD icon, HUD clock toggle, samp's main.scm default (no "Fat +" box; includes HELI-CONE c47670e6 and WRITER-SAMP's search lights). Serves 127.0.0.1:8644 with open.mp 7813 + proxy 7781. The newest SA:MP tree. |
| a6df07424cbb59315 (HELI-CONE) | c47670e6 | CHeli::SearchLightCone 0x6C58E0 (in SAMP-FX's tree) |
| acc62cadcdbd3be53 (AUDIO-STREAM) | 22e226f8 | CAEDataStream + lewton vorbisfile replacement, GetStreamID, 16-tap sinc mixer, native WAV tap; streams decode but the game holds the stream buffer at -10000 (open); web streams off by default (hang) |
| a43f6b653f2e96d67 (SCRIPT-5) | 17a4881f | 31 callees of the first missions' opcodes (LOCATE helpers, CleanUpThisPed, AddPassenger …); per-mission remaining list in its hand-back (05EB StartPlaybackRecordedCar, 070A, 0637, 0723, 07F5 …) |
| newgame-1 (NEWGAME) | 5752d60e | New Game from the pause menu (CMenuSystem::Initialise native, CTaskComplexWaitForPed); its CAEDataStream port duplicates AUDIO-STREAM's — keep AUDIO-STREAM's |
| a2c16c35b48eab431 (INTRO-1) | a87cf585 | New Game runs the real intro cutscene natively (no --start-in-world, no SCRIPT_SKIP); stops at the police-car ride: StartPlaybackRecordedCar 0x45A980, CleanUpThisPed (SCRIPT-5 has it), CTaskComplexDriveToPoint::IsTargetBlocked 0x6452C0 |
| ac8a20a070c956b08 (INTEGRATE-8) **in INTEGRATE-9** | f94f1ee3 | WEB-3's whole tree on b239e888; combined with master in INTEGRATE-9 |

Next steps, in order:
1. Fast-forward master to INTEGRATE-9. Then integrate, on top of it: SAMP-FX (newest SA:MP tree, based on WEB-3
   421ba072, so take its own commits after 421ba072), INTRO-1, SCRIPT-5, AUDIO-STREAM, NEWGAME (prefer AUDIO-STREAM's
   CAEDataStream), WRITER-WORLD-2, WRITER-SAMP-2 (FireInstantHit needs detwin to skip its scrub), SAMP-SCM;
   verify-all 13/13; record promotions.
2. SA:MP: adapter steps 5b–12 in `samp-rs/`, sa-samp write side (skin, anims, sounds, money,
   trains, proofs, bones for attached objects), unapplied RPCs (e.g. 60), audio streams (CAEDataStream 0x4DC620 +
   a Rust Vorbis decoder), the ~200 SA:MP-executed functions still unported (docs/samp/native-unported.csv),
   samp's main.scm (search-light render path, CHeli cone functions), class-selection panel, HUD clock toggle.
3. Native: burning/wrecked car path (0x4B1F10, 0x4DCBE0, 0x4B0DA0, 0x6375F0), remaining blockers from det runs.
4. Verification: the ~7,500 implemented-but-unverified hooks, SA:MP priority first; new recordings (driving,
   interiors, climbing, SA:MP sessions) so the gen2 hooks without callers get evidence.
5. Web: real viewport resolution, OPFS settings/saves, audio latency, streams.

Known issues: 5 doubled phase B registrations (0x5B0440, 0x618E70, 0x84A540, 0x84E300, 0x84F320); 0x60A820 needs a
preserve-edx spec; `docs/samp/samp_usage.py` outputs and `catalog/progress.md` are stale against the catalogue;
stream volume -10000; web stream hang; ProcessMissionAudioEvent 0x4ECCF0 (all non-frontend PlayerPlaySound), phone
task, class-selection << >> Spawn panel, CEventDamage::AffectsPedGroup 0x4B38D0, burning-car path. The wasm build
needs wasm-bindgen 0.2.130 (`cargo binstall --root target/tools wasm-bindgen-cli@0.2.130`).
