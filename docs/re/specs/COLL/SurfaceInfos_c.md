# SurfaceInfos_c loaders (COLL)

`SurfaceInfos_c` (size 0x8f4, global `g_surfaceInfos` 0xb79538): `float m_adhesiveLimits[6][6]` at +0x0,
`SurfaceInfo_c m_surfaces[]` at +0x90, stride 0xc. Entry layout (gta-reversed SurfaceInfo_c.h, offsets confirmed
by the bytes and by `IsSeeThrough` 0x55e6b0 which reads `this + id*0xc + 0x94` bit 12):

| entry off | absolute | field |
|---|---|---|
| +0 | +0x90 | `int8 tyreGrip` |
| +1 | +0x91 | `int8 wetGrip` |
| +2..3 | | padding, never written by the loaders |
| +4 | +0x94 | `uint32 flags1` |
| +8 | +0x98 | `uint32 flags2` |

`flags1` bits: 0-2 adhesion group, 3-4 skid-mark type, 5-7 friction effect, 8-10 bullet FX, 11 softLand,
12 seeThrough, 13 shootThrough, 14 sand, 15 water, 16 shallowWater, 17 beach, 18 steepSlope, 19 glass, 20 stairs,
21 skateable, 22 pavement, 23-24 roughness, 25-26 flammability, 27 createsSparks, 28 cantSprintOn,
29 leavesFootsteps, 30 producesFootDust, 31 makesCarDirty.
`flags2` bits: 0 makesCarClean, 1 wheelGrass, 2 wheelGravel, 3 wheelMud, 4 wheelDust, 5 wheelSand, 6 wheelSpray,
7 createsPlants, 8 createsObjects, 9 canClimb, 10 audioConcrete, 11 audioGrass, 12 audioSand, 13 audioGravel,
14 audioWood, 15 audioWater, 16 audioMetal, 17 audioLongGrass, 18 audioTile, 19-31 untouched.

Shared helpers (via `orig!`): `CFileMgr::OpenFile` 0x538900 (`__cdecl FILE*(const char* name, const char* mode)`),
`CFileLoader::LoadLine` 0x536f80 (`__cdecl char*(FILE*)`, NULL at EOF), `CFileMgr::CloseFile` 0x5389d0
(`__cdecl (FILE*)`), `SurfaceInfos_c::GetSurfaceIdFromName` 0x55d220 (`__thiscall uint32(const char*)`, `ret 4`;
returns 0 = DEFAULT for unknown names), CRT `sscanf` 0x8220ad (cdecl, caller cleans), CRT `_ftol` 0x821b40.

String compares in these loaders are `repe cmpsb` over `strlen(literal)+1` bytes, i.e. exact equality
(`strcmp == 0`); a chain is "first match wins", and when nothing matches the field is left unchanged.

## 0x55eb90 SurfaceInfos_c::LoadSurfaceInfos  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall void (SurfaceInfos_c* this = ECX)`, no stack args, plain `ret`.
  Frame `sub esp, 0x140`; saves EBP (= this), ESI (= file), EBX (= surface id), EDI.
- Callers: 1 (`SurfaceInfos_c::Init` 0x55f420; Init calls LoadAdhesiveLimits, LoadSurfaceInfos,
  LoadSurfaceAudioInfos in that order; only LoadAdhesiveLimits calls `CFileMgr::SetDir("")`).
  Callees: as listed above.
- Globals: `.rdata` strings: `"data\\surfinfo.dat"` 0x864db4, mode `"r"` 0x85a53c, format 0x864d48,
  `"RUBBER"` 0x864978, `"HARD"` 0x864d40, `"ROAD"` 0x864d38, `"LOOSE"` 0x864d30, `"SAND"` 0x864d28,
  `"WET"` 0x864d24, `"DEFAULT"` 0x85c658, `"SANDY"` 0x864d1c, `"MUDDY"` 0x864d14, `"NONE"` 0x859e38,
  `"SPARKS"` 0x864d0c, `"WOOD"` 0x864d04, `"DUST"` 0x864cfc. Constants: 10.0f at 0x85862c, 100.0f at 0x858628.
- Locals (offsets from ESP after the 4 pushes; buffer sizes from the frame): name `char[64]` +0x110,
  adhesion `char[32]` +0x90, skid `char[32]` +0xd0, friction `char[32]` +0xf0, bulletFx `char[32]` +0xb0,
  tyreGrip f32 +0x14, wetGrip f32 +0x5c, 29 int32 columns (see table).
- Behaviour:
  1. `file = OpenFile("data\\surfinfo.dat", "r")` (no NULL check, see Edge cases).
  2. `line = LoadLine(file)`; while `line != NULL`:
     - if `line[0] == '#'` or `line[0] == 0`: skip to the next line.
     - `sscanf(line, "%s %s %f %f %s %s" + " %d" x 29 + " %s", ...)` (36 conversions; the exact format string
       is `"%s %s %f %f %s %s %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %s"`).
       The return value is ignored; locals not filled by a short line keep their previous contents (values of
       the previous line, or stack garbage on the first line).
     - `id = GetSurfaceIdFromName(name)`; `e = this + 0x90 + id*0xc` (no range check).
     - Adhesion (column 2) into flags1 bits 0-2: `RUBBER` 0, `HARD` 1, `ROAD` 2, `LOOSE` 3, `SAND` 4, `WET` 5
       (each implemented as `flags1 = (flags1 & ~clearMask) | setMask`, net effect: field = value).
     - `e.tyreGrip = (int8) _ftol(tyreGrip * 10.0f)` (0x55ee16..0x55ee29; x87 `fld f32; fmul f32` then `_ftol`
       truncation; only AL is stored).
     - `e.wetGrip = (int8) _ftol(wetGrip * 100.0f)` (0x55ee2d..0x55ee56; low byte of the signed result, so a
       negative value keeps its sign as int8).
     - Skid mark (column 5) into flags1 bits 3-4: `DEFAULT` -> 0, `SANDY` -> 2 (`& ~8 | 0x10`), `MUDDY` -> 1
       (`& ~0x10 | 8`). Note: gta-reversed's `eSkidmarkType` has SANDY = 1, MUDDY = 2; the bytes store the
       opposite numbers. Store exactly 2 for "SANDY" and 1 for "MUDDY".
     - Friction effect (column 6) into bits 5-7: `NONE` -> 0, `SPARKS` -> 1.
     - Bullet FX (column 36, last `%s`) into bits 8-10: `NONE` 0, `SPARKS` 1, `SAND` 2, `WOOD` 3, `DUST` 4.
     - The 29 integer columns, in file order, each merged as `flags = flags ^ ((v << pos ^ flags) & mask)`,
       i.e. the low bit(s) of `v` replace the field (bit 0 only for 1-bit fields, `v & 3` for 2-bit fields):

       | col | gr name | target |
       |---|---|---|
       | 7 | SoftLand | flags1 bit 11 |
       | 8 | SeeThrough | flags1 bit 12 |
       | 9 | ShootThrough | flags1 bit 13 |
       | 10 | Sand | flags1 bit 14 |
       | 11 | Water | flags1 bit 15 |
       | 12 | ShallowWater | flags1 bit 16 |
       | 13 | Beach | flags1 bit 17 |
       | 14 | SteepSlope | flags1 bit 18 |
       | 15 | Glass | flags1 bit 19 |
       | 16 | Stairs | flags1 bit 20 |
       | 17 | Skateable | flags1 bit 21 |
       | 18 | Pavement | flags1 bit 22 |
       | 19 | Roughness | flags1 bits 23-24 (`v & 3`) |
       | 20 | Flame | flags1 bits 25-26 (`v & 3`) |
       | 21 | Sparks | flags1 bit 27 |
       | 22 | Sprint | flags1 bit 28 |
       | 23 | Footsteps | flags1 bit 29 |
       | 24 | FootDust | flags1 bit 30 |
       | 25 | CarDirt | flags1 bit 31 (`flags & 0x7fffffff | v << 31`) |
       | 26 | CarClean | flags2 bit 0 |
       | 27 | WheelGrass | flags2 bit 1 |
       | 28 | WheelGravel | flags2 bit 2 |
       | 29 | WheelMud | flags2 bit 3 |
       | 30 | WheelDust | flags2 bit 4 |
       | 31 | WheelSand | flags2 bit 5 |
       | 32 | WheelSpray | flags2 bit 6 |
       | 33 | ProcPlant | flags2 bit 7 |
       | 34 | ProcObj | flags2 bit 8 |
       | 35 | Climbable | flags2 bit 9 |

       Stack slots (ESP-relative, same base as above) in column order 7..35: +0x1c, +0x8c, +0x24, +0x64, +0x2c,
       +0x88, +0x34, +0x6c, +0x3c, +0x80, +0x44, +0x74, +0x4c, +0x84, +0x54, +0x7c, +0x10, +0x18, +0x20, +0x28,
       +0x30, +0x38, +0x40, +0x48, +0x50, +0x58, +0x60, +0x68, +0x70 (from the decompile's sscanf argument list;
       consistent with the reads at 0x55efe8.. which use +0x1c for bit 11 and +0x8c for bit 12). Each flag
       word is re-stored to memory after every single field update (observable only through aliasing; none).
     - `line = LoadLine(file)`.
  3. `CloseFile(file)`.
- Edge cases / oddities: no NULL check on `file` (a missing file is passed to LoadLine; behaviour then depends on
  LoadLine 0x536f80). `%s` conversions have no width: a token longer than 63 (name) or 31 (other strings)
  characters overflows the frame (name buffer ends at the saved registers / return address). Unknown surface
  names map to id 0 and overwrite the DEFAULT entry. Flag columns use bit 0 only (value 2 stores 0).
- gta-reversed: provided (SurfaceInfos_c.cpp:301, `addr-comment` mapping, hook not reversed). Differences: FIX_BUGS
  null check (row 304), `uint32(float)` conversion (row 329), skid-mark values swapped (SANDY/MUDDY), 1-bit flags
  set with `(bool)v` instead of `v & 1`, `sscanf_s` with buffer sizes instead of unbounded `%s`. Port the bytes.
- Open questions: none affecting stock data. (Behaviour of LoadLine(NULL) belongs to 0x536f80's spec.)

## 0x55f2b0 SurfaceInfos_c::LoadSurfaceAudioInfos  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall void (SurfaceInfos_c* this = ECX)`, plain `ret`. Frame `sub esp, 0x64`;
  ESI = this, EDI = file.
- Callers: 1 (`SurfaceInfos_c::Init` 0x55f420, tail call). Callees: OpenFile, LoadLine (x2), sscanf,
  GetSurfaceIdFromName, CloseFile.
- Globals: `"data\\surfaud.dat"` 0x864de8, mode `"r"` 0x85a53c, format `"%s %d %d %d %d %d %d %d %d %d"` 0x864dc8.
- Locals (ESP after the two pushes): name `char[64]` +0x2c (runs up to the saved registers and return address), ints
  d1..d9 at +0x08, +0x0c, +0x10, +0x14, +0x18, +0x1c, +0x20, +0x24, +0x28.
- Behaviour: `file = OpenFile("data\\surfaud.dat", "r")`; for each `line = LoadLine(file)` until NULL: skip if
  `line[0]` is `'#'` or 0; `sscanf(line, fmt, name, &d1..&d9)` (result ignored); `id =
  GetSurfaceIdFromName(name)`; `f = this + 0x98 + id*0xc` (flags2); then in order, each with the same xor-merge
  as above (bit 0 of the value): d1 -> bit 10 (audioConcrete), d2 -> 11 (grass), d3 -> 12 (sand), d4 -> 13
  (gravel), d5 -> 14 (wood), d6 -> 15 (water), d7 -> 16 (metal), d8 -> 17 (longGrass), d9 -> 18 (tile).
  Finally `CloseFile(file)`.
- Edge cases: no NULL-file check; unbounded `%s` (name buffer 64 bytes); missing columns keep previous values;
  unknown names write entry 0.
- gta-reversed: provided (SurfaceInfos_c.cpp:378): same columns and bits; differences: FIX_BUGS null check
  (row 381), `(bool)v` instead of bit 0, `sscanf_s`. Port the bytes.
- Open questions: none.
