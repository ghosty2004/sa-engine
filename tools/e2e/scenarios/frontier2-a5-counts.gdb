# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of
# the road block and gang war ports (the Rust functions themselves: ports reach each other directly, not through
# their native entries) the FRONTIER-2 turn 5 lane A scenarios (roadblock-script, roadblock-wanted,
# gangwar) are meant to reach, and prints the hit counts at the end.
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak ^sa_pop::[a-z0-9_]*::create_road_block_between_2_points$
rbreak ^sa_pop::[a-z0-9_]*::start_offensive_gang_war$
rbreak ^sa_pop::[a-z0-9_]*::start_defensive_gang_war$
rbreak ^sa_pop::[a-z0-9_]*::create_attack_wave$
rbreak ^sa_pop::[a-z0-9_]*::create_defending_group$
rbreak ^sa_pop::[a-z0-9_]*::pick_zone_to_attack$
rbreak ^sa_pop::[a-z0-9_]*::gang_wars_update$
commands 1-7
silent
continue
end
run
info breakpoints
