# gdb command file for tools/e2e/scenarios/frontier.sh (WRAP="gdb -q -batch -x THIS --args"): counts the calls of
# the FRONTIER-2 turn 14 lane A ports (the Rust functions themselves) the scenarios reach: shotgun (SetUpPelletCol),
# flamer (FireAreaEffect), driveby (DoDriveByAutoAiming), camera (TakePhotograph, PictureTaken), the cop runs
# (DoDoomAiming).
set pagination off
set breakpoint pending on
handle SIGSEGV stop nopass
handle SIGUSR1 nostop noprint pass
handle SIGUSR2 nostop noprint pass
handle SIGPIPE nostop noprint pass
rbreak ^sa_weapon::pellet::set_up_pellet_col$
rbreak ^sa_weapon::area_effect::fire_area_effect$
rbreak ^sa_weapon::aiming::do_drive_by_auto_aiming$
rbreak ^sa_weapon::aiming::do_doom_aiming$
rbreak ^sa_weapon::photograph::weapon_take_photograph$
rbreak ^sa_world::world::pickups::picture_taken$
commands 1-6
silent
continue
end
run
info breakpoints
