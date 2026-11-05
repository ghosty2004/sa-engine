# CPickups (WORLD) - full specs

Data used: `CPickups::aPickUps` 0x9788C0 (`CPickup[620]`, stride 0x20), `CPickups::NumMessages` 0x978678 (u16),
`CPickups::aMessages` 0x978680 (`tPickupMessage[16]`, stride 0x24). CPickup layout (gr `Pickup.h`, size 0x20):
+0x00 `m_fRevenueValue` f32, +0x04 `m_pObject` CObject*, +0x08 `m_nAmmo` u32, +0x0C `m_nRegenerationTime` u32,
+0x10 `m_vecPos` (3 x int16, world*8), +0x16 `m_nMoneyPerDay` u16, +0x18 `m_nModelIndex` i16,
+0x1A `m_nReferenceIndex` u16, +0x1C `m_nPickupType` u8, +0x1D flags u8 (bit0 bDisabled, bit1 bEmpty,
bit2 bHelpMessageDisplayed, bit3 bVisible, bits4-6 nPropertyTextIndex).
`0x821B40` is the CRT `_ftol2` (truncation toward zero to int64 in EDX:EAX); `0x821B1E` is CRT `rand()`.

## 0x455720 CPickups::DoPickUpEffects  (catalogue: specified/REA, port level 4)
- Convention: `__cdecl void (CEntity* entity [esp+4])` (always a pickup CObject), plain `ret`. Locals 0x28.
  The argument stack slot is reused as a temp ("kind", later the scale) - irrelevant for a port.
- Callers: CEntity::PreRender 0x535FA0.
- Callees: CPickups::FindPickUpForThisObject 0x4551C0 (cdecl(CObject*) -> CPickup*), CPickup::PickUpShouldBeInvisible
  0x454D20 (thiscall, AL), CPostEffects::IsVisionFXActive 0x7034F0 (AL), CCoronas::RegisterCorona (eCoronaType overload)
  0x6FC580 (cdecl, 21 args), CPickups::WeaponForModel 0x454AE0 (cdecl(int32 model) -> int32), CSprite::CalcScreenCoors
  0x70CE30 (cdecl(const RwV3d&, RwV3d* out, float* w, float* h, bool, bool) -> AL), CPickup::FindStringForTextIndex
  0x455540 (cdecl(int32) -> const char*), CText::Get 0x6A0050 (thiscall on TheText 0xC1B340), CPlaceable::AllocateMatrix
  0x54F560, CSimpleTransform::UpdateMatrix 0x54F1B0 (thiscall on entity+4, (CMatrix*)), rand, _ftol2.
- Globals read: MI_PICKUP_* u16 model ids (zero-extended before compare with the sign-extended `entity->m_nModelIndex` (+0x22,
  i16)): CAMERA 0x8CD5D8, ADRENALINE 0x8CD5BC, BODYARMOUR 0x8CD5C0, BRIBE 0x8CD5D0, INFO 0x8CD5C4, KILLFRENZY 0x8CD5D4,
  HEALTH 0x8CD5C8, BONUS 0x8CD5CC, PROPERTY 0x8CD5E8, PROPERTY_FORSALE 0x8CD5EC, REVENUE 0x8CD5E0, SAVEGAME 0x8CD5E4,
  CLOTHES 0x8CD5F0. `TheCamera` 0xB6F028 (`m_nActiveCam` u8 at +0x59 = 0xB6F081; `m_aCams[i].m_nMode` i16 at
  0xB6F1A8 + i*0x238; `m_matrix` 0xB6F03C; placement pos 0xB6F02C), `CClock::ms_nGameClockHours` 0xB70153 (u8),
  `CTimer::m_snTimeInMilliseconds` 0xB7CB84, `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8, colour table 0x8A5FB0
  (8 bytes per entry, bytes +0/+1/+2 = r/g/b, indexed by "kind"). Written: NumMessages, aMessages[NumMessages].
- Entity (CObject) fields: +0x04 placement pos, +0x14 m_matrix, +0x22 m_nModelIndex, +0x13D m_nBonusValue u8,
  +0x13E m_wCostValue u16, +0x140 m_nObjectFlags (0x4 bPickupPropertyForSale, 0x8 bPickupInShopOutOfStock,
  0x02000000 bDoNotRender).
- Constants: 0x858B14 = 3.0517578e-05f (1/32768), 0x859B60 = -50.0f, 0x858C54 = 4294967296.0f (dead unsigned fix-up),
  0x859B5C = 0.0007666016f, 0x859B58 = 1.7f, 0x859B54 = 3.7f, 0x858CB0 = 0.7f, 0x859030 = 14.0f,
  0x859B50 = 0.071428575f, 0x858624 = 1.0f, 0x859AAC = 255.0f, 0x858F08 = 1.2f, 0x858CC8 = 0.6f, 0x859B0C = 0.0030566407f.
- Behaviour:
  1. `pk = FindPickUpForThisObject(entity)`.
  2. Visibility (bDoNotRender = bit 0x02000000 of +0x140):
     - if `model == MI_PICKUP_CAMERA`:
       - if `TheCamera.m_aCams[TheCamera.m_nActiveCam].m_nMode == 46` (MODE_CAMERA): clear the bit; continue at 3.
       - else: set the bit. If `ClockHours < 5` (unsigned) or `IsVisionFXActive()`: register the corona:
         `a = _ftol2((float)(rand() & 0xFFFF) * 3.0517578e-05f * -50.0f)` (fild, two f32 fmuls, truncate; a in -49..0);
         `r = 100 - a` (int); `s = (sin((float)(time & 0x1FFF) * 0.0007666016f) + 1.7f) * 3.7f` (fild of a
         non-negative int, fmul f32, `fsin` on the x87 value, fadd, fmul, stored f32);
         `g = _ftol2((float)r * 0.7f)`;
         `RegisterCorona(id, nullptr, r, g, g, 255, entity->GetPosition() /*matrix ? &m->pos : &placement.pos*/,
         s /*size*/, 100.0f /*range*/, 1 /*coronaType*/, 0 /*flare*/, 0 /*refl*/, 0 /*LOS*/, 0 /*trail*/, 0.0f /*normalAngle*/,
         false /*neonFade*/, 1.5f /*pullTowardsCam*/, false /*fullBrightAtStart*/, 15.0f /*fadeSpeed*/, false, false)`.
         r/g/b/intensity are pushed as dwords, callee uses the low byte. `id` = `lea eax,[esp+0x85]` = the address
         (entry ESP + 1), i.e. a stack address one byte above this function's return-address slot (see Oddities).
         Then (bit is set) return.
     - else: bit := `pk->PickUpShouldBeInvisible()` (AL, bit 0 moved to bit 25; other bits kept).
  3. If the bit is now set: return.
  4. kind: `m = entity->m_nModelIndex`; kind = 0x2F if m is ADRENALINE, HEALTH, BONUS, PROPERTY_FORSALE or CLOTHES;
     0x30 if BODYARMOUR; 0x35 if REVENUE; and for BRIBE, INFO, KILLFRENZY, PROPERTY, SAVEGAME kind is NOT assigned
     (the slot keeps the previous content = the `entity` argument pointer value); otherwise kind = `WeaponForModel(m)`.
     Test order: ADRENALINE, BODYARMOUR, BRIBE, INFO, KILLFRENZY, HEALTH, BONUS, PROPERTY, PROPERTY_FORSALE, REVENUE,
     SAVEGAME, CLOTHES, else weapon.
  5. Help message: only if `(flags & 0xC) != 0 || m_nBonusValue != 0 || m_wCostValue != 0`:
     a. `dist = sqrt((cam.y - ent.y)^2 + (cam.x - ent.x)^2)` (2D; y-term computed first; cam = TheCamera matrix pos or
        placement pos). Continue only if `dist < 14.0f` (NaN/>= skip) and `NumMessages < 16` (unsigned u16 compare).
     b. `p = (ent.x, ent.y, ent.z + 0.7f)`; `if (!CalcScreenCoors(p, &out, &w, &h, true, true))` skip.
     c. `msg = &aMessages[NumMessages]`: +0x00 = out.x, +0x04 = out.y, +0x0C = w, +0x10 = h,
        +0x08 = `WeaponForModel(m)` as int32 (written into the slot gr calls pos.z),
        +0x14/+0x15/+0x16 = colour table[(int16)kind] r/g/b (index = low 16 bits of kind, sign-extended, *8),
        +0x17 = `_ftol2((1.0f - dist * 0.071428575f) * 255.0f)` (fld dist f32, fmul, fsubr 1.0, fmul 255),
        +0x18 bit0 = (flags & 0x8) ? 1 : 0 (other bits kept), +0x19 = m_nBonusValue, +0x1C = (int32)m_wCostValue * 5,
        +0x20 text and +0x18 bit1:
          - m == PROPERTY: text = `TheText.Get(FindStringForTextIndex((FindPickUpForThisObject(entity)->flags >> 4) & 7))`, bit1 = 0;
          - m == PROPERTY_FORSALE: same text, bit1 = 1;
          - else text = nullptr, bit1 = 0.
        `NumMessages += 1` (word inc, before storing the +0x18 byte).
  6. Spin/scale (always reached when not hidden):
     a. `bb = ms_modelInfoPtrs[m]->m_pColModel (+0x14)->m_boundBox` (min at +0, max at +0xC);
        `sx = max.x-min.x`, `sy = max.y-min.y`, `sz = max.z-min.z` (each rounded to f32).
     b. `m1 = (sy > sz) ? sy : sz`; `d = (sx > m1) ? sx : m1` (NaN -> the second operand).
     c. `q = 1.2f / d`; `if (1.0f > q) q = 1.0f` (i.e. q = max(q, 1.0), NaN keeps q);
        `sc = (q - 1.0f) * 0.6f + 1.0f` (stored f32); if `m == 0x16A` (362, MODEL_MINIGUN) `sc = 1.2f` (bits 0x3F99999A).
     d. `ang = (float)(time & 0x7FF) * 0.0030566407f`; `c = cos(ang) * sc`; `s = sin(ang) * sc` (both f32).
     e. Matrix writes, each preceded by "if (!entity->m_matrix) { AllocateMatrix(); placement.UpdateMatrix(m_matrix); }":
        right = (c, s, 0), up = (-s, c, 0), at = (0, 0, sc) in the order +0x00, +0x04, +0x08, +0x10, +0x14, +0x18,
        +0x20, +0x24, +0x28. Position and the pad words are untouched.
- Oddities: (a) the corona id is a stack address, so every camera pickup registers the SAME id each frame (only one
  camera corona exists at a time). A port must use one fixed id for all camera-pickup coronas (value arbitrary but
  constant). (b) kind left unassigned for BRIBE/INFO/KILLFRENZY/PROPERTY/SAVEGAME: the colour index is then the low 16
  bits of the entity pointer -> out-of-table read; the colour is only used if a message is built, which needs
  bonus/cost/flags. (c) WeaponForModel may return -1 -> index -1 reads 8 bytes before the table.
- gta-reversed: hooked `.Reversed = false`, body is `plugin::Call`. No code to compare.
- Open questions: (1) 0x455844 corona id - exact value is the address entry_ESP+1 (entry_ESP = address of the return
  address); port behaviour = constant id, confirm with an oracle that only identity matters. (2) 0x45594C kind for the
  5 unassigned models: confirm with an oracle whether such pickups ever carry bonus/cost/flags (if so the colour comes
  from out-of-table memory, which a port cannot reproduce exactly).

## 0x456F20 CPickups::GenerateNewOne  (catalogue: specified/REA, port level 17)
- Convention: `__cdecl int32 (CVector coors /*by value [esp+4..+0xC]*/, uint32 modelId [+0x10], uint8 pickupType [+0x14],
  uint32 ammo [+0x18], uint32 moneyPerDay [+0x1C], bool isEmpty [+0x20], const char* message [+0x24])`, returns the pickup
  reference in EAX (`(refIndex << 16) | slot`, or -1), plain `ret`.
- Callers (13): CGangWars::CreateAttackWave, CreateDefendingGroup, GenerateNewOne_WeaponType 0x457380, CreateSomeMoney
  0x458970, CRunningScript::ProcessCommands{500,700,800,1100,1300,2300}xx, Interior_c::AddPickups, CFileLoader::LoadPickup.
- Callees: CWorld::Remove 0x563280, virtual deleting destructor (vtable slot 0, arg 1), CPickup::FindTextIndexForString
  0x455500 (cdecl(const char*) -> AL), `_ftol2`, CPickup::IsVisible 0x454C70 (thiscall, AL), CPickup::GiveUsAPickUpObject
  0x4567E0 (thiscall(CObject** out, int32 slot)), CWorld::Add 0x563220.
- Globals: aPickUps (R/W), `CTimer::m_snTimeInMilliseconds` 0xB7CB84; const 0x859000 = 8.0f.
- Behaviour:
  1. Slot selection (all searches compare the type byte +0x1C):
     a. if `pickupType == 13 || pickupType == 11 || isEmpty`: search i = 619 down to 0 for type 0; if found go to 2 with i
        (no object removal).
     b. search i = 0 up to 619 for type 0; if found go to 2.
     c. search i = 0..619 for type 8 (PICKUP_MONEY); if found: evict (step 1e) and go to 2.
     d. search i = 0..619 for type 4 or 5; if found: evict and go to 2; else return -1.
     e. evict: `if (o = slot.m_pObject) { CWorld::Remove(o); if (slot.m_pObject) slot.m_pObject->vtable[0](1); slot.m_pObject = nullptr; }`.
  2. Initialise slot i (`p = &aPickUps[i]`), in this store order:
     `p->m_nAmmo = ammo`; `p->m_nMoneyPerDay = (u16)moneyPerDay`; `p->flags = (p->flags & 0xF8) | ((isEmpty & 1) << 1)`
     (clears bDisabled/bHelpMessageDisplayed, sets bEmpty); `p->m_nPickupType = pickupType`; `p->m_fRevenueValue = 0`
     (int 0 store); `now = CTimer::m_snTimeInMilliseconds`; `p->m_nRegenerationTime = now`; then by type:
     4 -> `now + 20000`; 5 -> `now + 120000`; 8 -> `now + 30000`; 9 or 10 -> type := 9, `now + 1500`;
     11 or 12 -> type := 11, `now + 1500`; others keep `now`.
  3. `p->m_nModelIndex = (i16)modelId`; `t = FindTextIndexForString(message)`; flags bits 4-6 := `t & 7`.
  4. `p->m_vecPos.x = (i16)_ftol2(coors.x * 8.0f)`, then y, then z (fld f32, fmul f32 8.0, truncate, store low 16 bits).
  5. flags bit 3 (bVisible) := `p->IsVisible() & 1`.
  6. `p->m_pObject = nullptr`; if bVisible: `p->GiveUsAPickUpObject(&p->m_pObject, -1)`; if `p->m_pObject` `CWorld::Add(p->m_pObject)`.
  7. `r = p->m_nReferenceIndex` (u16); if `r >= 0xFFFE` then `r = 1` else `r = r + 1`; store; return `(r << 16) | i`.
- Oddities: a reused type-0 slot is assumed to have no object (pointer overwritten without Remove). Coordinates are
  truncated toward zero (not rounded). For types 9/10/11/12 the stored type differs from the argument.
- gta-reversed: hook commented out; `GenerateNewOne` is a `plugin::Call`. No code to compare.
- Open questions: none.

## 0x458A80 CPickups::CreatePickupCoorsCloseToCoors  (catalogue: specified/REA, port level 9)
- Convention: `__cdecl void (float inX [esp+4], float inY [+8], float inZ [+0xC], float* outX [+0x10], float* outY [+0x14],
  float* outZ [+0x18])`, plain `ret`. Locals 0xA8.
- Callers: CPed::CreateDeadPedPickupCoors 0x459180, CTaskSimpleJetPack::DropJetPack 0x67B660.
- Callees: `rand` 0x821B1E, CWorld::FindGroundZFor3DCoord 0x5696C0 (cdecl(CVector by value, bool* found, CEntity** out) -> ST0),
  CWorld::FindPlayerCoors 0x56E010 (cdecl(CVector* out, int32 id) -> CVector*), CPickups::TestForPickupsInBubble 0x456450
  (cdecl(CVector by value, float) -> AL), CWorld::GetIsLineOfSightClear 0x56A490, CWorld::TestSphereAgainstWorld 0x569E20.
- Constants: 0x859BBC = 0.024543693f (2*pi/256), 0x858CE8 = 1.5f, 0x858B8C = 0.5f, 0x858C24 = 0.3f, 0x858EE8 = 0.4f,
  0x858CA0 = 2.0f; immediates 1.3f (0x3FA66666), 1.2f (0x3F99999A).
- Behaviour: `for (i = 0; i < 32; ++i)` (counter at [esp+0x14]):
  1. `r = rand() & 0xFF`; `ang = (float)r * 0.024543693f` (kept on the x87 stack);
     `cx = inX + sin(ang) * 1.5f` (f32 store); `cy = inY + cos(ang) * 1.5f` (f32 store; `fsin`/`fcos` of the same register).
  2. `gz = FindGroundZFor3DCoord({cx, cy, inZ}, &found, nullptr)`; `cz = gz + 0.5f` (f32). If `!found` -> next i.
  3. `A = (inX, inY, inZ + 0.3f)`; `D = (cx - A.x, cy - A.y, cz - A.z)` (each f32);
     `L = sqrt((D.z*D.z + D.y*D.y) + D.x*D.x) + 0.4f`; `M = sqrt((D.z*D.z + D.y*D.y) + D.x*D.x)`; `k = L / M`;
     `B = (A.x + D.x*k, A.y + D.y*k, A.z + D.z*k)` (each product rounded to f32 precision; B.y/B.z products stored f32 first).
     (B = A extended 0.4 units past the candidate.)
  4. `pd = sqrt((cy - P3.y)*(cy - P4.y) + (cx - P1.x)*(cx - P2.x))` where P1..P4 are four separate
     `FindPlayerCoors(&tmp, -1)` results called in the order P1, P2, P3, P4 (all equal in practice) - 2D distance
     to the player, y-term first.
  5. if `pd > 2.0f` (NaN -> else branch): if `i <= 16` and `TestForPickupsInBubble({cx, cy, cz}, 1.3f)` -> next i.
     else (pd <= 2.0 or NaN): if `i <= 16` -> next i.
  6. `v = (i < 16)`; `if (!GetIsLineOfSightClear(&B, &A, true, v, false, v, false, false, false))` -> next i.
  7. if `i > 16` -> success. Else `if (TestSphereAgainstWorld({cx, cy, cz}, 1.2f, nullptr, false, true, false, false, false, false) == nullptr)`
     -> success; else next i.
  8. success: `*outX = cx; *outY = cy; *outZ = cz`; return.
  - after 32 failures: `*outX = inX; *outY = inY; *outZ = inZ + 0.4f`.
- Oddities: for i in 0..16 the candidate must be more than 2 m from the player; from i = 17 on proximity and the bubble /
  sphere tests are skipped; vehicles/objects in the LOS test only for i < 16 (i == 16 uses buildings only).
  `fsin`/`fcos` are not affected by the 24-bit precision control (full 64-bit mantissa result), the following
  fmul/fadd are.
- gta-reversed: hooked `.Reversed = false`, `plugin::Call` only.
- Open questions: none.
