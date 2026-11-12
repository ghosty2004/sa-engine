// SAMP-1D gamemode: drives the SA:MP coverage scenarios of tools/e2e/samp-cov.sh.
//
// Classes for the class-selection screen, a static train, and a command channel: the driver appends
// lines "SEQ CMD|ARG|..." to scriptfiles/sampcov.txt; every 100 ms the lines with a SEQ above the last
// one run is executed and "[cov] SEQ done CMD" is printed. Every player callback prints a "[cov]" line
// so the driver can wait for server-side observations. "L" is the first (local) player.
//
//   msg|text  gametext|style|ms|text  dialog|id|style|caption|body  td|text  tdhide
//   pos|x|y|z|angle  interior|i  vw|world  hp|value  armour|value  kill  weapon|weapon|ammo  skin|skin
//   veh|model|x|y|z|angle  putin|vehid|seat  eject  vhp|vehid|health  vdel|vehid
//   obj|model|x|y|z  objmove|id|x|y|z|speed  pickup|model|type|x|y|z  label|text|x|y|z
//   cp|x|y|z|size  rcp|type|x|y|z|nx|ny|nz|size  cpoff  gz|minx|miny|maxx|maxy  icon|id|x|y|z|type
//   actor|skin|x|y|z|angle  explode|x|y|z|type|radius  time|h  weather|w  money|n  wanted|n
//   anim|lib|name  clranim  special|action  campos|x|y|z|lx|ly|lz  camback  spec|1/0  drunk|level  attach|model|bone
//   menu  sound|id  gravity|g  control|0/1  rcon|command
//   NPCs (open.mp NPC component; remote players for the client):
//   npc|name|x|y|z|skin|weapon  npcmove|id|x|y|z  npcfollow|id  npcaim|id|shoot  npcstop|id  npcmelee|id
//   npcanim|id|lib|name  npcveh|id|vehid|seat  npcenter|id|vehid|seat  npckill|id  npcdel|id
#include <open.mp>

new gLast = 0;
new gLocal = INVALID_PLAYER_ID;
new Text:gTd = Text:INVALID_TEXT_DRAW;
new Menu:gMenu = Menu:INVALID_MENU;

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
    new File:f = fopen("sampcov.txt", io_write);
    if (f) fclose(f);
    SetTimer("CovPoll", 100, true);
    SetTimer("CovState", 1000, true);
    print("[cov] loaded");
    return 1;
}

public OnPlayerConnect(playerid)
{
    new name[MAX_PLAYER_NAME + 1];
    GetPlayerName(playerid, name, sizeof name);
    if (gLocal == INVALID_PLAYER_ID) gLocal = playerid;
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
    SetPlayerPos(playerid, 2495.0, -1674.0, 13.4);
    SetPlayerFacingAngle(playerid, 180.0);
    SetPlayerCameraPos(playerid, 2495.0, -1680.0, 14.9);
    SetPlayerCameraLookAt(playerid, 2495.0, -1674.0, 13.9);
    printf("[cov] class id=%d class=%d", playerid, classid);
    return 1;
}

public OnPlayerSpawn(playerid) { printf("[cov] spawn id=%d", playerid); return 1; }
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
    else if (!strcmp(cmd, "putin")) PutPlayerInVehicle(p, in(line, 1), in(line, 2));
    else if (!strcmp(cmd, "eject")) RemovePlayerFromVehicle(p);
    else if (!strcmp(cmd, "vhp")) SetVehicleHealth(in(line, 1), fl(line, 2));
    else if (!strcmp(cmd, "vdel")) DestroyVehicle(in(line, 1));
    else if (!strcmp(cmd, "obj")) printf("[cov] %d obj %d", seq, CreateObject(in(line, 1), fl(line, 2), fl(line, 3), fl(line, 4), 0.0, 0.0, 0.0));
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
    else if (!strcmp(cmd, "explode")) CreateExplosion(fl(line, 1), fl(line, 2), fl(line, 3), in(line, 4), fl(line, 5));
    else if (!strcmp(cmd, "time")) SetPlayerTime(p, in(line, 1), 0);
    else if (!strcmp(cmd, "weather")) SetPlayerWeather(p, in(line, 1));
    else if (!strcmp(cmd, "money")) GivePlayerMoney(p, in(line, 1));
    else if (!strcmp(cmd, "wanted")) SetPlayerWantedLevel(p, in(line, 1));
    else if (!strcmp(cmd, "anim")) { field(line, 1, a); field(line, 2, b); ApplyAnimation(p, a, b, 4.1, true, false, false, false, 0, SYNC_ALL); }
    else if (!strcmp(cmd, "clranim")) ClearAnimations(p, SYNC_ALL);
    else if (!strcmp(cmd, "special")) SetPlayerSpecialAction(p, SPECIAL_ACTION:in(line, 1));
    else if (!strcmp(cmd, "campos")) { SetPlayerCameraPos(p, fl(line, 1), fl(line, 2), fl(line, 3)); SetPlayerCameraLookAt(p, fl(line, 4), fl(line, 5), fl(line, 6)); }
    else if (!strcmp(cmd, "camback")) SetCameraBehindPlayer(p);
    else if (!strcmp(cmd, "spec")) TogglePlayerSpectating(p, in(line, 1) != 0);
    else if (!strcmp(cmd, "drunk")) SetPlayerDrunkLevel(p, in(line, 1));
    else if (!strcmp(cmd, "attach")) SetPlayerAttachedObject(p, 0, in(line, 1), in(line, 2), 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0, 0);
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
    else if (!strcmp(cmd, "sound")) PlayerPlaySound(p, in(line, 1), 0.0, 0.0, 0.0);
    else if (!strcmp(cmd, "gravity")) SetGravity(fl(line, 1));
    else if (!strcmp(cmd, "control")) TogglePlayerControllable(p, in(line, 1) != 0);
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
