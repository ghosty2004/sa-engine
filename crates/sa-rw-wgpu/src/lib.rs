//! The Direct3D 9 seam of the native and web builds on wgpu (docs/arch/gpu.md): [`d3d9::WgpuDevice`] implements
//! `sa_platform::gpu::D3d9Device`, the device the ported RenderWare D3D9 driver (`sa-rwcore`) and the game's own
//! D3D9 calls reach through sa-game's COM objects; [`sm`] translates the game's shader-model 1–3 bytecode to WGSL;
//! [`dxt`] decodes DXT textures in software where the adapter has no BC formats.
//!
//! (The NATIVE-DEMO world renderer that used to live here, with the demo's own RenderWare loader `sa-rw`, the
//! `sa-formats` parsers and the `sa-viewer` binary, was retired by DEDUP-1 pass 2: the ported code renders the
//! world through this seam.)

pub mod d3d9;
pub mod dxt;
pub mod sm;
