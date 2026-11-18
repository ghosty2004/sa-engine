# PURE-1: what native sa-game takes from gta_sa.exe (measurements)

The facts behind `docs/arch/ADR-005-pure-build.md`, measured on master 8b81a601 (2026-11-18). Tools, all under
`tools/pure/`:

- `address_census.py`: every hex literal in `crates/**/*.rs` (comments and strings stripped) that lies in the
  1.0 US image, by section, by context and by crate (`--csv` writes one row per occurrence).
- `arena_trace.c`: an `LD_PRELOAD` library for the i686 sa-game. Once the arena is filled from the exe it makes every
  page of `.text`, `.rdata`, the initialised `.data`, `_TEXT_HA`/`_rwdseg`/`.rsrc` and `.HOODLUM` inaccessible; each
  fault records the address as read-first or write-first (page-fault error code), opens the page, single-steps the
  instruction and closes the page again. A page whose last 100,000 faults (`SA_TRACE_STABLE`) found nothing new is
  left open. The run's report is unchanged by the trace (crossing: `report.md` identical with and without it). With
  `SA_IMAGE_KEEP=FILE` it instead zeroes every byte of those sections outside the listed ranges, as an image built only
  from them would leave the arena (the hold-out check of §2).
- `trace_scenarios.sh`: runs FRONTIER scenarios (`tools/e2e/scenarios/frontier.sh`, `--fixed-step 40`, offscreen)
  with the trace.
- `arena_usage.py`: reads the traces and the exe image and widens the needed bytes to data objects (object starts:
  `catalog/data-tables.csv`, `catalog/rwsets.csv`, the Rust literals, pointers stored in the data sections; an object
  runs to the next start, at most one page), classified by kind. A byte written first counts too (a read-modify-write
  faults as a write), and every non-zero object on a page the trace left open counts.

Runs: 16 scenarios, 27,000 iterations in total (pausemenu, crossing, solicit, sniper, hunter, gangwar, boat, melee,
postfx, tank, cinema, replay, copsf, flamer, driveby, samsite), all with 0 blocked calls, 29 to 121 s each with the
trace. The reports of untraced runs were the same in two runs of nitro and of escalator (the scenarios are
deterministic).

## 1. Raw addresses in the sources

61,252 occurrences of image addresses in game crates (1,732 more in tests and the phase A / oracle crates).

Context × section (occurrences):

| context | code | .rdata vtable | .rdata IAT | .rdata const | .data | .bss | _TEXT_HA.. | no-CD image | .HOODLUM | total |
|---|---|---|---|---|---|---|---|---|---|---|
| hook (`#[hook]` / `#[native]` address) | 13,940 | | | | | | | | | 13,940 |
| other (comparisons, tables, pokes, rand sites, fn identities) | 2,875 | 1,375 | 2 | 4,469 | 1,372 | 2,933 | 2 | 100 | 8 | 13,136 |
| raw (`g`, `gf`, `p8`, `GPtr::from_addr`, ...) | 41 | | | 3,708 | 947 | 5,937 | | | 1 | 10,634 |
| orig (`orig!` callee) | 9,427 | | | | | | | 1 | | 9,428 |
| gen-check (provenance tables of generated files) | 6,553 | | | | | | | | 35 | 6,588 |
| const (named constant) | 269 | 45 | 3 | 839 | 632 | 3,925 | 2 | 18 | 5 | 5,738 |
| globals (`globals!` line) | 8 | | | 8 | 247 | 988 | | | 9 | 1,260 |
| at-site (caller attribution) | 264 | | | | | | | | 3 | 267 |
| orig-cell (`cell` / `table` forms) | | | 102 | | 2 | 157 | | | | 261 |
| **total** | 33,377 | 1,420 | 107 | 9,024 | 3,200 | 13,940 | 4 | 119 | 61 | 61,252 |

Distinct addresses: code 15,844 (2,385 of them used as values: function identities, rand caller sites, switch-table
reads, protection reads), vtables 536, IAT cells 66, other `.rdata` 3,771, `.data` 1,977, `.bss` 5,319, `.HOODLUM` 59
(21 as data), no-CD image 33.

Data addresses (contexts raw, const, other, globals, orig-cell; data sections only): **27,695 occurrences, 11,672
distinct addresses**. By crate: sa-ai 4,705, sa-render 3,546, sa-world 3,093, sa-audio 2,437, sa-core 2,332, sa-veh
1,919, sa-hud 1,727, sa-rwcore 1,671, sa-pop 1,349, sa-stream 1,149, sa-script 1,116, sa-cam 921, sa-ped 451,
sa-weapon 449, sa-anim 413, sa-input 218, sa-game 109, sa-samp 59, sa-math 31.

Crate × section (occurrences):

| crate | code | .rdata vtable | .rdata IAT | .rdata const | .data | .bss | _TEXT_HA.. | no-CD | .HOODLUM | total |
|---|---|---|---|---|---|---|---|---|---|---|
| sa-ai | 11,737 | 1,145 | 0 | 2,178 | 149 | 1,233 | 0 | 1 | 19 | 16,462 |
| sa-world | 3,216 | 24 | 0 | 614 | 350 | 2,105 | 0 | 7 | 4 | 6,320 |
| sa-render | 2,661 | 84 | 0 | 678 | 274 | 2,510 | 0 | 15 | 0 | 6,222 |
| sa-core | 2,579 | 14 | 13 | 1,092 | 49 | 1,164 | 0 | 3 | 6 | 4,920 |
| sa-rwcore | 2,902 | 0 | 11 | 234 | 448 | 976 | 2 | 51 | 0 | 4,624 |
| sa-audio | 1,978 | 62 | 41 | 831 | 598 | 905 | 0 | 2 | 2 | 4,419 |
| sa-veh | 1,437 | 26 | 0 | 792 | 254 | 847 | 0 | 8 | 2 | 3,366 |
| sa-stream | 1,309 | 6 | 20 | 469 | 361 | 293 | 0 | 7 | 7 | 2,472 |
| sa-hud | 703 | 0 | 3 | 943 | 39 | 742 | 0 | 2 | 2 | 2,434 |
| sa-pop | 1,008 | 7 | 0 | 385 | 96 | 861 | 0 | 2 | 5 | 2,364 |
| sa-script | 882 | 7 | 0 | 161 | 56 | 892 | 0 | 5 | 10 | 2,013 |
| sa-cam | 492 | 2 | 0 | 50 | 418 | 451 | 0 | 1 | 0 | 1,414 |
| sa-ped | 815 | 22 | 0 | 108 | 66 | 255 | 0 | 0 | 0 | 1,266 |
| sa-anim | 664 | 4 | 0 | 148 | 17 | 244 | 0 | 0 | 0 | 1,077 |
| sa-weapon | 520 | 5 | 0 | 163 | 14 | 267 | 0 | 0 | 0 | 969 |
| sa-input | 266 | 0 | 0 | 135 | 3 | 80 | 0 | 0 | 1 | 485 |
| sa-game | 198 | 0 | 19 | 9 | 8 | 71 | 2 | 6 | 3 | 316 |
| sa-samp | 6 | 12 | 0 | 3 | 0 | 44 | 0 | 9 | 0 | 74 |
| sa-math | 2 | 0 | 0 | 31 | 0 | 0 | 0 | 0 | 0 | 33 |
| sa-rw-wgpu | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 |

The literal count is a lower bound of the accesses: a named constant is counted once, where it is declared. The
`orig!` forms that read exe bytes at run time: 70 `site` declarations (67 distinct import call sites, resolved from a
copy of `.text` that sa-game keeps), 2 `callsite`, 110 `cell` (IAT cells and global function pointers) and 152 `table`
(RW's engine function table through the `.bss` pointer 0xC97B24, filled at run time).

Host pointers handed to the game as game pointers (they fit only because the process is 32-bit): `GPtr::from_raw`
3,520 calls; `field::ptr_of` 31 calls under its own name and, imported under other names, `lp` 1,040, `local` 771,
`vp` 550, `buf` 280, `slot` 225, `p3` 33 calls (some of those names are also local helpers, so 1,071–2,930);
`vec3f_ptr` / `vec_ptr` / `buf_ptr` 79. In all about 4,700–6,500 sites.

## 2. The exe's data at run time

Bytes whose first access was a read (or a write, see above), union of the 16 runs (4-byte marks per fault):

| section | image bytes | non-zero in image | needed | of them non-zero | pages left open |
|---|---|---|---|---|---|
| `.rdata` | 311,296 | 204,708 | 31,430 | 24,353 | 18 |
| `.data` (initialised part) | 262,144 | 179,549 | 53,844 | 34,478 | 14 |
| `_TEXT_HA`/`_rwdseg`/`.rsrc` | 77,824 | 51,581 | 8 | 0 | 1 |
| `.text`/`_rwcseg` | 4,550,656 | 4,016,684 | 174 | 152 | 0 |
| `.HOODLUM` | 135,168 | 99,602 | 12 | 11 | 0 |

Widened to objects: the measured set (objects holding a needed non-zero byte, and every non-zero object on the 33
pages left open), the static set (every data object the original's game code reads or takes the address of per
`catalog/rwsets.csv`, CRT / third-party / protection code left out, the Rust literals, and what the pointers in those
objects reach), and their union:

| kind | measured: objects / bytes | static: objects / bytes | **union: objects / bytes / non-zero** |
|---|---|---|---|
| `.rdata` strings | 2,666 / 39,543 | 3,279 / 54,204 | 3,656 / 60,543 / 49,046 |
| `.rdata` other data (tables, structs) | 379 / 17,989 | 193 / 25,388 | 439 / 28,462 / 19,631 |
| `.rdata` vtables / function tables | 540 / 25,108 | 542 / 25,020 | 548 / 25,224 / 18,447 |
| `.rdata` floats / dwords | 3,018 / 12,072 | 2,318 / 9,272 | 3,383 / 13,532 / 11,467 |
| `.rdata` doubles | 131 / 1,048 | 156 / 1,248 | 156 / 1,248 / 686 |
| `.rdata` IAT cells | 112 / 1,464 | 111 / 1,381 | 112 / 1,464 / 1,237 |
| `.rdata` pointer tables | 67 / 356 | 10 / 184 | 72 / 444 / 234 |
| `.data` other data (tables, structs, initial values) | 1,280 / 84,941 | 835 / 72,157 | 1,326 / 93,303 / 52,640 |
| `.data` strings | 307 / 18,331 | 219 / 18,202 | 352 / 20,213 / 15,213 |
| `.data` function tables (incl. the CRT initialiser tables) | 26 / 11,708 | 4 / 488 | 26 / 11,708 / 8,448 |
| `.data` floats / dwords | 1,825 / 7,300 | 1,047 / 4,188 | 1,853 / 7,412 / 6,219 |
| `.data` pointer tables | 301 / 6,436 | 50 / 2,184 | 303 / 6,468 / 4,705 |
| `.data` doubles | 56 / 448 | 52 / 416 | 57 / 456 / 220 |
| `_TEXT_HA`/`.rsrc` | 0 / 0 | 18 / 2,228 | 18 / 2,228 / 1,230 |
| **total** | **10,708 / 226,744** | **8,834 / 216,560** | **12,301 / 272,705 / 189,423** |

The measured set has 3,467 objects (56,145 bytes) the static set misses (data reached through computed addresses), so
neither alone is complete. Of the union, the kinds regenerated from the catalogue rather than copied (IAT cells: the
import table; vtables and function tables: code addresses, which are registry keys; pointer tables: addresses of
other carried objects) are 45,628 bytes. **Unique to the exe: 227,077 bytes (156,349 non-zero)**: strings 80,894,
tables / structs / initial values 123,531, floats and doubles 22,652 (many of these are compiler constants such as
`1.0f` at 0x858624 that become Rust literals).

The whole initialised image, for comparison: 641,024 raw bytes; as objects 61,559 objects, 648,555 bytes, 455,453
non-zero (`_TEXT_HA` alone 63,416 bytes, of which game code references 2,228).

**Hold-out check.** An exe-filled arena cut down to the union of a first pass (the same 16 scenarios with a
4,000-fault budget, 285,786 bytes) plus every IAT cell and the 66 code dwords, everything else zeroed
(`SA_IMAGE_KEEP`), against the full arena (reports compared without the frame-pacing line):

| scenario | in the measured set | result |
|---|---|---|
| pausemenu, crossing | yes | identical |
| shootdriver | no | identical |
| plane, nitro, monster, escalator | no | reports differ (2,419 / 982 / 1,014 / 992 lines) |
| quad, jetpack, shotgun, roadblocks | no | SIGSEGV (a null game pointer read out of a zeroed table) |

Traces of three of them (quad, plane, jetpack) find 85 objects (5,012 bytes) outside the 16-scenario union: few
bytes, each of them behaviour. A needed set measured on scenarios is not closed; the image has to hold everything the
code can reach, which without whole-game coverage means the whole initialised image.

Unique data of the union by owning subsystem (the most common catalogue subsystem of the game functions that reference
the object):

| subsystem | objects | bytes | non-zero |
|---|---|---|---|
| (reached through pointers only) | 3,975 | 66,101 | 47,221 |
| AUDIO | 560 | 52,530 | 33,217 |
| RW | 405 | 16,580 | 13,058 |
| HUD | 1,109 | 15,443 | 10,916 |
| RENDER | 510 | 10,912 | 7,233 |
| CORE | 121 | 10,516 | 8,602 |
| UNK-game | 886 | 7,872 | 6,020 |
| STREAM | 481 | 7,030 | 5,094 |
| WORLD | 534 | 6,954 | 4,707 |
| VEH | 636 | 6,696 | 3,779 |
| PED | 126 | 4,904 | 1,008 |
| AI | 566 | 4,812 | 3,721 |
| ANIM | 72 | 4,226 | 1,866 |
| CAM | 431 | 3,587 | 2,720 |
| COLL | 204 | 2,835 | 2,360 |
| POP | 176 | 1,802 | 1,266 |
| INPUT | 107 | 1,518 | 1,332 |
| WEAPON | 90 | 912 | 735 |
| SAVE | 94 | 556 | 458 |
| PHYS | 72 | 504 | 416 |
| SCRIPT | 59 | 491 | 389 |
| CALLED-ONLY | 18 | 264 | 217 |
| MATH | 7 | 32 | 14 |

Limits: the object boundaries are approximate (an object runs to the next known start, at most one page); the access
width is not decoded (4 bytes are marked per fault); a second access of one instruction to the same page is not seen.

## 3. Code bytes read as data

Measured: 63 `.text` dwords and 3 `.HOODLUM` dwords (264 bytes; the same in both passes). They are of two kinds:

- **switch byte tables MSVC placed in `.text`**, read by ports as data: 0x4382E4.. (`CCarCtrl` missions, sa-pop
  `carctrl3.rs`), 0x4B3F3C.. (sa-ai `knock_off_bike_damage_anims.rs`), 0x4E1C7C, 0x4F4F48, 0x4F7998.., 0x501640
  (sa-audio `port2_b.rs`), 0x4DADB4, 0x4DAE2C;
- **no-CD protection reads**: a `.HOODLUM` dword combined with code bytes (0x1560910 ^ 0x4D4314 in sa-veh
  `enterexit.rs`, 0x1560628 + 0x563470 and 0x15607EC ^ 0x727253 in sa-audio `port4_service.rs`); every such value is
  static (only these reads reference the cells), i.e. a constant.

Statically: 33 distinct protection-read targets (`catalog/code-data-reads.csv`) appear as literals in 49 files (78
occurrences). The catalogue lists 3,386 such reads in the original; the ports keep the ones on the paths they port.
`resolve_site` keeps a copy of `.text` for the 67 import call sites.

## 4. Third-party code and the 51 functions that stay original

Referenced from ports (`orig!` / values) and how phase B handles them today:

| library / class | referenced | phase B today |
|---|---|---|
| MSVC CRT (510 catalogued) | 56 addresses | ~60 native replacements in sa-game `crt.rs` (allocation, strings, stdio, `sscanf` / `sprintf`, maths, `qsort`, `eh vector` iterators, `atexit`) |
| D3DX9 (955) | 11 | `D3DXAssembleShader` native (`d3d9_shader.rs`, `sa_rw_wgpu::sm::asm`); the five matrix / vector helpers RW's vertex-shader path calls (0x7672E1, 0x7676C7, 0x7675EE, 0x766AB7, 0x766A32) native (`d3dx_math.rs`); the path runs only with `SA_GAME_SHADERS=1` |
| libjpeg 6b (204) | 11 | the seven calls of `JPegCompressScreen` native over `jpeg-encoder` (`libjpeg.rs`): the gallery photo is written |
| libpng 1.0.12 + zlib 1.1.3 (197) | 12 | `RtPNGImageWrite`'s eleven calls and `png_error` native over `png` (`libpng.rs`) |
| QuickTime / Media Foundation (39) | 2 | the loader and `MFStartup` succeed and `CAEMFDecoder`'s methods decode MP3 over `symphonia` (`mfdecoder.rs`) |
| `CAEAudioChannel` / `CAEStaticChannel` / `CAEStreamingChannel`, `CAEStreamThread`, `CAESmoothFadeThread` | stays original | DirectSound itself is native (`dsound.rs`: buffers, 3D, software mixer into `sa_platform::audio`); only the class destructors are unported (shutdown) |
| `CAEVorbisDecoder` / `CAEDataStream` | stays original | decoding goes through the `ov_*` imports, native via `lewton` (`vorbisfile.rs`); the IStream stubs are unreached |
| `CAEWaveDecoder`, `CAEMFDecoder`, `CAEWMADecoder` (user tracks) | stays original | `CAEMFDecoder` served (MP3); `CAEWaveDecoder` is game code still to port; WMA unavailable (wmvcore.dll); the track scan thread is not ported |
| `std::logic_error` / `length_error` (3) | stays original | unreachable (the vector `_Xlen` natives stop first) |
| bug-report writer (2), Winsock client `vt_866db0` (8) | stays original | development tooling / unused |

## 5. Web

The web loader (`web/main.js`, `web/loader.js`) requires `gta_sa.exe` in the chosen folder and reads it whole;
`platform/web.rs` maps the same arena in the reserved low linear memory (below 0x1600000, `--global-base`). The wasm
module is 34.7 MB (samp + mods).
