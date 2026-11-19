# Runs a step file (tools/e2e/samp/*.steps) against sampcov.pwn, for the scripts that drive a SA:MP client
# (samp-rs-ab.sh: samp-rs's DLL on the original exe; mod-join.sh: the samp mod on sa-engine). Sourced; the caller
# defines `cmd TEXT` (send one server command and wait for its acknowledgement), `shot NAME` (a screenshot) and
# `SLOG` (the server log).
#
# One step per line, `#` starts a comment:
#   a sampcov.pwn command (`pos|x|y|z|angle`, ...); `LASTVEH` in it is the id of the vehicle the last `veh` made
#   sleep|S        wait S seconds
#   shot|NAME      a screenshot NAME
#   say|TEXT       a line in the step log
#   probe|NAME     the caller's game-state probe NAME, when it has one (`probe`)

run_steps() { # FILE
    local line text veh lines
    mapfile -t lines < "$1"
    for line in "${lines[@]}"; do
        line="${line%%#*}"
        line="$(echo "$line" | sed -E 's/[[:space:]]+$//')"
        [ -n "$line" ] || continue
        case "$line" in
            sleep\|*) sleep "${line#sleep|}" ;;
            shot\|*) shot "${line#shot|}" ;;
            say\|*) echo "[$(date +%T)] ${line#say|}" ;;
            probe\|*) if declare -F probe > /dev/null; then probe "${line#probe|}"; fi ;;
            *)
                text="$line"
                if [[ "$text" == *LASTVEH* ]]; then
                    veh="$(grep -oE '\[cov\] [0-9]+ veh -?[0-9]+' "$SLOG" | tail -1 | awk '{print $4}')"
                    text="${text//LASTVEH/$veh}"
                fi
                cmd "$text"
                ;;
        esac
    done
}
