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
