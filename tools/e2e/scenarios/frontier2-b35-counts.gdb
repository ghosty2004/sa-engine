# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of
# the procedural interior furnishing ports and the interior set-up around them (the Rust functions themselves) under
# any frontier.sh scenario (interior: a burglary house shell furnished by the interior manager), and
# DeleteRwObjectsBehindCamera, and prints the hit counts at the end.
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak ^sa_world::world::interior_init::interior_init$
rbreak ^sa_world::world::interior_room_furnish::furnish_bedroom$
rbreak ^sa_world::world::interior_room_furnish::furnish_lounge$
rbreak ^sa_world::world::interior_room_furnish::kitchen_furnish_edges$
rbreak ^sa_world::world::interior_room_furnish::lounge_add_tv$
rbreak ^sa_world::world::interior_room_furnish::lounge_add_sofa_info$
rbreak ^sa_world::world::interior_room_items::furnish_kitchen$
rbreak ^sa_stream::streaming::delete_rw_objects_behind_camera$
commands 1-8
silent
continue
end
run
info breakpoints
