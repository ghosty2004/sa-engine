#!/usr/bin/env bash
# Shared environment for running gta_sa.exe 1.0 US under Wine.
# Usage: source tools/wine-env.sh    (LANE=N selects wine/lane-N when it exists)
SA_ROOT="${SA_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")/.." && pwd)}"
if [ -n "${LANE:-}" ] && [ -d "$SA_ROOT/wine/lane-$LANE" ]; then
    export WINEPREFIX="${WINEPREFIX:-$SA_ROOT/wine/lane-$LANE/prefix}"
    export GTA_DIR="${GTA_DIR:-$SA_ROOT/wine/lane-$LANE/gta-sa}"
else
    export WINEPREFIX="${WINEPREFIX:-$SA_ROOT/wine/prefix}"
    export GTA_DIR="${GTA_DIR:-$SA_ROOT/wine/gta-sa}"
fi
export WINEDEBUG="${WINEDEBUG:--all}"
export DXVK_LOG_LEVEL="${DXVK_LOG_LEVEL:-warn}"
# INF-19: no vsync / frame-rate limiting in DXVK. The game's own pacing decides the frame rate (in
# determinism mode the virtual clock does), so replays run ~3.3x faster (113 s -> 34 s for the
# 5,200-frame grove walk) with an identical state trace. SA_VSYNC=1 restores DXVK's defaults.
if [ "${SA_VSYNC:-0}" != 1 ]; then
    export DXVK_CONFIG="${DXVK_CONFIG:-d3d9.presentInterval = 0; d3d9.maxFrameRate = 0; dxgi.syncInterval = 0}"
fi
# Runs must never be audible: route audio to a private null sink unless SA_AUDIBLE=1.
if [ "${SA_AUDIBLE:-0}" != 1 ] && command -v pactl > /dev/null; then
    pactl list short sinks 2>/dev/null | grep -q sa_engine_null \
        || pactl load-module module-null-sink sink_name=sa_engine_null > /dev/null 2>&1 || true
    export PULSE_SINK=sa_engine_null
fi
