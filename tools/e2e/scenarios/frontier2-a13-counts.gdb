# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of
# the FRONTIER-2 turn 13 lane A ports (the Rust functions themselves) the scenarios reach: monster runs
# CCollision::ProcessColModels' A-disk path (disk_pass) and ProcessDiscCollision for the truck's wheel disks.
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak ^sa_world::collision::col_models::process_col_models$
rbreak ^sa_world::collision::col_models::disk_pass$
rbreak ^sa_world::collision::prim::process_disc_collision$
commands 1-3
silent
continue
end
run
info breakpoints
