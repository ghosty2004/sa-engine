//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after
//! `sa_cam::hooks::ALL`, in this order), and every module's globals.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

use crate::mvpc_fx1;
use crate::mvpc_fx2;
use crate::mvpc_render1;
use crate::{
    bullettraces, clouds, coronas, cullzones, fx, mirrors, occlusion, pointlights, posteffects, renderer,
    shadows, specialfx, sprite, sprite2d, weather,
};
use crate::harness5;
use crate::{carplates, draw, glass, markers, rwplugins, stencilshadows, timecycle, waterlevel};
use crate::{mvpa_fx1, mvpa_fx2, mvpa_render1, mvpa_render2, mvpa_render3};

/// Every module's registrations, in this order.
pub const MODULES: &[&[HookReg]] = &[
    renderer::HOOKS,
    occlusion::HOOKS,
    cullzones::HOOKS,
    sprite2d::HOOKS,
    sprite::HOOKS,
    clouds::HOOKS,
    weather::HOOKS,
    coronas::HOOKS,
    pointlights::HOOKS,
    shadows::HOOKS,
    bullettraces::HOOKS,
    specialfx::HOOKS,
    mirrors::HOOKS,
    posteffects::HOOKS,
    fx::HOOKS,
    waterlevel::HOOKS,
    timecycle::HOOKS,
    markers::HOOKS,
    carplates::HOOKS,
    harness5::HOOKS,
    crate::cast_shadow_entity::HOOKS,
    glass::HOOKS,
    stencilshadows::HOOKS,
    crate::stencil_register::HOOKS,
    mvpa_render1::HOOKS,
    mvpa_render2::HOOKS,
    mvpa_fx1::HOOKS,
    crate::fx_info_trail_value::HOOKS,
    crate::rw_tex_dict_find_hash::HOOKS,
    mvpa_render3::HOOKS,
    mvpa_fx2::HOOKS,
    mvpc_fx1::HOOKS,
    mvpc_fx2::HOOKS,
    mvpc_render1::HOOKS,
    crate::mvpb_r1::HOOKS,
    crate::transform_vectors::HOOKS,
    crate::mvpd_r1::HOOKS,
    crate::mvpd_d3d::HOOKS,
    crate::mvpd_r2::HOOKS,
    crate::mvpd_r3::HOOKS,
    crate::mvpd_r4::HOOKS,
    crate::mvpd_r5::HOOKS,
    crate::quaternion::HOOKS,
    crate::fx_info_get_value_samp::HOOKS,
    crate::fx_info_dir_value::HOOKS,
    crate::shadows_permanent::HOOKS,
    crate::motion_blur_streaks::HOOKS,
    crate::quad_marker_wind_index_buffer::HOOKS,
    crate::visibility_big_vehicle_boat_heli::HOOKS,
    crate::shadows_indicator_oil_birds::HOOKS,
    crate::fx_trigger_gunshot::HOOKS,
    crate::checkpoints_place_marker::HOOKS,
    crate::pole_shadow::HOOKS,
    crate::ropes_register::HOOKS,
    crate::rope_update::HOOKS,
    crate::building_dn_render::HOOKS,
    crate::visibility_ped::HOOKS,
    crate::visibility_vehicle::HOOKS,
    crate::visibility_weapon::HOOKS,
    crate::heat_haze_fx::HOOKS,
    crate::fx_attach_to_bone::HOOKS,
    crate::visibility_lists::HOOKS,
    crate::visibility_entity::HOOKS,
    crate::render_buffer_store::HOOKS,
    crate::grass_globals::HOOKS,
    crate::rw_search_callbacks::HOOKS,
    crate::list_remove_item::HOOKS,
    crate::render_lists::HOOKS,
    crate::mvpd_unk::HOOKS,
    rwplugins::HOOKS,
    draw::HOOKS,
    crate::custompipes::HOOKS,
    crate::fx_alloc::HOOKS,
    crate::car_dirt::HOOKS,
    crate::player_skin::HOOKS,
    crate::clouds_render::HOOKS,
    crate::water_render::HOOKS,
    crate::coronas_render::HOOKS,
    crate::mirrors_render::HOOKS,
    crate::brightlights_render::HOOKS,
    crate::weather_render::HOOKS,
    crate::water_init::HOOKS,
    crate::visibility_access::HOOKS,
    crate::pipe_ids::HOOKS,
    crate::collision_plugin_mi::HOOKS,
    crate::car_env_stream::HOOKS,
    crate::car_pipe_setup::HOOKS,
    crate::clock_misc::HOOKS,
    crate::effect_2d_stream::HOOKS,
    crate::building_dn::HOOKS,
    crate::birds_render::HOOKS,
    crate::postfx_render::HOOKS,
    crate::shadows_render::HOOKS,
    crate::plant_loc_tris::HOOKS,
    crate::fx_interp_integral::HOOKS,
    crate::render_tail_leaves::HOOKS,
    crate::weapon_effects_render::HOOKS,
    crate::specialfx_render::HOOKS,
    crate::grass_render::HOOKS,
    crate::shadows_cast::HOOKS,
    crate::occluder_process::HOOKS,
    crate::fog_render::HOOKS,
    crate::shadows_store::HOOKS,
    crate::clouds_create::HOOKS,
    crate::vector_compare::HOOKS,
    crate::sprite_render::HOOKS,
    crate::list_item_dtor::HOOKS,
    crate::world_scan::HOOKS,
    crate::plant_col_entity_cache::HOOKS,
    crate::stencil_shadow_volumes::HOOKS,
    crate::roadsign_text_raster::HOOKS,
    crate::roadsign_line_row::HOOKS,
    crate::roadsign_atomic::HOOKS,
    crate::generated::HOOKS,
    crate::fx_emitter_render::HOOKS,
    crate::plant_statics_init::HOOKS,
    crate::visibility_train_big_vehicle::HOOKS,
    crate::car_env_map_render::HOOKS,
    // WRITER-NATIVE
    crate::fx_add_sparks::HOOKS,
    crate::fx_add_punch_impact::HOOKS,
    crate::postfx_speed_fx::HOOKS,
    crate::fx_debris_wheel_grass::HOOKS,
    crate::skidmark_register::HOOKS,
    crate::fx_info_colour_value::HOOKS,
    // WRITER-SAMP-2
    crate::water_line_test::HOOKS,
    crate::fx_add_bullet_impact::HOOKS,
    crate::bullet_trace_add::HOOKS,
    crate::checkpoint_render::HOOKS,
    crate::glass_break_physically::HOOKS,
    crate::underwater_ripple::HOOKS,
    crate::water_split_x::HOOKS,
    crate::water_wake_segment::HOOKS,
    crate::fx_add_blood::HOOKS,
    crate::proc_objects::HOOKS,
    // FRONTIER-2
    crate::posteffects_switches::HOOKS,
    // FRONTIER-2 lane B
    crate::fx_lifecycle::HOOKS,
    crate::posteffects_filters::HOOKS,
    // FRONTIER-2 turn 2, lane B
    crate::frontier_b2::HOOKS,
    crate::frontier_b3::HOOKS,
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    renderer::GLOBALS,
    occlusion::GLOBALS,
    cullzones::GLOBALS,
    sprite2d::GLOBALS,
    sprite::GLOBALS,
    clouds::GLOBALS,
    weather::GLOBALS,
    coronas::GLOBALS,
    pointlights::GLOBALS,
    shadows::GLOBALS,
    bullettraces::GLOBALS,
    specialfx::GLOBALS,
    mirrors::GLOBALS,
    posteffects::GLOBALS,
    fx::GLOBALS,
    waterlevel::GLOBALS,
    timecycle::GLOBALS,
    markers::GLOBALS,
    carplates::GLOBALS,
    glass::GLOBALS,
    stencilshadows::GLOBALS,
    rwplugins::GLOBALS,
    crate::custompipes::GLOBALS,
];

use sa_core::hooks::total;

use sa_core::hooks::concat;

#[allow(clippy::large_const_arrays)]
const ALL_ARRAY: [HookReg; total(MODULES)] = concat(MODULES);

/// All registrations of this crate (the concatenation of [`MODULES`]).
pub const ALL: &[HookReg] = &ALL_ARRAY;

/// Every module's `#[native]` registrations (phase B entries of ports that cannot be hooked in
/// phase A); `sa_ports::NATIVES` lists each of these.
pub const NATIVE_MODULES: &[&[sa_core::abi::NativeReg]] =
    &[sprite2d::NATIVES, rwplugins::NATIVES, draw::NATIVES, crate::custompipes::NATIVES, crate::fx_alloc::NATIVES, crate::car_dirt::NATIVES, crate::visibility_access::NATIVES, crate::frame_node_name::NATIVES, crate::collision_plugin_mi::NATIVES, crate::mvpb_r1::NATIVES, crate::car_pipe_setup::NATIVES, crate::short_leaf_natives::NATIVES, crate::cullzones::NATIVES, crate::renderer::NATIVES, crate::fx_trigger_water_hydrant::NATIVES];

pub const COUNT: usize = ALL.len();

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use sa_testkit::{count_hooks, exe_size};
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;
    
    use std::vec::Vec;

    /// Phase B/C storage starts with the original image's initial values (checked when the exe is
    /// available: `$SA_GTA_EXE` or `~/sa-engine/re/gta/1.0-US/gta_sa.exe`).
    #[test]
    fn global_initial_values_match_the_exe() {
        sa_testkit::check_global_initial_values(GLOBALS);
    }

    /// No global of this crate overlaps another one of this crate or one of sa-core's / sa-stream's /
    /// sa-world's / sa-pop's.
    #[test]
    fn globals_are_declared_once_across_crates() {
        let mine: Vec<(u32, u32, &str)> = GLOBALS
            .iter()
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let others: Vec<(u32, u32, &str)> = sa_core::hooks::GLOBALS
            .iter()
            .chain(sa_stream::hooks::GLOBALS.iter())
            .chain(sa_world::hooks::GLOBALS.iter())
            .chain(sa_pop::hooks::GLOBALS.iter())
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let mut bad = Vec::new();
        for (i, a) in mine.iter().enumerate() {
            for b in mine[i + 1..].iter().chain(others.iter()) {
                if !(a.1 <= b.0 || b.1 <= a.0) {
                    bad.push(std::format!("{} @ {:#x} overlaps {} @ {:#x}", a.2, a.0, b.2, b.0));
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    /// Every `#[hook(` in the sources is in `ALL`; addresses are unique, also against the other crates.
    #[test]
    fn list_is_complete_and_unique() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        assert_eq!(
            count_hooks(&dir),
            COUNT,
            "a #[hook] is missing from a module's HOOKS"
        );
        let addrs: BTreeSet<u32> = ALL.iter().map(|h| h.addr).collect();
        assert_eq!(addrs.len(), COUNT);
        for h in sa_core::hooks::ALL
            .iter()
            .chain(sa_stream::hooks::ALL.iter())
            .chain(sa_world::hooks::ALL.iter())
            .chain(sa_pop::hooks::ALL.iter())
        {
            assert!(!addrs.contains(&h.addr), "{} hooked by another crate too", h.name);
        }
        for h in ALL {
            #[cfg(not(target_arch = "x86"))]
            assert_eq!((h.entry)(), 0, "no shims off x86");
            assert_eq!((h.calls)(), 0);
        }
    }
}
