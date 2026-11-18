# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of
# the replay ports (the Rust functions themselves) under any frontier.sh scenario (replay: recording, F1, playback,
# the end of playback), and prints the hit counts at the end.
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak ^sa_world::world::mvpd_w1::replay_record_this_frame$
rbreak ^sa_world::world::replay_playback::trigger_playback$
rbreak ^sa_world::world::replay_playback::replay_store_stuff_in_mem$
rbreak ^sa_world::world::replay_setup::initialise_ped_pool_conversion_table$
rbreak ^sa_world::world::replay_setup::initialise_vehicle_pool_conversion_table$
rbreak ^sa_world::world::replay_peds::create_player_ped$
rbreak ^sa_world::world::replay_peds::deal_with_new_ped_packet$
rbreak ^sa_world::world::replay_peds::process_ped_update$
rbreak ^sa_world::world::replay_peds::retrieve_ped_animation$
rbreak ^sa_world::world::frontier_b2::replay_play_back_this_frame$
rbreak ^sa_world::world::replay_playback::finish_playback$
rbreak ^sa_world::world::replay_playback::replay_restore_stuff_from_mem$
commands 1-12
silent
continue
end
run
info breakpoints
