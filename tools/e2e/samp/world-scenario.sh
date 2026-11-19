# shellcheck shell=bash
# The world scenario both SA:MP hosts run (samp-rs-ab.sh SCENARIO=world: the DLL client on the original exe;
# mod-join.sh SCENARIO=world: the samp mod in sa-game), so that their server events and screenshots compare step by
# step: the server places pickups, map icons, a checkpoint, a race checkpoint, a flashing gang zone and an actor
# around Grove Street; the player is moved onto both pickups and into both checkpoints (the server reports
# OnPlayerPickUpPickup / OnPlayerEnterCheckpoint / OnPlayerEnterRaceCheckpoint); then `gmx` restarts the gamemode
# (RPC 40, class selection again, spawn), and finally the server process is killed (SIGKILL: no disconnection
# notification, as in a crash) and started again (the client loses the connection, reconnects with samp's timing and
# spawns once more). open.mp calls OnPlayerConnect again after gmx, so the reconnect is the third connect line.
#
# The caller sources this file and provides:
#   cmd "CMD|ARG|..."       a sampcov.pwn command, waited for
#   shot NAME               a screenshot
#   say TEXT                a log line
#   wait_count REGEX N S    waits up to S seconds for the N-th server log line matching REGEX
#   client_wait REGEX S     waits up to S seconds for a client log line matching REGEX
#   spawn_again N           gets the player spawned for the N-th time (keys on the DLL host; the mod presses Shift)
#   server_kill (SIGKILL), server_start

world_scenario() {
    cmd "pos|2495|-1684|13.5|0"
    sleep 2
    cmd "pickup|1240|2|2495|-1676|13.5"
    cmd "pickup|1239|1|2490|-1676|13.5"
    cmd "icon|1|2500|-1700|13|6"
    cmd "icon|2|2470|-1690|13|0"
    cmd "cp|2495|-1660|13.5|3"
    cmd "rcp|1|2480|-1660|13.5|2470|-1640|13.5|5"
    cmd "gz|2440|-1720|2540|-1640"
    cmd "actor|105|2498|-1680|13.5|180"
    sleep 3
    shot world-entities
    cmd "pos|2495|-1676|13.5|0"
    wait_count '\[cov\] pickup id=' 1 10 || say "no OnPlayerPickUpPickup for the first pickup"
    sleep 2
    cmd "pos|2490|-1676|13.5|0"
    wait_count '\[cov\] pickup id=' 2 10 || say "no OnPlayerPickUpPickup for the second pickup"
    sleep 2
    cmd "pos|2495|-1660|13.5|0"
    wait_count '\[cov\] cp-enter id=' 1 10 || say "no OnPlayerEnterCheckpoint"
    sleep 1
    shot world-checkpoint
    cmd "pos|2480|-1660|13.5|0"
    wait_count '\[cov\] rcp-enter id=' 1 10 || say "no OnPlayerEnterRaceCheckpoint"
    sleep 2
    shot world-race-checkpoint
    say "gmx"
    cmd "rcon|gmx"
    wait_count '\[cov\] loaded' 2 30 || say "the gamemode did not restart"
    client_wait 'reset routine' 20 || say "no reset on the client"
    spawn_again 2 || say "no spawn after gmx"
    sleep 4
    shot after-gmx
    say "server kill"
    server_kill
    client_wait 'packet ConnectionLost' 60 || say "the client did not see the connection loss"
    sleep 3
    shot connection-lost
    server_start
    wait_count '\[cov\] connect id=' 3 120 || say "no reconnect"
    spawn_again 3 || say "no spawn after the reconnect"
    sleep 4
    shot after-reconnect
}
