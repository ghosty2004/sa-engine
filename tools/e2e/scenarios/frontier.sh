#!/bin/sh
# FRONTIER-1 scenarios: native sa-game runs (deterministic, --fixed-step 40, offscreen) that reach the blockers the
# user hit. Each must end with an empty blocker list in OUT/report.md and no crash.
#
#   tools/e2e/scenarios/frontier.sh pausemenu|solicit|crossing|crossing-long|roadblocks|melee|sniper|shootcar|shootdriver|quad|heli|plane|roadblock-script|roadblock-wanted|gangwar|boat OUTDIR [FRAMES]
#
# pausemenu (400)  Esc, down x3, Enter (pause-menu item 4: Stats), down, Enter, Enter: the Stats pages draw
#                  (was a SIGSEGV at iteration ~171: CStats::GetStatValue's X87 result dispatched as cdecl)
# solicit  (4000)  PEDHIT's car run without leaving the car: the prostitutes' CTaskComplexProstituteSolicit
#                  (ControlSubTask 0x6669D0) and IsEntityBlockingTarget 0x5F3970 run
# crossing (1200)  the Greenwood taken to the level crossing south of Grove Street (2200, -1895; traincross1/2) and
#                  driven over the tracks: CObject::ProcessTrainCrossingBehaviour 0x5A0B50 for the barriers; the
#                  trailers there (CTrailer 0x6CF1A0 / 0x6CF590 / 0x6CF6A0 / 0x6CFAC0, UpdateMovingCollision 0x6A1460)
#                  and the damage the car does (CVehicle::InflictDamage 0x6D7C90) bring the police: CCarAI::UpdateCarAI's
#                  law-enforcer path and CCarCtrl::SteerAICarWithPhysics_OnlyMission's chase missions (whole ports since
#                  FRONTIER-2 part 2): 0 blocked
# melee    (2600)  on foot at Grove Street, CJ sprints into the peds and punches (LMB): CTaskSimpleFight::FightHitPed,
#                  the damage anim / hit sounds (0x4B3FC0, 0x4E1CC0) and the victims' fight back (0x62A0A0, 0x69D170):
#                  0 blocked since FRONTIER-2
# sniper   (1200)  on foot at Grove Street: the weapon cheat KJKSZPJ, six `e` presses (next weapon) to the sniper rifle, then
#                  the right mouse button held with mouse motion: CCam::Process_M16_1stPerson 0x5105C0 (mode 7, the zoom, the
#                  look input, the line-of-sight near clip)
# shootcar (1200) the Greenwood left on the road at the level crossing, CJ 7 m south of it facing it, the weapon cheat
#                  KJKSZPJ, two `e` presses (the pistol slot), aiming (right button: camera mode 0x35, the shot line includes
#                  car tyres) and firing (left button) at it: the line of sight takes the car's tyres (CAutomobile::SetUpWheelColModel 0x6A3060, FRONTIER-2 part 3)
# shootdriver (1200) on foot at Grove Street, the weapon cheat and the pistol, CJ put 7 m in front of the nearest car
#                  with a driver (SA_GAME_JACK), aiming and firing at it (the user's free-roam case, FRONTIER-2 part 4): the
#                  shots damage the car, CVehicle::InflictDamage gives its occupants a CEventVehicleDamageWeapon, and the
#                  event group's clone is deleted later (0x61C390 -> 0x61C310 -> CEventVehicleDamage's destructor)
# crossing-long (4000) the crossing run continued to 4000 iterations: the police chase reaches
#                  CTaskComplexKillCriminal::CreateNextSubTask 0x68E4F0 / CreateSubTask 0x68C050 (FRONTIER-2 turn 2 lane A;
#                  frontier2-a-counts.gdb counts them): 0 blocked
# roadblocks (2000) the crossing run's Greenwood near Grove Street, the wanted-level cheat OSRBLHH typed twice in it (four
#                  stars: a road block chance), then driven: CRoadBlocks::GenerateRoadBlocks 0x4629E0's wanted-level path
#                  (whole port since FRONTIER-2 turn 2 lane A)
# quad (1200)     on foot at Grove Street, the quad cheat AKJJYGLC, CJ gets on the quad and drives it: the player quad's
#                  CQuadBike::ProcessControl 0x6CDCC0 (user-hit in the SA:MP web build, FRONTIER-2 turn 3 lane B)
# heli (1200)     on foot at Grove Street, the Hunter cheat OHDUDE, CJ gets in and lifts off (w held, then turns):
#                  CHeli::ProcessControl 0x6C7050 / ProcessFlyingCarStuff 0x6C4E60, the rotor, CVehicle::FlyingControl
#                  (FRONTIER-2 turn 4 lane B)
# plane (1200)    on foot at Grove Street, the stunt plane cheat URKQSRK, CJ gets in and throttles up along the street:
#                  CPlane::ProcessControl / ProcessFlyingCarStuff / PreRender, CVehicle::FlyingControl
# roadblock-script (1000) on foot at Grove Street; a script road block (aScriptRoadBlocks[0], 0xA43AB8) poked across the
#                  road 35 m west of CJ (SA_GAME_POKE), as SET_ROAD_BLOCK-style script state: GenerateRoadBlocks creates it
#                  (CRoadBlocks::CreateRoadBlockBetween2Points 0x4619C0, FRONTIER-2 turn 5 lane A)
# roadblock-wanted (2000) the roadblocks run's car with four stars taken to Idlewood (five road block nodes within 90 m)
#                  and the road block chance (CWanted +0x1C) poked to 127: every node coming in makes a block
#                  (GenerateRoadBlocks' wanted path)
# gangwar  (2000)  on foot in Ballas-held Idlewood; gang wars switched on (0x96AB90) and the provocation timer (0x96AB5C)
#                  poked above 2.1: CGangWars::Update starts an offensive war (StartOffensiveGangWar 0x446050, the
#                  attack waves 0x444810)
# WRAP="gdb -q -batch -x tools/e2e/scenarios/frontier2-a5-counts.gdb --args" counts the road block / gang war calls.
# boat (1200)     the script's boat car generators at the Bayside marina (-2227, 2446): CJ taken to Grove Street, then onto
#                  the dock next to the Squalo / Marquis / Jetmax, `f` to get in, w held: CBoat::ProcessControl 0x6F1770
#                  and CVehicle::ProcessBoatControl 0x6DBCE0 for every boat there (FRONTIER-2 turn 6 lane B; the Santa
#                  Maria pier has no boat and its teleport hung in the vorbis stream seek until ece2d244)
# WRAP="gdb -q -batch -x tools/e2e/scenarios/frontier2-counts.gdb --args" counts the FRONTIER-2 ports' calls
# (frontier2-3-counts.gdb: those of part 2).
# Env: SA_ROOT (checkout holding wine/gta-sa; default ~/sa-engine), SA_GAME (binary; default this checkout's
# i686-unknown-linux-gnu release build), WRAP (a command prefix, e.g. WRAP="gdb -q -batch -x tools/e2e/scenarios/frontier-counts.gdb --args"
# to count the ports' calls), plus any SA_GAME_* (SA_GAME_PRESENT_GAME=1 + PRESENT_DIR for frames).
set -eu
HERE=$(cd "$(dirname "$0")/../../.." && pwd)
NAME=$1; OUT=$2
B=${SA_GAME:-$HERE/target/i686-unknown-linux-gnu/release/sa-game}
GAME=${SA_ROOT:-$HOME/sa-engine}/wine/gta-sa
mkdir -p "$OUT"
export GLIBC_TUNABLES=glibc.malloc.mmap_max=0
run() { # run FRAMES [args]
    n=$1; shift
    if [ -n "${PRESENT_DIR:-}" ]; then set -- "$@" --present-dir "$PRESENT_DIR"; fi
    # shellcheck disable=SC2086
    exec systemd-run --user --scope -q -p MemoryMax=6G setarch i686 -R ${WRAP:-} "$B" --game "$GAME" --fixed-step 40 \
        --frames "$n" --report "$OUT/report.md" "$@" > "$OUT/stdout.txt" 2> "$OUT/stderr.txt"
}
case $NAME in
pausemenu)
    export SA_GAME_SCRIPT_SKIP=1
    export SA_GAME_KEYS="esc:100:102,down:130:132,down:136:138,down:142:144,enter:170:172,down:200:202,enter:210:212,enter:240:242"
    run "${3:-400}" --start-in-world ;;
solicit)
    export SA_GAME_TELEPORT="60:2505.0:-1666.4:13.4;300:2470.0:-1665.0:13.6:-2.246;400:2470.0:-1665.0:13.6:-2.364;530:2497.35:-1678.6:13.6:3.1416"
    export SA_GAME_KEYS=f:100:103,w:301:360,w:401:450,w:531:553
    run "${3:-4000}" --start-in-world ;;
crossing|crossing-long)
    export SA_GAME_TELEPORT="60:2505.0:-1666.4:13.4;300:2200.6:-1850.0:13.6:3.1416"
    export SA_GAME_KEYS=f:100:103,w:330:420,s:520:560
    if [ "$NAME" = crossing ]; then n=1200; else n=4000; fi
    run "${3:-$n}" --start-in-world ;;
roadblocks)
    export SA_GAME_TELEPORT="60:2505.0:-1666.4:13.4"
    keys="f:100:103"; i=150; for c in o s r b l h h o s r b l h h; do keys="$keys,$c:$i:$((i + 2))"; i=$((i + 6)); done
    export SA_GAME_KEYS="$keys,w:300:900,a:600:640,w:1000:1600,d:1200:1240"
    run "${3:-2000}" --start-in-world ;;
roadblock-script)
    export SA_GAME_SCRIPT_SKIP=1 SA_GAME_TELEPORT="60:2505.0:-1666.4:13.4"
    # aScriptRoadBlocks[0] (0xA43AB8): A (x, y, z), B (x, y, z), active +0x18, safe to create +0x19, gang +0x1A
    export SA_GAME_POKE="200:0xA43AB8:2470.0;200:0xA43ABC:-1655.0;200:0xA43AC0:13.4;200:0xA43AC4:2470.0;200:0xA43AC8:-1677.0;200:0xA43ACC:13.4;200:0xA43AD0:0x101"
    run "${3:-1000}" --start-in-world ;;
roadblock-wanted)
    # the roadblocks run's car (four stars), taken at 400 to Idlewood's east-west road (2060, -1752.3, heading west):
    # five road block nodes (ROADBLOX.DAT; 1981.5 / 2125.75 / 2058.4 / 2051.5, -1752 .. -1812) come within 90 m at once
    export SA_GAME_TELEPORT="60:2505.0:-1666.4:13.4;400:2060.0:-1752.3:13.5:1.5708"
    keys="f:100:103"; i=150; for c in o s r b l h h o s r b l h h; do keys="$keys,$c:$i:$((i + 2))"; i=$((i + 6)); done
    export SA_GAME_KEYS="$keys,w:300:380,w:420:900"
    # the chance (CWanted +0x1C) is recomputed whenever the chaos level changes: poked every 10 iterations
    p=; i=240; while [ $i -le "${3:-2000}" ]; do p="$p${p:+;}$i:[0xB7CD9C]+0x1C:127:2"; i=$((i + 10)); done
    export SA_GAME_POKE="$p"
    run "${3:-2000}" --start-in-world ;;
gangwar)
    export SA_GAME_TELEPORT="60:2070.0:-1730.0:13.5"
    export SA_GAME_POKE="200:0x96AB90:1:1;200:0x96AB5C:3.0"
    run "${3:-2000}" --start-in-world ;;
melee)
    # the PEDHIT foot run's sprint into the Grove Street peds (they are next to CJ at ~905), then punches every 25
    # iterations: CTaskSimpleFight::FightHitPed lands (7 hits by 2600); stops on HandlePedHit 0x4E1CC0 and
    # CEventDamage::ComputeDamageAnim 0x4B3FC0 (not ported yet)
    export SA_GAME_SCRIPT_SKIP=1 SA_GAME_KEYS="w:100:910,space:120:900"
    export SA_GAME_MOUSE="15:0:200:400,-20:3:500:700,25:-2:800:1000"
    export SA_GAME_LMB="${SA_GAME_LMB:-$(i=905; s=; while [ $i -lt 2400 ]; do s="$s${s:+,}$i:$((i + 4))"; i=$((i + 25)); done; echo "$s")}"
    run "${3:-2600}" --start-in-world --system-heap ;;
sniper)
    export SA_GAME_SCRIPT_SKIP=1
    keys=; i=150; for c in k j k s z p j; do keys="$keys${keys:+,}$c:$i:$((i + 2))"; i=$((i + 6)); done
    i=240; while [ $i -le 340 ]; do keys="$keys,e:$i:$((i + 2))"; i=$((i + 20)); done
    export SA_GAME_KEYS="$keys"
    export SA_GAME_RMB="${SA_GAME_RMB:-400:1150}"
    export SA_GAME_MOUSE="10:0:500:600,0:-6:650:700,-12:4:750:850"
    run "${3:-1200}" --start-in-world ;;
shootcar)
    # the crossing scenario's Greenwood, taken to the level crossing road (2200.6, -1850) and left there; CJ then stands
    # 7 m south of it facing north, takes the weapon cheat KJKSZPJ, two `e` presses (the pistol slot), aims and fires
    export SA_GAME_TELEPORT="60:2505.0:-1666.4:13.4;300:2200.6:-1850.0:13.6:3.1416;450:2200.6:-1857.0:13.6:0.0"
    keys="f:100:103,f:340:343"; i=470; for c in k j k s z p j; do keys="$keys,$c:$i:$((i + 2))"; i=$((i + 6)); done
    keys="$keys,e:540:542,e:560:562"
    export SA_GAME_KEYS="$keys"
    export SA_GAME_RMB="${SA_GAME_RMB:-600:1150}" SA_GAME_LMB="${SA_GAME_LMB:-640:1150}"
    run "${3:-1200}" --start-in-world ;;
shootdriver)
    # on foot at Grove Street: the weapon cheat KJKSZPJ, two `e` presses (the pistol slot), then CJ is put 7 m in front
    # of the nearest car with a driver (SA_GAME_JACK), aims (right button) and fires (left button) at it
    export SA_GAME_SCRIPT_SKIP=1
    keys=; i=150; for c in k j k s z p j; do keys="$keys${keys:+,}$c:$i:$((i + 2))"; i=$((i + 6)); done
    keys="$keys,e:240:242,e:260:262"
    export SA_GAME_KEYS="$keys" SA_GAME_JACK="${SA_GAME_JACK:-400:7}" SA_GAME_MOUSE="${SA_GAME_MOUSE:--3:-4:425:445}"
    export SA_GAME_RMB="${SA_GAME_RMB:-420:1150}" SA_GAME_LMB="${SA_GAME_LMB:-450:1150}"
    run "${3:-1200}" --start-in-world ;;
quad)
    # on foot at Grove Street: the quad cheat AKJJYGLC (CCheat::VehicleCheat puts a quad in front of CJ), CJ gets on it
    # (`f`) and drives (w, then a left and a right turn): CQuadBike::ProcessControl 0x6CDCC0 for the player's quad
    export SA_GAME_SCRIPT_SKIP=1
    keys=; i=150; for c in a k j j y g l c; do keys="$keys${keys:+,}$c:$i:$((i + 2))"; i=$((i + 6)); done
    export SA_GAME_KEYS="$keys,f:260:263,w:400:900,a:600:640,d:750:790"
    run "${3:-1200}" --start-in-world ;;
heli)
    # the Hunter cheat OHDUDE (CCheat::VehicleCheat, model info type 3: a CHeli in front of CJ), `f` to get in, w held
    # (the rotor spins up and the heli lifts), then a left and a right turn
    export SA_GAME_SCRIPT_SKIP=1
    keys=; i=150; for c in o h d u d e; do keys="$keys${keys:+,}$c:$i:$((i + 2))"; i=$((i + 6)); done
    export SA_GAME_KEYS="$keys,f:260:263,w:400:1150,a:700:760,d:850:910"
    run "${3:-1200}" --start-in-world ;;
plane)
    # the stunt plane cheat URKQSRK (model info type 4: a CPlane in front of CJ), `f` to get in, w held (throttle)
    export SA_GAME_SCRIPT_SKIP=1
    keys=; i=150; for c in u r k q s r k; do keys="$keys${keys:+,}$c:$i:$((i + 2))"; i=$((i + 6)); done
    export SA_GAME_KEYS="$keys,f:260:263,w:400:1150,a:800:840"
    run "${3:-1200}" --start-in-world ;;
boat)
    export SA_GAME_TELEPORT="60:2505.0:-1666.4:13.4;150:-2240.0:2420.0:8.0"
    export SA_GAME_KEYS="f:200:203,w:300:1100,a:700:760"
    run "${3:-1200}" --start-in-world ;;
*)
    echo "usage: $0 pausemenu|solicit|crossing|crossing-long|roadblocks|melee|sniper|shootcar|shootdriver|quad|heli|plane|roadblock-script|roadblock-wanted|gangwar|boat OUTDIR [FRAMES]" >&2
    exit 2 ;;
esac
