# Concurrent game lanes (INF-15)

Symptom (2026-11-05, once): lane 1 running `tools/e2e/inf3-hooks.sh` (real-time, determinism off) at the
same time as lane 2 running `inf4-determinism.sh` hung on the loading screen of a new game: the frame
counter stopped at 333 and `CTimer::Update` was no longer called. `inf3-hooks.sh` alone passed.

## Cause

A lost-wake-up race in the original game's streaming code, **not** a resource shared between lanes.
All addresses are gta_sa.exe 1.0 US. Threaded streaming (`0x8E3FE4` != 0):

```text
CdStreamSync (stub 0x406460 -> .HOODLUM 0x156CD80)   CdStreamThread (0x406560)
  0x156CD9D  if (s->nSectorsToRead) {                0x406669  s->nSectorsToRead = 0;
  0x156CDA4    s->bLocked = 1;                       0x40666C  if (s->bLocked)
  0x156CDAE    WaitForSingleObject(s->sema,          0x406678      ReleaseSemaphore(s->sema, 1, 0);
                                 INFINITE); }        0x40667E  s->bInUse = 0;
```

(`CdStream` = 0x30 bytes at `*0x8E3FFC`: +4 nSectorsToRead, +0xD bLocked, +0xE bInUse, +0x10 status,
+0x14 semaphore; request queue at 0x8E3FEC data / 0x8E3FF0 head / 0x8E3FF4 tail / 0x8E3FF8 size.)

If the streaming thread executes 0x406669..0x40666C after the main thread has read `nSectorsToRead != 0`
but before its `bLocked = 1` becomes visible (a preemption of the main thread inside that window, or
both stores still in the store buffers: x86-TSO allows the store→load reorder), the thread sees
`bLocked == 0` and does not release; the main thread then waits forever on a semaphore nobody will
signal. A new game hits it in `CTheScripts::Process` → `CStreaming::LoadScene` (0x40EB70) →
`CStreaming::LoadAllRequestedModels` (0x40EA10) → `CdStreamSync`, i.e. the loading screen.

Concurrency is only the trigger: other lanes (and anything else loading the CPUs: the samp-rs games, the
L1 verifier's Wine runs) make preemption of the main thread inside the window much more likely. The race
is in the original code, so a single lane can hit it too.

Determinism mode is immune: there `CdStreamRead` completes synchronously (determinism.md §4), so
`nSectorsToRead` is already 0 when `CdStreamSync` runs and the wait is never reached. That is why the
lane-2 inf4 runs never hung.

## Evidence

Harness (scratch, not in the repo): the global lock `target/locks/game.lock` held for the whole session;
lanes 1–3 run concurrently; a monitor samples `frames` every 3 s; a load loop per real-time lane boots
to the menu, starts a new game and waits for `gGameState == 9`; a load that does not reach it within
120 s is a HANG and is examined live (sa-hook `stack` five times 1 s apart, `peek` of the CdStream
globals, stream array and queue, `/proc/PID/task/*` states).

| run | lanes | loads | hangs |
|---|---|---|---|
| pair1 | 1 inf3 + 2 inf4 | 1 inf3 | 0 (inf4 PASS) |
| stress1 | 1 inf3 ×3 + 2 inf4 + 3 inf3 ×3 | 6 inf3 | 0 (inf4 PASS) |
| stress2 | 1, 3 load loop ×12 + 2 inf4 | 24 | 2 (lane 3 it 7, lane 1 it 10) |
| stress3 | 1, 3 load loop ×15 + 2 inf4 ×2 | 30 | 2 (lane 3 it 3, it 14) |
| baseline1 | 1 load loop ×20, alone (no other sa-engine lane) | 20 | 1 (it 7) |
| fixed1 (cdsync fix) | 1, 3 load loop ×20 + 2 inf4 ×2 | 40 | 0 (4 lost wake-ups recovered) |

Unfixed: 5 hangs in 74 new-game loads (about 7 %; 4/54 with three lanes, 1/20 alone). The single-lane
hang happened with no other sa-engine lane running (the samp-rs project's games were running on the
machine, as during all runs). So the original "inf3 alone passes" was luck; one inf3 run has a few-percent
chance to hang, more under load.

Every hang had the same shape:

- `frames hook=294 game=294 state=8 time=0` (loading screen; inf3's longer menu wait puts the same point
  at frame ~333, the number of the original report), unchanged across the five samples.
- Main thread (`stack`, identical five times): EIP 0x7BC8D2F0 in ntdll (wait), first game return address
  0x156CDB4 = the instruction after `call [WaitForSingleObject]` in CdStreamSync, then 0x40EAAC
  (LoadAllRequestedModels), 0x40EC89 (LoadScene), … 0x469FF7 (CRunningScript::Process), 0x46A220
  (CTheScripts::Process), 0x53BCCE (CGame::Initialise).
- Memory (stress3 it 3; it 14 identical in every field that matters):

  ```text
  peek 0x8e3fe4 01000000 01000000 50fa1500 00000000 00000000 06000000 50c31500 28010000 a4000000
       -> threaded streaming on, overlapped I/O on, queue head 0 == tail 0 (empty), 5 streams
  stream 0 @0x15c350: sector 0x29c5c  nSectorsToRead 0  bLocked 1  bInUse 0  status 0  sema 0x90
  stream 1 @0x15c380: nSectorsToRead 0  bLocked 0  bInUse 0  sema 0x94
  ```

  The read the main thread waits for is finished (`nSectorsToRead == 0`, `bInUse == 0`, nothing queued,
  status 0), `bLocked` is 1, and the main thread is still blocked on stream 0's semaphore: the thread
  checked `bLocked` before the main thread set it. That is the lost wake-up, and nothing else is waiting.
- All other game threads idle (`futex`/`ntsync` waits, no CPU growth over 5 s); no Wine errors in the
  lane's wine.log.

Shared resources checked and ruled out:

- Game directory: `wine/lane-N/gta-sa` entries are symlinks into `wine/gta-sa`. `inotifywait -r` on
  `wine/gta-sa` during stress2 (24 loads + inf4 runs on three lanes): **0 events**. The game writes its
  user files (`gta_sa.set`, `Gallery`, `User Tracks`) to `Documents/GTA San Andreas User Files` in the
  lane's own prefix (`e2e_isolate_userfiles` keeps that a real directory); sa-hook/launcher logs and
  `det/` are real files in the lane directory (`e2e_put`).
- DXVK: its pipeline cache is `AppData/Local/dxvk/*.dxvk.{bin,lut}` inside each prefix (per lane). Mesa's
  `~/.cache/mesa_shader_cache` is shared but designed for concurrent processes; the hung main thread is
  not in Vulkan/DXVK code and the DXVK threads were idle.
- Audio: one shared Pulse null sink `sa_engine_null` (a single module, no duplicates after three lanes
  started at the same second). The hang is not in audio code.
- Control ports 47801–47803 and the lane locks are per lane; separate gamescope instances and Xwayland
  displays; separate wineservers (one per prefix).
- Real-time waits: the hang is an INFINITE wait, not a timeout or a busy-wait (`Game::Idle`'s 14 ms
  wait and the frame limiter are not on the stack).

Side finding: `wine/prefix` (the base that new lanes are copied from) is written continuously while the
L1 verifier tests run `sa-oracle-native/ref.exe` in it (inotify: ~100k events in 11 min: `reg*.tmp`,
`dosdevices`). Existing lanes are not affected; a lane created during such a run copies a prefix in
the middle of a registry save. Create new lanes when no wineserver runs for `wine/prefix`.

## Fix

`crates/sa-hook/src/cdsync.rs`, installed at attach in every mode: only the `call [WaitForSingleObject]`
at 0x156CDAE is redirected (its disp32 points at a cell holding `cd_sync_wait`). The wait polls every
20 ms; when the stream is finished (`nSectorsToRead == 0 && bInUse == 0`) it ends. Because the thread's
release (if any) happens before its `bInUse = 0`, a zero-timeout wait first consumes a release that
raced with the poll, so the semaphore stays balanced. Game state is not touched; the only effect is
that the deadlock cannot happen. Every recovery is logged in sa-hook.log
(`cdsync: lost wake-up #N recovered …`) and counted by the control command `cdsync`
(`cdsync recovered=N slow=M`; `slow` = waits that outlived one poll interval). `SA_CDSYNC_FIX=0` keeps
the original wait (to reproduce the hang). This is a harness change to the original, like the
determinism patches; it changes no game function and is not a port.

`inf3-hooks.sh` now records `stack` and `cdsync` when the gameplay state is not reached, and `cdsync` at
the end of every run.

Verification of the fix:

- fixed1 (lanes 1 and 3: 20 new-game loads each, lane 2: inf4 twice, all at once): 0 hangs in 40 loads;
  sa-hook.log recorded 4 recoveries in 3 loads, each of which would have been a hang:
  `cdsync: lost wake-up #1 recovered (stream 0x15c380, frame 296, after 20 ms)` (lane 3 it 9),
  `#1 … 0x15c380, frame 295` and `#2 … 0x15c350, frame 295` (lane 1 it 16), `#1 … 0x15c380, frame 296`
  (lane 1 it 18). `slow` equalled `recovered` in every load: no legitimate read took longer than 20 ms,
  so the poll adds no wake-ups in normal operation. Both inf4 runs PASS (determinism unaffected).
- Acceptance (2026-11-05 10:42–11:12, game lock held, lanes started at the same second): three
  back-to-back iterations of lane 1 `inf3-hooks.sh` + lane 2 `inf4-determinism.sh`, then one iteration
  of lanes 1 and 3 `inf3-hooks.sh` + lane 2 `inf4-determinism.sh`: every script exit 0 (inf3 14/14 PASS,
  inf4 12/12 PASS in each of the 9 lane runs; `cdsync recovered=0` in these runs).

## Rule for lanes

PLAN §8 (proposal): several game lanes may run at once (lanes 1–3), each under its own
`target/locks/lane-N.lock`; the global `game.lock` is no longer required (inf8-l2.sh still takes it and
can drop it). Requirements: sa-hook with the cdsync fix (any real-time lane without it can hang on a
loading screen, alone or not), ≥ 20 GB free, the memory cap per lane.
