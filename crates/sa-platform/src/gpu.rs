//! The render-device seam of phase B: the `IDirect3DDevice9` subset the ported RenderWare D3D9
//! driver (`sa-rwcore`, `_rwD3D9*`, `RwD3D9*`) calls, as a Rust trait.
//!
//! In phase A the ported RW driver calls the real `IDirect3DDevice9` (COM `vslot` calls through
//! `orig!`). In phase B `sa-game` binds those declarations to a [`D3d9Device`]; `sa-rw-wgpu`'s
//! `d3d9::WgpuDevice` implements it on wgpu (Vulkan/Metal/DX12 natively, WebGPU in the browser).
//! The trait mirrors D3D9 one to one (method names, argument meaning, enum values), so the
//! binding layer is a mechanical translation and a D3D9 call stream captured in phase A
//! (PLAN M4, F7) can be replayed through it.
//!
//! Resource lifetime follows D3D9: `create_*` returns an id, `release_*` frees it. Locks are
//! modelled as "write these bytes" calls at unlock time ([`D3d9Device::update_texture`],
//! [`D3d9Device::update_vertex_buffer`], …): RW locks, fills and unlocks without reading back,
//! except for `RwRasterLock(rwRASTERLOCKREAD)` on camera rasters (screen grabs), which is
//! [`D3d9Device::read_render_target`].
//!
//! Values are D3D9's (`d3d9types.h`): `D3DRS_*`, `D3DTSS_*`, `D3DSAMP_*`, `D3DFMT_*`,
//! `D3DPT_*`, `D3DFVF_*`, `D3DDECLTYPE_*`, `D3DTS_*`. Only the ones RenderWare and the game use
//! have named constants here; others pass through as numbers.

/// Resource ids (non-zero).
pub type TextureId = u32;
pub type VertexBufferId = u32;
pub type IndexBufferId = u32;
pub type VertexDeclId = u32;
pub type ShaderId = u32;

/// `HRESULT`-style failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum D3dError {
    /// `D3DERR_INVALIDCALL`.
    InvalidCall(String),
    /// `D3DERR_OUTOFVIDEOMEMORY` / `E_OUTOFMEMORY`.
    OutOfMemory,
    /// `D3DERR_NOTAVAILABLE`: the backend cannot do this yet (e.g. D3D9 shader bytecode).
    NotAvailable(String),
    /// `D3DERR_DEVICELOST`.
    DeviceLost,
}

impl D3dError {
    /// The `HRESULT` the ported code compares with.
    pub fn hresult(&self) -> u32 {
        match self {
            D3dError::InvalidCall(_) => 0x8876_086C,
            D3dError::OutOfMemory => 0x8876_017C,
            D3dError::NotAvailable(_) => 0x8876_086A,
            D3dError::DeviceLost => 0x8876_0868,
        }
    }
}

pub type D3dResult<T> = Result<T, D3dError>;

/// `D3DFORMAT` values RenderWare uses.
pub mod fmt {
    pub const R8G8B8: u32 = 20;
    pub const A8R8G8B8: u32 = 21;
    pub const X8R8G8B8: u32 = 22;
    pub const R5G6B5: u32 = 23;
    pub const X1R5G5B5: u32 = 24;
    pub const A1R5G5B5: u32 = 25;
    pub const A4R4G4B4: u32 = 26;
    pub const A8: u32 = 28;
    pub const X4R4G4B4: u32 = 30;
    pub const A8B8G8R8: u32 = 32;
    pub const P8: u32 = 41;
    pub const L8: u32 = 50;
    pub const A8L8: u32 = 51;
    pub const V8U8: u32 = 60;
    pub const D32: u32 = 71;
    pub const D24S8: u32 = 75;
    pub const D24X8: u32 = 77;
    pub const D16: u32 = 80;
    pub const INDEX16: u32 = 101;
    pub const INDEX32: u32 = 102;
    pub const DXT1: u32 = u32::from_le_bytes(*b"DXT1");
    pub const DXT2: u32 = u32::from_le_bytes(*b"DXT2");
    pub const DXT3: u32 = u32::from_le_bytes(*b"DXT3");
    pub const DXT4: u32 = u32::from_le_bytes(*b"DXT4");
    pub const DXT5: u32 = u32::from_le_bytes(*b"DXT5");

    /// Bytes per pixel of an uncompressed format, `None` for block or unknown formats.
    pub fn bytes_per_pixel(f: u32) -> Option<u32> {
        Some(match f {
            R8G8B8 => 3,
            A8R8G8B8 | X8R8G8B8 | A8B8G8R8 | D32 | D24S8 | D24X8 => 4,
            R5G6B5 | X1R5G5B5 | A1R5G5B5 | A4R4G4B4 | X4R4G4B4 | A8L8 | V8U8 | D16 => 2,
            A8 | P8 | L8 => 1,
            _ => return None,
        })
    }

    /// Bytes per 4x4 block of a DXT format.
    pub fn block_bytes(f: u32) -> Option<u32> {
        match f {
            DXT1 => Some(8),
            DXT2 | DXT3 | DXT4 | DXT5 => Some(16),
            _ => None,
        }
    }

    /// Size in bytes of one mip level.
    pub fn level_size(f: u32, width: u32, height: u32) -> Option<u32> {
        if let Some(b) = block_bytes(f) {
            return Some(width.div_ceil(4).max(1) * height.div_ceil(4).max(1) * b);
        }
        Some(width * height * bytes_per_pixel(f)?)
    }
}

/// `D3DRENDERSTATETYPE` values RenderWare's state cache sets.
pub mod rs {
    pub const ZENABLE: u32 = 7;
    pub const FILLMODE: u32 = 8;
    pub const SHADEMODE: u32 = 9;
    pub const ZWRITEENABLE: u32 = 14;
    pub const ALPHATESTENABLE: u32 = 15;
    pub const LASTPIXEL: u32 = 16;
    pub const SRCBLEND: u32 = 19;
    pub const DESTBLEND: u32 = 20;
    pub const CULLMODE: u32 = 22;
    pub const ZFUNC: u32 = 23;
    pub const ALPHAREF: u32 = 24;
    pub const ALPHAFUNC: u32 = 25;
    pub const DITHERENABLE: u32 = 26;
    pub const ALPHABLENDENABLE: u32 = 27;
    pub const FOGENABLE: u32 = 28;
    pub const SPECULARENABLE: u32 = 29;
    pub const FOGCOLOR: u32 = 34;
    pub const FOGTABLEMODE: u32 = 35;
    pub const FOGSTART: u32 = 36;
    pub const FOGEND: u32 = 37;
    pub const FOGDENSITY: u32 = 38;
    pub const RANGEFOGENABLE: u32 = 48;
    pub const STENCILENABLE: u32 = 52;
    pub const STENCILFAIL: u32 = 53;
    pub const STENCILZFAIL: u32 = 54;
    pub const STENCILPASS: u32 = 55;
    pub const STENCILFUNC: u32 = 56;
    pub const STENCILREF: u32 = 57;
    pub const STENCILMASK: u32 = 58;
    pub const STENCILWRITEMASK: u32 = 59;
    pub const TEXTUREFACTOR: u32 = 60;
    pub const WRAP0: u32 = 128;
    pub const CLIPPING: u32 = 136;
    pub const LIGHTING: u32 = 137;
    pub const AMBIENT: u32 = 139;
    pub const FOGVERTEXMODE: u32 = 140;
    pub const COLORVERTEX: u32 = 141;
    pub const LOCALVIEWER: u32 = 142;
    pub const NORMALIZENORMALS: u32 = 143;
    pub const DIFFUSEMATERIALSOURCE: u32 = 145;
    pub const SPECULARMATERIALSOURCE: u32 = 146;
    pub const AMBIENTMATERIALSOURCE: u32 = 147;
    pub const EMISSIVEMATERIALSOURCE: u32 = 148;
    pub const VERTEXBLEND: u32 = 151;
    pub const CLIPPLANEENABLE: u32 = 152;
    pub const POINTSIZE: u32 = 154;
    pub const COLORWRITEENABLE: u32 = 168;
    pub const BLENDOP: u32 = 171;
    pub const SCISSORTESTENABLE: u32 = 174;
    pub const SLOPESCALEDEPTHBIAS: u32 = 175;
    pub const DEPTHBIAS: u32 = 195;
    pub const SEPARATEALPHABLENDENABLE: u32 = 206;
    pub const SRCBLENDALPHA: u32 = 207;
    pub const DESTBLENDALPHA: u32 = 208;
    pub const BLENDOPALPHA: u32 = 209;
    /// One past the largest `D3DRENDERSTATETYPE` (210 = `D3DRS_BLENDOPALPHA` + 1).
    pub const COUNT: usize = 210;
}

/// `D3DTEXTURESTAGESTATETYPE`.
pub mod tss {
    pub const COLOROP: u32 = 1;
    pub const COLORARG1: u32 = 2;
    pub const COLORARG2: u32 = 3;
    pub const ALPHAOP: u32 = 4;
    pub const ALPHAARG1: u32 = 5;
    pub const ALPHAARG2: u32 = 6;
    pub const TEXCOORDINDEX: u32 = 11;
    pub const TEXTURETRANSFORMFLAGS: u32 = 24;
    pub const COLORARG0: u32 = 26;
    pub const ALPHAARG0: u32 = 27;
    pub const RESULTARG: u32 = 28;
    pub const CONSTANT: u32 = 32;
    pub const COUNT: usize = 33;
}

/// `D3DSAMPLERSTATETYPE`.
pub mod samp {
    pub const ADDRESSU: u32 = 1;
    pub const ADDRESSV: u32 = 2;
    pub const ADDRESSW: u32 = 3;
    pub const BORDERCOLOR: u32 = 4;
    pub const MAGFILTER: u32 = 5;
    pub const MINFILTER: u32 = 6;
    pub const MIPFILTER: u32 = 7;
    pub const MIPMAPLODBIAS: u32 = 8;
    pub const MAXMIPLEVEL: u32 = 9;
    pub const MAXANISOTROPY: u32 = 10;
    pub const COUNT: usize = 14;
}

/// `D3DTRANSFORMSTATETYPE`.
pub mod ts {
    pub const VIEW: u32 = 2;
    pub const PROJECTION: u32 = 3;
    pub const TEXTURE0: u32 = 16;
    pub const WORLD: u32 = 256;
}

/// `D3DPRIMITIVETYPE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PrimitiveType {
    PointList = 1,
    LineList = 2,
    LineStrip = 3,
    TriangleList = 4,
    TriangleStrip = 5,
    TriangleFan = 6,
}

impl PrimitiveType {
    pub fn from_d3d(v: u32) -> Option<PrimitiveType> {
        Some(match v {
            1 => PrimitiveType::PointList,
            2 => PrimitiveType::LineList,
            3 => PrimitiveType::LineStrip,
            4 => PrimitiveType::TriangleList,
            5 => PrimitiveType::TriangleStrip,
            6 => PrimitiveType::TriangleFan,
            _ => return None,
        })
    }

    /// Vertices (or indices) `count` primitives use.
    pub fn vertex_count(self, count: u32) -> u32 {
        match self {
            PrimitiveType::PointList => count,
            PrimitiveType::LineList => count * 2,
            PrimitiveType::LineStrip => count + 1,
            PrimitiveType::TriangleList => count * 3,
            PrimitiveType::TriangleStrip | PrimitiveType::TriangleFan => count + 2,
        }
    }
}

/// `D3DFVF_*` bits.
pub mod fvf {
    pub const XYZ: u32 = 0x002;
    pub const XYZRHW: u32 = 0x004;
    pub const POSITION_MASK: u32 = 0x400E;
    pub const NORMAL: u32 = 0x010;
    pub const PSIZE: u32 = 0x020;
    pub const DIFFUSE: u32 = 0x040;
    pub const SPECULAR: u32 = 0x080;
    pub const TEXCOUNT_MASK: u32 = 0xF00;
    pub const TEXCOUNT_SHIFT: u32 = 8;
}

/// `D3DDECLTYPE`.
pub mod decltype {
    pub const FLOAT1: u8 = 0;
    pub const FLOAT2: u8 = 1;
    pub const FLOAT3: u8 = 2;
    pub const FLOAT4: u8 = 3;
    pub const D3DCOLOR: u8 = 4;
    pub const UBYTE4: u8 = 5;
    pub const SHORT2: u8 = 6;
    pub const SHORT4: u8 = 7;
    pub const UBYTE4N: u8 = 8;
    pub const SHORT2N: u8 = 9;
    pub const SHORT4N: u8 = 10;
    pub const USHORT2N: u8 = 11;
    pub const USHORT4N: u8 = 12;
    pub const FLOAT16_2: u8 = 15;
    pub const FLOAT16_4: u8 = 16;
    pub const UNUSED: u8 = 17;

    /// Size in bytes.
    pub fn size(t: u8) -> Option<u32> {
        Some(match t {
            FLOAT1 | D3DCOLOR | UBYTE4 | SHORT2 | UBYTE4N | SHORT2N | USHORT2N | FLOAT16_2 => 4,
            FLOAT2 | SHORT4 | SHORT4N | USHORT4N | FLOAT16_4 => 8,
            FLOAT3 => 12,
            FLOAT4 => 16,
            _ => return None,
        })
    }
}

/// `D3DDECLUSAGE`.
pub mod declusage {
    pub const POSITION: u8 = 0;
    pub const BLENDWEIGHT: u8 = 1;
    pub const BLENDINDICES: u8 = 2;
    pub const NORMAL: u8 = 3;
    pub const PSIZE: u8 = 4;
    pub const TEXCOORD: u8 = 5;
    pub const TANGENT: u8 = 6;
    pub const BINORMAL: u8 = 7;
    pub const POSITIONT: u8 = 9;
    pub const COLOR: u8 = 10;
    pub const FOG: u8 = 11;
}

/// `D3DVERTEXELEMENT9`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VertexElement {
    pub stream: u16,
    pub offset: u16,
    pub decl_type: u8,
    pub method: u8,
    pub usage: u8,
    pub usage_index: u8,
}

impl VertexElement {
    /// The vertex declaration equivalent to an FVF code (what `SetFVF` means), with the stride.
    pub fn from_fvf(code: u32) -> Option<(Vec<VertexElement>, u32)> {
        let mut v = Vec::new();
        let mut off = 0u16;
        let mut push = |decl_type: u8, usage: u8, usage_index: u8, off: &mut u16| {
            v.push(VertexElement {
                stream: 0,
                offset: *off,
                decl_type,
                method: 0,
                usage,
                usage_index,
            });
            *off += decltype::size(decl_type).unwrap_or(0) as u16;
        };
        match code & fvf::POSITION_MASK {
            fvf::XYZ => push(decltype::FLOAT3, declusage::POSITION, 0, &mut off),
            fvf::XYZRHW => push(decltype::FLOAT4, declusage::POSITIONT, 0, &mut off),
            0 => {}
            _ => return None, // blend weights (XYZB1..5) are not used by RW's FVF paths
        }
        if code & fvf::NORMAL != 0 {
            push(decltype::FLOAT3, declusage::NORMAL, 0, &mut off);
        }
        if code & fvf::PSIZE != 0 {
            push(decltype::FLOAT1, declusage::PSIZE, 0, &mut off);
        }
        if code & fvf::DIFFUSE != 0 {
            push(decltype::D3DCOLOR, declusage::COLOR, 0, &mut off);
        }
        if code & fvf::SPECULAR != 0 {
            push(decltype::D3DCOLOR, declusage::COLOR, 1, &mut off);
        }
        let n = (code & fvf::TEXCOUNT_MASK) >> fvf::TEXCOUNT_SHIFT;
        for i in 0..n {
            // D3DFVF_TEXCOORDSIZEn bits (16 + 2i): 0 = 2 floats, 1 = 3, 2 = 4, 3 = 1
            let t = match (code >> (16 + 2 * i)) & 3 {
                0 => decltype::FLOAT2,
                1 => decltype::FLOAT3,
                2 => decltype::FLOAT4,
                _ => decltype::FLOAT1,
            };
            push(t, declusage::TEXCOORD, i as u8, &mut off);
        }
        Some((v, off as u32))
    }
}

/// `D3DUSAGE`/`D3DPOOL`-level description of a texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureDesc {
    pub width: u32,
    pub height: u32,
    /// Mip levels (0 = full chain, as `CreateTexture(Levels = 0)`).
    pub levels: u32,
    pub format: u32,
    /// `D3DUSAGE_RENDERTARGET` (camera rasters).
    pub render_target: bool,
    /// `D3DUSAGE_DEPTHSTENCIL` (Z rasters).
    pub depth_stencil: bool,
    /// `D3DUSAGE_DYNAMIC`.
    pub dynamic: bool,
    /// Cube map (`CreateCubeTexture`): six faces per level, face index in `update_texture`.
    pub cube: bool,
}

/// `D3DVIEWPORT9`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub min_z: f32,
    pub max_z: f32,
}

/// `D3DCOLORVALUE` (r, g, b, a).
pub type ColorValue = [f32; 4];

/// `D3DMATERIAL9`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Material {
    pub diffuse: ColorValue,
    pub ambient: ColorValue,
    pub specular: ColorValue,
    pub emissive: ColorValue,
    pub power: f32,
}

/// `D3DLIGHT9`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Light {
    /// `D3DLIGHTTYPE`: 1 point, 2 spot, 3 directional.
    pub kind: u32,
    pub diffuse: ColorValue,
    pub specular: ColorValue,
    pub ambient: ColorValue,
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub range: f32,
    pub falloff: f32,
    pub attenuation: [f32; 3],
    pub theta: f32,
    pub phi: f32,
}

/// `D3DCLEAR_*` flags.
pub mod clear {
    pub const TARGET: u32 = 1;
    pub const ZBUFFER: u32 = 2;
    pub const STENCIL: u32 = 4;
}

/// The capabilities RenderWare queries (`D3DCAPS9` fields it reads in `_rwD3D9System`/the raster
/// format checks), reduced to what the backend can honour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caps {
    pub max_texture_width: u32,
    pub max_texture_height: u32,
    pub max_anisotropy: u32,
    pub max_active_lights: u32,
    pub max_simultaneous_textures: u32,
    /// `VertexShaderVersion`/`PixelShaderVersion` as D3D encodes them (0xFFFE0101 = vs_1_1).
    pub vertex_shader_version: u32,
    pub pixel_shader_version: u32,
    pub dxt: bool,
    pub non_pow2_textures: bool,
}

/// The `IDirect3DDevice9` subset of the RW D3D9 driver, plus the swap-chain operations of
/// `_rwD3D9CameraEndUpdate`/`RasterShowRaster` ([`D3d9Device::present`]).
///
/// The device is used from the game thread only (D3D9 device without `D3DCREATE_MULTITHREADED`,
/// as the game creates it).
pub trait D3d9Device {
    fn caps(&self) -> Caps;
    /// Back-buffer size (the `D3DPRESENT_PARAMETERS` the device was created with or reset to).
    fn back_buffer_size(&self) -> (u32, u32);
    /// `Reset` with a new back-buffer size (window resize / video mode change). Resources survive
    /// (wgpu has no lost-device state); render-target contents are undefined afterwards.
    fn reset(&mut self, width: u32, height: u32) -> D3dResult<()>;

    // ---------------------------------------------------------------------------- resources ---
    fn create_texture(&mut self, desc: &TextureDesc) -> D3dResult<TextureId>;
    /// `LockRect(level)` + writes + `UnlockRect`: `data` holds rows of `pitch` bytes (block rows
    /// for DXT) covering the rectangle `rect` = (x, y, w, h), or the whole level when `None`.
    /// `face` is the cube face (0 for 2D textures).
    fn update_texture(
        &mut self,
        tex: TextureId,
        face: u32,
        level: u32,
        rect: Option<(u32, u32, u32, u32)>,
        data: &[u8],
        pitch: u32,
    ) -> D3dResult<()>;
    fn release_texture(&mut self, tex: TextureId);
    fn create_vertex_buffer(&mut self, size: u32, dynamic: bool) -> D3dResult<VertexBufferId>;
    /// `Lock(offset, …)` + writes + `Unlock` (`D3DLOCK_DISCARD`/`NOOVERWRITE` only change
    /// synchronisation, which the backend handles).
    fn update_vertex_buffer(&mut self, vb: VertexBufferId, offset: u32, data: &[u8]) -> D3dResult<()>;
    fn release_vertex_buffer(&mut self, vb: VertexBufferId);
    /// `format`: [`fmt::INDEX16`] or [`fmt::INDEX32`].
    fn create_index_buffer(&mut self, size: u32, format: u32, dynamic: bool) -> D3dResult<IndexBufferId>;
    fn update_index_buffer(&mut self, ib: IndexBufferId, offset: u32, data: &[u8]) -> D3dResult<()>;
    fn release_index_buffer(&mut self, ib: IndexBufferId);
    fn create_vertex_declaration(&mut self, elements: &[VertexElement]) -> D3dResult<VertexDeclId>;
    fn release_vertex_declaration(&mut self, decl: VertexDeclId);
    /// D3D9 shader bytecode (`D3DXAssembleShader` output or the game's precompiled shaders).
    fn create_vertex_shader(&mut self, bytecode: &[u32]) -> D3dResult<ShaderId>;
    fn create_pixel_shader(&mut self, bytecode: &[u32]) -> D3dResult<ShaderId>;
    fn release_shader(&mut self, shader: ShaderId);

    // -------------------------------------------------------------------------------- state ---
    fn set_render_state(&mut self, state: u32, value: u32);
    fn render_state(&self, state: u32) -> u32;
    fn set_texture_stage_state(&mut self, stage: u32, kind: u32, value: u32);
    fn set_sampler_state(&mut self, sampler: u32, kind: u32, value: u32);
    fn set_texture(&mut self, stage: u32, tex: Option<TextureId>);
    /// `SetTransform`: a row-major D3D matrix (row vectors, `v' = v * M`).
    fn set_transform(&mut self, kind: u32, m: &[[f32; 4]; 4]);
    fn set_material(&mut self, m: &Material);
    fn set_light(&mut self, index: u32, l: &Light);
    fn light_enable(&mut self, index: u32, on: bool);
    fn set_viewport(&mut self, vp: &Viewport);
    /// `SetScissorRect`: a D3D `RECT` (left, top, right, bottom), unlike `update_texture`'s
    /// (x, y, w, h).
    fn set_scissor_rect(&mut self, rect: (u32, u32, u32, u32));
    fn set_fvf(&mut self, fvf: u32);
    fn set_vertex_declaration(&mut self, decl: Option<VertexDeclId>);
    fn set_vertex_shader(&mut self, shader: Option<ShaderId>);
    fn set_pixel_shader(&mut self, shader: Option<ShaderId>);
    fn set_vertex_shader_constant_f(&mut self, start: u32, data: &[[f32; 4]]);
    fn set_pixel_shader_constant_f(&mut self, start: u32, data: &[[f32; 4]]);
    /// `SetVertexShaderConstantI` (loop counters `i0..i15`).
    fn set_vertex_shader_constant_i(&mut self, start: u32, data: &[[i32; 4]]);
    fn set_pixel_shader_constant_i(&mut self, start: u32, data: &[[i32; 4]]);
    /// `SetVertexShaderConstantB` (`b0..b15`).
    fn set_vertex_shader_constant_b(&mut self, start: u32, data: &[bool]);
    fn set_pixel_shader_constant_b(&mut self, start: u32, data: &[bool]);
    fn set_stream_source(&mut self, stream: u32, vb: Option<VertexBufferId>, offset: u32, stride: u32);
    fn set_indices(&mut self, ib: Option<IndexBufferId>);
    /// `SetRenderTarget(0, …)`/`SetDepthStencilSurface`: a render-target texture (camera raster)
    /// or `None` for the back buffer. `depth: None` with a colour target gives the backend's own
    /// depth buffer of the target's size (RW always pairs a camera raster with a Z raster; D3D
    /// would disable depth testing, which RW never relies on).
    fn set_render_target(&mut self, color: Option<TextureId>, depth: Option<TextureId>);

    // ------------------------------------------------------------------------------- frames ---
    fn begin_scene(&mut self) -> D3dResult<()>;
    fn end_scene(&mut self) -> D3dResult<()>;
    /// `Clear(0, NULL, flags, color, z, stencil)` on the current viewport. `color` is a
    /// `D3DCOLOR` (0xAARRGGBB).
    fn clear(&mut self, flags: u32, color: u32, z: f32, stencil: u32);
    fn draw_primitive(&mut self, prim: PrimitiveType, start_vertex: u32, count: u32) -> D3dResult<()>;
    fn draw_indexed_primitive(
        &mut self,
        prim: PrimitiveType,
        base_vertex: i32,
        min_index: u32,
        num_vertices: u32,
        start_index: u32,
        count: u32,
    ) -> D3dResult<()>;
    /// `DrawPrimitiveUP`: vertices from user memory (`stride` bytes each) with the current
    /// FVF/declaration (Im2D/Im3D).
    fn draw_primitive_up(
        &mut self,
        prim: PrimitiveType,
        count: u32,
        vertices: &[u8],
        stride: u32,
    ) -> D3dResult<()>;
    #[allow(clippy::too_many_arguments)]
    fn draw_indexed_primitive_up(
        &mut self,
        prim: PrimitiveType,
        min_index: u32,
        num_vertices: u32,
        count: u32,
        indices: &[u8],
        index_format: u32,
        vertices: &[u8],
        stride: u32,
    ) -> D3dResult<()>;
    /// `Present`: shows the back buffer (on the window surface, if any).
    fn present(&mut self) -> D3dResult<()>;
    /// Reads the back buffer (or a render-target texture) as RGBA8 rows (screen grabs, tests).
    fn read_render_target(&mut self, tex: Option<TextureId>) -> D3dResult<(u32, u32, Vec<u8>)>;
    /// `StretchRect` between render targets (`None` = the back buffer, else a render-target
    /// texture): the rectangle (x, y, w, h; `None` = the whole target) of `src` scaled into the
    /// one of `dst`, bilinear when `linear`, else nearest. Also `GetRenderTargetData`'s and the RW
    /// camera-texture copies' building block (added by STD-4).
    fn stretch_rect(
        &mut self,
        src: Option<TextureId>,
        src_rect: Option<(u32, u32, u32, u32)>,
        dst: Option<TextureId>,
        dst_rect: Option<(u32, u32, u32, u32)>,
        linear: bool,
    ) -> D3dResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fvf_declarations() {
        // RW Im2D vertex: XYZRHW | DIFFUSE | TEX1
        let (d, stride) = VertexElement::from_fvf(fvf::XYZRHW | fvf::DIFFUSE | 0x100).unwrap();
        assert_eq!(stride, 28);
        assert_eq!(
            d.iter().map(|e| (e.offset, e.usage)).collect::<Vec<_>>(),
            vec![
                (0, declusage::POSITIONT),
                (16, declusage::COLOR),
                (20, declusage::TEXCOORD)
            ]
        );
        // Im3D: XYZ | NORMAL | DIFFUSE | TEX1
        let (_, stride) = VertexElement::from_fvf(fvf::XYZ | fvf::NORMAL | fvf::DIFFUSE | 0x100).unwrap();
        assert_eq!(stride, 36);
    }

    #[test]
    fn sizes() {
        assert_eq!(fmt::level_size(fmt::DXT1, 64, 64), Some(2048));
        assert_eq!(fmt::level_size(fmt::DXT5, 2, 2), Some(16));
        assert_eq!(fmt::level_size(fmt::A8R8G8B8, 3, 2), Some(24));
        assert_eq!(PrimitiveType::TriangleFan.vertex_count(4), 6);
    }
}
