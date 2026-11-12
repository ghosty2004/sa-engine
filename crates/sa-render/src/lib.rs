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
pub mod harness5;
pub mod cast_shadow_entity;

pub mod bullettraces;
pub mod clouds;
pub mod coronas;
pub mod cullzones;
pub mod draw;
pub mod fx;
pub mod mirrors;
pub mod occlusion;
pub mod pointlights;
pub mod posteffects;
pub mod renderer;
pub mod rwplugins;
pub mod shadows;
pub mod specialfx;
pub mod sprite;
pub mod sprite2d;
pub mod weather;

pub mod carplates;
pub mod glass;
pub mod markers;
pub mod stencilshadows;
pub mod stencil_register;
pub mod timecycle;
pub mod transform_vectors;
pub mod waterlevel;

pub mod mvpa_render1;
pub mod mvpa_render2;
pub mod mvpa_fx1;
pub mod mvpa_render3;
pub mod mvpa_fx2;
pub mod mvpc_fx1;
pub mod mvpc_fx2;
pub mod mvpc_render1;
pub mod mvpb_r1;
pub mod mvpd_d3d;
pub mod mvpd_r1;
pub mod mvpd_r2;
pub mod mvpd_r3;
pub mod mvpd_r4;
pub mod mvpd_r5;
pub mod quaternion;
pub mod pole_shadow;
pub mod ropes_register;
pub mod rope_update;
pub mod building_dn_render;
pub mod visibility_ped;
pub mod visibility_lists;
pub mod visibility_entity;
pub mod render_buffer_store;
pub mod grass_globals;
pub mod rw_search_callbacks;
pub mod list_remove_item;
pub mod render_lists;
pub mod mvpd_unk;
pub mod custompipes;
pub mod fx_alloc;
pub mod car_dirt;
pub mod player_skin;
pub mod clouds_render;
pub mod water_render;
pub mod coronas_render;
pub mod mirrors_render;
pub mod brightlights_render;
pub mod weather_render;
pub mod water_init;
pub mod visibility_access;
pub mod pipe_ids;
pub mod frame_node_name;
pub mod collision_plugin_mi;
pub mod car_env_stream;
pub mod car_pipe_setup;
pub mod clock_misc;
pub mod effect_2d_stream;
pub mod building_dn;
pub mod birds_render;
pub mod postfx_render;
pub mod shadows_render;
pub mod sprite_render;
pub mod vector_compare;
pub mod clouds_create;
pub mod shadows_store;
pub mod fog_render;
pub mod occluder_process;
pub mod shadows_cast;
pub mod grass_render;
pub mod specialfx_render;
pub mod weapon_effects_render;
pub mod plant_loc_tris;
pub mod fx_interp_integral;
pub mod render_tail_leaves;
pub mod short_leaf_natives;
pub mod list_item_dtor;
pub mod world_scan;
pub mod plant_col_entity_cache;
pub mod stencil_shadow_volumes;
pub mod roadsign_text_raster;
pub mod roadsign_line_row;
pub mod roadsign_atomic;
pub mod generated;
pub mod visibility_vehicle;
pub mod visibility_weapon;
pub mod heat_haze_fx;
pub mod fx_attach_to_bone;
pub mod fx_emitter_render;
pub mod plant_statics_init;
pub mod visibility_train_big_vehicle;
pub mod car_env_map_render;
pub mod visibility_vehicle;
pub mod visibility_weapon;
