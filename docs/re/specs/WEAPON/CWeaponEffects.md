# CWeaponEffects (WEAPON) - full specs

## 0x742CF0 CWeaponEffects::Render  (catalogue: specified/REA, port level 4)
- Convention: `__cdecl void ()`, plain `ret`. Locals 0x58 (`S` = ESP after the 4 register pushes; offsets below are S+n).
- Callers: Game::RenderEffects 0x53E170.
- Callees: `RwRenderStateSet` = `[[RwEngineInstance 0xC97B24]+0x20](state, value)` (cdecl; listed as (state,value));
  CSprite::CalcScreenCoors 0x70CE30 `(pos, &out, &w, &h, bool, bool) -> AL`; CSprite::RenderOneXLUSprite_Rotate_Aspect 0x70D490
  `(x, y, z, w, h, r, g, b, int16 intensity, float rz, float rotation, uint8 alpha)` (dwords); CSprite::RenderOneXLUSprite_Triangle
  0x70D320 `(x1, y1, x2, y2, x3, y3, z, r, g, b, int16 intensity, float recipZ, uint8 alpha)`; FUN_404330 = `min(a,b)`
  (`a < b ? a : b`, NaN -> b, ST0) and FUN_420800 = `max(a,b)` (`a > b ? a : b`), both cdecl f32 args;
  CPlayerPed::GetWeaponRadiusOnScreen 0x609CD0 (thiscall -> ST0); CGameLogic::IsCoopGameGoingOn 0x441390 (AL);
  CPed::GetWeaponSkill 0x5E6580 (thiscall); CWeaponInfo::GetWeaponInfo 0x743C60 (cdecl(type, skill));
  CWeapon::FindNearestTargetEntityWithScreenCoors 0x73E240; 0x56EF90 (crosshair trail helper, see `_unattributed-G.md`).
- Data: `gCrossHair` 0xC8A838 (2 x CWeaponEffects, 0x2C): +0x00 m_bActive u8, +0x04 m_nTimeWhenToDeactivate u32 (-1 = permanent),
  +0x08 pos CVector, +0x14 r,g,b,a u8, +0x18 size f32, +0x1C spin angle f32 (gr field_1C), +0x20 lock-on offset f32 (gr field_20),
  +0x24 locked-on f32 0/1 (gr m_fRotation), +0x28 u8 mode (gr m_bClearImmediately; value 1 = "lock-on flight" drawing).
  Per-crosshair statics: `pulse[2]` f32 at 0xC8A8A0 (ebx), `dir[2]` u8 at 0x8D6144 (.data), `gLastTarget` 0xC8A894 (CEntity*),
  `gLastTargetTime` 0xC8A890 (u32), `gLockOffX` 0xC8A898 / `gLockOffY` 0xC8A89C (f32). `.data` f32 constants (never written by a
  catalogued function): 0x8D6140 = 15.0f, 0x8D6148 = 30.0f. Textures: gpCrossHairTexFlight[0..1] 0xC8A810/0xC8A814,
  gpCrossHairTex 0xC8A818 (raster = dword at texture+0). `CWorld::Players` 0xB7CD98 (stride 0x190; +0 m_pPed),
  RsGlobal.maximumWidth/Height 0xC17044/0xC17048 (i32 W, H), CTimer::m_snTimeInMilliseconds 0xB7CB84 (`now`).
- Constants (.rdata f32): 0x858BA4 20.0, 0x858624 1.0, 0x859AD8 28.0, 0x859BB4 1.8, 0x865034 0.0061359233 (2pi/1024),
  0x858B50 0.0, 0x858C98 0.8, 0x858C80 5.0, 0x858FE8 25.0, 0x858FA0 2.5, 0x858BB0 120.0, 0x8595EC 0.017453292, 0x858B48 15.0,
  0x859040 0.33333334, 0x858F34 0.75, 0x858CA0 2.0, 0x858C20 0.9, 0x858C28 0.05, 0x858CBC 6.2831855, 0x858B8C 0.5,
  0x858B1C 0.1f, **0x86EEE0 0.1 (f64)**, 0x863A44 0.001953125, 0x858B3C 3.0, 0x858C54 4294967296.0 (dead unsigned fix-ups).
  `ang(t)` below = `(float)(t & 0x3FF) * 0.0061359233f` (fild, fmul; non-negative so the fix-up never runs).
- Behaviour:
  0. If `*(u8*)0xB6F065` (TheCamera+0x3D, a camera bool flag) != 0: return immediately (nothing below runs, not even step 3).
  1. For i = 0, 1 (`c = &gCrossHair[i]`, `P = &CWorld::Players[i]`):
     a. `T = c->time`; if `T != 0 && T < now` (unsigned): `active = 0; time = 0; pulse[i] = 0`. Then if `c->time != -1`: `pulse[i] = 0`.
        If `P->m_pPed == 0`: `active = 0`. If `!active` continue.
     b. **Mode 1** (`+0x28 == 1`, 0x742D61-0x743144):
        - `if (!CalcScreenCoors(c->pos, &o, &w, &h, true, true))` goto (b-end). `k = 20.0f / h`; if `k > 1.0f`: `w = w*k; h = k*h` (stored).
        - RS (8,0) (6,0) (0xC,1) (0xA,5) (0xB,6) (1, TexFlight[0] raster). `mh = (20.0f < h) ? 20.0f : h`, `mw = (28.0f < w) ? 28.0f : w`
          (recomputed before every sprite below).
        - for j = 0,1 with `s = {0.95f, 1.05f}[j]`: sprite at (o.x, o.y, o.z), size (`mw*s*1.8f`, `mh*s*1.8f`), rgb (0,0,0),
          intensity 255, rz 0.01f, rotation 0, alpha 255.
        - sprite at o, size (`mw*1.8f`, `mh*1.8f`), rgb = c->r,g,b, intensity 255, rz 0.01f, rotation 0, alpha = c->a.
        - RS (1, TexFlight[1] raster). `a = ang(now)` (stored f32). `f = c->+0x20 - 20.0f*a` (stored); if `f < 0.0f` `f = 0`.
          `c->+0x24 = (f == 0.0f) ? 1.0f : 0.0f` (NaN -> 0). `gLockOffY = sin(a) * f`, `gLockOffX = cos(a) * f` (x87 fsin/fcos, stored f32).
        - for j = 0,1: sprite at (`o.x - gLockOffY`, `o.y - gLockOffX`, o.z), size (`mw*s*0.8f`, `mh*s*0.8f`), rgb 0, intensity 255,
          rz 0.01f, rotation = a, alpha 255. Then the same position, size (`mw*0.8f`, `mh*0.8f`), rgb = c->r,g,b, alpha c->a, rotation a.
        - (b-end) RS (8,1) (6,1); continue with the next i (the 0xC reset of step 1e is NOT done in this mode).
     c. **Normal** (0x743149): RS (8,0) (6,0) (0xC,1) (1, 0). `CalcScreenCoors(c->pos, &o, &w, &h, false, false)` - the result is
        ignored. On-screen iff not (`W < o.x` or `o.x < 0` or `H < o.y` or `o.y < 0`) (strict; NaN counts as on-screen).
        On-screen (0x7431E9):
        - `sz = max(min(c->size, 1.2f), 0.3f)`; `c->size = sz`. `rin = 5.0f * sz`, `rout = sz * 25.0f` (stored f32).
          If `o.z < 2.5f`: `o.z = 2.5f`. `r = P->m_pPed->GetWeaponRadiusOnScreen()`; if `r > 0.0f`: `rin = r * 30.0f [0x8D6148]`,
          `rout = c->size * 20.0f + rin`.
        - RS (0xA,1) (0xB,1). For k = 0..2: `b = (float)k * 120.0f`; `a1 = 0.017453292f*b + spin`; `a2 = (b + 15.0f)*0.017453292f + spin`;
          `a3 = (b - 15.0f)*0.017453292f + spin` (spin = +0x1C); `R1 = rin + pulse[i]`, `R2 = rout + pulse[i]`;
          `V0 = (-(sin(a1)*R1) + o.x, -(cos(a1)*R1) + o.y)`, `V1 = (-(sin(a2)*R2) + o.x, -(cos(a2)*R2) + o.y)`,
          `V2 = (-(sin(a3)*R2) + o.x, -(cos(a3)*R2) + o.y)` (offsets stored f32 before adding o).
          Triangle(V0, V1, V2, z = o.z, rgb 0, intensity 255, recipZ 1.0f, alpha c->a). Then `C = ((V2+V1)+V0) * 0.33333334f`
          (x and y separately, x87) and each `V = (V - C) * 0.75f + C` (stored f32); RS (0xA,5) (0xB,6) (0x14,1);
          Triangle(V0, V1, V2, o.z, c->r, c->g, c->b, 255, 1.0f, c->a).
        - `T = c->time`: if `T != 0 && T != -1`: `spin += 0.75f; pulse[i] += 2.0f; c->size *= 0.9f`; else `spin += 0.05f`.
          If `spin > 6.2831855f` (strict, NaN no): `spin = 0`.
        - Only if `T == 0`: if `dir[i]`: `pulse[i] = c->size + c->size + pulse[i]` (stored); if `pulse[i] > c->size * 20.0f`
          (strict): `dir[i] = 0`. Else: `pulse[i] -= 2.0f` (stored); if `pulse[i] < 0.0f`: `dir[i] = 1`.
        Off-screen (0x7435C1): only if `IsCoopGameGoingOn()`. Let `M` = matrix at 0xB6F99C (TheCamera+0x974): `R` = M+0x00,
        `U` = M+0x20, `Q` = M+0x30 (all CVector f32).
        - `d = c->pos - Q`; `th1 = atan2((U.x*d.x + U.z*d.z) + U.y*d.y, (R.x*d.x + R.z*d.z) + R.y*d.y)` (x87 fpatan, ST1/ST0);
          `s1 = sin(th1)` stored, `c1 = cos(th1)` kept in x87.
        - `d = c->pos - P->m_pPed->GetPosition()`; `th2` with the same formula; `s2 = sin(th2)`, `c2 = cos(th2)` (stored f32).
        - `X = (1.0f - c1) * W * 0.5f`, `Y = (1.0f - s1) * H * 0.5f` (fimul, stored).
          `t = s2*0.5f + c2` (stored), `u = c2*0.5f + s2` (stored); `A = 15*s2`, `B = 15*c2` (15 = [0x8D6140], x87):
          Triangle1: `V0 = (X, Y)`, `V1 = (t*15 + X, (A + Y) - B*0.5f)`, `V2 = ((B + X) - A*0.5f, u*15 + Y)`; z 2.5f, rgb 0,
          intensity 255, recipZ 1.0f, alpha 255.
          Triangle2: `X' = c2*15 * 0.1 (f64 constant 0x86EEE0) + X`, `Y' = 0.1f (f32 0x858B1C) * (15*s2) + Y` (both stored);
          `V0 = (X', Y')`, `V1 = (t*15*0.8f + X', (A - B*0.5f)*0.8f + Y')`, `V2 = ((B - A*0.5f)*0.8f + X', u*15*0.8f + Y')`;
          z 2.5f, rgb = c->r,g,b, intensity 255, recipZ 1.0f, alpha 255.
     e. (0x7438B3) RS (0xC,0) (8,1) (6,1).
  2. Coop target (only if `TheCamera.m_aCams[TheCamera.m_nActiveCam (u8 0xB6F081)].m_nMode (i16 at 0xB6F1A8 + n*0x238) == 0x31`):
     - RS (8,0) (6,0) (0xC,1) (0xA,5) (0xB,6) (1, gpCrossHairTex raster). With `cx = cam[n]+0x27C` (f32 0xB6F2A4 + n*0x238),
       `cy = cam[n]+0x280` (0xB6F2A8): sprite at (`(cx + 1.0f)*W*0.5f`, `(cy + 1.0f)*H*0.5f`, 100.0f), size (10, 10),
       rgb (255, 128, 0), intensity 128, rz 0.01f, rotation `ang(now)`, alpha 255.
     - `p0 = Players[0].m_pPed`; `shooter = Players[1].m_pPed (0xB7CF28)`; if `p0->+0x58C (vehicle) != 0 && vehicle->+0x460 (driver) != p0`:
       `shooter = p0`. If shooter: `wi = GetWeaponInfo(shooter->m_aWeapons[(i8)shooter->+0x718].m_Type (+0x5A0 + slot*0x1C),
       shooter->GetWeaponSkill())`; `tgt = FindNearestTargetEntityWithScreenCoors(cx, cy, wi->+8 (target range) * 2 (fadd st0,st0),
       shooter->GetPosition(), null, null)`; else `tgt = null`.
     - If `tgt != gLastTarget`: `gLastTarget = tgt; gLastTargetTime = now`.
     - If tgt and `CalcScreenCoors(tgt->GetPosition(), &o, &w, &h, true, true)`: k-scaling as in 1b (`k = 20/h`, if `k > 1`);
       `dt = now - gLastTargetTime` (u32 math, used as int32); `s = 3.0f - (float)dt * 0.001953125f`; if `1.0f > s` `s = 1.0f`;
       `al = dt/4 (signed, toward zero) + 0x46`, if `al > 255` (signed) `al = 255`; sprite at o, size (`w*s`, `h*s`), rgb (al, 0, 0),
       intensity al, rz 0.01f, rotation `ang(now)`, alpha 255.
     - RS (8,1) (6,1).
  3. For i = 0, 1: `0x56EF90(&CWorld::Players[i] + 0x158, i)` (thiscall).
- Edge cases: the off-screen arrow uses `th1` (camera relative) for its screen position and `th2` (relative to the player ped)
  for its direction. X offset of triangle 2 uses a double 0.1, Y uses 0.1f (keep). `al` is not clamped below (dt is small).
- gta-reversed: `plugin::Call` (WeaponEffects.cpp:102). Open questions: (1) 0x742CF0: the gr name of the camera byte
  at TheCamera+0x3D is not verified (behaviour is clear: non-zero -> skip all). (2) 0x7435CE: the matrix at 0xB6F99C is
  presumably TheCamera's view matrix (gr m_mViewMatrix); only the addresses are certain.
