# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of the
# FRONTIER-2 part 2 ports (batches frontier2-3 / frontier2-4; the Rust functions and their phase B entries) and prints the hit
# counts at the end (`info breakpoints`).
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak ::update_car_ai$
rbreak ::tell_occupants_to_leave_car$
rbreak ::steer_ai_car_with_physics_only_mission$
rbreak ::add_ambulance_occupants$
rbreak ::add_firetruck_occupants$
rbreak ::kill_criminal_find_next_criminal_to_kill$
rbreak ::kill_criminal_change_target$
rbreak ::kill_criminal_control_sub_task$
rbreak ::process_m16_1st_person$
rbreak ::weapon_cheat2$
commands 1-30
silent
continue
end
run
info breakpoints
