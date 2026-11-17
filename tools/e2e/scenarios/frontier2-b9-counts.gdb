# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of
# the cinema camera ports (the Rust functions themselves) the FRONTIER-2 turn 9 lane B scenario `cinema` is meant
# to reach, and prints the hit counts at the end.
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak ^sa_cam::newcam::is_it_time_for_newcam$
rbreak ^sa_cam::newcam::try_to_start_new_cam_mode$
rbreak ^sa_cam::dwcams::process_dw_heli_chase_cam$
rbreak ^sa_cam::dwcams::process_dw_birdy_cam$
rbreak ^sa_cam::dwcams::process_dw_cam_man_cam$
rbreak ^sa_cam::dwcams::process_dw_plane_spotter_cam$
rbreak ^sa_cam::dwcams::heli_chase_settings_randomise$
rbreak ^sa_cam::dwcams::finalise_dw_ciney_cams$
commands 1-8
silent
continue
end
run
info breakpoints
