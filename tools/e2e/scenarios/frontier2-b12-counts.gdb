# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of
# the FRONTIER-2 turn 12 lane B ports (the Rust functions themselves) under any frontier.sh scenario (hunter: the
# heli's guns and rockets), and prints the hit counts at the end.
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak ^sa_veh::vehicle_guns::fire_plane_guns$
rbreak ^sa_veh::vehicle_guns::get_plane_guns_position$
rbreak ^sa_veh::vehicle_guns::do_plane_gun_fire_fx$
rbreak ^sa_veh::vehicle_guns::fire_unguided_missile$
rbreak ^sa_veh::vehicle_plane_weapons::vehicle_get_plane_weapon_firing_status$
rbreak ^sa_cam::cammodes::process_wheel_cam$
rbreak ^sa_veh::vehicle_blade_collision::blade_col_sector_list$
commands 1-7
silent
continue
end
run
info breakpoints
