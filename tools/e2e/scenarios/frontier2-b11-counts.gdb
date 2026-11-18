# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of
# the FRONTIER-2 turn 11 lane B ports (the Rust functions themselves) the scenarios tank / hunter / heli reach, and
# prints the hit counts at the end.
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak ^sa_weapon::doom_aiming::do_tank_doom_aiming$
rbreak ^sa_veh::vehicle_guns::fire_unguided_missile$
rbreak ^sa_veh::automobile_nitrous::do_nitro_effect$
rbreak ^sa_veh::vehicle_tow_link::tow_truck_control$
rbreak ^sa_veh::harvester::process_harvester$
rbreak ^sa_veh::rcbandit::rcbandit_check_hit_wheels$
rbreak ^sa_veh::rcbandit::rcbandit_check_1_car_wheels$
rbreak ^sa_veh::automobile_render::add_water_splash_particles$
commands 1-8
silent
continue
end
run
info breakpoints
