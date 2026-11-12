// SAMP-UI reference gamemode: deterministic SA:MP UI content for screenshots of the original client
// (tools/e2e/samp-ui-ref.sh). The same content is shown by `sa-game --samp-ui-demo --samp-ui-scene ref`
// (crates/sa-game/src/samp_ui_scene.rs); keep both in step.
//
// Command channel as in sampcov.pwn: the driver appends "SEQ CMD|ARG" lines to scriptfiles/sampui.txt,
// every 100 ms new lines run and "[ui] SEQ done CMD" is printed.
//
//   scene              chat lines, NPC with nametag + bars, 3D label, textdraws, game text, kill list
//   dialog|style       one dialog of each style 0..5 with fixed text (style 2 / 4 / 5 bodies below)
//   hide               hides the dialog
#include <open.mp>

new gLast = 0;
new gLocal = INVALID_PLAYER_ID;
new gNpc = INVALID_PLAYER_ID;
new gNpc2 = INVALID_PLAYER_ID;

main() {}

public OnGameModeInit()
{
    SetGameModeText("sampui");
    AddPlayerClass(0, 2495.0, -1687.0, 13.5, 180.0, WEAPON_FIST, 0, WEAPON_FIST, 0, WEAPON_FIST, 0);
    ShowPlayerMarkers(PLAYER_MARKERS_MODE_OFF);
    new File:f = fopen("sampui.txt", io_write);
    if (f) fclose(f);
    SetTimer("UiPoll", 100, true);
    print("[ui] loaded");
    return 1;
}

public OnPlayerConnect(playerid)
{
    new name[MAX_PLAYER_NAME + 1];
    GetPlayerName(playerid, name, sizeof name);
    if (gLocal == INVALID_PLAYER_ID && !IsPlayerNPC(playerid)) gLocal = playerid;
    printf("[ui] connect id=%d name=%s", playerid, name);
    return 1;
}

public OnPlayerRequestClass(playerid, classid)
{
    SetPlayerPos(playerid, 2495.0, -1687.0, 13.5);
    SetPlayerFacingAngle(playerid, 180.0);
    SetPlayerCameraPos(playerid, 2495.0, -1693.0, 15.0);
    SetPlayerCameraLookAt(playerid, 2495.0, -1687.0, 13.5);
    printf("[ui] class id=%d", playerid);
    return 1;
}

public OnPlayerSpawn(playerid) { printf("[ui] spawn id=%d", playerid); return 1; }
public OnDialogResponse(playerid, dialogid, response, listitem, inputtext[])
{
    printf("[ui] dialog-response dialog=%d response=%d item=%d text='%s'", dialogid, response, listitem, inputtext);
    return 1;
}

Td(Float:x, Float:y, const text[], font, Float:lx, Float:ly, colour, outline, shadow)
{
    new Text:t = TextDrawCreate(x, y, "%s", text);
    TextDrawFont(t, TEXT_DRAW_FONT:font);
    TextDrawLetterSize(t, lx, ly);
    TextDrawColour(t, colour);
    TextDrawSetOutline(t, outline);
    TextDrawSetShadow(t, shadow);
    TextDrawSetProportional(t, true);
    TextDrawBackgroundColour(t, 0x000000FF);
    return _:t;
}

Scene(p)
{
    SetPlayerColor(p, 0xFF8000FF);
    SetPlayerHealth(p, 100.0);
    // Chat: server messages with colour codes, a light-blue info line, a long line, player chat.
    SendClientMessage(p, 0xFFFFFFFF, "Welcome to the {FFFF00}SA-MP UI{FFFFFF} reference server");
    SendClientMessage(p, 0xA9C4E4FF, "Server: this line is light blue");
    SendClientMessage(p, 0xFF6347FF, "{FF0000}Red {00FF00}green {0000FF}blue {FFFFFF}white {FF6347}and back");
    SendClientMessage(p, 0x88AA62FF, "A long line: the quick brown fox jumps over the lazy dog, again and again, until it runs off");
    SendClientMessage(p, 0xFFFF00FF, "Yellow 0123456789 !\"#$%%&'()*+,-./:;<=>?@[\\]^_`|~");
    // A remote player (open.mp NPC) 4 m in front of the local player, with health and armour.
    gNpc = NPC_Create("Big_Smoke");
    NPC_Spawn(gNpc);
    NPC_SetPos(gNpc, 2495.0, -1691.0, 13.5);
    NPC_SetFacingAngle(gNpc, 0.0);
    NPC_SetSkin(gNpc, 269);
    NPC_SetHealth(gNpc, 70.0);
    NPC_SetArmour(gNpc, 50.0);
    SetPlayerColor(gNpc, 0x2B8F2BFF);
    gNpc2 = NPC_Create("Ryder");
    NPC_Spawn(gNpc2);
    NPC_SetPos(gNpc2, 2490.0, -1695.0, 13.5);
    NPC_SetSkin(gNpc2, 271);
    SetPlayerColor(gNpc2, 0x9966CCFF);
    SendPlayerMessageToPlayer(p, gNpc, "hello from the remote player");
    // 3D label.
    Create3DTextLabel("{FFFF00}Grove Street\n{FFFFFF}Home of the {00FF00}Grove Street Families", 0xFFFFFFFF,
        2497.0, -1692.0, 14.5, 40.0, 0, false);
    // Textdraws: each font, outline / shadow, a box, alignments, sprites.
    TextDrawShowForPlayer(p, Text:Td(40.0, 140.0, "Font 0 ~r~red~w~ white", 0, 0.5, 1.5, 0xFFFFFFFF, 1, 0));
    TextDrawShowForPlayer(p, Text:Td(40.0, 170.0, "Font 1 shadow 2", 1, 0.4, 1.4, 0xFFFFFFFF, 0, 2));
    TextDrawShowForPlayer(p, Text:Td(40.0, 200.0, "FONT 2 OUTLINE", 2, 0.4, 1.6, 0xFFFF00FF, 1, 0));
    TextDrawShowForPlayer(p, Text:Td(40.0, 230.0, "Font 3 Pricedown", 3, 0.6, 2.0, 0x2B8F2BFF, 1, 0));
    new Text:b = Text:Td(320.0, 380.0, "Boxed centre text", 1, 0.35, 1.2, 0xFFFFFFFF, 0, 0);
    TextDrawAlignment(b, TEXT_DRAW_ALIGN_CENTER);
    TextDrawUseBox(b, true);
    TextDrawBoxColour(b, 0x00000088);
    TextDrawTextSize(b, 0.0, 200.0);
    TextDrawShowForPlayer(p, b);
    new Text:r = Text:Td(600.0, 330.0, "Right aligned", 1, 0.35, 1.2, 0xA9C4E4FF, 1, 0);
    TextDrawAlignment(r, TEXT_DRAW_ALIGN_RIGHT);
    TextDrawShowForPlayer(p, r);
    new Text:s = TextDrawCreate(500.0, 140.0, "LD_SPAC:white");
    TextDrawFont(s, TEXT_DRAW_FONT_SPRITE_DRAW);
    TextDrawColour(s, 0xFF000099);
    TextDrawTextSize(s, 40.0, 40.0);
    TextDrawShowForPlayer(p, s);
    new Text:s2 = TextDrawCreate(560.0, 140.0, "LD_BEAT:chit");
    TextDrawFont(s2, TEXT_DRAW_FONT_SPRITE_DRAW);
    TextDrawColour(s2, 0xFFFFFFFF);
    TextDrawTextSize(s2, 32.0, 32.0);
    TextDrawShowForPlayer(p, s2);
    // Game text and the kill list.
    GameTextForPlayer(p, "~r~reference ~w~game text", 600000, 3);
    SendDeathMessageToPlayer(p, gNpc, p, 24);
    SendDeathMessageToPlayer(p, p, gNpc2, 31);
    SendDeathMessageToPlayer(p, INVALID_PLAYER_ID, gNpc, 200);
    SendDeathMessageToPlayer(p, gNpc2, gNpc, 0);
}

ShowStyle(p, style)
{
    switch (style)
    {
        case 0: ShowPlayerDialog(p, 10, DIALOG_STYLE_MSGBOX, "Message box",
            "{FFFFFF}Welcome to the server!\nThis is a {FF0000}message box{A9C4E4} with two lines.", "OK", "Cancel");
        case 1: ShowPlayerDialog(p, 11, DIALOG_STYLE_INPUT, "Input", "Enter your name:", "Send", "Cancel");
        case 2: ShowPlayerDialog(p, 12, DIALOG_STYLE_LIST, "Weapons",
            "Desert Eagle\nM4\nMP5\nShotgun\n{FF0000}Sniper Rifle", "Select", "Close");
        case 3: ShowPlayerDialog(p, 13, DIALOG_STYLE_PASSWORD, "Login", "Enter your password:", "Login", "Quit");
        case 4: ShowPlayerDialog(p, 14, DIALOG_STYLE_TABLIST, "Shop",
            "Deagle\t$5000\t100\nSawnoff\t$5000\t100\nPistol\t$1000\t50", "Buy", "Close");
        case 5: ShowPlayerDialog(p, 15, DIALOG_STYLE_TABLIST_HEADERS, "Shop",
            "Weapon\tPrice\tAmmo\nDeagle\t$5000\t100\nSawnoff\t$5000\t100\nPistol\t$1000\t50", "Buy", "Close");
    }
}

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

Run(seq, const line[])
{
    new cmd[24], a[32];
    new p = gLocal;
    field(line, 0, cmd);
    field(line, 1, a);
    if (!strcmp(cmd, "scene")) Scene(p);
    else if (!strcmp(cmd, "dialog")) ShowStyle(p, strval(a));
    else if (!strcmp(cmd, "hide")) ShowPlayerDialog(p, -1, DIALOG_STYLE_MSGBOX, "", "", "", "");
    else { printf("[ui] %d unknown %s", seq, cmd); return; }
    printf("[ui] %d done %s", seq, cmd);
}

forward UiPoll();
public UiPoll()
{
    new File:f = fopen("sampui.txt", io_read);
    if (!f) return 1;
    new line[256];
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
