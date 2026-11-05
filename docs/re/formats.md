# File format findings (FMT-1)

Findings from `crates/sa-formats` and its whole-install test
(`cargo test -p sa-formats --release --test game_data -- --ignored --nocapture`, `SA_GAME_DIR`
pointing at the game copy). Sources: gta-reversed (`Directory.h`, `ColHelpers.h`,
`FileLoader.cpp`, `CollisionData.*`, `Game.cpp`, RW headers `rwplcore.h`/`rwcore.h`/`rpcriter.h`/
`rpworld.h`), gtamods wiki and librw for RW struct layouts. Where these disagree with the files, the
files win; every claim below was checked on the 1.0 US install unless marked otherwise.

## IMG (VER2)

- 8 archives: `models/{gta3,gta_int,player,cutscene}.img`, `anim/{anim,cuts}.img`,
  `data/Paths/carrec.img`, `data/script/script.img`; 21,038 entries. No IMG v1 (`.dir`) exists in the
  install, so v1 is not implemented.
- Entry: `u32 offset`, `u16 size`, `u16 size_in_archive`, `char name[24]` (sectors of 2048 bytes).
  The second u16 is 0 in every entry (gta-reversed calls it `SizeInArchive`, "always 0 when read";
  gtamods calls the first one the streaming size).
- 20,550 of 21,038 names contain non-zero bytes after the terminating NUL (uninitialised tool
  memory). Byte-exact writing must keep the full 24 raw bytes.
- The directory is followed by zero bytes up to the first entry's sector; entries do not overlap and
  lie inside their archives.
- Entry data is padded with zeros to whole sectors; RW and COL parsers keep that as trailing bytes.

## RenderWare streams (DFF, TXD)

- 15,347 DFF and 4,037 TXD files (loose + in `models/*.img`); all parse into a chunk tree that
  serialises back byte-exactly. No container chunk had to be kept raw: every container payload
  tiles exactly into child chunks.
- Top-level chunks: 15,651 Clumps, 4,037 TexDictionaries, 25 UV animation dictionaries (0x2B, in
  DFFs, before the clump). Some DFFs hold several top-level chunks.
- After the last top-level chunk only zero padding occurs (IMG sector fill). Stream parsing stops at
  chunk type 0 or at a chunk that would not fit, and keeps the rest as trailing bytes.
- Library stamps seen: 0x1803FFFF (3.6.0.3, nearly all), 0x1400FFFF (3.5.0.0, 1,122 chunks),
  0x1003FFFF (3.4.0.3, 6 chunks). Unpacked with librw's rule; all are >= 0x34000, so no geometry
  carries the pre-3.4 surface properties, and every material struct carries them (> 0x30400).
- Typed decoders (clump, frame list, geometry list, geometry struct, material list, material,
  texture, atomic, texture dictionary, native texture) rebuild every clump and texture dictionary in
  the install byte-exactly (15,651 clumps, 4,037 dictionaries, 32,869 native textures).
- Clump struct: `numAtomics, numLights, numCameras`. 2,203 lights occur, each as a Struct
  (`i32` frame index) + Light chunk; no cameras.
- Geometry: no native (`rpGEOMETRYNATIVE`) geometry and no embedded atomic geometry occur.
- Triangle word order in the geometry struct is `v1, v0, material, v2` (librw reading two u32);
  the decoder exposes `v = [v0, v1, v2]`. **Not yet confirmed against `gta_sa.exe`'s
  RpGeometryStreamRead** (RW code is not reversed in gta-reversed); confirm before the renderer
  depends on winding.
- Texture struct word: filter in bits 0-7, U/V addressing in bits 8-11/12-15 (gtamods, librw). The
  upper 16 bits are kept raw; their meaning in SA is unverified.
- Plugin chunks seen (kept raw, in Extensions): Skin 0x116 (935), HAnim 0x11E (35,460), MatFX 0x120
  (11,558), UVAnim 0x135 (29), ADC 0x134 (27), SkyMipmapVal 0x110 (131), RightToRender 0x1F
  (4,113), BinMesh 0x50E (one per geometry), and Rockstar 0x0253F2xx: F3 pipeline set (549),
  F6 specular map (8,167), F8 2dfx (1,851), F9 night vertex colours (12,972), FA collision (214),
  FC environment/reflection map (17,386), FD breakable (19,543, one per geometry but 4), FE node name
  (59,629). IDs from gta-reversed plugin headers.

### Native textures (TXD)

- 32,846 D3D9 (platform 9) and 23 D3D8 (platform 8) textures; 31,525 are DXT-compressed.
  The D3D8 textures use the D3D8 layout (word after the raster format = "has alpha", last byte =
  DXT number), as gtamods documents.
- Formats seen: DXT1 with 565/1555 raster formats, DXT3 with 4444, 8888 and 888 uncompressed, one
  PAL8 texture (1024-byte palette). Raster type is always 4 (texture). No PAL4, no cube textures:
  both are rejected as unsupported (PAL4 palette size not verified).
- D3D9 flags byte seen: 0, 1, 8, 9 (bit 0 alpha, bit 3 compressed).
- The second u16 of the dictionary struct ("device id") takes the values 0, 1, 2 and 6; kept raw,
  meaning for the loader not verified.
- Names/masks are 32 raw bytes, kept raw.

## COL (COLL/COL2/COL3)

- 3 loose files (`models/coll/{peds,vehicles,weapons}.col`) and 251 `.col` entries in IMG archives.
  Models: 6 COLL (that parse), 885 COL2, 9,270 COL3, 0 COL4.
- COL2+ offsets are relative to byte 4 of the model (after the FourCC), confirmed by the loader's
  pointer arithmetic and by the files (every section lands exactly; 0 uncovered bytes anywhere).
- Face groups (when flag 8 is set) sit directly before the faces, followed by their u32 count
  (gta-reversed `CCollisionData::GetFaceGroups`); this matches every file. No model has lines/disks (`nLines` = 0), a triangle plane offset, or
  a non-zero header pad byte.
- Vertex arrays have no count. The vertex section runs to the next section; 2,132 models have 2
  extra bytes there (not verified as alignment, kept raw), 143 have 2 extra bytes after the shadow
  vertices. 4 models (e.g. `sfse_1.col`, `law_3.col`, `law2_4.col`) contain vertices
  beyond the highest index used by a face (unreferenced vertices, not padding). The parser decodes
  `region / 6` vertices and keeps the 0-5 leftover bytes raw.
- Face groups appear in 3,612 models (flag 8), shadow meshes in 578 (flag 16).
- **peds.col is not a valid collision file.** SA never loads it as collision (default.dat/gta.dat
  load only WEAPONS.COL). The exe opens it in `ValidateVersion` (0x5BA060), which reads every 100th
  byte from offset 100, minus 23, until NUL: the text is `grandtheftauto3Version 1.0`. These stamp
  bytes overwrite model data: byte 0x190 is the low byte of model 0 (`cop`)'s box count, so the count
  reads 133 (`'n' + 23`) with no box data. Independently, model 7 (`male01`, at 0x1550) is one byte
  shorter than its declared size, so a walk like `LoadCollisionFile` loses sync at 0x1834. The strict
  parser rejects the file; the test asserts exactly this error and prints the split and the decoded
  version text. `col::peds_col_version_text` implements the decoding.

# Data definition files (FMT-2)

Code: `crates/sa-formats/src/{lines,scanf}.rs` and `src/data/`. Whole-install check:
`SA_GAME_DIR=… cargo test -p sa-formats --release --test game_data_defs -- --ignored --nocapture`
(`SA_DUMP_DIR=<dir>` also writes one canonical dump per file for FMT-2 part b). Source of every
statement below is the 1.0 US code (Ghidra decompile of the addresses given, mode strings read from
`.rdata`), cross-checked with gta-reversed; where the two differ it is noted.

## How the exe reads text files

- `CFileMgr::OpenFile` is `fopen` (MSVC 7.1 CRT). The mode matters: `"rb"` keeps `\r`, `"r"` turns
  `\r\n` into `\n` and stops at a 0x1A byte. No data file contains 0x1A, all text files use CRLF
  (except `water*.dat` and `timecycp.dat`, LF only), and no text line exceeds 511 bytes, so the
  modes only differ in the trailing blank (see `gta.dat` below).
- `CFileLoader::LoadLine(FILE*)` (0x536F80): `fgets(ms_line, 512, f)`; every byte `< 0x20` and every
  `,` becomes a space; leading bytes `1..=0x20` are skipped. `lines::load_lines`.
- `CFileLoader::LoadLine(char*&, int32&)` (0x536FE0): copies from a memory buffer up to `\n`, NUL or
  the size, same sanitising (gta-reversed skips `\r` instead of turning it into a space; same effect
  for the parsers). `lines::buffer_lines`.
- Fields are read with `sscanf` from the CRT; `scanf::sscanf` reimplements its semantics
  (32-bit wrap-around integers, `%x` with optional `0x`, `%f` collection `[-]d[.d][e[-]d]`, `%s`
  without length limit, `EOF` when the input ends before the first conversion, unassigned
  destinations keep their old value). Loaders call `sscanf` several times on one line with the same
  variables; `data::Vars` replays that, and fields the exe may leave unassigned are `Option`.
- The original uses plain `sscanf` with `char[24]`-style buffers (gta-reversed uses `sscanf_s`
  with sizes, and `%11s` for the ped anim file). No name in the install is long enough to overflow.

## gta.dat / default.dat (`CFileLoader::LoadLevel` 0x5B9030)

- Mode `"r"`. `#` lines are skipped; then `strncmp` in this order: `EXIT`, `TEXDICTION` (arg at +11),
  `IMG` (+4), `COLFILE` (+10, skipping the zone digit), `MODELFILE` (+10), `HIERFILE` (+9), `IDE`
  (+4), `IPL` (+4), `SPLASH` (no action). Everything else is ignored.
- The argument is the rest of the line **including the trailing space** that replaced `\n`.
  Windows ignores it when opening the file, but two string tests see it:
  - `IMG`: the archive is skipped only when the 19 bytes at +4 equal `MODELS\GTA_INT.IMG\0`; with the
    trailing space this never matches (no `IMG MODELS\GTA_INT.IMG` line exists in the install).
  - The `occl` loader (0x5B4C80) sets its "interior" argument when bytes `len-7..len-5` of the IPL
    path are `int` (meant for `…int.ipl`). With the trailing space, `DATA\MAPS\occluint.ipl ` gives
    `nt.` and **no occluder is flagged interior**. Derived from the code; confirm in part b
    (`COcclusion` interior flags after load).
- The first `IPL` line runs `object.dat`, vehicle setup, streaming init and collision bounds first.
- gta.dat: 4 IMG, 1 COLFILE (default.dat), 57 IDE (3 in default.dat), 52 IPL/ZON, 6 SPLASH; every
  IDE/IPL path exists in the install (case-insensitive).

## IDE (`LoadObjectTypes` 0x5B8400)

- Mode `"rb"`, `LoadLine`. Lines starting with NUL or `#` skipped. Section names are compared on
  the first 4 bytes (`objs tobj weap hier anim cars peds path 2dfx txdp`), `end` on 3 bytes.
- Formats per section: see the table in `data/ide.rs`. Notable:
  - `objs`/`tobj`: the new format is tried first; if it does not return all fields **or the draw
    distance is < 4**, the old "type + 1..3 draw distances" format is read. Only `ret != 4` of the
    `%d %s %s %d` probe rejects the line.
  - `cars`: one 15-field `sscanf` without a return check. 11 lines of `vehicles.ide` (boats,
    trains, aircraft without wheels) have 11 fields, so the two wheel scales stay **uninitialised**
    in 1.0 US (gta-reversed initialises them to 0.7, a fix) and the upgrade class keeps -1.
  - `anim`: anim name defaults to `null`, draw distance to 2000; must return 6.
  - `2dfx` (0x5B7670, not reversed in gta-reversed): `%d %f %f %f %d` then a second format by type
    (0 light with two hand-copied `"…"` strings, 1, 3, 5 (38 fields), 6, 7, 8, 9, 10). The install
    has 97 entries; all are kept as the raw `sscanf` results (`ScanRec`).
  - `path`: a VC leftover; no IDE has a non-empty `path` section; lines would be kept raw.
- Install: 14,052 objs, 160 tobj, 50 weap, 35 hier, 54 anim, 212 cars, 276 peds, 97 2dfx, 38 txdp.

## Text IPL and ZON (`LoadScene` 0x5B8700)

- Mode `"rb"`, `LoadLine`, same skip/section rules (13 sections; `mult` has no loader).
- **A `path` section ends the whole file**: the first non-`end` line inside it breaks the read
  loop (gta-reversed reproduces this). `paths.ipl` … `paths5.ipl` are therefore read only up to
  line 7: 165,232 lines (all their path data) are never parsed. The `(type, -1)` header/12-node
  logic in the same function is unreachable in `LoadScene` (section id 7 is never set).
- Formats and acceptance per section: table in `data/ipl.rs`. Notable:
  - `occl`: `%f %f %f %f %f %f %f %f %f %d ` with rot y/x and flags pre-set to 0; `occlu.ipl` (not
    loaded) has 17 old 7-field lines (no rotations/flags).
  - `cull`: 14-field mirror form if it returns 14, else the 11-field form (no check).
  - `grge`: needs 11 fields. Two `sfe.ipl` garages (lines 297, 298) have an empty name and
    return 10, so **they are not created**.
  - `enex`: `%s` name token; `strrchr(token, '"')` is cut and the name starts at byte 1.
  - `tcyc`: three trailing floats pre-set 100, 1, 1; when fewer than 12 fields are read the last
    argument takes the 11th's value.
  - `auzo`: box form (9 fields) else sphere form (7).
- Install (53 IPL + 2 ZON): 9,315 inst, 384 zone, 1,266 cull, 1,012 occl, 52 grge, 376 enex,
  5 pick, 8 tcyc, 155 auzo; no text `cars`/`jump`/`mult`.

## Binary IPL (`CIplStore::LoadIpl` 0x406080)

- `bnry` + 18 `u32`: 6 counts (inst, 3 unused, car generators, 1 unused) and 6 `(offset, size)`
  pairs. Size fields are 0 everywhere; the unused sections are empty everywhere. Instances
  (`CFileObjectInstance`, 0x28) at 0x4C, car generators (`CFileCarGenerator`, 0x30) right after;
  the rest is zero sector padding.
- 190 files (164 in `gta3.img`, 26 in `gta_int.img`): 41,667 instances, 1,045 car generators, all
  byte-exact round trips.

## Path nodes (`CPathFind::LoadPathFindData(RwStream*, int32)` 0x4529F0)

- Header 5 `u32` (nodes, vehicle nodes, ped nodes, navi nodes, links `L`), nodes (0x1C), navi
  nodes (0x0E), then `L + 192` `CNodeAddress`, `L` navi link words, `L + 192` link lengths,
  `L + 192` intersection bytes (192 = 16 dynamic link slots x 12, read from the file too). This
  accounts for every byte of all 64 files (rest = zero IMG padding).
- The 2 bytes before the node flag word (offset 22) and the dynamic link slots are non-zero in
  every file; both are kept raw.
- 64 `nodesN.dat` in `gta3.img` + 64 loose `data/Paths/NODES*.DAT` (same bytes without padding):
  136,474 nodes (61,174 vehicle, 75,300 ped), 62,932 navi nodes, 287,244 links; all byte-exact.
  The loose copies are not loaded: the only reference to `data\paths\nodes%d.dat` is in
  `LoadPathFindData(int32)` 0x452F40, which has no callers.

## Vehicle and population tables (`data::vehicle`)

Mode strings: 0x859C6C = `"rb"`, 0x85A53C = `"r"`.

| file | loader | mode / reader | rules and formats |
|---|---|---|---|
| handling.cfg | cHandlingDataMgr::LoadHandlingData 0x5BD830 | rb / LoadLine | Stops when the line equals `;the end\0` (9 bytes); with `LoadLine` the CRLF becomes two trailing spaces, so this never matches the shipped file (it is a `;` comment, so the read runs to EOF). First byte: `;` comment, `!` bike, `$` flying, `%` boat, `^` anim group, else car (an empty line too). `strtok(line, " \t")`; each token by index: `(float)atof`, `atol`, `sscanf("%x")` (car 31, 32; 32 also copied to +0x78) or first byte (car 15, 16). Per-token destinations in `handling::conv()`. Car token 13 is multiplied by 0.4 (x87) before storing; the parser keeps the `atof` value. The `return` Ghidra shows after the `^` branch is a protection trampoline (0x5BE16E -> 0x40825C -> 0x5BE179); the loop continues. Names via FindExactWord 0x6F4F30 (210 names x 14 bytes), kept raw. |
| carcols.dat | CVehicleModelInfo::LoadVehicleColours 0x5B6890 | r / own `fgets` 1024 | Verified: leading bytes `< 0x21` skipped (no limit; gta-reversed stops at 10), `,` and `\r` -> space up to `\n` -> NUL. **Not verified**: the rest runs through an obfuscated jump at 0x405FB1 Ghidra does not follow; sections `col`/`car4`/`car` and formats `%d %d %d`, `%s` + 16 `%d`, `%s` + 32 `%d` are from gta-reversed. |
| carmods.dat | LoadVehicleUpgrades 0x5B65A0 | r / LoadLine | `#`/NUL skipped; `link`(4) `mods`(4) `wheel`(5), `end`(3); `strtok(" \t,")`; wheel set via `sscanf("%d")`. |
| ped.dat | CPedType::LoadPedData 0x608B30 | r / LoadLine | `%s` first word; `Hate`/`Dislike`/`Like`/`Respect` lines: `strtok(" ,\t")`, each name sets bit `FindPedType(name)` (< 32) in a mask that replaces the entry word; any other word selects the current type (initial index 32). |
| pedstats.dat | CPedStats::LoadPedStats 0x5BB890 | r / LoadLine | `%s %f %f %d %d %d %d %f %f %d %d`, no ret check, id = running index, locals kept between lines. |
| pedgrp.dat / cargrp.dat | LoadPedGroups 0x5BCFE0 / LoadCarGroups 0x5BD1A0 | r / own `fgets` 1024 (buffer persists) | `,`/`\r` -> space up to `\n` (scan ignores NUL), `\n` -> NUL; tokens = runs of bytes > 0x20, `#` ends the line; at most 21 (ped) / 23 (car) tokens; a car group is created only if a model resolves. |

Install: handling 210 car / 13 bike / 24 flying / 12 boat / 30 anim lines (each with exactly the
token count its loader converts); carcols 127 colours, 196 `car`, 3 `car4`; carmods 23 link,
77 mods, 3 wheel; ped.dat 17 types, 30 relation lines; pedstats 43; pedgrp 57 groups / 426 names;
cargrp 34 groups / 299 names. Findings: cargrp lines 14 and 15 lose 3 and 6 names beyond the
23-token limit; carcols line 127 (`77.93,96`) reads 1 of 3 components (`%d` stops at `.`).

## World and render tables (`data::world`)

| file | loader | mode | skip rule | format(s) |
|---|---|---|---|---|
| timecyc.dat | CTimeCycle::Initialise 0x5BBAC0 | rb | first byte `/` or NUL | 52 conversions (`%d`x21 `%f %f %f %d %d %d %f %f %f` `%d`x6 `%f`x12 `%f %d %d %f`); 23 weathers x 8 hours = 184 records, the rest of the file is not read |
| popcycle.dat | CPopCycle::Initialise 0x5BC090 | rb (g-r: r) | `/` or NUL | 24 `%d` (g-r: `%hhu`); 20 zone types x 2 day types x 12 slots = 480 |
| water.dat / water1.dat | CWaterLevel::WaterLevelInitialise 0x6EAE80 | rb (g-r: r) | `;`, NUL, `*`, `p` | flags := 1; 28 `%f` + `%d` (29: quad), else 28 `%f` (quad), else 21 `%f` + `%d` (triangle) |
| object.dat | CObjectData::Initialise 0x5B5360 | rb | `;`, NUL; **`*` ends the read** | `%s %f %f %f %f %f %f %f %d %d %d %d %d %f %f %f %s %f %f %f %f %f %d %d` |
| surface.dat | SurfaceInfos_c::LoadAdhesiveLimits 0x55D0E0 | rb | `;`, NUL | `%s`, then per column 0..=row: skip blanks, `-` = 0 else `%f` |
| surfinfo.dat | SurfaceInfos_c::LoadSurfaceInfos 0x55EB90 | r | `#`, NUL | `%s %s %f %f %s %s` + 29 `%d` + `%s` |
| surfaud.dat | SurfaceInfos_c::LoadSurfaceAudioInfos 0x55F2B0 | r | `#`, NUL | `%s` + 9 `%d` |
| procobj.dat | ProcObjectMan_c::LoadDataFile 0x5A3140 | r | `#`, NUL | `%s %s %f %f %d %d %f %f %f %f %f %f %d %d` |
| plants.dat | CPlantSurfPropMgr::LoadPlantsDat 0x5DD3B0 | r | `;` | `strtok(" \t")`: surface name, 10 `atol`, 7 `(float)atof` |

- timecyc.dat: the lines have 51 columns, so the 52nd variable (`dir_mult`) is never assigned (stack
  garbage for all 184 records). Line 320 (weather 16, hour 6) starts with one `255` instead of an RGB
  triple; `sscanf` returns 20 and the record keeps fields 21-52 from hour 5.
- timecycp.dat: no `timecycp` string in the exe; not loaded (parsed for information, same layout + a
  52nd column).
- water1.dat: the 6 triangles have no flags column (ret 21), flags stay at the preset 1. The path
  literal is `DATA\water.dat` (g-r: `DATA//`).
- object.dat: the 80-byte scan slot is zeroed before each line (slot fields reset), the stack fields
  (percent submerged, damage effect, special response, camera avoid, explosion, FX type/name) persist.
  `#` is not a comment: line 649 is read as an entry named `#` (ret 1; g-r skips it). The line
  `*********melee weapons*************` (1232) ends the read: `flowera` after it is never loaded.
- surface.dat: the first skip after the name has no NUL check.
- plants.dat: the end test compares 9 bytes with `;the end\0`; with `"r"` only a final line without a
  line break matches (true in the install). Fewer than 18 tokens or an unknown surface aborts the load.

Install: timecyc 184 (183 with 51 fields, 1 with 20), popcycle 480, water 301 quads + 6 triangles,
water1 261 quads + 6 triangles, object.dat 995 entries (235 x 24, 757 x 17, 2 x 13, 1 x 1 field) +
2 lines not read, surface 6 rows / 21 limits, surfinfo 179, surfaud 179, procobj 95, plants 68.

## Other tables (`data::misc`)

| file | loader | mode / reader | format and rules |
|---|---|---|---|
| weapon.dat | CWeaponInfo::LoadWeaponData 0x5BE670 | rb / LoadLine | First byte: `\xA3` melee `%s %s %s %f %f %d %d %d %s %d %x %s`; `$` gun (30 fields, last 4 floats pre-set 0); `%` aim `%s %s %f %f %f %f %d %d %d %d`; else `%s`, stop when it starts with `ENDWEAPONDATA` (13). No ret checks (g-r uses `%*s` and searches the whole line). |
| melee.dat | 0x5BEDC0 (not in g-r) | rb / LoadLine | `END_MELEE_DATA` stops; `START_COMBO`/`START_LEVELS`/`END_COMBO`; levels `%s %f %f %f`; combo line i: 0 `%s %s`, 1 `%s %f`, 2-6 `%s %f %f %f %s %d %d %d %f`, 7 `%s %f %f`, 8 `%s %x`, 9+ skipped |
| ar_stats.dat | CStats::LoadActionReactionStats 0x5599B0 | rb / LoadLine | `%d %s %f` |
| statdisp.dat | CStats::LoadStatUpdateConditions 0x559860 | rb / LoadLine | `%d %s %s %f %s`; condition `lessthan`/`morethan` exact, else not written; text into 8 bytes |
| shopping.dat | CShopping::LoadStats 0x49B6A0 (init); LoadPrices 0x49B8D0, LoadShop 0x49BBE0 (in game) | r / LoadLine | `section` (7) + name, `end` (3); items `strtok(" \t,")`; names `_stricmp`. Kept as a tree of raw tokens. |
| furnitur.dat | FurnitureManager_c::LoadFurniture 0x5C0280 | rb / LoadLine | `GROUP:` `%s %s`; `SUBGROUP:` `%s %s` + 7 `%d`; `ITEM:` `%s %s` + 4 `%d` |
| animgrp.dat | CAnimManager::ReadAnimAssociationDefinitions 0x5BC910 | rb / LoadLine | header `%s %s %s %d`, then `%s` per line; ret 1 and exactly `end` closes |
| stream.ini | CStreaming::ReadIniFile 0x5BCCD0 | r / LoadLine | `strtok(" ,\t")` key/value, `_stricmp` keys; `atol` (memory `<< 10`), `(float)atof`; `memory` after `devkit_memory` is ignored |
| fonts.dat | CFont::LoadFontValues 0x7187C0 | rb / LoadLine | exact keys; `[TOTAL_FONTS]`, `[FONT_ID]`, `[REPLACEMENT_SPACE_CHAR]`, `[UNPROP]` read the next line with `%d` (never skipped); `[PROP]` the next 26 lines with 8 `%d` (low bytes) |
| gridref.dat | CGridRef::Init 0x71D4E0 | rb / LoadLine | `%c%d %s` |
| clothes.dat | CClothes::LoadClothesFile 0x5A7B30 | r / LoadLine | `rule`/`end`; `strtok(" \t,")`; `_stricmp` keywords; an unknown keyword reuses the previous rule type |
| paths/tracks*.dat | CTrain::ReadAndInterpretTrackFile 0x6F55D0 | whole file (buffer 0xB530), hand-copied lines, no sanitising | optional `processed` line (`strcmp`; a `\r` defeats it), `%d` count, then count lines `%f %f %f %d`; lines are copied without NUL into a shared buffer (stale bytes after shorter lines; no effect in the install) |
| paths/roadblox.dat | CRoadBlocks::Init 0x461100 (body 0x1568D90 in `.HOODLUM`) | rb, binary | `i32` count + fixed 0x514 bytes = 325 `CNodeAddress`; 1,304 bytes; byte-exact round trip |
| decision/allowed/*.ped, *.grp | CDecisionMakerTypesFileLoader::LoadDecisionMaker 0x6076B0 | r, raw `fgets` 512 (no sanitising) | first line discarded; each line one `sscanf` with the 80-field comma format at 0x86CDA0 (g-r's `%n` loop is a refactor) |
| decision/PedEvent.txt | LoadEventIndices 0x600840 -> 0x5BB9F0 | r, raw `fgets` 256 | skip lines starting with NUL or `\n`; `%s %d` |

Install: weapon 53 guns / 17 melee / 21 aim; melee 13 combos / 117 combo lines / 7 level lines;
ar_stats 59; statdisp 122; shopping 36 sections / 1,195 item lines; furnitur 6 / 51 / 218; animgrp 21
groups / 126 anims; stream.ini 11 keys; fonts 2 fonts; gridref 100; clothes 147 rules; tracks
926 / 67 / 300 / 656 points; roadblox 325; decision 17 files (273 lines, all 80 of 80 fields);
PedEvent 41.

## Inventory of `data/` (and `stream.ini`)

Loaded by gta_sa.exe 1.0 US and parsed:
- `gta.dat`, `default.dat` (LoadLevel); every IDE and text IPL/ZON they name (57 IDE, 52 IPL/ZON;
  `txdcut.ide`, `map.zon`, `info.zon` included).
- `handling.cfg`, `carcols.dat`, `carmods.dat`, `ped.dat`, `pedstats.dat`, `pedgrp.dat`,
  `cargrp.dat`, `timecyc.dat`, `popcycle.dat`, `water.dat` (or `water1.dat`, by water
  configuration), `object.dat`, `surface.dat`, `surfinfo.dat`, `surfaud.dat`, `procobj.dat`,
  `plants.dat`, `weapon.dat`, `melee.dat`, `ar_stats.dat`, `statdisp.dat`, `shopping.dat`,
  `furnitur.dat`, `animgrp.dat`, `fonts.dat`, `gridref.dat`, `clothes.dat`, `stream.ini`,
  `Paths/tracks{,2,3,4}.dat`, `Paths/ROADBLOX.DAT`, `Decision/PedEvent.txt`,
  `Decision/Allowed/*` (14 at start via LoadDefaultDecisionMaker 0x5BF400; `m_steal`, `m_tough`,
  `m_weak` only by script).
- Inside IMGs: 190 binary IPL (`gta3.img`, `gta_int.img`), 64 `nodesN.dat` (`gta3.img`).
- FMT-3 (not FMT-2): `script/main.scm`, `script/script.img`, `Paths/carrec.img`.

Not loaded (no file-name string or caller in the exe), parsed only where noted:
- `Paths/NODES*.DAT` (the only reference, 0x452F40, has no callers; parsed + round-tripped),
  `gta_quick.dat` (parsed as a level file), `timecycp.dat` (parsed as timecyc),
  `maps/leveldes/leveldes.ide`, `maps/leveldes/leveldes.ipl`, `maps/occlu.ipl`, `maps/txd.ide`,
  `maps/vegas/vegaxref.ipl` (not named in gta.dat/default.dat, no `.ide`/`.ipl` string in the exe;
  parsed as IDE/IPL),
  `Decision/*.ped|*.grp` outside `Allowed/` (older layout), `numplate.dat`, `polydensity.dat`,
  `animviewer.dat`, `Paths/spath0.dat`, `Paths/train.dat`, `Paths/train2.dat`, `AudioEvents.txt`,
  `main.sc`, `Icons/*` (the exe's icons are PE resources).

## Open questions (FMT-2)

- carcols.dat after line preparation (sections, formats, variation counts, what the unassigned
  components of line 127 hold): the original jumps through an obfuscated stub at 0x405FB1; needs a
  REA trace or the part b memory comparison.
- cargrp: the group count depends on model lookups; part b confirms all 34 groups are created.
- Uninitialised values the parsers mark `None` (timecyc `dir_mult`, IDE `cars` wheel scales of 11
  vehicles, carcols line 127, fields of short first lines) can only be read from the original's
  memory (part b).
- plants.dat aborts on an unknown surface name (GetSurfaceIdFromName 0x55D220); names kept raw.
- shopping.dat: per-section token use in LoadStats and `GetPriceSectionFromName` on unknown names
  are from gta-reversed; LoadPrices (gta-reversed counts comment lines) not checked against
  0x49B8D0. Clothes body-part mapping (0x5A7A20) and gridref case conversion (0x836C58) not modelled.

- Float conversion: `sscanf %f` goes through `__strgtold12` + `_ld12tof`, `atof` through
  `strtod` then a `(float)` cast; `scanf.rs` uses correctly rounded conversions. Equal except in
  rare double-rounding cases; part b compares the loaded floats bit for bit.
- The `occl` interior flag and `IMG` GTA_INT test results above are derived from code; confirm
  in memory in part b.

## Open questions

- Triangle vertex order and texture-struct upper bits: confirm against `gta_sa.exe` RW stream code.
- COL4 header extra word: no COL4 file in the install; kept raw.
- TXD device id values 0/1/6 next to 2: whether the loader checks them.

# FMT-3: animations, text, scripts, recordings, audio, saves

Code: `crates/sa-formats/src/{ifp,gxt,carrec,save}.rs`, `src/scm/`, `src/audio/`. Whole-install
checks (read-only, `SA_GAME_DIR` = the game copy):

```text
cargo test -p sa-formats --release --test game_data_fmt3 -- --ignored --nocapture   # IFP, GXT, SCM, carrec, saves
cargo test -p sa-formats --release --test game_data_audio -- --ignored --nocapture  # audio containers
SA_SCRIPT_USAGE=catalog/script-usage.csv cargo test ... --test game_data_fmt3 -- --ignored scm   # writes the usage table
```

Addresses are 1.0 US; bodies of `.HOODLUM` stubs are given where the stub only jumps. Static
analysis used the venv (capstone, pefile) and `tools/catalog/script_params.py`; the install is the
Steam 3.0 data set run with the 1.0 US exe (ROADMAP decision), so file contents may differ from
retail 1.0 data.

## IFP (`CAnimManager::LoadAnimFile` 0x4D47F0)

- Dispatch: `strncmp(header, "ANP2", 4)` and `"ANP3"` go to the ANP path (0x4D5172); every other
  FourCC takes the ANPK path. The header size word is unused by both.
- ANP2/ANP3: block name `char[24]`, `u32` animation count; per animation `char[24]` name, `u32`
  sequence count, ANP3 only `u32` frame memory size + `u32` flags; per sequence `char[24]` name,
  `u32` frame type, `u32` frame count, `i32` bone tag, frames read straight into memory. Frame
  types (jump table 0x4D55B8): 1 `f32 q[4], f32 t` (0x14), 2 + `f32 pos[3]` (0x20), 3
  `i16 q[4], i16 t` (0xA), 4 + `i16 pos[3]` (0x10). Unknown types skip the read (desync); the
  parser rejects them.
- Compressed scales (`CAnimBlendSequence::Uncompress` 0x4D0D40, `fild` + `fmul dword`): rotation
  `* 0x39800000` (1/4096) at 0x858BFC, time `* 0x3C888889` (1/60) at 0x859044, translation
  `* 0x3A800000` (1/1024) at 0x85C6D4. The ANPK loader compresses with 4096.0 (0x858BF8) and
  1024.0 (0x85C6D8) + `_ftol`.
- Quirk: the hierarchy's compressed flag is first set from `flags & 1` (0x4D52CC), then
  overwritten from the first sequence's frame type (types 1/2 → uncompressed, 0x4D53F0).
- ANPK (cutscene anims): only headers are read and sizes are rounded up to 4; no FourCC is
  checked except the key-frame one (`KRTS` 0x2C/frame: rot, pos, scale, time; `KRT0` 0x20; `KR00`
  0x14). `ANIM` carries the bone tag only when its rounded size is 44 (`cmp [esp+24h], 2Ch` at
  0x4D4BA8). The exe stores the conjugate (`fchs` of x, y, z) and drops the scale.
- Install: `ped.ifp` + 132 `anim.img` entries are ANP3 (1,851 animations, 46,432 sequences,
  980,487 key frames, all frame types 3 or 4); all 148 `cuts.img` `.ifp` are ANPK (974 animations,
  33,892 sequences, all `ANIM` of 44 bytes, 21,928 `KR00` + 11,964 `KRT0`, 11,797,614 frames). No
  ANP2, no `KRTS`. All 281 files round-trip byte-exactly; the unused size words (ANP3 header and
  frame memory size, ANPK/DGAN/CPAN/key-frame sizes) all match the data they describe; the only
  trailing bytes are zero IMG sector padding.

## GXT (`CText::Load` 0x6A01A0, `LoadMissionText` 0x69FBF0)

- `u16 version`, `u16 bits per char` read and not checked. Then the exe reads 8-byte chunk headers
  until both `TKEY` and `TDAT` were seen: size 0 → skipped, `TABL` → `CMissionTextOffsets::Load`
  0x69F670 (12-byte entries `char[8]`, `u32 offset`), `TKEY` → `CKeyArray::Load` 0x69F490
  (`u32` TDAT offset, `u32` hash), `TDAT` → `CData::Load` 0x69F5D0, others skipped by size.
  Mission tables: at the TABL offset `char[8]` name, then the same loop.
- Key hash `CKeyGen::GetUppercaseKey` 0x53CF30: CRC-32 table at 0x8CD068 (= the standard
  reflected 0xEDB88320 table, compared word for word), init `0xFFFFFFFF`, **no final XOR**, each
  byte through CRT `toupper` (0x823A0E, locale-aware; keys are ASCII). `GetKey` 0x53CF00 is the
  same without `toupper`. Verified: `CDERROR` = 0xEF128EC3, `FEM_MM` = 0x440B851C, `FESZ_CA`,
  `IE23` found in every MAIN table. Lookup is a binary search with `int16` bounds
  (`BinarySearch` 0x69F570); every table is strictly sorted by hash.
- Install: 5 files, version 4, 8-bit; 635 TABL entries (127 per file), 82,997 keys, every key
  points at a NUL-terminated string. The `MAIN` entry points at the main `TKEY` header (not at a
  name). Mission tables start on 4-byte boundaries: 469 gaps of 1–3 zero bytes. Round trip
  byte-exact. 16-bit files (encoding 16) are supported by the parser (synthetic test only).

## Vehicle recordings (`CVehicleRecording::Load` 0x45A8F0, body 0x156F510)

- The whole IMG entry is read (`RwStreamRead(.., 9999999)`), then scanned in 0x20 steps; the first
  frame with index > 0 and time 0 cuts the recording (sector padding ends it). `SmoothRecording`
  0x45A0F0 rewrites times after loading.
- Frame: `u32` time, `i16 speed[3]` (`* 0x38800100` ≈ 1/16383.5), `i8 right[3]`, `i8 up[3]`
  (`* 0x3C010204` = 1/127), `i8` steer (`* 0.05`), `i8` gas, `i8` brake (`* 0.01`, all read with
  `movsx`), `u8` handbrake, `f32 pos[3]` (`RestoreInfoForMatrix` 0x459960, `RestoreInfoForCar`
  0x459A30).
- Install: 426 entries, 59,824 frames, byte-exact; every entry starts at time 0, times never
  decrease, only zero bytes after the cut; 87 entries have negative gas/brake values.

## Saves (`CGenericGameStorage::GenericSave` 0x5D13E0 / `GenericLoad` 0x5D17B0)

- 28 blocks, each introduced by the 5 bytes `BLOCK` (0x8D2BD8, `push 5`); no size fields. Block 0
  is `CSimpleVariablesSaveStructure` (0x138 bytes, `push 138h` at 0x5D14B7) starting with the
  version id `GetKey("Apr 28 2005" "10:28:55")` (0x5D0F50).
- Data padded to 202,748 bytes (`cmp eax, 317FCh` at 0x5D15D1) by writing the 0xC800-byte work
  buffer without clearing it (stale bytes, possibly old `BLOCK` tags), then the checksum: `u32`
  wrapping sum of all preceding bytes (`movzx`/`add` loop at 0x5D0FB0, `SaveWorkBuffer(true)`).
  File size 202,752.
- `save::SaveFile` splits at the first 28 markers (the last block keeps the padding) and checks
  the sum; correct only while no block body contains `BLOCK`, which the typed SAVE parsers will
  settle. **No save exists in the install** and none was generated: synthetic tests only.

## SCM (`main.scm`, `script.img`)

Code: `src/scm/mod.rs` (header), `src/scm/decode.rs` (decoder), `src/scm/opcodes.rs` (generated
by `tools/catalog/script_params.py`, also written to `catalog/script-params.csv`).

### Header (`CTheScripts::Init` 0x468D50)

- Chain of segments, each `GOTO <i32 next>` (`02 00 01`) + id byte: `'s'` globals (43,800 bytes),
  0 models, 1 missions, 2 streamed scripts, 3 (`u32` 0), 4 (`u32` 43,800 = globals size, `u32`
  569). Code starts at 0xDAA8 (55,976).
- `ReadObjectNamesFromScript` 0x486720 (body 0x156F5A0) reads only the low `u16` of the model
  count (389 names). `ReadMultiScriptFileOffsetsFromScript` 0x4867C0 (body 0x1565E20): main size
  194,125, largest mission 68,439, 135 missions, 0 exclusive, 964 largest locals.
  `CStreamedScripts::ReadStreamedScriptData` 0x470750 (body 0x156F450) ignores the table's count
  and offsets: for each script registered from `script.img` it looks the name up (`_stricmp`) and
  takes only the size. The table offsets are positions in "main.scm followed by script.img"
  (first = 3,079,744 = main.scm size). `CheckStreamedScriptVersion` 0x464FF0 is `mov al, 1; ret`.
- Segments 3/4 are not read by the exe. Header + code round-trip byte-exactly.
- `script.img` `aaa.scm` (8 bytes) holds the segment-4 words (43,800, 569), not code; nothing in
  the decoded scripts starts it. It is listed and skipped.

### Instruction encoding (verified in the exe)

- `ProcessOneCommand` 0x469EB0: `u16`, bit 15 = NOT flag, `& 0x7FFF` dispatched through the
  table at 0x8A6168 (27 `ProcessCommands<N>To<N+99>` functions; opcodes ≥ 2700 have no entry).
- Parameter types and readers: table in `scm/decode.rs`. Jump tables read: `CollectParameters`
  0x464224 (types 1–8), `StoreParameters` 0x4644DC (2, 3, 7, 8), `GetPointerToScriptVariable`
  0x4648B0 + byte index 0x4648C4 (2, 3, 7, 8, 0x0A–0x0D, 0x10–0x13), `ReadTextLabelFromScript`
  0x464054 (9–0x13), `GetIndexOfGlobalVariable` body 0x156F340 (2, 7). Type 0x0E: signed length
  (`movsx`), a negative length reads nothing and the bytes are copied without a bound; type 0x0F
  always advances 16 bytes. A type a reader does not handle consumes only the type byte.
- Array operand (`ReadArrayInformation` 0x463CF0, body 0x156E350): `u16` base, `u16` index
  variable, `u8` size, `u8` flags (bit 7 = global index).
- Jump targets: `>= 0` = offset in main.scm, `< 0` = `base - value` (`UpdatePC` 0x464DA0).
  606 mission/streamed instructions jump (positive targets) into main.scm code.

### Parameter table from the exe (`tools/catalog/script_params.py`)

There is no command table in the exe and none with counts in gta-reversed, so the counts were
derived from the handlers: for every opcode the range function is symbolically executed from its
entry with the opcode as the stack argument (registers, stack slots, flags, partial registers),
so the switch and the opcode tests inside shared helpers (LOCATE/IN_AREA/ANGLED_AREA, which take
the opcode as an argument) resolve to the opcode's own path. Helpers called with the script in
`ecx` are executed when they can reach a reader. Result (2,700 opcodes): 1,969 handled, 1,915
with one exact read list, 44 `partial` (step budget exhausted in a large helper; the reads found
are used as a candidate), 9 path-dependent, 1 unknown count; 731 go to the switch default.
Three handlers move `m_IP` themselves and are overridden from the disassembly: `SWITCH_START`
0x0871 (`C4` + `C14`, `mov bl, 0Eh` at 0x4717B2), `SWITCH_CONTINUED` 0x0872 (`C18`, `mov bl, 12h`
at 0x4717BC), `DECLARE_MISSION_FLAG` 0x0180 (skips the type byte unread and reads a `u16`: 3
inline bytes, 0x47CE00), `SAVE_STRING_TO_DEBUG_FILE` 0x05B6 (copies 0x80 raw bytes, 0x48FAEC;
missions contain these debug strings with stack garbage after the text). `RETURN` also writes
`m_IP` (pops the gosub stack).

For non-exact opcodes the decoder tries every candidate list and keeps the one after which the
most following instructions (up to 8) decode strictly; ties use the list the opcode's other
occurrences select (2 passes). Every selection agrees with the exe's longest path, see the test
notes (e.g. `CREATE_MENU` 9, `SET_MENU_COLUMN` 15, `CREATE_RANDOM_CHAR` 4). Weakest:
`TASK_PLAY_ANIM_WITH_FLAGS` 0x088A (`C1 T T C?`), 1 occurrence, selected as 11.

### Decoding and coverage

Control flow from each entry (main: 0xDAA8; missions and streamed scripts: offset 0), following
`GOTO`, `GOTO_IF_*`, `GOSUB`, `START_NEW_SCRIPT`, `LAUNCH_MISSION` and every `SWITCH_*` label
(unused switch pairs hold value -1 and a valid label), stopping at `GOTO`, `RETURN`,
`TERMINATE_THIS_SCRIPT`. Every jump target must land on an instruction start inside its script.

| | bytes | covered | instructions | unreached (all decode linearly = dead code) |
|---|---|---|---|---|
| main | 138,149 | 133,745 | 16,096 | 4,404 in 31 ranges |
| 135 missions | 2,885,619 | 2,869,452 | 342,093 | 16,167 in 372 ranges |
| 78 streamed | 498,487 | 481,375 | 54,758 | 17,112 in 215 ranges |

0 decode failures, 0 unreached ranges that are data or undecodable. 1,568 distinct opcodes
used (1,524 with an exact list from the exe, 44 selected by the data). `catalog/script-usage.csv`:
opcode, name, handler, source of the list, counts in main / missions / streamed, missions (index
and `SCRIPT_NAME`) and streamed scripts using it.

## Open questions (FMT-3)

- Saves: verify against real files (needs a save from the original; SAVE milestone).
- Carrec: the in-memory result of `SmoothRecording` (float division of `u32` times) is not modelled.
- GXT: `toupper` follows the CRT locale; whether the game changes the locale was not checked
  (keys are ASCII in the install, so no effect).
- IFP: what the exe does with ANPK files whose ANIM section is not 44 bytes (none in the install).
- SCM: 55 opcodes have no exact read list from the static analysis (44 partial, 9 path-dependent,
  1 unknown); the data-selected lists should be confirmed by tracing the handlers (L2) or by
  extending the executor (`TASK_PLAY_ANIM_WITH_FLAGS` has a single occurrence).
- SCM: whether `aaa.scm` is read by anything (CheckStreamedScriptVersion is a stub in 1.0 US).
- Audio: see the open questions of the audio section below.

## Audio containers

Code: `crates/sa-formats/src/audio/{config,sfx,stream}.rs`. Whole-install check:
`SA_GAME_DIR=… cargo test -p sa-formats --release --test game_data_audio -- --ignored --nocapture`.
The install is the Steam 3.0 data set run with the 1.0 US exe (ROADMAP decision); findings about
file contents (counts, the empty `StrmPaks.dat` slot, the stale track sizes) may differ from retail
1.0 data. Directory names are mixed case in the install (`audio/CONFIG`, `audio/sfx`,
`audio/streams`); the exe uses upper case and Windows ignores case.

### Index files (`audio/CONFIG`)

Every index file is read in one call into an array of fixed records, so the parsers keep every
byte (padding and string tails hold MSVC debug-heap fill `0xCC`/`0xCD`) and round-trip exactly.

| file | loader (1.0 US) | open | layout | install |
|---|---|---|---|---|
| BankLkup.dat | `CAEBankLoader::LoadBankLookupFile` 0x4DFBD0 | `CFileMgr` `"rb"` | `size / 12` x {`u8` pak, 3 pad (`CC CC CC`), `u32` byte offset, `u32` sample-data size} | 710 banks |
| PakFiles.dat | `CAEBankLoader::LoadSFXPakLookupFile` 0x4DFC70 | `"rb"` | `size / 0x34` x {`char[12]` name, `u32[10]`} (`imul eax, eax, 0x34`); each name is opened as `AUDIO\SFX\<name>` with `CdStreamOpen` | 9 paks, `u32[10]` all 0 |
| BankSlot.dat | `CAEBankLoader::LoadBankSlotFile` 0x4E0590 | `"rb"` | `u16` count, then `size - 2` bytes into `count` x 0x12D4 (`CAEBankSlot`: `u32` offset, `u32` size, 2 x `u32` unused, `i16` bank, `i16` sound count, 400 x 12-byte sound entries) | 45 slots, buffer 8,578,144 bytes |
| StrmPaks.dat | `CAEMP3TrackLoader::LoadStreamPackTable` 0x4E0970 | `CreateFileA` | `size >> 4` x `char[16]` (`shr eax, 4` at 0x4E09D6) | 17 entries, entry 2 empty |
| TrakLkup.dat | `CAEMP3TrackLoader::LoadTrackLookupTable` 0x4E09F0 | `CreateFileA` | `size / 12` x {`u8` pack, 3 pad (`CD CD CD`), `u32` offset, `u32` size} | 1,922 tracks |
| EventVol.dat | inline in `CAudioEngine::Initialise` 0x5B9C60 (0x5B9D59–0x5B9D9B) | `"r"` (0x85A53C) | `fread(buf, 1, 0xB159)`; must return 0xB159 = 45,401, else init fails | 45,402 bytes: the last byte is never read; 597 events != -128 |

- `CalculateBankSlotsInfosOffsets` (0x4DFBA0) overwrites each slot's offset with the running sum
  of the sizes (stride 0x12D4), so the stored offsets are stale (all 45 differ). The SFX buffer
  is `last offset + last size`.
- `LoadBankSlotFile` reads `size - 2` bytes into an array sized from the count; the file is exactly
  `2 + 45 x 0x12D4`. A longer file would overflow the heap block.
- `EventVol.dat` is binary data opened in text mode. In the read range there is no 0x0D and no
  0x1A byte, so the CRT translation is the identity for the shipped file; the parser rejects input
  containing either byte instead of modelling the translation.
- `StrmPaks.dat` entry 2 has an empty name and no track uses it (Steam data; the 16 named packs
  exist). `GetTrackInfo` (0x4E0A70) and `GetDataStream` (0x4E0D20) bound the pack index with
  `jbe` (`packId <= count` passes, one past the end); `GetTrackInfo` also bounds the track id with
  `ja` (`trackId <= count` passes) where `GetDataStream` uses `jae`.

### SFX bank packs (`audio/SFX/*`)

`CAEMP3BankLoader::Service` (0x4DFE30; port in gta-reversed): a bank at `BankLkup` offset `o`
with data size `n` is a 0x12C4-byte header (`AEAudioStream`: `i16` sound count, 2 bytes, 400 x
{`u32` offset from the end of the header, `u32` loop start, `u16` sample rate, `i16` headroom})
followed by `n` bytes of sample data. Whole-bank loads read `0x12C4 + n` bytes starting at sector
`o / 2048` (the pointer is advanced by `o % 2048`); single-sound loads read the header, then
`sounds[id + 1].offset - sounds[id].offset` bytes (`n - offset` for the last sound).

- Samples are 16-bit mono PCM: `CAEStaticChannel::SetAudioBuffer` (0x4F0C40) builds the
  WAVEFORMATEX with `nChannels = 1` (`mov word [ebp+0x49], 1`), `nBlockAlign = 2`,
  `wBitsPerSample = 16` (0x4F0D9F–0x4F0DA5).
- Loop start is in samples: `SetAudioBuffer` receives it as a 16-bit argument, `0xFFFF` = no loop,
  byte offset `= movsx(x) << 4 >> 3` (0x4F0CD9). The file field is a `u32`; 2 sounds have a loop
  start > 0x7FFF, which the 16-bit argument truncates.
- Install: 9 paks (2,291,472,732 bytes), 710 banks, 61,993 sounds, 2,288,061,892 sample bytes.
  The banks tile every pak exactly (no gap, no overlap: each bank starts at the previous
  `offset + 0x12C4 + n`). All 710 headers round-trip; the header pad and all unused entries are 0;
  every sound size is even and non-zero; 351 sounds loop. Sample rates: 103 distinct values,
  2,021–44,100 Hz (12,000 Hz for 54,273 sounds).

### Stream packs (`audio/streams/*`)

- Encryption: `CAEDataStream::FillBuffer` (0x4DC1C0) passes every buffer it reads to
  `CAEStreamTransformer::TransformBuffer` (0x4F17D0, called through the stub 0x40233E with
  `ecx = 0xB612D8`): `buf[i] ^= table[(pos + i) & 0xF]`, `pos` = the stream's current absolute
  file position (`SetFilePointer` result stored by `CAEDataStream::Initialise` 0x4DC2B0, advanced
  by each read). `CAEStreamTransformer::Initialise` (0x4F1750) stores the 16 immediates
  `EA 3B C6 A2 9E AD 12 F4 40 B9 DD 28 91 E5 F1 FE` (0x4F175D–0x4F17A8) and XORs each with its
  index (loop at 0x4F17B0), giving the key
  `EA 3A C4 A1 9A A8 14 F3 48 B0 D7 23 9D E8 FF F1` (gta-reversed's table is the same). The key
  depends only on the absolute offset, so a whole pack decrypts in one pass.
- Layout: per track, at its `TrakLkup` offset, a 0x1F84-byte `tTrackInfo` (read by
  `GetTrackInfo` 0x4E0A70, `push 0x1f84`), then the audio stream (`GetDataStream` 0x4E0D20 starts
  the `CAEDataStream` at `offset + 0x1F84`, length `size`). `tTrackInfo`: 1000 x {`u32` time,
  `u32` key} beats, 16 `u32` at 0x1F40, `u16` at 0x1F80, 2 pad bytes.
- Beats: `CAEAudioHardware::GetBeatInfo` (0x4D8FA0) scans while `key != 0`. Only the 6 tracks of
  `BEATS` have beats (856 in total); in the 1,916 others `key[0] == 0`.
- Payload: every track is exactly one Ogg logical stream (single serial, sequence 0..n, BOS on the
  first page and EOS on the last only, all 258,115 page CRCs valid) that fills the space up to
  the next track header (or the end of the pack). Vorbis identification headers: 1,882 tracks
  2 ch 32,000 Hz, 40 (AMBIENCE) 2 ch 24,000 Hz.
- The parser rebuilds every pack from the parsed headers plus the raw page bytes and re-encrypts
  it: 16 packs (1,092,619,734 bytes) byte-exact.
- `TrakLkup` size: 1,547 tracks have `size == Ogg length`; 375 tracks (6 in AA, 105 CO, 80 HC,
  91 NJ, 93 TK) have `size == Ogg length + 0x1F84`, so their stream length runs 0x1F84 bytes into
  the next track's header (or past the end of the pack for the last track). Probably a Steam
  repack artefact (those packs had tracks removed); the effect on playback depends on how the
  Vorbis callbacks use the length (`Seek(SEEK_END)` uses `start + length`; `FillBuffer` does not
  bound reads by it, see below).
- `FillBuffer` computes the readable size as `pos - length - start` (0x4DC1C3–0x4DC1D0), which
  wraps to a huge value, so reads are never clamped to the track length (gta-reversed keeps this).

### Open questions (audio)

- `tTrackInfo` words at 0x1F40: word 0 equals the Ogg length in 1,916 tracks; word 1 is 48,000
  (1,738), 24,000 (40), 0 (137), 25,137 (1) while the Vorbis rate is 32,000/24,000; in the 6 BEATS
  tracks words 0–1 are `0xCDCDCDCD` and the values sit in words 2–3. No reader was found in
  gta-reversed; whether the exe reads them is unverified (search `CAEStreamingDecoder` /
  `CAEVorbisDecoder` users in the exe).
- Whether the 375 over-long `TrakLkup` sizes change playback (end-of-track detection) in the
  1.0 US exe: needs a measurement with the stream thread.
- The 2 sounds with loop start > 0x7FFF: confirm in the caller of `SetAudioBuffer` that the
  `u32` is truncated (not clamped) before the call.
- `PakFiles.dat` `u32[10]` (gta-reversed: DVD copy LSNs) and `BankSlot.dat` unused words are not
  read on PC as far as the loaders show; kept raw.
