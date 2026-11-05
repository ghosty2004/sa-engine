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

## Open questions

- Triangle vertex order and texture-struct upper bits: confirm against `gta_sa.exe` RW stream code.
- COL4 header extra word: no COL4 file in the install; kept raw.
- TXD device id values 0/1/6 next to 2: whether the loader checks them.
