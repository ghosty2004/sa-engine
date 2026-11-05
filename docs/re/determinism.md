# Determinism of gta_sa.exe 1.0 US under Wine (INF-4/5/6)

Determinism mode (`SA_DET=1`, implemented in `crates/sa-hook/src/{oracle,detwin,detaudio,arena}.rs`) makes
a run of the original game a pure function of its start conditions and its per-tick pad input. This file
lists every nondeterminism source found, how it was found, how it is neutralised and what is still open.
All addresses are gta_sa.exe 1.0 US. Measurements were made on lane 2 (Wine + DXVK, headless gamescope,
Pulse null sink) with `tools/e2e/inf4-determinism.sh` and the `SA_DET_RANDLOG`/`SA_DET_HEAPLOG` logs.

Method: static IAT cross-references of every time/thread/heap import (capstone over the exe, mapped to
`catalog/procedures.csv`), then record → replay runs from process start whose per-frame state hashes
(INF-6) and per-call rand logs were diffed. A source counts as neutralised only when a replay is
hash-identical to the recording over the whole run.

## 1. Time

Imports and their game call sites (`FF 15 [IAT]` / `FF 25` thunks; thunks at 0x81E4xx and 0x8174F6 have no
callers):

| import | site | function | effect | handling |
|---|---|---|---|---|
| QueryPerformanceCounter | 0x5617A8 | 0x5617A0 (= `CTimer::ms_fnTimerFunction`) | frame delta in `CTimer::Update` (0x561B10) | virtual |
| QueryPerformanceCounter | 0x561A88 | `CTimer::GetCurrentTimeInCycles` | CGame::Process (streaming time decides `CPopulation::Update(bool)`), LoadMonitor, Idle wait, WinMain limiter, `CPostEffects::Grain` srand, texture-load stats 0x730E60 | virtual |
| QueryPerformanceCounter | 0x5900FF | `CLoadingScreen::LoadSplashes` | `srand(QPC.LowPart)` | virtual |
| QueryPerformanceCounter | 0x4D9E88 | `CAEAudioUtility::GetCurrentTimeInMS` | audio stream/fade timing | virtual (§4) |
| QueryPerformanceCounter, GetTickCount, GetSystemTimeAsFileTime | 0x8339CA | CRT security-cookie init | stack cookie only, no game state | left real |
| QueryPerformanceFrequency | 0x56186C, 0x5B9868 | CTimer::Initialise, CAEAudioUtility::StaticInitialise | divider | real (Wine: always 10,000,000; recorded in the recording header `det.qpf`) |
| timeGetTime | 0x74528C | `psTimer` ← `RsTimer` 0x619410 | `GameInit` `srand(RsTimer())` (0x5BF3B0), loading-screen splash timing (0x590xxx) | virtual |
| GetSystemTimeAsFileTime | 0x82410F | CRT `time()` | `CPostEffects::Grain` reseeds | virtual |
| GetLocalTime | 0x5D1E54, 0x823A9B | save-game timestamp, CRT helper (stats file) | save contents | virtual |

Virtual clock: the main thread's QueryPerformanceCounter is `NOW`, which starts at `SA_DET_SEED` (default
10^12 ticks; this is the determinism seed, because GameInit seeds rand from it) and advances by exactly one
step (`SA_DET_STEP_US`, default 40,000 µs, i.e. `ms_fTimeStep` = 2.0) at every frame boundary (entry of
`CTimer::Update`). Inside a frame time does not move, so every "how long did this take" measurement in
`CGame::Process` reads 0. Calls from other threads get real time (counted as `offthread_time` in `det`;
measured 0 in the acceptance run once the audio threads are stepped, §4). timeGetTime and the wall clock
(FILETIME epoch 2005-06-07 UTC + NOW) are derived from NOW.

Only the game's call sites are patched (displacement of `call [IAT]` redirected to a pointer cell in
sa-hook), never the IAT, so Wine, DXVK and the CRT start-up keep real time.

Busy-wait loops that would spin forever on a frozen clock (found by the `stack` control command on the
first hang at frame 202):

- WinMain frame limiter, `GAME_STATE_IDLE` (0x748D1F: `elapsed = GetCurrentTimeInCycles()/div`, idle only
  when `1000/RsGlobal.frameLimit < elapsed`): the call is redirected to a function returning 0x7FFFFFFF,
  so the limiter never waits.
- `Game::Idle` (0x53E920) starts with a 14 ms busy wait that gta-reversed comments out but 1.0 US has:
  `do t = GetCurrentTimeInCycles()/div while (t - g_LastTickTime < 14)` (unsigned, 0x53E930..0x53E94D),
  then `g_LastTickTime = GetCurrentTimeInCycles()/div` (0x53E94F). The loop read returns 0x7FFFFFFF and the
  post-loop read returns 0, so `g_LastTickTime` (0xB72CA8, read and written only here, catalogue rwsets) is
  0 and the wait ends at once every frame.

Both only pace real time; the game logic sees the fixed step through `CTimer`.

## 2. Randomness

The game uses the CRT `rand`/`srand` of its static VC7.1 multithreaded CRT: `rand` at 0x821B1E (found by the
MSVC LCG constants 214013 at 0x821B28 and 2531011 at 0x821B2E; the only other hit, 0x115D193, is inside the
protection junk region), `srand` at 0x821B11. State is per thread (`_getptd()` 0x827B3D, `_holdrand` at
ptd+0x14), so other threads cannot disturb the main thread's sequence. `CGeneral::GetRandomNumber` calls it.

Both are hooked (caller = return address). Main-thread calls feed the `randcalls` hash (FNV over kind,
caller, value) and optionally `SA_DET_RANDLOG`; other threads' calls are counted per (thread, caller):
measured **0** other-thread rand/srand calls in every run (`rand_other=[]`). Main thread: about 1.4 M rand and
30 k srand calls in 2,500 frames.

srand seeds (all main thread): `GameInit` (RsTimer → virtual), `CLoadingScreen::LoadSplashes` (QPC → virtual),
`CPostEffects::Grain` (time → virtual), `CEntity::CreateEffects` 0x533790 (**`this` + game days → heap
address, §3**), `CCarCtrl::PickNextNodeToChaseCar` / `FindLinksToGoWithTheseNodes` / `PickNextNodeRandomly`,
`Interior_c::Init`, `ProcSurfaceInfo_c::AddObjects`, `CGrassRenderer::DrawTriPlants`,
`CPPTriPlantBuffer::Flush` (game-state seeds).

## 3. Heap

Measured (first gameplay frame, pool storage addresses logged by sa-hook): run A ped pool header 0x396C88,
building pool storage 0x8C50020; run B 0x396BE0 and 0x8E00020. Wine's heap placement differs per run (large
blocks are VirtualAlloc'ed into an address space that Wine/DXVK/other threads also use). This is not cosmetic:
`CEntity::CreateEffects` seeds `srand` with the entity pointer (rand log: `srand(147666233)` from 0x5339F5,
i.e. an address in the building pool), so the rand sequence diverged at frame 202.

Neutralised by a deterministic allocator (`arena.rs`): the CRT heap primitives `__heap_alloc` 0x8241B0
(malloc, `_nh_malloc`, operator new end here), `free` 0x82413F, `realloc` 0x824269, `calloc` 0x824416,
`_msize` 0x828C4A are hooked; main-thread requests are served from a region reserved at the fixed address
0x20000000 (768 MB; candidates 0x30000000, 0x18000000 if taken), with a policy that depends only on the
request sequence (16-byte size classes ≤ 2 KB with LIFO lists, best fit with coalescing above, bump
pointer). Frees/reallocs are routed by address, so CRT-heap blocks keep going to the CRT heap.

A second measurement (`SA_DET_HEAPLOG`) showed the first differing request was the CRT start-up copy of the
environment block (8,822 vs 8,746 bytes) and one `_setenvp` string per variable: the environment differs
between runs (SA_DET_PROGRAM vs SA_DET_REPLAY, gamescope's DISPLAY). The arena is therefore activated only
when mainCRTStartup calls `_cinit` (call at 0x8246AC redirected through a shim): the I/O tables, environment
and argv stay in the CRT heap, everything from the static constructors on is deterministic. After this the
main thread's allocation sequence (seq, size, address) was identical between runs.

Counters in `det`: `xthread` (arena blocks freed or reallocated by another thread; would perturb the arena:
measured 0), `other_allocs` (other threads' CRT allocations; measured 4–12, all served by the CRT heap and
never seen by game logic), `oom` (arena exhausted: 0; high water about 40 MB in 2,500 frames).

Not covered: memory the game gets from `LocalAlloc` (CdStream structures 0x406C22/0x40693F, 0x820C90) and
from DirectX/DXVK. Their addresses are not used as game logic input as far as the traces show.

## 4. Threads

`CreateThread` sites: `CdStreamInitThread` 0x4069C0, `CAESmoothFadeThread::Initialise` 0x4EEEE9,
`CAEStreamThread::Initialise` 0x4F16E6, `CAEUserRadioTrackManager::ScanUserTracks` 0x4F4BCB.

- **CdStream** (0x406560): completion order and timing of model reads decided in which frame a model
  became loaded. `CdStreamRead` (stub 0x406A20 → .HOODLUM 0x156C2C0) is replaced in determinism mode by a
  synchronous read on the calling thread that leaves the stream exactly as the thread leaves it
  (`nSectorsToRead = 0`, `bInUse = 0`, status 0 or 0xFE, `gLastCdStreamPosn = sectors + pos`, overlapped
  offset at +0x24; offsets read from 0x156C2C0 and 0x406560). The streaming thread then never gets work.
  Callers: CStreaming (RequestModelStream, RetryLoadFile) and `CAEMP3BankLoader::Service` (sound banks).
  Real-time runs keep the thread; its lost-wake-up race with `CdStreamSync` (a loading-screen hang) is
  fixed by `cdsync.rs`, see lanes.md (INF-15). With synchronous reads the race cannot occur.
- **Audio** (measured feedback): with time, rand, heap and CdStream fixed, a replay still diverged in some
  runs at frame 204/205 because `CAEWeatherAudioEntity::UpdateParameters` (0x506647) called
  `CAEAudioUtility::ResolveProbability` (rand) one frame earlier; its call chain contains
  `CAEStaticChannel::GetPlayTime` (0x4F0F78), i.e. the DirectSound play cursor. Neutralised (`detaudio.rs`):
  - `CAEStreamThread` and `CAESmoothFadeThread` are created suspended; their `Start` (0x4F11F0, 0x4EEA10)
    `call [ResumeThread]` sites (0x4F11F7, 0x4EEA1A) go to shims that record `this` and do not resume. Their
    loop bodies run on the main thread at every `CAEAudioHardware::Service` call (0x4D9870, hooked; stream:
    `Service` 0x4F1290 then the streaming channel's vtable `Service`; fade: `Service` 0x4EED10), while their
    active flags (+0x11 and +0x809, read from the loops at 0x4F15C7 and 0x4EEE95) are set. First attempt
    stepped them at the frame boundary instead: the game hung at frame 252 in a wait loop (0x4DC0E0, reached
    from `CCutsceneMgr::LoadCutsceneData_preload` → `CAudioEngine::PreloadCutsceneTrack(id, wait=true)`)
    that spins on `CAEAudioHardware::Service` until the stream thread has prepared the track. Stepping inside
    hardware Service keeps such waits finite and their iteration count deterministic.
  - `CAEAudioUtility::GetCurrentTimeInMS`' QPC (0x4D9E88) is virtual (the bodies now run on the main thread).
  - DirectSound secondary buffers report a virtual play state: the shared secondary-buffer vtable (found
    by creating a throw-away buffer when `_cinit` runs, before the game creates any) has GetCurrentPosition,
    GetStatus, Play, Stop, SetCurrentPosition, SetFrequency and Release wrapped. The position advances
    with the virtual clock at frequency × block align bytes per second; a non-looping buffer stops at its end
    (position 0). The real methods are still called, so sound is still produced; only what the game reads
    back is virtual.
- **User radio scan** (0x4F4BCB): only when the user scans user tracks (menu); not reached by the corpus.
  Not neutralised.

## 5. Input

Window messages and DirectInput are real, but in a replay every input the game consumes is injected per
input tick (`CPad::UpdatePads` pass): `TempKeyState` before `CPad::UpdateMouse`, `NewMouseControllerState`
after it, `PCTempKeyState/JoyState/MouseState` of `Pads[0]` before `CPad::Update(0)` (see
`crates/sa-oracle-format/src/pad.rs`). Menu navigation and the logo/intro key presses are part of the
input, so a recording starts at process start.

Not injected: Pads[1] temps (a second player's joystick), the absolute cursor position the frontend reads
from Windows (`FrontEndMenuManager` mouse position; the headless cursor does not move, menu navigation in
the corpus is keyboard only).

## 6. Evidence (acceptance run, lane 2, 2026-11-05)

`tools/e2e/inf4-determinism.sh` with `tools/e2e/programs/grove-walk.txt` (boot, menu, new game, intro
cutscene skipped, then walk/sprint/turn around Grove Street; 5,200 frames = 208 s of virtual time, of which
frames 1,500–5,200 = 148 s are walking). Every run starts at process start; summary:

```
PASS record run reached the end frame: 5200 frames, last 5200
PASS recording written: ticks=5197 first_frame=0 last_frame=5199 input_changes=915 bytes=747960
PASS replay1 / replay2 reached the end frame, input in sync (desyncs=0, rand_other=[], offthread_time=0)
PASS replay1 vs replay2 identical for all 5200 frames: IDENTICAL over 5200 common frames
PASS record vs replay1 identical for all 5200 frames: IDENTICAL over 5200 common frames
PASS record vs replay2 identical for all 5200 frames: IDENTICAL over 5200 common frames
PASS 1-ulp player position change at frame 3700 detected exactly there in peds: DIVERGED at frame 3700: peds
     (perturbed pos.x 0x450ed582 (2285.3442) -> 0x450ed583); afterwards camera 3701, vehicles 3833,
     objects 3936, streaming 3945; timer, rand, scripts, player, pad stay identical
```

All 17 subsystem hashes, including `heapaddr` (raw pool and matrix addresses), are identical between
record and replays. A separate run with `SA_DET_RANDLOG` produced three byte-identical 53 MB rand logs
(1.94 M rand + 34 k srand calls with caller and value). Before each fix the same comparison failed:
frozen clock → hang at frame 202 (Idle busy-wait); Wine heap → divergence at frame 202 (`rand`, from
`srand(entity pointer)`); environment-sized start-up allocations → divergence at frame 202; audio cursor
→ divergence at frame 204/205 in some runs.

## 7. What is still open (not neutralised or not covered)

- **Pacing differs from a free-running original by design.** Determinism mode replaces real time: frame
  step fixed (default 40 ms), WinMain limiter and Idle wait never wait, audio threads advance only when
  the main thread services audio, DirectSound positions follow the virtual clock (model: a non-looping
  buffer stops at its end with position 0). Both the original and the Rust engine are compared under the
  same model; results obtained in determinism mode say nothing about real-time-dependent behaviour.
- **User radio track scan thread** (0x4F4BCB, `CAEUserRadioTrackManager::ScanUserTracks`): not neutralised;
  only runs when the user scans user tracks.
- **Window messages / focus**: not virtualised. Losing focus pauses the game (`IsForegroundApp`) at a
  real-time moment. Lanes are headless gamescope displays where focus does not change; a recording made
  on a desktop could be disturbed by focus changes.
- **Input not injected**: `Pads[1]` temps (second player), the frontend's absolute cursor position.
- **Heap**: only the main thread's CRT allocations from `_cinit` on are deterministic. `LocalAlloc`,
  DirectX/DXVK and other threads' allocations are not (measured `other_allocs` 0–12 per run, `xthread` 0).
  An arena block freed by another thread would perturb the arena; it is counted (`xthread`), never seen.
- **Hash coverage**: the hashes cover what INF-6 lists (pools with transforms and velocities, world
  lists, streaming, script space and running scripts, camera, clock/weather, rand, player info, pads). Not
  hashed yet: ped/vehicle internals beyond transform and velocity (health, tasks, damage), audio entity
  state, menu state, particle/FX state. A divergence there shows up only when it reaches hashed state.
- **Environment assumptions**: measured on one machine with one Wine/DXVK version; QueryPerformanceFrequency
  is taken from the system (10 MHz under Wine) and recorded in the recording header (`det.qpf`), as are the
  step, clock base, exe/data/settings fingerprints and the program hash.

