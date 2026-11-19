# The key scenario of samp-rs-ab.sh and mod-join.sh (SCENARIO=keys): the same game keys pressed one at a time on foot,
# as a vehicle's driver and as its passenger, so the server's key events (sampcov.pwn's "[cov] keys" lines:
# OnPlayerKeyStateChange and the polled GetPlayerKeys) can be compared between samp-rs's DLL client and the samp mod.
# Sourced. Each step is `PHASE KEY XKEY`: KEY in sa-game's SA_GAME_KEYS names (`lmb` / `rmb`: the mouse buttons), XKEY
# the xdotool name (`button1` / `button3`: the mouse buttons). Default PC controls: Tab action, C crouch, the left
# button and Left Ctrl fire, Space sprint (handbrake in a car), Left Shift jump, F enter / exit, the right button aim,
# Caps Lock lock-on (horn in a car), Left Alt walk, numpad 8 / 2 / 4 / 6 look (turret in a car), Y / N / H answer yes,
# no and back, Q / E look left / right in a car, 2 submission in a car.
KEY_STEPS=(
    "foot w w" "foot s s" "foot a a" "foot d d" "foot tab Tab" "foot c c" "foot c c" "foot lmb button1"
    "foot ctrl Control_L" "foot space space" "foot shift Shift_L" "foot rmb button3" "foot capslock Caps_Lock"
    "foot alt Alt_L" "foot num8 KP_8" "foot num2 KP_2" "foot num4 KP_4" "foot num6 KP_6" "foot y y" "foot n n"
    "foot h h" "foot q q" "foot e e" "foot 2 2" "foot f f"
    "driver w w" "driver s s" "driver a a" "driver d d" "driver space space" "driver capslock Caps_Lock"
    "driver lmb button1" "driver ctrl Control_L" "driver alt Alt_L" "driver q q" "driver e e" "driver 2 2"
    "driver num8 KP_8" "driver num2 KP_2" "driver num4 KP_4" "driver num6 KP_6" "driver y y" "driver n n"
    "driver h h" "driver f f"
    "passenger w w" "passenger s s" "passenger capslock Caps_Lock" "passenger ctrl Control_L" "passenger q q"
    "passenger e e" "passenger 2 2" "passenger y y" "passenger n n" "passenger h h" "passenger f f"
)
# The mod's iterations: a phase's first key at its start, one key every KEY_GAP iterations held KEY_HOLD of them.
KEY_GAP=30
KEY_HOLD=10
KEY_START_FOOT=1600
KEY_START_DRIVER=2700
KEY_START_PASSENGER=3700
KEY_FRAMES=4200

# SA_GAME_KEYS, SA_GAME_LMB and SA_GAME_RMB for the mod's run of the steps.
keys_mod_env() {
    local step phase key x start i=0 last= keys= lmb= rmb= down
    for step in "${KEY_STEPS[@]}"; do
        read -r phase key x <<< "$step"
        [ "$phase" = "$last" ] || { i=0; last="$phase"; }
        case "$phase" in
            foot) start=$KEY_START_FOOT ;;
            driver) start=$KEY_START_DRIVER ;;
            passenger) start=$KEY_START_PASSENGER ;;
        esac
        down=$((start + i * KEY_GAP))
        case "$key" in
            lmb) lmb="${lmb:+$lmb,}$down:$((down + KEY_HOLD))" ;;
            rmb) rmb="${rmb:+$rmb,}$down:$((down + KEY_HOLD))" ;;
            *) keys="${keys:+$keys,}$key:$down:$((down + KEY_HOLD))" ;;
        esac
        i=$((i + 1))
    done
    export SA_GAME_KEYS="$keys" SA_GAME_LMB="$lmb" SA_GAME_RMB="$rmb"
}

# Presses one phase's keys on the DLL client's display (e2e_keydown / e2e_keyup of tools/e2e/lib.sh).
keys_dll_phase() { # PHASE
    local step phase key x
    for step in "${KEY_STEPS[@]}"; do
        read -r phase key x <<< "$step"
        [ "$phase" = "$1" ] || continue
        case "$x" in
            button*) DISPLAY="$E2E_DISPLAY" xdotool mousedown "${x#button}"; sleep 0.25
                DISPLAY="$E2E_DISPLAY" xdotool mouseup "${x#button}" ;;
            *) e2e_keydown "$x"; sleep 0.25; e2e_keyup "$x" ;;
        esac
        sleep 0.4
    done
}

# The server's key events of SLOG, one phase per block (the "[cov] keys" lines between the phase marks of the
# step log): what the two clients' runs are compared by.
keys_events() { # SLOG
    grep -oE '\[cov\] ([0-9]+ done|keys id=[0-9]+ (change|poll) .*)' "$1" | sed -E 's/^\[cov\] //; s/id=[0-9]+ //'
}
