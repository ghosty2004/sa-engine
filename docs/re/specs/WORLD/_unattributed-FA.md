# _unattributed-FA (WORLD) - RE-SPEC-FINAL group A

## 0x449FF0 sub_449ff0 (WORLD/CAM callers, 127 insns)  (catalogue: unknown/REA, port level 0)
Catalogue name: <none>, proposed: `void CGarage::FindDoorsWithGarage(CObject** door1, CObject** door2)` (gr Garage.h:170 declares exactly this,
no body; evidence: `this` is a CGarages::aGarages entry (index computed from 0x96C048, stride 0xD8) and it returns the two closest objects whose
garage index matches; callers CGarage::Update 0x44AA50, CCamera::CamControl 0x527FA0 (3), 0x44A660 (2), 0x44A750 (2)).
- Convention: `__thiscall void (CGarage* this, CObject** door1 [esp+4], CObject** door2 [esp+8])`, `ret 8`. The door1 arg slot is reused as
  the f32 distance local after door1 is cached in EBP. The best-1 distance lives in an x87 register for the whole function.
- Globals: 99999.9f [0x85999C]=99999.9f; 0.5f [0x858B8C]=0.5f; CGarages::aGarages 0x96C048; CPools::ms_pObjectPool [0xB7449C].
- Fields: garage +0 pos.x, +4 pos.y, +0xC dirA (x,y), +0x14 dirB (x,y), +0x20 width, +0x24 height (gr m_fLeft..: names per gr CGarage: m_Base,
  m_DirA, m_DirB, m_LenA? - offsets are what matter). Object +0x147 garage-door garage index (byte), size 0x19C.
- Behaviour:
  1. `best1 = 99999.9f` (st), `*door2 = 0`, `*door1 = 0`, `best2 = 99999.9f` (memory). `idx = (this - 0x96C048) / 0xD8` (signed magic-number
     division). Centre: `hw = width*0.5f`, `cx = (dirA.x*hw + pos.x)` (dirA.x*hw stored f32 first), `cy = dirA.y*hw + pos.y` (register);
     `hh = height*0.5f`, `cx = dirB.x*hh (f32) + cx (f32)` -> f32; `cy = dirB.y*hh + cy` -> f32.
  2. Object pool, `for (i = size-1; i >= 0; i--)`: skip free slots (`flags & 0x80`) / null; need `obj->+0x147 == (u8)idx`.
     `d = sqrt((cx - p.x)^2 + (cy - p.y)^2)` (x87: dx, dy in st; `dy*dy + dx*dx`; fsqrt) -> f32 (p = matrix ? matrix+0x30 : obj+4).
     - If `*door1 == 0`: `*door1 = obj`, `best1 = d` (reloaded f32).
     - Else if `d < best1` (`fcomp st(1); test ah,5; jp`): `best2 = best1` (stored f32), `*door2 = old *door1`, `*door1 = obj`, `best1 = d`.
     - Else if `*door2 == 0` or `d < best2` (`test ah,5; jp skip`): `*door2 = obj`, `best2 = d`.
- gta-reversed: declared only. Open questions: none.

## 0x453E40 sub_453e40 (BlendAnimation + AddMessageQ)  (catalogue: unknown/REA, port level 9)
Catalogue name: <none>, proposed: anim finish callback `static void <Pickups/AssetPurchase>::FinishAnimCB(CAnimBlendAssociation* anim, void* data)`
(evidence: its address is pushed with SetFinishCallback 0x4CEBE0 at 0x454733 inside the unnamed function 0x4541E0 (just before the CPickup
code); also stored in a data table at 0x8D0958). Owner class UNCLEAR.
- Convention: `__cdecl void (CAnimBlendAssociation* anim [esp+4], T* data [esp+8])`, plain `ret`.
- Globals: .data block 0x977BB8 (CPed* ped), 0x977BBC (u8), 0x977BC0 (T*), 0x977BC4 (u8), 0x977BC8 (u32 time); now [0xB7CB84]=0; 0.5f [0x858B8C]=0.5f.
- Behaviour:
  1. `total = 0`; for k = 0..5: `t = data->+0xC[k]`; if t != 0: `CMessages::AddMessageQ(t, 3000, 0, 1)` 0x69F0B0 (cdecl), `total += 3000`.
  2. `[0x977BC8] = now + (u16)total` (`movzx eax, si`); `[0x977BC4] = 1`; `[0x977BC0] = data`; `[0x977BBC] = 0`.
  3. `if (data->+0x2C == 4) data->+0x2C = 7; else { data->+0x2C = 8; data->+0x24 = now; }`.
  4. `p = [0x977BB8]` (no null check): `p->+0x534 = 1` (m_nMoveState = PEDMOVE_STILL); `CAnimManager::BlendAnimation(p->m_pRwClump (+0x18),
     p->+0x4D4 (anim group), 3 (ANIM_ID_IDLE), 8.0f)` 0x4D4610.
  5. If `anim->+0x18 (m_fBlendAmount) > 0.5f` (`test ah,0x41; jne` -> else branch): if `[0x977BB8] != 0` `BlendAnimation(clump, 0, 0x93, 8.0f)`;
     in both cases `[0x977BB8] = 0`.
- gta-reversed: absent. Open questions: owner class and the meaning of the 0x977BB8.. block (no gr names).

## 0x45C4B0 replay block copy (sub_45c4b0, operator=-like)  (catalogue: unknown/REA, port level 0)
Catalogue name: <none>, proposed: `CPlayerPedData& CPlayerPedData::operator=(const CPlayerPedData& rhs)` (evidence: size 0xAC = gr
VALIDATE_SIZE(CPlayerPedData, 0xAC); only caller CPlayerInfo::operator= 0x45DEF0 passes `&rhs->m_PlayerData` (+4) with ECX = `&this->+4`; the
bitfield merges at +0x34 (gr m_bStoppedMoving.. flags dword) and +0xA0 (m_nLastHSMissileLOSTime:31 / m_bLastHSMissileLOS:1) match gr).
- Convention: `__thiscall CPlayerPedData* (this, const CPlayerPedData* rhs [esp+4])`, `ret 4`, EAX = this. 0x45C4B0 jumps to the HOODLUM body
  0x1561CF0. Uses ESI and EDI (pushed) - and READS the caller's ESI before overwriting it (see below).
- Behaviour (field copies in this order, size as in the bytes): dwords +0x00..+0x1C; bytes +0x20, +0x21; dwords +0x24..+0x30;
  flags +0x34 (below); dwords +0x38, +0x3C; bytes +0x40..+0x43; dwords +0x44..+0x70; 3 dwords +0x74..+0x7C; dword +0x80; bytes +0x84..+0x86;
  words +0x88, +0x8A; bytes +0x8C..+0x8E; dwords +0x90..+0x9C; +0xA0 (below); dwords +0xA4, +0xA8.
  - +0x34 (0x1561D48-0x1561D91): `d = this->+0x34` then, with S = the value of ESI at entry (no load into ESI precedes the use; the slot
    where a load would be is ten NOPs at 0x1561D4C-0x1561D55): `d = ((d ^ S) & 0x200) ^ S` (bit 9 from the old dest, every other bit from S),
    stored; then bits 10, 11, 12 are each merged from `rhs->+0x34` (`x = ((rhs ^ x) & bit) ^ x`, stored after each). Result: bits 10..12 =
    rhs, bit 9 = old dest, all other bits = S. From the only caller S = the destination CPlayerInfo* (0x45DEF0 sets ESI = this before the call).
  - +0xA0: `x = this ^ ((this ^ rhs) & 0x7FFFFFFF)` stored, then `x = rhs ^ ((rhs ^ x) & 0x7FFFFFFF)` stored -> the final value equals
    `rhs->+0xA0` (all 32 bits).
- Original bug / anomaly: the +0x34 flags word of the destination gets the bits of a POINTER (caller ESI) instead of rhs's bits 0..8 and
  13..31 (flags such as m_bStoppedMoving, m_bAdrenaline, m_bCanBeDamaged, m_bRequireHandleBreath, m_bGroupStuffDisabled, m_bRenderWeapon? -
  note bit 12 = m_bRenderWeapon IS copied). A faithful port must reproduce it with the address the original would have in ESI.
- gta-reversed: gr's CPlayerInfo::operator= (PlayerInfo.cpp:473) uses the compiler's default CPlayerPedData copy (all bits from rhs) - DIFFERS.
- Open questions: whether the ten NOPs at 0x1561D4C are a protection/packer artefact of this executable (the non-HOODLUM code it replaces is
  not available here); confirm with a 1.0 US image that has no HOODLUM relocation (address 0x1561D4C).

## 0x45DEF0 replay player-info restore helper (sub_45def0)  (catalogue: unknown/REA, port level 1)
Catalogue name: <none>; gr name: `CPlayerInfo& CPlayerInfo::operator=(const CPlayerInfo& rhs)` (PlayerInfo.cpp:472 comment "0x45DEF0"; size
0x190). Callers: CReplay::RestorePlayerInfoVariables 0x45E1F0, RestoreStuffFromMem 0x45ECD0, StorePlayerInfoVariables 0x45F020,
StoreStuffInMem 0x45F180.
- Convention: `__thiscall CPlayerInfo* (this, const CPlayerInfo* rhs [esp+4])`, `ret 4`, EAX = this. 0x45DEF0 jumps to the HOODLUM body 0x156AE50.
- Behaviour (order): `this->+0 = rhs->+0` (m_pPed); `CPlayerPedData::operator=(this+4, rhs+4)` 0x45C4B0 with ESI = this (matters, see there);
  dwords +0xB0..+0xD0; bytes +0xD4, +0xD5; dword +0xD8; bytes +0xDC..+0xDF; dwords +0xE0..+0x12C; word +0x130; dwords +0x134..+0x140; word
  +0x144; dword +0x148; bytes +0x14C..+0x154; word +0x156; 3 dwords +0x158..+0x160; 32 bytes +0x164..+0x183 copied one byte at a time
  (ascending); dword +0x184; byte +0x188; dword +0x18C. Padding bytes are not copied. No reference counting of the copied pointers.
- gta-reversed: PlayerInfo.cpp:473 copies member-wise (equivalent except for the CPlayerPedData flags anomaly of 0x45C4B0). Open questions: none.

## 0x49ACD0 CShopping price-modifier lookup (sub_49acd0)  (catalogue: unknown/REA, port level 0)
Name correction: catalogue "price-modifier lookup" -> `CShopping::RemovePriceModifier(uint32 key)` (gr Shopping.cpp:652, "inlined"; this
out-of-line copy is called from CRunningScript::ProcessCommands2200To2299 0x474900). 0x49ACD0 jumps to the HOODLUM body 0x156AC70.
- Convention: `__cdecl void (uint32 key [esp+4])`, plain `ret`.
- Globals: CShopping::ms_priceModifiers 0xA98650 ({key, price}[], 8 bytes), ms_numPriceModifiers [0xA9A7D0] (i32).
- Behaviour: `n = ms_numPriceModifiers`; find the FIRST `i` in [0, n) with `mods[i].key == key` (i = n if none / n <= 0). If `i == n` return.
  `n--`; store n; if `n > 0`: `mods[i] = mods[n]` (key then price). Only one entry is removed.
- gta-reversed: Shopping.cpp:652 DIFFERS: gr keeps iterating the (shrinking) span after a match and can remove several entries / compare the
  moved-in entry; the original stops at the first match. gr is not hooked (commented RH_ScopedInstall). Open questions: none.

## 0x49BDD0 CShopping price-modifier set (sub_49bdd0)  (catalogue: unknown/REA, port level 0)
Proposed: `CShopping::AddPriceModifier(uint32 key, int32 price)` (gr Shopping.cpp:632, "inlined"; caller ProcessCommands2200To2299 0x474900).
- Convention: `__cdecl void (uint32 key [esp+4], int32 price [esp+8])`, plain `ret`.
- Globals: ms_priceModifiers 0xA98650, ms_numPriceModifiers [0xA9A7D0]; CShopping::ms_prices 0xA986F0 (stride 0x18: +0 key, +4 price),
  ms_numPrices [0xA9A7CC].
- Behaviour: `n = ms_numPriceModifiers`; search i in [0, n) for `mods[i].key == key`: if found `mods[i].price = price`; if not found
  (i == n, also when n == 0) append `mods[n] = {key, price}`, `ms_numPriceModifiers = n + 1` (no capacity check). (n < 0: nothing appended.)
  Then (always) search `ms_prices[j].key == key` for j in [0, ms_numPrices): if found `ms_prices[j].+4 = price` (the redundant `j != -1` test
  is always true); stop.
- gta-reversed: Shopping.cpp:632 DIFFERS: gr returns right after updating an existing modifier and never updates ms_prices; the original
  also patches the matching ms_prices entry in both cases. Open questions: none.
