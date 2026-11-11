# STD-GFX: the game's D3D9 shaders on the wgpu device

What the in-world rendering needs from the native device beyond the fixed-function emulation of
platform.md §4.1: the game's vertex/pixel shaders (translated from D3D9 bytecode to WGSL), shader
constants, cube textures, and the native `IDirect3DDevice9` shader methods. Code:
`crates/sa-rw-wgpu/src/sm/` (toolchain), `crates/sa-rw-wgpu/src/d3d9.rs` (device),
`crates/sa-platform/src/gpu.rs` (seam), `crates/sa-game/src/d3d9_shader.rs` (native COM side).

## 1. Shader inventory (gta_sa.exe 1.0 US)

Found by scanning every section for D3D9 version tokens followed by an end token, and every
indirect call through `IDirect3DDevice9` slots 91 (`CreateVertexShader`, vtable offset 0x16C) and
106 (`CreatePixelShader`, 0x1A8). Only RenderWare's wrappers call the device:

| Creator | Address | Called from | What it creates |
|---|---|---|---|
| `RwD3D9CreateVertexShader` | 0x7FAC60 | 0x760C7F in `_rpD3D9VertexShaderGenerate` (0x75F0B0) | every vertex shader of the game |
| `RwD3D9CreatePixelShader` | 0x7FACC0 | 0x816007..0x816052 in `_rpMatFXPipelinesCreate` (0x815E60) | RpMatFX's six pixel shaders |

**Vertex shaders are generated at run time.** `_rpD3D9VertexShaderGenerate` (0x75F0B0, called
from `_rpD3D9GetVertexShader` 0x75EED0 at 0x75EFA0; that one is called from
`_rxD3D9VertexShaderDefaultGetMaterialShaderCallBack` 0x76101A and RpMatFX at 0x816545, 0x816FD8,
0x8170FB) prints assembly text with `RwEngineInstance->stringFuncs.vecSprintf` (slot 0xF0) into
the buffer 0xC93AF8 from 166 format strings at 0x8D641C..0x8D736C (`vs_1_1`/`vs_2_0`/`vs_2_x`
headers, `dcl_*` lines, then fragments for skinning through `a0`, morphing, normals/tangents,
lighting, fog, texture-coordinate generation for env/bump/UV-anim effects), then calls the
statically linked D3DX `D3DXAssembleShader` (0x7652AE; arguments `(0xC93AF8, len, NULL, NULL,
D3DXSHADER_SKIPOPTIMIZATION, &buffer, NULL)`), `ID3DXBuffer::GetBufferPointer` and
`RwD3D9CreateVertexShader`. The instruction repertoire of those strings: `mov`, `mova`, `add`,
`mul`, `mad`, `dp3`, `dp4`, `max`, `slt`, `rcp`, `rsq`, `expp`, `m4x4`, `m4x3`, `m3x3`, `dcl_*`,
registers `r0..r11`, `v0..v15`, `c[n]`, `c[a0.x/y/z/w + n]`, `c[3 + 5]`-style sums, `oPos`,
`oD0/oD1`, `oT0..oT3`, `oFog`, swizzles and negation.

**Pixel shaders are precompiled** `ps_1_1` blobs in `.rdata`: 0x8852F0 (92 bytes), 0x885350 (76),
0x8853A0 (64), 0x8853E0 (136), 0x885468 (176), 0x885518 (160), created only when
`PixelShaderVersion > 1.0` (dual-pass/env-map combinations of RpMatFX: `tex`, `mul`, `mad`, `mov`,
the `1-x` modifier on `t.a`, coissued alpha `mov`, constant `c0`, three texture stages).

The game's own pipelines (`CCustomBuildingDNPipeline`, `CCustomCarEnvMapPipeline`) set
`RwD3D9SetVertexShader(mesh->vertexShader)` / `RwD3D9SetPixelShader(NULL)` (0x5DA643..0x5DA743):
fixed function on PC (the shader is whatever RW's instancing stored, NULL for them). RpSkin's D3D9
vertex-shader path goes through the same generator (`_rxD3D9SkinVertexShader*` callbacks, the
default material-shader callback). No HLSL is compiled; there are no post-effect shaders.

Other version-token hits are false positives (D3DX tables in `.text` 0x772D2C.., version name
tables in `.rdata` 0x87542C.., `.data` 0x8B1AFC.., 0x8C7A78.., 0x8DBE18..). Nothing from the exe
is stored in the repository: the tests read the blobs and format strings from the user's exe at
run time (section 5).

## 2. Toolchain (`sa_rw_wgpu::sm`)

* `parse`: token stream → `Shader` (`Instr` with `Dst`/`Src`/relative addressing/`dcl`/`def`/
  predicate), SM1 (no lengths: parameter tokens by bit 31, fixed counts for `def`/`dcl`) and SM2+
  (length field), comments skipped; `token_length` for copying a stream out of game memory;
  `Display` disassembles to D3DX syntax.
* `asm`: D3DX assembler syntax → tokens (versions 1_1..3_0 incl. `vs_2_x`/`ps_2_x`; all SM1/2
  ops, flow control, `dcl`/`def`/`defi`/`defb`, instruction and source modifiers, coissue,
  predicates, `c[a0.x + n]`/`c[n + a0.x]`/`c[3 + 5]`/`c[aL + n]`). It replaces the game's
  `D3DXAssembleShader`. Round trip: `assemble(parse(t).to_string()) == t` holds for D3DX's own
  output (the six MatFX blobs, checked at test time).
* `interp`: the reference CPU interpreter (the executable spec), written independently of the
  translator. Implements every SM1/SM2 arithmetic op, ps_1_x `tex`/`texcoord`/`texkill`, ps_1_4
  `texcrd`/`texld`/`phase`, ps_2_x `texld`/`texldp`/`texldb`/`texkill`, `rep`/`loop`/`if`/`ifc`/
  `else`/`break`/`breakc`/`call`/`callnz`/`label`/`ret`, predicates.
* `wgsl`: `translate(shader, sampler_kinds)` → WGSL functions `vs_body(v: array<vec4f, 16>) ->
  SmVsOut` / `ps_body(d0, d1, tc: array<vec4f, 8>) -> SmPsOut`, plus `PRELUDE` (constant block
  `SmConsts` at group 0 binding 1: 256 vertex float, 32 pixel float, 16+16 int, 16+16 bool
  registers; helper functions). Each instruction becomes `let _tN = <vec4f expr>;` and masked
  component writes; coissued pairs compute both before writing; `def` constants become exact
  bit-pattern literals; relative constant reads go through a bounds-checked function;
  `rep`/`loop`/`if`/`ifc`/`break(c)` become WGSL loops and branches.

### Semantics (shared by interpreter and translator)

* Source: swizzle, then modifier (`-`, `_bias`, `_bx2`, `1-`, `_x2`, `_dz`, `_dw`, `_abs`).
  Destination: shift (`_x2`.. `_d8`), then `_sat`, then the write mask.
* Scalar ops (`rcp rsq exp log expp logp pow sincos`) read lane w of the swizzled source (the
  assembler encodes `r.x` as `.xxxx`). `rcp`/`rsq`: 1 → 1, 0 → +inf; `rsq` uses `|x|`; `log` of 0
  is -inf; `expp`/`logp` give the 4-component vs_1_1 forms in SM1 and the scalar in SM2.
* `a0`: vs_1_1 `mov a0` floors, `mova` rounds half away from zero. Relative constant reads out
  of range give 0.
* Unwritten outputs are 0, except `oFog` (1: no fog). `oD0`/`oD1` are clamped to [0, 1] by the
  device before interpolation.
* ps_1_x constants (set or `def`) are clamped to [-1, 1]; `tex tN` samples stage N at texcoord N;
  `texcoord` saturates and sets w = 1; `texkill` tests x, y, z (ps_2_x: the write-mask lanes);
  ps_1_4 `texld rN` samples stage N; `phase` is a no-op; output r0 (ps_1_x) / `oC0` (ps_2_x).
* Coissued instructions read the registers from before their partner.
* Translator only: WGSL may assume finite floats, so the ±infinities above are ±`f32::MAX` on
  the GPU (tests treat |x| ≥ 3e38 as infinite); `mad` may be fused on the GPU.

## 3. Device (`WgpuDevice`)

* `create_vertex_shader`/`create_pixel_shader` decode and translate once (to fail early);
  `release_shader` drops cached modules/pipelines. Caps report vs_2_0/ps_2_0.
* Programs: `PipeKey.prog = (vs, ps, cube mask)`. A draw with a shader builds a module from the
  translation plus generated entry points over common varyings (`SV`: `oD0`, `oD1`, `oT0..7`,
  fog). Vertex shader: declaration elements go to the input register whose `dcl` has their usage
  and index (missing registers read (0, 0, 0, 1)); D3D9's half-pixel offset is applied to `oPos`;
  fog distance for table fog is `oPos.w`. Without a vertex shader the fixed-function vertex stage
  feeds `SV` (texcoord sets 0 and 1 only). Pixel shader: `ps_body` then fog blending and alpha
  test (as D3D9 applies them after ps_1_x/2_x); `oDepth` → `frag_depth`. Without a pixel shader
  the fixed-function texture stages run on `SV` (`oT0`/`oT1`).
* Constants: `set_{vertex,pixel}_shader_constant_{f,i,b}` (the `i`/`b` setters are new on the
  seam) write a `ConstBlock`; it is pushed into a per-flush uniform arena once per change and
  bound with a dynamic offset next to the fixed-function uniforms.
* Textures: the shader path binds eight stages (bindings 2s/2s+1), each 2D or cube: ps_2_x from
  `dcl_2d`/`dcl_cube`, ps_1_x from the bound texture; a mismatching texture binds white.
* Cube textures: `TextureDesc::cube` creates a 6-layer texture with a cube view; `update_texture`
  takes the face (0..5 = +X, -X, +Y, -Y, +Z, -Z). The fixed-function stages sample a cube texture
  as white.
* Render-target textures, depth surfaces, offscreen surfaces, `StretchRect`/`GetRenderTargetData`
  are STD-4's (platform.md); nothing here depends on them.

## 4. Native COM side (`sa-game`, `d3d9_shader.rs`)

Device slots 91–99 and 106–114 (create/set/get shaders, set/get float/int/bool constants) and the
shader objects (`AddRef`, `Release`, `GetDevice`, `GetFunction`) on their own vtable page
(0x34000, traps 0x80000..). `D3DXAssembleShader` (0x7652AE) is a native replacement returning an
`ID3DXBuffer` object (`GetBufferPointer`, `GetBufferSize`, `Release`; errors as an error buffer
and `E_FAIL`).

**Caps gate.** `D3DCAPS9` reports shader support only with `SA_GAME_SHADERS=1` (vs_2_0 with 256
constants, ps_2_0, `PixelShader1xMaxValue` 8). Default off: with shader caps RW instances skinned
and MatFX geometry through `_rpD3D9VertexShaderGenerate`, which is not ported yet; with the
default caps RW keeps its fixed-function paths. Switching it on by default is the main session's
call once the generator (and its callers) are ported.

## 5. Tests

`cargo test -p sa-rw-wgpu` (GPU tests on the real adapter, offscreen; skipped without one):

* `sm::gpu_tests`: each shader runs translated on the GPU (vertex shaders in a compute harness,
  pixel shaders in a fragment harness over `Rgba32Float`, textures sampled nearest at texel
  centres) and in the interpreter, on 32–160 random inputs; every output lane must agree
  (relative 2e-4). Shaders: RW-style vs_1_1 skinning + lighting + fog; vs_2_0 with `mova`, `a0.y`
  relative reads, cross products; every vs_2_0 ALU op with `def`/`defi`/`defb`, `rep`, `loop`/`aL`,
  `if`/`else`, `ifc`, `breakc`; vs_1_1 special values of `rcp`/`rsq`/`log`/`expp`/`logp`; ps_1_1
  with modifiers, coissue, `cnd`, `lrp`, `texcoord`, `texkill`, shifts, constant clamping; ps_1_4
  with phases and dependent reads; ps_2_0 with `texldp`, a cube sampler, `cmp`, `dp2add`.
* `exe_matfx_pixel_shaders` / `exe_rw_vertex_shader_fragments` (need the user's exe: `SA_GTA_EXE`
  or `re/gta/1.0-US/gta_sa.exe` above the crate): the six MatFX blobs decode, round-trip through
  the assembler byte for byte and match the interpreter on the GPU; every generator format string
  (instantiated with in-range arguments) assembles into one vs_1_1 and one vs_2_0 shader that
  match the interpreter.
* `sm::asm`, `sm::interp`, `sm::parse` unit tests (encodings, semantics).
* `d3d9::tests`: shader creation errors; a vs_2_0 shader over a two-stream declaration with
  float/int/bool constants changed between draws; a ps_1_1 shader after fixed-function vertices
  with constant clamping and the alpha test; a cube texture sampled by ps_2_0 with coordinates
  from a vertex shader. The 13 fixed-function tests are unchanged (the old "shaders not
  available" test is replaced).

`sa-game` (`--target i686-unknown-linux-gnu`): `D3DXAssembleShader` into buffer objects, error
buffer, token-stream copy.

## 6. Open gaps

* Not translated (creation returns `NotAvailable`): ps_1_x `texbem`/`texbeml`/`texm3x*`/
  `texreg2*`/`texdp3*`/`texdepth`/`bem`, SM3, subroutines (`call`/`label`), predicates,
  `dsx`/`dsy` in the interpreter. None is used by the game.
* Fixed-function vertices feeding a pixel shader provide texcoord sets 0 and 1 (2 components);
  a vertex shader feeding the fixed-function pixel stage provides `oT0`/`oT1` only (texture
  stages 2–7 and `TEXCOORDINDEX` > 1 are not emulated); ps_1_x projected texture coordinates
  (`D3DTTFF_PROJECTED`) and texture transforms are ignored; the `texldb` bias is ignored by the
  interpreter.
* Cube render targets (dynamic env maps) are `NotAvailable`; volume textures are not on the seam.
* Pixel shaders with flow control (ps_2_x) would sample in non-uniform control flow.
* `_rpD3D9VertexShaderGenerate` (0x75F0B0) and its callers are not ported; until they are, the
  shader path is reached only with `SA_GAME_SHADERS=1` and RW then stops at the generator.
* The generator's output has not been compared against the original's (phase A capture of the
  generated text/bytecode would make a test corpus; the assembler is verified on D3DX's own
  output only through the MatFX round trip).
