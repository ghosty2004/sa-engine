//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after every other
//! engine crate's registrations), and every module's globals.
//! Each porting module adds its `HOOKS` / `GLOBALS` lists here.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

/// Every module's registrations, in this order.
pub const MODULES: &[&[HookReg]] = &[
    crate::trivial::HOOKS,
    crate::dtors::HOOKS,
    crate::tasks::HOOKS,
    crate::events::HOOKS,
    crate::misc::HOOKS,
    crate::car_enter::HOOKS,
    crate::car_exit::HOOKS,
    crate::car_drive::HOOKS,
    crate::taskmgr::HOOKS,
    crate::usegun::HOOKS,
    crate::fight::HOOKS,
    crate::fight_combo_style::HOOKS,
    crate::partner_meeting::HOOKS,
    crate::handler::HOOKS,
    crate::intel::HOOKS,
    crate::group::HOOKS,
    crate::small::HOOKS,
    crate::pgi::HOOKS,
    crate::gen_leaves::HOOKS,
    crate::gen_virtuals::HOOKS,
    crate::gen_ctors::HOOKS,
    crate::gen_clones::HOOKS,
    crate::gen_dtors::HOOKS,
    crate::gen_subtasks::HOOKS,
    crate::gen_bodies::HOOKS,
    crate::intelrest::HOOKS,
    crate::groups::HOOKS,
    crate::decisions::HOOKS,
    crate::smallai5::HOOKS,
    crate::conversation::HOOKS,
    crate::stuck::HOOKS,
    crate::collscan::HOOKS,
    crate::fighthelp::HOOKS,
    crate::killtasks::HOOKS,
    crate::gangtasks::HOOKS,
    crate::walk::HOOKS,
    crate::fall::HOOKS,
    crate::flee::HOOKS,
    crate::events2::HOOKS,
    crate::wander::HOOKS,
    crate::walkround::HOOKS,
    crate::seek::HOOKS,
    crate::geh::HOOKS,
    crate::gang6::HOOKS,
    crate::partner6::HOOKS,
    crate::mvpa_ai1::HOOKS,
    crate::mvpa_ai2::HOOKS,
    crate::event_stuck_in_air_priority::HOOKS,
    crate::wander_prostitute_stuck_in_air_dtors::HOOKS,
    crate::mvpc_ai1::HOOKS,
    crate::mvpc_ai2::HOOKS,
    crate::mvpc_ai3::HOOKS,
    crate::node_route_tasks::HOOKS,
    crate::anim_tasks::HOOKS,
    crate::ped_geometry::HOOKS,
    crate::player_on_foot::HOOKS,
    crate::car_drive_ped::HOOKS,
    crate::vehicle_collision_scan::HOOKS,
    crate::fire_scan::HOOKS,
    crate::walk_into_ped_scan::HOOKS,
    crate::walk_into_object_scan::HOOKS,
    crate::attractor_range_scan::HOOKS,
    crate::wander_chat_scan::HOOKS,
    crate::seek_entity_dtor::HOOKS,
    crate::conversation_ready::HOOKS,
    crate::ik_tasks::HOOKS,
    crate::hold_entity::HOOKS,
    crate::cover_update::HOOKS,
    crate::event_global_group::HOOKS,
    crate::entity_scanner_scan::HOOKS,
    crate::mvpb_ai::HOOKS,
    crate::mvpb_ai2::HOOKS,
    crate::mvpb_ai3::HOOKS,
    crate::mvpb_ai4::HOOKS,
    crate::attractor_scanner::HOOKS,
    crate::hold_entity_process::HOOKS,
    crate::ped_group_random_place::HOOKS,
    crate::walk_alongside_control::HOOKS,
    crate::ped_collision_responses::HOOKS,
    crate::entity_bbox_corners::HOOKS,
    crate::event_responses_simple::HOOKS,
    crate::event_responses_peds::HOOKS,
    crate::event_responses_misc::HOOKS,
    crate::event_responses_shot_vehicle::HOOKS,
    crate::player_ped_collision::HOOKS,
    crate::ped_player_collision::HOOKS,
    crate::dragged_out_car::HOOKS,
    crate::vehicle_potential_collision::HOOKS,
    crate::attractor_approach_event::HOOKS,
    crate::task_in_air_process_ped::HOOKS,
    crate::event_attractor_affects_ped::HOOKS,
    crate::stuck_in_air_subtask::HOOKS,
    crate::group_membership_remove::HOOKS,
    crate::mental_state_anger::HOOKS,
    crate::walk_round_object_route::HOOKS,
    crate::smart_flee_point_abortable::HOOKS,
    crate::smart_flee_point_position::HOOKS,
    crate::event_history_store::HOOKS,
    crate::geometry_line_of_sight::HOOKS,
    crate::ik_chain_blend_out::HOOKS,
    crate::attractor_ped_use_effect::HOOKS,
    crate::general_node_heading::HOOKS,
    crate::route_round_entity_bbox::HOOKS,
    crate::route_round_sphere::HOOKS,
    crate::attractor_place_ped::HOOKS,
    crate::attractor_registry::HOOKS,
    crate::interesting_events_add::HOOKS,
    crate::attractor_ptr_vectors::HOOKS,
    crate::attractor_register_ped::HOOKS,
    crate::attractor_task_pair_vector::HOOKS,
    crate::ped_attractor_base::HOOKS,
    crate::ped_attractor_dtor::HOOKS,
    crate::go_to_attractor_first_sub_task::HOOKS,
    crate::respected_friend_range::HOOKS,
    crate::attractor_arrival::HOOKS,
    crate::partner_deal_dtor::HOOKS,
    crate::sit_down_idle_stand_up::HOOKS,
    crate::enter_car_sub_tasks::HOOKS,
    crate::leader_entered_car_event_dtor::HOOKS,
    crate::go_to_car_door_control::HOOKS,
    crate::line_up_ped_with_car::HOOKS,
    crate::slide_to_coord_process_ped::HOOKS,
    crate::chat_task_subtasks::HOOKS,
    crate::hand_signal_clone_fire_dtor::HOOKS,
    crate::hand_signal_anim_tasks::HOOKS,
    crate::use_attractor_tasks::HOOKS,
    crate::wander_set_dir::HOOKS,
    crate::walk_round_object_sub_task::HOOKS,
    crate::event_vehicle_damage::HOOKS,
    crate::event_vehicle_damage_ctor::HOOKS,
    // WRITER-NATIVE
    crate::task_clones_native::HOOKS,
    crate::group_task_pairs::HOOKS,
    crate::duck_sequence_groups::HOOKS,
    crate::police_pursuit::HOOKS,
    crate::intel_group_misc::HOOKS,
    crate::car_shuffle_fall_out::HOOKS,
    crate::samp_reached_small::HOOKS,
    crate::enter_exit_car_samp::HOOKS,
    crate::die_tasks_samp::HOOKS,
    crate::bike_door_swim_samp::HOOKS,
    crate::throw_projectile_anim::HOOKS,
    crate::swim_task_process::HOOKS,
    crate::door_route_fight_strafe::HOOKS,
    crate::dead_task_process::HOOKS,
    crate::knock_off_bike_damage_anims::HOOKS,
    crate::group_list_loops::HOOKS,
    crate::event_affects_ped_native::HOOKS,
    crate::event_damage_dtor::HOOKS,
    crate::car_set_ped_out_native::HOOKS,
    crate::road_rage_events_native::HOOKS,
    crate::road_rage_sub_tasks::HOOKS,
    crate::be_in_group_clone::HOOKS,
    crate::leave_car_ctor::HOOKS,
    crate::group_closest_ped::HOOKS,
    crate::road_rage_control::HOOKS,
    crate::car_wait_slow_down::HOOKS,
    crate::lean_on_vehicle_point::HOOKS,
    crate::group_scan_loops::HOOKS,
    crate::group_events_loops::HOOKS,
    crate::event_pass_object_ctor::HOOKS,
    crate::pass_object_tasks::HOOKS,
    crate::interesting_events_scan::HOOKS,
    crate::fight_sphere_col_and_anim_cb::HOOKS,
    crate::stare_at_ped_clone::HOOKS,
    crate::fight_ground_target::HOOKS,
    crate::stare_at_ped_control::HOOKS,
    crate::jump_sub_tasks::HOOKS,
    crate::fight_strike_damage::HOOKS,
    crate::event_vehicle_collision_ctor::HOOKS,
    crate::closest_surface_flee_next::HOOKS,
    crate::simple_jump_tasks::HOOKS,
    crate::climb_test::HOOKS,
    crate::event_ref_dtors::HOOKS,
    crate::wander_target_heading::HOOKS,
    crate::can_ped_jump_obstacle::HOOKS,
    crate::climb_scan::HOOKS,
    crate::climb_anim::HOOKS,
    crate::climb_process::HOOKS,
    crate::climb_camera::HOOKS,
    crate::follow_node_route_blend::HOOKS,
    crate::fight_hit_object::HOOKS,
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    crate::misc::GLOBALS,
    crate::conversation::GLOBALS,
    crate::collscan::GLOBALS,
];

const fn total(parts: &[&[HookReg]]) -> usize {
    let (mut n, mut i) = (0, 0);
    while i < parts.len() {
        n += parts[i].len();
        i += 1;
    }
    n
}

const fn concat<const N: usize>(parts: &[&[HookReg]]) -> [HookReg; N] {
    let mut out = [sa_core::hooks::ALL[0]; N];
    let (mut k, mut i) = (0, 0);
    while i < parts.len() {
        let mut j = 0;
        while j < parts[i].len() {
            out[k] = parts[i][j];
            k += 1;
            j += 1;
        }
        i += 1;
    }
    assert!(k == N);
    out
}

#[allow(clippy::large_const_arrays)]
const ALL_ARRAY: [HookReg; total(MODULES)] = concat(MODULES);

/// All registrations of this crate (the concatenation of [`MODULES`]).
pub const ALL: &[HookReg] = &ALL_ARRAY;

pub const COUNT: usize = ALL.len();

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;
    use std::string::String;
    use std::vec::Vec;

    /// Initial bytes of a virtual address range in the exe image (`.bss` beyond the raw data = 0).
    fn exe_image_bytes(exe: &[u8], va: u32, n: usize) -> Option<Vec<u8>> {
        let u16_at = |o: usize| u16::from_le_bytes([exe[o], exe[o + 1]]) as usize;
        let u32_at = |o: usize| u32::from_le_bytes(exe[o..o + 4].try_into().unwrap());
        let pe = u32_at(0x3C) as usize;
        let (nsec, opt) = (u16_at(pe + 6), u16_at(pe + 20));
        let base = u32_at(pe + 24 + 28);
        for s in 0..nsec {
            let h = pe + 24 + opt + s * 40;
            let (vsize, va0, rsize, raw) = (
                u32_at(h + 8),
                base + u32_at(h + 12),
                u32_at(h + 16),
                u32_at(h + 20),
            );
            if va >= va0 && va + n as u32 <= va0 + vsize.max(rsize) {
                return Some(
                    (0..n as u32)
                        .map(|k| {
                            let o = va + k - va0;
                            if o < rsize {
                                exe[(raw + o) as usize]
                            } else {
                                0
                            }
                        })
                        .collect(),
                );
            }
        }
        None
    }

    /// The exe's size of a global: pointer-sized elements are 4 bytes there, possibly 8 on a 64-bit
    /// host. Globals made of pointers are checked as all-zero (their exe image must be zero too).
    fn exe_size(g: &GlobalDesc) -> usize {
        let p = core::mem::size_of::<usize>();
        if p == 8 && g.size.is_multiple_of(8) && g.init.iter().all(|b| *b == 0) {
            g.size / 2
        } else {
            g.size
        }
    }

    /// Phase B/C storage starts with the original image's initial values (checked when the exe is
    /// available: `$SA_GTA_EXE` or `~/sa-engine/re/gta/1.0-US/gta_sa.exe`).
    #[test]
    fn global_initial_values_match_the_exe() {
        let path = std::env::var("SA_GTA_EXE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join("sa-engine/re/gta/1.0-US/gta_sa.exe")
            });
        let Ok(exe) = std::fs::read(&path) else {
            std::eprintln!("note: gta_sa.exe not found, initial values not checked");
            return;
        };
        for g in GLOBALS.iter().flat_map(|l| l.iter()) {
            let n = exe_size(g);
            let img =
                exe_image_bytes(&exe, g.addr, n).unwrap_or_else(|| panic!("{} not in the image", g.name));
            if n == g.size {
                assert_eq!(&img[..], &g.init[..n], "{} @ {:#x}", g.name, g.addr);
            } else {
                assert!(img.iter().all(|b| *b == 0), "{} @ {:#x}", g.name, g.addr);
            }
        }
    }

    /// No global of this crate overlaps another one of this crate or one of another crate's (a
    /// global declared twice would be two separate statics in phases B/C). Sizes are the exe's
    /// (32-bit).
    #[test]
    fn globals_are_declared_once_across_crates() {
        let mine: Vec<(u32, u32, &str)> = GLOBALS
            .iter()
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let core: Vec<(u32, u32, &str)> = sa_core::hooks::GLOBALS
            .iter()
            .chain(sa_stream::hooks::GLOBALS.iter())
            .chain(sa_world::hooks::GLOBALS.iter())
            .chain(sa_pop::hooks::GLOBALS.iter())
            .chain(sa_script::hooks::GLOBALS.iter())
            .chain(sa_weapon::hooks::GLOBALS.iter())
            .chain(sa_ped::hooks::GLOBALS.iter())
            .chain(sa_input::hooks::GLOBALS.iter())
            .chain(sa_anim::hooks::GLOBALS.iter())
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let mut bad = Vec::new();
        for (i, a) in mine.iter().enumerate() {
            for b in mine[i + 1..].iter().chain(core.iter()) {
                if !(a.1 <= b.0 || b.1 <= a.0) {
                    bad.push(std::format!("{} @ {:#x} overlaps {} @ {:#x}", a.2, a.0, b.2, b.0));
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    fn count_hooks(dir: &Path) -> usize {
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                n += count_hooks(&p);
            } else if p.extension().is_some_and(|x| x == "rs") && p.file_name().unwrap() != "hooks.rs" {
                let s: String = std::fs::read_to_string(&p).unwrap();
                n += s
                    .lines()
                    .filter(|l| l.trim_start().starts_with("#[hook("))
                    .count();
            }
        }
        n
    }

    /// Every `#[hook(` in the sources is in `ALL`; addresses are unique, also against sa-core's and
    /// sa-stream's.
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
            .chain(sa_script::hooks::ALL.iter())
            .chain(sa_weapon::hooks::ALL.iter())
            .chain(sa_ped::hooks::ALL.iter())
            .chain(sa_input::hooks::ALL.iter())
            .chain(sa_anim::hooks::ALL.iter())
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
