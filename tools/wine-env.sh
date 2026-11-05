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
# Runs must never be audible: route audio to a private null sink unless SA_AUDIBLE=1.
if [ "${SA_AUDIBLE:-0}" != 1 ] && command -v pactl > /dev/null; then
    pactl list short sinks 2>/dev/null | grep -q sa_engine_null \
        || pactl load-module module-null-sink sink_name=sa_engine_null > /dev/null 2>&1 || true
    export PULSE_SINK=sa_engine_null
fi
