// SAMP-1D gamemode: drives the SA:MP coverage scenarios of tools/e2e/samp-cov.sh.
//
// Classes for the class-selection screen, a static train, and a command channel: the driver appends
// lines "SEQ CMD|ARG|..." to scriptfiles/sampcov.txt; every 100 ms the lines with a SEQ above the last
// one run is executed and "[cov] SEQ done CMD" is printed. Every player callback prints a "[cov]" line
// so the driver can wait for server-side observations. "L" is the first (local) player.
//
//   msg|text  gametext|style|ms|text  dialog|id|style|caption|body  td|text  tdhide
//   pos|x|y|z|angle  interior|i  vw|world  hp|value  armour|value  kill  weapon|weapon|ammo  skin|skin
//   veh|model|x|y|z|angle  vehs|model|count|x|y|z (a grid, 6 m apart)  putin|vehid|seat  eject  vhp|vehid|health
//   vdel|vehid  stunt|1/0
//   obj|model|x|y|z  objr|model|x|y|z|rx|ry|rz  objmove|id|x|y|z|speed  pickup|model|type|x|y|z  label|text|x|y|z
//   cp|x|y|z|size  rcp|type|x|y|z|nx|ny|nz|size  cpoff  gz|minx|miny|maxx|maxy  icon|id|x|y|z|type
//   actor|skin|x|y|z|angle  explode|x|y|z|type|radius  time|h  weather|w  money|n  wanted|n
//   anim|lib|name  clranim  special|action  campos|x|y|z|lx|ly|lz  camback  spec|1/0  drunk|level  attach|model|bone
//   animfor|lib|name|ms (an animation that moves the player, cleared after ms: "[cov] anim-end pos=...")
//   menu  hidemenu  forceclass  sound|id  gravity|g  control|0/1  rcon|command  spawnp (SpawnPlayer)  freezeonspawn|0/1
//   (control off in OnPlayerSpawn, as login scripts do)
//   Objects (tools/e2e/samp/objects.cmds): objscene (a small mapping with materials, a moving and turning object,
//   an object on the player and one on a bone, a removed house)  objscene2 (moves, stops, rotations, a deletion)
//   objwhere (every scene object's position)  rmbuild|model|x|y|z|radius
//   track|ms (the "where" line every ms besides the once a second one; 0 stops it)
//   NPCs (open.mp NPC component; remote players for the client):
//   npc|name|x|y|z|skin|weapon  npcmove|id|x|y|z  npcfollow|id  npcaim|id|shoot  npcstop|id  npcmelee|id
//   npcanim|id|lib|name  npcveh|id|vehid|seat  npcenter|id|vehid|seat  npckill|id  npcdel|id  clearnpcs  clearvehs (all but the train)
//   More (tools/e2e/samp/chaos.py): actoranim|id|lib|name  actordel|id  vcolour|vehid|c1|c2  paintjob|vehid|n
//   tune|vehid|component  iconoff|id  objdel|id
#include <open.mp>

new gLast = 0;
new gLocal = INVALID_PLAYER_ID;
new Text:gTd = Text:INVALID_TEXT_DRAW;
new Menu:gMenu = Menu:INVALID_MENU;
// The vehicles `vehc` created since the last `clearvehs`; the vehicle commands below name them 1, 2, ...
new gVehs[16];
new gVehCount = 0;
new gObj[8] = { INVALID_OBJECT_ID, ... };
// The local player's last polled GetPlayerKeys (keys, up/down, left/right): the arrow keys raise no key-change event.
new gKeys[3];
new bool:gFreezeOnSpawn = false;
new gTrack = 0;

// Built with FILTERSCRIPT=1 it is a filterscript beside another gamemode (the soak runs of soak.sh): the command
// channel and the "[cov]" lines without the classes, the train and the class-selection camera.
#if defined FILTERSCRIPT
public OnFilterScriptInit()
{
    CovStart();
    return 1;
}
#else
main() {}

public OnGameModeInit()
{
    SetGameModeText("sampcov");
    // Grove Street, Unity Station area, Los Santos airport.
    AddPlayerClass(0, 2495.0, -1687.0, 13.5, 0.0, WEAPON_DEAGLE, 50, WEAPON_M4, 300, WEAPON_FIST, 0);
    AddPlayerClass(105, 2495.0, -1687.0, 13.5, 0.0, WEAPON_COLT45, 100, WEAPON_SHOTGUN, 30, WEAPON_FIST, 0);
    AddPlayerClass(285, 2495.0, -1687.0, 13.5, 0.0, WEAPON_MP5, 200, WEAPON_GRENADE, 3, WEAPON_FIST, 0);
    // Trains can only be added statically.
    printf("[cov] static train %d", AddStaticVehicle(538, 1700.7551, -1953.6531, 14.8756, 200.0, 1, 1));
    UsePlayerPedAnims();
    // Built with NO_ENEX=1 (PAWN_DEFINES of the drivers): the server's InitGame tells the clients to disable the
    // entry-exits.
#if defined NO_ENEX
    DisableInteriorEnterExits();
    print("[cov] entry-exits disabled");
#endif
    CovStart();
    return 1;
}
#endif

CovStart()
{
    new File:f = fopen("sampcov.txt", io_write);
    if (f) fclose(f);
    gLast = 0;
    SetTimer("CovPoll", 100, true);
    SetTimer("CovState", 1000, true);
    print("[cov] loaded");
}

public OnPlayerConnect(playerid)
{
    new name[MAX_PLAYER_NAME + 1];
    GetPlayerName(playerid, name, sizeof name);
    if (gLocal == INVALID_PLAYER_ID && !IsPlayerNPC(playerid)) gLocal = playerid;
    printf("[cov] connect id=%d name=%s", playerid, name);
    return 1;
}

public OnPlayerDisconnect(playerid, reason)
{
    if (playerid == gLocal) gLocal = INVALID_PLAYER_ID;
    printf("[cov] disconnect id=%d reason=%d", playerid, reason);
    return 1;
}

public OnPlayerRequestClass(playerid, classid)
{
#if !defined FILTERSCRIPT
    SetPlayerPos(playerid, 2495.0, -1674.0, 13.4);
    SetPlayerFacingAngle(playerid, 180.0);
    SetPlayerCameraPos(playerid, 2495.0, -1680.0, 14.9);
    SetPlayerCameraLookAt(playerid, 2495.0, -1674.0, 13.9);
#endif
    printf("[cov] class id=%d class=%d", playerid, classid);
    return 1;
}

public OnPlayerSpawn(playerid)
{
    printf("[cov] spawn id=%d", playerid);
    if (gFreezeOnSpawn) TogglePlayerControllable(playerid, false);
    return 1;
}
public OnPlayerDeath(playerid, killerid, WEAPON:reason) { printf("[cov] death id=%d killer=%d reason=%d", playerid, killerid, _:reason); return 1; }
public OnPlayerText(playerid, text[]) { printf("[cov] text id=%d '%s'", playerid, text); return 1; }
public OnPlayerCommandText(playerid, cmdtext[]) { printf("[cov] command id=%d '%s'", playerid, cmdtext); return 1; }
public OnPlayerStateChange(playerid, PLAYER_STATE:newstate, PLAYER_STATE:oldstate)
{
    printf("[cov] state id=%d %d->%d veh=%d", playerid, _:oldstate, _:newstate, GetPlayerVehicleID(playerid));
    return 1;
}
public OnDialogResponse(playerid, dialogid, response, listitem, inputtext[])
{
    printf("[cov] dialog-response id=%d dialog=%d response=%d item=%d", playerid, dialogid, response, listitem);
    return 1;
}
public OnPlayerWeaponShot(playerid, WEAPON:weaponid, BULLET_HIT_TYPE:hittype, hitid, Float:fX, Float:fY, Float:fZ)
{
    printf("[cov] shot id=%d weapon=%d hittype=%d", playerid, _:weaponid, _:hittype);
    return 1;
}
public OnPlayerTakeDamage(playerid, issuerid, Float:amount, WEAPON:weaponid, bodypart)
{
    printf("[cov] take-damage id=%d from=%d amount=%.1f weapon=%d", playerid, issuerid, amount, _:weaponid);
    return 1;
}
public OnPlayerPickUpPickup(playerid, pickupid) { printf("[cov] pickup id=%d pickup=%d", playerid, pickupid); return 1; }
public OnPlayerEnterCheckpoint(playerid) { printf("[cov] cp-enter id=%d", playerid); return 1; }
public OnPlayerEnterRaceCheckpoint(playerid) { printf("[cov] rcp-enter id=%d", playerid); return 1; }
public OnPlayerInteriorChange(playerid, newinteriorid, oldinteriorid) { printf("[cov] interior id=%d %d->%d", playerid, oldinteriorid, newinteriorid); return 1; }
public OnVehicleDeath(vehicleid, killerid) { printf("[cov] vehicle-death %d by %d", vehicleid, killerid); return 1; }
public OnPlayerSelectedMenuRow(playerid, row) { printf("[cov] menu-row id=%d row=%d", playerid, row); return 1; }
public OnPlayerExitedMenu(playerid) { printf("[cov] menu-exit id=%d", playerid); return 1; }
public OnObjectMoved(objectid) { printf("[cov] object-moved %d", objectid); return 1; }
public OnPlayerKeyStateChange(playerid, KEY:newkeys, KEY:oldkeys)
{
    printf("[cov] keys id=%d change new=%d old=%d", playerid, _:newkeys, _:oldkeys);
    return 1;
}

forward CovAnimEnd();
public CovAnimEnd()
{
    if (gLocal == INVALID_PLAYER_ID) return 1;
    new Float:x, Float:y, Float:z;
    ClearAnimations(gLocal, SYNC_ALL);
    GetPlayerPos(gLocal, x, y, z);
    printf("[cov] anim-end pos=%.2f,%.2f,%.2f", x, y, z);
    return 1;
}

forward CovState();
public CovState()
{
    if (gLocal == INVALID_PLAYER_ID) return 1;
    new Float:x, Float:y, Float:z, Float:hp;
    GetPlayerPos(gLocal, x, y, z);
    GetPlayerHealth(gLocal, hp);
    printf("[cov] where st=%d pos=%.2f,%.2f,%.2f int=%d veh=%d hp=%.0f weapon=%d",
        _:GetPlayerState(gLocal), x, y, z, GetPlayerInterior(gLocal), GetPlayerVehicleID(gLocal), hp,
        _:GetPlayerWeapon(gLocal));
    return 1;
}

// Field N (0-based) of a '|'-separated line.
field(const line[], n, dest[], size = sizeof dest)
{
    new k = 0, start = 0;
    for (new i = 0; ; i++)
    {
        if (line[i] == '|' || line[i] == 0)
        {
            if (k == n)
            {
                strmid(dest, line, start, i, size);
                return 1;
            }
            if (line[i] == 0) break;
            k++;
            start = i + 1;
        }
    }
    dest[0] = 0;
    return 0;
}

in(const line[], n) { new b[32]; field(line, n, b); return strval(b); }
vs(const line[], n) { new k = in(line, n); return (k >= 1 && k <= gVehCount) ? gVehs[k - 1] : INVALID_VEHICLE_ID; }
Float:fl(const line[], n) { new b[32]; field(line, n, b); return floatstr(b); }

Run(seq, const line[])
{
    new cmd[24], a[256], b[256];
    new p = gLocal;
    field(line, 0, cmd);
    if (!strcmp(cmd, "msg")) { field(line, 1, a); SendClientMessage(p, 0xFFFF00FF, "%s", a); SendClientMessageToAll(0xFFFFFFFF, "all: %s", a); }
    else if (!strcmp(cmd, "gametext")) { field(line, 3, a); GameTextForPlayer(p, "%s", in(line, 2), in(line, 1), a); }
    else if (!strcmp(cmd, "dialog")) { field(line, 3, a); field(line, 4, b); ShowPlayerDialog(p, in(line, 1), DIALOG_STYLE:in(line, 2), a, b, "OK", "Cancel"); }
    else if (!strcmp(cmd, "td"))
    {
        field(line, 1, a);
        if (gTd != Text:INVALID_TEXT_DRAW) TextDrawDestroy(gTd);
        gTd = TextDrawCreate(320.0, 200.0, "%s", a);
        TextDrawFont(gTd, TEXT_DRAW_FONT_1);
        TextDrawLetterSize(gTd, 0.5, 1.5);
        TextDrawUseBox(gTd, true);
        TextDrawBoxColour(gTd, 0x00000088);
        TextDrawAlignment(gTd, TEXT_DRAW_ALIGN_CENTER);
        TextDrawShowForPlayer(p, gTd);
        new Text:t2 = TextDrawCreate(100.0, 300.0, "LD_SPAC:white");
        TextDrawFont(t2, TEXT_DRAW_FONT_SPRITE_DRAW);
        TextDrawTextSize(t2, 40.0, 40.0);
        TextDrawShowForPlayer(p, t2);
        new Text:t3 = TextDrawCreate(500.0, 300.0, "_");
        TextDrawFont(t3, TEXT_DRAW_FONT_MODEL_PREVIEW);
        TextDrawSetPreviewModel(t3, 411);
        TextDrawTextSize(t3, 80.0, 80.0);
        TextDrawShowForPlayer(p, t3);
    }
    else if (!strcmp(cmd, "tdhide")) TextDrawHideForAll(gTd);
    else if (!strcmp(cmd, "pos")) { SetPlayerPos(p, fl(line, 1), fl(line, 2), fl(line, 3)); SetPlayerFacingAngle(p, fl(line, 4)); SetCameraBehindPlayer(p); }
    else if (!strcmp(cmd, "interior")) SetPlayerInterior(p, in(line, 1));
    else if (!strcmp(cmd, "vw")) SetPlayerVirtualWorld(p, in(line, 1));
    else if (!strcmp(cmd, "hp")) SetPlayerHealth(p, fl(line, 1));
    else if (!strcmp(cmd, "armour")) SetPlayerArmour(p, fl(line, 1));
    else if (!strcmp(cmd, "kill")) SetPlayerHealth(p, 0.0);
    else if (!strcmp(cmd, "weapon")) { GivePlayerWeapon(p, WEAPON:in(line, 1), in(line, 2)); SetPlayerArmedWeapon(p, WEAPON:in(line, 1)); }
    else if (!strcmp(cmd, "skin")) SetPlayerSkin(p, in(line, 1));
    else if (!strcmp(cmd, "veh"))
    {
        new v = CreateVehicle(in(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), fl(line, 5), -1, -1, -1);
        printf("[cov] %d veh %d", seq, v);
    }
    else if (!strcmp(cmd, "vehs"))
    {
        new n = 0;
        for (new i = 0; i < in(line, 2); i++)
        {
            if (CreateVehicle(in(line, 1), fl(line, 3) + float(i % 12) * 6.0, fl(line, 4) + float(i / 12) * 6.0, fl(line, 5), 0.0, -1, -1, -1) != INVALID_VEHICLE_ID) n++;
        }
        printf("[cov] %d vehs %d", seq, n);
    }
    else if (!strcmp(cmd, "stunt")) EnableStuntBonusForPlayer(p, in(line, 1) != 0);
    else if (!strcmp(cmd, "putin")) PutPlayerInVehicle(p, in(line, 1), in(line, 2));
    else if (!strcmp(cmd, "eject")) RemovePlayerFromVehicle(p);
    else if (!strcmp(cmd, "vhp")) SetVehicleHealth(in(line, 1), fl(line, 2));
    else if (!strcmp(cmd, "vdel")) DestroyVehicle(in(line, 1));
    else if (!strcmp(cmd, "obj")) printf("[cov] %d obj %d", seq, CreateObject(in(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), 0.0, 0.0, 0.0));
    else if (!strcmp(cmd, "objr")) printf("[cov] %d obj %d", seq, CreateObject(in(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), fl(line, 5), fl(line, 6), fl(line, 7)));
    else if (!strcmp(cmd, "track"))
    {
        if (gTrack) KillTimer(gTrack);
        gTrack = in(line, 1) > 0 ? SetTimer("CovState", in(line, 1), true) : 0;
    }
    else if (!strcmp(cmd, "objmove")) MoveObject(in(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), fl(line, 5));
    else if (!strcmp(cmd, "pickup")) printf("[cov] %d pickup %d", seq, CreatePickup(in(line, 1), in(line, 2), fl(line, 3), fl(line, 4), fl(line, 5)));
    else if (!strcmp(cmd, "label")) { field(line, 1, a); Create3DTextLabel(a, 0x00FF00FF, fl(line, 2), fl(line, 3), fl(line, 4), 50.0, 0); }
    else if (!strcmp(cmd, "cp")) SetPlayerCheckpoint(p, fl(line, 1), fl(line, 2), fl(line, 3), fl(line, 4));
    else if (!strcmp(cmd, "rcp")) SetPlayerRaceCheckpoint(p, CP_TYPE:in(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), fl(line, 5), fl(line, 6), fl(line, 7), fl(line, 8));
    else if (!strcmp(cmd, "cpoff")) { DisablePlayerCheckpoint(p); DisablePlayerRaceCheckpoint(p); }
    else if (!strcmp(cmd, "gz"))
    {
        new z = GangZoneCreate(fl(line, 1), fl(line, 2), fl(line, 3), fl(line, 4));
        GangZoneShowForPlayer(p, z, 0xFF000088);
        GangZoneFlashForPlayer(p, z, 0x0000FF88);
    }
    else if (!strcmp(cmd, "icon")) SetPlayerMapIcon(p, in(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), in(line, 5), 0, MAPICON_GLOBAL);
    else if (!strcmp(cmd, "actor")) printf("[cov] %d actor %d", seq, CreateActor(in(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), fl(line, 5)));
    else if (!strcmp(cmd, "actoranim")) { field(line, 2, a); field(line, 3, b); ApplyActorAnimation(in(line, 1), a, b, 4.1, true, false, false, false, 0); }
    else if (!strcmp(cmd, "actordel")) DestroyActor(in(line, 1));
    else if (!strcmp(cmd, "vcolour")) ChangeVehicleColours(in(line, 1), in(line, 2), in(line, 3));
    else if (!strcmp(cmd, "paintjob")) ChangeVehiclePaintjob(in(line, 1), in(line, 2));
    else if (!strcmp(cmd, "tune")) AddVehicleComponent(in(line, 1), in(line, 2));
    else if (!strcmp(cmd, "iconoff")) RemovePlayerMapIcon(p, in(line, 1));
    else if (!strcmp(cmd, "objdel")) DestroyObject(in(line, 1));
    else if (!strcmp(cmd, "explode")) CreateExplosion(fl(line, 1), fl(line, 2), fl(line, 3), in(line, 4), fl(line, 5));
    else if (!strcmp(cmd, "time")) SetPlayerTime(p, in(line, 1), 0);
    else if (!strcmp(cmd, "weather")) SetPlayerWeather(p, in(line, 1));
    else if (!strcmp(cmd, "money")) GivePlayerMoney(p, in(line, 1));
    else if (!strcmp(cmd, "wanted")) SetPlayerWantedLevel(p, in(line, 1));
    else if (!strcmp(cmd, "anim")) { field(line, 1, a); field(line, 2, b); ApplyAnimation(p, a, b, 4.1, true, false, false, false, 0, SYNC_ALL); }
    else if (!strcmp(cmd, "animfor"))
    {
        field(line, 1, a);
        field(line, 2, b);
        ApplyAnimation(p, a, b, 4.1, true, true, true, false, 0, SYNC_ALL);
        SetTimer("CovAnimEnd", in(line, 3), false);
    }
    else if (!strcmp(cmd, "clranim")) ClearAnimations(p, SYNC_ALL);
    else if (!strcmp(cmd, "special")) SetPlayerSpecialAction(p, SPECIAL_ACTION:in(line, 1));
    else if (!strcmp(cmd, "campos")) { SetPlayerCameraPos(p, fl(line, 1), fl(line, 2), fl(line, 3)); SetPlayerCameraLookAt(p, fl(line, 4), fl(line, 5), fl(line, 6)); }
    else if (!strcmp(cmd, "camback")) SetCameraBehindPlayer(p);
    else if (!strcmp(cmd, "spec")) TogglePlayerSpectating(p, in(line, 1) != 0);
    else if (!strcmp(cmd, "drunk")) SetPlayerDrunkLevel(p, in(line, 1));
    else if (!strcmp(cmd, "attach")) SetPlayerAttachedObject(p, 0, in(line, 1), in(line, 2), 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0, 0);
    else if (!strcmp(cmd, "rmbuild")) RemoveBuildingForPlayer(p, in(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), fl(line, 5));
    else if (!strcmp(cmd, "objscene"))
    {
        // a metal platform with a marble top, a red fence, a tower drawn from afar, a marble roof piece, a crate
        // that rises and turns, a crate above the player and a rifle on the player's back
        gObj[0] = CreateObject(3095, 2495.0, -1662.0, 12.4, 0.0, 0.0, 0.0);
        gObj[1] = CreateObject(987, 2486.0, -1668.0, 12.5, 0.0, 0.0, 90.0);
        gObj[2] = CreateObject(3279, 2506.0, -1662.0, 12.5, 0.0, 0.0, 45.0, 300.0);
        gObj[3] = CreateObject(1697, 2495.0, -1652.0, 15.0, 10.0, 0.0, 30.0);
        gObj[4] = CreateObject(1271, 2491.0, -1668.0, 13.0, 0.0, 0.0, 0.0);
        gObj[5] = CreateObject(1271, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        SetObjectMaterial(gObj[0], 0, 3922, "bistro", "Marble2", 0xFFFFFFFF);
        SetObjectMaterial(gObj[1], 0, -1, "none", "none", 0xFFFF0000);
        MoveObject(gObj[4], 2491.0, -1668.0, 17.0, 1.0, 0.0, 0.0, 180.0);
        SetObjectMaterial(gObj[3], 0, 3922, "bistro", "Marble2", 0);
        AttachObjectToPlayer(gObj[5], p, 0.0, 0.0, 3.0, 0.0, 0.0, 45.0);
        SetObjectNoCameraCol(gObj[3]);
        SetPlayerAttachedObject(p, 1, 355, 1, 0.1, -0.2, 0.0, 0.0, 60.0, 0.0, 1.0, 1.0, 1.0, 0, 0);
        RemoveBuildingForPlayer(p, -1, 2517.48, -1644.70, 15.2, 6.0);
        for (new i = 0; i < 6; i++) printf("[cov] %d objscene %d id %d", seq, i, gObj[i]);
    }
    else if (!strcmp(cmd, "objscene2"))
    {
        SetObjectPos(gObj[3], 2497.0, -1652.0, 16.0);
        SetObjectRot(gObj[0], 0.0, 0.0, 15.0);
        StopObject(gObj[4]);
        MoveObject(gObj[1], 2486.0, -1668.0, 14.5, 0.5);
        DestroyObject(gObj[2]);
        gObj[2] = INVALID_OBJECT_ID;
    }
    else if (!strcmp(cmd, "objwhere"))
    {
        new Float:x, Float:y, Float:z, Float:rx, Float:ry, Float:rz;
        for (new i = 0; i < 6; i++)
        {
            if (gObj[i] == INVALID_OBJECT_ID) continue;
            GetObjectPos(gObj[i], x, y, z);
            GetObjectRot(gObj[i], rx, ry, rz);
            printf("[cov] %d objwhere %d pos=%.2f,%.2f,%.2f rot=%.1f,%.1f,%.1f moving=%d", seq, gObj[i], x, y, z, rx, ry, rz, IsObjectMoving(gObj[i]));
        }
    }
    else if (!strcmp(cmd, "menu"))
    {
        if (gMenu == Menu:INVALID_MENU)
        {
            gMenu = CreateMenu("Coverage", 1, 200.0, 150.0, 150.0);
            AddMenuItem(gMenu, 0, "First");
            AddMenuItem(gMenu, 0, "Second");
        }
        ShowMenuForPlayer(gMenu, p);
    }
    else if (!strcmp(cmd, "hidemenu")) HideMenuForPlayer(gMenu, p);
    else if (!strcmp(cmd, "forceclass")) ForceClassSelection(p);
    else if (!strcmp(cmd, "sound")) PlayerPlaySound(p, in(line, 1), 0.0, 0.0, 0.0);
    else if (!strcmp(cmd, "vehc"))
    {
        new v = CreateVehicle(in(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), fl(line, 5), in(line, 6), in(line, 7), -1);
        if (gVehCount < sizeof gVehs) gVehs[gVehCount++] = v;
        printf("[cov] %d veh %d", seq, v);
    }
    else if (!strcmp(cmd, "vin")) PutPlayerInVehicle(p, vs(line, 1), in(line, 2));
    else if (!strcmp(cmd, "vhealth")) SetVehicleHealth(vs(line, 1), fl(line, 2));
    else if (!strcmp(cmd, "vcol")) ChangeVehicleColor(vs(line, 1), in(line, 2), in(line, 3));
    else if (!strcmp(cmd, "vpaint")) ChangeVehiclePaintjob(vs(line, 1), in(line, 2));
    else if (!strcmp(cmd, "vmod")) AddVehicleComponent(vs(line, 1), in(line, 2));
    else if (!strcmp(cmd, "vunmod")) RemoveVehicleComponent(vs(line, 1), in(line, 2));
    else if (!strcmp(cmd, "vdmg"))
        UpdateVehicleDamageStatus(vs(line, 1), VEHICLE_PANEL_STATUS:in(line, 2), VEHICLE_DOOR_STATUS:in(line, 3), VEHICLE_LIGHT_STATUS:in(line, 4), VEHICLE_TYRE_STATUS:in(line, 5));
    else if (!strcmp(cmd, "vrepair")) RepairVehicle(vs(line, 1));
    else if (!strcmp(cmd, "vparams")) SetVehicleParamsEx(vs(line, 1), in(line, 2), in(line, 3), in(line, 4), in(line, 5), in(line, 6), in(line, 7), in(line, 8));
    else if (!strcmp(cmd, "vdoors")) SetVehicleParamsCarDoors(vs(line, 1), in(line, 2), in(line, 3), in(line, 4), in(line, 5));
    else if (!strcmp(cmd, "vwindows")) SetVehicleParamsCarWindows(vs(line, 1), in(line, 2), in(line, 3), in(line, 4), in(line, 5));
    else if (!strcmp(cmd, "vpos")) SetVehiclePos(vs(line, 1), fl(line, 2), fl(line, 3), fl(line, 4));
    else if (!strcmp(cmd, "vangle")) SetVehicleZAngle(vs(line, 1), fl(line, 2));
    else if (!strcmp(cmd, "vvel")) SetVehicleVelocity(vs(line, 1), fl(line, 2), fl(line, 3), fl(line, 4));
    else if (!strcmp(cmd, "vlink")) LinkVehicleToInterior(vs(line, 1), in(line, 2));
    else if (!strcmp(cmd, "vplate")) { field(line, 2, a); SetVehicleNumberPlate(vs(line, 1), a); }
    else if (!strcmp(cmd, "trailer")) AttachTrailerToVehicle(vs(line, 1), vs(line, 2));
    else if (!strcmp(cmd, "untrailer")) DetachTrailerFromVehicle(vs(line, 1));
    else if (!strcmp(cmd, "campath"))
        InterpolateCameraPos(p, fl(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), fl(line, 5), fl(line, 6), in(line, 7), CAM_MOVE:in(line, 8));
    else if (!strcmp(cmd, "camlookpath"))
        InterpolateCameraLookAt(p, fl(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), fl(line, 5), fl(line, 6), in(line, 7), CAM_MOVE:in(line, 8));
    else if (!strcmp(cmd, "widescreen")) TogglePlayerWidescreen(p, in(line, 1) != 0);
    else if (!strcmp(cmd, "pgravity")) SetPlayerGravity(p, fl(line, 1));
    else if (!strcmp(cmd, "shop")) { field(line, 1, a); SetPlayerShopName(p, a); }
    else if (!strcmp(cmd, "skill")) SetPlayerSkillLevel(p, WEAPONSKILL:in(line, 1), in(line, 2));
    else if (!strcmp(cmd, "fstyle")) SetPlayerFightingStyle(p, FIGHT_STYLE:in(line, 1));
    else if (!strcmp(cmd, "vel")) SetPlayerVelocity(p, fl(line, 1), fl(line, 2), fl(line, 3));
    else if (!strcmp(cmd, "findz")) SetPlayerPosFindZ(p, fl(line, 1), fl(line, 2), fl(line, 3));
    else if (!strcmp(cmd, "gravity")) SetGravity(fl(line, 1));
    else if (!strcmp(cmd, "control")) TogglePlayerControllable(p, in(line, 1) != 0);
    else if (!strcmp(cmd, "spawnp")) SpawnPlayer(p);
    else if (!strcmp(cmd, "freezeonspawn")) gFreezeOnSpawn = in(line, 1) != 0;
    else if (!strcmp(cmd, "rcon")) { field(line, 1, a); SendRconCommand(a); }
    // Server-side NPCs (open.mp NPC component): remote players for the client.
    else if (!strcmp(cmd, "npc"))
    {
        field(line, 1, a);
        new n = NPC_Create(a);
        NPC_Spawn(n);
        NPC_SetPos(n, fl(line, 2), fl(line, 3), fl(line, 4));
        NPC_SetSkin(n, in(line, 5));
        NPC_SetWeapon(n, WEAPON:in(line, 6));
        printf("[cov] %d npc %d", seq, n);
    }
    else if (!strcmp(cmd, "clearvehs")) { gVehCount = 0; for (new v = 1; v <= MAX_VEHICLES; v++) if (GetVehicleModel(v) != 0 && GetVehicleModel(v) != 538) DestroyVehicle(v); }
    else if (!strcmp(cmd, "clearnpcs")) { for (new i = 0; i < MAX_PLAYERS; i++) if (IsPlayerConnected(i) && IsPlayerNPC(i)) NPC_Destroy(i); }
    else if (!strcmp(cmd, "npcmove")) NPC_Move(in(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), NPC_MOVE_TYPE_AUTO);
    else if (!strcmp(cmd, "npcfollow")) NPC_MoveToPlayer(in(line, 1), p, NPC_MOVE_TYPE_WALK);
    else if (!strcmp(cmd, "npcaim")) NPC_AimAtPlayer(in(line, 1), p, in(line, 2) != 0, 400);
    else if (!strcmp(cmd, "npcstop")) { NPC_StopAim(in(line, 1)); NPC_StopMove(in(line, 1)); }
    else if (!strcmp(cmd, "npcmelee")) NPC_MeleeAttack(in(line, 1), 2000);
    else if (!strcmp(cmd, "npcanim")) { field(line, 2, a); field(line, 3, b); NPC_ApplyAnimation(in(line, 1), a, b, 4.1, true, false, false, false, 0); }
    else if (!strcmp(cmd, "npcveh")) NPC_PutInVehicle(in(line, 1), in(line, 2), in(line, 3));
    else if (!strcmp(cmd, "npcenter")) NPC_EnterVehicle(in(line, 1), in(line, 2), in(line, 3), NPC_MOVE_TYPE_WALK);
    else if (!strcmp(cmd, "npckill")) NPC_Kill(in(line, 1), INVALID_PLAYER_ID, REASON_SUICIDE);
    else if (!strcmp(cmd, "npcdel")) NPC_Destroy(in(line, 1));
    else { printf("[cov] %d unknown %s", seq, cmd); return; }
    printf("[cov] %d done %s", seq, cmd);
}

forward CovPoll();
public CovPoll()
{
    if (gLocal != INVALID_PLAYER_ID)
    {
        new KEY:keys, ud, lr;
        GetPlayerKeys(gLocal, keys, ud, lr);
        if (_:keys != gKeys[0] || ud != gKeys[1] || lr != gKeys[2])
        {
            gKeys[0] = _:keys;
            gKeys[1] = ud;
            gKeys[2] = lr;
            printf("[cov] keys id=%d poll keys=%d ud=%d lr=%d", gLocal, _:keys, ud, lr);
        }
    }
    new File:f = fopen("sampcov.txt", io_read);
    if (!f) return 1;
    new line[512];
    while (fread(f, line))
    {
        new n = strlen(line);
        while (n > 0 && (line[n - 1] == '\n' || line[n - 1] == '\r')) line[--n] = 0;
        new sp = strfind(line, " ");
        if (sp <= 0) continue;
        new num[12];
        strmid(num, line, 0, sp);
        new seq = strval(num);
        if (seq <= gLast) continue;
        gLast = seq;
        Run(seq, line[sp + 1]);
    }
    fclose(f);
    return 1;
}
