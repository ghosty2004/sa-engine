# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of
# the FRONTIER-2 turn 4 lane A ports (their registered phase B entries) and prints the hit counts at the end.
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak __sa_hook_native_create_road_block_between_2_points$
rbreak __sa_hook_native_gang_wars_update$
rbreak __sa_hook_native_kill_on_foot_allocate_tasks$
rbreak __sa_hook_native_kill_threats_driveby_allocate_tasks$
rbreak __sa_hook_native_player_command_attack_allocate_tasks$
rbreak __sa_hook_native_end_conversation$
rbreak __sa_hook_native_test_for_stealth_kill$
commands 1-7
silent
continue
end
run
info breakpoints
