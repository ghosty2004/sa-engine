# samp-bot as a second real player inside a scenario's command list (sourced by mod-join.sh and samp-rs-ab.sh):
#
#   bot|ARGS     starts samp-bot (SAMP_BOT, default samp-rs/target/release/samp-bot) with ARGS against our server,
#                after ending a bot still running; {me} in ARGS is our player's name, @veh the vehicle id of the
#                server's last `veh` command and @veh2 the one before it
#   botwait      waits until the bot has left (its --seconds)
#
# The caller sets BOT_ROOT (the repository), BOT_PORT, BOT_ME (our player's name), BOT_SLOG (the server log),
# BOT_LOG (where the bot's output goes, appended) and optionally BOT_AUTH (an auth table).
CMD_BOT_PID=

bot_vehicle() { # N: the vehicle id of the N-th last `veh` command
    grep -oE '\[cov\] [0-9]+ veh [0-9]+' "$BOT_SLOG" | tail -n "$1" | head -1 | awk '{print $4}'
}

bot_start() { # ARGS
    local args="$1"
    args="${args//\{me\}/$BOT_ME}"
    args="${args//@veh2/$(bot_vehicle 2)}"
    args="${args//@veh/$(bot_vehicle 1)}"
    bot_stop
    echo "---- samp-bot $args" >> "$BOT_LOG"
    # shellcheck disable=SC2086
    "${SAMP_BOT:-$BOT_ROOT/samp-rs/target/release/samp-bot}" "127.0.0.1:$BOT_PORT" $args \
        ${BOT_AUTH:+--auth-table "$BOT_AUTH"} >> "$BOT_LOG" 2>&1 &
    CMD_BOT_PID=$!
}

bot_wait() {
    [ -n "$CMD_BOT_PID" ] && wait "$CMD_BOT_PID" 2> /dev/null
    CMD_BOT_PID=
}

bot_stop() {
    [ -n "$CMD_BOT_PID" ] && kill "$CMD_BOT_PID" 2> /dev/null && wait "$CMD_BOT_PID" 2> /dev/null
    CMD_BOT_PID=
}
