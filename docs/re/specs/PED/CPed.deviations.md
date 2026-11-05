# CPed: gta-reversed deviation notes (PED)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x5D4640 CPed::Load  (gr_deviations = 2)
Original bytes (`__thiscall bool()`, AL = 1, SEH frame): stack CPedSaveStructure (0x18C bytes) whose 13 CWeapon members (stride 0x1C) are
constructed with `eh_vector_constructor_iterator(.., 0x1C, 13, CWeapon ctor 0x441E00, dtor 0x4411A0)`; `LoadDataFromWorkBuffer(&size, 4)`
(0x5D1300); `LoadDataFromWorkBuffer(&save, 0x18C)` - a fixed 0x18C bytes, the size read first is ignored; `save.Extract(this)` (0x5D44B0);
`eh_vector_destructor_iterator` on the 13 weapons; return 1.
Row Ped.cpp:501 (comment about template/copy-ctor workaround, "bugfix-comment"):
- Verdict: REFUTED (gr reads the same two blocks with the same fixed size). Class: harmless.
Row Ped.cpp:504 (`assert(size == sizeof(save))`): Verdict REFUTED (debug only; the original never checks). Class: harmless.

## 0x5DEF60 CPed::GetLocalDirection  (gr_deviations = 2)
Original bytes (`__thiscall int32(const CVector2D* point [esp+4])`, `ret 4`; the function is split in two chunks 0x5DEF60-0x5DEF88
and 0x5DEF90-0x5DEFC8):
  1. `a = fpatan(ST1 = -point.x, ST0 = point.y)` = atan2(-x, y) (0x5DEF64-0x5DEF6B); `a -= this->m_fCurrentRotation (+0x558)`;
     `a += 0.7853982f [0x859AB0]` - all in x87 registers, never stored.
  2. while (`a < 0.0f`) `a += 6.2831855f [0x858CBC]` (`fcom 0.0f; test ah,5; jp` exits on >= 0 and on NaN, 0x5DEF79-0x5DEFA1).
  3. `dir = _ftol2(a * 0.63661975f [0x858FB8])` (2/pi as f32, truncation; 0x5DEFA3-0x5DEFA9).
  4. if `dir > 3` (signed): `dir = (dir - 4) & 3` computed as `dir + (-4 - (((u32)(dir-4) >> 2) << 2))` (0x5DEFB3-0x5DEFC3);
     equivalent to `while (dir > 3) dir -= 4`. Values <= 3 (including negative, e.g. 0x80000000 from a NaN) are returned unchanged.
Row Ped.cpp:838 (`return ((int32)RadiansToDegrees(angle) / 90) % 4`):
- Verdict: CONFIRMED - gr converts to degrees, truncates to int degrees and integer-divides by 90; the original multiplies by
  0.63661975f in x87 and truncates once. Results can differ by one quadrant for angles within float error of a multiple of
  pi/2, and for NaN (original returns INT_MIN, gr `INT_MIN / 90 % 4` = -2 under MSVC semantics... i.e. not INT_MIN).
- Class: keep-original.
Row Ped.cpp:840 (comment "Original R* code - Kinda stupid, we just use modulo instead"):
- Verdict: REFUTED as a separate behaviour - for the non-negative `dir` the original reduction equals `% 4`; the commented
  "original" loop in gr is wrong (it tests `angle`, not `dir`), the bytes test `dir`. Class: harmless (covered by row 838).

## 0x5DEFD0 CPed::IsPedShootable  (gr_deviations = 1)
Row Ped.cpp:851 (comment: switch vs `<= PEDSTATE_STATES_CAN_SHOOT`).
- Original bytes: `cmp dword [ecx+0x530], 0x2E; setle al; ret` (0x5DEFD0-0x5DEFDA): `return m_nPedState <= 46` (signed;
  46 = PEDSTATE_STATES_CAN_SHOOT). Only AL is set.
- Verdict: REFUTED - gr's switch lists all 47 values 0..46 and returns false otherwise; identical for every non-negative state.
  (Negative states would return true in the original and false in gr; no code stores a negative state.)
- Class: harmless (port as the signed compare).

## 0x5DF170 CPed::RemoveGogglesModel  (gr_deviations = 1)
Row Ped.cpp:1047 (`#ifdef SA_SKINNED_PEDS` around the skin-anim removal).
- Original bytes (split body: 0x5DF170 -> 0x40151C -> 0x5DF179; `__thiscall void()`): if `m_pGogglesObject (+0x4FC)`:
  `CVisibilityPlugins::GetClumpModelInfo(clump)->RemoveRef()`; **if `GetFirstAtomic(clump)` and `RpSkinGeometryGetSkin(atomic->geometry)`:
  `RpClumpForAllAtomics(clump, AtomicRemoveAnimFromSkinCB, 0)`** (0x5DF18D-0x5DF1C8); `RpClumpDestroy(clump)`; `m_pGogglesObject = 0`;
  if `m_pGogglesState (+0x500)`: `*m_pGogglesState = 0` (byte), `m_pGogglesState = 0`.
- Verdict: REFUTED (gr matches the bytes when SA_SKINNED_PEDS is defined). Class: harmless (port always includes the skinned branch).

## 0x5DF220 CPed::GrantAmmo  (gr_deviations = 1)
Original bytes (`__thiscall void(eWeaponType type [esp+4], int32 ammo [esp+8])`, `ret 8`): `slot = CWeaponInfo::GetWeaponInfo(type, 1)->m_nSlot (+0x14)`
(0x743C60, cdecl); if `slot == -1` return. `w = &this->m_aWeapons[slot]` (CPed+0x5A0, stride 0x1C: +4 state, +8 ammoInClip, +0xC totalAmmo).
`w->total = w->total + ammo` (stored, 0x5DF253), then `if (total >= 99999) total = 99999` (**signed** `jl`, stored again, 0x5DF24D-0x5DF267);
if `w->state == 3 (OUT_OF_AMMO) && total > 0` (signed `jle`): `state = 0 (READY)` (0x5DF260-0x5DF273).
Row Ped.cpp:1085 (`// TODO: Inlined` around the state update):
- Verdict: REFUTED (the inlined state update matches). Class: harmless.
- Additional difference (not a catalogued row): gr clamps with `std::min(total + ammo, 99'999u)` (unsigned) and tests `> 0` on a
  uint32; the original is signed, so a negative sum stays negative (and does not set READY) in the original, while gr gives 99999.
  Keep-original (signed).

## 0x5DF290 CPed::SetAmmo  (gr_deviations = 1)
Original bytes (`__thiscall void(eWeaponType type [esp+4], int32 ammo [esp+8])`, `ret 8`): slot via `GetWeaponInfo(type, 1)->m_nSlot`;
`-1` -> return. `w->total = ammo` (0x5DF2B9), then `total = (ammo < 99999 ? ammo : 99999)` (signed, stored at 0x5DF2CE);
**`w->ammoInClip = (total < clip) ? total : clip`** (`cmp ecx, edx; jge` keeps the old clip when total >= clip, 0x5DF2C6-0x5DF2D8),
i.e. `clip = min(total, clip)`; then `if (state == 3 && total > 0) state = 0` (signed).
Row Ped.cpp:1106 (`// TODO: Inlined`):
- Verdict: REFUTED for the state update (matches). Class: harmless.
- Additional difference (gr logic error, not a catalogued row): gr `Ped.cpp:1103` does
  `m_AmmoInClip = std::max(m_TotalAmmo, m_AmmoInClip)`; the bytes compute the **min**. gr also clamps unsigned. Keep-original.

## 0x5DF340 CPed::DoGunFlash  (gr_deviations = 1)
Row Ped.cpp:1142 (rng-helper `GetRandomNumberInRange(-360.0f, 360.0f)`).
- Original bytes (`__thiscall bool(int32 duration [esp+4], bool leftHand [esp+8])`, `ret 8`, AL): if `m_pWeaponObject (+0x4F4)` or
  `m_pGunflashObject (+0x4F8)` is null return 0. `s = (u16)m_sGunFlashBlendStart` (word 0x8D1370): leftHand -> `+0x508 = s`, `+0x50A = (int)s / duration`
  (signed idiv of the zero-extended value, low word stored); else `+0x504 = s`, `+0x506 = s / duration`. Then
  `angle = (float)rand() * 3.0518509e-05f [0x858C7C, = 1/32767] * 720.0f [0x86C2F8] - 360.0f [0x859E2C]` (fild of the full rand() result,
  no `& 0xFFFF`; x87, stored f32) (0x5DF3AA-0x5DF3D5); `RwMatrixRotate(&gunflashFrame->modelling (+0x10), &CPedIK::XaxisIK (0x8D232C), angle, 1)`; return 1.
- Verdict: CONFIRMED (inlined formula `t*720 - 360` with t = rand/32767; gr's helper computes `360*t + (-360)*(1-t)`: same range, different rounding).
- Class: keep-original.

## 0x5DF560 CPed::ShoulderBoneRotation  (gr_deviations = 1)
Row Ped.cpp:1226 (original `if` on a static bool around `rx /= 2`).
- Original bytes: after each `CMatrix::ConvertToEulerAngles` (0x59A840; two inlined copies, left and right side, 0x5DF680 and 0x5DF7F9):
  `if (byte [0x8D21E0] != 0) rx = rx * 0.5f [0x858B8C]` (0x5DF685-0x5DF698 and 0x5DF7FE-0x5DF815), then ConvertFromEulerAngles with flags 0x15.
  The byte at 0x8D21E0 is 1 in the shipped .data and never written.
- Verdict: REFUTED (always taken; `* 0.5f` equals `/ 2.f` exactly). Class: harmless (a port may read the global).

## 0x5E0360 CPed::DropEntityThatThisPedIsHolding  (gr_deviations = 1)
Original bytes (`__thiscall void(bool deleteHeld [esp+4])`, `ret 4`):
  1. `t = intel->m_TaskMgr.FindActiveTaskByType(0x133)`, else `(0x134)`, else `(0x135)` (TASK_SIMPLE_HOLD_ENTITY / PICKUP / PUTDOWN;
     `this->m_pIntelligence (+0x47C) + 4`, 0x681740) - inlined GetHoldingTask; none -> return.
  2. **`held = t->m_pEntityToHold (+0x8)` is read BEFORE** `t->DropEntity(this, 1)` (0x6930F0) (0x5E03A9-0x5E03B1).
  3. if `held && deleteHeld`: unless `held` is an object (type bits 4) with `m_nObjectType (+0x13C) == 2` (OBJECT_MISSION only):
     `held->DeleteRwObject()` (vtable +0x20), `CWorld::Remove(held)`, `delete held` (vtable[0](1)) (0x5E03C2-0x5E03EA).
Row Ped.cpp:1470 (`// TODO; Are these 3 lines inlined?`):
- Verdict: REFUTED (the three calls are real calls in that order, as gr has them). Class: harmless.
- Additional gr logic errors (not catalogued rows), keep-original:
  - gr reads `task->m_pEntityToHold` AFTER `DropEntity`, which always ends with `CEntity::ClearReference(m_pEntityToHold)`
    (`TaskSimpleHoldEntity.cpp:476`), so gr never deletes the held entity. The original captured the pointer first and deletes it.
  - gr's `IsMissionObject()` also exempts OBJECT_MISSION2 (6); the original only exempts type 2.

## 0x5E04B0 CPed::GetWalkAnimSpeed  (gr_deviations = 1)
Row Ped.cpp:1522 (comment "quite garbage ... assumptions"; the code below it).
- Original bytes (`__thiscall float()`, return ST0, plain `ret`): `h = CAnimManager::GetAnimAssociation(m_nAnimGroup (+0x4D4), 0)->m_BlendHier`
  (0x4D3A60 cdecl, +0x10); `seq = h->m_pSequences` (+0x4, read BEFORE uncompressing); `UncompressAnimation(h)` (0x4D41C0);
  `n = (int16)seq->m_FramesNum`; if `n <= 0` return 0.0f (0x5E050E). `f = seq->m_Frames` (+0x8, read after uncompress).
  If `seq->flags & 2` (HasTranslation): `return (f[(n-1)*0x20 + 0x18] - f[0x18]) / h->m_fTotalTime` (Trans.y of last and first frame;
  fld, fsub, fdiv; 0x5E04E9-0x5E04F5).
  Else: `return (f[(n-1)*0x14 + 0x18] - f[0x18]) / h->m_fTotalTime` (0x5E04FB-0x5E0508): with 0x14-byte KeyFrames, offset +0x18 of
  the last frame is 4 bytes into the frame AFTER the last one (reads past the array: Rot.y of frame n), and f[0x18] is Rot.y of frame 1.
- Verdict: CONFIRMED - in the no-translation case gr reads `lastFrame->Rot.imag.y` (+0x4 of frame n-1) instead of the original's +0x18
  (one frame past the end); the translation case and first-frame read match.
- Class: keep-original (byte offset +0x18 from frame n-1 in both cases).

## 0x5E0730 CPed::CanSeeEntity  (gr_deviations = 2)
Original bytes (`__thiscall bool(CEntity* e [esp+4], float limit [esp+8])`, `ret 8`, AL):
  1. `a = CGeneral::GetAngleBetweenPoints(e.x, e.y, this.x, this.y)` (0x53CEA0, degrees, ST0) `* 0.017453292f` [0x8595EC] (x87).
  2. N(v): if `v > 6.2831855f` (strictly; `test ah,0x41; jne` keeps <, == and NaN) `v -= 6.2831855f`; else if `v < 0` `v += 6.2831855f`.
     Applied to `a` (0x5E0775-0x5E07A1) **and to `r = m_fCurrentRotation (+0x558)`** (0x5E07A7-0x5E07D2).
  3. `d = a - r` (fsubp); if `d < 0` `d = -d` (0x5E07D8-0x5E07E7).
  4. return `d < limit || d > 6.2831855f - limit` (0x5E07E9-0x5E0810; NaN -> false).
Row Ped.cpp:1550 (TODO: is FixRadianAngle inlined, 0x5E0780 / 0x5E07BB):
- Verdict: CONFIRMED - it is inlined twice, and the second copy normalises `m_fCurrentRotation`, which gr does not do; also gr subtracts
  2*pi when `angle == 2*pi` (its test is `angle < TWO_PI`), the original only when `angle > 2*pi`. Class: keep-original.
Row Ped.cpp:1562 (gr uses GetRadianAngleBetweenPoints instead of degrees * DEG2RAD):
- Verdict: CONFIRMED (the original converts the degree result with an f32 multiply by 0.017453292f; rounding differs). Class: keep-original.

## 0x5E1660 CPed::OurPedCanSeeThisEntity  (gr_deviations = 1)
Row Ped.cpp:1642 (NOTSA: `dir2D.SquaredMagnitude() >= 40.f * 40.f` instead of the magnitude).
- Original bytes (`__thiscall bool(CEntity* e [esp+4], bool isSpotted [esp+8])`, `ret 8`, return AL):
  `dx = e.x - this.x`, `dy = e.y - this.y` (positions via matrix or placement; both stored f32, 0x5E1687-0x5E16AD).
  If `!isSpotted`: `dot = dy * m_matrix->forward.y (+0x14) + dx * m_matrix->forward.x (+0x10)` (x87; `this->m_matrix` used
  without null check); if `dot < 0` **or NaN** (`test ah,1`) return false (0x5E16B3-0x5E16CE); `d = sqrt(dy*dy + dx*dx)`
  (x87, sum not stored); if `d >= 40.0f` (0x858A10) or NaN (`test ah,5; jp`) return false (0x5E16D4-0x5E16F3).
  Then `from = this pos + (0,0,1)`, `to = e pos (+ (0,0,1) if e is a ped, type bits 3)`; `CWorld::ProcessLineOfSight(from, to, cp,
  hitEnt, 1, 0, 0, isSpotted, 0, 0, 0, isSpotted)` (two literal call sites, 0x5E1760 / 0x5E179C); return `!hit`.
- Verdict: CONFIRMED - squared-magnitude vs sqrt compare differ for distances whose sqrt rounds up to 40.0f (sum just below 1600)
  and for NaN (original: not visible; gr: continues). gr's `dot < 0.f` also lets NaN through where the original returns false.
- Class: keep-original (use sqrt and the NaN-false compares).

## 0x5E1950 CPed::ClearLookFlag  (gr_deviations = 1)
Row Ped.cpp:2013 (comment: the original do-while never iterates more than once).
- Original bytes: entry 0x5E1950 is `nop; nop; jmp 0x4096D5`, which does `test byte [ecx+0x46C], 4` (bIsLooking) and jumps back
  to 0x5E1957 (a split body; port as one function). If clear: return. Loop body 0x5E1960-0x5E19EC:
  `flags2 (+0x470) &= ~0x800` (bit 11 = **bShakeFist** in gr's own bitfield order, `Ped.h:179`), `flags1 (+0x46C) = (flags1 & ~4) | 8` (bIsLooking off, bIsRestoringLook on);
  if `m_nPedState (+0x530) != 50 && != 58 && !(flags1 & 0x04000000 /*bIsDucking*/)`: `m_pedIK.m_flags (+0x528) &= ~2` (bTorsoUsed);
  `m_nLookTime (+0x748) = CTimer::m_snTimeInMilliseconds (0xB7CB84) + (m_nPedType (+0x598) is 0 or 1 ? 2000 : 4000)`;
  loop again only if `(state == 3 || state == 2) && (byte[+0x46C] & 4)` - the bit was just cleared, so never.
- Verdict: REFUTED for the loop (gr's straight-line version is equivalent; CanUseTorsoWhenLooking = the same three tests).
- Class: harmless.
- Additional gr logic error (not a catalogued row): gr clears `bIsDrowning` (bit 19 of +0x470); the bytes clear bit 11 = `bShakeFist`
  (`and edx, 0xFFFFF7FF`, 0x5E196C). Keep-original.

## 0x5E3990 CPed::RemoveWeaponModel  (gr_deviations = 1)
Row Ped.cpp:1880 (`#ifdef SA_SKINNED_PEDS`).
- Original bytes (`__thiscall void(int32 modelIndex [esp+4])`, `ret 4`): if `m_nPedType (+0x598)` is 0 or 1 (inlined IsPlayer): active weapon
  (`m_aWeapons[(int8)m_nActiveWeaponSlot (+0x718)]`, +0x18 m_FxSystem) -> `g_fxMan.DestroyFxSystem(fx)` and clear. If `m_pWeaponObject (+0x4F4)`
  and (`modelIndex == -1` or `ms_modelInfoPtrs[modelIndex] == GetClumpModelInfo(clump)`): `RemoveRef`; **skinned check +
  `RpClumpForAllAtomics(clump, AtomicRemoveAnimFromSkinCB, 0)`** (0x5E3A0D-0x5E3A47); `RpClumpDestroy`; `+0x4F4 = 0`; `+0x4F8 (m_pGunflashObject) = 0`.
  Always: `+0x504 (w) = 0`, `+0x508 (w) = 0`, `+0x740 = -1` (0x5E3A66-0x5E3A74).
- Verdict: REFUTED (gr matches with SA_SKINNED_PEDS defined). Class: harmless.

## 0x5E4880 CPed::SetModelIndex  (gr_deviations = 2)
Row Ped.cpp:3646 (assert): Verdict REFUTED (debug only). Class: harmless.
Row Ped.cpp:3664 (rng-helper `CGeneral::GetRandomNumberInRange(0.f, 100.f) < 3.f`, "Moved up here"):
- Original bytes (money block 0x5E490E-0x5E4982): first
  `m_nMoneyCount (+0x756, u16) = (CPopCycle::IsPedInGroup(model, 1) || IsPedInGroup(model, 6)) ? rand() % 50 + 20 : rand() % 25`
  (CRT rand 0x821B1E, signed `idiv`; the group tests are done first, model = (int16)+0x22); THEN always a second
  `r = _ftol2((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * 100.0f [0x858628])` (fild, two f32 fmul, truncation);
  if `r < 3` (signed int compare) `m_nMoneyCount = 400`.
- Verdict: CONFIRMED - (1) gr evaluates the 3% test first and skips the money rand() when it hits, so the original always consumes two
  rand() values in the order money-then-chance, gr one or two in the opposite order; (2) gr's float helper formula differs
  (cat14 finding 1); (3) the original compares the truncated integer with 3.
- Class: keep-original.

## 0x5E6080 CPed::GiveWeapon  (gr_deviations = 1)
Row Ped.cpp:2587 (`// TODO: Inlined` around the OUT_OF_AMMO -> READY update in the same-weapon branch).
- Original bytes (`__thiscall eWeaponSlot(eWeaponType type [esp+4], int32 ammo [esp+8], bool unused [esp+0xC])`, `ret 0xC`):
  `slot = GetWeaponInfo(type, 1)->m_nSlot`; `w = &m_aWeapons[slot]` (+0x5A0). Same type already in slot: slot 10 (GIFT) -> return 10;
  `w->total = w->total + ammo` stored, then clamped `>= 99999 -> 99999` (signed `jl`), `CWeapon::Reload(w, this)` (0x73AEB0);
  if `state == 3 && total > 0` (signed): state = 0; final: `if (state != 3) state = 0` (0x5E60C4-0x5E6117, 0x5E61CF-0x5E61D8).
  Different weapon: if `w->type != 0`: for slots 3, 4, 5 `ammo += w->total`; `RemoveWeaponModel(GetWeaponInfo(w->type,1)->m_nModelId1)`;
  if `slot == GetWeaponInfo(45 /*INFRARED*/, 1)->m_nSlot` `RemoveGogglesModel()`; `CWeapon::Shutdown(w)`. Then `CWeapon::Initialise(w,
  type, ammo, this)`; if `slot == (int8)m_nActiveWeaponSlot (+0x718)` and `!(flags1 (+0x46C) & 0x100)` (bInVehicle):
  `AddWeaponModel(GetWeaponInfo(m_aWeapons[active].type, 1)->m_nModelId1)`; then `if (state != 3) state = 0`. Return slot.
- Verdict: REFUTED (the inlined update matches). Class: harmless.
- Additional difference (not catalogued): gr clamps with unsigned `std::min(99'999u, total + ammo)`; the original is signed (negative
  sums are kept). Keep-original.

## 0x5E65A0 CPed::PreRenderAfterTest  (gr_deviations = 10)
Inline RNG form used by every rng row below (the original does not call 0x41BD90 there):
`v = (float)rand() * 3.0518509e-05f [0x858C7C, 1/32767] * (max - min) + min` (fild of the full rand() int; `max - min` computed as a
separate fld/fsub of the f32-stored bounds; x87, stored f32). gr's helper is `lerp` = `max*t + min*(1-t)` (cat14 finding 1).
Row Ped.cpp:2895 (FIX_BUGS: real-time shadow quality test):
- Original bytes (0x5E6759-0x5E6789): real-time shadow path iff `g_fx.GetFxQuality() == 3` or (`GetFxQuality() == 2` and
  `m_nPedType (+0x598) == 0` - PLAYER1 only). gr uses `IsPlayer()` (types 0 and 1). Verdict: CONFIRMED. Class: keep-original.
- Same block, further gr differences (not catalogued), keep-original: (a) distance test: the original compares the SQUARED 2D distance
  bone(root) - camera (`dy*dy + dx*dx`, 0x5E67BE-0x5E67DE) with `MAX_DISTANCE_PED_SHADOWS_SQR` (global 0xC4B6B0, `<=` passes, NaN fails);
  gr compares `DistanceBetweenPoints2D` (a non-squared distance, `Vector2D.h:274`) with the squared constant. (b) if `flags1 & 0x100`
  (in vehicle) the original sets the flag to 0 when `m_pVehicle (+0x58C)` is null (0x5E680F); gr keeps true. (c) after the spine test:
  when `!IsAlive()` the original uses `bIsDucking` (0x5E686D -> 0x5E6892); gr keeps the accumulated value. Order: the alive test comes first,
  then `pos.z - 0.2f > spine.z` -> bIsDucking, else the vehicle/task flag.
Row :2918 (notsa::cast to CTaskComplexEnterCarAsDriver, deviates=no): the original searches tasks 0x2C0, 0x2BF, 0x2BD (0x5E6711-0x5E6754) and
  uses the found 0x2BD task's target car (+0xC) subtype in {10, 9, 2}. Verdict: REFUTED (same test). Class: harmless.
Rows :2960, :2961, :2962 (rng in ScaleAnimHierMat, three components x three calls = 9 rand() at 0x5E6A96 ... 0x5E6D7F):
- Original: `k = [0x8D1378] (0.2f, .data global) * speed` (resp. [0x8D137C] 0.1f, [0x8D1380]); `hi = 1 + k`, `lo = 1 - k` stored f32; each
  component = the inline form above. Verdict: CONFIRMED (formula; gr also makes the three range factors constexpr instead of reading the
  .data globals). Class: keep-original.
Row :3001 (earthquake swing):
- Original (0x5E6F95-0x5E6FEC): only if `CWeather::Earthquake (0xC81340) > 0`: `swing = (rand()*c*(E - (-E)) + (-E)) * 0.0025f [0x86C340]`.
  Verdict: CONFIRMED - besides the helper formula, **gr omits the final `* 0.0025f` scale** (its swing is 400x larger). Class: keep-original.
Row :3034 (`notsa::contains({FALL, ATTACK, FIGHT})`, deviates=no): original tests states 0x2A, 0x36, 0x37, 0x10, 0x11 (0x5E7372-0x5E738F) after
  `|s.x| <= 0.05f`, `|s.y| <= 0.05f`. Verdict: REFUTED. Class: harmless.
Row :3040 (piece set {5, 6, 9}, deviates=no): original `piece in [5,6] or == 9` (0x5E73DA-0x5E73E8). Verdict: REFUTED for the set.
  Additional difference: the particle position is `center.x + GetRandomNumberInRange(-0.08f, 0.08f)` (0x41BD90), `center.y + (same)`,
  `center.z + radius (+0xC) * 0.75f` (no third random number, 0x5E742C-0x5E747F); gr's `CVector::Random` draws a third value for z.
  Keep-original.
Rows :3051, :3052, :3053 (wetness drip position):
- Original (0x5E756B-0x5E7612): `pos.x = ((float)rand() * c * 0.6f + pos.x) - 0.3f`, `pos.y` likewise, `pos.z = ((float)rand() * c * 1.0f +
  pos.z) - 0.8f` (constants 0.6f 0x858CC8, 0.3f 0x858C24, 1.0f 0x858624, 0.8f 0x858C98). Verdict: CONFIRMED - formula, and **gr uses the
  range +-0.03 for x/y where the original uses +-0.3**; z range matches (-0.8..0.2). Class: keep-original.
