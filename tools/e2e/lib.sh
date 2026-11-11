#!/usr/bin/env bash
# Shared helpers for sa-engine end-to-end runs. Source from a bash script with LANE=N set (default 1).
#
#   e2e_lane_lock               take target/locks/lane-N.lock for the rest of the script (blocks)
#   e2e_lane_setup              create wine/lane-N/{prefix,gta-sa} if missing (prefix copied from
#                               wine/prefix, game dir = symlinks into wine/gta-sa)
#   e2e_disk_check              abort if /home has < 20 GB free
#   e2e_install                 copy sa-launcher.exe + sa_hook.dll (i686 release build) into the lane
#   e2e_headless_start          start the lane's isolated headless gamescope display
#   e2e_game_start [ARGS...]    run gta_sa.exe through sa-launcher on that display (background)
#   e2e_game_alive              true while the lane's game process runs
#   e2e_ctl CMD                 one control-channel round trip (sa-hook, port SA_HOOK_PORT)
#   e2e_wait_ctl SECS           wait until the control channel answers `ping`
#   e2e_wait_state STATE SECS   wait until gGameState == STATE (7 = main menu, 9 = playing)
#   e2e_boot_to_menu SECS       press Space through the (movie-less) logo/intro states to the menu
#   e2e_shot NAME               screenshot of the lane's display → target/e2e/lane-N/NAME.png
#   e2e_key KEY... / e2e_keydown KEY / e2e_keyup KEY   xdotool keys on the lane's display only
#   e2e_cleanup                 stop the game (wineserver -k for the lane prefix only) and gamescope
#
# Isolation rules (CLAUDE.md, PLAN.md §8): nothing runs on the user's desktop (the game gets the
# gamescope display, no Wine virtual desktop), screenshots come from gamescope only, teardown never
# uses pkill/killall patterns and touches only this lane's Wine prefix and this script's gamescope.

E2E_SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# Checkout that holds the gitignored wine/ and the shared lane locks (set SA_ROOT from a worktree).
SA_ROOT="${SA_ROOT:-$E2E_SCRIPT_ROOT}"
LANE="${LANE:-1}"
case "$LANE" in '' | *[!0-9]*) echo "e2e: LANE must be a number" >&2; return 1 2>/dev/null || exit 1 ;; esac
E2E_LANE_DIR="$SA_ROOT/wine/lane-$LANE"
export WINEPREFIX="$E2E_LANE_DIR/prefix"
export GTA_DIR="$E2E_LANE_DIR/gta-sa"
export SA_HOOK_PORT="${SA_HOOK_PORT:-$((47800 + LANE))}"
E2E_OUT="${E2E_OUT:-$E2E_SCRIPT_ROOT/target/e2e/lane-$LANE}"
E2E_BIN="${E2E_BIN:-${CARGO_TARGET_DIR:-$E2E_SCRIPT_ROOT/target}/i686-pc-windows-gnu/release}"
E2E_RES="${E2E_RES:-1280x720}"
mkdir -p "$E2E_OUT"
# Every Wine command of a lane (winepath, wineserver, the game) must see only the lane's headless
# display: the first Wine process starts the session (explorer.exe), which keeps the display it was
# started with. Inherited desktop variables made winepath start the session on the user's desktop,
# so the game got the desktop's modes (7680x2160, 240 Hz) and failed to set 800x600@240.
# e2e_headless_start exports DISPLAY again for the lane's gamescope.
unset WAYLAND_DISPLAY WAYLAND_SOCKET DISPLAY
# Nested runs (l3.sh --probe started by sa-oracle) inherit the lane display of their parent.
[ -n "${E2E_DISPLAY:-}" ] && export DISPLAY="$E2E_DISPLAY"
# WINEDEBUG, DXVK log level and the null audio sink (PULSE_SINK=sa_engine_null).
# shellcheck source=../wine-env.sh
source "$E2E_SCRIPT_ROOT/tools/wine-env.sh"

e2e_say() { printf '[e2e lane %s %(%H:%M:%S)T] %s\n' "$LANE" -1 "$*"; }

e2e_lane_lock() {
    mkdir -p "$SA_ROOT/target/locks"
    exec {E2E_LOCK_FD}> "$SA_ROOT/target/locks/lane-$LANE.lock"
    if ! flock -n "$E2E_LOCK_FD"; then
        e2e_say "waiting for target/locks/lane-$LANE.lock"
        flock "$E2E_LOCK_FD"
    fi
}

e2e_disk_check() {
    local free_kb
    free_kb="$(df -Pk /home | awk 'NR == 2 { print $4 }')"
    if [ "$free_kb" -lt $((20 * 1024 * 1024)) ]; then
        echo "e2e: less than 20 GB free on /home ($((free_kb / 1024 / 1024)) GB), refusing to run" >&2
        return 1
    fi
    # INF-19: more lanes run at once now that replays are CPU-bound; no swap on this machine, so wait
    # (up to 10 min) until at least E2E_MIN_MEM_MB (default 4096) MB are available before a game run.
    local avail i
    for i in $(seq 1 120); do
        avail="$(awk '/^MemAvailable:/ { print int($2 / 1024) }' /proc/meminfo)"
        [ "$avail" -ge "${E2E_MIN_MEM_MB:-4096}" ] && return 0
        [ "$i" = 1 ] && echo "e2e: only $avail MB available, waiting for memory" >&2
        sleep 5
    done
    echo "e2e: less than ${E2E_MIN_MEM_MB:-4096} MB available for 10 min, refusing to run" >&2
    return 1
}

# A prefix's Documents must never point at the user's real ~/Documents (Wine may create it as a
# symlink); GTA writes gta_sa.set and saves there.
e2e_isolate_userfiles() {
    local docs="$WINEPREFIX/drive_c/users/$USER/Documents"
    if [ -L "$docs" ]; then rm "$docs" && mkdir -p "$docs"; fi
}

e2e_lane_setup() {
    local base_prefix="$SA_ROOT/wine/prefix" base_gta="$SA_ROOT/wine/gta-sa" e
    [ -d "$base_prefix" ] && [ -f "$base_gta/gta_sa.exe" ] || { echo "e2e: no wine/prefix or wine/gta-sa (INF-1)" >&2; return 1; }
    if [ ! -d "$WINEPREFIX" ]; then
        e2e_say "creating lane prefix $WINEPREFIX (copy of wine/prefix)"
        mkdir -p "$E2E_LANE_DIR"
        cp -a --reflink=auto "$base_prefix" "$WINEPREFIX.tmp" && mv "$WINEPREFIX.tmp" "$WINEPREFIX"
    fi
    [ -d "$GTA_DIR" ] || e2e_say "creating lane game dir $GTA_DIR (symlinks into wine/gta-sa)"
    mkdir -p "$GTA_DIR"
    # Idempotent: links entries added to wine/gta-sa since the lane was created.
    for e in "$base_gta"/*; do
        [ -e "$GTA_DIR/$(basename "$e")" ] || ln -s "$e" "$GTA_DIR/$(basename "$e")"
    done
    e2e_isolate_userfiles
}

# The Windows path of a Unix path (Z: = / in every lane prefix). `winepath` right after
# `wineserver -k` sometimes prints nothing (it has to start a new Wine session): an empty
# SA_DET_TRACE then made sa-hook write its trace to the default sa-trace.bin, and port-l2.sh compared
# the stale trace a previous run with the same tag had left (PORT-VEH "rust diverges at frame 2614",
# 2026-11-11). Falls back to the Z: mapping and never returns an empty string.
e2e_winpath() {
    local p
    p="$(winepath -w "$1" 2> /dev/null)"
    if [ -z "$p" ] && [ "$(readlink "$WINEPREFIX/dosdevices/z:" 2> /dev/null)" = / ]; then
        case "$1" in /*) p="Z:${1//\//\\}" ;; esac
    fi
    [ -n "$p" ] || { echo "e2e: no Windows path for $1" >&2; return 1; }
    printf '%s\n' "$p"
}

# Replace a top-level file in the lane game dir; never writes through a symlink into wine/gta-sa.
e2e_put() { rm -f "$GTA_DIR/$2"; cp "$1" "$GTA_DIR/$2"; }

e2e_install() {
    local f
    for f in sa-launcher.exe sa_hook.dll; do
        [ -f "$E2E_BIN/$f" ] || { echo "e2e: $E2E_BIN/$f missing (cargo build --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook)" >&2; return 1; }
        e2e_put "$E2E_BIN/$f" "$f"
    done
}

# ---- isolated headless display (gamescope) ------------------------------------------------------
# gamescope runs `sleep infinity` as its child, in its own process group (setsid), so teardown can
# signal exactly that group. The child writes the Xwayland DISPLAY and gamescope's Wayland socket.
e2e_headless_start() {
    local w="${E2E_RES%x*}" h="${E2E_RES#*x}" env_file="$E2E_OUT/headless.env" child="$E2E_OUT/headless-child.sh" i
    rm -f "$env_file"
    {
        echo '#!/usr/bin/env bash'
        echo "env | grep -E '^(DISPLAY|GAMESCOPE_WAYLAND_DISPLAY)=' > '$env_file'"
        echo 'exec sleep infinity'
    } > "$child"
    chmod +x "$child"
    setsid gamescope --backend headless -W "$w" -H "$h" -w "$w" -h "$h" -- "$child" > "$E2E_OUT/gamescope.log" 2>&1 &
    E2E_GAMESCOPE_PID=$!
    for ((i = 0; i < 80; i++)); do [ -s "$env_file" ] && break; sleep 0.25; done
    [ -s "$env_file" ] || { echo "e2e: gamescope did not start (see $E2E_OUT/gamescope.log)" >&2; return 1; }
    E2E_DISPLAY="$(sed -n 's/^DISPLAY=//p' "$env_file")"
    E2E_GAMESCOPE="$(sed -n 's/^GAMESCOPE_WAYLAND_DISPLAY=//p' "$env_file")"
    [ -n "$E2E_DISPLAY" ] && [ -n "$E2E_GAMESCOPE" ] || { echo "e2e: no DISPLAY/GAMESCOPE_WAYLAND_DISPLAY from gamescope" >&2; return 1; }
    export E2E_DISPLAY E2E_GAMESCOPE
    export DISPLAY="$E2E_DISPLAY"
    e2e_say "gamescope pid $E2E_GAMESCOPE_PID, DISPLAY=$E2E_DISPLAY, wayland=$E2E_GAMESCOPE"
}

e2e_headless_stop() {
    [ -n "${E2E_GAMESCOPE_PID:-}" ] || return 0
    kill -TERM -- "-$E2E_GAMESCOPE_PID" 2>/dev/null
    local i; for ((i = 0; i < 20; i++)); do kill -0 "$E2E_GAMESCOPE_PID" 2>/dev/null || break; sleep 0.25; done
    kill -KILL -- "-$E2E_GAMESCOPE_PID" 2>/dev/null
    wait "$E2E_GAMESCOPE_PID" 2>/dev/null || true
    E2E_GAMESCOPE_PID=
}

# e2e_shot NAME → $E2E_OUT/NAME.png (gamescope's own screenshot of the lane display).
e2e_shot() {
    local f="$E2E_OUT/$1.png"
    rm -f "$f"
    GAMESCOPE_WAYLAND_DISPLAY="$E2E_GAMESCOPE" gamescopectl screenshot "$f" > /dev/null 2>&1
    local i; for ((i = 0; i < 20; i++)); do [ -s "$f" ] && break; sleep 0.25; done
    [ -s "$f" ] && e2e_say "screenshot $f"
}
e2e_key() { DISPLAY="$E2E_DISPLAY" xdotool key --delay 120 "$@"; }
e2e_keydown() { DISPLAY="$E2E_DISPLAY" xdotool keydown "$1"; }
e2e_keyup() { DISPLAY="$E2E_DISPLAY" xdotool keyup "$1"; }

# ---- game ------------------------------------------------------------------------------------------
# Runs directly on the gamescope display (no Wine virtual desktop). The launcher waits for the game,
# so E2E_GAME_PID (a wine process of this lane) lives exactly as long as gta_sa.exe.
e2e_game_start() {
    [ -n "${E2E_DISPLAY:-}" ] || { echo "e2e: start the headless display first" >&2; return 1; }
    rm -f "$GTA_DIR/sa-hook.log" "$GTA_DIR/sa-launcher.log"
    (
        cd "$GTA_DIR" || exit 1
        exec env -u WAYLAND_DISPLAY DISPLAY="$E2E_DISPLAY" wine "$GTA_DIR/sa-launcher.exe" \
            --gta "$(e2e_winpath "$GTA_DIR/gta_sa.exe")" --dll "$(e2e_winpath "$GTA_DIR/sa_hook.dll")" -- "$@"
    ) > "$E2E_OUT/wine.log" 2>&1 &
    E2E_GAME_PID=$!
    e2e_say "launcher started (pid $E2E_GAME_PID), log $E2E_OUT/wine.log"
}

e2e_game_alive() { [ -n "${E2E_GAME_PID:-}" ] && kill -0 "$E2E_GAME_PID" 2>/dev/null; }

e2e_ctl() {
    python3 -I - "$SA_HOOK_PORT" "$1" <<'PY'
import socket, sys
try:
    s = socket.create_connection(("127.0.0.1", int(sys.argv[1])), timeout=3)
    s.settimeout(20)
    s.sendall((sys.argv[2] + "\n").encode())
    print(s.makefile().readline().strip())
except OSError:
    pass
PY
}

e2e_wait_ctl() {
    local i
    for ((i = 0; i < $1 * 2; i++)); do
        [ "$(e2e_ctl ping)" = pong ] && return 0
        e2e_game_alive || return 1
        sleep 0.5
    done
    return 1
}

# Field of the `frames` reply: e2e_frames_field state|hook|game|time
e2e_frames_field() { e2e_ctl frames | tr ' ' '\n' | sed -n "s/^$1=//p"; }

e2e_wait_state() {
    local want="$1" secs="$2" i
    for ((i = 0; i < secs * 2; i++)); do
        [ "$(e2e_frames_field state)" = "$want" ] && return 0
        e2e_game_alive || return 1
        sleep 0.5
    done
    return 1
}

# e2e_boot_to_menu SECS — waits for the main menu (gGameState 7). wine/gta-sa has no intro movies, so
# the game sits in the logo/intro states (2, 4) until a key is pressed (WinMain.cpp 0x748B17): press
# Space only while in those states. Nothing in the game files is patched or deleted.
e2e_boot_to_menu() {
    local secs="$1" i s
    for ((i = 0; i < secs * 2; i++)); do
        s="$(e2e_frames_field state)"
        case "$s" in
            7) return 0 ;;
            2 | 4) e2e_key space ;;
        esac
        e2e_game_alive || return 1
        sleep 0.5
    done
    return 1
}

e2e_cleanup() {
    # Only this lane's Wine prefix: wineserver -k ends every process of $WINEPREFIX and nothing else.
    if [ -d "$WINEPREFIX" ]; then wineserver -k 2> /dev/null || true; fi
    if [ -n "${E2E_GAME_PID:-}" ]; then
        wait "$E2E_GAME_PID" 2> /dev/null || true
        E2E_GAME_PID=
    fi
    e2e_headless_stop
}
