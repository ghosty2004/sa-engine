//! RenderWare 3.6 core of `gta_sa.exe` 1.0 US, phase A port (PORT-RW-1, `docs/arch/rw-port.md`).
//!
//! The functions of `docs/re/specs/RW/*.md`, ported from the 1.0 US bytes in the original memory layout
//! (RwGlobals at `[0xC97B24]`, engine-plugin modules at `G + [offset global]`, freelists, frames, ...).
//! The exe is authoritative; the specs and librw are reading aids, and every constant and offset used
//! here was re-read from the bytes.
//!
//! Same rules as the other engine crates (ADR-001): game state through `sa_core::globals!`/`GPtr`,
//! calls to other catalogued functions through `sa_core::orig!` (RW callbacks reached through function
//! tables keep calling the stored pointer, so a hooked callee is still governed by its own toggle),
//! every port registered with `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]).
//!
//! Relation to `sa-rw`: `sa-rw` is the phase B native object model of the viewer (Rust-native
//! structs built from `sa-formats`); this crate is the in-place port that shares the original's
//! layouts. [`layout`] is the single source of the 1.0 US RW layout constants; `sa-rw` keeps its own
//! types until phase B moves the ported code onto engine-owned memory.
#![no_std]
#![deny(unsafe_code)]
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod common;
pub mod hooks;
pub mod layout;

pub mod atomic;
pub mod camera;
pub mod clump;
pub mod engine;
pub mod frame;
pub mod geometry;
pub mod im3d;
pub mod light;
pub mod material;
pub mod matrix;
pub mod memory;
pub mod mesh;
pub mod tristrip;
pub mod trilist_mesh;
pub mod skin_geometry_dtor;
pub mod atomic_world_sync;
pub mod plugin;
pub mod stream;
pub mod texture;
pub mod vector;
pub mod videomode;
pub mod rxpipeline;
pub mod rx_execute;
pub mod d3d9_mesh_instance;
pub mod d3d9_skin_node;
pub mod d3d9_skin_blend;
pub mod imagemod;
pub mod worldplugin;
pub mod anim;
pub mod worldcreate;
pub mod d3d9state;
pub mod d3d9dev;
pub mod d3d9vb;
pub mod d3d9pipe;
pub mod d3d9misc;
pub mod im2d;
pub mod raster;
pub mod image;
pub mod skin;
pub mod matfx;
pub mod d3d9light;
pub mod d3d9vsmat;
pub mod d3d9node;
pub mod d3d9native;
pub mod d3d9cam;
pub mod d3d9vsgen;
pub mod image2;
pub mod core5;
pub mod d3d9inst;
pub mod rxcompile;
pub mod anisot;
pub mod d3d9sys;
pub mod d3d9rast;
pub mod d3d9rastrender;
pub mod clumpstream;
pub mod bmp_read;
pub mod hanim_stream;
pub mod matfx_stream;
pub mod skin_stream;
pub mod rw_helpers;
pub mod hanim_hierarchy;
pub mod hanim_update;
pub mod raster_copy;
pub mod generated;
pub mod rt_quat;
pub mod d3d9_destroy_instance;
