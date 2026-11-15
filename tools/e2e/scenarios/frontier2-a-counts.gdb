# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of the
# FRONTIER-2 turn 2 lane A ports (their registered phase B entries) and prints the hit counts at the end.
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak __sa_hook_native_kill_criminal_create_next_sub_task$
rbreak __sa_hook_native_kill_criminal_create_sub_task$
commands 1-2
silent
continue
end
run
info breakpoints
