# ADR-005: the pure build (sa-game without gta_sa.exe)

Status: **accepted** (2026-11-18, user; proposed the same day). ROADMAP task: PURE-1. Related: ADR-001 (globals, `orig!`, `#[hook]`), ADR-003 §2.1
(build flavours, pointer compression), ADR-004 (mods; `sa-access` / `sa-types` own the outside access to game state),
STD-3 (64-bit), PLAN F10 / F11, `docs/arch/platform.md` §6 and §8. The measurements behind every number here are in
`docs/arch/pure-build-census.md` (tools in `tools/pure/`).

## 1. Goal and non-goals

**Goal.** Native `sa-game` (i686 Linux) and the web build run with the game's own files only (`models/gta3.img`,
`data/*.dat|ide|ipl|zon`, `data/script/main.scm`, `audio/`, `text/`, `anim/`, ...) and **no `gta_sa.exe`**: nothing is
read from it at start-up or at run time. The game logic stays the verified 1:1 port; its results do not change.

User decisions (2026-11-18): data tables, constants and initial values that come from the exe today are **included in
the Rust code** (not extracted from the user's exe at run time or install time); a 64-bit build is a follow-up, not
part of the first pure build.

**Non-goals.** No change to port bodies beyond replacing code-byte reads with their constant values (§3.4); no
64-bit build (§3.9 lists what it needs); no new renderer or audio behaviour; phase A (the ports inside the original
exe under Wine, verify-all) keeps working unchanged and keeps needing the exe, which is its oracle.

## 2. What the exe provides today

`sa-game` reads `<game>/gta_sa.exe` at start-up (`arena::map_image`, SHA-256 checked), maps `.rdata`, `.data` + `.bss`,
`_TEXT_HA` / `_rwdseg` / `.rsrc` at 0x858000–0xCB1000 with their initial contents, writes import trap addresses into the
IAT cells, maps game `.text` and `.HOODLUM` read-only (never executable) for the no-CD protection reads, and keeps a copy
of `.text` to resolve the `orig!` `site` forms. Measured (census §1–§3):

- the sources hold **61,252** image addresses in game crates; **27,695** of them (11,672 distinct) are data addresses
  read or written by address (`g`, `gf`, `p8`, `GPtr::from_addr`, named constants, `globals!`, tables); the rest are
  hook and `orig!` addresses, generator provenance tables and caller-attribution sites, which are only numbers;
- at run time the ports need **31,430** `.rdata` and **53,844** `.data` bytes of the initial image (first access a
  read or a read-modify-write; 16 FRONTIER scenarios, 27,000 iterations); widened to whole objects and joined with the
  static set (everything the original's game code references, plus what those objects' pointers reach) that is
  **272,705 bytes** of objects (189,423 non-zero), of which **227,077 bytes (156,349 non-zero) are unique to the exe**
  (strings, tables, initial values, float constants); the rest (IAT cells,
  vtables and function tables, pointer tables) is regenerated from the catalogue. The whole initialised image is
  0.65 MB (0.46 MB non-zero);
- that needed set is **not closed**: an exe-filled arena cut down to it (everything else zeroed) gives identical
  reports on the scenarios it was measured on, but 8 of 9 hold-out scenarios diverge (plane, nitro, monster truck,
  escalator) or crash (quad, jet pack, shotgun, road blocks); only the jacked-driver run stays identical. Each new
  vehicle type, weapon or feature reaches tables no earlier run touched (census §2);
- **66 dwords of code** are read as data: switch byte tables MSVC placed in `.text` and the no-CD protection's
  `.HOODLUM` dwords, all static values;
- 70 `site` declarations (67 call sites) are resolved by reading `call [cell]` in `.text`.

Nothing in game data files duplicates the exe's tables (handling, weapons, car colours and the like are in `data/`
already and are not in the exe); what is derivable is structural: import names, vtable and function-table contents
(code addresses, i.e. registry keys), pointer tables (addresses of other carried objects), and the float constants the
compiler put in `.rdata` (literals in the source).

## 3. Decision

### 3.1 The arena stays as the engine's memory; its contents come from Rust

The legacy address arena is kept as the memory model: one region holding the game's globals and tables at their
original offsets, game pointers 32 bits wide, objects in the original layouts. It is already the engine context in
everything but name: sa-game owns it, ports reach it through `sa_core::mem` (`GPtr`, `Global`), and the wasm32 build
and the planned 64-bit build (pointer compression, ADR-003 §2.1) use the same model. What changes is where its initial
contents come from: a generated, typed **built-in image** in the Rust code instead of the user's exe.

Port code does not change. `globals!` keeps aliasing the arena on 32-bit phase B (one copy), and the 1,260
`globals!` declarations keep their initial values, which the image must equal (the existing `native::check_globals`
test, now against the built-in image).

Not chosen for the first pure build: moving every global out of the arena into its own Rust static or a context
struct field (ADR-001's option B). It would rewrite 27,695 data-address uses and 4,700–6,500 host-pointer
conversions, the raw accesses alias neighbouring globals and struct arrays at fixed distances, the frame hashes, save
blocks, `sa-access` and the SA:MP adapter read the original offsets, and pointer compression needs those offsets
anyway. It buys nothing the image does not.

### 3.2 The built-in image (`sa-image`)

A new leaf crate `sa-image` (no engine dependency, used only by sa-game's arena code, `publish = false`) holds the
initial contents of the arena, generated by `tools/gen/image_rs.py` from the exe and the catalogue (ADR-003 §3.5: one
generator, shared `tools/lib/exe.py`, `--check` in the writer gate; generated output never edited by hand):

- **the whole initialised image** of the arena's sections (`.rdata`, the initialised `.data`, `_TEXT_HA`, `_rwdseg`,
  `.rsrc`: 641,024 raw bytes, 455,453 of them non-zero), split into objects at the known starts (census §2) and stored
  as entries `(address, data)` sorted by address, zero runs left out. The first pure build is then byte-for-byte the
  arena of today, by construction; the measured needed set (0.27 MB) is not closed (§2) and is no safe cut. Dropping
  objects comes later, per subsystem, with evidence (§3.6);
- **typed where the kind is certain**: C strings as byte-string literals, `f32` / `f64` constants as float literals,
  dword tables as `u32` arrays, pointer tables and vtables as arrays of named addresses (`vt::CPed` = the original
  vtable address, slot entries = catalogue function addresses, which are the registry keys), everything else as
  bytes with the catalogue name as comment;
- the **import table** and the **call-site table** (§3.3) in their own modules. The code constants of §3.4 do not go
  here: they belong to the ports that read them.

As built (PURE-1b, 2026-11-18): the bytes are one committed blob, `crates/sa-image/image.bin` (566,667 bytes: 2,243
runs at their addresses, every zero stretch of 16 bytes or more left out) behind `include_bytes!`, with the runs, the
import table and the call-site table as generated Rust tables (`src/generated.rs`, 56 KB) and small accessors. The
typing above is not done in the image itself: the blob compiles in well under a second and keeps the byte-for-byte
equality trivial, and typed declarations belong to the owners the objects move to in PURE-2 (§3.6), not to a second
copy here. The image source without `--image` is `exe` when the game folder has gta_sa.exe and `builtin` otherwise;
the web build is always `builtin`.

`arena::map` gets an image source: `builtin` (default once PURE-1e lands), `exe` (today's path, kept for checks and for
anyone who wants the original bytes), `check` (both: fills from the built-in image, compares it byte for byte with the
exe's sections over the whole initialised range and reports every non-zero exe byte the image lacks, by object and
catalogue name). The arena outside the image's objects stays zero, as `.bss` does.

### 3.3 Imports and call sites without the exe

The IAT cell → `DLL!name` list (214 imports, `catalog/imports.csv` plus the cell addresses) and the 67 `site` / 2
`callsite` call-site → cell / target entries become generated tables in `sa-image`. `arena::resolve_site` and the IAT
trap writer read them; the `.text` copy goes.

### 3.4 Code bytes read as data become constants

The 66 code dwords (census §3) are replaced in the ports: the switch byte tables (`CCarCtrl` missions 0x4382E4,
0x4B3F3C, 0x4E1C7C, 0x4F4F48, 0x4F7998, 0x501640, 0x4DADB4, 0x4DAE2C, ...) become `static` tables in the owning crate,
named after the table, with a unit test against the exe bytes; the protection-derived values (`~[0x1560910] ^
[0x4D4314]`, `[0x1560628] + [0x563470]`, `~[0x15607EC] ^ [0x727253]` and the 33 literal protection-read targets in 49
files) become constants with the derivation in the doc comment, as `PROT_GO_FORWARD` already is. The values are the
same in phase A, so the ports' results there are unchanged; the touched hooks go through L1 / L2 as usual. After this,
`--image builtin` maps no code section at all.

### 3.5 Verification during the transition (dual mode)

- **Phase A is untouched.** `sa-image` is not linked into phase A; the ports keep their addresses and layouts. Every
  step keeps verify-all IDENTICAL 13/13, which the steps' acceptance requires.
- **Phase B equivalence.** The FRONTIER scenarios (`tools/e2e/scenarios/frontier.sh`, deterministic `--fixed-step 40`)
  run with `--image exe` and `--image builtin`; the reports (every function's registry dispatch count, platform calls,
  blocked calls, boot results) must be identical apart from the frame-pacing line. The trace tool already does this
  test without code changes: `SA_IMAGE_KEEP=FILE` in `tools/pure/arena_trace.c` zeroes everything outside a set of
  objects in an exe-filled arena (census §2 lists the results).
- **Completeness check.** `--image check` compares the built-in arena with the exe-filled one byte for byte at
  start-up and names every differing object. In PURE-2 a candidate drop is first run as `SA_IMAGE_KEEP` (or
  `--image check` with the object left out) over every FRONTIER scenario; a difference keeps the object.
- **Wine stays the oracle** for behaviour: the corpus recordings and L1–L3 run in phase A with the original exe as
  before; the pure build changes only where phase B's initial memory comes from.

### 3.6 Ownership afterwards (owned statics, step by step)

Once the exe is gone, the generated residue moves to its owners subsystem by subsystem (ADR-003 §3.2: one definition,
in the owning crate): state with initial values to `globals!` declarations, constant tables and strings to typed
`static`s or `rdata!`-style declarations next to the ports that read them, compiler float constants to literals in
the ports (`gf(0x858624)` → `1.0`). Each moved object leaves `sa-image`; the residue size is the progress metric
(census §2 has the referenced bytes per subsystem: AUDIO 52.5 KB, RW 16.6 KB, HUD 15.4 KB, RENDER 10.9 KB, CORE 10.5 KB,
then STREAM, WORLD, VEH, ... and 66.1 KB reached only through pointers). The arena keeps the objects' original
addresses throughout; only their declarations move.

Objects nothing can reach (CRT, D3DX, libpng / zlib and protection data, unreferenced strings) are dropped in the same
pass, one at a time and only with evidence: no reference by game code in the static analysis, not inside a table that
is indexed or reached through a pointer, and every FRONTIER scenario plus `--image check` unchanged without it. The
hold-out result of §2 is the reason for this caution.

### 3.7 Third-party code and the functions that stay original

None of them needs the exe's bytes; they need native implementations where a path reaches them:

- **CRT**: ~60 native replacements exist (`crt.rs`); new reaches are added there.
- **D3DX9**: `D3DXAssembleShader` is native; the five matrix / vector helpers of RW's vertex-shader path (0x7672E1,
  0x7676C7, 0x7675EE, 0x766AB7, 0x766A32) get native versions when the shader path is enabled (`SA_GAME_SHADERS=1`).
- **libjpeg 6b** (gallery photos): a pure-Rust JPEG encoder behind the existing `jpeg_*` declarations (output bytes are
  not compared; the photo file is the player's).
- **libpng / zlib**: not reached.
- **Audio classes that stay original**: DirectSound is native (`dsound.rs`), Ogg Vorbis through `lewton`
  (`vorbisfile.rs`); user tracks (`CAEWaveDecoder`, `CAEMFDecoder`, `CAEWMADecoder`, QuickTime) get a pure-Rust decoder
  (WAV / MP3; WMA unsupported, skipped like a broken file) when user tracks are enabled.
- **C++ exception classes, the bug-report writer, the Winsock class**: unreachable or development tooling; stay
  unported with their loud `unported` panic.

As built (PURE-1f, 2026-11-18): sa-game registers the third-party entry points at their original addresses, never the
libraries' internals. `libjpeg.rs`: the seven libjpeg calls of `JPegCompressScreen` over `jpeg-encoder`, writing through
the game's destination manager (its `empty_output_buffer` 0x5D0400 is now ported in sa-render). `libpng.rs`: the eleven
libpng calls of `RtPNGImageWrite` and `png_error` over the `png` crate, the file passed to the game's
`_rtPNGWriteData` (0x7CF980, ported in sa-rwcore); errors no longer jump anywhere (ROADMAP divergence ledger).
`d3dx_math.rs`: the five D3DX helpers. `mfdecoder.rs`: the media library start-up succeeds and `CAEMFDecoder`'s
QuickTime-backed methods decode MP3 with `symphonia` (its constructor and `GetStreamID` are ported in sa-audio); WMA
stays unavailable (no pure-Rust decoder). `CAEWaveDecoder` is game code (a RIFF reader over `CAEDataStream`, no
library), so it is a port for PORT-AUDIO, not a replacement. User tracks still need the scan (`WriteUserTracksThread`
0x4F4A20, not ported; `ScanUserTracks` starts no thread in phase B) to build `sa-utrax.dat`. The gallery photo stops
before libjpeg: `psGrabScreen` (0x7452B0) reads the front buffer through `IDirect3DDevice9::GetDisplayMode`,
`CreateOffscreenPlainSurface`, `GetFrontBufferData` and the surface's `LockRect`, which the native D3D9 device does
not implement yet (a read-back of the presented frame; platform work, not third-party code).

### 3.8 Web

The page stops asking for `gta_sa.exe` (`web/main.js`, `web/loader.js`, `platform/web.rs`); the folder import copies
only the game data. The arena code is shared, so the built-in image fills the reserved low linear memory the same way;
the module grows by the image (0.46 MB of non-zero data, against 34.7 MB today) and the page no longer copies the
14 MB exe into the browser's storage. Nothing else changes.

### 3.9 Layouts, pointers and saves

What must stay as it is, during the transition and after:

- **the original layouts and addresses in phase A** (the ports run inside the exe): objects, pools (`CPool` headers,
  byte maps, 4-byte pointers), the 536 original vtable addresses the ports store into objects and compare against (1,420 places), global addresses,
  calling conventions, stack footprints;
- **the same layouts in phase B**: the corpus hashes, `sa-access` / the SA:MP adapter and the mods read original offsets,
  and the save blocks (SAVE-1) are written from these layouts, so save files stay interchangeable with the original's;
- **the heap below 0x3F800000** (`detheap`, platform.md §23): stale pointers are read as floats.

What a 64-bit build (STD-3, follow-up) needs on top of the pure build: `GPtr` as a 32-bit offset from the arena base
(one place, `sa_core::mem`); the 4,700–6,500 host-pointer conversions (`GPtr::from_raw` 3,520, `ptr_of` under its
import names 1,071–2,930, `vec3f_ptr` / `vec_ptr` / `buf_ptr` 79) moved onto an arena-resident scratch stack; registry dispatch through the
type-erased path wasm32 already uses (`sa_core::wabi`) instead of the x86 conventions; the heap inside the arena. Layouts
and saves stay 32-bit, so the corpus replays with the same hashes.

## 4. Step plan

Every step keeps verify-all IDENTICAL 13/13 and the writer gate green; every phase B step keeps the FRONTIER scenario
reports identical between `--image exe` and the new path.

| step | what | acceptance | estimate | basis |
|---|---|---|---|---|
| PURE-1a | census and tooling (this ADR, `tools/pure/`) | done | done | |
| PURE-1b | `tools/gen/image_rs.py` + `sa-image` (the whole initialised image, typed where provable); `arena::map` with `exe` / `builtin` / `check`; native `check_globals` against the built-in image | `--image check`: the built-in arena equals the exe-filled one byte for byte; FRONTIER reports identical exe vs builtin; generator `--check` in the gate | 2–3 agent-days | 641,024 bytes / ~61,500 objects (strings, floats, dword and pointer tables, bytes); the generator reuses `tools/lib/exe.py` and the object segmentation of `arena_usage.py`; a scenario pass takes ~10 min |
| PURE-1c | generated import table and call-site table; no `.text` copy | reports identical; `--image builtin` keeps no exe bytes in memory | 0.5 day | 214 imports, 69 sites |
| PURE-1d | code bytes as data → constants / tables in the ports; no code section mapped | verify-all IDENTICAL; L1 / L2 of the touched hooks; reports identical | 1–1.5 days | 66 dwords, ~10 switch tables, 33 protection targets in 49 files |
| PURE-1e | `gta_sa.exe` optional: native and web start without it (the exe only for `--image exe|check`); README, platform.md §6 / §8, web README, CLAUDE.md rule on exe-derived tables | a game folder without `gta_sa.exe` runs every FRONTIER scenario natively with the same reports; the web page loads a folder without it | 0.5–1 day | |
| **first pure build** | PURE-1b..e | F10's "no gta_sa.exe dependency" part | **4–6 agent-days** | |
| PURE-1f | third-party paths of §3.7 (D3DX helpers, JPEG photos, user-track decoders) | the gallery photo and a user track work natively; shader path with `SA_GAME_SHADERS=1` | 2–3 agent-days | 5 D3DX helpers, 11 libjpeg calls, 3 decoder classes |
| PURE-2 | ownership of the residue, subsystem by subsystem, and dropping what nothing reaches (§3.6) | per subsystem: its objects leave `sa-image`, verify-all IDENTICAL, FRONTIER reports identical, `--image check` clean | 8–12 agent-days | 0.23 MB referenced unique data over 22 subsystems (AUDIO, RW, HUD, RENDER, CORE are half of it, 66 KB is reached only through pointers); the rest of the 0.64 MB image is dropped only with evidence |
| STD-3 | 64-bit (§3.9) | x86_64 replays the FRONTIER scenarios with identical reports | 2–4 agent-weeks | 4,700–6,500 host-pointer sites, `GPtr` representation, dispatch |

## 5. Risks

1. **Cutting the image too early.** The image is complete by construction in PURE-1b; the risk moves to PURE-2's
   dropping of objects. Data reached through computed addresses or pointer tables on paths no scenario takes would read
   as zero: the needed set measured on 16 scenarios failed 8 of 9 hold-out scenarios. Mitigations: drop only with the
   evidence of §3.6, keep `--image check` and the trace tool in the scenario runs, and treat the residue that cannot be
   proven unreachable as permanent data (it costs nothing at run time).
2. **Wrong object boundaries** when typing (a struct of floats and pointers read as a string or a table). The
   generator types only kinds it can prove and emits bytes otherwise; the byte-for-byte `check` mode catches any
   difference.
3. **Values computed at run time from protection state.** The protection-read targets the ports use are static (only
   these reads reference their cells); a value that depends on run-time state would make its constant wrong. Each one is
   checked against phase A by L1 / L2 when its port is touched in PURE-1d.
4. **Source size and build cost.** 0.64 MB of data (0.46 MB non-zero) becomes roughly 3–4 MB of generated Rust in one
   leaf crate; compiled once, it does not affect incremental builds of the ports. Memory during `cargo build` stays within CARGO_BUILD_JOBS=4.
5. **Repository status.** The repository then holds data derived from the exe (§6).
6. **Exe drift.** The generator is pinned to the 1.0 US SHA-256, as sa-game's arena is now.

## 6. Legal note

The image tables are copied from gta_sa.exe 1.0 US into the source tree, by the user's decision. The project is
private; the repository must stay private while it holds them. The rule "never commit Rockstar files" (CLAUDE.md)
then reads: the exe and the game's assets are never committed; tables generated from the exe's data sections live only
in `sa-image` (and the code constants of §3.4), generated by `tools/gen/image_rs.py`, so they can be removed or
regenerated in one step. Should the code ever be published, `sa-image` is replaced by the `exe` image source (today's
start-up read of the user's own exe), which stays supported.

## 7. Alternatives considered

- **Keep reading the user's exe at start-up** (today): excluded by F10 and the user's decision.
- **Extract the tables from the user's exe once, into a cache file**: excluded by the user's decision (in the code).
- **Rewrite every address access to context fields first**: 27,695 data-address uses and 4,700–6,500 host-pointer
  conversions, no parity gain, and the offsets are needed for pointer compression anyway (§3.1).
- **Carry only the needed set** (the measured ∪ static objects, 0.27 MB): half the data, but not closed: 8 of 9
  hold-out scenarios diverged or crashed with it (census §2). It stays the direction of PURE-2's trimming, with
  evidence per object.
