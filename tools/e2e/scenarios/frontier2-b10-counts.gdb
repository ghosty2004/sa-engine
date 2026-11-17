# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of
# the FRONTIER-2 turn 10 lane B VEH ports (the Rust functions themselves) the scenarios monster / tanker / tank /
# hydra are meant to reach, and prints the hit counts at the end.
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak ^sa_veh::monster_truck::monster_truck_setup_suspension_lines$
rbreak ^sa_veh::monster_truck::monster_truck_process_suspension$
rbreak ^sa_veh::vehicle_tow_link::update_trailer_link$
rbreak ^sa_veh::vehicle_tow_link::update_tractor_link$
rbreak ^sa_veh::tank_control::tank_control$
rbreak ^sa_veh::vehicle_guns::fire_fixed_machine_guns$
rbreak ^sa_veh::vehicle_guns::scan_and_mark_target_for_heat_seeking_missile$
rbreak ^sa_veh::vehicle_guns::fire_heat_seaking_missile$
rbreak ^sa_veh::car_door_positions::get_nearest_car_passenger_door$
rbreak ^sa_world::world::cheat::tanker_cheat$
commands 1-10
silent
continue
end
run
info breakpoints
