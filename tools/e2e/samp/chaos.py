#!/usr/bin/env python3
"""A random sampcov.pwn command list for the soak runs (tools/e2e/samp/soak.sh): what servers send in play and what
they send by mistake, in bursts, all over the map.

    python3 -I tools/e2e/samp/chaos.py SEED COUNT > chaos.cmds

Each line is a sampcov.pwn command (`sleep|S` waits, `restart|S` restarts the server; mod-join.sh runs them in
order). The mix: teleports across Los Santos, San Fierro, Las Venturas, the countryside, the sea, the sky and
interiors (with their interior ids and other virtual worlds), sometimes far outside the map; bursts of vehicles
(every model 400..611 and some that are none), objects (the game's, SA:MP's own 18631+ that the game does not
define, and ids that are no object), pickups, checkpoints, race checkpoints, map icons (also past the 64 radar
sprites), actors and their animations, player animations from the game's libraries and unknown ones, special
actions, weapons (also invalid ids), skins (also invalid ids), weather and time (also out of range), text draws,
dialogs, game texts, menus, sounds, explosions, attachments, camera moves, spectating, vehicle colours, paint jobs
and components, NPCs (spawned, moving, aiming, in vehicles, killed), and now and then a gamemode restart (`gmx`) or
a server kill and restart. The same SEED gives the same list.
"""
import random
import sys

# (x, y, z, interior) teleport targets
PLACES = [
    (2495, -1680, 13.5, 0), (1544, -1675, 13.6, 0), (1481, -1771, 18.8, 0), (2215, -1150, 25.8, 0),  # LS
    (1958, -2183, 13.5, 0), (370, -2050, 7.7, 0), (2811, -1088, 30.7, 0), (1172, -1323, 15.4, 0),
    (-1987, 138, 27.7, 0), (-2026, -101, 35.2, 0), (-1574, 663, 7.2, 0), (-2680, 1595, 64.0, 0),  # SF
    (-1421, -293, 14.1, 0), (-2316, -1655, 483.7, 0),  # SF airport, Mount Chiliad
    (2027, 1008, 10.8, 0), (2184, 1677, 11.0, 0), (1319, 1250, 10.8, 0), (2470, 2397, 10.8, 0),  # LV
    (-80, 1380, 10.0, 0), (-1302, 2531, 87.6, 0), (408, 2531, 16.6, 0), (-694, 945, 13.6, 0),  # desert, Verdant
    (693, -480, 16.3, 0), (2325, 52, 26.5, 0), (-2155, -2420, 30.6, 0), (1102, -321, 74.0, 0),  # countryside
    (0, 0, 3, 0), (-3500, 3500, 5, 0), (3000, -3000, 5, 0), (500, 500, 900, 0),  # sea, sky
    (2496, -1692, 1014.7, 3), (286, -41, 1001.5, 1), (-25, -186, 1003.5, 17), (1726, -1640, 20.2, 18),  # interiors
    (2233, 1714, 1012.4, 1), (-2637, 1404, 906.5, 3), (1494, 1304, 1093.3, 3), (-1405, -258, 1043.6, 7),
    (2324, -1149, 1050.7, 12), (-975, 1061, 1345.7, 10), (1412, -2, 1000.9, 1),
]
FAR = [(20000, 20000, 10), (-20000, 5, 10), (5, -30000, 3000), (3000, 3000, -500)]
ANIM_LIBS = {
    "PED": ["WALK_DRUNK", "SEAT_DOWN", "KO_SKID_FRONT", "HANDSUP", "IDLE_CHAT", "FALL_FALL"],
    "DANCING": ["DNCE_M_A", "DNCE_M_B", "DAN_LOOP_A"],
    "GANGS": ["PRTIAL_GNGTLKA", "HNDSHKFA", "SMKCIG_PRTL"],
    "BEACH": ["BATHER", "LAY_BAC_LOOP"],
    "SWEET": ["SWEET_ASS_SLAP", "SWEET_INJUREDLOOP"],
    "CRACK": ["CRCKDETH2", "CRCKIDLE1"],
    "ON_LOOKERS": ["SHOUT_01", "WAVE_LOOP"],
    "BOMBER": ["BOM_PLANT"],
    "SHOP": ["SHP_ROB_HANDSUP"],
    "RIOT": ["RIOT_CHANT", "RIOT_PUNCHES"],
}
UNKNOWN_ANIMS = [("NOTALIB", "NOTANIM"), ("PED", "NO_SUCH_ANIM"), ("CUSTOMLIB_SERVER", "DANCE1"),
                 ("A_VERY_LONG_LIBRARY_NAME_X", "A_VERY_LONG_ANIMATION_NAME_THAT_OVERFLOWS"), ("", ""), ("ped", "walk_drunk")]
# object models: common map objects, weapons, SA:MP's own (not in the game), ids that are no object
OBJECTS = [1225, 1337, 1454, 2780, 3095, 3524, 4206, 1318, 1239, 1240, 1242, 1247, 1274, 1275, 1210, 1212, 2985,
           19300, 18631, 18700, 19999, 345, 356, 371, 0, 400, 411, 0x7FFF, 20000, 65535, 1, 106]
WEAPONS = list(range(0, 47)) + [47, 50, 100, 255, -1]
SKINS = list(range(0, 312)) + [74, 312, 400, 1000, -5]
VEHICLES = list(range(400, 612)) + [399, 612, 0, 1337, 19000, -1]
PICKUP_MODELS = [1240, 1242, 1239, 1247, 1274, 1275, 1210, 1212, 1318, 356, 371, 1550, 954, 18631, 411, 105, 0]


class Gen:
    def __init__(self, seed):
        self.r = random.Random(seed)
        self.here = PLACES[0]
        self.npcs = 0
        self.out = []

    def emit(self, *fields):
        self.out.append("|".join(str(f) for f in fields))

    def f(self, v):
        return f"{v:.2f}"

    def near(self, d=15):
        x, y, z, _ = self.here
        r = self.r
        return self.f(x + r.uniform(-d, d)), self.f(y + r.uniform(-d, d)), self.f(z + r.uniform(0, 3))

    def teleport(self):
        r = self.r
        roll = r.random()
        if roll < 0.08:
            x, y, z = r.choice(FAR)
            self.emit("pos", x, y, z, r.randint(0, 359))
            return
        if roll < 0.25:
            x, y, z, i = r.uniform(-2990, 2990), r.uniform(-2990, 2990), r.uniform(0, 200), 0
        else:
            x, y, z, i = r.choice(PLACES)
        self.here = (x, y, z, i)
        self.emit("interior", i)
        self.emit("vw", 0 if r.random() < 0.85 else r.randint(1, 100))
        self.emit("pos", self.f(x), self.f(y), self.f(z), r.randint(0, 359))
        self.emit("sleep", r.choice([1, 1, 2, 3]))

    def vehicles(self):
        r = self.r
        for _ in range(r.randint(1, 25)):
            x, y, z = self.near(30)
            self.emit("veh", r.choice(VEHICLES), x, y, z, r.randint(0, 359))
        if r.random() < 0.5:
            v = r.randint(1, 300)
            self.emit("putin", v, r.choice([0, 0, 1, 2, 3, 4, 7, 8, 255]))
            self.emit("sleep", 1)
            self.emit("vcolour", v, r.randint(0, 255), r.randint(-1, 300))
            self.emit("paintjob", v, r.randint(0, 5))
            self.emit("tune", v, r.choice([1010, 1025, 1073, 1087, 999, 1200, 0]))
            self.emit("vhp", v, r.choice([1000, 300, 0, -100]))
            if r.random() < 0.5:
                self.emit("eject")

    def objects(self):
        r = self.r
        for _ in range(r.randint(1, 40)):
            x, y, z = self.near(40)
            self.emit("obj", r.choice(OBJECTS), x, y, z)
        if r.random() < 0.3:
            o = r.randint(1, 1000)
            x, y, z = self.near(40)
            self.emit("objmove", o, x, y, z, r.choice([1, 5, 50]))
        if r.random() < 0.2:
            self.emit("objdel", r.randint(1, 1000))
        if r.random() < 0.1:
            self.emit("objscene")
            self.emit("sleep", 2)
            self.emit("objscene2")

    def markers(self):
        r = self.r
        for _ in range(r.randint(1, 10)):
            x, y, z = self.near(50)
            self.emit("pickup", r.choice(PICKUP_MODELS), r.choice([1, 2, 3, 8, 14, 19, 22, 23, 0, 100]), x, y, z)
        x, y, z = self.near(50)
        self.emit("cp", x, y, z, r.choice([3, 8, 0.5, 50]))
        nx, ny, nz = self.near(80)
        self.emit("rcp", r.randint(0, 9), x, y, z, nx, ny, nz, r.choice([5, 1, 20]))
        for _ in range(r.randint(1, 8)):
            x, y, z = self.near(300)
            self.emit("icon", r.randint(0, 99), x, y, z, r.choice(list(range(0, 64)) + [64, 99, 200, 255]))
        if r.random() < 0.3:
            self.emit("iconoff", r.randint(0, 99))
        if r.random() < 0.3:
            self.emit("cpoff")
        if r.random() < 0.2:
            x, y, _ = self.near(100)
            self.emit("gz", x, y, self.f(float(x) + r.uniform(10, 300)), self.f(float(y) + r.uniform(10, 300)))
        if r.random() < 0.2:
            x, y, z = self.near(10)
            self.emit("label", r.choice(["label", "{FF0000}red ~r~x", "x" * 200]), x, y, z)

    def actors(self):
        r = self.r
        for _ in range(r.randint(1, 6)):
            x, y, z = self.near(10)
            self.emit("actor", r.choice(SKINS), x, y, z, r.randint(0, 359))
        a = r.randint(0, 50)
        lib, name = self.anim()
        self.emit("actoranim", a, lib, name)
        if r.random() < 0.2:
            self.emit("actordel", r.randint(0, 50))

    def anim(self):
        r = self.r
        if r.random() < 0.3:
            return r.choice(UNKNOWN_ANIMS)
        lib = r.choice(list(ANIM_LIBS))
        return lib, r.choice(ANIM_LIBS[lib])

    def player(self):
        r = self.r
        k = r.randint(0, 15)
        if k == 0:
            lib, name = self.anim()
            self.emit("anim", lib, name)
        elif k == 1:
            self.emit("clranim")
        elif k == 2:
            self.emit("special", r.choice([0, 1, 2, 5, 6, 7, 8, 10, 11, 12, 13, 20, 21, 22, 23, 24, 25, 68, 99, 255]))
        elif k == 3:
            self.emit("weapon", r.choice(WEAPONS), r.choice([1, 100, 9999]))
        elif k == 4:
            self.emit("skin", r.choice(SKINS))
        elif k == 5:
            self.emit("weather", r.choice(list(range(0, 23)) + [23, 44, 150, 255, 500]))
            self.emit("time", r.choice(list(range(0, 24)) + [24, 99, -1]))
        elif k == 6:
            self.emit("hp", r.choice([100, 50, 1, 0, 1000]))
            self.emit("armour", r.choice([100, 0, 50]))
        elif k == 7:
            self.emit("wanted", r.choice([0, 1, 3, 6, 7, 10, 255]))
            self.emit("money", r.randint(-100000, 100000))
        elif k == 8:
            self.emit("drunk", r.choice([0, 2000, 50000]))
            self.emit("gravity", r.choice(["0.008", "0.001", "0.05", "0.008"]))
        elif k == 9:
            x, y, z = self.near(20)
            lx, ly, lz = self.near(5)
            self.emit("campos", x, y, z, lx, ly, lz)
            self.emit("sleep", 1)
            self.emit("camback")
        elif k == 10:
            self.emit("spec", 1)
            self.emit("sleep", 2)
            self.emit("spec", 0)
        elif k == 11:
            self.emit("attach", r.choice(OBJECTS), r.randint(0, 20))
        elif k == 12:
            x, y, z = self.near(20)
            self.emit("explode", x, y, z, r.randint(0, 13), r.choice([1, 5, 20]))
        elif k == 13:
            self.emit("sound", r.choice([1052, 1053, 1054, 1057, 1058, 1139, 1149, 0, 99999, 40000]))
        elif k == 14:
            self.emit("control", r.randint(0, 1))
            self.emit("control", 1)
        else:
            self.emit("kill")
            self.emit("sleep", 4)

    def ui(self):
        r = self.r
        k = r.randint(0, 5)
        if k == 0:
            self.emit("td", r.choice(["~r~red ~w~white", "x" * 400, "~k~~PED_FIREWEAPON~", "~", "_"]))
        elif k == 1:
            self.emit("tdhide")
        elif k == 2:
            self.emit("dialog", r.randint(0, 32767), r.randint(0, 5), "Caption", r.choice(["Body", "a\\tb\\nc\\td", "x" * 300]))
        elif k == 3:
            self.emit("gametext", r.randint(0, 7), r.randint(100, 5000), r.choice(["~g~text", "~", "x" * 200]))
        elif k == 4:
            self.emit("menu")
        else:
            self.emit("msg", r.choice(["hello", "{FF0000}red", "x" * 140, "%s %d"]))

    def npcs_step(self):
        r = self.r
        k = r.randint(0, 9)
        n = r.randint(0, self.npcs + 1)
        if k <= 2 and self.npcs < 30:
            x, y, z = self.near(10)
            self.emit("npc", f"Npc{self.npcs}", x, y, z, r.choice(SKINS), r.choice(WEAPONS))
            self.npcs += 1
        elif k == 3:
            x, y, z = self.near(30)
            self.emit("npcmove", n, x, y, z)
        elif k == 4:
            self.emit("npcfollow", n)
        elif k == 5:
            self.emit("npcaim", n, r.randint(0, 1))
        elif k == 6:
            lib, name = self.anim()
            self.emit("npcanim", n, lib, name)
        elif k == 7:
            self.emit(r.choice(["npcveh", "npcenter"]), n, r.randint(1, 300), r.randint(0, 4))
        elif k == 8:
            self.emit("npckill", n)
        else:
            self.emit("npcdel", n)

    def step(self):
        r = self.r
        roll = r.random()
        if roll < 0.10:
            self.teleport()
        elif roll < 0.22:
            self.vehicles()
        elif roll < 0.36:
            self.objects()
        elif roll < 0.46:
            self.markers()
        elif roll < 0.53:
            self.actors()
        elif roll < 0.73:
            self.player()
        elif roll < 0.81:
            self.ui()
        elif roll < 0.93:
            self.npcs_step()
        elif roll < 0.965:
            self.emit("clearvehs")
            self.emit("clearnpcs")
            self.npcs = 0
        elif roll < 0.99:
            self.emit("sleep", r.choice([2, 5, 10]))
        elif roll < 0.995:
            self.emit("rcon", "gmx")
            self.emit("sleep", 20)
        else:
            self.emit("restart", r.choice([1, 5]))
            self.emit("sleep", 30)
            self.npcs = 0


def main():
    seed, count = int(sys.argv[1]), int(sys.argv[2])
    g = Gen(seed)
    while len(g.out) < count:
        g.step()
    print("\n".join(g.out[:count]))


if __name__ == "__main__":
    main()
