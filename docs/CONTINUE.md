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

Next steps, in order:
1. INTEGRATE-8: bring WEB-3's tree (and the branches above not in it) onto master; verify-all 13/13; record promotions.
2. SA:MP: adapter steps 5b–12 in samp-rs (sa-engine/adapter), sa-samp write side (skin, anims, sounds, money,
   trains, proofs, bones for attached objects), unapplied RPCs (e.g. 60), audio streams (CAEDataStream 0x4DC620 +
   a Rust Vorbis decoder), the ~200 SA:MP-executed functions still unported (docs/samp/native-unported.csv),
   samp's main.scm (search-light render path, CHeli cone functions), class-selection panel, HUD clock toggle.
3. Native: burning/wrecked car path (0x4B1F10, 0x4DCBE0, 0x4B0DA0, 0x6375F0), remaining blockers from det runs.
4. Verification: the ~7,500 implemented-but-unverified hooks, SA:MP priority first; new recordings (driving,
   interiors, climbing, SA:MP sessions) so the gen2 hooks without callers get evidence.
5. Web: real viewport resolution, OPFS settings/saves, audio latency, streams.
