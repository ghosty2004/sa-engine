//! RENDER subsystem (game side) of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-render`,
//! PORT-RENDER, `docs/arch/render-port.md`): `CRenderer` visibility and sector scanning, `CCullZones`,
//! `CSprite` / `CSprite2d`, `CShadows`, `CCoronas`, `CPointLights`, `CBrightLights`, `CClouds`, the
//! render side of `CWeather`, `CPostEffects`, `CBulletTraces`, `CSpecialFX`, `CMirrors` and the FX system.
//!
//! Same rules as sa-core, sa-stream, sa-world, sa-pop and sa-cam (ADR-001): game state through
//! `sa_core::globals!`/`GPtr`, calls to other catalogued functions (RenderWare and D3D included) through
//! `sa_core::orig!` with the original address, every port registered with `#[hook]` and listed in its
//! module's `HOOKS` ([`hooks::ALL`]). Ported from the 1.0 US bytes; gta-reversed and
//! `docs/re/specs/RENDER/*.md` are reading aids. Where they differ, the bytes win.
#![no_std]
#![deny(unsafe_code)]
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod common;
pub mod hooks;

pub mod bullettraces;
pub mod clouds;
pub mod coronas;
pub mod cullzones;
pub mod fx;
pub mod mirrors;
pub mod occlusion;
pub mod pointlights;
pub mod posteffects;
pub mod renderer;
pub mod shadows;
pub mod specialfx;
pub mod sprite;
pub mod sprite2d;
pub mod weather;

pub mod carplates;
pub mod glass;
pub mod markers;
pub mod stencilshadows;
pub mod timecycle;
pub mod waterlevel;
