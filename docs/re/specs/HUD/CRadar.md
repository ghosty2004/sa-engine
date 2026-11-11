# CRadar (HUD) - RE-SPEC-REST group B

## 0x585040 CRadar::ClipRadarPoly  (catalogue: specified/REA, port level 1)
- `__cdecl int32 (CVector2D* out [ebp+8], const CVector2D* in [ebp+0xC])` (in = 4 vertices in radar space), plain `ret`; EBP frame,
  ESP aligned to 8. Returns the number of vertices written to `out`. Callers: DrawAreaOnRadar 0x5853D0, DrawRadarSection 0x586110.
  Callee: CRadar::LineRadarBoxCollision 0x584E00 (cdecl `(CVector2D* out, const CVector2D* a, const CVector2D* b)` -> edge index or -1,
  caller pops 0xC).
- Locals: `n = 0` (count), `lastEdge = -1`, corner table `C[4]` = {(1,-1), (1,1), (-1,1), (-1,-1)} built on the stack (0x585067-0x5850A2),
  `inside[4]` bytes.
- 1. inside[i] for i = 0..3 (0x58504D-0x5851C6): `!(in[i].x < -1.0f) && !(in[i].x > 1.0f) && !(in[i].y < -1.0f) && !(in[i].y > 1.0f)`
  with [0x858C1C]=-1.0f and [0x858624]=1.0f; tests are x vs -1 (`test ah,5; jnp` = outside only when strictly less), x vs +1
  (`test ah,0x41; je` = outside only when strictly greater), then y likewise. NaN coordinates count as inside.
- 2. Loop i = 0..3 (cursor `cur = &in[i]`):
  - if inside[i]: `out[n++] = in[i]`; next i.
  - else: `e1 = LRBC(&out[n], cur, &in[(i-1)&3])`; if e1 != -1 { `lastEdge = e1`; n++ }.
    `p = &out[n]`; `e2 = LRBC(p, cur, &in[(i+1)&3])`. If e2 == -1: next i.
    If e1 != -1: n++ (keeps the exit point written at p); next i.
    Else (e1 == -1): if lastEdge == -1, search j = 3 down to i (`for (j = 3; j >= i; j--)`): `r = LRBC(&scratch, &in[j], j == 0 ?
    &in[3] : &in[j-1])` (scratch = an 8-byte dead stack slot, the slot that held the loop index; the index is reloaded afterwards);
    first r != -1 -> `lastEdge = r`, stop. Then (0x5852B5) save `q = *p` (the exit point, x87 loads x then y), and while
    `lastEdge != e2`: `out[n++] = C[lastEdge]` written from p onwards; `lastEdge = (lastEdge + 1) & 3`. Then `out[n] = q`; n++.
    Note: lastEdge is NOT updated to e2 after this walk (it keeps the last value of the walk = e2, because the loop ends at e2).
- 3. If n != 0 return n.
- 4. n == 0 (0x585333): `s1 = (in[0].y - in[1].y) / (in[0].x - in[1].x)`; `s2 = (in[0].y - in[3].y) / (in[0].x - in[3].x)` (stored f32).
  `A = (s1*in[3].x - in[3].y) * (s1*in[0].x - in[0].y)` (x87: s1*x3 - y3 first, then s1*x0 - y0, fmulp); if `!(A < 0.0f)` [0x858B50]
  (`test ah,5; jp`: >=, == and NaN) return 0. `B = (s2*in[1].x - in[1].y) * (s2*in[0].x - in[0].y)`; if `!(B < 0.0f)` return 0.
  Else write the whole box `out[0..3] = (1,-1), (1,1), (-1,1), (-1,-1)` and return 4.
- Edge cases: if the search in step 2 finds no edge (lastEdge stays -1), the corner walk reads `C[-1]` = the 8 bytes just below the
  corner table (two stack words: the `&inside[-1]` pointer and `p`) as floats before wrapping to C[0] (original bug, keep). Division by
  zero in step 4 follows x87 rules (inf/NaN, masked).
- gta-reversed: not reversed (hook `.Reversed = false`).
- Open questions: none.

## 0x587000 CRadar::DrawEntityBlip  (catalogue: specified/REA, port level 5)
- `__cdecl void (int32 blipIndex [esp+4], uint8 isSprite [esp+8])`, plain `ret`, `sub esp,0x3C` + 4 pushes. Only caller: CRadar::DrawBlips
  0x588050. T = `ms_RadarTrace[blipIndex]` (0xBA86F0 + 0x28*blipIndex; EDI = 0x28*index). Two obfuscated tail stubs (0x405B03: loads
  `T.m_nColour` into EDX and jumps to 0x587143; 0x401418: loads the byte at 0xBA67A1 into AL and jumps to 0x587B89) are plain code.
- tRadarTrace fields (gr Radar.h, size 0x28): +0 colour, +4 entity handle, +8 position (CVector), +0x18 sphere radius (f32), +0x1C blip
  size (i16), +0x20 CEntryExit*, +0x24 sprite (u8), +0x25 bits (0x1 bright, 0x4 shortRange, 0x8 friendly, 0x10 blipRemain), +0x26 bits
  (0..1 display flag, 2..5 blip type).
- Globals: 0xBA67A1 `MAP` (FrontEndMenuManager byte, gr m_bDrawingMap), 0xBA678F `SHOWMB` (FrontEndMenuManager byte, gr m_ShowMissionBlips),
  CTheScripts::RadarZoomValue 0xA444A3 (u8), RadarShowBlipOnAllLevels 0xA444A2 (u8), DbgFlag 0x859CF8 (u8), CRadar::m_fRadarOrientation
  0xBA8310, m_radarRange 0xBA8314, airstrip_location 0xBA8300 (u8), airstrip_table 0x8D06E0 (16-byte entries: +0 x, +4 y, +8 heading in
  degrees, +0xC), RsGlobal.maximumWidth 0xC17044 / maximumHeight 0xC17048 (i32), CTimer::m_snTimeInMillisecondsPauseMode 0xB7CB7C, the
  airstrip-arrow statics (catalogue calls them g_RadarGangResetOverlay+N): `AS_DONE` u8 0xBAA370, `AS_TIME` u32 0xBAA374, `AS_A` i16 0xBAA378,
  `AS_B` i16 0xBAA37C, `AS_INIT` u32 bit flags 0xBAA380, TheCamera active cam index 0xB6F081 / cam mode i16 at 0xB6F1A8 + 0x238*index.
- Sizes used for sprites: `W8 = _ftol(maxWidth*[0x859520]=0.0015625f*[0x859000]=8.0f)`, `H8 = _ftol(maxHeight*[0x859524]=0.0022321429f*8.0f)`;
  W16/H16 the same with [0x8599D0]=16.0f (fild of the int, fmul, fmul, _ftol 0x821B40 truncation). Colour bytes of a u32 `col`: r = col>>24,
  g = (col>>16)&0xFF, b = (col>>8)&0xFF, a = col&0xFF. CRGBA ctor 0x7170C0(r, g, b, a). White = CRGBA(255,255,255,255).
- Locals: `ent` (esp+0x14, initially 0), `sl` (esp+0x1C, searchlight, initially 0), `isAS` (BL, initially 0), `pos` (esp+0x40, CVector).
- 1. Type `t = (T.+0x26 >> 2) & 0xF`; `t-1 > 7` unsigned -> return. Jump table 0x587BEC:
  - 1 CAR: `ent = VehiclePool->GetAt(handle)` 0x4048E0 (ECX = [0xB74494]); POSENT.
  - 2 CHAR: `p = PedPool->GetAtRef(handle)` 0x404910; p == 0 -> return; `ent = (p->+0x46C & 0x100) ? p->+0x58C : p`; POSENT.
  - 3 OBJECT: `ent = ObjectPool->GetAt(handle)` 0x465040 (ECX = [0xB7449C]); POSENT.
  - 4, 5 (COORD, CONTACT_POINT): return.
  - 6 SPOTLIGHT: `i = CTheScripts::GetActualScriptThingIndex(handle, 2)` 0x4839A0; i < 0 -> return; `sl = 0xA94D68 + 0x7C*i`;
    `pos = sl->+0x14` (3 floats); COMMON.
  - 7 PICKUP: `i = CPickups::GetActualPickupIndex(handle)` 0x4552A0; i < 0 -> return; `pos = CPickup::GetPosn(&aPickUps[i])` 0x4549A0
    (0x9788C0 + 0x20*i, result via out pointer); COMMON.
  - 8 AIRSTRIP: see AIRSTRIP below, then COMMON with isAS = 1.
  - POSENT (0x58705C): ent == 0 -> return; `pos = ent->m_matrix (+0x14) ? matrix.pos (+0x30) : ent->+4` ; if `(ent->+0x36 & 7) == 3` (ped)
    and `ent->+0x78C != 0`: `((CEntryExit*)ent->+0x78C)->GetPositionRelativeToOutsideWorld(&pos)` 0x43EA00. COMMON.
- 2. COMMON (0x587143): `col = GetRadarTraceColour(T.colour, T.+0x25 & 1, (T.+0x25 >> 3) & 1)` 0x584770. `d = T.+0x26 & 3`.
  If (d == 3 or d == 1) and DbgFlag: `ShowRadarMarker(pos (by value), col, T.+0x18)` 0x584480; `T.+0x18 = T.+0x18 - [0x858B1C]=0.1f` (stored
  f32); if the new value `< 1.0f` (`test ah,5; jp` skips >= and NaN) `T.+0x18 = 5.0f`.
  If `T.+0x20 != 0`: `T.+0x20->GetPositionRelativeToOutsideWorld(&pos)`. If d != 3 and d != 2 -> return.
  `R = TransformRealWorldPointToRadarSpace((pos.x, pos.y))` 0x583530 (out esp+0x28); `L = LimitRadarPoint(&R)` 0x5832F0 (ST0, stored
  f32; R is modified in place by the callee); `S = TransformRadarPointToScreenSpace(R)` 0x583480 (out esp+0x34).
  If `T.+0x25 & 4` (short range) and `L > 1.0f` (strictly; NaN does not hide) and !MAP -> return.
- 3. ZOOM: if RadarZoomValue != 0 and `CWorld::FindPlayerVehicle(-1, 0)` 0x56E0D0 == 0 and t != 3 and t != 7 (0x587264-0x58729A):
  - if isSprite -> return. If `!(T.+0x25 & 0x10)` and `!(sqrt(R.y*R.y + R.x*R.x) <= 1.0f)` (fsqrt; `test ah,0x41; jp`: > and NaN) -> return.
  - `h = ent ? CPlaceable::GetHeading(ent) 0x441DB0 : 0.0f` (stored over the blipIndex argument slot - blipIndex is not used again on
    this path). If MAP and !SHOWMB -> return.
  - `c = CWorld::FindPlayerCentreOfWorldForMap(0)` 0x56E400 (written over R). If `pos.z - 2.0f [0x858CA0] > c.z` (strict; NaN -> no):
    if !RadarShowBlipOnAllLevels return; `ShowRadarTraceWithHeight(S.x, S.y, (i16)T.+0x1C, r, g, b, a, 0)` 0x584070; return.
  - Else if `pos.z + 4.0f [0x858B90] < c.z` (`test ah,5; jp` -> >= and NaN go on): if !RadarShowBlipOnAllLevels return; same call with
    height 1; return.
  - Else (0x58767E) rotating arrow: colour CRGBA(r, g, b, 255); if cam mode (i16 at 0xB6F1A8 + 0x238*[0xB6F081]) == 1:
    `angle = h + [0x858CB8]=3.1415927f`; else `angle = h - (m_fRadarOrientation + 3.1415927f)`; `DrawRotatingRadarSprite(RadarBlipSprites
    + 8 (0xBAA258), S.x, S.y, angle, W8, H8, colour)` 0x584850 (CRGBA passed by value); return.
- 4. NOT ZOOM (0x587790): `sp = T.+0x24`.
  - isSprite: if sp != 0 and `HasThisBlipBeenRevealed(blipIndex)` 0x583AF0: `DrawRadarSprite((i16)(i8)sp, S.x, S.y, 255)` 0x585FF0. Return.
  - sp != 0 -> return. If MAP and !SHOWMB -> return.
  - SPOTLIGHT (sl != 0): if `!(sqrt(R.y*R.y + R.x*R.x) <= 0.9f [0x858C20])` (> or NaN) -> LEGEND; if MAP -> LEGEND. Else
    `r1 = maxWidth * 0.0015625f * [0x866B78]=94.0f / m_radarRange * sl->+0x20 * [0x858CC8]=0.6f` (that exact left-to-right x87 order,
    stored f32); `DrawCircleAtNearClip(&S, r1, &CRGBA(0,0,0,0x96), 15)` 0x727D60; `r2 = r1 - maxWidth * 0.0015625f`;
    `DrawCircleAtNearClip(&S, r2, &CRGBA(0xDC,0xDC,0xDC,0xC8), 15)`; LEGEND.
  - AIRSTRIP (isAS): if !MAP and `sqrt(R.y*R.y + R.x*R.x) < 0.9f` (`test ah,5; jp`) and `asDist < 500.0f` [0x858B58]:
    `DrawRotatingRadarSprite(RadarBlipSprites + 0xE0 (0xBAA330), S.x, S.y, 0.0f, W8, H8, white)`. Then recompute from the trace position:
    `pos = T.+8`; R = radar(pos.xy); LimitRadarPoint(&R) (result discarded); S = screen(R) (0x587959-0x5879A0).
    If MAP: `a = airstrip_table[loc].+8`; if loc != 1 `a -= 90.0f [0x85991C]`; `DrawRotatingRadarSprite(0xBAA334, S.x, S.y,
    a * [0x8595EC]=0.017453292f, W16, H16, white)`; `AddBlipToLegendList(0, 0x39)` 0x5859F0; LEGEND.
    Else: if AS_DONE != 0 return; `DrawRotatingRadarSprite(0xBAA334, S.x, S.y, -m_fRadarOrientation - (heading_deg - 90.0f) * 0.017453292f,
    W16, H16, white)` (x87: fld orient, fchs, then (heading - 90)*k, fsubp); LEGEND.
  - PLAIN (0x587B02): `c = FindPlayerCentreOfWorldForMap(0)`; `hgt = (pos.z - 2.0f > c.z) ? 0 : ((pos.z + 4.0f < c.z) ? 1 : 2)` (the
    first compare is `test ah,0x41; jne` so NaN gives not-greater; the second `test ah,5; jnp` so NaN gives 2). The height argument is
    pushed as the dword at esp+0x14 whose low byte was set (upper bytes = upper bytes of `ent`); the callee's parameter is u8.
    `ShowRadarTraceWithHeight(S.x, S.y, (i16)T.+0x1C, r, g, b, a, hgt)`; LEGEND.
  - LEGEND (0x587B84 -> 0x401418 -> 0x587B89): if MAP and sl == 0 and !isAS: `AddBlipToLegendList(1, blipIndex)`. Return.
- AIRSTRIP (0x58737B, t == 8; isAS = 1): `V = FindPlayerVehicle(-1, 0)` called 4 times (no null check: crashes if the player has no
  vehicle); `P = V->m_matrix ? &matrix.pos : &V->+4`. `A = airstrip_table[loc]`. `asDist = sqrt((A.y-P.y)*(A.y-P.y) + (A.x-P.x)*(A.x-P.x))`
  (y term first, stored f32 at esp+0x18).
  - If `!(asDist < 500.0f)` (>= or NaN): `AS_DONE = 0`; COMMON (note: `pos` is NOT written on this path, so COMMON's radar point R and
    L come from uninitialised stack; they only feed the short-range / zoom / 0.9 tests; the drawing in step 4 recomputes from T.+8).
  - Else: `b = AS_B`: if !(AS_INIT & 1) { AS_INIT |= 1; `b = AS_B = (i16)_ftol(-A.+0xC)` }. If !(AS_INIT & 2) { AS_INIT |= 2;
    `AS_A = (i16)_ftol(-A.+0xC)` }. `pos = T.+8`; `R0 = radar((pos.x, pos.y))`. `rad = A.+8 * 0.017453292f`; point
    `Q = (pos.x + cos(rad)*b, pos.y - sin(rad)*b)` (fild of (i32)b; fcos then fmul; fsin then fmul; x stored as f32 into the local and
    pos.x, y likewise; exact sequence 0x587507-0x58754E); `R = radar(Q)`. `m = sqrt(R.y*R.y + R.x*R.x)` kept in ST0.
    If (`m < 0.9f` or NaN) (fcom, `test ah,1; je`) and `(u32)(now - AS_TIME) <= 3` (now = [0xB7CB7C]): pop and skip the update. Else:
    `b += 100`; AS_B = b; if `(f32)(i32)b > A.+0xC` (strict; `test ah,0x41; jne`) `b = AS_B = (i16)_ftol(-A.+0xC)`;
    if AS_A == b `AS_DONE = 0`; if `m < 0.9f` (`test ah,5; jp`) { AS_A = b; AS_DONE = 1 }; `AS_TIME = now`.
    Then `LimitRadarPoint(&R)` (result popped); COMMON with isAS = 1 and the moved `pos` (z = T.z).
- gta-reversed: plugin::Call stub only (Radar.cpp:1729). No gr reading aid exists for the airstrip animation.
- Open questions: the upper 3 bytes of the height dword at 0x587B54 are garbage; this
  only matters if 0x584070 reads more than the low byte (not checked).
