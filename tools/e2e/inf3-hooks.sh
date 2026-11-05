#!/usr/bin/env bash
# INF-2/INF-3 acceptance: boots gta_sa.exe 1.0 US headless on a lane with sa-hook injected, takes
# screenshots of the main menu and of gameplay, checks the control channel and the frame counter,
# toggles every hook live (also a toggle storm), runs the Rust-vs-original selftest and an optional soak.
#
#   tools/e2e/inf3-hooks.sh [--soak SECONDS] [--no-gameplay]
#
# Env: LANE (default 1), SA_ROOT (checkout holding wine/; set it from a worktree). Build first:
#   cargo build --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook
# Output: target/e2e/lane-N/ (screenshots, wine.log, sa-hook.log copy, summary.txt). Exit 0 = all passed.
set -uo pipefail
SOAK=0
GAMEPLAY=1
while [ $# -gt 0 ]; do
    case "$1" in
        --soak) SOAK="$2"; shift 2 ;;
        --no-gameplay) GAMEPLAY=0; shift ;;
        *) echo "usage: $0 [--soak SECONDS] [--no-gameplay]" >&2; exit 2 ;;
    esac
done
LANE="${LANE:-1}"
# shellcheck source=lib.sh
source "$(dirname "$0")/lib.sh"

FAILS=0
SUMMARY="$E2E_OUT/summary.txt"
: > "$SUMMARY"
check() { # check NAME OK? DETAIL
    local r=PASS
    [ "$2" = 1 ] || { r=FAIL; FAILS=$((FAILS + 1)); }
    printf '%s %s: %s\n' "$r" "$1" "$3" | tee -a "$SUMMARY"
}
ctl() { local r; r="$(e2e_ctl "$1")"; printf '  > %s\n  < %s\n' "$1" "$r" >> "$SUMMARY"; echo "$r"; }
calls_of() { # calls_of NAME → calls=N of that hook from `hooks`
    e2e_ctl hooks | tr ';' '\n' | grep -F " $1 " | sed -n 's/.*calls=\([0-9]*\).*/\1/p'
}
finish() {
    [ -f "$GTA_DIR/sa-hook.log" ] && cp "$GTA_DIR/sa-hook.log" "$E2E_OUT/sa-hook.log"
    [ -f "$GTA_DIR/sa-launcher.log" ] && cp "$GTA_DIR/sa-launcher.log" "$E2E_OUT/sa-launcher.log"
    e2e_cleanup
    e2e_say "$FAILS failure(s); summary $SUMMARY"
}

e2e_disk_check || exit 1
e2e_lane_lock
e2e_lane_setup || exit 1
e2e_install || exit 1
trap finish EXIT
wineserver -k 2> /dev/null || true # leftovers of an earlier run in this lane's prefix only

e2e_headless_start || exit 1
e2e_game_start
e2e_wait_ctl 90 || { check "control channel" 0 "no answer; game alive: $(e2e_game_alive && echo yes || echo no)"; exit 1; }
check "control channel" "$([ "$(ctl ping)" = pong ] && echo 1)" "ping -> pong"
H="$(ctl hooks)"
check "hooks installed" "$([ "$(echo "$H" | grep -o ' rust calls=' | wc -l)" = 3 ] && echo 1)" "$H"

e2e_boot_to_menu 240; check "main menu" "$([ "$(e2e_frames_field state)" = 7 ] && echo 1)" "gGameState=7"
sleep 4
e2e_shot 01-main-menu; check "menu screenshot" "$([ -s "$E2E_OUT/01-main-menu.png" ] && echo 1)" "$E2E_OUT/01-main-menu.png"
F1="$(e2e_frames_field hook)"; G1="$(e2e_frames_field game)"; sleep 3
F2="$(e2e_frames_field hook)"; G2="$(e2e_frames_field game)"
check "frame counter increases" "$([ "${F2:-0}" -gt "${F1:-0}" ] && [ "${G2:-0}" -gt "${G1:-0}" ] && echo 1)" \
    "hook $F1 -> $F2, CTimer::m_FrameCounter $G1 -> $G2 in 3 s"

if [ "$GAMEPLAY" = 1 ]; then
    # Main menu: Start Game → New Game; the intro cutscene is skipped with Return once the game runs.
    e2e_key Return; sleep 1.5; e2e_key Return
    e2e_wait_state 9 180
    check "gameplay state" "$([ "$(e2e_frames_field state)" = 9 ] && echo 1)" "gGameState=9"
    # Hang diagnosis (INF-15): main-thread stack and the CdStreamSync lost-wake-up counter.
    [ "$(e2e_frames_field state)" = 9 ] || { ctl stack > /dev/null; ctl cdsync > /dev/null; }
    sleep 20
    e2e_shot 02-intro; e2e_key Return; sleep 25
    e2e_shot 03-gameplay; check "gameplay screenshot" "$([ -s "$E2E_OUT/03-gameplay.png" ] && echo 1)" "$E2E_OUT/03-gameplay.png"
fi

# Live toggles: each hook to orig and back. While a hook is orig its Rust call counter must stand
# still (it may still move by calls already in flight in that frame); once rust again it must grow.
for name in CTimer::Update CGeneral::LimitRadianAngle CGameLogic::IsCoopGameGoingOn; do
    r1="$(ctl "hook $name orig")"
    sleep 0.5; a="$(calls_of "$name")"; sleep 2; b="$(calls_of "$name")"
    r2="$(ctl "hook $name rust")"
    sleep 2; c="$(calls_of "$name")"
    ok=0
    case "$r1/$r2" in "ok $name orig "*"/ok $name rust "*) [ "$a" = "$b" ] && ok=1 ;; esac
    grow="$([ "${c:-0}" -gt "${b:-0}" ] && echo grows || echo "not called in 2 s")"
    check "toggle $name" "$ok" "orig: calls $a -> $b (frozen); rust again: -> $c ($grow)"
done
storm_ok=1
for i in $(seq 1 100); do
    m=$([ $((i % 2)) = 0 ] && echo rust || echo orig)
    for name in CTimer::Update CGeneral::LimitRadianAngle CGameLogic::IsCoopGameGoingOn; do
        case "$(e2e_ctl "hook $name $m")" in "ok $name $m "*) ;; *) storm_ok=0 ;; esac
    done
done
check "toggle storm (300 switches)" "$storm_ok" "$(e2e_ctl hooks)"
ST="$(ctl selftest)"
st_ok="$(echo "$ST" | grep -q '^selftest' && ! echo "$ST" | grep -o 'mismatches=[0-9]*' | grep -qv '=0$' && echo 1)"
check "selftest Rust vs original" "$st_ok" "$ST"

if [ "$SOAK" -gt 0 ]; then
    e2e_say "soak for $SOAK s (hooks toggled every 10 s)"
    start=$(date +%s); last="$(e2e_frames_field hook)"; stalls=0; n=0
    while [ $(($(date +%s) - start)) -lt "$SOAK" ]; do
        sleep 10
        e2e_game_alive || break
        n=$((n + 1))
        m=$([ $((n % 2)) = 0 ] && echo rust || echo orig)
        e2e_ctl "hook CGeneral::LimitRadianAngle $m" > /dev/null
        e2e_ctl "hook CGameLogic::IsCoopGameGoingOn $m" > /dev/null
        cur="$(e2e_frames_field hook)"
        [ "${cur:-0}" -gt "${last:-0}" ] || stalls=$((stalls + 1))
        last="$cur"
        [ $((n % 6)) = 0 ] && e2e_say "soak $((n * 10)) s: frames $cur, $(e2e_ctl hooks)"
    done
    e2e_ctl "hook CGeneral::LimitRadianAngle rust" > /dev/null
    e2e_ctl "hook CGameLogic::IsCoopGameGoingOn rust" > /dev/null
    e2e_shot 04-after-soak
    elapsed=$(($(date +%s) - start))
    check "soak ${SOAK}s" "$(e2e_game_alive && [ "$stalls" = 0 ] && echo 1)" \
        "alive after ${elapsed} s, frames $last, $stalls stalled 10-s windows, $n toggle rounds"
fi

ctl cdsync > /dev/null # CdStreamSync lost wake-ups recovered by sa-hook (INF-15), for the record
check "game still running" "$(e2e_game_alive && echo 1)" "$(e2e_ctl frames)"
crash="$(grep -E -c 'Unhandled exception|page fault|PANIC|wine: Unhandled' "$E2E_OUT/wine.log" "$GTA_DIR/sa-hook.log" 2> /dev/null | awk -F: '{ s += $2 } END { print s + 0 }')"
check "no crash / panic in logs" "$([ "$crash" = 0 ] && echo 1)" "$crash crash lines in wine.log + sa-hook.log"
exit $((FAILS > 0))
