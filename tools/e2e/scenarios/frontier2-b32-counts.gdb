# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of
# the replay playback ports (the Rust functions themselves) under any frontier.sh scenario (replay: F1, every
# playback frame, the end of playback), and prints the hit counts at the end.
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak ^sa_world::world::replay_playback::trigger_playback$
rbreak ^sa_world::world::frontier_b2::replay_play_back_this_frame$
rbreak ^sa_world::world::replay_frame::play_back_this_frame_interpolation$
rbreak ^sa_world::world::replay_frame::process_replay_camera$
rbreak ^sa_world::world::replay_frame::process_look_around_cam$
rbreak ^sa_veh::replay_vehicle::extract_vehicle_update_data$
rbreak ^sa_veh::replay_vehicle::extract_train_update_data$
rbreak ^sa_world::world::replay_peds::deal_with_new_ped_packet$
rbreak ^sa_world::world::replay_peds::process_ped_update$
rbreak ^sa_world::world::replay_playback::finish_playback$
rbreak ^sa_world::world::replay_playback::replay_restore_stuff_from_mem$
commands 1-11
silent
continue
end
run
info breakpoints
